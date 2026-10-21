# Research: 25 — Diagnostics architecture (consolidated)

**Date**: 2026-10-14. **Status**: research input for the grilling on
[issues/25-diagnostics-architecture.md](../issues/25-diagnostics-architecture.md).
**Method**: five parallel subagent reports (codebase grounding; framework API; LSP/ecosystem survey;
performance/scope; severity/namespacing/rumdl), reviewed and integrated into this single file; the
part files have been removed. Every claim carries its source inline. Round-1 decisions (Q1–Q12)
were then put through three review passes (conflict stress-test; performance benchmarks;
CodeGraph design alternatives). Findings, including strengthened/revised versions of every
round-1 decision, are consolidated in **§10**; those part files were also removed.

**Evidence markers**: **V** = verified against crate/repo source this session; **[S]** = LSP spec
text (3.17/3.18, fetched this session — `docs/refs/lsp_spec.md` has *unmaterialized stubs* for the
diagnostics includes, so the local file cannot be cited for this topic); **[D f:l]** = local digest
(file:line); **[G]** = primary upstream (PR/issue/official docs) fetched this session; **[G·]** =
primary source named but fetched earlier; **[R f:l]** = local `research/` or `issues/` file;
**[?]** = memory only, UNVERIFIED.

---

## §0 Findings at a glance

1. **Framework**: tower-lsp-server 0.23.0 serves pull diagnostics *fully* (`textDocument/diagnostic`,
   `workspace/diagnostic`, `workspace/diagnostic/refresh` all wired) — capability advertisement,
   result-id bookkeeping, and state are ours. Types come from **`ls-types` 0.0.6**, not `lsp-types`
   0.97: import `tower_lsp_server::ls_types::…` (V).
2. **Push+pull simultaneously = duplicate diagnostics** — rumdl's own bug (#182, fixed by skipping
   push when the client can pull), ryl #323/#324, unresolved spec ambiguity (LSP #1743). The spec
   is *silent* on coexistence, so capability-gating alone is not enough: Traces must pick an
   explicit coexistence pattern (four observed; §2.2). **ty's `publish_diagnostics_if_needed` is the
   reference implementation** (G).
3. **All five provisional debounce figures reconcile into three tiers** (§3): T1 content/index
   **100ms**, T2 per-document publish **300ms**, T3 workspace **event-driven, no timer** (in the
   recommended scope option). Plus three hard rules: no `sleep` inside any handler, generation+
   version staleness checks, never republish an identical set.
4. **Scope**: full-workspace diagnostics for closed files ≈ **1.5–2s @20K files with rayon** (off-lane)
   and ~8–10MB cached — affordable as a *background warm-up served via pull*, not as a startup sweep.
   Recommended option: **push open docs + pull workspace with dirty-set/resultIds/refresh**
   (§4, options ladder B > A > C > D).
5. **rumdl non-overlap is measurable, not assumed**: `source: "rumdl"`, codes `MD\d{3}` + exactly two
   pseudo-codes (`inline-config`, `code-block-tools`), severity ∈ {1,2,3} (never Hint), no
   `tags`/`data` — all **probed live against rumdl 0.2.79** (§5). Wikilinks are never checked (clean),
   but **three real overlaps surfaced**: standard-link existence (MD057, Error), **all three of ticket
   17's footnote diagnostics (MD066)**, and frontmatter-missing-delimiter double-reporting. Also two
   corrections fall out: cross-file heading anchors are *unowned* by rumdl (contradicts `map.md:83`),
   and ticket 15's Warning for broken standard links contradicts the cede decision.
6. **Severity**: today's crate has *no severity type at all* — ticket 25 introduces it from scratch.
   Only ticket 19 proposes `Error`; ≥12 rules pile into `Information`; placement policy conflicts
   three ways (20:Q14 vs 23/31 vs 15). Every production precedent does per-rule severity remap
   **server-side** (§6).
7. **Namespace recommendation (Scheme C)**: one `source: "traces"` (always set) + hierarchical
   `<feature>.<rule>` codes (`links.unresolved`, `schema.type-mismatch`, …) + pull
   `identifier: "traces"` + per-rule `codeDescription.href` — amends 20:Q12 (`traces-schema`) and 23
   (`traces-tasks`) to the single-source form (§5.4).

---

## §1 What today's codebase offers (and lacks)

### 1.1 Existing diagnostic-shaped surfaces

- Exactly **two `miette::Diagnostic` impls** exist:
  - `QueryError` (`src/query/error.rs:66`) — `QuerySyntaxError` carries `#[source_code]` +
    `SourceSpan` (byte+len **into the query string**, `:117-137`) and chained `diagnostic_source`
    (`:67`), but **no `code()`** and no severity.
  - `CliError` (`src/cli/error.rs:258`) — supplies the only **code-namespacing precedent**
    (`traces::cli::*`, `:260-332`) and per-variant `help()`; no severity.
  - **Nothing implements `Diagnostic::severity()` anywhere** in `src/`; no severity enum exists
    (grep confirms). Ticket 25's calibration policy has no code surface yet — it is built from
    scratch.
- Rendering today: `eprintln!("{:?}", miette::Report::new(...))` (`src/main.rs:31`), no `set_hook`.
- **Note parsing is infallible** (`parse_markdown` returns `Note` unconditionally,
  `src/index/service.rs:255-269`) — malformed Markdown cannot produce a diagnostic today.

### 1.2 Span availability (three domains, one missing)

