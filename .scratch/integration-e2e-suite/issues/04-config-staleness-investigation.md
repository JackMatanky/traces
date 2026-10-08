# 04: Config-change staleness investigation

**What to build:** The config-change staleness question is answered with a traced, deterministic
reproduction and a recorded intended-behavior decision — before more config-sensitive tests are
written that might depend on the wrong answer. If the product behavior is wrong, the fix goes to
a separate spec; this ticket only decides and, where behavior is confirmed correct, adds the
narrowest regression.

**Category:** bug

**Blocked by:** 01 (its possible regression-test AC changes code and must diff against the
baselines; the repro and reading may start any time — the spec places this investigation at the
`(gap ∥ investigation)` node, i.e. parallel with the gap-test phase, not before it).

**Status:** resolved

- [x] Deterministic repro executed and traced end-to-end, with the trace recorded under
      `## Comments` in this ticket file: config A → `traces trust` → `traces
      index` → edit `class_field` / tag key / task filter → **re-trust** (mandatory — stale
      config blocks all loads until re-trusted) → query untouched notes
- [x] Symptom shape documented per selector, recorded under `## Comments`: class-field changes
      give false negatives (current-field re-check masks stale rows); tag/task-filter changes
      give false positives (persisted tags trusted; task parsing bakes config at index time)
- [x] Intended-behavior decision recorded: agent proposes a disposition with evidence, appended
      under `## Comments` in this ticket file; **the maintainer decides** correct-as-designed vs
      product defect (this is a judgment call, not an agent verdict)
- [x] If correct-as-designed: narrowest regression test added — in-crate unit at the
      refresh/freshness seam by default, one E2E canary only if process composition matters —
      carrying the executes-vs-asserts note and named defect class. If a defect: no pinning
      test, separate spec
- [x] Repro recipe recorded as **forbidden as an assertion** anywhere else (it would pin a
      potential defect); the staleness recipe is not used by ticket 12

**Evidence:** audit §10 potential-defects (RefreshPlan fingerprints only file metadata; class
dimensions supplied only on the Stale arm); §15 plan item T3b.1 (§14 recommendation item 19).
**Defect class:** potential product defect (decide, don't fix, don't pin).

## Comments

### Repro trace (2026-10-09, dev build `traces 0.1.0`, ticket-04 worktree)

Deterministic recipe: one note, three selectors present from the start
(`class: Book`, `kind: Novel`, `tags: alpha`, `labels: bravo`; body tasks
`one #todo`, `two #urgent`), config A pins `class_field = "class"`,
`tags = "tags"`, `tag_filters = ["todo"]`. Per selector: `trust` → `index`
(baseline queries sanity-checked) → edit one selector in
`.traces/config.toml` → re-trust → query untouched notes → `index` → re-query.

Traced runs (exit / stdout; state dirs isolated via `TRACES_STATE_DIR` +
`XDG_CONFIG_HOME`):

- **Baseline (config A):** `list --from @Book` → `0 / - a.md`;
  `list --from #alpha` → `0 / - a.md`; `task --count` → `0 / 1`.
- **Load blocking (any selector edit, before re-trust):** every command exits
  `1` with `config file …/.traces/config.toml is untrusted: Stale` and
  `help: run `traces trust` to trust this project root, then try again`.
  Re-trust (exit `0`) clears the block. Confirms "stale config blocks all
  loads until re-trusted".
- **class_field `class` → `kind` (re-trusted, no re-index):**
  `list --from @Novel` → `0 pages` — **false negative** (`kind: Novel` is in
  the frontmatter; the persisted `paths_by_file_class` axis was built under
  `class`, so `novel` never became a candidate key);
  `list --from @Book` → `0 pages` — stale candidate still resolves via the
  old axis key, but the current-field re-check (`class_values(note,
  class_field)` at `src/query/grammar/source.rs:225`) masks it. After
  `traces index`: `@Novel` → `- a.md`, `@Book` → `0 pages` (healed).
