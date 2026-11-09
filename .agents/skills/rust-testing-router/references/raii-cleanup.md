# RAII Cleanup and Hermetic Resources

> Use RAII (`Drop`) guards and isolated sandboxes so test teardown runs even when an assertion panics.

## 1. Why Teardown Must Use `Drop`

Code written after an `assert!` does not execute when the assertion panics. Wrap temporary files, child processes, background servers, and database transactions in types that clean up inside `Drop`.

## 2. Temporary Files and Directories

Use `tempfile::TempDir` or `tempfile::NamedTempFile` for filesystem sandboxes:

```rust
#[test]
fn writes_index_into_workspace_directory() {
    let workspace = tempfile::TempDir::new().expect("create temp workspace");
    let config_path = workspace.path().join("config.toml");
    std::fs::write(&config_path, "version = 1").expect("write config");

    let status = initialize_index(workspace.path());

    assert!(status.is_ok(), "expected index init to succeed, got {status:?}");
} // workspace directory is removed here even on panic
```

Keep the `TempDir` binding alive for the full scope of the test. Calling `.path()` on a temporary `TempDir::new().unwrap().path()` drops the directory before the statement finishes.

## 3. Environment Variables Without `unsafe`

In Rust 2024 and crates with `unsafe_code = "deny"`, `std::env::set_var` and `std::env::remove_var` are `unsafe` because mutating process-global environment variables races with other threads and C libraries. Never write an `unsafe` `EnvGuard` in test code.

Use one of these three safe patterns instead:

### Pattern A: Hermetic Child Process Environment (Preferred for CLI Tests)

Clear the child environment and pass explicit variables on `std::process::Command`:

```rust
use std::process::Command;

#[test]
fn cli_respects_custom_config_dir() {
    let sandbox = tempfile::TempDir::new().expect("create sandbox");

    let output = Command::new(env!("CARGO_BIN_EXE_my_app"))
        .current_dir(sandbox.path())
        .env_clear()
        .env("HOME", sandbox.path())
        .env("MY_APP_CONFIG", sandbox.path().join("custom.toml"))
        .arg("status")
        .output()
        .expect("spawn binary");

    assert!(output.status.success());
}
```

### Pattern B: Dependency Injection (Preferred for Unit Tests)

Pass environment values as parameters or through a small lookup trait/closure instead of reading `std::env::var` deep inside domain logic:

```rust
pub fn resolve_cache_dir(get_env: impl Fn(&str) -> Option<String>) -> PathBuf {
    get_env("MY_APP_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/default"))
}
```

### Pattern C: Scoped `temp-env` Crate Under Process Isolation

When legacy library code calls `std::env::var` directly, use `temp_env::with_var` and run with `cargo nextest` (which isolates each test in its own process):

```rust
#[test]
fn reads_legacy_env_variable() {
    temp_env::with_var("LEGACY_TIMEOUT_SECS", Some("30"), || {
        assert_eq!(read_legacy_timeout(), 30);
    });
}
```

## 4. Common Resource Guards

| Resource | Safe Pattern |
|---|---|
| Temporary directory | `tempfile::TempDir` held in a local variable |
| Temporary file | `tempfile::NamedTempFile` |
| Child process / server | Struct implementing `Drop` that calls `child.kill()` and `child.wait()` |
| One-off rollback action | `scopeguard::defer! { ... }` |
| Subprocess environment | `Command::env_clear()` + `.env(k, v)` |

## Completion

Completion: zero manual teardown lines appear after assertions, zero `unsafe` environment mutations exist, and every resource cleans up on panic.
