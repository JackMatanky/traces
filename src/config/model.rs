//! Holds resolved configuration values after loading and validation.
//!
//! Consumers read merged values through [`Config`]. Separate local and global
//! template directories preserve lookup precedence.
//!
//! # Key Types
//!
//! - [`Config`]: resolved values and the workspace root.
//! - [`TemplateConfig`]: local and global Template Directories.
//! - [`SchemasConfig`]: schema registry and class field.
//! - [`FrontmatterConfig`]: canonical metadata key names.
//! - [`DateFieldConfig`]: date field name and format.
//! - [`TaskConfig`]: Task Configuration statuses and tag filters.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{
    error::ConfigValueError,
    raw::{
        RawDateFieldConfig, RawFrontmatterConfig, RawTaskConfig,
        RawTaskStatusKind,
    },
};
use crate::{
    Blake3Digest, FieldName, FieldNameError, Tag, TaskStatus, TaskStatusMap,
    TaskStatusSymbol, TaskStatusType,
    hash::Blake3Hasher,
    path::{PathError, RelativePath, SafeRelativePath},
};

/// Names the default local schemas directory.
pub(super) const DEFAULT_LOCAL_SCHEMAS_DIR: &str = ".traces/schemas/";

/// Names the default local Template Directory.
pub(crate) const DEFAULT_LOCAL_TEMPLATES_DIR: &str = ".traces/templates";

/// Names the default schema class field.
pub(super) const DEFAULT_CLASS_FIELD: &str = "class";

/// Names the default title field.
const DEFAULT_TITLE_FIELD: &str = "title";

/// Names the default aliases field.
const DEFAULT_ALIASES_FIELD: &str = "aliases";

/// Names the default tags field.
const DEFAULT_TAGS_FIELD: &str = "tags";

/// Names the default creation date field.
const DEFAULT_DATE_CREATED_FIELD: &str = "date_created";

/// Names the default modification date field.
const DEFAULT_DATE_MODIFIED_FIELD: &str = "date_modified";

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

    /// Computes a deterministic Config Fingerprint over every resolved value.
    ///
    /// Length-prefixes variable-length fields, hashes paths as raw OS bytes,
    /// and tags optional paths for an unambiguous encoding. Hashes task
    /// statuses in symbol order and tag filters in configured order, so
    /// reordering filters changes the [`Blake3Digest`]. Persisted fingerprints
    /// support index cache invalidation across process restarts.
    #[inline]
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "no current caller outside tests; reserved for future \
                      index cache invalidation"
        )
    )]
    pub(crate) fn hash(&self) -> Blake3Digest {
        let Self {
            root,
            templates,
            schemas,
            frontmatter,
            tasks,
        } = self;
        let mut fp = Blake3Hasher::new();
        fp.path(root);
        templates.feed_into(&mut fp);
        schemas.feed_into(&mut fp);
        frontmatter.feed_into(&mut fp);
        tasks.feed_into(&mut fp);
        fp.finish()
    }
}

/// Holds local and global Template Directories and the output directory.
///
/// Keeps local and global directories separate for local-first lookup.
#[derive(Clone, Debug)]
pub(super) struct TemplateConfig {
    local: Option<PathBuf>,
    global: Option<PathBuf>,
    output: PathBuf,
}

impl TemplateConfig {
    /// Assembles resolved template directories and the output directory.
    #[inline]
    #[must_use]
    pub(super) const fn new(
        local: Option<PathBuf>,
        global: Option<PathBuf>,
        output: PathBuf,
    ) -> Self {
        Self {
            local,
            global,
            output,
        }
    }

    /// Returns the local Template Directory, if present.
    #[inline]
    #[must_use]
    pub(super) fn local(&self) -> Option<&Path> {
        self.local.as_deref()
    }

    /// Returns the global Template Directory, if present.
    #[inline]
    #[must_use]
    pub(super) fn global(&self) -> Option<&Path> {
        self.global.as_deref()
    }

    /// Returns the resolved output directory.
    #[inline]
    #[must_use]
    pub(super) fn output(&self) -> &Path {
        &self.output
    }

    /// Feeds every field into the Config Fingerprint.
    ///
    /// Destructuring all fields forces new fields into this encoding.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            local,
            global,
            output,
        } = self;
        fp.optional_path(local.as_deref());
        fp.optional_path(global.as_deref());
        fp.path(output);
    }
}

