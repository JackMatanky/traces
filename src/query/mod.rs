//! Evaluates declarative queries over indexed Markdown notes and flat list
//! items.
//!
//! The query subsystem reads notes and list items from a [`WorkspaceIndex`].
//! It filters, sorts, limits, groups, flattens, and formats query rows.
//!
//! # Architecture and Pipeline
//!
//! 1. **Build**: Create a [`QueryBuilder`] with a [`SourceSelector`] and row
//!    type.
//! 2. **Select**: [`QueryService::run`] matches notes and can expand File
//!    Classes through a [`FileClassExpander`].
//! 3. **Create rows**: Matching notes produce [`QueryRow`] values. List and
//!    task queries produce one positional row for each matching list item.
//! 4. **Transform**: The planner combines filters and sorts. It can also
//!    replace a sort followed by a limit with a bounded top-k selection.
//! 5. **Read**: A [`QuerySet`] runs pending transforms on the first read. It
//!    caches rows for later reads and formatting.
//!
//! # Query Expressions
//!
//! - **Source selectors (`--from`)** match tags, paths, globs, and File
//!   Classes. Combine terms with `and`, `or`, `not`, and parentheses.
//! - **Filter expressions (`--where`)** compare row fields or call
//!   `contains(...)`.
//! - **Fields** include `file.<field>`, `list.<field>`, tags, inbound links,
//!   frontmatter, and inline metadata.
//!
//! # Key Types
//!
//! - [`QueryBuilder`]: Declarative builder for query configuration.
//! - [`QueryService`]: Executes query plans against an index.
//! - [`QuerySet`]: Holds query rows and pending transforms.
//! - [`QueryRow`]: Represents one indexed file or list item.
//! - [`SourceSelector`]: Selects source documents.
//! - [`QueryError`]: Reports query construction, parsing, and rendering errors.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "test-utils")]
//! # {
//! use std::sync::Arc;
//!
//! use traces::testing::{
//!     IndexerService, QueryBuilder, QueryService, SourceSelector,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let temp = tempfile::tempdir()?;
//! std::fs::write(temp.path().join("a.md"), "---\nrating: 5\n---\n")?;
//!
//! let index = Arc::new(IndexerService::for_tests(temp.path()).build()?);
//! let service = QueryService::new("class");
//! let builder =
//!     QueryBuilder::pages(SourceSelector::All).filter("rating >= 5")?;
//!
//! let set = service.run(&index, builder);
//! assert_eq!(set.len(), 1);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! [`WorkspaceIndex`]: crate::index::WorkspaceIndex
//! [`FileClassExpander`]: crate::query::grammar::FileClassExpander
mod builder;
mod error;
mod format;
mod grammar;
mod plan;
mod results;
mod service;
mod sort;
mod value;

pub use builder::QueryBuilder;
pub(crate) use builder::QueryMode;
#[cfg(test)]
pub(crate) use error::{FieldPathError, QuerySyntaxError};
pub use error::{QueryBuilderError, QueryDialect, QueryError, QueryResult};
pub(crate) use format::TaskPathStyle;
#[cfg(any(test, feature = "test-utils"))]
pub(crate) use grammar::FilterExpr;
pub use grammar::SourceSelector;
pub(crate) use grammar::{
    ClassExpansionMode, FieldPath, FileClassExpander, FileField, ListField,
    SourceAtom, SourceExpr,
};
use plan::{ExecutionPlan, QueryTransform};
pub use results::{QueryRow, QuerySet};
pub use service::QueryService;
pub(crate) use sort::{SortDirection, SortOrder};

#[cfg(test)]
pub(super) mod test_support {
    use std::path::Path;

    use super::*;
    /// Builds an in-memory index over `files` and returns a query for all
    /// notes.
    pub(super) fn rows_for_files(files: &[(&str, &str)]) -> QuerySet {
        let index = crate::test_support::build_test_index(files);
        QueryService::new("class")
            .run(&index, QueryBuilder::pages(SourceSelector::All))
    }

    /// Builds an all-notes page query over one in-memory Markdown note.
    pub(super) fn rows_for(content: &str) -> QuerySet {
        rows_for_files(&[("note.md", content)])
    }

    pub(super) fn find_entry<'a>(
        entries: &'a [crate::index::FileEntry],
        path: &Path,
    ) -> &'a crate::index::FileEntry {
        entries
            .iter()
            .find(|e| e.file().path() == path)
            .expect("entry not found")
    }

    pub(super) fn find_base<'a>(
        entries: &'a [crate::index::FileEntry],
        path: &Path,
    ) -> &'a crate::file::FileMeta {
        find_entry(entries, path).file()
    }
}
