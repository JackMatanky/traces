//! Date points and the precision-aware arithmetic over them.
//!
//! Owns [`DatePoint`], a Date written at a month or day [`DatePrecision`], or a
//! Date-Time stored as UTC that renders as the local clock. Parsing retains the
//! written precision; month precision denotes the first of the month.
//! Arithmetic applies calendar units on the local clock and exact units as
//! exact elapsed time. Callers hold the validated date point or receive a
//! [`DateError`].

use jiff::{Span, Timestamp, civil};
use num_traits::ToPrimitive as _;

use super::{
    CalendarError, CalendarPeriod, DATE_PATTERN, DateComponent, DateError,
    DateTimeValue, DateValue, Difference, DurationComponent, DurationValue,
    PeriodEdge, TimeUnit,
    calendar::{shift_datetime, span_from_seconds},
    date::DateShape,
    datetime::{DateTimeShape, ParsedDateTime},
};

/// Calendar precision of a zone-free date point.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePrecision {
    /// Year and month.
    Month,
    /// Full calendar date.
    Day,
}

/// A Date written at a month or day precision, or a Date-Time.
///
/// Calendar points are zone-free and carry the written [`DatePrecision`].
/// Date-Time points store UTC and resolve naive input in the process's local
/// zone (a repeated local time takes the earlier occurrence; a skipped local
/// time shifts forward by the gap length).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePoint {
    /// A zone-free Date at the written precision.
    Calendar {
        /// The calendar date; month precision stores the first of the month.
        date: DateValue,
        /// How much of `date` was written.
        date_precision: DatePrecision,
    },
    /// A Date-Time stored as UTC, rendered as the local clock.
    DateTime(DateTimeValue),
}

impl From<DateValue> for DatePoint {
    fn from(date: DateValue) -> Self {
        Self::Calendar {
            date,
            date_precision: DatePrecision::Day,
        }
    }
}

impl From<DateTimeValue> for DatePoint {
    fn from(date_time: DateTimeValue) -> Self {
        Self::DateTime(date_time)
    }
}
impl DatePoint {
    /// Whether this date point carries a time-of-day component.
    #[must_use]
    pub(crate) const fn has_time(self) -> bool {
        matches!(self, Self::DateTime(_))
    }

    /// Calendar date of this date point in the local zone.
    pub(crate) fn date(self) -> DateValue {
        match self {
            Self::Calendar {
                date,
                ..
            } => date,
            Self::DateTime(dt) => dt.local_date(),
        }
    }

    /// Local datetime reading of this date point: midnight for a calendar date,
    /// the local clock time for a date-time point.
    #[must_use]
    pub(crate) fn local_datetime_or_utc(self) -> jiff::civil::DateTime {
        match self {
            Self::Calendar {
                date,
                ..
            } => date.civil_date().at(0, 0, 0, 0),
            Self::DateTime(dt) => dt.local_datetime_or_utc(),
        }
    }

    /// Returns the UTC-stored Date-Time equivalent of this date point.
    pub(crate) fn instant(self) -> Result<Timestamp, CalendarError> {
        match self {
            Self::Calendar {
                date,
                ..
            } => DateTimeValue::try_from(date)
                .map(DateTimeValue::utc)
                .map_err(|_| CalendarError::OutOfRange),
            Self::DateTime(datetime) => Ok(datetime.utc()),
        }
    }