/// Holds the schema registry directory and class field name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemasConfig {
    directory: PathBuf,
    class_field: FieldName,
}

impl SchemasConfig {
    /// Assembles validated schema settings from resolved values.
    ///
    /// Requires `directory` to have been resolved against its Configuration
    /// Scope root; this constructor performs no further validation.
    #[inline]
    #[must_use]
    pub(super) const fn new(
        directory: PathBuf,
        class_field: FieldName,
    ) -> Self {
        Self {
            directory,
            class_field,
        }
    }

    /// Feeds every field into the Config Fingerprint.
    ///
    /// Destructuring all fields forces new fields into this encoding.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            directory,
            class_field,
        } = self;
        fp.bytes(class_field.as_str().as_bytes());
        fp.path(directory);
    }

    /// Returns the frontmatter key for a note's file class.
    ///
    /// Defaults to `class` when unconfigured.
    #[inline]
    #[must_use]
    pub fn class_field_name(&self) -> &str {
        self.class_field.as_str()
    }

    /// Returns the schema registry directory.
    ///
    /// Config File loading resolves a configured directory against its
    /// Configuration Scope root, or the workspace root when unconfigured.
    /// [`SchemasConfig::default`] retains a relative directory because it has
    /// no workspace root.
    #[inline]
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Builds test schema settings with a validated class field.
    ///
    /// # Panics
    ///
    /// - `class_field` is not a valid field key.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test inputs must validate; Default carries the same \
                  expectation"
    )]
    pub fn for_test(class_field: &str) -> Self {
        Self {
            directory: PathBuf::from(DEFAULT_LOCAL_SCHEMAS_DIR),
            class_field: FieldName::try_from(class_field)
                .expect("test class field validates as a field key"),
        }
    }

    /// Resolves the default schema directory against `root` for tests.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    pub(super) fn default_for_root(root: &Path) -> Self {
        Self {
            directory: root.join(DEFAULT_LOCAL_SCHEMAS_DIR),
            ..Self::default()
        }
    }
}

impl Default for SchemasConfig {
    /// Builds schema defaults from the hardcoded class field.
    ///
    /// # Panics
    ///
    /// - [`DEFAULT_CLASS_FIELD`] stops being a valid field key.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_CLASS_FIELD is a hardcoded constant; failure here \
                  means the constant itself is malformed, not a recoverable \
                  caller error"
    )]
    fn default() -> Self {
        Self {
            directory: PathBuf::from(DEFAULT_LOCAL_SCHEMAS_DIR),
            class_field: FieldName::try_from(DEFAULT_CLASS_FIELD)
                .expect("DEFAULT_CLASS_FIELD is a valid field key"),
        }
    }
}

/// Holds canonical metadata key names and date formats from `[frontmatter]`.
#[derive(Clone, Debug)]
pub struct FrontmatterConfig {
    title: FieldName,
    aliases: FieldName,
    tags: FieldName,
    date_created: DateFieldConfig,
    date_modified: DateFieldConfig,
}

impl FrontmatterConfig {
    /// Returns the frontmatter key for a note's display title.
    #[inline]
    #[must_use]
    pub fn title_name(&self) -> &str {
        self.title.as_str()
    }

    /// Returns the frontmatter key for a note's aliases.
    #[inline]
    #[must_use]
    pub fn aliases_name(&self) -> &str {
        self.aliases.as_str()
    }

    /// Returns the frontmatter key for a note's tags.
    #[inline]
    #[must_use]
    pub fn tags_name(&self) -> &str {
        self.tags.as_str()
    }

    /// Returns the creation date field name and format.
    #[inline]
    #[must_use]
    pub const fn date_created(&self) -> &DateFieldConfig {
        &self.date_created
    }

    /// Returns the modification date field name and format.
    #[inline]
    #[must_use]
    pub const fn date_modified(&self) -> &DateFieldConfig {
        &self.date_modified
    }

