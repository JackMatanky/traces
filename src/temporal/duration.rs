//! A validated duration with a nominal identity and a calendar application
//! regime.
//!
//! Owns [`DurationValue`], the nominal-seconds measure [`DurationSeconds`],
//! and the written components ([`DurationComponent`]) that witness the
//! regime. Every duration-shaped string in the crate is recognized here;
//! callers hold a validated [`DurationValue`] or receive a
//! [`DurationError`].
//!
//! # Identity and application
//!
//! Nominal seconds are the identity of a Duration: one month counts as 30
//! days and one year as 365 days, so equality, ordering, and hashing compare
//! nominal seconds and never the spelling. The written components decide how
//! a value applies: a calendar unit (day, week, month, year) applies on the
//! local clock, while an exact unit (hour and smaller) applies as exact
//! elapsed time. Values synthesized from a magnitude carry no components and
//! always apply exactly.
//!
//! # Grammar
//!
//! Written input is one or more `<number><unit>` components with an optional
//! sign on the whole span, separated by whitespace or commas. Unit
//! spellings are case-insensitive, and a component may carry a fraction or
//! an exponent. Surrounding whitespace is insignificant.

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{self, Write as _},
    hash::Hash,
    ops::{Add, Mul, Neg, Sub},
    str::FromStr,
};

use chrono::TimeDelta;
use num_traits::ToPrimitive as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    error::DurationError,
    scan,
    unit::{CANONICAL_UNITS, DecomposedPart, HUMAN_UNITS, TimeUnit, decompose},
};

/// A validated duration with a nominal identity and a calendar application
/// regime.
///
/// Holds the written spelling, the nominal-seconds identity, and the written
/// components that witness the regime. Parsing guarantees finite nominal
/// seconds; scaling by a non-finite factor can produce non-finite seconds,
/// which consuming conversions reject with
/// [`DurationError::NonFiniteSeconds`]. Equality, ordering, and hashing
/// compare nominal seconds, so two spellings of one elapsed time are one
/// value.
///
/// # Examples
///
/// ```
/// use traces_pkm::{DurationError, DurationValue};
///
/// let written: DurationValue = "1h 30m".parse()?;
/// assert_eq!(written.to_string(), "1h 30m");
///
/// // Equality compares nominal seconds, not spelling.
/// assert_eq!(written, "90m".parse::<DurationValue>()?);
///
/// let error = "not a duration"
///     .parse::<DurationValue>()
///     .expect_err("input does not parse");
/// assert!(matches!(error, DurationError::MissingNumber));
/// assert_eq!(error.to_string(), "no number before unit");
/// # Ok::<(), DurationError>(())
/// ```
#[derive(Clone, Debug)]
pub struct DurationValue {
    spelling: Box<str>,
    seconds: DurationSeconds,
    components: Option<Box<[DurationComponent]>>,
}

