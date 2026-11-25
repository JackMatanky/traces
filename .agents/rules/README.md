# Project agent rules

These rules use the pi-warden rules format. One `#` heading is one rule.
They hold only conventions that belong to this repo. General Rust and style rules live in the global set at `~/.agents/rules/`.

## Layout

- `traces-<topic>.md` groups the project rules by topic.
- A rule file has no title heading, because the highest heading level delimits rules.
- A rule file has no `source:` header. A path or a line number goes stale when files move.
- Every rule has a `forbid:` header, so the check runs locally and costs no judge request.
- `README.md` is not a rule file. `.pi/pi-warden.json` never lists it in `rules.files`.
- `mise run rules:sync` writes `.pi/pi-warden.json` from the global rules and these files. `mise run rules:check` reports drift.

## Why the set is small

- Your `Cargo.toml` lint table, `clippy.toml`, and `rustfmt.toml` enforce most standards.
- The global rules cover general Rust and style conventions.
- A rule lives here only when it names a repo path or a repo decision.
- `paths:` has no negation, so a rule cannot skip a folder. A rule lists the folders it covers.

## Origin of each rule

| Rule | Origin |
| --- | --- |
| `traces-adr-human-accept` | traces AGENTS.md, section "ADRs" |
| `traces-bench-no-rustflags` | traces CODING_STANDARDS, section "Benchmarking" |
