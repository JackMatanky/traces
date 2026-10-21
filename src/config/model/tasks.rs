//! Holds Task Configuration and its config digests.

use super::digest::setting;
use crate::{
    Blake3Digest, Blake3Hasher, Tag, TaskStatus, TaskStatusMap,
    TaskStatusSymbol, TaskStatusType,
    config::{
        error::ConfigValueError,
        raw::{RawTaskConfig, RawTaskStatusKind},
    },
};

/// Holds task status mappings and tag filters from Task Configuration.
#[derive(Clone, Debug)]
pub struct TaskConfig {
    statuses: TaskStatusMap,
    tag_filters: Vec<Tag>,
}

impl TaskConfig {
    /// Returns the configured task tag filters.
    ///
    /// An empty list accepts every status-marked list item as a task.
    #[inline]
    #[must_use]
    pub fn tag_filters(&self) -> &[Tag] {
        &self.tag_filters
    }

    /// Returns the task status lookup map used by the parser.
    #[inline]
    #[must_use]
    pub(crate) const fn statuses(&self) -> &TaskStatusMap {
        &self.statuses
    }

    /// Digests the task statuses and tag filters.
    ///
    /// Statuses enter in symbol order so map iteration order never leaks;
    /// filters enter in configured order.
    pub(super) fn digests(&self) -> TaskDigests {
        let Self {
            statuses,
            tag_filters,
        } = self;
        TaskDigests {
            statuses: setting(TASKS_STATUSES, |fp| {
                let mut ordered: Vec<&TaskStatus> = statuses.iter().collect();
                ordered
                    .sort_unstable_by_key(|status| status.symbol().as_char());
                fp.len(ordered.len());
                for status in ordered {
                    let mut symbol = [0_u8; 4];
                    fp.bytes(
                        status
                            .symbol()
                            .as_char()
                            .encode_utf8(&mut symbol)
                            .as_bytes(),
                    );
                    fp.bytes(status.name().as_bytes());
                    fp.bytes(status.kind().as_str().as_bytes());
                }
            }),
            tag_filters: setting(TASKS_TAG_FILTERS, |fp| {
                fp.len(tag_filters.len());
                for tag in tag_filters {
                    fp.bytes(tag.as_str().as_bytes());
                }
            }),
        }
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
    /// - [`ConfigValueError::Task`] if a status symbol is `(`, `)`, `[`, `]`,
    ///   `{`, or `}`.
    /// - [`ConfigValueError::InvalidTagFilter`] if a tag filter cannot form a
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

const TASKS_STATUSES: &[u8] = b"tasks.statuses";
const TASKS_TAG_FILTERS: &[u8] = b"tasks.tag_filters";

/// Holds the digests of [`TaskConfig`] settings.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskDigests {
    pub(crate) statuses: Blake3Digest,
    pub(crate) tag_filters: Blake3Digest,
}

impl TaskDigests {
    /// Feeds every setting digest in declaration order.
    pub(super) fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            statuses,
            tag_filters,
        } = self;
        fp.digest(statuses);
        fp.digest(tag_filters);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskError;

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

    mod digests {
        use pretty_assertions::{assert_eq, assert_ne};
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::added_status(TaskStatus::new(
            TaskStatusSymbol::new('!'),
            "Urgent",
            TaskStatusType::Todo,
        ))]
        #[case::overridden_status_kind(TaskStatus::new(
            TaskStatusSymbol::new(' '),
            "Todo",
            TaskStatusType::Done,
        ))]
        fn changes_only_the_statuses_digest_when_a_status_changes(
            #[case] status: TaskStatus,
        ) {
            let before = TaskConfig::default().digests();
            let mut changed = TaskConfig::default();
            changed.statuses.insert(status).expect("insert status");
            let after = changed.digests();

            assert_ne!(before.statuses, after.statuses);
            assert_eq!(before.tag_filters, after.tag_filters);
        }

        #[test]
        fn changes_only_the_tag_filters_digest_when_a_filter_is_added() {
            let before = TaskConfig::default().digests();
            let after = TaskConfig::from_tags(&["#work"]).digests();

            assert_eq!(before.statuses, after.statuses);
            assert_ne!(before.tag_filters, after.tag_filters);
        }
    }
}