impl DurationValue {
    /// Parses a duration string containing one or more `<number><unit>`
    /// components.
    ///
    /// Accepts integers, decimals (`".5h"`), and exponential floats (`"1e3s"`),
    /// optional leading `+` or `-` on the first part, redundant `+` on
    /// subsequent components, and whitespace or comma separators between
    /// components.
    ///
    /// # Errors
    ///
    /// - [`DurationError::Empty`] if `input` is empty or whitespace.
    /// - [`DurationError::MissingNumber`] if a segment has a unit without a
    ///   number.
    /// - [`DurationError::MissingUnit`] if a number appears without a unit.
    /// - [`DurationError::UnknownUnit`] if a unit is unrecognized.
    /// - [`DurationError::MisplacedSign`] if a non-leading part carries a
    ///   negative sign.
    /// - [`DurationError::MalformedNumber`] if float syntax is invalid.
    /// - [`DurationError::NumberOverflow`] if a literal overflows to infinity.
    /// - [`DurationError::NonFiniteSeconds`] if the folded nominal seconds are
    ///   non-finite.
    pub(crate) fn parse(input: &str) -> Result<Self, DurationError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(DurationError::Empty);
        }
        Self::scan(trimmed, scan::ScanMode::Whole).map(|(val, _)| val)
    }

    /// Parses a duration prefix from `input`, returning the parsed
    /// [`DurationValue`] and the number of consumed bytes.
    ///
    /// Recognizes the same `<number><unit>` grammar as [`Self::parse`],
    /// including the leading-sign rule: only the first part may carry an
    /// explicit `-`; a redundant `+` is accepted anywhere. In prefix mode,
    /// commas are treated as atom delimiters rather than part separators,
    /// preserving list precedence at inline note boundaries. Returns `None` if:
    ///
    /// - The input does not start with a valid duration segment, or no
    ///   components could be parsed.
    /// - A part after the first carries an explicit `-` or fails to parse; the
    ///   loop bails on the first error rather than returning the prefix parsed
    ///   so far.
    /// - The folded nominal seconds are not finite.
    pub(crate) fn parse_prefix(input: &str) -> Option<(Self, usize)> {
        Self::scan(input, scan::ScanMode::Prefix).ok()
    }

    /// Synthesizes a canonical [`DurationValue`] from a [`DurationSeconds`]
    /// value.
    ///
    /// Greedily decomposes `seconds.0.abs()` into Weeks, Days, Hours, Minutes,
    /// Seconds, and Milliseconds (ratios from
    /// [`TimeUnit::nominal_seconds`]); Month and Year are omitted because
    /// their lengths are calendar-dependent. Zero yields `"0s"`; a negative
    /// duration starts with `"-"`. When the decomposition cannot represent a
    /// nonzero magnitude (a sub-millisecond remainder rounds to zero, or `u64`
    /// overflows at the other extreme), it falls back to the
    /// [`DurationSeconds`] [`Display`](fmt::Display) so the result never lies
    /// as `"0s"`.
    #[inline]
    pub(crate) fn from_nominal_seconds(seconds: DurationSeconds) -> Self {
        Self {
            spelling: Self::canonical_spelling(seconds),
            seconds,
            components: None,
        }
    }

    /// Renders this duration in human-readable compound English (for example
    /// `"3 days, 4 hours"`), decomposing its magnitude greedily from years
    /// down to seconds over [`HUMAN_UNITS`].
    ///
    /// The magnitude is rounded to whole milliseconds first, so a fractional
    /// remainder renders as one tail millisecond count and a near-whole
    /// magnitude rolls up (`"2 seconds"` for 1.9999 seconds). A non-finite or
    /// sub-half-millisecond magnitude renders as `"0 seconds"` and as
    /// `"< 1 millisecond"` respectively; magnitudes too large for whole-unit
    /// counts render in scientific notation.
    #[must_use]
    pub(crate) fn humanize(&self) -> String {
        let total_secs = self.seconds.as_f64();
        // A non-finite magnitude satisfies no comparison, so the pre-catalog
        // loop fell through to the empty-parts `"0 seconds"`; the guard keeps
        // that pinned shape instead of a `"NaN seconds"` spelling.
        if total_secs == 0.0 || total_secs.is_nan() {
            return "0 seconds".to_owned();
        }
        // `{:e}` carries the sign of the original magnitude, so the
        // scientific fallback keeps it too.
        let Some(total_ms) = (total_secs.abs() * 1_000.0).round().to_u64()
        else {
            return format!("{total_secs:e} seconds");
        };
        let is_negative = total_secs < 0.0;

        let (parts, remainder_ms) = decompose(total_ms, &HUMAN_UNITS);
        let mut text = String::new();
        for part in parts {
            let (singular, plural) = part.unit.human_names();
            let name = if part.count == 1 {
                singular
            } else {
                plural
            };
            if !text.is_empty() {
                text.push_str(", ");
            }
            let _ = write!(text, "{} {name}", part.count);
        }
        if remainder_ms > 0 {
            let name = if remainder_ms == 1 {
                "millisecond"
            } else {
                "milliseconds"
            };
            if !text.is_empty() {
                text.push_str(", ");
            }
            let _ = write!(text, "{remainder_ms} {name}");
        }

        if text.is_empty() {
            return "< 1 millisecond".to_owned();
        }
        if is_negative {
            text.insert(0, '-');
        }
        text
    }

    /// Returns the written components that witness the regime, if any are
    /// retained.
    ///
    /// Values parsed from text keep their written components:
    ///
    /// - [`Some`] containing a calendar unit ([`TimeUnit::Day`],
    ///   [`TimeUnit::Week`], [`TimeUnit::Month`], [`TimeUnit::Year`]): the
    ///   value applies on the local clock.
    /// - [`Some`] with only exact units, or [`None`] (synthesized from a
    ///   magnitude): the value applies as exact elapsed time.
    #[inline]
    #[must_use]
    pub(crate) fn components(&self) -> Option<&[DurationComponent]> {
        self.components.as_deref()
    }

    /// Returns `true` if the written components include a calendar unit.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn has_calendar_component(&self) -> bool {
        self.components().is_some_and(|components| {
            components.iter().any(|component| component.unit.is_calendar())
        })
    }

    /// Returns the written spelling of the duration.
    #[inline]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.spelling
    }

    /// Returns the nominal-seconds identity of the duration.
    #[inline]
    #[must_use]
    pub(crate) const fn nominal_seconds(&self) -> DurationSeconds {
        self.seconds
    }

    /// Scans a duration in either whole-input or prefix mode.
    fn scan(
        input: &str,
        mode: scan::ScanMode,
    ) -> Result<(Self, usize), DurationError> {
        let mut scanner = scan::Scanner::new(input);
        match mode {
            scan::ScanMode::Whole => {
                scanner.skip_separators();
                if scanner.is_at_end() {
                    return Err(DurationError::Empty);
                }
            }
            scan::ScanMode::Prefix => {
                if !scanner.can_start_component() {
                    return Err(DurationError::Empty);
                }
            }
        }

        let mut is_negative = false;
        let mut last_end = 0;
        let mut raw_components = Vec::new();

        while !scanner.is_at_end() {
            let (number, unit) = scanner.component()?;

            if number.is_sign_negative() {
                if !raw_components.is_empty() {
                    return Err(DurationError::MisplacedSign);
                }
                is_negative = true;
            }

            raw_components.push(DurationComponent {
                magnitude: number.abs(),
                unit,
            });
            last_end = scanner.position();

            match mode {
                scan::ScanMode::Whole => {
                    scanner.skip_separators();
                    if scanner.is_at_end() {
                        break;
                    }
                }
                scan::ScanMode::Prefix => {
                    if !scanner.skip_to_next_component() {
                        break;
                    }
                }
            }
        }

        if raw_components.is_empty() {
            return Err(DurationError::Empty);
        }

        if is_negative {
            for component in &mut raw_components {
                component.magnitude = -component.magnitude;
            }
        }

        let total = Self::sum_nominal_seconds(&raw_components);
        let seconds = DurationSeconds::try_from(total)?;
        let spelling = match mode {
            scan::ScanMode::Whole => input.trim().into(),
            scan::ScanMode::Prefix => input[..last_end].trim().into(),
        };

        Ok((
            Self {
                spelling,
                seconds,
                components: Some(raw_components.into_boxed_slice()),
            },
            last_end,
        ))
    }

    /// Folds duration components in left-to-right order to compute nominal
    /// seconds.
    #[inline]
    #[expect(
        clippy::suboptimal_flops,
        reason = "mul_add fuses to a single rounding step and would silently \
                  change user-visible Duration floats that tests pin with \
                  exact equality; not a hot path"
    )]
    fn sum_nominal_seconds(components: &[DurationComponent]) -> f64 {
        components.iter().fold(
            0.0f64,
            |acc,
             &DurationComponent {
                 magnitude,
                 unit,
             }| { acc + magnitude * unit.nominal_seconds() },
        )
    }

    /// Synthesizes the canonical spelling for a fixed-magnitude duration: the
    /// greedy sub-year decomposition of [`CANONICAL_UNITS`], or the
    /// [`DurationSeconds`] [`Display`](fmt::Display) dialect when the
    /// decomposition can't represent the magnitude (see
    /// [`Self::from_nominal_seconds`]).
    ///
    /// Accumulates into one `String`: decomposition emits at most six integer
    /// counts, so no per-part intermediate strings are needed.
    fn canonical_spelling(seconds: DurationSeconds) -> Box<str> {
        let total_secs = seconds.0;
        if total_secs == 0.0 {
            return "0s".into();
        }

        let is_negative = total_secs < 0.0;
        let total_ms =
            (total_secs.abs() * 1_000.0).round().to_u64().unwrap_or(0);
        let (parts, _) = decompose(total_ms, &CANONICAL_UNITS);
        if parts.is_empty() {
            // The decomposition can't represent the magnitude: a sub-half
            // millisecond remainder rounds `total_ms` to zero, and a huge
            // magnitude overflows the `u64` conversion (read back as 0). Fall
            // back to the honest `DurationSeconds` dialect instead of lying
            // with `"0s"`.
            return format!("{}s", DurationSeconds(total_secs))
                .into_boxed_str();
        }

        let mut spelling = String::new();
        if is_negative {
            spelling.push('-');
        }
        for (
            index,
            DecomposedPart {
                unit,
                count,
            },
        ) in parts.into_iter().enumerate()
        {
            if index > 0 {
                spelling.push(' ');
            }
            // Writing the `Display` implementation of `u64` into a `String`
            // is infallible.
            let _ = write!(spelling, "{count}{}", unit.spelling());
        }
        spelling.into_boxed_str()
    }

    /// Combines `rhs` with `self`, negating the components of `rhs` (and
    /// seconds, in the fixed regime) when `subtract` is `true`; the single
    /// implementation behind [`Add`] and [`Sub`].
    ///
    /// - Both operands [`Some`]: components concatenate left-to-right and one
    ///   fold over the combined components preserves the bit-exact Σ invariant
    ///   by construction.
    /// - Either operand [`None`]: seconds add or subtract directly and the
    ///   result carries `None` (fixed regime).
    ///
    /// Negative zero normalizes to positive zero.
    fn combine(self, rhs: Self, subtract: bool) -> Self {
        if let (Some(lhs_components), Some(rhs_components)) =
            (self.components, rhs.components)
        {
            let mut combined = Vec::with_capacity(
                lhs_components.len().saturating_add(rhs_components.len()),
            );
            combined.extend_from_slice(&lhs_components);
            if subtract {
                combined.extend(rhs_components.iter().map(
                    |&DurationComponent {
                         magnitude,
                         unit,
                     }| DurationComponent {
                        magnitude: -magnitude,
                        unit,
                    },
                ));
            } else {
                combined.extend_from_slice(&rhs_components);
            }
            let fold_total = Self::sum_nominal_seconds(&combined);
            let seconds = DurationSeconds::canonicalize_zero(fold_total);
            Self {
                spelling: Self::canonical_spelling(seconds),
                seconds,
                components: Some(combined.into_boxed_slice()),
            }
        } else {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "duration arithmetic preserves total_cmp ordering \
                          and handles overflow without panicking"
            )]
            let seconds = if subtract {
                self.seconds - rhs.seconds
            } else {
                self.seconds + rhs.seconds
            };
            Self {
                spelling: Self::canonical_spelling(seconds),
                seconds,
                components: None,
            }
        }
    }
}

impl FromStr for DurationValue {
    type Err = DurationError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<DurationValue> for TimeDelta {
    type Error = DurationError;

    /// Converts to a [`TimeDelta`] via the nominal seconds of the duration,
    /// returning [`DurationError::NonFiniteSeconds`] when the seconds value is
    /// non-finite and [`DurationError::SecondsOutOfRange`] when it lies outside
    /// the range [`TimeDelta`] can represent.
    #[inline]
    fn try_from(duration: DurationValue) -> Result<Self, Self::Error> {
        Self::try_from(duration.nominal_seconds())
    }
}

/// Renders the value's raw spelling: the original text for parsed values, the
/// canonical sub-year decomposition for synthesized or computed ones.
impl fmt::Display for DurationValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.spelling)
    }
}

/// Compares by nominal seconds, not written spelling: `"1h 30m"` and `"90m"`
/// are equal.
impl PartialEq for DurationValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.seconds == other.seconds
    }
}

impl Eq for DurationValue {}

/// Orders by nominal seconds, not written spelling, consistent with
/// [`PartialEq`].
impl PartialOrd for DurationValue {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DurationValue {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.seconds.cmp(&other.seconds)
    }
}

/// Hashes the nominal seconds, not written spelling, so equal values per
/// [`PartialEq`] always hash identically.
impl Hash for DurationValue {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.seconds.hash(state);
    }
}

impl Add for DurationValue {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.combine(rhs, false)
    }
}

impl Sub for DurationValue {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.combine(rhs, true)
    }
}

