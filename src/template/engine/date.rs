//! Register date and time helpers for templates.
//!
//! [`DateNamespace`] provides the `date` namespace object registered as a
//! minijinja global by [`super::TemplateEngine`]. It also registers the flat
//! `date_*` filters and `is_*` tests used in pipelines.
//!
//! Template-facing operations split into three groups:
//!
//! - Namespace generators: `date.now()`, `date.today()`, `date.tomorrow()`,
//!   `date.yesterday()`, and `date.from_timestamp(ts)` produce formatted
//!   strings from the current instant, current date, or a Unix timestamp.
//! - Filters: `date_format`, `timestamp`, `date_add`, `date_sub`, `add_days`,
//!   `sub_days`, `add_months`, `sub_months`, `add_years`, `sub_years`,
//!   `start_of_month`, `end_of_month`, `weekday`, and `date_diff` transform a
//!   piped date/time string. `date_format` is prefixed to avoid minijinja's
//!   built-in `format` filter.
//! - Tests: `is_past`, `is_future`, and `is_leap_year` inspect a piped value.
//!
//! Date/time string parsing funnels through [`DatePoint::parse`] via
//! [`parse_local_datetime`]. A full datetime is tried first, falling back to a
//! bare ISO date (`YYYY-MM-DD` or reduced-precision `YYYY-MM`) at midnight.
//! Arithmetic filters re-serialize at the input's original precision via the
//! calendar owner.
//!
//! # Clock doctrine
//!
//! A naive datetime input means the reader's local clock time: it parses to a
//! UTC instant through the core date module's DST resolver, while a date-only
//! input stays a zone-free calendar date. Human-facing filters render the local
//! clock time; `timestamp`, `is_past`, `is_future`, and fixed-unit
//! [`date_diff`] measure the stored UTC instant. `date.now()`,
//! `date.today()`, `date.tomorrow()`, `date.yesterday()`, and
//! `date.from_timestamp()` read or render the local clock for display.

use std::sync::Arc;

use minijinja::{
    Environment, Error, ErrorKind,
    value::{Enumerator, Kwargs, Object, Value},
};

use super::error::{TemplateEngineResult, invalid_operation};
use crate::{
    DATE_PATTERN, DateTimeValue, DurationSeconds, DurationValue, TimeUnit,
    UNIT_SPELLINGS_HINT,
    temporal::{
        CalendarError, CalendarPeriod, DateComponent, DateDiff, DateError,
        DatePoint, PeriodEdge, weekday_from_number,
    },
};
const NAMESPACE_METHODS: &[&str] = &[
    "now",
    "today",
    "tomorrow",
    "yesterday",
    "from_timestamp",
    "start_of_week",
    "sow",
    "end_of_week",
    "eow",
    "start_of_month",
    "som",
    "end_of_month",
    "eom",
    "start_of_year",
    "soy",
    "end_of_year",
    "eoy",
    "weekday",
];
/// Backs the `date` namespace object. Stateless; see the module docs.
#[derive(Debug)]
pub(super) struct DateNamespace;

impl DateNamespace {
    /// Registers the `date` global plus every flat `date_*` filter and `is_*`
    /// test this module owns.
    ///
    /// Filters/tests are added first because they are zero-capture free
    /// functions and need no `self`; then `self` is consumed registering the
    /// `date` namespace object last.
    #[inline]
    pub(super) fn register(self, env: &mut Environment<'static>) {
        env.add_filter("date_format", date_format);
        env.add_filter("duration_format", duration_format);
        env.add_filter("durationformat", duration_format);
        env.add_filter("timestamp", timestamp);
        env.add_filter("date_add", date_add);
        env.add_filter("date_sub", date_sub);
        env.add_filter("add_days", add_days);
        env.add_filter("sub_days", sub_days);
        env.add_filter("add_months", add_months);
        env.add_filter("sub_months", sub_months);
        env.add_filter("add_years", add_years);
        env.add_filter("sub_years", sub_years);
        for &(name, period, edge) in &crate::temporal::PERIOD_SHORTHANDS {
            env.add_filter(name, move |value: &str| {
                render_period_edge(value, period, edge)
            });
        }
        env.add_filter("weekday", weekday);
        env.add_filter("date_diff", date_diff);
        env.add_test("is_past", is_past);
        env.add_test("is_future", is_future);
        env.add_test("is_leap_year", is_leap_year);
        env.add_global("date", Value::from_object(self));
    }
}

fn apply_offset(
    date_point: DatePoint,
    offset_val: &Value,
) -> TemplateEngineResult<DatePoint> {
    if let Some(n) = offset_val.as_i64() {
        return date_point
            .shift(
                n,
                TimeUnit::Day,
                &crate::temporal::local_zone().map_err(render_date_error)?,
            )
            .map_err(render_calendar_error);
    }
    if let Some(s) = offset_val.as_str() {
        let duration = DurationValue::parse(s).map_err(|source| {
            invalid_operation(format!("invalid duration offset {s:?}"), source)
        })?;
        return date_point
            .apply(
                &duration,
                &crate::temporal::local_zone().map_err(render_date_error)?,
            )
            .map_err(render_calendar_error);
    }
    Err(Error::new(
        ErrorKind::InvalidOperation,
        format!(
            "expected integer days or duration string for offset, got \
             {offset_val:?}"
        ),
    ))
}

/// A named anchor for the `date.*` shorthands: where the value starts and how
/// it is moved before the offset applies.
#[derive(Copy, Clone)]
enum Anchor {
    Now,
    Today,
    Tomorrow,
    Yesterday,
    StartOf(CalendarPeriod),
    EndOf(CalendarPeriod),
    Weekday(jiff::civil::Weekday),
}

impl Anchor {
    /// The date point a shorthand starts from when no reference is given.
    fn base(self) -> Result<DatePoint, CalendarError> {
        let now = DatePoint::from(DateTimeValue::from_timestamp(
            jiff::Timestamp::now(),
        ));
        match self {
            Self::Now => Ok(now),
            Self::Today
            | Self::Tomorrow
            | Self::Yesterday
            | Self::StartOf(_)
            | Self::EndOf(_)
            | Self::Weekday(_) => Ok(DatePoint::from(now.date()?)),
        }
    }

    /// Moves the date point as this anchor requires.
    fn transform(
        self,
        date_point: DatePoint,
    ) -> TemplateEngineResult<DatePoint> {
        match self {
            Self::Now | Self::Today => Ok(date_point),
            anchor => {
                let zone =
                    crate::temporal::local_zone().map_err(render_date_error)?;
                match anchor {
                    Self::Now | Self::Today => Ok(date_point),
                    Self::Tomorrow => date_point
                        .shift(1, TimeUnit::Day, &zone)
                        .map_err(render_calendar_error),
                    Self::Yesterday => date_point
                        .shift(-1, TimeUnit::Day, &zone)
                        .map_err(render_calendar_error),
                    Self::StartOf(calendar_period) => date_point
                        .start_of(calendar_period, &zone)
                        .map_err(render_calendar_error),
                    Self::EndOf(calendar_period) => date_point
                        .end_of(calendar_period, &zone)
                        .map_err(render_calendar_error),
                    Self::Weekday(target) => date_point
                        .with_weekday(target, &zone)
                        .map_err(render_calendar_error),
                }
            }
        }
    }

