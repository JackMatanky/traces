//! Defines on-disk Schema TOML shapes and serde deserialization.
//!
//! These types match `.traces/schemas/<name>.toml` and deny unknown fields.
//! During deserialization, serde parses `$ref` strings and `type`/`$ref`
//! sources into validated [`FieldAddress`] and [`RawSchemaFieldSource`] values.
//!
//! Type-specific keys land in [`RawSchemaFieldDef::options`] as generic
//! [`FieldValue`] entries, which [`SchemaFieldBuilder::build`] validates later.
//!
//! [`SchemaFieldBuilder::build`]: super::fields::SchemaFieldBuilder::build

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

use super::{SchemaName, fields::FieldAddress};
use crate::{FieldName, FieldValue};

const ALLOWED_OPTION_KEYS: &[&str] = &[
    "type", "$ref", "required", "multi", "values", "folders", "ext", "class",
    "min", "max", "step", "format",
];

/// Represents one `.traces/schemas/<name>.toml` file, parsed but not yet
/// resolved.
///
/// The filename stem (not any field on this type) is the Schema name. Unknown
/// keys at any level are rejected by `#[serde(deny_unknown_fields)]`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSchema {
    /// Lists parent Schema names. The first-listed parent wins on shared
    /// fields.
    #[serde(default)]
    pub(crate) extends: Vec<SchemaName>,
    /// Lists field names to drop from inherited definitions.
    #[serde(default)]
    pub(crate) excludes: Vec<FieldName>,
    /// Maps field names to their raw definitions.
    #[serde(default)]
    pub(crate) fields: IndexMap<FieldName, RawSchemaFieldDef>,
}

/// Represents a field definition parsed from TOML, before `$ref` resolution.
///
/// Deserialization sets [`source`](Self::source) to either
/// [`RawSchemaFieldSource::Direct`] or [`RawSchemaFieldSource::Ref`] based on
/// `type` and `$ref` in the TOML table.
///
/// Type-specific keys land in [`options`](Self::options) as generic
/// [`FieldValue`] entries, which
/// [`SchemaFieldBuilder::build`](super::fields::SchemaFieldBuilder::build)
/// validates later.
#[derive(Clone, Debug)]
pub(crate) struct RawSchemaFieldDef {
    /// Specifies the field's source: a direct `type` or a `$ref` with an
    /// optional override.
    pub(crate) source: RawSchemaFieldSource,
    /// Indicates whether the field must be set. The Global Schema ignores this
    /// flag with a warning.
    pub(crate) required: Option<bool>,
    /// Indicates whether the field accepts multiple values.
    pub(crate) multi: Option<bool>,
    /// Stores type-specific options as raw [`FieldValue`]s keyed by TOML name.
    pub(crate) options: IndexMap<String, FieldValue>,
}

impl RawSchemaFieldDef {
    /// Builds a `Direct` field definition with no options.
    ///
    /// Test-only: production code deserializes from TOML.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn direct(kind: RawSchemaFieldType) -> Self {
        Self {
            source: RawSchemaFieldSource::Direct(kind),
            required: None,
            multi: None,
            options: IndexMap::new(),
        }
    }

    /// Builds a `$ref`-only field definition with no options.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn reference(address: FieldAddress) -> Self {
        Self {
            source: RawSchemaFieldSource::Ref {
                address,
                override_type: None,
            },
            required: None,
            multi: None,
            options: IndexMap::new(),
        }
    }
}

impl<'de> Deserialize<'de> for RawSchemaFieldDef {
    /// Deserializes the `[fields.<name>]` TOML table, converting its
    /// `type`/`$ref` keys into a validated [`RawSchemaFieldSource`] and every
    /// other present key into [`RawSchemaFieldDef::options`].
    ///
    /// # Errors
    ///
    /// - [`RawFieldDefError::MissingSource`] if neither `type` nor `$ref` is
    ///   present.
    /// - A serde error if `$ref` does not match `#<schema>/<field>` or if an
    ///   unknown key is present.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RawSchemaFieldDefVisitor)
    }
}