    /// Builds test metadata settings with custom title and aliases keys.
    ///
    /// # Panics
    ///
    /// - `title` or `aliases` is empty, whitespace-only, or contains `/`.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test-only constructor; an invalid literal here is a test \
                  fixture bug, not a recoverable caller error"
    )]
    pub fn for_test<T: Into<String>, A: Into<String>>(
        title: T,
        aliases: A,
    ) -> Self {
        Self {
            title: FieldName::try_from(title.into())
                .expect("test fixture title is a valid field key"),
            aliases: FieldName::try_from(aliases.into())
                .expect("test fixture aliases is a valid field key"),
            ..Self::default()
        }
    }

    /// Overrides the tags key in test metadata settings.
    ///
    /// # Panics
    ///
    /// - `tags` is empty, whitespace-only, or contains `/`.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test-only constructor; an invalid literal here is a test \
                  fixture bug, not a recoverable caller error"
    )]
    pub fn with_tags_name<T: Into<String>>(mut self, tags: T) -> Self {
        self.tags = FieldName::try_from(tags.into())
            .expect("test fixture tags is a valid field key");
        self
    }

    /// Feeds every field into the Config Fingerprint.
    ///
    /// Destructuring all fields forces new fields into this encoding.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            title,
            aliases,
            tags,
            date_created,
            date_modified,
        } = self;
        fp.bytes(title.as_str().as_bytes());
        fp.bytes(aliases.as_str().as_bytes());
        fp.bytes(tags.as_str().as_bytes());
        date_created.feed_into(fp);
        date_modified.feed_into(fp);
    }
}

impl Default for FrontmatterConfig {
    /// Builds canonical metadata defaults from hardcoded field names.
    ///
    /// # Panics
    ///
    /// - [`DEFAULT_TITLE_FIELD`], [`DEFAULT_ALIASES_FIELD`],
    ///   [`DEFAULT_TAGS_FIELD`], [`DEFAULT_DATE_CREATED_FIELD`], or
    ///   [`DEFAULT_DATE_MODIFIED_FIELD`] stops being a valid field key.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_TITLE_FIELD/DEFAULT_ALIASES_FIELD/\
                  DEFAULT_TAGS_FIELD are hardcoded constants; failure here \
                  means a constant itself is malformed, not a recoverable \
                  caller error"
    )]
    fn default() -> Self {
        Self {
            title: FieldName::try_from(DEFAULT_TITLE_FIELD)
                .expect("DEFAULT_TITLE_FIELD is a valid field key"),
            aliases: FieldName::try_from(DEFAULT_ALIASES_FIELD)
                .expect("DEFAULT_ALIASES_FIELD is a valid field key"),
            tags: FieldName::try_from(DEFAULT_TAGS_FIELD)
                .expect("DEFAULT_TAGS_FIELD is a valid field key"),
            date_created: DateFieldConfig::default_for(
                DEFAULT_DATE_CREATED_FIELD,
            ),
            date_modified: DateFieldConfig::default_for(
                DEFAULT_DATE_MODIFIED_FIELD,
            ),
        }
    }
}

impl TryFrom<RawFrontmatterConfig> for FrontmatterConfig {
    type Error = ConfigValueError;

