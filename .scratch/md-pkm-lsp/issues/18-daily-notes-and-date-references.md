# Daily notes & date-based references

Type: grilling
Status: resolved
Blocked by: 04

## Question

This ticket uses zk's daily-note/date conventions research (ticket 04) as input.
Decide:

- Does Traces recognize a "daily note" as a distinct note kind at the
  language-service level? Recognition can happen via filename date-pattern, a
  Schema File Class, or a dedicated config field. Or is this purely a
  Template/Config-level convention (existing `[frontmatter]` canonical metadata
  roles, `TemplateVariable` `date`) with no LSP-specific semantics needed at
  all?
- Date-shorthand references appear in link/task context (Traces' Task model
  already has "date-shorthand emoji markers" per `src/note/GLOSSARY.md`). Does
  the LSP offer completion/validation for these? Is that the same mechanism as
  generic date-field completion (ticket 19/20), or a distinct one?
- If daily notes are in scope: handle a date-shaped wikilink target that does
  not yet exist on disk (for example `[[2026-09-04]]`). Does hover/definition
  offer to create it (code action, ties to ticket 28), and from which template
  (ties to ticket 22)?

## Answer

**Resolved:** 2026-09-18 **Research:**
[18-daily-notes-and-date-references.md](../research/18-daily-notes-and-date-references.md)

### Decisions

1. **LSP-aware daily notes.** Traces recognizes date-shaped filenames as a
distinct note kind at the LSP level. Index-time extraction puts `is_date_shaped:
bool` + `parsed_date: Option<NaiveDate>` into `FileEntry` (not `FileRecord`,
which does not exist yet). Cost: ~0.5ms for 20K files. **Redb caveat (reconciled
2026-09-23)**: `FileEntry` is persisted in the `FILES` table. Adding these
fields changes its encoding and triggers the existing `check_rebuild_needed`
wipe-and-rebuild once (one-time user-visible index rebuild, accepted under
ticket 13; flag it in the eventual implementation spec).

2. **Configurable matching per granularity.** Four modes: `file` (stem matches
format pattern, default), `folder` (file lives in configured folder). `both`
(stem + folder), `path` (full vault-relative path matches
`folder/<date-stem>.md`). This covers flat vaults and split-folder setups.

3. **All periodic granularities.** Config supports
`day`/`week`/`month`/`quarter`/`year`. LSP features (completion, hover,
definition) start with daily and extend incrementally. Config is
forward-compatible.

4. **Hybrid completion with cap.** Generate dates algorithmically (today ± 7
days with relative labels: today/tomorrow/yesterday/next weekday/last weekday)
and match them against on-disk files. Cap algorithmic candidates to avoid 400+
item lists.

5. **Natural language dates in v1.** A custom ~150-line parser (zero
dependencies) handles: today/tomorrow/yesterday, next/last weekday, ±Nd/Nw/Nm.
Full NL parsing (chrono-node equivalent) defers to v2. No viable Rust crate
exists for full NL; the custom parser matches Markdown Oxide's proven approach.

6. **Diagnostic-driven creation.** Definition returns `null` for nonexistent
date notes (no precedent for Location-on-nonexistent in major LSPs). Hover shows
"Note not found" (informational, no creation prompt). A code action on the
diagnostic offers "Create from template {X}"; that is the action surface.

7. **Past-date-only diagnostics.** Diagnose date-shaped links where `date <
today` and no matching file exists (likely forgotten/deleted). Future dates are
intentional forward references, so they get no diagnostic.

8. **`[periodic]` config designed from scratch.** Use short granularity names
and flat independent-per-granularity sub-tables. `format` is optional with
per-granularity defaults (`%Y-%m-%d`, `%G-W%V`, `%Y-%m`, `%Y-Q%q`, `%Y`).
`folder` and `template` are optional. `match` defaults to `"file"`. No `enabled`
field: a granularity is enabled when its table exists.

9. **Explicit reuse.** Builds on `DateValue` (existing),
`FileEntry`/`WorkspaceIndex` (existing), `LinkResolver` (ticket 15), inverted
index (ticket 33), `RefreshPlan` (ticket 13). Completion dispatch defers to
ticket 24.

### What NOT to build

- No frontmatter date parsing for daily note resolution (filename sufficient).
- No Dataview-like query language in v1 (ticket 21+).
- No creation prompt on hover (diagnostic-driven only).
- No diagnostic for future-date unresolved links (intentional forward
  references).
- No full NL parsing in v1 (custom parser covers common patterns;
  `chrono-english` for v2).

### Amended by 25 (2026-10-15)

- **Day-rollover timer (25 Q12-R; 23 routed re-evaluation here)**:
  date-dependency invalidation is a single background timer armed to the next
  local-midnight boundary. On fire: bump a generation, then re-enqueue
  date-dependent diagnostics for the affected set (daily notes + their
  referrers, class D in 25's dependency table). Recompute+publish rides the T3
  event-driven path (β lane for open docs; pull-cache invalidation for the
  workspace). There is no polling loop and no periodic re-scan. This covers 18's
  past-date unresolved links and 23's C1/C2 relative-date severities (both need
  `today`).

### Amended by 26 (2026-10-18)

- **Date links are three-state (`Unresolved | Unique | Ambiguous`)**: this
  ticket's folder/both/path match modes can produce stem ties. Therefore,
  `DailyDate` CAN be `Ambiguous` (this corrects 26's early "DailyDate never
  ambiguous" draft). Behavior per 26's consumer matrix: definition returns all
  candidates
  and raises no diagnostic; hover is informational on `Unresolved`;
  prepareRename/rename **refuse with `Ok(null)` regardless of whether the target
  note exists** (a DailyDate reference is never renamable, existence or not).
