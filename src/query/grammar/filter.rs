//! Parses and evaluates record filter expressions for `--where` queries.
//!
//! This module parses and evaluates row-level filter expressions. Expressions
//! support:
//! - Dotted and bare field path lookups resolved via [`FieldPath`].
//! - Comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`) with type
//!   coercion.
//! - Function calls (for example, contains(field, target)) support tag-prefix
//!   matching.
//! - Value expressions: date ± duration, date − date (duration result),
//!   duration ± duration and duration × number, plus `dur`, `date_add`,
//!   `date_diff`, and `date_component` calls. Arithmetic groups left-to-right
//!   without precedence; null and invalid row values do not satisfy ordered
//!   comparisons. Static invalid function arguments fail at parse time.
//! - Boolean combinators (`and`, `or`, `not`, parentheses) parsed via the
//!   shared boolean expression grammar.
use logos::{Lexer, Logos};

use super::{
    FieldPath,
    expr::{
        AtomParser, BooleanExpr, LogicalControl, LogicalOp, parse_boolean_expr,
    },
};
use crate::{
    DurationValue, LexError, NoteFieldValue, NoteFieldValueRef, Spanned,
    SpannedTokenStream, TimeUnit, TokenSpec, lexical_unquote,
    query::{
        QueryRow,
        error::{QueryBuilderError, QueryDialect, QuerySyntaxError},
        sort::TextShape,
        value::QueryFieldValueRef,
    },
    temporal::{
        CalendarError, CalendarPeriod, DatePoint, DateUnit, PeriodEdge,
    },
};

/// Represents a parsed filter expression AST used by
/// [`crate::query::QuerySet::filter`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FilterExpr(BooleanExpr<FilterAtom>);

impl FilterExpr {
    /// Parses filter syntax into a logical expression tree.
    ///
    /// # Errors
    ///
    /// - [`Syntax`] if the expression syntax is invalid.
    /// - [`FieldPath`] if any field path is invalid.
    ///
    /// [`Syntax`]: QueryBuilderError::Syntax
    /// [`FieldPath`]: QueryBuilderError::FieldPath
    pub(crate) fn parse(input: &str) -> Result<Self, QueryBuilderError> {
        let tokens =
            SpannedTokenStream::<FilterToken>::tokenize_with(input, |token| {
                let (value, span) = token.into_parts();
                match value {
                    FilterToken::Ident(word) => match word.parse::<f64>() {
                        Ok(number) if number.is_finite() => Ok(Spanned::new(
                            FilterToken::Literal(NoteFieldValue::Number(
                                number,
                            )),
                            span,
                        )),
                        Ok(_) => Err(LexError::UnexpectedToken {
                            span: span.to_range(),
                            found: "NaN or infinity".to_owned(),
                            expected: "a finite numeric literal",
                        }),
                        Err(_) => {
                            if let Ok(dv) = DurationValue::parse(&word) {
                                Ok(Spanned::new(
                                    FilterToken::Literal(
                                        NoteFieldValue::Duration(dv),
                                    ),
                                    span,
                                ))
                            } else {
                                Ok(Spanned::new(FilterToken::Ident(word), span))
                            }
                        }
                    },
                    FilterToken::DateLiteral(text) => {
                        match TextShape::classify(&text) {
                            TextShape::DateTime(datetime) => Ok(Spanned::new(
                                FilterToken::Literal(NoteFieldValue::DateTime(
                                    datetime,
                                )),
                                span,
                            )),
                            TextShape::Date(date) => Ok(Spanned::new(
                                FilterToken::Literal(NoteFieldValue::Date(
                                    date,
                                )),
                                span,
                            )),
                            TextShape::Duration(_) | TextShape::Plain => {
                                Err(LexError::UnexpectedToken {
                                    span: span.to_range(),
                                    found: text,
                                    expected: "a valid date or date-time \
                                               literal",
                                })
                            }
                        }
                    }
                    other => Ok(Spanned::new(other, span)),
                }
            })
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;
        parse_boolean_expr(input, tokens, FilterGrammar).map(Self)
    }

    /// Combines with logical AND, flattening nested `And` nodes.
    pub(crate) fn and(self, other: Self) -> Self {
        let mut children = match self.0 {
            BooleanExpr::And(children) => children,
            atom => vec![atom],
        };
        match other.0 {
            BooleanExpr::And(more) => children.extend(more),
            atom => children.push(atom),
        }
        Self(BooleanExpr::And(children))
    }

    pub(crate) fn is_matching(&self, row: &QueryRow) -> bool {
        self.0.is_satisfied_by(|atom| atom.is_matching(row))
    }
}

/// Represents an atomic predicate: either a comparison or a recognized function
/// call.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FilterAtom {
    /// Wraps a comparison expression between two value expressions.
    Comparison(ComparisonExpr),
    /// Wraps a filter function with type-specific matching semantics.
    Function(FilterFunction),
}

impl FilterAtom {
    fn is_matching(&self, row: &QueryRow) -> bool {
        match self {
            Self::Comparison(comparison) => comparison.is_matching(row),
            Self::Function(function) => function.is_matching(row),
        }
    }
}

/// Represents a filter function with type-specific matching semantics.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FilterFunction {
    /// `contains(field, target)` predicate.
    ///
    /// Lists match by exact value or tag prefix, such as `#book` matching
    /// `#book/fiction`; other field kinds fall back to substring containment.
    Contains {
        field: FieldPath,
        target: NoteFieldValue,
    },
}

impl FilterFunction {
    /// Accepts `contains` case-insensitively.
    fn build(
        name: &str,
        field: FieldPath,
        target: NoteFieldValue,
    ) -> Option<Self> {
        if name.eq_ignore_ascii_case("contains") {
            Some(Self::Contains {
                field,
                target,
            })
        } else {
            None
        }
    }

    fn is_matching(&self, row: &QueryRow) -> bool {
        match self {
            Self::Contains {
                field,
                target,
            } => row.resolve_ref(field).is_containing(target),
        }
    }
}

/// Represents a comparison expression between two value expressions.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComparisonExpr {
    lhs: ValueExpr,
    op: CompareOp,
    rhs: ValueExpr,
}

impl ComparisonExpr {
    pub(super) fn new(lhs: ValueExpr, op: CompareOp, rhs: ValueExpr) -> Self {
        Self {
            lhs,
            op,
            rhs,
        }
    }

    /// Returns `true` if `row` satisfies `self.op` between `self.lhs` and
    /// `self.rhs`.
    pub(super) fn is_matching(&self, row: &QueryRow) -> bool {
        if let (ValueExpr::Field(path), ValueExpr::Literal(literal)) =
            (&self.lhs, &self.rhs)
        {
            return self.op.is_satisfied_by(&row.resolve_ref(path), literal);
        }
        let Some(lhs_val) = self.lhs.evaluate(row) else {
            return false;
        };
        let Some(rhs_val) = self.rhs.evaluate(row) else {
            return false;
        };
        self.op.is_satisfied_by_values(&lhs_val, &rhs_val)
    }

    /// Promotes a quoted filter literal's `String` payload to `Date`/
    /// `DateTime`/`Duration` when its text has that shape, once, at query-build
    /// time (not per row).
    ///
    /// Only `NoteFieldValue::String` needs inspection: bare date literals are
    /// promoted while tokenizing, and the grammar's other literal shapes
    /// (`Null`, `Bool`, `Number`) are already typed. Reuses
    /// [`TextShape::classify`] (the same heuristic `SortKey::from_text` uses),
    /// so filter literals and sort-key text classify identically, not via a
    /// duplicate implementation.
    pub(super) fn classify_literal(literal: NoteFieldValue) -> NoteFieldValue {
        let NoteFieldValue::String(text) = &literal else {
            return literal;
        };
        match TextShape::classify(text) {
            TextShape::DateTime(value) => NoteFieldValue::DateTime(value),
            TextShape::Date(value) => NoteFieldValue::Date(value),
            TextShape::Duration(value) => NoteFieldValue::Duration(value),
            TextShape::Plain => literal,
        }
    }
}

