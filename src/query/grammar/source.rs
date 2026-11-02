//! Parses source expressions and selects documents for `--from` queries.
//!
//! Source expressions match notes in a
//! [`WorkspaceIndex`](crate::index::WorkspaceIndex). They support hierarchical
//! tags, paths, glob patterns, and File Classes. Combine source terms with
//! `and`, `or`, `not`, and parentheses.
use std::{collections::BTreeSet, path::Path};

use logos::{Lexer, Logos};
use regex::Regex;

use super::expr::{
    AtomParser, BooleanExpr, LogicalControl, LogicalOp, parse_boolean_expr,
};
use crate::{
    SpannedTokenStream, Tag, TokenSpec,
    index::FileEntry,
    lexical_unquote,
    note::{Note, NoteFieldValue},
    query::{
        QueryError, QueryResult,
        error::{QueryBuilderError, QueryDialect, QuerySyntaxError},
    },
};

/// Selects source documents for a query, matching all notes or a parsed
/// expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceSelector {
    /// Every note satisfies this source.
    All,
    /// Matching notes satisfy this source.
    Expr(SourceExpr),
}

impl SourceSelector {
    /// Parses `input` as a source expression.
    ///
    /// Blank input yields [`Self::All`].
    ///
    /// # Errors
    ///
    /// - `QueryError::Syntax` if `input` contains an invalid source expression.
    #[inline]
    pub fn parse(input: &str) -> QueryResult<Self> {
        if input.trim().is_empty() {
            Ok(Self::All)
        } else {
            SourceExpr::parse(input).map(Self::Expr).map_err(QueryError::from)
        }
    }

    #[must_use]
    pub(crate) fn is_match(
        &self,
        entry: &FileEntry,
        canonical_class_field: &str,
    ) -> bool {
        match self {
            Self::All => true,
            Self::Expr(expr) => expr.is_match(entry, canonical_class_field),
        }
    }

    /// Reports whether the source contains File Class atoms that need
    /// expansion.
    #[must_use]
    pub(crate) fn has_classes(&self) -> bool {
        match self {
            Self::All => false,
            Self::Expr(expression) => expression.has_classes(),
        }
    }

    /// Expands each File Class atom with `expander`.
    pub(crate) fn resolve_classes(
        &mut self,
        expander: &(impl FileClassExpander + ?Sized),
    ) {
        if let Self::Expr(expression) = self {
            expression.visit_atoms_mut(&mut |atom| {
                if let SourceAtom::Class {
                    names,
                    mode,
                } = atom
                {
                    expander.expand(names, mode);
                }
            });
        }
    }
}

/// Represents a parsed source expression with boolean atom leaves.
///
/// `NOT` has higher precedence than `AND`. `AND` has higher precedence than
/// `OR`. Parentheses override these rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceExpr(BooleanExpr<SourceAtom>);

impl SourceExpr {
    #[must_use]
    pub(crate) fn inner(&self) -> &BooleanExpr<SourceAtom> {
        &self.0
    }

    /// Parses `input` as a source expression.
    ///
    /// # Errors
    ///
    /// - [`QueryBuilderError::Syntax`] if tokenizing or parsing `input` fails.
    pub(crate) fn parse(input: &str) -> Result<Self, QueryBuilderError> {
        let tokens = SpannedTokenStream::<SourceToken>::tokenize(input)
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Source, input, e)
            })?;
        parse_boolean_expr(input, tokens, SourceGrammar).map(Self)
    }

    #[must_use]
    pub(crate) fn is_match(
        &self,
        entry: &FileEntry,
        canonical_class_field: &str,
    ) -> bool {
        self.0
            .is_satisfied_by(|atom| atom.is_match(entry, canonical_class_field))
    }

    /// Checks whether the query needs to resolve class names before matching.
    #[must_use]
    pub(crate) fn has_classes(&self) -> bool {
        self.0.has_any_atom(|atom| matches!(atom, SourceAtom::Class { .. }))
    }

    pub(crate) fn visit_atoms_mut(
        &mut self,
        visitor: &mut impl FnMut(&mut SourceAtom),
    ) {
        self.0.visit_atoms_mut(visitor);
    }

    #[must_use]
    pub(crate) const fn atom(atom: SourceAtom) -> Self {
        Self(BooleanExpr::Atom(atom))
    }

    /// Builds an OR expression, collapsing to `first` when `rest` is empty.
    #[must_use]
    pub(crate) fn or(first: SourceAtom, rest: Vec<SourceAtom>) -> Self {
        if rest.is_empty() {
            Self::atom(first)
        } else {
            let mut atoms = Vec::with_capacity(rest.len().saturating_add(1));
            atoms.push(BooleanExpr::Atom(first));
            atoms.extend(rest.into_iter().map(BooleanExpr::Atom));
            Self(BooleanExpr::Or(atoms))
        }
    }

    /// Builds an AND expression, collapsing to `first` when `rest` is empty.
    #[must_use]
    pub(crate) fn and(first: Self, rest: Vec<Self>) -> Self {
        if rest.is_empty() {
            first
        } else {
            let mut terms = Vec::with_capacity(rest.len().saturating_add(1));
            terms.push(first.0);
            terms.extend(rest.into_iter().map(|term| term.0));
            Self(BooleanExpr::And(terms))
        }
    }
}

