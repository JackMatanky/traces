//! Centralizes access to the process current working directory.
//!
//! [`Cwd`] is the only production wrapper around [`env::current_dir`].
//! `clippy.toml` disallows direct calls elsewhere, so all working directory
//! access goes through this boundary.

use std::{
    env, io,
    path::{Path, PathBuf},
};

/// Captures the process current working directory at construction time.
///
/// Wraps [`env::current_dir`] to keep working directory access auditable at a
/// single boundary (enforced by `clippy.toml`).
#[derive(Clone, Debug)]
pub(crate) struct Cwd(PathBuf);

impl Cwd {
    /// Reads the process current working directory.
    ///
    /// # Errors
    ///
    /// - [`io::Error`] if the current directory does not exist or the process
    ///   lacks access.
    #[inline]
    #[expect(
        clippy::disallowed_methods,
        reason = "sole canonical call site for process cwd access"
    )]
    pub(crate) fn new() -> io::Result<Self> {
        env::current_dir().map(Self)
    }

    /// Consumes the wrapper and returns the inner [`PathBuf`].
    #[inline]
    #[must_use]
    pub(crate) fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for Cwd {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Serializes every [`CwdGuard`]-mediated test.
///
/// The process current working directory is global state, so concurrent changes
/// race. A dropped `tempfile::TempDir` can also remove the directory underneath
/// another test. Each [`CwdGuard`] holds this lock for its whole lifetime.
#[cfg(test)]
static CWD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Enters a directory for a scope and restores the original on drop.
///
/// Tests that change the working directory should use this instead of calling
/// [`env::set_current_dir`] directly. Holding [`CWD_TEST_LOCK`] serializes
/// concurrent working-directory changes.
#[cfg(test)]
pub(crate) struct CwdGuard {
    /// Original working directory that the guard restores on drop.
    original: PathBuf,
    /// Mutex guard serializing working-directory changes across tests.
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl CwdGuard {
    /// Saves the current directory and enters `dir`, holding [`CWD_TEST_LOCK`]
    /// until dropped.
    ///
    /// # Panics
    ///
    /// Panics if reading the current directory fails or changing to `dir`
    /// fails. Both failures indicate a broken test environment.
    #[expect(
        clippy::disallowed_methods,
        reason = "CwdGuard is the sole authorized caller of \
                  env::set_current_dir"
    )]
    #[inline]
    #[must_use]
    pub(crate) fn enter(dir: &Path) -> Self {
        let lock = CWD_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = Cwd::new().expect("read current dir").0;
        env::set_current_dir(dir).expect("enter test directory");
        Self {
            original,
            _lock: lock,
        }
    }

    /// Holds [`CWD_TEST_LOCK`] while keeping the working directory unchanged.
    ///
    /// Protects tests that never change the directory from observing another
    /// thread's [`CwdGuard`]-mediated change, because the current working
    /// directory is global state.
    #[inline]
    #[must_use]
    pub(crate) fn same_dir() -> Self {
        let lock = CWD_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = Cwd::new().expect("read current dir").0;
        Self {
            original,
            _lock: lock,
        }
    }
}

#[cfg(test)]
impl Drop for CwdGuard {
    #[expect(
        clippy::disallowed_methods,
        reason = "CwdGuard is the sole authorized caller of \
                  env::set_current_dir"
    )]
    #[inline]
    fn drop(&mut self) {
        env::set_current_dir(&self.original).expect("restore original cwd");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the current working directory under [`CWD_TEST_LOCK`].
    ///
    /// Reading without holding the lock can observe another thread's
    /// [`CwdGuard`]-entered directory disappearing mid-read once that guard
    /// drops and removes the backing `TempDir`.
    fn locked_cwd() -> Cwd {
        let _lock = CWD_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Cwd::new().expect("read cwd")
    }

    #[test]
    fn new_returns_a_non_empty_path() {
        let cwd = locked_cwd();
        assert!(!cwd.as_ref().as_os_str().is_empty());
    }

    #[test]
    fn into_inner_returns_the_same_path() {
        let cwd = locked_cwd();
        let path = cwd.as_ref().to_path_buf();
        assert_eq!(cwd.into_inner(), path);
    }

    #[test]
    fn guard_enters_and_restores_on_drop() {
        let original = locked_cwd();
        let temp = tempfile::tempdir().expect("create temp dir");
        {
            let _guard = CwdGuard::enter(temp.path());
            // `current_dir` returns the non-verbatim spelling on Windows while
            // `canonicalize` returns `\\?\`, so canonicalize both sides.
            assert_eq!(
                Cwd::new()
                    .expect("read inside guard")
                    .into_inner()
                    .canonicalize()
                    .expect("canonicalize cwd"),
                temp.path().canonicalize().expect("canonicalize temp dir")
            );
        }
        assert_eq!(
            Cwd::new().expect("read after drop").into_inner(),
            original.into_inner()
        );
    }
}
