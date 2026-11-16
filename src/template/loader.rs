//! Searches local and global template directories for templates.
//!
//! [`TemplateLoader::find`] is the single resolution path for validated
//! top-level `-i <path>` and minijinja `{% include %}`/`{% extends %}` inputs.
//! Its caller must pass a [`TemplatePathInput`], which proves the raw path was
//! validated before search. It then searches [`TemplateLoader::directories`] in
//! local-before-global order with [`TemplateLoader::find_path_in`] and
//! [`TemplateLoader::find_name_in`].
//!
//! Validation rejects an absolute name, `..`, an empty name, or bare `.` before
//! it searches any directory. It returns [`TemplatePathError::Absolute`] or
//! [`TemplatePathError::UnsafeComponent`].
//!
//! [`TemplatePathError::TemplateNotFound`] is different.
//! [`TemplateLoader::find`] returns it only after it searches every directory
//! and finds no match.
//!
//! # Why Not `minijinja::path_loader`
//!
//! [`TemplateLoader::load`] supplies minijinja's `{% include %}` and `{%
//! extends %}` loader callback. [`TemplateEngine::new`] connects it with
//! [`Environment::set_loader`]. The code does
//! not use [`minijinja::path_loader`] because its `safe_join` rejects
//! dot-prefixed path segments. That rule rejects `{% include ".draft.md" %}`
//! and the default template directory, `.traces/templates`.
//!
//! [`Environment::set_loader`]: minijinja::Environment::set_loader
//! [`TemplateEngine::new`]: super::engine::TemplateEngine::new

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use minijinja::{Error, ErrorKind};

use super::path::{TemplatePath, TemplatePathError, TemplatePathInput};
use crate::{Config, DirTree};

/// Template loader searches at most one local directory and one global
/// directory, in that order.
///
/// Production code builds it from [`Config::local_template_dir`] and
/// [`Config::global_template_dir`] with [`From`]. Tests use [`Self::new`]
/// without [`Config`].
///
/// [`Clone`] is cheap because the struct stores two [`Option<PathBuf>`] values.
/// [`TemplateEngine::new`] clones one loader into minijinja's `set_loader`
/// callback and keeps the original for [`Self::find`]. This avoids a second
/// derivation from [`Config`].
///
/// [`TemplateEngine::new`]: super::engine::TemplateEngine::new
#[derive(Clone, Debug)]
pub(super) struct TemplateLoader {
    local: Option<PathBuf>,
    global: Option<PathBuf>,
}

impl TemplateLoader {
    /// Builds a loader from explicit `local`/`global` directories, bypassing
    /// [`Config`].
    #[inline]
    #[must_use]
    pub(super) const fn new(
        local: Option<PathBuf>,
        global: Option<PathBuf>,
    ) -> Self {
        Self {
            local,
            global,
        }
    }

    /// Searches the local and global directories for a validated template path
    /// input.
    ///
    /// Top-level `-i` resolution and `{% include %}`/`{% extends %}` loading
    /// both use this method after validation.
    ///
    /// # Errors
    ///
    /// - [`AmbiguousTemplate`] if `name`'s stem matches multiple files within a
    ///   single template directory.
    /// - [`DirectoryRead`] if reading a template directory fails during stem
    ///   lookup.
    /// - [`TemplateNotFound`] if no template directory has a match.
    /// - [`UnsafeComponent`] if a matched file name is not a safe relative
    ///   path.
    /// - [`Absolute`] if a matched file name is absolute.
    ///
    /// [`AmbiguousTemplate`]: TemplatePathError::AmbiguousTemplate
    /// [`DirectoryRead`]: TemplatePathError::DirectoryRead
    /// [`TemplateNotFound`]: TemplatePathError::TemplateNotFound
    /// [`UnsafeComponent`]: TemplatePathError::UnsafeComponent
    /// [`Absolute`]: TemplatePathError::Absolute
    pub(super) fn find(
        &self,
        name: &TemplatePathInput,
    ) -> Result<TemplatePath, TemplatePathError> {
        for dir in self.directories() {
            if let Some(template) = Self::find_path_in(dir, name) {
                return Ok(template);
            }
            if let Some(template) = Self::find_name_in(dir, name)? {
                return Ok(template);
            }
        }

        Err(TemplatePathError::TemplateNotFound(name.as_ref().to_path_buf()))
    }

