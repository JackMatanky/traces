//! Declarative token recognition for date and date-time input shapes.
//!
//! Uses [`logos::Logos`] to classify date and date-time shapes in a single
//! pass, replacing procedural heuristics with parse-don't-validate recognition.

use logos::Logos;

use super::{DateValue, date::DateShape, point::DatePoint};

/// Token classification for temporal prefix recognition.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
pub(crate) enum DatePrefixToken {
    /// Full ISO date: `YYYY-MM-DD` or `-YYYY-MM-DD`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}-[0-9]{2}")]
    FullDate,

    /// Year-month ISO date: `YYYY-MM` or `-YYYY-MM`.
    #[regex(r"-?[0-9]{4}-[0-9]{2}")]
    YearMonth,
}

/// Token classification for whole temporal input shapes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
pub(crate) enum TemporalShapeToken {
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
pub(crate) fn scan_date_prefix(input: &str) -> Option<(DatePoint, usize)> {
    let mut lexer = DatePrefixToken::lexer(input);
    let token = lexer.next()?.ok()?;
    let span = lexer.span();
    if span.start != 0 {
        return None;
    }
    // An input like "2026-07-2" has a dash following the month; that indicates
    // an incomplete day prefix rather than a valid year-month prefix.
    if token == DatePrefixToken::YearMonth
        && input.as_bytes().get(span.end) == Some(&b'-')
    {
        return None;
    }
    let matched = lexer.slice();
    let (date, date_precision) = DateShape::parse_any(matched).ok()?;
    Some((
        DatePoint::Calendar {
            date: DateValue::from_civil_date(date),
            date_precision,
        },
        span.end,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::point::DatePrecision;

    #[test]
    fn recognizes_full_date_and_year_month_tokens() {
        let mut full = DatePrefixToken::lexer("2026-07-29 rest");
        assert_eq!(full.next(), Some(Ok(DatePrefixToken::FullDate)));
        assert_eq!(full.span(), 0..10);

        let mut month = DatePrefixToken::lexer("2026-07 rest");
        assert_eq!(month.next(), Some(Ok(DatePrefixToken::YearMonth)));
        assert_eq!(month.span(), 0..7);
    }

    #[test]
    fn scans_date_prefixes_without_manual_byte_indexing() {
        let (point, len) =
            scan_date_prefix("2026-07-29 after").expect("parses");
        assert_eq!(len, 10);
        assert!(matches!(point, DatePoint::Calendar {
            date_precision: DatePrecision::Day,
            ..
        }));

        let (month_point, month_len) =
            scan_date_prefix("2026-07 after").expect("parses");
        assert_eq!(month_len, 7);
        assert!(matches!(month_point, DatePoint::Calendar {
            date_precision: DatePrecision::Month,
            ..
        }));
    }

    #[test]
    fn rejects_truncated_day_prefix_after_dash() {
        assert_eq!(scan_date_prefix("2026-07-2"), None);
        assert_eq!(scan_date_prefix("2026-07-3"), None);
    }
}
