//! Holds resolved configuration values after loading and validation.
//!
//! Consumers read merged values through [`Config`]. Each settings section lives
//! in its own submodule beside its config digests, and `Config` stays here.
//! Separate local and global template directories preserve lookup precedence.
//!
//! # Key Types
//!
//! - [`Config`]: resolved values and the workspace root.
//! - [`TemplateConfig`]: local and global Template Directories.
//! - [`SchemasConfig`]: schema registry and class field.
//! - [`FrontmatterConfig`]: canonical metadata key names and date fields.
//! - [`TaskConfig`]: Task Configuration statuses and tag filters.
mod digest;
mod frontmatter;
mod schemas;
mod tasks;
mod templates;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) use digest::ConfigDigests;
use digest::setting;
pub use frontmatter::FrontmatterConfig;
pub(crate) use frontmatter::{DateFieldDigests, FrontmatterDigests};
pub(crate) use schemas::SchemaDigests;
pub use schemas::SchemasConfig;
pub(super) use schemas::{DEFAULT_CLASS_FIELD, DEFAULT_LOCAL_SCHEMAS_DIR};
pub use tasks::TaskConfig;
pub(crate) use tasks::TaskDigests;
pub(crate) use templates::DEFAULT_LOCAL_TEMPLATES_DIR;
pub(super) use templates::TemplateConfig;

use super::error::ConfigValueError;
use crate::{
    Blake3Digest,
    path::{PathError, RelativePath, SafeRelativePath},
};

/// Labels the workspace root setting in the configuration fingerprint.
const ROOT: &[u8] = b"root";

/// Holds resolved Config File values after the discovery, read, trust and value
/// stages.
#[derive(Clone, Debug)]
pub struct Config {
    root: Arc<Path>,
    templates: TemplateConfig,
    schemas: SchemasConfig,
    frontmatter: FrontmatterConfig,
    tasks: TaskConfig,
}

impl Config {
    /// Assembles resolved values from builder-owned parts.
    #[inline]
    #[must_use]
    pub(super) fn new(
        templates: TemplateConfig,
        schemas: SchemasConfig,
        frontmatter: FrontmatterConfig,
        tasks: TaskConfig,
        root: PathBuf,
    ) -> Self {
        Self {
            root: Arc::from(root.into_boxed_path()),
            templates,
            schemas,
            frontmatter,
            tasks,
        }
    }

    /// Returns the workspace root used to resolve relative local paths.
    #[inline]
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the local Template Directory, if configured.
    #[inline]
    #[must_use]
    pub fn local_template_dir(&self) -> Option<&Path> {
        self.templates.local()
    }

    /// Returns the global Template Directory, if configured.
    #[inline]
    #[must_use]
    pub fn global_template_dir(&self) -> Option<&Path> {
        self.templates.global()
    }

