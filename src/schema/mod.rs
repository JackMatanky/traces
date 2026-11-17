//! Resolves schemas, merges fields, and executes hierarchy queries for
//! frontmatter-driven file classification.
//!
//! Reads `.traces/schemas/*.toml` files and orders the `extends` DAG with a
//! topological sort ([`graph::SchemaGraph`]). Resolves each Schema's effective
//! [`fields::SchemaFieldDef`]s by merging parent fields and applying
//! `excludes`. Builds own fields with `$ref` resolution limited to the Global
//! Schema or transitive `extends` ancestors.
//!
//! [`SchemaService`] is the domain's public entry point. It wraps loading
//! ([`SchemaService::new`]) and hierarchy queries ([`SchemaService::get`],
//! [`SchemaService::children_of`], [`SchemaService::descendants_of`],
//! [`SchemaService::matches`]) behind a single facade. Callers that need
//! recoverable construction diagnostics use [`SchemaService::load_verbose`].
//!
//! Template exposure, `file` field filtering, and class queries consume
//! [`SchemaService`] and [`Schema`] from outside this module.

mod builder;
mod error;
mod fields;
mod graph;
mod model;
mod name;
mod raw;
mod service;

pub(crate) use error::SchemaError;
/// Exposes [`error::SchemaFileError`] for `template::engine::schema` unit
/// tests.
///
/// Production code imports [`error::SchemaFileError`] directly. This
/// test-only export avoids exposing the private `error` module.
#[cfg(test)]
pub(crate) use error::SchemaFileError;
#[cfg(any(test, feature = "test-utils"))]
pub use fields::SchemaFieldDef;
pub(crate) use fields::{SchemaFileFieldRef, SchemaSelectFieldEntry};
pub use model::Schema;
pub(crate) use name::{SchemaName, SchemaNameRef};
pub(crate) use raw::{
    RawSchema, RawSchemaFieldDef, RawSchemaFieldSource, RawSchemaFieldType,
};
pub use service::SchemaService;
#[cfg(any(test, feature = "test-utils"))]
pub use service::SchemaServiceError;
pub(crate) use service::warn_schema_construction_diagnostics;

/// Defines the reserved Global Schema file stem for the flat `$ref` reference
/// pool.
///
/// The Global Schema is never a Note's File Class. It never inherits through
/// `extends`, and it degrades `required = true` fields to `false`.
pub(crate) const GLOBAL_SCHEMA_NAME: &str = "global";