/// Represents a value expression in a query filter.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ValueExpr {
    /// Field path lookup on a query row.
    Field(FieldPath),
    /// A literal value.
    Literal(NoteFieldValue),
    /// A value-returning function call (e.g. `dur`, `date_add`, `date_diff`,
    /// `date_component`).
    Call {
        function: QueryFunction,
        args: Vec<Self>,
    },
    /// An arithmetic binary operation evaluated left-to-right.
    Binary {
        lhs: Box<Self>,
        op: ArithOp,
        rhs: Box<Self>,
    },
}

impl ValueExpr {
    /// Evaluates a value expression; an invalid row value cannot satisfy a
    /// comparison.
    fn evaluate(&self, row: &QueryRow) -> Option<NoteFieldValue> {
        match self {
            Self::Field(path) => Some(row.resolve_ref(path).to_owned_value()),
            Self::Literal(value) => Some(value.clone()),
            Self::Call {
                function,
                args,
            } => {
                let values = args
                    .iter()
                    .map(|arg| arg.evaluate(row))
                    .collect::<Option<Vec<_>>>()?;
                function.evaluate(&values)
            }
            Self::Binary {
                lhs,
                op,
                rhs,
            } => {
                let left = lhs.evaluate(row)?;
                op.evaluate(&left, rhs.evaluate(row)?)
            }
        }
    }

    /// Whether this is arithmetic over number literals only.
    fn is_number_only(&self) -> bool {
        match self {
            Self::Literal(NoteFieldValue::Number(_)) => true,
            Self::Binary {
                lhs,
                rhs,
                ..
            } => lhs.is_number_only() && rhs.is_number_only(),
            Self::Field(_)
            | Self::Literal(_)
            | Self::Call {
                ..
            } => false,
        }
    }

    /// Whether this is a string literal that names no duration unit.
    fn is_unrecognized_unit(&self) -> bool {
        matches!(
            self,
            Self::Literal(NoteFieldValue::String(unit))
                if TimeUnit::from_spelling(unit).is_none()
        )
    }

    /// Whether this is a string literal that is not a valid duration.
    fn is_invalid_duration_literal(&self) -> bool {
        matches!(
            self,
            Self::Literal(NoteFieldValue::String(text))
                if DurationValue::parse(text).is_err()
        )
    }
}

/// Represents a binary arithmetic operator.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ArithOp {
    /// Addition operator (`+`).
    Add,
    /// Subtraction operator (`-`).
    Sub,
    /// Multiplication operator (`*`).
    Mul,
}

impl ArithOp {
    /// Applies the operator to two evaluated values.
    ///
    /// Returns `Null` when either operand is `Null` or the operand types do not
    /// support the operator, and `None` when a date or duration result is not
    /// representable.
    fn evaluate(
        self,
        left: &NoteFieldValue,
        right: NoteFieldValue,
    ) -> Option<NoteFieldValue> {
        match left {
            NoteFieldValue::Date(date) => {
                self.evaluate_date((*date).into(), right)
            }
            NoteFieldValue::DateTime(datetime) => {
                self.evaluate_date((*datetime).into(), right)
            }
            NoteFieldValue::Duration(duration) => {
                self.evaluate_duration(*duration, right)
            }
            NoteFieldValue::Number(factor) => {
                self.evaluate_number(*factor, right)
            }
            _ => Some(NoteFieldValue::Null),
        }
    }

    fn evaluate_date(
        self,
        date: DatePoint,
        right: NoteFieldValue,
    ) -> Option<NoteFieldValue> {
        match self {
            Self::Add => match right {
                NoteFieldValue::Duration(duration) => {
                    Some(NoteFieldValue::from(
                        date.apply(
                            &duration,
                            &crate::temporal::local_zone().ok()?,
                        )
                        .ok()?,
                    ))
                }
                _ => Some(NoteFieldValue::Null),
            },
            Self::Sub => match right {
                NoteFieldValue::Duration(duration) => {
                    let duration = duration.checked_neg().ok()?;
                    Some(NoteFieldValue::from(
                        date.apply(
                            &duration,
                            &crate::temporal::local_zone().ok()?,
                        )
                        .ok()?,
                    ))
                }
                other @ (NoteFieldValue::Date(_)
                | NoteFieldValue::DateTime(_)) => {
                    let diff = other
                        .as_date_point()?
                        .diff(
                            date,
                            TimeUnit::Second,
                            &crate::temporal::local_zone().ok()?,
                        )
                        .ok()?;
                    let seconds = diff.to_f64()?;
                    Some(NoteFieldValue::Duration(
                        DurationValue::from_nominal_seconds(seconds).ok()?,
                    ))
                }
                _ => Some(NoteFieldValue::Null),
            },
            Self::Mul => Some(NoteFieldValue::Null),
        }
    }

    fn evaluate_duration(
        self,
        duration: DurationValue,
        right: NoteFieldValue,
    ) -> Option<NoteFieldValue> {
        match self {
            Self::Add => match right {
                date @ (NoteFieldValue::Date(_)
                | NoteFieldValue::DateTime(_)) => Some(NoteFieldValue::from(
                    date.as_date_point()?
                        .apply(&duration, &crate::temporal::local_zone().ok()?)
                        .ok()?,
                )),
                NoteFieldValue::Duration(other) => Some(
                    NoteFieldValue::Duration(duration.checked_add(other).ok()?),
                ),
                _ => Some(NoteFieldValue::Null),
            },
            Self::Sub => match right {
                NoteFieldValue::Duration(other) => Some(
                    NoteFieldValue::Duration(duration.checked_sub(other).ok()?),
                ),
                _ => Some(NoteFieldValue::Null),
            },
            Self::Mul => match right {
                NoteFieldValue::Number(factor) => {
                    Some(NoteFieldValue::Duration(
                        duration.checked_mul(factor).ok()?,
                    ))
                }
                _ => Some(NoteFieldValue::Null),
            },
        }
    }

    fn evaluate_number(
        self,
        factor: f64,
        right: NoteFieldValue,
    ) -> Option<NoteFieldValue> {
        match (self, right) {
            (Self::Mul, NoteFieldValue::Duration(duration)) => Some(
                NoteFieldValue::Duration(duration.checked_mul(factor).ok()?),
            ),
            _ => Some(NoteFieldValue::Null),
        }
    }
}

/// A function callable from a filter expression, resolved at parse time.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum QueryFunction {
    /// `dur(text)`: duration literal conversion.
    Dur,
    /// `date_add(date, duration)` or `date_add(date, n, unit)`.
    DateAdd,
    /// `date_diff(date1, date2[, unit])`.
    DateDiff,
    /// `date_component(date, name)`.
    DateComponent,
    /// First day of a calendar period (`sow`, `som`, `soy` and long spellings).
    StartOf(CalendarPeriod),
    /// Last day of a calendar period (`eow`, `eom`, `eoy` and long spellings).
    EndOf(CalendarPeriod),
    /// `date(text[, pattern])`: date literal conversion.
    Date,
}

