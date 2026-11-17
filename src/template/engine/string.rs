//! Registers stateless string filters for templates.
//!
//! [`StrOps`] registers thirteen filters: five case conversions, six string
//! operations, and two regex operations. The filters convert case, trim text,
//! truncate text, count words, repeat text, and process regex patterns.
//!
//! Each filter is a function registered with [`Environment::add_filter`]. The
//! filters have no shared state and do not use [`Object`] dispatch.
//!
//! Case conversion uses [`Casing::to_case`]. Prefix and suffix trimming, string
//! repetition, and word splitting use the Rust standard library. Regex filters
//! compile each pattern on every call with [`Regex::new`].
//!
//! # Examples
//!
//! ```text
//! {{ "hello world" | snake_case }}          -> hello_world
//! {{ "notes.md" | trim_suffix(".md") }}     -> notes
//! {{ "Hello World" | truncate(8) }}         -> Hello...
//! {{ "one two three" | truncate_words(2) }} -> one two ...
//! {{ "one two three" | word_count }}        -> 3
//! {{ "ab" | repeat(3) }}                    -> ababab
//! {{ "a1b2" | regex_replace("\d", "#") }}   -> a#b#
//! ```
//!
//! [`Object`]: minijinja::value::Object
//! [`Casing::to_case`]: convert_case::Casing::to_case

use convert_case::{Case, Casing as _};
use minijinja::{Environment, Error, value::Kwargs};
use regex::Regex;

use super::error::TemplateEngineResult;

/// Names the stateless string filters registered with the template environment.
pub(super) struct StrOps;

impl StrOps {
    /// Registers the stateless string filters with `env`.
    ///
    /// This associated function needs no [`StrOps`] instance because the
    /// filters carry no state.
    #[inline]
    pub(super) fn register(env: &mut Environment<'static>) {
        env.add_filter("snake_case", |value: &str| value.to_case(Case::Snake));
        env.add_filter("kebab_case", |value: &str| value.to_case(Case::Kebab));
        env.add_filter("camel_case", |value: &str| value.to_case(Case::Camel));
        env.add_filter("pascal_case", |value: &str| {
            value.to_case(Case::Pascal)
        });
        env.add_filter("title_case", |value: &str| value.to_case(Case::Title));
        env.add_filter("trim_prefix", trim_prefix);
        env.add_filter("trim_suffix", trim_suffix);
        env.add_filter("truncate", truncate);
        env.add_filter("truncate_words", truncate_words);
        env.add_filter("word_count", |value: &str| {
            value.split_whitespace().count()
        });
        env.add_filter("repeat", str::repeat);
        env.add_filter("regex_replace", regex_replace);
        env.add_filter("regex_match", regex_match);
    }
}

/// Removes `prefix` from the start of `value` when it is present.
///
/// Returns `value` unchanged when [`str::strip_prefix`] finds no `prefix`.
///
/// # Examples
///
/// ```
/// # let mut env = minijinja::Environment::new();
/// # env.add_filter("trim_prefix", |value: &str| {
/// #     value.strip_prefix("foo_").unwrap_or(value).to_owned()
/// # });
/// let rendered = env
///     .render_str("{{ 'foo_bar' | trim_prefix }}", ())
///     .expect("template renders");
/// assert_eq!(rendered, "bar");
/// ```
fn trim_prefix(value: &str, prefix: &str) -> String {
    value.strip_prefix(prefix).unwrap_or(value).to_owned()
}

/// Removes `suffix` from the end of `value` when it is present.
///
/// Returns `value` unchanged when [`str::strip_suffix`] finds no `suffix`.
fn trim_suffix(value: &str, suffix: &str) -> String {
    value.strip_suffix(suffix).unwrap_or(value).to_owned()
}

/// Truncates `value` to `length` characters and returns the result.
///
/// Counts Unicode characters, not bytes, and never splits a UTF-8 character.
/// Includes the ellipsis within `length`. Returns `value` unchanged
/// when its character count does not exceed `length`.
///
/// # Errors
///
/// - [`minijinja::ErrorKind::InvalidOperation`] if `ellipsis` is not a string.
/// - [`minijinja::ErrorKind::TooManyArguments`] if `kwargs` has a key other
///   than `ellipsis`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Kwargs::assert_all_used consumes self by value; &Kwargs \
              couldn't call it, so passing by value is required"
)]
fn truncate(
    value: &str,
    length: usize,
    kwargs: Kwargs,
) -> TemplateEngineResult<String> {
    let ellipsis = ellipsis_kwarg(&kwargs)?;

    if value.chars().count() <= length {
        return Ok(value.to_owned());
    }

    // If the ellipsis meets or exceeds `length`, return its first `length`
    // characters. This avoids subtracting `ellipsis_len` from `length`.
    let ellipsis_len = ellipsis.chars().count();
    if ellipsis_len >= length {
        return Ok(ellipsis.chars().take(length).collect());
    }

    let kept: String =
        value.chars().take(length.saturating_sub(ellipsis_len)).collect();
    Ok(format!("{kept}{ellipsis}"))
}

