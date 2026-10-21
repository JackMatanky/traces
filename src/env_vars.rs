//! Provides environment variable parsing and directory filters for workspace
//! discovery.

use std::{collections::HashSet, ffi::OsStr, path::PathBuf, sync::LazyLock};

/// Holds absolute ceiling paths parsed from `TRACES_CEILING_DIRS` where upward
/// discovery stops.
pub(crate) static CEILING_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    std::env::var_os("TRACES_CEILING_DIRS")
        .map(|val| {
            std::env::split_paths(&val)
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default()
});

/// Holds the default directory names ignored during scans; user-configured
/// names from `TRACES_IGNORED_DIRS` are added on top of these.
pub(crate) const DEFAULT_IGNORED_DIRS: [&str; 4] =
    [".git", "target", "node_modules", ".traces"];

/// Holds the directory names to prune during scans: [`DEFAULT_IGNORED_DIRS`]
/// plus any names parsed from `TRACES_IGNORED_DIRS`.
pub(crate) static IGNORED_DIRS: LazyLock<HashSet<String>> =
    LazyLock::new(|| {
        let mut set: HashSet<String> =
            DEFAULT_IGNORED_DIRS.into_iter().map(str::to_owned).collect();
        if let Ok(val) = std::env::var("TRACES_IGNORED_DIRS") {
            for part in val.split([':', ',']) {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    set.insert(trimmed.to_owned());
                }
            }
        }
        set
    });

/// Returns `true` if `file_name` matches any default or user-configured ignored
/// directory name.
#[inline]
pub(crate) fn is_ignored_dir(file_name: &OsStr) -> bool {
    file_name.to_str().is_some_and(|name| IGNORED_DIRS.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ignored_dir {
        use super::*;

        #[test]
        fn returns_true_for_default_ignored_dirs() {
            assert!(is_ignored_dir(OsStr::new(".git")));
            assert!(is_ignored_dir(OsStr::new("target")));
            assert!(is_ignored_dir(OsStr::new("node_modules")));
            assert!(is_ignored_dir(OsStr::new(".traces")));
        }

        #[test]
        fn returns_false_for_regular_dirs() {
            assert!(!is_ignored_dir(OsStr::new("src")));
            assert!(!is_ignored_dir(OsStr::new("notes")));
        }
    }
}
