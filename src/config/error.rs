//! Separates errors from discovery, reads, trust, and value checks.
//!
//! [`ConfigError`] combines [`ConfigDiscoveryError`], [`ConfigReadError`],
//! [`ConfigTrustError`], and [`ConfigValueError`] for callers such as
//! [`crate::cli::CliError`]. Each stage reports its own failures.

use std::{io, path::PathBuf};

use thiserror::Error;

use super::trust::ConfigTrustStatus;
use crate::{
    FieldNameError, FilePathTrackerError, TagError, hash::HashError,
    task::TaskError,
};

/// Combines failures from the discovery, read, trust, and value stages.
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    /// Reports a failure while finding a Config File.
    #[error(transparent)]
    Discovery(#[from] ConfigDiscoveryError),
    /// Reports a file read or TOML deserialization failure.
    #[error(transparent)]
    Read(#[from] ConfigReadError),
    /// Reports a Trust Verification or store I/O failure.
    #[error(transparent)]
    Trust(#[from] ConfigTrustError),
    /// Reports a failure while checking parsed configuration values.
    #[error(transparent)]
    Value(#[from] ConfigValueError),
}

/// Names the result type for Config File discovery operations.
pub(crate) type ConfigDiscoveryResult<T> =
    std::result::Result<T, ConfigDiscoveryError>;

/// Reports failures while finding candidate Config Files on disk.
///
/// Discovery does not read TOML or check Trust Status.
#[derive(Debug, Error)]
pub(crate) enum ConfigDiscoveryError {
    /// Reports that the ancestor walk found no local Config File.
    #[error("no local config found from {cwd}")]
    NotFound {
        /// Holds the directory where the ancestor walk began.
        cwd: PathBuf,
    },
    /// Reports a file system inspection failure during discovery.
    #[error("failed to access path {path} during discovery")]
    Inaccessible {
        /// Holds the path that the file system could not inspect.
        path: PathBuf,
        /// Holds the I/O failure.
        #[source]
        source: io::Error,
    },
}

/// Reports failures while reading or parsing a Config File.
///
/// Parsing an in-memory buffer does not find paths or check Trust Status.
#[derive(Debug, Error)]
pub(crate) enum ConfigReadError {
    /// Reports a failure while reading a Config File from disk.
    #[error("failed to read config file {path}")]
    Io {
        /// Holds the path of the Config File that could not be read.
        path: PathBuf,
        /// Holds the I/O failure.
        #[source]
        source: io::Error,
    },
    /// Reports a TOML parsing or deserialization failure.
    #[error("failed to parse TOML syntax in {path}")]
    Parse {
        /// Holds the path of the Config File with invalid TOML data.
        path: PathBuf,
        /// Holds the TOML deserialization error.
        #[source]
        source: Box<toml::de::Error>,
    },
}

/// Reports failures during Trust Verification and store operations.
///
/// The Trust Store and Tracker Store use [`crate::FilePathTracker`].
#[derive(Debug, Error)]
pub enum ConfigTrustError {
    /// Reports a Trust Status that blocks configuration loading.
    #[error(
        "config file {path} (workspace root {root}) is untrusted: {status}"
    )]
    Untrusted {
        /// Holds the workspace root whose Trust Status blocks loading.
        root: PathBuf,
        /// Holds the local Config File that failed Trust Verification.
        path: PathBuf,
        /// Holds the Trust Status that blocked loading.
        status: ConfigTrustStatus,
    },
    /// Reports a Trust Store or Tracker Store operation failure.
    #[error(transparent)]
    Tracker(#[from] FilePathTrackerError),
    /// Reports a Config File read failure during Trust Verification.
    #[error(transparent)]
    Hash(#[from] HashError),
}

/// Reports failures while checking parsed configuration values.
///
/// In-memory checks do not read files or run Trust Verification.
#[derive(Debug, Error)]
pub(crate) enum ConfigValueError {
    /// Reports a Template Directory or schema path outside its allowed root.
    #[error("invalid config subdirectory path {path}")]
    InvalidSubDir {
        /// Holds the rejected subdirectory.
        path: PathBuf,
        /// Holds the path check failure.
        #[source]
        source: crate::path::PathError,
    },
    /// Reports an invalid field key in a `frontmatter` or `schemas` table.
    #[error("invalid `{table}` config")]
    InvalidFieldKey {
        /// Names the TOML table with the invalid field key.
        table: &'static str,
        /// Holds the field-name check failure.
        #[source]
        source: FieldNameError,
    },
    /// Reports a `tasks` tag-filter entry that [`crate::Tag`] rejects.
    #[error("invalid `tasks.tag_filters` entry {entry:?}")]
    InvalidTagFilter {
        /// Holds the entry before the builder adds `#` when needed.
        entry: String,
        /// Holds the tag check failure.
        #[source]
        source: TagError,
    },
    /// Reports an invalid task status.
    #[error(transparent)]
    Task(#[from] TaskError),
}

impl ConfigValueError {
    /// Builds an invalid field-key error for the named configuration table.
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
