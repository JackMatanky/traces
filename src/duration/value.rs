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

use super::{scan, unit::DurationUnit};

/// At or above this magnitude, [`DurationSeconds`]'s [`Display`](fmt::Display)
/// switches to scientific notation.
const SCIENTIFIC_ABOVE: f64 = 1e15;

/// Below this magnitude (excluding exact zero), [`DurationSeconds`]'s
/// [`Display`](fmt::Display) switches to scientific notation.
const SCIENTIFIC_BELOW: f64 = 1e-6;

/// A validated duration expression with its total seconds and original source
/// spelling.
///
/// Fresh values come from the crate-internal `parse`, `parse_prefix`, and
/// `from_nominal_seconds` constructors or the [`FromStr`] implementation;
/// `Add`, `Sub`, and `Mul` derive new values from those. Parsing guarantees a
/// finite seconds value; arithmetic such as [`Mul`] may produce non-finite
/// values, which consuming conversions reject.
///
/// # Examples
///
/// ```rust
/// use traces_pkm::DurationValue;
///
/// let written: DurationValue = "1h 30m".parse().expect("valid duration");
/// assert_eq!(written.to_string(), "1h 30m");
///
/// // Equality compares total seconds, not spelling.
/// assert_eq!(written, "90m".parse().expect("same elapsed time"));
///
/// let err = "not a duration"
///     .parse::<DurationValue>()
///     .expect_err("unparseable input");
/// assert!(err.to_string().contains("no number before unit"));
/// ```
#[derive(Clone, Debug)]
pub struct DurationValue {
    spelling: Box<str>,
    seconds: DurationSeconds,
    components: Option<Components>,
}

/// The six fixed sub-year units used for greedy millisecond decomposition in
/// [`DurationValue::from_nominal_seconds`].
///
/// Month and Year are intentionally omitted from magnitude-derived conversion:
/// their lengths are calendar-dependent and cannot be soundly synthesized from
/// a raw magnitude.
const DECOMPOSITION_ORDER: [DurationUnit; 6] = [
    DurationUnit::Week,
    DurationUnit::Day,
    DurationUnit::Hour,
    DurationUnit::Minute,
    DurationUnit::Second,
    DurationUnit::Millisecond,
];

/// Units, largest first, used by [`DurationValue::humanize`].
const HUMAN_ORDER: [DurationUnit; 7] = [
    DurationUnit::Year,
    DurationUnit::Month,
    DurationUnit::Week,
    DurationUnit::Day,
    DurationUnit::Hour,
    DurationUnit::Minute,
    DurationUnit::Second,
];

/// Renders a magnitude too large for whole-unit counts in scientific notation.
fn scientific_seconds_text(total_secs: f64, is_negative: bool) -> String {
    let sign = if is_negative {
        "-"
    } else {
        ""
    };
    format!("{sign}{total_secs:e} seconds")
}

