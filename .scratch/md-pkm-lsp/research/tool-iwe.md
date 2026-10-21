# Tool Study: iwe (iwe-org/iwe)

Research document for the Traces Markdown/PKM LSP design map (`.scratch/md-pkm-lsp/map.md`).
Covers the eight pre-review questions. Corpus evidence from
`docs/digests/iwe-org-iwe-digest.txt` (the superset digest: docs + src; line numbers below
are cited as `L<n>` against that file). Q8 (adoption/maintenance) is the only axis that uses
web search and is explicitly labeled as such.

**Version state of the evidence.** The src digest reflects iwe v0.29.0-era code
(2026-10-20; `crates/iwes/CHANGELOG` L38408). The docs digest includes `docs/comparison.md`
verified 2026-09-03 against v0.23.0. Both are first-party docs but the corpus mixes a
~4-week-old documentation layer with current source; version-sensitive claims are flagged
in each part. Where docs and code disagree, the code wins and the disagreement is called out.

**Source classes used.** (a) src: `iwes` LSP server, `iwec` MCP server, `diwe` core engine,
`liwe` shared model; (b) docs: feature pages, `comparison.md` (first-party but carefully
hedged — treat as self-reported), `benchmark.md` (first-party, methodology stated openly),
`keys.md`; (c) criterion benches in-repo; (d) web, Q8 only.

---

## Part 1 — Runtime shape (Q1; tickets 35/09/12)

**Finding: iwes is a single-threaded, fully synchronous `lsp_server` process with one
panic-isolated select loop. No async runtime.**

Workspace layout (L1–535): four binaries over one core — `iwe` (CLI), `iwes` (LSP),
`iwec` (MCP, tokio + rmcp, stdio + streamable-HTTP transports, L34534+), plus the shared
crates `diwe` (engine) and `liwe` (model). The MCP server is the only surface with an async
runtime; iwes has none (`crates/iwes/Cargo.toml` L38334–38375: `lsp-server`,
`crossbeam-channel`, `rayon`, `fuzzy-matcher`, `once_cell` — no tokio).

The transport is `lsp_server::Connection::stdio()` (L39431) with a
`crossbeam_channel::unbounded::<FsChange>()` side channel created in `main_loop`
(L39288, L39321). `main_loop` (L39308–39368) does, in order: detect the Helix client from
`client_name` (L39309–39314, `LspClient` enum L39569–39573), decide watching
(`watch_filesystem = params.state.is_none()`, L39316), build the initial graph state from
either `new_from_hashmap(state)` or a disk scan `new_for_path(...)` (L39323–39350), start
the bundled watcher (poll or native, L39357–39364), then hand off to `Router::run`.

`Router::run` (L39630–39671) is the whole runtime:

```rust
loop {
    let idle_rebuild = if self.server.search_index_is_dirty() {
        after(IDLE_REBUILD_DELAY)            // 300ms, L39567
    } else { never() };
    select! {
        recv(receiver) -> message => { /* panic::catch_unwind(handle_message) */ }
        recv(fs_events) -> event => { self.on_fs_event(event); }
        recv(idle_rebuild) -> _ => self.refresh_search_index(),
    }
}
```

Three properties worth noting for the runtime-model tickets:

1. **Everything is inline on one thread.** Handlers, graph mutations, search-index
   rebuilds, and workspace-write dispatch all run in the `recv(receiver)` arm. There is no
   worker pool for requests (rayon appears only inside index *rebuilds* and CLI paths).
   A slow handler stalls every subsequent message, including `$/cancelRequest` — which
   iwes does not implement at all (no handler anywhere; grep across L38000–48000 returns
   only the three unrelated `cancel` hits at L34624/L35710/L103454).
2. **Per-message panic isolation.** Each message is processed inside
   `panic::catch_unwind(AssertUnwindSafe(...))` (L39645–39657); a panicking handler is
   logged and the loop continues. Unhandled *requests* get a `MethodNotFound` response;
   unhandled notifications are logged at debug (pattern visible in the dispatch arms
   L39777–39870).