struct RawSchemaFieldDefVisitor;

impl<'de> serde::de::Visitor<'de> for RawSchemaFieldDefVisitor {
    type Value = RawSchemaFieldDef;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter.write_str("a field definition map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut kind = None;
        let mut reference = None;
        let mut required = None;
        let mut multi = None;
        let mut options = IndexMap::new();

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => kind = Some(map.next_value()?),
                "$ref" => reference = Some(map.next_value()?),
                "required" => required = Some(map.next_value()?),
                "multi" => multi = Some(map.next_value()?),
                "values" | "folders" | "ext" | "class" | "min" | "max"
                | "step" | "format" => {
                    let val: FieldValue = map.next_value()?;
                    options.insert(key, val);
                }
                other => {
                    return Err(serde::de::Error::unknown_field(
                        other,
                        ALLOWED_OPTION_KEYS,
                    ));
                }
            }
        }

        let source = match (kind, reference) {
            (Some(k), None) => RawSchemaFieldSource::Direct(k),
            (Some(k), Some(r)) => RawSchemaFieldSource::Ref {
                address: r,
                override_type: Some(k),
            },
            (None, Some(r)) => RawSchemaFieldSource::Ref {
                address: r,
                override_type: None,
            },
            (None, None) => {
                return Err(serde::de::Error::custom(
                    RawFieldDefError::MissingSource,
                ));
            }
        };

        Ok(RawSchemaFieldDef {
            source,
            required,
            multi,
            options,
        })
    }
}

/// Specifies how a raw field was declared: `type` alone, or `$ref` with
/// optional `type` override.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RawSchemaFieldSource {
    /// Declares a field using a `type` key with no `$ref`.
    Direct(RawSchemaFieldType),
    /// Declares a field using a `$ref` address to a base definition, with an
    /// optional local `type` override.
    Ref {
        address: FieldAddress,
        override_type: Option<RawSchemaFieldType>,
    },
}

/// Represents the `type` key of a raw field definition.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawSchemaFieldType {
    /// Represents free-form text input.
    Input,
    /// Represents configured selectable values.
    Select,
    /// Represents a boolean value.
    Boolean,
    /// Represents a numeric value.
    Number,
    /// Represents a date value.
    Date,
    /// Represents a file link with optional filters.
    File,
}

impl std::fmt::Display for RawSchemaFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Input => "input",
            Self::Select => "select",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Date => "date",
            Self::File => "file",
        })
    }
}

/// Represents the deserialized root shape of a `select` values file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSchemaSelectFieldValues {
    /// Lists raw select entries when present in the values file.
    pub(crate) entries: Option<Vec<RawSchemaSelectFieldEntry>>,
}

/// Represents a raw entry in a `select` values file, constrained to bare
/// strings or structured objects.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawSchemaSelectFieldEntry {
    /// Represents a bare string option entry.
    Bare(String),
    /// Represents a structured key-value option entry.
    Structured(IndexMap<String, FieldValue>),
}

impl From<RawSchemaSelectFieldEntry> for FieldValue {
    fn from(entry: RawSchemaSelectFieldEntry) -> Self {
        match entry {
            RawSchemaSelectFieldEntry::Bare(s) => Self::String(s),
            RawSchemaSelectFieldEntry::Structured(map) => Self::Object(map),
        }
    }
}

/// Describes why a [`RawSchemaFieldDef`] failed to deserialize.
#[derive(Debug, Error)]
pub(crate) enum RawFieldDefError {
    /// Indicates neither `type` nor `$ref` was present.
    #[error("field definition has neither `type` nor `$ref`")]
    MissingSource,
}

#[cfg(test)]
mod tests {
    mod raw_schema_field_def {
        use pretty_assertions::assert_eq;

        use super::super::*;