impl DurationValue {
    /// Classifies `s` as a duration expression, returning `None` if it cannot
    /// start a duration.
    ///
    /// Employs a cheap `O(1)` shape gate upfront that returns `None` without
    /// allocation when the first byte cannot begin a duration segment, or
    /// when `s` is empty or whitespace. Otherwise delegates to
    /// [`Self::parse`].
    ///
    /// # Errors
    ///
    /// Returns `Some(Err(_))` with:
    ///
    /// - [`DurationError::MissingNumber`] if a segment has a unit without a
    ///   number.
    /// - [`DurationError::MissingUnit`] if a number appears without a unit.
    /// - [`DurationError::UnknownUnit`] if a unit is unrecognized.
    /// - [`DurationError::InvalidNumber`] if a non-leading part carries a
    ///   negative sign.
    /// - [`DurationError::MalformedNumber`] if float syntax is invalid.
    /// - [`DurationError::NonFiniteNumber`] if a literal overflows to infinity.
    /// - [`DurationError::NonFiniteSeconds`] if the folded total is non-finite.
    #[must_use]
    pub(crate) fn classify(s: &str) -> Option<Result<Self, DurationError>> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || !scan::can_start_component(bytes, 0) {
            return None;
        }
        Some(Self::parse(trimmed))
    }

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
    /// - [`DurationError::InvalidNumber`] if a non-leading part carries a
    ///   negative sign.
    /// - [`DurationError::MalformedNumber`] if float syntax is invalid.
    /// - [`DurationError::NonFiniteNumber`] if a literal overflows to infinity.
    /// - [`DurationError::NonFiniteSeconds`] if the folded total is non-finite.
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
    /// preserving list precedence at inline note boundaries. Returns `None`
    /// if:
    ///
    /// - The input does not start with a valid duration segment, or no
    ///   components could be parsed.
    /// - A part after the first carries an explicit `-` or fails to parse; the
    ///   loop bails on the first error rather than returning the prefix parsed
    ///   so far.
    /// - The folded total is not finite.
    pub(crate) fn parse_prefix(input: &str) -> Option<(Self, usize)> {
        Self::scan(input, scan::ScanMode::Prefix).ok()
    }

    /// Scans a duration in either whole-input or prefix mode.
    fn scan(
        input: &str,
        mode: scan::ScanMode,
    ) -> Result<(Self, usize), DurationError> {
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut pos = 0;
        match mode {
            scan::ScanMode::Whole => {
                scan::skip_separators(bytes, &mut pos);
                if pos >= len {
                    return Err(DurationError::Empty);
                }
            }
            scan::ScanMode::Prefix => {
                if len == 0 || !scan::can_start_component(bytes, pos) {
                    return Err(DurationError::Empty);
                }
            }
        }

        let mut parsed_any = false;
        let mut is_negative = false;
        let mut last_end = 0;
        let mut raw_components = Vec::new();

        while pos < len {
            let (number, kind, pos_after_part) =
                scan::scan_component(bytes, pos, input)?;
            pos = pos_after_part;

            if number.is_sign_negative() {
                if parsed_any {
                    return Err(DurationError::InvalidNumber {
                        input: input.to_owned(),
                    });
                }
                is_negative = true;
            }

            raw_components.push(DurationComponent {
                magnitude: number.abs(),
                unit: kind,
            });
            parsed_any = true;
            last_end = pos;

            match mode {
                scan::ScanMode::Whole => {
                    scan::skip_separators(bytes, &mut pos);
                    if pos >= len {
                        break;
                    }
                }
                scan::ScanMode::Prefix => {
                    let mut next_pos = pos;
                    scan::skip_whitespace(bytes, &mut next_pos);
                    if next_pos < len
                        && scan::can_start_component(bytes, next_pos)
                    {
                        pos = next_pos;
                    } else {
                        break;
                    }
                }
            }
        }

        if !parsed_any {
            return Err(DurationError::Empty);
        }

        scan::apply_leading_sign(&mut raw_components, is_negative);

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

    /// Folds duration components in left-to-right order to compute total
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

    /// Synthesizes a canonical [`DurationValue`] from a [`DurationSeconds`]
    /// value.
    ///
    /// Greedily decomposes `seconds.0.abs()` into Weeks, Days, Hours, Minutes,
    /// Seconds, and Milliseconds (ratios from
    /// [`DurationUnit::nominal_seconds`]); Month and Year are omitted
    /// because their lengths are calendar-dependent. Zero yields `"0s"`; a
    /// negative duration starts with `"-"`. When the decomposition cannot
    /// represent a nonzero magnitude (a sub-millisecond remainder rounds to
    /// zero, or `u64` overflows at the other extreme), falls back to
    /// [`DurationSeconds`]'s [`Display`](fmt::Display) so the result never
    /// lies as `"0s"`.
    #[inline]
    pub(crate) fn from_nominal_seconds(seconds: DurationSeconds) -> Self {
        Self {
            spelling: Self::canonical_spelling(seconds),
            seconds,
            components: None,
        }
    }

    /// Synthesizes the canonical spelling for a fixed-magnitude duration: the
    /// greedy sub-year decomposition of [`DECOMPOSITION_ORDER`], or
    /// [`DurationSeconds`]'s [`Display`](fmt::Display) dialect when the
    /// decomposition can't represent the magnitude (see
    /// [`Self::from_nominal_seconds`]).
    ///
    /// Accumulates into one `String`: decomposition emits at most six integer
    /// counts, so no per-part intermediate strings are needed.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "unit_ms is non-zero (>= 1) and total_ms >= unit_ms, so \
                  division and modulo never panic"
    )]
    fn canonical_spelling(seconds: DurationSeconds) -> Box<str> {
        let total_secs = seconds.0;
        if total_secs == 0.0 {
            return "0s".into();
        }

        let is_negative = total_secs < 0.0;
        let rem = total_secs.abs();
        let mut total_ms = (rem * 1_000.0).round().to_u64().unwrap_or(0);
        let mut spelling = String::new();
        if is_negative {
            spelling.push('-');
        }
        let mut decomposed = false;
        for unit in DECOMPOSITION_ORDER {
            let unit_ms = (unit.nominal_seconds() * 1_000.0)
                .round()
                .to_u64()
                .unwrap_or(0);
            if total_ms >= unit_ms {
                let count = total_ms / unit_ms;
                total_ms %= unit_ms;
                if decomposed {
                    spelling.push(' ');
                }
                // `u64`'s `Display` into a `String` is infallible.
                let _ = write!(spelling, "{count}{}", unit.spelling());
                decomposed = true;
            }
        }

        // The decomposition can't represent every magnitude: a sub-millisecond
        // remainder rounds to zero, and a huge magnitude overflows `u64` in the
        // milliseconds conversion above (read back as 0). Either way nothing
        // was written for a nonzero `total_secs`; fall back to the honest
        // `DurationSeconds` dialect instead of lying with `"0s"`.
        if decomposed {
            spelling.into_boxed_str()
        } else {
            format!("{}s", DurationSeconds(total_secs)).into_boxed_str()
        }
    }

    /// Renders this duration in human-readable compound English (for example
    /// `"3 days, 4 hours"`), decomposing its nominal seconds greedily from
    /// years down to seconds with a rounded millisecond remainder.
    ///
    /// Magnitudes too large for whole-unit counts render in scientific
    /// notation; zero renders as `"0 seconds"`.
    #[must_use]
    pub(crate) fn humanize(&self) -> String {
        let total_secs = self.seconds.as_f64();
        if total_secs == 0.0 {
            return "0 seconds".to_owned();
        }
        let is_negative = total_secs < 0.0;
        let mut rem = total_secs.abs();

        let mut parts = Vec::new();
        for unit in HUMAN_ORDER {
            let unit_secs = unit.nominal_seconds();
            if rem >= unit_secs {
                let count = (rem / unit_secs).floor();
                let Some(count_u64) = count.to_u64() else {
                    return scientific_seconds_text(total_secs, is_negative);
                };
                rem = count.mul_add(-unit_secs, rem);
                let (singular, plural) = unit.human_names();
                if count_u64 == 1 {
                    parts.push(format!("1 {singular}"));
                } else {
                    parts.push(format!("{count_u64} {plural}"));
                }
            }
        }

        if rem > 0.0 {
            let ms_round = (rem * 1000.0).round();
            if let Some(ms) = ms_round.to_u64()
                && ms > 0
            {
                if ms == 1 {
                    parts.push("1 millisecond".to_owned());
                } else {
                    parts.push(format!("{ms} milliseconds"));
                }
            } else if parts.is_empty() {
                parts.push("< 1 millisecond".to_owned());
            } else {
                // Sub-millisecond residual after whole units are already
                // present is omitted.
            }
        }

        if parts.is_empty() {
            return "0 seconds".to_owned();
        }

        let joined = parts.join(", ");
        if is_negative {
            format!("-{joined}")
        } else {
            joined
        }
    }

    /// Returns the parsed duration components, if retained.
    ///
    /// Values parsed from text retain their written components and witness the
    /// regime:
    ///
    /// - [`Some`] containing a calendar unit ([`DurationUnit::Day`],
    ///   [`DurationUnit::Week`], [`DurationUnit::Month`],
    ///   [`DurationUnit::Year`]): calendar application semantics.
    /// - [`Some`] with only sub-day units, or [`None`] (synthesized from
    ///   seconds): fixed-magnitude semantics.
    #[inline]
    #[must_use]
    pub(crate) fn components(&self) -> Option<&[DurationComponent]> {
        self.components.as_deref()
    }

    /// Returns true if this duration contains any calendar-application units
    /// in its parsed components.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn has_calendar_component(&self) -> bool {
        self.components().is_some_and(|components| {
            components.iter().any(|component| component.unit.is_calendar())
        })
    }

    /// Returns the duration spelling.
    #[inline]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.spelling
    }

    /// Returns the total duration as [`DurationSeconds`].
    #[inline]
    #[must_use]
    pub(crate) const fn nominal_seconds(&self) -> DurationSeconds {
        self.seconds
    }

    /// Combines `rhs` with `self`, negating `rhs`'s components (and seconds, in
    /// the fixed regime) when `subtract` is `true`; the single
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

    /// Converts to a [`TimeDelta`] via the duration's total seconds, returning
    /// a `NonFiniteSeconds` error when the seconds value is non-finite or
    /// outside `TimeDelta`'s representable range.
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

