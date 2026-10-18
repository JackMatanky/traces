//! Encodes resolved settings as per-section digests.
//!
//! Every setting digests on its own under a section-qualified label, so a
//! stored digest says which setting it covers and cannot collide with a
//! sibling. [`Config::hash`] composes all of them under one version label.
//!
//! # Exhaustive destructuring
//!
//! Each `digests` constructor, each bundle's `feed_into`, and
//! [`ConfigDigests::compose`] destructure their type without `..`. Adding a
//! setting to a config type, or a bundle to [`ConfigDigests`], then fails to
//! compile here until it gets a label and a place in the encoding. A
//! source-scanning test pins the convention because the compiler cannot tell a
//! rest pattern from an exhaustive one.
//!
//! # Key Types
//!
//! - [`ConfigDigests`]: one digest per setting, grouped by section.

use super::{
    Config, frontmatter::FrontmatterDigests, schemas::SchemaDigests,
    tasks::TaskDigests, templates::TemplateDigests,
};
use crate::{Blake3Digest, Blake3Hasher};

/// Versions the composed Config Fingerprint encoding.
const CONFIG_LABEL: &[u8] = b"traces.config.v1";

const ROOT: &[u8] = b"root";

/// Holds one framed digest per resolved setting, grouped by config section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ConfigDigests {
    root: Blake3Digest,
    templates: TemplateDigests,
    schemas: SchemaDigests,
    frontmatter: FrontmatterDigests,
    tasks: TaskDigests,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for index cache invalidation")
)]
impl ConfigDigests {
    /// Returns the digest of the workspace root setting.
    #[inline]
    #[must_use]
    pub(crate) const fn root(&self) -> Blake3Digest {
        self.root
    }

    /// Returns the Template Directory digests.
    #[inline]
    #[must_use]
    pub(crate) const fn templates(&self) -> TemplateDigests {
        self.templates
    }

    /// Returns the schema settings digests.
    #[inline]
    #[must_use]
    pub(crate) const fn schemas(&self) -> SchemaDigests {
        self.schemas
    }

    /// Returns the frontmatter settings digests.
    #[inline]
    #[must_use]
    pub(crate) const fn frontmatter(&self) -> FrontmatterDigests {
        self.frontmatter
    }

    /// Returns the Task Configuration digests.
    #[inline]
    #[must_use]
    pub(crate) const fn tasks(&self) -> TaskDigests {
        self.tasks
    }
}

/// Digests one setting: the label's length-prefixed bytes, then the value
/// bytes `feed` adds.
///
/// The label frame lives here rather than in [`Blake3Hasher`], whose value
/// methods stay label-agnostic.
pub(super) fn setting<F>(label: &[u8], feed: F) -> Blake3Digest
where
    F: FnOnce(&mut Blake3Hasher),
{
    let mut fp = Blake3Hasher::new();
    fp.bytes(label);
    feed(&mut fp);
    fp.finish()
}

impl ConfigDigests {
    /// Composes every setting digest into one digest under
    /// `b"traces.config.v1"`.
    ///
    /// Settings enter in declaration order, so the result is the Config
    /// Fingerprint. Changing the order, the labels, or the version label
    /// invalidates every persisted fingerprint.
    pub(crate) fn compose(&self) -> Blake3Digest {
        let Self {
            root,
            templates,
            schemas,
            frontmatter,
            tasks,
        } = self;
        let mut fp = Blake3Hasher::new();
        fp.bytes(CONFIG_LABEL);
        fp.digest(root);
        templates.feed_into(&mut fp);
        schemas.feed_into(&mut fp);
        frontmatter.feed_into(&mut fp);
        tasks.feed_into(&mut fp);
        fp.finish()
    }
}