        #[test]
        fn deserializes_a_direct_type() {
            let raw: RawSchemaFieldDef =
                toml::from_str(r#"type = "input""#).expect("valid toml");

            assert_eq!(
                raw.source,
                RawSchemaFieldSource::Direct(RawSchemaFieldType::Input)
            );
        }

        #[test]
        fn deserializes_a_ref_only_definition() {
            let raw: RawSchemaFieldDef =
                toml::from_str(r##""$ref" = "#global/status""##)
                    .expect("valid toml");

            assert_eq!(raw.source, RawSchemaFieldSource::Ref {
                address: FieldAddress::try_from("#global/status")
                    .expect("valid ref"),
                override_type: None,
            });
        }

        #[test]
        fn deserializes_a_ref_with_a_local_type_override() {
            let raw: RawSchemaFieldDef = toml::from_str(
                r##"
                type = "file"
                "$ref" = "#global/cover"
                "##,
            )
            .expect("valid toml");

            assert_eq!(raw.source, RawSchemaFieldSource::Ref {
                address: FieldAddress::try_from("#global/cover")
                    .expect("valid ref"),
                override_type: Some(RawSchemaFieldType::File),
            });
        }

        #[test]
        fn rejects_a_definition_with_neither_type_nor_ref() {
            let err = toml::from_str::<RawSchemaFieldDef>("required = true")
                .expect_err("missing source rejected");

            assert!(
                err.to_string()
                    .contains("field definition has neither `type` nor `$ref`"),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn rejects_an_unknown_key() {
            let err = toml::from_str::<RawSchemaFieldDef>(
                "type = \"input\"\ntypo_key = true",
            )
            .expect_err("unknown key rejected");

            assert!(
                err.to_string().contains("typo_key"),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn collects_declared_type_specific_keys_into_options() {
            let raw: RawSchemaFieldDef = toml::from_str(
                "type = \"select\"\nvalues = [\"draft\", \"done\"]",
            )
            .expect("valid toml");

            assert_eq!(
                raw.options.get("values"),
                Some(&FieldValue::List(vec![
                    FieldValue::String("draft".to_owned()),
                    FieldValue::String("done".to_owned()),
                ]))
            );
        }

        #[test]
        fn omits_absent_type_specific_keys_from_options() {
            let raw: RawSchemaFieldDef =
                toml::from_str("type = \"input\"").expect("valid toml");

            assert!(raw.options.is_empty());
        }

        #[test]
        fn accepts_a_wrongly_shaped_value_at_parse_leaving_validation_to_the_builder()
         {
            // `min` is a `number` key, but this layer does not know the field's
            // type. `SchemaFieldBuilder` validates both the key and its value
            // shape later (see `SchemaFieldParserError::TypeMismatch`).
            let raw: RawSchemaFieldDef =
                toml::from_str("type = \"number\"\nmin = \"abc\"")
                    .expect("valid toml: value shape isn't validated here");

            assert_eq!(
                raw.options.get("min"),
                Some(&FieldValue::String("abc".to_owned()))
            );
        }

        #[test]
        fn deserializes_a_number_with_a_step() {
            let raw: RawSchemaFieldDef =
                toml::from_str("type = \"number\"\nstep = 0.5")
                    .expect("valid toml");

            assert_eq!(
                raw.source,
                RawSchemaFieldSource::Direct(RawSchemaFieldType::Number)
            );
            assert_eq!(raw.options.get("step"), Some(&FieldValue::Float(0.5)));
        }
    }

    mod raw_field_type {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::super::*;

        #[rstest]
        #[case::input(RawSchemaFieldType::Input, "input")]
        #[case::select(RawSchemaFieldType::Select, "select")]
        #[case::boolean(RawSchemaFieldType::Boolean, "boolean")]
        #[case::number(RawSchemaFieldType::Number, "number")]
        #[case::date(RawSchemaFieldType::Date, "date")]
        #[case::file(RawSchemaFieldType::File, "file")]
        fn display_matches_the_toml_type_key_value(
            #[case] kind: RawSchemaFieldType,
            #[case] expected: &str,
        ) {
            assert_eq!(kind.to_string(), expected);
        }
    }
}