    /// Canonical strftime pattern preserving written precision.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        match self {
            Self::Calendar {
                date_precision: DatePrecision::Month,
                ..
            } => "%Y-%m",
            Self::Calendar {
                date_precision: DatePrecision::Day,
                ..
            } => DATE_PATTERN,
            Self::DateTime(dt) => dt.canonical_pattern(),
        }
    }

    /// Shifts by `n` whole units: calendar units move the local clock, exact
    /// units move the exact time (calendar dates move only when the shift
    /// crosses midnight).
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the shift exceeds the supported date
    ///   range.
    /// - [`CalendarError::Zone`] if a date-time point’s shifted local datetime
    ///   cannot be resolved.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: TimeUnit,
    ) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date_precision,
                ..
            } => {
                let local_datetime =
                    shift_datetime(self.local_datetime_or_utc(), n, unit)?;
                Ok(Self::Calendar {
                    date: DateValue::from_civil_date(local_datetime.date()),
                    date_precision,
                })
            }
            Self::DateTime(dt) => dt.shift(n, unit).map(Self::DateTime),
        }
    }

    /// Applies retained components in written order, or fixed seconds exactly.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if a component is non-finite, a calendar
    ///   magnitude does not fit `i64`, or any shift overflows.
    /// - [`CalendarError::Zone`] if a date-time point’s shifted local datetime
    ///   cannot be resolved.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
    ) -> Result<Self, CalendarError> {
        match duration.components() {
            Some(components) => {
                components.iter().try_fold(self, |date_point, &component| {
                    date_point.apply_component(component)
                })
            }
            None => self.advance_span(duration.span()),
        }
    }

    /// Returns the first day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
    ///   supported date range.
    /// - [`CalendarError::Zone`] if a date-time point’s boundary datetime
    ///   cannot be resolved.
    pub(crate) fn start_of(
        self,
        calendar_period: CalendarPeriod,
    ) -> Result<Self, CalendarError> {
        self.boundary(calendar_period, PeriodEdge::Start)
    }

    /// Returns the last day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
    ///   supported date range.
    /// - [`CalendarError::Zone`] if a date-time point’s boundary datetime
    ///   cannot be resolved.
    pub(crate) fn end_of(
        self,
        calendar_period: CalendarPeriod,
    ) -> Result<Self, CalendarError> {
        self.boundary(calendar_period, PeriodEdge::End)
    }

    /// Selects a weekday of this date point's ISO week.
    pub(crate) fn with_weekday(
        self,
        target: civil::Weekday,
    ) -> Result<Self, CalendarError> {
        let local_date = self.local_datetime_or_utc().date();
        let iso = local_date.iso_week_date();
        let target_date =
            civil::ISOWeekDate::new(iso.year(), iso.week(), target)
                .map(civil::Date::from)
                .map_err(|_| CalendarError::OutOfRange)?;
        self.with_date(target_date)
    }

    /// Returns one calendar or clock component of the local datetime reading.
    /// Weekdays are ISO numbered, Monday = 1 through Sunday = 7.
    #[must_use]
    pub(crate) fn component(self, component: DateComponent) -> i64 {
        let local_datetime = self.local_datetime_or_utc();
        match component {
            DateComponent::Year => i64::from(local_datetime.year()),
            DateComponent::Month => i64::from(local_datetime.month()),
            DateComponent::Day => i64::from(local_datetime.day()),
            DateComponent::Hour => i64::from(local_datetime.hour()),
            DateComponent::Minute => i64::from(local_datetime.minute()),
            DateComponent::Second => i64::from(local_datetime.second()),
            DateComponent::Weekday => {
                i64::from(match local_datetime.weekday() {
                    civil::Weekday::Monday => 1,
                    civil::Weekday::Tuesday => 2,
                    civil::Weekday::Wednesday => 3,
                    civil::Weekday::Thursday => 4,
                    civil::Weekday::Friday => 5,
                    civil::Weekday::Saturday => 6,
                    civil::Weekday::Sunday => 7,
                })
            }
            DateComponent::Week => {
                i64::from(local_datetime.iso_week_date().week())
            }
        }
    }

    /// Measures the difference to another date point in the requested unit.
    /// Whole units truncate toward zero when either value is a calendar point.
    pub(crate) fn diff(
        self,
        to: Self,
        unit: TimeUnit,
    ) -> Result<Difference, CalendarError> {
        let local_delta = || {
            to.local_datetime_or_utc()
                .duration_since(self.local_datetime_or_utc())
        };
        match unit {
            TimeUnit::Year => {
                Ok(Difference::Whole(self.date().whole_years_until(to.date())))
            }
            TimeUnit::Month => {
                Ok(Difference::Whole(self.date().whole_months_until(to.date())))
            }
            fixed if self.has_time() && to.has_time() => {
                let seconds = match (self, to) {
                    (Self::DateTime(from), Self::DateTime(to)) => {
                        to.duration_since(from).as_secs_f64()
                    }
                    _ => return Err(CalendarError::OutOfRange),
                };
                Ok(Difference::Fractional(seconds / fixed.nominal_seconds()))
            }
            TimeUnit::Week => {
                Ok(Difference::Whole(local_delta().as_secs() / 604_800))
            }
            TimeUnit::Day => {
                Ok(Difference::Whole(local_delta().as_secs() / 86_400))
            }
            TimeUnit::Hour => {
                Ok(Difference::Whole(local_delta().as_secs() / 3_600))
            }
            TimeUnit::Minute => {
                Ok(Difference::Whole(local_delta().as_secs() / 60))
            }
            TimeUnit::Second => Ok(Difference::Whole(local_delta().as_secs())),
            TimeUnit::Millisecond => i64::try_from(local_delta().as_millis())
                .map(Difference::Whole)
                .map_err(|_| CalendarError::OutOfRange),
        }
    }

    /// Advances on the local clock, preserving written precision.
    fn advance_local(self, span: Span) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance_span(span)?,
                date_precision,
            }),
            Self::DateTime(datetime) => {
                let shifted = datetime
                    .local_datetime()
                    .checked_add(span)
                    .map_err(|_| CalendarError::OutOfRange)?;
                Ok(Self::DateTime(DateTimeValue::from_local(shifted)?))
            }
        }
    }

    /// Advances by an exact Jiff span, preserving the point's date/time kind.
    fn advance_span(self, span: Span) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance_span(span)?,
                date_precision,
            }),
            Self::DateTime(datetime) => {
                Ok(Self::DateTime(datetime.checked_add_span(span)?))
            }
        }
    }

    /// Advances by exact elapsed time, preserving written precision.
    fn advance_exact(self, span: Span) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance_span(span)?,
                date_precision,
            }),
            Self::DateTime(datetime) => datetime
                .utc()
                .checked_add(span)
                .map(DateTimeValue::from_timestamp)
                .map(Self::DateTime)
                .map_err(|_| CalendarError::OutOfRange),
        }
    }

    /// Applies one duration component in its own frame.
    fn apply_component(
        self,
        component: DurationComponent,
    ) -> Result<Self, CalendarError> {
        let (magnitude, unit) = (component.magnitude(), component.unit());
        if !magnitude.is_finite() {
            return Err(CalendarError::OutOfRange);
        }
        if unit.is_calendar() {
            let whole =
                magnitude.trunc().to_i64().ok_or(CalendarError::OutOfRange)?;
            let shifted = if whole != 0 {
                self.shift(whole, unit)?
            } else {
                self
            };
            let remainder = magnitude.fract() * unit.nominal_seconds();
            if remainder == 0.0 {
                Ok(shifted)
            } else {
                shifted.advance_local(span_from_seconds(remainder)?)
            }
        } else {
            self.advance_exact(span_from_seconds(
                magnitude * unit.nominal_seconds(),
            )?)
        }
    }

    /// Replaces the local date, preserving written precision and time.
    fn with_date(self, date: civil::Date) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date_precision,
                ..
            } => Ok(Self::Calendar {
                date: DateValue::from_civil_date(date),
                date_precision,
            }),
            Self::DateTime(_) => {
                let local = self.local_datetime_or_utc();
                let local_datetime = date.to_datetime(local.time());
                Ok(Self::DateTime(DateTimeValue::from_local(local_datetime)?))
            }
        }
    }

    /// Resolves a period edge to its local date, preserving precision.
    fn boundary(
        self,
        calendar_period: CalendarPeriod,
        period_edge: PeriodEdge,
    ) -> Result<Self, CalendarError> {
        let date = self.local_datetime_or_utc().date();
        let new_date = match calendar_period {
            CalendarPeriod::Week => {
                let iso = date.iso_week_date();
                let target = if period_edge == PeriodEdge::Start {
                    civil::Weekday::Monday
                } else {
                    civil::Weekday::Sunday
                };
                civil::ISOWeekDate::new(iso.year(), iso.week(), target)
                    .map(civil::Date::from)
            }
            CalendarPeriod::Month => civil::Date::new(
                date.year(),
                date.month(),
                if period_edge == PeriodEdge::Start {
                    1
                } else {
                    date.days_in_month()
                },
            ),
            CalendarPeriod::Year => civil::Date::new(
                date.year(),
                if period_edge == PeriodEdge::Start {
                    1
                } else {
                    12
                },
                if period_edge == PeriodEdge::Start {
                    1
                } else {
                    31
                },
            ),
        }
        .map_err(|_| CalendarError::OutOfRange)?;
        self.with_date(new_date)
    }

    /// Parses a date or date-time shape, retaining date precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if the input does not begin with two or
    ///   four ASCII digits followed by `-`, or if no accepted shape matches.
    /// - [`DateError::YearNotFourDigits`] if it starts that way but the year is
    ///   not four ASCII digits.
    /// - [`DateError::LocalZoneLookup`] if a naive date-time cannot be resolved
    ///   in the local zone.
    pub(crate) fn parse(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_like_date(trimmed.as_bytes()) {
            return Err(DateError::InvalidValue {
                input: trimmed.into(),
                reason: "not a date or date-time shape",
            });
        }
        if !starts_with_year(trimmed) {
            return Err(DateError::YearNotFourDigits {
                input: trimmed.into(),
            });
        }
        let calendar_point = |(date, date_precision)| Self::Calendar {
            date: DateValue::from(date),
            date_precision,
        };
        // Date-only shapes cannot match the date-time cascade.
        if trimmed.len() <= 10 {
            return DateShape::parse_any(trimmed).map(calendar_point).map_err(
                |reason| DateError::InvalidValue {
                    input: trimmed.into(),
                    reason,
                },
            );
        }
        match DateTimeShape::parse_any(trimmed) {
            Ok(ParsedDateTime::Instant(instant)) => {
                Ok(Self::DateTime(DateTimeValue::from_timestamp(instant)))
            }
            Ok(ParsedDateTime::Unzoned(local_datetime)) => Ok(Self::DateTime(
                DateTimeValue::from_local(local_datetime)
                    .map_err(|error| error.into_date_error(trimmed))?,
            )),
            Err(datetime_err) => match DateShape::parse_any(trimmed) {
                Ok(date) => Ok(calendar_point(date)),
                Err(date_err) => {
                    let reason = if matches!(
                        trimmed.as_bytes().get(10),
                        Some(b'T' | b' ')
                    ) {
                        datetime_err
                    } else {
                        date_err
                    };
                    Err(DateError::InvalidValue {
                        input: trimmed.into(),
                        reason,
                    })
                }
            },
        }
    }

    /// Parses a 10-byte `YYYY-MM-DD` or 7-byte `YYYY-MM` date prefix of `s`.
    ///
    /// Returns the date point and the number of bytes consumed, or [`None`]
    /// when `s` does not start with either shape.
    #[must_use]
    pub(crate) fn parse_prefix(s: &str) -> Option<crate::Spanned<Self>> {
        crate::lexer::lex_prefix(s, Self::parse_prefix_raw)
    }

    fn parse_prefix_raw(s: &str) -> Option<(Self, usize)> {
        let bytes = s.as_bytes();
        if bytes.len() >= 8 && bytes.get(7) == Some(&b'-') {
            if let Some(prefix) = s.get(..10)
                && let Ok(
                    date_point @ Self::Calendar {
                        date_precision: DatePrecision::Day,
                        ..
                    },
                ) = Self::parse(prefix)
            {
                return Some((date_point, 10));
            }
            return None;
        }
        if let Some(prefix) = s.get(..7)
            && let Ok(
                date_point @ Self::Calendar {
                    date_precision: DatePrecision::Month,
                    ..
                },
            ) = Self::parse(prefix)
        {
            return Some((date_point, 7));
        }
        None
    }

    /// Parses a date or date-time with a Jiff strtime pattern.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `text` matches `pattern` neither as an
    ///   offset date-time, a naive date-time, nor a date.
    /// - [`DateError::LocalZoneLookup`] if a naive date-time match cannot be
    ///   resolved in the local zone.
    pub(crate) fn parse_with_pattern(
        text: &str,
        pattern: &str,
    ) -> Result<Self, DateError> {
        let trimmed_text = text.trim();
        let trimmed_pattern = pattern.trim();
        let has_time_fields = pattern_has_time_fields(trimmed_pattern)
            .ok_or_else(|| DateError::InvalidValue {
                input: trimmed_text.into(),
                reason: "bad format pattern",
            })?;
        if let Ok(timestamp) =
            Timestamp::strptime(trimmed_pattern, trimmed_text)
        {
            return Ok(Self::DateTime(DateTimeValue::from_timestamp(
                timestamp,
            )));
        }
        if has_time_fields
            && let Ok(local_datetime) =
                civil::DateTime::strptime(trimmed_pattern, trimmed_text)
        {
            return Ok(Self::DateTime(
                DateTimeValue::from_local(local_datetime)
                    .map_err(|error| error.into_date_error(trimmed_text))?,
            ));
        }
        if trimmed_pattern == "%Y-%m"
            && let Ok(
                date_point @ Self::Calendar {
                    date_precision: DatePrecision::Month,
                    ..
                },
            ) = Self::parse(trimmed_text)
        {
            return Ok(date_point);
        }
        let date = civil::Date::strptime(trimmed_pattern, trimmed_text)
            .map_err(|source| DateError::InvalidValue {
                input: trimmed_text.into(),
                reason: if source.is_range() {
                    "a field is out of range"
                } else {
                    "invalid input"
                },
            })?;
        Ok(Self::Calendar {
            date: DateValue::from_civil_date(date),
            date_precision: DatePrecision::Day,
        })
    }
}
fn pattern_has_time_fields(pattern: &str) -> Option<bool> {
    let mut characters = pattern.chars().peekable();
    let mut has_time_fields = false;
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }
        let Some(mut directive) = characters.next() else {
            break;
        };
        if directive == '%' {
            continue;
        }
        while matches!(directive, '-' | '_' | '0' | '^' | '#' | ':' | 'E' | 'O')
            || directive.is_ascii_digit()
        {
            let Some(next) = characters.next() else {
                return Some(has_time_fields);
            };
            directive = next;
        }
        if directive == '.' {
            while characters.peek().is_some_and(char::is_ascii_digit) {
                let _ = characters.next();
            }
            let Some(next) = characters.next() else {
                break;
            };
            directive = next;
        }
        if directive == 'Q' {
            return None;
        }
        if matches!(
            directive,
            'H' | 'I'
                | 'k'
                | 'l'
                | 'M'
                | 'S'
                | 'f'
                | 'T'
                | 'R'
                | 'r'
                | 'X'
                | 'c'
                | 'p'
                | 's'
                | 'Z'
                | 'z'
                | '+'
        ) {
            has_time_fields = true;
        }
    }
    Some(has_time_fields)
}