/// Compares by parsed seconds, not raw spelling: `"1h 30m"` and `"90m"` are
/// equal.
impl PartialEq for DurationValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.seconds == other.seconds
    }
}

impl Eq for DurationValue {}

/// Orders by parsed seconds, not raw spelling, consistent with [`PartialEq`].
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

/// Hashes the parsed seconds, not raw spelling, so equal values per
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
    pub(crate) magnitude: f64,
    /// Unit associated with this magnitude.
    pub(crate) unit: DurationUnit,
}

/// Retained duration components in written left-to-right order.
type Components = Box<[DurationComponent]>;

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
pub(crate) struct DurationSeconds(pub(crate) f64);

impl DurationSeconds {
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

    /// Returns the raw seconds value as `f64`.
    #[inline]
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0
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

    /// Converts to a [`TimeDelta`], returning a `NonFiniteSeconds` error when
    /// the seconds value is non-finite or outside `TimeDelta`'s representable
    /// range.
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
            nanos.round().to_i64().ok_or(DurationError::NonFiniteSeconds)?;
        if nanos_rounded >= 1_000_000_000 {
            secs += 1.0;
            nanos_rounded = nanos_rounded.saturating_sub(1_000_000_000);
        }
        let secs_i64 = secs.to_i64().ok_or(DurationError::NonFiniteSeconds)?;
        let nanos_u32 = u32::try_from(nanos_rounded)
            .map_err(|_| DurationError::NonFiniteSeconds)?;
        Self::new(secs_i64, nanos_u32).ok_or(DurationError::NonFiniteSeconds)
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
/// [`SCIENTIFIC_ABOVE`] or falls below [`SCIENTIFIC_BELOW`] (exact
/// zero excluded), so an extreme magnitude never dumps a hundreds-of-digits
/// decimal literal.
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

/// Error returned when a duration operation fails.
///
/// Three failure domains:
///
/// - Parsing human-readable duration text;
/// - Validating raw seconds;
/// - Converting a duration or seconds value to a `TimeDelta`.
///
/// # Examples
///
/// ```rust
/// use traces_pkm::{DurationError, DurationValue};
///
/// let err = "".parse::<DurationValue>().expect_err("empty input");
/// assert!(matches!(err, DurationError::Empty));
/// ```
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DurationError {
    /// Input is empty or contains only separators.
    #[error("duration is empty")]
    Empty,

    /// A unit appears without a preceding number (e.g., `"h"` or `"hours"`).
    #[error("no number before unit in `{input}`")]
    MissingNumber {
        /// The raw input that failed to parse.
        input: String,
    },

    /// The number portion has a malformed sign (e.g., `"+h"`), or a part after
    /// the first carries an explicit `-` (e.g., `"1h -30m"`).
    #[error("invalid number in `{input}`")]
    InvalidNumber {
        /// The raw input that failed to parse.
        input: String,
    },

    /// The number portion is not valid float syntax (e.g., a bare `"."`).
    #[error("invalid number syntax in `{input}`: {source}")]
    MalformedNumber {
        /// The raw input that failed to parse.
        input: String,
        /// The underlying float-parse failure.
        #[source]
        source: std::num::ParseFloatError,
    },

    /// A number appears without a trailing unit (e.g., `"1"` or `"42"`).
    #[error("no unit after number in `{input}`")]
    MissingUnit {
        /// The raw input that failed to parse.
        input: String,
    },

    /// The unit string is not recognized (e.g., `"1x"`).
    #[error("unknown unit in `{input}`")]
    UnknownUnit {
        /// The raw input that failed to parse.
        input: String,
    },

    /// A single part's number overflows to infinity (e.g., a 400-digit
    /// literal).
    #[error("number overflows to infinity in `{input}`")]
    NonFiniteNumber {
        /// The raw input that failed to parse.
        input: String,
    },

    /// A raw seconds value is `NaN` or infinite, or does not fit
    /// `TimeDelta`'s representable range.
    #[error("duration seconds must be finite")]
    NonFiniteSeconds,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod golden_serde {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn written_durations_keep_their_json_and_postcard_spellings() {
            // Arrange
            let compound = DurationValue::parse("1h 30m").unwrap();
            let minutes = DurationValue::parse("90m").unwrap();

            // Act
            let compound_bytes = postcard::to_allocvec(&compound).unwrap();
            let minutes_bytes = postcard::to_allocvec(&minutes).unwrap();

            // Assert
            assert_eq!(serde_json::to_string(&compound).unwrap(), "\"1h 30m\"");
            assert_eq!(serde_json::to_string(&minutes).unwrap(), "\"90m\"");
            assert_eq!(compound_bytes, b"\x061h 30m");
            assert_eq!(minutes_bytes, b"\x0390m");
            assert_eq!(
                postcard::from_bytes::<DurationValue>(&compound_bytes).unwrap(),
                compound
            );
            assert_eq!(
                postcard::from_bytes::<DurationValue>(&minutes_bytes).unwrap(),
                minutes
            );
        }
    }

    mod value {
        use super::*;

        mod parse {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

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
            fn parses_single_unit(#[case] input: &str, #[case] expected: f64) {
                assert_eq!(
                    DurationValue::parse(input).unwrap().nominal_seconds(),
                    DurationSeconds::try_from(expected).unwrap()
                );
            }

            #[rstest]
            #[case::with_space("1h 30m")]
            #[case::with_comma("1h, 30m")]
            #[case::without_separator("1h30m")]
            #[case::consecutive_separators("1  h  30  m")]
            fn parses_multi_part(#[case] input: &str) {
                assert_eq!(
                    DurationValue::parse(input).unwrap().nominal_seconds(),
                    DurationSeconds::try_from(5_400.0).unwrap()
                );
            }

            #[test]
            fn applies_the_leading_sign_to_the_whole_duration() {
                assert_eq!(
                    DurationValue::parse("-1h 30m").unwrap().nominal_seconds(),
                    DurationSeconds::try_from(-5_400.0).unwrap()
                );
            }

            #[rstest]
            #[case::unsigned("1.5h", 5_400.0)]
            #[case::without_leading_digit(".5h", 1_800.0)]
            #[case::negative_without_leading_digit("-.5h", -1_800.0)]
            #[case::positive_with_leading_plus("+.5h", 1_800.0)]
            #[case::exponent("1e3s", 1_000.0)]
            #[case::negative_exponent("1e-2h", 36.0)]
            #[case::explicit_exponent_sign("1e+2d", 8_640_000.0)]
            #[case::uppercase_exponent("1E3m", 60_000.0)]
            #[case::exponent_after_decimal(".5e2s", 50.0)]
            fn parses_decimal(#[case] input: &str, #[case] expected: f64) {
                assert_eq!(
                    DurationValue::parse(input).unwrap().nominal_seconds(),
                    DurationSeconds::try_from(expected).unwrap()
                );
            }

            #[test]
            fn trims_leading_trailing_whitespace() {
                let d = DurationValue::parse(" 1h ").unwrap();
                assert_eq!(
                    d.nominal_seconds(),
                    DurationSeconds::try_from(3_600.0).unwrap()
                );
            }

            #[rstest]
            #[case::trailing_space("1h ")]
            #[case::trailing_comma("1h,")]
            fn accepts_trailing_separator(#[case] input: &str) {
                let d = DurationValue::parse(input).unwrap();
                assert_eq!(
                    d.nominal_seconds(),
                    DurationSeconds::try_from(3_600.0).unwrap()
                );
            }

            #[test]
            fn accepts_redundant_plus_on_non_leading_part() {
                let d = DurationValue::parse("1h +30m").unwrap();
                assert_eq!(
                    d.nominal_seconds(),
                    DurationSeconds::try_from(5_400.0).unwrap()
                );
            }

            mod error {
                use pretty_assertions::assert_eq;
                use rstest::rstest;

                use super::*;

                #[rstest]
                #[case::empty("")]
                #[case::whitespace_only("   ")]
                #[case::separator_only(",")]
                fn rejects_empty_input(#[case] input: &str) {
                    assert!(matches!(
                        DurationValue::parse(input),
                        Err(DurationError::Empty)
                    ));
                }

                #[rstest]
                #[case::positive_sign("+h")]
                #[case::negative_sign("-h")]
                fn rejects_sign_without_digit(#[case] input: &str) {
                    let err = DurationValue::parse(input).unwrap_err();
                    assert!(matches!(err, DurationError::InvalidNumber { .. }));
                }

                #[test]
                fn rejects_negative_on_non_leading_part() {
                    let err = DurationValue::parse("1h -30m").unwrap_err();
                    assert!(matches!(err, DurationError::InvalidNumber { .. }));
                }

                #[test]
                fn rejects_malformed_number_syntax_with_a_chained_source() {
                    use std::error::Error as _;
                    let err = DurationValue::parse(".h").unwrap_err();
                    assert!(matches!(
                        err,
                        DurationError::MalformedNumber { .. }
                    ));
                    assert!(err.source().is_some());
                }

                #[test]
                fn treats_a_single_part_overflowing_to_infinity_as_non_finite()
                {
                    let overflowing = format!("1{}h", "0".repeat(400));
                    let err = DurationValue::parse(&overflowing).unwrap_err();
                    assert!(matches!(
                        &err,
                        DurationError::NonFiniteNumber { input }
                        if input == &overflowing
                    ));
                }

                #[test]
                fn rejects_a_multi_part_fold_overflowing_to_infinity() {
                    // Each part is finite (1e308 is below `f64::MAX`), but
                    // their Σ overflows: the parse-level guard is
                    // `NonFiniteSeconds`, not `NonFiniteNumber`, which
                    // targets per-part magnitudes.
                    let huge =
                        format!("1{}s 1{}s", "0".repeat(308), "0".repeat(308));
                    let err = DurationValue::parse(&huge).unwrap_err();
                    assert!(matches!(err, DurationError::NonFiniteSeconds));
                }

                #[rstest]
                #[case::bare_unit("h")]
                #[case::unit_without_number(", h")]
                fn rejects_unit_without_number(#[case] input: &str) {
                    assert!(matches!(
                        DurationValue::parse(input),
                        Err(DurationError::MissingNumber { .. })
                    ));
                }

                #[rstest]
                #[case::double_decimal("1.2.3h")]
                #[case::bare_number("1")]
                fn rejects_number_without_unit(#[case] input: &str) {
                    assert!(matches!(
                        DurationValue::parse(input),
                        Err(DurationError::MissingUnit { .. })
                    ));
                }

                #[rstest]
                #[case::unknown_unit("1x")]
                #[case::longer_than_16_bytes("1aaaaaaaaaaaaaaaaa")]
                // A truncated exponent (`"1e"`, `"1e+"`) is not consumed as
                // a number: the `e` falls to the unit scanner, which
                // rejects it as unknown rather than misreading the value.
                #[case::truncated_exponent("1e")]
                #[case::exponent_sign_without_digits("1e+")]
                fn rejects_unknown_or_long_unit(#[case] input: &str) {
                    assert!(DurationValue::parse(input).is_err());
                }

                #[rstest]
                #[case("1fortnight", "unknown unit in `1fortnight`")]
                #[case("hours", "no number before unit in `hours`")]
                #[case("42", "no unit after number in `42`")]
                fn returns_error_message(
                    #[case] input: &str,
                    #[case] expected: &str,
                ) {
                    let err = DurationValue::parse(input).unwrap_err();
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod prefix {
                use pretty_assertions::assert_eq;
                use rstest::rstest;

                use super::*;

                #[rstest]
                #[case::simple("45m]", 3, "45m", 2_700.0)]
                #[case::decimal(".5h remainder", 3, ".5h", 1_800.0)]
                #[case::multi_part_whitespace(
                    "1h 30m, extra",
                    6,
                    "1h 30m",
                    5_400.0
                )]
                #[case::comma_stops_atom("1h, 30m, extra", 2, "1h", 3_600.0)]
                #[case::negative("-15m extra", 4, "-15m", -900.0)]
                #[case::positive("+15m extra", 4, "+15m", 900.0)]
                #[case::decimal_after_sign("-.5h rest", 4, "-.5h", -1_800.0)]
                #[case::exact_end("1h", 2, "1h", 3_600.0)]
                fn parses_valid_prefix(
                    #[case] input: &str,
                    #[case] expected_consumed: usize,
                    #[case] expected_str: &str,
                    #[case] expected_secs: f64,
                ) {
                    let (dv, consumed) =
                        DurationValue::parse_prefix(input).expect("prefix");
                    assert_eq!(consumed, expected_consumed);
                    assert_eq!(dv.as_str(), expected_str);
                    assert_eq!(
                        dv.nominal_seconds(),
                        DurationSeconds::try_from(expected_secs).unwrap()
                    );
                }

                #[rstest]
                #[case::non_duration("not a duration")]
                #[case::hyphen_bullet("- 15m")]
                #[case::non_leading_negative("1h -30m")]
                #[case::empty("")]
                fn rejects_invalid_prefix(#[case] input: &str) {
                    assert!(DurationValue::parse_prefix(input).is_none());
                }
            }

            mod classify {
                use pretty_assertions::assert_eq;
                use rstest::rstest;

                use super::*;

                #[test]
                fn returns_none_for_non_temporal_text() {
                    assert!(DurationValue::classify("hello").is_none());
                    assert!(DurationValue::classify("").is_none());
                    assert!(DurationValue::classify("+").is_none());
                    assert!(DurationValue::classify(".").is_none());
                }

                #[test]
                fn returns_some_err_for_shape_matching_but_invalid_input() {
                    let res1 = DurationValue::classify("1x")
                        .expect("starts like duration");
                    assert!(matches!(
                        res1,
                        Err(DurationError::UnknownUnit { .. })
                    ));

                    let res2 = DurationValue::classify("1")
                        .expect("starts like duration");
                    assert!(matches!(
                        res2,
                        Err(DurationError::MissingUnit { .. })
                    ));

                    let res3 = DurationValue::classify("1h 1x")
                        .expect("starts like duration");
                    assert!(matches!(
                        res3,
                        Err(DurationError::UnknownUnit { .. })
                    ));
                }

                #[test]
                fn returns_some_ok_for_valid_input() {
                    let res = DurationValue::classify("1h 30m")
                        .expect("starts like duration");
                    assert!(res.is_ok());
                }

                #[test]
                fn pins_one_h_one_x_divergence_between_classify_and_prefix() {
                    let classified = DurationValue::classify("1h 1x");
                    assert!(matches!(
                        classified,
                        Some(Err(DurationError::UnknownUnit { .. }))
                    ));

                    let prefixed = DurationValue::parse_prefix("1h 1x");
                    assert!(prefixed.is_none());
                }

                #[test]
                fn pins_leading_comma_divergence_between_parse_and_prefix() {
                    let parsed = DurationValue::parse(",1h");
                    assert!(parsed.is_ok());
                    assert_eq!(parsed.unwrap().as_str(), ",1h");

                    let prefixed = DurationValue::parse_prefix(",1h");
                    assert!(prefixed.is_none());
                }

                #[test]
                fn pins_trailing_comma_spelling_in_parse_and_prefix() {
                    let parsed = DurationValue::parse("1h,").unwrap();
                    assert_eq!(parsed.as_str(), "1h,");

                    let (prefixed, consumed) =
                        DurationValue::parse_prefix("1h,").unwrap();
                    assert_eq!(prefixed.as_str(), "1h");
                    assert_eq!(consumed, 2);
                }

                #[rstest]
                #[case::digit("1h")]
                #[case::decimal_point(".5h")]
                #[case::plus_sign("+1h")]
                #[case::minus_sign("-30m")]
                #[case::sign_with_decimal("+.5h")]
                fn accepts_valid_start(#[case] input: &str) {
                    assert!(scan::can_start_component(input.as_bytes(), 0));
                }

                #[rstest]
                #[case::non_numeric("hello")]
                #[case::empty("")]
                #[case::lone_sign("+")]
                #[case::lone_decimal_point(".")]
                fn rejects_invalid_start(#[case] input: &str) {
                    assert!(!scan::can_start_component(input.as_bytes(), 0));
                }
            }
        }

        mod constructor {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            use super::*;

            #[rstest]
            #[case::zero(0.0, "0s")]
            #[case::positive(5_400.0, "1h 30m")]
            #[case::negative(-5_400.0, "-1h 30m")]
            fn synthesizes(#[case] raw_secs: f64, #[case] expected_str: &str) {
                let secs = DurationSeconds::try_from(raw_secs).unwrap();
                let dv = DurationValue::from_nominal_seconds(secs);
                assert_eq!(dv.as_str(), expected_str);
                assert_eq!(dv.nominal_seconds(), secs);
            }

            #[rstest]
            #[case::negative_zero(-0.0, "0s")]
            #[case::sub_second(0.5, "500ms")]
            #[case::large_compound(
                604_800.0 + 86_400.0 + 3_600.0 + 60.0 + 1.0 + 0.5,
                "1w 1d 1h 1m 1s 500ms"
            )]
            fn handles(#[case] raw_secs: f64, #[case] expected_str: &str) {
                let secs = DurationSeconds::try_from(raw_secs).unwrap();
                let dv = DurationValue::from_nominal_seconds(secs);
                assert_eq!(dv.as_str(), expected_str);
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
            fn roundtrips_through_parser(#[case] raw_seconds: f64) {
                let secs = DurationSeconds::try_from(raw_seconds).unwrap();
                let synthesized = DurationValue::from_nominal_seconds(secs);
                let reparsed = DurationValue::parse(synthesized.as_str())
                    .expect("synthesized string must parse");
                assert_eq!(reparsed.nominal_seconds(), secs);
            }
        }

        mod equality {
            use pretty_assertions::{assert_eq, assert_ne};

            use super::*;

            #[test]
            fn equates_semantically_equivalent_spellings() {
                let a = DurationValue::parse("1h 30m").unwrap();
                let b = DurationValue::parse("90m").unwrap();
                assert_eq!(a, b);
            }

            #[test]
            fn distinguishes_different_durations() {
                let a = DurationValue::parse("1h").unwrap();
                let b = DurationValue::parse("2h").unwrap();
                assert_ne!(a, b);
            }

            #[test]
            fn treats_negative_zero_as_equal_to_positive_zero() {
                let neg = DurationValue::parse("-0m").unwrap();
                let pos = DurationValue::parse("0m").unwrap();
                assert_eq!(neg, pos);
            }
        }

        mod ordering {
            use super::*;

            #[test]
            fn partial_ord_matches_ord() {
                let a = DurationValue::parse("30m").unwrap();
                let b = DurationValue::parse("1h").unwrap();
                assert!(a < b);
                assert!(b > a);
            }

            #[test]
            fn orders_signed_durations_correctly() {
                let neg = DurationValue::parse("-15m").unwrap();
                let zero = DurationValue::parse("0m").unwrap();
                let pos = DurationValue::parse("+15m").unwrap();
                assert!(neg < zero);
                assert!(zero < pos);
            }

            #[test]
            fn orders_negative_zero_as_greater_than_or_equal_to_positive_zero()
            {
                let neg_zero = DurationValue::parse("-0m").unwrap();
                let zero = DurationValue::parse("0m").unwrap();
                assert!(neg_zero >= zero);
            }
        }

        mod hashing {
            use std::{
                collections::hash_map::DefaultHasher,
                hash::{Hash, Hasher},
            };

            use rstest::rstest;

            use super::*;

            fn hash_val<T: Hash>(val: &T) -> u64 {
                let mut hasher = DefaultHasher::new();
                val.hash(&mut hasher);
                hasher.finish()
            }

            #[rstest]
            #[case::equal("1h 30m", "90m", true)]
            #[case::different("1h", "2h", false)]
            fn duration_value_hash(
                #[case] a: &str,
                #[case] b: &str,
                #[case] should_match: bool,
            ) {
                let a = DurationValue::parse(a).unwrap();
                let b = DurationValue::parse(b).unwrap();
                if should_match {
                    assert_eq!(hash_val(&a), hash_val(&b));
                } else {
                    assert_ne!(hash_val(&a), hash_val(&b));
                }
            }

            #[rstest]
            #[case::equal(100.0, 100.0, true)]
            #[case::different(100.0, 200.0, false)]
            fn seconds_hash(
                #[case] a: f64,
                #[case] b: f64,
                #[case] should_match: bool,
            ) {
                let a = DurationSeconds::try_from(a).unwrap();
                let b = DurationSeconds::try_from(b).unwrap();
                if should_match {
                    assert_eq!(hash_val(&a), hash_val(&b));
                } else {
                    assert_ne!(hash_val(&a), hash_val(&b));
                }
            }
        }

        mod formatting {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            use super::*;

            #[test]
            fn returns_raw_spelling_for_duration_value() {
                let d = DurationValue::parse("4 yrs, 6 wks").unwrap();
                assert_eq!(format!("{d}"), "4 yrs, 6 wks");
                assert_eq!(d.as_str(), "4 yrs, 6 wks");
            }

            #[test]
            fn seconds_displays_as_number() {
                assert_eq!(
                    DurationSeconds::try_from(3600.0).unwrap().to_string(),
                    "3600"
                );
                assert_eq!(
                    DurationSeconds::try_from(0.5).unwrap().to_string(),
                    "0.5"
                );
            }

            #[rstest]
            #[case::huge_positive(1e300, "1e300")]
            #[case::huge_negative(-1e300, "-1e300")]
            #[case::tiny_positive(1e-300, "1e-300")]
            #[case::at_upper_threshold(SCIENTIFIC_ABOVE, "1e15")]
            #[case::just_below_upper_threshold(1e14, "100000000000000")]
            #[case::at_lower_threshold(SCIENTIFIC_BELOW, "0.000001")]
            #[case::just_above_lower_threshold(0.0004, "0.0004")]
            fn seconds_display_switches_dialect_by_magnitude(
                #[case] value: f64,
                #[case] expected: &str,
            ) {
                assert_eq!(
                    DurationSeconds::try_from(value).unwrap().to_string(),
                    expected
                );
            }

            #[rstest]
            #[case::huge_positive(1e300, "1e300s")]
            #[case::huge_negative(-1e300, "-1e300s")]
            #[case::tiny_positive(0.0004, "0.0004s")]
            fn from_nominal_seconds_never_lies_about_a_nonzero_magnitude(
                #[case] value: f64,
                #[case] expected: &str,
            ) {
                let seconds = DurationSeconds::try_from(value).unwrap();
                assert_eq!(
                    DurationValue::from_nominal_seconds(seconds).as_str(),
                    expected
                );
            }
        }

        mod serialization {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn serializes_to_raw_string() {
                let dv = DurationValue::parse("1h 30m").unwrap();
                let json = serde_json::to_string(&dv).unwrap();
                assert_eq!(json, "\"1h 30m\"");
            }
        }

        mod deserialization {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn deserializes_from_raw_string() {
                let json = "\"4 yrs, 6 wks\"";
                let dv: DurationValue = serde_json::from_str(json).unwrap();
                assert_eq!(dv.as_str(), "4 yrs, 6 wks");
                assert_eq!(
                    dv.nominal_seconds(),
                    DurationValue::parse("4 yrs, 6 wks")
                        .unwrap()
                        .nominal_seconds()
                );
            }

            #[test]
            fn rejects_unparseable_duration_string() {
                let json = "\"not a duration\"";
                let err =
                    serde_json::from_str::<DurationValue>(json).unwrap_err();
                assert!(err.to_string().contains("no number before unit"));
            }
        }

        mod conversions {
            use super::*;

            #[test]
            fn roundtrips_valid_input() {
                let d: DurationValue = "1h 30m".parse().unwrap();
                assert_eq!(
                    d.nominal_seconds(),
                    DurationSeconds::try_from(5_400.0).unwrap()
                );
            }

            #[test]
            fn returns_error_for_invalid_input() {
                let err =
                    "not a duration".parse::<DurationValue>().unwrap_err();
                assert!(matches!(err, DurationError::MissingNumber { .. }));
            }
        }
        mod components {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            use super::*;

            #[test]
            fn components_carry_written_magnitude_with_whole_duration_sign_applied()
             {
                // Arrange
                let input = "-1h 30m";

                // Act
                let dv = DurationValue::parse(input).unwrap();
                let components = dv.components().unwrap();

                // Assert
                assert_eq!(components.len(), 2);
                assert_eq!(
                    components.first().copied(),
                    Some(DurationComponent {
                        magnitude: -1.0,
                        unit: DurationUnit::Hour
                    })
                );
                assert_eq!(
                    components.get(1).copied(),
                    Some(DurationComponent {
                        magnitude: -30.0,
                        unit: DurationUnit::Minute
                    })
                );
            }

            #[test]
            fn prefix_retains_identical_components_to_full_parse() {
                // Arrange
                let text = "-1h 30m trailing";

                // Act
                let (prefix_dv, consumed) =
                    DurationValue::parse_prefix(text).unwrap();
                let full_dv = DurationValue::parse("-1h 30m").unwrap();

                // Assert
                assert_eq!(consumed, 7);
                assert_eq!(prefix_dv.components(), full_dv.components());
                assert_eq!(
                    prefix_dv.nominal_seconds(),
                    full_dv.nominal_seconds()
                );
            }

            #[rstest]
            #[case::simple("1h 30m", 5_400.0)]
            #[case::negative("-1h 30m", -5_400.0)]
            #[case::mixed_units("4 yrs, 6 wks", 129_772_800.0)]
            #[case::sub_second("500ms", 0.5)]
            #[case::float_values("0.5h 15m", 2_700.0)]
            #[case::float_non_associative("10000000000000000s 1s 1s", 1.0e16)]
            fn parsed_values_store_the_exact_literal_seconds(
                #[case] input: &str,
                #[case] expected: f64,
            ) {
                // Arrange & Act
                let dv = DurationValue::parse(input).unwrap();

                // Assert
                assert_eq!(
                    dv.nominal_seconds().0.to_bits(),
                    expected.to_bits()
                );
            }
            #[test]
            fn identifies_calendar_units_and_durations() {
                assert!(DurationUnit::Day.is_calendar());
                assert!(DurationUnit::Week.is_calendar());
                assert!(DurationUnit::Month.is_calendar());
                assert!(DurationUnit::Year.is_calendar());
                assert!(!DurationUnit::Hour.is_calendar());
                assert!(!DurationUnit::Minute.is_calendar());
                assert!(!DurationUnit::Second.is_calendar());
                assert!(!DurationUnit::Millisecond.is_calendar());

                let cal =
                    DurationValue::parse("1mo 2d").expect("valid duration");
                assert!(cal.has_calendar_component());

                let fixed =
                    DurationValue::parse("2h 30m").expect("valid duration");
                assert!(!fixed.has_calendar_component());

                let synth = DurationValue::from_nominal_seconds(
                    DurationSeconds::try_from(86_400.0).unwrap(),
                );
                assert!(!synth.has_calendar_component());
            }
        }

        mod arithmetic {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn add_and_sub_combine_components_for_some_operands() {
                // Arrange
                let expected = DurationValue::parse("1h 30m").unwrap();

                // Act
                let sum = DurationValue::parse("1h").unwrap()
                    + DurationValue::parse("30m").unwrap();

                // Assert: `1h` + `30m` equals the `1h 30m` value (equality is
                // by total seconds), and `1h 30m` minus `30m` folds back to
                // `1h`.
                assert_eq!(sum, expected);
                let sub = expected - DurationValue::parse("30m").unwrap();
                assert_eq!(
                    sub.nominal_seconds(),
                    DurationSeconds::try_from(3_600.0).unwrap()
                );

                // `Sub` keeps the negated rhs entry as the regime witness.
                let sub_components =
                    sub.components().expect("Some+Some Sub retains components");
                assert_eq!(sub_components.len(), 3);
                assert_eq!(
                    sub_components.last().copied(),
                    Some(DurationComponent {
                        magnitude: -30.0,
                        unit: DurationUnit::Minute
                    })
                );
            }

            #[test]
            fn arithmetic_results_store_the_exact_literal_seconds() {
                // Arrange
                let a = DurationValue::parse("10000000000000000s").unwrap();
                let b = DurationValue::parse("1s 1s").unwrap();

                // Act
                let sum = a + b;

                // Assert
                assert_eq!(
                    sum.nominal_seconds().0.to_bits(),
                    1.0e16_f64.to_bits()
                );
            }

            #[test]
            fn arithmetic_with_none_operand_clears_components_and_adds_seconds()
            {
                // Arrange
                let none_dv = DurationValue::from_nominal_seconds(
                    DurationSeconds::try_from(3_600.0).unwrap(),
                );
                let some_dv = DurationValue::parse("30m").unwrap();

                // Act
                let sum = none_dv + some_dv;

                // Assert
                assert!(sum.components().is_none());
                assert_eq!(
                    sum.nominal_seconds(),
                    DurationSeconds::try_from(5_400.0).unwrap()
                );
            }

            #[test]
            fn sub_with_none_operand_clears_components_and_subtracts_seconds() {
                // Arrange
                let none_dv = DurationValue::from_nominal_seconds(
                    DurationSeconds::try_from(3_600.0).unwrap(),
                );
                let some_dv = DurationValue::parse("30m").unwrap();

                // Act
                let diff = none_dv - some_dv;

                // Assert: mixed `Some`+`None` ⇒ `None` witness; seconds
                // subtract directly as the fixed-regime operands.
                assert!(diff.components().is_none());
                assert_eq!(
                    diff.nominal_seconds(),
                    DurationSeconds::try_from(1_800.0).unwrap()
                );
            }

            #[test]
            fn mul_stores_the_exact_literal_seconds() {
                // Arrange
                let dv = DurationValue::parse("1h 30m").unwrap();

                // Act
                let scaled = dv * 2.5;
                let components = scaled.components().unwrap();

                // Assert
                assert_eq!(components.len(), 2);
                assert_eq!(
                    components.first().copied(),
                    Some(DurationComponent {
                        magnitude: 2.5,
                        unit: DurationUnit::Hour
                    })
                );
                assert_eq!(
                    components.get(1).copied(),
                    Some(DurationComponent {
                        magnitude: 75.0,
                        unit: DurationUnit::Minute
                    })
                );
                assert_eq!(
                    scaled.nominal_seconds().0.to_bits(),
                    13_500.0_f64.to_bits()
                );
            }

            #[test]
            fn neg_equals_scaling_by_minus_one() {
                // Arrange
                let dv = DurationValue::parse("1h").unwrap();

                // Act
                let negated = -dv.clone();
                let scaled = dv * -1.0;

                // Assert
                assert_eq!(negated, DurationValue::parse("-1h").unwrap());
                assert_eq!(negated.as_str(), scaled.as_str());
            }

            #[test]
            fn mul_non_finite_scalar_produces_non_finite_seconds_and_never_panics()
             {
                // Arrange
                let dv = DurationValue::parse("1h").unwrap();

                // Act
                let inf_dv = dv.clone() * f64::INFINITY;
                let nan_dv = dv * f64::NAN;

                // Assert
                assert!(!inf_dv.nominal_seconds().0.is_finite());
                assert_eq!(inf_dv.as_str(), "infs");
                assert!(matches!(
                    TimeDelta::try_from(inf_dv),
                    Err(DurationError::NonFiniteSeconds)
                ));

                assert!(!nan_dv.nominal_seconds().0.is_finite());
                assert!(matches!(
                    TimeDelta::try_from(nan_dv),
                    Err(DurationError::NonFiniteSeconds)
                ));
            }

            #[test]
            fn mul_overflow_leaves_non_finite_seconds_rejected_by_conversions()
            {
                // Arrange: `1s` has a unit ratio of exactly 1, so scaling by
                // `f64::MAX` stays finite and the next doubling overflows.
                let dv = DurationValue::parse("1s").unwrap();

                // Act
                let huge = dv * f64::MAX;
                assert!(huge.nominal_seconds().0.is_finite());
                let overflowed = huge * 2.0;

                // Assert: the overflow leaves non-finite seconds that every
                // consuming conversion rejects, and the display never lies as
                // `"0s"`.
                assert!(!overflowed.nominal_seconds().0.is_finite());
                assert_eq!(overflowed.as_str(), "infs");
                assert!(matches!(
                    TimeDelta::try_from(overflowed),
                    Err(DurationError::NonFiniteSeconds)
                ));
            }
        }
    }