/// Represents an atomic predicate in a source expression.
///
/// # Matching rules
///
/// - **[`SourceAtom::Tag`]** matches the named tag or a nested sub-tag. For
///   example, `#book` matches `#book/fiction`.
/// - **[`SourceAtom::Path`]** matches a path against the compiled glob. The
///   pattern `books/` compiles to `books/**`.
/// - **[`SourceAtom::Class`]** matches a class value in the resolved
///   [`ClassExpansionMode`] set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SourceAtom {
    /// Matches a `#tag` pattern.
    Tag(String),
    /// Matches a path pattern.
    Path(GlobPattern),
    /// Matches a File Class hierarchy.
    Class {
        /// Stores the class names from the query.
        names: Vec<String>,
        /// Holds the expansion mode and its resolved class names.
        mode: ClassExpansionMode,
    },
}

impl SourceAtom {
    /// Compiles `pattern` into a path atom.
    ///
    /// # Errors
    ///
    /// - [`regex::Error`] if regex compilation rejects `pattern`.
    pub(crate) fn path(pattern: &str) -> Result<Self, regex::Error> {
        GlobPattern::compile(pattern).map(Self::Path)
    }

    /// Matches `entry` against this atom.
    ///
    /// Non-Markdown files have no [`Note`]. [`Self::Tag`] and [`Self::Class`]
    /// do not match them. [`Self::Path`] matches their paths.
    fn is_match(&self, entry: &FileEntry, canonical_class_field: &str) -> bool {
        match self {
            Self::Tag(tag) => entry.note().is_some_and(|note| {
                note.tags().iter().any(|value| value.is_contained_in(tag))
            }),
            Self::Path(pattern) => pattern.is_match(entry.file().path()),
            Self::Class {
                mode,
                ..
            } => entry.note().is_some_and(|note| {
                class_values(note, canonical_class_field)
                    .any(|value| mode.classes().contains(value))
            }),
        }
    }
}

/// Extracts string values from `canonical_class_field` frontmatter.
pub(crate) fn class_values<'a, 'b>(
    note: &'a Note,
    canonical_class_field: &'b str,
) -> impl Iterator<Item = &'a str> + 'b
where
    'a: 'b,
{
    let values =
        note.frontmatter().map(|fm| fm.get_values(canonical_class_field));
    values.into_iter().flatten().filter_map(NoteFieldValue::as_str)
}

/// Represents a compiled glob used by path source atoms.
///
/// `*` matches characters except `/`. `**` also crosses `/`.
/// Other characters match literally. Native separators become `/` before
/// matching. The compiled regex matches the whole path.
#[derive(Clone)]
pub(crate) struct GlobPattern {
    regex: Regex,
    pattern: String,
}

impl GlobPattern {
    /// Compiles `pattern` into an anchored path matcher.
    ///
    /// Use `/` separators in the pattern. The matcher changes candidate
    /// separators to `/`. A `\` in the pattern stays literal.
    ///
    /// # Errors
    ///
    /// - [`regex::Error`] if regex compilation rejects the translated pattern.
    fn compile(pattern: &str) -> Result<Self, regex::Error> {
        let mut regex_source = String::from("^");
        let mut rest = pattern;
        while let Some(index) = rest.find('*') {
            regex_source.push_str(&regex::escape(&rest[..index]));
            rest = &rest[index.saturating_add(1)..];
            if let Some(stripped) = rest.strip_prefix('*') {
                regex_source.push_str(".*");
                rest = stripped;
            } else {
                regex_source.push_str("[^/]*");
            }
        }
        regex_source.push_str(&regex::escape(rest));
        regex_source.push('$');
        Regex::new(&regex_source).map(|regex| Self {
            regex,
            pattern: pattern.to_owned(),
        })
    }

