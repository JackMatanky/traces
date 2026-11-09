# How do mature tools enforce root confinement and portable path identity?

Type: research
Status: resolved
Blocked by: none

## Question

Root confinement is enforced by `strict-path` at construction
(`src/path.rs:181-189`). But `SafeRelativePath` returns a plain join with an
acknowledged TOCTOU window (`:159-161,190`). `FileMeta::path` and `CliError`
fields degrade back to raw `PathBuf`. Template path inspection accepts absolute
paths (`src/template/engine/path.rs:162`). DryRun skips confinement
(`src/template/service.rs:225`). The redb path codec stores raw OS bytes/wide
units. This makes the cache non-portable (`src/path.rs:340-437`). See
`survey/root-primitives.md` H7, `survey/cross-cutting.md` H4,
`survey/template.md` H4/H5.

Study these comparables:

- `cap-std`/`cap-primitives` (`Dir`, `open_beneath`, Windows strategy)
- Linux `openat2(RESOLVE_BENEATH)`
- `strict-path` own source (what it actually guarantees and where it cannot)
- gitoxide `gix-path`/`gix-fs`/`gix-worktree` path-safety checks
  (`gix-validate`, protect-NTFS/HFS)
- Cargo `cargo-util::paths` + `restricted_names`
- Ruff `ruff_db::system::SystemPath` vs `SystemPathBuf` (typed UTF-8 paths)
- `camino` UTF-8 paths
- `relative-path` crate

Focus:

- Capability handle vs validated path newtype: tradeoffs on macOS/Linux/Windows?
- What does a CLI that only writes templates and reads a vault actually need?
- UTF-8 path typing vs OS-bytes storage for persistent caches?
- Should the confinement type be carried into long-lived structs, or only
  applied at the I/O boundary?

Informs: path and capability model, index read model (persisted path form),
crate topology.

Deliverable: `.scratch/arch-design/research/09-capability-path-precedent.md` (<=
300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at
[.scratch/arch-design/research/09-capability-path-precedent.md](../research/09-capability-path-precedent.md).
