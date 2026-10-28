//! Defines validated durations with nominal identity and a calendar application
//! regime.
//!
//! Owns [`DurationValue`], the nominal identity measure, and the written
//! components that witness the regime. Every duration-shaped string in the
//! crate is recognized here; callers hold a validated [`DurationValue`] or
//! receive a [`DurationError`].
//!
//! # Identity and application
//!
//! Nominal seconds are the identity of a Duration: one month counts as 30 days
//! and one year as 365 days, so equality, ordering, and hashing compare rounded
//! nominal seconds and never the spelling.
//!
//! # Key Types
//!
//! - [`DurationValue`]: A validated duration with nominal identity and
//!   calendar-aware application.

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    hash::Hash,
    ops::{Add, Mul, Neg, Sub},
    str::FromStr,
};

use jiff::{
    Span, SpanRound, Unit,
    fmt::friendly::{Designator, Direction, SpanPrinter},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    error::DurationError,
    unit::{TimeUnit, decompose},
};

/// Largest-first order for rendering a compound English magnitude.
///
/// Milliseconds are rounded and rendered from the remaining magnitude.
pub(super) const COMPOUND_FORMAT_ORDER: [TimeUnit; 7] = [
    TimeUnit::Year,
    TimeUnit::Month,
    TimeUnit::Week,
    TimeUnit::Day,
    TimeUnit::Hour,
    TimeUnit::Minute,
    TimeUnit::Second,
];

const MAX_NOMINAL_SECONDS: f64 = 631_107_417_600.0;

const PRINTER: SpanPrinter = SpanPrinter::new()
    .designator(Designator::Compact)
    .direction(Direction::Sign);

/// Rounds a span's fractional seconds exactly once for nominal identity and
/// fieldwise arithmetic. Calendar fields stay untouched.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "Jiff bounds each subsecond field below one second"
)]
fn rounded_seconds(span: Span) -> i64 {
    let nanos = i128::from(span.get_milliseconds()) * 1_000_000
        + i128::from(span.get_microseconds()) * 1_000
        + i128::from(span.get_nanoseconds());
    let carry = if nanos >= 500_000_000 {
        1
    } else if nanos <= -500_000_000 {
        -1
    } else {
        0
    };
    span.get_seconds().saturating_add(carry)
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "Jiff's bounded span units safely sum into i64 nominal seconds"
)]
fn nominal_seconds(span: Span) -> i64 {
    i64::from(span.get_years()) * TimeUnit::Year.nominal_seconds()
        + i64::from(span.get_months()) * TimeUnit::Month.nominal_seconds()
        + i64::from(span.get_weeks()) * TimeUnit::Week.nominal_seconds()
        + i64::from(span.get_days()) * TimeUnit::Day.nominal_seconds()
        + i64::from(span.get_hours()) * TimeUnit::Hour.nominal_seconds()
        + span.get_minutes() * TimeUnit::Minute.nominal_seconds()
        + rounded_seconds(span)
}

/// Adds each written unit independently, preserving the calendar regime.
///
/// Jiff's `checked_add` balances calendar units relative to a date instead of
/// adding fields. `None` denotes mixed signs, which must be decomposed by
/// nominal identity.
fn add_fields(lhs: Span, rhs: Span) -> Result<Option<Span>, DurationError> {
    macro_rules! sum {
        ($getter:ident) => {
            i64::from(lhs.$getter())
                .checked_add(i64::from(rhs.$getter()))
                .ok_or(DurationError::SecondsOutOfRange)?
        };
    }
    let years = sum!(get_years);
    let months = sum!(get_months);
    let weeks = sum!(get_weeks);
    let days = sum!(get_days);
    let hours = sum!(get_hours);
    let minutes = sum!(get_minutes);
    let seconds = rounded_seconds(lhs)
        .checked_add(rounded_seconds(rhs))
        .ok_or(DurationError::SecondsOutOfRange)?;
    let fields = [years, months, weeks, days, hours, minutes, seconds];
    if fields.iter().any(|&value| value > 0)
        && fields.iter().any(|&value| value < 0)
    {
        return Ok(None);
    }
    Span::new()
        .try_years(years)
        .and_then(|span| span.try_months(months))
        .and_then(|span| span.try_weeks(weeks))
        .and_then(|span| span.try_days(days))
        .and_then(|span| span.try_hours(hours))
        .and_then(|span| span.try_minutes(minutes))
        .and_then(|span| span.try_seconds(seconds))
        .map(Some)
        .map_err(|_| DurationError::SecondsOutOfRange)
}

/// Represents a validated duration with a nominal identity and a calendar
/// application regime.
///
/// Stores a Jiff [`Span`] and its rounded nominal seconds. Checked arithmetic
/// rejects values outside Jiff's span range.
///
/// # Examples
///
/// ```
/// use traces::{DurationError, DurationValue};
///
/// let written: DurationValue = "1h 30m".parse()?;
/// assert_eq!(written.to_string(), "1h 30m");
///
/// // Equality compares rounded nominal seconds, not spelling.
/// assert_eq!(written, "90m".parse::<DurationValue>()?);
///
/// let error = "not a duration"
///     .parse::<DurationValue>()
///     .expect_err("input does not parse");
/// assert!(matches!(error, DurationError::MissingNumber));
/// assert_eq!(error.to_string(), "no number before unit");
/// # Ok::<(), DurationError>(())
/// ```
#[derive(Copy, Clone)]
pub struct DurationValue {
    span: Span,
    nominal_seconds: i64,
}

impl DurationValue {
    #[inline]
    fn new(span: Span) -> Self {
        Self {
            span,
            nominal_seconds: nominal_seconds(span),
        }
    }

