//! Defines date points and their precision-aware arithmetic.
//!
//! Owns [`DatePoint`], a Date written at a month or day [`DatePrecision`], or a
//! Date-Time stored as UTC that renders as the local clock. Parsing retains the
//! written precision; month precision denotes the first of the month.
//! Arithmetic applies calendar units on the local clock and exact units as
//! exact elapsed time. Callers hold the validated date point or receive a
//! [`DateError`].

use jiff::{Span, Timestamp, civil, tz::TimeZone};

use super::{
    CalendarError, CalendarPeriod, DEFAULT_DATE_PATTERN, DateComponent,
    DateDiff, DateError, DateTimeValue, DateValue, DurationValue, PeriodEdge,
    TimeUnit,
    calendar::{shift_date, shift_datetime},
    date::DateShape,
    datetime::{DateTimeShape, ParsedDateTimeKind},
    zone::local_zone,
};

/// Resolves the process-local time zone for calendar operations.
///
/// # Errors
///
/// - [`CalendarError::Zone`] if system time-zone discovery fails.
fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
    local_zone().map_err(|error| CalendarError::Zone {
        input: error.to_string().into(),
    })
}
/// Tracks whether a zone-free date point uses month or full-date precision.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePrecision {
    /// Specifies year and month precision.
    Month,
    /// Specifies full calendar date precision.
    Day,
}

