//! Duration parsing, validation, and arithmetic.
//!
//! Accepts human-readable duration strings like `"1h 30m"` or `"4 yrs, 6 wks"`
//! and converts them to a total seconds value. Parts are `<number><unit>`
//! pairs, optionally separated by whitespace or commas.
//!
//! # Key types
//!
//! - [`DurationValue`] - A parsed duration carrying its total seconds, its
//!   original spelling, and its retained written parts (which witness the
//!   calendar-versus-fixed regime).
//! - [`DurationUnit`] - Unit registry (parsing and seconds conversion). Single
//!   source of truth; callers should not maintain their own registries.
//! - [`DurationSeconds`] - An `f64` newtype that is finite on construction,
//!   with [`Ord`], [`Add`], [`Sub`], and [`Mul`].
//! - [`DurationError`] - Error type for parse and conversion failures.

mod scan;
mod unit;
mod value;

pub(crate) use unit::{DurationUnit, UNIT_HINT};
pub(crate) use value::DurationSeconds;
pub use value::{DurationError, DurationValue};