    #[must_use]
    pub(crate) fn is_match(&self, path: &Path) -> bool {
        let normalized = crate::path::display_portable(path);
        self.regex.is_match(&normalized)
    }
}

impl std::fmt::Debug for GlobPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GlobPattern").field(&self.pattern).finish()
    }
}

impl Eq for GlobPattern {}

impl PartialEq for GlobPattern {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

/// Expansion depth for a File Class match.
///
/// - `Exact` matches the named class. Use the `@C` sigil or the `class(C)`
///   form.
/// - `Children` matches the named class and its direct sub-classes. Use the
///   `@C+` sigil or the `class(C, children)` form.
/// - `Descendants` matches the named class and all transitive sub-classes. Use
///   the `@C*` sigil or the `class(C, descendants)` form.
///
/// The set of resolved class names starts empty after parsing.
/// The query fills it before matching.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClassExpansionMode {
    Exact(BTreeSet<String>),
    Children(BTreeSet<String>),
    Descendants(BTreeSet<String>),
}

impl ClassExpansionMode {
    /// Returns the resolved class names.
    /// [`Self::set_classes`] fills the set before matching.
    #[must_use]
    pub(crate) const fn classes(&self) -> &BTreeSet<String> {
        match self {
            Self::Exact(classes)
            | Self::Children(classes)
            | Self::Descendants(classes) => classes,
        }
    }

    /// Sets the resolved class names and leaves expansion depth unchanged.
    pub(crate) fn set_classes(&mut self, resolved: BTreeSet<String>) {
        match self {
            Self::Exact(classes)
            | Self::Children(classes)
            | Self::Descendants(classes) => *classes = resolved,
        }
    }
}

/// Resolves File Class names against the Schema domain.
///
/// The `Send + Sync` bounds let `QueryOps` store this trait object in
/// [`std::sync::Arc`]. The [`minijinja::Object`] trait also needs these
/// bounds.
pub(crate) trait FileClassExpander: Send + Sync {
    /// Adds names from `classes` to the match set in `mode`.
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode);
}

/// Parser adapter for source atoms in [`parse_boolean_expr`].
struct SourceGrammar;