    /// Returns the resolved output directory, defaulting to [`Self::root`].
    ///
    /// Resolves relative configured paths against the workspace root and
    /// preserves absolute paths.
    #[inline]
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        self.templates.output()
    }

    /// Returns the resolved `SchemasConfig`.
    #[inline]
    #[must_use]
    pub const fn schemas(&self) -> &SchemasConfig {
        &self.schemas
    }

    /// Returns the resolved `FrontmatterConfig`.
    #[inline]
    #[must_use]
    pub const fn frontmatter(&self) -> &FrontmatterConfig {
        &self.frontmatter
    }

    /// Returns the resolved [`TaskConfig`].
    #[inline]
    #[must_use]
    pub const fn tasks(&self) -> &TaskConfig {
        &self.tasks
    }

    /// Shares the workspace root with consumers that cannot borrow [`Config`].
    #[inline]
    #[must_use]
    pub(crate) fn root_arc(&self) -> Arc<Path> {
        Arc::clone(&self.root)
    }

    /// Shares the schema class field with consumers that cannot borrow
    /// [`Config`].
    #[inline]
    #[must_use]
    pub(crate) fn class_field_arc(&self) -> Arc<str> {
        Arc::from(self.schemas().class_field_name())
    }

    /// Computes the Config Fingerprint: every setting digest of
    /// [`Config::digests`] composed under `b"traces.config.v1"`.
    ///
    /// The index does not persist this digest; it stores a subset of the
    /// per-setting digests instead. The encoding is pinned by tests; see the
    /// `digest` module for the framing.
    #[inline]
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "no caller outside tests; the index fingerprints \
                      per-setting digests instead"
        )
    )]
    pub(crate) fn hash(&self) -> Blake3Digest {
        self.digests().compose()
    }

    /// Digests every resolved setting separately.
    ///
    /// Each digest starts with its constant, section-qualified label, so a
    /// stored digest names the setting it covers. [`ConfigDigests::compose`]
    /// folds them into the Config Fingerprint.
    #[inline]
    #[must_use]
    pub(crate) fn digests(&self) -> ConfigDigests {
        let Self {
            root,
            templates,
            schemas,
            frontmatter,
            tasks,
        } = self;
        ConfigDigests::new(
            setting(ROOT, |fp| fp.path(root)),
            templates.digests(),
            schemas.digests(),
            frontmatter.digests(),
            tasks.digests(),
        )
    }

    /// Builds a resolved configuration without Config File discovery for tests.
    ///
    /// Use [`super::service::ConfigService::at`] with TOML fixtures to test the
    /// real loading pipeline.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn for_test(
        root: PathBuf,
        local: Option<PathBuf>,
        global: Option<PathBuf>,
        output: PathBuf,
    ) -> Self {
        let root: Arc<Path> = Arc::from(root.into_boxed_path());
        let schemas = SchemasConfig::default_for_root(&root);
        Self {
            root,
            templates: TemplateConfig::new(local, global, output),
            schemas,
            frontmatter: FrontmatterConfig::default(),
            tasks: TaskConfig::default(),
        }
    }

    /// Builds a test configuration with default paths under `root`.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn test_default<P: Into<PathBuf>>(root: P) -> Self {
        let root = root.into();
        let root: Arc<Path> = Arc::from(root.into_boxed_path());
        let templates = TemplateConfig::new(None, None, root.to_path_buf());
        let schemas = SchemasConfig::default_for_root(&root);
        Self {
            root,
            templates,
            schemas,
            frontmatter: FrontmatterConfig::default(),
            tasks: TaskConfig::default(),
        }
    }

    /// Sets the local Template Directory to `root/templates` for tests.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn with_templates(mut self) -> Self {
        self.templates = TemplateConfig::new(
            Some(self.root.join("templates")),
            None,
            self.root.to_path_buf(),
        );
        self
    }

    /// Overrides canonical metadata key names in a test configuration.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn with_frontmatter(mut self, frontmatter: FrontmatterConfig) -> Self {
        self.frontmatter = frontmatter;
        self
    }

    /// Overrides Task Configuration in a test configuration.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn with_tasks(mut self, tasks: TaskConfig) -> Self {
        self.tasks = tasks;
        self
    }

    /// Overrides schema settings in a test configuration.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn with_schemas(mut self, schemas: SchemasConfig) -> Self {
        self.schemas = schemas;
        self
    }
}

/// Holds a nonempty, lexically safe workspace-relative subdirectory.
///
/// Rejects absolute paths, parent components, and paths with no named component
/// before any filesystem checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigSubDir(RelativePath);

impl ConfigSubDir {
    /// Returns the subdirectory as a relative [`Path`].
    #[inline]
    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.0.as_ref()
    }

    /// Resolves this subdirectory within `root`.
    ///
    /// # Errors
    ///
    /// - [`InvalidSubDir`] if the path escapes `root` or filesystem
    ///   verification fails.
    ///
    /// [`InvalidSubDir`]: ConfigValueError::InvalidSubDir
    pub(crate) fn resolve_against(
        &self,
        root: &Path,
    ) -> Result<PathBuf, ConfigValueError> {
        SafeRelativePath::parse(root, self.as_path())
            .map(SafeRelativePath::into_path_buf)
            .map_err(|source| ConfigValueError::InvalidSubDir {
                path: self.as_path().to_path_buf(),
                source,
            })
    }

    /// Validates `dir` and resolves it within `root`.
    ///
    /// # Errors
    ///
    /// - [`InvalidSubDir`] if `dir` is lexically unsafe, escapes `root`, or
    ///   fails filesystem verification.
    ///
    /// [`InvalidSubDir`]: ConfigValueError::InvalidSubDir
    pub(crate) fn resolve(
        dir: &Path,
        root: &Path,
    ) -> Result<PathBuf, ConfigValueError> {
        Self::try_from(dir.to_path_buf())
            .map_err(|source| ConfigValueError::InvalidSubDir {
                path: dir.to_path_buf(),
                source,
            })?
            .resolve_against(root)
    }

    /// Parses a configured subdirectory or its relative default.
    ///
    /// # Errors
    ///
    /// - [`Absolute`] if the selected path is absolute.
    /// - [`UnsafeComponent`] if it has `..` or no named component.
    ///
    /// [`Absolute`]: PathError::Absolute
    /// [`UnsafeComponent`]: PathError::UnsafeComponent
    pub(super) fn from_raw_or_default(
        raw: Option<PathBuf>,
        default_rel: &'static str,
    ) -> Result<Self, PathError> {
        match raw {
            Some(path) => Self::try_from(path),
            None => Self::try_from(default_rel),
        }
    }
}

impl TryFrom<PathBuf> for ConfigSubDir {
    type Error = PathError;

