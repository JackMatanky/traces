//! Holds Template Directory settings and their config digests.

use std::path::{Path, PathBuf};

use super::digest::setting;
use crate::{Blake3Digest, Blake3Hasher};

/// Names the default local Template Directory.
pub(crate) const DEFAULT_LOCAL_TEMPLATES_DIR: &str = ".traces/templates";

/// Holds local and global Template Directories and the output directory.
///
/// Keeps local and global directories separate for local-first lookup.
#[derive(Clone, Debug)]
pub(in crate::config) struct TemplateConfig {
    local: Option<PathBuf>,
    global: Option<PathBuf>,
    output: PathBuf,
}

impl TemplateConfig {
    /// Assembles resolved template directories and the output directory.
    #[inline]
    #[must_use]
    pub(in crate::config) const fn new(
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
}

const TEMPLATES_LOCAL: &[u8] = b"templates.local";
const TEMPLATES_GLOBAL: &[u8] = b"templates.global";
const TEMPLATES_OUTPUT: &[u8] = b"templates.output";

/// Holds the digests of [`TemplateConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TemplateDigests {
    local: Blake3Digest,
    global: Blake3Digest,
    output: Blake3Digest,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for index cache invalidation")
)]
impl TemplateDigests {
    /// Returns the digest of the local Template Directory setting.
    #[inline]
    #[must_use]
    pub(crate) const fn local(&self) -> Blake3Digest {
        self.local
    }

    /// Returns the digest of the global Template Directory setting.
    #[inline]
    #[must_use]
    pub(crate) const fn global(&self) -> Blake3Digest {
        self.global
    }

    /// Returns the digest of the output directory setting.
    #[inline]
    #[must_use]
    pub(crate) const fn output(&self) -> Blake3Digest {
        self.output
    }
}

impl TemplateDigests {
    /// Feeds every setting digest in declaration order.
    pub(super) fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            local,
            global,
            output,
        } = self;
        fp.digest(local);
        fp.digest(global);
        fp.digest(output);
    }
}

impl TemplateConfig {
    /// Digests the Template Directories and the output directory.
    pub(super) fn digests(&self) -> TemplateDigests {
        let Self {
            local,
            global,
            output,
        } = self;
        TemplateDigests {
            local: setting(TEMPLATES_LOCAL, |fp| {
                fp.optional_path(local.as_deref());
            }),
            global: setting(TEMPLATES_GLOBAL, |fp| {
                fp.optional_path(global.as_deref());
            }),
            output: setting(TEMPLATES_OUTPUT, |fp| fp.path(output)),
        }
    }
}

#[cfg(test)]
mod tests {
    mod digests {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::super::*;

        fn base() -> TemplateConfig {
            TemplateConfig::new(
                Some(PathBuf::from("/project/local")),
                Some(PathBuf::from("/global")),
                PathBuf::from("/project/out"),
            )
        }

        fn ordered(digests: &TemplateDigests) -> [Blake3Digest; 3] {
            [digests.local(), digests.global(), digests.output()]
        }

        #[test]
        fn frames_a_present_local_directory_with_its_presence_tag() {
            let mut raw = blake3::Hasher::new();
            raw.update(&15_u64.to_le_bytes());
            raw.update(b"templates.local");
            raw.update(&[1]);
            raw.update(&14_u64.to_le_bytes());
            raw.update(b"/project/local");

            let digests = base().digests();

            assert_eq!(
                digests.local().to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[rstest]
        #[case::local(
            TemplateConfig::new(
                None,
                Some(PathBuf::from("/global")),
                PathBuf::from("/project/out")
            ),
            0
        )]
        #[case::global(
            TemplateConfig::new(
                Some(PathBuf::from("/project/local")),
                None,
                PathBuf::from("/project/out")
            ),
            1
        )]
        #[case::output(
            TemplateConfig::new(
                Some(PathBuf::from("/project/local")),
                Some(PathBuf::from("/global")),
                PathBuf::from("/other")
            ),
            2
        )]
        fn changes_only_the_digest_of_the_changed_setting(
            #[case] changed: TemplateConfig,
            #[case] expected: usize,
        ) {
            let before = ordered(&base().digests());
            let after = ordered(&changed.digests());

            let differing: Vec<usize> = before
                .iter()
                .zip(after.iter())
                .enumerate()
                .filter(|(_, (old, new))| old != new)
                .map(|(index, _)| index)
                .collect();

            assert_eq!(differing, [expected]);
        }
    }
}
