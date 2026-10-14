//! Resolves the local time zone and owns the serialized process-wide test
//! override.

use jiff::tz::TimeZone;

use super::DateError;

/// Maps a failed local-zone lookup to [`DateError::LocalZoneLookup`].
///
/// # Errors
///
/// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
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

/// Returns the process-local time zone, falling back to UTC if system discovery
/// fails.
#[inline]
#[must_use]
pub(crate) fn local_zone() -> TimeZone {
    #[cfg(any(test, feature = "test-utils"))]
    let override_zone = ZONE_OVERRIDE
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();

    #[cfg(any(test, feature = "test-utils"))]
    if let Some(zone) = override_zone {
        return zone;
    }

    resolve(TimeZone::try_system).unwrap_or(TimeZone::UTC)
}

#[cfg(any(test, feature = "test-utils"))]
thread_local! {
    static ZONE_GUARD: std::cell::RefCell<Option<ZoneGuard>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "test-utils"))]
static ZONE_OVERRIDE: std::sync::RwLock<Option<TimeZone>> =
    std::sync::RwLock::new(None);

#[cfg(any(test, feature = "test-utils"))]
static ZONE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serializes test access to the process-wide local-zone override and restores
/// the previous override when its owning test thread exits. Reads are
/// synchronized separately, so they do not wait on the guard lifecycle lock.
///
/// Repeated calls to [`Self::set`] replace the active override while retaining
/// the original zone and serialization lock.
///
/// # Examples
///
/// ```
/// use traces_pkm::{DateTimeValue, testing::ZoneGuard};
///
/// let value = DateTimeValue::from_unix_seconds(0)?;
/// let utc = value.to_string();
/// let worker = std::thread::spawn(move || {
///     ZoneGuard::set("Etc/GMT-2");
///     assert_eq!(value.to_string(), "1970-01-01T02:00:00");
/// });
/// worker.join().expect("zone test thread exits");
/// assert_eq!(value.to_string(), utc);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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

    /// Sets the process-wide local zone for the current test thread.
    ///
    /// Readers on other threads observe the override. Repeated calls replace
    /// it; the original value is restored when the owning test thread
    /// exits.
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

    fn install(zone: Option<TimeZone>) -> Self {
        let lock =
            ZONE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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

#[cfg(any(test, feature = "test-utils"))]
impl Drop for ZoneGuard {
    #[inline]
    fn drop(&mut self) {
        let previous = self.previous.take();
        *ZONE_OVERRIDE
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = previous;
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    mod lookup {
        use super::*;

        #[test]
        fn returns_the_guarded_zone() {
            // Arrange
            let expected =
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone");

            // Act
            ZoneGuard::set("Etc/GMT-2");

            // Assert
            assert_eq!(local_zone(), expected);
        }

        #[test]
        fn returns_the_system_zone_without_a_guard() {
            // Arrange
            let expected =
                resolve(TimeZone::try_system).unwrap_or(TimeZone::UTC);

            // Act
            let actual = local_zone();

            // Assert
            assert_eq!(actual, expected);
        }

        #[test]
        fn maps_a_failing_resolver_to_the_lookup_error() {
            // Arrange
            let unavailable = || Err("system zone unavailable");

            // Act
            let result = resolve(unavailable);

            // Assert
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
        fn keeps_the_current_zone_unchanged() {
            // Arrange
            let expected = local_zone();

            // Act
            ZoneGuard::keep();
            ZoneGuard::keep();

            // Assert
            assert_eq!(local_zone(), expected);
        }

        #[test]
        fn restores_the_previous_zone_when_dropped() {
            // Arrange
            let previous = local_zone();
            let guard = ZoneGuard::install(Some(
                TimeZone::get("Etc/GMT-2").expect("valid fixture zone"),
            ));

            // Act
            let installed = local_zone();
            drop(guard);

            // Assert
            assert_ne!(installed, previous);
            assert_eq!(local_zone(), previous);
        }

        struct ZoneRestoreObserver {
            previous: TimeZone,
            sender: mpsc::Sender<(TimeZone, TimeZone)>,
        }

        impl Drop for ZoneRestoreObserver {
            fn drop(&mut self) {
                let _ = self.sender.send((self.previous.clone(), local_zone()));
            }
        }

        #[test]
        fn restores_the_previous_zone_during_unwinding() {
            // Arrange
            let previous = local_zone();
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

                // The mismatched assertion deliberately unwinds with the guard
                // live.
                assert_eq!(local_zone(), previous, "force guard unwinding");
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
            assert_eq!(local_zone(), TimeZone::UTC);
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
                local_zone(),
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
