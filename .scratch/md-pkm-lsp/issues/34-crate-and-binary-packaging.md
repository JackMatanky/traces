# Crate/binary packaging & integration into the Traces build

Type: grilling
Status: open
Blocked by: 09, 10
Assigned: agent

## Question

Grounding: Traces is currently a **single package**, not a Cargo workspace. `Cargo.toml` has one `[package]` and no `[workspace]` table. It produces one binary (`traces-pkm`, `src/main.rs`) that wraps the library crate's `pub mod cli` entrypoint (`cli::run()`). Treat that as the starting state only, **not** a constraint to preserve. Adding a comprehensive LSP is a large enough addition that converting to a Cargo workspace is a live, expected option. It is not a "larger structural change" that needs to clear a higher bar than staying single-crate. Decide this once the framework/runtime (09) and analysis-host (10) decisions are settled.

Decide these points:

- Decide where LSP code lives. Option one: a new `src/lsp/` module inside the existing single crate. Option two: convert to a Cargo workspace with a separate `traces-lsp` (or similarly named) crate that depends on the existing crate's `test-utils`-gated public API. Weigh both directions.
  - Workspace separation keeps "LSP protocol DTOs and client-specific concerns... at the protocol boundary" (a standing constraint). It isolates the CLI binary's dependency footprint, build time, and binary size from whatever ticket 09 selects. It forces the core crate's LSP-facing API to be deliberately designed rather than incidentally exposed.
  - A workspace also means two crates to version in lockstep. It means a `pub` surface on the core crate that must be right upfront, or churned across two `Cargo.toml`s as the LSP's needs evolve. It means duplicated CI/lint/release configuration: `clippy.toml`, `deny.toml`, `rustfmt.toml` etc. need workspace-level or per-member handling (check which).
  - A single-crate module has the inverse trade: no crate-boundary API design tax today, one `Cargo.toml`, one release cadence. But the LSP's concerns can leak into what is currently a clean `pub`/`pub(crate)` split (`src/lib.rs`'s existing `test-utils`-gated re-export pattern) as the LSP surface grows. It also permanently couples the CLI binary's dependency graph to the LSP's, unless the feature-flag boundary below is maintained with discipline over time.
  - Neither option is free. Pick based on how the LSP's actual shape (once 09/10 are known) stresses each trade-off. Do not pick based on which one is less work to start.
- Decide the binary and invocation shape. Option one: a new `[[bin]]` target (e.g. `traces-lsp`) that clients launch directly. Option two: a hidden `traces lsp` CLI subcommand (reusing the existing `clap` `Commands` enum in `src/cli/mod.rs`) that starts the server over stdio. Check what convention rumdl (`rumdl server`) and Marksman/Markdown Oxide use as precedent (tickets 01-03), because that is what editor-integration configs will expect. A separate `[[bin]]` fits a workspace split; a subcommand fits the single-crate module. Let the shape follow from the packaging decision, and do not force it.
- Decide the feature-flag boundary if the single-crate module is chosen: does the existing `test-utils` feature-flag pattern extend to gate LSP-only code? That pattern already exposes otherwise-`pub(crate)` items to `tests`/`benches`. The goal: a plain `cargo build` for CLI-only use does not pull in LSP dependencies by default. This point is moot if a workspace split is chosen, because separate crates already have separate dependency graphs.
- Check release and packaging implications. `profile.release` already sets `panic = "abort"`, `lto = true`, `codegen-units = 1`, `strip = true`. Confirm these stay compatible with whatever runtime ticket 09 selects, for example `panic = "abort"` interacting with any async runtime's panic-handling expectations. If a workspace split is chosen, confirm workspace-wide vs per-crate profile inheritance.
- Inputs pending from ticket 22 (deliberately *not* a `Blocked by: 22` edge; it is an optional input, per 22's Q8): template-analysis module placement (`src/template/analysis.rs`, currently assumed pending this ticket's crate decision); any `pub(super)` widening its helper APIs need within `src/template/`; and the `unstable_machinery` feature-gate plus CLI-build cost if the gate must ride an LSP-only feature (or `derived_names()` visibility if the analysis lands outside `src/template/`). 34 changes plumbing, not semantics. These resolve before implementation, not at this ticket's claim.
- (amended 2026-10-21 by iwe research) iwe ships a four-binary workspace over shared `diwe`/`liwe` crates with release-plz and diverging crate versions. That is direct precedent for the workspace option ([tool-iwe](../research/tool-iwe.md) Parts 1, 5, 8).

## Inputs from 25 (2026-10-15)

- **Diagnostics fold module home pending here (25 Q4-R)**: the protocol-free diagnostic type, the fold functions, the `DiagnosticRule` registry, and the `OverrideSet` need a module home. Constraint from 25 Q15-R2: the registry must be a **leaf module**. Feature modules (links, footnotes, frontmatter, schema, query, template, tasks) and the LSP boundary must reach it without cycles. The shape: dotted `<feature>.<rule>` codes, `enum DiagnosticRule` with `code()`/`default_severity()`/`const ALL`. Placement interacts with 22's engine-adjacent `src/template/analysis.rs` gate question (the protocol-free invariant applies here too).

## Inputs from 26 (2026-10-18)

- **Module homes to place**: the nav spine (`nav::Occurrence`/`NavTarget`/`Resolution`, a protocol-free core module) and the `convert` module (a protocol-boundary module). Both carry the same "pending 34" status as 25's fold. 26's Answer carries that qualifier verbatim.
- **Visibility widening budgeted as part of placement**: `ByteSpan` to pub(crate); a `LinkResolver` facade method (`resolve_link`-style); and `paths_with_tag` exposure. These are two named changes, billed to this ticket rather than discovered downstream.
- **Pin the tower-lsp-server <-> ls-types pairing**: ls-types' `Uri` helpers are load-bearing for 14's `Url` -> `Uri` migration and 26's edge conversion. Re-check the pairing at implementation time (this feeds 09 as well).
