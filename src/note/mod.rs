//! Parses Obsidian-style Markdown notes into structured records.
//!
//! [`parse_markdown`] walks a `pulldown-cmark` event stream once, building a
//! [`Note`] that holds YAML frontmatter, headings, lists, outgoing links,
//! inline fields, and tags.
//!
//! # Architecture and Parsing Pipeline
//!
//! 1. `pulldown-cmark` tokenizes raw Markdown with YAML metadata blocks and
//!    wikilinks enabled. The parser recognizes task markers at item-leading
//!    positions with `pulldown-cmark` whitespace rules.
//! 2. `ParserContext` accumulates block-level state, including frontmatter
//!    text, list nesting, and link targets. Its plain-text scan buffers exclude
//!    fenced code blocks, indented code blocks, and inline code spans.
//! 3. When a text block closes, inline token lexing scans the block buffer for
//!    `Key:: Value`, `[Key:: Value]`, `(Key:: Value)`, and `#tag` tokens.
//!    Status-marked list items also recognize task emoji shorthand (`🗓️`, `➕`,
//!    `🛫`, `⏳`, `✅`).
//! 4. The assembled [`Note`] stores all extracted data in document order.
//!
//! # Key Types
//!
//! - [`Heading`]: Markup-stripped heading text, level, and source line.
//! - [`Note`]: Parsed record for one Markdown file.
//! - [`ListItem`], [`ListItemType`], [`TaskListItem`]: Ordered and unordered
//!   list items, including classified task items and checkboxes.
//! - [`ListText`], [`TaskDateSet`](crate::task::TaskDateSet),
//!   [`TaskPriority`](crate::task::TaskPriority): Normalized list text,
//!   extracted task dates, and priority levels.
//! - [`Link`], [`LinkType`], [`LinkTarget`]: Outgoing links from Markdown
//!   `[text](target)` and Obsidian `[[target|alias]]` syntax.
//! - [`Frontmatter`], `RawFrontmatter`: YAML frontmatter as structured fields
//!   or raw text.
//! - [`NoteFieldValue`], [`NoteFieldValueRef`]: Owned and borrowed metadata
//!   values parsed from YAML frontmatter and inline `Key:: Value` syntax.
//! - [`Tag`](crate::Tag): Markdown tags such as `#book` and `#projects/active`.

mod cursor;
mod field;
mod heading;
mod links;
mod lists;
mod metadata;
mod model;
mod parser;

pub(crate) use field::{NoteFieldType, normalize_zero};
pub use field::{NoteFieldValue, NoteFieldValueRef};
pub use heading::Heading;
pub use links::{Link, LinkTarget, LinkType};
pub use lists::{ListItem, ListItemType, ListText, TaskListItem};
pub use metadata::Frontmatter;
pub(crate) use metadata::RawFrontmatter;
pub use model::Note;
pub use parser::{MarkdownParserInput, parse_markdown};
