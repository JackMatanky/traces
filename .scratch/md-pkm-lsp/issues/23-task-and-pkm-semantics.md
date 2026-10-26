# Task & other PKM-specific semantics

Type: grilling
Blocked by: 01, 02
Status: resolved

## Question

Grounding: `ListItem` (`src/note/lists.rs:64`) already models `kind:
ListItemType::Task(TaskStatus)` with `text`, `children`, local `fields`, and a
line-only `position: ListItemPosition`. `TaskStatus`/`TaskStatusMap` are
configurable via `[tasks]` in Config (checkbox-symbol to status mapping,
tag-filter classification). Query DSL already has a dedicated `Query
Mode::Tasks` (one row per task). Informed by Markdown Oxide/Marksman task
handling if any (tickets 01, 02) and the obsidian-tasks parity findings folded
into ticket 08. iwe ships no task intelligence either (amended 2026-10-21 by
iwe/fileclass research, [tool-iwe](../research/tool-iwe.md) Part 7).

Decide:

- Completion: task-status checkbox-symbol completion, informed by the project's
  configured `TaskStatusMap`, not a hardcoded set.
- Diagnostics: recognized-but-misconfigured task syntax (e.g. a checkbox symbol
  not in the configured map). Is this even diagnosable, or does it silently
  become non-task per existing `TaskStatusType::NonTask` semantics?
- Symbols: are tasks surfaced as document symbols / a task outline, and do child
  sub-tasks nest under parents (using the existing `ListItemPosition.parent:
  Option<SourceLine>` relationship)?
- Whether task date-shorthand emoji markers (mentioned in
  `src/note/GLOSSARY.md`) get their own completion/hover treatment, or fall
  entirely under ticket 18 (daily notes/date references).

## Answer

The grounding in this Question is wrong on two counts (corrected during
grilling): `ListItem` is at `src/note/lists.rs:40-48` with `kind:
ListItemType::Task(TaskListItem)` (`lists.rs:284`). Hierarchy comes via
`parent`/`depth`, not `children`, and `ListItemPosition` was dissolved (commit
`50bdc493`). Unknown checkbox symbols do **not** become NonTask. They resolve to
a synthetic Todo that keeps the symbol for diagnostics (`src/task.rs:142-153`).
NonTask is a configured kind classification that never consults. Full evidence:
[research/23-task-pkm-semantics.md](../research/23-task-pkm-semantics.md) (Parts
0 to XIV; XIV = four adversarial stress-tests).

Decisions (23 owns product content; mechanics deferred per the seams below):

**1. Completion - checkbox symbols (23 content, 24 dispatch).**

- Mid-typing `- [` position-gated completion: a buffer-local O(line) predicate
  (indent + bullet + ws + `[`). Offer only when the `scan_marker_prefix` state
  is `Incomplete` (`marker.rs:70-77,92-113`). `Rejected` falls through to 19's
  contexts; `Complete` goes to the field/value layer. The accept edit range =
  after-`[`..cursor. Items come from the configured `TaskStatusMap` (add
  `iter()`; ordering = config insertion order = `sort_text`). `preselect` is
  top. No unmarked/removal item. Invoked fallback +
  `TriggerForIncompleteCompletions` re-requests. No space trigger. Trigger
  registration and the shared completion-generation function are **24's** (24's
  Question owns the enumeration; every trigger claim here is a proposal into
  24). `scan_marker_prefix` is module-private and `TaskScan` is `pub(super)`.
  Widening that visibility rides 34.

**2. Completion - date values (23 owns candidates + content for both
spellings).**

- Claim date-value completion for `📅` AND `[due:: `. Both use the same
  `NoteFieldValue::Date` (`lexer.rs:30`). This closes a gap that 19 (keys only)
  and 20 (frontmatter only) provably do not cover. **Seam split: 19 = position
  predicate, 23 = candidates + value content, 24 = dispatch.**