    /// Resolves canonical metadata fields from raw Config File values.
    ///
    /// # Errors
    ///
    /// - [`InvalidFieldKey`] if a configured title, aliases, tags, creation
    ///   date, or modification date key is empty or contains `/`.
    ///
    /// [`InvalidFieldKey`]: ConfigValueError::InvalidFieldKey
    #[inline]
    fn try_from(raw: RawFrontmatterConfig) -> Result<Self, Self::Error> {
        let invalid_key =
            |source| ConfigValueError::invalid_field_key("frontmatter", source);
        Ok(Self {
            title: FieldName::try_from(
                raw.title.unwrap_or_else(|| DEFAULT_TITLE_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            aliases: FieldName::try_from(
                raw.aliases.unwrap_or_else(|| DEFAULT_ALIASES_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            tags: FieldName::try_from(
                raw.tags.unwrap_or_else(|| DEFAULT_TAGS_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            date_created: DateFieldConfig::from_raw_or_default(
                raw.date_created,
                DEFAULT_DATE_CREATED_FIELD,
            )
            .map_err(invalid_key)?,
            date_modified: DateFieldConfig::from_raw_or_default(
                raw.date_modified,
                DEFAULT_DATE_MODIFIED_FIELD,
            )
            .map_err(invalid_key)?,
        })
    }
}

/// Holds task status mappings and tag filters from Task Configuration.
#[derive(Clone, Debug)]
pub struct TaskConfig {
    statuses: TaskStatusMap,
    tag_filters: Vec<Tag>,
}

impl TaskConfig {
    /// Returns the task status lookup map used by the parser.
    #[inline]
    #[must_use]
    pub(crate) const fn statuses(&self) -> &TaskStatusMap {
        &self.statuses
    }

    /// Returns the configured task tag filters.
    ///
    /// An empty list accepts every status-marked list item as a task.
    #[inline]
    #[must_use]
    pub fn tag_filters(&self) -> &[Tag] {
        &self.tag_filters
    }

    /// Builds test Task Configuration with custom tag filters.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn for_test(tag_filters: Vec<Tag>) -> Self {
        Self {
            statuses: TaskStatusMap::default(),
            tag_filters,
        }
    }

    /// Parses tag filters from string slices for test Task Configuration.
    ///
    /// # Panics
    ///
    /// - Any tag does not parse as a [`Tag`].
    #[cfg(any(test, feature = "test-utils"))]
    #[expect(
        clippy::expect_used,
        reason = "test-only constructor; an invalid literal here is a test \
                  fixture bug, not a recoverable caller error"
    )]
    #[inline]
    #[must_use]
    pub fn from_tags(tags: &[&str]) -> Self {
        let tag_filters =
            tags.iter().map(|&t| Tag::parse(t).expect("valid tag")).collect();
        Self::for_test(tag_filters)
    }

    /// Feeds statuses and tag filters into the Config Fingerprint.
    ///
    /// Hashes statuses in symbol order and filters in configured order.
    /// Destructuring all fields forces new fields into this encoding.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            statuses,
            tag_filters,
        } = self;
        let mut ordered: Vec<&TaskStatus> = statuses.iter().collect();
        ordered.sort_unstable_by_key(|status| status.symbol().as_char());
        fp.len(ordered.len());
        for status in ordered {
            let mut symbol = [0_u8; 4];
            fp.bytes(
                status.symbol().as_char().encode_utf8(&mut symbol).as_bytes(),
            );
            fp.bytes(status.name().as_bytes());
            fp.bytes(status.kind().as_str().as_bytes());
        }
        fp.len(tag_filters.len());
        for tag in tag_filters {
            fp.bytes(tag.as_str().as_bytes());
        }
    }
}

impl Default for TaskConfig {
    #[inline]
    fn default() -> Self {
        Self {
            statuses: TaskStatusMap::default(),
            tag_filters: Vec::new(),
        }
    }
}

impl TryFrom<RawTaskConfig> for TaskConfig {
    type Error = ConfigValueError;

