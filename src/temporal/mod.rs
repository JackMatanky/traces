//! Provides calendar dates, date-times, durations, and the arithmetic that
//! relates them.
//!
//! This module owns recognition, parsing, formatting, and value semantics for
//! date-shaped and duration-shaped input. Callers use validated values; parse
//! failures are public errors, while arithmetic failures stay internal and map
//! at the query and template boundaries.
//!
//! # Clock doctrine
//!
//! Naive Date-Time input resolves in the process-local zone and is stored as
//! UTC; explicit-offset input is normalized to UTC. A Date remains zone-free.
//! Date-Time display uses the local clock. Repeated local times resolve to the
//! earlier occurrence; skipped local times shift forward by the gap length.
//!
//! # Calendar arithmetic
//!
//! A duration is applied in one Jiff calendar operation, independent of the
//! order in which its units were written. Calendar units use the local clock;
//! exact units measure elapsed time. Calendar-day differences count local
//! calendar boundaries, while hour and smaller differences measure elapsed
//! time.
//!
//! # Key types
//!
//! - [`DateValue`]: A zone-free calendar date on the proleptic Gregorian
//!   calendar with whole-day precision.
//! - [`DateTimeValue`]: Stores a UTC timestamp and renders it as local civil
//!   time.
//! - [`DurationValue`]: A validated duration with nominal identity and
//!   calendar-aware application.
mod calendar;
mod date;
mod datetime;
mod duration;
mod error;
mod lexer;
mod point;
mod unit;
mod zone;

pub(crate) use calendar::{
    CalendarPeriod, PERIOD_EDGE_LOOKUP, PeriodEdge, resolve_period_edge,
    weekday_from_number,
};
pub(crate) use date::DEFAULT_DATE_PATTERN;
pub use date::DateValue;
pub(crate) use datetime::DEFAULT_DATETIME_PATTERN;
pub use datetime::DateTimeValue;
pub use duration::DurationValue;
pub(crate) use error::CalendarError;
pub use error::{DateError, DurationError};
pub(crate) use point::{DateDiff, DatePoint};
pub(crate) use unit::{DateUnit, TimeUnit, UNIT_SPELLINGS_HINT};
pub(crate) use zone::local_zone;

#[cfg(any(test, feature = "test-utils"))]
pub(crate) mod test_support {
    use jiff::tz::TimeZone;

    /// Serializes test access to the process-wide local-zone override and
    /// restores the previous override when its owning test thread exits. Reads
    /// are synchronized separately, so they do not wait on the guard lifecycle
    /// lock.
    ///
    /// Repeated calls to [`Self::set`] replace the active override while
    /// retaining the original zone and serialization lock. Use [`Self::with`]
    /// for nested scoped changes; each scope restores its immediate parent even
    /// during unwind.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "test-utils")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use pretty_assertions::assert_eq;
    /// use traces::{DateTimeValue, testing::ZoneGuard};
    ///
    /// ZoneGuard::set("UTC");
    /// let value = DateTimeValue::from_unix_seconds(0)?;
    /// let utc = value.to_string();
    /// ZoneGuard::with("Etc/GMT-2", || {
    ///     assert_eq!(value.to_string(), "1970-01-01T02:00:00");
    /// });
    /// assert_eq!(value.to_string(), utc);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "test-utils"))]
    /// # fn main() {}
    /// ```
    #[derive(Debug)]
    #[must_use]
    pub struct ZoneGuard {
        previous: Option<TimeZone>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ZoneGuard {
        /// Sets the process-wide local zone for the current test thread.
        ///
        /// Readers on other threads observe the override. Repeated calls
        /// replace it; the original value is restored when the owning test
        /// thread exits. Use [`Self::with`] for nested overrides.
        ///
        /// # Panics
        ///
        /// Panics if `zone` is not a recognized time-zone name.
        #[inline]
        #[expect(
            clippy::panic,
            reason = "unknown test zones indicate a fixture error"
        )]
        pub fn set(zone: &str) {
            let zone = TimeZone::get(zone)
                .unwrap_or_else(|_| panic!("unknown time zone: {zone}"));
            Self::set_timezone(zone);
        }