impl SourceGrammar {
    /// Parses a sigil-form File Class term such as `@Class`, `@Class+`, or
    /// `@Class*`.
    ///
    /// # Errors
    ///
    /// - [`QueryBuilderError::Syntax`] if the sigil is empty or malformed.
    fn parse_sigil(
        input: &str,
        sigil: &str,
        span: std::ops::Range<usize>,
    ) -> Result<SourceAtom, QueryBuilderError> {
        let raw = sigil.strip_prefix('@').ok_or_else(|| {
            QuerySyntaxError::unexpected_end(
                QueryDialect::Source,
                input,
                span.clone(),
                "a File Class name",
            )
        })?;
        let (name, mode) = raw.strip_suffix('+').map_or_else(
            || {
                raw.strip_suffix('*').map_or(
                    (raw, ClassExpansionMode::Exact(BTreeSet::new())),
                    |name| {
                        (name, ClassExpansionMode::Descendants(BTreeSet::new()))
                    },
                )
            },
            |name| (name, ClassExpansionMode::Children(BTreeSet::new())),
        );
        if name.is_empty() {
            return Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Source,
                input,
                span,
                "a File Class name",
            )
            .into());
        }
        Ok(SourceAtom::Class {
            names: vec![name.to_owned()],
            mode,
        })
    }

    /// Parses a function-form File Class term such as `class(Name)`.
    ///
    /// Accepts `class(Name, children)` and `.with_descendants()` modifiers.
    ///
    /// # Errors
    ///
    /// - [`QueryBuilderError::Syntax`] if the form is malformed, the class name
    ///   is empty, the expansion mode is unknown, or both expansion forms
    ///   appear.
    fn parse_class_function(
        input: &str,
        tokens: &mut SpannedTokenStream<SourceToken>,
        class_span: std::ops::Range<usize>,
    ) -> Result<SourceAtom, QueryBuilderError> {
        let lex =
            |e| QuerySyntaxError::from_lex(QueryDialect::Source, input, e);

        tokens
            .expect(
                input,
                TokenSpec::new(&SourceToken::LParen, "`(` after `class`"),
            )
            .map_err(&lex)?;

        let name_spanned = tokens
            .expect_map(input, "a File Class name", |token| {
                let spanned = token;
                match spanned.into_value() {
                    SourceToken::Quoted(name) | SourceToken::Bare(name) => {
                        Some(name)
                    }
                    _ => None,
                }
            })
            .map_err(&lex)?;
        if name_spanned.value().is_empty() {
            return Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Source,
                input,
                class_span,
                "a File Class name",
            )
            .into());
        }

        let mut mode = ClassExpansionMode::Exact(BTreeSet::new());
        let has_mode_argument =
            tokens.peek_is_value(&SourceToken::Comma).then(|| {
                tokens.next();
            });
        if has_mode_argument.is_some() {
            mode = tokens
                .expect_map(input, "`children` or `descendants`", |token| {
                    let spanned = token;
                    match spanned.into_value() {
                        SourceToken::Bare(value) if value == "children" => {
                            Some(ClassExpansionMode::Children(BTreeSet::new()))
                        }
                        SourceToken::Bare(value) if value == "descendants" => {
                            Some(ClassExpansionMode::Descendants(
                                BTreeSet::new(),
                            ))
                        }
                        _ => None,
                    }
                })
                .map_err(&lex)?
                .into_value();
        }

        tokens
            .expect(
                input,
                TokenSpec::new(&SourceToken::RParen, "`)` after `class(...)`"),
            )
            .map_err(&lex)?;

        let modifier = if tokens.peek_is_value(&SourceToken::WithChildren) {
            tokens.next();
            Some(ClassExpansionMode::Children(BTreeSet::new()))
        } else if tokens.peek_is_value(&SourceToken::WithDescendants) {
            tokens.next();
            Some(ClassExpansionMode::Descendants(BTreeSet::new()))
        } else {
            None
        };

        if has_mode_argument.is_some() && modifier.is_some() {
            let next_span = tokens.next_span(input);
            return Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Source,
                input,
                next_span,
                "one File Class expansion mode",
            )
            .into());
        }

        if let Some(modifier) = modifier {
            mode = modifier;
        }

        Ok(SourceAtom::Class {
            names: vec![name_spanned.into_value()],
            mode,
        })
    }

    /// Strips leading `./` and `.\\` prefixes from path atoms.
    fn normalize_path_prefix(mut path: &str) -> &str {
        while let Some(stripped) =
            path.strip_prefix("./").or_else(|| path.strip_prefix(".\\"))
        {
            path = stripped;
        }
        path
    }
}

impl AtomParser for SourceGrammar {
    type Atom = SourceAtom;
    type Token = SourceToken;

    fn control(&self, token: &Self::Token) -> Option<LogicalControl> {
        match token {
            SourceToken::Logical(operator) => {
                Some(LogicalControl::Operator(*operator))
            }
            SourceToken::Not => Some(LogicalControl::Not),
            SourceToken::LParen => Some(LogicalControl::LeftParen),
            SourceToken::RParen => Some(LogicalControl::RightParen),
            SourceToken::Comma
            | SourceToken::Class
            | SourceToken::WithChildren
            | SourceToken::WithDescendants
            | SourceToken::Tag(_)
            | SourceToken::ClassSigil(_)
            | SourceToken::Quoted(_)
            | SourceToken::Bare(_) => None,
        }
    }

    fn parse_atom(
        &self,
        input: &str,
        tokens: &mut SpannedTokenStream<Self::Token>,
    ) -> Result<Self::Atom, QueryBuilderError> {
        let next_span = tokens.next_span(input);
        match tokens.next() {
            Some(spanned) => {
                let (value, span) = spanned.into_parts_usize();
                match value {
                    SourceToken::Tag(tag) => Ok(SourceAtom::Tag(tag)),
                    SourceToken::ClassSigil(sigil) => {
                        Self::parse_sigil(input, &sigil, span)
                    }
                    SourceToken::Quoted(path) | SourceToken::Bare(path) => {
                        let normalized = Self::normalize_path_prefix(&path);
                        let glob = if normalized.ends_with('/') {
                            format!("{normalized}**")
                        } else {
                            normalized.to_owned()
                        };
                        GlobPattern::compile(&glob)
                            .map(SourceAtom::Path)
                            .map_err(|_| {
                                QuerySyntaxError::unexpected_end(
                                    QueryDialect::Source,
                                    input,
                                    span,
                                    "a valid path glob",
                                )
                                .into()
                            })
                    }
                    SourceToken::Class => {
                        Self::parse_class_function(input, tokens, span)
                    }
                    _ => Err(QuerySyntaxError::unexpected_end(
                        QueryDialect::Source,
                        input,
                        span,
                        "a source term",
                    )
                    .into()),
                }
            }
            None => Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Source,
                input,
                next_span,
                "a source term",
            )
            .into()),
        }
    }

    fn syntax_error(
        &self,
        input: &str,
        span: std::ops::Range<usize>,
        expected: &'static str,
    ) -> QuerySyntaxError {
        QuerySyntaxError::unexpected_end(
            QueryDialect::Source,
            input,
            span,
            expected,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Logos)]
