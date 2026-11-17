//! Validates and labels template paths.
//!
//! # Key Types
//!
//! - [`TemplatePathInput`]: Validated relative template identifier.
//! - [`TemplatePath`]: Template path that the loader has resolved to a file.
//! - [`DeclaredOutputPath`]: Raw `file.write_to()` path captured during
//!   rendering.
//! - [`TemplatePathError`]: Validation and lookup errors for template
//!   identifiers.
//!
//! [`TemplateLoader`] creates [`TemplatePath`] after it finds a file. This
//! module does not validate [`DeclaredOutputPath`]. The [`writer`] module
//! resolves and confines it with `-o` output paths.
//!
//! [`TemplateLoader`]: super::loader::TemplateLoader
//! [`writer`]: super::writer

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::path::{PathError, RelativePath};

/// The default extension for each rendered note unless `-o` or
/// `file.write_to()` sets another path.
const DEFAULT_EXTENSION: &str = "md";

/// Holds a validated template identifier before the loader resolves it.
///
/// The identifier is relative and contains at least one normal path component.
/// It cannot escape through parent traversal, but it does not prove that the
/// template file exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplatePathInput(RelativePath);

impl TemplatePathInput {
    /// Parses `path` as a template identifier.
    ///
    /// Accepts relative names such as `daily`, `daily.md`, and
    /// `folder/daily.md`. Rejects absolute paths, parent traversal, empty
    /// paths, and paths without a normal component.
    ///
    /// # Errors
    ///
    /// - [`TemplatePathError::Absolute`] if `path` is absolute.
    /// - [`TemplatePathError::UnsafeComponent`] if `path` contains `..`, an
    ///   unsupported component, or no [`Component::Normal`].
    ///
    /// [`Component::Normal`]: std::path::Component::Normal
    /// [`TemplatePathError::Absolute`]: TemplatePathError::Absolute
    /// [`TemplatePathError::UnsafeComponent`]: TemplatePathError::UnsafeComponent
    ///
    /// ```rust
    /// # #[cfg(feature = "test-utils")]
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::path::Path;
    ///
    /// use traces::testing::TemplatePathInput;
    ///
    /// let input = TemplatePathInput::parse(Path::new("daily.md"))?;
    /// assert_eq!(input.as_ref(), Path::new("daily.md"));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "test-utils"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn parse(path: &Path) -> Result<Self, TemplatePathError> {
        RelativePath::parse(path).map(Self).map_err(|error| match error {
            PathError::Absolute => {
                TemplatePathError::Absolute(path.to_path_buf())
            }
            PathError::UnsafeComponent
            | PathError::OutsideRoot
            | PathError::StrictPath(_) => {
                TemplatePathError::UnsafeComponent(path.to_path_buf())
            }
        })
    }
}

impl AsRef<Path> for TemplatePathInput {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

/// A validated template path resolved against a template directory, proven to
/// exist on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TemplatePath {
    input: TemplatePathInput,
    source_dir: PathBuf,
}

impl TemplatePath {
    /// Creates a [`TemplatePath`] after [`TemplateLoader::find_path_in`] or
    /// [`TemplateLoader::find_name_in`] confirms that the file exists. Later
    /// pipeline stages do not check that the file still exists.
    ///
    /// [`TemplateLoader`]: super::loader::TemplateLoader
    /// [`TemplateLoader::find_path_in`]: super::loader::TemplateLoader::find_path_in
    /// [`TemplateLoader::find_name_in`]: super::loader::TemplateLoader::find_name_in
    #[inline]
    #[must_use]
    pub(super) const fn verified(
        input: TemplatePathInput,
        source_dir: PathBuf,
    ) -> Self {
        Self {
            input,
            source_dir,
        }
    }

    /// Creates a test fixture without the existence guarantee of
    /// [`Self::verified`]. Production code uses [`TemplateLoader::find`].
    ///
    /// [`TemplateLoader::find`]: super::loader::TemplateLoader::find
    #[cfg(test)]
    #[must_use]
    pub(super) const fn for_test(
        input: TemplatePathInput,
        source_dir: PathBuf,
    ) -> Self {
        Self {
            input,
            source_dir,
        }
    }

    /// Returns the absolute path formed by joining `source_dir` and `input`.
    #[inline]
    #[must_use]
    pub(super) fn absolute(&self) -> PathBuf {
        self.source_dir.join(self.input.as_ref())
    }

    /// Returns the default output filename with the `md` extension and
    /// preserves its directory segments.
    #[inline]
    #[must_use]
    pub(super) fn default_output_filename(&self) -> PathBuf {
        self.input.as_ref().with_extension(DEFAULT_EXTENSION)
    }

    /// Reads this resolved template's source from disk and returns it as a
    /// string.
    ///
    /// # Errors
    ///
    /// - [`io::Error`] if the resolved template file cannot be read.
    pub(super) fn read(&self) -> io::Result<String> {
        fs::read_to_string(self.absolute())
    }
}

impl AsRef<Path> for TemplatePath {
    fn as_ref(&self) -> &Path {
        self.input.as_ref()
    }
}

/// Holds the raw `file.write_to()` path that a template declares during
/// rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeclaredOutputPath(PathBuf);