    mod seconds {
        use super::*;

        mod arithmetic {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn add_combines_seconds() {
                let a = DurationSeconds::try_from(100.0).unwrap();
                let b = DurationSeconds::try_from(200.0).unwrap();
                assert_eq!(a + b, DurationSeconds::try_from(300.0).unwrap());
            }

            #[test]
            fn sub_subtracts_seconds() {
                let a = DurationSeconds::try_from(300.0).unwrap();
                let b = DurationSeconds::try_from(100.0).unwrap();
                assert_eq!(a - b, DurationSeconds::try_from(200.0).unwrap());
            }

            #[test]
            fn mul_scales_seconds() {
                let a = DurationSeconds::try_from(100.0).unwrap();
                assert_eq!(a * 3.0, DurationSeconds::try_from(300.0).unwrap());
                assert_eq!(3.0 * a, DurationSeconds::try_from(300.0).unwrap());
            }

            #[test]
            fn mul_normalizes_a_resulting_signed_zero_from_either_operand_order()
             {
                let zero = DurationSeconds::try_from(0.0).unwrap();
                assert_eq!(zero * -1.0, zero);
                assert_eq!(-1.0 * zero, zero);
                assert!(zero * -1.0 >= zero);
            }
        }

        mod ordering {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn orders_by_total_cmp() {
                let a = DurationSeconds::try_from(100.0).unwrap();
                let b = DurationSeconds::try_from(200.0).unwrap();
                assert!(a < b);
                assert!(b > a);
                assert_eq!(a, DurationSeconds::try_from(100.0).unwrap());
            }

