//! Separates errors across discovery, read, trust and value stages.
//!
//! [`ConfigError`] composes [`ConfigDiscoveryError`], [`ConfigReadError`],
//! [`ConfigTrustError`] and [`ConfigValueError`] for callers such as
//! [`crate::cli::CliError`]. Each stage reports only its own failures.

use std::{io, path::PathBuf};

use thiserror::Error;

use super::trust::ConfigTrustStatus;
use crate::{
    FieldNameError, FilePathTrackerError, TagError, hash::HashError,
    task::TaskError,
};

/// Combines failures from the discovery, read, trust and value stages.
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    /// Reports a discovery-stage failure before reading a Config File.
    #[error(transparent)]
    Discovery(#[from] ConfigDiscoveryError),
    /// Reports a read-stage failure in file I/O or TOML deserialization.
    #[error(transparent)]
    Read(#[from] ConfigReadError),
    /// Reports a trust-stage failure in Trust Verification or store I/O.
    #[error(transparent)]
    Trust(#[from] ConfigTrustError),
    /// Reports a value-stage failure in parsed configuration validation.
    #[error(transparent)]
    Value(#[from] ConfigValueError),
}

/// Names the result type for discovery-stage operations.
pub(crate) type ConfigDiscoveryResult<T> =
    std::result::Result<T, ConfigDiscoveryError>;

/// Reports failures while discovering candidate Config Files on disk.
///
/// Discovery does not read TOML or perform Trust Verification.
#[derive(Debug, Error)]
pub(crate) enum ConfigDiscoveryError {
    /// Reports the absence of a local Config File in the ancestor walk.
    #[error("no local config found from {cwd}")]
    NotFound {
        /// Holds the directory where ancestor discovery began.
        cwd: PathBuf,
    },
    /// Reports a filesystem inspection failure during discovery.
    #[error("failed to access path {path} during discovery")]
    Inaccessible {
        /// Holds the path that could not be inspected.
        path: PathBuf,
        /// Holds the underlying I/O failure.
        #[source]
        source: io::Error,
    },
}

/// Reports failures while reading or deserializing a Config File.
///
/// Parsing an in-memory buffer does not discover paths or verify trust.
#[derive(Debug, Error)]
pub(crate) enum ConfigReadError {
    /// Reports a failed Config File read from disk.
    #[error("failed to read config file {path}")]
    Io {
        /// Holds the Config File that could not be read.
        path: PathBuf,
        /// Holds the underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// Reports a Config File with invalid TOML syntax.
    #[error("failed to parse TOML syntax in {path}")]
    Parse {
        /// Holds the Config File containing invalid TOML.
        path: PathBuf,
        /// Holds the underlying TOML deserialization error.
        #[source]
        source: Box<toml::de::Error>,
    },
}

/// Reports failures during Trust Verification and store operations.
///
/// Uses [`crate::FilePathTracker`] for the Trust Store and Tracker Store.
#[derive(Debug, Error)]
pub enum ConfigTrustError {
    /// Reports a Trust Status that requires user action before loading.
    #[error(
        "config file {path} (workspace root {root}) is untrusted: {status}"
    )]
    Untrusted {
        /// Holds the workspace root requiring Trust Verification.
        root: PathBuf,
        /// Holds the local Config File that could not be trusted.
        path: PathBuf,
        /// Holds the Trust Status that stopped loading.
        status: ConfigTrustStatus,
    },
    /// Reports a failed Trust Store or Tracker Store operation.
    #[error(transparent)]
    Tracker(#[from] FilePathTrackerError),
    /// Reports a failed Config File read for Trust Verification.
    #[error(transparent)]
    Hash(#[from] HashError),
}

/// Reports failures while validating parsed configuration values.
///
/// In-memory validation does not read files or perform Trust Verification.
#[derive(Debug, Error)]
pub(crate) enum ConfigValueError {
    /// Reports a Template Directory or schema path that fails confinement.
    #[error("invalid config subdirectory path {path}")]
    InvalidSubDir {
        /// Holds the subdirectory that failed validation.
        path: PathBuf,
        /// Holds the underlying path confinement failure.
        #[source]
        source: crate::path::PathError,
    },
    /// Reports an invalid field key in a `frontmatter` or `schemas` table.
    #[error("invalid `{table}` config")]
    InvalidFieldKey {
        /// Names the TOML table that failed validation.
        table: &'static str,
        /// Holds the underlying field-name validation failure.
        #[source]
        source: FieldNameError,
    },
    /// Reports a `tasks` tag-filter entry rejected by [`crate::Tag`].
    #[error("invalid `tasks.tag_filters` entry {entry:?}")]
    InvalidTagFilter {
        /// Holds the entry before `#` normalization.
        entry: String,
        /// Holds the underlying tag validation failure.
        #[source]
        source: TagError,
    },
    /// Reports an invalid Task Configuration.
    #[error(transparent)]
    Task(#[from] TaskError),
}

impl ConfigValueError {
    /// Builds an invalid field-key error for a named configuration table.
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