/// Truncates `value` to `count` whitespace-separated words and returns the
/// result.
///
/// Uses one pass over [`str::split_whitespace`] to collect retained words and
/// check for omitted words. Returns the input unchanged when no word is
/// omitted.
///
/// # Errors
///
/// - [`minijinja::ErrorKind::InvalidOperation`] if `ellipsis` is not a string.
/// - [`minijinja::ErrorKind::TooManyArguments`] if `kwargs` has a key other
///   than `ellipsis`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Kwargs::assert_all_used consumes self by value; &Kwargs \
              couldn't call it, so passing by value is required"
)]
fn truncate_words(
    value: &str,
    count: usize,
    kwargs: Kwargs,
) -> TemplateEngineResult<String> {
    let ellipsis = ellipsis_kwarg(&kwargs)?;

    let mut words = value.split_whitespace();
    let mut kept = String::new();
    for (index, word) in words.by_ref().take(count).enumerate() {
        if index > 0 {
            kept.push(' ');
        }
        kept.push_str(word);
    }

    // If `words` is exhausted, every word fits within `count`. Return `value`
    // unchanged to preserve its original whitespace.
    if words.next().is_none() {
        return Ok(value.to_owned());
    }
    if kept.is_empty() {
        return Ok(ellipsis.to_owned());
    }
    kept.push(' ');
    kept.push_str(ellipsis);
    Ok(kept)
}

/// Gets the `ellipsis` kwarg for truncation filters.
///
/// Uses `"..."` when the kwarg is absent and rejects unknown kwargs with
/// [`Kwargs::assert_all_used`].
///
/// # Errors
///
/// - [`minijinja::ErrorKind::InvalidOperation`] if `ellipsis` is not a string.
/// - [`minijinja::ErrorKind::TooManyArguments`] if `kwargs` has a key other
///   than `ellipsis`.
fn ellipsis_kwarg(kwargs: &Kwargs) -> TemplateEngineResult<&str> {
    let ellipsis = kwargs.get::<Option<&str>>("ellipsis")?.unwrap_or("...");
    kwargs.assert_all_used()?;
    Ok(ellipsis)
}

/// Replaces every non-overlapping regex match in `value` with `replacement`.
///
/// The replacement can refer to capture groups such as `$1` and `$2` with the
/// syntax of [`Regex::replace_all`]. Compiles `pattern` on every call with
/// [`Regex::new`].
///
/// # Errors
///
/// - [`minijinja::ErrorKind::InvalidOperation`] if `pattern` is not a valid
///   regex (see [`regex_compile_error`]).
fn regex_replace(
    value: &str,
    pattern: &str,
    replacement: &str,
) -> TemplateEngineResult<String> {
    let re = Regex::new(pattern)
        .map_err(|source| regex_compile_error(pattern, source))?;
    Ok(re.replace_all(value, replacement).into_owned())
}

/// Checks whether `value` contains a match for the regex `pattern`.
///
/// Returns `true` when the pattern matches and `false` otherwise.
///
/// # Errors
///
/// - [`minijinja::ErrorKind::InvalidOperation`] if `pattern` is not a valid
///   regex (see [`regex_compile_error`]).
fn regex_match(value: &str, pattern: &str) -> TemplateEngineResult<bool> {
    let re = Regex::new(pattern)
        .map_err(|source| regex_compile_error(pattern, source))?;
    Ok(re.is_match(value))
}

