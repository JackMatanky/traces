//! Declarative token recognition for date and date-time input shapes.
//!
//! Uses [`logos::Logos`] to classify date, date-time, and duration shapes in a
//! single pass, replacing procedural heuristics with parse-don't-validate
//! recognition.

use logos::Logos;

use super::{DateValue, DurationValue, date::DateShape, point::DatePoint};

/// Token classification for temporal prefix recognition.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
enum DatePrefixToken {
    /// Full ISO date: `YYYY-MM-DD` or `-YYYY-MM-DD`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}")]
    FullDate,

    /// Year-month ISO date: `YYYY-MM` or `-YYYY-MM`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}")]
    YearMonth,
}

/// Token classification for duration prefix recognition.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
enum DurationToken {
    #[regex(r"[+-]?([0-9]+(\.[0-9]+)?|\.[0-9]+)\s*[a-zA-Z]+")]
    Part,

    #[regex(r"[ \t\n\f\r]+", logos::skip)]
    Whitespace,
}

/// Token classification for whole temporal input shapes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
pub(super) enum TemporalShapeToken {
    /// Explicit RFC 3339 date-time with timezone offset or `Z`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?([zZ]|[+-][0-9]{2}:[0-9]{2})")]
    Rfc3339,

    /// Local date-time with seconds precision and optional fractional seconds.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?")]
    LocalSeconds,

    /// Local date-time with minute-only precision.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}")]
    LocalMinutes,

    /// Full ISO date: `YYYY-MM-DD` or `-YYYY-MM-DD`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}")]
    FullDate,

    /// Year-month ISO date: `YYYY-MM` or `-YYYY-MM`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}")]
    YearMonth,

    /// Candidate date shape with a 2-digit year (e.g. `26-08-22`).
    #[regex(r"-?[0-9]{2}-[0-9]{2}(-[0-9]{2})?")]
    TwoDigitYear,
}

/// Returns `true` if `bytes` begins with two or four ASCII digits followed by
/// a hyphen.
#[must_use]
pub(crate) fn starts_like_date(bytes: &[u8]) -> bool {
    bytes.len() >= 5
        && [2, 4].into_iter().any(|digits| {
            bytes
                .get(..digits)
                .is_some_and(|head| head.iter().all(u8::is_ascii_digit))
                && bytes.get(digits) == Some(&b'-')
        })
}

/// Returns `true` if `s` begins with four ASCII year digits, optionally
/// preceded by a minus sign for a signed four-digit year.
#[must_use]
pub(crate) fn starts_with_year(s: &str) -> bool {
    let year = if s.starts_with('-') {
        s.get(1..5)
    } else {
        s.get(..4)
    };
    year.is_some_and(|bytes| {
        bytes.len() == 4 && bytes.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Scans a date prefix from `input`, returning the parsed [`DatePoint`] and
/// consumed byte count.
#[must_use]
pub(super) fn scan_date_prefix(input: &str) -> Option<(DatePoint, usize)> {
    let trimmed_start = input.trim_start();
    let leading_ws = input.len().saturating_sub(trimmed_start.len());
    let mut lexer = DatePrefixToken::lexer(trimmed_start);

    let _token = lexer.next()?.ok()?;
    let span = lexer.span();
    if span.start != 0 {
        return None;
    }

    let matched = &trimmed_start[span.clone()];
    if matched.len() == 7 && trimmed_start[span.end..].starts_with('-') {
        return None;
    }
    let (date, precision) = DateShape::parse_any(matched).ok()?;
    let point = DatePoint::Calendar {
        date: DateValue::from_civil_date(date),
        date_precision: precision,
    };
    Some((point, leading_ws.saturating_add(span.end)))
}

/// Scans a duration prefix from `input`, returning the parsed [`DurationValue`]
/// and consumed byte count.
#[must_use]
pub(super) fn scan_duration_prefix(
    input: &str,
) -> Option<(DurationValue, usize)> {
    let mut lexer = DurationToken::lexer(input);
    let mut last_end = 0;
    while let Some(Ok(DurationToken::Part)) = lexer.next() {
        let span = lexer.span();
        if last_end == 0 && span.start != 0 {
            return None;
        }
        last_end = span.end;
        let rem = &input[last_end..];
        let trimmed = rem.trim_start();
        if trimmed.starts_with(['+', '-']) {
            return None;
        }
        if !trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            break;
        }
    }
    if last_end == 0 {
        return None;
    }
    let rem = &input[last_end..];
    let trimmed = rem.trim_start();
    if trimmed.starts_with(['+', '-']) {
        return None;
    }
    if trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let candidate = &input[..last_end];
    let val = DurationValue::parse(candidate).ok()?;
    Some((val, last_end))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::standard_full_date("2026-07-29 after", 10)]
    #[case::standard_year_month("2026-07 after", 7)]
    #[case::leading_whitespace("   2026-07-29 after", 13)]
    fn scans_valid_date_prefixes(
        #[case] input: &str,
        #[case] expected_len: usize,
    ) {
        let (point, len) = scan_date_prefix(input).expect("valid prefix");
        assert_eq!(len, expected_len);
        assert!(matches!(point, DatePoint::Calendar { .. }));
    }

    #[rstest]
    #[case::empty("")]
    #[case::no_prefix("not a date")]
    #[case::two_digit_prefix("26-07-29")]
    fn rejects_invalid_date_prefixes(#[case] input: &str) {
        assert_eq!(scan_date_prefix(input), None);
    }

    #[rstest]
    #[case::single_unit("1h remaining", 2)]
    #[case::compound("1h 30m remaining", 6)]
    #[case::signed("-2d extra", 3)]
    fn scans_valid_duration_prefixes(
        #[case] input: &str,
        #[case] expected_len: usize,
    ) {
        let (_val, len) =
            scan_duration_prefix(input).expect("valid duration prefix");
        assert_eq!(len, expected_len);
    }
}
