//! Registers numeric filters for templates.
//!
//! [`NumOps`] registers four filters that carry no shared state:
//!
//! - `ceil`
//! - `floor`
//! - `sqrt`
//! - `num_format`
//!
//! Each filter is a function registered with [`Environment::add_filter`], not
//! an [`Object`]. The `num_format` name avoids minijinja's built-in `format`
//! filter.
//!
//! Minijinja converts integer and float [`Value`] arguments to `f64`. It
//! returns its standard argument-type error for other values.
//!
//! [`Object`]: minijinja::value::Object
//! [`Value`]: minijinja::value::Value

use minijinja::{Environment, Error, ErrorKind};

use super::error::TemplateEngineResult;

/// Provides the registration entry point for the stateless numeric filters.
pub(super) struct NumOps;

impl NumOps {
    /// Registers the numeric filters with `env`.
    ///
    /// The associated function needs no [`NumOps`] state.
    #[inline]
    pub(super) fn register(env: &mut Environment<'static>) {
        env.add_filter("ceil", |value: f64| value.ceil());
        env.add_filter("floor", |value: f64| value.floor());
        env.add_filter("sqrt", sqrt);
        env.add_filter("num_format", num_format);
    }
}

/// Computes the square root of `value`.
///
/// [`f64::sqrt`] returns `NaN` for a negative value. This filter returns a
/// template error for a negative value instead of rendering `NaN`. It accepts
/// `-0.0` because IEEE 754 makes `-0.0 < 0.0` false.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if `value` is negative.
///
/// [`ErrorKind::InvalidOperation`]: minijinja::ErrorKind::InvalidOperation
fn sqrt(value: f64) -> TemplateEngineResult<f64> {
    if value < 0.0 {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("sqrt of negative number: {value}"),
        ));
    }
    Ok(value.sqrt())
}

/// Formats `value` with `decimals` decimal places using Rust's `{:.N$}` format.
///
/// # Arguments
///
/// - `value`: the number to format.
/// - `decimals`: the number of digits after the decimal point.
fn num_format(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn env() -> Environment<'static> {
        let mut env = Environment::new();
        NumOps::register(&mut env);
        env
    }

    mod ceil {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::positive_fraction(3.62, "4.0")]
        #[case::negative_fraction(-3.62, "-3.0")]
        #[case::already_whole(4.0, "4.0")]
        #[case::zero(0.0, "0.0")]
        #[case::large_number(1_000_000.5, "1000001.0")]
        fn rounds_up_to_the_nearest_integer(
            #[case] input: f64,
            #[case] expected: &str,
        ) {
            let output = env()
                .render_str(
                    "{{ value | ceil }}",
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");
            assert_eq!(output, expected);
        }
    }

    mod floor {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::positive_fraction(3.62, "3.0")]
        #[case::negative_fraction(-3.62, "-4.0")]
        #[case::already_whole(4.0, "4.0")]
        #[case::zero(0.0, "0.0")]
        #[case::large_number(1_000_000.5, "1000000.0")]
        fn rounds_down_to_the_nearest_integer(
            #[case] input: f64,
            #[case] expected: &str,
        ) {
            let output = env()
                .render_str(
                    "{{ value | floor }}",
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");
            assert_eq!(output, expected);
        }
    }

    mod sqrt {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::perfect_square(9.0, "3.0")]
        #[case::zero(0.0, "0.0")]
        #[case::negative_zero(-0.0, "-0.0")]
        #[case::non_perfect_square(42.0, "6.48074069840786")]
        #[case::large_number(1_000_000.0, "1000.0")]
        fn returns_the_square_root(#[case] input: f64, #[case] expected: &str) {
            let output = env()
                .render_str(
                    "{{ value | sqrt }}",
                    minijinja::context! { value => input },
                )
                .expect("render succeeds");
            assert_eq!(output, expected);
        }

        #[test]
        fn errors_on_negative_input() {
            let err = env()
                .render_str(
                    "{{ value | sqrt }}",
                    minijinja::context! { value => -4.0 },
                )
                .expect_err("negative sqrt must error");
            assert_eq!(err.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod num_format {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::rounds_to_two_places(7.4567, 2, "7.46")]
        #[case::pads_a_whole_number(3.0, 2, "3.00")]
        #[case::zero_decimals_rounds_to_integer(3.6, 0, "4")]
        #[case::negative_number(-7.4567, 2, "-7.46")]
        #[case::zero_value(0.0, 3, "0.000")]
        #[case::large_number(1_234_567.891, 1, "1234567.9")]
        fn formats_to_exactly_n_decimal_places(
            #[case] input: f64,
            #[case] decimals: usize,
            #[case] expected: &str,
        ) {
            let output = env()
                .render_str(
                    "{{ value | num_format(decimals) }}",
                    minijinja::context! { value => input, decimals => decimals },
                )
                .expect("render succeeds");
            assert_eq!(output, expected);
        }
    }
}