3. **`Arc::get_mut` as the mutation gate.** `Router` is `Clone` and holds
   `Arc<Server>` (L39584–39589). Every mutation path goes through
   `Arc::get_mut(&mut self.server)` and degrades to
   `error!("Failed to get mutable reference to server")` when the refcount exceeds 1
   (L39674–39681, L39704–39710). This works because exactly one Router exists per
   process; the design quietly assumes single-connection-per-process rather than
   defending multi-client sharing.

The Helix quirk handling (L39309–39314) exists because Helix's client sends requests
( notably code-action resolve) that other clients don't; the workaround lives in the
code-action path (§Part 6).

**Implications for tickets 35/09/12.** A synchronous, single-threaded `lsp_server` loop is
a shipped, battle-tested precedent for this exact domain at this exact corpus scale — it is
not a toy. But the two gaps it carries (no cancellation, all handlers block the loop) are
the precise reasons rust-analyzer-style designs move work off-thread; Traces should treat
"inline-on-one-thread" as a deliberate simplification with known costs, not a free win.
The per-message `catch_unwind` and the `Arc::get_mut`-or-log mutation pattern are both
cheap resilience idioms worth copying verbatim.

---

## Part 2 — Filesystem watching design (Q2; ticket 37)

**Finding: the map's claim of "zero server-bundled watcher precedent" is wrong for iwe.
iwes bundles a `notify`-based watcher, with a poll fallback, debounce, gitignore-aware
filtering, and open-buffer deferral — all tested.**

The watcher lives in `diwe/src/watcher.rs` (L6592–6974). `start_watcher(base_path, format,
PathFilter, debounce_ms=300, handler, cancel_token)` uses `notify`'s
`RecommendedWatcher`; `start_poll_watcher` is the identical API plus a poll interval, used
as fallback (both imported at L39290; selection at L39357–39364 — poll mode is also
selectable via `ServerParams.watch_poll_interval`, L39305). Events are debounced ~300ms
through a spawned debouncer, then `dispatch` filters each path twice: extension match via
`path_to_key` (only `.md`), and a `PathFilter::includes` check built from the workspace
gitignore via the `ignore` crate's `Gitignore` (diwe/fs.rs L2646, L2695–3157). Renames
surface as Remove events (notify-level handling in watcher.rs); the net behavior is
verified by test `keeps_a_single_document_after_external_rename` (L52917–52928) — after a
rename the corpus converges to exactly one document.

Events leave the watcher as `FsChange::{Update(Key, String), Remove(Key)}` over the
crossbeam channel (L6592–6974 definition; send sites within), and arrive at
`Router::on_fs_event` (L39673–39683), which calls `apply_external_update` /
`apply_external_removal` on the graph (see §Part 3 for the open-document guard).

**Dynamic registration: none.** iwes never calls `client/registerCapability` (grep across
the full iwes source range: zero hits). It *accepts* `workspace/didChangeWatchedFiles`
notifications (dispatch arm L39777–39786, handler L40169–40181) but processes only
`FileChangeType::DELETED`; `CREATED` and `CHANGED` are explicit no-ops because the bundled
watcher owns those. The server does not advertise file-operation capabilities
(`did_create`/`did_rename` etc. all `None`, L39479–39484). Whether the VS Code extension
registers watched-file patterns client-side was not examined (extension not in corpus —
flagged in §Part 9).

**The MCP process watches too.** `iwec` runs its own watcher over its own
`Arc<Mutex<Graph>>` (`crates/iwec/src/watcher.rs` L34636–34685, `start(graph, ...)`
at L34675). Two processes, two watchers, one directory (see §Part 5).

Test coverage is behavior-level and worth reading as a spec (L52797–52928):
- deletion batches and split batches both reflected in workspace symbols;
- deleting an *open* document keeps it in symbols until close;
- external create/modify/delete all picked up (with `wait_for_symbols` polling helper).