/// Converts a [`regex::Error`] into a [`minijinja::Error`].
///
/// The regex filters return this template error when a pattern is invalid.
fn regex_compile_error(pattern: &str, source: regex::Error) -> Error {
    super::error::invalid_operation(
        format!("invalid regex pattern {pattern:?}"),
        source,
    )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn env() -> Environment<'static> {
        let mut env = Environment::new();
        StrOps::register(&mut env);
        env
    }

    /// Renders `template` with one `value` binding.
    ///
    /// Uses embedded literal filter arguments when the piped string is the only
    /// dynamic input.
    ///
    /// # Panics
    ///
    /// Panics if the template fails to render.
    fn render(
        env: &Environment<'static>,
        template: &str,
        value: &str,
    ) -> String {
        env.render_str(template, minijinja::context! { value => value })
            .expect("template renders")
    }

    /// Renders `template` with `value` and returns its render error.
    ///
    /// # Panics
    ///
    /// Panics if the template renders successfully.
    fn render_err(
        env: &Environment<'static>,
        template: &str,
        value: &str,
    ) -> Error {
        env.render_str(template, minijinja::context! { value => value })
            .expect_err("template must fail to render")
    }

    #[rstest]
    #[case::snake_case("snake_case", "hello world", "hello_world")]
    #[case::kebab_case("kebab_case", "hello world", "hello-world")]
    #[case::camel_case("camel_case", "hello world", "helloWorld")]
    #[case::pascal_case("pascal_case", "hello world", "HelloWorld")]
    #[case::title_case("title_case", "hello world", "Hello World")]
    fn converts_a_multi_word_phrase(
        #[case] filter: &str,
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        let template = format!("{{{{ value | {filter} }}}}");

        let result =
            env().render_str(&template, minijinja::context! { value => input });
        let rendered = result.expect("render succeeds");

        assert_eq!(rendered, expected);
    }

    #[rstest]
    #[case::snake_case("snake_case", "already_snake", "already_snake")]
    #[case::kebab_case("kebab_case", "already-kebab", "already-kebab")]
    #[case::camel_case("camel_case", "alreadyCamel", "alreadyCamel")]
    #[case::pascal_case("pascal_case", "AlreadyPascal", "AlreadyPascal")]
    #[case::title_case("title_case", "Already Title", "Already Title")]
    fn is_idempotent_on_already_converted_input(
        #[case] filter: &str,
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        let template = format!("{{{{ value | {filter} }}}}");

        let result =
            env().render_str(&template, minijinja::context! { value => input });
        let rendered = result.expect("render succeeds");

        assert_eq!(rendered, expected);
    }

    /// Covers boundary inputs that the two behavior tables do not include.
    ///
    /// Tests empty strings with all five filters, plus a single word, Unicode,
    /// digits, and punctuation. The five [`Case`] targets share splitting
    /// logic.
    #[rstest]
    #[case::snake_case_with_empty_input("snake_case", "", "")]
    #[case::kebab_case_with_empty_input("kebab_case", "", "")]
    #[case::camel_case_with_empty_input("camel_case", "", "")]
    #[case::pascal_case_with_empty_input("pascal_case", "", "")]
    #[case::title_case_with_empty_input("title_case", "", "")]
    #[case::snake_case_with_a_single_word("snake_case", "hello", "hello")]
    #[case::kebab_case_with_unicode_input(
        "kebab_case",
        "café münchen",
        "café-münchen"
    )]
    #[case::camel_case_with_digits(
        "camel_case",
        "v2 release 10",
        "v2Release10"
    )]
    #[case::pascal_case_with_punctuation(
        "pascal_case",
        "hello, world!",
        "Hello,World!"
    )]
    fn converts_boundary_inputs(
        #[case] filter: &str,
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        let template = format!("{{{{ value | {filter} }}}}");

        let result =
            env().render_str(&template, minijinja::context! { value => input });
        let rendered = result.expect("render succeeds");

        assert_eq!(rendered, expected);
    }

    mod trim_prefix {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::removes_the_prefix_when_present("foo_bar", "foo_", "bar")]
        #[case::no_op_when_prefix_absent("bar", "foo_", "bar")]
        #[case::no_op_on_an_empty_string("", "foo_", "")]
        #[case::unicode_prefix("café_bar", "café_", "bar")]
        fn strips_the_prefix(
            #[case] input: &str,
            #[case] prefix: &str,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!("{{{{ value | trim_prefix({prefix:?}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }
    }

    mod trim_suffix {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::removes_the_suffix_when_present("bar.md", ".md", "bar")]
        #[case::no_op_when_suffix_absent("bar", ".md", "bar")]
        #[case::no_op_on_an_empty_string("", ".md", "")]
        #[case::unicode_suffix("café_résumé", "_résumé", "café")]
        fn strips_the_suffix(
            #[case] input: &str,
            #[case] suffix: &str,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!("{{{{ value | trim_suffix({suffix:?}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }
    }

    mod truncate {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::respects_the_length_including_the_default_ellipsis(
            "Hello World",
            5,
            "He..."
        )]
        #[case::no_op_for_short_strings("Hi", 10, "Hi")]
        #[case::no_op_at_the_exact_length_boundary("Hello", 5, "Hello")]
        #[case::no_op_on_an_empty_string("", 5, "")]
        #[case::unicode_input_is_truncated_by_char_not_byte(
            "héllo wörld",
            5,
            "hé..."
        )]
        fn truncates_by_character_count(
            #[case] input: &str,
            #[case] length: usize,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!("{{{{ value | truncate({length}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }

        #[test]
        fn accepts_a_custom_ellipsis() {
            let env = env();
            let output = render(
                &env,
                r#"{{ value | truncate(7, ellipsis="…") }}"#,
                "Hello World",
            );
            assert_eq!(output, "Hello …");
        }

        #[test]
        fn shrinks_the_ellipsis_when_it_alone_exceeds_the_length() {
            let env = env();
            let output =
                render(&env, "{{ value | truncate(2) }}", "Hello World");
            assert_eq!(output, "..");
        }
    }

    mod truncate_words {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::truncates_a_multi_word_phrase(
            "one two three four",
            2,
            "one two ..."
        )]
        #[case::no_op_when_word_count_is_within_the_limit(
            "one   two",
            5,
            "one   two"
        )]
        #[case::no_op_on_an_empty_string("", 5, "")]
        #[case::no_op_on_a_single_word("hello", 1, "hello")]
        #[case::truncates_a_single_word_phrase("hello world", 1, "hello ...")]
        #[case::zero_count_returns_just_the_ellipsis("hello world", 0, "...")]
        fn truncates_by_word_count(
            #[case] input: &str,
            #[case] count: usize,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!("{{{{ value | truncate_words({count}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }

        #[test]
        fn accepts_a_custom_ellipsis() {
            let env = env();
            let output = render(
                &env,
                r#"{{ value | truncate_words(1, ellipsis="…") }}"#,
                "one two three",
            );
            assert_eq!(output, "one …");
        }
    }

    mod word_count {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::counts_whitespace_separated_tokens("hello world", 2)]
        #[case::counts_zero_for_an_empty_string("", 0)]
        #[case::collapses_varied_whitespace("  hello\tworld\n\n foo  ", 3)]
        #[case::counts_one_for_a_single_word("hello", 1)]
        #[case::counts_zero_for_whitespace_only("   \t\n ", 0)]
        fn counts_words(#[case] input: &str, #[case] expected: usize) {
            let env = env();
            let output = render(&env, "{{ value | word_count }}", input);
            assert_eq!(output, expected.to_string());
        }
    }

    mod repeat {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::zero_repetitions_is_empty("ab", 0, "")]
        #[case::one_repetition_is_the_identity("ab", 1, "ab")]
        #[case::multiple_repetitions("ab", 3, "ababab")]
        #[case::empty_string_input("", 5, "")]
        #[case::unicode_input("é", 2, "éé")]
        fn repeats_the_string(
            #[case] input: &str,
            #[case] n: usize,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!("{{{{ value | repeat({n}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }
    }

    mod regex_replace {
        use minijinja::ErrorKind;
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::replaces_all_matches("hello@world", "@.*", "", "hello")]
        #[case::no_match_returns_the_string_unchanged(
            "hello", "xyz", "abc", "hello"
        )]
        #[case::unicode_input("café", "é", "e", "cafe")]
        fn replaces_matches(
            #[case] input: &str,
            #[case] pattern: &str,
            #[case] replacement: &str,
            #[case] expected: &str,
        ) {
            let env = env();
            let template = format!(
                "{{{{ value | regex_replace({pattern:?}, {replacement:?}) }}}}"
            );
            assert_eq!(render(&env, &template, input), expected);
        }

        #[test]
        fn supports_capture_group_references() {
            let env = env();
            let output = render(
                &env,
                r#"{{ value | regex_replace("(\w+) (\w+)", "$2 $1") }}"#,
                "John Smith",
            );
            assert_eq!(output, "Smith John");
        }

        #[test]
        fn an_invalid_pattern_raises_a_minijinja_error_instead_of_panicking() {
            let env = env();
            let error = render_err(
                &env,
                r#"{{ value | regex_replace("(", "x") }}"#,
                "hello",
            );
            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }

    mod regex_match {
        use minijinja::ErrorKind;
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        #[case::true_on_a_match("hello@world", "@", "True")]
        #[case::false_on_no_match("hello", "xyz", "False")]
        #[case::empty_pattern_matches_anything("", "", "True")]
        fn matches_the_pattern(
            #[case] input: &str,
            #[case] pattern: &str,
            #[case] expected: &str,
        ) {
            let env = env();
            let template =
                format!("{{{{ value | regex_match({pattern:?}) }}}}");
            assert_eq!(render(&env, &template, input), expected);
        }

        #[test]
        fn an_invalid_pattern_raises_a_minijinja_error_instead_of_panicking() {
            let env = env();
            let error =
                render_err(&env, r#"{{ value | regex_match("(") }}"#, "hello");
            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
    }
}
