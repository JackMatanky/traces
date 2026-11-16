//! Registers date and time helpers for templates.
//!
//! [`DateOps`] registers the `date` namespace as a global in
//! [`super::TemplateEngine`]. It also registers the flat `date_*` filters and
//! `is_*` tests for template pipelines.
//!
//! Namespace generators return formatted strings from the current instant,
//! date, or Unix timestamp: `date.now()`, `date.today()`, `date.tomorrow()`,
//! `date.yesterday()`, and `date.from_timestamp(ts)`.
//!
//! Filters transform piped date/time strings: `date_format`, `timestamp`,
//! `date_add`, `date_sub`, `add_days`, `sub_days`, `add_months`, `sub_months`,
//! `add_years`, `sub_years`, `weekday`, and `date_diff`. Period-edge filters
//! register from [`PERIOD_EDGE_LOOKUP`](crate::temporal::PERIOD_EDGE_LOOKUP).
//!
//! The `date_format` name avoids a conflict with minijinja's built-in `format`
//! filter. Tests inspect piped values: `is_past`, `is_future`, and
//! `is_leap_year`.
//!
//! [`parse_date_point`] sends each date/time string through
//! [`DatePoint::parse`], which selects parsing behavior from the input shape. A
//! full date-time shape parses as a date-time. A bare ISO date (`YYYY-MM-DD` or
//! reduced-precision `YYYY-MM`) parses as a calendar date at midnight.
//!
//! Arithmetic filters serialize values at the input's original precision with
//! the calendar owner.
//!
//! # Clock rules
//!
//! A naive date-time input represents the reader's local clock time and parses
//! to a UTC instant through the core date module's DST resolver. A date-only
//! input remains a zone-free calendar date.
//!
//! Human-facing filters render local clock time. `timestamp`, `is_past`,
//! `is_future`, and fixed-unit [`date_diff`] use the stored UTC instant.
//! `date.now()`, `date.today()`, `date.tomorrow()`, `date.yesterday()`, and
//! `date.from_timestamp()` use the local clock for display.

use std::sync::Arc;

use minijinja::{
    Environment, Error, ErrorKind,
    value::{Enumerator, Kwargs, Object, Value},
};