    /// Synthesizes a canonical [`DurationValue`] from a nominal seconds value.
    ///
    /// # Errors
    ///
    /// - [`DurationError::NonFiniteSeconds`] if `seconds` is not finite.
    /// - [`DurationError::SecondsOutOfRange`] if `seconds` exceeds Jiff's
    ///   representable span range.
    #[inline]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "finite nominal seconds bounded by Jiff span limits"
    )]
    pub(crate) fn from_nominal_seconds(
        seconds: f64,
    ) -> Result<Self, DurationError> {
        if !seconds.is_finite() {
            return Err(DurationError::NonFiniteSeconds);
        }
        if seconds.abs() > MAX_NOMINAL_SECONDS {
            return Err(DurationError::SecondsOutOfRange);
        }
        let total_secs = seconds.round() as i64;
        if total_secs == 0 {
            return Ok(Self::new(Span::new()));
        }

        let span = Span::new()
            .try_seconds(total_secs)
            .and_then(|span| {
                span.round(
                    SpanRound::new().largest(Unit::Week).days_are_24_hours(),
                )
            })
            .map_err(|_| DurationError::SecondsOutOfRange)?;
        Ok(Self::new(span))
    }

    /// Parses a human-readable or ISO 8601 duration string.
    ///
    /// # Errors
    ///
    /// - [`DurationError::Empty`] if `input` is empty or contains only
    ///   whitespace.
    /// - [`DurationError::MissingNumber`] if a unit appears without a preceding
    ///   number.
    /// - [`DurationError::MisplacedSign`] if a sign appears in an invalid
    ///   position.
    /// - [`DurationError::MissingUnit`] if a number appears without a following
    ///   unit.
    /// - [`DurationError::UnknownUnit`] if a component names an unrecognized
    ///   unit.
    /// - [`DurationError::NumberOverflow`] if a component magnitude overflows
    ///   to infinity.
    /// - [`DurationError::NonFiniteSeconds`] if the combined magnitude is not
    ///   finite.
    /// - [`DurationError::SecondsOutOfRange`] if the duration exceeds Jiff's
    ///   representable span range.
    pub(crate) fn parse(input: &str) -> Result<Self, DurationError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(DurationError::Empty);
        }

        let unsigned = trimmed
            .strip_prefix('-')
            .or_else(|| trimmed.strip_prefix('+'))
            .unwrap_or(trimmed);

        if unsigned.starts_with('P') || unsigned.starts_with('p') {
            let span: Span =
                trimmed.parse().map_err(|_| DurationError::UnknownUnit)?;
            let val = Self::new(span);
            if val.magnitude() > MAX_NOMINAL_SECONDS {
                return Err(DurationError::SecondsOutOfRange);
            }
            return Ok(val);
        }

        validate_friendly_input(trimmed)?;

        let normalized = normalize_friendly_input(trimmed);
        let span: Span = normalized
            .parse()
            .map_err(|err| classify_jiff_error(&normalized, err))?;

        let val = Self::new(span);
        if val.magnitude() > MAX_NOMINAL_SECONDS {
            return Err(DurationError::SecondsOutOfRange);
        }

        Ok(val)
    }

    /// Parses a duration prefix from `input`, returning its spanned extent.
    pub(crate) fn parse_prefix(input: &str) -> Option<crate::Spanned<Self>> {
        crate::lexer::lex_prefix(input, super::lexer::parse_duration_prefix)
    }

    /// Adds two durations using field-by-field arithmetic.
    ///
    /// # Errors
    ///
    /// - [`DurationError::SecondsOutOfRange`] if the sum exceeds Jiff's
    ///   representable span range.
    #[inline]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "nominal seconds fits within f64"
    )]
    pub fn checked_add(self, rhs: Self) -> Result<Self, DurationError> {
        match add_fields(self.span, rhs.span) {
            Ok(Some(span)) => Ok(Self::new(span)),
            // Jiff spans have one sign for all fields. The original mixed-sign
            // rule decomposes the nominal sum instead of changing calendar
            // math.
            Ok(None) => {
                let total_secs = (self.nominal_seconds() as f64)
                    + (rhs.nominal_seconds() as f64);
                Self::from_nominal_seconds(total_secs)
            }
            Err(err) => Err(err),
        }
    }

    /// Subtracts another duration using field-by-field arithmetic.
    ///
    /// # Errors
    ///
    /// - [`DurationError::SecondsOutOfRange`] if the difference exceeds Jiff's
    ///   representable span range.
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "Neg uses Jiff's infallible Span::negate on validated spans"
    )]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, DurationError> {
        self.checked_add(-rhs)
    }

    /// Multiplies a duration by a finite scale factor.
    ///
    /// # Errors
    ///
    /// - [`DurationError::NonFiniteSeconds`] if `factor` is not finite.
    /// - [`DurationError::SecondsOutOfRange`] if the scaled duration exceeds
    ///   Jiff's representable span range.
    #[inline]
    pub fn checked_mul(self, factor: f64) -> Result<Self, DurationError> {
        if !factor.is_finite() {
            return Err(DurationError::NonFiniteSeconds);
        }
        if factor == 0.0 {
            return Ok(Self::new(Span::new()));
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            reason = "integral factor is checked before truncation"
        )]
        if factor.fract() == 0.0 {
            let f = factor as i64;
            // Arithmetic historically rounds any parsed fractional second
            // before multiplying written fields.
            let rounded = if self.span.get_milliseconds() == 0
                && self.span.get_microseconds() == 0
                && self.span.get_nanoseconds() == 0
            {
                self.span
            } else {
                self.span
                    .try_milliseconds(0)
                    .and_then(|s| s.try_microseconds(0))
                    .and_then(|s| s.try_nanoseconds(0))
                    .and_then(|s| s.try_seconds(rounded_seconds(self.span)))
                    .map_err(|_| DurationError::SecondsOutOfRange)?
            };
            let span = rounded
                .checked_mul(f)
                .map_err(|_| DurationError::SecondsOutOfRange)?;
            Ok(Self::new(span))
        } else {
            let total = (self.nominal_seconds() as f64) * factor;
            Self::from_nominal_seconds(total)
        }
    }

    /// Negates a duration.
    ///
    /// # Errors
    ///
    /// - [`DurationError::SecondsOutOfRange`] if the negated duration exceeds
    ///   Jiff's representable span range.
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "Neg uses Jiff's infallible Span::negate on validated spans"
    )]
    pub fn checked_neg(self) -> Result<Self, DurationError> {
        Ok(-self)
    }

    /// Renders this duration as a comma-separated English magnitude.
    #[must_use]
    pub(crate) fn to_compound_string(self) -> String {
        use std::fmt::Write as _;

        let total_secs = self.nominal_seconds();
        if total_secs == 0 {
            return "0 seconds".to_owned();
        }
        let is_negative = total_secs < 0;
        let (parts, _) =
            decompose(total_secs.unsigned_abs(), &COMPOUND_FORMAT_ORDER);
        let mut text = String::new();
        if is_negative {
            text.push('-');
        }
        for part in parts {
            let (singular, plural) = part.unit().human_names();
            let name = if part.count() == 1 {
                singular
            } else {
                plural
            };
            if !text.is_empty() && text != "-" {
                text.push_str(", ");
            }
            let _ = write!(text, "{} {name}", part.count());
        }
        if text.is_empty() || text == "-" {
            "0 seconds".to_owned()
        } else {
            text
        }
    }

    /// Returns the exact nominal duration in integer seconds.
    #[inline]
    #[must_use]
    pub(crate) fn nominal_seconds(&self) -> i64 {
        self.nominal_seconds
    }

    /// Returns the nominal duration in seconds as an `f64` value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "nominal seconds fits within f64"
    )]
    pub(crate) fn magnitude(&self) -> f64 {
        self.nominal_seconds() as f64
    }

    /// Returns the underlying Jiff [`Span`].
    #[inline]
    #[must_use]
    pub(crate) const fn span(self) -> Span {
        self.span
    }

    /// Wraps an existing Jiff [`Span`].
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn from_span(span: Span) -> Self {
        Self::new(span)
    }

    /// Returns the written components that witness the regime.
    #[cfg(test)]
    #[inline]
    #[must_use]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "unit counts stay far below f64 mantissa precision"
    )]
    pub(crate) fn components(&self) -> Vec<DurationComponent> {
        let mut result = Vec::new();
        let span = self.span;
        if span.get_years() != 0 {
            result.push(DurationComponent {
                magnitude: f64::from(span.get_years()),
                unit: TimeUnit::Year,
            });
        }
        if span.get_months() != 0 {
            result.push(DurationComponent {
                magnitude: f64::from(span.get_months()),
                unit: TimeUnit::Month,
            });
        }
        if span.get_weeks() != 0 {
            result.push(DurationComponent {
                magnitude: f64::from(span.get_weeks()),
                unit: TimeUnit::Week,
            });
        }
        if span.get_days() != 0 {
            result.push(DurationComponent {
                magnitude: f64::from(span.get_days()),
                unit: TimeUnit::Day,
            });
        }
        if span.get_hours() != 0 {
            result.push(DurationComponent {
                magnitude: f64::from(span.get_hours()),
                unit: TimeUnit::Hour,
            });
        }
        if span.get_minutes() != 0 {
            result.push(DurationComponent {
                magnitude: span.get_minutes() as f64,
                unit: TimeUnit::Minute,
            });
        }
        if span.get_seconds() != 0 {
            result.push(DurationComponent {
                magnitude: span.get_seconds() as f64,
                unit: TimeUnit::Second,
            });
        }
        result
    }

    /// Returns `true` if this duration contains a calendar component.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn has_calendar_component(&self) -> bool {
        self.span.get_years() != 0
            || self.span.get_months() != 0
            || self.span.get_weeks() != 0
            || self.span.get_days() != 0
    }
}

