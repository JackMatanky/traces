//! Resolves, renders, and writes Markdown templates.
//!
//! The template pipeline has four stages:
//!
//! - Validate a [`TemplatePathInput`] before lookup.
//! - Resolve the template through [`loader`] using local-before-global search.
//! - Render with [`engine`], which wraps minijinja and registers the template
//!   helper namespaces.
//! - Resolve and write the output through [`writer`], honoring dry-run and
//!   commit modes.
//!
//! # Key Types
//!
//! - [`TemplateService`]: Entry point that chains resolution, rendering, and
//!   writing for the CLI.
//! - [`TemplatePathInput`]: Validated relative template identifier.
//! - [`TemplatePathError`]: Validation and lookup failures for template
//!   identifiers.
//! - [`TemplateError`]: User-facing error for resolve, read, render, and write
//!   failures.
//! - [`RenderFailureKind`]: Coarse render-error category for diagnostics.
//! - [`WriteMode`]: Choice between previewing rendered content and committing
//!   it to disk.
//! - [`WriteOutcome`]: Output produced by a write-mode decision.
//! - [`CommitPolicy`]: Existing-file behavior for committed writes.
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