impl QueryFunction {
    /// Functions whose names do not come from the calendar period lookup.
    const FIXED_NAMES: &'static [(&'static str, Self)] = &[
        ("dur", Self::Dur),
        ("date_add", Self::DateAdd),
        ("date_diff", Self::DateDiff),
        ("date_component", Self::DateComponent),
        ("date", Self::Date),
    ];

    /// Resolves a function name, ASCII case-insensitively.
    ///
    /// Period-boundary names (`sow`, `start_of_week`) resolve through
    /// [`PERIOD_EDGE_LOOKUP`](crate::temporal::PERIOD_EDGE_LOOKUP), so the
    /// filter grammar and the template filters accept the same spellings.
    fn from_name(name: &str) -> Option<Self> {
        Self::FIXED_NAMES
            .iter()
            .find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
            .map(|&(_, function)| function)
            .or_else(|| {
                crate::temporal::resolve_period_edge(name).map(
                    |(period, edge)| match edge {
                        PeriodEdge::Start => Self::StartOf(period),
                        PeriodEdge::End => Self::EndOf(period),
                    },
                )
            })
    }

    /// Checks the argument count and the statically known literal arguments.
    ///
    /// # Errors
    ///
    /// - [`QueryBuilderError::Syntax`] if the argument count does not fit the
    ///   function, or a literal duration, duration unit, or date component
    ///   argument is not valid.
    fn validate_args(
        self,
        args: &[ValueExpr],
        input: &str,
        span: std::ops::Range<usize>,
    ) -> Result<(), QueryBuilderError> {
        let expected = match self {
            Self::Dur => {
                if args.len() != 1 {
                    Some("1 argument for dur()")
                } else if args
                    .first()
                    .is_some_and(ValueExpr::is_invalid_duration_literal)
                {
                    Some("valid duration")
                } else {
                    None
                }
            }
            Self::DateAdd => {
                if !matches!(args.len(), 2 | 3) {
                    Some("2 or 3 arguments for date_add()")
                } else if args.len() == 2
                    && args
                        .get(1)
                        .is_some_and(ValueExpr::is_invalid_duration_literal)
                {
                    Some("valid duration")
                } else if args.len() == 3
                    && args.get(2).is_some_and(ValueExpr::is_unrecognized_unit)
                {
                    Some("recognized duration unit")
                } else {
                    None
                }
            }
            Self::DateDiff => {
                if !matches!(args.len(), 2 | 3) {
                    Some("2 or 3 arguments for date_diff()")
                } else if args
                    .get(2)
                    .is_some_and(ValueExpr::is_unrecognized_unit)
                {
                    Some("recognized duration unit")
                } else {
                    None
                }
            }
            Self::DateComponent => {
                if args.len() != 2 {
                    Some("2 arguments for date_component()")
                } else if matches!(
                    args.get(1),
                    Some(ValueExpr::Literal(NoteFieldValue::String(name)))
                        if DateUnit::from_name(name).is_none()
                ) {
                    Some(
                        "valid component: year, month, day, hour, minute, \
                         second, weekday, week",
                    )
                } else {
                    None
                }
            }
            Self::StartOf(_) | Self::EndOf(_) => {
                if args.len() == 1 {
                    None
                } else {
                    Some("1 argument for bucketing function")
                }
            }
            Self::Date => {
                if matches!(args.len(), 1 | 2) {
                    None
                } else {
                    Some("1 or 2 arguments for date()")
                }
            }
        };
        if let Some(expected) = expected {
            return Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Filter,
                input,
                span,
                expected,
            )
            .into());
        }
        Ok(())
    }

    /// Evaluates the call over already evaluated argument values.
    ///
    /// Returns `None` when the arguments cannot satisfy a comparison.
    fn evaluate(self, args: &[NoteFieldValue]) -> Option<NoteFieldValue> {
        match self {
            Self::Dur => match args {
                [NoteFieldValue::Duration(duration)] => {
                    Some(NoteFieldValue::Duration(*duration))
                }
                [NoteFieldValue::String(text)] => Some(
                    NoteFieldValue::Duration(DurationValue::parse(text).ok()?),
                ),
                [_] => Some(NoteFieldValue::Null),
                _ => None,
            },
            Self::DateAdd => Self::date_add(args),
            Self::DateDiff => Self::date_diff(args),
            Self::DateComponent => Self::date_component(args),
            Self::StartOf(calendar_period) => {
                let zone = crate::temporal::local_zone().ok()?;
                Self::date_boundary(args, |date_point| {
                    date_point.start_of(calendar_period, &zone)
                })
            }
            Self::EndOf(calendar_period) => {
                let zone = crate::temporal::local_zone().ok()?;
                Self::date_boundary(args, |date_point| {
                    date_point.end_of(calendar_period, &zone)
                })
            }
            Self::Date => Self::date(args),
        }
    }

    fn date_add(args: &[NoteFieldValue]) -> Option<NoteFieldValue> {
        match args {
            [date, NoteFieldValue::Duration(duration)] => {
                ArithOp::Add.evaluate(date, NoteFieldValue::Duration(*duration))
            }
            [date, NoteFieldValue::String(text)] => ArithOp::Add.evaluate(
                date,
                NoteFieldValue::Duration(DurationValue::parse(text).ok()?),
            ),
            [date, NoteFieldValue::Number(n), NoteFieldValue::String(unit)] => {
                use num_traits::ToPrimitive as _;
                if matches!(date, NoteFieldValue::Null) {
                    return Some(NoteFieldValue::Null);
                }
                let unit = TimeUnit::from_spelling(unit)?;
                let n = (n.fract() == 0.0).then(|| n.to_i64()).flatten()?;
                let shifted = date
                    .as_date_point()?
                    .shift(n, unit, &crate::temporal::local_zone().ok()?)
                    .ok()?;
                Some(NoteFieldValue::from(shifted))
            }
            _ => None,
        }
    }

    fn date_diff(args: &[NoteFieldValue]) -> Option<NoteFieldValue> {
        let (date1, date2, unit) = match args {
            [date1, date2] => (date1, date2, TimeUnit::Day),
            [date1, date2, NoteFieldValue::String(unit)] => {
                (date1, date2, TimeUnit::from_spelling(unit)?)
            }
            _ => return None,
        };
        if matches!(date1, NoteFieldValue::Null)
            || matches!(date2, NoteFieldValue::Null)
        {
            return Some(NoteFieldValue::Null);
        }
        let diff = date1
            .as_date_point()?
            .diff(
                date2.as_date_point()?,
                unit,
                &crate::temporal::local_zone().ok()?,
            )
            .ok()?;
        Some(NoteFieldValue::Number(diff.to_f64()?))
    }

    fn date(args: &[NoteFieldValue]) -> Option<NoteFieldValue> {
        match args {
            [NoteFieldValue::Date(d)]
            | [NoteFieldValue::Date(d), NoteFieldValue::String(_)] => {
                Some(NoteFieldValue::Date(*d))
            }
            [NoteFieldValue::DateTime(dt)]
            | [NoteFieldValue::DateTime(dt), NoteFieldValue::String(_)] => {
                Some(NoteFieldValue::DateTime(*dt))
            }
            [NoteFieldValue::String(s)] => {
                Some(NoteFieldValue::from(DatePoint::parse(s).ok()?))
            }
            [NoteFieldValue::String(s), NoteFieldValue::String(fmt)] => {
                Some(NoteFieldValue::from(
                    DatePoint::parse_with_pattern(s, fmt).ok()?,
                ))
            }
            [NoteFieldValue::Null] | [NoteFieldValue::Null, _] => {
                Some(NoteFieldValue::Null)
            }
            _ => None,
        }
    }

    fn date_boundary(
        args: &[NoteFieldValue],
        op: impl FnOnce(DatePoint) -> Result<DatePoint, CalendarError>,
    ) -> Option<NoteFieldValue> {
        let date = args.first()?;
        if matches!(date, NoteFieldValue::Null) {
            return Some(NoteFieldValue::Null);
        }
        let point = match op(date.as_date_point()?) {
            Ok(point) => point,
            Err(
                CalendarError::OutOfRange
                | CalendarError::Zone {
                    ..
                },
            ) => {
                return None;
            }
        };
        Some(NoteFieldValue::from(point))
    }

    fn date_component(args: &[NoteFieldValue]) -> Option<NoteFieldValue> {
        use num_traits::ToPrimitive as _;
        let [date, NoteFieldValue::String(component)] = args else {
            return Some(NoteFieldValue::Null);
        };
        if matches!(date, NoteFieldValue::Null) {
            return Some(NoteFieldValue::Null);
        }
        let component = DateUnit::from_name(component)?;
        let zone = crate::temporal::local_zone().ok()?;
        Some(NoteFieldValue::Number(
            date.as_date_point()?.component(component, &zone).to_f64()?,
        ))
    }
}

