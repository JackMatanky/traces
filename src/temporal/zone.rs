//! Resolves the local time zone and owns the scoped test override.

use jiff::tz::TimeZone;

use super::DateError;

/// Resolves a time-zone source into a user-facing lookup error.
fn resolve<E>(
    source: impl FnOnce() -> Result<TimeZone, E>,
) -> Result<TimeZone, DateError>
where
    E: std::fmt::Display,
{
    source().map_err(|error| DateError::LocalZoneLookup {
        input: format!("system zone: {error}").into(),
    })
}

/// Returns the system time zone, falling back to UTC if system discovery fails.
#[inline]
#[must_use]
pub(crate) fn local_zone() -> TimeZone {
    #[cfg(any(test, feature = "test-utils"))]
    if let Some(zone) = ZONE_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return zone;
    }

    resolve(TimeZone::try_system).unwrap_or(TimeZone::UTC)
}

#[cfg(any(test, feature = "test-utils"))]
thread_local! {
    static ZONE_OVERRIDE: std::cell::RefCell<Option<TimeZone>> =
        const { std::cell::RefCell::new(None) };
    static ZONE_GUARD: std::cell::RefCell<Option<ZoneGuard>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "test-utils"))]
static ZONE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serializes one test's local-zone override and restores it when its thread
/// exits.
///
/// Repeated calls to [`Self::set`] replace the current thread's override while
/// keeping the original zone and lock until the test thread exits.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug)]
#[must_use]
pub struct ZoneGuard {
    previous: Option<TimeZone>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(any(test, feature = "test-utils"))]
impl ZoneGuard {
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

    /// Sets the local zone for the current test thread.
    ///
    /// Repeated calls replace the override; the original value is restored when
    /// the test thread exits.
    ///
    /// # Panics
    ///
    /// Panics if `zone` is not a recognized time-zone name.
    #[inline]
    #[allow(
        clippy::panic,
        reason = "unknown test zones indicate a fixture error"
    )]
    pub fn set(zone: &str) {
        let zone = TimeZone::get(zone)
            .unwrap_or_else(|_| panic!("unknown time zone: {zone}"));
        ZONE_GUARD.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_some() {
                ZONE_OVERRIDE
                    .with(|current| *current.borrow_mut() = Some(zone));
            } else {
                *slot = Some(Self::install(Some(zone)));
            }
        });
    }

    fn install(zone: Option<TimeZone>) -> Self {
        let lock =
            ZONE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = ZONE_OVERRIDE.with(|slot| {
            let mut current = slot.borrow_mut();
            match zone {
                Some(zone) => (*current).replace(zone),
                None => current.clone(),
            }
        });
        Self {
            previous,
            _lock: lock,
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Drop for ZoneGuard {
    #[inline]
    fn drop(&mut self) {
        let previous = self.previous.take();
        let _ = ZONE_OVERRIDE.try_with(|slot| *slot.borrow_mut() = previous);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::mpsc,
        time::Duration,
    };

    use super::*;

    mod lookup {
        use super::*;

        #[test]
        fn returns_the_guarded_zone() {
            let expected =
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone");
            ZoneGuard::set("Etc/GMT-2");

            assert_eq!(local_zone(), expected);
        }

        #[test]
        fn returns_the_system_zone_without_a_guard() {
            let expected =
                resolve(TimeZone::try_system).unwrap_or(TimeZone::UTC);

            assert_eq!(local_zone(), expected);
        }

        #[test]
        fn maps_a_failing_resolver_to_the_lookup_error() {
            let result = resolve(|| Err("system zone unavailable"));

            assert_eq!(
                result,
                Err(DateError::LocalZoneLookup {
                    input: "system zone: system zone unavailable".into(),
                })
            );
        }
    }

    mod guard {
        use super::*;

        #[test]
        fn restores_the_previous_zone_when_dropped() {
            let previous = local_zone();
            let guard = ZoneGuard::install(Some(
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone"),
            ));
            assert_ne!(local_zone(), previous);
            drop(guard);

            assert_eq!(local_zone(), previous);
        }

        #[test]
        #[allow(
            clippy::panic,
            clippy::disallowed_methods,
            reason = "exercise guard restoration during unwinding"
        )]
        fn restores_the_previous_zone_during_unwinding() {
            let previous = local_zone();
            let result = catch_unwind(AssertUnwindSafe(|| {
                let _guard = ZoneGuard::install(Some(
                    TimeZone::get("Etc/GMT-2").expect("valid fixture zone"),
                ));
                panic!("test unwind");
            }));

            assert!(result.is_err());
            assert_eq!(local_zone(), previous);
        }

        #[test]
        fn repeated_sets_replace_the_override_and_keep_the_original_guard() {
            ZoneGuard::set("Etc/GMT-2");
            ZoneGuard::set("UTC");

            assert_eq!(local_zone(), TimeZone::UTC);
        }

        #[test]
        fn serializes_concurrent_guards() {
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
            let second = std::thread::spawn(move || {
                started_tx.send(()).expect("receiver remains open");
                ZoneGuard::set("Etc/GMT-2");
                acquired_tx.send(()).expect("receiver remains open");
            });
            started_rx.recv().expect("second thread starts");
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
            ZoneGuard::set("invalid/zone");
        }
    }
}