/// Returns `true` if `bytes` begins with a date-like prefix: two or four ASCII
/// digits followed by `-`.
fn starts_like_date(bytes: &[u8]) -> bool {
    bytes.len() >= 5
        && [2, 4].into_iter().any(|digits| {
            bytes
                .get(..digits)
                .is_some_and(|b| b.iter().all(u8::is_ascii_digit))
                && bytes.get(digits) == Some(&b'-')
        })
}

/// Returns `true` if the first four bytes of `s` are ASCII digits.
pub(super) fn starts_with_year(s: &str) -> bool {
    s.as_bytes().get(..4).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        temporal::fixtures::{fixed_datetime, ymd},
        test_support::ZoneGuard,
    };

    // ZoneGuard pins a Jiff time zone to the test thread without changing the
    // process TZ environment variable; repeated sets replace the override.

    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
    }

    mod parse {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::day("2026-07-29", DatePrecision::Day, ymd(2026, 7, 29))]
        #[case::month("2026-07", DatePrecision::Month, ymd(2026, 7, 1))]
        fn parses_a_date_shape_to_a_calendar_point(
            #[case] text: &str,
            #[case] expected_precision: DatePrecision,
            #[case] expected_date: DateValue,
        ) {
            // Arrange: `text` is a zone-free date shape.

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::Calendar {
                    date: expected_date,
                    date_precision: expected_precision,
                }),
                "{text:?} must parse as a calendar point at its written \
                 precision"
            );
        }
        #[test]
        fn rejects_unpadded_calendar_date() {
            // Arrange
            let input = "2026-1-1";

            // Act
            let result = DatePoint::parse(input);

            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "input is too short",
                })
            );
        }

        #[rstest]
        #[case::t_with_seconds("2026-07-29T14:30:00", "2026-07-29T14:30:00Z")]
        #[case::t_with_minutes("2026-07-29T14:30", "2026-07-29T14:30:00Z")]
        #[case::space_with_seconds(
            "2026-07-29 14:30:00",
            "2026-07-29T14:30:00Z"
        )]
        #[case::space_with_minutes("2026-07-29 14:30", "2026-07-29T14:30:00Z")]
        #[case::space_with_fraction(
            "2026-07-29 14:30:00.123",
            "2026-07-29T14:30:00.123Z"
        )]
        #[case::rfc_3339_z("2026-07-29T14:30:00Z", "2026-07-29T14:30:00Z")]
        #[case::rfc_3339_offset(
            "2026-07-29T14:30:00+02:00",
            "2026-07-29T12:30:00Z"
        )]
        fn parses_a_date_time_shape_to_a_date_time_point(
            #[case] text: &str,
            #[case] expected: &str,
        ) {
            // Arrange: naive shapes resolve in the local zone.
            ZoneGuard::set("UTC");
            let expected =
                DateTimeValue::parse_iso(expected).expect("valid date-time");

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::DateTime(expected)),
                "{text:?} must parse as a date-time point"
            );
        }

        #[test]
        fn trims_surrounding_whitespace() {
            // Arrange: whitespace around a day-precision date.
            let text = "  2026-07-29  ";

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::Calendar {
                    date: ymd(2026, 7, 29),
                    date_precision: DatePrecision::Day,
                }),
                "surrounding whitespace must not defeat shape recognition"
            );
        }

        #[rstest]
        #[case::word("hello")]
        #[case::empty("")]
        #[case::blank("  ")]
        #[case::year_only("2026")]
        #[case::non_digit_year("abcd-01-01")]
        #[case::slashes("2026/08/22")]
        #[case::five_digit_year("20265-01-01")]
        fn rejects_text_without_a_date_shape(#[case] text: &str) {
            // Arrange: `text` lacks the two-or-four-digit year and dash prefix
            // that marks a date-like shape.

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert!(matches!(
                parsed,
                Err(DateError::InvalidValue {
                    input,
                    reason: "not a date or date-time shape",
                }) if input.as_ref() == text.trim()
            ));
        }

        #[test]
        fn rejects_a_two_digit_year() {
            // Arrange: a two-digit year passes the shape guard but fails the
            // four-digit year rule.
            let text = "26-08-22";

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert!(
                matches!(parsed, Err(DateError::YearNotFourDigits { .. })),
                "expected YearNotFourDigits, got {parsed:?}"
            );
        }

        #[rstest]
        #[case::month_out_of_range("2026-13", "a field is out of range")]
        #[case::impossible_february_day(
            "2026-02-30",
            "a field is out of range"
        )]
        #[case::impossible_april_day("2026-04-31", "a field is out of range")]
        #[case::signed_month("2026-+7", "not enough input")]
        #[case::trailing_text_after_a_date_time(
            "2026-07-29T14:30:00 junk",
            "invalid input"
        )]
        #[case::trailing_word_after_the_separator(
            "2026-07-29Tinvalid",
            "invalid input"
        )]
        #[case::hour_twenty_four("2026-07-29T24:00:00", "invalid input")]
        fn rejects_an_invalid_field_with_the_mapped_reason(
            #[case] text: &str,
            #[case] expected_reason: &str,
        ) {
            // Arrange: `text` matches a recognized shape but carries an invalid
            // field.

            // Act
            let parsed = DatePoint::parse(text);

            // Assert
            assert!(
                matches!(
                    parsed,
                    Err(DateError::InvalidValue { reason, .. })
                        if reason == expected_reason
                ),
                "expected reason {expected_reason:?}, got {parsed:?}"
            );
        }

        #[test]
        fn renders_the_parse_failure_in_the_error_message() {
            // Arrange: the rendered message is the CLI-facing contract.
            let text = "2026-13";

            // Act
            let error = DatePoint::parse(text).expect_err("an invalid date");

            // Assert
            assert_eq!(
                error.to_string(),
                "`2026-13` is not a recognized date/time: a field is out of \
                 range"
            );
        }
    }

    mod parse_prefix {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::full_day(
            "2026-07-29",
            DatePrecision::Day,
            ymd(2026, 7, 29),
            10
        )]
        #[case::day_before_a_date_time(
            "2026-07-29T14:30:00",
            DatePrecision::Day,
            ymd(2026, 7, 29),
            10
        )]
        #[case::day_before_trailing_text(
            "2026-07-29 rest",
            DatePrecision::Day,
            ymd(2026, 7, 29),
            10
        )]
        #[case::day_before_a_bare_separator(
            "2026-07-29T",
            DatePrecision::Day,
            ymd(2026, 7, 29),
            10
        )]
        #[case::full_month("2026-07", DatePrecision::Month, ymd(2026, 7, 1), 7)]
        #[case::month_before_trailing_text(
            "2026-07 remainder",
            DatePrecision::Month,
            ymd(2026, 7, 1),
            7
        )]
        #[case::month_before_a_digit(
            "2026-0729",
            DatePrecision::Month,
            ymd(2026, 7, 1),
            7
        )]
        fn returns_written_precision_with_its_extent(
            #[case] text: &str,
            #[case] expected_precision: DatePrecision,
            #[case] expected_date: DateValue,
            #[case] expected_extent: usize,
        ) {
            // Arrange: the input begins with a complete day or month prefix.

            // Act
            let parsed = DatePoint::parse_prefix(text);

            // Assert
            assert!(
                matches!(
                    parsed,
                    Some(parsed)
                        if *parsed.value() == DatePoint::Calendar {
                            date: expected_date,
                            date_precision: expected_precision,
                        }
                            && parsed.span_usize() == (0..expected_extent)
                ),
                "{text:?} must yield its written precision and exact extent"
            );
        }

        #[rstest]
        #[case::day_truncated_at_the_separator("2026-07-2")]
        #[case::day_truncated_mid_field("2026-07-3")]
        #[case::word("not a date")]
        #[case::two_digit_year("26-07 rest")]
        #[case::five_digit_year("20265-01-01")]
        #[case::invalid_month_and_day("2026-13-99 x")]
        #[case::non_digit_month("2026-0a-29")]
        #[case::month_out_of_range("1234-56-78 rest")]
        fn returns_none_without_a_complete_date_prefix(#[case] text: &str) {
            // Arrange: the input lacks a complete ten-byte day or seven-byte
            // month prefix.

            // Act
            let parsed = DatePoint::parse_prefix(text);

            // Assert
            assert_eq!(parsed, None, "{text:?} must not yield a date prefix");
        }
    }

    mod parse_with_pattern {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::iso_date("2026-07-29", "%Y-%m-%d", ymd(2026, 7, 29))]
        #[case::unpadded_fields("2026-1-1", "%Y-%m-%d", ymd(2026, 1, 1))]
        #[case::swapped_order("29/07/2026", "%d/%m/%Y", ymd(2026, 7, 29))]
        #[case::month_name("July 29, 2026", "%B %d, %Y", ymd(2026, 7, 29))]
        #[case::abbreviated_month_name(
            "Jul 29, 2026",
            "%b %d, %Y",
            ymd(2026, 7, 29)
        )]
        #[case::weekday_name(
            "Wednesday, July 29, 2026",
            "%A, %B %d, %Y",
            ymd(2026, 7, 29)
        )]
        #[case::iso_week_date("2026-W01-4", "%G-W%V-%u", ymd(2026, 1, 1))]
        #[case::iso_week_date_at_a_year_edge(
            "2020-W53-4",
            "%G-W%V-%u",
            ymd(2020, 12, 31)
        )]
        fn parses_a_date_pattern_to_a_calendar_point(
            #[case] text: &str,
            #[case] pattern: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange: `pattern` matches a zone-free date.

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{text:?} with {pattern:?} must parse as a calendar point"
            );
        }

        #[rstest]
        #[case::seconds(
            "2026-07-29 14:30:00",
            "%Y-%m-%d %H:%M:%S",
            "2026-07-29T14:30:00Z"
        )]
        #[case::minutes(
            "2026-07-29T14:30",
            "%Y-%m-%dT%H:%M",
            "2026-07-29T14:30:00Z"
        )]
        #[case::hour_12_clock(
            "2026-07-29 02:30:00 PM",
            "%Y-%m-%d %I:%M:%S %p",
            "2026-07-29T14:30:00Z"
        )]
        #[case::fractional_digits(
            "2026-07-29 14:30:00.123",
            "%Y-%m-%d %H:%M:%S.%3f",
            "2026-07-29T14:30:00.123Z"
        )]
        #[case::fractional_digits_fixed(
            "2026-07-29 14:30:00.123",
            "%Y-%m-%d %H:%M:%S%.3f",
            "2026-07-29T14:30:00.123Z"
        )]
        fn parses_a_date_time_pattern_to_a_date_time_point(
            #[case] text: &str,
            #[case] pattern: &str,
            #[case] expected: &str,
        ) {
            // Arrange: naive patterns resolve in the local zone.
            ZoneGuard::set("UTC");
            let expected =
                DateTimeValue::parse_iso(expected).expect("valid date-time");

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::DateTime(expected)),
                "{text:?} with {pattern:?} must parse as a date-time point"
            );
        }

        #[test]
        fn preserves_month_precision_for_the_year_month_pattern() {
            // Arrange: the `%Y-%m` pattern keeps the written month precision.
            let text = "2026-07";
            let pattern = "%Y-%m";

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert_eq!(
                parsed,
                Ok(DatePoint::Calendar {
                    date: ymd(2026, 7, 1),
                    date_precision: DatePrecision::Month,
                }),
                "the year-month pattern must preserve month precision"
            );
        }

        #[rstest]
        #[case::inconsistent_weekday(
            "Thursday, July 29, 2026",
            "%A, %B %d, %Y",
            "invalid input"
        )]
        #[case::inconsistent_abbreviated_weekday(
            "Thu, July 29, 2026",
            "%a, %B %d, %Y",
            "invalid input"
        )]
        #[case::zero_padded_weekday(
            "2026-W01-04",
            "%G-W%V-%u",
            "a field is out of range"
        )]
        #[case::unsupported_padding_flag("2026-07", "%Y-%-m", "invalid input")]
        fn rejects_a_pattern_mismatch_with_the_field_reason(
            #[case] text: &str,
            #[case] pattern: &str,
            #[case] expected_reason: &str,
        ) {
            // Arrange: `text` conflicts with the field semantics of `pattern`.

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert!(
                matches!(
                    parsed,
                    Err(DateError::InvalidValue { reason, .. })
                        if reason == expected_reason
                ),
                "expected reason {expected_reason:?}, got {parsed:?}"
            );
        }

        #[rstest]
        #[case::capital_e("2026-07-29", "%E")]
        #[case::capital_o("2026-07-29", "%O")]
        #[case::bare_dot("2026-07-29", "%.")]
        #[case::bare_underscore("2026-07-29", "%_")]
        #[case::fractional_q(
            "2026-07-29 14:30:00.123456789",
            "%Y-%m-%d %H:%M:%S%Q"
        )]
        fn rejects_an_unknown_specifier_without_panicking(
            #[case] text: &str,
            #[case] pattern: &str,
        ) {
            // Arrange: `pattern` carries a specifier the engine cannot parse.

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert!(
                matches!(parsed, Err(DateError::InvalidValue { .. })),
                "an invalid specifier must fail without panicking, got \
                 {parsed:?}"
            );
        }

        #[rstest]
        #[case::not_a_date("not-a-date", "%Y-%m-%d", "invalid input")]
        #[case::swapped_pattern("2026-07-29", "%d/%m/%Y", "invalid input")]
        fn rejects_text_that_cannot_match_the_pattern(
            #[case] text: &str,
            #[case] pattern: &str,
            #[case] expected_reason: &str,
        ) {
            // Arrange: `text` matches `pattern` in no parser of the cascade.

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert!(
                matches!(
                    parsed,
                    Err(DateError::InvalidValue { reason, .. })
                        if reason == expected_reason
                ),
                "expected reason {expected_reason:?}, got {parsed:?}"
            );
        }
    }

    mod shift {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::day_forward(1, TimeUnit::Day, ymd(2026, 7, 30))]
        #[case::day_backward(-1, TimeUnit::Day, ymd(2026, 7, 28))]
        #[case::week_backward(-1, TimeUnit::Week, ymd(2026, 7, 22))]
        #[case::year_backward(-1, TimeUnit::Year, ymd(2025, 7, 29))]
        fn shifts_a_calendar_date_by_whole_units(
            #[case] n: i64,
            #[case] unit: TimeUnit,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at("2026-07-29");

            // Act
            let shifted = base.shift(n, unit);

            // Assert
            assert_eq!(
                shifted,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{n} {unit:?} must shift the calendar date"
            );
        }

        #[rstest]
        #[case::five_hours(5, "2026-07-29")]
        #[case::just_under_a_day(23, "2026-07-29")]
        #[case::one_day(24, "2026-07-30")]
        #[case::over_a_day(25, "2026-07-30")]
        #[case::a_day_and_a_half(36, "2026-07-30")]
        fn moves_a_calendar_date_only_when_the_shift_crosses_midnight(
            #[case] hours: i64,
            #[case] expected: &str,
        ) {
            // Arrange
            let base = point_at("2026-07-29");
            let expected = point_at(expected);

            // Act
            let shifted = base.shift(hours, TimeUnit::Hour);

            // Assert
            assert_eq!(
                shifted,
                Ok(expected),
                "{hours} exact hours must move the calendar date only when \
                 the shift crosses midnight"
            );
        }

        #[rstest]
        #[case::non_leap_year("2023-01-31", ymd(2023, 2, 28))]
        #[case::leap_year("2024-01-31", ymd(2024, 2, 29))]
        fn clamps_a_month_shift_to_the_last_day_of_february(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let shifted = base.shift(1, TimeUnit::Month);

            // Assert
            assert_eq!(
                shifted,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "a month shift must clamp to the target month's last day and \
                 keep day precision"
            );
        }

        #[test]
        fn shifts_by_a_whole_month_magnitude_in_one_step() {
            // Arrange: one-step month arithmetic must not clamp incrementally.
            let base = point_at("2026-01-31");

            // Act
            let shifted = base.shift(2, TimeUnit::Month);

            // Assert
            assert_eq!(
                shifted,
                Ok(point_at("2026-03-31")),
                "a magnitude of two months must not clamp at the intermediate \
                 month end"
            );
        }

        #[rstest]
        #[case::month(1, TimeUnit::Month, ymd(2026, 8, 1))]
        #[case::day(1, TimeUnit::Day, ymd(2026, 7, 2))]
        fn preserves_month_precision_through_a_shift(
            #[case] n: i64,
            #[case] unit: TimeUnit,
            #[case] expected_date: DateValue,
        ) {
            // Arrange
            let base = point_at("2026-07");

            // Act
            let shifted = base.shift(n, unit);

            // Assert
            assert_eq!(
                shifted,
                Ok(DatePoint::Calendar {
                    date: expected_date,
                    date_precision: DatePrecision::Month,
                }),
                "{n} {unit:?} must move the date and keep month precision"
            );
        }

        #[test]
        fn keeps_the_local_clock_time_across_a_spring_forward_gap() {
            // Arrange: Saturday 2026-03-07 12:00 EST; the gap begins Sunday.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-03-07 12:00:00");
            let clock = |point: Result<DatePoint, CalendarError>| {
                point.map(|p| p.local_datetime_or_utc().to_string())
            };

            // Act
            let day = clock(base.shift(1, TimeUnit::Day));
            let week = clock(base.shift(1, TimeUnit::Week));
            let month = clock(base.shift(1, TimeUnit::Month));
            let year = clock(base.shift(1, TimeUnit::Year));
            let back = clock(base.shift(-1, TimeUnit::Day));

            // Assert
            assert_eq!(
                day,
                Ok("2026-03-08T12:00:00".to_owned()),
                "a one-day shift must keep the local clock time across the gap"
            );
            assert_eq!(
                week,
                Ok("2026-03-14T12:00:00".to_owned()),
                "a one-week shift must keep the local clock time"
            );
            assert_eq!(
                month,
                Ok("2026-04-07T12:00:00".to_owned()),
                "a one-month shift must keep the local clock time"
            );
            assert_eq!(
                year,
                Ok("2027-03-07T12:00:00".to_owned()),
                "a one-year shift must keep the local clock time"
            );
            assert_eq!(
                back,
                Ok("2026-03-06T12:00:00".to_owned()),
                "a backward day shift must keep the local clock time"
            );
        }

        #[test]
        fn keeps_the_local_clock_time_across_a_fall_back_fold() {
            // Arrange: Saturday 2026-10-31 12:00 EDT; the fold begins Sunday.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-10-31 12:00:00");

            // Act
            let day = base.shift(1, TimeUnit::Day);

            // Assert
            assert_eq!(
                day.map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-11-01T12:00:00".to_owned()),
                "a one-day shift must keep the local clock time across the \
                 fold"
            );
        }

        #[test]
        fn shifts_exact_units_by_elapsed_time_across_dst_transitions() {
            // Arrange: the spring gap and fall fold of America/New_York.
            ZoneGuard::set("America/New_York");
            let spring = point_at("2026-03-07 12:00:00");
            let fall = point_at("2026-10-31 12:00:00");

            // Act
            let spring_hours = spring.shift(168, TimeUnit::Hour);
            let spring_week = spring.shift(1, TimeUnit::Week);
            let spring_days = spring.shift(7, TimeUnit::Day);
            let fall_day = fall.shift(1, TimeUnit::Day);
            let fall_hours = fall.shift(24, TimeUnit::Hour);

            // Assert
            assert_eq!(
                spring_hours
                    .as_ref()
                    .map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-14T13:00:00".to_owned()),
                "168 exact hours must cross the gap as elapsed time"
            );
            assert_eq!(
                spring_week, spring_days,
                "a calendar week must equal seven calendar days"
            );
            assert_ne!(
                spring_week, spring_hours,
                "a calendar week must differ from 168 exact hours across the \
                 gap"
            );
            assert_eq!(
                fall_day.map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-11-01T12:00:00".to_owned()),
                "a calendar day must keep the local clock time across the fold"
            );
            assert_eq!(
                fall_hours.map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-11-01T11:00:00".to_owned()),
                "24 exact hours must cross the fold as elapsed time"
            );
        }

        #[test]
        fn reports_out_of_range_past_the_representable_edges() {
            // Arrange: points one unit inside each representable edge.
            ZoneGuard::set("UTC");
            let max_date = DatePoint::Calendar {
                date: DateValue::from_ymd(9999, 12, 31)
                    .expect("maximum Jiff date is valid"),
                date_precision: DatePrecision::Day,
            };
            let min_date = DatePoint::Calendar {
                date: DateValue::from_ymd(-9999, 1, 1)
                    .expect("minimum Jiff date is valid"),
                date_precision: DatePrecision::Day,
            };
            let max_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MAX,
            ));
            let min_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MIN,
            ));

            // Act
            let past_max_date = max_date.shift(1, TimeUnit::Day);
            let past_min_date = min_date.shift(-1, TimeUnit::Day);
            let past_max_point = max_point.shift(1, TimeUnit::Hour);
            let past_min_point = min_point.shift(-1, TimeUnit::Hour);

            // Assert
            assert_eq!(
                past_max_date,
                Err(CalendarError::OutOfRange),
                "shifting past the maximum date must report out of range"
            );
            assert_eq!(
                past_min_date,
                Err(CalendarError::OutOfRange),
                "shifting before the minimum date must report out of range"
            );
            assert_eq!(
                past_max_point,
                Err(CalendarError::OutOfRange),
                "shifting past the maximum date-time must report out of range"
            );
            assert_eq!(
                past_min_point,
                Err(CalendarError::OutOfRange),
                "shifting before the minimum date-time must report out of \
                 range"
            );
        }
    }

    mod apply {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn applies_compound_components_in_written_order() {
            // Arrange
            let base = point_at("2026-07-29");
            let duration =
                DurationValue::parse("1w 2d").expect("valid duration");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(point_at("2026-08-07")),
                "\"1w 2d\" must apply in written order and keep day precision"
            );
        }

        #[rstest]
        #[case::negative_day("-1d", ymd(2026, 7, 28))]
        #[case::fractional_day("1.5d", ymd(2026, 7, 30))]
        fn applies_signed_day_durations_to_a_calendar_date(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at("2026-07-29");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{text:?} must apply to the calendar date"
            );
        }

        #[rstest]
        #[case::repeated_exact_unit("12h 12h", ymd(2026, 7, 29))]
        #[case::single_exact_unit("24h", ymd(2026, 7, 30))]
        #[case::over_a_day("36h", ymd(2026, 7, 30))]
        fn applies_each_exact_component_from_local_midnight(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange: repeated units are accepted, and each sub-day component
            // restarts from the current midnight.
            let base = point_at("2026-07-29");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{text:?} must move the date only when a component crosses \
                 midnight"
            );
        }
        #[rstest]
        #[case::under_a_day(82_800.0, ymd(2026, 7, 29))]
        #[case::one_day(86_400.0, ymd(2026, 7, 30))]
        #[case::negative_day(-86_400.0, ymd(2026, 7, 28))]
        fn applies_synthesized_exact_spans_to_calendar_dates(
            #[case] seconds: f64,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at("2026-07-29");
            let duration = DurationValue::from_nominal_seconds(
                crate::temporal::DurationSeconds::try_from(seconds)
                    .expect("finite seconds"),
            )
            .expect("duration fits within the Jiff span range");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{seconds} exact seconds must advance from midnight and keep \
                 `Date` precision"
            );
        }

        #[rstest]
        #[case::sequential("1mo 1mo", ymd(2026, 3, 28))]
        #[case::one_step("2mo", ymd(2026, 3, 31))]
        fn applies_month_components_sequentially(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange: written order clamps month ends at each step, unlike a
            // single two-month shift.
            let base = point_at("2026-01-31");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{text:?} must apply month components in written order"
            );
        }

        #[rstest]
        #[case::forward("1h", "2026-07-29T15:30:05Z")]
        #[case::backward("-1h", "2026-07-29T13:30:05Z")]
        fn applies_signed_durations_to_a_date_time_point(
            #[case] text: &str,
            #[case] expected: &str,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let base = DatePoint::DateTime(fixed_datetime());
            let duration = DurationValue::parse(text).expect("valid duration");
            let expected = point_at(expected);

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(expected),
                "{text:?} must apply to the stored date-time"
            );
        }

        #[test]
        fn applies_synthesized_nominal_seconds_exactly() {
            // Arrange: a duration rebuilt from nominal seconds carries no
            // written components, so it advances the exact time.
            ZoneGuard::set("UTC");
            let base = DatePoint::DateTime(fixed_datetime());
            let synthesized = DurationValue::from_nominal_seconds(
                crate::temporal::DurationSeconds::try_from(3600.0)
                    .expect("finite seconds"),
            )
            .expect("one hour fits within the Jiff span range");

            // Act
            let applied = base.apply(&synthesized);

            // Assert
            assert_eq!(
                applied,
                Ok(point_at("2026-07-29T15:30:05Z")),
                "synthesized nominal seconds must advance the exact time"
            );
        }

        #[test]
        fn applies_equal_valued_units_differently_across_a_gap() {
            // Arrange: 1d equals 24h as values, but not across the spring gap.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-03-07 12:00:00");
            let one_day = DurationValue::parse("1d").expect("valid duration");
            let twenty_four_hours =
                DurationValue::parse("24h").expect("valid duration");
            assert_eq!(
                one_day, twenty_four_hours,
                "\"1d\" and \"24h\" must be equal as values"
            );

            // Act
            let by_calendar = base.apply(&one_day);
            let by_exact = base.apply(&twenty_four_hours);

            // Assert
            assert_eq!(
                by_calendar
                    .as_ref()
                    .map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-08T12:00:00".to_owned()),
                "the calendar day must keep the local clock time across the \
                 gap"
            );
            assert_eq!(
                by_exact
                    .as_ref()
                    .map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-08T13:00:00".to_owned()),
                "24 exact hours must cross the gap as elapsed time"
            );
            assert_ne!(
                by_calendar, by_exact,
                "equal values must apply differently across the gap"
            );
        }

        #[test]
        fn applies_a_calendar_week_differently_from_168_exact_hours_across_a_gap()
         {
            // Arrange: one week, 7d and 168h are equal as values.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-03-07 12:00:00");
            let one_week = DurationValue::parse("1w").expect("valid duration");
            let seven_days =
                DurationValue::parse("7d").expect("valid duration");
            let hours_168 =
                DurationValue::parse("168h").expect("valid duration");
            assert_eq!(one_week, seven_days, "\"1w\" and \"7d\" must be equal");
            assert_eq!(
                one_week, hours_168,
                "\"1w\" and \"168h\" must be equal as values"
            );

            // Act
            let by_week = base.apply(&one_week);
            let by_days = base.apply(&seven_days);
            let by_hours = base.apply(&hours_168);

            // Assert
            assert_eq!(
                by_week.as_ref().map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-14T12:00:00".to_owned()),
                "a calendar week must keep the local clock time"
            );
            assert_eq!(
                by_week, by_days,
                "\"1w\" and \"7d\" must apply identically"
            );
            assert_ne!(
                by_week, by_hours,
                "a calendar week must differ from 168 exact hours across the \
                 gap"
            );
        }

        #[test]
        fn applies_the_fractional_part_of_a_calendar_unit_on_the_local_clock() {
            // Arrange: 1.5d equals 36h equals \"1d 12h\" as values.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-03-07 06:00:00");
            let one_and_half_days =
                DurationValue::parse("1.5d").expect("valid duration");
            let thirty_six_hours =
                DurationValue::parse("36h").expect("valid duration");
            let written_order =
                DurationValue::parse("1d 12h").expect("valid duration");
            assert_eq!(
                one_and_half_days, thirty_six_hours,
                "\"1.5d\" and \"36h\" must be equal as values"
            );
            assert_eq!(
                one_and_half_days, written_order,
                "\"1.5d\" and \"1d 12h\" must be equal as values"
            );

            // Act
            let by_fraction = base.apply(&one_and_half_days);
            let by_written = base.apply(&written_order);
            let by_exact = base.apply(&thirty_six_hours);

            // Assert
            assert_eq!(
                by_fraction
                    .as_ref()
                    .map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-08T18:00:00".to_owned()),
                "the fractional day must keep the local clock time"
            );
            assert_eq!(
                by_fraction, by_written,
                "\"1.5d\" must decompose like its written-order equivalent"
            );
            assert_ne!(
                by_fraction, by_exact,
                "the fractional calendar part must differ from 36 exact hours \
                 across the gap"
            );
        }

        #[test]
        fn applies_written_components_left_to_right_across_a_gap() {
            // Arrange: 02:30 sits one hour before the spring gap opens.
            ZoneGuard::set("America/New_York");
            let base = point_at("2026-03-07 02:30:00");
            let day_then_hour =
                DurationValue::parse("1d 1h").expect("valid duration");
            let hour_then_day =
                DurationValue::parse("1h 1d").expect("valid duration");
            assert_eq!(
                day_then_hour, hour_then_day,
                "the written orders must be equal as values"
            );

            // Act
            let first = base.apply(&day_then_hour);
            let second = base.apply(&hour_then_day);

            // Assert
            assert_eq!(
                first,
                Ok(point_at("2026-03-08T08:30:00Z")),
                "\"1d 1h\" must apply the day first, then the hour"
            );
            assert_eq!(
                second,
                Ok(point_at("2026-03-08T07:30:00Z")),
                "\"1h 1d\" must apply the hour first, then the day"
            );
            assert_ne!(
                first, second,
                "written order must change the result across a gap"
            );
        }

        #[test]
        fn distinguishes_calendar_from_exact_application_for_equal_values() {
            // Arrange: \"1 month\" equals \"30 days\" as values.
            let base = point_at("2026-01-15");
            let one_month =
                DurationValue::parse("1 month").expect("valid duration");
            let thirty_days =
                DurationValue::parse("30 days").expect("valid duration");
            assert_eq!(
                one_month, thirty_days,
                "\"1 month\" and \"30 days\" must be equal as values"
            );

            // Act
            let by_month = base.apply(&one_month);
            let by_days = base.apply(&thirty_days);

            // Assert
            assert_eq!(
                by_month,
                Ok(point_at("2026-02-15")),
                "a calendar month must land on the same day of the next month"
            );
            assert_eq!(
                by_days,
                Ok(point_at("2026-02-14")),
                "thirty exact days must land thirty days later"
            );
            assert_ne!(
                by_month, by_days,
                "equal values must shift a date differently"
            );
        }

        #[test]
        fn applies_a_fractional_month_to_a_calendar_date() {
            // Arrange: the whole month clamps first, then the half-month
            // remainder (fifteen nominal days) applies as local time.
            let base = point_at("2026-01-31");
            let duration =
                DurationValue::parse("1.5mo").expect("valid duration");

            // Act
            let applied = base.apply(&duration);

            // Assert
            assert_eq!(
                applied,
                Ok(point_at("2026-03-15")),
                "1.5mo must clamp the month, then add the local remainder"
            );
        }

        #[test]
        fn reports_out_of_range_past_the_representable_edges() {
            // Arrange: the date-time range edges and a one-second duration.
            let one_second =
                DurationValue::parse("1s").expect("valid duration");
            let negative_one_second =
                DurationValue::parse("-1s").expect("valid duration");
            let max_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MAX,
            ));
            let min_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MIN,
            ));

            // Act
            let overflowed = max_point.apply(&one_second);
            let underflowed = min_point.apply(&negative_one_second);

            // Assert
            assert_eq!(
                overflowed,
                Err(CalendarError::OutOfRange),
                "applying past the maximum must report out of range"
            );
            assert_eq!(
                underflowed,
                Err(CalendarError::OutOfRange),
                "applying before the minimum must report out of range"
            );
        }
    }

    mod start_of {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::week("2026-02-01", CalendarPeriod::Week, ymd(2026, 1, 26))]
        #[case::month("2026-02-01", CalendarPeriod::Month, ymd(2026, 2, 1))]
        #[case::year("2026-02-01", CalendarPeriod::Year, ymd(2026, 1, 1))]
        #[case::week_across_a_year_edge(
            "2025-12-31",
            CalendarPeriod::Week,
            ymd(2025, 12, 29)
        )]
        #[case::week_from_a_year_start(
            "2026-01-01",
            CalendarPeriod::Week,
            ymd(2025, 12, 29)
        )]
        #[case::week_on_a_leap_day(
            "2024-02-29",
            CalendarPeriod::Week,
            ymd(2024, 2, 26)
        )]
        fn returns_the_first_day_of_the_period(
            #[case] text: &str,
            #[case] period: CalendarPeriod,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let start = base.start_of(period);

            // Assert
            assert_eq!(
                start,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{period:?} start must be the first day of the period"
            );
        }

        #[rstest]
        #[case::week("2026-07", CalendarPeriod::Week, ymd(2026, 6, 29))]
        #[case::month("2026-07", CalendarPeriod::Month, ymd(2026, 7, 1))]
        #[case::year("2026-07", CalendarPeriod::Year, ymd(2026, 1, 1))]
        fn preserves_month_precision(
            #[case] text: &str,
            #[case] period: CalendarPeriod,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let start = base.start_of(period);

            // Assert
            assert_eq!(
                start,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Month,
                }),
                "{period:?} start must keep the written month precision"
            );
        }

        #[rstest]
        #[case::week(CalendarPeriod::Week, "2026-07-27T14:30:05")]
        #[case::month(CalendarPeriod::Month, "2026-07-01T14:30:05")]
        #[case::year(CalendarPeriod::Year, "2026-01-01T14:30:05")]
        fn keeps_the_time_of_day_on_a_date_time_point(
            #[case] period: CalendarPeriod,
            #[case] expected: &str,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let base = point_at("2026-07-29 14:30:05");

            // Act
            let start = base.start_of(period);

            // Assert
            assert_eq!(
                start.map(|p| p.local_datetime_or_utc().to_string()),
                Ok(expected.to_owned()),
                "{period:?} start must keep the local time of day"
            );
        }
    }

    mod end_of {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::week("2026-02-01", CalendarPeriod::Week, ymd(2026, 2, 1))]
        #[case::month("2026-02-01", CalendarPeriod::Month, ymd(2026, 2, 28))]
        #[case::year("2026-02-01", CalendarPeriod::Year, ymd(2026, 12, 31))]
        #[case::week_across_a_year_edge(
            "2025-12-31",
            CalendarPeriod::Week,
            ymd(2026, 1, 4)
        )]
        #[case::week_from_a_year_start(
            "2026-01-01",
            CalendarPeriod::Week,
            ymd(2026, 1, 4)
        )]
        #[case::week_on_a_leap_day(
            "2024-02-29",
            CalendarPeriod::Week,
            ymd(2024, 3, 3)
        )]
        #[case::month_on_a_leap_day(
            "2024-02-29",
            CalendarPeriod::Month,
            ymd(2024, 2, 29)
        )]
        #[case::year_on_a_leap_day(
            "2024-02-29",
            CalendarPeriod::Year,
            ymd(2024, 12, 31)
        )]
        fn returns_the_last_day_of_the_period(
            #[case] text: &str,
            #[case] period: CalendarPeriod,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let end = base.end_of(period);

            // Assert
            assert_eq!(
                end,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{period:?} end must be the last day of the period"
            );
        }

        #[rstest]
        #[case::week("2026-07", CalendarPeriod::Week, ymd(2026, 7, 5))]
        #[case::month("2026-07", CalendarPeriod::Month, ymd(2026, 7, 31))]
        #[case::year("2026-07", CalendarPeriod::Year, ymd(2026, 12, 31))]
        fn preserves_month_precision(
            #[case] text: &str,
            #[case] period: CalendarPeriod,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let end = base.end_of(period);

            // Assert
            assert_eq!(
                end,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Month,
                }),
                "{period:?} end must keep the written month precision"
            );
        }

        #[rstest]
        #[case::week(CalendarPeriod::Week, "2026-08-02T14:30:05")]
        #[case::month(CalendarPeriod::Month, "2026-07-31T14:30:05")]
        #[case::year(CalendarPeriod::Year, "2026-12-31T14:30:05")]
        fn keeps_the_time_of_day_on_a_date_time_point(
            #[case] period: CalendarPeriod,
            #[case] expected: &str,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let base = point_at("2026-07-29 14:30:05");

            // Act
            let end = base.end_of(period);

            // Assert
            assert_eq!(
                end.map(|p| p.local_datetime_or_utc().to_string()),
                Ok(expected.to_owned()),
                "{period:?} end must keep the local time of day"
            );
        }
    }

    mod with_weekday {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::monday(civil::Weekday::Monday, ymd(2026, 7, 27))]
        #[case::tuesday(civil::Weekday::Tuesday, ymd(2026, 7, 28))]
        #[case::wednesday(civil::Weekday::Wednesday, ymd(2026, 7, 29))]
        #[case::thursday(civil::Weekday::Thursday, ymd(2026, 7, 30))]
        #[case::friday(civil::Weekday::Friday, ymd(2026, 7, 31))]
        #[case::saturday(civil::Weekday::Saturday, ymd(2026, 8, 1))]
        #[case::sunday(civil::Weekday::Sunday, ymd(2026, 8, 2))]
        fn selects_the_named_weekday_of_the_iso_week(
            #[case] target: civil::Weekday,
            #[case] expected: DateValue,
        ) {
            // Arrange: 2026-07-29 is a Wednesday.
            let base = point_at("2026-07-29");

            // Act
            let selected = base.with_weekday(target);

            // Assert
            assert_eq!(
                selected,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{target:?} must select the weekday of the ISO week"
            );
        }

        #[test]
        fn selects_backwards_across_a_week_edge() {
            // Arrange: Sunday 2026-08-02 belongs to the week that began on
            // Monday 2026-07-27, in the previous month.
            let base = point_at("2026-08-02");

            // Act
            let monday = base.with_weekday(civil::Weekday::Monday);

            // Assert
            assert_eq!(
                monday,
                Ok(point_at("2026-07-27")),
                "selecting Monday from a Sunday must move back across the \
                 week edge"
            );
        }

        #[test]
        fn preserves_month_precision() {
            // Arrange
            let base = point_at("2026-07");

            // Act
            let friday = base.with_weekday(civil::Weekday::Friday);

            // Assert
            assert_eq!(
                friday,
                Ok(DatePoint::Calendar {
                    date: ymd(2026, 7, 3),
                    date_precision: DatePrecision::Month,
                }),
                "the written month precision must survive the selection"
            );
        }

        #[test]
        fn keeps_the_time_of_day_on_a_date_time_point() {
            // Arrange
            ZoneGuard::set("UTC");
            let base = point_at("2026-07-29 14:30:05");

            // Act
            let sunday = base.with_weekday(civil::Weekday::Sunday);

            // Assert
            assert_eq!(
                sunday.map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-08-02T14:30:05".to_owned()),
                "the local time of day must survive the selection"
            );
        }
    }

    mod component {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::year("2026-07-29", DateComponent::Year, 2026)]
        #[case::month("2026-07-29", DateComponent::Month, 7)]
        #[case::day("2026-07-29", DateComponent::Day, 29)]
        #[case::hour("2026-07-29", DateComponent::Hour, 0)]
        #[case::minute("2026-07-29", DateComponent::Minute, 0)]
        #[case::second("2026-07-29", DateComponent::Second, 0)]
        #[case::weekday("2026-07-29", DateComponent::Weekday, 3)]
        #[case::week("2026-07-29", DateComponent::Week, 31)]
        fn returns_each_component_of_a_calendar_day(
            #[case] text: &str,
            #[case] component: DateComponent,
            #[case] expected: i64,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let value = base.component(component);

            // Assert
            assert_eq!(
                value, expected,
                "{component:?} of {text:?} must read from the local midnight"
            );
        }

        #[rstest]
        #[case::year_end_of_2020("2020-12-31", 53, 4)]
        #[case::first_sunday_of_2021("2021-01-03", 53, 7)]
        #[case::first_day_of_2026("2026-01-01", 1, 4)]
        fn returns_week_coordinates_at_year_edges(
            #[case] text: &str,
            #[case] expected_week: i64,
            #[case] expected_weekday: i64,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let week = base.component(DateComponent::Week);
            let weekday = base.component(DateComponent::Weekday);

            // Assert
            assert_eq!(
                week, expected_week,
                "the ISO week of {text:?} must span the year edge"
            );
            assert_eq!(
                weekday, expected_weekday,
                "weekdays are ISO numbered, Monday = 1 through Sunday = 7"
            );
        }

        #[rstest]
        #[case::day("2026-07", DateComponent::Day, 1)]
        #[case::week("2026-07", DateComponent::Week, 27)]
        #[case::weekday("2026-07", DateComponent::Weekday, 3)]
        fn reads_a_month_precision_point_as_the_first_of_its_month(
            #[case] text: &str,
            #[case] component: DateComponent,
            #[case] expected: i64,
        ) {
            // Arrange
            let base = point_at(text);

            // Act
            let value = base.component(component);

            // Assert
            assert_eq!(
                value, expected,
                "{component:?} of a month-precision point reads from the \
                 first of the month"
            );
        }

        #[rstest]
        #[case::hour("2026-07-29 14:30:05", DateComponent::Hour, 14)]
        #[case::minute("2026-07-29 14:30:05", DateComponent::Minute, 30)]
        #[case::second("2026-07-29 14:30:05", DateComponent::Second, 5)]
        #[case::week("2026-07-29 14:30:05", DateComponent::Week, 31)]
        #[case::weekday("2026-07-29 14:30:05", DateComponent::Weekday, 3)]
        fn returns_the_local_clock_components_of_a_date_time_point(
            #[case] text: &str,
            #[case] component: DateComponent,
            #[case] expected: i64,
        ) {
            // Arrange: naive input resolves in the local zone.
            ZoneGuard::set("UTC");
            let base = point_at(text);

            // Act
            let value = base.component(component);

            // Assert
            assert_eq!(
                value, expected,
                "{component:?} must read from the local clock rendering"
            );
        }
    }

    mod diff {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::year(
            "2020-06-15",
            "2026-07-15",
            TimeUnit::Year,
            Difference::Whole(6)
        )]
        #[case::month_across_month_ends(
            "2026-01-15",
            "2026-03-15",
            TimeUnit::Month,
            Difference::Whole(2)
        )]
        #[case::week(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Week,
            Difference::Whole(1)
        )]
        #[case::day(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Day,
            Difference::Whole(8)
        )]
        #[case::hour(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Hour,
            Difference::Whole(192)
        )]
        #[case::minute(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Minute,
            Difference::Whole(11520)
        )]
        #[case::second(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Second,
            Difference::Whole(691_200)
        )]
        #[case::millisecond(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Millisecond,
            Difference::Whole(691_200_000)
        )]
        fn returns_whole_counts_between_calendar_points(
            #[case] from: &str,
            #[case] to: &str,
            #[case] unit: TimeUnit,
            #[case] expected: Difference,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(to, unit);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "{unit:?} between calendar points must count whole units"
            );
        }

        #[rstest]
        #[case::month("2026-01-15", "2026-03-15", TimeUnit::Month, Difference::Whole(-2))]
        #[case::year("2020-06-15", "2026-07-15", TimeUnit::Year, Difference::Whole(-6))]
        #[case::day("2026-07-29", "2026-08-06", TimeUnit::Day, Difference::Whole(-8))]
        #[case::hour("2026-07-29", "2026-08-06", TimeUnit::Hour, Difference::Whole(-192))]
        fn returns_the_negated_difference_in_the_reverse_direction(
            #[case] from: &str,
            #[case] to: &str,
            #[case] unit: TimeUnit,
            #[case] expected: Difference,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = to.diff(from, unit);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "diff must be antisymmetric for {unit:?}"
            );
        }

        #[rstest]
        #[case::to_the_last_representable_day(
            "2026-01-01",
            "9999-12-31",
            Difference::Whole(7973)
        )]
        #[case::since_the_epoch(
            "1970-01-01",
            "2026-07-29",
            Difference::Whole(56)
        )]
        fn counts_whole_years_across_large_spans(
            #[case] from: &str,
            #[case] to: &str,
            #[case] expected: Difference,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(to, TimeUnit::Year);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "whole years must count across a large span"
            );
        }

        #[rstest]
        #[case::hour(TimeUnit::Hour, Difference::Fractional(1.5))]
        #[case::minute(TimeUnit::Minute, Difference::Fractional(90.0))]
        #[case::second(TimeUnit::Second, Difference::Fractional(5400.0))]
        #[case::millisecond(
            TimeUnit::Millisecond,
            Difference::Fractional(5_400_000.0)
        )]
        #[case::day(TimeUnit::Day, Difference::Fractional(0.0625))]
        #[case::week(
            TimeUnit::Week,
            Difference::Fractional(0.008_928_571_428_571_428)
        )]
        fn returns_fractional_counts_between_date_time_points(
            #[case] unit: TimeUnit,
            #[case] expected: Difference,
        ) {
            // Arrange: ninety minutes span the date boundary.
            ZoneGuard::set("UTC");
            let from = point_at("2026-07-28T23:00:00Z");
            let to = point_at("2026-07-29T00:30:00Z");

            // Act
            let difference = from.diff(to, unit);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "{unit:?} between date-time points must be fractional exact \
                 time"
            );
        }

        #[rstest]
        #[case::month(TimeUnit::Month, Difference::Whole(0))]
        #[case::year(TimeUnit::Year, Difference::Whole(0))]
        fn returns_whole_calendar_counts_between_date_time_points(
            #[case] unit: TimeUnit,
            #[case] expected: Difference,
        ) {
            // Arrange: the two points sit in the same month and year.
            ZoneGuard::set("UTC");
            let from = point_at("2026-07-28T23:00:00Z");
            let to = point_at("2026-07-29T00:30:00Z");

            // Act
            let difference = from.diff(to, unit);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "{unit:?} must count whole calendar units of the local dates"
            );
        }

        #[rstest]
        #[case::calendar_point(
            "2026-07-29",
            TimeUnit::Day,
            Difference::Whole(0)
        )]
        #[case::date_time_point(
            "2026-07-29 14:30:05",
            TimeUnit::Hour,
            Difference::Fractional(0.0)
        )]
        fn returns_zero_for_a_point_diffed_against_itself(
            #[case] text: &str,
            #[case] unit: TimeUnit,
            #[case] expected: Difference,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let point = point_at(text);

            // Act
            let difference = point.diff(point, unit);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "a point diffed against itself must count zero"
            );
        }

        #[rstest]
        #[case::date_time_to_date(
            "2026-07-28T23:00:00Z",
            "2026-07-29",
            Difference::Whole(1)
        )]
        #[case::date_to_date_time(
            "2026-07-29",
            "2026-07-29T00:30:00",
            Difference::Whole(0)
        )]
        fn truncates_toward_zero_when_either_point_lacks_a_time(
            #[case] from: &str,
            #[case] to: &str,
            #[case] expected: Difference,
        ) {
            // Arrange: mixed frames count whole units of the local reading.
            ZoneGuard::set("UTC");
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(to, TimeUnit::Hour);

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "a mixed-frame hour diff must truncate toward zero"
            );
        }

        #[test]
        fn measures_a_dst_day_by_elapsed_time_when_both_points_carry_a_time() {
            // Arrange: New York springs forward on 2026-03-08 (a 23-hour day)
            // and falls back on 2026-11-01 (a 25-hour day).
            ZoneGuard::set("America/New_York");
            let spring_from = point_at("2026-03-07 12:00:00");
            let spring_to = point_at("2026-03-08 12:00:00");
            let fall_from = point_at("2026-10-31 12:00:00");
            let fall_to = point_at("2026-11-01 12:00:00");

            // Act
            let spring_day = spring_from.diff(spring_to, TimeUnit::Day);
            let spring_hour = spring_from.diff(spring_to, TimeUnit::Hour);
            let fall_day = fall_from.diff(fall_to, TimeUnit::Day);
            let fall_hour = fall_from.diff(fall_to, TimeUnit::Hour);

            // Assert
            assert_eq!(
                spring_day,
                Ok(Difference::Fractional(0.958_333_333_333_333_4)),
                "the spring day spans twenty-three elapsed hours"
            );
            assert_eq!(
                spring_hour,
                Ok(Difference::Fractional(23.0)),
                "the spring day spans twenty-three elapsed hours"
            );
            assert_eq!(
                fall_day,
                Ok(Difference::Fractional(1.041_666_666_666_666_7)),
                "the fall day spans twenty-five elapsed hours"
            );
            assert_eq!(
                fall_hour,
                Ok(Difference::Fractional(25.0)),
                "the fall day spans twenty-five elapsed hours"
            );
        }

        #[test]
        fn counts_whole_days_for_date_only_points_across_dst() {
            // Arrange: date-only points have no time component to fold.
            ZoneGuard::set("America/New_York");
            let from = point_at("2026-03-07");
            let to = point_at("2026-03-08");

            // Act
            let days = from.diff(to, TimeUnit::Day);
            let hours = from.diff(to, TimeUnit::Hour);

            // Assert
            assert_eq!(
                days,
                Ok(Difference::Whole(1)),
                "a date-only day diff must count the local calendar day"
            );
            assert_eq!(
                hours,
                Ok(Difference::Whole(24)),
                "a date-only hour diff must count nominal whole hours"
            );
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn reports_no_time_for_a_calendar_point() {
            // Arrange: the minimum year in the Jiff date range.
            let point = DatePoint::Calendar {
                date: DateValue::from(
                    jiff::civil::Date::new(-9999, 1, 1)
                        .expect("minimum Jiff date is valid"),
                ),
                date_precision: DatePrecision::Day,
            };

            // Act
            let has_time = point.has_time();

            // Assert
            assert!(!has_time, "a calendar point must report no time-of-day");
        }

        #[test]
        fn reports_a_time_for_a_date_time_point() {
            // Arrange: the fixed reference date-time.
            let point = DatePoint::DateTime(fixed_datetime());

            // Act
            let has_time = point.has_time();

            // Assert
            assert!(has_time, "a date-time point must report a time-of-day");
        }

        #[test]
        fn returns_the_stored_date_for_a_calendar_point() {
            // Arrange: month precision stores the first of the month.
            let point = point_at("2026-07");

            // Act
            let date = point.date();

            // Assert
            assert_eq!(
                date,
                ymd(2026, 7, 1),
                "a calendar point must return its stored date"
            );
        }

        #[test]
        fn returns_the_local_calendar_date_for_a_date_time_point() {
            // Arrange
            ZoneGuard::set("UTC");
            let point = DatePoint::DateTime(fixed_datetime());

            // Act
            let date = point.date();

            // Assert
            assert_eq!(
                date,
                ymd(2026, 7, 29),
                "a date-time point must render its local calendar date"
            );
        }

        #[test]
        fn renders_a_calendar_point_as_local_midnight() {
            // Arrange
            let point = point_at("2026-07-29");

            // Act
            let local = point.local_datetime_or_utc();

            // Assert
            assert_eq!(
                local.to_string(),
                "2026-07-29T00:00:00",
                "a calendar point must render as local midnight"
            );
        }

        #[test]
        fn renders_the_local_clock_reading_of_a_date_time_point() {
            // Arrange: the naive reference date-time resolved in New York.
            ZoneGuard::set("America/New_York");
            let point = point_at("2026-07-29 14:30:05");

            // Act
            let local = point.local_datetime_or_utc();
            let utc = point.instant().expect("the point is representable");

            // Assert
            assert_eq!(
                local.to_string(),
                "2026-07-29T14:30:05",
                "the local clock must keep the written wall time"
            );
            assert_eq!(
                utc.to_string(),
                "2026-07-29T18:30:05Z",
                "the stored date-time must be the UTC value"
            );
        }

        #[test]
        fn resolves_a_calendar_point_at_utc_midnight_in_utc() {
            // Arrange
            ZoneGuard::set("UTC");
            let point = point_at("2026-07-29");

            // Act
            let utc =
                point.instant().expect("the date midnight is representable");

            // Assert
            assert_eq!(
                utc.to_string(),
                "2026-07-29T00:00:00Z",
                "in UTC a calendar point must resolve at UTC midnight"
            );
        }

        #[test]
        fn resolves_a_calendar_point_at_local_midnight_in_the_process_zone() {
            // Arrange: New York sits four hours behind UTC in July.
            ZoneGuard::set("America/New_York");
            let point = point_at("2026-07-29");

            // Act
            let utc =
                point.instant().expect("the date midnight is representable");

            // Assert
            assert_eq!(
                utc.to_string(),
                "2026-07-29T04:00:00Z",
                "a calendar point must resolve at local midnight"
            );
        }

        #[rstest]
        #[case::month("2026-07", "%Y-%m")]
        #[case::day("2026-07-29", "%Y-%m-%d")]
        #[case::whole_seconds("2026-07-29T14:30:00Z", "%Y-%m-%dT%H:%M:%S")]
        #[case::fractional_seconds(
            "2026-07-29T14:30:00.123Z",
            "%Y-%m-%dT%H:%M:%S%.3f"
        )]
        fn preserves_written_precision_in_the_canonical_pattern(
            #[case] text: &str,
            #[case] expected: &str,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let point = point_at(text);

            // Act
            let pattern = point.canonical_pattern();

            // Assert
            assert_eq!(
                pattern, expected,
                "{text:?} must render its written precision"
            );
        }
    }

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn period_start_is_idempotent(
                year in 2000_i32..2030,
                month in 1_u32..=12,
                day in 1_u32..=28,
                period in prop_oneof![
                    Just(CalendarPeriod::Week),
                    Just(CalendarPeriod::Month),
                    Just(CalendarPeriod::Year),
                ],
            ) {
                // Arrange
                let point = DatePoint::from(ymd(year, month, day));
                // Act
                let start = point
                    .start_of(period)
                    .expect("a valid in-range date has a period start");
                let repeated = start
                    .start_of(period)
                    .expect("a period start remains in range");

                // Assert
                prop_assert_eq!(repeated, start);
            }

            #[test]
            fn calendar_point_stays_between_period_edges(
                year in 2000_i32..2030,
                month in 1_u32..=12,
                day in 1_u32..=28,
                period in prop_oneof![
                    Just(CalendarPeriod::Week),
                    Just(CalendarPeriod::Month),
                    Just(CalendarPeriod::Year),
                ],
            ) {
                // Arrange
                let point = DatePoint::from(ymd(year, month, day));
                // Act
                let start = point
                    .start_of(period)
                    .expect("a valid in-range date has a period start");
                let end = point
                    .end_of(period)
                    .expect("a valid in-range date has a period end");

                // Assert
                let DatePoint::Calendar { date: start, .. } = start else {
                    prop_assert!(false, "a calendar point keeps its kind");
                    return Ok(());
                };
                let DatePoint::Calendar { date: end, .. } = end else {
                    prop_assert!(false, "a calendar point keeps its kind");
                    return Ok(());
                };
                let DatePoint::Calendar { date, .. } = point else {
                    prop_assert!(false, "the generated point is a calendar date");
                    return Ok(());
                };
                prop_assert!(start <= date);
                prop_assert!(date <= end);
            }

            #[test]
            fn date_only_diff_is_antisymmetric_for_each_unit(
                year_a in 2000_i32..2030,
                month_a in 1_u32..=12,
                day_a in 1_u32..=28,
                year_b in 2000_i32..2030,
                month_b in 1_u32..=12,
                day_b in 1_u32..=28,
                unit in prop_oneof![
                    Just(TimeUnit::Millisecond),
                    Just(TimeUnit::Second),
                    Just(TimeUnit::Minute),
                    Just(TimeUnit::Hour),
                    Just(TimeUnit::Day),
                    Just(TimeUnit::Week),
                    Just(TimeUnit::Month),
                    Just(TimeUnit::Year),
                ],
            ) {
                // Arrange
                let a = DatePoint::from(ymd(year_a, month_a, day_a));
                let b = DatePoint::from(ymd(year_b, month_b, day_b));

                // Act
                let forward = a.diff(b, unit);
                let backward = b.diff(a, unit);

                // Assert
                let (Ok(Difference::Whole(forward)), Ok(Difference::Whole(backward))) =
                    (forward, backward) else {
                    prop_assert!(false, "date-only differences are whole");
                    return Ok(());
                };
                prop_assert_eq!(
                    forward,
                    backward.checked_neg().expect("date diff fits i64"),
                );
            }

            #[test]
            fn day_shift_then_inverse_restores_calendar_date(
                year in 2000_i32..2030,
                month in 1_u32..=12,
                day in 1_u32..=28,
                delta in -365_i64..=365,
            ) {
                // Arrange
                let point = DatePoint::from(ymd(year, month, day));
                // Act
                let shifted = point
                    .shift(delta, TimeUnit::Day)
                    .expect("bounded day shifts fit the date range");
                let restored = shifted
                    .shift(delta.checked_neg().expect("generated offset fits i64"), TimeUnit::Day)
                    .expect("inverse bounded day shifts fit the date range");

                // Assert
                prop_assert_eq!(restored, point);
            }
        }
    }
}