    /// Returns the search directories in local-first order and removes
    /// duplicate paths.
    ///
    /// The method does not canonicalize paths. It searches two paths twice if
    /// they name the same location with different spellings, such as a path
    /// that uses a symlink.
    fn directories(&self) -> impl Iterator<Item = &Path> {
        let global = self
            .global
            .as_deref()
            .filter(|dir| self.local.as_deref() != Some(*dir));
        self.local.as_deref().into_iter().chain(global)
    }

    /// Finds `name` inside `dir` and returns a [`TemplatePath`] if a matching
    /// file exists.
    ///
    /// The method uses [`fs::symlink_metadata`] instead of [`Path::is_file`],
    /// like [`Self::find_name_in`]. A symlink never counts as a match.
    ///
    /// [`Path::is_file`]: std::path::Path::is_file
    fn find_path_in(
        dir: &Path,
        name: &TemplatePathInput,
    ) -> Option<TemplatePath> {
        fs::symlink_metadata(dir.join(name.as_ref()))
            .is_ok_and(|metadata| metadata.is_file())
            .then(|| TemplatePath::verified(name.clone(), dir.to_path_buf()))
    }

    /// Matches `name` by file stem inside `dir` when `name` has no extension.
    ///
    /// The method searches `dir` or the subdirectory named by `path`'s parent
    /// component. It looks for files with `path`'s stem. It returns `None`
    /// when no file matches and `Some` when one file matches.
    ///
    /// Like [`Self::find_path_in`], it excludes symlinks.
    ///
    /// [`crate::dirtree::DirNode::is_file`] reports a link's type, not its
    /// target's.
    ///
    /// # Errors
    ///
    /// - [`AmbiguousTemplate`] if multiple files in the search directory share
    ///   the stem.
    /// - [`DirectoryRead`] if reading the search directory fails.
    /// - [`UnsafeComponent`] if a matched file name is not a safe relative
    ///   path.
    /// - [`Absolute`] if a matched file name is absolute.
    ///
    /// [`AmbiguousTemplate`]: TemplatePathError::AmbiguousTemplate
    /// [`DirectoryRead`]: TemplatePathError::DirectoryRead
    /// [`UnsafeComponent`]: TemplatePathError::UnsafeComponent
    /// [`Absolute`]: TemplatePathError::Absolute
    fn find_name_in(
        dir: &Path,
        name: &TemplatePathInput,
    ) -> Result<Option<TemplatePath>, TemplatePathError> {
        let path = name.as_ref();
        if path.extension().is_some() {
            return Ok(None);
        }
        let subdir = path.parent().filter(|p| !p.as_os_str().is_empty());
        let search_dir =
            subdir.map_or_else(|| dir.to_path_buf(), |parent| dir.join(parent));
        let key = path.file_stem().unwrap_or(path.as_os_str());
        let mut hits = Vec::new();
        let entries = DirTree::children(&search_dir)
            .missing_as_empty()
            .sorted_by(|a, b| a.file_name().cmp(b.file_name()));
        for entry in entries {
            let node = match entry {
                Ok(node) => node,
                Err(error) => {
                    let (directory, source) = error.into_parts();
                    return Err(TemplatePathError::DirectoryRead {
                        directory,
                        source,
                    });
                }
            };
            let file_name = node.file_name();
            if node.is_file() && Path::new(file_name).file_stem() == Some(key) {
                hits.push(subdir.map_or_else(
                    || PathBuf::from(file_name),
                    |parent| parent.join(file_name),
                ));
            }
        }
        match hits.as_slice() {
            [] => Ok(None),
            [hit] => Ok(Some(TemplatePath::verified(
                TemplatePathInput::parse(hit)?,
                dir.to_path_buf(),
            ))),
            _ => Err(TemplatePathError::AmbiguousTemplate {
                name: path.to_path_buf(),
                candidates: hits,
            }),
        }
    }