**Implications for ticket 37.** iwe is the closest existing precedent to what Traces is
designing: bundled `notify` watcher, 300ms debounce, gitignore filter, poll fallback, and
the crucial open-buffer deferral rule. The design decisions Traces still owes an answer to
that iwe has already answered: (a) DELETED-only handling of client-sent watched-file
events (client events are a backstop, not the source of truth); (b) rename = Remove +
create pair at the notify layer, made safe by idempotent upsert; (c) extension-match +
gitignore double filter before any graph work; (d) one watcher per process is iwe's
accepted cost — Traces must decide explicitly whether an LSP+MCP dual-process layout can
afford that.

---

## Part 3 — Buffer overlay / state model (Q3; tickets 13/14)

**Finding: the overlay contract is "open buffer wins; disk reconciles on close; the
watcher fills everything else." Fully source-verified, including five reconcile tests.
There is no persisted or warm-started graph state — ever.**

Server state (L39963–39973, construction L39996):

```rust
pub struct Server {
    graph: Graph,                       // in-memory document graph
    search_index: SearchIndex,          // BM25 + path list (see Part 4)
    search_index_dirty: bool,
    open_documents: HashSet<Key>,       // overlay membership
    key_set_changed: bool,              // inlay-hint refresh trigger
}
```

Lifecycle of a document through the overlay:

- **didOpen** (L40059–40069): insert key into `open_documents`; store the text only if it
  differs from the graph's current content (dedup against the watcher's copy).
- **didChange** (L40158–40167): full-document sync — capabilities advertise
  `TextDocumentSyncKind::FULL` (L39456). Takes `content_changes.first()`, stores, marks
  the search index dirty. Brand-new unsaved files are first-class: a didChange on an
  unopened URI creates the document and it appears in workspace symbols immediately
  (test `new_file`, L46126–46138).
- **didSave** (L40148–40156): stores only `if let Some(text) = params.text`. The
  capabilities advertise `include_text: false` (L39458), so in practice didSave is a
  no-op for typical clients — disk convergence is delegated to the watcher. This is a
  quietly important design choice: **save is not a state transition; the watcher is.**
- **didClose** (L40071–40095): remove from `open_documents`, then reconcile against disk
  directly — re-read the file; if present and different, store it; if absent, drop the
  document. This is the only reconcile point, and it is disk-read-based, not
  snapshot-diff-based.
- **External events while open** (`apply_external_update` / `apply_external_removal`,
  L40037–40057): both early-return when `open_documents.contains(&key)` — the buffer is
  authoritative until close. Identical-content upserts are also deduped.

The reconcile test file (L51295–51374) pins the semantics with five tests that read like
a contract:
1. external edit to an open document applies on close;
2. unsaved buffer edits survive an external write until close (buffer wins, then disk wins);
3. external delete of an open document applies on close;
4. open buffer content wins over disk (open-time content immediately visible);
5. an unsaved new document is dropped on close.

**State handoff / embedding surface.** `ServerParams.state: Option<HashMap<String, String>>`
(L39295–39306) is a raw key→markdown map. When present, `main_loop` builds the graph via
`diwe::fs::new_from_hashmap` — skipping the disk scan entirely — **and disables the
filesystem watcher** (`watch_filesystem = params.state.is_none()`, L39316) and forces
`sequential_ids: Some(true)` (L39329). This is not serialized-graph warm start: the graph
and search index are rebuilt from raw markdown every time; there is no on-disk index
artifact anywhere in the design. Its consumers in-corpus are tests (fixture
`with_state`, L47886) and the embedding path used by the Mac app (inferred; flagged
unverifiable in §Part 9).

**Inlay-hint push.** `key_set_changed` (taken via `take_key_set_changed`, L40003) triggers
`workspace/inlayHint/refresh` — but only if the client has ever requested hints
(`inlay_hints_used: Arc<AtomicBool>`, L39588, L39694–39702), and only after a 100ms
delay implemented as a spawned thread + `sleep` (`delay_send`, L39602–39610). Refreshes
fire on FS events (L39682) and on code-action resolve, not on every keystroke.

