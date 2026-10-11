//! Loads TOML configuration from the filesystem into a resolved [`Config`].
//!
//! # Pipeline
//!
//! 1. **Discover** local `.traces/config.toml` and optional global
//!    `traces/config.toml` from a directory anchor
//!    ([`discovery::ConfigLocator`]).
//! 2. **Track** the local config in a best-effort store
//!    ([`tracker::TrackerStore`]) and **verify trust** before parsing its
//!    content; reject untrusted or stale files ([`trust::TrustStore`]). Both
//!    steps happen in one transition, [`file::LocalConfigFile::into_trusted`].
//! 3. **Parse** TOML into [`raw::RawConfig`] ([`mod@file`]).
//! 4. **Merge** global config before local so local values win
//!    ([`service::ConfigService`]).
//!
//! Path resolution and default filling happen in [`model`], which produces the
//! final [`Config`].
//!
//! # Main Types
//!
//! - [`ConfigService`] owns the load pipeline and delegates durable state to
//!   its two stores.
//! - [`trust::TrustStore`] and [`tracker::TrackerStore`] persist trust
//!   decisions and the seen-config registry.
//! - [`Config`] holds merged settings ready for consumers.
//! - [`TaskConfig`] holds resolved task status mappings and tag filters.
//! - [`TrustRequest`] names a workspace root or config file for trust
//!   operations.
//! - [`error::ConfigError`] composes the four stage errors raised by
//!   [`ConfigService::load`].
//!
//! Filesystem paths are resolved by the [`crate::dirs`] module.

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