/// Comparison operator parsed from filter syntax.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum CompareOp {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl CompareOp {
    fn is_satisfied_by(
        self,
        field: &QueryFieldValueRef<'_>,
        literal: &NoteFieldValue,
    ) -> bool {
        if matches!(self, Self::Eq | Self::Ne) {
            let equal = field.is_equal_to_literal(literal);
            return if matches!(self, Self::Eq) {
                equal
            } else {
                !equal
            };
        }
        let Some(value) = field.as_note_ref() else {
            return false;
        };
        if matches!(value, NoteFieldValueRef::Null)
            || matches!(literal, NoteFieldValue::Null)
        {
            return false;
        }
        match value.compare(&literal.as_ref()) {
            std::cmp::Ordering::Less => matches!(self, Self::Lt | Self::Le),
            std::cmp::Ordering::Equal => matches!(self, Self::Le | Self::Ge),
            std::cmp::Ordering::Greater => matches!(self, Self::Gt | Self::Ge),
        }
    }

    pub(super) fn is_satisfied_by_values(
        self,
        lhs: &NoteFieldValue,
        rhs: &NoteFieldValue,
    ) -> bool {
        if matches!(self, Self::Eq | Self::Ne) {
            let equal = lhs.as_ref().is_equal_to_literal(rhs);
            return if matches!(self, Self::Eq) {
                equal
            } else {
                !equal
            };
        }
        if matches!(lhs, NoteFieldValue::Null)
            || matches!(rhs, NoteFieldValue::Null)
        {
            return false;
        }
        match lhs.as_ref().compare(&rhs.as_ref()) {
            std::cmp::Ordering::Less => matches!(self, Self::Lt | Self::Le),
            std::cmp::Ordering::Equal => matches!(self, Self::Le | Self::Ge),
            std::cmp::Ordering::Greater => matches!(self, Self::Gt | Self::Ge),
        }
    }
}

impl TryFrom<&str> for CompareOp {
    type Error = ();

    fn try_from(spelling: &str) -> Result<Self, Self::Error> {
        match spelling {
            "==" => Ok(Self::Eq),
            "!=" => Ok(Self::Ne),
            ">=" => Ok(Self::Ge),
            "<=" => Ok(Self::Le),
            ">" => Ok(Self::Gt),
            "<" => Ok(Self::Lt),
            _ => Err(()),
        }
    }
}

/// Filter grammar adapter for [`parse_boolean_expr`].
struct FilterGrammar;

impl FilterGrammar {
    /// Parses a literal where a comparison or function argument requires a
    /// value.
    fn parse_literal_arg(
        input: &str,
        tokens: &mut SpannedTokenStream<FilterToken>,
    ) -> Result<NoteFieldValue, QueryBuilderError> {
        let spanned = tokens
            .expect_map(input, "a literal value", |token| {
                let spanned = token;
                match spanned.into_value() {
                    FilterToken::Literal(value) => Some(value),
                    _ => None,
                }
            })
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;
        Ok(spanned.into_value())
    }

    /// Parses a call argument list after the function name.
    fn parse_function_call(
        input: &str,
        tokens: &mut SpannedTokenStream<FilterToken>,
        name: &str,
    ) -> Result<FilterFunction, QueryBuilderError> {
        tokens
            .expect(
                input,
                TokenSpec::new(
                    &FilterToken::LParen,
                    "`(` after a function name",
                ),
            )
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;

        let field_ident = tokens
            .expect_map(input, "a field path", |token| {
                let spanned = token;
                match spanned.into_value() {
                    FilterToken::Ident(ident) => Some(ident),
                    _ => None,
                }
            })
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;
        let field = FieldPath::parse(field_ident.value())?;

        tokens
            .expect(
                input,
                TokenSpec::new(&FilterToken::Comma, "`,` after the field path"),
            )
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;

        let target = Self::parse_literal_arg(input, tokens)?;

        tokens
            .expect(
                input,
                TokenSpec::new(
                    &FilterToken::RParen,
                    "`)` after the function arguments",
                ),
            )
            .map_err(|e| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, e)
            })?;

        FilterFunction::build(name, field, target).ok_or_else(|| {
            QuerySyntaxError::unexpected_end(
                QueryDialect::Filter,
                input,
                0..name.len(),
                "`contains`",
            )
            .into()
        })
    }

    fn parse_registry_call(
        input: &str,
        tokens: &mut SpannedTokenStream<FilterToken>,
        name: &str,
        span: std::ops::Range<usize>,
    ) -> Result<ValueExpr, QueryBuilderError> {
        let function = QueryFunction::from_name(name).ok_or_else(|| {
            QuerySyntaxError::unexpected_end(
                QueryDialect::Filter,
                input,
                span.clone(),
                "recognized filter function",
            )
        })?;
        tokens.next();
        let mut args = Vec::new();
        if !tokens.peek_is_value(&FilterToken::RParen) {
            loop {
                args.push(Self::parse_value_expr(input, tokens)?);
                if !tokens.peek_is_value(&FilterToken::Comma) {
                    break;
                }
                tokens.next();
            }
        }
        tokens
            .expect(
                input,
                TokenSpec::new(
                    &FilterToken::RParen,
                    "`)` after function arguments",
                ),
            )
            .map_err(|err| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, err)
            })?;
        function.validate_args(&args, input, span)?;
        if function == QueryFunction::Dur
            && let [ValueExpr::Literal(value)] = args.as_slice()
            && let Some(value) = function.evaluate(std::slice::from_ref(value))
        {
            return Ok(ValueExpr::Literal(value));
        }
        Ok(ValueExpr::Call {
            function,
            args,
        })
    }

    fn parse_primary(
        input: &str,
        tokens: &mut SpannedTokenStream<FilterToken>,
    ) -> Result<ValueExpr, QueryBuilderError> {
        let negative = tokens.peek_is_value(&FilterToken::Minus);
        if negative {
            tokens.next();
        }
        let span = tokens.next_span(input);
        let token = tokens.next().ok_or_else(|| {
            QuerySyntaxError::unexpected_end(
                QueryDialect::Filter,
                input,
                span.clone(),
                "a field, literal, or function call",
            )
        })?;
        let expression = match token.into_value() {
            FilterToken::Literal(value) => {
                ValueExpr::Literal(ComparisonExpr::classify_literal(value))
            }
            FilterToken::Ident(name)
                if tokens.peek_is_value(&FilterToken::LParen) =>
            {
                Self::parse_registry_call(input, tokens, &name, span.clone())?
            }
            FilterToken::Ident(name) => {
                ValueExpr::Field(FieldPath::parse(&name)?)
            }
            _ => {
                return Err(QuerySyntaxError::unexpected_end(
                    QueryDialect::Filter,
                    input,
                    span,
                    "a field, literal, or function call",
                )
                .into());
            }
        };
        if !negative {
            return Ok(expression);
        }
        match expression {
            ValueExpr::Literal(NoteFieldValue::Number(number)) => {
                Ok(ValueExpr::Literal(NoteFieldValue::Number(-number)))
            }
            ValueExpr::Literal(NoteFieldValue::Duration(duration)) => {
                Ok(ValueExpr::Literal(NoteFieldValue::Duration(
                    duration.checked_neg().map_err(|_| {
                        QuerySyntaxError::unexpected_end(
                            QueryDialect::Filter,
                            input,
                            span.clone(),
                            "a representable negative duration",
                        )
                    })?,
                )))
            }
            _ => Err(QuerySyntaxError::unexpected_end(
                QueryDialect::Filter,
                input,
                span,
                "a number or duration after `-`",
            )
            .into()),
        }
    }

    /// Folds arithmetic in written order; comparison and logical tokens end it.
    fn parse_value_expr(
        input: &str,
        tokens: &mut SpannedTokenStream<FilterToken>,
    ) -> Result<ValueExpr, QueryBuilderError> {
        let mut lhs = Self::parse_primary(input, tokens)?;
        loop {
            let op = if tokens.peek_is_value(&FilterToken::Plus) {
                ArithOp::Add
            } else if tokens.peek_is_value(&FilterToken::Minus) {
                ArithOp::Sub
            } else if tokens.peek_is_value(&FilterToken::Star) {
                ArithOp::Mul
            } else {
                break;
            };
            tokens.next();
            let rhs_span = tokens.next_span(input);
            let rhs = Self::parse_primary(input, tokens)?;
            if lhs.is_number_only() && rhs.is_number_only() {
                return Err(QuerySyntaxError::unexpected_end(
                    QueryDialect::Filter,
                    input,
                    rhs_span,
                    "a field, date, or duration operand (arithmetic on number \
                     literals alone is not supported; quote date literals)",
                )
                .into());
            }
            lhs = ValueExpr::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }
}