use super::error::{TemplateEngineResult, invalid_operation};
use crate::{
    DEFAULT_DATE_PATTERN, DateTimeValue, DurationValue, TimeUnit,
    temporal::{
        CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint,
        DateUnit, DateValue, PeriodEdge, pattern_to_strftime,
        resolve_period_edge, weekday_from_number,
    },
};
/// Lists the method names that [`DateOps::enumerate`] exposes for `date`.
const METHODS: &[&str] = &[
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
/// Provides the `date` namespace object.
#[derive(Debug)]
pub(super) struct DateOps;

impl DateOps {
    /// Registers the `date` global and this module's `date_*` filters and
    /// `is_*` tests.
    ///
    /// Adds the filters and tests as free functions, then adds the `date`
    /// namespace object. This associated function uses no state from
    /// [`DateOps`].
    #[inline]
    pub(super) fn register(env: &mut Environment<'static>) {
        env.add_filter("date_format", date_format);
        env.add_filter("timestamp", timestamp);
        env.add_filter("date_add", date_add);
        env.add_filter("date_sub", date_sub);
        env.add_filter("add_days", add_days);
        env.add_filter("sub_days", sub_days);
        env.add_filter("add_months", add_months);
        env.add_filter("sub_months", sub_months);
        env.add_filter("add_years", add_years);
        env.add_filter("sub_years", sub_years);
        for (name, &(period, edge)) in
            crate::temporal::PERIOD_EDGE_LOOKUP.entries()
        {
            env.add_filter(*name, move |value: &str| {
                render_period_edge(value, period, edge)
            });
        }
        env.add_filter("weekday", weekday);
        env.add_filter("date_diff", date_diff);
        env.add_test("is_past", is_past);
        env.add_test("is_future", is_future);
        env.add_test("is_leap_year", is_leap_year);
        env.add_global("date", Value::from_object(Self));
    }
}

impl Object for DateOps {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let mut buf = [0u8; 16];
        let lower = if key.len() <= buf.len() {
            for (dst, byte) in buf.iter_mut().zip(key.as_bytes()) {
                *dst = byte.to_ascii_lowercase();
            }
            std::str::from_utf8(buf.get(..key.len())?).ok()?
        } else {
            key
        };
        match lower {
            "now" => Some(shorthand(Anchor::Now)),
            "today" => Some(shorthand(Anchor::Today)),
            "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
            "yesterday" => Some(shorthand(Anchor::Yesterday)),
            "weekday" => Some(Value::from_function(
                |n: i64,
                 format: Option<&str>,
                 reference: Option<&str>,
                 reference_format: Option<&str>,
                 kwargs: Kwargs|
                 -> TemplateEngineResult<String> {
                    Anchor::Weekday(n).render(
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
                    let value = DateTimeValue::from_unix_seconds(unix_ts)
                        .map_err(|source| {
                            invalid_operation(
                                format!("timestamp {unix_ts} is out of range"),
                                source,
                            )
                        })?;
                    let zone = crate::temporal::local_zone()
                        .map_err(render_date_error)?;
                    DatePoint::from(value)
                        .format_in(format, &zone)
                        .map_err(render_date_error)
                },
            )),
            edge_name => {
                let (period, edge) = resolve_period_edge(edge_name)?;
                let anchor = match edge {
                    PeriodEdge::Start => Anchor::StartOf(period),
                    PeriodEdge::End => Anchor::EndOf(period),
                };
                Some(shorthand(anchor))
            }
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(METHODS)
    }
}

/// Applies an integer-day or duration-string offset to `date_point`.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `offset_val` is not an integer or
///   duration string, the duration is invalid, the local zone lookup fails, or
///   the shift exceeds the supported range (see [`render_date_error`] and
///   [`render_calendar_error`]).
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

/// Names the starting value for a `date.*` shorthand and the change applied
/// before its offset.
#[derive(Copy, Clone)]
enum Anchor {
    Now,
    Today,
    Tomorrow,
    Yesterday,
    StartOf(CalendarPeriod),
    EndOf(CalendarPeriod),
    /// Selects ISO weekday `n` (`1..=7`, Monday = 1). Each call site checks `n`
    /// with [`weekday_from_number`] before rendering.
    Weekday(i64),
}

impl Anchor {
    /// Returns the starting date point when a shorthand has no reference.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::Zone`] if process-local time-zone discovery fails.
    fn base(self) -> Result<DatePoint, CalendarError> {
        match self {
            Self::Now => Ok(DatePoint::from(DateTimeValue::now())),
            Self::Today
            | Self::Tomorrow
            | Self::Yesterday
            | Self::StartOf(_)
            | Self::EndOf(_)
            | Self::Weekday(_) => Ok(DatePoint::from(DateValue::today()?)),
        }
    }

    /// Changes the date point according to this anchor.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidOperation`] if the local zone lookup fails, the
    ///   `Weekday` target is outside `1..=7`, or a shift, snap, or weekday move
    ///   leaves the representable range (see [`render_date_error`],
    ///   [`weekday_index_error`], and [`render_calendar_error`]).
    fn transform(
        self,
        date_point: DatePoint,
    ) -> TemplateEngineResult<DatePoint> {
        // Resolve the zone only when needed. `date.now()` does not fail on zone
        // lookup.
        let zone = || crate::temporal::local_zone().map_err(render_date_error);
        match self {
            Self::Now | Self::Today => Ok(date_point),
            Self::Tomorrow => date_point
                .shift(1, TimeUnit::Day, &zone()?)
                .map_err(render_calendar_error),
            Self::Yesterday => date_point
                .shift(-1, TimeUnit::Day, &zone()?)
                .map_err(render_calendar_error),
            Self::StartOf(calendar_period) => date_point
                .snap(calendar_period, PeriodEdge::Start, &zone()?)
                .map_err(render_calendar_error),
            Self::EndOf(calendar_period) => date_point
                .snap(calendar_period, PeriodEdge::End, &zone()?)
                .map_err(render_calendar_error),
            Self::Weekday(target) => {
                let target_weekday = weekday_from_number(target)
                    .ok_or_else(|| weekday_index_error(target))?;
                date_point
                    .with_weekday(target_weekday, &zone()?)
                    .map_err(render_calendar_error)
            }
        }
    }

    /// Renders the anchored date point with the optional reference, offset, and
    /// format arguments. These arguments can also arrive as kwargs.
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
            .unwrap_or(DEFAULT_DATE_PATTERN);
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
        let zoned = date_point
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?;
        let local_datetime = zoned.datetime();
        let pat = pattern_to_strftime(format, local_datetime)
            .map_err(|source| invalid_operation(source.to_string(), source))?;
        date_point.format_in(&pat, &zone).map_err(render_date_error)
    }
}

/// Builds a template helper for an anchor, with format, offset, reference,
/// reference format, and kwargs.
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

/// Builds an error for a weekday number outside `1..=7`.
fn weekday_index_error(n: i64) -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        format!(
            "weekday index {n} is out of range (expected 1..=7, Monday = 1 \
             through Sunday = 7)"
        ),
    )
}

/// Parses `s` as a date or date-time with [`DatePoint::parse`].
///
/// Attaches [`DateError`] as the source to preserve the full parse-error chain
/// in render diagnostics.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `s` doesn't parse as a recognized date
///   or datetime.
fn parse_date_point(s: &str) -> TemplateEngineResult<DatePoint> {
    DatePoint::parse(s)
        .map_err(|err| invalid_operation(format!("invalid date {s:?}"), err))
}

