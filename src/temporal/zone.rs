//! Resolves the local time zone.

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

/// Returns the process-local time zone.
///
/// # Errors
///
/// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
#[inline]
pub(crate) fn local_zone() -> Result<TimeZone, DateError> {
    #[cfg(any(test, feature = "test-utils"))]
    if let Some(zone) = super::test_support::current_override() {
        return Ok(zone);
    }

    resolve(TimeZone::try_system)
}
#[cfg(test)]
mod tests {

    use super::*;
    use crate::temporal::test_support::{ZoneGuard, fixtures};
    #[test]
    fn fixture_zones_are_resolved_without_system_zone_lookup() {
        let zones = [
            fixtures::fixed_offset_zone(19_800),
            fixtures::new_york_zone(),
            fixtures::london_zone(),
            fixtures::lord_howe_zone(),
            fixtures::midnight_gap_zone(),
        ];

        for zone in zones {
            ZoneGuard::set_timezone(zone.clone());
            assert_eq!(local_zone(), Ok(zone));
        }
    }

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
            assert_eq!(
                local_zone().expect("system time zone available"),
                expected
            );
        }

        #[test]
        fn returns_the_system_zone_without_a_guard() {
            // Arrange
            let expected = resolve(TimeZone::try_system)
                .expect("system time zone available");

            // Act
            let actual = local_zone().expect("system time zone available");

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
}