impl AtomParser for FilterGrammar {
    type Atom = FilterAtom;
    type Token = FilterToken;

    fn control(&self, token: &Self::Token) -> Option<LogicalControl> {
        match token {
            FilterToken::Logical(operator) => {
                Some(LogicalControl::Operator(*operator))
            }
            FilterToken::Not => Some(LogicalControl::Not),
            FilterToken::LParen => Some(LogicalControl::LeftParen),
            FilterToken::RParen => Some(LogicalControl::RightParen),
            FilterToken::Comma
            | FilterToken::Plus
            | FilterToken::Minus
            | FilterToken::Star
            | FilterToken::Op(_)
            | FilterToken::Literal(_)
            | FilterToken::DateLiteral(_)
            | FilterToken::Ident(_) => None,
        }
    }

    /// Parses a `contains` predicate or compares two value expressions.
    fn parse_atom(
        &self,
        input: &str,
        tokens: &mut SpannedTokenStream<Self::Token>,
    ) -> Result<Self::Atom, QueryBuilderError> {
        if matches!(
            tokens.peek().map(Spanned::value),
            Some(FilterToken::Ident(name)) if name.eq_ignore_ascii_case("contains")
        ) {
            let name = tokens
                .expect_map(input, "`contains`", |token| {
                    match token.into_value() {
                        FilterToken::Ident(name) => Some(name),
                        _ => None,
                    }
                })
                .map_err(|err| {
                    QuerySyntaxError::from_lex(QueryDialect::Filter, input, err)
                })?;
            let name = name.into_value();
            return Self::parse_function_call(input, tokens, &name)
                .map(FilterAtom::Function);
        }
        let lhs = Self::parse_value_expr(input, tokens)?;
        let operator = tokens
            .expect_map(input, "a comparison operator", |token| {
                match token.into_value() {
                    FilterToken::Op(op) => Some(op),
                    _ => None,
                }
            })
            .map_err(|err| {
                QuerySyntaxError::from_lex(QueryDialect::Filter, input, err)
            })?;
        let rhs = Self::parse_value_expr(input, tokens)?;
        Ok(FilterAtom::Comparison(ComparisonExpr::new(
            lhs,
            *operator.value(),
            rhs,
        )))
    }

    fn syntax_error(
        &self,
        input: &str,
        span: std::ops::Range<usize>,
        expected: &'static str,
    ) -> QuerySyntaxError {
        QuerySyntaxError::unexpected_end(
            QueryDialect::Filter,
            input,
            span,
            expected,
        )
    }
}

/// Filter expression lexer tokens.
#[derive(Clone, Debug, PartialEq, Logos)]
#[logos(skip r"[ \t\n\r\f]+")]
enum FilterToken {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[regex(
        "&&|and|\\|\\||or",
        |lex| LogicalOp::try_from(lex.slice()),
        ignore(case)
    )]
    Logical(LogicalOp),
    #[token("!")]
    #[token("not", ignore(case))]
    Not,
    #[regex("==|!=|>=|<=|>|<", |lex| CompareOp::try_from(lex.slice()))]
    Op(CompareOp),
    #[regex(r#""([^"\\]|\\.)*"|'([^'\\]|\\.)*'"#, string_callback)]
    #[token("true", |_| NoteFieldValue::Bool(true), priority = 3)]
    #[token("false", |_| NoteFieldValue::Bool(false), priority = 3)]
    #[token("null", |_| NoteFieldValue::Null, priority = 3)]
    #[token("Null", |_| NoteFieldValue::Null, priority = 3)]
    Literal(NoteFieldValue),
    /// A bare ISO date or date-time shape, validated while tokenizing.
    #[regex(
        r"[0-9]{4}-[0-9]{2}(-[0-9]{2}(T[0-9]{2}:[0-9]{2}(:[0-9]{2}(\.[0-9]+)?)?(Z|[+-][0-9]{2}:[0-9]{2})?)?)?",
        |lex| lex.slice().to_owned()
    )]
    DateLiteral(String),
    #[regex(r#"[^\s()'",=!<>&|+*-]+"#, |lex| lex.slice().to_owned())]
    Ident(String),
}

