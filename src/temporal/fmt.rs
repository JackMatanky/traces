//! Renders canonical date-time patterns with one year convention.
//!
//! The sign always sits outside the four-digit year width (`-0009-01-01`),
//! matching `DateValue`'s display and jiff's own `%F`. Jiff's `%Y` counts
//! the sign inside the width (`-009-01-01`), which the four-digit parse
//! rule rejects, so canonical output renders through
//! [`render_canonical_pattern`] instead of `%Y`.

use jiff::civil;

use super::{DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN};

/// One canonical output shape and the yearless pattern for each of its
/// halves.
enum CanonicalForm {
    /// `YYYY-MM-DD`: the date half only.
    Date,
    /// `YYYY-MM`: reduced year-month precision.
    Month,
    /// A full date-time; the payload renders the time half after `T`.
    /// Time carries no year, so it cannot reintroduce a sign convention.
    DateTime(&'static str),
}

/// Every pattern the crate renders canonically, paired with its form.
///
/// Entry one is the date pattern, entry two is month precision, the next
/// four are `DateTimeValue::canonical_pattern`'s shapes, and `"%F"` is the
/// user-facing ISO date.
static CANONICAL_PATTERNS: &[(&str, CanonicalForm)] = &[
    (DEFAULT_DATE_PATTERN, CanonicalForm::Date),
    ("%Y-%m", CanonicalForm::Month),
    (DEFAULT_DATETIME_PATTERN, CanonicalForm::DateTime("%H:%M:%S")),
    ("%Y-%m-%dT%H:%M:%S%.3f", CanonicalForm::DateTime("%H:%M:%S%.3f")),
    ("%Y-%m-%dT%H:%M:%S%.6f", CanonicalForm::DateTime("%H:%M:%S%.6f")),
    ("%Y-%m-%dT%H:%M:%S%.9f", CanonicalForm::DateTime("%H:%M:%S%.9f")),
    ("%F", CanonicalForm::Date),
];

/// Renders `pattern` over `datetime` with the sign outside the year width.
///
/// # Arguments
///
/// * `datetime` - civil fields to render
/// * `pattern` - a pattern from [`CANONICAL_PATTERNS`]
///
/// # Returns
///
/// The rendered spelling, or `None` when `pattern` is not canonical.
/// Callers keep their jiff-native path for non-canonical patterns.
pub(crate) fn render_canonical_pattern(
    datetime: civil::DateTime,
    pattern: &str,
) -> Option<String> {
    let (_, form) = CANONICAL_PATTERNS
        .iter()
        .find(|(canonical, _)| *canonical == pattern)?;
    let date_tail = match form {
        CanonicalForm::Month => "%m",
        CanonicalForm::Date | CanonicalForm::DateTime(_) => "%m-%d",
    };
    let mut rendered = render_year(datetime.year());
    rendered.push('-');
    rendered.push_str(&jiff::fmt::strtime::format(date_tail, datetime).ok()?);
    if let CanonicalForm::DateTime(time_pattern) = form {
        rendered.push('T');
        rendered.push_str(
            &jiff::fmt::strtime::format(time_pattern, datetime).ok()?,
        );
    }
    Some(rendered)
}

/// Writes the year with the sign outside its four-digit width.
fn render_year(year: i16) -> String {
    if year < 0 {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{year:04}")
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::{
        super::{DatePoint, DateValue},
        *,
    };

    #[rstest]
    #[case::year_zero(0, "0000")]
    #[case::two_digits(25, "0025")]
    #[case::negative_two_digits(-25, "-0025")]
    #[case::negative_single(-9, "-0009")]
    #[case::negative_max(-9999, "-9999")]
    fn writes_the_sign_outside_the_year_width(
        #[case] year: i16,
        #[case] expected: &str,
    ) {
        assert_eq!(render_year(year), expected);
    }

    #[test]
    fn renders_every_canonical_pattern_and_rejects_the_rest() {
        // Arrange
        let datetime: civil::DateTime = "2026-07-05T04:05:03.123456789"
            .parse()
            .expect("fixture datetime parses");

        // Act + Assert: the canonical ladder renders.
        for pattern in [
            DEFAULT_DATE_PATTERN,
            "%Y-%m",
            DEFAULT_DATETIME_PATTERN,
            "%Y-%m-%dT%H:%M:%S%.3f",
            "%Y-%m-%dT%H:%M:%S%.6f",
            "%Y-%m-%dT%H:%M:%S%.9f",
            "%F",
        ] {
            assert!(
                render_canonical_pattern(datetime, pattern).is_some(),
                "{pattern} must be canonical"
            );
        }

        // Non-canonical patterns fall through to the caller's jiff path.
        assert!(render_canonical_pattern(datetime, "%d %B %Y").is_none());
        assert!(render_canonical_pattern(datetime, "%F%.3f").is_none());
    }

    #[test]
    fn renders_negative_years_in_every_canonical_form() {
        // Arrange: a negative year with time attached.
        let datetime = civil::Date::new(-9, 1, 5)
            .expect("fixture date is valid")
            .at(4, 5, 3, 0);

        // Act + Assert
        assert_eq!(
            render_canonical_pattern(datetime, DEFAULT_DATE_PATTERN),
            Some("-0009-01-05".to_owned()),
        );
        assert_eq!(
            render_canonical_pattern(datetime, "%Y-%m"),
            Some("-0009-01".to_owned()),
        );
        assert_eq!(
            render_canonical_pattern(datetime, DEFAULT_DATETIME_PATTERN),
            Some("-0009-01-05T04:05:03".to_owned()),
        );
        assert_eq!(
            render_canonical_pattern(datetime, "%F"),
            Some("-0009-01-05".to_owned()),
        );
    }

    #[test]
    fn date_values_keep_the_same_year_spelling() {
        // Arrange: DateValue's Display predates the table and must stay
        // byte-identical to it.
        let date =
            DateValue::from_ymd(-9, 1, 5).expect("fixture date is valid");

        // Act
        let rendered = render_canonical_pattern(
            date.civil_date().at(4, 5, 3, 0),
            DEFAULT_DATE_PATTERN,
        );

        // Assert
        assert_eq!(rendered.as_deref(), Some(date.to_string().as_str()));
    }

    #[test]
    fn the_parser_accepts_wider_year_forms_than_the_renderer_emits() {
        // Arrange: jiff's strtime parser reads a sign plus up to four
        // digits; the crate's four-digit-year rule is stricter.

        // Act
        let strtime_accepts =
            DatePoint::parse_with_pattern("-009-01-01", "%Y-%m-%d").is_ok();
        let crate_rejects = "-009-01-01".parse::<DateValue>().is_err();

        // Assert: the renderer never emits the lenient form.
        assert!(strtime_accepts, "strtime parse reads sign+3 digits");
        assert!(crate_rejects, "the four-digit rule rejects sign+3 digits");
        let datetime = civil::Date::new(-9, 1, 1)
            .expect("fixture date is valid")
            .at(0, 0, 0, 0);
        assert_eq!(
            render_canonical_pattern(datetime, DEFAULT_DATE_PATTERN).as_deref(),
            Some("-0009-01-01"),
        );
    }

    proptest! {
        #[test]
        fn percent_f_matches_the_canonical_date_spelling(
            year in -9999i32..=9999,
            month in 1u32..=12,
            day in 1u32..=28,
        ) {
            // Arrange: jiff's `%F` writes the sign outside the year width
            // itself (printer.rs:454-468), so both spellings must agree
            // for every representable year.
            let date = DateValue::from_ymd(year, month, day)
                .expect("generated fields form a valid date");
            let datetime = date.civil_date().at(0, 0, 0, 0);

            // Act
            let ours = render_canonical_pattern(datetime, "%F")
                .expect("%F is canonical");
            let theirs = jiff::fmt::strtime::format("%F", datetime)
                .expect("%F renders");

            // Assert
            prop_assert_eq!(ours, theirs);
        }
    }
}
