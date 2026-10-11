use super::{
    error::DurationError, unit::DurationUnit, value::DurationComponent,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum ScanMode {
    Whole,
    Prefix,
}

/// Returns `true` if `bytes[pos]` can begin a numeric duration token.
pub(super) fn can_start_component(bytes: &[u8], pos: usize) -> bool {
    let Some(&b) = bytes.get(pos) else {
        return false;
    };
    if b.is_ascii_digit() {
        return true;
    }
    if b == b'.' {
        return bytes
            .get(pos.saturating_add(1))
            .is_some_and(u8::is_ascii_digit);
    }
    if b == b'+' || b == b'-' {
        return has_digits_after_sign(
            bytes.get(pos.saturating_add(1)),
            bytes.get(pos.saturating_add(2)),
        );
    }
    false
}

/// Returns `true` if the bytes following a `+`/`-` sign begin a valid
/// number: a digit, or `.` followed by one.
pub(super) fn has_digits_after_sign(
    next: Option<&u8>,
    next_next: Option<&u8>,
) -> bool {
    match next {
        Some(c) if c.is_ascii_digit() => true,
        Some(b'.') => next_next.is_some_and(u8::is_ascii_digit),
        _ => false,
    }
}

/// Advances `pos` past whitespace and commas.
pub(super) fn skip_separators(bytes: &[u8], pos: &mut usize) {
    let len = bytes.len();
    while *pos < len {
        let Some(&b) = bytes.get(*pos) else {
            break;
        };
        if !b.is_ascii_whitespace() && b != b',' {
            break;
        }
        *pos = (*pos).saturating_add(1);
    }
}

/// Advances `pos` past ASCII whitespace.
pub(super) fn skip_whitespace(bytes: &[u8], pos: &mut usize) {
    let len = bytes.len();
    while *pos < len {
        let Some(&b) = bytes.get(*pos) else {
            break;
        };
        if !b.is_ascii_whitespace() {
            break;
        }
        *pos = (*pos).saturating_add(1);
    }
}

/// Scans one `<number><unit>` part starting at `pos`, returning the
/// magnitude, unit, and end offset.
pub(super) fn scan_component(
    bytes: &[u8],
    pos: usize,
) -> Result<(f64, DurationUnit, usize), DurationError> {
    let (number, after_number) = scan_number(bytes, pos)?;
    let mut cursor = after_number;
    skip_whitespace(bytes, &mut cursor);
    let (unit, after_unit) = scan_unit(bytes, cursor)?;
    Ok((number, unit, after_unit))
}

/// Parses a decimal number starting at `pos`, supporting an optional leading
/// `+` or `-` and an optional trailing exponent (`"1e5"`, `".5e-3"`), the same
/// float syntax the [`super::value::DurationSeconds`]
/// [`Display`](std::fmt::Display) dialect emits for extreme magnitudes.
pub(super) fn scan_number(
    bytes: &[u8],
    mut pos: usize,
) -> Result<(f64, usize), DurationError> {
    let num_start = pos;
    if let Some(&b) = bytes.get(pos)
        && (b == b'+' || b == b'-')
    {
        if !has_digits_after_sign(
            bytes.get(pos.saturating_add(1)),
            bytes.get(pos.saturating_add(2)),
        ) {
            return Err(DurationError::MisplacedSign);
        }
        pos = pos.saturating_add(1);
    }
    let mut has_decimal = false;
    let mut has_digit = false;
    while pos < bytes.len() {
        let Some(&b) = bytes.get(pos) else {
            break;
        };
        if b.is_ascii_digit() {
            has_digit = true;
            pos = pos.saturating_add(1);
        } else if b == b'.' && !has_decimal {
            has_decimal = true;
            pos = pos.saturating_add(1);
        } else {
            break;
        }
    }
    // An exponent may follow a digit-bearing mantissa. A truncated exponent
    // (`"1e"`, `"1e+"`) is not consumed: the `e` falls to the unit scanner and
    // is reported as an unknown unit.
    if has_digit && matches!(bytes.get(pos), Some(b'e' | b'E')) {
        let mut exp_pos = pos.saturating_add(1);
        if matches!(bytes.get(exp_pos), Some(b'+' | b'-')) {
            exp_pos = exp_pos.saturating_add(1);
        }
        if bytes.get(exp_pos).is_some_and(u8::is_ascii_digit) {
            while bytes.get(exp_pos).is_some_and(u8::is_ascii_digit) {
                exp_pos = exp_pos.saturating_add(1);
            }
            pos = exp_pos;
        }
    }
    convert_number(bytes, num_start, pos)
}

/// Validates and converts a scanned number byte span into `f64`.
pub(super) fn convert_number(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<(f64, usize), DurationError> {
    if start == end {
        return Err(DurationError::MissingNumber);
    }
    // The span holds only single-byte ASCII (`+`/`-`/`.`/digit/`e`/`E`), so
    // slicing and UTF-8 decoding cannot fail.
    let Some(text) =
        bytes.get(start..end).and_then(|span| core::str::from_utf8(span).ok())
    else {
        return Err(DurationError::MissingNumber);
    };
    let number: f64 =
        text.parse().map_err(|source| DurationError::MalformedNumber {
            source,
        })?;
    if !number.is_finite() {
        return Err(DurationError::NumberOverflow);
    }
    Ok((number, end))
}

/// Parses a unit string starting at `pos`.
pub(super) fn scan_unit(
    bytes: &[u8],
    mut pos: usize,
) -> Result<(DurationUnit, usize), DurationError> {
    let unit_start = pos;
    while pos < bytes.len() {
        let Some(&b) = bytes.get(pos) else {
            break;
        };
        if !b.is_ascii_alphabetic() {
            break;
        }
        pos = pos.saturating_add(1);
    }
    resolve_unit(bytes, unit_start, pos)
}

/// Validates and converts a scanned unit byte span into [`DurationUnit`].
pub(super) fn resolve_unit(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<(DurationUnit, usize), DurationError> {
    if start == end {
        return Err(DurationError::MissingUnit);
    }
    let Some(unit_slice) = bytes.get(start..end) else {
        return Err(DurationError::MissingUnit);
    };
    let unit_str = core::str::from_utf8(unit_slice)
        .map_err(|_| DurationError::UnknownUnit)?;
    let kind = DurationUnit::from_spelling(unit_str)
        .ok_or(DurationError::UnknownUnit)?;
    Ok((kind, end))
}

/// Negates every magnitude in `components` when `is_negative` is `true`.
pub(super) fn apply_leading_sign(
    components: &mut [DurationComponent],
    is_negative: bool,
) {
    let sign = if is_negative {
        -1.0
    } else {
        1.0
    };
    for component in components {
        component.magnitude *= sign;
    }
}
