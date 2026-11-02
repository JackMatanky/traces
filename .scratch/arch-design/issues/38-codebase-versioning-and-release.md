# How does Traces version and release the codebase?

Type: decision
Status: open

## Question

Define how Traces bumps its version and cuts a release.

The project has no release process today. The version is `0.1.0`. The
repository has no version tags.

[Does the landed index freshness model
stand?](./25-index-freshness-and-invalidation.md) does not depend on this
ticket. The store identity uses an epoch and a fingerprint, not the binary
version. The binary version is only a label in notices.

Decide these points:

- Where the version lives. `Cargo.toml` is the proposed single source.
- When a bump is required.
- How a release checks that the version changed since the last tag.
  Use a `mise` task. Do not rely on habit.
- Whether to use `git-cliff` for the changelog. The `hk` commit hook already
  enforces Conventional Commits.
- How a developer build names itself between releases. `build.rs` can add
  `git describe` to `--version` only.
- The install instructions. They must say `cargo install --locked`. The
  fingerprint in ticket 25 reads the packaged `Cargo.lock`.
- Which tools to reject. `cargo-semver-checks` adds unneeded complexity, because
  `mise run check:visibility` already freezes the public API.

## Answer

Open.