#[logos(skip r"[ \t\n\r\f]+")]
enum SourceToken {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[regex(
        "&&|and|\\|\\||or",
        |lex| LogicalOp::try_from(lex.slice()),
        ignore(case)
    )]
    Logical(LogicalOp),
    #[expect(
        clippy::duplicated_attributes,
        reason = "logos requires one #[token] per literal; the shared \
                  `priority = 3` meta trips the lint"
    )]
    #[token("!", priority = 3)]
    #[token("not", priority = 3, ignore(case))]
    Not,
    #[token("class", priority = 3, ignore(case))]
    Class,
    #[token(".with_children()", priority = 4)]
    WithChildren,
    #[token(".with_descendants()", priority = 4)]
    WithDescendants,
    #[token("#", tag_callback)]
    Tag(String),
    #[regex(r"@[a-zA-Z0-9_\-./]+[+*]?", |lex| lex.slice().to_owned())]
    ClassSigil(String),
    #[regex(r#"\"([^\"\\]|\\.)*\"|'([^'\\]|\\.)*'"#, quoted_callback)]
    Quoted(String),
    #[regex(r#"[^\s(),!&|#@'"]+"#, |lex| lex.slice().to_owned())]
    Bare(String),
}

/// Parses a `#tag`-shaped token starting at the already-consumed leading `#`.
///
/// Uses [`Tag::prefix_len`], the scanner used by the Markdown body lexer.
/// This accepts the same tag shapes as notes, including one-letter tags.
/// Returns [`LexError::UnexpectedToken`](crate::LexError::UnexpectedToken) if
/// `#` is not followed by a valid tag body.
///
/// # Errors
///
/// Returns an error if no valid tag body follows `#`.
fn tag_callback(lex: &mut Lexer<'_, SourceToken>) -> Result<String, ()> {
    let tag_start = lex.span().start;
    let tail = lex.source().get(tag_start..).ok_or(())?;
    let tag_len = Tag::prefix_len(tail).ok_or(())?;
    lex.bump(tag_len.saturating_sub('#'.len_utf8()));
    Ok(lex.slice().to_owned())
}