    /// Resolves `name` with [`Self::find`] and reads it.
    ///
    /// [`TemplateEngine::new`] connects this method to minijinja's `{% include
    /// %}` and `{% extends %}` loader callback.
    ///
    /// A missing include returns `Ok(None)`, so minijinja can honor `ignore
    /// missing`. Invalid names, ambiguous matches, and inaccessible
    /// template directories return errors.
    ///
    /// # Errors
    ///
    /// - [`minijinja::Error`] if resolving `name` fails for anything except
    ///   absence, or if the resolved file cannot be read.
    ///
    /// [`TemplateEngine::new`]: super::engine::TemplateEngine::new
    pub(super) fn load(&self, name: &str) -> Result<Option<String>, Error> {
        let found = match TemplatePathInput::parse(Path::new(name))
            .and_then(|input| self.find(&input))
        {
            Ok(found) => found,
            Err(TemplatePathError::TemplateNotFound(_)) => return Ok(None),
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InvalidOperation,
                    "could not resolve template",
                )
                .with_source(error));
            }
        };
        match found.read() {
            Ok(source) => Ok(Some(source)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(Error::new(
                ErrorKind::InvalidOperation,
                "could not read template",
            )
            .with_source(err)),
        }
    }

    /// Lists available template names.
    ///
    /// Returns the stem of each top-level `.md` file in the local directory,
    /// then each new stem from the global directory. The method supplies
    /// names for `--list`, the interactive picker, and `traces completions
    /// --list-templates`. It does not scan subdirectories and uses the
    /// local-before-global order of [`Self::find`].
    ///
    /// The method skips missing or unreadable directories. This reduces the
    /// candidate list and returns no error because the method has no
    /// `Result` value.
    #[must_use]
    pub(super) fn list_available(&self) -> Vec<String> {
        let mut names = Self::stems_in(self.local.as_deref());
        for stem in Self::stems_in(self.global.as_deref()) {
            if !names.contains(&stem) {
                names.push(stem);
            }
        }
        names
    }

    /// Collects the stems of top-level `.md` files directly inside `dir`.
    ///
    /// The method excludes symlinks, like the
    /// [`crate::dirtree::DirNode::is_file`] check in
    /// [`Self::find_name_in`]. It returns an empty list if `dir` is `None`,
    /// does not exist, or cannot be read. Listing failures reduce the
    /// candidate list. The method has no `Result` value for these failures.
    /// It does not scan subdirectories.
    fn stems_in(dir: Option<&Path>) -> Vec<String> {
        let Some(dir) = dir else {
            return Vec::new();
        };
        DirTree::children(dir)
            .walk()
            .filter_map(|entry| {
                let Ok(node) = entry else {
                    return None;
                };
                if !node.is_file() {
                    return None;
                }
                let path = Path::new(node.file_name());
                let is_markdown =
                    path.extension().is_some_and(|ext| ext == "md");
                is_markdown
                    .then(|| {
                        path.file_stem()
                            .map(|stem| stem.to_string_lossy().into_owned())
                    })
                    .flatten()
            })
            .collect()
    }
}

impl From<&Config> for TemplateLoader {
    fn from(config: &Config) -> Self {
        Self::new(
            config.local_template_dir().map(Path::to_path_buf),
            config.global_template_dir().map(Path::to_path_buf),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let parent = path.parent().expect("template path parent");
        fs::create_dir_all(parent).expect("create template parent");
        fs::write(&path, "content").expect("write template");
        path
    }

    fn input(path: &str) -> TemplatePathInput {
        TemplatePathInput::parse(Path::new(path)).expect("valid template input")
    }

    mod find {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn delegates_to_the_validated_candidates_own_find() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let file = write_file(temp.path(), "daily.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn matches_a_literal_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let file = write_file(temp.path(), "daily.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let found =
                loader.find(&input("daily.md")).expect("find exact match");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn rejects_an_absolute_path_even_when_the_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            // A file that exists on disk, outside any template directory.
            let outside_file = write_file(temp.path(), "secret.md");
            let error = TemplatePathInput::parse(&outside_file)
                .expect_err("absolute path is rejected before loader search");

            assert!(matches!(error, TemplatePathError::Absolute(_)));
        }

        #[test]
        fn rejects_parent_traversal_even_when_the_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "secret.md");
            let error = TemplatePathInput::parse(Path::new("../secret.md"))
                .expect_err(
                    "parent traversal is rejected before loader search",
                );

            assert!(matches!(error, TemplatePathError::UnsafeComponent(_)));
        }

        #[test]
        fn rejects_an_empty_name() {
            let error = TemplatePathInput::parse(Path::new(""))
                .expect_err("empty input is rejected before loader search");

            assert!(matches!(error, TemplatePathError::UnsafeComponent(_)));
        }