/// Gets the shared `format="..."` kwarg for `date.*` namespace methods.
///
/// Uses [`DEFAULT_DATE_PATTERN`] when absent and rejects other kwargs with
/// [`Kwargs::assert_all_used`].
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `format` is present but is not a
///   string.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides
///   `format`.
fn format_kwarg(kwargs: &Kwargs) -> TemplateEngineResult<&str> {
    let format =
        kwargs.get::<Option<&str>>("format")?.unwrap_or(DEFAULT_DATE_PATTERN);
    kwargs.assert_all_used()?;
    Ok(format)
}

/// Gets the shared `unit="..."` kwarg for [`date_add`], [`date_sub`], and
/// [`date_diff`].
///
/// Uses `"days"` when absent and rejects other kwargs with
/// [`Kwargs::assert_all_used`].
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `unit` doesn't match
///   [`TimeUnit::parse_spelling`]'s accepted unit names or abbreviations.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides `unit`.
fn unit_kwarg(kwargs: &Kwargs) -> TemplateEngineResult<TimeUnit> {
    let unit_str = kwargs.get::<Option<&str>>("unit")?.unwrap_or("days");
    kwargs.assert_all_used()?;
    TimeUnit::parse_spelling(unit_str)
        .map_err(|error| invalid_operation(error.to_string(), error))
}

/// Gets the calendar year of `s` in the reader's local zone.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `s` does not parse as a date/time or
///   local-zone lookup fails.
fn parse_local_year(s: &str) -> TemplateEngineResult<i32> {
    let point = parse_date_point(s)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    Ok(i32::from(point.local_datetime_in(&zone).year()))
}

/// Formats a piped date/time string with a strftime pattern or moment-dialect
/// tokens.
///
/// Use `{{ value | date_format(format_string) }}`. The function accepts
/// strftime patterns with `%` directives outside bracket literals and moment
/// tokens (`YYYY MM DD HH mm ss`, `Do`/`S`, `dddd`/`ddd`/`MMM`/`MMMM`). Bracket
/// literals such as `[Daily]` render as literal text in both.
///
/// [`fmt::strtime`] formats a zoned date-time. Therefore, `%Z` shows the
/// resolved local zone on either side of a DST fold. Leap seconds are not
/// representable. Week numbers use ISO `%V`/`%G`, not `%U` or `%W`.
///
/// Explicit `%Y` displays negative years but does not round-trip: strtime
/// counts the sign inside the four-digit width, so year `-9` renders as `-009`.
///
/// The crate rejects that value on reparse. Canonical round-trip forms
/// (`%Y-%m-%d`, `%Y-%m`, `%F`, and canonical date-time patterns) keep
/// the sign outside the year digits and reparse exactly.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse, the local zone
///   lookup fails, or `pattern` has an unclosed bracket literal, invalid
///   strftime syntax, or an unsupported moment token (see
///   [`parse_date_point`]).
fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
    let point = parse_date_point(value)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    let zoned =
        point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
    let datetime = zoned.datetime();
    let translated = pattern_to_strftime(pattern, datetime)
        .map_err(|source| invalid_operation(source.to_string(), source))?;
    point.format_in(&translated, &zone).map_err(render_date_error)
}

/// Converts a piped date/time string to Unix seconds.
///
/// Use `{{ value | timestamp }}`. A naive input uses the reader's local zone
/// and compares correctly with file timestamps. An explicit-offset input
/// converts directly. A date-only input resolves at local-zone midnight.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or cannot
///   convert to Unix seconds (see [`parse_date_point`] and
///   [`render_calendar_error`]).
fn timestamp(value: &str) -> TemplateEngineResult<i64> {
    parse_date_point(value)?.unix_seconds().map_err(render_calendar_error)
}

/// Returns the first or last day of the input's `calendar_period` at its
/// original precision.
///
/// Implements every name in
/// [`PERIOD_EDGE_LOOKUP`](crate::temporal::PERIOD_EDGE_LOOKUP). The
/// `start_of_week`/`sow` filter returns Monday, the first day of the ISO week.
/// The `end_of_week`/`eow` filter returns Sunday, the last day. Month and year
/// filters use the same rule.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse, the boundary is
///   outside the supported range, or the boundary's local time cannot resolve
///   (see [`parse_date_point`] and [`render_calendar_error`]).
fn render_period_edge(
    value: &str,
    calendar_period: CalendarPeriod,
    period_edge: PeriodEdge,
) -> TemplateEngineResult<String> {
    let date_point = parse_date_point(value)?;
    let zone = crate::temporal::local_zone().map_err(render_date_error)?;
    let shifted = date_point
        .snap(calendar_period, period_edge, &zone)
        .map_err(render_calendar_error)?;
    let pat = shifted.canonical_pattern();
    shifted.format_in(pat, &zone).map_err(render_date_error)
}