            #[test]
            fn treats_negative_zero_as_equal_to_positive_zero() {
                let neg = DurationSeconds::try_from(-0.0).unwrap();
                let pos = DurationSeconds::try_from(0.0).unwrap();
                assert_eq!(neg, pos);
                assert!(neg >= pos && pos >= neg);
            }
        }

        mod conversions {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn try_from_rejects_nan() {
                assert!(DurationSeconds::try_from(f64::NAN).is_err());
            }

            #[test]
            fn try_from_rejects_infinity() {
                assert!(DurationSeconds::try_from(f64::INFINITY).is_err());
                assert!(DurationSeconds::try_from(f64::NEG_INFINITY).is_err());
            }

            #[test]
            fn extracts_seconds_from_duration_value() {
                let dv = DurationValue::parse("1h").unwrap();
                assert_eq!(
                    dv.nominal_seconds(),
                    DurationSeconds::try_from(3_600.0).unwrap()
                );
            }
        }
    }

    mod time_delta_conversion {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::whole_second("1h", TimeDelta::hours(1))]
        #[case::subsecond("500ms", TimeDelta::milliseconds(500))]
        fn converts_duration_value(
            #[case] input: &str,
            #[case] expected: TimeDelta,
        ) {
            let duration = DurationValue::parse(input).expect("valid duration");
            let converted = TimeDelta::try_from(duration).expect("in range");
            assert_eq!(converted, expected);
        }

        #[rstest]
        #[case::zero(0.0, TimeDelta::zero())]
        #[case::negative_whole(-10.0, TimeDelta::seconds(-10))]
        #[case::negative_fractional(
            -90.5,
            TimeDelta::milliseconds(-90_500)
        )]
        #[case::sub_microsecond(0.000_000_001, TimeDelta::nanoseconds(1))]
        fn converts_duration_seconds(
            #[case] input_secs: f64,
            #[case] expected: TimeDelta,
        ) {
            let seconds =
                DurationSeconds::try_from(input_secs).expect("finite seconds");
            let converted = TimeDelta::try_from(seconds).expect("in range");
            assert_eq!(converted, expected);
        }

        #[rstest]
        #[case::extreme_positive(1e300)]
        #[case::extreme_negative(-1e300)]
        fn rejects_seconds_outside_representable_range(
            #[case] input_secs: f64,
        ) {
            let seconds =
                DurationSeconds::try_from(input_secs).expect("finite seconds");
            let result = TimeDelta::try_from(seconds);
            assert!(matches!(result, Err(DurationError::NonFiniteSeconds)));
        }
    }
}
