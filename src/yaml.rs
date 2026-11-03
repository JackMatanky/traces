//! Provides shared YAML parsing for frontmatter and template filters.

use std::sync::LazyLock;

use noyalib::ParserConfig;

/// Configures YAML 1.2 parsing using noyalib's `serde_yaml_compat` preset.
///
/// Leading-zero integers (`01234`) resolve as strings, YAML 1.1 binary literals
/// resolve as integers, an overflowing float literal stays a string, an integer
/// past `u64::MAX` errors instead of degrading to `f64`, a non-scalar mapping
/// key errors, and `<<` merge keys stay ordinary entries. Built once and shared
/// by every call to [`parse`] so [`RawFrontmatter`]'s YAML parsing and the
/// template engine's `to_yaml`/`from_yaml` filters parse and render YAML
/// identically.
///
/// [`RawFrontmatter`]: crate::note::RawFrontmatter
static YAML_CONFIG: LazyLock<ParserConfig> =
    LazyLock::new(ParserConfig::serde_yaml_compat);

/// Parses `text` as a YAML document under the shared [`YAML_CONFIG`].
///
/// # Errors
///
/// Returns [`noyalib::Error`] if `text` is not valid YAML 1.2 under the shared
/// `serde_yaml_compat` parity config.
#[inline]
pub(crate) fn parse(text: &str) -> Result<noyalib::Value, noyalib::Error> {
    noyalib::from_str_with_config(text, &YAML_CONFIG)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn parses_binary_literals_as_integers() {
            let parsed = parse("val: 0b1010").expect("valid yaml");
            let mut expected_map = noyalib::Mapping::new();
            expected_map.insert(
                "val",
                noyalib::Value::Number(noyalib::Number::Integer(10)),
            );

            assert_eq!(parsed, noyalib::Value::Mapping(expected_map));
        }

        #[test]
        fn preserves_overflowing_float_literal_as_string() {
            let parsed = parse("val: 1e9999").expect("valid yaml");
            let mut expected_map = noyalib::Mapping::new();
            expected_map
                .insert("val", noyalib::Value::String("1e9999".to_owned()));

            assert_eq!(parsed, noyalib::Value::Mapping(expected_map));
        }

        #[test]
        fn parses_u64_max_as_float() {
            let parsed =
                parse("val: 18446744073709551615").expect("valid yaml");
            let mut expected_map = noyalib::Mapping::new();
            expected_map.insert(
                "val",
                noyalib::Value::Number(noyalib::Number::Float(
                    18_446_744_073_709_551_615.0,
                )),
            );

            assert_eq!(parsed, noyalib::Value::Mapping(expected_map));
        }

        #[test]
        fn rejects_integer_past_u64_max_with_integer_overflow_error() {
            let err = parse("val: 18446744073709551616")
                .expect_err("integer past u64::MAX must error");

            assert!(
                matches!(err, noyalib::Error::IntegerOverflow { .. }),
                "expected IntegerOverflow error, got: {err:?}"
            );
        }

        #[rstest]
        #[case::sequence("[1, 2]: val")]
        #[case::mapping("{a: 1}: val")]
        fn rejects_non_scalar_mapping_key_with_error(#[case] yaml: &str) {
            let err = parse(yaml).expect_err("non-scalar key must error");

            assert!(
                matches!(err, noyalib::Error::NonScalarKey { .. }),
                "expected NonScalarKey error, got: {err:?}"
            );
        }
    }
}