#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "logos Callback trait requires &mut Lexer"
)]
fn quoted_callback(lex: &mut Lexer<'_, SourceToken>) -> String {
    lexical_unquote(lex.slice())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn exact() -> ClassExpansionMode {
        ClassExpansionMode::Exact(BTreeSet::new())
    }

    fn children() -> ClassExpansionMode {
        ClassExpansionMode::Children(BTreeSet::new())
    }

    fn descendants() -> ClassExpansionMode {
        ClassExpansionMode::Descendants(BTreeSet::new())
    }

    mod parse {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        fn tag(value: &str) -> BooleanExpr<SourceAtom> {
            BooleanExpr::Atom(SourceAtom::Tag(value.to_owned()))
        }

        fn path(value: &str) -> BooleanExpr<SourceAtom> {
            let SourceExpr(parsed) =
                SourceExpr::parse(value).expect("valid path");
            match parsed {
                atom @ BooleanExpr::Atom(SourceAtom::Path(_)) => Some(atom),
                _ => None,
            }
            .expect("parsed value must be a path atom")
        }

        fn class(
            name: &str,
            mode: ClassExpansionMode,
        ) -> BooleanExpr<SourceAtom> {
            BooleanExpr::Atom(SourceAtom::Class {
                names: vec![name.to_owned()],
                mode,
            })
        }

        #[test]
        fn parses_tag_and_paths() {
            assert_eq!(
                SourceExpr::parse("#projects/active"),
                Ok(SourceExpr(tag("#projects/active")))
            );
            assert_eq!(
                SourceExpr::parse("\"books/dune.md\""),
                Ok(SourceExpr(path("books/dune.md")))
            );
            assert_eq!(
                SourceExpr::parse("books/"),
                Ok(SourceExpr(path("books/")))
            );
        }

        #[test]
        fn parses_a_single_letter_tag() {
            assert_eq!(SourceExpr::parse("#a"), Ok(SourceExpr(tag("#a"))));
        }

        #[test]
        fn parses_each_class_expansion_form() {
            assert_eq!(
                SourceExpr::parse("@Book"),
                Ok(SourceExpr(class("Book", exact())))
            );
            assert_eq!(
                SourceExpr::parse("@Book+"),
                Ok(SourceExpr(class("Book", children())))
            );
            assert_eq!(
                SourceExpr::parse("@Book*"),
                Ok(SourceExpr(class("Book", descendants())))
            );
            assert_eq!(
                SourceExpr::parse("class(Book, children)"),
                Ok(SourceExpr(class("Book", children())))
            );
            assert_eq!(
                SourceExpr::parse("class(Book).with_descendants()"),
                Ok(SourceExpr(class("Book", descendants())))
            );
        }

        #[test]
        fn parses_precedence_grouping_and_repeated_negation() {
            assert_eq!(
                SourceExpr::parse("#book or books/ and not @Archived"),
                Ok(SourceExpr(BooleanExpr::Or(vec![
                    tag("#book"),
                    BooleanExpr::And(vec![
                        path("books/"),
                        BooleanExpr::Not(Box::new(class("Archived", exact()))),
                    ]),
                ])))
            );
            assert_eq!(
                SourceExpr::parse("(#book or #movie) and !archive/"),
                Ok(SourceExpr(BooleanExpr::And(vec![
                    BooleanExpr::Or(vec![tag("#book"), tag("#movie")]),
                    BooleanExpr::Not(Box::new(path("archive/"))),
                ])))
            );
            assert_eq!(
                SourceExpr::parse("not not #book and #movie and archive/"),
                Ok(SourceExpr(BooleanExpr::And(vec![
                    BooleanExpr::Not(Box::new(BooleanExpr::Not(Box::new(
                        tag("#book"),
                    )))),
                    tag("#movie"),
                    path("archive/"),
                ])))
            );
        }
        #[test]
        fn accepts_symbolic_boolean_operators() {
            assert!(matches!(
                SourceExpr::parse("#book && !@Archived || @Movie"),
                Ok(SourceExpr(BooleanExpr::Or(_)))
            ));
        }

        #[test]
        fn rejects_incomplete_and_trailing_input() {
            for invalid in [
                "",
                "#book and",
                "(#book",
                "#book #movie",
                r#""books/dune.md"#,
                r"'books/dune.md",
                "class()",
                "class(Book, unknown)",
                "class(Book, children).with_descendants()",
                "class(Book).with_children() extra",
            ] {
                assert!(SourceExpr::parse(invalid).is_err(), "{invalid}");
            }
        }

        #[test]
        fn parses_a_single_star_wildcard_path_glob() {
            assert!(SourceExpr::parse("covers/*.md").is_ok());
        }

        #[test]
        fn parses_a_double_star_wildcard_path_glob() {
            assert!(SourceExpr::parse("**/draft.md").is_ok());
        }

        #[rstest]
        #[case("./notes/daily.md")]
        #[case(".\\notes/daily.md")]
        fn normalizes_leading_relative_path_prefix(#[case] input: &str) {
            let expr = SourceExpr::parse(input).expect("valid source");
            assert_eq!(
                expr,
                SourceExpr(BooleanExpr::Atom(
                    SourceAtom::path("notes/daily.md").expect("valid atom")
                ))
            );
        }

        #[test]
        fn parses_unquoted_markdown_file_path() {
            let expr =
                SourceExpr::parse("notes/daily.md").expect("valid source");
            assert_eq!(
                expr,
                SourceExpr(BooleanExpr::Atom(
                    SourceAtom::path("notes/daily.md").expect("valid atom")
                ))
            );
        }
    }
    mod glob_pattern {
        use std::borrow::Cow;

        use super::*;

        #[cfg(not(windows))]
        #[test]
        fn keeps_backslashes_literal_on_unix() {
            let path = r"covers\dune.md";
            let normalized = crate::path::display_portable(Path::new(path));

            assert!(matches!(normalized, Cow::Borrowed(_)));
            assert_eq!(normalized, path);
            let pattern =
                GlobPattern::compile("covers/*.md").expect("valid glob");
            assert!(!pattern.is_match(Path::new(path)));
        }

        #[cfg(windows)]
        #[test]
        fn normalizes_windows_separators_for_slash_globs() {
            let normalized =
                crate::path::display_portable(Path::new(r"notes\a.md"));
            assert_eq!(normalized, "notes/a.md");

            let pattern =
                GlobPattern::compile("notes/*.md").expect("valid glob");
            assert!(pattern.is_match(Path::new(r"notes\a.md")));
        }
    }

    mod constructors {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn collapses_or_with_no_rest_to_bare_atom() {
            let atom = SourceAtom::Tag("book".to_owned());
            assert_eq!(
                SourceExpr::or(atom.clone(), Vec::new()),
                SourceExpr::atom(atom)
            );
        }

        #[test]
        fn wraps_or_with_rest_in_order() {
            let first = SourceAtom::Tag("book".to_owned());
            let second = SourceAtom::Tag("movie".to_owned());
            assert_eq!(
                SourceExpr::or(first.clone(), vec![second.clone()]),
                SourceExpr(BooleanExpr::Or(vec![
                    BooleanExpr::Atom(first),
                    BooleanExpr::Atom(second),
                ]))
            );
        }

        #[test]
        fn collapses_and_with_no_rest_to_first_term() {
            let term = SourceExpr::atom(SourceAtom::Tag("book".to_owned()));
            assert_eq!(SourceExpr::and(term.clone(), Vec::new()), term);
        }

        #[test]
        fn wraps_and_with_rest_in_order() {
            let first = SourceExpr::atom(SourceAtom::Tag("book".to_owned()));
            let second = SourceExpr::atom(SourceAtom::Tag("movie".to_owned()));
            assert_eq!(
                SourceExpr::and(first.clone(), vec![second.clone()]),
                SourceExpr(BooleanExpr::And(vec![first.0, second.0]))
            );
        }
    }

    mod is_match {
        use std::{fs, path::Path};

        use rstest::rstest;

        use super::*;
        use crate::{IndexerService, index::WorkspaceIndex};

        fn find_entry<'a>(
            entries: &'a [crate::index::FileEntry],
            path: &Path,
        ) -> &'a crate::index::FileEntry {
            entries
                .iter()
                .find(|entry| entry.file().path() == path)
                .expect("entry not found")
        }
        fn indexed_note(
            content: &str,
            path: &str,
        ) -> (tempfile::TempDir, WorkspaceIndex) {
            let temp = tempfile::tempdir().expect("create temp dir");
            let full_path = temp.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(full_path, content).expect("write Note");
            let index = IndexerService::for_tests(temp.path())
                .build()
                .expect("build index");
            (temp, index)
        }

        #[test]
        fn matches_boolean_combinations_of_tags_and_paths() {
            let (_temp, index) = indexed_note("#book", "books/dune.md");
            let entry = find_entry(index.entries(), Path::new("books/dune.md"));
            let expression =
                SourceExpr::parse("(#book and books/) and not archive/")
                    .expect("valid source");

            assert!(expression.is_match(entry, "class"));
        }

        #[test]
        fn matches_a_tag_sourced_only_from_frontmatter() {
            let (_temp, index) = indexed_note(
                "---\ntags: [book]\n---\nNo body tag here.",
                "books/dune.md",
            );
            let entry = find_entry(index.entries(), Path::new("books/dune.md"));
            let expression = SourceExpr::parse("#book").expect("valid source");

            assert!(expression.is_match(entry, "class"));
        }

        #[test]
        fn matches_exact_file_path_without_matching_sibling() {
            let (_temp, index) = indexed_note("#book", "books/dune.md");
            let entry = find_entry(index.entries(), Path::new("books/dune.md"));

            assert!(
                SourceExpr::parse("books/dune.md")
                    .expect("valid source")
                    .is_match(entry, "class")
            );
            assert!(
                !SourceExpr::parse("books/hyperion.md")
                    .expect("valid source")
                    .is_match(entry, "class")
            );
        }

        #[rstest]
        #[case("./notes/daily.md")]
        #[case(".\\notes/daily.md")]
        fn matches_path_with_leading_relative_prefix(#[case] pattern: &str) {
            let (_temp, index) = indexed_note("#book", "notes/daily.md");
            let entry =
                find_entry(index.entries(), Path::new("notes/daily.md"));

            assert!(
                SourceExpr::parse(pattern)
                    .expect("valid source")
                    .is_match(entry, "class")
            );
        }

        #[test]
        fn reads_classes_from_the_execution_field() {
            let (_temp, index) =
                indexed_note("---\nkind: Book\n---\n", "book.md");
            let entry = find_entry(index.entries(), Path::new("book.md"));
            let expression = SourceExpr::atom(SourceAtom::Class {
                names: vec!["Book".to_owned()],
                mode: ClassExpansionMode::Exact(BTreeSet::from([
                    "Book".to_owned()
                ])),
            });

            assert!(expression.is_match(entry, "kind"));
            assert!(!expression.is_match(entry, "class"));
        }

        #[test]
        fn matches_one_folder_level_but_not_subfolder() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::create_dir_all(temp.path().join("covers/sub"))
                .expect("create nested dir");
            fs::write(temp.path().join("covers/dune.md"), "# Dune")
                .expect("write direct file");
            fs::write(temp.path().join("covers/sub/hidden.md"), "# Hidden")
                .expect("write nested file");
            let index = IndexerService::for_tests(temp.path())
                .build()
                .expect("build index");
            let expression =
                SourceExpr::parse("covers/*.md").expect("valid source");
            let direct =
                find_entry(index.entries(), Path::new("covers/dune.md"));
            let nested =
                find_entry(index.entries(), Path::new("covers/sub/hidden.md"));

            assert!(expression.is_match(direct, "class"));
            assert!(!expression.is_match(nested, "class"));
        }

        #[test]
        fn crosses_folder_boundaries_with_double_star() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::create_dir_all(temp.path().join("covers/sub"))
                .expect("create nested dir");
            fs::write(temp.path().join("covers/dune.md"), "# Dune")
                .expect("write direct file");
            fs::write(temp.path().join("covers/sub/hidden.md"), "# Hidden")
                .expect("write nested file");
            let index = IndexerService::for_tests(temp.path())
                .build()
                .expect("build index");
            // `covers/**/*.md` needs an intermediate path segment.
            // This checks that `**` crosses `/` boundaries.
            let expression =
                SourceExpr::parse("covers/**/*.md").expect("valid source");
            let direct =
                find_entry(index.entries(), Path::new("covers/dune.md"));
            let nested =
                find_entry(index.entries(), Path::new("covers/sub/hidden.md"));

            assert!(!expression.is_match(direct, "class"));
            assert!(expression.is_match(nested, "class"));
        }
    }

    mod has_classes {
        use super::*;

        #[test]
        fn returns_true_when_expression_contains_class_atom() {
            let expr = SourceExpr::atom(SourceAtom::Class {
                names: vec!["project".to_owned()],
                mode: ClassExpansionMode::Children(BTreeSet::new()),
            });
            assert!(
                expr.has_classes(),
                "expression with Class atom must return true"
            );
        }

        #[test]
        fn returns_false_for_all_source() {
            let source = SourceSelector::All;
            assert!(
                !source.has_classes(),
                "SourceSelector::All must return false"
            );
        }
    }

    mod class_values_extraction {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn extracts_values_from_list_field() {
            let note = crate::test_support::parse_note(
                "test.md",
                "---\ntags:\n  - rust\n  - pkm\n---\nBody.",
            );
            let values: Vec<&str> = class_values(&note, "tags").collect();
            assert_eq!(values, vec!["rust", "pkm"]);
        }
    }

    mod parse_class_function {
        use super::*;

        #[test]
        fn parses_descendants_expansion_mode() {
            let result = SourceExpr::parse("class(Book, descendants)");
            assert!(
                result.is_ok(),
                "class(Book, descendants) must parse: {result:?}"
            );
        }
    }

    mod resolve_classes {
        use pretty_assertions::assert_eq;

        use super::*;

        /// Test double that records expansion calls and resolves each class to
        /// itself.
        ///
        /// Uses [`std::sync::Mutex`] because [`std::cell::RefCell`] is not
        /// thread-safe.
        #[derive(Default)]
        struct RecordingExpander {
            calls: std::sync::Mutex<Vec<Vec<String>>>,
        }

        impl FileClassExpander for RecordingExpander {
            fn expand(
                &self,
                classes: &[String],
                mode: &mut ClassExpansionMode,
            ) {
                self.calls.lock().expect("lock calls").push(classes.to_vec());
                mode.set_classes(classes.iter().cloned().collect());
            }
        }

        #[test]
        fn walks_nested_expressions_and_preserves_unknowns() {
            let mut source = SourceSelector::parse("@thing+ and not @ghost*")
                .expect("source parses");
            let expander = RecordingExpander::default();

            source.resolve_classes(&expander);

            assert_eq!(expander.calls.into_inner().expect("lock calls"), vec![
                vec!["thing".to_owned()],
                vec!["ghost".to_owned()],
            ]);
        }
    }
}