/// Scales a duration by a scalar factor.
///
/// With retained components ([`Some`]), each part's magnitude scales by `rhs`
/// and seconds come from the single left-to-right fold over the scaled
/// components; with [`None`], seconds scale directly.
///
/// A non-finite scalar (`NaN` or `±inf`) or overflow yields non-finite seconds,
/// which consuming conversions reject with [`DurationError::NonFiniteSeconds`];
/// this operation never panics.
impl Mul<f64> for DurationValue {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f64) -> Self {
        if let Some(components) = self.components {
            let scaled: Vec<DurationComponent> = components
                .iter()
                .map(
                    |&DurationComponent {
                         magnitude,
                         unit,
                     }| DurationComponent {
                        magnitude: magnitude * rhs,
                        unit,
                    },
                )
                .collect();
            let fold_total = Self::sum_nominal_seconds(&scaled);
            let seconds = DurationSeconds::canonicalize_zero(fold_total);
            Self {
                spelling: Self::canonical_spelling(seconds),
                seconds,
                components: Some(scaled.into_boxed_slice()),
            }
        } else {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "duration scaling preserves non-finite results \
                          without panicking"
            )]
            let seconds = self.seconds * rhs;
            Self {
                spelling: Self::canonical_spelling(seconds),
                seconds,
                components: None,
            }
        }
    }
}

/// Negation; identical to scaling by `-1.0`.
impl Neg for DurationValue {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "duration scaling preserves non-finite results without \
                  panicking"
    )]
    fn neg(self) -> Self {
        self * -1.0
    }
}

/// Scales a duration by a scalar factor.
impl Mul<DurationValue> for f64 {
    type Output = DurationValue;

    #[inline]
    fn mul(self, rhs: DurationValue) -> DurationValue {
        rhs.mul(self)
    }
}

/// Serializes as the original raw spelling, not a canonical form: two equal
/// values (e.g. `"1h 30m"` and `"90m"`) can serialize to different strings.
impl Serialize for DurationValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.spelling)
    }
}

/// Deserializes via the same parser as [`FromStr`], so an unparseable string is
/// a hard deserialization error rather than a lossy fallback.
impl<'de> Deserialize<'de> for DurationValue {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// One written duration magnitude and its unit.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct DurationComponent {
    /// Signed magnitude, with the leading sign applied.
    magnitude: f64,
    /// Unit associated with this magnitude.
    unit: TimeUnit,
}

impl DurationComponent {
    /// Returns the signed magnitude.
    #[inline]
    #[must_use]
    pub(crate) const fn magnitude(self) -> f64 {
        self.magnitude
    }

    /// Returns the unit of the magnitude.
    #[inline]
    #[must_use]
    pub(crate) const fn unit(self) -> TimeUnit {
        self.unit
    }
}

/// A duration measured in seconds.
///
/// Wraps `f64` with NaN-safe ordering and arithmetic; operators do not
/// re-validate finiteness, so overflow can yield a non-finite result. Parsing
/// and [`DurationValue::from_nominal_seconds`] always produce finite values
/// ([`DurationSeconds::try_from`] rejects non-finite input); seconds read back
/// through [`DurationValue::nominal_seconds`] after arithmetic may not be. A
/// signed zero normalizes to positive zero at construction, so `"-0m"` and
/// `"0m"` compare, order, and hash identically.
#[derive(Copy, Clone, Debug)]
pub(crate) struct DurationSeconds(f64);

impl DurationSeconds {
    /// Returns the raw seconds value as `f64`.
    #[inline]
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0
    }

    /// Constructs from a raw `f64`, normalizing a signed zero to positive zero
    /// so the type's invariant (see the type docs) holds everywhere, not just
    /// at parse time.
    #[inline]
    fn canonicalize_zero(value: f64) -> Self {
        Self(if value == 0.0 {
            0.0
        } else {
            value
        })
    }
}

impl TryFrom<f64> for DurationSeconds {
    type Error = DurationError;

    /// Normalizes a signed zero to positive zero (see the type docs).
    #[inline]
    fn try_from(secs: f64) -> Result<Self, Self::Error> {
        secs.is_finite()
            .then(|| Self::canonicalize_zero(secs))
            .ok_or(DurationError::NonFiniteSeconds)
    }
}

impl TryFrom<DurationSeconds> for TimeDelta {
    type Error = DurationError;

    /// Converts to a [`TimeDelta`], returning
    /// [`DurationError::NonFiniteSeconds`] for `NaN`/infinity and
    /// [`DurationError::SecondsOutOfRange`] for a finite value outside the
    /// range [`TimeDelta`] can represent.
    #[inline]
    fn try_from(seconds: DurationSeconds) -> Result<Self, Self::Error> {
        let total = seconds.0;
        if !total.is_finite() {
            return Err(DurationError::NonFiniteSeconds);
        }
        let whole = total.trunc();
        let frac = total.fract();
        let (mut secs, nanos) = if frac < 0.0 {
            (whole - 1.0, (frac + 1.0) * 1_000_000_000.0)
        } else {
            (whole, frac * 1_000_000_000.0)
        };
        let mut nanos_rounded =
            nanos.round().to_i64().ok_or(DurationError::SecondsOutOfRange)?;
        if nanos_rounded >= 1_000_000_000 {
            secs += 1.0;
            nanos_rounded = nanos_rounded.saturating_sub(1_000_000_000);
        }
        let secs_i64 = secs.to_i64().ok_or(DurationError::SecondsOutOfRange)?;
        let nanos_u32 = u32::try_from(nanos_rounded)
            .map_err(|_| DurationError::SecondsOutOfRange)?;
        Self::new(secs_i64, nanos_u32).ok_or(DurationError::SecondsOutOfRange)
    }
}

/// Compares by [`f64::total_cmp`], so a `NaN` operand orders consistently
/// instead of comparing unequal to itself.
impl PartialEq for DurationSeconds {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for DurationSeconds {}

/// Orders through [`Self::cmp`]; see [`PartialEq`](Self::eq) for the total
/// order's rationale.
impl PartialOrd for DurationSeconds {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Orders by [`f64::total_cmp`], the same order [`PartialEq`] compares by.
impl Ord for DurationSeconds {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Renders in scientific notation once the magnitude reaches or clears
/// [`SCIENTIFIC_ABOVE`] or falls below [`SCIENTIFIC_BELOW`] (exact zero
/// excluded), so an extreme magnitude never dumps a hundreds-of-digits decimal
/// literal.
impl fmt::Display for DurationSeconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        let abs = value.abs();
        if abs != 0.0 && !(SCIENTIFIC_BELOW..SCIENTIFIC_ABOVE).contains(&abs) {
            write!(f, "{value:e}")
        } else {
            write!(f, "{value}")
        }
    }
}

/// Hashes the raw bit pattern, which agrees with [`PartialEq`] because
/// [`f64::total_cmp`] returns `Equal` only for identical patterns; normalizing
/// a signed zero at construction is what makes `"-0m"` and `"0m"` one value.
impl Hash for DurationSeconds {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Adds the underlying values, normalizing a resulting signed zero.
impl Add for DurationSeconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::canonicalize_zero(self.0 + rhs.0)
    }
}

/// Subtracts the underlying values, normalizing a resulting signed zero.
impl Sub for DurationSeconds {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::canonicalize_zero(self.0 - rhs.0)
    }
}

/// Scales the underlying value, normalizing a resulting signed zero.
impl Mul<f64> for DurationSeconds {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self::canonicalize_zero(self.0 * rhs)
    }
}

/// Scales a duration by a scalar factor, normalizing a resulting signed zero.
impl Mul<DurationSeconds> for f64 {
    type Output = DurationSeconds;

    #[inline]
    fn mul(self, rhs: DurationSeconds) -> DurationSeconds {
        DurationSeconds::canonicalize_zero(self * rhs.0)
    }
}

/// At or above this magnitude, the [`DurationSeconds`]
/// [`Display`](fmt::Display) switches to scientific notation.
const SCIENTIFIC_ABOVE: f64 = 1e15;