**Implications for tickets 13/14.** The overlay model is small enough to state in one
sentence and is enforced by five tests — this is the shape Traces should aim for: buffer
wins while open, close-time reconcile, watcher-as-truth-for-disk, dedup on identical
content at every boundary (open, external update). Two things Traces must decide that iwe
answered by fiat: didSave-without-text is a deliberate no-op (watcher owns persistence),
and the embedding/state param disables watching rather than trying to keep a hybrid —
simpler, but it means an embedded server can go stale with no repair path.

---

## Part 4 — Search index lifecycle and performance honesty (Q4; ticket 33)

**Finding: full BM25 rebuild, coalesced by a dirty flag and a 300ms idle timer — not
per-document incremental. The benchmark measures graph load and query separately and
explicitly does not measure LSP latency. The feature docs' "maintained per edit"
incrementality claim is marketing-speak the code does not support.**

`SearchIndex` (L42725–42730):

```rust
pub struct SearchIndex {
    paths: Vec<SearchPath>,        // section-grain fuzzy paths
    bm25: Option<Bm25Index>,       // full-text over title + body
    rebuilds: usize,               // diagnostic counter
}
```

`update()` (L42741–42792) is unambiguously a full rebuild: `bm25 =
Some(diwe::search_query::build_index(graph, language))` re-tokenizes every document
(parallelized with rayon at ≥128 docs, search_query.rs L5395–5430; BM25 tokenizer cost in
tokens.rs L6323–6592), and the `paths` list is rebuilt by walking **every** section id
with `par_iter`, collecting parent titles via inclusion edges, and sorting/deduping. The
`rebuilds` counter is the code's own admission that this is a rebuild, not an update.

What makes it cheap enough in practice is **coalescing**, at two levels:

1. Every mutation (didChange, didSave, external update/removal, didOpen, didClose) sets
   `search_index_dirty = true` (L40045, L40056, L40068, L40094, L40154, L40166);
   `refresh_search_index` is a no-op when the flag is clean (L40028–40035).
2. `refresh_search_index` runs (a) at the top of `handle_message` for **every** request
   (L39712–39714) and (b) on the 300ms idle timer when no messages arrive
   (L39634–39668).

The consequence is the direct answer to the staleness question: **there is effectively no
observable staleness for pull requests.** A dirty index is rebuilt inline, before the
request is dispatched. The cost is a stall on the first request after an edit, not
staleness. The 300ms idle path only helps when the user pauses; during active typing it
merely bounds how long the dirty state can linger without traffic. Tests
`did_change_reindexes_body_for_search` (L46142–46159) and `did_delete_removes_document_from_search`
(L46163–46183) confirm post-edit search correctness, not timing.

**Doc-vs-code discrepancy (flag).** `docs/feature-overview.md` states "Incremental: the
index is maintained per edit, so long editor sessions stay current" (L~99400 region,
feature-overview feature-search bullet). The code is full-rebuild-coalesced. The index
does stay *current*; it is not maintained *incrementally*. Traces should not cite iwe as
an incremental-indexing precedent.

**Benchmark honesty.** `docs/benchmark.md` (L92635–92913) and the criterion harnesses
(`crates/iwe/benches/load.rs` L9815–9860, `query.rs` L9861+, `common.rs` L9692+) measure:
- **graph load** at 5k/10k/20k generated docs (seed 42), plain vs indexed —
  802-doc corpus: plain load median 38.52ms, indexed 46.01ms; 20k: ~705ms plain,
  ~958ms indexed (M3 Pro);
- **query latency** on a preloaded graph (filters, graph operators, find).