    /// Resolves task statuses and tag filters from raw Config File values.
    ///
    /// # Errors
    ///
    /// - `ConfigValueError::Task` if a status symbol is `(`, `)`, `[`, `]`,
    ///   `{`, or `}`.
    /// - `ConfigValueError::InvalidTagFilter` if a tag filter cannot form a
    ///   valid [`Tag`].
    #[inline]
    fn try_from(raw: RawTaskConfig) -> Result<Self, Self::Error> {
        let mut statuses = TaskStatusMap::default();
        for status in raw.statuses {
            statuses.insert(TaskStatus::new(
                TaskStatusSymbol::new(status.symbol),
                status.name,
                status.kind.into(),
            ))?;
        }
        let tag_filters = raw
            .tag_filters
            .into_iter()
            .map(|entry| {
                Tag::parse_lenient(&entry).map_err(|source| {
                    ConfigValueError::InvalidTagFilter {
                        entry,
                        source,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            statuses,
            tag_filters,
        })
    }
}

impl From<RawTaskStatusKind> for TaskStatusType {
    #[inline]
    fn from(kind: RawTaskStatusKind) -> Self {
        match kind {
            RawTaskStatusKind::Todo => Self::Todo,
            RawTaskStatusKind::InProgress => Self::InProgress,
            RawTaskStatusKind::OnHold => Self::OnHold,
            RawTaskStatusKind::Done => Self::Done,
            RawTaskStatusKind::Cancelled => Self::Cancelled,
            RawTaskStatusKind::NonTask => Self::NonTask,
        }
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

/// Holds a frontmatter date key and its format string.
#[derive(Clone, Debug)]
pub struct DateFieldConfig {
    name: FieldName,
    format: String,
}

impl DateFieldConfig {
    /// Returns the frontmatter date key name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the date format used for this field's value.
    #[inline]
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Builds a date field from a hardcoded role name and default format.
    ///
    /// # Panics
    ///
    /// - `name` is not a valid field key; callers pass only
    ///   [`DEFAULT_DATE_CREATED_FIELD`] or [`DEFAULT_DATE_MODIFIED_FIELD`].
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "callers only pass hardcoded role-name constants; failure \
                  here means a constant itself is malformed, not a \
                  recoverable caller error"
    )]
    fn default_for(name: &str) -> Self {
        Self {
            name: FieldName::try_from(name)
                .expect("role-name constant is a valid field key"),
            format: crate::DATETIME_PATTERN.to_owned(),
        }
    }

    /// Resolves a date field from raw values or role-specific defaults.
    ///
    /// # Errors
    ///
    /// - [`Empty`] if the configured name is empty or whitespace-only.
    /// - [`ContainsSlash`] if the configured name contains `/`.
    ///
    /// [`Empty`]: FieldNameError::Empty
    /// [`ContainsSlash`]: FieldNameError::ContainsSlash
    ///
    /// # Panics
    ///
    /// - `default_name` is invalid when `raw` is absent.
    #[inline]
    fn from_raw_or_default(
        raw: Option<RawDateFieldConfig>,
        default_name: &str,
    ) -> Result<Self, FieldNameError> {
        Ok(match raw {
            None => Self::default_for(default_name),
            Some(raw) => Self {
                name: FieldName::try_from(
                    raw.name.unwrap_or_else(|| default_name.to_owned()),
                )?,
                format: raw
                    .format
                    .unwrap_or_else(|| crate::DATETIME_PATTERN.to_owned()),
            },
        })
    }

    /// Feeds both fields into the Config Fingerprint.
    ///
    /// Destructuring both fields forces new fields into this encoding.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            name,
            format,
        } = self;
        fp.bytes(name.as_str().as_bytes());
        fp.bytes(format.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskError;

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
        use pretty_assertions::{assert_eq, assert_ne};
        use rstest::rstest;

        use super::*;

        fn field_name(name: &str) -> FieldName {
            FieldName::try_from(name).expect("valid field name")
        }

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
        fn matches_the_pinned_digest_for_a_fixed_config() {
            // Persisted Config Fingerprints must survive releases; this pins
            // the encoding and field order.
            let digest = base().hash().to_hex();

            assert_eq!(
                digest.as_ref(),
                "ce5503dd2df8687312cdd13cdff34d5490a8aacc1615d5f8765ee39f3ce6887b"
            );
        }

        #[rstest]
        #[case::root(|c: &mut Config| c.root = Arc::from(Path::new("/other")))]
        #[case::local_templates(|c: &mut Config| c.templates.local = None)]
        #[case::global_templates(|c: &mut Config| c.templates.global = None)]
        #[case::output(|c: &mut Config| c.templates.output = PathBuf::from("/o"))]
        #[case::class_field(|c: &mut Config| c.schemas.class_field = field_name("kind"))]
        #[case::schemas_directory(|c: &mut Config| c.schemas.directory = PathBuf::from("/s"))]
        #[case::title(|c: &mut Config| c.frontmatter.title = field_name("heading"))]
        #[case::aliases(|c: &mut Config| c.frontmatter.aliases = field_name("akas"))]
        #[case::tags(|c: &mut Config| c.frontmatter.tags = field_name("labels"))]
        #[case::created_name(|c: &mut Config| c.frontmatter.date_created.name = field_name("made"))]
        #[case::created_format(|c: &mut Config| c.frontmatter.date_created.format = "%Y".to_owned())]
        #[case::modified_name(|c: &mut Config| c.frontmatter.date_modified.name = field_name("edited"))]
        #[case::modified_format(|c: &mut Config| c.frontmatter.date_modified.format = "%Y".to_owned())]
        #[case::tag_filters(|c: &mut Config| c.tasks = TaskConfig::from_tags(&["#work"]))]
        #[case::added_status(|c: &mut Config| c.tasks.statuses.insert(TaskStatus::new(
            TaskStatusSymbol::new('!'),
            "Urgent",
            TaskStatusType::Todo,
        )).expect("insert status"))]
        #[case::overridden_status_kind(|c: &mut Config| c.tasks.statuses.insert(TaskStatus::new(
            TaskStatusSymbol::new(' '),
            "Todo",
            TaskStatusType::Done,
        )).expect("insert status"))]
        fn changes_when_a_resolved_field_changes(
            #[case] mutate: fn(&mut Config),
        ) {
            let original = base();
            let mut changed = original.clone();

            mutate(&mut changed);

            assert_ne!(original.hash(), changed.hash());
        }
    }

    mod frontmatter_for_test {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn sets_the_expected_title_and_aliases() {
            let config = FrontmatterConfig::for_test("heading", "also_known");

            assert_eq!(config.title_name(), "heading");
            assert_eq!(config.aliases_name(), "also_known");
        }

        #[test]
        fn defaults_tags_to_tags() {
            let config = FrontmatterConfig::for_test("heading", "also_known");

            assert_eq!(config.tags_name(), "tags");
        }
    }
    mod schemas_for_test {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn sets_the_expected_class_field() {
            let config = SchemasConfig::for_test("kind");

            assert_eq!(config.class_field_name(), "kind");
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

    mod task_config {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::config::raw::RawTaskStatus;

        #[test]
        fn defaults_to_empty_tag_filters_and_default_statuses() {
            let config = TaskConfig::default();

            assert_eq!(config.tag_filters(), []);
            assert!(config.statuses().by_symbol(' '.into()).is_some());
        }

        #[test]
        fn normalizes_entries_with_and_without_a_leading_hash() {
            let raw = RawTaskConfig {
                tag_filters: vec!["task".to_owned(), "#todo".to_owned()],
                statuses: Vec::new(),
            };

            let config = TaskConfig::try_from(raw).expect("valid tag filters");

            assert_eq!(config.tag_filters(), [
                Tag::parse("#task").expect("valid tag"),
                Tag::parse("#todo").expect("valid tag"),
            ]);
        }

        #[test]
        fn allows_an_empty_tag_filters_list() {
            let raw = RawTaskConfig {
                tag_filters: Vec::new(),
                statuses: Vec::new(),
            };

            let config = TaskConfig::try_from(raw).expect("empty is valid");

            assert_eq!(config.tag_filters(), []);
        }

        #[test]
        fn adds_a_custom_status_with_its_configured_kind() {
            let raw = RawTaskConfig {
                statuses: vec![RawTaskStatus {
                    symbol: '?',
                    name: "Blocked".to_owned(),
                    kind: RawTaskStatusKind::OnHold,
                }],
                ..RawTaskConfig::default()
            };

            let config = TaskConfig::try_from(raw).expect("valid status");

            let status =
                config.statuses().by_symbol('?'.into()).expect("custom status");
            assert_eq!(status.name(), "Blocked");
            assert_eq!(status.kind(), TaskStatusType::OnHold);
        }

        #[test]
        fn overriding_a_symbol_removes_the_replaced_name() {
            let raw = RawTaskConfig {
                statuses: vec![RawTaskStatus {
                    symbol: '!',
                    name: "Waiting".to_owned(),
                    kind: RawTaskStatusKind::Todo,
                }],
                ..RawTaskConfig::default()
            };

            let config = TaskConfig::try_from(raw).expect("valid status");

            let status = config
                .statuses()
                .by_symbol('!'.into())
                .expect("overridden status");
            assert_eq!(status.name(), "Waiting");
            assert_eq!(status.kind(), TaskStatusType::Todo);
            assert!(config.statuses().by_name("On Hold").is_none());
        }

        #[test]
        fn rejects_an_invalid_tag_filter_entry() {
            let raw = RawTaskConfig {
                tag_filters: vec!["1invalid".to_owned()],
                statuses: Vec::new(),
            };

            let error = TaskConfig::try_from(raw).expect_err("invalid entry");

            assert!(matches!(error, ConfigValueError::InvalidTagFilter {
                entry,
                ..
            } if entry == "1invalid"));
        }

        #[test]
        fn rejects_a_whitespace_only_tag_filter_entry() {
            let raw = RawTaskConfig {
                tag_filters: vec!["   ".to_owned()],
                statuses: Vec::new(),
            };

            let error = TaskConfig::try_from(raw).expect_err("blank entry");

            assert!(matches!(error, ConfigValueError::InvalidTagFilter { .. }));
        }

        #[test]
        fn rejects_a_prohibited_status_symbol() {
            let raw = RawTaskConfig {
                statuses: vec![RawTaskStatus {
                    symbol: '[',
                    name: "Square".to_owned(),
                    kind: RawTaskStatusKind::Todo,
                }],
                ..RawTaskConfig::default()
            };

            let error =
                TaskConfig::try_from(raw).expect_err("prohibited delimiter");

            assert!(matches!(
                error,
                ConfigValueError::Task(TaskError::ProhibitedStatusSymbol {
                    symbol: '['
                })
            ));
        }
    }
}
