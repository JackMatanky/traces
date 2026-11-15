//! Resolves, renders, and writes Markdown templates.
//!
//! The template pipeline has four stages:
//!
//! - Validate a [`TemplatePathInput`] before resolution.
//! - Resolve the template with [`loader`] in local-before-global order.
//! - Render the template with [`engine`], which registers template helper
//!   namespaces.
//! - Resolve and write the output with [`writer`], according to
//!   [`WriteMode::DryRun`] or [`WriteMode::Commit`].
//!
//! # Key Types
//!
//! - [`TemplateService`]: chains template resolution, rendering, and writing.
//! - [`TemplatePathInput`]: validated relative template identifier.
//! - [`TemplatePathError`]: validation and resolution errors for identifiers.
//! - [`TemplateError`]: error shown in CLI output for pipeline failures.
//! - [`RenderFailureKind`]: render-error category for CLI diagnostics.
//! - [`WriteMode`]: selects whether the service previews or writes output.
//! - [`WriteOutcome`]: result of the selected write mode.
//! - [`CommitPolicy`]: controls how a write handles an existing file.
//!
//! [`CommitPolicy`]: writer::CommitPolicy

pub use error::{RenderFailureKind, TemplateError};
pub use path::{TemplatePathError, TemplatePathInput};
pub use service::TemplateService;
#[cfg(any(test, feature = "test-utils"))]
pub use writer::CommitPolicy;
pub use writer::{WriteMode, WriteOutcome};

mod engine;
mod error;
mod loader;
mod path;
mod service;
mod writer;
