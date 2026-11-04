//! Formats and renders query result collections into Markdown.
//!
//! The formatter converts [`QueryRow`] items into Markdown tables, bullet
//! lists, and task lists. [`TaskPathStyle`] adds file paths to task output.
use std::borrow::Cow;

use unicode_width::UnicodeWidthStr as _;

use super::{QueryError, QueryResult, grammar::FieldPath, results::QueryRow};

/// Controls file-path rendering in task list output.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum TaskPathStyle {
    /// Omits paths for template rendering.
    #[default]
    None,
    /// Adds each row's path to `traces task` output.
    /// Adds source line numbers in `traces task -l` output.
    Suffix,
    /// Adds file paths and 1-indexed source line numbers for `traces task -l`.
    Coordinates,
}

/// Represents Markdown display formats supported by query results.
pub(super) enum QueryDisplayFormat {
    /// Renders a Markdown table with display headers and field-path columns.
    Table {
        headers: Vec<String>,
        columns: Vec<String>,
    },
    /// Renders a Markdown bullet list from one field path.
    List {
        field: String,
    },
    /// Renders a Markdown task list. It can add the path for each row.
    TaskList {
        path_style: TaskPathStyle,
    },
}

impl QueryDisplayFormat {
    #[must_use]
    pub(super) fn table(headers: &[&str], columns: &[&str]) -> Self {
        Self::Table {
            headers: headers
                .iter()
                .map(|header| (*header).to_owned())
                .collect(),
            columns: columns
                .iter()
                .map(|column| (*column).to_owned())
                .collect(),
        }
    }

    #[must_use]
    pub(super) fn list(field: &str) -> Self {
        Self::List {
            field: field.to_owned(),
        }
    }

    #[must_use]
    pub(super) const fn task_list(style: TaskPathStyle) -> Self {
        Self::TaskList {
            path_style: style,
        }
    }

    /// Renders `rows` according to this display format.
    ///
    /// # Errors
    ///
    /// - [`FieldPath`] if parsing a column or list field path fails.
    /// - [`TableColumnCountMismatch`] if table headers and columns differ in
    ///   length.
    /// - [`TaskListRequiresTaskRows`] if task-list formatting receives page
    ///   rows.
    ///
    /// [`FieldPath`]: QueryError::FieldPath
    /// [`TableColumnCountMismatch`]: QueryError::TableColumnCountMismatch
    /// [`TaskListRequiresTaskRows`]: QueryError::TaskListRequiresTaskRows
    pub(super) fn render(&self, rows: &[QueryRow]) -> QueryResult<String> {
        match self {
            Self::Table {
                headers,
                columns,
            } => Self::render_table(headers, columns, rows),
            Self::List {
                field,
            } => Self::render_list(field, rows),
            Self::TaskList {
                path_style,
            } => Self::render_task_list(rows, *path_style),
        }
    }