/// Decodes a quoted string literal into a string field value.
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "logos Callback trait requires &mut Lexer"
)]
fn string_callback(lex: &mut Lexer<'_, FilterToken>) -> NoteFieldValue {
    NoteFieldValue::String(lexical_unquote(lex.slice()))
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use super::FilterExpr;
    use crate::{
        IndexerService,
        query::{QueryError, *},
    };

    fn rows_for_files(files: &[(&str, &str)]) -> QuerySet {
        let index = crate::test_support::build_test_index(files);
        QueryService::new("class")
            .run(&index, QueryBuilder::pages(SourceSelector::All))
    }

    fn rows_for(content: &str) -> QuerySet {
        rows_for_files(&[("note.md", content)])
    }

    fn rated_rows() -> QuerySet {
        rows_for_files(&[
            ("low.md", "---\nrating: 3\nstatus: draft\n---"),
            ("high.md", "---\nrating: 7\nstatus: done\n---"),
            ("unrated.md", "---\nstatus: done\n---"),
        ])
    }

    fn names(rows: &QuerySet) -> Vec<String> {
        rows.iter().map(|row| row.file().name().as_str().to_owned()).collect()
    }

    mod parse {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::no_operator("rating")]
        #[case::empty_field(" > 5")]
        #[case::empty_value("rating >")]
        #[case::unknown_function("unknown(tags, \"#book\")")]
        #[case::function_missing_target("contains(tags)")]
        fn rejects_malformed_expressions(#[case] expr: &str) {
            let rows = rated_rows();

            assert!(matches!(
                rows.filter(expr),
                Err(QueryError::Builder(QueryBuilderError::Syntax(_)))
            ));
        }

        #[rstest]
        #[case::empty("")]
        #[case::trailing_operator("rating > 5 and")]
        #[case::unmatched_left_parenthesis("(rating > 5")]
        #[case::unmatched_right_parenthesis("rating > 5)")]
        #[case::adjacent_expressions("rating > 5 status == \"done\"")]
        fn rejects_incomplete_boolean_logic(#[case] expr: &str) {
            assert!(matches!(
                FilterExpr::parse(expr),
                Err(QueryBuilderError::Syntax(_))
            ));
        }

        #[rstest]
        #[case("rating > NaN")]
        #[case("rating > inf")]
        #[case("rating > -inf")]
        fn rejects_non_finite_numeric_literals(#[case] expr: &str) {
            assert!(matches!(
                FilterExpr::parse(expr),
                Err(QueryBuilderError::Syntax(_))
            ));
        }

        #[test]
        fn rejects_malformed_field_path_in_expression() {
            let rows = rated_rows();

            assert_eq!(
                rows.filter("file.zzzz == 1"),
                Err(QueryError::Builder(QueryBuilderError::FieldPath(
                    FieldPathError::new("file.zzzz", None)
                )))
            );
        }

        #[test]
        fn rejects_malformed_field_path_in_function() {
            assert_eq!(
                FilterExpr::parse("contains(file.zzzz, \"x\")"),
                Err(QueryBuilderError::FieldPath(FieldPathError::new(
                    "file.zzzz",
                    None
                )))
            );
        }
    }

    mod filter {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;
        use crate::temporal::test_support::ZoneGuard;

        #[rstest]
        #[case::greater_than("rating > 5", &["high"])]
        #[case::greater_or_equal("rating >= 7", &["high"])]
        #[case::less_than("rating < 5", &["low"])]
        #[case::less_or_equal("rating <= 3", &["low"])]
        #[case::numeric_equal("rating == 7", &["high"])]
        #[case::string_equal("status == \"done\"", &["high", "unrated"])]
        #[case::single_quoted_string_equal("status == 'done'", &["high", "unrated"])]
        #[case::string_not_equal("status != \"done\"", &["low"])]
        fn keeps_only_matching_records(
            #[case] expr: &str,
            #[case] expected: &[&str],
        ) {
            let rows = rated_rows();

            let filtered = rows.filter(expr).expect("valid filter");

            assert_eq!(names(&filtered), expected);
        }

        #[test]
        fn missing_field_never_matches_equality_or_ordering() {
            let rows = rated_rows();

            let filtered = rows.filter("rating > 0").expect("valid filter");

            assert_eq!(names(&filtered), ["high", "low"]);
        }

        #[test]
        fn missing_field_matches_not_equal() {
            let rows = rated_rows();

            let filtered = rows.filter("rating != 7").expect("valid filter");

            assert_eq!(names(&filtered), ["low", "unrated"]);
        }

        #[test]
        fn cross_kind_ordering_follows_canonical_rank() {
            let rows = rated_rows();

            // Text (rank 5) is greater than Number 5 (rank 2).
            let filtered =
                rows.clone().filter("status > 5").expect("valid filter");
            assert_eq!(names(&filtered), ["high", "low", "unrated"]);

            // Number 5 is not greater than Text, so status < 5 matches nothing.
            let below = rows.filter("status < 5").expect("valid filter");
            assert!(below.is_empty());
        }

        #[test]
        fn mixed_kind_column_ordering_matches_sort_order() {
            let rows = rows_for_files(&[
                ("num_low.md", "---\nrating: 3\n---"),
                ("num_high.md", "---\nrating: 7\n---"),
                ("text.md", "---\nrating: gold\n---"),
            ]);

            // In canonical rank: Number < Text.
            // Rating > 5 matches the numeric row (7 > 5) and the text row
            // ("gold" > 5 by rank).
            let filtered =
                rows.clone().filter("rating > 5").expect("valid filter");
            assert_eq!(names(&filtered), ["num_high", "text"]);

            // Sorting rating ascending yields the numeric rows (3, 7), then
            // text ("gold").
            let sorted = rows.sort("rating", false).expect("valid sort");
            assert_eq!(names(&sorted), ["num_low", "num_high", "text"]);
        }

        #[test]
        fn chains_across_multiple_filter_calls() {
            let rows = rated_rows();

            let filtered = rows
                .filter("status == \"done\"")
                .expect("valid filter")
                .filter("rating >= 7")
                .expect("valid filter");

            assert_eq!(names(&filtered), ["high"]);
        }

        #[test]
        fn equal_matches_a_date_field_against_a_string_literal() {
            let rows = rows_for("---\ndue: 2026-01-01\n---");

            let filtered =
                rows.filter("due == \"2026-01-01\"").expect("valid filter");

            assert_eq!(filtered.len(), 1);
        }

        fn dated_rows() -> QuerySet {
            rows_for_files(&[
                ("has.md", "---\ndue: 2026-01-01\n---"),
                ("other.md", "---\ndue: 2026-06-01\n---"),
                ("nodue.md", "---\nstatus: draft\n---"),
            ])
        }

        #[rstest]
        #[case::equal("due == 2026-01-01", &["has"])]
        #[case::less("due < 2026-03-01", &["has"])]
        #[case::greater_equal("due >= 2026-03-01", &["other"])]
        fn evaluates_unquoted_iso_date_literals(
            #[case] expr: &str,
            #[case] expected: &[&str],
        ) {
            let rows = dated_rows();

            let filtered = rows.filter(expr).expect("valid filter");

            assert_eq!(names(&filtered), expected);
        }

        #[rstest]
        #[case::date("due != 2026-01-01", "due != \"2026-01-01\"")]
        #[case::datetime(
            "due < 2026-07-29T14:30:00Z",
            "due < \"2026-07-29T14:30:00Z\""
        )]
        fn unquoted_iso_literal_matches_its_quoted_form(
            #[case] unquoted: &str,
            #[case] quoted: &str,
        ) {
            let rows = dated_rows();

            let bare = rows.clone().filter(unquoted).expect("valid filter");
            let string = rows.filter(quoted).expect("valid filter");

            assert_eq!(names(&bare), names(&string));
        }

        #[rstest]
        #[case::unpadded_date("due == 2026-1-1")]
        #[case::number_arithmetic("1 + 2 == 3")]
        fn rejects_arithmetic_on_number_literals_alone(#[case] expr: &str) {
            assert!(matches!(
                FilterExpr::parse(expr),
                Err(QueryBuilderError::Syntax(_))
            ));
        }

        #[rstest]
        #[case::impossible_day("due != 2026-02-30")]
        #[case::month_out_of_range("due != 2026-13")]
        #[case::month_and_day_out_of_range("due == 2026-13-45")]
        #[case::impossible_time("due == 2026-01-01T25:00")]
        fn rejects_unquoted_date_shapes_that_are_not_dates(#[case] expr: &str) {
            let err = FilterExpr::parse(expr).expect_err("not a date");

            assert!(matches!(err, QueryBuilderError::Syntax(_)), "{err}");
        }

        #[rstest]
        #[case::duration_times_number("dur(\"1d\") * 2 == dur(\"2d\")")]
        #[case::number_times_duration("2 * dur(\"1d\") == dur(\"2d\")")]
        fn accepts_duration_scaling_by_a_number(#[case] expr: &str) {
            assert!(FilterExpr::parse(expr).is_ok());
        }

        #[test]
        fn equal_null_matches_rows_with_a_null_field() {
            let rows = rated_rows();

            let filtered = rows.filter("rating == null").expect("valid filter");

            assert_eq!(names(&filtered), ["unrated"]);
        }

        #[test]
        fn not_equal_null_matches_rows_with_a_non_null_field() {
            let rows = rated_rows();

            let filtered = rows.filter("rating != null").expect("valid filter");

            assert_eq!(names(&filtered), ["high", "low"]);
        }

        #[test]
        fn r_where_alias_filters_records_identically_to_filter() {
            let rows = rated_rows();

            let filtered = rows.r#where("rating >= 7").expect("valid where");

            assert_eq!(names(&filtered), ["high"]);
        }

        #[test]
        fn evaluates_duration_comparisons_temporally() {
            let rows = rows_for_files(&[
                ("a.md", "---\nspent: 1h\n---"),
                ("b.md", "---\nspent: 30m\n---"),
            ]);
            let filtered =
                rows.filter("spent > \"30m\"").expect("valid filter");
            assert_eq!(names(&filtered), ["a"]);
        }

        #[test]
        fn matches_duration_equality_across_differing_spellings() {
            let rows = rows_for_files(&[
                ("a.md", "---\nspent: 90m\n---"),
                ("b.md", "---\nspent: 45m\n---"),
            ]);
            let filtered =
                rows.filter("spent == \"1h 30m\"").expect("valid filter");
            assert_eq!(names(&filtered), ["a"]);
        }

        #[test]
        fn evaluates_a_date_only_literal_at_local_midnight() {
            // A naive literal uses the reader's zone. In the pinned UTC+2 zone,
            // local midnight falls at December 31 at 22:00 UTC through January
            // 1 at 22:00 UTC, which still separates the fixture instants.
            ZoneGuard::set("Etc/GMT-2");

            let temp = tempfile::tempdir().expect("create temp dir");
            let p1 = temp.path().join("jan1.md");
            let p2 = temp.path().join("jan2.md");
            fs::write(&p1, "# Jan 1\n").expect("write jan1.md");
            fs::write(&p2, "# Jan 2\n").expect("write jan2.md");

            let t1 = std::time::UNIX_EPOCH
                + std::time::Duration::from_mins(29_454_630);
            let t2 = std::time::UNIX_EPOCH
                + std::time::Duration::from_hours(490_928);

            let f1 =
                fs::File::options().write(true).open(&p1).expect("open p1");
            f1.set_times(std::fs::FileTimes::new().set_modified(t1))
                .expect("set mtime p1");
            drop(f1);

            let f2 =
                fs::File::options().write(true).open(&p2).expect("open p2");
            f2.set_times(std::fs::FileTimes::new().set_modified(t2))
                .expect("set mtime p2");
            drop(f2);

            let index = Arc::new(
                IndexerService::for_tests(temp.path())
                    .build()
                    .expect("build index"),
            );
            let rows = QueryService::new("class")
                .run(&index, QueryBuilder::pages(SourceSelector::All));

            let filtered = rows
                .filter(
                    "file.mtime >= \"2026-01-01\" and file.mtime < \
                     \"2026-01-02\"",
                )
                .expect("valid filter");

            assert_eq!(names(&filtered), ["jan1"]);
        }

        #[test]
        fn matches_calendar_days_via_mdate() {
            // This field is a local calendar date derived from file
            // modification time, so the expectation only holds in
            // the pinned zone.
            ZoneGuard::set("UTC");

            let temp = tempfile::tempdir().expect("create temp dir");
            let p1 = temp.path().join("jan1.md");
            let p2 = temp.path().join("jan2.md");
            fs::write(&p1, "# Jan 1\n").expect("write jan1.md");
            fs::write(&p2, "# Jan 2\n").expect("write jan2.md");

            let t1 = std::time::UNIX_EPOCH
                + std::time::Duration::from_mins(29_454_630);
            let t2 = std::time::UNIX_EPOCH
                + std::time::Duration::from_hours(490_928);

            let f1 =
                fs::File::options().write(true).open(&p1).expect("open p1");
            f1.set_times(std::fs::FileTimes::new().set_modified(t1))
                .expect("set mtime p1");
            drop(f1);

            let f2 =
                fs::File::options().write(true).open(&p2).expect("open p2");
            f2.set_times(std::fs::FileTimes::new().set_modified(t2))
                .expect("set mtime p2");
            drop(f2);

            let index = Arc::new(
                IndexerService::for_tests(temp.path())
                    .build()
                    .expect("build index"),
            );
            let rows = QueryService::new("class")
                .run(&index, QueryBuilder::pages(SourceSelector::All));

            let filtered = rows
                .filter("file.mdate == \"2026-01-01\"")
                .expect("valid filter");

            assert_eq!(names(&filtered), ["jan1"]);
        }

        #[test]
        fn and_combination_keeps_only_records_matching_both_sides() {
            let rows = rated_rows();

            let filtered = rows
                .filter("rating > 5 AND status == \"done\"")
                .expect("valid filter");

            assert_eq!(names(&filtered), ["high"]);
        }

        #[test]
        fn or_combination_keeps_records_matching_either_side() {
            let rows = rated_rows();

            let filtered = rows
                .filter("rating == 3 OR status == \"done\"")
                .expect("valid filter");

            assert_eq!(names(&filtered), ["high", "low", "unrated"]);
        }

        #[test]
        fn not_combination_reverses_the_matching_condition() {
            let rows = rated_rows();

            let filtered =
                rows.filter("NOT status == \"done\"").expect("valid filter");

            assert_eq!(names(&filtered), ["low"]);
        }

        #[test]
        fn default_boolean_precedence_evaluates_correctly() {
            let rows = rated_rows();

            let filtered = rows
                .filter(
                    "status == \"done\" OR rating == 3 AND status == \"draft\"",
                )
                .expect("valid filter");

            assert_eq!(names(&filtered), ["high", "low", "unrated"]);
        }

        #[test]
        fn nested_parentheses_override_precedence() {
            let rows = rated_rows();

            let nested = rows
                .filter(
                    "(rating > 5 OR status == \"draft\") AND NOT rating == 3",
                )
                .expect("valid filter");

            assert_eq!(names(&nested), ["high"]);
        }

        #[test]
        fn logical_op_spellings_do_not_swallow_identifier_prefixes() {
            let rows = rows_for("---\norder: 5\nandrew: 3\n---");

            let lower = rows
                .clone()
                .filter("order == 5 and andrew == 3")
                .expect("valid filter: lowercase and");
            assert_eq!(lower.len(), 1);

            let symbolic_and = rows
                .clone()
                .filter("order == 5 && andrew == 3")
                .expect("valid filter: &&");
            assert_eq!(symbolic_and.len(), 1);

            let symbolic_or = rows
                .clone()
                .filter("order == 999 || andrew == 3")
                .expect("valid filter: ||");
            assert_eq!(symbolic_or.len(), 1);

            let lower_or = rows
                .clone()
                .filter("order == 999 or andrew == 3")
                .expect("valid filter: lowercase or");
            assert_eq!(lower_or.len(), 1);

            // Regression: fields named `order`/`andrew` must stay whole
            // identifiers, not logical-op prefixes.
            let ident_prefix =
                rows.filter("order == 5").expect("valid filter: bare field");
            assert_eq!(ident_prefix.len(), 1);
        }
    }

    mod contains {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn contains_matches_tags_by_prefix_hierarchy() {
            let rows = rows_for_files(&[
                (
                    "book.md",
                    "---\ntitle: Rust Handbook\n---\nFiled under #book/fiction",
                ),
                (
                    "article.md",
                    "---\ntitle: Async Guide\n---\nFiled under #article",
                ),
            ]);

            let filtered =
                rows.filter("contains(tags, \"#book\")").expect("valid filter");

            assert_eq!(names(&filtered), ["book"]);
        }

        #[test]
        fn contains_matches_string_fields_by_substring() {
            let rows = rows_for_files(&[
                (
                    "book.md",
                    "---\ntitle: Rust Handbook\n---\nFiled under #book/fiction",
                ),
                (
                    "article.md",
                    "---\ntitle: Async Guide\n---\nFiled under #article",
                ),
            ]);

            let filtered = rows
                .filter("contains(title, \"Async\")")
                .expect("valid filter");

            assert_eq!(names(&filtered), ["article"]);
        }

        #[test]
        fn contains_distinguishes_list_values_from_tag_hierarchy() {
            let rows = rows_for_files(&[
                (
                    "handbook.md",
                    "---\ncategories: [handbook]\n---\nTagged #bookworm",
                ),
                (
                    "book.md",
                    "---\ncategories: [book]\n---\nTagged #book/fiction",
                ),
            ]);

            let category_match = rows
                .clone()
                .filter("contains(categories, \"book\")")
                .expect("valid filter");
            assert_eq!(names(&category_match), ["book"]);

            let tag_match =
                rows.filter("contains(tags, \"#book\")").expect("valid filter");
            assert_eq!(names(&tag_match), ["book"]);
        }
    }

    mod classify_literal {
        use super::super::{ComparisonExpr, NoteFieldValue};

        #[test]
        fn promotes_iso_date_string_to_date_value() {
            let lit = NoteFieldValue::String("2026-07-29".to_owned());
            let classified = ComparisonExpr::classify_literal(lit);
            assert!(matches!(classified, NoteFieldValue::Date(_)));
        }

        #[test]
        fn promotes_iso_datetime_string_to_datetime_value() {
            let lit = NoteFieldValue::String("2026-07-29T14:30:00".to_owned());
            let classified = ComparisonExpr::classify_literal(lit);
            assert!(matches!(classified, NoteFieldValue::DateTime(_)));
        }

        #[test]
        fn promotes_duration_string_to_duration_value() {
            let lit = NoteFieldValue::String("1h30m".to_owned());
            let classified = ComparisonExpr::classify_literal(lit);
            assert!(matches!(classified, NoteFieldValue::Duration(_)));
        }

        #[test]
        fn preserves_plain_string_literal() {
            let lit = NoteFieldValue::String("active".to_owned());
            let classified = ComparisonExpr::classify_literal(lit);
            assert_eq!(classified, NoteFieldValue::String("active".to_owned()));
        }

        #[test]
        fn preserves_non_string_literals_unmodified() {
            assert_eq!(
                ComparisonExpr::classify_literal(NoteFieldValue::Number(42.0)),
                NoteFieldValue::Number(42.0)
            );
            assert_eq!(
                ComparisonExpr::classify_literal(NoteFieldValue::Bool(true)),
                NoteFieldValue::Bool(true)
            );
            assert_eq!(
                ComparisonExpr::classify_literal(NoteFieldValue::Null),
                NoteFieldValue::Null
            );
        }
    }

    #[test]
    fn query_date_arithmetic_matches_calendar_application_and_written_order() {
        let rows = rows_for_files(&[
            ("jan.md", "---\nwhen: 2026-01-31\n---"),
            ("feb.md", "---\nwhen: 2026-02-15\n---"),
        ]);
        let shifted = rows
            .clone()
            .filter("when + dur(\"1 month\") == \"2026-02-28\"")
            .expect("valid filter");
        assert_eq!(names(&shifted), vec!["jan"]);
        let compound = rows
            .filter("date_add(when, dur(\"1mo 1d\")) == \"2026-03-01\"")
            .expect("valid filter");
        assert_eq!(names(&compound), vec!["jan"]);
    }

    #[test]
    fn query_temporal_values_support_difference_components_and_scaling() {
        let rows = rows_for_files(&[
            ("jan.md", "---\nwhen: 2026-01-31\n---"),
            ("feb.md", "---\nwhen: 2026-02-15\n---"),
        ]);
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("when - \"2026-01-31\" == dur(\"15d\")")
                    .expect("date difference")
            ),
            vec!["feb"],
        );
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("date_diff(\"2026-01-31\", when, \"days\") == 15")
                    .expect("forward date_diff")
            ),
            vec!["feb"],
        );
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("date_diff(when, \"2026-01-31\", \"days\") == -15")
                    .expect("backward date_diff")
            ),
            vec!["feb"],
        );
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("date_component(when, \"month\") == 2")
                    .expect("date_component")
            ),
            vec!["feb"],
        );
        assert_eq!(
            names(
                &rows
                    .filter(
                        "date_add(when, dur(\"1 day\") * -2) == \"2026-02-13\""
                    )
                    .expect("duration scaling")
            ),
            vec!["feb"],
        );
    }
    #[test]
    fn query_arithmetic_uses_written_order_and_handles_null_and_wrong_types() {
        let rows = rows_for("---\nwhen: 2026-01-31\n---");
        for expression in [
            "dur(\"1d\") + dur(\"2d\") * 2 == dur(\"6d\")",
            "dur(\"5d\") - dur(\"2d\") == dur(\"3d\")",
            "when - dur(\"1d\") == \"2026-01-30\"",
            "date_add(when, 1, \"months\") == \"2026-02-28\"",
            "date_add(when, 1, \"mo\") == \"2026-02-28\"",
            "date_component(when, \"week\") == 5",
            "date_component(when, \"weekday\") == 6",
            "date_add(null, dur(\"1d\")) == null",
            "date_add(3, dur(\"1d\")) == null",
        ] {
            assert_eq!(
                names(&rows.clone().filter(expression).expect(expression)),
                vec!["note"],
                "{expression}"
            );
        }
        for expression in [
            "dur(\"1d\") + dur(\"2d\") * 2 == dur(\"5d\")",
            "date_add(when, 0.5, \"months\") == \"2026-01-31\"",
            "date_add(missing, dur(\"1d\")) == \"2026-02-01\"",
        ] {
            assert!(
                rows.clone().filter(expression).expect(expression).is_empty(),
                "{expression}"
            );
        }
    }

    #[test]
    fn query_registry_rejects_invalid_static_arguments_and_unlisted_value_forms()
     {
        for expression in [
            "date_component(\"2026-01-31\", \"week99\") == 5",
            "date_add(\"2026-01-31\", 1, \"fortnights\") == \"2026-02-01\"",
            "date_diff(\"2026-01-31\", \"2026-02-01\", \"nonsense\") == 1",
            "dur(\"not a duration\") == dur(\"1d\")",
            "unknown(\"2026-01-31\") == 1",
            "date_add(\"2026-01-31\", 1, \"day\" and true) == null",
            "date_add((\"2026-01-31\"), dur(\"1d\")) == null",
        ] {
            assert!(FilterExpr::parse(expression).is_err(), "{expression}");
        }
    }

    #[test]
    fn query_sow_returns_iso_start_of_week() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(&rows.filter("sow(when) == \"2026-01-26\"").expect("sow")),
            vec!["sun"],
        );
    }

    #[test]
    fn query_eow_returns_iso_end_of_week() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(&rows.filter("eow(when) == \"2026-02-01\"").expect("eow")),
            vec!["sun"],
        );
    }

    #[test]
    fn query_som_returns_first_day_of_month() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("som(when) == \"2026-07-01\"")
                    .expect("som")
            ),
            vec!["wed"],
        );
        assert_eq!(
            names(
                &rows
                    .filter("start_of_month(when) == \"2026-07-01\"")
                    .expect("start_of_month")
            ),
            vec!["wed"],
        );
    }

    #[test]
    fn query_eom_returns_last_day_of_month() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(
                &rows
                    .clone()
                    .filter("eom(when) == \"2026-07-31\"")
                    .expect("eom")
            ),
            vec!["wed"],
        );
        assert_eq!(
            names(
                &rows
                    .filter("end_of_month(when) == \"2026-07-31\"")
                    .expect("end_of_month")
            ),
            vec!["wed"],
        );
    }

    #[test]
    fn query_soy_returns_first_day_of_year() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(&rows.filter("soy(when) == \"2026-01-01\"").expect("soy")),
            vec!["sun", "wed"],
        );
    }

    #[test]
    fn query_eoy_returns_last_day_of_year() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(&rows.filter("eoy(when) == \"2026-12-31\"").expect("eoy")),
            vec!["sun", "wed"],
        );
    }

    #[test]
    fn query_date_function_evaluates_parse_with_format() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(
                &rows
                    .filter("date(\"29/07/2026\", \"%d/%m/%Y\") == when")
                    .expect("date parse_with")
            ),
            vec!["wed"],
        );
    }

    #[test]
    fn query_date_function_evaluates_iso_literal() {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);
        assert_eq!(
            names(
                &rows
                    .filter("date(\"2026-07-29\") == when")
                    .expect("date classify")
            ),
            vec!["wed"],
        );
    }

    #[rstest::rstest]
    #[case::start_of_week("start_of_week(when) == sow(when)")]
    #[case::end_of_week("end_of_week(when) == eow(when)")]
    #[case::start_of_year("start_of_year(when) == soy(when)")]
    #[case::end_of_year("end_of_year(when) == eoy(when)")]
    fn query_long_period_names_match_their_short_aliases(#[case] expr: &str) {
        let rows = rows_for_files(&[
            ("sun.md", "---\nwhen: 2026-02-01\n---"),
            ("wed.md", "---\nwhen: 2026-07-29\n---"),
        ]);

        let filtered = rows.filter(expr).expect("recognized alias");

        assert_eq!(names(&filtered), vec!["sun", "wed"]);
    }
}
