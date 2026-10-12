//! Shared builders for the temporal test suites.
//!
//! Contains only value builders; assertions live in the owning test modules.

use chrono::{TimeZone as _, Utc};

use super::DateTimeValue;

pub(super) fn fixed_datetime() -> DateTimeValue {
    DateTimeValue::from(
        Utc.with_ymd_and_hms(2026, 7, 29, 14, 30, 5)
            .single()
            .expect("2026-07-29 14:30:05 UTC is a valid, unambiguous instant"),
    )
}
