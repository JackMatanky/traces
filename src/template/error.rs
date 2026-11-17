//! Defines errors for template resolution, rendering, and writing.
//!
//! [`TemplateError`] reports the failed pipeline stage and preserves source
//! errors. CLI code adds diagnostic codes and help text outside this module.
//!
//! # Key Types
//!
//! - [`TemplateError`]: reports resolution, read, render, output-path, write,
//!   and prompt errors.
//! - [`RenderFailureKind`]: classifies render errors for CLI diagnostics.

use std::{error::Error as StdError, io, path::PathBuf};

use thiserror::Error;

use super::path::TemplatePathError;
use crate::{DialogError, index::IndexError, query::QueryError};

/// Reports the failed stage of a template operation.
///
/// Variants preserve source errors when available. Callers can match a variant
/// instead of parsing display text.
#[derive(Debug, Error)]
pub enum TemplateError {
    /// Reports a failure to resolve the requested template identifier.
    ///
    /// Covers validation errors, ambiguous stem matches, unreadable template
    /// directories, and missing templates. The wrapped [`TemplatePathError`]
    /// identifies the condition.
    #[error(transparent)]
    Resolve(#[from] TemplatePathError),

    /// Reports a failure to read the resolved template source.
    #[error("failed to read template {path}")]
    Read {
        /// Absolute path selected during template resolution.
        path: PathBuf,
        /// Filesystem error from the source-file read.
        #[source]
        source: io::Error,
    },

    /// Reports an existing output file when the policy is
    /// [`CommitPolicy::CreateNew`].
    ///
    /// [`CommitPolicy::Overwrite`] truncates the existing file.
    ///
    /// [`CommitPolicy::CreateNew`]: super::writer::CommitPolicy::CreateNew
    /// [`CommitPolicy::Overwrite`]: super::writer::CommitPolicy::Overwrite
    #[error("output file already exists at {path}")]
    OutputFileAlreadyExists {
        /// Existing file that blocks the write.
        path: PathBuf,
    },

    /// Reports an output path that escapes the project root.
    ///
    /// Covers absolute `-o` and `file.write_to()` paths, paths with unsafe
    /// components such as `..`, and paths that use an external symlink.
    #[error("output path {path} escapes the project root")]
    OutputPathEscapesRoot {
        /// Rejected output path from the caller or template.
        path: PathBuf,
    },

    /// Reports an explicit path that fails the root confinement check.
    ///
    /// Happens when `strict-path` cannot check the root boundary or resolve
    /// existing path components. The writer stops instead of writing to a path
    /// that it has not checked.
    #[error("failed to verify output path {path} is inside the project root")]
    OutputPathUnverifiable {
        /// Output path rejected by the root confinement check.
        path: PathBuf,
        /// Filesystem error from the path-boundary check.
        #[source]
        source: io::Error,
    },

    /// Reports a failure to render the template source.
    ///
    /// Covers `MiniJinja` syntax errors, missing helpers, and bad helper
    /// arguments, plus include or extends errors, query/index/schema errors,
    /// file include errors, and dialog errors.
    #[error("failed to render template {path}")]
    Render {
        /// Absolute path used as the `MiniJinja` template name.
        path: PathBuf,
        /// `MiniJinja` error with template context and source-chain details.
        #[source]
        source: minijinja::Error,
    },

    /// Reports a failure to write rendered output.
    ///
    /// Covers parent-directory creation, file creation except existing-file
    /// collisions reported by [`Self::OutputFileAlreadyExists`], and writes of
    /// rendered bytes.
    #[error("failed to write rendered output to {path}")]
    Write {
        /// Template output path being written.
        path: PathBuf,
        /// Filesystem error from file creation or writing.
        #[source]
        source: io::Error,
    },

    /// Reports a failed or canceled output-collision prompt.
    ///
    /// Covers the writer prompt before it replaces an output file.
    #[error(transparent)]
    Prompt(#[from] DialogError),

    /// Reports a registry-wide Schema loading error during service
    /// construction.
    ///
    /// Covers these registry-wide errors:
    ///
    /// - The service cannot read or list the registry directory.
    /// - A Schema TOML file has invalid syntax or an unknown key.
    /// - The `extends` DAG contains a cycle.
    ///
    /// The wrapped [`SchemaError`] identifies the failure.
    ///
    /// A field-level error in a valid Schema TOML file does not reach this
    /// variant. Examples include an invalid attribute value, an out-of-bounds
    /// `$ref`, or an ambiguous field name. The registry excludes that Schema
    /// and logs a construction error. Other Schemas remain available.
    ///
    /// [`SchemaError`]: crate::schema::SchemaError
    #[error(transparent)]
    #[expect(
        private_interfaces,
        reason = "SchemaError is deliberately crate-internal; TemplateError \
                  is only pub behind the test-utils/test cfg, and transparent \
                  wrapping only needs Display/Error, never external code \
                  naming SchemaError itself"
    )]
    SchemaLoad(#[from] crate::schema::SchemaError),
}

/// Classifies template rendering failures for CLI diagnostics.
///
/// The method checks [`minijinja::Error::kind`] and the retained source chain.
/// It does not parse display text. New template helpers need no CLI string
/// matching.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RenderFailureKind {
    /// Reports invalid `MiniJinja` syntax in the template.
    Syntax,
    /// Reports a failed `ui.*` prompt, except for a deliberate user abort.
    Prompt,
    /// Reports a `query.*` error from a malformed field path, filter, sort
    /// path, or `.table()` header and column mismatch.
    Query,
    /// Reports a filesystem, database, or serialization error while a `query.*`
    /// call refreshes the file index.
    Index,
    /// Reports an I/O error from `file.include()` or another template helper.
    Io,
    /// Reports another engine error, such as an unknown function, filter, or
    /// test, a bad argument, or an undefined-value operation.
    Other,
}

impl RenderFailureKind {
    /// Classifies a [`minijinja::Error`] as a [`RenderFailureKind`].
    ///
    /// The method checks the error kind and source chain. It does not parse
    /// display text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "test-utils")]
    /// # {
    /// use minijinja::{Error, ErrorKind};
    /// use traces::testing::RenderFailureKind;
    ///
    /// let error = Error::new(ErrorKind::SyntaxError, "bad syntax");
    /// assert_eq!(RenderFailureKind::classify(&error), RenderFailureKind::Syntax);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn classify(error: &minijinja::Error) -> Self {
        if error.kind() == minijinja::ErrorKind::SyntaxError {
            return Self::Syntax;
        }
        let mut cause: Option<&(dyn StdError + 'static)> =
            StdError::source(error);
        while let Some(err) = cause {
            if err.downcast_ref::<DialogError>().is_some() {
                return Self::Prompt;
            }
            if err.downcast_ref::<QueryError>().is_some() {
                return Self::Query;
            }
            if err.downcast_ref::<IndexError>().is_some() {
                return Self::Index;
            }
            if err.downcast_ref::<io::Error>().is_some() {
                return Self::Io;
            }
            cause = err.source();
        }
        Self::Other
    }
}