/// Adds `n` units to a piped date/time string.
///
/// Use `{{ value | date_add(n, unit="days") }}`. The `unit` defaults to
/// `"days"` and accepts any [`TimeUnit`] spelling: `"years"`/`"y"`,
/// `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`, `"hours"`/`"h"`,
/// `"minutes"`/`"m"`, or `"seconds"`/`"s"`.
///
/// A shift shorter than one day moves a date-only value only when it crosses
/// midnight. A `25h` shift moves the date by one day. A `5h` shift keeps the
/// date unchanged. The `"years"`, `"months"`, `"weeks"`, and `"days"` units
/// preserve local clock time across a DST transition. Other units shift the
/// exact instant, so the clock time can differ from a naive `n`-unit shift (see
/// [`shift_by_unit`]).
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse, `unit` is not
///   an accepted unit name, or the shift overflows the representable range.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides `unit`.
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

/// Subtracts `n` units from a piped date/time string.
///
/// Use `{{ value | date_sub(n, unit="days") }}`. The `unit` defaults to
/// `"days"` and accepts any [`TimeUnit`] spelling: `"years"`/`"y"`,
/// `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`, `"hours"`/`"h"`,
/// `"minutes"`/`"m"`, or `"seconds"`/`"s"`.
///
/// A shift shorter than one day moves a date-only value only when it crosses
/// midnight. A `25h` shift moves the date by one day. A `5h` shift keeps the
/// date unchanged.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse, `unit` is not
///   an accepted unit name, `n` is [`i64::MIN`], or the shift overflows the
///   representable range.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides `unit`.
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

/// Shifts `value` by `n` `unit`s with calendar or fixed-duration behavior.
///
/// A [`DatePoint::DateTime`] shifts calendar units on local clock time, which
/// preserves the hour across DST. It shifts sub-day units on the exact
/// date-time. A [`DatePoint::Calendar`] shifts at local midnight. A shift
/// shorter than one day moves the date only when it crosses midnight. A
/// `25h` shift moves the date by one day. A `5h` shift keeps the date
/// unchanged.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or the shift
///   exceeds the representable range (see [`render_calendar_error`]).
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
    shifted_date_point.format_in(pat, &zone).map_err(render_date_error)
}

/// Adds `n` days to a piped date/time string.
///
/// Uses `{{ value | date_add(n, unit="days") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn add_days(value: &str, n: u64) -> TemplateEngineResult<String> {
    let n_i64 = i64::try_from(n).map_err(|_| date_out_of_range_error())?;
    shift_by_unit(value, n_i64, TimeUnit::Day)
}

/// Subtracts `n` days from a piped date/time string.
///
/// Uses `{{ value | date_sub(n, unit="days") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn sub_days(value: &str, n: u64) -> TemplateEngineResult<String> {
    let n_i64 = i64::try_from(n).map_err(|_| date_out_of_range_error())?;
    let n_i64 = n_i64.checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Day)
}

/// Adds `n` months to a piped date/time string.
///
/// Uses `{{ value | date_add(n, unit="months") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn add_months(value: &str, n: u32) -> TemplateEngineResult<String> {
    shift_by_unit(value, i64::from(n), TimeUnit::Month)
}

/// Subtracts `n` months from a piped date/time string.
///
/// Uses `{{ value | date_sub(n, unit="months") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn sub_months(value: &str, n: u32) -> TemplateEngineResult<String> {
    let n_i64 =
        i64::from(n).checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Month)
}

/// Adds `n` years to a piped date/time string.
///
/// Uses `{{ value | date_add(n, unit="years") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn add_years(value: &str, n: u32) -> TemplateEngineResult<String> {
    shift_by_unit(value, i64::from(n), TimeUnit::Year)
}

/// Subtracts `n` years from a piped date/time string.
///
/// Uses `{{ value | date_sub(n, unit="years") }}` as a shortcut.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or arithmetic
///   exceeds the representable range.
fn sub_years(value: &str, n: u32) -> TemplateEngineResult<String> {
    let n_i64 =
        i64::from(n).checked_neg().ok_or_else(date_out_of_range_error)?;
    shift_by_unit(value, n_i64, TimeUnit::Year)
}

/// Returns the ISO weekday number or the formatted date for a weekday in the
/// same week.
///
/// Use `{{ value | weekday }}` to return `1` for Monday through `7` for Sunday.
/// Supply `n` to return the formatted date for that weekday in `value`'s week.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse, `n` is outside
///   `1..=7`, or `format` is invalid.
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides
///   `format`.
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
        let target_weekday = weekday_from_number(target_num)
            .ok_or_else(|| weekday_index_error(target_num))?;
        let shifted = date_point
            .with_weekday(target_weekday, &zone)
            .map_err(render_calendar_error)?;
        let zoned =
            shifted.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
        let local_datetime = zoned.datetime();
        let pat = pattern_to_strftime(format, local_datetime)
            .map_err(|source| invalid_operation(source.to_string(), source))?;
        Ok(Value::from(
            shifted.format_in(&pat, &zone).map_err(render_date_error)?,
        ))
    } else {
        kwargs.assert_all_used()?;
        Ok(Value::from(date_point.component(DateUnit::Weekday, &zone)))
    }
}