impl fmt::Debug for DurationValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DurationValue").field(&self.span).finish()
    }
}

impl fmt::Display for DurationValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        PRINTER
            .print_span(&self.span, jiff::fmt::StdFmtWrite(f))
            .map_err(|_| fmt::Error)
    }
}

impl FromStr for DurationValue {
    type Err = DurationError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq for DurationValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.nominal_seconds() == other.nominal_seconds()
    }
}

impl Eq for DurationValue {}

impl PartialOrd for DurationValue {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DurationValue {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.nominal_seconds().cmp(&other.nominal_seconds())
    }
}

impl Hash for DurationValue {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.nominal_seconds().hash(state);
    }
}

impl Add for DurationValue {
    type Output = Result<Self, DurationError>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
    }
}

impl Sub for DurationValue {
    type Output = Result<Self, DurationError>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
    }
}

impl Mul<f64> for DurationValue {
    type Output = Result<Self, DurationError>;

    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        self.checked_mul(rhs)
    }
}

impl Mul<i64> for DurationValue {
    type Output = Result<Self, DurationError>;

    #[inline]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "scale factor converted to float for checked multiplication"
    )]
    fn mul(self, rhs: i64) -> Self::Output {
        self.checked_mul(rhs as f64)
    }
}

impl Neg for DurationValue {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self::new(self.span.negate())
    }
}

impl Serialize for DurationValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DurationValue {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Stores one written duration magnitude and its time unit.
#[cfg(test)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct DurationComponent {
    pub(crate) magnitude: f64,
    pub(crate) unit: TimeUnit,
}

#[cfg(test)]
impl DurationComponent {
    /// Returns the signed magnitude of this component.
    #[must_use]
    pub(crate) fn magnitude(&self) -> f64 {
        self.magnitude
    }

    /// Returns the unit of this component.
    #[must_use]
    pub(crate) fn unit(&self) -> TimeUnit {
        self.unit
    }
}