        #[test]
        fn searches_the_candidates_own_subdirectory_for_a_stem_match() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "daily.txt");
            let file = write_file(temp.path(), "notes/daily.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let found =
                loader.find(&input("notes/daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn misses_when_the_named_subdirectory_does_not_exist() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            assert!(matches!(
                loader.find(&input("notes/daily")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }

        #[test]
        fn skips_stem_matching_when_the_candidate_has_an_extension() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "daily.txt");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            assert!(matches!(
                loader.find(&input("daily.md")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }

        #[test]
        fn rejects_an_ambiguous_stem_match() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("daily.md"), "content")
                .expect("write template");
            fs::write(temp.path().join("daily.txt"), "content")
                .expect("write template");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let error =
                loader.find(&input("daily")).expect_err("ambiguous stem match");
            assert!(
                matches!(
                    &error,
                    TemplatePathError::AmbiguousTemplate { name, candidates }
                        if name.as_path() == Path::new("daily")
                            && *candidates == vec![
                                PathBuf::from("daily.md"),
                                PathBuf::from("daily.txt"),
                            ]
                ),
                "expected ambiguous template error for daily.{{md,txt}}, got \
                 {error:?}"
            );
        }

        #[test]
        fn ignores_directories_when_stem_matching() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::create_dir(temp.path().join("daily")).expect("create dir");
            let file = write_file(temp.path(), "daily.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn prefers_local_over_global_even_via_stem_match() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            let local_file = write_file(&local_dir, "daily.md");
            write_file(&global_dir, "daily");
            let loader = TemplateLoader::new(Some(local_dir), Some(global_dir));

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), local_file);
        }

        #[test]
        fn falls_through_to_global_when_local_has_no_match() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            fs::create_dir_all(&local_dir).expect("create local dir");
            let file = write_file(&global_dir, "daily.md");
            let loader = TemplateLoader::new(Some(local_dir), Some(global_dir));

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn matches_an_exact_name_in_the_global_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            fs::create_dir_all(&local_dir).expect("create local dir");
            let file = write_file(&global_dir, "daily.md");
            let loader = TemplateLoader::new(Some(local_dir), Some(global_dir));

            let found = loader.find(&input("daily.md")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn returns_not_found_when_no_match_exists_anywhere() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            fs::create_dir_all(&local_dir).expect("create local dir");
            fs::create_dir_all(&global_dir).expect("create global dir");
            let loader = TemplateLoader::new(Some(local_dir), Some(global_dir));

            assert!(matches!(
                loader.find(&input("missing")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }

        #[test]
        fn dedups_directories_when_local_and_global_are_identical() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let loader = TemplateLoader::new(
                Some(temp.path().to_path_buf()),
                Some(temp.path().to_path_buf()),
            );

            assert_eq!(loader.directories().count(), 1);
        }
        #[test]
        fn still_works_when_local_and_global_are_identical() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let dir = temp.path().join("templates");
            fs::create_dir_all(&dir).expect("create templates dir");
            let loader = TemplateLoader::new(Some(dir.clone()), Some(dir));

            assert!(matches!(
                loader.find(&input("missing")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }

        #[cfg(unix)]
        #[test]
        fn exact_match_does_not_follow_a_symlink() {
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().expect("create temp dir");
            let outside_file = write_file(temp.path(), "outside/secret.md");
            let local_dir = temp.path().join("templates");
            fs::create_dir_all(&local_dir).expect("create local templates");
            symlink(&outside_file, local_dir.join("daily.md"))
                .expect("create symlink");
            let loader = TemplateLoader::new(Some(local_dir), None);

            assert!(matches!(
                loader.find(&input("daily.md")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }

        #[cfg(unix)]
        #[test]
        fn stem_match_does_not_follow_a_symlink() {
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().expect("create temp dir");
            let outside_file = write_file(temp.path(), "outside/secret.md");
            let local_dir = temp.path().join("templates");
            fs::create_dir_all(&local_dir).expect("create local templates");
            symlink(&outside_file, local_dir.join("daily"))
                .expect("create symlink");
            let loader = TemplateLoader::new(Some(local_dir), None);

            assert!(matches!(
                loader.find(&input("daily")),
                Err(TemplatePathError::TemplateNotFound(_))
            ));
        }
    }

    mod list_available {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_empty_when_no_directories_are_configured() {
            let loader = TemplateLoader::new(None, None);

            assert_eq!(loader.list_available(), Vec::<String>::new());
        }

        #[test]
        fn returns_stems_from_a_single_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "daily.md");
            write_file(temp.path(), "weekly.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let mut names = loader.list_available();
            names.sort();

            assert_eq!(names, vec!["daily".to_owned(), "weekly".to_owned()]);
        }

        #[test]
        fn lists_local_before_global_and_filters_local_duplicates() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            write_file(&local_dir, "daily.md");
            write_file(&global_dir, "daily.md");
            write_file(&global_dir, "weekly.md");
            let loader = TemplateLoader::new(Some(local_dir), Some(global_dir));

            assert_eq!(loader.list_available(), vec![
                "daily".to_owned(),
                "weekly".to_owned()
            ]);
        }

        #[test]
        fn skips_a_missing_directory_silently() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("does-not-exist");
            let loader = TemplateLoader::new(Some(missing), None);

            assert_eq!(loader.list_available(), Vec::<String>::new());
        }

        #[test]
        fn only_yields_top_level_md_files_not_subdirectories() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "daily.md");
            write_file(temp.path(), "nested/weekly.md");
            fs::write(temp.path().join("notes.txt"), "content")
                .expect("write non-md file");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            assert_eq!(loader.list_available(), vec!["daily".to_owned()]);
        }

        #[cfg(unix)]
        #[test]
        fn excludes_a_symlinked_entry() {
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().expect("create temp dir");
            let outside_file = write_file(temp.path(), "outside/secret.md");
            let local_dir = temp.path().join("templates");
            fs::create_dir_all(&local_dir).expect("create local templates");
            symlink(&outside_file, local_dir.join("linked.md"))
                .expect("create symlink");
            let loader = TemplateLoader::new(Some(local_dir), None);

            assert_eq!(loader.list_available(), Vec::<String>::new());
        }
    }

    mod load {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn resolves_a_dot_prefixed_include_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join(".draft.md"), "secret")
                .expect("write template");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let source = loader.load(".draft.md").expect("load succeeds");

            assert_eq!(source, Some("secret".to_owned()));
        }

        #[test]
        fn returns_none_for_a_missing_include() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let source = loader.load("missing.md").expect("load succeeds");

            assert_eq!(source, None);
        }

        #[test]
        fn rejects_an_unsafe_include_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let error = loader
                .load("../outside.md")
                .expect_err("unsafe include must be rejected");

            assert_eq!(
                error.to_string(),
                "invalid operation: could not resolve template"
            );
        }

        #[test]
        fn falls_back_to_a_stem_match() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_file(temp.path(), "daily.md");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            let source = loader.load("daily").expect("load succeeds");

            assert_eq!(source, Some("content".to_owned()));
        }

        #[test]
        fn returns_none_when_read_fails_with_not_found() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let loader =
                TemplateLoader::new(Some(temp.path().to_path_buf()), None);

            // The name has no extension, so `find_name_in` checks the directory
            // and finds no match. `find` returns
            // `TemplateNotFound`, then `load` returns `None`.
            let source = loader.load("missing").expect("load succeeds");

            assert_eq!(source, None);
        }
    }

    mod conversions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn finds_a_local_template() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local-templates");
            let file = write_file(&local_dir, "daily.md");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let loader = TemplateLoader::from(&config);

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn finds_a_global_template_when_local_is_absent() {
            // A project with no local template directory must still resolve
            // templates from the global directory. An empty local
            // directory is a different case.
            let temp = tempfile::tempdir().expect("create temp dir");
            let global_dir = temp.path().join("global-templates");
            let file = write_file(&global_dir, "daily.md");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                None,
                Some(global_dir),
                temp.path().to_path_buf(),
            );
            let loader = TemplateLoader::from(&config);

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), file);
        }

        #[test]
        fn prefers_local_over_global() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local-templates");
            let local_file = write_file(&local_dir, "daily");
            let global_dir = temp.path().join("global-templates");
            write_file(&global_dir, "daily");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                Some(global_dir),
                temp.path().to_path_buf(),
            );
            let loader = TemplateLoader::from(&config);

            let found = loader.find(&input("daily")).expect("find succeeds");

            assert_eq!(found.absolute(), local_file);
        }
    }
}
