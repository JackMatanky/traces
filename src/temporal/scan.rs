//! A byte cursor over duration text.
//!
//! Owns [`Scanner`], the cursor primitives behind duration parsing: number
//! scanning with signs and exponents, unit scanning, separator skipping, and
//! prefix lookahead. [`ScanMode`] names how far a scan may stop before the
//! end of the input.

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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    mod can_start_component {

        use super::*;

        #[rstest]
        #[case::digit("1h")]
        #[case::zero("0")]
        #[case::decimal_point(".5h")]
        #[case::plus_sign("+1h")]
        #[case::minus_sign("-30m")]
        #[case::sign_with_decimal("+.5h")]
        fn accepts_a_component_start(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let can_start = Scanner::new(input).can_start_component();
            // Assert
            assert!(can_start);
        }

        #[rstest]
        #[case::empty("")]
        #[case::non_numeric("hello")]
        #[case::whitespace(" ")]
        #[case::lone_sign("+")]
        #[case::lone_minus("-")]
        #[case::lone_decimal_point(".")]
        #[case::decimal_point_without_digit(".h")]
        #[case::multibyte("é")]
        fn rejects_a_non_component_start(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let can_start = Scanner::new(input).can_start_component();
            // Assert
            assert!(!can_start);
        }
    }

    mod number {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::integer("1", 1.0)]
        #[case::leading_zero("01", 1.0)]
        #[case::two_digits("42", 42.0)]
        #[case::decimal("1.5", 1.5)]
        #[case::fraction("0.5", 0.5)]
        #[case::leading_decimal_point(".5", 0.5)]
        #[case::trailing_decimal_point("1.", 1.0)]
        #[case::leading_plus("+1", 1.0)]
        #[case::leading_minus("-1", -1.0)]
        #[case::signed_fraction("-.5", -0.5)]
        #[case::exponent("1e3", 1_000.0)]
        #[case::uppercase_exponent("1E3", 1_000.0)]
        #[case::exponent_sign("+1e+2", 100.0)]
        #[case::exponent_minus("1e-2", 0.01)]
        #[case::exponent_after_decimal(".5e2", 50.0)]
        // A truncated exponent is not consumed: the scan stops after the
        // mantissa and leaves the `e` for the unit scanner.
        #[case::truncated_exponent("1e", 1.0)]
        #[case::truncated_exponent_sign("1e+", 1.0)]
        fn scans_decimal_magnitudes(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result =
                Scanner::new(input).number().map_err(|error| error.to_string());
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::empty("")]
        #[case::letters("abc")]
        #[case::unit("h")]
        fn rejects_a_missing_number(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = Scanner::new(input).number().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MissingNumber)));
        }

        #[rstest]
        #[case::bare_decimal_point(".")]
        #[case::decimal_point_then_exponent(".e3")]
        fn rejects_a_malformed_number(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = Scanner::new(input).number().err();
            // Assert
            assert!(matches!(
                error,
                Some(DurationError::MalformedNumber { .. })
            ));
        }

        #[rstest]
        #[case::lone_plus("+")]
        #[case::lone_minus("-")]
        #[case::plus_decimal_point("+.")]
        #[case::minus_decimal_point("-.")]
        fn rejects_a_sign_without_digits(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = Scanner::new(input).number().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MisplacedSign)));
        }

        #[test]
        fn rejects_a_magnitude_that_overflows_to_infinity() {
            // Arrange
            let overflowing = format!("1{}0", "0".repeat(400));
            // Act
            let error = Scanner::new(&overflowing).number().err();
            // Assert
            assert!(matches!(error, Some(DurationError::NumberOverflow)));
        }

        #[test]
        fn stops_after_the_first_decimal_point() {
            // Arrange
            let input = "1.2.3";
            // Act
            let mut scanner = Scanner::new(input);
            let result = scanner.number().map_err(|error| error.to_string());
            // Assert
            assert_eq!(result, Ok(1.2));
            assert_eq!(scanner.position(), 3);
        }
    }

    mod unit_scan {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::hour("h", DurationUnit::Hour)]
        #[case::uppercase_hour("H", DurationUnit::Hour)]
        #[case::minutes("m", DurationUnit::Minute)]
        #[case::months("mo", DurationUnit::Month)]
        #[case::milliseconds("ms", DurationUnit::Millisecond)]
        #[case::long_minutes("mins", DurationUnit::Minute)]
        #[case::long_hours("hours", DurationUnit::Hour)]
        #[case::long_years("yrs", DurationUnit::Year)]
        #[case::long_weeks("wks", DurationUnit::Week)]
        #[case::long_seconds("secs", DurationUnit::Second)]
        fn scans_a_unit_spelling(
            #[case] input: &str,
            #[case] expected: DurationUnit,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result =
                Scanner::new(input).unit().map_err(|error| error.to_string());
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[test]
        fn rejects_an_unknown_unit_spelling() {
            // Arrange
            let input = "fortnight";
            // Act
            let error = Scanner::new(input).unit().err();
            // Assert
            assert!(matches!(error, Some(DurationError::UnknownUnit)));
        }

        #[test]
        fn rejects_an_empty_unit_span() {
            // Arrange
            let input = "";
            // Act
            let error = Scanner::new(input).unit().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MissingUnit)));
        }
    }

    mod component {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::hour("1h", 1.0, DurationUnit::Hour)]
        #[case::minutes("30m", 30.0, DurationUnit::Minute)]
        #[case::spaced_unit("1 h", 1.0, DurationUnit::Hour)]
        #[case::decimal("1.5h", 1.5, DurationUnit::Hour)]
        #[case::exponent("1e3s", 1_000.0, DurationUnit::Second)]
        #[case::uppercase_unit("12H", 12.0, DurationUnit::Hour)]
        #[case::negative("-15m", -15.0, DurationUnit::Minute)]
        fn scans_a_number_and_unit_pair(
            #[case] input: &str,
            #[case] expected_magnitude: f64,
            #[case] expected_unit: DurationUnit,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = Scanner::new(input)
                .component()
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(result, Ok((expected_magnitude, expected_unit)));
        }

        #[rstest]
        #[case::no_unit("1")]
        // The micro sign is not an ASCII letter, so the unit scan is empty.
        #[case::non_ascii_unit("1µs")]
        // A truncated exponent leaves the `e` for the unit scanner, which
        // rejects it as unknown.
        #[case::truncated_exponent("1e")]
        fn rejects_a_component_without_a_known_unit(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = Scanner::new(input).component().err();
            // Assert
            assert!(matches!(
                error,
                Some(DurationError::MissingUnit | DurationError::UnknownUnit)
            ));
        }

        #[test]
        fn rejects_an_unknown_unit_after_a_number() {
            // Arrange
            let input = "1x";
            // Act
            let error = Scanner::new(input).component().err();
            // Assert
            assert!(matches!(error, Some(DurationError::UnknownUnit)));
        }

        #[test]
        fn rejects_a_sign_without_a_magnitude() {
            // Arrange
            let input = "+h";
            // Act
            let error = Scanner::new(input).component().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MisplacedSign)));
        }

        #[test]
        fn rejects_a_missing_number() {
            // Arrange
            let input = "h";
            // Act
            let error = Scanner::new(input).component().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MissingNumber)));
        }

        #[test]
        fn rejects_a_malformed_number() {
            // Arrange
            let input = ".h";
            // Act
            let error = Scanner::new(input).component().err();
            // Assert
            assert!(matches!(
                error,
                Some(DurationError::MalformedNumber { .. })
            ));
        }
    }

    mod skip_separators {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn advances_past_whitespace_and_commas() {
            // Arrange
            let input = " , 1";
            // Act
            let mut scanner = Scanner::new(input);
            scanner.skip_separators();
            // Assert
            assert_eq!(scanner.position(), 3);
        }

        #[test]
        fn stays_put_without_separators() {
            // Arrange
            let input = "1h";
            // Act
            let mut scanner = Scanner::new(input);
            scanner.skip_separators();
            // Assert
            assert_eq!(scanner.position(), 0);
        }

        #[test]
        fn consumes_an_input_of_only_separators() {
            // Arrange
            let input = ", , ";
            // Act
            let mut scanner = Scanner::new(input);
            scanner.skip_separators();
            // Assert
            assert!(scanner.is_at_end());
        }
    }

    mod skip_whitespace {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn advances_past_ascii_whitespace() {
            // Arrange
            let input = "  1";
            // Act
            let mut scanner = Scanner::new(input);
            scanner.skip_whitespace();
            // Assert
            assert_eq!(scanner.position(), 2);
        }

        #[test]
        fn leaves_commas_in_place() {
            // Arrange
            let input = ",1";
            // Act
            let mut scanner = Scanner::new(input);
            scanner.skip_whitespace();
            // Assert
            assert_eq!(scanner.position(), 0);
        }
    }

    mod skip_to_next_component {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn moves_to_the_next_component_across_whitespace() {
            // Arrange
            let mut scanner = Scanner::new("1h 30m");
            let (magnitude, unit) =
                scanner.component().expect("the hour component scans");
            // Act
            let moved = scanner.skip_to_next_component();
            // Assert
            assert_eq!((magnitude, unit), (1.0, DurationUnit::Hour));
            assert!(moved);
            assert_eq!(scanner.position(), 3);
        }

        #[test]
        fn stays_put_at_a_letter() {
            // Arrange
            let input = "x";
            // Act
            let mut scanner = Scanner::new(input);
            let moved = scanner.skip_to_next_component();
            // Assert
            assert!(!moved);
            assert_eq!(scanner.position(), 0);
        }

        #[test]
        fn stays_put_at_the_end_of_input() {
            // Arrange
            let input = "   ";
            // Act
            let mut scanner = Scanner::new(input);
            let moved = scanner.skip_to_next_component();
            // Assert
            assert!(!moved);
            assert_eq!(scanner.position(), 0);
        }

        #[test]
        fn stays_put_at_a_comma() {
            // Arrange: comma separation is handled by `skip_separators`, so
            // the whitespace-only lookahead must not jump across it.
            let mut scanner = Scanner::new("1,30m");
            let scanned = scanner.number().expect("the one scans");
            // Act
            let moved = scanner.skip_to_next_component();
            // Assert
            assert_eq!(scanned.to_bits(), 1.0_f64.to_bits());
            assert!(!moved);
            assert_eq!(scanner.position(), 1);
        }

        #[test]
        fn stays_put_before_a_multibyte_character() {
            // Arrange
            let mut scanner = Scanner::new("1é");
            let scanned = scanner.number().expect("the one scans");
            // Act
            let moved = scanner.skip_to_next_component();
            // Assert
            assert_eq!(scanned.to_bits(), 1.0_f64.to_bits());
            assert!(!moved);
            assert_eq!(scanner.position(), 1);
        }
    }

    mod position {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn starts_at_zero_and_advances_per_byte() {
            // Arrange
            let input = "1e3";
            // Act
            let mut scanner = Scanner::new(input);
            let start = scanner.position();
            let result = scanner.number().map_err(|error| error.to_string());
            // Assert
            assert_eq!(start, 0);
            assert_eq!(result, Ok(1_000.0));
            assert_eq!(scanner.position(), 3);
            assert!(scanner.is_at_end());
        }

        #[test]
        fn reports_the_end_of_an_empty_input() {
            // Arrange
            let input = "";
            // Act
            let scanner = Scanner::new(input);
            // Assert
            assert!(scanner.is_at_end());
            assert_eq!(scanner.position(), 0);
        }

        #[test]
        fn reports_a_cursor_before_the_end_of_input() {
            // Arrange
            let mut scanner = Scanner::new("1h");
            let scanned = scanner.number().expect("the one scans");
            // Act
            let at_end = scanner.is_at_end();
            // Assert
            assert_eq!(scanned.to_bits(), 1.0_f64.to_bits());
            assert!(!at_end);
            assert_eq!(scanner.position(), 1);
        }
    }

    mod scan_mode {

        use super::*;

        #[test]
        fn distinguishes_the_whole_input_and_prefix_variants() {
            // Arrange
            let whole = ScanMode::Whole;
            // Act
            let prefix = ScanMode::Prefix;
            // Assert
            assert_ne!(whole, prefix);
        }
    }
}
