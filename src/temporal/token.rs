//! Translates moment-dialect date patterns to strftime patterns.
//!
//! The moment dialect is the template-facing token language (`YYYY-MM-DD`,
//! `Do MMMM YYYY`, `[Daily]`) that the `date_format` filter accepts. This
//! module owns the tokenizer that converts it to a strftime pattern jiff can
//! render; patterns it cannot translate fail with [`TemporalTokenError`]
//! (declared in `error.rs`). This module's `pattern_to_strftime` owns
//! the `%`-contains dialect dispatch; the engine's same-named wrapper
//! calls it and maps the error to a template render error.

use super::error::TemporalTokenError;

/// The moment-dialect to strftime token table (scope 1).
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

/// Translates `pattern` to a strftime pattern.
///
/// If `pattern` contains `%`, it is treated as a strftime pattern and passed
/// through. Otherwise, it is translated from common moment-dialect tokens:
/// bracket literals (`[Daily]`), `YYYY MM DD HH mm ss` family, `Do`/`S`
/// ordinals, and `dddd`/`ddd`/`MMM`/`MMMM`.
///
/// # Errors
///
/// Returns [`TemporalTokenError::UnclosedBracket`] for an unterminated `[...]`
/// literal and [`TemporalTokenError::UnsupportedToken`] for a moment token
/// with no strftime equivalent.
pub(crate) fn pattern_to_strftime(
    pattern: &str,
    datetime: jiff::civil::DateTime,
) -> Result<String, TemporalTokenError> {
    if pattern.contains('%') {
        return Ok(pattern.to_owned());
    }
    moment_to_strftime(pattern, datetime)
}

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
) -> Result<String, TemporalTokenError> {
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
                return Err(TemporalTokenError::UnclosedBracket {
                    pattern: pattern.into(),
                });
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
                return Err(TemporalTokenError::UnsupportedToken {
                    token: token.into(),
                    pattern: pattern.into(),
                });
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
