//! Defines error and warning types for Schema loading, resolution, and field
//! construction.
//!
//! [`SchemaError`] stops resolution. [`SchemaWarning`] records a recoverable
//! defect that resolution degrades past.

use std::{fmt, path::PathBuf};

use thiserror::Error;

use super::{SchemaName, fields::SchemaFieldBuilderError};
use crate::FieldName;

/// Describes why [`SchemaName`] construction failed.
#[derive(Debug, Eq, PartialEq, Error)]
pub(crate) enum SchemaNameError {
    /// Rejects an empty name.
    #[error("Schema name must not be empty")]
    Empty,
}

/// Defines a result alias carrying [`SchemaError`].
pub(crate) type SchemaResult<T> = std::result::Result<T, SchemaError>;

/// Represents a hard failure that stops Schema loading or resolution.
///
/// Defects that resolution can recover from are emitted as [`SchemaWarning`]
/// instead.
///
/// Deliberately `pub(crate)`, not `pub`. [`TemplateError::SchemaLoad`] wraps
/// this type for its `Display` and `Error` chain with a scoped
/// `#[expect(private_interfaces)]`. Making this `pub` would force `pub` onto
/// every field type below (`SchemaName`, `FieldName`, and
/// `SchemaFieldBuilderError`), which are intentionally crate-internal.
///
/// [`TemplateError::SchemaLoad`]: crate::template::TemplateError::SchemaLoad
#[derive(Debug, Error)]
pub(crate) enum SchemaError {
    /// Indicates the registry directory exists but could not be read.
    #[error("failed to read Schema registry directory {directory}: {source}")]
    ReadDirectory {
        directory: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Indicates a Schema TOML file could not be read or parsed.
    #[error("failed to load Schema file {path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: SchemaFileError,
    },
    /// Indicates the `extends` DAG contains a cycle.
    #[error("cycle detected among Schemas: {}", .schemas.join(", "))]
    Cycle {
        schemas: Vec<SchemaName>,
    },
    /// Indicates two effective fields share a [`FieldKey`](crate::FieldKey)
    /// canonical form.
    #[error(
        "Schema {schema:?} has ambiguous fields {first:?} and {second:?}: \
         both canonicalize to the same metadata key"
    )]
    AmbiguousFieldName {
        schema: SchemaName,
        first: FieldName,
        second: Box<FieldName>,
    },
    /// Indicates a field failed to build due to an unrecognized attribute,
    /// malformed value, or `$ref` resolution failure.
    #[error(transparent)]
    FieldBuilder(Box<SchemaFieldBuilderError>),
}

/// Describes why loading a Schema TOML file failed once its path is known.
///
/// Separate from `select`'s file loader (`SchemaSelectFieldFileError` in
/// `fields::error`, a private submodule). Schema files are always TOML, whereas
/// values files can be TOML or JSON. Both follow the "read, then parse"
/// pattern, but the two types do not share the same shape.
#[derive(Debug, Error)]
pub(crate) enum SchemaFileError {
    /// Indicates an I/O failure.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Indicates a TOML parsing failure.
    #[error("{0}")]
    Parse(#[from] Box<toml::de::Error>),
}

impl From<SchemaFieldBuilderError> for SchemaError {
    fn from(error: SchemaFieldBuilderError) -> Self {
        Self::FieldBuilder(Box::new(error))
    }
}

/// Represents a recoverable Schema resolution defect.
///
/// Resolution skips the offending key or parent and continues. Every warning
/// gives diagnostic context to the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SchemaWarning {
    /// Indicates an `extends` target has no corresponding Schema file.
    ///
    /// Resolution skips the missing parent. The Schema's own fields still
    /// resolve, and other valid parents still contribute their fields.
    MissingExtendsTarget {
        schema: SchemaName,
        target: SchemaName,
    },
    /// Indicates the same `extends` target is declared more than once.
    ///
    /// Resolution ignores duplicates. Only the first occurrence contributes
    /// fields.
    DuplicateExtendsTarget {
        schema: SchemaName,
        target: SchemaName,
    },
    /// Indicates a declared `extends` parent exists but failed to build its own
    /// fields.
    ///
    /// This Schema resolves without that parent's contribution. Other valid
    /// parents still contribute.
    ParentFailedToResolve {
        schema: SchemaName,
        parent: SchemaName,
    },
    /// Indicates `required = true` appeared on the Global Schema.
    ///
    /// Global Schema fields are never required. A referencing Schema can mark
    /// the field required locally instead.
    StrayGlobalRequired {
        field: String,
    },
    /// Indicates a bare `$ref` override declared an invalid attribute,
    /// wrongly-shaped value, or invalid `number`/`select` configuration.
    ///
    /// Resolution drops the key (or whole `values` override) and uses the base
    /// field's value as-is. `message` matches what a hard failure for this
    /// attribute reports (`SchemaFieldParserError`'s `Display`). Because
    /// `message` already carries the address, kind, and key, this variant only
    /// notes that resolution degraded the override.
    DegradedOverride {
        message: String,
    },
}