    /// Renders the anchored date point after the optional reference, offset
    /// and format arguments, which may also arrive as kwargs.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidOperation`] if the reference, offset, or format is
    ///   invalid, or the anchored date point is out of range.
    /// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides
    ///   `format`, `offset`, `reference`, or `reference_format`.
    #[expect(
        clippy::too_many_arguments,
        reason = "accepts format, offset, reference, and reference_format \
                  options"
    )]
    fn render(
        self,
        format: Option<&str>,
        offset: Option<Value>,
        reference: Option<&str>,
        reference_format: Option<&str>,
        kwargs: &Kwargs,
    ) -> TemplateEngineResult<String> {
        let format = format
            .or(kwargs.get::<Option<&str>>("format")?)
            .unwrap_or(DATE_PATTERN);
        let offset = offset.or(kwargs.get::<Option<Value>>("offset")?);
        let reference = reference.or(kwargs.get::<Option<&str>>("reference")?);
        let reference_format = reference_format
            .or(kwargs.get::<Option<&str>>("reference_format")?);
        kwargs.assert_all_used()?;

        let mut date_point = if let Some(ref_text) = reference {
            if let Some(ref_fmt) = reference_format {
                DatePoint::parse_with_pattern(ref_text, ref_fmt)
                    .map_err(render_date_error)?
            } else {
                parse_date_point(ref_text)?
            }
        } else {
            self.base().map_err(render_calendar_error)?
        };
        date_point = self.transform(date_point)?;

        if let Some(off_val) = offset {
            date_point = apply_offset(date_point, &off_val)?;
        }

        let zone = crate::temporal::local_zone().map_err(render_date_error)?;
        let local_datetime = date_point.local_datetime_in(&zone);
        let pat = pattern_to_strftime(format, local_datetime)?;
        let zoned = date_point
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?;
        format_strtime(&zoned, &pat, format)
    }
}

/// Builds the shared `(format, offset, reference, reference_format, **kwargs)`
/// template function for one anchor.
fn shorthand(anchor: Anchor) -> Value {
    Value::from_function(
        move |format: Option<&str>,
              offset: Option<Value>,
              reference: Option<&str>,
              reference_format: Option<&str>,
              kwargs: Kwargs|
              -> TemplateEngineResult<String> {
            anchor.render(format, offset, reference, reference_format, &kwargs)
        },
    )
}

/// Maps an ISO weekday number (`1` Monday through `7` Sunday) to a weekday.
fn weekday_or_error(n: i64) -> TemplateEngineResult<jiff::civil::Weekday> {
    weekday_from_number(n).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "weekday index {n} is out of range (expected 1..=7, Monday = \
                 1 through Sunday = 7)"
            ),
        )
    })
}

impl Object for DateNamespace {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "now" => Some(shorthand(Anchor::Now)),
            "today" => Some(shorthand(Anchor::Today)),
            "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
            "yesterday" => Some(shorthand(Anchor::Yesterday)),
            "start_of_week" | "sow" => {
                Some(shorthand(Anchor::StartOf(CalendarPeriod::Week)))
            }
            "end_of_week" | "eow" => {
                Some(shorthand(Anchor::EndOf(CalendarPeriod::Week)))
            }
            "start_of_month" | "som" => {
                Some(shorthand(Anchor::StartOf(CalendarPeriod::Month)))
            }
            "end_of_month" | "eom" => {
                Some(shorthand(Anchor::EndOf(CalendarPeriod::Month)))
            }
            "start_of_year" | "soy" => {
                Some(shorthand(Anchor::StartOf(CalendarPeriod::Year)))
            }
            "end_of_year" | "eoy" => {
                Some(shorthand(Anchor::EndOf(CalendarPeriod::Year)))
            }
            "weekday" => Some(Value::from_function(
                |n: i64,
                 format: Option<&str>,
                 reference: Option<&str>,
                 reference_format: Option<&str>,
                 kwargs: Kwargs|
                 -> TemplateEngineResult<String> {
                    let target = weekday_or_error(n)?;
                    Anchor::Weekday(target).render(
                        format,
                        None,
                        reference,
                        reference_format,
                        &kwargs,
                    )
                },
            )),
            "from_timestamp" => Some(Value::from_function(
                |unix_ts: i64,
                 kwargs: Kwargs|
                 -> TemplateEngineResult<String> {
                    let format = format_kwarg(&kwargs)?;
                    let timestamp =
                        jiff::Timestamp::new(unix_ts, 0).map_err(|_| {
                            Error::new(
                                ErrorKind::InvalidOperation,
                                format!("timestamp {unix_ts} is out of range"),
                            )
                        })?;
                    let zone = crate::temporal::local_zone()
                        .map_err(render_date_error)?;
                    let zoned = DateTimeValue::from_timestamp(timestamp)
                        .utc()
                        .to_zoned(zone);
                    format_strtime(&zoned, format, format)
                },
            )),
            _ => None,
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(NAMESPACE_METHODS)
    }
}

/// Parses `s` as a date or date-time expression via [`DatePoint::parse`].
///
/// Attaches [`DateError`] as the underlying source so render diagnostics keep
/// the full parse-failure chain.
fn parse_date_point(s: &str) -> TemplateEngineResult<DatePoint> {
    DatePoint::parse(s)
        .map_err(|err| invalid_operation(format!("invalid date {s:?}"), err))
}

/// Extracts the shared `format="..."` kwarg every `date.*` namespace method
/// takes, defaulting to [`DATE_PATTERN`], and rejects any other kwarg
/// via [`Kwargs::assert_all_used`].
///
/// This is the one place all five
/// `now`/`today`/`tomorrow`/`yesterday`/`from_timestamp` closures decide how
/// their optional `format=` argument is read.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `format` is present but is not a
///   string.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries any key besides
///   `format`.
fn format_kwarg(kwargs: &Kwargs) -> TemplateEngineResult<&str> {
    let format = kwargs.get::<Option<&str>>("format")?.unwrap_or(DATE_PATTERN);
    kwargs.assert_all_used()?;
    Ok(format)
}

/// Extracts the shared `unit="..."` kwarg [`date_add`], [`date_sub`], and
/// [`date_diff`] all take, defaulting to `"days"`, and rejects any other kwarg
/// via [`Kwargs::assert_all_used`], mirroring [`format_kwarg`] for the `date.*`
/// namespace methods' `format=` kwarg.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `unit` is not one of
///   [`TimeUnit::from_spelling`]'s accepted unit names or abbreviations.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries any key besides
///   `unit`.
fn unit_kwarg(kwargs: &Kwargs) -> TemplateEngineResult<TimeUnit> {
    let unit_str = kwargs.get::<Option<&str>>("unit")?.unwrap_or("days");
    kwargs.assert_all_used()?;
    TimeUnit::from_spelling(unit_str)
        .ok_or_else(|| unknown_unit_error(unit_str))
}

/// Formats a zoned date-time with Jiff's strict strftime implementation.
fn format_strtime(
    datetime: &jiff::Zoned,
    strftime: &str,
    source_pattern: &str,
) -> TemplateEngineResult<String> {
    jiff::fmt::strtime::format(strftime, datetime).map_err(|source| {
        invalid_operation(
            format!("invalid date format {source_pattern:?}"),
            source,
        )
    })
}

/// The shared date/time string parser for filters that work on the human clock
/// time (`date_format`, `weekday`, `is_leap_year`); instant-facing filters
/// parse via [`parse_date_point`] directly.
///
/// # Errors
///
/// Returns an invalid-operation error when `s` is not parseable.
fn parse_local_datetime(s: &str) -> Result<jiff::civil::DateTime, Error> {
    let point = parse_date_point(s)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    Ok(point.local_datetime_in(&zone))
}

/// `{{ value | date_format(format_string) }}` formats a piped date/time string
/// according to either a strftime pattern (containing `%`) or common
/// moment-dialect tokens (`YYYY MM DD HH mm ss`, `Do`/`S`,
/// `dddd`/`ddd`/`MMM`/`MMMM`, bracket literals like `[Daily]`).
///
/// Jiff [`fmt::strtime`] formats a zoned date-time, so `%Z` reflects the
/// resolved local zone even on either side of a DST fold. Leap seconds are
/// not representable. Week numbers use ISO `%V`/`%G`, never `%U` or `%W`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string or the local zone cannot be resolved; see [`parse_date_point`].
/// - [`ErrorKind::InvalidOperation`] if `pattern` is an invalid strftime
///   pattern or contains an unsupported moment token.
fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
    let point = parse_date_point(value)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    let datetime = point.local_datetime_in(&zone);
    let translated = pattern_to_strftime(pattern, datetime)?;
    let zoned =
        point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
    format_strtime(&zoned, &translated, pattern)
}