/// Returns the signed difference from the piped value to `other`.
///
/// Use `{{ value | date_diff(other, unit="days") }}`. The result is positive
/// when `other` is later. The `unit` kwarg defaults to `"days"` and accepts
/// `"years"`, `"months"`, `"weeks"`, `"hours"`, `"minutes"`, or `"seconds"`.
///
/// The `"years"` and `"months"` units count elapsed whole calendar units and
/// account for the day of the month (see [`DatePoint::diff`]). They always
/// return `i64`, regardless of input precision. Other units, including
/// `"weeks"` and the default `"days"`, use fixed durations. They return
/// `f64` when both inputs include a time component. They return a whole-unit
/// `i64` otherwise.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if either value does not parse or `unit`
///   does not match [`TimeUnit::parse_spelling`]'s accepted names (see
///   [`parse_date_point`] and [`unit_kwarg`]).
/// - [`ErrorKind::TooManyArguments`] if `kwargs` carries a key besides `unit`.
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

/// Checks whether a piped date/time string is before now.
///
/// Use `{% if value is is_past %}`. A naive input uses the reader's local zone
/// and compares correctly with file timestamps.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or cannot
///   convert to an instant (see [`parse_date_point`] and
///   [`render_calendar_error`]).
fn is_past(value: &str) -> TemplateEngineResult<bool> {
    let instant =
        parse_date_point(value)?.instant().map_err(render_calendar_error)?;
    // Compare instants directly. No zone or date point is needed.
    Ok(instant < crate::temporal::now_instant())
}

/// Checks whether a piped date/time string is after now.
///
/// Use `{% if value is is_future %}`. This checks future instants, like
/// [`is_past`] checks past instants.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` does not parse or cannot
///   convert to an instant (see [`parse_date_point`] and
///   [`render_calendar_error`]).
fn is_future(value: &str) -> TemplateEngineResult<bool> {
    let instant =
        parse_date_point(value)?.instant().map_err(render_calendar_error)?;
    // Compare instants directly. No zone or date point is needed.
    Ok(instant > crate::temporal::now_instant())
}

/// Checks whether a year or date/time string falls in a leap year.
///
/// Use `{% if value is is_leap_year %}`. The input can be an integer year
/// (`2024 is is_leap_year`) or a date/time string that [`parse_local_year`]
/// checks.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is not a representable [`i32`]
///   year or parseable date/time, or the year is outside the civil-date range
///   (see [`parse_local_year`] and [`leap_year_input_error`]).
fn is_leap_year(value: &Value) -> TemplateEngineResult<bool> {
    let year = if let Some(year) = value.as_i64() {
        i32::try_from(year).map_err(|_| leap_year_input_error(value))?
    } else if let Some(s) = value.as_str() {
        parse_local_year(s)?
    } else {
        return Err(leap_year_input_error(value));
    };
    DateValue::in_leap_year(year).map_err(|_| leap_year_input_error(value))
}

/// Builds the `is_leap_year` error when the input is not a representable year
/// or valid date string.
fn leap_year_input_error(value: &Value) -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        format!(
            "is_leap_year expects an integer year or a date string, got \
             {value:?}"
        ),
    )
}

/// Builds an error for integer conversion overflow before calendar arithmetic.
///
/// `checked_neg` fails for `i64::MIN`, and conversion from `u64` to `i64` can
/// fail. Calendar arithmetic failures use [`render_calendar_error`] and
/// temporal's display text.
fn date_out_of_range_error() -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        "date arithmetic overflowed the supported range",
    )
}

/// Converts a core parse error to a render error.
fn render_date_error(error: DateError) -> Error {
    invalid_operation(error.to_string(), error)
}

