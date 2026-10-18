# Who owns config-derived types, trust/tracking state, and the config lifecycle?

Type: grilling
Status: open
Blocked by: 17

## Question
Resolve the config subsystem's boundary: (a) why `task.rs`/`tag.rs`/`field.rs` live at crate root and whether `TaskStatusMap`, tag filters and status/emoji parsing belong to config, note, or a leaf; (b) the upward `note -> config` edge (`src/note/parser/input.rs:9`) and whether parsers take primitives or a typed parser-config; (c) `ConfigDigests` (412 LoC + syn scan) with no production consumer vs a fingerprint over what the index actually consumes; (d) twin `TrustStore`/`TrackerStore` over `FilePathTracker` and the two-store ordering invariant; (e) `ConfigFile` phantom typestate vs a direct pipeline; (f) where `file_tracker`/`dirs`/`env_vars` live; (g) scaffolding (`cli::init`) ownership.

Facts: `survey/config-dialog.md`, `survey/root-primitives.md` §3-5, `survey/note.md` H4, `src/file_tracker.rs` has exactly two callers (verify in `research/01`), ADR-0002.
Alternatives per sub-question including keep-as-is; note which are independent so the spec can sequence them.
Inputs: execution-context decision, `research/04`, `research/05` (digest vs consumed-input fingerprint), `research/01`.