The docs state outright "We don't benchmark iwes" — these are library numbers, **not**
time-to-first-LSP-response, and they exclude watcher startup and per-request overhead. The
public headline "20,000 notes parsed in 0.6s" (iwe.md, see §Part 8) is this benchmark.
For comparison against Traces' target (cold <2s at 20k files, ticket 33): iwe's *indexed*
load number (~958ms on an M3 Pro for its corpus shape) is the honest apples-to-apples
figure, plus watcher warm-up iwe never measures.

**Implications for ticket 33.** Two decisions Traces owes: (1) is full-rebuild-coalesced
acceptable? iwe's shipped experience at 20k docs suggests yes — but iwe has **no LSP
latency benchmark under typing load** to prove the stall is invisible; Traces should not
repeat that gap. (2) If Traces claims incremental indexing in its own docs, it must mean
per-document updates; iwe's counter-example shows how easily the word gets used for
coalescing instead.

---

## Part 5 — MCP server vs LSP process split (Q5; tickets 36/34)

**Finding: two separate processes, each with its own graph, its own watcher, and no
shared state of any kind. Consistency exists only through the filesystem. The shared core
library is real and is the load-bearing architectural claim.**

Three surfaces over one core (workspace tree L1–535):
- `iwe` CLI — thin shell over `diwe`/`liwe`;
- `iwes` LSP — sync, single-threaded (§Part 1);
- `iwec` MCP — separate process, `tokio` + `rmcp`, stdio **and** streamable-HTTP
  transports (L34534+), 14 tools: `iwe_find`, `iwe_retrieve`, `iwe_tree`, `iwe_stats`,
  `iwe_squash`, `iwe_create`, `iwe_update`, `iwe_delete`, `iwe_query`, `iwe_rename`,
  `iwe_extract`, `iwe_inline`, `iwe_normalize`, `iwe_attach` (L34055–34072), plus prompts
  (`explore`, `review`, `refactor`) and resources (docs/tree/stats/config) per
  `docs/mcp.md` (L100924+). Dry-run preview is supported on every write tool.

iwec's state: `IweServer::new(base_path, config)` + `start_watching()`
(L34599–34600); the server owns `Arc<Mutex<Graph>>` (L34648–34685) and spawns the same
diwe watcher against it (L34675). The LSP process owns none of this and vice versa.

**Answer to the map's open question ("does the MCP share the LSP's state?"): no.** No
IPC, no shared memory, no state handoff, no common daemon. If an editor writes a note and
an agent queries iwec milliseconds later, iwec sees it only after its own watcher fires.
The design trades consistency-latency for total process isolation. The consistency story
is "the filesystem is the single source of truth and every process watches it" — the same
principle as git.

`docs/feature-overview.md`'s architectural claim — "the CLI, the LSP server, and the MCP
server are thin shells over one domain model, so a new graph operation appears in all
three at once" (L~99400 region) — is corroborated by the workspace layout and by shared
type usage (`diwe::watcher::FsChange` in iwes, `liwe::model::Key`/`State` everywhere).

**Unverifiable gap:** `crates/iwec/src/lib.rs` is **absent from the digest corpus** —
IweServer's internals (mutex discipline around reads, tool→graph call paths, error
surface) could not be inspected. Flagged in §Part 9.

**Implications for tickets 36/34.** iwe proves "shared core, N thin shells" works in
production for this domain, and gives a concrete MCP tool list to argue against. But its
split has an explicit cost Traces should not inherit by default: duplicate watchers and
duplicate in-memory graphs per process, with filesystem-mediated eventual consistency
between them. Traces must decide deliberately between (a) MCP in the same process as the
LSP (shared graph, single watcher — cheaper, but couples agent traffic to editor
latency), (b) a shared daemon both attach to, or (c) iwe's full isolation. iwe chose (c)
because it also ships a CLI and a native Mac app that each need independent lives; if
Traces' MCP is primarily an editor-adjacent surface, (a) may dominate.

---

## Part 6 — Capability inventory (Q6; tickets 24/26/27)

**Finding: a deliberately document-granularity capability set — navigation, search,
refactor, hints — with no diagnostics, no semantic tokens, and two advertised-but-stubbed
capabilities. Rich where PKM users live; empty where compiler tooling lives.**

