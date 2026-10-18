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
    pub(crate) local: Blake3Digest,
    pub(crate) global: Blake3Digest,
    pub(crate) output: Blake3Digest,
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