/// Translates `pattern` to a strftime pattern.
///
/// If `pattern` contains `%`, it is treated as a strftime pattern and passed
/// through. Otherwise, it is translated from common moment-dialect tokens:
/// bracket literals (`[Daily]`), `YYYY MM DD HH mm ss` family, `Do`/`S`
/// ordinals, and `dddd`/`ddd`/`MMM`/`MMMM`.
fn pattern_to_strftime(
    pattern: &str,
    datetime: jiff::civil::DateTime,
) -> TemplateEngineResult<String> {
    if pattern.contains('%') {
        return Ok(pattern.to_owned());
    }
    moment_to_strftime(pattern, datetime)
}

const MOMENT_TOKENS: &[(&str, &str)] = &[
    ("YYYY", "%Y"),
    ("YY", "%y"),
    ("MMMM", "%B"),
    ("MMM", "%b"),
    ("MM", "%m"),
    ("M", "%-m"),
    ("dddd", "%A"),
    ("ddd", "%a"),
    ("DD", "%d"),
    ("D", "%-d"),
    ("HH", "%H"),
    ("H", "%-H"),
    ("hh", "%I"),
    ("h", "%-I"),
    ("mm", "%M"),
    ("m", "%-M"),
    ("ss", "%S"),
    ("s", "%-S"),
    ("SSS", "%3f"),
    ("SS", "%2f"),
    ("S", "%1f"),
];

/// Returns the English ordinal suffix (`"st"`, `"nd"`, `"rd"`, `"th"`) for
/// `day`.
fn ordinal_suffix(day: i8) -> &'static str {
    match day {
        11..=13 => "th",
        _ => match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

/// Translates a moment-dialect pattern into a strftime format string.
fn moment_to_strftime(
    pattern: &str,
    datetime: jiff::civil::DateTime,
) -> TemplateEngineResult<String> {
    use std::fmt::Write as _;

    let mut result = String::with_capacity(pattern.len().saturating_mul(2));
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let Some(&current_byte) = bytes.get(i) else {
            break;
        };
        if current_byte == b'[' {
            i = i.saturating_add(1);
            let start = i;
            while i < len && bytes.get(i) != Some(&b']') {
                i = i.saturating_add(1);
            }
            if bytes.get(i) != Some(&b']') {
                return Err(Error::new(
                    ErrorKind::InvalidOperation,
                    format!(
                        "unclosed bracket literal in moment-dialect pattern \
                         `{pattern}`"
                    ),
                ));
            }
            let literal = pattern.get(start..i).unwrap_or_default();
            i = i.saturating_add(1);
            for ch in literal.chars() {
                if ch == '%' {
                    result.push_str("%%");
                } else {
                    result.push(ch);
                }
            }
        } else if pattern.get(i..).is_some_and(|tail| tail.starts_with("Do")) {
            i = i.saturating_add(2);
            let day = datetime.day();
            let _ = write!(result, "{day}{}", ordinal_suffix(day));
        } else if current_byte.is_ascii_alphabetic() {
            let mut matched = false;
            let tail = pattern.get(i..).unwrap_or_default();
            for &(tok, spec) in MOMENT_TOKENS {
                if tail.starts_with(tok) {
                    result.push_str(spec);
                    i = i.saturating_add(tok.len());
                    matched = true;
                    break;
                }
            }
            if !matched {
                let start = i;
                let ch = current_byte;
                while i < len && bytes.get(i) == Some(&ch) {
                    i = i.saturating_add(1);
                }
                let token = pattern.get(start..i).unwrap_or_default();
                return Err(Error::new(
                    ErrorKind::InvalidOperation,
                    format!(
                        "unsupported moment-dialect token `{token}` in \
                         pattern `{pattern}`; supported tokens are YYYY, YY, \
                         MMMM, MMM, MM, M, dddd, ddd, Do, DD, D, HH, H, hh, \
                         h, mm, m, ss, s, SSS, SS, S, and [bracketed literals]"
                    ),
                ));
            }
        } else {
            let ch = pattern
                .get(i..)
                .and_then(|tail| tail.chars().next())
                .unwrap_or(' ');
            i = i.saturating_add(ch.len_utf8());
            if ch == '%' {
                result.push_str("%%");
            } else {
                result.push(ch);
            }
        }
    }

    Ok(result)
}

/// `{{ value | duration_format }}` / `durationformat` renders a duration in
/// human-readable compound format (e.g. `"3 days, 4 hours"`).
fn duration_format(value: Value) -> TemplateEngineResult<String> {
    let dv = if let Some(s) = value.as_str() {
        DurationValue::parse(s).map_err(|source| {
            invalid_operation(format!("invalid duration {s:?}"), source)
        })?
    } else if let Ok(f) = f64::try_from(value) {
        DurationSeconds::try_from(f)
            .and_then(DurationValue::from_nominal_seconds)
            .map_err(|source| {
                invalid_operation(
                    format!("out-of-range duration seconds {f}"),
                    source,
                )
            })?
    } else {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "durationformat expects a duration string or number",
        ));
    };
    Ok(dv.humanize())
}
/// `{{ value | timestamp }}` converts a piped date/time string to Unix seconds.
///
/// A naive input means the reader's local zone, so it compares correctly
/// against file timestamps; an explicit-offset input converts directly; a
/// date-only input resolves at local-zone midnight.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string; see [`parse_date_point`].
fn timestamp(value: &str) -> TemplateEngineResult<i64> {
    parse_date_point(value)?
        .instant()
        .map(jiff::Timestamp::as_second)
        .map_err(render_calendar_error)
}

/// Returns the first or last day of the input's `calendar_period` at its
/// original precision.
///
/// Backs every name in
/// [`PERIOD_SHORTHANDS`](crate::temporal::PERIOD_SHORTHANDS): the
/// `start_of_week`/`sow` filter returns the first day (Monday) of the ISO
/// week, `end_of_week`/`eow` the last (Sunday), and the month and year
/// spellings behave the same on their period.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string; see [`parse_date_point`].
/// - [`ErrorKind::InvalidOperation`] if the boundary is outside the supported
///   range or its local time cannot be resolved; see [`render_calendar_error`].
fn render_period_edge(
    value: &str,
    calendar_period: CalendarPeriod,
    period_edge: PeriodEdge,
) -> TemplateEngineResult<String> {
    let date_point = parse_date_point(value)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    let shifted = match period_edge {
        PeriodEdge::Start => date_point.start_of(calendar_period, &zone),
        PeriodEdge::End => date_point.end_of(calendar_period, &zone),
    }
    .map_err(render_calendar_error)?;
    let pat = shifted.canonical_pattern();
    format_strtime(
        &shifted.zoned_datetime_in(&zone).map_err(render_calendar_error)?,
        pat,
        pat,
    )
}

/// `{{ value | date_add(n, unit="days") }}` adds `n` `unit`s to a piped
/// date/time string.
///
/// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
/// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
/// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`, or `"ms"`).
/// A sub-day shift of a date-only value moves the date only when it crosses
/// midnight (`25h` moves it one day; `5h` does not).
///
/// `"years"`, `"months"`, `"weeks"`, and `"days"` preserve the local clock
/// time across a DST transition; the remaining units shift the exact
/// instant, so the clock time can land earlier or later than a naive `n`-unit
/// shift (see [`shift_by_unit`]).
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string, `unit` is not an accepted unit name, or the shift overflows Jiff
///   representable range.
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja's Function trait extracts a filter's trailing Kwargs \
              argument by value; only `&self` methods on it are needed here"
)]
fn date_add(
    value: &str,
    n: i64,
    kwargs: Kwargs,
) -> TemplateEngineResult<String> {
    let unit = unit_kwarg(&kwargs)?;
    shift_by_unit(value, n, unit)
}

/// `{{ value | date_sub(n, unit="days") }}` subtracts `n` `unit`s from a piped
/// date/time string.
///
/// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
/// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
/// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`, or `"ms"`).
/// A sub-day shift of a date-only value moves the date only when it crosses
/// midnight (`25h` moves it one day; `5h` does not).
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string, `unit` is not an accepted unit name, `n` is [`i64::MIN`], or the
///   Jiff representable range.
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja's Function trait extracts a filter's trailing Kwargs \
              argument by value; only `&self` methods on it are needed here"
)]
fn date_sub(
    value: &str,
    n: i64,
    kwargs: Kwargs,
) -> TemplateEngineResult<String> {
    let unit = unit_kwarg(&kwargs)?;
    shift_by_unit(
        value,
        n.checked_neg().ok_or_else(date_out_of_range_error)?,
        unit,
    )
}

