//! Implements [`DialogProvider`] for interactive terminals using [`inquire`].

use super::{DialogError, DialogProvider, DialogResult};

/// Prompts users through an interactive terminal with [`inquire`].
///
/// Delegates to [`inquire`] when standard input is a terminal. When standard
/// input is not a terminal (CI, pipes, scripts, dry runs), it returns fallback
/// values instead of blocking:
///
/// - [`DialogProvider::text`]: default or `""`
/// - [`DialogProvider::confirm`]: default or `false`
/// - [`DialogProvider::select`]: index `0`
/// - [`DialogProvider::multi_select`]: empty [`Vec`]
///
/// [`DialogProvider::select`] checks for empty `items` before the terminal
/// guard and returns [`DialogError::EmptySelectionInput`] in every runtime
/// mode.
///
/// # Examples
///
/// ```
/// use traces::{DialogProvider, TerminalDialogProvider};
///
/// let p = TerminalDialogProvider::new();
/// // When standard input is not a terminal, methods return fallback defaults:
/// let name = p.text("name", Some("carol"))?;
/// assert_eq!(name, "carol");
/// # Ok::<_, traces::DialogError>(())
/// ```
#[derive(Copy, Clone, Debug, Default)]
pub struct TerminalDialogProvider;

impl TerminalDialogProvider {
    /// Creates a [`TerminalDialogProvider`] with default prompt behavior.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl DialogProvider for TerminalDialogProvider {
    #[inline]
    fn is_interactive(&self) -> bool {
        stdin_is_tty()
    }

    #[inline]
    fn text(&self, label: &str, default: Option<&str>) -> DialogResult<String> {
        if !stdin_is_tty() {
            return Ok(default.unwrap_or_default().to_owned());
        }
        let mut prompt = inquire::Text::new(label);
        if let Some(d) = default {
            prompt = prompt.with_default(d);
        }
        Ok(prompt.prompt()?)
    }

    #[inline]
    fn confirm(
        &self,
        label: &str,
        default: Option<bool>,
    ) -> DialogResult<bool> {
        if !stdin_is_tty() {
            return Ok(default.unwrap_or(false));
        }
        let mut prompt = inquire::Confirm::new(label);
        if let Some(d) = default {
            prompt = prompt.with_default(d);
        }
        Ok(prompt.prompt()?)
    }

    #[inline]
    fn select(&self, label: &str, items: &[String]) -> DialogResult<usize> {
        // A select over zero options can never yield a choice, in either mode.
        if items.is_empty() {
            return Err(DialogError::EmptySelectionInput);
        }
        if !stdin_is_tty() {
            return Ok(0);
        }
        Ok(inquire::Select::new(label, items.iter().collect())
            .raw_prompt()?
            .index)
    }

    #[inline]
    fn multi_select(
        &self,
        label: &str,
        items: &[String],
    ) -> DialogResult<Vec<usize>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        if !stdin_is_tty() {
            return Ok(Vec::new());
        }
        Ok(inquire::MultiSelect::new(label, items.iter().collect())
            .raw_prompt()?
            .into_iter()
            .map(|op| op.index)
            .collect())
    }
}

/// Returns `true` when standard input is connected to an interactive terminal.
#[inline]
fn stdin_is_tty() -> bool {
    use std::io::IsTerminal as _;
    std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {

    use super::*;

    fn skip_if_tty(test: &str) -> bool {
        let is_tty = stdin_is_tty();
        if is_tty {
            eprintln!(
                "skipping {test}: stdin is a real TTY, cannot assert the \
                 non-TTY fallback"
            );
        }
        is_tty
    }

    mod text {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_default_when_not_a_tty() {
            if skip_if_tty("returns_default_when_not_a_tty") {
                return;
            }
            let p = TerminalDialogProvider::new();
            assert_eq!(p.text("name", Some("carol")).unwrap(), "carol");
        }

        #[test]
        fn returns_empty_when_not_a_tty_and_no_default() {
            if skip_if_tty("returns_empty_when_not_a_tty_and_no_default") {
                return;
            }
            let p = TerminalDialogProvider::new();
            assert_eq!(p.text("name", None).unwrap(), "");
        }
    }

    mod confirm {
        use super::*;

        #[test]
        fn returns_default_when_not_a_tty() {
            if skip_if_tty("returns_default_when_not_a_tty") {
                return;
            }
            let p = TerminalDialogProvider::new();
            assert!(p.confirm("ok?", Some(true)).unwrap());
            assert!(!p.confirm("ok?", Some(false)).unwrap());
        }

        #[test]
        fn returns_false_when_not_a_tty_and_no_default() {
            if skip_if_tty("returns_false_when_not_a_tty_and_no_default") {
                return;
            }
            let p = TerminalDialogProvider::new();
            assert!(!p.confirm("ok?", None).unwrap());
        }
    }

    mod object_safety {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn is_usable_as_dyn_when_not_a_tty() {
            if skip_if_tty("is_usable_as_dyn_when_not_a_tty") {
                return;
            }
            let concrete = TerminalDialogProvider::new();
            let p: &dyn DialogProvider = &concrete;
            assert_eq!(p.text("l", Some("d")).unwrap(), "d");
            assert!(p.confirm("l", Some(true)).unwrap());
        }
    }

    mod select {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_index_zero_when_not_a_tty() {
            if skip_if_tty("returns_index_zero_when_not_a_tty") {
                return;
            }
            let items = vec!["first".to_owned(), "second".to_owned()];
            let p = TerminalDialogProvider::new();

            assert_eq!(p.select("pick", &items).unwrap(), 0);
        }

        #[test]
        fn returns_error_when_items_are_empty() {
            let p = TerminalDialogProvider::new();

            assert!(matches!(
                p.select("pick", &[]),
                Err(DialogError::EmptySelectionInput)
            ));
        }
    }

    mod multi_select {
        use super::*;

        #[test]
        fn returns_empty_when_items_are_empty() {
            use pretty_assertions::assert_eq;
            let p = TerminalDialogProvider::new();
            assert_eq!(
                p.multi_select("pick", &[]).unwrap(),
                Vec::<usize>::new()
            );
        }

        #[test]
        fn returns_empty_when_not_a_tty() {
            use pretty_assertions::assert_eq;
            if skip_if_tty("returns_empty_when_not_a_tty") {
                return;
            }
            let items = vec!["a".to_owned(), "b".to_owned()];
            let p = TerminalDialogProvider::new();

            assert_eq!(
                p.multi_select("pick", &items).unwrap(),
                Vec::<usize>::new()
            );
        }
    }

    mod is_interactive {
        use super::*;

        #[test]
        fn returns_false_when_not_a_tty() {
            if skip_if_tty("returns_false_when_not_a_tty") {
                return;
            }
            let p = TerminalDialogProvider::new();
            assert!(!p.is_interactive());
        }
    }
}
