//! Date shape recognition and byte-exact duration prefix parsing.
//!
//! Logos classifies date shapes; winnow locates duration boundaries before
//! Jiff validates the complete duration.

use logos::Logos;
use winnow::{
    ascii::digit1,
    combinator::{alt, opt},
    prelude::*,
    token::{one_of, take_while},
};

use super::{DurationValue, date::DateShape, point::DatePoint};

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

/// Returns `true` if `bytes` begins with a two- or four-digit date year, or a
/// signed four-digit year, followed by a hyphen.
#[must_use]
pub(crate) fn starts_like_date(bytes: &[u8]) -> bool {
    if bytes.first() == Some(&b'-') {
        return bytes
            .get(1..5)
            .is_some_and(|year| year.iter().all(u8::is_ascii_digit))
            && bytes.get(5) == Some(&b'-');
    }

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
    let (date, shape) = DateShape::parse_any(matched).ok()?;
    let point = DatePoint::from_date_shape(date, shape);
    Some((point, leading_ws.saturating_add(span.end)))
}

/// Parses a duration prefix, returning its validated value and consumed bytes.
///
/// Jiff's `SpanParser::parse_span` consumes complete input but does not report
/// a prefix length; winnow locates the candidate without interpreting units.
#[must_use]
pub(super) fn parse_duration_prefix(
    input: &str,
) -> Option<(DurationValue, usize)> {
    let mut rest = input;
    duration_part.parse_next(&mut rest).ok()?;
    loop {
        let trimmed = rest.trim_start();
        if trimmed.starts_with(['+', '-']) {
            return None;
        }
        if !trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            break;
        }
        rest = trimmed;
        duration_part.parse_next(&mut rest).ok()?;
    }
    let end = input.len().saturating_sub(rest.len());
    Some((DurationValue::parse(&input[..end]).ok()?, end))
}

/// Recognizes one signed numeric component and its unit without allocating.
fn duration_part(input: &mut &str) -> ModalResult<()> {
    (
        opt(one_of(['+', '-'])),
        alt(((digit1, opt(('.', digit1))).take(), ('.', digit1).take())),
        take_while(0.., char::is_whitespace),
        take_while(1.., |c: char| c.is_ascii_alphabetic()),
    )
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::standard_full_date("2026-07-29 after", 10)]
    #[case::standard_year_month("2026-07 after", 7)]
    #[case::negative_full_date("-2026-07-29 after", 11)]
    #[case::negative_year_month("-2026-07 after", 8)]
    #[case::minimum_year("-9999-01-01 after", 11)]
    #[case::maximum_year("9999-12-31 after", 10)]
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
    #[case::fractional(".5h remaining", 3)]
    fn parses_valid_duration_prefixes(
        #[case] input: &str,
        #[case] expected_len: usize,
    ) {
        let (_val, len) =
            parse_duration_prefix(input).expect("valid duration prefix");
        assert_eq!(len, expected_len);
    }

    #[rstest]
    #[case::packed("2w3d] trailing", 4)]
    #[case::utf8_boundary("1h é", 2)]
    #[case::attached_utf8("1h°C", 2)]
    #[case::comma_boundary("1h, 30m", 2)]
    fn returns_exact_byte_extent_of_duration_candidate(
        #[case] input: &str,
        #[case] expected_len: usize,
    ) {
        let (_, len) = parse_duration_prefix(input).expect("valid prefix");
        assert_eq!(len, expected_len);
        assert!(input.is_char_boundary(len));
    }

    #[rstest]
    #[case::invalid_next_unit("1h 1x")]
    #[case::incomplete_next_part("1h 2")]
    #[case::internal_negative("1h -30m")]
    #[case::internal_positive("1h +30m")]
    #[case::invalid_unit("1ms")]
    #[case::leading_space(" 1h")]
    #[case::unicode_separator("1h\u{2003}30m")]
    fn rejects_invalid_duration_prefix(#[case] input: &str) {
        assert!(parse_duration_prefix(input).is_none());
    }
}