| Source | Position data today | Status |
|---|---|---|
| Query syntax | `SourceSpan` in **query-string domain** (`src/query/error.rs:137`) | usable only after the query is located inside a document (21's fenced-block plan) |
| Template errors | minijinja `error.range()` in **template-file domain** (`src/cli/error.rs:751-763`) | usable when the template itself is open; wrong file when invoked from a note |
| YAML frontmatter | `noyalib::Error::location()` → byte+line+column (**V**, `noyalib-0.0.53/src/error.rs:1063-1077`) | **currently discarded** (`src/note/metadata.rs:181-188`); recoverable with zero new parser work |
| Schema TOML files | `toml::de::Error::span() -> Option<Range<usize>>` (**V**, pinned `toml 1.1.6`) | currently unused (string-only formatting) |
| **Document domain** | **missing**: `Link` has no position field (`src/note/links.rs`), `Heading`/`ListItem` carry `SourceLine` only | depends on ticket 11 (designed, not built); `byte_to_utf16_cu` also designed-not-built |

→ Three span domains must be reconciled by the diagnostics pipeline; the document domain arrives
with ticket 11, the other three need explicit domain-tagging (query/template diagnostics point at
the *embedded* range within the open document, or at another file entirely — decide what an
out-of-document range means: `relatedInformation` vs a document diagnostic).

### 1.3 Seams

- **`RefreshReport` is counts-only** (`upserted/deleted/links_modified`, `src/index/refresh.rs:31-74`)
  — changed *path sets* exist inside `RefreshPlan`/`FileDelta` but never reach callers. Diagnostics
  invalidation needs that set: either expand `RefreshReport` or emit from within refresh.
  The single-file `RefreshPlan` constructor (13's decision) does not exist yet.
- **`InlinkMap::patch_and_diff`** (`src/index/inlinks.rs:217`) already returns exact per-edit deltas
  in one pass — the ready-made ripple signal.
- **`SchemaConstruction { warnings, failures }`** collected-then-fanned
  (`src/schema/service.rs:262-307`, today → `tracing::warn!`) is the closest aggregation precedent —
  but payloads are `Display` strings: codes/severity/spans must be added.
- **No timers exist anywhere** (one 15ms test sleep only). All debounce figures are unreconciled
  designs.
- **No LSP dependency, no `src/lsp/` code** (zero grep hits). `pub mod cli` is the only public
  module, guarded by `compile_fail` visibility probes (`src/lib.rs:63-109`); `pub mod testing` is
  frozen at 45 identifiers (`:176-180`). Packaging/visibility is ticket 34's decision.
- **Rayon** already fans out parse/refresh/store/query (`src/index/service.rs:251-257`,
  `refresh.rs:282,361`, `query/service.rs:138-153`) — diagnostics should fold into existing parallel
  stages, never add a second whole-vault pass.

### 1.4 Map-vs-code discrepancies (fix at resolution)

1. `map.md:52` cites `src/note/parser/line.rs` — **file gone**; `LineIndex` is `src/position.rs:33`,
   and `Heading` *does* carry `line` (map says only list items/tasks do).
2. `map.md:34/:75` cite `src/query/service.rs:132-139` and `QueryService::execute` — the function is
   `run_from_store` (`:129`); the linear scan is `matched_file_rows` (`:318-329`); **no
   `QueryService::execute` exists**.
3. Designed-but-absent (expected): single-file `RefreshPlan`, `WorkspaceIndex::files()`,
   `byte_to_utf16_cu`, AST spans, all debounce timers, any LSP code.

---

## §2 Push vs pull

### 2.1 Spec + framework facts

- Pull = client request `textDocument/diagnostic` (+ `workspace/diagnostic`), server cap
  `diagnosticProvider { identifier?, interFileDependencies, workspaceDiagnostics }`; incremental
  via `previousResultId`/`unchanged`; `workspace/diagnostic/refresh` = server→client re-pull request
  (**[S]**).
- **tower-lsp-server 0.23.0 wires all three methods** (V: `src/server.rs:923-931, :961`;
  `service/client.rs:370-388`; FEATURES.md 🟢×3). The default `diagnostic` handler is a
  method-not-found stub — implement it ourselves. `diagnostic_provider` capability, result-id cache,
  and unchanged-report bookkeeping are ours (V, `ls-types` `document_diagnostic.rs`).
- **Type surface** (V, `ls-types` 0.0.6): `Diagnostic { range, severity, code: NumberOrString,
  code_description, source, message, related_information, tags, data: Option<serde_json::Value>,
  … }`; `data` is spec-guaranteed to survive publish→`codeAction` (**[S]** since 3.16) — the lazy
  quick-fix channel for tickets 27/28.
- **Push path**: `Client::publish_diagnostics(uri, diags, version).await` (V) — thin pass-through on
  an `mpsc(1)`; notification order FIFO. **Pitfall**: messages are silently dropped (trace-level)
  until the initialize *response* completes (V, `client.rs:584-594`; ast-grep #2638 closed wontfix)
  → never publish from inside `initialize`; startup results go through pull or post-`initialized`.
- **Spec semantics** (**[S]**): empty array clears; each push fully replaces; `version` lets the
  client discard stale payloads; omission of `severity` ⇒ interpreted as **Error**.
- **Cancellation**: `$/cancelRequest` = drop-a-Future; at `concurrency_level(1)` the framework docs
  say cancellation is "implicitly disabled" (V, `transport.rs:78-84`) — structurally enforces ticket
  12's stale-result discard; staleness checks are ours. Inbound queue = 100 slots (debounce also
  protects it from `didChange` floods).

### 2.2 Coexistence reality — four observed patterns (pick one explicitly)

The spec is **silent** on push+pull coexistence (confirmed **[S]**; LSP #1743 never resolved). Observed:

1. **rumdl — "pull client never pushed to"**: gate on client cap; publish *empty* once to invalidate;
   `workspace/diagnostic/refresh` on index-ready; never `result_id` (**[D]** src-digest:32234-32253,
   33153-33168).
2. **rust-analyzer — "push only when pull unsupported"** + separate `identifier` for push/pull
   namespaces (cargo check stays push) (**[G·]** PRs #18404/#18709/#19266).
3. **Client-side keyed merge** — Helix per-provider, Zed per-registration-ID; Neovim instead renders
   **duplicates** (separate push/pull namespaces) (**[G]**).
4. **The ignored path**: pushing non-empty to a pull-capable client ⇒ last-writer-wins clobbering
   ("latest arrived wins", **[G·]** RA #18404).

**Reference implementation**: ty's `publish_diagnostics_if_needed` — return early when
`supports_pull_diagnostics()` (**[G]** `crates/ty_server/.../diagnostics.rs`), carve-out only where
the client lacks pull for a surface (notebooks).

### 2.3 Adoption matrix (evidence both axes)

- **Clients**: all four major clients ship pull — VS Code, Neovim ≥0.10 (#24128), Zed (#19230, pull
  **default** when advertised; `lsp_pull_diagnostics_debounce_ms`), Helix (#11315, debounced,
  cancelable) (**[G]**). Zed #13102: advertising pull without serving it made servers go **silent** —
  never advertise what isn't implemented.
- **Servers**: server-side pull remains a minority — rust-analyzer (hybrid), ty, oxc, rumdl, MS
  markdown-ls serve it; gopls keeps it **off by default "until the performance … is comparable to
  push"** (**[G·]**); clangd none; tsserver uses its own staged `geterr`; **Marksman, Markdown Oxide,
  zk, yaml-ls are push-only** (**[D/G]**). Pull is the emerging *client* default, not the
  markdown-server default.
- Watch: VS Code push/pull land in **separate collections** → visible doubling; Neovim #29927/#37936.

### 2.4 Working conclusion for the grilling

Advertise `diagnosticProvider`; if the client pulls ⇒ **never push non-empty** (rumdl/ty pattern);
keep push for open-doc serving only against pull-incapable clients (or skip push entirely if 29
finds every primary target pulls — Zed/Neovim/Helix/VS Code all do). Set `identifier: "traces"`,
`interFileDependencies: true` (wikilinks cross files), `workspaceDiagnostics` only when workspace
pull is actually served. Send `workspace/diagnostic/refresh` unconditionally and swallow the error
(rumdl lesson: the `refreshSupport` capability arrives under a key some type layers don't read —
gating on it would silence every real client, **[D]** 33155-33168).

---

## §3 Debounce & recompute triggers — the three-tier model

### 3.1 The tiers (reconciles `map.md:32`)

| Tier | Window | Gates | Reconciles |
|---|---|---|---|
| **T1 — content/index** | **100ms** | buffer re-parse → structural diff → single-file `RefreshPlan` integration | 13's 50–100ms, 33's 100ms; **14's 150ms drops** |
| **T2 — per-document diagnostics publish** | **300ms** | diagnostics compute+publish for the edited doc (off-lane) | 14/20/21/23's 300ms; Markdown Oxide's 300ms |
| **T3 — workspace scope** | **event-driven, no settle-timer** | dependent-set recompute after T1's diff; pull served from cache; `refresh` after invalidation bursts | gopls' ~1s idle delay only applies to the rejected idle-sweep option |

### 3.2 Trigger inventory

| Trigger | Action |
|---|---|
| `didChange` | T1 overlay re-parse + structural diff (inline: 0.1–0.3ms typical, 18.6ms worst); if changed → dependents enqueued (coalesced); T2 publish self after 300ms |
| `didSave` | integrate into refresh typestate (14) → same as above |
| `didChangeWatchedFiles` | single-file `RefreshPlan` → structural diff → class B/E propagation |
| `didOpen` | parse now; publish after T2 |
| `didClose` | clear published set; pull continues to serve |
| config/schema change | generation bump → class C affected set |
| day rollover | timer — **blocked on 18** (23 routed re-evaluation to 25) |
| client pull | on demand from resultId cache |

**Diagnostics-lifetime note (amended 2026-10-21 by fileclass research):** definition-side (schema-file) diagnostics plausibly want a persisted findings log — fileclass's sessionly, fingerprinted once-per-problem pattern — versus this document's ephemeral per-document publish model: two tiers, two lifetimes. Flagged for the implementation spec; no decision change ([tool-fileclass](tool-fileclass.md)).

### 3.3 Invariants (transferable lessons — no corpus number is authoritative)

Observed keystroke-debounce spread: **50ms (clangd) / 200ms (Marksman, yaml-ls) / 300ms (Markdown
Oxide) / 1000ms (zk)**, plus quiescence (rust-analyzer) and staged-on-demand (tsserver). The
structure that recurs everywhere:

1. **Debounce only the expensive validation path**; parse eagerly/deduct cheaply.
2. **Generation counter + document version**: bump on edit; after the timer, publish only if still
   current (Markdown Oxide's `diag_gen: AtomicU64`, **[D]** src-digest:1694-1701); drop stale results.
3. **Never republish an identical set** (rust-analyzer equality early-cutoff, gopls `publishedHash`,
   clangd no-op suppression, Marksman diff-publish) (**[G·]**).
4. **Burst-drain**: one index update naming N dependents drains into one lint pass per file
   (rumdl, **[D]** 33110-33127).

### 3.4 The 14-vs-12/33 tension (must be locked)

14 puts `tokio::time::sleep` **inside the handler flow**; 12 runs `concurrency_level(1)` (one
handler polled at a time, awaits hold the lane); 33 budgets interactive <100ms. An in-lane 150–300ms
debounce sleep stalls any concurrently-arriving gotodef/completion for up to 300ms.

**Rules for the decision**: (1) no `sleep` in any handler — debounce = spawn + generation counter;
(2) publish through 22's **β background publisher** (its `Compute/Clear/Opened` FIFO, coalesce-by-URI,
publish-gate — generalize 22's template-only scope to all diagnostics); (3) never `rayon::join`
build-scale work inside a handler — dispatch off-lane, deliver via β or a pre-populated cache the
pull handler reads in µs.

---

## §4 Scope: workspace-wide vs open-document-only

### 4.1 Cost model (measured inputs)

| Fact | Number | Source |
|---|---|---|
| per-file diagnostic budget | <20ms p95 / <10ms | `issues/33:54` |
| actual per-file cost (re-parse + rules) | ~0.1–0.5ms typical; 18.6ms worst-known long line | `research/33:194`, `issues/23:55` |
| **full-workspace compute for closed files @20K** (needs disk re-read — text not in redb, LRU holds only ~256 ASTs) | 10–15s single-thread → **1.5–2s on 8 threads rayon** | §4 cost derivation |
| warm diagnostics cache @20K (40K diags × ~200B) | **~8–10MB RAM** + 0.8MB resultIds + 10–14MB JSON if pushed whole | engineered estimate |
| `[tasks]` config change @20K | 466ms rebuild (known) | `issues/23:57` |
| cold-start budget | <2s total — a startup sweep eats it | `issues/33` |

Nobody in the corpus does blocking full-workspace diagnostics at cold startup — workspace
completeness is always **index-backed, incremental, refresh-driven** (rumdl's index-ready ⇒
`workspace/diagnostic/refresh` is the cleanest template, **[D]** 70777).

### 4.2 Invalidation classes (six)

| # | Class | Affected set | Mechanism |
|---|---|---|---|
| A | self edit | the file | 14 overlay (authoritative) |
| B | structural export change (headings/links/tags) | referrers of X | `InlinkMap[X]` ∪ **missing raw-target referrers** (below) |
| C | schema/config change | notes of that fileClass (vault-wide for `[tasks]`) | `PATHS_BY_FILE_CLASS` O(1) (`src/index/tables.rs:48-52`); `[tasks]` = full rebuild |
| D | day rollover | daily notes + referrers | none yet — blocked on 18 |
| E | external FS create/delete/rename | deleted file's referrers | single-file `RefreshPlan` + `InlinkMap`; rumdl `handle_file_removed` precedent |
| F | query/template dependency | files containing them | **no occurrence index** (bodies unpersisted) → dirty the open set + lazy pull |

**Two missing edges**:
1. **Raw-target → referrers**: `InlinkMap`/LINKS hold *resolved* edges only, so the
   broken→fixed transition (file created into place) has no edge today. `Note.outlinks` *does*
   persist raw targets → build a `target → referrers` multimap (analog of rumdl's `get_dependents`,
   **[D]** 31110-31133).
2. **File → queries/templates**: not derivable without text; open-set dirtying + lazy pull is the
   pragmatic answer (free under pull).

**Structural gate**: prose-only edits must not fan out — compare persisted
`(headings, outlinks, tags)` before/after (rumdl's `extracted_data_differs` gate, **[D]** 31091-31123,
tests 31969-32003). All three inputs are persisted — no re-parse needed to *decide* whether to
recompute.

### 4.3 Options ladder (capability-per-cost, ranked)

- **B (recommended): push open docs + pull workspace.** Background off-lane warm-up (chunked rayon,
  ~1.5–2s @20K) pre-computes closed-file results into a bounded ~10MB cache; pull handler = cache
  lookup (µs, safe in-lane); classes B/C/E mark dirty URIs (generation bump → `unchanged` next pull)
  + issue `workspace/diagnostic/refresh`. Full Problems-panel completeness, protocol-native
  staleness, zero push flood, works on every 3.17+ (2022+) client. First pull after restart is the
  expensive one — spec partial-result streaming covers the warm-up gap.
- **A (cheapest MVP)**: push open-docs only; reuses 14/22; **closed broken links stay silent** —
  exactly this ticket's own gap. B is a strict superset; A's paths are B's phase 1.
- **C (A + idle sweep)**: only beats B for pre-3.17 clients — recurring CPU + 10–14MB push JSON;
  **dominated** by B.
- **D (eager startup sweep fused into cold build): rejected** — results unstorable (13: no new redb
  tables) and stale after the first edit.

**Storage**: diagnostics never go in redb (13); warm cache bounded in-session; resultIds are the
only durable-in-session state. Note 33 puts "diagnostic results" in the *transient* tier — keeping
a bounded ~10MB workspace cache **amends that line** (state it explicitly in the resolution).

### 4.4 Cross-ticket inputs & measurement debts

- **41 (span persistence)**: even 41-B doesn't give closed-file *link* ranges — `Link` has no
  position field at all. Byte-precision closed-file diagnostics ⇒ re-read text (costed above) or a
  model extension outside 25's scope — feed 41's grilling either way.
- **29**: capability advertisement inputs (`text_document.diagnostic`,
  `workspace.diagnostics.refresh_support`, `data_support`).
- **18**: class D day-rollover timer.
- **Owed to 33**: a diagnostics rule-eval bench (23's "≪1ms" is a label, no bench exists); full
  redb scan+decode wall time; warm-up wall time @1K/5K/20K in `lsp_latency.rs`.

---

## §5 rumdl non-overlap (measured) + namespacing

### 5.1 rumdl's convention, probed live (0.2.79, 2026-10-14)

| Field | Observed |
|---|---|
| `source` | `"rumdl"` on **every** diagnostic |
| `code` | string `MD013`-style rule number (**even when configured by alias**); pseudo-codes `inline-config`, `code-block-tools` |
| `codeDescription.href` | `https://rumdl.dev/<rule>/` (absent on pseudo-codes) |
| `severity` | 3-level scale `error/warning/info` → LSP 1/2/3; **never Hint(4)** |
| `tags`, `data` | never present |
| `diagnosticProvider` | `{identifier: "rumdl", interFileDependencies: true, workspaceDiagnostics: false}` |
| push/pull | pull-capable client ⇒ publish **empty** + skip push; `workspace/diagnostic/refresh` on index-ready; **no `result_id`** |

Code space to stay disjoint from: **83 × `MD\d{3}`** + the two pseudo-codes. Rumdl link *diagnostics*
(MD057/MD051) are gated by lint-rule config, **not** by its `enable*` LSP settings — ceding them
requires `disableRules`/`.rumdl.toml`, not capability toggles.

### 5.2 Category split — probe results vs Traces tickets

Clean (no overlap): wikilinks & wiki-embeds (never checked, by design), wikilink heading refs,
malformed YAML frontmatter, schema/field validation, `traces-query` fences, daily-note date logic,
callouts/tags (no diagnostics planned), template `{{ }}` (**by catalogue, unverified by probe** —
low risk).

**Conflicts requiring rulings in this ticket**:

| Traces plan | rumdl measured | Verdict |
|---|---|---|
| 15: standard link → missing file = **Warning** | **MD057 fires at Error** (fragment stripped) | **T6**: cede to MD057 (amend 15) *or* keep + document "disable MD057" — both servers otherwise red-line the same span |
| 15: image target missing | **MD057 fires on `![](missing.png)`** | same shape — define 15's "embed" as `![[..]]` only, or cede |
| 17: footnote undefined/unused/duplicate | **MD066 fires on all three** (duplicate check is docs-silent, probe-only finding) | **full overlap** — cede (drop three of 17's diagnostics) or require `disable MD066`; `tags: [Unnecessary]` is the idiomatic unused-def rendering if kept |
| 19: missing `---` delimiters = **Error** | MD041 + MD065 fire on same file | double-reporting, unsuppressible without disabling rumdl structural rules → decide 19's Error earns its noise (T7) |
| map.md:83: "rumdl owns standard-link anchors" | rumdl owns **same-document `#frag` only (MD051)**; `file.md#heading` → **no diagnostic** | **map correction**: cross-file heading anchors are **unowned** — claim for Traces or leave silent? |

Also: style rules (MD004/030/032/013…) may fire on the same *lines* as task diagnostics — that is
different-question noise, not category overlap; 23 already bans style diagnostics.

**Ownership summary (feeds 32's contract)**: rumdl owns structure/style rules, standard-link file
existence incl. images (MD057), same-document fragments (MD051), footnote well-formedness
(MD066/67/68), formatting. Traces owns wikilinks, YAML frontmatter well-formedness + schema,
query/template/daily/task/tag semantics, and — pending rulings — cross-file anchors and possibly
nothing of standard links or footnotes.

### 5.3 Transport-level non-overlap

Independent of *content* split: capability-gate push off when the client pulls (§2.2), and never let
Traces' pull and push collide with rumdl's either — distinct `identifier` (`"traces"` vs `"rumdl"`)
plus the always-set distinct `source` is what keeps a two-server buffer attributable. Clients won't
do it for us: VS Code renders no cross-server attribution unless `source` is set (a decade of
confusion: #141987/#98633/#12675), and dedups across owners on source+code+range (PR #295774) —
identical `source`+`code` from two servers would *collapse*. Neovim shows sources only under
`source = "if_many"` — i.e. exactly the Traces+rumdl case.

### 5.4 Namespace scheme candidates → recommendation

- **A — per-feature sources** (`traces-schema`, `traces-tasks`, …): matches 20:Q12/23's proposals,
  but 7–9 rows in every attribution/filter UI, contradicts the measured single-source pattern
  (RA/eslint/yaml-ls all one constant), and still needs disjoint codes.
- **B — one source + flat prefixed codes**: trivial but scales badly past ~40 codes, arbitrary
  vocabulary.
- **C (recommended) — one `source: "traces"` + hierarchical `<feature>.<rule>` codes + pull
  `identifier: "traces"` + per-rule `codeDescription.href`**: `links.unresolved`,
  `frontmatter.yaml-syntax`, `schema.type-mismatch`, `query.unknown-class`,
  `template.undefined-variable`, `daily.past-date-unresolved`, `tasks.overdue`, … Disjoint from
  `MD\d{3}` *and* from rumdl's word-shaped pseudo-codes; one source; feature grouping preserved for
  docs/tests/filters; per-rule overrides key off one stable string. **Amends 20:Q12 and 23's source
  strings** (their per-rule codes just gain the feature prefix). Cost: a central code registry (one
  table) so tickets can't mint colliding strings.

**Invariants under any scheme**: never `MD…` or bare `inline-config`/`code-block-tools`-style words;
always set `source` *and* `severity`; set `codeDescription.href` per rule (docs-URL pattern open —
Q8); pull `identifier = "traces"`; prefer `tags: [Unnecessary]` over severity bumps for dead/unused.

---

## §6 Severity taxonomy (cross-ticket inventory + tensions)

### 6.1 What tickets proposed

| Ticket | Severities | Notes |
|---|---|---|
| 15 links | ambiguity = no diagnostic; wikilink zero-match = Information; **standard link missing = Warning**; heading ref missing = Information; embed missing = Warning; `UnresolvedLinkPolicy {Silent, Information, Warning, Error}` per link type | decided, but T6 below |
| 17 footnotes | **none declared** | gap → 25; and §5.2 says rumdl MD066 covers all three |
| 18 daily notes | **none declared** | gap → 25 |
| 19 frontmatter | malformed YAML = Warning; **root-level = Error** | only ticket proposing Error |
| 20 schema | Information (unknown, no match) / Warning (typo match, invalid select, List-on-non-multi); `source: "traces-schema"`; `[schemas.diagnostics]` | source amends to Scheme C |
| 21 query | Warning (unknown class + close match) / Information (no match, unknown field) | "25 owns severity calibration" |
| 22 template | four phase-1 classes **proposed Information → 25** | |
| 23 tasks | Hint = vocabulary (A1); Warning = behavior mismatch (B1/B2/B3/B5/D1/D2, C1 overdue); Information = likely-mistake (C2/C4/C5/C6); **Error = none**; B4 dropped | "23 proposes, 25 decides"; `[tasks.diagnostics]` → 31 |
| 16 tags | no diagnostics | n/a |

### 6.2 Tensions to rule

- **T1 — Error budget.** Only 19 proposes Error; rumdl already emits Error (MD057/051/066) on the
  same files — two servers' red squiggles competing. Candidate rule: **Error = "Traces cannot
  parse/analyze the document at all"** (19's root-level YAML, internal parse failure); everything
  semantic below it.
- **T2 — Information dumping ground.** ≥12 rules across five tickets land in Information. Escape
  hatches from precedent: escalate to Warning only for *behavior mismatch* (23's framework), and use
  `tags: [Unnecessary]` for dead/unused instead of a bump (renders faded in all four editors without
  a Problems row).
- **T3 — Hint is real but quiet.** Spec-legal; Neovim renders it; Zed accepts `"hint"`; **Helix's
  statusline counts only warning/error by default**. UNVERIFIED: whether VS Code's Problems panel
  lists Hint. 23's A1-as-Hint must tolerate being invisible in some aggregate views.
- **T4 — where overrides live (3-way conflict).** 20:Q14 says editor-settings-not-project-config;
  23 wants `[tasks.diagnostics]` but defers to 31 (`deny_unknown_fields`); 15 bakes a project enum
  into policy. Precedents on both sides (zk/hadolint/markdownlint/rumdl = project file;
  vscode-eslint/RA = editor settings sourced server-side). Resolution candidate in §6.3.
- **T5 — scale asymmetry.** rumdl never uses Hint; Traces will. Not a collision — one line in 32's doc.
- **T6 — 15 vs non-overlap policy** (internal contradiction): 15's Warning for broken standard links
  vs `tool-rumdl-boundary.md:64` cede vs measured MD057 Error. Resolve here or amend 15; same shape
  for image targets and footnotes (rows 5/7-9).
- **T7 — frontmatter missing-delimiter double-report** (19's Error + MD041/MD065).

### 6.3 Configuration surface

**Load-bearing fact**: per-rule severity remapping is a **server-side job in every precedent** —
vscode-eslint (`RuleSeverities` overrides from client settings), rust-analyzer
(`DiagnosticsMapConfig` from client settings or `rust-analyzer.toml`), MS markdown-ls
(`DiagnosticLevel = ignore|hint|warning|error` per code), rumdl (`.rumdl.toml` *and* LSP settings,
file wins), zk (`[lsp.diagnostics]` project config), markdownlint/hadolint (project file). Client-side
filters (Zed `diagnostics_max_severity`, Neovim, Helix) can only filter, **never remap one rule** —
including in VS Code, where users have no native per-code lever at all. So 20:Q14's "per-source
severity via editor LSP settings" works only *as a filter* in Neovim/Zed/Helix; in VS Code the server
must still read `workspace/configuration`/`initializationOptions` and map (a refinement of Q14, not
a refutation).

**Candidates**:
- **O1** editor-sourced only (workspace configuration + `initializationOptions`, consumed
  server-side); **O2** project `.traces/config.toml` (`[lsp.diagnostics]`, zk shape) — **cannot land
  before 31**; **O3** no user severity control in v1 — fails 15's promised configurable escalation.
- **R-A (recommended)**: hard-coded per-rule defaults in one registry (hadolint style — severity lives
  with the rule definition); overrides via **O1 only** in v1; project-level `[lsp.diagnostics]`
  **deferred to 31** with zk's shape recorded as target; 15's per-link-type escalation lands as O1
  keys (`links.wikilink.unresolved: info|warn|error|off`) unless 31 lands first.
- R-B: O2 immediately (requires 31 to sequence it first). R-C: split enable/debounce project-side,
  severity editor-side — weakest (one concern, two files).

---

## §7 Corrections & contradictions discovered (for map/ticket fixes at resolution)

1. **`map.md:52`**: `src/note/parser/line.rs` no longer exists (`LineIndex` = `src/position.rs:33`);
   `Heading` does carry a line — the "only list items/tasks" claim is wrong.
2. **`map.md:34/:75`**: `QueryService::execute` doesn't exist (`run`/`run_from_store`); linear scan
   is `matched_file_rows` at `:318-329`, not `:132-139`.
3. **`map.md:83`**: "rumdl owns standard-link anchors" — rumdl owns *same-document* fragments only;
   cross-file `file.md#heading` is unowned (probed).
4. **`research/20`**: yaml-ls debounce is **200ms**, not 300ms; its "industry standard 300ms"
   overstates (spread 50–1000ms + quiescence + staged); rust-analyzer is **hybrid push+pull**, not
   "pull (revision-based)".
5. **Ticket 15**: Warning row for broken standard links contradicts the cede decision and measured
   MD057-Error (T6).
6. **Ticket 17**: all three planned footnote diagnostics collide with MD066 — plan needs a ruling.
7. **Ticket 20/23**: `source: "traces-schema"` / `traces-tasks` amend to Scheme C if confirmed.
8. **Ticket 22/14**: 14's in-handler `sleep` must not survive 25 (β publisher generalization) —
   22's β scope was template-only; 25 generalizes it.
9. **Ticket 33**: "diagnostic results = transient tier" amends to a bounded ~10MB workspace cache
   if option B lands.
10. **Framework**: types are `ls-types` 0.0.6 — any prior reference to `lsp-types` 0.97 for
    tower-lsp-server 0.23.0 is wrong.
11. **Corpus gap**: `docs/refs/lsp_spec.md` has unmaterialized stubs for the diagnostics includes —
    future tickets needing spec text must fetch primary spec, not the local file.

---

## §8 Open questions for the grilling (merged)

1. **Push/pull pattern** (§2.2): adopt rumdl's "pull client never pushed to" + ty's gate? Does
   push survive at all, given all four primary target clients pull? (Feeds 29.)
2. **Scope option** (§4.3): confirm B (push-open + pull-workspace + warm-up) vs A-then-B phasing?
3. **Three-tier debounce** (§3.1): confirm T1 100 / T2 300 / T3 event-driven, and generalize 22's β
   publisher to all diagnostics (amends 12 §1 / 14 §2 scope)?
4. **Error budget** (T1): "cannot analyze" only, or semantic Errors allowed?
5. **Information density** (T2/T3): cap Information behind "likely-mistake"; push FYI-class to
   Hint/tags?
6. **T6 ruling**: cede standard-link existence (+ image targets) to MD057, amending 15's Warning row?
7. **Footnotes**: cede undefined/unused/duplicate to MD066 (drop three of 17's diagnostics) or keep +
   `disable MD066`?
8. **Cross-file heading anchors**: unowned today — claim for Traces or leave silent (map correction)?
9. **Namespace**: confirm Scheme C; amend 20:Q12 + 23 sources; adopt the registry invariant?
10. **Config placement**: R-A (editor-sourced v1, project `[lsp.diagnostics]` deferred to 31)?
11. **`codeDescription.href`**: docs-URL pattern per rule (rumdl's `https://rumdl.dev/<rule>/` shape)?
12. **Tags**: adopt `Unnecessary` for dead refs — and where does 15's resolved-but-dead case land?
13. **19's frontmatter-missing-delimiter Error** (T7): keep (accept MD041/MD065 double-report) or
    downgrade?
14. **`RefreshReport` seam**: expand it with changed-path sets, or emit diagnostics from inside
    refresh? Plus the raw-target→referrers reverse index (build from `Note.outlinks`) — in scope here
    or filed as a follow-on?
15. **Day-rollover (class D)**: defer to 18, or does 25 own the timer decision now (23 routed it
    here)?

---

## §9 Source index

- **Framework/crate (V)**: tower-lsp-server 0.23.0 + ls-types 0.0.6 cached under `.rust-docs/cache/`;
  FEATURES.md @ v0.23.0; tower-lsp 0.20.0 PR #396 (pull shipped); CHANGELOG (ls-types switch).
- **Spec [S]**: microsoft/language-server-protocol @ 3.17 `language/pullDiagnostics.md`,
  `language/publishDiagnostics.md`, `types/diagnostic.md`; LSP 3.18 `#diagnostic`; LSP #1743, #519.
- **rumdl (D + live probe)**: `docs/digests/lsp_rvben-rumdl-{digest,docs-digest,src-digest}.txt`
  (key lines: push/pull branch 32234-32253, 33153-33168; structural gate 31091-31123, tests
  31969-32003; dependents 31110-31133; deletion 31181-31198; burst drain 33110-33127; settings
  precedence 30219-30320; severity docs 17486-17541; MD057 :9968, MD051 :9069, MD066 :12177-12259);
  live probes of rumdl 0.2.79 (transcripts in the removed part-5 appendix — recreate via
  `rumdl server` stdio probe if needed); rumdl #182.
- **Other servers/clients [G/G·]**: rust-analyzer (PRs #18404, #18709, #19266, #6466, #5481;
  `diagnostics.rs`, `handlers/request.rs`, main_loop); ty/ruff (commit ef81835, PRs #18939, #18309);
  oxc #11447; gopls diagnostics docs + go#60122; clangd #2108/#2484, ClangdServer.h; tsserver geterr;
  Marksman digest 11183-11290; Markdown Oxide src-digest 1660-1712, 2197; zk digest/docs (4700-4810,
  724-790, 10300); yaml-language-server `yamlValidation.ts`/`server.ts`, issue #1091; hadolint README;
  djlint rules docs; Dataview digest (render-time query errors); VS Code #141987/#98633/#12675,
  PR #295774; Neovim #24128/#29927/#37936 + `vim.diagnostic` docs; Zed #19230/#13102/#43703/#23639;
  Helix #11315/#7900 + `helix-view/src/handlers/lsp.rs`.
- **Local**: `issues/12,13,14,15,17,18,19,20,21,22,23,33` (severity inventory, debounce figures,
  budgets); `research/{tool-rumdl-boundary,tool-marksman,tool-zk,tool-markdown-oxide,tool-ms-markdown-language-service,tool-rust-analyzer-precedent,33-lsp-performance-targets,x-lsp-performance-best-practices,21-query-language-intelligence,23-task-pkm-semantics}.md`;
  code citations as marked in §1.

---

## §10 Round-1 review consolidation (three review passes)

**Date**: 2026-10-15. Three parallel review subagents examined the round-1 decision set
(Q1–Q12): (1) conflict stress-test against locked tickets and spec/client reality;
(2) performance benchmarks against ecosystem leaders; (3) CodeGraph-grounded design
alternatives and blast radius. Their part files are deleted; everything load-bearing is
below. Each entry states the **revised** decision the grilling should confirm.

### 10.1 Revised decision deltas

**Q1 scope (was: B, push open docs + pull workspace).**
Phase A as sketched ("push open docs first") is a **no-op on every pull-capable client**:
once `diagnosticProvider` is advertised, pushing non-empty clobbers pull results
(§2.2 pattern 4). Revised: **phase A = document-level pull serving** (per-file
`textDocument/diagnostic` with resultIds; push retained only as fallback for
pull-incapable clients), phase B adds workspace pull + warm-up cache. Same ladder,
corrected mechanism.

**Q2 rumdl/ty push gate.** Unchanged: gate push off when the client pulls; empty-publish
once to invalidate; `workspace/diagnostic/refresh` unconditional with swallowed error.

**Q3 debounce tiers (was: T1 100ms / T2 300ms / T3 event-driven + β generalization).**
Performance review: **T2 300ms lags every leader** (ts-ls/clangd 50ms, ty 4–58ms, ruff
onType). Revised: **T2 → ~150ms** with an adaptive clamp (floor 100ms; widen under load,
clangd D73949 pattern); T1 100ms and T3 event-driven stand. Also: adopt 22's β publisher
**as-is for MVP** (generation counters arrive with scope B, inside the diagnostics module);
separate lane and facade DirtySet both rejected (§10.2 D4/D6). T2-under-pull noted: for
pull clients the timer only bounds cache freshness, not UI latency.

**Q4 yaml-ls aggregation pipeline (was: aggregation pipeline).**
**Drop the "Pipeline" abstraction**: plain per-document fold functions over a
protocol-free diagnostic type, following `SchemaConstruction`'s collect-then-emit shape
(`src/schema/service.rs:262-307`); conversion to `ls-types::Diagnostic` happens only at
the LSP boundary. Placement of the fold home is **pending ticket 34** (record on 34's
inputs line, per 22's Q8 precedent).

**Q5 severity policy.** As sketched it **conflicts with locked tickets**: it would
re-decide 22's Error for import/from (locked) and invert 23's C-rules = Information.
Revised: 25's calibration policy **applies only to undeclared severities** (17, 18, 22's
four phase-1 classes, plus the §6.1 gaps); already-locked severities are carved out.
Error budget rule ("cannot parse/analyze at all") is retained for *new* decisions.

**Q6 Scheme C namespace.** Unchanged, with design-review strengthening: the registry is
the **single static table** with a `resolve(code, &OverrideSet)` lookup so 31 slots in
without touching features; 15's `UnresolvedLinkPolicy` is demoted to **override-key
sugar** (emits `links.*` keys into the generic override map, never resolves severity
directly); per-ticket severity config types are rejected as parallel models.

**Q7 config R-A.** Unchanged (editor-sourced O1 in v1; project `[lsp.diagnostics]`
deferred to 31), with the registry lookup seam above as its implementation shape.

**Q8 cede MD057 (was: cede standard-link existence to rumdl).**
Ceding **strands three 15 artifacts**: the code action, the `15:198` flag, and the
`15:209-213` `.rumdl.toml` disable guidance (which inverts the cede). Revised: cede
**plus a full 15 amendment** covering all three: code action retargets to nothing (drop
or repoint to rumdl docs), the flag flips, the `.rumdl.toml` guidance becomes the
*primary* configuration path rather than a fallback. Image targets (`![](x.png)`): cede
the same way; 15's "embed" narrows to `![[..]]` wikilink embeds only.

**Q9 cede MD066 (was: cede three footnote diagnostics).**
Covers only **3 of 17's 5**: footnote *self-reference* and *blockquoted-definition* are
**unruled by rumdl** (probe), so they stay Traces-owned. Revised: cede undefined/unused/
duplicate to MD066 (drop those three from 17, keep `tags: [Unnecessary]` guidance),
**retain** the two unruled footnote diagnostics, and drop the footnote hover idea's
dependency on the ceded diagnostics (hover stays only where Traces still owns the rule).

**Q10 claim cross-file anchors.** Stands, with prereqs made explicit: needs ticket 11's
`Link` spans, slug machinery, and a **suppress-when-target-missing** rule (a missing
target file already reports as `links.unresolved`; don't double-report the anchor).

**Q11 RefreshReport seam (rewritten by design review).**
- **(a) Reject expanding `RefreshReport`**: it is `Copy` and baseline-pinned
  (`tests/baselines/public-api-test-utils.txt:1443`); `IndexUpdate::report()` collapses
  real deltas to counts (`src/index/refresh.rs:424`). Instead **derive dirty/referrer
  sets in the LSP facade** from the old/new `Arc<WorkspaceIndex>` at the 10 swap instant
  plus the trigger's own change list (didSave URI / watched-files list).
- **(b) Reject the raw-target reverse map**: it would re-implement `LinkResolver`
  semantics (a parallel resolution model). Referrers derive from `InlinkDelta` edge
  pairs + snapshot `inlinks()`. Fallback if whole-tree refresh surprises matter: widen
  `FileDelta`/`InlinkDelta` from `pub(super)` to `pub(crate)` and thread them out.
- **Interlock (must be decided consciously)**: trigger-derived sets alone **miss
  referrer retargets** (a referrer whose file did not change but whose edge retargeted).
  Revised default: **combine trigger list + edge deltas** (take the `pub(crate)` widening
  as the primary, not the fallback) so retargets are caught; the pure-facade variant is
  accepted only if we consciously accept rare staleness until the next structural event.

**Q12 day-rollover.** 18 is **already resolved**, so "defer to 18" is wrong: 25 must
**amend 18** (or re-open it) to carry the timer/invalidation decision, not defer to a
closed ticket.

### 10.2 Design-review verdicts (least-change-first, CodeGraph-grounded)

- **D1 aggregation**: adopt plain functions + protocol-free type; no Pipeline struct
  (owns no invariant); miette only at its existing seam (`QueryError`).
- **D2 change summary**: facade derivation primary; `pub(crate)` delta widening fallback;
  expanding `RefreshReport` rejected (churn: Copy loss, `new()` arity, ~40 baseline
  lines, 8 callers).
- **D3 reverse map**: rejected outright; `InlinkMap`/`InlinkDelta` already encode the
  truth; LINKS ≡ InlinkMap. Re-check if 41 persists occurrences (would subsume).
- **D4 β generalization**: adopt 22's β as designed; template `&mut` env is a
  compute-body concern, not a queue concern. State the split: β covers the per-URI
  push/open-doc lane only; **workspace warm-up bypasses the FIFO** (chunked background
  compute → cache → `workspace/diagnostic/refresh`). Initial Compute runs post-`initialized`
  (startup drop pitfall, §2.1).
- **D5 registry**: static table + OverrideSet param; R-A placement; reject per-ticket
  severity types (3–4× re-implementation, recreates T4).
- **D6 dirty state**: no new struct in MVP (the β FIFO coalesce *is* the open-doc dirty
  set); scope B adds generation counters **inside the diagnostics module** (shared with β
  and the pull cache); facade `DirtySet` rejected (10's thin-facade constraint; third
  representation of dirtiness).

### 10.3 Performance verdicts (vs ecosystem leaders)

| Metric | Round-1 | Verdict | Revised |
|---|---|---|---|
| T2 publish latency | 300ms | **lagging** (leaders 50ms/4–58ms/onType) | **~150ms**, floor 100ms, adaptive clamp |
| warm-up @20K | 1.5–2s | leading, **pending measurement** | budget it; bench owed to 33 |
| per-file compute | <20ms | leading (4× headroom) | keep |
| memory (~10MB cache) | ~10MB | leading (3–40×) | keep, bounded |
| startup (warm) | <2s | leading, ambitious | keep |
| **cold first pull** | **unbudgeted** | gap | add budgets: document pull <300ms; workspace pull served as **streamed partial results** during warm-up |

Optional follow-on flagged (not decided): revisit 13's "no new redb tables" via a
blake3 sidecar cache + persisted spans (41) to eliminate disk re-read on cold compute.
Feed as an input to 13/41, do not decide here.

### 10.4 Additional conflict flags (part 1)

1. **Single-root assumption**: the whole design assumes one vault root; **ticket 30
   (multi-root?) is open**. Record every workspace-scoped decision as conditional on 30.
2. **A4**: 22's amendment to 14 §2 must be worded as **superseding** 14 §2 (in-handler
   sleep removal), not merely amending.
3. **Registry vs 20's `[schemas.diagnostics]`**: seam must state that the feature toggle
   gates *emission* while the registry resolves *severity* (orthogonal, no overlap).
4. **Warm-up lifecycle**: chunked, cancellable-on-shutdown, generation-checked; never a
   single blocking startup job.
5. **Q10/Q11b overlay patching**: dirty derivation must account for unsaved overlay text
   (buffer authoritative, map.md:21), not just index state.

### 10.5 Corrections to carry (added by reviews)

- `map.md:32` debounce figure → revised tiers (T2 150ms).
- `LineIndex` lives at `src/position.rs:33` (not `src/note/parser/line.rs`);
  `QueryService::execute` doesn't exist (`run`/`run_from_store`).
- Round-1 text assuming "single-file RefreshPlan reports its change set" is **wrong**:
  `RefreshPlan::collect` is whole-tree only (`src/index/refresh.rs:281`).
- Round-1 text assuming `WorkspaceIndex::files()` is wrong: only `entries()`
  (`src/index/entry.rs:108`).

### 10.6 pulldown-cmark link structures: effect on Q15 (checked 2026-10-15)

Question: can pulldown-cmark's link errors/structs simplify Q15 (15's escalation override keys)?

- **No.** pulldown-cmark has **no link error types at all**: parsing is infallible
  (the only error struct is `StringTooLongError`, unrelated); a malformed/unresolved
  link is silently rendered as text. There is no severity, no diagnostic, nothing to
  map override keys onto. Severity mapping stays Traces-owned (the registry, Q6-R).
- **Its `LinkType` vocabulary is wrong for config keys**: 10 syntax-shape variants
  (`Inline/Reference/Collapsed/Shortcut/*Unknown/Autolink/Email/WikiLink{has_pothole}`)
  vs Traces' semantic 2-variant `LinkType { Wikilink, Markdown }` + `embedded` flag,
  which the parser already derives by collapsing at `src/note/parser.rs:417-421`.
  Keying overrides on pulldown's enum would leak parser internals into user config and
  explode the key space. **Q15 stands as decided: override key ≡ registry code,
  vocabulary from `Link { kind, embedded }`.**
- **Two side findings worth more than the question**:
  1. **Coverage gap for 15**: unresolved *reference-style* links (`[text][undef]`)
     emit **no event at all** (parser uses `Parser::new_ext`, no callback,
     `src/note/parser.rs:109`) → no outlink is recorded → today undiagnosable.
     `Parser::new_with_broken_link_callback` yields `BrokenLink { span, link_type,
     reference }` (`parse.rs:180`) with an exact byte range for free. Whether Traces
     should rule on these at all (CommonMark renders them as literal text) is
     **ticket 15's scope question**, flagged there at resolution.
     **Scope of BrokenLink (do not over-read it)**: it is NOT the source of
     `links.unresolved`. Wikilinks/inline links are syntactically valid: pulldown
     emits `Tag::Link` with the raw target as `dest_url` no matter whether the file
     exists (no vault knowledge), so `links.unresolved` stays derived from
     `Note.outlinks` × `LinkResolver` × file index. BrokenLink covers only undefined
     reference-style labels; at most it becomes one extra narrow source diagnostic
     (e.g. `links.reference-undefined`) feeding the same fold, as a
     **diagnostics side-channel only**. Resolving its results into real outlinks
     would change index/resolver semantics (15's territory).
  2. **Markdown images strengthen Q8-R**: `Tag::Image` is only a task-marker barrier
     (`src/note/parser.rs:234`); `![](x.png)` produces **no outlink** — Traces never
     tracked markdown image targets, only `![[..]]` embeds (`embedded: true`).
     Ceding image-target existence to MD057 therefore costs Traces nothing.
- Also confirmed: the parser already iterates `.into_offset_iter()` and threads link
  spans into `handle_event` (`src/note/parser.rs:109,241-247`) — pulldown link spans
  exist today; persisting them on `Link` is ticket 11's remaining work, not new
  parser machinery.

---

## §11 Round-2 review consolidation (2026-10-15)

Three review passes ran on the round-2 proposals (Q13–Q16) plus re-verification of locked
round-1 items: (1) conflict stress-test against every ticket/map/rumdl digest with live
rumdl 0.2.79 and GitHub-render probes; (2) performance verification via internet search
against competitor benchmarks with URLs; (3) CodeGraph-grounded design alternatives,
least-change-first. Part files deleted; findings and the strengthened decision set below.

### 11.1 Revised round-2 decisions

**Q13-R2 (frontmatter Error): keep Error, sharpen the premise.** 19:88 already declares
Error for missing `---` delimiters "where no intelligence is possible", consistent with the
Q5-R Error budget. Corrected premises (live rumdl 0.2.79 probe): MD041 (Warning) fires on
every broken shape; MD065 (Warning) fires only when the leading `---` parses as a horizontal
rule (missing closing delimiter); both are suppressible only by disabling the rule itself
(per-file `<!-- rumdl-disable MD041 -->` or vault-wide `.rumdl.toml`/`disableRules`), and
per-file suppression is impractical for transient mid-edit breakage. Traces' Error is a
frontmatter-parse code (`frontmatter.missing-delimiter`), not a heading judgment, so it does
not duplicate rumdl's ceded heading rules (15:195). Ships as a one-paragraph coexistence
note into 32 (see Q16-R2).

**Q14-R2 (docs href): raw anchors + generated catalogue (blocker fixed).** As originally
drafted every link would be dead: GitHub's heading slugger strips dots (`#links.wikilink.unresolved`
matches nothing; verified on live blob HTML, `gh api markdown`). Revised:
`codeDescription.href = https://github.com/JackMatanky/traces-pkm/blob/main/docs/diagnostics.md#<fragment>`,
where `<fragment>` is an explicit registry field equal to the code; `docs/diagnostics.md`
carries one raw anchor per code (`<a id="<feature>.<rule>"></a>` above the heading; GitHub
rewrites to `user-content-` prefix but preserves dots). The rule table is **generated from
the registry** with a golden-diff cargo test (public-api baseline pattern, mise.toml:137-143),
replacing a mere parity assertion: drift becomes impossible, not merely detected. One live
browser check of dotted-fragment click-through is a pre-adoption task; fallback is a shared
slug function used identically by emitter and test. No docs site exists (rumdl's per-rule-page
pattern unavailable); repo is public, so blob hrefs resolve.

**Q15-R2 (escalation keys): per-type codes enumerated; enum registry; hint added; enum deleted.**
- **Blocker fixed**: the registry enumerates link codes per kind now:
  `links.wikilink.unresolved`, `links.wikilink.heading-ref.missing`,
  `links.wikilink.embed.missing`. No `links.markdown.*` codes (ceded to MD057, Q8-R);
  ambiguity has no code. Every round-1 reference to `links.unresolved` is superseded; it
  survives only as a docs grouping heading, never as an emitted code.
- Override vocabulary on every key: **`hint|info|warn|error|off`** (hint added; 23's A1
  is Hint and users must be able to remap to/from it). `off` = 15's Silent.
- **One demotion level, the stronger one**: 15's `UnresolvedLinkPolicy` enum is **deleted
  from the design** (not "sugar"): per-link-type strictness exists only as O1 override keys
  documented in `docs/diagnostics.md`; 15's configuration-model section (15:155-166) is
  amended to the key form. This supersedes Q6-R's "override-key sugar" wording.
- **Registry shape (design review)**: `enum DiagnosticRule` with per-variant `code()`,
  `default_severity()`, and `const ALL` for iteration (CliError precedent,
  src/cli/error.rs:258-339), **not** a static row table: exhaustiveness forces a severity
  on every new rule; ~40 codes counted across 15/17/18/19/20/21/22/23 fit ergonomically.
  Keep dotted `<feature>.<rule>` spelling (do not inherit miette's `traces::cli::…` `::`).
  Disjointness = cheap ALL-iteration test (unique full codes, unique feature prefixes).
- **OverrideSet**: deliberately thin new type (code→override map) built at the LSP boundary
  from O1; reuse only the `merge_optional` precedence idiom (src/config/builder.rs:208-213),
  NOT ConfigService machinery (trust-gated TOML is the wrong source model; `deny_unknown_fields`
  blocks smuggling O1 keys into project config pre-31). Record OverrideSet as the exact type
  31 must extend.
- Placement: inputs-pending on 34; constraint recorded: registry must be a leaf module
  reachable from feature modules and the LSP boundary.
- **BrokenLink side-channel (adopted, designed)**: shared internal parse returns
  `(Note, Vec<BrokenLinkRef>)`; `parse_markdown` wraps and discards the Vec (sole production
  caller src/index/service.rs:276 unaffected; Note/serde/index untouched; callback returns
  `None`, never resolves). Owner 15, coordination note to 11 (independent parser edits).
  Pseudo-outlinks recorded-and-filtered rejected as a parallel-model violation; post-hoc
  second parse rejected as shallower and slower.

**Q16-R2 (amendment list): extended per conflict findings.** Full list now:
- **Outbound amendments**: 12 (§1 owed by 22); 14 (§2 **superseded**, not scope-limited: no
  in-handler sleep; debounce = spawn + generation counter; T1 100ms, T2 ~50ms; β is the
  publication path for all diagnostics); 15 (cession amendment per Q8-R: code action
  drop/repoint, 15:198 flag flip, `.rumdl.toml` guidance primary (15:209-213), embed narrows
  to `![[..]]`; plus UnresolvedLinkPolicy enum deletion (15:155-166); plus
  `links.reference-undefined` scope question); 17 (drop undefined/unused/duplicate to MD066,
  retain self-reference + blockquoted-definition); 18 (amend to carry day-rollover);
  19 (record Q13-R2 ruling); 20 (`source` to Scheme C; `debounce_ms` default 300 (20:97) →
  150→50-class T2 value, key survives as optional override; Q14's "per-source severity"
  (20:56) → per-code O1 keys); 21 (300ms (21:39) → T2; Phase-2 "pull diagnostics" (21:64)
  withdrawn as a transport decision); 23 (adopt 23's proposed severity set verbatim,
  closing "23 proposes, 25 decides": A1 Hint, B1/B2/B3/B5/D1/D2/C1 Warning, C2/C4/C5/C6
  Information, B4 dropped, B6 default-off; source `traces-tasks` → Scheme C; delivery line
  → Q1-R transport with T2 ~50ms; "per-source override precedent = 20:Q14" → per-code keys);
  22 (β generalized beyond templates; fix 22:34's citation map.md:31 → :32; 14 §2 amendment
  worded as superseding); 33 (transient tier → bounded ~10MB cache; workspace-scoped,
  conditional on 30); map.md corrections: :32, :34, :52, :75, **:80, :81, :82**, :83
  (also image-target cession + per-code escalation), **:87, :88, :90**.
- **Input receipts (owed rows answered, not just outbound)**: 22:72's inputs list accepted
  with per-item pointers; 22:64 schema-file v2 deferred to 20/31 with a named vehicle;
  21:68 and 23:37 seams answered by Q5-R plus per-ticket severity adoptions.
- **Appends to open tickets** (status sweep correction: 26, 27, 28, **29, 30, 31, 32, 34,
  41** are open; earlier notes wrongly counted 29 and 32 as resolved): **32** (post-cession
  ownership table, Q13-R2 frontmatter note, Hint asymmetry line, transport attribution facts);
  **29** (capability fields: `text_document.diagnostic`, refresh support, `data` support;
  push gate depends on pull-capability detection at initialize); **31** (`[lsp.diagnostics]`
  zk-shape per-code overrides land here; `[tasks.diagnostics]` lands here; `[schemas.diagnostics]`
  stays an emission gate); **34** (diagnostics fold module home pending); **41** (byte-precision
  closed-file diagnostics need link ranges; 41-B vs 41-C decides cold-compute re-read; the
  optional blake3 sidecar cache question routes **here**, not to resolved 13, which is
  unchanged: a sidecar is not a redb table).

### 11.2 Round-1 amendments requiring re-confirmation

- **Q3-R T2: ~150ms → ~50ms typical**, adaptive clamp [50ms, measured compute, 500ms]
  (clangd D73949 shape), coalesced per URI, equality early-cutoff. Evidence: clangd 50ms
  floor; Zed client pull 50ms; gopls tens of ms open-file; ty 1–4ms measured pulls;
  quick-lint-js 0.24–0.38ms; **ts-ls is 200+50ms and Deno 200ms** (both slower than our
  previous 150ms; the §10.1 "ts-ls 50ms" citation was wrong). Client debounces stack
  (Neovim 150ms), so perceived = client + T2. Markdown keystroke compute is 0.1–0.5ms:
  no expensive rebuild to amortize, so a low floor is honest. T1 100ms and T3
  event-driven stand.
- **Q11-R primary reversed**: facade **old/new Arc snapshot-diff** is primary (zero core
  changes): dirty set = FileMeta symmetric difference over `entries()` (pub,
  src/index/entry.rs:108; FileMeta PartialEq src/file.rs:43); referrer set = union of old
  and new `FileEntry::inlinks()` (pub, entry.rs:168) per dirty path. Retarget soundness:
  a referrer's edges are a pure function of its content and the file set; any edge move
  without the referrer changing implies some candidate file changed, and the moved edge is
  observable in that dirty path's inlink union (deterministic tie-breaks). O(N) per refresh
  rides the whole-tree scan RefreshPlan::collect already does. The `pub(crate)` delta
  widening is **demoted to fallback** (retained value: exact-edge consumers like 41);
  RefreshReport expansion stays rejected; baseline pub-tokens lines 473-474 stay untouched.
  Caveat: mtime+size equality is the indexer's own staleness envelope; no new failure mode.
- **Perf budgets paragraph revised**: document pull warm p95 ≤10ms (cache hit, µs in-lane),
  cold p95 ≤100ms, 300ms absolute ceiling; workspace pull during warm-up = streamed partials
  with **~50ms flush throttle** (ty #19657); scope-B additions: **long-polling**
  (ty #19670, stops client 2s re-polls) and **server-initiated `$/progress` every ~50–100ms**
  (ty #19616). Explicit rejections recorded: on-type workspace sweeps (luau-lsp #1076 had to
  leave on-type) and in-handler synchronous recompute (markdown-oxide #473 crash loop).
  Warm-up 1.5–2s @20K confirmed bracketed by external data (rumdl 0.45ms/file incl. startup;
  53-rule lint 1.46ms/file upper; rules-only 0.27ms/file lower); per-file <20ms, ~10MB
  cache, cold start <2s all confirmed top-tier; benches owed to 33 remain the gate.

### 11.3 Additional design verdicts adopted

- **Q4-D1 fold**: adopt, plus one sentence: workspace/class-level sources (schema-TOML
  construction diagnostics) publish via the T3 lane to their own URIs, outside the
  per-document fold. All six emission sites audited; none needs a channel the
  `SchemaConstruction` precedent lacks.
- **Q3-D4 β**: adopt with two notes: template-specific single-owner `&mut` env must not
  inflate `&mut` through read-only non-template computes; the open-during-warm-up race is
  named explicitly (a doc opened mid-warm-up must neither receive a stale warm-up result
  after its first interactive compute nor lose open-doc lane diagnostics to a late cache
  write; generation counters, scope B, are the mechanism).
- Q15 placement constraint and 31-extension record as in Q15-R2 above.

### 11.4 Facts and corrections added by this pass

- Open tickets are 26, 27, 28, 29, 30, 31, 32, 34, 41 (29 and 32 previously miscounted
  as resolved).
- GitHub blob rendering facts: heading slugger strips dots/parentheses, prefixes
  `user-content-`; raw `<a id>` anchors are preserved (dots intact) and rewritten with the
  same prefix.
- ts-ls debounce evidence correction (200+50ms, not 50ms); rust-analyzer #8636 (dead URL)
  replaced by #1331/#17491; zk 1000ms and Marksman 200ms/68s stay `[?]` internal-only.
- markdown-oxide #473: a real crash-loop failure mode from per-keystroke synchronous
  recompute under vault lock (cautionary evidence for β/coalescing/off-lane rules).
- 22:34 cites map.md:31; the debounce bullet is map.md:32.
- Research §0/§3.1 still show pre-revision T2 figures; §10–§11 supersede them (research is
  not retro-edited to match tickets).

### 11.5 Unverifiable / pre-adoption tasks

1. One live browser check: dotted-fragment (`#<feature>.<rule>`) click-through to raw
   `<a id>` anchors on github.com blob pages (F2 fallback: shared slug function).
2. Whether VS Code's Problems panel lists Hint (affects visibility of 23's A1-Hint and the
   hint override vocabulary); carried UNVERIFIED since §6.2 T3.
3. VS Code cross-server dedup key (source+code+range) not re-fetched this pass; leans on
   §5.3's carried claim.
4. O1 plumbing (workspace/configuration / initializationOptions flow) is greenfield
   (no src/lsp/); ticket 29's territory.
5. 15's internal tension (reactive AmbiguousLink for duplicate headings at 15:70-77 vs
   "no diagnostic for ambiguity" at 15:137,149): affects the code-count upper bound only;
   noted for 15/25 resolution.
