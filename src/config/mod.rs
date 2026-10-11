//! Loads Config Files into a resolved [`Config`].
//!
//! The discovery, read, trust and value stages separate failure types. Loading
//! follows this order:
//!
//! 1. [`discovery::ConfigLocator`] finds the nearest local Config File and an
//!    optional global Config File without reading TOML.
//! 2. [`file::LocalConfigFile::into_trusted`] records the local Tracked Config
//!    Path in the Tracker Store on a best-effort basis, then performs Trust
//!    Verification against the Trust Store before parsing local TOML.
//! 3. [`mod@file`] parses the trusted local content and reads the global file
//!    without a trust gate, producing [`raw::RawConfig`] values.
//! 4. [`ConfigBuilder`] merges global defaults before local values, validates
//!    them and resolves paths into [`Config`].
//!
//! Granting trust records a local Tracked Config Path before granting trust;
//! revoking trust leaves that path tracked. [`crate::dirs`] supplies OS paths
//! for both stores.
//!
//! # Key Types
//!
//! - [`ConfigService`]: loads configuration and administers trust.
//! - [`Config`]: holds resolved values and the workspace root.
//! - [`TaskConfig`]: holds Task Configuration status mappings and tag filters.
//! - [`TrustRequest`]: identifies a workspace root or local Config File for
//!   trust operations.
//! - [`trust::TrustStore`]: persists trusted roots and Companion Hashes.
//! - [`tracker::TrackerStore`]: persists Tracked Config Paths independently of
//!   trust decisions.
//! - [`error::ConfigError`]: combines failures from the four stages.

mod builder;
mod discovery;
mod error;
mod file;
mod model;
mod raw;
mod service;
mod tracker;
mod trust;

pub(crate) use builder::ConfigBuilder;
pub(crate) use error::{
    ConfigDiscoveryError, ConfigError, ConfigReadError, ConfigTrustError,
    ConfigValueError,
};
#[cfg(any(test, feature = "test-utils"))]
pub(crate) use file::LocalConfigFile;
pub(crate) use file::{LOCAL_CONFIG_DIR, LOCAL_CONFIG_FILE};
pub use model::{Config, FrontmatterConfig, SchemasConfig, TaskConfig};
pub(crate) use model::{ConfigSubDir, DEFAULT_LOCAL_TEMPLATES_DIR};
pub(crate) use raw::{RawConfig, RawTemplateConfig};
pub use service::ConfigService;
pub(crate) use tracker::TrackerStore;
#[cfg(test)]
pub(crate) use trust::ConfigTrustStatus;
pub use trust::TrustRequest;
pub(crate) use trust::{TrustRequests, TrustScope, TrustStore};
