# `rstest` Fixtures and Parameter Matrices

> Use `#[fixture]` to inject composable, per-test Arrange state without global variables or repetitive setup blocks.

## 1. Basic `#[fixture]` and Composition

A `#[fixture]` function builds Arrange data and injects it into any `#[rstest]` function whose parameter name matches the fixture name. Fixtures can take other fixtures as parameters to form a dependency graph:

```rust
use rstest::{fixture, rstest};

#[fixture]
fn base_config() -> Config {
    Config { retries: 3, timeout_secs: 5 }
}

#[fixture]
fn client(base_config: Config) -> Client {
    Client::new(base_config)
}

#[rstest]
fn connects_with_default_retries(client: Client) {
    assert_eq!(client.max_retries(), 3);
}
```

## 2. Partial Overrides with `#[with(...)]` and `#[default(...)]`

Give a fixture parameters with `#[default(...)]` values so callers only override what their test cares about using `#[with(...)]`:

```rust
use rstest::{fixture, rstest};

#[fixture]
fn user(
    #[default("alice")] name: &str,
    #[default(true)] active: bool,
) -> User {
    User { name: name.to_owned(), active }
}

#[rstest]
fn rejects_inactive_user(#[with("bob", false)] user: User) {
    let status = authorize(&user);

    assert!(status.is_err());
}
```

## 3. Binding Renamed Parameters with `#[from(...)]`

When a parameter name differs from the fixture name (or starts with `_`), bind it with `#[from(fixture_name)]`. Do not pass a string literal:

```rust
#[rstest]
fn processes_admin_account(
    #[from(user)]
    #[with("admin", true)]
    admin_account: User,
) {
    assert!(admin_account.active);
}
```

## 4. Lifetime Semantics: Fresh Per-Test vs. `#[once]`

- **Default (fresh per-test):** Each `#[rstest]` invocation calls the fixture function anew. Each fixture parameter inside a single test receives its own instance unless you inspect the same variable.
- **`#[once]` (shared static reference):** Evaluates the fixture once per test binary and injects `&'static T`. Use the `#[once]` attribute only for expensive, read-only setup. Never mutate shared state across tests.

## 5. Matrix Cross-Products (`#[values]`) and File Discovery (`#[files]`)

- Put the `#[values(a, b)]` attribute on multiple parameters to check the Cartesian product of orthogonal options.
- Put the `#[files("tests/fixtures/*.json")]` attribute on a `PathBuf` parameter to generate a test case per matching file.

```rust
use rstest::rstest;

#[rstest]
fn roundtrips_across_encodings_and_levels(
    #[values(Encoding::Utf8, Encoding::Ascii)] encoding: Encoding,
    #[values(1, 9)] compression_level: u8,
) {
    let encoded = encode("payload", encoding, compression_level);

    assert_eq!(decode(&encoded, encoding), Ok("payload".to_owned()));
}
```

## 6. Async Fixtures and Clippy Formatting

- Mark async setup functions with `#[fixture] async fn my_fixture() -> T` and `#[awt]` on the test or `.await` on the injected `impl Future<Output = T>` parameter.
- Under strict Clippy rules (`clippy::unused_unit`), format multiline `#[fixture]` bodies as standard block expressions with zero hidden assertions.

## Completion

Completion: fixtures build Arrange state only (zero hidden assertions), use `#[from(ident)]` for renamed bindings, and default to fresh per-test instances.