#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "bounded slice lookaheads and index accesses"
)]
fn validate_friendly_input(input: &str) -> Result<(), DurationError> {
    for word in input.split(|c: char| c.is_whitespace() || c == ',') {
        let digits_len = word
            .trim_start_matches(['+', '-'])
            .chars()
            .take_while(char::is_ascii_digit)
            .count();
        if digits_len > 300 {
            return Err(DurationError::NumberOverflow);
        }
        // The closed unit vocabulary stops at seconds; Jiff's own `ms`, `us`
        // and `ns` spellings must not leak through.
        let unit = word.trim_start_matches(|c: char| {
            c.is_ascii_digit() || matches!(c, '.' | '+' | '-')
        });
        if matches!(unit, "ms" | "millisecond" | "milliseconds" | "us" | "ns") {
            return Err(DurationError::UnknownUnit);
        }
    }

    let bytes = input.as_bytes();
    if let Some(&first) = bytes.first() {
        if first.is_ascii_alphabetic() {
            return Err(DurationError::MissingNumber);
        }
        if (first == b'+' || first == b'-')
            && bytes.get(1).is_some_and(u8::is_ascii_alphabetic)
        {
            return Err(DurationError::MisplacedSign);
        }
    }

    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if (b == b'+' || b == b'-')
            && bytes
                .get(i.saturating_sub(1))
                .is_some_and(u8::is_ascii_whitespace)
        {
            return Err(DurationError::MisplacedSign);
        }
    }

    for (i, &b) in bytes.iter().enumerate() {
        if b == b','
            && let Some(&next) = bytes.get(i.saturating_add(1))
            && !next.is_ascii_whitespace()
        {
            return Err(DurationError::MissingUnit);
        }
    }

    for (i, &b) in bytes.iter().enumerate() {
        if (b == b'e' || b == b'E')
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && let Some(&next) = bytes.get(i + 1)
            && (next.is_ascii_digit()
                || ((next == b'+' || next == b'-')
                    && bytes.get(i + 2).is_some_and(u8::is_ascii_digit)))
        {
            return Err(DurationError::UnknownUnit);
        }
    }

    let last_token = input.split_whitespace().last().unwrap_or(input);
    if last_token
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '+' || c == '-')
    {
        return Err(DurationError::MissingUnit);
    }

    if !input.is_ascii() {
        return Err(DurationError::MissingUnit);
    }

    Ok(())
}