/// Shifts `value` by `n` `unit`s, applying calendar or fixed-duration
/// semantics per `unit`.
/// - Date-Time inputs ([`DatePoint::DateTime`]) shift calendar units on the
///   local clock time (preserving clock-time hour across DST) and sub-day units
///   exactly on the Date-Time.
/// - Calendar inputs ([`DatePoint::Calendar`]) shift on the local datetime at
///   midnight. A sub-day shift moves the date only when it crosses midnight
///   (`25h` moves it one day; `5h` does not).
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or the shift
///   exceeds the Jiff representable range (see the calendar error mapping).
fn shift_by_unit(
    value: &str,
    n: i64,
    unit: TimeUnit,
) -> TemplateEngineResult<String> {
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    let shifted_date_point = parse_date_point(value)?
        .shift(n, unit, &zone)
        .map_err(render_calendar_error)?;
    let pat = shifted_date_point.canonical_pattern();
    format_strtime(
        &shifted_date_point
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?,
        pat,
        pat,
    )
}

/// `{{ value | add_days(n) }}` is a convenience shortcut for
/// `{{ value | date_add(n, unit="days") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn add_days(value: &str, n: u64) -> TemplateEngineResult<String> {
    let n_i64 = i64::try_from(n).map_err(|_| date_out_of_range_error())?;
    shift_by_unit(value, n_i64, TimeUnit::Day)
}

/// `{{ value | sub_days(n) }}` is a convenience shortcut for
/// `{{ value | date_sub(n, unit="days") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn sub_days(value: &str, n: u64) -> TemplateEngineResult<String> {
    let n_i64 = i64::try_from(n).map_err(|_| date_out_of_range_error())?;
    let n_i64 = n_i64.checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Day)
}

/// `{{ value | add_months(n) }}` is a convenience shortcut for
/// `{{ value | date_add(n, unit="months") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn add_months(value: &str, n: u32) -> TemplateEngineResult<String> {
    shift_by_unit(value, i64::from(n), TimeUnit::Month)
}

/// `{{ value | sub_months(n) }}` is a convenience shortcut for
/// `{{ value | date_sub(n, unit="months") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn sub_months(value: &str, n: u32) -> TemplateEngineResult<String> {
    let n_i64 =
        i64::from(n).checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Month)
}

/// `{{ value | add_years(n) }}` is a convenience shortcut for
/// `{{ value | date_add(n, unit="years") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn add_years(value: &str, n: u32) -> TemplateEngineResult<String> {
    shift_by_unit(value, i64::from(n), TimeUnit::Year)
}

/// `{{ value | sub_years(n) }}` is a convenience shortcut for
/// `{{ value | date_sub(n, unit="years") }}`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not parseable or arithmetic
///   exceeds the Jiff representable range.
fn sub_years(value: &str, n: u32) -> TemplateEngineResult<String> {
    let n_i64 =
        i64::from(n).checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Year)
}

/// `{{ value | weekday }}` returns the ISO weekday number: `1` for Monday
/// through `7` for Sunday. When `n` is supplied, returns the formatted date of
/// that weekday in `value`'s week.
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja Function trait extracts trailing Kwargs argument by \
              value"
)]
fn weekday(
    value: &str,
    n: Option<i64>,
    kwargs: Kwargs,
) -> TemplateEngineResult<Value> {
    let date_point = parse_date_point(value)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    if let Some(target_num) = n {
        let format = format_kwarg(&kwargs)?;
        let target_weekday = weekday_or_error(target_num)?;
        let shifted = date_point
            .with_weekday(target_weekday, &zone)
            .map_err(render_calendar_error)?;
        let local_datetime = shifted.local_datetime_in(&zone);
        let pat = pattern_to_strftime(format, local_datetime)?;
        let zoned =
            shifted.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
        Ok(Value::from(format_strtime(&zoned, &pat, format)?))
    } else {
        kwargs.assert_all_used()?;
        Ok(Value::from(date_point.component(DateComponent::Weekday, &zone)))
    }
}

/// `{{ value | date_diff(other, unit="days") }}` returns the signed difference
/// from the piped value to `other`, positive when `other` is later.
///
/// The `unit` kwarg defaults to `"days"` and accepts `"years"`, `"months"`,
/// `"weeks"`, `"hours"`, `"minutes"`, `"seconds"`, or `"ms"`.
/// `"years"`/`"months"` are calendar counts: whole units elapsed, day-of-month
/// aware (see `DateValue::whole_months_until`),
/// always an `i64` regardless of input precision. The remaining units,
/// including `"weeks"`, `"ms"`, and the default `"days"`, are fixed-duration:
/// `f64` when both inputs carry a time component, otherwise an `i64` whole-unit
/// count.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` or `other` is not a parseable
///   date/time string (see [`parse_date_point`]) or `unit` is not one of
///   [`TimeUnit::from_spelling`]'s accepted names (see [`unit_kwarg`]).
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries any key besides
///   `unit`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja's Function trait extracts a filter's trailing Kwargs \
              argument by value; only `&self` methods on it are needed here"
)]
fn date_diff(
    value: &str,
    other: &str,
    kwargs: Kwargs,
) -> TemplateEngineResult<Value> {
    let unit = unit_kwarg(&kwargs)?;
    let from = parse_date_point(value)?;
    let to = parse_date_point(other)?;

    let diff = from
        .diff(
            to,
            unit,
            &crate::temporal::local_zone().map_err(render_date_error)?,
        )
        .map_err(render_calendar_error)?;
    Ok(match diff {
        DateDiff::Whole(n) => Value::from(n),
        DateDiff::Fractional(f) => Value::from(f),
    })
}

/// `{% if value is is_past %}` returns `true` when the piped date/time string
/// is before now.
///
/// A naive input is interpreted in the reader's local zone, so it compares
/// correctly against file timestamps.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string; see [`parse_date_point`].
fn is_past(value: &str) -> TemplateEngineResult<bool> {
    let instant =
        parse_date_point(value)?.instant().map_err(render_calendar_error)?;
    Ok(instant < jiff::Timestamp::now())
}

/// `{% if value is is_future %}` mirrors [`is_past`] for future instants.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a parseable date/time
///   string; see [`parse_date_point`].
fn is_future(value: &str) -> TemplateEngineResult<bool> {
    let instant =
        parse_date_point(value)?.instant().map_err(render_calendar_error)?;
    Ok(instant > jiff::Timestamp::now())
}

/// `{% if value is is_leap_year %}` accepts either an integer year (`2024 is
/// is_leap_year`) or a date/time string checked through
/// [`parse_local_datetime`].
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] (via [`leap_year_input_error`]) if `value`
///   is neither an integer year representable as [`i32`] nor a parseable
///   date/time string; see [`parse_local_datetime`].
/// - `InvalidOperation` if the year is outside the Jiff-supported civil-date
///   range.
fn is_leap_year(value: &Value) -> TemplateEngineResult<bool> {
    let year = if let Some(year) = value.as_i64() {
        i16::try_from(year).map_err(|_| leap_year_input_error(value))?
    } else if let Some(s) = value.as_str() {
        parse_local_datetime(s)?.year()
    } else {
        return Err(leap_year_input_error(value));
    };
    if jiff::civil::Date::new(year, 1, 1).is_err() {
        return Err(leap_year_input_error(value));
    }
    let year = i32::from(year);
    Ok(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0))
}

/// Builds the `is_leap_year` error for an argument that's neither a
/// representable year nor a valid date string.
fn leap_year_input_error(value: &Value) -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        format!(
            "is_leap_year expects an integer year or a date string, got \
             {value:?}"
        ),
    )
}

/// Builds the error for calendar arithmetic that exceeds the supported Jiff
/// date or time range.
///
/// Checked operations return this instead of producing an invalid value.
fn date_out_of_range_error() -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        "date arithmetic overflowed the supported range",
    )
}