/// Represents a date at its written precision or a Date-Time.
///
/// Calendar points are zone-free and carry the written [`DatePrecision`].
/// Date-Time points store UTC and resolve naive input in the process-local
/// zone (a repeated local time takes the earlier occurrence; a skipped local
/// time shifts forward by the gap length).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePoint {
    /// Stores a zone-free Date at its written precision.
    Calendar {
        /// Stores the calendar date; month precision stores the first of the
        /// month.
        date: DateValue,
        /// Records how much of `date` the input specified.
        date_precision: DatePrecision,
    },
    /// Stores a UTC Date-Time and renders it as the local clock.
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
    /// Returns `true` if this date point carries a time-of-day component.
    #[must_use]
    pub(crate) const fn has_time(self) -> bool {
        matches!(self, Self::DateTime(_))
    }

    /// Returns the calendar date of this date point in the supplied zone.
    pub(crate) fn date_in(self, zone: &TimeZone) -> DateValue {
        match self {
            Self::Calendar {
                date,
                ..
            } => date,
            Self::DateTime(dt) => dt.local_datetime_in(zone).date().into(),
        }
    }

    /// Returns the calendar date of this date point in the process-local zone.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::Zone`] if system time-zone discovery fails.
    pub(crate) fn date(self) -> Result<DateValue, CalendarError> {
        Ok(self.date_in(&local_zone_for_calendar()?))
    }

    /// Returns the local civil datetime in the supplied zone.
    #[must_use]
    pub(crate) fn local_datetime_in(
        self,
        zone: &TimeZone,
    ) -> jiff::civil::DateTime {
        match self {
            Self::Calendar {
                date,
                ..
            } => date.civil_date().at(0, 0, 0, 0),
            Self::DateTime(dt) => dt.local_datetime_in(zone),
        }
    }

    /// Converts this date point to a zoned value without losing its stored
    /// instant.
    ///
    /// Calendar values resolve at local midnight.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if local midnight is outside the
    ///   supported range.
    pub(crate) fn zoned_datetime_in(
        self,
        zone: &TimeZone,
    ) -> Result<jiff::Zoned, CalendarError> {
        match self {
            Self::Calendar {
                date,
                ..
            } => date
                .civil_date()
                .at(0, 0, 0, 0)
                .to_zoned(zone.clone())
                .map_err(|_| CalendarError::OutOfRange),
            Self::DateTime(datetime) => {
                Ok(datetime.utc().to_zoned(zone.clone()))
            }
        }
    }

    #[cfg(test)]
    fn local_datetime_or_utc(self) -> jiff::civil::DateTime {
        self.local_datetime_in(
            &local_zone().expect("system time zone available for tests"),
        )
    }

    /// Returns the UTC-stored Date-Time equivalent of this date point.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if converting a calendar date to a UTC
    ///   instant exceeds the supported range.
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

    /// Returns the canonical strftime pattern preserving written precision.
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
            } => DEFAULT_DATE_PATTERN,
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
    pub(crate) fn shift(
        self,
        n: i64,
        unit: TimeUnit,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } if unit.is_calendar() => Ok(Self::Calendar {
                date: DateValue::from_civil_date(shift_date(
                    date.civil_date(),
                    n,
                    unit,
                )?),
                date_precision,
            }),
            Self::Calendar {
                date_precision,
                ..
            } => {
                let local_datetime = self.local_datetime_in(zone);
                let local_datetime = shift_datetime(local_datetime, n, unit)?;
                Ok(Self::Calendar {
                    date: DateValue::from_civil_date(local_datetime.date()),
                    date_precision,
                })
            }
            Self::DateTime(dt) => dt.shift(n, unit, zone).map(Self::DateTime),
        }
    }

    /// Applies the validated duration as one calendar operation.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the span exceeds the supported
    ///   date-time range.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        self.advance_span(duration.span(), zone)
    }

    /// Returns the first day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
    ///   supported range.
    pub(crate) fn start_of(
        self,
        calendar_period: CalendarPeriod,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        self.boundary(calendar_period, PeriodEdge::Start, zone)
    }

    /// Returns the last day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
    ///   supported range.
    pub(crate) fn end_of(
        self,
        calendar_period: CalendarPeriod,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        self.boundary(calendar_period, PeriodEdge::End, zone)
    }

    /// Selects a weekday of this date point's ISO week in zone.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the target date is not representable.
    pub(crate) fn with_weekday(
        self,
        target: civil::Weekday,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        let local_date = self.local_datetime_in(zone).date();
        let iso = local_date.iso_week_date();
        let target_date =
            civil::ISOWeekDate::new(iso.year(), iso.week(), target)
                .map(civil::Date::from)
                .map_err(|_| CalendarError::OutOfRange)?;
        self.with_date(target_date, zone)
    }

    /// Returns one calendar or clock component of the local datetime reading.
    /// Weekdays are ISO numbered, Monday = 1 through Sunday = 7.
    #[must_use]
    pub(crate) fn component(
        self,
        component: DateComponent,
        zone: &TimeZone,
    ) -> i64 {
        let local_datetime = self.local_datetime_in(zone);
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
    /// Calendar days and weeks count local civil-clock time: a 23- or 25-hour
    /// DST day is one day, with sub-day clock remainders retained when both
    /// points have a time. Date-only differences return whole units, truncated
    /// toward zero. Hours and smaller measure elapsed time for two date-times.
    /// Reversing the operands negates the result.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if converting a point or computing its
    ///   calendar difference exceeds the representable range.
    pub(crate) fn diff(
        self,
        to: Self,
        unit: TimeUnit,
        zone: &TimeZone,
    ) -> Result<DateDiff, CalendarError> {
        let local_delta = || {
            to.local_datetime_in(zone)
                .duration_since(self.local_datetime_in(zone))
        };
        let calendar_days = || {
            self.date_in(zone)
                .civil_date()
                .until(to.date_in(zone).civil_date())
                .map(|span| i64::from(span.get_days()))
                .map_err(|_| CalendarError::OutOfRange)
        };
        match unit {
            TimeUnit::Year => Ok(DateDiff::Whole(
                self.date_in(zone).whole_years_until(to.date_in(zone)),
            )),
            TimeUnit::Month => Ok(DateDiff::Whole(
                self.date_in(zone).whole_months_until(to.date_in(zone)),
            )),
            calendar @ (TimeUnit::Week | TimeUnit::Day)
                if self.has_time() && to.has_time() =>
            {
                // The civil clock counts a DST-short or DST-long day as one
                // calendar day, while preserving a fractional clock remainder.
                Ok(DateDiff::Fractional(
                    local_delta().as_secs_f64() / calendar.nominal_seconds(),
                ))
            }
            TimeUnit::Week => Ok(DateDiff::Whole(calendar_days()? / 7)),
            TimeUnit::Day => Ok(DateDiff::Whole(calendar_days()?)),
            fixed @ (TimeUnit::Hour
            | TimeUnit::Minute
            | TimeUnit::Second
            | TimeUnit::Millisecond)
                if self.has_time() && to.has_time() =>
            {
                let seconds = match (self, to) {
                    (Self::DateTime(from), Self::DateTime(to)) => {
                        to.duration_since(from).as_secs_f64()
                    }
                    _ => return Err(CalendarError::OutOfRange),
                };
                Ok(DateDiff::Fractional(seconds / fixed.nominal_seconds()))
            }
            TimeUnit::Hour => {
                Ok(DateDiff::Whole(local_delta().as_secs() / 3_600))
            }
            TimeUnit::Minute => {
                Ok(DateDiff::Whole(local_delta().as_secs() / 60))
            }
            TimeUnit::Second => Ok(DateDiff::Whole(local_delta().as_secs())),
            TimeUnit::Millisecond => i64::try_from(local_delta().as_millis())
                .map(DateDiff::Whole)
                .map_err(|_| CalendarError::OutOfRange),
        }
    }

    /// Applies one Jiff span, preserving the point's date/time kind.
    fn advance_span(
        self,
        span: Span,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance_span(span)?,
                date_precision,
            }),
            Self::DateTime(datetime) => {
                Ok(Self::DateTime(datetime.checked_add_span(span, zone)?))
            }
        }
    }

    /// Replaces the local date, preserving written precision and time.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the resulting date-time exceeds the
    ///   supported range.
    fn with_date(
        self,
        date: civil::Date,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        match self {
            Self::Calendar {
                date_precision,
                ..
            } => Ok(Self::Calendar {
                date: DateValue::from_civil_date(date),
                date_precision,
            }),
            Self::DateTime(_) => {
                let local = self.local_datetime_in(zone);
                let local_datetime = date.to_datetime(local.time());
                Ok(Self::DateTime(DateTimeValue::from_local_in(
                    local_datetime,
                    zone,
                )?))
            }
        }
    }

    /// Resolves a period edge to its local date, preserving precision.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
    ///   supported range.
    fn boundary(
        self,
        calendar_period: CalendarPeriod,
        period_edge: PeriodEdge,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        let date = self.local_datetime_in(zone).date();
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
        self.with_date(new_date, zone)
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
            Ok(ParsedDateTimeKind::Instant(instant)) => {
                Ok(Self::DateTime(DateTimeValue::from_timestamp(instant)))
            }
            Ok(ParsedDateTimeKind::Unzoned(local_datetime)) => {
                Ok(Self::DateTime(
                    DateTimeValue::from_local(local_datetime)
                        .map_err(|error| error.into_date_error(trimmed))?,
                ))
            }
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

    /// Parses a raw date prefix from `s`, returning the [`DatePoint`] and byte
    /// length.
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
/// Returns whether a strtime `pattern` includes any time-of-day directives.
///
/// Returns [`None`] if the pattern contains an unsupported `%Q` directive.
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

/// Returns `true` if `s` begins with four ASCII year digits, optionally
/// preceded by a minus sign for a signed four-digit year.
pub(super) fn starts_with_year(s: &str) -> bool {
    let year = if s.starts_with('-') {
        s.get(1..5)
    } else {
        s.get(..4)
    };
    year.is_some_and(|bytes| {
        bytes.len() == 4 && bytes.bytes().all(|byte| byte.is_ascii_digit())
    })
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
            let shifted = base.shift(
                n,
                unit,
                &local_zone().expect("system time zone available"),
            );

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
            let shifted = base.shift(
                hours,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

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
            let shifted = base.shift(
                1,
                TimeUnit::Month,
                &local_zone().expect("system time zone available"),
            );

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
            let shifted = base.shift(
                2,
                TimeUnit::Month,
                &local_zone().expect("system time zone available"),
            );

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
            let shifted = base.shift(
                n,
                unit,
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let base = point_at("2026-03-07 12:00:00");
            let clock = |point: Result<DatePoint, CalendarError>| {
                point.map(|p| p.local_datetime_or_utc().to_string())
            };

            // Act
            let day = clock(base.shift(
                1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            ));
            let week = clock(base.shift(
                1,
                TimeUnit::Week,
                &local_zone().expect("system time zone available"),
            ));
            let month = clock(base.shift(
                1,
                TimeUnit::Month,
                &local_zone().expect("system time zone available"),
            ));
            let year = clock(base.shift(
                1,
                TimeUnit::Year,
                &local_zone().expect("system time zone available"),
            ));
            let back = clock(base.shift(
                -1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            ));

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let base = point_at("2026-10-31 12:00:00");

            // Act
            let day = base.shift(
                1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let spring = point_at("2026-03-07 12:00:00");
            let fall = point_at("2026-10-31 12:00:00");

            // Act
            let spring_hours = spring.shift(
                168,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );
            let spring_week = spring.shift(
                1,
                TimeUnit::Week,
                &local_zone().expect("system time zone available"),
            );
            let spring_days = spring.shift(
                7,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let fall_day = fall.shift(
                1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let fall_hours = fall.shift(
                24,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

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
                date: DateValue::from_civil_date(jiff::civil::Date::MIN),
                date_precision: DatePrecision::Day,
            };
            let max_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MAX,
            ));
            let min_point = DatePoint::DateTime(DateTimeValue::from_timestamp(
                Timestamp::MIN,
            ));

            // Act
            let past_max_date = max_date.shift(
                1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let past_min_date = min_date.shift(
                -1,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let past_max_point = max_point.shift(
                1,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );
            let past_min_point = min_point.shift(
                -1,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

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
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                applied,
                Ok(point_at("2026-08-07")),
                "\"1w 2d\" must apply in written order and keep day precision"
            );
        }

        #[rstest]
        #[case::negative_day("-1d", ymd(2026, 7, 28))]
        fn applies_signed_day_durations_to_a_calendar_date(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange
            let base = point_at("2026-07-29");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

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
        #[case::sub_day("12h", ymd(2026, 7, 29))]
        #[case::one_day("24h", ymd(2026, 7, 30))]
        #[case::over_a_day("36h", ymd(2026, 7, 30))]
        fn applies_exact_spans_to_calendar_dates(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange: exact elapsed time is measured from local midnight.
            let base = point_at("2026-07-29");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                })
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
            let duration = DurationValue::from_nominal_seconds(seconds)
                .expect("duration fits within the Jiff span range");

            // Act
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

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
        #[case::one_step("2mo", ymd(2026, 3, 31))]
        fn applies_a_duration_in_one_calendar_step(
            #[case] text: &str,
            #[case] expected: DateValue,
        ) {
            // Arrange: the two-month Span clamps from January 31 directly to
            // March 31 instead of clamping after each month.
            let base = point_at("2026-01-31");
            let duration = DurationValue::parse(text).expect("valid duration");

            // Act
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                applied,
                Ok(DatePoint::Calendar {
                    date: expected,
                    date_precision: DatePrecision::Day,
                }),
                "{text:?} must apply its calendar span in one operation"
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
            let applied = base.apply(
                &duration,
                &local_zone().expect("system time zone available"),
            );

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
            let synthesized = DurationValue::from_nominal_seconds(3600.0)
                .expect("one hour fits within the Jiff span range");

            // Act
            let applied = base.apply(
                &synthesized,
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let base = point_at("2026-03-07 12:00:00");
            let one_day = DurationValue::parse("1d").expect("valid duration");
            let twenty_four_hours =
                DurationValue::parse("24h").expect("valid duration");
            assert_eq!(
                one_day, twenty_four_hours,
                "\"1d\" and \"24h\" must be equal as values"
            );

            // Act
            let by_calendar = base.apply(
                &one_day,
                &local_zone().expect("system time zone available"),
            );
            let by_exact = base.apply(
                &twenty_four_hours,
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
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
            let by_week = base.apply(
                &one_week,
                &local_zone().expect("system time zone available"),
            );
            let by_days = base.apply(
                &seven_days,
                &local_zone().expect("system time zone available"),
            );
            let by_hours = base.apply(
                &hours_168,
                &local_zone().expect("system time zone available"),
            );

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
            // Arrange: a calendar day plus an exact 12-hour remainder spans
            // the spring gap; 36 exact hours does not.
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let base = point_at("2026-03-07 06:00:00");
            let written_order =
                DurationValue::parse("1d 12h").expect("valid duration");
            let thirty_six_hours =
                DurationValue::parse("36h").expect("valid duration");

            // Act
            let by_written = base.apply(
                &written_order,
                &local_zone().expect("system time zone available"),
            );
            let by_exact = base.apply(
                &thirty_six_hours,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                by_written
                    .as_ref()
                    .map(|p| p.local_datetime_or_utc().to_string()),
                Ok("2026-03-08T18:00:00".to_owned()),
                "the calendar day must keep the local clock time"
            );
            assert_ne!(
                by_written, by_exact,
                "the calendar day must differ from 36 exact hours across the \
                 gap"
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
            let by_month = base.apply(
                &one_month,
                &local_zone().expect("system time zone available"),
            );
            let by_days = base.apply(
                &thirty_days,
                &local_zone().expect("system time zone available"),
            );

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
            let overflowed = max_point.apply(
                &one_second,
                &local_zone().expect("system time zone available"),
            );
            let underflowed = min_point.apply(
                &negative_one_second,
                &local_zone().expect("system time zone available"),
            );

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
            let start = base.start_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let start = base.start_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let start = base.start_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let end = base.end_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let end = base.end_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let end = base.end_of(
                period,
                &local_zone().expect("system time zone available"),
            );

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
            let selected = base.with_weekday(
                target,
                &local_zone().expect("system time zone available"),
            );

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
            let monday = base.with_weekday(
                civil::Weekday::Monday,
                &local_zone().expect("system time zone available"),
            );

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
            let friday = base.with_weekday(
                civil::Weekday::Friday,
                &local_zone().expect("system time zone available"),
            );

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
            let sunday = base.with_weekday(
                civil::Weekday::Sunday,
                &local_zone().expect("system time zone available"),
            );

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
            let value = base.component(
                component,
                &local_zone().expect("system time zone available"),
            );

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
            let week = base.component(
                DateComponent::Week,
                &local_zone().expect("system time zone available"),
            );
            let weekday = base.component(
                DateComponent::Weekday,
                &local_zone().expect("system time zone available"),
            );

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
            let value = base.component(
                component,
                &local_zone().expect("system time zone available"),
            );

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
            let value = base.component(
                component,
                &local_zone().expect("system time zone available"),
            );

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
            DateDiff::Whole(6)
        )]
        #[case::month_across_month_ends(
            "2026-01-15",
            "2026-03-15",
            TimeUnit::Month,
            DateDiff::Whole(2)
        )]
        #[case::week(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Week,
            DateDiff::Whole(1)
        )]
        #[case::day(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Day,
            DateDiff::Whole(8)
        )]
        #[case::hour(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Hour,
            DateDiff::Whole(192)
        )]
        #[case::minute(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Minute,
            DateDiff::Whole(11520)
        )]
        #[case::second(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Second,
            DateDiff::Whole(691_200)
        )]
        #[case::millisecond(
            "2026-07-29",
            "2026-08-06",
            TimeUnit::Millisecond,
            DateDiff::Whole(691_200_000)
        )]
        fn returns_whole_counts_between_calendar_points(
            #[case] from: &str,
            #[case] to: &str,
            #[case] unit: TimeUnit,
            #[case] expected: DateDiff,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(
                to,
                unit,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "{unit:?} between calendar points must count whole units"
            );
        }

        #[rstest]
        #[case::month("2026-01-15", "2026-03-15", TimeUnit::Month, DateDiff::Whole(-2))]
        #[case::year("2020-06-15", "2026-07-15", TimeUnit::Year, DateDiff::Whole(-6))]
        #[case::day("2026-07-29", "2026-08-06", TimeUnit::Day, DateDiff::Whole(-8))]
        #[case::hour("2026-07-29", "2026-08-06", TimeUnit::Hour, DateDiff::Whole(-192))]
        fn returns_the_negated_difference_in_the_reverse_direction(
            #[case] from: &str,
            #[case] to: &str,
            #[case] unit: TimeUnit,
            #[case] expected: DateDiff,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = to.diff(
                from,
                unit,
                &local_zone().expect("system time zone available"),
            );

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
            DateDiff::Whole(7973)
        )]
        #[case::since_the_epoch(
            "1970-01-01",
            "2026-07-29",
            DateDiff::Whole(56)
        )]
        fn counts_whole_years_across_large_spans(
            #[case] from: &str,
            #[case] to: &str,
            #[case] expected: DateDiff,
        ) {
            // Arrange
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(
                to,
                TimeUnit::Year,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "whole years must count across a large span"
            );
        }

        #[rstest]
        #[case::hour(TimeUnit::Hour, DateDiff::Fractional(1.5))]
        #[case::minute(TimeUnit::Minute, DateDiff::Fractional(90.0))]
        #[case::second(TimeUnit::Second, DateDiff::Fractional(5400.0))]
        #[case::millisecond(
            TimeUnit::Millisecond,
            DateDiff::Fractional(5_400_000.0)
        )]
        #[case::calendar_day(TimeUnit::Day, DateDiff::Fractional(0.0625))]
        #[case::calendar_week(
            TimeUnit::Week,
            DateDiff::Fractional(0.008_928_571_428_571_428)
        )]
        fn returns_exact_fractions_and_calendar_counts_between_date_time_points(
            #[case] unit: TimeUnit,
            #[case] expected: DateDiff,
        ) {
            // Arrange: ninety minutes span one local date boundary.
            ZoneGuard::set("UTC");
            let from = point_at("2026-07-28T23:00:00Z");
            let to = point_at("2026-07-29T00:30:00Z");

            // Act
            let difference = from.diff(
                to,
                unit,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(difference, Ok(expected));
        }

        #[rstest]
        #[case::month(TimeUnit::Month, DateDiff::Whole(0))]
        #[case::year(TimeUnit::Year, DateDiff::Whole(0))]
        fn returns_whole_calendar_counts_between_date_time_points(
            #[case] unit: TimeUnit,
            #[case] expected: DateDiff,
        ) {
            // Arrange: the two points sit in the same month and year.
            ZoneGuard::set("UTC");
            let from = point_at("2026-07-28T23:00:00Z");
            let to = point_at("2026-07-29T00:30:00Z");

            // Act
            let difference = from.diff(
                to,
                unit,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "{unit:?} must count whole calendar units of the local dates"
            );
        }

        #[rstest]
        #[case::calendar_point("2026-07-29", TimeUnit::Day, DateDiff::Whole(0))]
        #[case::date_time_point(
            "2026-07-29 14:30:05",
            TimeUnit::Hour,
            DateDiff::Fractional(0.0)
        )]
        fn returns_zero_for_a_point_diffed_against_itself(
            #[case] text: &str,
            #[case] unit: TimeUnit,
            #[case] expected: DateDiff,
        ) {
            // Arrange
            ZoneGuard::set("UTC");
            let point = point_at(text);

            // Act
            let difference = point.diff(
                point,
                unit,
                &local_zone().expect("system time zone available"),
            );

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
            DateDiff::Whole(1)
        )]
        #[case::date_to_date_time(
            "2026-07-29",
            "2026-07-29T00:30:00",
            DateDiff::Whole(0)
        )]
        fn truncates_toward_zero_when_either_point_lacks_a_time(
            #[case] from: &str,
            #[case] to: &str,
            #[case] expected: DateDiff,
        ) {
            // Arrange: mixed frames count whole units of the local reading.
            ZoneGuard::set("UTC");
            let from = point_at(from);
            let to = point_at(to);

            // Act
            let difference = from.diff(
                to,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                difference,
                Ok(expected),
                "a mixed-frame hour diff must truncate toward zero"
            );
        }

        #[test]
        fn counts_local_calendar_days_across_dst_independently_of_elapsed_hours()
         {
            // Arrange: New York springs forward on 2026-03-08 (a 23-hour day)
            // and falls back on 2026-11-01 (a 25-hour day).
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let spring_from = point_at("2026-03-07 12:00:00");
            let spring_to = point_at("2026-03-08 12:00:00");
            let fall_from = point_at("2026-10-31 12:00:00");
            let fall_to = point_at("2026-11-01 12:00:00");

            let spring_day = spring_from.diff(
                spring_to,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let spring_reverse = spring_to.diff(
                spring_from,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let spring_hour = spring_from.diff(
                spring_to,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );
            let fall_day = fall_from.diff(
                fall_to,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let fall_reverse = fall_to.diff(
                fall_from,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let fall_hour = fall_from.diff(
                fall_to,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(spring_day, Ok(DateDiff::Fractional(1.0)));
            assert_eq!(spring_reverse, Ok(DateDiff::Fractional(-1.0)));
            assert_eq!(spring_hour, Ok(DateDiff::Fractional(23.0)));
            assert_eq!(fall_day, Ok(DateDiff::Fractional(1.0)));
            assert_eq!(fall_reverse, Ok(DateDiff::Fractional(-1.0)));
            assert_eq!(fall_hour, Ok(DateDiff::Fractional(25.0)));
        }

        #[test]
        fn counts_whole_days_for_date_only_points_across_dst() {
            // Arrange: date-only points have no time component to fold.
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let from = point_at("2026-03-07");
            let to = point_at("2026-03-08");

            // Act
            let days = from.diff(
                to,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );
            let hours = from.diff(
                to,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

            // Assert
            assert_eq!(
                days,
                Ok(DateDiff::Whole(1)),
                "a date-only day diff must count the local calendar day"
            );
            assert_eq!(
                hours,
                Ok(DateDiff::Whole(24)),
                "a date-only hour diff must count nominal whole hours"
            );
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn converts_a_date_to_a_day_precision_calendar_point() {
            // Arrange: a calendar date that must not acquire a time component.
            let date = ymd(2026, 7, 29);

            // Act
            let point = DatePoint::from(date);

            // Assert
            assert_eq!(
                point,
                DatePoint::Calendar {
                    date,
                    date_precision: DatePrecision::Day,
                },
                "converting a Date must preserve the date and use day \
                 precision"
            );
        }

        #[test]
        fn converts_a_date_time_to_a_date_time_point() {
            // Arrange: the fixed reference Date-Time.
            let date_time = fixed_datetime();

            // Act
            let point = DatePoint::from(date_time);

            // Assert
            assert_eq!(
                point,
                DatePoint::DateTime(date_time),
                "converting a Date-Time must preserve its value"
            );
        }

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
            let date = point.date().expect("calendar date exists");

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
            let date = point.date().expect("system zone available");

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
            let local = point.local_datetime_in(
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let point = point_at("2026-07-29 14:30:05");

            // Act
            let local = point.local_datetime_in(
                &local_zone().expect("system time zone available"),
            );
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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
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
                    .start_of(period, &local_zone().expect("system time zone available"))
                    .expect("a valid in-range date has a period start");
                let repeated = start
                    .start_of(period, &local_zone().expect("system time zone available"))
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
                    .start_of(period, &local_zone().expect("system time zone available"))
                    .expect("a valid in-range date has a period start");
                let end = point
                    .end_of(period, &local_zone().expect("system time zone available"))
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
                let forward = a.diff(b, unit, &local_zone().expect("system time zone available"));
                let backward = b.diff(a, unit, &local_zone().expect("system time zone available"));

                // Assert
                let (Ok(DateDiff::Whole(forward)), Ok(DateDiff::Whole(backward))) =
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
                    .shift(delta, TimeUnit::Day, &local_zone().expect("system time zone available"))
                    .expect("bounded day shifts fit the date range");
                let restored = shifted
                    .shift(delta.checked_neg().expect("generated offset fits i64"), TimeUnit::Day, &local_zone().expect("system time zone available"))
                    .expect("inverse bounded day shifts fit the date range");

                // Assert
                prop_assert_eq!(restored, point);
            }
        }
    }
}
