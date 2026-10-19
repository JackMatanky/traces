# How do mature tools enforce root confinement and portable path identity?

Type: research
Status: resolved
Blocked by: none

## Question

Root confinement is enforced by `strict-path` at construction (`src/path.rs:181-189`) but `SafeRelativePath` returns a plain join with an acknowledged TOCTOU window (`:159-161,190`); `FileMeta::path` and `CliError` fields degrade back to raw `PathBuf`; template path inspection accepts absolute paths (`src/template/engine/path.rs:162`) and DryRun skips confinement (`src/template/service.rs:225`); the redb path codec stores raw OS bytes/wide units, making the cache non-portable (`src/path.rs:340-437`). See `survey/root-primitives.md` H7, `survey/cross-cutting.md` H4, `survey/template.md` H4/H5.

Study: `cap-std`/`cap-primitives` (`Dir`, `open_beneath`, Windows strategy), Linux `openat2(RESOLVE_BENEATH)`, `strict-path` own source (what it actually guarantees and where it can't), gitoxide `gix-path`/`gix-fs`/`gix-worktree` path-safety checks (`gix-validate`, protect-NTFS/HFS), Cargo `cargo-util::paths` + `restricted_names`, Ruff `ruff_db::system::SystemPath` vs `SystemPathBuf` (typed UTF-8 paths), `camino` UTF-8 paths, `relative-path` crate.

Focus: capability handle vs validated path newtype tradeoffs on macOS/Linux/Windows; what a CLI that only ever writes templates + reads a vault actually needs; UTF-8 path typing vs OS-bytes storage for persistent caches; whether the confinement type should be carried into long-lived structs or only applied at the I/O boundary.

Informs: path and capability model, index read model (persisted path form), crate topology.

Deliverable: `.scratch/arch-design/research/09-capability-path-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at [.scratch/arch-design/research/09-capability-path-precedent.md](../research/09-capability-path-precedent.md).
