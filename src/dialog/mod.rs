//! Defines the I/O seam separating template rendering from user interaction.
//!
//! The [`DialogProvider`] trait provides the object-safe contract for prompting
//! users and collecting responses. Two implementations cover the supported
//! runtime modes:
//!
//! - [`PresetDialogProvider`] replays queued responses for tests and unattended
//!   MCP execution.
//! - [`TerminalDialogProvider`] delegates to [`inquire`] for real terminal
//!   interaction and returns fallback values when standard input is not a
//!   terminal.
//!
//! # Key Types
//!
//! - [`DialogProvider`]: object-safe trait for prompting and collecting
//!   responses.
//! - [`PresetDialogProvider`]: deterministic replay provider for tests and
//!   automation.
//! - [`TerminalDialogProvider`]: interactive terminal provider backed by
//!   [`inquire`].
//! - [`DialogError`]: error type for dialog prompt failures.
//!
//! # Selection by Position
//!
//! [`DialogProvider::select`] and [`DialogProvider::multi_select`] return
//! indices into the `items` slice, not copied labels. Index-based selection
//! lets callers recover non-string values from a parallel list and keeps
//! duplicate labels distinguishable.

mod error;
mod preset;
mod terminal;

pub use error::{DialogError, DialogResult};
pub use preset::PresetDialogProvider;
pub use terminal::TerminalDialogProvider;

/// Prompts users and collects responses behind an object-safe interface.
///
/// The trait is object-safe, so callers can hold a `&dyn DialogProvider` chosen
/// at runtime. Thread-safe template-rendering closures capture shared
/// providers, so implementations need both [`Send`] and [`Sync`].
///
/// [`Self::select`] and [`Self::multi_select`] return indices into the `items`
/// slice. See the module-level documentation for the rationale.
pub trait DialogProvider: Send + Sync {
    /// Reports whether this provider can do interactive prompting.
    ///
    /// Returns `false` when standard input is not a terminal or when no preset
    /// answers remain.
    #[inline]
    #[must_use]
    fn is_interactive(&self) -> bool {
        true
    }

    /// Displays a yes/no confirmation prompt.
    ///
    /// Shows `label` as the prompt message. When the user provides no input,
    /// this method returns `default`, or `false` if `default` is `None`.
    ///
    /// # Errors
    ///
    /// - [`DialogError::UserCancelled`] if the user cancels the prompt.
    /// - [`DialogError::UserInterrupted`] if the user interrupts (such as
    ///   pressing Control-C).
    /// - [`DialogError::NotInteractive`] if standard input is not a terminal.
    /// - [`DialogError::Io`] if an I/O error occurs during prompting.
    /// - [`DialogError::BackendFailure`] if the backend reports an unexpected
    ///   error.
    ///
    /// # Examples
    ///
    /// ```
    /// use traces::{DialogProvider, PresetDialogProvider};
    ///
    /// let p = PresetDialogProvider::new().with_confirm(true);
    /// assert!(p.confirm("proceed?", None)?);
    /// # Ok::<_, traces::DialogError>(())
    /// ```
    fn confirm(&self, label: &str, default: Option<bool>)
    -> DialogResult<bool>;

    /// Displays a multi-selection prompt and returns the chosen indices.
    ///
    /// Shows `label` above the selectable `items`. An empty `items` slice
    /// yields an empty [`Vec`] (not an error).
    ///
    /// # Errors
    ///
    /// - [`DialogError::UserCancelled`] if the user cancels the prompt.
    /// - [`DialogError::UserInterrupted`] if the user interrupts.
    /// - [`DialogError::NotInteractive`] if standard input is not a terminal.
    /// - [`DialogError::Io`] if an I/O error occurs during prompting.
    /// - [`DialogError::BackendFailure`] if the backend reports an unexpected
    ///   error.
    ///
    /// # Examples
    ///
    /// ```
    /// use traces::{DialogProvider, PresetDialogProvider};
    ///
    /// let items = vec!["x".into(), "y".into(), "z".into()];
    /// let p = PresetDialogProvider::new().with_multi_select([0, 2]);
    /// assert_eq!(p.multi_select("pick", &items)?, vec![0, 2]);
    /// # Ok::<_, traces::DialogError>(())
    /// ```
    fn multi_select(
        &self,
        label: &str,
        items: &[String],
    ) -> DialogResult<Vec<usize>>;

    /// Displays a single-selection prompt and returns the chosen index.
    ///
    /// Shows `label` above the selectable `items`. Index-based selection lets
    /// the caller recover the chosen entry from a parallel list, supporting
    /// non-string item types and disambiguating duplicate labels. See the
    /// module-level documentation for the full rationale.
    ///
    /// # Errors
    ///
    /// - [`DialogError::EmptySelectionInput`] if `items` is empty.
    /// - [`DialogError::InvalidConfiguration`] if a queued index is out of
    ///   bounds (only [`PresetDialogProvider`]).
    /// - [`DialogError::UserCancelled`] if the user cancels the prompt.
    /// - [`DialogError::UserInterrupted`] if the user interrupts.
    /// - [`DialogError::NotInteractive`] if standard input is not a terminal.
    /// - [`DialogError::Io`] if an I/O error occurs during prompting.
    /// - [`DialogError::BackendFailure`] if the backend reports an unexpected
    ///   error.
    ///
    /// # Examples
    ///
    /// ```
    /// use traces::{DialogProvider, PresetDialogProvider};
    ///
    /// let items = vec!["alpha".into(), "beta".into(), "gamma".into()];
    /// let p = PresetDialogProvider::new().with_select(1);
    /// assert_eq!(p.select("pick", &items)?, 1);
    /// # Ok::<_, traces::DialogError>(())
    /// ```
    fn select(&self, label: &str, items: &[String]) -> DialogResult<usize>;

    /// Displays a text prompt and returns the user's input.
    ///
    /// Shows `label` as the prompt message. When the user submits an empty
    /// string, this method returns `default`, or an empty string if `default`
    /// is `None`.
    ///
    /// # Errors
    ///
    /// - [`DialogError::UserCancelled`] if the user cancels the prompt.
    /// - [`DialogError::UserInterrupted`] if the user interrupts.
    /// - [`DialogError::NotInteractive`] if standard input is not a terminal.
    /// - [`DialogError::Io`] if an I/O error occurs during prompting.
    /// - [`DialogError::BackendFailure`] if the backend reports an unexpected
    ///   error.
    ///
    /// # Examples
    ///
    /// ```
    /// use traces::{DialogProvider, PresetDialogProvider};
    ///
    /// let p = PresetDialogProvider::new().with_text("claude");
    /// assert_eq!(p.text("name", None)?, "claude");
    /// # Ok::<_, traces::DialogError>(())
    /// ```
    fn text(&self, label: &str, default: Option<&str>) -> DialogResult<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PresetDialogProvider>();
        assert_send_sync::<TerminalDialogProvider>();
        assert_send_sync::<std::sync::Arc<dyn DialogProvider>>();
    }

    #[test]
    fn is_interactive_default_returns_true() {
        struct DummyProvider;

        impl DialogProvider for DummyProvider {
            fn confirm(
                &self,
                _: &str,
                _: Option<bool>,
            ) -> Result<bool, DialogError> {
                Ok(false)
            }

            fn multi_select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<Vec<usize>, DialogError> {
                Ok(vec![])
            }

            fn select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<usize, DialogError> {
                Ok(0)
            }

            fn text(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<String, DialogError> {
                Ok(String::new())
            }
        }

        let p = DummyProvider;
        assert!(p.is_interactive());
    }
}