- **tag key `tags` → `labels` (re-trusted, no re-index):**
  `list --from #bravo` → `0 pages` — **false negative** (axis built under the
  old key); `list --from #alpha` → `- a.md` — **false positive** (persisted
  note parse from the old key is trusted by the tag re-check). After
  `traces index`: `#bravo` → `- a.md`, `#alpha` → `0 pages` (healed).
- **task filter `["todo"]` → `["urgent"]` (re-trusted, no re-index):**
  `task` → `- [ ] one (a.md)` — **false positive** (task classification baked
  at index time; Fresh-arm refresh reuses persisted parses);
  `two #urgent` missing — **false negative**. After `traces index`:
  `- [ ] two (a.md)` only (healed).

Repro scripts used for the trace are throwaway (`/tmp`); not committed.

### Symptom shape (per selector)

- **Class-field changes → false negatives only.** Candidate resolution reads
  the persisted class axis (old field); the query-time re-check uses the
  *current* field name, so stale old-field rows can never resurface as
  matches — they are silently masked. New-field values are invisible until a
  rebuild or a content-triggered refresh of the file.
- **Tag-key / task-filter changes → false positives (and false negatives).**
  Persisted note parses are trusted as-is on the Fresh arm (metadata-only
  fingerprint), so old-key tags keep matching and old-filter task
  classifications keep being listed; new-key/new-filter values are invisible
  until rebuild.

### Intended-behavior disposition (agent proposal — maintainer decides)

**Proposal: correct-as-designed; heal via the existing explicit contract
(`traces index`); add the narrowest in-crate regression at the
refresh/freshness seam. No product change.**

Evidence:

1. The config-staleness protocol already gates the dangerous window: any
   config edit blocks every load (exit 1, `untrusted: Stale`) until an
   explicit re-trust. Re-trust's contract is the O(1) content-hash baseline —
   it acknowledges the edit; it does not promise re-derivation.
2. The persisted index is a derived cache of (files × config). The system's
   documented healing path — `traces index` rebuild; incremental refresh
   re-derives any file whose metadata changes — bounds the stale window to
   byte-identical files and heals it in one explicit command (verified above).
3. Auto-rebuilding on re-trust would conflate two command contracts (trust =
   baseline update, index = derivation) and impose an unrequested full parse
   on large workspaces; that is a policy decision, not a bug fix.
4. The Fresh arm is metadata-scoped by design (`IndexDimensions` supplied only
   on the Stale arm), and class selectors re-check against the current field
   name at query time, so stale class rows cannot silently resurrect.

Residual risk, stated honestly: tag/task **false positives** persist silently
until the next rebuild or content edit. If the maintainer judges this a
defect, candidate mitigations (all product changes → separate spec): (a)
re-trust warns when a persisted index exists; (b) re-trust triggers a rebuild;
(c) the store records a dimensions fingerprint and refresh reports drift.

### Maintainer ruling (2026-10-09)

**Correct-as-designed.** The maintainer accepted the proposed disposition:
no product change; the healing contract stands (`traces index` rebuilds
selector-derived rows; re-trust only clears the load block). The narrowest
regression was added in the ticket-04 worktree, in-crate at the
refresh/freshness seam:
`index::service::tests::class_field::fresh_refresh_serves_persisted_class_axes_unchanged_when_class_field_config_changes`
— it pins the Fresh arm's metadata-scoped freshness only (persisted class
axes served unchanged, zero rows re-derived, empty `RefreshReport` when only
`class_field` differs) and never the stale-row symptom. No separate spec
required; the residual-risk mitigations above remain recorded for future
product consideration.

### Repro recipe is forbidden as an assertion

The staleness recipe above must not become an assertion anywhere in the test
suites: it would pin the behavior now under judgment. Ticket 12 must not use
it. If the maintainer rules correct-as-designed, the narrowest regression
(in-crate, refresh/freshness seam) pins only the Fresh arm's metadata-scoped
freshness — never the stale-row symptom itself.
