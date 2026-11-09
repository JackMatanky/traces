# Snapshot Testing (`insta`)

> Use `insta` for multi-line structures, tables, or diagnostics where a reviewable `.snap` diff beats inline strings.

## 1. When to Use

| Output Shape | Prefer |
|---|---|
| Scalar value, tuple, or small struct | `pretty_assertions::assert_eq!` |
| Multi-line CLI table, help text, or diagnostic report | `insta::assert_snapshot!` |
| Serialized JSON or YAML payload | `insta::assert_json_snapshot!` / `assert_yaml_snapshot!` |
| Complex nested `Debug` tree | `insta::assert_debug_snapshot!` |

Do not snapshot unredacted timestamps, random UUIDs, temporary file paths, or hash-map iteration orders.

## 2. Basic Usage

```rust
use insta::{assert_json_snapshot, assert_snapshot};

#[test]
fn formats_diagnostic_report() {
    let report = render_diagnostic("missing field `title`", 4, 12);

    assert_snapshot!(report);
}
```

## 3. Redacting Non-Deterministic Fields

Scrub dynamic values (timestamps, generated IDs, or sandbox paths) before snapshotting, or use `insta` redactions:

```rust
#[test]
fn serializes_audit_event() {
    let event = AuditEvent::now("user_login");

    assert_json_snapshot!(
        "audit_event_login",
        event,
        {
            ".timestamp" => "[timestamp]",
            ".event_id" => "[uuid]",
        }
    );
}
```

## 4. Review and CI Gate Commands

1. Run the test suite to generate `.snap.new` files for changed outputs.
2. Inspect and accept intentional changes interactively with `cargo insta review`.
3. Commit accepted `.snap` files alongside the code change.
4. Enforce snapshot freshness and reject stale `.snap` files in CI:

```bash
INSTA_UPDATE=no cargo nextest run
cargo insta test --check --unreferenced=reject
```

## Completion

Completion: all dynamic fields are redacted or normalized, `.snap` files are committed, and `INSTA_UPDATE=no` passes with zero pending or unreferenced snapshots.