    /// Parses an owned, lexically safe relative subdirectory.
    ///
    /// # Errors
    ///
    /// - [`Absolute`] if `path` is absolute.
    /// - [`UnsafeComponent`] if `path` has `..` or no named component.
    ///
    /// [`Absolute`]: PathError::Absolute
    /// [`UnsafeComponent`]: PathError::UnsafeComponent
    #[inline]
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        RelativePath::parse(&path).map(Self)
    }
}

impl TryFrom<&str> for ConfigSubDir {
    type Error = PathError;

    /// Parses a borrowed, lexically safe relative subdirectory.
    ///
    /// # Errors
    ///
    /// - [`Absolute`] if `s` is absolute.
    /// - [`UnsafeComponent`] if `s` has `..` or no named component.
    ///
    /// [`Absolute`]: PathError::Absolute
    /// [`UnsafeComponent`]: PathError::UnsafeComponent
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        RelativePath::parse(Path::new(s)).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod root_arc {
        use super::*;

        #[test]
        fn repeated_calls_share_the_cached_allocation() {
            let config = Config::test_default(PathBuf::from("/vault"));
            let first = config.root_arc();
            let second = config.root_arc();

            assert!(Arc::ptr_eq(&first, &second));
        }
    }

    mod hash {
        use pretty_assertions::assert_eq;

        use super::*;

        fn base() -> Config {
            Config::for_test(
                PathBuf::from("/project"),
                Some(PathBuf::from("/project/local")),
                Some(PathBuf::from("/global")),
                PathBuf::from("/project/out"),
            )
        }

        #[test]
        fn is_equal_for_separately_built_equal_configs() {
            assert_eq!(base().hash(), base().hash());
        }

        #[test]
        fn frames_the_root_digest_with_its_setting_label() {
            let config = base();
            let root = config.root();
            let mut raw = blake3::Hasher::new();
            raw.update(
                &u64::try_from(ROOT.len())
                    .expect("root label length fits in u64")
                    .to_le_bytes(),
            );
            raw.update(ROOT);
            let path = root.as_os_str().as_encoded_bytes();
            raw.update(
                &u64::try_from(path.len())
                    .expect("root path length fits in u64")
                    .to_le_bytes(),
            );
            raw.update(path);

            assert_eq!(
                config.digests().root.to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[test]
        fn matches_the_pinned_digest_for_a_fixed_config() {
            // Persisted Config Fingerprints must survive releases; this pins
            // the per-setting encoding, labels, and compose order. Re-pinned
            // once when the aggregate chain became compose-all.
            let digest = base().hash().to_hex();

            assert_eq!(
                digest.as_ref(),
                "eab509cd36bf89013b65124fb1f456889f859d84b600ec6ef7ecb38f1852e571"
            );
        }
    }

    mod config_sub_dir {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn accepts_safe_relative_path() {
            let subdir = ConfigSubDir::try_from("schemas/dir")
                .expect("valid safe relative path");
            assert_eq!(subdir.as_path(), Path::new("schemas/dir"));
        }

        #[test]
        fn rejects_absolute_and_parent_paths() {
            assert!(ConfigSubDir::try_from("/absolute/path").is_err());
            assert!(ConfigSubDir::try_from("../parent").is_err());
            assert!(ConfigSubDir::try_from("dir/../../escaped").is_err());
        }

        #[test]
        fn resolve_against_joins_subdir_onto_root() {
            let temp = tempfile::tempdir().expect("create temp directory");
            let root = temp.path().join("vault");
            let schemas = root.join(".traces/schemas");
            std::fs::create_dir_all(&schemas).expect("create schema directory");
            let subdir = ConfigSubDir::try_from(DEFAULT_LOCAL_SCHEMAS_DIR)
                .expect("valid safe relative path");

            assert_eq!(
                subdir.resolve_against(&root).expect("resolves"),
                schemas
            );
        }

        #[cfg(unix)]
        #[test]
        fn resolve_against_rejects_subdir_that_escapes_root() {
            let temp = tempfile::tempdir().expect("create temp directory");
            let root = temp.path().join("root");
            let external = temp.path().join("external");
            std::fs::create_dir_all(&root).expect("create root directory");
            std::fs::create_dir_all(&external)
                .expect("create external directory");
            std::os::unix::fs::symlink(&external, root.join(".traces"))
                .expect("create escaping schema symlink");
            let subdir = ConfigSubDir::try_from(DEFAULT_LOCAL_SCHEMAS_DIR)
                .expect("valid safe relative path");

            let error = subdir
                .resolve_against(&root)
                .expect_err("schema directory escapes root through symlink");

            assert!(matches!(error, ConfigValueError::InvalidSubDir {
                source: crate::path::PathError::OutsideRoot,
                ..
            }));
        }
    }
}