impl fmt::Display for SchemaWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingExtendsTarget {
                schema,
                target,
            } => write!(
                f,
                "Schema {schema:?} extends unknown Schema {target:?}; its own \
                 fields still resolve"
            ),
            Self::DuplicateExtendsTarget {
                schema,
                target,
            } => write!(
                f,
                "Schema {schema:?} declares extends target {target:?} more \
                 than once; duplicates are ignored"
            ),
            Self::ParentFailedToResolve {
                schema,
                parent,
            } => write!(
                f,
                "Schema {schema:?} extends {parent:?}, which failed to \
                 resolve; its own fields still resolve, without {parent:?}'s \
                 fields"
            ),
            Self::StrayGlobalRequired {
                field,
            } => write!(
                f,
                "the reserved Global Schema declared field {field:?} as \
                 required; ignoring, since Global Schema fields can never be \
                 required"
            ),
            Self::DegradedOverride {
                message,
            } => write!(
                f,
                "$ref override degraded: {message}; using the base value \
                 instead"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    mod schema_error {
        use std::path::PathBuf;

        use pretty_assertions::assert_eq;

        use super::super::*;
        use crate::schema::fields::{
            FieldAddress, SchemaFieldParserError, SchemaFieldTypeTag,
        };

        fn assert_display(error: &SchemaError, expected: &str) {
            assert_eq!(
                error.to_string(),
                expected,
                "unexpected SchemaError display"
            );
        }

        #[test]
        fn read_directory_formats_display_message() {
            let error = SchemaError::ReadDirectory {
                directory: PathBuf::from("/schemas"),
                source: std::io::Error::other("denied"),
            };

            assert_display(
                &error,
                "failed to read Schema registry directory /schemas: denied",
            );
        }

        #[test]
        fn file_formats_display_message_for_an_io_failure() {
            let error = SchemaError::File {
                path: PathBuf::from("/schemas/book.toml"),
                source: SchemaFileError::Io(std::io::Error::other("denied")),
            };

            assert_display(
                &error,
                "failed to load Schema file /schemas/book.toml: denied",
            );
        }

        #[test]
        fn file_formats_display_message_for_a_parse_failure() {
            let source = "not valid toml".parse::<toml::Value>().unwrap_err();
            let error = SchemaError::File {
                path: PathBuf::from("/schemas/book.toml"),
                source: SchemaFileError::Parse(Box::new(source)),
            };

            let message = error.to_string();
            assert!(
                message.starts_with(
                    "failed to load Schema file /schemas/book.toml: "
                ),
                "expected message to open with the file context, got: \
                 {message:?}"
            );
        }

        #[test]
        fn cycle_formats_display_message_joining_every_schema() {
            let error = SchemaError::Cycle {
                schemas: vec![
                    SchemaName::for_test("a"),
                    SchemaName::for_test("b"),
                ],
            };

            assert_display(&error, "cycle detected among Schemas: a, b");
        }

        #[test]
        fn ambiguous_field_name_formats_display_message() {
            let error = SchemaError::AmbiguousFieldName {
                schema: SchemaName::for_test("book"),
                first: FieldName::try_from("status").expect("valid name"),
                second: Box::new(
                    FieldName::try_from("Status").expect("valid name"),
                ),
            };

            assert_display(
                &error,
                "Schema \"book\" has ambiguous fields \"status\" and \
                 \"Status\": both canonicalize to the same metadata key",
            );
        }

        #[test]
        fn field_builder_delegates_display_to_the_wrapped_error() {
            let inner = SchemaFieldBuilderError::Parser(vec![
                SchemaFieldParserError::UnknownKey {
                    address: FieldAddress::try_from("#book/status")
                        .expect("valid ref"),
                    kind: SchemaFieldTypeTag::Date,
                    key: "values".to_owned(),
                },
            ]);

            let error = SchemaError::from(inner);

            let msg = error.to_string();
            assert!(
                msg.contains("1 field option error(s)"),
                "expected wrapped error display, got: {msg}"
            );
        }

        #[test]
        fn stays_small() {
            // Guards `SchemaError` size so `Result<_, SchemaError>` stays cheap
            // to move:
            //
            // - `UnresolvedRef` stores `reference` without duplicating
            //   `ref_schema` and `ref_field` (formerly 5 owned `String`s, 120
            //   bytes).
            // - `RefOutOfBounds` and `RefFieldNotFound` box `own` and
            //   `reference` as `Box<FieldAddress>` because each `$ref` pairs a
            //   `SchemaName` and `FieldName`.
            // - `AmbiguousFieldName` boxes `second` for the same reason.
            // - `FieldBuilder` boxes `SchemaFieldBuilderError` to isolate
            //   `SchemaError` size from future enum growth.
            assert!(
                std::mem::size_of::<SchemaError>() <= 64,
                "SchemaError grew to {} bytes; box or trim the offending \
                 variant",
                std::mem::size_of::<SchemaError>()
            );
        }
    }

    mod schema_field_builder_error {
        use pretty_assertions::assert_eq;

        use super::super::*;
        use crate::schema::fields::{
            FieldAddress, SchemaFieldParserError, SchemaFieldTypeTag,
        };

        #[test]
        fn parser_wraps_multiple_errors() {
            let errors = vec![
                SchemaFieldParserError::UnknownKey {
                    address: FieldAddress::try_from("#book/status")
                        .expect("valid ref"),
                    kind: SchemaFieldTypeTag::Date,
                    key: "values".to_owned(),
                },
                SchemaFieldParserError::TypeMismatch {
                    address: FieldAddress::try_from("#book/rating")
                        .expect("valid ref"),
                    kind: SchemaFieldTypeTag::Number,
                    key: "min".to_owned(),
                    value: "\"abc\"".to_owned(),
                    expected: "a number",
                },
            ];
            let error = SchemaFieldBuilderError::Parser(errors);

            let msg = error.to_string();
            assert!(
                msg.contains("2 field option error(s)"),
                "expected count in display, got: {msg}"
            );
        }

        #[test]
        fn ref_out_of_bounds_formats_display_message() {
            let error = SchemaFieldBuilderError::RefOutOfBounds {
                own: Box::new(
                    FieldAddress::try_from("#movie/status").expect("valid ref"),
                ),
                reference: Box::new(
                    FieldAddress::try_from("#book/status").expect("valid ref"),
                ),
            };

            assert_eq!(
                error.to_string(),
                "$ref #book/status in field #movie/status is out of bounds: \
                 not the Global Schema or a transitive `extends` ancestor"
            );
        }

        #[test]
        fn ref_field_not_found_formats_display_message() {
            let error = SchemaFieldBuilderError::RefFieldNotFound {
                own: Box::new(
                    FieldAddress::try_from("#book/status").expect("valid ref"),
                ),
                reference: Box::new(
                    FieldAddress::try_from("#book/status").expect("valid ref"),
                ),
            };

            assert_eq!(
                error.to_string(),
                "$ref #book/status in field #book/status does not resolve"
            );
        }
    }

    mod schema_warning {
        use pretty_assertions::assert_eq;

        use super::super::*;

        #[test]
        fn missing_extends_target_message_names_schema_and_target() {
            let warning = SchemaWarning::MissingExtendsTarget {
                schema: SchemaName::for_test("sci_fi"),
                target: SchemaName::for_test("ghost"),
            };

            assert_eq!(
                warning.to_string(),
                "Schema \"sci_fi\" extends unknown Schema \"ghost\"; its own \
                 fields still resolve"
            );
        }

        #[test]
        fn duplicate_extends_target_message_names_schema_and_target() {
            let warning = SchemaWarning::DuplicateExtendsTarget {
                schema: SchemaName::for_test("sci_fi"),
                target: SchemaName::for_test("book"),
            };

            assert_eq!(
                warning.to_string(),
                "Schema \"sci_fi\" declares extends target \"book\" more than \
                 once; duplicates are ignored"
            );
        }

        #[test]
        fn parent_failed_to_resolve_message_names_schema_and_parent() {
            let warning = SchemaWarning::ParentFailedToResolve {
                schema: SchemaName::for_test("sci_fi"),
                parent: SchemaName::for_test("book"),
            };

            assert_eq!(
                warning.to_string(),
                "Schema \"sci_fi\" extends \"book\", which failed to resolve; \
                 its own fields still resolve, without \"book\"'s fields"
            );
        }

        #[test]
        fn stray_global_required_message_names_the_field() {
            let warning = SchemaWarning::StrayGlobalRequired {
                field: "priority".to_owned(),
            };

            assert_eq!(
                warning.to_string(),
                "the reserved Global Schema declared field \"priority\" as \
                 required; ignoring, since Global Schema fields can never be \
                 required"
            );
        }

        #[test]
        fn degraded_override_message_wraps_the_hard_failure_text() {
            let warning = SchemaWarning::DegradedOverride {
                message: "field #sci_fi/cover has no attribute \"folders\""
                    .to_owned(),
            };

            assert_eq!(
                warning.to_string(),
                "$ref override degraded: field #sci_fi/cover has no attribute \
                 \"folders\"; using the base value instead"
            );
        }
    }
}