impl DeclaredOutputPath {
    #[inline]
    #[must_use]
    pub(super) const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[inline]
    #[must_use]
    pub(super) fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Reports validation and lookup errors for template identifiers.
///
/// Validation errors occur before the loader searches a template directory.
///
/// Lookup errors occur after validation succeeds.
#[derive(Debug, Error)]
pub enum TemplatePathError {
    /// The identifier is absolute.
    ///
    /// Template lookup accepts only paths relative to configured template
    /// directories.
    #[error("template path {0} must be relative, not absolute")]
    Absolute(PathBuf),
    /// The identifier is not a safe relative path.
    ///
    /// This error covers parent traversal, unsupported components, an empty
    /// path, and `.` without a normal component.
    #[error("template path {0} is not a valid template identifier")]
    UnsafeComponent(PathBuf),
    /// Stem lookup matched more than one file in a template directory.
    #[error("template name \"{name}\" matched multiple files: {candidates:?}")]
    AmbiguousTemplate {
        /// Requested identifier that produced multiple matches.
        name: PathBuf,
        /// Matching paths relative to the searched template directory.
        candidates: Vec<PathBuf>,
    },
    /// Reading a template directory failed during stem lookup.
    #[error("failed to read template directory {directory}")]
    DirectoryRead {
        /// Directory that could not be read.
        directory: PathBuf,
        /// Filesystem error returned while reading the directory.
        #[source]
        source: io::Error,
    },
    /// No configured template directory contains the requested template.
    #[error("template \"{0}\" not found")]
    TemplateNotFound(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validated(name: &str) -> TemplatePath {
        let name =
            TemplatePathInput::parse(Path::new(name)).expect("valid candidate");
        TemplatePath::for_test(name, PathBuf::from("/dir"))
    }

    fn write_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let parent = path.parent().expect("template path parent");
        fs::create_dir_all(parent).expect("create template parent");
        fs::write(&path, "content").expect("write template");
        path
    }

    mod validation {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn accepts_a_plain_relative_name() {
            let path = validated("daily.md");

            assert_eq!(path.as_ref(), Path::new("daily.md"));
        }

        #[test]
        fn accepts_a_nested_relative_path() {
            let path = validated("folder/daily.md");

            assert_eq!(path.as_ref(), Path::new("folder/daily.md"));
        }

        #[test]
        fn accepts_a_path_with_a_leading_current_dir_segment() {
            // "./daily.md" splits into [CurDir, Normal("daily.md")]. A CurDir
            // segment at the start does not count as a normal component, but it
            // does not make the path invalid. The trailing Normal component
            // makes the path valid. A bare "." has no Normal component.
            let path = validated("./daily.md");

            assert_eq!(path.as_ref(), Path::new("./daily.md"));
        }

        #[test]
        fn rejects_an_absolute_path() {
            // The parser rejects an absolute path before I/O because it never
            // reads the filesystem. This check does not access any file at this
            // path.
            let error = TemplatePathInput::parse(&std::env::temp_dir())
                .expect_err("absolute path is rejected");

            assert!(matches!(error, TemplatePathError::Absolute(_)));
        }

        #[rstest]
        #[case::parent_traversal("../outside.md")]
        #[case::nested_parent_traversal("folder/../../outside.md")]
        #[case::empty_path("")]
        #[case::bare_current_dir(".")]
        fn rejects_unsafe_components(#[case] input: &str) {
            let error = TemplatePathInput::parse(Path::new(input))
                .expect_err("unsafe component is rejected");

            assert!(matches!(error, TemplatePathError::UnsafeComponent(_)));
        }
    }

    mod default_output_filename {
        use pretty_assertions::assert_eq;

        use super::*;

        fn found(dir: &Path, name: &str) -> TemplatePath {
            let path = write_file(dir, name);
            let rel = path.strip_prefix(dir).expect("relative path");
            TemplatePath::for_test(
                TemplatePathInput::parse(rel).expect("relative path is safe"),
                dir.to_path_buf(),
            )
        }

        #[test]
        fn keeps_the_directory_and_the_default_extension() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let resolved = found(temp.path(), "folder/report.md");

            assert_eq!(
                resolved.default_output_filename(),
                Path::new("folder/report.md")
            );
        }

        #[test]
        fn forces_the_default_extension_over_the_source_files_own() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let resolved = found(temp.path(), "daily.txt");

            assert_eq!(
                resolved.default_output_filename(),
                Path::new("daily.md")
            );
        }
    }

    mod read {
        use pretty_assertions::assert_eq;

        use super::*;

        fn found(dir: &Path, name: &str) -> TemplatePath {
            let path = write_file(dir, name);
            let rel = path.strip_prefix(dir).expect("relative path");
            TemplatePath::for_test(
                TemplatePathInput::parse(rel).expect("relative path is safe"),
                dir.to_path_buf(),
            )
        }

        #[test]
        fn reads_the_resolved_templates_content() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let resolved = found(temp.path(), "daily.md");
            fs::write(resolved.absolute(), "hello world")
                .expect("overwrite fixture content");

            assert_eq!(resolved.read().expect("read succeeds"), "hello world");
        }

        #[test]
        fn propagates_the_io_error_when_the_file_is_removed_after_resolution() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let resolved = found(temp.path(), "daily.md");
            fs::remove_file(resolved.absolute()).expect("remove fixture");

            let error =
                resolved.read().expect_err("removed file fails to read");

            assert_eq!(error.kind(), io::ErrorKind::NotFound);
        }
    }
}