    /// Renders a Markdown table with resolved, escaped cells.
    ///
    /// Builds the `ASCII_MARKDOWN` layout with [`String`] operations.
    ///
    /// The formatter pads each column to its widest cell and uses
    /// [`unicode_width`] to measure text. This avoids unused table features.
    /// At 1,000 rows, this formatter took 35 microseconds. `comfy-table` took
    /// 1.1 milliseconds.
    ///
    /// # Errors
    ///
    /// - [`QueryError::TableColumnCountMismatch`] if the header and column
    ///   counts differ.
    /// - [`QueryError::FieldPath`] if a column path is invalid.
    fn render_table(
        headers: &[String],
        columns: &[String],
        rows: &[QueryRow],
    ) -> QueryResult<String> {
        if headers.len() != columns.len() {
            return Err(QueryError::TableColumnCountMismatch {
                headers: headers.len(),
                columns: columns.len(),
            });
        }
        // Keep the prior `comfy-table` output for a table with no columns.
        // No caller creates one, but this branch preserves its output.
        if headers.is_empty() {
            return Ok("||\n".to_owned());
        }
        let paths = columns
            .iter()
            .map(|column| FieldPath::parse(column))
            .collect::<Result<Vec<_>, _>>()?;
        let headers: Vec<String> = headers
            .iter()
            .map(|header| Self::escape_table_cell(header.as_str()))
            .collect();
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                paths
                    .iter()
                    .map(|path| {
                        Self::escape_table_cell(row.resolve_ref(path).text())
                    })
                    .collect()
            })
            .collect();
        let mut widths: Vec<usize> =
            headers.iter().map(|header| header.width()).collect();
        for row in &data {
            for (width, cell) in widths.iter_mut().zip(row) {
                *width = (*width).max(cell.width());
            }
        }
        let mut out = String::new();
        Self::write_table_row(&mut out, &headers, &widths);
        Self::write_table_separator(&mut out, &widths);
        for row in &data {
            Self::write_table_row(&mut out, row, &widths);
        }
        Ok(out)
    }

    /// Writes one `| cell | cell |` row, left-aligned and space-padded to each
    /// column's precomputed width.
    fn write_table_row(out: &mut String, cells: &[String], widths: &[usize]) {
        out.push('|');
        for (cell, &width) in cells.iter().zip(widths) {
            out.push(' ');
            out.push_str(cell);
            for _ in 0..width.saturating_sub(cell.width()) {
                out.push(' ');
            }
            out.push(' ');
            out.push('|');
        }
        out.push('\n');
    }

    /// Writes the `|------|------|` header/data divider matching each column's
    /// padded width (`width + 2` for the one-space margin on each side).
    fn write_table_separator(out: &mut String, widths: &[usize]) {
        out.push('|');
        for &width in widths {
            for _ in 0..width.saturating_add(2) {
                out.push('-');
            }
            out.push('|');
        }
        out.push('\n');
    }

    /// Renders resolved `field` values as Markdown bullets.
    ///
    /// # Errors
    ///
    /// - [`QueryError::FieldPath`] if `field` is not a valid field path.
    fn render_list(field: &str, rows: &[QueryRow]) -> QueryResult<String> {
        let field_path = FieldPath::parse(field)?;
        let mut out = String::new();
        for row in rows {
            out.push_str("- ");
            row.resolve_ref(&field_path).append_text(&mut out);
            out.push('\n');
        }
        Ok(out)
    }

    /// Renders task rows as Markdown checkbox lines.
    ///
    /// # Errors
    ///
    /// - [`QueryError::TaskListRequiresTaskRows`] if any row in `rows` is not a
    ///   task row.
    fn render_task_list(
        rows: &[QueryRow],
        path_style: TaskPathStyle,
    ) -> QueryResult<String> {
        use std::fmt::Write as _;

        let mut out = String::with_capacity(rows.len().saturating_mul(48));
        for row in rows {
            let Some(text) = row.task_text() else {
                return Err(QueryError::TaskListRequiresTaskRows);
            };
            for _ in 0..row.depth() {
                out.push_str("  ");
            }
            let symbol = row.status_symbol().map_or_else(
                || match row.task_completed() {
                    Some(true) => 'x',
                    Some(false) => ' ',
                    None => '-',
                },
                |symbol| symbol.as_char(),
            );
            let _ = write!(out, "- [{symbol}] {text}");
            match path_style {
                TaskPathStyle::Suffix => {
                    let _ = write!(
                        out,
                        " ({})",
                        crate::path::display_portable(row.file().path())
                    );
                }
                TaskPathStyle::Coordinates => {
                    if let Some(line) = row.line() {
                        let _ = write!(
                            out,
                            " ({}:{})",
                            crate::path::display_portable(row.file().path()),
                            line
                        );
                    } else {
                        let _ = write!(
                            out,
                            " ({})",
                            crate::path::display_portable(row.file().path())
                        );
                    }
                }
                TaskPathStyle::None => {}
            }
            out.push('\n');
        }
        Ok(out)
    }

    /// Replaces newlines with spaces and escapes pipes in table cells.
    ///
    /// Returns plain text without another allocation when no escaping is
    /// needed.
    fn escape_table_cell<'a>(text: impl Into<Cow<'a, str>>) -> String {
        let text = text.into();
        if !text.contains(['\n', '|']) {
            return text.into_owned();
        }
        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '\n' => out.push(' '),
                '|' => out.push_str("\\|"),
                other => out.push(other),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod escape_table_cell {
        use pretty_assertions::assert_eq;

        use super::QueryDisplayFormat;

        #[test]
        fn escapes_pipe_characters_in_table_cells() {
            assert_eq!(
                QueryDisplayFormat::escape_table_cell("A | B"),
                "A \\| B"
            );
        }

        #[test]
        fn replaces_newlines_with_spaces_in_table_cells() {
            assert_eq!(
                QueryDisplayFormat::escape_table_cell("line1\nline2"),
                "line1 line2"
            );
        }

        #[test]
        fn passes_plain_text_unmodified() {
            assert_eq!(
                QueryDisplayFormat::escape_table_cell("hello world"),
                "hello world"
            );
        }

        #[test]
        fn escapes_pipes_and_newlines_together() {
            assert_eq!(
                QueryDisplayFormat::escape_table_cell("A\n| B\n"),
                "A \\| B "
            );
        }

        #[test]
        fn escapes_consecutive_pipes() {
            assert_eq!(QueryDisplayFormat::escape_table_cell("||"), "\\|\\|");
        }

        #[test]
        fn passes_empty_string_unmodified() {
            assert_eq!(QueryDisplayFormat::escape_table_cell(""), "");
        }
    }

    mod proptests {
        use proptest::prelude::*;

        use super::QueryDisplayFormat;

        proptest! {
            /// Proves the Markdown table cell escaping law (01:G30, 02:G8):
            /// 1. Safety against row breaking: escaped text contains no literal newline (`\n`).
            /// 2. Delimiter containment: every pipe (`|`) in the escaped output is
            ///    preceded by a backslash. This checks the inserted character,
            ///    not Markdown parity for pre-existing backslashes.
            /// 3. Identity on clean inputs: any input lacking newlines and pipes
            ///    passes through unmodified.
            #[test]
            fn escapes_table_cells_preserving_safety_and_cell_integrity(
                s in "\\PC*",
            ) {
                let escaped = QueryDisplayFormat::escape_table_cell(&s);

                // Safety law: no bare newline exists.
                prop_assert!(!escaped.contains('\n'));

                // Delimiter law: every pipe is preceded by a backslash.
                for (idx, ch) in escaped.char_indices() {
                    if ch == '|' {
                        let preceding = &escaped[..idx];
                        prop_assert!(
                            preceding.ends_with('\\'),
                            "pipe at offset {idx} was not preceded by backslash in {escaped:?}",
                        );
                    }
                }

                // Identity law: clean inputs pass unmodified.
                if !s.contains(['\n', '|']) {
                    prop_assert_eq!(&escaped, &s);
                }
            }
        }
    }

    mod render_task_list {
        use std::{fs, sync::Arc};

        use pretty_assertions::assert_eq;

        use super::*;
        use crate::{
            index::IndexerService,
            query::{QueryBuilder, QueryService, SourceSelector},
        };

        fn rows_for_tasks(
            temp: &std::path::Path,
            source: &str,
        ) -> crate::query::QuerySet {
            fs::write(temp.join("todo.md"), source).expect("write todo.md");
            let index = Arc::new(
                IndexerService::for_tests(temp).build().expect("build index"),
            );
            QueryService::new("class")
                .run(&index, QueryBuilder::tasks(SourceSelector::All))
        }

        #[test]
        fn preserves_custom_status_markers() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let source = "- [ ] buy milk\n- [x] pay rent\n- [/] in \
                          progress\n- [-] cancelled\n- [!] urgent\n- [?] \
                          question\n";
            let rows = rows_for_tasks(temp.path(), source);
            let rendered =
                rows.task_list(TaskPathStyle::None).expect("render task list");
            assert_eq!(
                rendered,
                "- [ ] buy milk\n- [x] pay rent\n- [/] in progress\n- [-] \
                 cancelled\n- [!] urgent\n- [?] question\n"
            );
        }

        #[test]
        fn indents_nested_tasks_by_two_spaces_per_depth() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let source = "- [ ] parent\n  - [/] child\n    - [x] grandchild\n";
            let rows = rows_for_tasks(temp.path(), source);
            let rendered =
                rows.task_list(TaskPathStyle::None).expect("render task list");
            assert_eq!(
                rendered,
                "- [ ] parent\n  - [/] child\n    - [x] grandchild\n"
            );
        }

        #[test]
        fn formats_suffix_path_style() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let source = "- [ ] buy milk\n";
            let rows = rows_for_tasks(temp.path(), source);
            let rendered = rows
                .task_list(TaskPathStyle::Suffix)
                .expect("render task list");
            assert_eq!(rendered, "- [ ] buy milk (todo.md)\n");
        }

        #[test]
        fn formats_coordinate_path_style_with_line_numbers() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let source = "# Header\n\n- [ ] first\n  - [x] second\n";
            let rows = rows_for_tasks(temp.path(), source);
            let rendered = rows
                .task_list(TaskPathStyle::Coordinates)
                .expect("render task list");
            assert_eq!(
                rendered,
                "- [ ] first (todo.md:3)\n  - [x] second (todo.md:4)\n"
            );
        }

        #[test]
        fn rejects_non_task_rows_with_error() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("page.md"), "# Heading\n")
                .expect("write page.md");
            let index = Arc::new(
                IndexerService::for_tests(temp.path())
                    .build()
                    .expect("build index"),
            );
            let rows = QueryService::new("class")
                .run(&index, QueryBuilder::pages(SourceSelector::All));
            let error = rows
                .task_list(TaskPathStyle::None)
                .expect_err("non-task rows must fail");
            assert!(matches!(error, QueryError::TaskListRequiresTaskRows));
        }
    }
}
