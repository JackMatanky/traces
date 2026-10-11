//! Loads TOML configuration from the filesystem into a resolved [`Config`].
//!
//! # Pipeline
//!
//! 1. **Discover** local `.traces/config.toml` and optional global
//!    `traces/config.toml` from a directory anchor ([`discovery`]).
//! 2. **Track** discovered local configs in a best-effort store
//!    ([`tracker::TrackerStore`]).
//! 3. **Verify trust** before parsing local content; reject untrusted or stale
//!    files ([`trust::TrustStore`]).
//! 4. **Parse** TOML into [`raw::RawConfig`] ([`mod@file`]).
//! 5. **Merge** global config before local so local values win
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
pub(crate) use discovery::{
    LOCAL_CONFIG_DIR, LOCAL_CONFIG_FILE, TrustScope, trust_requests,
};
pub(crate) use error::{
    ConfigDiscoveryError, ConfigError, ConfigReadError, ConfigTrustError,
    ConfigValueError,
};
#[cfg(any(test, feature = "test-utils"))]
pub(crate) use file::{Discovered, LocalConfigFile};
pub(crate) use model::DEFAULT_LOCAL_TEMPLATES_DIR;
pub use model::{Config, FrontmatterConfig, SchemasConfig, TaskConfig};
pub(crate) use raw::{RawConfig, RawTemplateConfig};
pub use service::ConfigService;
pub(crate) use tracker::TrackerStore;
#[cfg(test)]
pub(crate) use trust::ConfigTrustStatus;
pub use trust::TrustRequest;
pub(crate) use trust::{TrustRequests, TrustStore};
