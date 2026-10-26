# Pre-grilling brief: ticket 22 (config ownership and lifecycle)

Inputs, round 1 (evidence): [a1](./a1-inventory-type-placement.md), [a2](./a2-inventory-digests-lifecycle.md),
[a3](./a3-inventory-trust-tracking-init.md), [b1](./b1-prior-decisions-envelope-schema.md),
[b2](./b2-prior-decisions-context-research-downstream.md), [c](./c-digest-lessons.md),
[d1](./d1-external-trust-tracking-lifecycle-precedent.md), [d2](./d2-external-fingerprint-config-ownership-precedent.md).
Round 2 (designs, written independently): [e](./e-design-type-placement.md) (a, b, f),
[f](./f-design-fingerprint-and-lifecycle.md) (c, e, loader), [g](./g-design-trust-tracking-scaffolding.md) (d, g).
Round 3 (attack): [h red team and interface check](./h-redteam-and-interface-check.md),
[i fact audit](./i-fact-audit.md). Tags: [V] verified in source or document, [H] inference, [P] proposal. Nothing here is a
decision; it is the menu for the grilling. Baseline HEAD `8064cf76`. Format follows
[21-prep](../21-prep/brief-r3.md). Names in sketches (`HostInputs`, `NoteParserConfig`, `TrustManager`) are placeholders.

## 0. Things to know before the first question

1. **Ticket 21's fingerprint obligation is unmet in code.** [V, i #1] `IndexFingerprint` is `[Blake3Digest; 10]`
   (`src/index/fingerprint.rs:143`): the 10 consumed setting digests only. It has no ignore policy, no filename-period
   parse policy, no schema or tool version. Ticket 21's Answer (`:93-95`) says freshness must include both. Sub-question
   (c) therefore cannot be answered as "retire the dead digests"; it must also say where those two inputs enter.
   Today an edit to `TRACES_IGNORED_DIRS` does not trigger a rebuild [H; confirm in grilling].
2. **`ConfigDigests` dead surface is exact.** [V, a2, i] 5 of 15 digests (root, templates local/global/output, schemas
   directory) are computed and discarded on every `IndexerService::from(&Config)`; they are excluded on purpose so a
   moved vault keeps its cache (`fingerprint.rs:140-141,160-164`). `Config::hash`/`compose` have no non-test caller.
   `digest.rs` is 380 lines. **Deleting it does not drop `syn`**: a second scan test lives in
   `src/index/fingerprint.rs:506-552`. Earlier claims that C3 "drops the `syn` dev-dependency" are wrong unless both go.