/// Converts a computed calendar failure to a render error.
fn render_calendar_error(error: CalendarError) -> Error {
    invalid_operation(error.to_string(), error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::test_support::ZoneGuard;

    fn env() -> Environment<'static> {
        // Date filters read the thread-local zone, so concurrent tests need no
        // serialization.
        let mut env = Environment::new();
        DateOps::register(&mut env);
        env
    }

    /// Renders `{{ value | <filter_call> }}` in the registered environment and
    /// checks the exact output.
    ///
    /// The per-filter `shifts_a_piped_date` tables use this helper.
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
            let ops = Arc::new(DateOps);

            assert!(ops.get_value(&Value::from("later")).is_none());
        }

        #[test]
        fn get_value_returns_none_for_a_non_string_key() {
            let ops = Arc::new(DateOps);

            assert!(ops.get_value(&Value::from(1)).is_none());
        }

        #[test]
        fn get_value_accepts_uppercase_method_names() {
            let ops = Arc::new(DateOps);

            for method in METHODS {
                let upper = method.to_ascii_uppercase();
                assert!(
                    ops.get_value(&Value::from(upper.as_str())).is_some(),
                    "{upper:?} (uppercase of {method:?}) does not resolve"
                );
            }
        }

        #[test]
        fn get_value_resolves_mixed_case_period_edges() {
            let ops = Arc::new(DateOps);

            assert!(ops.get_value(&Value::from("Start_Of_Week")).is_some());
            assert!(ops.get_value(&Value::from("End_Of_Month")).is_some());
        }

        #[test]
        fn get_value_returns_none_for_a_long_unknown_key() {
            let ops = Arc::new(DateOps);
            let long_key = "this_key_is_deliberately_longer_than_16_bytes";

            assert!(ops.get_value(&Value::from(long_key)).is_none());
        }
    }

    mod now {
        use pretty_assertions::assert_eq;

        use super::*;

        /// Checks the rendered shape, not the current date.
        ///
        /// The clock makes the value nondeterministic. The default format
        /// always has the shape `YYYY-MM-DD`.
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

        /// Checks that invalid format specifiers such as `%J` return a render
        /// error instead of panicking.
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

        #[test]
        fn now_formats_a_negative_year_reference_sign_outside_the_width() {
            // Set up the test.
            ZoneGuard::set("UTC");

            // Run the operation.
            let rendered = env()
                .render_str(
                    r#"{{ date.now(reference="-0009-01-01", reference_format="%Y-%m-%d", format="%Y-%m-%d") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            // Check the result.
            assert_eq!(rendered, "-0009-01-01");
        }
    }

    /// Checks the relative dates from `today`, `tomorrow`, and `yesterday` in
    /// one render window.
    ///
    /// One template evaluation reads the clock once for all three values.
    ///
    /// Separate renders can cross local midnight and make the one-day
    /// assertions fail.
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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

            // Midnight UTC is 02:00 local on the same calendar day.
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
            let ops = Arc::new(DateOps);

            assert!(matches!(ops.enumerate(), Enumerator::Str(METHODS)));
        }

        #[test]
        fn every_enumerated_method_resolves_via_get_value() {
            let ops = Arc::new(DateOps);

            for method in METHODS {
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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

            // The reader's clock shows 14:30Z as 16:30 local, not the old UTC
            // value of 12:30.
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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

            // A naive date-time uses the reader's clock time, so every zone
            // displays the written time.
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

        /// Checks that [`DateError`] remains in the source chain of a template
        /// render error.
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

        /// Checks that invalid date-time input returns its date-time parse
        /// error, not an unrelated date-only shape error (spec N4).
        #[test]
        fn surfaces_the_datetime_parsers_error_not_the_date_only_fallbacks() {
            use std::error::Error as _;

            let input = "2026-07-29T99:99:99"; // Use a date-time shape with an invalid time.
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
            let message = error.to_string();
            assert!(
                message.contains("invalid date format \"%J\":"),
                "the message names the pattern and Jiff's reason: {message}"
            );
            let source = std::error::Error::source(&error)
                .expect("a temporal source rides along");
            assert!(
                matches!(
                    source.downcast_ref::<DateError>(),
                    Some(DateError::InvalidFormat { pattern, .. })
                        if pattern.as_ref() == "%J"
                ),
                "the source is DateError::InvalidFormat, got {source:?}"
            );
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

        #[rstest]
        #[case::year_full("YYYY", "2026")]
        #[case::year_short("YY", "26")]
        #[case::month_full("MMMM", "July")]
        #[case::month_abbr("MMM", "Jul")]
        #[case::month_padded("MM", "07")]
        #[case::month_short("M", "7")]
        #[case::weekday_full("dddd", "Sunday")]
        #[case::weekday_abbr("ddd", "Sun")]
        #[case::day_padded("DD", "05")]
        #[case::day_short("D", "5")]
        #[case::hour24_padded("HH", "04")]
        #[case::hour24_short("H", "4")]
        #[case::hour12_padded("hh", "04")]
        #[case::hour12_short("h", "4")]
        #[case::minute_padded("mm", "05")]
        #[case::minute_short("m", "5")]
        #[case::second_padded("ss", "03")]
        #[case::second_short("s", "3")]
        #[case::fraction_3("SSS", "123")]
        #[case::fraction_2("SS", "12")]
        #[case::fraction_1("S", "1")]
        #[case::ordinal_day("Do", "5th")]
        #[case::bracket_literal("[Daily]", "Daily")]
        fn covers_the_full_moment_token_table(
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-07-05T04:05:03.123" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[rstest]
        #[case::triple_d("DDD", "DDD")]
        #[case::quadruple_d("DDDD", "DDDD")]
        #[case::five_m("MMMMM", "MMMMM")]
        #[case::triple_upper_h("HHH", "HHH")]
        #[case::triple_lower_h("hhh", "hhh")]
        #[case::triple_lower_m("mmm", "mmm")]
        #[case::triple_lower_s("sss", "sss")]
        #[case::four_upper_s("SSSS", "SSSS")]
        #[case::six_y("YYYYYY", "YYYYYY")]
        #[case::eight_y("YYYYYYYY", "YYYYYYYY")]
        #[case::triple_y("YYY", "YYY")]
        #[case::five_d("ddddd", "ddddd")]
        #[case::two_lower_d("dd", "dd")]
        #[case::one_lower_d("d", "d")]
        #[case::three_lower_y("yyy", "yyy")]
        #[case::lowercase_do_reports_d_run("do", "d")]
        fn rejects_runs_that_are_not_moment_tokens(
            #[case] format: &str,
            #[case] token: &str,
        ) {
            ZoneGuard::set("UTC");

            let error = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-07-05" },
                )
                .expect_err("a non-token run fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            let msg = error.to_string();
            assert!(
                msg.contains(&format!(
                    "unsupported moment-dialect token `{token}`"
                )),
                "expected token `{token}` in: {msg}"
            );
        }

        #[rstest]
        #[case::first("2026-07-01", "1st")]
        #[case::second("2026-07-02", "2nd")]
        #[case::third("2026-07-03", "3rd")]
        #[case::eleventh("2026-07-11", "11th")]
        #[case::twelfth("2026-07-12", "12th")]
        #[case::thirteenth("2026-07-13", "13th")]
        #[case::twenty_first("2026-07-21", "21st")]
        fn covers_ordinal_suffix_edges(
            #[case] date: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("Do") }}"#,
                    minijinja::context! { value => date },
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
        #[rstest]
        #[case::bracket_percent_year("[%Y]", "%Y")]
        #[case::bracket_percent_day("[%d] YYYY", "%d 2026")]
        #[case::bracket_apostrophe("[Today's %Y]", "Today's %Y")]
        #[case::mixed_percent_bracket_and_directive(
            "[100%] and %H:%M",
            "100% and 14:30"
        )]
        #[case::stray_close_bracket("1]2", "1]2")]
        #[case::multiple_brackets_with_percents(
            "%H [a] %b [c%]",
            "14 a Jun c%"
        )]
        #[case::nested_looking_bracket("[a[b]", "a[b")]
        #[case::plain_strftime("%H:%M", "14:30")]
        fn renders_bracket_literals_as_text_in_both_dialects(
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-06-05T14:30:00" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn rejects_unclosed_bracket_literal_in_any_dialect() {
            let error = env()
                .render_str(
                    r#"{{ "2026-06-05T14:30:00" | date_format("[Daily %H") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unclosed bracket must fail");
            assert!(
                error.to_string().contains("unclosed bracket literal"),
                "expected unclosed bracket error, got: {error}"
            );
        }

        #[test]
        fn unclosed_bracket_takes_precedence_over_unsupported_token() {
            let error = env()
                .render_str(
                    r#"{{ "2026-06-05" | date_format("Q [x") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unclosed bracket must fail first");
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

        #[test]
        fn renders_a_negative_year_with_the_sign_outside_the_width() {
            // Use UTC for this test.
            ZoneGuard::set("UTC");

            // Render the template.
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("%Y-%m-%d") }}"#,
                    minijinja::context! { value => "-0009-01-01" },
                )
                .expect("render succeeds");

            // Check the result.
            assert_eq!(rendered, "-0009-01-01");
        }

        #[test]
        fn skipped_local_days_render_one_day_in_both_dialects() {
            // Samoa skipped 2011-12-30. Midnight shifts to 2011-12-31, which
            // both dialects must render.
            ZoneGuard::set("Pacific/Apia");

            // Run the operation.
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("Do") }}|{{ value | date_format("%d") }}"#,
                    minijinja::context! { value => "2011-12-30" },
                )
                .expect("render succeeds");

            // Check the result.
            assert_eq!(rendered, "31st|31");
        }

        #[test]
        fn zone_resolution_errors_precede_pattern_errors() {
            // Local midnight is before Jiff's minimum instant, and the pattern
            // is invalid.
            ZoneGuard::set("Etc/GMT-14");

            // Run the operation.
            let error = env()
                .render_str(
                    r#"{{ value | date_format("Q") }}"#,
                    minijinja::context! { value => "-9999-01-01" },
                )
                .expect_err("local midnight falls outside the range");

            // Check the result.
            let msg = error.to_string();
            assert!(
                !msg.contains("moment-dialect"),
                "the pattern error must lose to the range error: {msg}"
            );
            assert!(msg.contains("range"), "expected a range error: {msg}");
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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

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
            ZoneGuard::set_timezone(
                crate::temporal::test_support::fixtures::new_york_zone(),
            );

            // 02:30 does not exist on 2026-03-08. The `compatible` policy moves
            // it across the gap to 03:30 EDT (07:30Z).
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

    mod period_edge_errors {
        use super::*;

        #[test]
        fn start_of_year_reports_a_range_failure_not_a_parse_failure() {
            // In UTC+14, snapped year-start midnight is before Jiff's earliest
            // instant. `format_in` resolves it first.
            ZoneGuard::set("Etc/GMT-14");

            // Run the operation.
            let error = env()
                .render_str(
                    "{{ value | start_of_year }}",
                    minijinja::context! { value => "-9999-02-15" },
                )
                .expect_err("year-start midnight falls outside the range");

            // Check the result.
            let message = error.to_string();
            assert!(
                message.contains("is out of the supported date/time range"),
                "expected range-truthful wording, got {message}"
            );
            assert!(
                !message.contains("not a recognized"),
                "a computed value must not read as a parse failure: {message}"
            );
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
        fn seconds_are_a_whole_count_for_date_only_inputs() {
            let rendered = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_diff("2026-07-24", unit="s") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "86400");
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
            ZoneGuard::set("Etc/GMT-2"); // Set UTC+02:00 without DST.

            // The mixed-precision branch subtracts calendar clocks in the
            // pinned zone, not the parser's naive representations.
            // Naive 14:30 local and equivalent 12:30Z inputs both
            // resolve to 14:30 local and produce the same truncated 14-hour
            // difference. The mixed branch returns i64, not f64.
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
            assert!(
                error
                    .to_string()
                    .contains("unknown unit \"fortnight\" (expected "),
                "the unknown-unit message must keep its shape, got: {error}"
            );
        }

        /// Checks that `1e16` seconds returns an error: it fits `i64` and
        /// passes `checked_mul`, but exceeds the Jiff civil-date range.
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

        /// Checks the difference between calendar and exact units. A day unit
        /// shifts the calendar clock across the 2026-03-08 gap. A
        /// 24-hour shift moves the instant and lands one clock
        /// hour later.
        #[test]
        fn a_day_unit_preserves_the_clock_time_across_a_dst_gap_while_hours_do_not()
         {
            ZoneGuard::set_timezone(
                crate::temporal::test_support::fixtures::new_york_zone(),
            );

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

        #[test]
        fn rejects_a_year_outside_the_representable_range() {
            let error = env()
                .render_str(
                    "{{ 40000 is is_leap_year }}",
                    minijinja::context!(),
                )
                .expect_err("an out-of-range year fails cleanly");

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
            let message = error.to_string();
            assert_eq!(
                message.matches("local zone lookup failed").count(),
                1,
                "one zone diagnosis without a doubled prefix, got {message}"
            );
            assert!(
                !message.contains("local timezone lookup failed"),
                "temporal's Display replaces the engine rewording, got \
                 {message}"
            );

            // Keep the typed DateError in the source chain instead of replacing
            // it with the generic overflow message.
            let source = std::error::Error::source(&error)
                .expect("source chain present")
                .downcast_ref::<DateError>()
                .expect("DateError preserved as source");
            assert!(matches!(source, DateError::LocalZoneLookup { .. }));
        }

        #[test]
        fn renders_a_zone_failure_with_one_lookup_prefix() {
            // Match today's constructor, which stores DateError's display text.
            // This assertion gives the required diagnosis.
            let inner = DateError::LocalZoneLookup {
                input: "system zone: clock unavailable".into(),
            };
            let error = render_calendar_error(CalendarError::Zone {
                source: inner,
            });

            // Run the operation.
            let message = error.to_string();

            // Check the result.
            assert_eq!(
                message,
                "invalid operation: local zone lookup failed for `system \
                 zone: clock unavailable`"
            );
        }

        #[test]
        fn reports_range_failures_with_temporal_display_and_a_source() {
            let error = render_calendar_error(CalendarError::OutOfRange);
            assert_eq!(
                error.to_string(),
                "invalid operation: date/time value is out of range"
            );

            // CalendarError remains in the source chain, so miette prints
            // `Caused by:` for range failures.
            assert!(
                std::error::Error::source(&error).is_some_and(|source| {
                    source.downcast_ref::<CalendarError>().is_some()
                }),
                "CalendarError preserved as source"
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
            // This reference does not match `reference_format`, so
            // `parse_with_pattern` fails before arithmetic.
            let rendered = env().render_str(
                r"{{ date.today(reference='not-a-date', reference_format='%Y-%m-%d') }}",
                minijinja::context! {},
            );
            // Run the operation.
            let error = rendered.expect_err("a mismatched reference fails");
            // Check the result.
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