/// Translates a core parse error into a render error.
fn render_date_error(error: DateError) -> Error {
    match error {
        DateError::LocalZoneLookup {
            input,
        } => invalid_operation(
            format!("local timezone lookup failed for {input:?}"),
            DateError::LocalZoneLookup {
                input,
            },
        ),
        DateError::InvalidValue {
            ..
        }
        | DateError::YearNotFourDigits {
            ..
        } => invalid_operation(error.to_string(), error),
    }
}

/// Translates a computed calendar failure into a render error.
fn render_calendar_error(error: CalendarError) -> Error {
    match error {
        CalendarError::OutOfRange => date_out_of_range_error(),
        CalendarError::Zone {
            input,
        } => invalid_operation(
            format!("local timezone lookup failed for {input:?}"),
            CalendarError::Zone {
                input,
            },
        ),
    }
}

/// Builds the error for a `unit="..."` kwarg naming anything outside
/// [`TimeUnit::from_spelling`]'s accepted unit names.
///
/// The expected-spellings hint comes from [`UNIT_SPELLINGS_HINT`], kept beside
/// the unit registry so the message cannot drift from what [`unit_kwarg`]
/// actually accepts.
///
/// Shared by [`date_add`], [`date_sub`], and [`date_diff`] via [`unit_kwarg`].
fn unknown_unit_error(unit: &str) -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        format!("unknown unit {unit:?} (expected {UNIT_SPELLINGS_HINT})"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ZoneGuard;

    fn env() -> Environment<'static> {
        // Every date filter reads the local zone through the core parser, so
        // the render below must own `TZ` against concurrent tests.
        ZoneGuard::keep();
        let mut env = Environment::new();
        DateNamespace.register(&mut env);
        env
    }

    /// Renders `{{ value | <filter_call> }}` under the registered
    /// environment and asserts the exact output.
    ///
    /// Shared by the per-filter `shifts_a_piped_date` tables.
    fn assert_filter_render(input: &str, filter_call: &str, expected: &str) {
        let rendered = env()
            .render_str(
                &format!("{{{{ value | {filter_call} }}}}"),
                minijinja::context! { value => input },
            )
            .expect("render succeeds");
        pretty_assertions::assert_eq!(rendered, expected);
    }

    mod get_value {
        use super::*;

        #[test]
        fn get_value_returns_none_for_an_unknown_key() {
            let ops = Arc::new(DateNamespace);

            assert!(ops.get_value(&Value::from("later")).is_none());
        }

        #[test]
        fn get_value_returns_none_for_a_non_string_key() {
            let ops = Arc::new(DateNamespace);

            assert!(ops.get_value(&Value::from(1)).is_none());
        }
    }

    mod now {
        use pretty_assertions::assert_eq;

        use super::*;

        /// Asserts the rendered shape, not the current date.
        ///
        /// The clock makes the literal value nondeterministic, but the default
        /// format still has a stable `YYYY-MM-DD` shape.
        #[test]
        fn now_formats_with_the_default_format_when_no_kwarg_is_given() {
            let rendered = env()
                .render_str("{{ date.now() }}", minijinja::context!())
                .expect("render succeeds");

            assert_eq!(rendered.len(), "YYYY-MM-DD".len());
            assert!(
                rendered.chars().all(|c| c.is_ascii_digit() || c == '-'),
                "expected an all-digit-or-hyphen date, got {rendered:?}"
            );
            assert_eq!(
                rendered.as_bytes().get(4),
                Some(&b'-'),
                "expected a hyphen at index 4 of {rendered:?}"
            );
            assert_eq!(
                rendered.as_bytes().get(7),
                Some(&b'-'),
                "expected a hyphen at index 7 of {rendered:?}"
            );
        }

        #[test]
        fn now_formats_using_an_explicit_format_kwarg() {
            let rendered = env()
                .render_str(
                    r#"{{ date.now(format="%Y") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered.len(), 4);
            assert!(
                rendered.chars().all(|c| c.is_ascii_digit()),
                "expected an all-digit year, got {rendered:?}"
            );
        }

        /// Regression: invalid format specifiers such as %J must surface as a
        /// normal render error rather than panic during formatting.
        #[test]
        fn now_returns_an_error_instead_of_panicking_on_an_invalid_format() {
            let error = env()
                .render_str(
                    r#"{{ date.now(format="%J") }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid format specifier fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        #[test]
        fn now_rejects_a_non_string_format_kwarg() {
            let error = env()
                .render_str("{{ date.now(format=1) }}", minijinja::context!())
                .expect_err("non-string format kwarg fails");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        #[test]
        fn now_rejects_an_unknown_kwarg() {
            let error = env()
                .render_str("{{ date.now(bogus=1) }}", minijinja::context!())
                .expect_err("unknown kwarg fails");

            assert_eq!(error.kind(), ErrorKind::TooManyArguments);
        }
    }

    /// `today`/`tomorrow`/`yesterday` share the same nondeterministic clock,
    /// but their relative dates are deterministic within one render window.
    ///
    /// All three render inside one template so they read the clock in a
    /// single evaluation; a local-midnight crossing between separate renders
    /// would otherwise flake the one-day-apart assertions.
    mod today_tomorrow_yesterday {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn tomorrow_and_yesterday_are_one_day_from_today() {
            let rendered = env()
                .render_str(
                    "{{ date.yesterday() }}/{{ date.today() }}/{{ \
                     date.tomorrow() }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");
            let mut parts = rendered.split('/');
            let yesterday: jiff::civil::Date = parts
                .next()
                .expect("yesterday segment")
                .parse()
                .expect("valid ISO date");
            let today: jiff::civil::Date = parts
                .next()
                .expect("today segment")
                .parse()
                .expect("valid ISO date");
            let tomorrow: jiff::civil::Date = parts
                .next()
                .expect("tomorrow segment")
                .parse()
                .expect("valid ISO date");
            assert_eq!(
                today.tomorrow().expect("date has a tomorrow"),
                tomorrow
            );
            assert_eq!(
                today.yesterday().expect("date has a yesterday"),
                yesterday
            );
        }

        #[rstest]
        #[case::today("today")]
        #[case::tomorrow("tomorrow")]
        #[case::yesterday("yesterday")]
        fn accepts_an_explicit_format_kwarg(#[case] function: &str) {
            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ date.{function}(format="%Y") }}}}"#),
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered.len(), 4);
            assert!(rendered.chars().all(|c| c.is_ascii_digit()));
        }

        #[rstest]
        #[case::today("today")]
        #[case::tomorrow("tomorrow")]
        #[case::yesterday("yesterday")]
        fn returns_an_error_instead_of_panicking_on_an_invalid_format(
            #[case] function: &str,
        ) {
            let error = env()
                .render_str(
                    &format!(r#"{{{{ date.{function}(format="%J") }}}}"#),
                    minijinja::context!(),
                )
                .expect_err("invalid format specifier fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod from_timestamp {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn formats_the_unix_epoch_with_the_default_format() {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    "{{ date.from_timestamp(0) }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1970-01-01");
        }

        #[test]
        fn accepts_an_explicit_format_kwarg() {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    r#"{{ date.from_timestamp(0, format="%H:%M") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "00:00");
        }

        #[test]
        fn formats_in_the_local_clock_time() {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // Midnight UTC reads as 02:00 local on the same calendar day.
            let rendered = env()
                .render_str(
                    r#"{{ date.from_timestamp(0, format="%H:%M") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "02:00");
        }

        #[test]
        fn rejects_a_timestamp_out_of_range_instead_of_panicking() {
            let error = env()
                .render_str(
                    "{{ date.from_timestamp(9999999999999999) }}",
                    minijinja::context!(),
                )
                .expect_err("out-of-range timestamp fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod enumerate {
        use super::*;

        #[test]
        fn enumerate_lists_every_method() {
            let ops = Arc::new(DateNamespace);

            assert!(matches!(
                ops.enumerate(),
                Enumerator::Str(NAMESPACE_METHODS)
            ));
        }

        #[test]
        fn every_enumerated_method_resolves_via_get_value() {
            let ops = Arc::new(DateNamespace);

            for method in NAMESPACE_METHODS {
                assert!(
                    ops.get_value(&Value::from(*method)).is_some(),
                    "{method:?} is enumerated but get_value has no matching \
                     arm"
                );
            }
        }
    }

    mod date_format {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;
        use crate::DateError;

        #[rstest]
        #[case::date_only("2026-07-23", "%d/%m/%Y", "23/07/2026")]
        #[case::datetime_input("2026-07-23 14:30", "%H:%M", "14:30")]
        #[case::rfc3339_with_z_offset("2026-07-29T14:30:00Z", "%Y", "2026")]
        #[case::rfc3339_with_numeric_offset(
            "2026-07-29T14:30:00+02:00",
            "%H:%M",
            "12:30"
        )]
        #[case::fractional_seconds(
            "2026-07-29T14:30:00.123",
            "%H:%M:%S%.3f",
            "14:30:00.123"
        )]
        #[case::year_month_reduced_precision("2026-08", "%Y-%m", "2026-08")]
        fn reformats_a_piped_date_string(
            #[case] input: &str,
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn formats_fall_back_fold_abbreviations_from_the_resolved_zone() {
            ZoneGuard::set("America/New_York");

            let env = env();
            let daylight = env
                .render_str(
                    r#"{{ value | date_format("%Y-%m-%d %H:%M %Z") }}"#,
                    minijinja::context! { value => "2026-11-01T05:30:00Z" },
                )
                .expect("daylight occurrence renders");
            let standard = env
                .render_str(
                    r#"{{ value | date_format("%Y-%m-%d %H:%M %Z") }}"#,
                    minijinja::context! { value => "2026-11-01T06:30:00Z" },
                )
                .expect("standard occurrence renders");

            assert_eq!(daylight, "2026-11-01 01:30 EDT");
            assert_eq!(standard, "2026-11-01 01:30 EST");
        }

        #[test]
        fn renders_an_explicit_offset_input_in_the_local_clock_time() {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // The everyday spelling is the reader's clock time: 14:30Z is 16:30
            // local, not the UTC 12:30 the old behavior printed.
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("%H:%M") }}"#,
                    minijinja::context! { value => "2026-07-29T14:30:00Z" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, "16:30");
        }

        #[test]
        fn reformats_a_naive_input_to_its_own_clock_time() {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // A naive datetime means the reader's clock time, so displaying it
            // back yields the written time in any zone.
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("%H:%M") }}"#,
                    minijinja::context! { value => "2026-07-29 14:30" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, "14:30");
        }

        #[test]
        fn rejects_an_unparseable_date_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "not a date" | date_format("%Y") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unparseable date fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        /// Regression: `ParsedDate::parse` discarded the underlying
        /// [`DateError`], leaving template render errors with an empty source
        /// chain.
        #[test]
        fn rejects_an_unparseable_date_preserving_the_parse_error_source() {
            use std::error::Error as _;

            let error = env()
                .render_str(
                    r#"{{ "not a date" | date_format("%Y") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unparseable date fails cleanly");

            let source =
                error.source().expect("render error carries its source");
            assert!(
                source.downcast_ref::<DateError>().is_some(),
                "source chain must retain the core DateError, got {source:?}"
            );
        }

        /// Regression: for a datetime-shaped input that fails for a
        /// datetime-specific reason, the old parser discarded the datetime
        /// error and surfaced an unrelated date-only shape mismatch instead
        /// (spec N4).
        #[test]
        fn surfaces_the_datetime_parsers_error_not_the_date_only_fallbacks() {
            use std::error::Error as _;

            let input = "2026-07-29T99:99:99"; // datetime-shaped, invalid time
            let datetime_error = DateTimeValue::parse_iso(input)
                .expect_err("datetime parser rejects an invalid time");

            let error = env()
                .render_str(
                    r#"{{ value | date_format("%Y") }}"#,
                    minijinja::context! { value => input },
                )
                .expect_err("unparseable datetime fails cleanly");

            let source =
                error.source().expect("render error carries its source");
            assert_eq!(
                source.to_string(),
                datetime_error.to_string(),
                "the surfaced source must be the datetime parser's own error, \
                 not the date-only fallback's unrelated shape-mismatch error"
            );
        }

        #[test]
        fn rejects_an_invalid_format_specifier_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_format("%J") }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid format specifier fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
        #[rstest]
        #[case::moment_iso_date("2026-07-29", "YYYY-MM-DD", "2026-07-29")]
        #[case::bracket_literal(
            "2026-07-29",
            "[Daily] YYYY-MM-DD",
            "Daily 2026-07-29"
        )]
        #[case::ordinal_day("2026-07-29", "Do MMMM YYYY", "29th July 2026")]
        #[case::weekday_and_month(
            "2026-07-29",
            "dddd, MMMM Do, YYYY",
            "Wednesday, July 29th, 2026"
        )]
        #[case::short_weekday_month(
            "2026-07-29",
            "ddd, MMM D, YY",
            "Wed, Jul 29, 26"
        )]
        #[case::time_tokens("2026-07-29T14:30:05", "HH:mm:ss", "14:30:05")]
        #[case::time_fractional(
            "2026-07-29T14:30:05.123",
            "HH:mm:ss.SSS",
            "14:30:05.123"
        )]
        fn formats_moment_tokens(
            #[case] input: &str,
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn rejects_unsupported_moment_token_with_token_and_dialect_name() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-29" | date_format("YYYY Q") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unsupported moment token fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            let msg = error.to_string();
            assert!(msg.contains("unsupported moment-dialect token `Q`"));
            assert!(msg.contains("supported tokens are"));
        }
        #[test]
        fn rejects_unclosed_bracket_literal_in_moment_dialect() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-29" | date_format("[Daily YYYY-MM-DD") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unclosed bracket must fail");
            assert!(
                error.to_string().contains("unclosed bracket literal"),
                "expected unclosed bracket error, got: {error}"
            );
        }

        #[test]
        fn format_bindings_never_emit_percent_plus() {
            let sample_pattern = "YYYY-MM-DD HH:mm:ss [Daily]";
            let dt = jiff::civil::Date::new(2026, 7, 29)
                .expect("valid date")
                .at(14, 30, 0, 0);

            let translated = pattern_to_strftime(sample_pattern, dt).unwrap();
            assert!(!translated.contains("%+"));
        }
    }

    mod durationformat {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::compound_days_hours("3d 4h", "3 days, 4 hours")]
        #[case::single_day("1d", "1 day")]
        #[case::single_hour("1h", "1 hour")]
        #[case::minutes_seconds("1m 30s", "1 minute, 30 seconds")]
        #[case::zero_duration("0s", "0 seconds")]
        #[case::negative_duration("-3d 4h", "-3 days, 4 hours")]
        #[case::sub_second_milliseconds("500ms", "500 milliseconds")]
        #[case::fractional_seconds("1.5s", "1 second, 500 milliseconds")]
        #[case::sub_millisecond_residual("0.0001s", "< 1 millisecond")]
        #[case::negative_fractional_seconds(
            "-1.5s",
            "-1 second, 500 milliseconds"
        )]
        fn formats_compound_durations(
            #[case] input: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ "{input}" | durationformat }}}}"#),
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn formats_duration_from_numeric_seconds() {
            let rendered = env()
                .render_str(
                    r"{{ 86400 | durationformat }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1 day");
        }

        #[test]
        fn duration_format_is_an_alias_of_durationformat() {
            let rendered = env()
                .render_str(
                    r"{{ 90 | duration_format }}|{{ 90 | durationformat }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1 minute, 30 seconds|1 minute, 30 seconds");
        }

        #[test]
        fn rejects_invalid_duration_value() {
            let error = env()
                .render_str(
                    r#"{{ "not a duration" | durationformat }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid duration fails");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod timestamp {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::epoch("1970-01-01", 0)]
        #[case::one_second_after_epoch("1970-01-01 00:00:01", 1)]
        fn converts_a_piped_date_to_unix_seconds(
            #[case] input: &str,
            #[case] expected: i64,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    "{{ value | timestamp }}",
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected.to_string());
        }

        #[test]
        fn interprets_a_naive_datetime_in_the_local_zone() {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-29 14:30:00" | timestamp
                        - "2026-07-29T12:30:00Z" | timestamp }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            // Local 14:30 in UTC+2 is the same instant as 12:30Z.
            assert_eq!(rendered, "0");
        }

        #[test]
        fn resolves_a_date_only_input_at_local_midnight() {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-29" | timestamp
                        - "2026-07-29T00:00:00" | timestamp }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "0");
        }

        #[test]
        fn resolves_a_spring_forward_gap_input_instead_of_failing() {
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());

            // 02:30 does not exist on 2026-03-08; the adopted 'compatible'
            // policy shifts it forward by the gap to 03:30 EDT (07:30Z).
            let rendered = env()
                .render_str(
                    r#"{{ "2026-03-08 02:30:00" | timestamp
                        - "2026-03-08T07:30:00Z" | timestamp }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "0");
        }

        #[test]
        fn rejects_an_unparseable_date_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "not a date" | timestamp }}"#,
                    minijinja::context!(),
                )
                .expect_err("unparseable date fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod add_and_sub_days {
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::crosses_into_a_leap_day(
            "2024-02-28",
            "add_days(1)",
            "2024-02-29"
        )]
        #[case::crosses_a_non_leap_month_end(
            "2023-02-28",
            "add_days(1)",
            "2023-03-01"
        )]
        #[case::sub_days_symmetric("2024-02-29", "sub_days(1)", "2024-02-28")]
        #[case::preserves_the_time_component(
            "2026-07-23 14:30",
            "add_days(1)",
            "2026-07-24T14:30:00"
        )]
        fn shifts_a_piped_date(
            #[case] input: &str,
            #[case] filter_call: &str,
            #[case] expected: &str,
        ) {
            assert_filter_render(input, filter_call, expected);
        }

        #[test]
        fn overflow_returns_an_error_instead_of_panicking() {
            let error = env()
                .render_str(
                    "{{ \"2026-07-23\" | add_days(100000000000000) }}",
                    minijinja::context!(),
                )
                .expect_err("date-range overflow fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod add_and_sub_months {
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::clamps_to_a_shorter_month(
            "2023-01-31",
            "add_months(1)",
            "2023-02-28"
        )]
        #[case::clamps_into_a_leap_february(
            "2024-01-31",
            "add_months(1)",
            "2024-02-29"
        )]
        #[case::sub_months_symmetric(
            "2023-03-31",
            "sub_months(1)",
            "2023-02-28"
        )]
        fn shifts_a_piped_date(
            #[case] input: &str,
            #[case] filter_call: &str,
            #[case] expected: &str,
        ) {
            assert_filter_render(input, filter_call, expected);
        }
    }

    mod add_and_sub_years {
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::clamps_a_leap_day_into_a_non_leap_year(
            "2024-02-29",
            "add_years(1)",
            "2025-02-28"
        )]
        #[case::lands_on_a_leap_day_again(
            "2024-02-29",
            "add_years(4)",
            "2028-02-29"
        )]
        #[case::sub_years_symmetric("2028-02-29", "sub_years(4)", "2024-02-29")]
        fn shifts_a_piped_date(
            #[case] input: &str,
            #[case] filter_call: &str,
            #[case] expected: &str,
        ) {
            assert_filter_render(input, filter_call, expected);
        }
    }

    mod start_and_end_of_month {
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::start_of_month("2024-02-15", "start_of_month", "2024-02-01")]
        #[case::end_of_a_leap_february(
            "2024-02-15",
            "end_of_month",
            "2024-02-29"
        )]
        #[case::end_of_a_non_leap_february(
            "2023-02-15",
            "end_of_month",
            "2023-02-28"
        )]
        #[case::end_of_month_preserves_the_time_component(
            "2024-02-15 10:00",
            "end_of_month",
            "2024-02-29T10:00:00"
        )]
        fn shifts_a_piped_date(
            #[case] input: &str,
            #[case] filter_call: &str,
            #[case] expected: &str,
        ) {
            assert_filter_render(input, filter_call, expected);
        }
    }

    mod weekday {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::monday("2026-07-20", 1)]
        #[case::tuesday("2026-07-21", 2)]
        #[case::wednesday("2026-07-22", 3)]
        #[case::thursday("2026-07-23", 4)]
        #[case::friday("2026-07-24", 5)]
        #[case::saturday("2026-07-25", 6)]
        #[case::sunday("2026-07-26", 7)]
        fn returns_the_iso_weekday_number(
            #[case] input: &str,
            #[case] expected: u32,
        ) {
            let rendered = env()
                .render_str(
                    "{{ value | weekday }}",
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected.to_string());
        }
    }

    mod date_diff {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;
        use crate::test_support::ZoneGuard;

        #[test]
        fn returns_an_integer_day_count_when_neither_input_has_time() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_diff("2026-07-30") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "7");
        }

        #[test]
        fn milliseconds_are_a_whole_count_for_date_only_inputs() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_diff("2026-07-24", unit="ms") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "86400000");
        }

        #[rstest]
        #[case::hours(
            "2026-07-23 00:00:00",
            "2026-07-24 12:00:00",
            "hours",
            "36.0"
        )]
        #[case::minutes(
            "2026-07-23 00:00:00",
            "2026-07-23 01:30:00",
            "minutes",
            "90.0"
        )]
        #[case::seconds(
            "2026-07-23 00:00:00",
            "2026-07-23 00:01:00",
            "seconds",
            "60.0"
        )]
        #[case::default_unit_is_days(
            "2026-07-23 00:00:00",
            "2026-07-25 00:00:00",
            "days",
            "2.0"
        )]
        #[case::fractional_days_from_a_sub_day_remainder(
            "2026-07-23 00:00:00",
            "2026-07-23 12:00:00",
            "days",
            "0.5"
        )]
        #[case::weeks_as_a_fixed_unit(
            "2026-07-23 00:00:00",
            "2026-08-06 00:00:00",
            "weeks",
            "2.0"
        )]
        #[case::milliseconds_as_a_fixed_unit(
            "2026-07-23 00:00:00",
            "2026-07-23T00:00:01.5",
            "ms",
            "1500.0"
        )]
        fn returns_sub_day_precision_when_both_inputs_have_time(
            #[case] value: &str,
            #[case] other: &str,
            #[case] unit: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(
                    "{{ value | date_diff(other, unit=unit) }}",
                    minijinja::context! { value, other, unit },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn negative_when_other_precedes_the_piped_value() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-24" | date_diff("2026-07-23") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "-1");
        }

        #[test]
        fn rejects_an_unknown_unit_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_diff("2026-07-24", unit="fortnights") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unknown unit fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        #[rstest]
        #[case::exact_anniversary("2020-06-15", "2025-06-15", "5")]
        #[case::before_anniversary_rounds_down("2020-06-15", "2025-06-14", "4")]
        #[case::after_anniversary_rounds_up_to_the_next_whole_year(
            "2020-06-15",
            "2025-06-16",
            "5"
        )]
        #[case::negative_when_other_precedes_the_piped_value(
            "2025-06-15",
            "2020-06-15",
            "-5"
        )]
        fn computes_whole_calendar_years_between_dates(
            #[case] value: &str,
            #[case] other: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(
                    r#"{{ value | date_diff(other, unit="years") }}"#,
                    minijinja::context! { value, other },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[rstest]
        #[case::exact_month_boundary("2026-01-15", "2026-04-15", "3")]
        #[case::before_day_of_month_rounds_down(
            "2026-01-15",
            "2026-04-14",
            "2"
        )]
        #[case::after_day_of_month_rounds_up_to_the_next_whole_month(
            "2026-01-15",
            "2026-04-16",
            "3"
        )]
        #[case::spans_a_year_boundary("2025-11-15", "2026-02-15", "3")]
        #[case::negative_when_other_precedes_the_piped_value(
            "2026-04-15",
            "2026-01-15",
            "-3"
        )]
        fn computes_whole_calendar_months_between_dates(
            #[case] value: &str,
            #[case] other: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(
                    r#"{{ value | date_diff(other, unit="months") }}"#,
                    minijinja::context! { value, other },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn years_and_months_ignore_the_time_of_day_component() {
            let rendered = env()
                .render_str(
                    r#"{{ "2020-06-15 23:59:59" | date_diff("2025-06-15 00:00:00", unit="years") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "5");
        }

        #[rstest]
        #[case::naive_input("2026-07-29T14:30:00")]
        #[case::explicit_offset_input("2026-07-29T12:30:00Z")]
        fn truncates_to_a_whole_hour_count_for_a_mixed_date_and_datetime_pair(
            #[case] other: &str,
        ) {
            ZoneGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // Regression: the mixed-precision branch previously subtracted
            // whichever naive representations the two parsers happened to
            // produce; it must subtract calendar clocks under the pinned
            // zone instead. A naive 14:30 local input and its 12:30Z
            // explicit-offset equivalent both resolve to the same 14:30
            // local clock time, so both land on the same 14-hour truncated
            // difference (the mixed branch is i64, not f64).
            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-29" | date_diff(other, unit="hours") }}"#,
                    minijinja::context! { other },
                )
                .expect("render succeeds");

            assert_eq!(rendered, "14");
        }
    }

    mod date_add_and_date_sub {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn date_add_defaults_to_days() {
            let rendered = env()
                .render_str(
                    r"{{ '2026-07-26' | date_add(5) }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "2026-07-31");
        }

        #[test]
        fn date_add_accepts_units_and_singular_plural_forms() {
            let rendered = env()
                .render_str(
                    r"{{ '2026-07-26' | date_add(1, unit='month') }}-{{ '2026-07-26' | date_add(2, unit='years') }}-{{ '2026-07-26 12:00:00' | date_add(3, unit='hours') }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "2026-08-26-2028-07-26-2026-07-26T15:00:00");
        }

        #[test]
        fn date_sub_subtracts_units() {
            let rendered = env()
                .render_str(
                    r"{{ '2026-07-26' | date_sub(10, unit='days') }}-{{ '2026-07-26' | date_sub(1, unit='year') }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "2026-07-16-2025-07-26");
        }

        #[test]
        fn date_add_rejects_unknown_unit() {
            let error = env()
                .render_str(
                    r"{{ '2026-07-26' | date_add(1, unit='fortnight') }}",
                    minijinja::context!(),
                )
                .expect_err("unknown unit fails");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        /// Regression: `1e16` seconds fits `i64` (so the `checked_mul` guard
        /// passes) but exceeds the Jiff civil-date range, returning an error.
        #[test]
        fn date_add_returns_an_error_for_an_out_of_range_second_magnitude() {
            let error = env()
                .render_str(
                    r"{{ '2026-07-26' | date_add(10000000000000000, unit='seconds') }}",
                    minijinja::context!(),
                )
                .expect_err("out-of-range shift magnitude fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        /// Pins the declared split between calendar and exact units: a day unit
        /// shifts the calendar clock across the 2026-03-08 gap, while 24
        /// hours shifts the instant and lands an hour later on the clock.
        #[test]
        fn a_day_unit_preserves_the_clock_time_across_a_dst_gap_while_hours_do_not()
         {
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());

            let rendered = env()
                .render_str(
                    r"{{ '2026-03-07 17:00:00' | date_add(1, unit='days') }}-{{ '2026-03-07 17:00:00' | date_add(24, unit='hours') }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "2026-03-08T17:00:00-2026-03-08T18:00:00");
        }
    }
    mod is_past_and_is_future {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn a_far_past_date_is_past_but_not_future() {
            let rendered = env()
                .render_str(
                    "{{ '2000-01-01' is is_past }}{{ '2000-01-01' is \
                     is_future }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "TrueFalse");
        }

        #[test]
        fn a_far_future_date_is_future_but_not_past() {
            let rendered = env()
                .render_str(
                    "{{ '2999-01-01' is is_past }}{{ '2999-01-01' is \
                     is_future }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "FalseTrue");
        }

        #[test]
        fn rejects_an_unparseable_date_instead_of_panicking() {
            let error = env()
                .render_str(
                    "{{ 'not a date' is is_past }}",
                    minijinja::context!(),
                )
                .expect_err("unparseable date fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }

        #[test]
        fn current_timestamp_is_neither_past_nor_future() {
            let now = jiff::Timestamp::now();
            let formatted = now.strftime("%Y-%m-%dT%H:%M:%S").to_string();

            let past = is_past(&formatted).expect("parse succeeds");
            let future = is_future(&formatted).expect("parse succeeds");

            assert!(
                !past || !future,
                "is_past and is_future must be mutually exclusive, got \
                 past={past}, future={future}"
            );
        }
    }

    mod diff_precision {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn uses_float_precision_for_datetime_values() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-01-01T00:00:00" | date_diff("2026-01-01T00:00:01.500", unit="seconds") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            let value: f64 = rendered.parse().expect("numeric result");
            assert!(
                (value - 1.5).abs() < 0.01,
                "datetime diff must use float precision, got: {value}"
            );
        }

        #[test]
        fn uses_integer_division_for_date_only_values() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-01-01" | date_diff("2026-01-02", unit="days") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1");
        }
    }

    mod is_leap_year {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::divisible_by_4("2024 is is_leap_year", "True")]
        #[case::not_divisible_by_4("2023 is is_leap_year", "False")]
        #[case::divisible_by_100_not_400("1900 is is_leap_year", "False")]
        #[case::divisible_by_400("2000 is is_leap_year", "True")]
        #[case::leap_date_string("'2024-02-15' is is_leap_year", "True")]
        #[case::non_leap_date_string("'2023-02-15' is is_leap_year", "False")]
        fn checks_a_year_or_date_string(
            #[case] expr: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(&format!("{{{{ {expr} }}}}"), minijinja::context!())
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn rejects_a_non_integer_non_string_argument() {
            let error = env()
                .render_str("{{ [] is is_leap_year }}", minijinja::context!())
                .expect_err("a list argument fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod error_translation {
        use super::*;

        #[test]
        fn preserves_the_local_zone_lookup_diagnosis() {
            let error = render_date_error(DateError::LocalZoneLookup {
                input: "2026-03-08 02:30:00".into(),
            });
            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            assert!(
                error.to_string().contains("local timezone lookup failed"),
                "expected the zone diagnosis, got {error}"
            );

            // The typed DateError survives in the source chain for callers
            // that inspect it, instead of being flattened into the generic
            // overflow message.
            let source = std::error::Error::source(&error)
                .expect("source chain present")
                .downcast_ref::<DateError>()
                .expect("DateError preserved as source");
            assert!(matches!(source, DateError::LocalZoneLookup { .. }));
        }

        #[test]
        fn keeps_the_range_message_for_an_overflow() {
            let error = render_calendar_error(CalendarError::OutOfRange);
            assert!(
                error
                    .to_string()
                    .contains("date arithmetic overflowed the supported range"),
                "unexpected detail: {error}"
            );
        }

        #[test]
        fn renders_parse_failures_with_their_own_message() {
            let error = render_date_error(DateError::YearNotFourDigits {
                input: "26-08-22".into(),
            });
            assert!(
                error.to_string().contains("does not have a 4-digit year"),
                "expected the parse diagnosis, got {error}"
            );
            assert!(
                !error
                    .to_string()
                    .contains("date arithmetic overflowed the supported range"),
                "a parse failure must not render as an overflow: {error}"
            );
        }

        #[test]
        fn renders_a_bad_reference_format_as_a_parse_error() {
            // Arrange: a reference whose text cannot satisfy its
            // `reference_format`, so `parse_with_pattern` fails before any
            // arithmetic runs.
            let rendered = env().render_str(
                r"{{ date.today(reference='not-a-date', reference_format='%Y-%m-%d') }}",
                minijinja::context! {},
            );
            // Act
            let error = rendered.expect_err("a mismatched reference fails");
            // Assert
            let message = error.to_string();
            assert!(
                message.contains("does not have a 4-digit year")
                    || message.contains("not a recognized date/time")
                    || message.contains("not a date or date-time shape"),
                "expected a parse diagnosis, got: {message}"
            );
            assert!(
                !message.contains("date arithmetic overflowed"),
                "a parse failure must not render as an overflow: {message}"
            );
        }
    }
}