/// Below this magnitude (excluding exact zero), the [`DurationSeconds`]
/// [`Display`](fmt::Display) switches to scientific notation.
const SCIENTIFIC_BELOW: f64 = 1e-6;

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher as _},
    };

    use rstest::rstest;

    use super::*;

    /// Parses a valid duration and projects its nominal seconds, erasing
    /// [`DurationError`] to [`String`] so success rows compare against `Ok`.
    fn parse_seconds(input: &str) -> Result<f64, String> {
        DurationValue::parse(input)
            .map(|duration| duration.nominal_seconds().as_f64())
            .map_err(|error| error.to_string())
    }

    /// Extracts the error variant of a failed parse, if any.
    fn parse_error(input: &str) -> Option<DurationError> {
        DurationValue::parse(input).err()
    }

    /// Parses a valid prefix and projects `(spelling, consumed, seconds)` so
    /// prefix rows compare against one `Ok` triple.
    fn parse_prefix_parts(input: &str) -> Result<(String, usize, f64), String> {
        DurationValue::parse_prefix(input)
            .map(|(duration, consumed)| {
                (
                    duration.to_string(),
                    consumed,
                    duration.nominal_seconds().as_f64(),
                )
            })
            .ok_or_else(|| "prefix rejected".to_owned())
    }

    /// Synthesizes a duration from raw nominal seconds.
    fn synthesized(raw_secs: f64) -> DurationValue {
        DurationValue::from_nominal_seconds(
            DurationSeconds::try_from(raw_secs).expect("finite seconds"),
        )
    }

    /// Computes the default hash of any value.
    fn hash_val<T: Hash>(val: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        val.hash(&mut hasher);
        hasher.finish()
    }

    mod parse {
        use std::error::Error as _;

        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::milliseconds("500ms", 0.5)]
        #[case::seconds("90s", 90.0)]
        #[case::minutes("30m", 1_800.0)]
        #[case::hours("1h", 3_600.0)]
        #[case::days("2d", 172_800.0)]
        #[case::weeks("1w", 604_800.0)]
        #[case::months("3mo", 7_776_000.0)]
        #[case::years("1y", 31_536_000.0)]
        fn parses_a_single_unit_component(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::space("1h 30m", 5_400.0)]
        #[case::comma("1h, 30m", 5_400.0)]
        #[case::tightly_packed("1h30m", 5_400.0)]
        #[case::consecutive_separators("1  h  30  m", 5_400.0)]
        #[case::repeated_unit("1h 1h", 7_200.0)]
        #[case::ascending_order("1m 1h", 3_660.0)]
        #[case::uppercase_unit("1 H", 3_600.0)]
        #[case::comma_without_space("1h,30m", 5_400.0)]
        #[case::fractional_days("1.5d", 129_600.0)]
        #[case::fractional_hours_with_minutes("1.5h 30m", 7_200.0)]
        #[case::fractional_months("1.5mo", 3_888_000.0)]
        #[case::month_then_day("1mo 1d", 2_678_400.0)]
        #[case::day_then_month("1d 1mo", 2_678_400.0)]
        #[case::years_and_weeks("4 yrs, 6 wks", 129_772_800.0)]
        #[case::fractional_scalars("0.5h 15m", 2_700.0)]
        #[case::massive_day_literal("7304484d", 631_107_417_600.0)]
        fn accepts_multi_component_input(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::unsigned("1.5h", 5_400.0)]
        #[case::without_leading_digit(".5h", 1_800.0)]
        #[case::without_leading_digit_seconds(".5s", 0.5)]
        #[case::negative_without_leading_digit("-.5h", -1_800.0)]
        #[case::positive_with_leading_plus("+.5h", 1_800.0)]
        #[case::exponent("1e3s", 1_000.0)]
        #[case::negative_exponent("1e-2h", 36.0)]
        #[case::explicit_exponent_sign("1e+2d", 8_640_000.0)]
        #[case::uppercase_exponent("1E3m", 60_000.0)]
        #[case::exponent_after_decimal(".5e2s", 50.0)]
        fn parses_decimal_and_exponent_magnitudes(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[rstest]
        #[case::plus("+1h", 3_600.0)]
        #[case::minus("-30m", -1_800.0)]
        #[case::leading_sign_on_compound("-1h 30m", -5_400.0)]
        fn applies_the_leading_sign_to_the_whole_span(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(expected));
        }

        #[test]
        fn trims_surrounding_whitespace() {
            // Arrange
            let input = " 1h ";
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(3_600.0));
        }

        #[rstest]
        #[case::trailing_space("1h ")]
        #[case::trailing_comma("1h,")]
        #[case::leading_comma(",1h")]
        fn accepts_stray_separators_at_the_edges(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(3_600.0));
        }

        #[test]
        fn accepts_redundant_plus_on_a_non_leading_part() {
            // Arrange
            let input = "1h +30m";
            // Act
            let result = parse_seconds(input);
            // Assert
            assert_eq!(result, Ok(5_400.0));
        }

        #[test]
        fn preserves_the_written_spelling() {
            // Arrange
            let duration = DurationValue::parse("1h, 30m")
                .expect("separators are valid input");
            // Act
            let spelling = duration.as_str();
            // Assert
            assert_eq!(spelling, "1h, 30m");
        }

        #[rstest]
        #[case::empty("")]
        #[case::whitespace_only("   ")]
        #[case::separator_only(",")]
        fn rejects_input_with_no_component(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::Empty)));
        }

        #[rstest]
        #[case::positive_sign("+h")]
        #[case::negative_sign("-h")]
        #[case::lone_positive_sign("+")]
        #[case::iso_with_leading_minus("-PT1H")]
        fn rejects_a_sign_without_a_magnitude(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::MisplacedSign)));
        }

        #[test]
        fn rejects_a_negative_sign_on_a_non_leading_part() {
            // Arrange
            let input = "1h -30m";
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::MisplacedSign)));
        }

        #[test]
        fn rejects_a_malformed_number_with_a_chained_source() {
            // Arrange
            let error = parse_error(".h").expect("`.h` must fail to parse");
            // Act
            let malformed =
                matches!(error, DurationError::MalformedNumber { .. });
            let has_source = error.source().is_some();
            // Assert
            assert!(malformed);
            assert!(has_source);
        }

        #[rstest]
        #[case::bare_unit("h")]
        #[case::unit_without_number(", h")]
        #[case::bare_iso_unit("PT90M")]
        #[case::iso_designator("P1DT2H")]
        #[case::exponent_without_number("e3s")]
        #[case::text_without_number("hello")]
        fn rejects_a_component_without_a_number(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::MissingNumber)));
        }

        #[rstest]
        #[case::double_decimal("1.2.3h")]
        #[case::bare_number("1")]
        #[case::unit_without_number("1h30")]
        #[case::lone_comma_before_unit("1,h")]
        #[case::spaced_number_without_unit("1 h 30")]
        // The micro sign is not an ASCII letter, so the unit scan is empty.
        #[case::non_ascii_unit("1µs")]
        fn rejects_a_number_without_a_unit(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::MissingUnit)));
        }

        #[rstest]
        #[case::unknown_unit("1x")]
        #[case::unknown_word_unit("1fortnight")]
        #[case::micro_symbol_unit("1us")]
        #[case::longer_than_16_bytes("1aaaaaaaaaaaaaaaaa")]
        // A truncated exponent (`"1e"`, `"1e+"`) is not consumed as a
        // number: the `e` falls to the unit scanner, which rejects it as
        // unknown rather than misreading the value.
        #[case::truncated_exponent("1e")]
        #[case::exponent_sign_without_digits("1e+")]
        // `1ex`: the exponent marker is not followed by a digit or sign, so
        // `ex` falls through to the unit scanner whole.
        #[case::exponent_letter_before_unit("1ex")]
        fn rejects_an_unknown_or_overlong_unit(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let error = parse_error(input);
            // Assert
            assert!(matches!(error, Some(DurationError::UnknownUnit)));
        }

        #[test]
        fn rejects_a_single_part_magnitude_that_overflows_to_infinity() {
            // Arrange
            let overflowing = format!("1{}h", "0".repeat(400));
            // Act
            let error = parse_error(&overflowing);
            // Assert
            assert!(matches!(error, Some(DurationError::NumberOverflow)));
        }

        #[test]
        fn rejects_a_multi_part_fold_that_overflows_to_infinity() {
            // Arrange: each part is finite (1e308 is below `f64::MAX`), but
            // their sum overflows, so the parse-level guard is
            // `NonFiniteSeconds` rather than the per-part `NumberOverflow`.
            let huge = format!("1{}s 1{}s", "0".repeat(308), "0".repeat(308));
            // Act
            let error = parse_error(&huge);
            // Assert
            assert!(matches!(error, Some(DurationError::NonFiniteSeconds)));
        }

        #[rstest]
        #[case::unknown_unit("1fortnight", "unknown unit")]
        #[case::missing_number("hours", "no number before unit")]
        #[case::missing_unit("42", "no unit after number")]
        #[case::empty_input("", "duration is empty")]
        fn formats_the_expected_error_message(
            #[case] input: &str,
            #[case] expected: &str,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = DurationValue::parse(input)
                .map(|duration| duration.to_string())
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(result, Err(expected.to_owned()));
        }

        #[test]
        fn pins_the_unknown_unit_divergence_between_parse_and_prefix() {
            // Arrange
            let input = "1h 1x";
            // Act
            let parsed = parse_error(input);
            let prefixed = parse_prefix_parts(input);
            // Assert
            assert!(matches!(parsed, Some(DurationError::UnknownUnit)));
            assert_eq!(prefixed, Err("prefix rejected".to_owned()));
        }

        #[test]
        fn pins_the_leading_comma_divergence_between_parse_and_prefix() {
            // Arrange
            let input = ",1h";
            // Act
            let parsed = DurationValue::parse(input);
            let prefixed = parse_prefix_parts(input);
            // Assert
            assert_eq!(
                parsed
                    .map(|duration| duration.as_str().to_owned())
                    .map_err(|error| error.to_string()),
                Ok(",1h".to_owned())
            );
            assert_eq!(prefixed, Err("prefix rejected".to_owned()));
        }

        #[test]
        fn pins_the_trailing_comma_divergence_between_parse_and_prefix() {
            // Arrange
            let input = "1h,";
            // Act
            let parsed = DurationValue::parse(input);
            let prefixed = parse_prefix_parts(input);
            // Assert
            assert_eq!(
                parsed
                    .map(|duration| duration.as_str().to_owned())
                    .map_err(|error| error.to_string()),
                Ok("1h,".to_owned())
            );
            assert_eq!(prefixed, Ok(("1h".to_owned(), 2, 3_600.0)));
        }
    }

    mod parse_prefix {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::whole_input("1h", 2, "1h", 3_600.0)]
        #[case::bracket_stops("45m]", 3, "45m", 2_700.0)]
        #[case::decimal(".5h remainder", 3, ".5h", 1_800.0)]
        #[case::multi_part_whitespace("1h 30m, extra", 6, "1h 30m", 5_400.0)]
        #[case::tightly_packed("1h30m rest", 5, "1h30m", 5_400.0)]
        #[case::comma_stops_atom("1h, 30m, extra", 2, "1h", 3_600.0)]
        #[case::negative("-15m extra", 4, "-15m", -900.0)]
        #[case::positive("+15m extra", 4, "+15m", 900.0)]
        #[case::decimal_after_sign("-.5h rest", 4, "-.5h", -1_800.0)]
        #[case::exponent_with_trailing_comma("1e3s,x", 4, "1e3s", 1_000.0)]
        fn parses_a_valid_prefix(
            #[case] input: &str,
            #[case] expected_consumed: usize,
            #[case] expected_str: &str,
            #[case] expected_secs: f64,
        ) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_prefix_parts(input);
            // Assert
            assert_eq!(
                result,
                Ok((expected_str.to_owned(), expected_consumed, expected_secs))
            );
        }

        #[test]
        fn stops_before_a_multibyte_character() {
            // Arrange
            let input = "1h é";
            // Act
            let result = parse_prefix_parts(input);
            // Assert
            assert_eq!(result, Ok(("1h".to_owned(), 2, 3_600.0)));
        }

        #[test]
        fn stops_before_an_attached_symbol() {
            // Arrange
            let input = "1h°C";
            // Act
            let result = parse_prefix_parts(input);
            // Assert
            assert_eq!(result, Ok(("1h".to_owned(), 2, 3_600.0)));
        }

        #[rstest]
        #[case::non_duration("not a duration")]
        #[case::hyphen_bullet("- 15m")]
        #[case::non_leading_negative("1h -30m")]
        #[case::empty("")]
        #[case::clock_time("10:30:00")]
        #[case::unit_attached_to_word("1hello")]
        #[case::bare_number("5")]
        fn rejects_an_invalid_prefix(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let result = parse_prefix_parts(input);
            // Assert
            assert_eq!(result, Err("prefix rejected".to_owned()));
        }
    }

    mod constructor {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::zero(0.0, "0s")]
        #[case::negative_zero(-0.0, "0s")]
        #[case::positive(5_400.0, "1h 30m")]
        #[case::negative(-5_400.0, "-1h 30m")]
        #[case::sub_second(0.5, "500ms")]
        #[case::large_compound(
            604_800.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0 + 0.5,
            "1w 1d 1h 1m 1s 500ms"
        )]
        fn synthesizes_a_canonical_spelling(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            // Arrange: `raw_secs` is the case literal.
            // Act
            let duration = synthesized(raw_secs);
            // Assert
            assert_eq!(duration.as_str(), expected);
            assert_eq!(duration.components(), None);
        }

        #[rstest]
        #[case::seconds(90.0)]
        #[case::minutes(5_400.0)]
        #[case::compound(
            604_800.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0 + 0.5
        )]
        #[case::negative(-5_400.0)]
        #[case::negative_compound(-(86_400.0 + 3_600.0))]
        #[case::scientific_huge(1e300)]
        #[case::scientific_negative_huge(-1e300)]
        #[case::scientific_tiny(1e-300)]
        fn roundtrips_through_the_parser(#[case] raw_secs: f64) {
            // Arrange
            let duration = synthesized(raw_secs);
            // Act
            let reparsed = parse_seconds(duration.as_str());
            // Assert
            assert_eq!(reparsed, Ok(raw_secs));
        }
    }

    mod decomposition {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::zero(0.0, "0s")]
        #[case::sub_minute(59.0, "59s")]
        #[case::exact_minute(60.0, "1m")]
        #[case::sub_hour(3_540.0, "59m")]
        #[case::exact_hour(3_600.0, "1h")]
        #[case::sub_day(82_800.0, "23h")]
        #[case::exact_day(86_400.0, "1d")]
        #[case::sub_week(518_400.0, "6d")]
        #[case::exact_week(604_800.0, "1w")]
        #[case::compound_sub_month(2_592_000.0, "4w 2d")]
        #[case::compound_sub_year(31_536_000.0, "52w 1d")]
        #[case::negative_compound(-5_400.0, "-1h 30m")]
        #[case::millisecond_remainder(0.000_4, "0.0004s")]
        #[case::sub_millisecond(0.000_1, "0.0001s")]
        fn decomposes_nominal_seconds_greedily(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            // Arrange: `raw_secs` is the case literal.
            // Act
            let duration = synthesized(raw_secs);
            // Assert
            assert_eq!(duration.as_str(), expected);
        }

        #[test]
        fn renders_an_unrepresentable_magnitude_in_scientific_notation() {
            // Arrange
            let raw_secs = 1e300;
            // Act
            let duration = synthesized(raw_secs);
            // Assert
            assert_eq!(duration.as_str(), "1e300s");
            assert_ne!(duration.as_str(), "0s");
        }
    }

    mod equality {

        use super::*;

        #[rstest]
        #[case::compound_and_minutes("1h 30m", "90m")]
        #[case::day_and_hours("1d", "24h")]
        #[case::month_and_days("1mo", "30d")]
        fn equates_spellings_with_equal_nominal_seconds(
            #[case] left: &str,
            #[case] right: &str,
        ) {
            // Arrange: `left` and `right` are the case literals.
            let left =
                DurationValue::parse(left).expect("left spelling is valid");
            let right =
                DurationValue::parse(right).expect("right spelling is valid");
            // Act
            let equal = left == right;
            // Assert
            assert!(equal);
        }

        #[test]
        fn distinguishes_different_durations() {
            // Arrange
            let hour = DurationValue::parse("1h").expect("hour is valid");
            let two_hours =
                DurationValue::parse("2h").expect("two hours is valid");
            // Act
            let equal = hour == two_hours;
            // Assert
            assert!(!equal);
        }

        #[test]
        fn treats_negative_zero_as_equal_to_positive_zero() {
            // Arrange
            let negative = synthesized(-0.0);
            let positive = DurationValue::parse("0m")
                .expect("zero minutes is valid input");
            // Act
            let equal = negative == positive;
            // Assert
            assert!(equal);
        }
    }

    mod ordering {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::sub_hour("30m", "1h")]
        #[case::hour_below_day("1h", "1d")]
        fn orders_smaller_spellings_below_larger_ones(
            #[case] small: &str,
            #[case] large: &str,
        ) {
            // Arrange: `small` and `large` are the case literals.
            let small =
                DurationValue::parse(small).expect("small spelling is valid");
            let large =
                DurationValue::parse(large).expect("large spelling is valid");
            // Act
            let ordering = small.partial_cmp(&large);
            // Assert
            assert!(small < large);
            assert!(large > small);
            assert_eq!(ordering, Some(Ordering::Less));
        }

        #[test]
        fn orders_signed_durations_around_zero() {
            // Arrange
            let negative =
                DurationValue::parse("-15m").expect("negative is valid");
            let zero = DurationValue::parse("0m").expect("zero is valid input");
            let positive =
                DurationValue::parse("+15m").expect("positive is valid");
            // Act
            let negative_below_zero = negative < zero;
            let zero_below_positive = zero < positive;
            // Assert
            assert!(negative_below_zero);
            assert!(zero_below_positive);
        }

        #[test]
        fn orders_negative_zero_as_equal_to_positive_zero() {
            // Arrange
            let negative_zero = synthesized(-0.0);
            let zero = DurationValue::parse("0m").expect("zero is valid input");
            // Act
            let at_least = negative_zero >= zero;
            let at_most = negative_zero <= zero;
            // Assert
            assert!(at_least);
            assert!(at_most);
        }
    }

    mod hashing {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn hashes_equal_spellings_identically() {
            // Arrange
            let compound = DurationValue::parse("1h 30m")
                .expect("compound spelling is valid");
            let minutes =
                DurationValue::parse("90m").expect("minutes spelling is valid");
            // Act
            let compound_hash = hash_val(&compound);
            let minutes_hash = hash_val(&minutes);
            // Assert
            assert_eq!(compound_hash, minutes_hash);
        }

        #[test]
        fn hashes_different_durations_differently() {
            // Arrange
            let hour = DurationValue::parse("1h").expect("hour is valid");
            let two_hours = DurationValue::parse("2h").expect("two hours");
            // Act
            let hour_hash = hash_val(&hour);
            let two_hours_hash = hash_val(&two_hours);
            // Assert
            assert_ne!(hour_hash, two_hours_hash);
        }

        #[test]
        fn hashes_equal_seconds_identically() {
            // Arrange
            let left =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            let right =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            // Act
            let left_hash = hash_val(&left);
            let right_hash = hash_val(&right);
            // Assert
            assert_eq!(left_hash, right_hash);
        }

        #[test]
        fn hashes_different_seconds_differently() {
            // Arrange
            let left =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            let right =
                DurationSeconds::try_from(200.0).expect("finite seconds");
            // Act
            let left_hash = hash_val(&left);
            let right_hash = hash_val(&right);
            // Assert
            assert_ne!(left_hash, right_hash);
        }
    }

    mod formatting {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn displays_the_written_spelling() {
            // Arrange
            let duration = DurationValue::parse("4 yrs, 6 wks")
                .expect("valid compound input");
            // Act
            let displayed = format!("{duration}");
            // Assert
            assert_eq!(displayed, "4 yrs, 6 wks");
            assert_eq!(duration.as_str(), "4 yrs, 6 wks");
        }

        #[rstest]
        #[case::decimal(3_600.0, "3600")]
        #[case::fraction(0.5, "0.5")]
        #[case::huge_positive(1e300, "1e300")]
        #[case::huge_negative(-1e300, "-1e300")]
        #[case::tiny_positive(1e-300, "1e-300")]
        #[case::at_upper_threshold(SCIENTIFIC_ABOVE, "1e15")]
        #[case::just_below_upper_threshold(1e14, "100000000000000")]
        #[case::at_lower_threshold(SCIENTIFIC_BELOW, "0.000001")]
        #[case::just_above_lower_threshold(0.0004, "0.0004")]
        fn switches_seconds_display_dialect_by_magnitude(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            // Arrange: `raw_secs` is the case literal.
            let seconds =
                DurationSeconds::try_from(raw_secs).expect("finite seconds");
            // Act
            let displayed = seconds.to_string();
            // Assert
            assert_eq!(displayed, expected);
        }
    }

    mod humanize {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::zero(0.0, "0 seconds")]
        #[case::compound(5_400.0, "1 hour, 30 minutes")]
        #[case::negative(-5_400.0, "-1 hour, 30 minutes")]
        #[case::months(2_592_000.0, "1 month")]
        #[case::years(31_536_000.0, "1 year")]
        #[case::large_compound(
            604_800.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0 + 0.5,
            "1 week, 1 day, 1 hour, 1 minute, 1 second, 500 milliseconds"
        )]
        #[case::sub_millisecond(0.000_4, "< 1 millisecond")]
        #[case::scientific_huge(1e300, "1e300 seconds")]
        fn renders_compound_english_from_nominal_seconds(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            // Arrange: `raw_secs` is the case literal.
            // Act
            let duration = synthesized(raw_secs);
            // Assert
            assert_eq!(duration.humanize(), expected);
        }

        #[test]
        fn renders_a_non_finite_magnitude_as_zero_seconds() {
            // Arrange: a raw non-finite `DurationSeconds` bypasses the
            // finite-only `try_from` that scaling already rejects.
            let duration =
                DurationValue::from_nominal_seconds(DurationSeconds(f64::NAN));
            // Act
            let humanized = duration.humanize();
            // Assert
            assert_eq!(humanized, "0 seconds");
        }

        #[rstest]
        #[case::second_roll_up(1.999_9, "2 seconds")]
        #[case::minute_boundary(59.999_9, "1 minute")]
        fn rolls_a_near_whole_magnitude_into_the_next_unit(
            #[case] raw_secs: f64,
            #[case] expected: &str,
        ) {
            // Arrange: the magnitude rounds to a whole unit boundary before
            // decomposition, so no `"…, 1000 milliseconds"` tail exists.
            // Act
            let duration = synthesized(raw_secs);
            // Assert
            assert_eq!(duration.humanize(), expected);
        }

        #[test]
        fn renders_a_one_millisecond_tail_in_the_singular() {
            // Arrange: 1 millisecond is below every human unit; the tail count
            // is 1. Act
            let humanized = synthesized(0.001).humanize();
            // Assert
            assert_eq!(humanized, "1 millisecond");
        }
    }

    mod serialization {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn serializes_to_the_written_spelling() {
            // Arrange
            let duration =
                DurationValue::parse("1h 30m").expect("compound is valid");
            // Act
            let json = serde_json::to_string(&duration)
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(json, Ok("\"1h 30m\"".to_owned()));
        }

        #[test]
        fn keeps_json_and_postcard_spellings_per_value() {
            // Arrange
            let compound =
                DurationValue::parse("1h 30m").expect("compound is valid");
            let minutes =
                DurationValue::parse("90m").expect("minutes are valid");
            // Act
            let compound_json = serde_json::to_string(&compound)
                .map_err(|error| error.to_string());
            let minutes_json = serde_json::to_string(&minutes)
                .map_err(|error| error.to_string());
            let compound_bytes = postcard::to_allocvec(&compound)
                .map_err(|error| error.to_string());
            let minutes_bytes = postcard::to_allocvec(&minutes)
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(compound_json, Ok("\"1h 30m\"".to_owned()));
            assert_eq!(minutes_json, Ok("\"90m\"".to_owned()));
            assert_eq!(compound_bytes, Ok(b"\x061h 30m".to_vec()));
            assert_eq!(minutes_bytes, Ok(b"\x0390m".to_vec()));
        }
    }

    mod deserialization {
        use serde::Deserialize;

        use super::*;

        #[test]
        fn deserializes_from_a_string() {
            // Arrange
            let json = "\"4 yrs, 6 wks\"";
            // Act
            let duration: Result<String, String> =
                serde_json::from_str::<DurationValue>(json)
                    .map(|value| value.as_str().to_owned())
                    .map_err(|error| error.to_string());
            // Assert
            assert_eq!(duration, Ok("4 yrs, 6 wks".to_owned()));
            assert_eq!(parse_seconds("4 yrs, 6 wks"), Ok(129_772_800.0));
        }

        #[test]
        fn rejects_an_unparseable_string() {
            // Arrange
            let json = "\"not a duration\"";
            // Act
            let result = serde_json::from_str::<DurationValue>(json)
                .map_err(|error| error.to_string());
            // Assert
            assert!(result.unwrap_err().contains("no number before unit"));
        }

        #[test]
        fn renders_an_extreme_duration_in_scientific_notation() {
            #[derive(Deserialize)]
            struct ExtremeNote {
                duration: DurationValue,
            }

            // Arrange
            let extreme_digits = "9".repeat(50);
            let yaml = format!("duration: \"{extreme_digits}y\"\n");
            let note = noyalib::from_str::<ExtremeNote>(&yaml)
                .expect("extreme duration deserializes");
            // Act
            let rendered =
                synthesized(note.duration.nominal_seconds().as_f64())
                    .to_string();
            // Assert
            assert_ne!(rendered, "0s");
            assert!(rendered.contains('e'));
        }

        #[test]
        fn normalizes_a_negative_zero_duration_to_zero() {
            #[derive(Deserialize)]
            struct ZeroNote {
                duration: DurationValue,
            }

            // Arrange
            let yaml = "duration: \"-0m\"\n";
            let note = noyalib::from_str::<ZeroNote>(yaml)
                .expect("negative-zero duration deserializes");
            let zero = DurationValue::parse("0m").expect("zero is valid");
            // Act
            let equal = note.duration == zero;
            // Assert
            assert!(equal);
        }
    }

    mod conversions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn roundtrips_valid_input_through_from_str() {
            // Arrange
            let duration: DurationValue =
                "1h 30m".parse().expect("compound is valid");
            // Act
            let seconds = duration.nominal_seconds();
            // Assert
            assert_eq!(
                seconds,
                DurationSeconds::try_from(5_400.0).expect("finite seconds")
            );
        }

        #[test]
        fn returns_the_expected_error_for_invalid_input() {
            // Arrange
            let input = "not a duration";
            // Act
            let error = input.parse::<DurationValue>().err();
            // Assert
            assert!(matches!(error, Some(DurationError::MissingNumber)));
        }
    }

    mod components {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn components_carry_the_written_magnitude_with_the_span_sign() {
            // Arrange
            let duration = DurationValue::parse("-1h 30m")
                .expect("signed compound is valid");
            let components =
                duration.components().expect("parser retains components");
            // Act
            let first = components.first().copied();
            let second = components.get(1).copied();
            // Assert
            assert_eq!(components.len(), 2);
            assert_eq!(
                first,
                Some(DurationComponent {
                    magnitude: -1.0,
                    unit: TimeUnit::Hour,
                })
            );
            assert_eq!(
                second,
                Some(DurationComponent {
                    magnitude: -30.0,
                    unit: TimeUnit::Minute,
                })
            );
        }

        #[test]
        fn prefix_retains_the_same_components_as_a_full_parse() {
            // Arrange
            let (prefixed, consumed) =
                DurationValue::parse_prefix("-1h 30m trailing")
                    .expect("valid prefix");
            let full = DurationValue::parse("-1h 30m").expect("valid compound");
            // Act
            let same_components = prefixed.components() == full.components();
            let same_seconds =
                prefixed.nominal_seconds() == full.nominal_seconds();
            // Assert
            assert_eq!(consumed, 7);
            assert!(same_components);
            assert!(same_seconds);
        }

        #[rstest]
        #[case::simple("1h 30m", 5_400.0)]
        #[case::negative("-1h 30m", -5_400.0)]
        #[case::mixed_units("4 yrs, 6 wks", 129_772_800.0)]
        #[case::sub_second("500ms", 0.5)]
        #[case::fractional_scalars("0.5h 15m", 2_700.0)]
        #[case::float_non_associative("10000000000000000s 1s 1s", 1.0e16)]
        fn stores_the_exact_literal_seconds(
            #[case] input: &str,
            #[case] expected: f64,
        ) {
            // Arrange: `input` is the case literal.
            let duration =
                DurationValue::parse(input).expect("case input is valid");
            // Act
            let bits = duration.nominal_seconds().0.to_bits();
            // Assert
            assert_eq!(bits, expected.to_bits());
        }

        #[test]
        fn identifies_calendar_and_exact_regimes_from_components() {
            // Arrange
            let calendar = DurationValue::parse("1mo 2d")
                .expect("calendar compound is valid");
            let exact = DurationValue::parse("2h 30m")
                .expect("exact compound is valid");
            // Act
            let calendar_regime = calendar.has_calendar_component();
            let exact_regime = exact.has_calendar_component();
            let synthesized_regime =
                synthesized(86_400.0).has_calendar_component();
            // Assert
            assert!(calendar_regime);
            assert!(!exact_regime);
            assert!(!synthesized_regime);
        }
    }

    mod arithmetic {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn add_and_sub_combine_components_for_component_operands() {
            // Arrange
            let expected =
                DurationValue::parse("1h 30m").expect("compound is valid");
            let hour = DurationValue::parse("1h").expect("hour is valid");
            let minutes =
                DurationValue::parse("30m").expect("minutes are valid");
            // Act
            let sum = hour + minutes.clone();
            let diff = expected.clone() - minutes;
            // Assert: equality is by nominal seconds, and `Sub` keeps the
            // negated rhs entry as the written witness.
            assert_eq!(sum, expected);
            assert_eq!(
                diff.nominal_seconds(),
                DurationSeconds::try_from(3_600.0).expect("finite seconds")
            );
            let components = diff.components().expect("Some plus Some retains");
            assert_eq!(components.len(), 3);
            assert_eq!(
                components.last().copied(),
                Some(DurationComponent {
                    magnitude: -30.0,
                    unit: TimeUnit::Minute,
                })
            );
        }

        #[test]
        fn stores_the_exact_literal_seconds_after_addition() {
            // Arrange
            let left = DurationValue::parse("10000000000000000s")
                .expect("large literal is valid");
            let right = DurationValue::parse("1s 1s")
                .expect("repeated seconds are valid");
            // Act
            let sum = left + right;
            // Assert
            assert_eq!(sum.nominal_seconds().0.to_bits(), 1.0e16_f64.to_bits());
        }

        #[test]
        fn an_add_operand_without_components_clears_the_witness() {
            // Arrange
            let magnitude_only = synthesized(3_600.0);
            let written =
                DurationValue::parse("30m").expect("minutes are valid");
            // Act
            let sum = magnitude_only + written;
            // Assert
            assert!(sum.components().is_none());
            assert_eq!(
                sum.nominal_seconds(),
                DurationSeconds::try_from(5_400.0).expect("finite seconds")
            );
        }

        #[test]
        fn a_sub_operand_without_components_clears_the_witness() {
            // Arrange
            let magnitude_only = synthesized(3_600.0);
            let written =
                DurationValue::parse("30m").expect("minutes are valid");
            // Act
            let diff = magnitude_only - written;
            // Assert: mixed operands drop the written witness and the
            // seconds subtract directly.
            assert!(diff.components().is_none());
            assert_eq!(
                diff.nominal_seconds(),
                DurationSeconds::try_from(1_800.0).expect("finite seconds")
            );
        }

        #[test]
        fn add_synthesizes_a_canonical_spelling_for_borrowed_hours() {
            // Arrange
            let left = DurationValue::parse("1d").expect("day is valid");
            let right = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let sum = left + right;
            let components = sum.components();

            // Assert
            assert_eq!(sum.as_str(), "1d 1h");
            assert_eq!(components.map(<[DurationComponent]>::len), Some(2));
            assert_eq!(
                components.and_then(|entries| entries.first().copied()),
                Some(DurationComponent {
                    magnitude: 1.0,
                    unit: TimeUnit::Day,
                })
            );
            assert_eq!(
                components.and_then(|entries| entries.get(1).copied()),
                Some(DurationComponent {
                    magnitude: 1.0,
                    unit: TimeUnit::Hour,
                })
            );
        }

        #[test]
        fn add_decomposes_borrowed_hours_into_a_day() {
            // Arrange
            let left = DurationValue::parse("23h").expect("hours are valid");
            let right = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let sum = left + right;
            let components = sum.components();
            // Assert: the written witness concatenates while the spelling
            // decomposes by nominal seconds.
            assert_eq!(sum.as_str(), "1d");
            assert_eq!(components.map(<[DurationComponent]>::len), Some(2));
        }

        #[test]
        fn sub_folds_compound_minutes_back_to_zero() {
            // Arrange
            let compound =
                DurationValue::parse("1h 30m").expect("compound is valid");
            let minutes =
                DurationValue::parse("90m").expect("minutes are valid");
            // Act
            let diff = compound - minutes;
            // Assert
            assert_eq!(diff.as_str(), "0s");
            assert_eq!(
                diff.nominal_seconds(),
                DurationSeconds::try_from(0.0).expect("finite seconds")
            );
        }

        #[test]
        fn neg_equals_scaling_by_minus_one() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let negated = -duration.clone();
            let scaled = duration * -1.0;
            // Assert
            assert_eq!(
                negated,
                DurationValue::parse("-1h").expect("negative hour is valid")
            );
            assert_eq!(negated.as_str(), scaled.as_str());
        }
    }

    mod scaling {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn scales_each_component_and_folds_the_seconds() {
            // Arrange
            let duration =
                DurationValue::parse("1h 30m").expect("compound is valid");
            // Act
            let scaled = duration * 2.5;
            let components = scaled.components();
            // Assert
            assert_eq!(components.map(<[DurationComponent]>::len), Some(2));
            assert_eq!(
                components.and_then(|entries| entries.first().copied()),
                Some(DurationComponent {
                    magnitude: 2.5,
                    unit: TimeUnit::Hour,
                })
            );
            assert_eq!(
                components.and_then(|entries| entries.get(1).copied()),
                Some(DurationComponent {
                    magnitude: 75.0,
                    unit: TimeUnit::Minute,
                })
            );
            assert_eq!(
                scaled.nominal_seconds().0.to_bits(),
                13_500.0_f64.to_bits()
            );
        }

        #[test]
        fn scales_the_component_magnitudes_without_decomposing_them() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let scaled = duration * 0.5;
            let components = scaled.components();
            // Assert: the witness keeps the scaled literal while the
            // spelling decomposes by nominal seconds.
            assert_eq!(
                components.and_then(|entries| entries.first().copied()),
                Some(DurationComponent {
                    magnitude: 0.5,
                    unit: TimeUnit::Hour,
                })
            );
            assert_eq!(scaled.as_str(), "30m");
        }

        #[test]
        fn scaling_by_zero_folds_to_zero() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let scaled = duration * 0.0;
            // Assert
            assert_eq!(scaled.as_str(), "0s");
            assert_eq!(
                scaled.nominal_seconds(),
                DurationSeconds::try_from(0.0).expect("finite seconds")
            );
        }

        #[test]
        fn scaling_by_a_negative_factor_rewrites_the_spelling() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let scaled = duration * -2.0;
            // Assert
            assert_eq!(scaled.as_str(), "-2h");
        }

        #[test]
        fn a_non_finite_scalar_yields_non_finite_seconds_without_panicking() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let infinite = duration.clone() * f64::INFINITY;
            let not_a_number = duration * f64::NAN;
            // Assert
            assert!(!infinite.nominal_seconds().0.is_finite());
            assert_eq!(infinite.as_str(), "infs");
            assert!(matches!(
                TimeDelta::try_from(infinite),
                Err(DurationError::NonFiniteSeconds)
            ));
            assert!(!not_a_number.nominal_seconds().0.is_finite());
            assert!(matches!(
                TimeDelta::try_from(not_a_number),
                Err(DurationError::NonFiniteSeconds)
            ));
        }

        #[test]
        fn overflow_leaves_non_finite_seconds_rejected_by_conversions() {
            // Arrange: `1s` has a unit ratio of exactly 1, so scaling by
            // `f64::MAX` stays finite and the next doubling overflows.
            let duration = DurationValue::parse("1s").expect("second is valid");
            // Act
            let huge = duration * f64::MAX;
            let overflowed = huge.clone() * 2.0;
            // Assert: the overflow leaves non-finite seconds that every
            // consuming conversion rejects, and the spelling never lies as
            // `"0s"`.
            assert!(huge.nominal_seconds().0.is_finite());
            assert!(!overflowed.nominal_seconds().0.is_finite());
            assert_eq!(overflowed.as_str(), "infs");
            assert!(matches!(
                TimeDelta::try_from(overflowed),
                Err(DurationError::NonFiniteSeconds)
            ));
        }
    }

    mod seconds {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn adds_and_subtracts_seconds() {
            // Arrange
            let left =
                DurationSeconds::try_from(300.0).expect("finite seconds");
            let right =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            // Act
            let sum = left + right;
            let diff = left - right;
            // Assert
            assert_eq!(
                sum,
                DurationSeconds::try_from(400.0).expect("finite seconds")
            );
            assert_eq!(
                diff,
                DurationSeconds::try_from(200.0).expect("finite seconds")
            );
        }

        #[test]
        fn scales_seconds_from_either_operand_order() {
            // Arrange
            let seconds =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            // Act
            let scaled = seconds * 3.0;
            let mirrored = 3.0 * seconds;
            // Assert
            assert_eq!(
                scaled,
                DurationSeconds::try_from(300.0).expect("finite seconds")
            );
            assert_eq!(mirrored, scaled);
        }

        #[test]
        fn normalizes_a_resulting_signed_zero_from_either_order() {
            // Arrange
            let zero = DurationSeconds::try_from(0.0).expect("finite seconds");
            // Act
            let scaled = zero * -1.0;
            let mirrored = -1.0 * zero;
            // Assert
            assert_eq!(scaled, zero);
            assert_eq!(mirrored, zero);
            assert!(scaled >= zero);
        }

        #[test]
        fn orders_by_total_cmp() {
            // Arrange
            let left =
                DurationSeconds::try_from(100.0).expect("finite seconds");
            let right =
                DurationSeconds::try_from(200.0).expect("finite seconds");
            // Act
            let ordering = left.cmp(&right);
            // Assert
            assert!(left < right);
            assert!(right > left);
            assert_eq!(ordering, Ordering::Less);
        }

        #[test]
        fn treats_negative_zero_as_equal_to_positive_zero() {
            // Arrange
            let negative =
                DurationSeconds::try_from(-0.0).expect("finite seconds");
            let positive =
                DurationSeconds::try_from(0.0).expect("finite seconds");
            // Act
            let equal = negative == positive;
            // Assert
            assert!(equal);
            assert!(negative >= positive && positive >= negative);
        }

        #[test]
        fn rejects_non_finite_input_at_construction() {
            // Arrange
            let not_a_number = f64::NAN;
            let positive_infinity = f64::INFINITY;
            let negative_infinity = f64::NEG_INFINITY;
            // Act
            let nan = DurationSeconds::try_from(not_a_number).err();
            let infinity = DurationSeconds::try_from(positive_infinity).err();
            let neg_infinity =
                DurationSeconds::try_from(negative_infinity).err();
            // Assert
            assert!(matches!(nan, Some(DurationError::NonFiniteSeconds)));
            assert!(matches!(infinity, Some(DurationError::NonFiniteSeconds)));
            assert!(matches!(
                neg_infinity,
                Some(DurationError::NonFiniteSeconds)
            ));
        }

        #[test]
        fn extracts_nominal_seconds_from_a_duration_value() {
            // Arrange
            let duration = DurationValue::parse("1h").expect("hour is valid");
            // Act
            let seconds = duration.nominal_seconds();
            // Assert
            assert_eq!(
                seconds,
                DurationSeconds::try_from(3_600.0).expect("finite seconds")
            );
        }
    }

    mod time_delta {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::whole_hour("1h", TimeDelta::hours(1))]
        #[case::subsecond("500ms", TimeDelta::milliseconds(500))]
        fn converts_duration_values_in_range(
            #[case] input: &str,
            #[case] expected: TimeDelta,
        ) {
            // Arrange: `input` is the case literal.
            let duration =
                DurationValue::parse(input).expect("case input is valid");
            // Act
            let converted = TimeDelta::try_from(duration)
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(converted, Ok(expected));
        }

        #[rstest]
        #[case::zero(0.0, TimeDelta::zero())]
        #[case::negative_whole(-10.0, TimeDelta::seconds(-10))]
        #[case::negative_fractional(-90.5, TimeDelta::milliseconds(-90_500))]
        #[case::sub_nanosecond(0.000_000_001, TimeDelta::nanoseconds(1))]
        fn converts_duration_seconds_in_range(
            #[case] raw_secs: f64,
            #[case] expected: TimeDelta,
        ) {
            // Arrange: `raw_secs` is the case literal.
            let seconds =
                DurationSeconds::try_from(raw_secs).expect("finite seconds");
            // Act
            let converted =
                TimeDelta::try_from(seconds).map_err(|error| error.to_string());
            // Assert
            assert_eq!(converted, Ok(expected));
        }

        #[rstest]
        #[case::extreme_positive(1e300)]
        #[case::extreme_negative(-1e300)]
        fn rejects_seconds_outside_the_representable_range(
            #[case] raw_secs: f64,
        ) {
            // Arrange: `raw_secs` is the case literal.
            let seconds =
                DurationSeconds::try_from(raw_secs).expect("finite seconds");
            // Act
            let result = TimeDelta::try_from(seconds);
            // Assert
            assert!(matches!(result, Err(DurationError::SecondsOutOfRange)));
        }
    }
}