Advertised in `initialize` (L39433–39490): references, formatting, definition, hover,
completion (`resolve_provider: Some(true)`, trigger chars default `["["]`, configurable
list L39438–39450), workspace symbols, document symbols, FULL text sync, inlay hints,
inline values, rename with `prepare_provider: Some(true)`, code actions (kinds from
`all_action_types(config)`, `resolve_provider: Some(true)`), and file-operation
capabilities — all `None` (L39479–39484).

What actually works, from source + tests:

| Capability | Behavior | Evidence |
|---|---|---|
| Go-to-definition | link → target document; fragment-insensitive | feature-navigation L99285+; hover L40124 |
| References | backlinks across the corpus | tests L51379+; feature-navigation |
| Hover | target note as markdown, frontmatter stripped, untruncated | feature-hover-preview L98926+ |
| Completion | eager: builds **all** keys per request (O(N), no cache), collator-sorted; bracket prefix picks link shape (`[`→markdown, `[[`→wiki); `min_prefix_length` config | L40183–40208; feature-autocomplete L98335+ |
| Completion resolve | **identity** — `resolve_completion` returns the item unchanged (L40279–40281); resolve is advertised but lazy-loading is not implemented | L40279–40281 |
| Workspace symbols | section-grain `SearchPath`s, skim-fuzzy + BM25 blended (RRF per docs), `.take(100)` cap; empty query → top-100 by node rank | L40283–40292, L42800–42820; feature-search L99591+ |
| Document symbols | flat deprecated `SymbolInformation`, header tree filtered to depth < 4 | L40424+ |
| Rename | `prepareRename` → `RangeWithPlaceholder` (L40463); rename → WorkspaceEdit with create+delete file operations; link text preserved, targets rewritten | L40479; feature-renaming L99495+ |
| Code actions | ~10 kinds (extract, inline, attach, transform, sort, delete, link, list, section, extract_all) with custom kind strings; **resolve contract**: client round-trips `data: {key, range}` JSON, server re-resolves | L41336–41357, L40642; Helix 1-char-range workaround |
| Inlay hints | backlink count + parent title at note top; parent list on inclusion links; refresh pushed via 100ms-delayed `workspace/inlayHint/refresh`, only if hints were used | feature-inlay-hints L98954+; §Part 3 |
| Formatting | normalize links/headers/lists/wrapping | capabilities L39435; feature-auto-format |
| Folding / execute-command | present | capabilities block |

Stubs and absences (verified, not assumed):
- **`inline_value_provider` is advertised but the handler returns `vec![]`**
  (L39464, L40420–40422) — a registered stub.
- **No diagnostics.** Zero `PublishDiagnostics` in iwes (grep). `comparison.md`
  confirms: "None published by the LSP; `iwe stats` reports broken links in batch"
  (L96620).
- **No semantic tokens, no call hierarchy, no `$/cancelRequest`, no document
  highlight, no selection range, no signature help** (grep across iwes source: zero).
- Completion is eager O(N) per keystroke (§ above) — fine at 1–5k docs, a live question
  at 20k; the only mitigations are trigger characters and `min_prefix_length`.

**Implications for tickets 24/26/27.** iwe is existence proof that a markdown LSP can be
useful and well-regarded with **zero diagnostics** and document-granularity resolution —
the PKM surface (search, backlinks, rename, extract/inline refactoring, hints) carries the
product. Three directly reusable patterns: the `{key, range}` code-action resolve-data
round-trip; the custom action-kind strings fed from config (`all_action_types`); the
eager-completion-with-identity-resolve shortcut (simple, honest, but O(N) — Traces should
make its laziness choice deliberately rather than defaulting). The two advertised-but-empty
capabilities (inline values; resolve that resolves nothing) are warnings about advertising
capabilities before implementing them — clients light up UI for them.

---

## Part 7 — PKM semantics: pre-review claim check (Q7; tickets 24/26/27)