impl Config {
    /// Digests every resolved setting separately.
    ///
    /// Each digest starts with its section-qualified label, so a stored
    /// digest names the setting it covers. [`ConfigDigests::compose`] folds
    /// them into the Config Fingerprint.
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
        ConfigDigests {
            root: setting(ROOT, |fp| fp.path(root)),
            templates: templates.digests(),
            schemas: schemas.digests(),
            frontmatter: frontmatter.digests(),
            tasks: tasks.digests(),
        }
    }
}

#[cfg(test)]
mod tests {
    mod fixtures {
        use std::path::PathBuf;

        use super::super::*;

        pub(in super::super) const NAMES: [&str; 15] = [
            "root",
            "templates.local",
            "templates.global",
            "templates.output",
            "schemas.directory",
            "schemas.class_field",
            "frontmatter.title",
            "frontmatter.aliases",
            "frontmatter.tags",
            "frontmatter.date_created.name",
            "frontmatter.date_created.format",
            "frontmatter.date_modified.name",
            "frontmatter.date_modified.format",
            "tasks.statuses",
            "tasks.tag_filters",
        ];

        pub(in super::super) fn base() -> Config {
            Config::for_test(
                PathBuf::from("/project"),
                Some(PathBuf::from("/project/local")),
                Some(PathBuf::from("/global")),
                PathBuf::from("/project/out"),
            )
        }

        /// Lists every setting digest in [`NAMES`] order.
        pub(in super::super) fn ordered(
            digests: &ConfigDigests,
        ) -> [Blake3Digest; 15] {
            let templates = digests.templates();
            let schemas = digests.schemas();
            let frontmatter = digests.frontmatter();
            let tasks = digests.tasks();
            [
                digests.root(),
                templates.local(),
                templates.global(),
                templates.output(),
                schemas.directory(),
                schemas.class_field(),
                frontmatter.title(),
                frontmatter.aliases(),
                frontmatter.tags(),
                frontmatter.date_created().name(),
                frontmatter.date_created().format(),
                frontmatter.date_modified().name(),
                frontmatter.date_modified().format(),
                tasks.statuses(),
                tasks.tag_filters(),
            ]
        }
    }

    mod setting {
        use pretty_assertions::assert_eq;

        use super::super::*;

        #[test]
        fn hashes_the_label_frame_before_the_value_frame() {
            let mut raw = blake3::Hasher::new();
            raw.update(&13_u64.to_le_bytes());
            raw.update(b"test.my_label");
            raw.update(&5_u64.to_le_bytes());
            raw.update(b"value");

            let digest = setting(b"test.my_label", |fp| fp.bytes(b"value"));

            assert_eq!(
                digest.to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }
    }

    mod config_digests {
        use std::{
            path::{Path, PathBuf},
            sync::Arc,
        };

        use pretty_assertions::{assert_eq, assert_ne};

        use super::{
            super::*,
            fixtures::{NAMES, base, ordered},
        };
        use crate::{
            FieldName,
            config::model::{FrontmatterConfig, SchemasConfig},
        };