        /// Runs `action` with a process-wide local-zone override.
        ///
        /// Nested calls restore the immediately enclosing zone. The override is
        /// restored during normal return and panic unwinding.
        ///
        /// # Panics
        ///
        /// Panics if `zone` is not a recognized time-zone name.
        #[inline]
        #[expect(
            clippy::panic,
            reason = "unknown test zones indicate a fixture error"
        )]
        pub fn with<R, F: FnOnce() -> R>(zone: &str, action: F) -> R {
            let zone = TimeZone::get(zone)
                .unwrap_or_else(|_| panic!("unknown time zone: {zone}"));
            let has_owner = ZONE_GUARD.with(|slot| slot.borrow().is_some());
            if has_owner {
                let previous = ZONE_OVERRIDE
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .replace(zone);
                let restore = ScopedZoneRestore {
                    previous,
                };
                let result = action();
                drop(restore);
                result
            } else {
                ZONE_GUARD.with(|slot| {
                    *slot.borrow_mut() = Some(Self::install(Some(zone)));
                });
                let _restore = ThreadZoneRestore;
                action()
            }
        }

        /// Holds the zone lock without changing the current zone.
        #[inline]
        pub fn keep() {
            ZONE_GUARD.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Self::install(None));
                }
            });
        }

        /// Sets the process-wide local zone to a previously resolved zone.
        ///
        /// Repeated calls replace the active override; the original value is
        /// restored when the owning test thread exits.
        pub(crate) fn set_timezone(zone: TimeZone) {
            ZONE_GUARD.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.is_some() {
                    *ZONE_OVERRIDE
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        Some(zone);
                } else {
                    *slot = Some(Self::install(Some(zone)));
                }
            });
        }

        /// Installs a new override zone and acquires the serialization lock.
        fn install(zone: Option<TimeZone>) -> Self {
            let lock = ZONE_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut previous = ZONE_OVERRIDE
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let previous = match zone {
                Some(zone) => previous.replace(zone),
                None => previous.clone(),
            };
            Self {
                previous,
                _lock: lock,
            }
        }
    }

    impl Drop for ZoneGuard {
        #[inline]
        fn drop(&mut self) {
            let previous = self.previous.take();
            *ZONE_OVERRIDE
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = previous;
        }
    }

    #[inline]
    pub(super) fn current_override() -> Option<TimeZone> {
        ZONE_OVERRIDE
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Restores the previous zone override when dropped.
    struct ScopedZoneRestore {
        previous: Option<TimeZone>,
    }

    impl Drop for ScopedZoneRestore {
        fn drop(&mut self) {
            *ZONE_OVERRIDE
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                self.previous.take();
        }
    }

    /// Cleans up thread-local zone state on test thread exit.
    struct ThreadZoneRestore;

    impl Drop for ThreadZoneRestore {
        fn drop(&mut self) {
            let guard = ZONE_GUARD.with(|slot| slot.borrow_mut().take());
            drop(guard);
        }
    }

    thread_local! {
        static ZONE_GUARD: std::cell::RefCell<Option<ZoneGuard>> =
            const { std::cell::RefCell::new(None) };
    }

    static ZONE_OVERRIDE: std::sync::RwLock<Option<TimeZone>> =
        std::sync::RwLock::new(None);

    static ZONE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(test)]
    pub(crate) mod fixtures {
        //! Defines shared builders for the temporal test suites.
        //!
        //! Contains only value builders; assertions live in the owning test
        //! modules.

        use jiff::tz::{Offset, TimeZone};

        use super::super::{DateTimeValue, DateValue};

        /// Builds a fixed-offset zone without consulting the host time-zone
        /// database.
        ///
        /// # Panics
        ///
        /// Panics if `seconds` is outside Jiff's supported offset range.
        pub(crate) fn fixed_offset_zone(seconds: i32) -> TimeZone {
            let offset = Offset::from_seconds(seconds)
                .expect("fixture offset is within Jiff's supported range");
            TimeZone::fixed(offset)
        }

        /// Builds a POSIX-rule fixture for modern New York daylight-saving
        /// rules.
        pub(crate) fn new_york_zone() -> TimeZone {
            TimeZone::posix("EST5EDT,M3.2.0,M11.1.0")
                .expect("New York POSIX fixture is valid")
        }

        /// Builds a POSIX-rule fixture for modern London daylight-saving rules.
        pub(crate) fn london_zone() -> TimeZone {
            TimeZone::posix("GMT0BST,M3.5.0/1,M10.5.0/2")
                .expect("London POSIX fixture is valid")
        }

        /// Builds a POSIX-rule fixture for Lord Howe's half-hour
        /// daylight-saving rules.
        pub(crate) fn lord_howe_zone() -> TimeZone {
            TimeZone::posix("LHST-10:30LHDT-11,M10.1.0,M4.1.0")
                .expect("Lord Howe POSIX fixture is valid")
        }

        /// Builds a POSIX-rule fixture whose transition occurs at local
        /// midnight.
        pub(crate) fn midnight_gap_zone() -> TimeZone {
            TimeZone::posix("STD0DST,M3.2.0/0,M11.1.0/0")
                .expect("midnight-gap POSIX fixture is valid")
        }

        /// Builds a [`DateValue`] from calendar fields.
        ///
        /// # Panics
        ///
        /// Panics if `year`, `month`, and `day` do not form a valid calendar
        /// date; fixtures state dates that are valid by construction.
        pub(crate) fn ymd(year: i32, month: u32, day: u32) -> DateValue {
            DateValue::from_ymd(year, month, day)
                .expect("fixture fields form a valid calendar date")
        }

        /// Builds the fixed reference date-time `2026-07-29 14:30:05 UTC`.
        ///
        /// # Panics
        ///
        /// Panics only if the constant fixture ceases to be a valid date-time;
        /// the literal is unambiguous (UTC carries no fold or gap).
        pub(crate) fn fixed_datetime() -> DateTimeValue {
            DateTimeValue::parse_iso("2026-07-29T14:30:05Z")
                .expect("fixture timestamp is valid and in Jiff's range")
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{sync::mpsc, time::Duration};

        use super::*;
        use crate::temporal::local_zone;

        struct ZoneRestoreObserver {
            previous: TimeZone,
            sender: mpsc::Sender<(TimeZone, TimeZone)>,
        }

        impl Drop for ZoneRestoreObserver {
            fn drop(&mut self) {
                let _ = self.sender.send((
                    self.previous.clone(),
                    local_zone().expect("system time zone available"),
                ));
            }
        }

        #[test]
        fn keeps_the_current_zone_unchanged() {
            // Arrange
            let expected = local_zone().expect("system time zone available");

            // Act
            ZoneGuard::keep();
            ZoneGuard::keep();

            // Assert
            assert_eq!(
                local_zone().expect("system time zone available"),
                expected
            );
        }

        #[test]
        fn restores_the_previous_zone_when_dropped() {
            // Arrange
            let previous = local_zone().expect("system time zone available");
            let guard = ZoneGuard::install(Some(
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone"),
            ));

            // Act
            let installed = local_zone().expect("system time zone available");
            drop(guard);

            // Assert
            assert_ne!(installed, previous);
            assert_eq!(
                local_zone().expect("system time zone available"),
                previous
            );
        }

        #[test]
        fn nested_scopes_restore_the_parent_zone_and_outer_zone() {
            let original = local_zone().expect("system time zone available");

            ZoneGuard::with("Etc/GMT-2", || {
                let outer =
                    TimeZone::get("Etc/GMT-2").expect("valid fixture zone");
                assert_eq!(
                    local_zone().expect("outer zone is available"),
                    outer
                );
                ZoneGuard::with("UTC", || {
                    assert_eq!(
                        local_zone().expect("inner zone is available"),
                        TimeZone::UTC
                    );
                });
                assert_eq!(
                    local_zone().expect("outer zone is restored"),
                    outer
                );
            });

            assert_eq!(
                local_zone().expect("original zone is restored"),
                original
            );
        }

        #[test]
        #[expect(
            clippy::disallowed_methods,
            clippy::panic,
            reason = "test deliberately unwinds to prove scoped restoration"
        )]
        fn scoped_zone_is_restored_during_unwind() {
            let original = local_zone().expect("system time zone available");
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    ZoneGuard::with("Etc/GMT-2", || {
                        panic!("exercise unwind restoration")
                    });
                }));

            assert!(result.is_err());
            assert_eq!(
                local_zone().expect("original zone is restored"),
                original
            );
        }

        #[test]
        fn restores_the_previous_zone_during_unwinding() {
            // Arrange
            let previous = local_zone().expect("system time zone available");
            let replacement = if previous == TimeZone::UTC {
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone")
            } else {
                TimeZone::UTC
            };
            let (sender, restored_zone) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let _observer = ZoneRestoreObserver {
                    previous: previous.clone(),
                    sender,
                };
                let _guard = ZoneGuard::install(Some(replacement));

                // The mismatched assertion deliberately unwinds with the
                // guard live.
                assert_eq!(
                    local_zone().expect("system time zone available"),
                    previous,
                    "force guard unwinding"
                );
            });

            // Act
            let result = worker.join();
            let restored = restored_zone.recv();

            // Assert
            assert!(result.is_err(), "the worker must unwind while guarded");
            assert!(
                matches!(&restored, Ok((before, after)) if before == after),
                "the thread-local zone must be restored before the thread \
                 exits: {restored:?}"
            );
        }

        #[test]
        fn repeated_sets_replace_the_active_override() {
            // Arrange: the first set call installs the thread-local guard.

            // Act
            ZoneGuard::set("Etc/GMT-2");
            ZoneGuard::set("UTC");

            // Assert
            assert_eq!(
                local_zone().expect("guarded zone is available"),
                TimeZone::UTC
            );
        }

        #[test]
        fn serializes_concurrent_guards() {
            // Arrange
            let (held_tx, held_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let first = std::thread::spawn(move || {
                ZoneGuard::set("UTC");
                held_tx.send(()).expect("receiver remains open");
                release_rx.recv().expect("release signal arrives");
            });
            held_rx.recv().expect("first guard acquires the lock");

            let (started_tx, started_rx) = mpsc::channel();
            let (acquired_tx, acquired_rx) = mpsc::channel();

            // Act
            let second = std::thread::spawn(move || {
                started_tx.send(()).expect("receiver remains open");
                ZoneGuard::set("Etc/GMT-2");
                acquired_tx.send(()).expect("receiver remains open");
            });
            started_rx.recv().expect("second thread starts");

            assert_eq!(
                local_zone().expect("active override is available"),
                TimeZone::UTC,
                "other threads observe the active override"
            );
            assert!(
                acquired_rx.recv_timeout(Duration::from_millis(20)).is_err()
            );

            release_tx.send(()).expect("first guard can be released");
            acquired_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("second guard acquires the lock after the first drops");
            first.join().expect("first thread exits cleanly");
            second.join().expect("second thread exits cleanly");
        }

        #[test]
        #[should_panic(expected = "unknown time zone: invalid/zone")]
        fn rejects_an_unknown_zone_name() {
            // Arrange: the name is absent from the installed time-zone
            // database.

            // Act
            ZoneGuard::set("invalid/zone");

            // Assert
            // `#[should_panic]` checks the error message.
        }
    }
}
