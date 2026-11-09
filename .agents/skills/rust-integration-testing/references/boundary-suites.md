# Integration Binary Layout and Boundary Suites

> Each integration target in `tests/` compiles as a separate crate linked against your library's public API.

## 1. Valid `tests/` Directory Layouts

Cargo compiles every top-level `tests/<name>.rs` file and every `tests/<name>/main.rs` directory entry as a separate test binary.

Never create `tests/Cargo.toml` (Cargo ignores nested manifests). Never create `tests/api/mod.rs` without `tests/api.rs` or `tests/api/main.rs`, or Cargo skips the target.

### Layout A: Consolidated Multi-Module Binary (`tests/<target>/main.rs` or `tests/<target>.rs`)

Prefer consolidating related integration tests into one or two top-level binaries (`tests/integration.rs` + `tests/integration/*.rs`, or `tests/integration/main.rs`). Fewer top-level binaries reduce Cargo link times significantly:

```text
my_project/
├── Cargo.toml
├── src/
│   └── lib.rs
└── tests/
    ├── integration.rs           # Top-level binary crate root
    ├── integration/             # Submodules included by integration.rs
    │   ├── persistence.rs
    │   └── query.rs
    └── support/                 # Shared helpers (not compiled as a standalone binary)
        └── mod.rs
```

```rust
// tests/integration.rs
#[path = "integration/persistence.rs"]
mod persistence;
#[path = "integration/query.rs"]
mod query;
#[path = "support/mod.rs"]
mod support;
```

Alternatively, place `main.rs` inside the subdirectory:

```text
tests/
├── api/
│   ├── main.rs                  # Compiled as the `api` integration binary
│   ├── auth.rs
│   └── orders.rs
└── common/
    └── mod.rs                   # Shared helper module (not a binary)
```

## 2. Feature-Gated Integration Targets (`required-features`)

When an integration binary needs opt-in test builders or inspection hooks (`test-utils`), declare `required-features` in `Cargo.toml`:

```toml
[[test]]
name = "integration"
path = "tests/integration.rs"
required-features = ["test-utils"]
```

Always pass `--features test-utils` (or `--all-features`) and `--no-tests=fail` when running this target so Cargo does not silently skip the binary:

```bash
cargo nextest run --test integration --features test-utils --no-tests=fail
```

## 3. Boundary Strategy Decision Matrix

Select the lightest strategy that faithfully reproduces the boundary contract:

| Strategy | Use When | Isolation Mechanism |
|---|---|---|
| `temp-local` | Filesystem, SQLite, config discovery, atomic file writes | `tempfile::TempDir` per test |
| `hand-written fake` | In-memory failure injection (disk full, corrupted header, clock) | Trait seam or `test-utils` constructor |
| `local server` | HTTP client, webhook, socket timeout, or retry protocol | Ephemeral `127.0.0.1:0` listener per test |
| `container` | External database engine (Postgres, Redis) with SQL dialect semantics | Ephemeral container or disposable schema |
| `real` | Owned local CLI binary (`CARGO_BIN_EXE_<name>`) | Subprocess with `TempDir` + `env_clear()` |

## 4. Hermetic CLI and Process Boundary Recipe

Keep `src/main.rs` as a thin entrypoint over `src/lib.rs`. Test process-level contracts (exit codes, `stdout`/`stderr` separation, broken pipes, working directory, and environment scrubbing) by spawning the compiled binary inside a hermetic sandbox:

```rust
use std::process::Command;
use pretty_assertions::assert_eq;

#[test]
fn init_command_creates_config_in_clean_environment() {
    let sandbox = tempfile::TempDir::new().expect("create temp sandbox");

    let output = Command::new(env!("CARGO_BIN_EXE_my_cli"))
        .current_dir(sandbox.path())
        .env_clear()
        .env("HOME", sandbox.path())
        .env("NO_COLOR", "1")
        .args(["init", "--quiet"])
        .output()
        .expect("spawn my_cli binary");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"");
    assert!(sandbox.path().join(".my_cli.toml").is_file());
}
```

## 5. See Also

- [`../../rust-testing-router/references/boundaries.md`](../../rust-testing-router/references/boundaries.md) - arbitrating between BDD, integration, and unit layers.
- [`../../rust-testing-router/references/commands.md`](../../rust-testing-router/references/commands.md) - `nextest` target selectors and `--no-tests=fail`.
- [`../../rust-testing-router/references/raii-cleanup.md`](../../rust-testing-router/references/raii-cleanup.md) - panic-safe `TempDir` and subprocess guards.
- [`../../rust-testing-router/references/assertions.md`](../../rust-testing-router/references/assertions.md) - structured error and output assertions.
- [`../../rust-testing-router/references/snapshots.md`](../../rust-testing-router/references/snapshots.md) - `insta` snapshots for CLI tables and help output.

## Completion

Completion: integration targets compile without orphan files, isolate all filesystem and environment state, and pass under `cargo nextest run -E 'kind(test)' --no-tests=fail`.
