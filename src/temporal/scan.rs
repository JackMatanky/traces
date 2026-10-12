use super::{error::DurationError, unit::DurationUnit};

/// A byte cursor over duration text.
///
/// Every method that consumes input advances the cursor; a method that returns
/// an error leaves it at an unspecified position, because a failed scan aborts
/// the whole parse.
#[derive(Copy, Clone, Debug)]
pub(super) struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    /// Starts a cursor at the beginning of `input`.
    pub(super) const fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    /// Returns the byte offset of the cursor.
    pub(super) const fn position(&self) -> usize {
        self.pos
    }

    /// Returns `true` if the cursor is past the last byte.
    pub(super) const fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Returns `true` if a numeric duration token can begin at the cursor.
    pub(super) fn can_start_component(&self) -> bool {
        match self.byte_at(0) {
            Some(byte) if byte.is_ascii_digit() => true,
            Some(b'.') => self.byte_at(1).is_some_and(|b| b.is_ascii_digit()),
            Some(b'+' | b'-') => self.has_digits_after_sign(),
            _ => false,
        }
    }

    /// Advances past whitespace and commas.
    pub(super) fn skip_separators(&mut self) {
        while self
            .byte_at(0)
            .is_some_and(|byte| byte.is_ascii_whitespace() || byte == b',')
        {
            self.bump();
        }
    }

    /// Advances past ASCII whitespace.
    pub(super) fn skip_whitespace(&mut self) {
        while self.byte_at(0).is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.bump();
        }
    }

    /// Moves to the next component if whitespace is followed by a token that
    /// can start one, returning whether it moved. The cursor stays put
    /// otherwise.
    pub(super) fn skip_to_next_component(&mut self) -> bool {
        let mut lookahead = *self;
        lookahead.skip_whitespace();
        let can_continue =
            !lookahead.is_at_end() && lookahead.can_start_component();
        if can_continue {
            *self = lookahead;
        }
        can_continue
    }

    /// Scans one `<number><unit>` part, returning the magnitude and unit.
    ///
    /// # Errors
    ///
    /// - [`DurationError::MisplacedSign`] if a sign is not followed by a digit
    ///   or a `.` and a digit.
    /// - [`DurationError::MissingNumber`] if no number starts at the cursor.
    /// - [`DurationError::MalformedNumber`] if the number is not valid float
    ///   syntax.
    /// - [`DurationError::NumberOverflow`] if the number overflows to infinity.
    /// - [`DurationError::MissingUnit`] if no unit follows the number.
    /// - [`DurationError::UnknownUnit`] if the unit is not recognized.
    pub(super) fn component(
        &mut self,
    ) -> Result<(f64, DurationUnit), DurationError> {
        let number = self.number()?;
        self.skip_whitespace();
        let unit = self.unit()?;
        Ok((number, unit))
    }

    /// Scans a decimal number with an optional leading `+` or `-` and an
    /// optional trailing exponent (`"1e5"`, `".5e-3"`), the same float syntax
    /// the [`super::duration::DurationSeconds`] [`Display`](std::fmt::Display)
    /// dialect emits for extreme magnitudes.
    fn number(&mut self) -> Result<f64, DurationError> {
        let start = self.pos;
        if matches!(self.byte_at(0), Some(b'+' | b'-')) {
            if !self.has_digits_after_sign() {
                return Err(DurationError::MisplacedSign);
            }
            self.bump();
        }
        let mut has_decimal = false;
        let mut has_digit = false;
        while let Some(byte) = self.byte_at(0) {
            if byte.is_ascii_digit() {
                has_digit = true;
            } else if byte == b'.' && !has_decimal {
                has_decimal = true;
            } else {
                break;
            }
            self.bump();
        }
        // An exponent may follow a digit-bearing mantissa. A truncated exponent
        // (`"1e"`, `"1e+"`) is not consumed: the `e` falls to the unit scanner
        // and is reported as an unknown unit.
        if has_digit && matches!(self.byte_at(0), Some(b'e' | b'E')) {
            self.skip_exponent();
        }
        self.convert_number(start)
    }

    /// Consumes an `e`/`E` exponent at the cursor when it has digits.
    fn skip_exponent(&mut self) {
        let mut offset = 1;
        if matches!(self.byte_at(offset), Some(b'+' | b'-')) {
            offset = offset.saturating_add(1);
        }
        if self.byte_at(offset).is_some_and(|b| b.is_ascii_digit()) {
            while self.byte_at(offset).is_some_and(|b| b.is_ascii_digit()) {
                offset = offset.saturating_add(1);
            }
            self.pos = self.pos.saturating_add(offset);
        }
    }

    /// Converts the scanned number span `start..cursor` into `f64`.
    fn convert_number(&self, start: usize) -> Result<f64, DurationError> {
        if start == self.pos {
            return Err(DurationError::MissingNumber);
        }
        // The span holds only single-byte ASCII (`+`/`-`/`.`/digit/`e`/`E`), so
        // slicing and UTF-8 decoding cannot fail.
        let Some(text) = self
            .bytes
            .get(start..self.pos)
            .and_then(|span| core::str::from_utf8(span).ok())
        else {
            return Err(DurationError::MissingNumber);
        };
        let number: f64 =
            text.parse().map_err(|source| DurationError::MalformedNumber {
                source,
            })?;
        if number.is_finite() {
            Ok(number)
        } else {
            Err(DurationError::NumberOverflow)
        }
    }

    /// Scans a run of ASCII letters as a unit spelling.
    fn unit(&mut self) -> Result<DurationUnit, DurationError> {
        let start = self.pos;
        while self.byte_at(0).is_some_and(|byte| byte.is_ascii_alphabetic()) {
            self.bump();
        }
        self.resolve_unit(start)
    }

    /// Converts the scanned unit span `start..cursor` into a [`DurationUnit`].
    fn resolve_unit(
        &self,
        start: usize,
    ) -> Result<DurationUnit, DurationError> {
        if start == self.pos {
            return Err(DurationError::MissingUnit);
        }
        let Some(unit_slice) = self.bytes.get(start..self.pos) else {
            return Err(DurationError::MissingUnit);
        };
        let spelling = core::str::from_utf8(unit_slice)
            .map_err(|_| DurationError::UnknownUnit)?;
        DurationUnit::from_spelling(spelling).ok_or(DurationError::UnknownUnit)
    }

    /// Returns `true` if the bytes after the sign at the cursor begin a valid
    /// number: a digit, or `.` followed by one.
    fn has_digits_after_sign(&self) -> bool {
        match self.byte_at(1) {
            Some(byte) if byte.is_ascii_digit() => true,
            Some(b'.') => self.byte_at(2).is_some_and(|b| b.is_ascii_digit()),
            _ => false,
        }
    }

    /// Returns the byte `offset` bytes past the cursor.
    fn byte_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos.saturating_add(offset)).copied()
    }

    /// Moves the cursor one byte forward.
    fn bump(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }
}

/// How far a scan may stop before the end of the input.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum ScanMode {
    /// The whole input must be consumed.
    Whole,
    /// Scanning stops at the first byte that cannot continue a component.
    Prefix,
}