- Candidates: obsidian's generic 12 as the parity set (`today` through `next
  year`) + `autoSuggestMaxItems: 20`. ISO dates today ±7 (needs the new `today`
  primitive, input to 18/`src/date.rs`). 18's NL-v1 phrases serve as labels.
  18's ±7 generator is design-only; if implemented, 23 consumes its algorithmic
  half. On-disk file matching is excluded for task lines.
- Suppression = token-based show/hide keyed on the **slot across both
  spellings** (first-wins, emoji precedence `task.rs:62-79`). Invalid
  occurrences stay visible (fix affordance), and accept emits a spanning
  `textEdit` (needs `byte_to_utf16_cu`, an 11/19 prerequisite that is currently
  absent).
- Emoji-token completion: label `📅 due date`, insert `📅 `, filterText keyword.
  The stage-2 value list follows after accepting `📅` via
  `triggerSuggest`/isIncomplete session. Completion gates on **marker state
  (marked)**, not classified-task, because filtered Checkboxes have emoji
  fields.

**3. Hover - content-gated, not blanket silence (render/kind goes to 26).**

- Mechanism: `ContentResolver::text_of` source line (14) + one
  `tokenize_item_text(line, Include)` pass + a marker-state read + a **raw
  substring scan** for glyphs the lexer skips (Exclude gate `lexer.rs:238-240`;
  non-ISO skips `:241-262`). Three cases: task lines (accessors), plain bullets
  with `📅` (visible-and-inert; raw scan only), filtered Checkboxes (the gate is
  marker state, not taskness). Ancestors = the list-parent chain only. Heading
  context is deferred (no `Heading` node). Markup kind, caps, and trust go to 26
  (isTrusted is client-side and cannot be detected server-side). Relative-date
  rendering depends on `today` (18).

**4. Diagnostics - existence + relative severity only; 25 owns
taxonomy/publish.**

- Set: A1 unknown symbol (Hint; the CLI report is primary; client survival goes
  to 29), B1 invalid date (Warning), B2 emoji-without-date (Warning, marked
  items), B3 duplicate slot (**conflicting values only**, Warning; B1 suppresses
  B3 on the same glyph pair). B5 unparseable task field (Warning, task keys
  only, `as_date().is_none()` precision, no double-fire with 19's structure
  checks), B6 shorthand-stripped-but-inert on plain lines (**default-off**;
  vault-calibrated: 9/9 hits are intentional), C1 overdue (Warning; needs
  `today` from 18), C2 due-today (Information), C4 status/date mismatch
  (Information, O(1)), C5 date-ordering (Information), C6 stale subtask
  (Information; Done parents only, Cancelled excluded, `fully_complete` counts
  descendant tasks only), D1 missing-space `[x]task` + D2 rejected `[]`/`[xx]`
  (Warning; Plain-item raw, first line, with code + link exclusion; the
  detection path must specify raw vs source-line). **B4 (unread emoji) is
  dropped**: the definition is unrecoverable and readings collide, so use hover
  instead.
- Detection caveat: code spans are not on the AST, so raw scans need
  code-exclusion specified. C1/C2 exclude Done **and** Cancelled.
- Every severity reads "23 proposes; 25 decides" (severity framework + Hint=4
  confirmed; per-source override precedent = 20:Q14). Conventions: source
  `traces-tasks` + per-rule codes, never `MD…` (32's boundary). Toggles +
  `[tasks.diagnostics]` go to 31 (`deny_unknown_fields` means the key cannot
  land early; the interim valve is the editor override).
- Delivery: push for open docs via 14's settled 300ms debounce. The cross-file
  panel uses a **recommendation** of pull `workspace/diagnostic` + `resultId`
  caching; push-vs-pull is 25's open question, capability gating is 29's, and
  the workspace-scope default severity goes to 25.

**5. Symbols - S3 outline with corrected mechanics (content 23, mechanics 27).**

- Shape: hierarchical, Task items, `SymbolKind::Array(18)` (floor-safe; the
  rationale is independent hideability from headings, which must be String(15);
  this is a 23-to-27 kind dependency), line-granular ranges.
- Nesting: a **single pass over `note.lists()` with an ancestor stack**. Each
  Task attaches to the nearest Task ancestor, else to the root. Plain
  intermediates are traversed, not emitted. Never use `Note::tasks()` + depth,
  because that mis-nests: filter-to-Task detaches tasks from Plain parents
  (fixture `lists.rs:649`).
- Ranges: leaf `(line,0)..(line+1,0)`; parent
  `(parent.line,0)..(last_descendant.line+1,0)` (last descendant *start* line
  +1; no end-line field exists). `selection_range ⊆ range` is asserted; the
  equal-line degenerate case is pinned by a test.
- `detail` ≤64 chars from `clean_text()` name + status name + priority +
  complete/incomplete (bool rollup; **no counts in v1**; v2 is priced as one
  reverse pass) + a single Due ISO. `"(untitled)"` is the fallback for empty
  clean. detail is lost on flat SymbolInformation fallback (29's minimum-viable
  client). Node cap 1,000 soft / 5,000 hard + post-cap behavior.
- Serve via 14's `ContentResolver` + a request-start `Arc<WorkspaceIndex>`
  snapshot (map.md:22). Staleness = 14's 150ms debounce. Range precision (byte
  spans) = 11/41. Registration, code-action plumbing, and VS Code
  built-in-provider coexistence = 27/29 (coexistence is currently unowned and
  must land before any symbol shape ships). Heading nesting goes to 27, blocked
  on 11's `Heading`.
- Status-cycle code action: 23 defines the invariant (*cycle order = config
  insertion order = completion sort_text order*). 27 owns plumbing. "Steps over
  symbols vs kinds" stays 27's open sub-question.

**6. Workspace/symbol task picker - claimed conditionally (resolves research
open call A3).**

- Conditions: (a) `Amends 33`: task rows join the always-resident tier *or* 33
  prices vault-wide refill (full reload 15.7-19.6ms baseline / 21.9-28.9ms
  current; rebuild 406ms@20K). (b) 33 gains a **proposed, unfilled**
  `workspace/symbol` p95 row derived from `bench_query_tasks_density`, with the
  mean-to-p95 factor stated. (c) Cap + ranking are assigned to 27/29: one shared
  cap across tags 16/headings 27/tasks 23, one agreed SymbolKind policy +
  containerName convention (16's PROPERTY-vs-STRING is unresolved).
- Serve via a direct walk over `note.lists()`/`Note::tasks()` +
  `is_task()`/`clean_text()`. This bypasses QueryService only to avoid
  `QueryRow` materialization and the linear `matched_file_rows` scan
  (`query/service.rs:305-316`), **not** to build a parallel task model. uri-only
  locations + `workspaceSymbol/resolve` with a **full-Location fallback when
  `resolveSupport` is absent** (nobody has checked this client capability; it is
  input to 29). Snapshot + overlay via 14.
- Hard needs, not optimizations: without `$/cancelRequest`/cooperative
  cancel/`$/progress` (12), the walk cannot be stopped. Cap 128 (16 precedent) +
  residency defend interactive latency (rust-analyzer 50s incident). No
  `WorkspaceSymbol.detail`.

**7. Performance (surfaced per map.md:33; `Amends 33`).**

- Resident/line-local features rate ignore-the-budget on measured substrate
  (prose 0.069ms/10KB; 1,000-item list 1.03ms; 50K-item clone 0.81ms = *floor*,
  symbol build unmeasured; diagnostics ≪1ms = a labeled estimate; no lint bench
  exists).
- Tension: measured 51.2KB long-line reparse 18.6ms exceeds 33's <10ms
  single-file target. The bench self-flags this as an unexplained anomaly
  (`note_parsing.rs:470-476`); root cause is owed to the implementation spec
  (planning-only). Corrected: the bench param is line *length* at fixed bytes,
  not line count; a realistic 4,000-line note ≈2-6ms.
- `[tasks]` config change = full reparse + re-persist + atomic Arc swap ≈
  **48/90/466ms @1K/5K/20K** (build 11.3/50.6/406 + persist 36.4/39.6/60.1).
  This is not 13's single-file `RefreshPlan`; content edits are unaffected.
  Config detection = 15-50ms background sweep. Same-config-generation reads
  across completion/diagnostics/classification (10's facade + 31's reload). All
  debounce figures are provisional and go to 25.

**Seams / deferred (prose inputs; `Blocked by: 23` edges added to
24/25/26/27):**

- To 24: dispatch, trigger registration, shared completion-generation function,
  `[`-context ambiguity at `- [t`.
- To 25: severity taxonomy, publishing, debounce (final tiers), push-vs-pull,
  day-rollover re-evaluation, workspace-scope defaults, B1/B3 precedence.
- To 26: MarkupContent kind/capability/dispatch, hover mechanics, caps, trust.
- To 27: symbol mechanics, ranges/precision seam, registration, cycle
  code-action plumbing, coexistence owner, heading nesting (blocked on 11).
- To 29: capability gating (pull diagnostics, resolveSupport fallback, Hint
  survival), minimum-viable client degradation.
- To 31: `[tasks.diagnostics]` shape + toggles + reload mechanics; record zk
  per-kind project config vs 20:Q14 tension.
- To 18: `today` primitive (local-vs-UTC, hoisting, day-rollover) as input,
  since C1/C2 are blocked on it; candidate generator interface.
- To 32: rumdl non-overlap assertion for task diagnostics. To 34: predicate
  extraction, `TaskStatusMap::iter()`, visibility widening. To 16: row-shape
  conventions shared with 23.

Hidden dependencies registered: `byte_to_utf16_cu` absent (11/19 prereq),
`today` absent, `Heading` absent (11), `scan_marker_prefix`/`TaskScan`
visibility (34), tower-lsp-server not yet in Cargo.toml (09), 18/24/26 open.

### Amended by 25 (2026-10-15)

- **Severities adopted verbatim, closing "23 proposes; 25 decides" (25 Q5-R)**:
  A1 = Hint; B1/B2/B3/B5/D1/D2/C1 = Warning; C2/C4/C5/C6 = Information. **B4
  dropped** (confirmed). **B6 default-off** (registry default `off`; the user
  enables it via an override key). C1/C2 still need `today` (18's timer, amended
  by 25).
- **Source and codes (25 Q6-R/Q15-R2)**: `source: "traces"` (Scheme C;
  `traces-tasks` retired) + per-rule codes under `tasks.*` (e.g.
  `tasks.date.invalid`), never `MD…`.
- **Delivery line (25 Q1-R/Q3-R')**: "push for open docs via 14's settled 300ms
  debounce" becomes: document-level **pull is primary** (push only as fallback
  for pull-incapable clients); the T2 publish timer (for push fallback and cache
  freshness) = ~50ms adaptive clamp. The cross-file panel recommendation (pull
  `workspace/diagnostic` + `resultId`) is now the Phase A/B primary mechanism,
  not a recommendation.
- **23:37 receipt**: "workspace-scope default severity → 25". Workspace-scoped
  task diagnostics (C1/C2 relative-date classes) inherit their per-rule defaults
  from the registry (Warning/Information as adopted above). No separate
  workspace-severity tier exists (per-code O1 overrides apply uniformly, 31's
  schema).

## Input from 27 (2026-10-22)

- Workspace/symbol serving: 27 ratifies eager full `Location` responses and NOT
  advertising `workspaceSymbol/resolve`. This proposes an amendment to 23's
  Section 6 uri-only+resolve+fallback design; implementation ownership stays
  with 29. Task children in document symbols follow 23's S3 pin. It is a
  single pass over `note.lists()` with an ancestor stack (each Task attaches to
  the nearest Task ancestor, else to the root; Plain intermediates are
  traversed, not emitted; never `Note::tasks()` + depth, per 23:43's
  mis-nesting fixture), fed by 27's walk and never QueryService (`QueryMode` is
  pub(crate), a 34 crate-split hazard). 27's research first phrased this as a `Note::tasks()` +
  depth/parent walk; it was corrected 2026-10-22 to 23's pin (research Part
  XII.1 amended in step with this block). Tasks also appear in the workspace
  picker inside the shared merged cap 128. This restates 23's conditional A3
  row at the 10k vault tier (<50ms per 27's row set). The row's original 20K-anchor
  derivation and its mean-to-p95 factor remain open per 23:50(b)/33:117.
