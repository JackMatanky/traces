//! Defines query languages for source selection, row filtering, and field
//! access.
//!
//! The [`source`] and [`filter`] modules parse query expressions.
//! The [`expr`] module parses their shared boolean operators.
//! The [`field`] module parses and validates field paths.
mod expr;
mod field;
mod filter;
mod source;

pub(crate) use expr::BooleanExpr;
pub(crate) use field::{FieldPath, FileField, ListField, TaskField};
pub(crate) use filter::FilterExpr;
pub use source::SourceSelector;
pub(crate) use source::{
    ClassExpansionMode, FileClassExpander, SourceAtom, SourceExpr,
};