#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "pre-checked index access on char slice"
)]
fn normalize_friendly_input(input: &str) -> Cow<'_, str> {
    if !input.contains('.') {
        return Cow::Borrowed(input);
    }
    let mut result = String::with_capacity(input.len() + 4);
    let chars: Vec<char> = input.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '.' {
            let needs_zero = if i == 0 {
                true
            } else {
                let prev = chars[i - 1];
                prev == '+' || prev == '-' || prev.is_whitespace()
            };
            if needs_zero {
                result.push('0');
            }
        }
        result.push(c);
    }
    Cow::Owned(result)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "takes error to inspect formatted message"
)]
fn classify_jiff_error(input: &str, err: jiff::Error) -> DurationError {
    let msg = err.to_string();
    let msg_lower = msg.to_ascii_lowercase();

    if msg_lower.contains("range")
        || msg_lower.contains("overflow")
        || msg_lower.contains("too big")
        || msg_lower.contains("exceed")
    {
        return DurationError::SecondsOutOfRange;
    }
    if msg_lower.contains("empty") {
        return DurationError::Empty;
    }
    if msg_lower.contains("missing number") || msg_lower.contains("no number") {
        return DurationError::MissingNumber;
    }
    if input.ends_with(|c: char| c.is_ascii_digit()) {
        return DurationError::MissingUnit;
    }
    if msg_lower.contains("unit")
        || msg_lower.contains("designator")
        || msg_lower.contains("unknown")
    {
        return DurationError::UnknownUnit;
    }
    if msg_lower.contains("sign") {
        return DurationError::MisplacedSign;
    }
    if msg_lower.contains("unparsed input") {
        return DurationError::MissingUnit;
    }
    if input.chars().any(|c| c.is_ascii_digit())
        && !input.chars().any(|c| c.is_ascii_alphabetic())
    {
        return DurationError::MissingUnit;
    }
    DurationError::UnknownUnit
}
#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher as _},
    };

    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    fn synthesized(raw_secs: f64) -> DurationValue {
        DurationValue::from_nominal_seconds(raw_secs)
            .expect("test duration is within the Jiff span range")
    }

    fn hash_val<T: Hash>(val: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        val.hash(&mut hasher);
        hasher.finish()
    }

    fn parse_seconds(input: &str) -> Result<f64, String> {
        DurationValue::parse(input)
            .map(|duration| duration.magnitude())
            .map_err(|error| error.to_string())
    }

    fn parse_error(input: &str) -> Option<DurationError> {
        DurationValue::parse(input).err()
    }

    fn parse_prefix_parts(input: &str) -> Result<(String, usize, f64), String> {
        DurationValue::parse_prefix(input)
            .map(|parsed| {
                let duration = parsed.value();
                (
                    duration.to_string(),
                    parsed.span_usize().end,
                    duration.magnitude(),
                )
            })
            .ok_or_else(|| "prefix rejected".to_owned())
    }

    mod constructor {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::zero(0.0, "0s")]
        #[case::seconds(90.0, "1m 30s")]
        #[case::minutes(5_400.0, "1h 30m")]
        #[case::sub_second(0.5, "1s")]
        #[case::large_compound(
            604_800.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0,
            "1w 1d 1h 1m 1s"
        )]
        #[case::negative(-5_400.0, "-1h 30m")]
        #[case::signed_zero(-0.0, "0s")]
        fn constructs_from_nominal_seconds(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            let duration = synthesized(raw_secs);
            assert_eq!(duration.to_string(), expected);
            {
                if raw_secs == 0.0 || raw_secs == -0.0 {
                    assert_eq!(duration.nominal_seconds(), 0);
                }
            }
        }

        #[test]
        fn rejects_nominal_seconds_outside_the_span_range() {
            let seconds = 1e20;
            let result = DurationValue::from_nominal_seconds(seconds);
            assert!(matches!(result, Err(DurationError::SecondsOutOfRange)));
        }

        #[test]
        fn rejects_non_finite_nominal_seconds() {
            assert!(matches!(
                DurationValue::from_nominal_seconds(f64::NAN),
                Err(DurationError::NonFiniteSeconds)
            ));
            assert!(matches!(
                DurationValue::from_nominal_seconds(f64::INFINITY),
                Err(DurationError::NonFiniteSeconds)
            ));
        }

        #[test]
        fn nominal_seconds_rounding_is_symmetric_and_accepts_the_bound() {
            for (seconds, expected_seconds) in [
                (1.0, 1_i64),
                (60.0, 60),
                (617_145.0, 617_145),
                (MAX_NOMINAL_SECONDS, 631_107_417_600),
            ] {
                let positive =
                    DurationValue::from_nominal_seconds(seconds).unwrap();
                let negative =
                    DurationValue::from_nominal_seconds(-seconds).unwrap();

                assert_eq!(
                    negative.to_string(),
                    format!("-{positive}"),
                    "seconds: {seconds}"
                );
                assert_eq!(
                    positive.nominal_seconds(),
                    expected_seconds,
                    "seconds: {seconds}"
                );
            }
        }

        #[test]
        fn nominal_seconds_just_outside_the_bound_are_rejected() {
            for seconds in
                [MAX_NOMINAL_SECONDS + 1.0, -MAX_NOMINAL_SECONDS - 1.0]
            {
                assert!(matches!(
                    DurationValue::from_nominal_seconds(seconds),
                    Err(DurationError::SecondsOutOfRange)
                ));
            }
        }
        #[test]
        fn caches_nominal_seconds_for_every_constructor() {
            let written = DurationValue::parse("-1mo 2d 3h").unwrap();
            let iso = DurationValue::parse("PT1H").unwrap();
            let fraction = DurationValue::parse("-1.5s").unwrap();
            let synthesized =
                DurationValue::from_nominal_seconds(-5_400.0).unwrap();
            let from_span =
                DurationValue::from_span(Span::new().hours(2).minutes(15));
            for duration in [written, iso, fraction, synthesized, from_span] {
                assert_eq!(
                    duration.nominal_seconds(),
                    nominal_seconds(duration.span())
                );
            }
        }
    }
    mod parse {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::seconds("90s", 90.0)]
        #[case::minutes("30m", 1_800.0)]
        #[case::hours("1h", 3_600.0)]
        #[case::days("2d", 172_800.0)]
        #[case::weeks("1w", 604_800.0)]
        #[case::months("3mo", 7_776_000.0)]
        #[case::years("1y", 31_536_000.0)]
        #[case::long_year("1 year", 31_536_000.0)]
        #[case::long_months("2 months", 5_184_000.0)]
        #[case::long_weeks("3 weeks", 1_814_400.0)]
        #[case::long_days("4 days", 345_600.0)]
        #[case::long_hours("5 hours", 18_000.0)]
        #[case::long_minutes("6 minutes", 360.0)]
        #[case::long_seconds("7 seconds", 7.0)]
        fn accepts_each_unit_alone(#[case] input: &str, #[case] expected: f64) {
            let result = parse_seconds(input);
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::space("1h 30m", 5_400.0)]
        #[case::comma("4 yrs, 6 wks", 129_772_800.0)]
        #[case::compound_three("1w 2d 3h", 788_400.0)]
        #[case::iso_date_and_time("P1DT2H", 93_600.0)]
        #[case::iso_time_minutes("PT90M", 5_400.0)]
        #[case::iso_negative_time("-PT1H", -3_600.0)]
        #[case::iso_positive_month("P1M", 2_592_000.0)]
        #[case::iso_negative_month_before_p("-P1M", -2_592_000.0)]
        #[case::iso_negative_date_time("-P1DT2H", -93_600.0)]
        #[case::iso_time_only_minutes("PT30M", 1_800.0)]
        fn accepts_compound_and_iso_durations(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            let result = parse_seconds(input);
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::plus("+1h", 3_600.0)]
        #[case::minus("-30m", -1_800.0)]
        #[case::leading_sign_on_compound("-1h 30m", -5_400.0)]
        fn accepts_leading_sign(#[case] input: &str, #[case] expected: f64) {
            let result = parse_seconds(input);
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::hour_fraction("1.5h", 5_400.0)]
        #[case::fraction_without_zero(".5h", 1_800.0)]
        #[case::seconds_fraction(".5s", 1.0)]
        #[case::negative_fraction("-.5h", -1_800.0)]
        #[case::positive_fraction("+.5h", 1_800.0)]
        fn accepts_hour_or_smaller_fraction_on_lowest_unit(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            let result = parse_seconds(input);
            assert_eq!(result, Ok(expected));
        }

        #[test]
        fn accepts_range_boundary() {
            assert!(DurationValue::parse("7304484d").is_ok());
            assert!(matches!(
                DurationValue::parse("7304485d"),
                Err(DurationError::SecondsOutOfRange)
            ));
        }

        #[test]
        fn trims_surrounding_whitespace() {
            assert_eq!(parse_seconds("  1h  "), Ok(3_600.0));
        }

        #[rstest]
        #[case::empty("")]
        #[case::whitespace("   ")]
        fn rejects_empty_or_blank(#[case] input: &str) {
            assert!(matches!(parse_error(input), Some(DurationError::Empty)));
        }

        #[rstest]
        #[case::hours("hours")]
        #[case::h("h")]
        fn rejects_unit_without_number(#[case] input: &str) {
            assert!(matches!(
                parse_error(input),
                Some(DurationError::MissingNumber)
            ));
        }

        #[rstest]
        #[case::bare_number("42")]
        #[case::bare_one("1")]
        #[case::unit_without_number_mid("1h30")]
        #[case::lone_comma_before_unit("1,h")]
        #[case::spaced_number_without_unit("1 h 30")]
        #[case::non_ascii_unit("1µs")]
        fn rejects_number_without_unit(#[case] input: &str) {
            assert!(matches!(
                parse_error(input),
                Some(DurationError::MissingUnit)
            ));
        }

        #[rstest]
        #[case::unknown_unit("1x")]
        #[case::unknown_word_unit("1fortnight")]
        #[case::milli_unit("1ms")]
        #[case::milli_word("8 milliseconds")]
        #[case::micro_unit("1us")]
        #[case::capital_with_space("1 H")]
        #[case::exponent("1e3s")]
        fn rejects_unknown_or_unsupported_unit(#[case] input: &str) {
            assert!(matches!(
                parse_error(input),
                Some(DurationError::UnknownUnit)
            ));
        }

        #[rstest]
        #[case::sign_inside_neg("1h -30m")]
        #[case::sign_inside_pos("1h +30m")]
        #[case::misplaced_plus("+h")]
        #[case::misplaced_minus("-h")]
        fn rejects_misplaced_sign(#[case] input: &str) {
            assert!(matches!(
                parse_error(input),
                Some(DurationError::MisplacedSign)
            ));
        }

        #[rstest]
        #[case::repeated_unit("1h 1h")]
        #[case::ascending_order("1m 1h")]
        #[case::fraction_calendar_day("1.5d")]
        #[case::fraction_calendar_month("1.5mo")]
        #[case::fraction_non_lowest("1.5h 30m")]
        #[case::comma_without_space("1h,30m")]
        #[case::malformed_iso_fraction("P1.5D")]
        #[case::malformed_iso_negative("P-1M")]
        fn rejects_invalid_grammar(#[case] input: &str) {
            assert!(
                DurationValue::parse(input).is_err(),
                "grammar form {input:?} must be rejected"
            );
        }

        #[test]
        fn rejects_number_overflow() {
            let huge = format!("1{}h", "0".repeat(400));
            assert!(matches!(
                parse_error(&huge),
                Some(DurationError::NumberOverflow)
            ));
        }
    }

    mod parse_prefix {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::whole_input("1h", 2, "1h", 3_600.0)]
        #[case::bracket_stops("45m]", 3, "45m", 2_700.0)]
        #[case::decimal(".5h remainder", 3, "30m", 1_800.0)]
        #[case::multi_part_whitespace("1h 30m, extra", 6, "1h 30m", 5_400.0)]
        #[case::tightly_packed("1h30m rest", 5, "1h 30m", 5_400.0)]
        #[case::comma_stops_atom("1h, 30m, extra", 2, "1h", 3_600.0)]
        #[case::negative("-15m extra", 4, "-15m", -900.0)]
        #[case::positive("+15m extra", 4, "15m", 900.0)]
        #[case::decimal_after_sign("-.5h rest", 4, "-30m", -1_800.0)]
        fn parses_a_valid_prefix(
            #[case] input: &str,
            #[case] expected_consumed: usize,
            #[case] expected_str: &str,
            #[case] expected_secs: f64,
        ) {
            let result = parse_prefix_parts(input);
            assert_eq!(
                result,
                Ok((expected_str.to_owned(), expected_consumed, expected_secs))
            );
        }

        #[test]
        fn stops_before_a_multibyte_character() {
            let input = "1h é";
            let result = parse_prefix_parts(input);
            assert_eq!(result, Ok(("1h".to_owned(), 2, 3_600.0)));
        }

        #[test]
        fn stops_before_an_attached_symbol() {
            let input = "1h°C";
            let result = parse_prefix_parts(input);
            assert_eq!(result, Ok(("1h".to_owned(), 2, 3_600.0)));
        }

        #[rstest]
        #[case::non_duration("not a duration")]
        #[case::unknown_suffix("1h 1x")]
        #[case::leading_comma(",1h")]
        #[case::hyphen_bullet("- 15m")]
        #[case::non_leading_negative("1h -30m")]
        #[case::empty("")]
        #[case::clock_time("10:30:00")]
        #[case::unit_attached_to_word("1hello")]
        #[case::bare_number("5")]
        fn rejects_an_invalid_prefix(#[case] input: &str) {
            let result = parse_prefix_parts(input);
            assert_eq!(result, Err("prefix rejected".to_owned()));
        }
    }

    mod arithmetic {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn fieldwise_addition_preserves_distinct_units() {
            let day = DurationValue::parse("1d").unwrap();
            let hour = DurationValue::parse("1h").unwrap();
            let sum = day.checked_add(hour).unwrap();
            assert_eq!(sum.to_string(), "1d 1h");
        }

        #[test]
        fn fieldwise_subtraction_preserves_remaining_units() {
            let compound = DurationValue::parse("1d 1h").unwrap();
            let hour = DurationValue::parse("1h").unwrap();
            let diff = compound.checked_sub(hour).unwrap();
            assert_eq!(diff.to_string(), "1d");
        }

        #[test]
        fn subtracts_to_zero() {
            let day = DurationValue::parse("1d").unwrap();
            let diff = day.checked_sub(day).unwrap();
            assert_eq!(diff.to_string(), "0s");
            {
                assert_eq!(diff.nominal_seconds(), 0);
            }
        }

        #[test]
        fn mixed_sign_subtraction_decomposes_nominally() {
            let day = DurationValue::parse("1d").unwrap();
            let hour = DurationValue::parse("1h").unwrap();
            let diff = day.checked_sub(hour).unwrap();
            assert_eq!(diff.to_string(), "23h");
        }

        #[test]
        fn preserves_calendar_fields_when_signs_agree() {
            let left = DurationValue::parse("-1mo 2d").unwrap();
            let right = DurationValue::parse("-2mo 1d").unwrap();
            let sum = left.checked_add(right).unwrap();
            assert_eq!(sum.to_string(), "-3mo 3d");
            assert_eq!(sum.nominal_seconds(), -3 * 2_592_000 - 3 * 86_400);
        }

        #[test]
        fn decomposes_mixed_calendar_and_clock_signs_nominally() {
            let month = DurationValue::parse("1mo").unwrap();
            let day = DurationValue::parse("1d").unwrap();
            let difference = month.checked_sub(day).unwrap();
            assert_eq!(difference.to_string(), "4w 1d");
            assert_eq!(difference.nominal_seconds(), 29 * 86_400);
        }

        #[test]
        fn rounds_fractional_seconds_before_fieldwise_arithmetic() {
            let fraction = DurationValue::parse(".5s").unwrap();
            let second = DurationValue::parse("1s").unwrap();
            assert_eq!(fraction.checked_add(second).unwrap().to_string(), "2s");
            assert_eq!(
                (-fraction).checked_add(-second).unwrap().to_string(),
                "-2s"
            );
        }

        #[test]
        fn checked_negation() {
            let hour = DurationValue::parse("1h").unwrap();
            let neg = hour.checked_neg().unwrap();
            assert_eq!(neg.to_string(), "-1h");
            assert_eq!(neg.checked_neg().unwrap(), hour);
        }

        #[test]
        fn maintains_cached_seconds_through_arithmetic() {
            let month = DurationValue::parse("1mo").unwrap();
            let day = DurationValue::parse("1d").unwrap();
            let fraction = DurationValue::parse("1.5s").unwrap();
            let results = [
                month.checked_add(day).unwrap(),
                month.checked_sub(day).unwrap(),
                fraction.checked_mul(2.0).unwrap(),
                day.checked_mul(0.5).unwrap(),
                (-fraction).checked_neg().unwrap(),
                day.checked_mul(0.0).unwrap(),
            ];
            for duration in results {
                assert_eq!(
                    duration.nominal_seconds(),
                    nominal_seconds(duration.span())
                );
            }
        }
    }

    mod scaling {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn integral_scaling_preserves_units() {
            let dur = DurationValue::parse("1d 2h").unwrap();
            let scaled = dur.checked_mul(2.0).unwrap();
            assert_eq!(scaled.to_string(), "2d 4h");
        }

        #[test]
        fn rounds_fractional_seconds_before_integral_scaling() {
            let fraction = DurationValue::parse("1.5s").unwrap();
            assert_eq!(fraction.checked_mul(2.0).unwrap().to_string(), "4s");
            assert_eq!(
                (-fraction).checked_mul(2.0).unwrap().to_string(),
                "-4s"
            );
        }

        #[test]
        fn fractional_scaling_decomposes_nominally() {
            let dur = DurationValue::parse("1d").unwrap();
            let scaled = dur.checked_mul(0.5).unwrap();
            assert_eq!(scaled.to_string(), "12h");
        }

        #[test]
        fn scaling_by_zero_yields_zero() {
            let dur = DurationValue::parse("1d").unwrap();
            let scaled = dur.checked_mul(0.0).unwrap();
            assert_eq!(scaled.to_string(), "0s");
        }

        #[test]
        fn negative_scaling_negates_units() {
            let dur = DurationValue::parse("1d").unwrap();
            let scaled = dur.checked_mul(-1.0).unwrap();
            assert_eq!(scaled.to_string(), "-1d");
        }

        #[test]
        fn non_finite_scaling_is_rejected() {
            let dur = DurationValue::parse("1d").unwrap();
            assert!(matches!(
                dur.checked_mul(f64::NAN),
                Err(DurationError::NonFiniteSeconds)
            ));
        }
    }

    mod equality {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;
        #[test]
        fn compares_by_nominal_nanoseconds() {
            let a = DurationValue::parse("1h 30m").unwrap();
            let b = DurationValue::parse("90m").unwrap();
            assert_eq!(a, b);

            let day = DurationValue::parse("1d").unwrap();
            let hours = DurationValue::parse("24h").unwrap();
            assert_eq!(day, hours);

            let month = DurationValue::parse("1mo").unwrap();
            let days30 = DurationValue::parse("30d").unwrap();
            assert_eq!(month, days30);

            let zero_pos = DurationValue::parse("0s").unwrap();
            let zero_neg = DurationValue::parse("-0s").unwrap();
            assert_eq!(zero_pos, zero_neg);

            assert_ne!(a, day);
        }
    }

    mod ordering {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn orders_consistently_with_equality() {
            let neg = DurationValue::parse("-1h").unwrap();
            let zero = DurationValue::parse("0s").unwrap();
            let pos = DurationValue::parse("1h").unwrap();

            assert!(neg < zero);
            assert!(zero < pos);
            assert!(neg < pos);

            let month = DurationValue::parse("1mo").unwrap();
            let days30 = DurationValue::parse("30d").unwrap();
            assert_eq!(month.cmp(&days30), Ordering::Equal);
        }
    }

    mod hashing {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn equal_durations_hash_identically() {
            let a = DurationValue::parse("1h 30m").unwrap();
            let b = DurationValue::parse("90m").unwrap();
            assert_eq!(hash_val(&a), hash_val(&b));

            let day = DurationValue::parse("1d").unwrap();
            let hours = DurationValue::parse("24h").unwrap();
            assert_eq!(hash_val(&day), hash_val(&hours));

            let month = DurationValue::parse("1mo").unwrap();
            let days30 = DurationValue::parse("30d").unwrap();
            assert_eq!(hash_val(&month), hash_val(&days30));
        }
    }

    mod formatting {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::zero(0.0, "0s")]
        #[case::negative(-5_400.0, "-1h 30m")]
        #[case::sub_second(0.5, "1s")]
        #[case::compound_calendar(2.0f64.mul_add(2_592_000.0, 31_536_000.0), "1y 2mo")]
        #[case::mixed(
            31_536_000.0 + 2_592_000.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0,
            "1y 1mo 1d 1h 1m 1s"
        )]
        fn formats_durations_canonically(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            #[expect(
                clippy::float_cmp,
                reason = "case literals compare exactly against themselves"
            )]
            let duration = if raw_secs
                == 2.0f64.mul_add(2_592_000.0, 31_536_000.0)
            {
                DurationValue::parse("1y 2mo").unwrap()
            } else if raw_secs
                == 31_536_000.0 + 2_592_000.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0
            {
                DurationValue::parse("1y 1mo 1d 1h 1m 1s").unwrap()
            } else {
                synthesized(raw_secs)
            };
            assert_eq!(duration.to_string(), expected);
        }

        #[test]
        fn preserves_tuple_debug_format_when_cached() {
            let duration = DurationValue::parse("1h 30m").unwrap();
            assert_eq!(
                format!("{duration:?}"),
                format!("DurationValue({:?})", duration.span())
            );
        }
    }

    mod serialization {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::simple("1h 30m")]
        #[case::negative("-1h 30m")]
        #[case::zero("0s")]
        fn serializes_to_canonical_string(#[case] input: &str) {
            let duration = DurationValue::parse(input).expect("valid duration");
            let json = serde_json::to_string(&duration).expect("serializes");
            assert_eq!(json, format!("\"{input}\""));
        }
    }

    mod deserialization {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn accepts_valid_string() {
            let json = "\"1h 30m\"";
            let duration: DurationValue =
                serde_json::from_str(json).expect("deserializes");
            assert_eq!(duration, DurationValue::parse("1h 30m").unwrap());
        }

        #[test]
        fn restores_cached_seconds_when_deserializing() {
            let duration: DurationValue =
                serde_json::from_str("\"-1mo 2d\"").unwrap();
            assert_eq!(
                duration.nominal_seconds(),
                nominal_seconds(duration.span())
            );
        }

        #[test]
        fn rejects_non_string_scalar() {
            let json = "42";
            let result: Result<DurationValue, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn rejects_invalid_string() {
            let json = "\"invalid\"";
            let result: Result<DurationValue, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }
    }

    mod decomposition {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::zero(0.0, "0s")]
        #[case::sub_minute(59.0, "59s")]
        #[case::exact_minute(60.0, "1m")]
        #[case::sub_hour(3_540.0, "59m")]
        #[case::exact_hour(3_600.0, "1h")]
        #[case::sub_day(82_800.0, "23h")]
        #[case::exact_day(86_400.0, "1d")]
        #[case::sub_week(518_400.0, "6d")]
        #[case::exact_week(604_800.0, "1w")]
        #[case::compound_sub_month(2_592_000.0, "4w 2d")]
        #[case::compound_sub_year(31_536_000.0, "52w 1d")]
        #[case::negative_compound(-5_400.0, "-1h 30m")]
        fn decomposes_nominal_seconds_greedily(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            let duration = synthesized(raw_secs);
            assert_eq!(duration.to_string(), expected);
        }

        #[test]
        fn fractional_seconds_round_to_nearest_whole_second() {
            let dur = synthesized(0.4);
            assert_eq!(dur.to_string(), "0s");
            let dur2 = synthesized(1.5);
            assert_eq!(dur2.to_string(), "2s");
        }
    }

    mod compound_format {
        use pretty_assertions::assert_eq;

        use super::*;
        #[rstest]
        #[case::zero(0.0, "0 seconds")]
        #[case::seconds(45.0, "45 seconds")]
        #[case::minute(60.0, "1 minute")]
        #[case::minutes(120.0, "2 minutes")]
        #[case::hours_and_minutes(3_660.0, "1 hour, 1 minute")]
        #[case::compound_calendar(
            31_536_000.0 + 2_592_000.0 + 86_400.0,
            "1 year, 1 month, 1 day"
        )]
        #[case::negative(-3_600.0, "-1 hour")]
        #[case::fractional_rounds(0.4, "0 seconds")]
        fn renders_compound_english_from_nominal_seconds(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            let duration = if raw_secs > 2_592_000.0 {
                DurationValue::parse("1y 1mo 1d").unwrap()
            } else {
                synthesized(raw_secs)
            };
            assert_eq!(duration.to_compound_string(), expected);
        }
    }

    mod components {
        use pretty_assertions::assert_eq;

        use super::*;
        #[test]
        fn component_iteration_order_largest_to_smallest() {
            let duration =
                DurationValue::parse("1y 2mo 3w 4d 5h 6m 7s").unwrap();
            let components = duration.components();
            assert_eq!(components.len(), 7);
            #[expect(
                clippy::float_cmp,
                reason = "component magnitudes are exact unit counts"
            )]
            {
                let first = components.first().expect("at least one component");
                assert_eq!(first.unit(), TimeUnit::Year);
                assert_eq!(first.magnitude(), 1.0);
                let last = components.last().expect("at least one component");
                assert_eq!(last.unit(), TimeUnit::Second);
                assert_eq!(last.magnitude(), 7.0);
            }
        }

        #[test]
        fn negative_duration_has_negative_components() {
            let duration = DurationValue::parse("-1h 30m").unwrap();
            let components = duration.components();
            #[expect(
                clippy::float_cmp,
                reason = "component magnitudes are exact unit counts"
            )]
            {
                let first = components.first().expect("at least one component");
                let second = components.get(1).expect("exactly two components");
                assert_eq!(first.magnitude(), -1.0);
                assert_eq!(second.magnitude(), -30.0);
            }
        }

        #[test]
        fn calendar_component_check() {
            let cal = DurationValue::parse("1d 2h").unwrap();
            assert!(cal.has_calendar_component());
            let exact = DurationValue::parse("2h 30m").unwrap();
            assert!(!exact.has_calendar_component());
        }
    }

    mod proptests {
        use super::*;

        proptest! {
            #[test]
            fn display_round_trips_through_parser(
                seconds in -1_000_000_i32..=1_000_000_i32,
            ) {
                let original = synthesized(f64::from(seconds));
                let displayed = original.to_string();
                let reparsed = DurationValue::parse(&displayed).expect("display must parse");
                prop_assert_eq!(original, reparsed);
            }

            #[test]
            fn nominal_constructor_preserves_integer_seconds(
                seconds in -1_000_000_000_i32..=1_000_000_000_i32,
            ) {
                let duration = synthesized(f64::from(seconds));
                prop_assert_eq!(
                    duration.nominal_seconds(),
                    i64::from(seconds),
                );
            }

            #[test]
            fn negating_twice_preserves_nominal_identity(
                seconds in -1_000_000_i32..=1_000_000_i32,
            ) {
                let duration = synthesized(f64::from(seconds));
                let original = duration.nominal_seconds();
                let restored = duration
                    .checked_neg()
                    .and_then(DurationValue::checked_neg);
                {
                    prop_assert!(matches!(
                        restored,
                        Ok(value) if value.nominal_seconds() == original
                    ));
                }
            }

            #[test]
            fn add_then_subtract_preserves_nominal_identity(
                left in -1_000_000_i32..=1_000_000_i32,
                right in -1_000_000_i32..=1_000_000_i32,
            ) {
                let left_val = synthesized(f64::from(left));
                let right_val = synthesized(f64::from(right));
                let expected = left_val.nominal_seconds();

                let sum = left_val.checked_add(right_val).unwrap();
                let diff = sum.checked_sub(right_val).unwrap();

                prop_assert_eq!(diff.nominal_seconds(), expected);
            }

            #[test]
            fn nominal_identity_obeys_eq_ord_hash_contract(
                seconds in -1_000_000_i32..=1_000_000_i32,
            ) {
                let nominal = synthesized(f64::from(seconds));
                let written = DurationValue::parse(&format!("{seconds}s"))
                    .expect("integer-second spelling parses");

                let ordering = nominal.cmp(&written);
                let nominal_hash = hash_val(&nominal);
                let written_hash = hash_val(&written);

                prop_assert!(nominal == written);
                prop_assert_eq!(ordering, std::cmp::Ordering::Equal);
                prop_assert_eq!(nominal_hash, written_hash);
            }

            #[test]
            fn scaling_by_one_preserves_identity(
                seconds in -1_000_000_i32..=1_000_000_i32,
            ) {
                let duration = synthesized(f64::from(seconds));
                let original = duration.nominal_seconds();
                let scaled = duration.checked_mul(1.0);
                {
                    prop_assert!(matches!(
                        scaled,
                        Ok(value) if value.nominal_seconds() == original
                    ));
                }
            }
        }
    }
}
