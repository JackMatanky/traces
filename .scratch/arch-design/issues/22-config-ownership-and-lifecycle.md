# Who owns config-derived types, trust/tracking state, and the config lifecycle?

Type: grilling
Status: open
Blocked by: 21, 12

## Question

Resolve the boundary of the config subsystem:

- (a) Check why `task.rs`/`tag.rs`/`field.rs` live at the crate root. Decide
  where `TaskStatusMap`, the tag filters and the status/emoji parsing belong to
  config, note, or a leaf.
- (b) Check the upward `note -> config` edge (`src/note/parser/input.rs:9`).
  Decide whether the parsers take primitives or a typed parser-config.
- (c) Decide the residual `ConfigDigests` surface now that drift detection
  landed (main @ `207cc3fb`). 10 of 15 per-setting digests feed the index-owned
  `IndexFingerprint` (`src/index/fingerprint.rs:158`). `Config::hash`/`compose`
  (pinned `traces.config.v1` framing), the root/templates/schema-directory
  digests, and the 380-line `syn` source-scan test (`syn` dev-dep) still have no
  production consumer. Every `IndexerService` construction computes and discards
  5 root/templates/schema-directory digest values. Choose one option: retire,
  keep as a general config fingerprint, or fold into the index.
- (d) Decide the design of the twin `TrustStore`/`TrackerStore` over
  `FilePathTracker`. Decide the two-store ordering invariant.
- (e) Choose between the `ConfigFile` phantom typestate and a direct pipeline.
- (f) Decide where `file_tracker`/`dirs`/`env_vars` live.
- (g) Decide the ownership of the scaffolding (`cli::init`).

Facts: `survey/config-dialog.md`, `survey/root-primitives.md` §3-5,
`survey/note.md` H4, `src/file_tracker.rs` has exactly two callers (verify in
`research/01`), ADR-0002.

Alternatives: List the alternatives for each sub-question, including keep-as-is.
Record which sub-questions are independent so the spec can sequence them.

Inputs: execution-context decision, `research/04`, `research/05` (its
digest-vs-fingerprint lesson; `207cc3fb` realized that lesson; see
`reverification.md`), `research/01`.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE keeps
one `.iwe/config.toml` with `deny_unknown_fields`, a `version` key and in-place
migrations through `toml_edit` that preserve user comments
(`crates/diwe/src/config.rs`, `migrate`) [V]; it still had to rename a whole
table (`[library]` to `[workspace]`, v0.25.0). Schema binding lives in this host
config (`[schemas.<name>].match`), not in the schema files; [ticket
20](./20-schema-note-typing-and-validation.md) chose class-owned `[binding]`.
The only transferable point for this ticket is the comment-preserving migration
technique if the config format ever changes.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). The
adapter snapshots typed OS paths, discovery inputs and overrides at workspace
open; specify precedence, an explicit base for relative paths (or reject
relative overrides), and where config-derived policy lives. Do not use a
Unicode-only environment map for paths. Hash-verify local config before parsing
and revalidate trust/content at each new held-workspace request; admitted
requests retain their immutable generation. Global config freshness is separate.
Supply one workspace's policy to its services, not a process-global value, so
different roots can coexist. The config surface also owns `DateConfig`
integration (zone source, week rules and opt-in bare-year filenames); decide its
syntax with ticket 27, not here.