        #[test]
        fn composes_every_setting_digest_under_the_version_label() {
            let digests = base().digests();
            let mut raw = blake3::Hasher::new();
            raw.update(&16_u64.to_le_bytes());
            raw.update(b"traces.config.v1");
            for digest in ordered(&digests) {
                let hash = blake3::Hash::from_hex(digest.to_hex().as_ref())
                    .expect("digest hex parses");
                raw.update(&32_u64.to_le_bytes());
                raw.update(hash.as_bytes());
            }

            assert_eq!(
                digests.compose().to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[test]
        fn keeps_settings_with_equal_values_apart_by_label() {
            let same = || PathBuf::from("same");
            let mut config =
                Config::for_test(same(), Some(same()), Some(same()), same());
            config.schemas = SchemasConfig::new(
                same(),
                FieldName::try_from("same").expect("valid field name"),
            );
            config.frontmatter = FrontmatterConfig::for_test("same", "same")
                .with_tags_name("same");

            let named: Vec<_> =
                NAMES.into_iter().zip(ordered(&config.digests())).collect();

            for (i, (first_name, first)) in named.iter().enumerate() {
                for (second_name, second) in named.iter().skip(i + 1) {
                    assert_ne!(
                        first, second,
                        "{second_name} collides with {first_name}"
                    );
                }
            }
        }

        #[test]
        fn changes_only_the_root_digest_when_the_root_changes() {
            let before = ordered(&base().digests());
            let mut changed = base();
            changed.root = Arc::from(Path::new("/other"));
            let after = ordered(&changed.digests());

            let differing: Vec<&str> = NAMES
                .iter()
                .zip(before.iter().zip(after.iter()))
                .filter(|(_, (old, new))| old != new)
                .map(|(name, _)| *name)
                .collect();

            assert_eq!(differing, ["root"]);
        }
    }

    mod exhaustive_destructuring {
        use pretty_assertions::assert_eq;
        use rstest::rstest;
        use syn::{
            ImplItemFn, ItemImpl, PatRest, Type,
            visit::{self, Visit},
        };

        /// Collects rest patterns and `digests` constructors from a source
        /// file.
        #[derive(Default)]
        struct Scan {
            self_type: String,
            current_fn: String,
            rest_patterns: Vec<String>,
            constructors: Vec<String>,
        }

        impl<'ast> Visit<'ast> for Scan {
            fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
                if let Type::Path(path) = &*node.self_ty {
                    self.self_type = path
                        .path
                        .segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                        .unwrap_or_default();
                }
                visit::visit_item_impl(self, node);
            }

            fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
                self.current_fn = node.sig.ident.to_string();
                if self.current_fn == "digests" {
                    self.constructors.push(self.self_type.clone());
                }
                visit::visit_impl_item_fn(self, node);
                self.current_fn.clear();
            }

            fn visit_pat_rest(&mut self, _: &'ast PatRest) {
                // Other code (error matching in tests, `Raw*` parsing) may use
                // rest patterns freely; only the digest encoders must not.
                if matches!(
                    self.current_fn.as_str(),
                    "digests" | "feed_into" | "compose"
                ) {
                    self.rest_patterns.push(format!(
                        "{}::{}",
                        self.self_type, self.current_fn
                    ));
                }
            }
        }

        fn scan(source: &str) -> Scan {
            let file = syn::parse_file(source).expect("source parses");
            let mut scan = Scan::default();
            scan.visit_file(&file);
            scan
        }

        #[rstest]
        #[case::constructor("digests")]
        #[case::bundle_encoder("feed_into")]
        #[case::composer("compose")]
        fn flags_a_rest_pattern_in_a_digest_encoder(#[case] name: &str) {
            let source = format!(
                "impl Section {{ fn {name}(&self) {{ let Self {{ a, .. }} = \
                 self; }} }}"
            );

            assert_eq!(scan(&source).rest_patterns, [format!(
                "Section::{name}"
            )]);
        }

        /// Every file that defines a digest constructor or a bundle encoder.
        const SOURCES: [&str; 5] = [
            include_str!("digest.rs"),
            include_str!("frontmatter.rs"),
            include_str!("schemas.rs"),
            include_str!("tasks.rs"),
            include_str!("templates.rs"),
        ];

        #[test]
        fn finds_no_rest_pattern_in_any_digest_encoder() {
            let rest_patterns: Vec<String> = SOURCES
                .iter()
                .flat_map(|source| scan(source).rest_patterns)
                .collect();

            assert_eq!(rest_patterns, Vec::<String>::new());
        }

        #[test]
        fn finds_a_digests_constructor_on_every_section_type() {
            let mut constructors: Vec<String> = SOURCES
                .iter()
                .flat_map(|source| scan(source).constructors)
                .collect();
            constructors.sort_unstable();

            assert_eq!(constructors, [
                "Config",
                "DateFieldConfig",
                "FrontmatterConfig",
                "SchemasConfig",
                "TaskConfig",
                "TemplateConfig",
            ]);
        }
    }
}
