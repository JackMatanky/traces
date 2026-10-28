//! Registers duration rendering filters for templates.
//!
//! [`DurationOps`] adds the two compound-format aliases for [`DurationValue`]:
//! `duration_format` and `durationformat` (e.g. `"3 days, 4 hours"`).
//!
//! Each filter is a plain function registered through
//! [`Environment::add_filter`], not an [`Object`], because no filter carries
//! shared state.
//!
//! [`DurationValue`]: crate::DurationValue
//! [`Object`]: minijinja::value::Object

use minijinja::{Environment, Error, ErrorKind, value::Value};

use super::error::{TemplateEngineResult, invalid_operation};
use crate::DurationValue;

/// Registration namespace for the stateless duration filters.
pub(super) struct DurationOps;

impl DurationOps {
    /// Registers the duration filters with `env`.
    ///
    /// This is an associated function because [`DurationOps`] carries no state.
    #[inline]
    pub(super) fn register(env: &mut Environment<'static>) {
        env.add_filter("duration_format", duration_format);
        env.add_filter("durationformat", duration_format);
    }
}

/// Renders a duration value or string in human-readable compound format.
///
/// Syntax: `{{ value | duration_format }}` or `{{ value | durationformat }}`
/// (e.g. `"3 days, 4 hours"`).
///
/// # Errors
///
/// Returns a [`minijinja::Error`] with kind
/// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
/// `value` cannot be parsed or converted into a [`DurationValue`].
fn duration_format(value: Value) -> TemplateEngineResult<String> {
    let dv = if let Some(s) = value.as_str() {
        DurationValue::parse(s).map_err(|source| {
            invalid_operation(format!("invalid duration {s:?}"), source)
        })?
    } else if let Ok(f) = f64::try_from(value) {
        DurationValue::from_nominal_seconds(f).map_err(|source| {
            invalid_operation(
                format!("out-of-range duration seconds {f}"),
                source,
            )
        })?
    } else {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "durationformat expects a duration string or number",
        ));
    };
    Ok(dv.to_compound_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Environment<'static> {
        let mut env = Environment::new();
        DurationOps::register(&mut env);
        env
    }

    mod durationformat {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::compound_days_hours("3d 4h", "3 days, 4 hours")]
        #[case::single_day("1d", "1 day")]
        #[case::single_hour("1h", "1 hour")]
        #[case::minutes_seconds("1m 30s", "1 minute, 30 seconds")]
        #[case::zero_duration("0s", "0 seconds")]
        #[case::negative_duration("-3d 4h", "-3 days, 4 hours")]
        #[case::fractional_seconds("1.5s", "2 seconds")]
        #[case::negative_fractional_seconds("-1.5s", "-2 seconds")]
        fn formats_compound_durations(
            #[case] input: &str,
            #[case] expected: &str,
        ) {
            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ "{input}" | durationformat }}}}"#),
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn formats_duration_from_numeric_seconds() {
            let rendered = env()
                .render_str(
                    r"{{ 86400 | durationformat }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1 day");
        }

        #[test]
        fn duration_format_is_an_alias_of_durationformat() {
            let rendered = env()
                .render_str(
                    r"{{ 90 | duration_format }}|{{ 90 | durationformat }}",
                    minijinja::context!(),
                )
                .expect("render succeeds");

            assert_eq!(rendered, "1 minute, 30 seconds|1 minute, 30 seconds");
        }

        #[test]
        fn rejects_invalid_duration_value() {
            let error = env()
                .render_str(
                    r#"{{ "not a duration" | durationformat }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid duration fails");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }
}