3. **`traces init` leaves the new workspace untrusted and untracked.** [V, a3, i #2] The next command fails with
   `ConfigTrustError::Untrusted` until the user runs `traces trust`. This is current behaviour, possibly intended
   (security), possibly a trap. Ticket 16 freezes trust-gate behaviour (`:82`), not the init flow.
4. **Ticket 16 does not block removing `traces tracked`.** [V, `issues/16:76-83`] Subcommand spellings and flags are
   Migratable, the symlink layout is Free, and ADR-0002 says `traces tracked` "was delivered beyond MVP scope"
   (`docs/adr/0002:131-132`). The red team's "REFUTED because Ticket 16 compatibility" (h L7) is wrong on that ground. The real
   question is value: what the tracker gives a user (below, D).
5. **The note parser needs 3 config values, not two config structs.** [V, a1, i #6] `parse_markdown` reads
   `tag_filters: &[Tag]`, `statuses: &TaskStatusMap`, `tags_name: &str`; 4 of 5 `FrontmatterConfig` fields are never read
   by it; inline-field settings do not exist in `Config`. The only `note -> config` import is
   `src/note/parser/input.rs:9`.
6. **`field.rs` holds two disjoint hierarchies.** [V, a1, i #7] `FieldName`/`FieldValue` (schema/config identity, 0 note
   callers) and `FieldKey`/`FieldValueRef` (note/query identity, 0 config callers). `tag.rs` was extracted from `note` to
   break `query`/`index -> note` edges (`07b9f7de`); `task.rs` was hoisted from `note/lists.rs` for thematic grouping
   (`f60cc1b3`). So "why at root" has a history answer: tag yes (a real cycle break), task and field no.
7. **`file_tracker` has three config callers, not two** [V, i]; none outside `config`. `dirs`/`env_vars` have no consumer
   outside config and cli except `env_vars::is_ignored_dir`, which the index walker calls (`src/index/service.rs:424`).
   Folding `env_vars` into `config` would create an `index -> config` edge.
8. **Latent defect in the stores.** [V, i #5, reproduced] `FilePathTracker::record` checks `exists()` (follows symlinks)
   then `symlink()`; a dangling entry returns false and then fails with `EEXIST`. Any option that keeps the tracker needs
   `symlink_metadata` or temp-symlink-then-`rename`. Findings-ledger candidate, not an architecture decision.
9. **Global config has no trust gate, no hash, no tracking.** [V, a2 §3.3] Only local config is gated. Ticket 21 already
   says global freshness is a separate path; 22 must say what that path is.
10. **The designs lean toward deletion.** They were written by agents prompted to evaluate `delete`. The red team amended
    five leans and refuted two. Those amendments are in section 2; the underlying evidence for each is in h, but I
    accepted h's verdicts only where I could check them (item 4 above; the trust-on-init predicate in (g) is my own [H]).

## 1. Three structural facts that change the discussion

- **Sub-questions are mostly independent.** [H, e/f/g §6] (b) and (a) share only the `TaskStatusMap` placement; (c)
  depends on where `IgnorePolicy` and `DateConfig` land; (d) and (g) share the init-then-trust composition; (e) is
  internal to config but shapes the loader that 30 will host. Suggested order is in section 5.
- **Ticket 20 and 21 constrain the loader shape more than the typestate question.** The loader must: take a host
  snapshot, hash-verify local config before parse, recheck at request admission, give admitted requests an immutable
  generation, treat global freshness separately, and run without process globals. `ConfigLocator::from_env` is called
  at two production sites (`ConfigService::load`, `trust_subjects`) [V, a2 §3.2]; those are the bypasses of
  "injectable".
- **22 must not decide what 23, 30, 32 own.** [V, h §4] The red team flags that `NoteParserConfig`'s exact fields
  belong partly to ticket 23, that any crate split belongs to 32 (map steer: evaluate, not decided), and that held-workspace
  scheduling belongs to 30.

## 2. What the grilling must decide (each independent unless noted)

Leans are [P] from designs e/f/g, after the red team's amendments (h §5). "Keep" is always listed. Dissent and flip facts
are the strongest counter found.

### (a) Root `task.rs` / `tag.rs` / `field.rs`

| Option | Shape | Notes |
|---|---|---|
| a1 keep | all three at root | `field.rs` still holds disjoint hierarchies; `task.rs` (1,911 LoC) holds config-shaped (`TaskStatusMap`) and note-shaped (emoji tables, `TaskDate`) code together |
| a2 split (lean, amended) | `TaskStatusSymbol/Type/Map` to a leaf `task_status.rs`; emoji/date AST to `note`; `FieldName` to a Tier 0 leaf (NOT `note`); `FieldKey`/`FieldValueRef` to `note`; `tag.rs` stays Tier 0 | Red team: `FieldName` in `note` creates `config -> note` and `schema -> note`, a cycle risk (h C3). `TaskItem` provisional until 23 |
| a3 `domain/` leaf module | one module for tag, status, field name | Same edges as a2, fewer root entries; flat-vs-grouped is taste (e Q1) |
| a4 crate split | `traces-core` primitives crate | Red team: out of 22's remit (32). Keep as input to 32 |

Dissent: a1 is coherent if 23 replaces `TaskStatusMap` with interned statuses (then a2's work is wasted). Flip fact: 23's
status model. Value question (e Q3): shrink `TaskStatus` to `(symbol, type)` and resolve names via the map (~10k fewer
heap `String`s per large vault, i #21)? That is a 23/24 call; record as handoff unless you want it here.

### (b) The `note -> config` edge

| Option | Shape |
|---|---|
| b1 keep | `MarkdownParserInput` borrows `&TaskConfig` and `&FrontmatterConfig` |
| b2 consumer-owned (lean, amended) | `note` defines `NoteParserConfig`; `IndexerService` projects it from `Config` (rust-analyzer `*Config`, IWE `MarkdownOptions`, zk `ParserOpts`: c, d2) |
| b3 primitives | `parse_markdown(.., statuses, tag_filters, tags_name)` |
| b4 pass `&Config` | widening; rejected by every precedent |

Amendment: use owned/cloneable fields or deconstructed args so 23 is not bound to a borrow shape. Dissent: b3 is the
smallest change if the parser stays 3-valued and 23 rewrites the input anyway. Flip: 23 adds inline-field or
callout settings (then a typed struct pays off).

### (c) Residual `ConfigDigests`

| Option | Removes | Keeps |
|---|---|---|
| C1 keep | nothing | dead `Config::hash`, 5 dead digests, both `syn` scans |
| C2 trim | 5 dead digests, `hash`, `compose`, `traces.config.v1` | 10-digest bundle in `config`, `syn` scans |
| C3 consumer-owned (lean, amended) | `digest.rs` (380), per-section `.digests()` (~180), `Config::hash`, config-side `syn` scan; index scan too if you accept exhaustive destructure | `IndexFingerprint` built by `index` from an `IndexRelevantConfig` view that `config` exposes, so a new setting is a compile error |
| C4 add-on | none | a separate `ConfigContentHash` of the local `config.toml` bytes for per-request admission (21) and LSP reload dedupe |
| C5 single hash | the 10-array in `META` (320 B to 32 B) | on-demand dirty reason (Cargo) |

Obligation to settle in this ticket: where ignore policy and filename-period policy enter freshness (section 0.1), and
that zone does NOT (ticket 21: civil values persist, zone is evaluation-only). Dissent: C2 is the smallest change and
leaves the spec free to move to C3 once 25 lands. C3's `IndexRelevantConfig` view must sit where `index` imports it
without `index -> config` growing, which the view struct solves only if it is a plain data type owned by `index`.
Open (h): does exhaustive destructure alone cover settings that never live in `Config` (ignore set)? Flip: 25 chooses
whole-cache rebuild on any config edit (per-setting fingerprints then vanish).

### (d) `TrustStore` / `TrackerStore`

| Option | Shape | Invariant owner |
|---|---|---|
| D1 keep | two stores, `ConfigService` orders them | caller convention plus a test |
| D2 one registry | single store with `Seen / Trusted{hash} / Revoked` | the registry (trivially) |
| D3 generic `PathStore<Kind>` | removes duplication, two directories | caller convention |
| D4 trust only | delete `TrackerStore`, `src/cli/tracked.rs` (165), `tracker.rs` (244) | none needed |
| D5 `TrustManager` (red-team pick, as fallback to D4) | one private owner of both stores; `grant()` writes tracker then trust | `TrustManager`; unbypassable |

Facts: [V] the tracker has no engine consumer; only `traces tracked list|clean` read it. Ticket 16 permits D4 with a
change-table row. What D4 loses: a machine-wide listing of configs that were loaded or refused but never trusted
(`untrust` keeps the tracker entry on purpose, ADR-0002:71), and the ordering invariant disappears with it. What D5 keeps:
that audit trail, at the cost of two directories and one more type. Dissent against D4: ADR-0002 designed the tracker
for exactly the "refused config stays visible" case; the mise precedent (d1) keeps both stores. Dissent against D5:
it preserves a store nobody in the core needs. Flip fact (value, Q1): do you use or want `traces tracked`? D3 and D2
are the least supported (D2 loses symlink auditability; D3 only deduplicates). All options except D4 need the `EEXIST`
fix (section 0.8).

### (e) `ConfigFile<Source, State>` typestate

| Option | Shape | Removable |
|---|---|---|
| E1 keep | 5 markers over 632 lines; all transitions consumed in one 18-line private fn (`ConfigService::build`, `service.rs:95`); 0 callers outside config | nothing |
| E2 trust-only sealed newtype | `VerifiedLocalConfig` (private ctor, trust module) carries the hashed text into the parser; compile-checked spike exists in f | ~480 per f (unaudited) |
| E3 plain functions | order by one pipeline fn and tests | most |
| E4 `RawConfig -> ResolvedConfig` | two structural phases (rumdl style) | partial |

Amendment (h L5): E2 must also model `Global` vs `Local` provenance, because global has no trust gate; a `ConfigProvenance`
enum does that. Dissent: the typestate's single real guarantee (no parse before trust, with the hashed bytes reused so
there is no TOCTOU) is also what E2 provides, so E2 loses little; E1's cost is size, not wrongness. Precedent: d1 finds
none for a phantom typestate on config trust (gix-sec uses trust levels as data); c finds rumdl's
`SourcedConfig<ConfigLoaded/Validated>` (validation, not trust). Both reported as read; neither opposes E2. Flip: a host
(30) that needs to hold an unparsed-but-discovered config across requests would value an explicit state type.

### (f) `file_tracker` / `dirs` / `env_vars`

| Option | Shape |
|---|---|
| f1 keep | root modules, `LazyLock` globals |
| f2 (lean, amended) | fold `file_tracker` into `config` (private); replace `dirs`/`env_vars` globals with an adapter-built `HostInputs`; `IgnorePolicy` NOT in `dirtree.rs` (red team C1): `src/host/ignore.rs` or `config` |
| f3 `host/` module | `dirs`, `env_vars`, `ignore` together as Tier 0 host-adapter |
| f4 all into `config` | creates `index -> config` via `is_ignored_dir` |

Use `HostInputs` (start path, state home, config home, ceiling dirs, ignored dirs: f §4.2, which matches ticket 21
`:76-78`), not e's smaller `HostPaths` (h C2). Preserve path conventions exactly (ticket 21). Dissent: f1 is coherent for a
one-process CLI (21's own dissent); 21 already chose acquisition at the edge, so f1 survives only for tests. Flip: LSP
needs only one policy per process (then globals plus an explicit start path suffice).

### (g) Scaffolding `cli::init` (557 LoC, 277 tests, 280 production)

| Option | Shape |
|---|---|
| G1 keep in CLI | prompts, decisions, filesystem undo journal, default text all in `cli::init` |
| G2 core use case | `plan -> apply(undo)` over plain `InitOptions`; CLI keeps prompts |
| G3 canonical starter | default text produced by `RawConfig::starter()` so init output is checked against the parser; fixes the template-default asymmetry (ledger 19) |
| G4 plus template scaffold | `TemplateService` owns starter templates |
| G5 init-then-trust | `TrustPolicy {Auto, Prompt, Leave}` parameter on the use case |

Lean: G2 + G3 (core, canonical content). G5's auto-trust is the contested part. h's amendment: auto-trust only if no
template files pre-exist. I think that is too narrow [H]: ADR-0002's boundary is the workspace root, and a cloned repo can
bring `.traces/schemas/`, ignore settings or other content besides templates. A safer predicate is "`.traces/` did not
exist before init created it, and init wrote every file in it". Value question (Q2): is the post-init `Untrusted` error a
bug to fix, or a deliberate "you must review it" step? Flip: ticket 30 decides a host-level trust policy.

### Loader lifecycle (ties c, d, e to ticket 21)

f sketches three shapes: L1 `ConfigLoader::load(&HostInputs) -> Generation`; L2 `ConfigLocator` + `Resolver`; L3 a held
`WorkspaceConfig` with `admit_request()`. `ConfigLocator::from_env` is removed under all. Open item (f Q2 choice B,
"stale fallback: allow read-only queries on the previous generation") is **already ruled out**: ticket 21 says reject changed or
revoked local config instead of using the old parsed value. Do not ask it again. Debounce and watcher policy are 30's; 22
supplies `ConfigContentHash` plus a verify call. Relative-path base: local paths resolve against the project root, global
against the global config dir [V, a2 §3.4]; ticket 21 requires an explicit base or rejection of relative overrides.

## 3. Cross-design contradictions already resolved (do not re-ask)

| # | Contradiction | Resolution (h §3) |
|---|---|---|
| C1 | `IgnorePolicy` in `dirtree` (e) vs fingerprint input (f) | host-owned, snapshotted in `HostInputs` |
| C2 | `HostPaths` (e) vs `HostInputs` (f) | `HostInputs` |
| C3 | `FieldName` into `note` (e) vs consumed by config/schema | Tier 0 leaf |
| C4 | tracker folded (e), kept (f), deleted (g) | decide as sub-question (d) |
| C5 | `AutoTrust` (g) vs hash-verified admission (f) | auto-trust must write the hash-verified trust entry during scaffolding, with a predicate narrower than h's |

## 4. Questions only you can answer

1. **Q1.** Do you use or want `traces tracked` (list/clean of every config ever loaded, including refused)? Yes keeps D5,
   no allows D4.
2. **Q2.** After `traces init` in an empty directory, should the first command work, or must the user run `traces trust`?
   And may init trust a directory where `.traces/` already existed?
3. **Q3.** Is retiring `ConfigDigests` acceptable if its replacement is consumer-owned, and do you want any
   "which setting drifted" diagnostic, or is a single rebuild message enough? The rebuild-time claim in f ("sub-50 ms") is
   unmeasured [H]; research 11 measured parse costs, not rebuild.
4. **Q4.** Should ignored directories become part of the persisted index freshness now (ticket 21 says yes), changing
   the `META` layout? The layout is Free under ticket 16.
5. **Q5.** Should `TaskStatus` stop carrying an owned name (a 23 call), or stay as-is here?
6. **Q6.** Should `Local` vs `Global` provenance be a type, so a host cannot mistake the ungated
   global file for a trusted one?
7. **Q7.** Crate shape: leave all physical splitting to ticket 32 (red team and e's own recommendation), with 22 only
   making the module graph split-ready?

## 5. Proposed grilling script (a "yes" accepts one default, not the package)

Frontier 1 (independent, ask together):

- **F1 (b, a-part).** Define the parser inputs in `note`, project from `Config`, no `note -> config` import. Default: yes,
  owned fields, exact field set left to 23.
- **F2 (a).** Split `task.rs` into a leaf status module plus note-owned emoji/date code; `FieldName` stays a Tier 0
  leaf; `FieldKey` moves to `note`; `tag.rs` stays. Default: yes, subject to 23's status model.
- **F3 (c).** Which of C1 to C5, and where the ignore and filename-period policies enter freshness. Default: C3 + C4.
- **F4 (e).** E2 with a provenance enum. Default: yes.

Frontier 2:

- **F5 (d).** D4 or D5 (Q1). No default; this is a value call.
- **F6 (g).** G2 + G3, and the trust-on-init rule (Q2). No default for the trust rule.
- **F7 (f).** `HostInputs` built by adapters; `IgnorePolicy` host-owned; `file_tracker` private to `config`. Default: yes.

Frontier 3:

- **F8.** Handoffs and the independence list in 22's Answer; reconcile with 30 and 32 before any public type is frozen.

## 6. Handoffs 22 should write

| Ticket | 22 supplies | Left open |
|---|---|---|
| 23 note model | parser input field set (provisional), status model dependency | `TaskItem`, interned statuses |
| 24 / 25 index | fingerprint ownership (consumer-owned), ignore/period policy inputs, `ConfigContentHash` | rebuild strategy, store locking |
| 27 temporal | `DateConfig` surface and where it parses; zone not in fingerprint | grammar, week rule |
| 29 paths | relative-base rule, confinement points in config | capability model |
| 30 host | `HostInputs`, verify/admit call, trust API (`trust/revoke/status/list`) | scheduling, debounce, watchers |
| 31 errors | config/trust/init error sets | rendering |
| 32 topology | placement of each type, `pub` list | crate cuts |
| 35 / 36 schema | class `[binding]` parsing location in config | internals |

## 7. Evidence gaps

- No compiled spike for the loader shapes or `IndexRelevantConfig`; only the E2 newtype spike exists (in f).
- Rebuild time on a large vault after drift is not measured.
- Whether `traces tracked` has users is unknowable from the repo.
- `mise` CVE and tracking details in d1 were not independently re-checked by me; treat d1's external claims as read-only
  leads. i confirmed the repo-side facts.
- The red team's "refuted" verdicts are one agent's review, not a ratified outcome. I overrode one (item 4).

## 8. Process note

This prep ran in this folder (the convention of 19-prep, 20-prep, 21-prep) rather than the `.design/` tree that the
`rust-design` skill defaults to, because the output feeds a wayfinder ticket. Three agents died on a model retry budget
and were re-run in smaller pieces. No source, ADR, ticket or map file was changed.
