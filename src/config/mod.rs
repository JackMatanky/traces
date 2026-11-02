//! Loads configuration files and returns resolved values in [`Config`].
//!
//! The discovery, read, trust, and value stages report separate failures.
//! Loading uses these stages in order:
//!
//! 1. [`discovery::ConfigLocator`] finds the nearest local Config File and an
//!    optional global Config File. It does not read TOML.
//! 2. [`file::LocalConfigFile::into_trusted`] records the local Tracked Config
//!    Path in the Tracker Store. It then checks Trust Status before it parses
//!    local TOML. A tracking failure does not stop loading.
//! 3. [`mod@file`] parses trusted local content and reads the global file
//!    without a trust check. It returns [`raw::RawConfig`] values.
//! 4. [`ConfigBuilder`] applies global defaults before local values. It checks
//!    each value and resolves paths. It returns [`Config`].
//!
//! A trust grant records the local Tracked Config Path before it grants trust.
//! Revoking trust leaves the path tracked. [`crate::dirs`] supplies OS paths
//! for both stores.
//!
//! # Config Digests
//!
//! [`Config::digests`] hashes each resolved setting under a section-qualified
//! label. Each stored digest names its setting. [`Config::hash`] combines the
//! digests under `b"traces.config.v1"` to make the Config Fingerprint. This is
//! the only encoding in use.
//!
//! Every digest constructor and bundle destructures its type without `..`.
//! Adding a setting to a configuration type then causes a compile error until
//! the code gives it a label and a place in the encoding. A `syn` scan checks
//! this rule because Rust cannot distinguish a rest pattern from a full
//! pattern.
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
pub(crate) use model::{
    ConfigDigests, ConfigSubDir, DEFAULT_LOCAL_TEMPLATES_DIR, DateFieldDigests,
    FrontmatterDigests, SchemaDigests, TaskDigests,
};
pub(crate) use raw::{RawConfig, RawTemplateConfig};
pub use service::ConfigService;
pub(crate) use tracker::TrackerStore;
#[cfg(test)]
pub(crate) use trust::ConfigTrustStatus;
pub use trust::TrustRequest;
pub(crate) use trust::{TrustRequests, TrustScope, TrustStore};
