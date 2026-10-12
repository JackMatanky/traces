use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};

use super::{DATE_PATTERN, DateTimeValue, DateValue};

/// Calendar precision of a zone-free date point.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePrecision {
    /// Year and month.
    Month,
    /// Full calendar date.
    Day,
}

/// A parsed date or date-time at the precision it was written.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePoint {
    /// Zone-free calendar date.
    Calendar {
        /// The calendar date; month precision stores the first of the month.
        date: DateValue,
        /// How much of `date` was written.
        date_precision: DatePrecision,
    },
    /// UTC instant.
    Instant(DateTimeValue),
}

impl DatePoint {
    /// Whether this date point carries a time-of-day component.
    #[must_use]
    pub(crate) const fn has_time(self) -> bool {
        matches!(self, Self::Instant(_))
    }

    /// Calendar date of this date point in the local zone.
    #[must_use]
    pub(crate) fn date(self) -> DateValue {
        match self {
            Self::Calendar {
                date,
                ..
            } => date,
            Self::Instant(dt) => dt.local_date(),
        }
    }

    /// Local datetime reading of this date point: midnight for a calendar date,
    /// the local clock time (UTC when the offset cannot apply) for an instant.
    #[must_use]
    pub(crate) fn local_datetime_or_utc(self) -> NaiveDateTime {
        match self {
            Self::Calendar {
                date,
                ..
            } => date.naive_date().and_time(NaiveTime::MIN),
            Self::Instant(dt) => dt.local_datetime_or_utc(),
        }
    }

    /// UTC instant represented by this date point.
    #[must_use]
    pub(crate) fn instant(self) -> DateTime<Utc> {
        match self {
            Self::Calendar {
                date,
                ..
            } => DateTimeValue::from(date).utc(),
            Self::Instant(dt) => dt.utc(),
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
            Self::Instant(dt) => dt.canonical_pattern(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::{
        temporal::{DurationValue, fixtures::fixed_datetime},
        test_support::TzGuard,
    };

    mod date_point {
        use chrono::NaiveDate;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn has_time_is_false_for_a_calendar_point() {
            let point = DatePoint::Calendar {
                date: DateValue::from(NaiveDate::MIN),
                date_precision: DatePrecision::Day,
            };

            assert!(!point.has_time());
        }

        #[test]
        fn has_time_is_true_for_an_instant() {
            assert!(DatePoint::Instant(fixed_datetime()).has_time());
        }

        #[rstest]
        #[case::month("2026-07", "%Y-%m")]
        #[case::day("2026-07-29", "%Y-%m-%d")]
        #[case::whole_seconds("2026-07-29T14:30:00Z", "%Y-%m-%dT%H:%M:%S")]
        #[case::fractional_seconds(
            "2026-07-29T14:30:00.123Z",
            "%Y-%m-%dT%H:%M:%S%.f"
        )]
        fn canonical_pattern_preserves_precision_and_fractional_seconds(
            #[case] text: &str,
            #[case] expected: &str,
        ) {
            // Arrange
            TzGuard::set("UTC");
            let point = DatePoint::parse(text).expect("valid date point");

            // Act
            let pattern = point.canonical_pattern();

            // Assert
            assert_eq!(pattern, expected);
        }
    }

    mod golden_serde {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn date_and_datetime_keep_their_json_and_postcard_spellings() {
            // Arrange
            TzGuard::set("UTC");
            let date = DateValue::parse_iso("2026-07-29").unwrap();
            let datetime =
                DateTimeValue::parse_iso("2026-07-29T14:30:05Z").unwrap();

            // Act
            let date_bytes = postcard::to_allocvec(&date).unwrap();
            let datetime_bytes = postcard::to_allocvec(&datetime).unwrap();

            // Assert
            assert_eq!(serde_json::to_string(&date).unwrap(), "\"2026-07-29\"");
            assert_eq!(
                serde_json::to_string(&datetime).unwrap(),
                "\"2026-07-29T14:30:05Z\""
            );
            assert_eq!(date_bytes, b"\x0a2026-07-29");
            assert_eq!(datetime_bytes, b"\x142026-07-29T14:30:05Z");
            assert_eq!(
                postcard::from_bytes::<DateValue>(&date_bytes).unwrap(),
                date
            );
            assert_eq!(
                postcard::from_bytes::<DateTimeValue>(&datetime_bytes).unwrap(),
                datetime
            );
        }
    }

    mod hostile_yaml_note {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[derive(Deserialize)]
        struct HostileNote {
            duration_extreme: DurationValue,
            duration_zero: DurationValue,
            date: DateValue,
            datetime: DateTimeValue,
        }

        #[test]
        fn parses_round_trips_and_displays_without_panic_or_lying() {
            let extreme_digits = "9".repeat(50);
            let yaml = format!(
                "duration_extreme: {extreme_digits}y\nduration_zero: \
                 \"-0m\"\ndate: \"2026-07\"\ndatetime: \
                 \"2026-07-29T14:30:00Z\"\n"
            );

            let note = noyalib::from_str::<HostileNote>(&yaml)
                .expect("hostile note deserializes without panicking");

            // A value rebuilt from its parsed seconds (no original
            // spelling to echo) renders in scientific notation, never a
            // silent "0s".
            let synthesized = DurationValue::from_nominal_seconds(
                note.duration_extreme.nominal_seconds(),
            );
            let rendered_extreme = synthesized.to_string();
            assert_ne!(rendered_extreme, "0s");
            assert!(rendered_extreme.contains('e'));

            // "-0m": equals and orders as zero, not a hidden negative.
            assert_eq!(
                note.duration_zero,
                DurationValue::parse("0m").expect("valid duration")
            );
            assert!(note.duration_zero >= DurationValue::parse("0m").unwrap());

            // "YYYY-MM" date: precision defaults to day 1, matches inline
            // parsing exactly.
            assert_eq!(
                note.date,
                DateValue::parse_iso("2026-07-01").expect("valid date")
            );

            // "…Z" datetime: parses to the same instant as the explicit-offset
            // spelling (naive would mean the local zone) and re-serializes
            // with the same spelling.
            assert_eq!(
                note.datetime,
                DateTimeValue::parse_iso("2026-07-29T14:30:00+00:00")
                    .expect("valid datetime")
            );
            assert_eq!(
                serde_json::to_string(&note.datetime).expect("serializable"),
                "\"2026-07-29T14:30:00Z\""
            );
        }
    }
}
