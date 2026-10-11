//! Stage-isolated error hierarchy for configuration discovery, reading, trust
//! verification, and semantic domain validation.
//!
//! # Architecture
//!
//! The configuration lifecycle is an orthogonal 4-stage pipeline:
//!
//! 1. [`ConfigDiscoveryError`] - candidate file discovery and ancestor walks.
//! 2. [`ConfigReadError`] - physical file I/O and TOML syntax deserialization.
//! 3. [`ConfigTrustError`] - workspace/config-file trust and tracking-store
//!    I/O.
//! 4. [`ConfigValueError`] - domain semantic validation of parsed values.
//!
//! Composed by [`ConfigError`] into the result [`crate::cli::CliError`]
//! reports.

use std::{io, path::PathBuf};

use thiserror::Error;

use super::trust::ConfigTrustStatus;
use crate::{
    FieldNameError, FilePathTrackerError, TagError, hash::HashError,
    task::TaskError,
};

/// Unified error type for the entire configuration subsystem.
///
/// Composes errors across the four sequential pipeline stages. Matches
/// workspace naming convention (`IndexError`, `QueryError`, `SchemaError`,
/// `CliError`).
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    /// Discovery stage failure: candidate file discovery or ancestor
    /// traversal.
    #[error(transparent)]
    Discovery(#[from] ConfigDiscoveryError),
    /// Read stage failure: physical file reading or TOML syntax
    /// deserialization.
    #[error(transparent)]
    Read(#[from] ConfigReadError),
    /// Trust stage failure: workspace or config-file trust verification, or
    /// trust-store and tracking-store I/O.
    #[error(transparent)]
    Trust(#[from] ConfigTrustError),
    /// Value stage failure: semantic validation of parsed domain values.
    #[error(transparent)]
    Value(#[from] ConfigValueError),
}

/// Convenience alias for discovery-stage operations.
pub(crate) type ConfigDiscoveryResult<T> =
    std::result::Result<T, ConfigDiscoveryError>;

/// Discovery stage: errors raised while discovering candidate config files on
/// disk.
///
/// Discovery checks never emit TOML syntax, trust, or domain model errors.
#[derive(Debug, Error)]
pub(crate) enum ConfigDiscoveryError {
    /// No local `.traces/config.toml` was found in any ancestor directory.
    #[error("no local config found from {cwd}")]
    NotFound {
        /// Working directory from which ancestor discovery originated.
        cwd: PathBuf,
    },
    /// A filesystem path could not be accessed during ancestor traversal.
    #[error("failed to access path {path} during discovery")]
    Inaccessible {
        /// Path that could not be accessed.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The discovered path is not a valid local `.traces/config.toml`
    /// candidate.
    ///
    /// Local config paths must end with `.traces/config.toml` and have a parent
    /// directory named `.traces`.
    #[error("unsupported local config path {path}")]
    UnsupportedLocalCandidate {
        /// Rejected local config path.
        path: PathBuf,
    },
    /// The discovered path is not a valid global `config.toml` candidate.
    ///
    /// Global config paths must end with `config.toml`.
    #[error("unsupported global config path {path}")]
    UnsupportedGlobalCandidate {
        /// Rejected global config path.
        path: PathBuf,
    },
}

/// Read stage: errors raised while reading file contents or deserializing
/// TOML.
///
/// In-memory buffer parsing never emits path discovery or trust errors.
#[derive(Debug, Error)]
pub(crate) enum ConfigReadError {
    /// The physical config file could not be read from disk.
    #[error("failed to read config file {path}")]
    Io {
        /// File that failed to read.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The config file content contains invalid TOML syntax.
    #[error("failed to parse TOML syntax in {path}")]
    Parse {
        /// File containing invalid TOML.
        path: PathBuf,
        /// Underlying TOML deserialization error.
        #[source]
        source: Box<toml::de::Error>,
    },
}

/// Trust stage: errors raised while verifying trust, or reading and writing
/// the trust and tracking stores.
///
/// Governs cryptographic trust verification and the shared
/// [`crate::FilePathTracker`] I/O that backs both the trust store and the
/// tracked-config store.
#[derive(Debug, Error)]
pub enum ConfigTrustError {
    /// Workspace root or config file trust verification halted, requiring user
    /// action.
    #[error(
        "config file {path} (workspace root {root}) is untrusted: {status}"
    )]
    Untrusted {
        /// Untrusted workspace root.
        root: PathBuf,
        /// Untrusted config file path.
        path: PathBuf,
        /// Specific trust condition causing the halt.
        status: ConfigTrustStatus,
    },
    /// The underlying file path tracker operation failed.
    #[error(transparent)]
    Tracker(#[from] FilePathTrackerError),
    /// Hashing a config file failed.
    #[error(transparent)]
    Hash(#[from] HashError),
}

/// Value stage: errors raised while validating parsed domain model values.
///
/// Pure in-memory construction; never emits I/O or trust errors.
#[derive(Debug, Error)]
pub(crate) enum ConfigValueError {
    /// A configured template or schema subdirectory escaped the workspace root
    /// or violated strict path security invariants.
    #[error("invalid config subdirectory path {path}")]
    InvalidSubDir {
        /// Subdirectory path that failed validation.
        path: PathBuf,
        /// Underlying path confinement failure.
        #[source]
        source: crate::path::PathError,
    },
    /// A `[frontmatter]` or `[schemas]` config table named an invalid field
    /// key.
    #[error("invalid `{table}` config")]
    InvalidFieldKey {
        /// The TOML table that failed validation ("frontmatter" or "schemas").
        table: &'static str,
        /// The underlying validation failure.
        #[source]
        source: FieldNameError,
    },
    /// A `[tasks] tag_filters` entry failed [`crate::Tag`] validation.
    #[error("invalid `tasks.tag_filters` entry {entry:?}")]
    InvalidTagFilter {
        /// The offending entry, before `#`-normalization.
        entry: String,
        /// The underlying tag validation failure.
        #[source]
        source: TagError,
    },
    /// A `[tasks]` configuration failed validation.
    #[error(transparent)]
    Task(#[from] TaskError),
}

impl ConfigValueError {
    /// Builds an invalid field-key error for a named config table.
    pub(crate) fn invalid_field_key(
        table: &'static str,
        source: FieldNameError,
    ) -> Self {
        Self::InvalidFieldKey {
            table,
            source,
        }
    }
}