**All five pre-review claims verified — from source code, not only the first-party
`comparison.md`. iwe's PKM surface is real but narrower than a PKM-native tool: the gaps
the map attributed to it are genuine and current as of v0.29.0.**

1. **Zero LSP diagnostics.** Confirmed by grep (no `PublishDiagnostics` anywhere in
   iwes) and stated in `comparison.md` (L96620). Broken links are a batch CLI concern
   (`iwe stats`), invisible to editor squiggles.
2. **`#fragment` links are document-granularity.** The parser retains fragments —
   `Target::Path { key, fragment }` exists in the model (reference.rs L70624–70634) —
   but the LSP drops them: hover splits the URL at `#` (L40124) and navigation resolves
   by key alone; `docs/keys.md` documents it as intended behavior ("A `#section`
   fragment is dropped before the key is computed", L100769+). Fragments are parsed,
   stored, and ignored. Heading-level linking is not a feature iwe has and chose to hide;
   it is a feature iwe does not have.
3. **No frontmatter aliases.** Only CLI-flag and template aliases exist; keys are paths
   (keys.md L100769+; grep across corpus). No alias-aware resolution, no alias
   completion.
4. **Inline hashtags are not LSP features.** The `#tag` detection regex lives in
   `iwe init`'s workspace detection (liwe/src/init/detect.rs, L61630) — used to infer
   whether a folder is a knowledge base at scaffold time. There is no tag completion,
   navigation, or hover.
5. **No natural-language date resolution.** `comparison.md` concedes this explicitly:
   "Natural-language dates are better in markdown-oxide" (L96590–96781 region) — iwe
   resolves wiki-link path suffixes, not dates.

**First-party-doc handling.** `comparison.md` is self-reported but unusually honest: it
is dated (2026-09-03), versioned (v0.23.0), includes a "Where IWE Is Weaker" section, and
was cross-checked here against v0.29.0 source — every "weaker" claim above still holds in
the newer code. One version-skew note: iwes 0.27.0 (2026-10-16, CHANGELOG L38429–38435)
fixed stale backlinks/references/inlay-hints when a linking note was edited — i.e. a
correctness bug in exactly the backlink-refresh path §Part 3 describes, fixed within the
last week of the corpus. Treat backlink-refresh behavior as recently churned, not settled.

**Implications for tickets 24/26/27.** The differentiator space the map carved out for
Traces is genuinely unoccupied by iwe: diagnostics for broken/ambiguous links,
heading-fragment navigation, alias resolution, tag intelligence, date resolution. iwe's
own comparison document concedes most of it. The counter-lesson: iwe shipped 1.8k stars of
adoption *without* those features — the PKM floor that sells is search + backlinks +
rename + structural refactoring, and the map should not assume the differentiators are
load-bearing for adoption, only for capability parity.

---

## Part 8 — Maintenance and adoption (Q8) — WEB-SOURCED, non-corpus

**Everything in this part comes from web search (crawls and snapshots, October 2026), not
the digest corpus. Third-party crawler numbers drift; as-of dates are given inline.**

- **Repository:** github.com/iwe-org/iwe — **1,778 stars, 102 forks, 1 open issue, 354
  commits** (USASI API snapshot fetched 2026-10-18; org-page crawl same week agrees:
  "1.8k / 102"). An older crawl on the same research pass showed 1.4k stars (GitHub
  releases page cache, ~August 2026) and 1.6k (org page cache, ~September) — the project
  is growing steadily, not viral. Created 2024-09-20; last push 2026-10-17.
- **Maintenance:** effectively **single-maintainer** (@gimalay authors every release on
  the releases page). Release velocity is high: iwes 0.14.0 (2026-07-21) → 0.29.0
  (2026-10-20) — roughly weekly-to-biweekly for three months, corroborated by in-corpus
  CHANGELOGs (L38408+). Per-crate release-plz versioning means the CLI (`iwe` 0.19.1,
  Aug 2026) and LSP (`iwes` 0.29.0, Oct 2026) version numbers diverge — read GitHub
  "latest release" with care. Bus-factor risk is real but velocity has not slackened.
- **Distribution (multi-channel, unusual for a project this size):** Homebrew tap,
  npm `@iwe-org/iwe`, `cargo install iwe iwes iwec`, community conda-forge package,
  VS Code marketplace extension, Zed extension, `iwe.nvim` (54 stars), native Mac app
  (iwe-mac repo), and an official MCP Registry listing (`io.github.iwe-org/iwe`, per
  v0.19.1 release notes). Crates.io: **26,763 downloads** (iwe.md site, Oct 2026).
- **Positioning shift:** the current README describes iwe as "a shared memory for you and
  your AI agents" with four work modes (Mac app / editor LSP / CLI / Claude-Codex
  integration) — the LSP is now one of four surfaces, and the 2026 marketing pivot is
  clearly toward agent memory (topics now include `ai-agents`, `ai-memory`, `okf`;
  sister repos `skills`, `skills-eval-harness`, workspace templates). For the map: iwe's
  center of gravity is moving from "markdown LSP" to "agent memory substrate" — its LSP
  is in maintenance-grade maturity, still releasing, but the strategic energy is
  elsewhere.
- **Community:** GitHub Discussions + r/iwe + X presence; two org workspace templates
  (dev/marketing); OKF (Open Knowledge Format) conformance work with CI validation.

**Assessment.** Actively maintained, multi-distribution, real-but-modest adoption
(~1.8k stars, ~27k downloads). Strong release cadence; single-maintainer concentration
risk; strategic pivot toward agent-memory positioning. It is the most complete reference
implementation for the design map's domain — and its own docs acknowledge the
capabilities Traces is targeting do not exist in it.

---

## Part 9 — Caveats, limitations, and unverifiable items

**Evidence limitations.**
1. **`comparison.md` is first-party** (iwe's own comparison page). It is carefully
   hedged and dated, and every load-bearing "weaker" claim was cross-checked against
   v0.29.0 source — but its "stronger" claims (e.g. performance framing) inherit the
   self-report discount, and its verification date (2026-09-03) is ~6 weeks before the
   src digest.
2. **Benchmark scope.** `benchmark.md` measures library graph-load and query latency, not
   end-to-end LSP request latency; the docs themselves state "We don't benchmark iwes."
   No number in this document should be cited as an LSP responsiveness measurement.
3. **Version skew.** Docs describe v0.23.0; source is v0.29.0 (2026-10-20). Backlink /
   reference / inlay-hint refresh was actively bug-fixed in 0.27.0 (2026-10-16) — that
   subsystem was in churn within days of the corpus snapshot.
4. **Adoption numbers are third-party crawls** (USASI API snapshot 2026-10-18; GitHub
   and iwe.md page caches at various ~2026 dates). Cited with as-of dates; expect drift.

**Unverifiable from the corpus (explicit gaps).**
- `crates/iwec/src/lib.rs` is **absent from the digest** — `IweServer` internals
  (Graph mutex discipline, tool→graph call paths, error handling) were not inspectable;
  Part 5's MCP claims rest on `iwec/src/main.rs`, `iwec/src/watcher.rs`, and docs.
- **Who populates `ServerParams.state` in production** is inferred (test fixtures +
  the Mac app embedding path); no production call site is in the corpus.
- **The VS Code extension's client-side configuration** (whether it registers
  watched-file patterns or inlay-hint settings) was not examined — the server itself
  registers nothing dynamically, but the extension may.
- **`docs/search-ranking.md` and `docs/inclusion-links.md`** were read only via feature
  pages and source spot-checks, not end-to-end; RRF weighting details rest on
  `feature-overview`/`feature-search` plus the skim+BM25 code path.

**Scope note.** This document is analysis with implications only. No design decisions for
the map are made or recommended here; every "Traces should decide" statement defers to
the map's ticket process.
