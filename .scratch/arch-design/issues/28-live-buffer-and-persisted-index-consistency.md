# How should open editor buffers relate to the persisted index?

Type: grilling
Status: resolved
Blocked by: 12, 23, 24, 25

## Question

The LSP is a committed long-lived consumer. The MCP is a likely consumer. An
editor can hold unsaved content that differs from the persisted vault and the
index. Decide the source-of-truth and consistency rules for that state:

- Decide which reads observe the buffer and which reads observe the persisted
  generation.
- Decide how queries and links observe edits in an open buffer.
- Decide how the save, close, deletion, rename and external-change transitions
  reconcile buffered and indexed state.
- Decide the host and request ownership rules for the overlay on a pinned
  generation.
- Record the multi-process limits that the store lock sets. Decide the overlay
  rules inside them.
- Place the open-buffer schema check that ticket 20 hands to this ticket.

Type: HITL. Grill the user. Do not answer for them.

Tickets 23, 24 and 25 are resolved. This ticket resolves on top of them. Keep
protocol-specific behavior outside the shared semantics. Do not move protocol
behavior into core unless a host creates a real variation.

## Inputs from the forward-consumer decision

See [ticket 17's
Answer](./17-grilling-forward-consumers-and-design-pressure.md).

- The LSP is the next major consumer. The MCP is a likely near-term consumer.
- Source-aware editor features are the design pressure: diagnostics, navigation
  and completion.
- General note write-back is not a requirement of this redesign.
- Ticket 17 names index and editor-buffer consistency a sharp decision. It gives
  the decision to this ticket.
- The intent evidence is `.scratch/md-pkm-lsp/`: an LSP map with a live buffer
  overlay, spans, cancellation and multi-root.
- Core stays synchronous. Async lives at host or transport seams.

## Inputs from the note document model decision

See [ticket 23's Answer](./23-note-document-model.md).

- `note` gets `NoteDoc::scan` and a public `parse_with_doc` for an unsaved
  buffer. Ticket 23 promises no incremental reparse.
- The host owns the text, the version, the debounce, the revision gate and the
  UTF-16 conversion.
- A stored position never drives an edit. An edit reads the current bytes.
- The open buffer wins over disk and over background work.
- A closed-file edit compares the full source snapshot at commit.
- Ticket 23 leaves the host memory at 50,000 notes and a two-process store open.
  This ticket shares that evidence need.

## Inputs from the index read model decision

See [ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- The store keeps 11 facets: files, fields, tags, classes, headings, outlinks,
  links, tasks, lists, notices, meta.
- Ticket 24 owns the generation. This ticket owns the buffer overlay on a pinned
  generation.
- Each request pins one generation. A row owns its matching `Arc<FileEntry>`.
- A host opens the store only to commit a write batch. Then it closes the store.
- A host may publish a coherent in-memory generation after a failed persistence.
  It must build that generation from a validated pinned base.
- Persisted parse-time notices ride with the file facts. A path-scoped read gets
  them.
- A CLI process in another terminal can query when it can parse the sources.
- If required facets are missing or corrupt, the read has a complete cache
  miss. The result never looks silently complete.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- A host checks the sources within a staleness bound. It does not check before
  each query. This ticket and ticket 33 set the bound.
- A host holds the store only during a refresh and a persist.
- Each query result names the generation that it read. The pair is the
  incarnation and the revision.
- A host that meets a store with a higher format epoch never writes that store.
  It serves from its in-memory generation, or it parses fresh when it has none.
  Tickets 33 and 28 own that behavior.
- Each file row carries a `racy` bit. Set it when `mtime >= scan_start - 2 s`. A
  save inside that window re-reads on the next refresh.
- A file with an unchanged size, mtime and (on Unix) ctime counts as unchanged
  outside the racy window. `traces index --verify` checks the rest.
- After an uncertain commit, the host drops the handle and the plan. Each
  persist tries the write-open again.
- Crash recovery without a journal is a no-op refresh of the uncommitted files.
  It costs 12 to 65 ms.
- Ticket 27 proposes a numeric bound. The next command produces a queryable
  store within one completed publish cycle of the same host. Measured recovery
  plus first query is 361 to 450 ms at 5k against a 770 ms cycle. At 50k it is
  6.2 to 6.3 s against a 10.8 s cycle. The runs are VM figures. [Ticket
  33](./33-workspace-host-and-composition.md) re-measures the bound on real
  hosts.

Cross-reference check [V]: ticket 25 now names tickets 33 and 28 in
"Confirmed decisions". Its "Host store use" change row names tickets 28 and
33. The earlier reference to ticket 27 is obsolete.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md).

- The check runs on an in-memory parsed buffer without persistence. The index
  never depends on schemas.
- The check is advisory. Each host sets gating policy over stable violation
  codes. The check does not gate.
- `check_note(before, after)` takes a path and content on each side. Stdin and
  dry-run inputs can lack a path. Path bindings then do not apply.
- In the LSP, diagnostics are feedback only. `didChange` cannot deny an edit.
  `willSaveWaitUntil` cannot abort a save.
- Ticket 20 hands the open-buffer check placement to this ticket.

## Inputs from the execution-context and config decisions

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md) and
[ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- One coherent index generation supplies the rows of an initial query.
- An admitted request keeps its immutable generation and may finish.
- A watcher only signals the need for possible revalidation. It does not enforce
  it.
- Core supports concurrent roots with different policies. The design builds no
  registry.
- A registry key cannot isolate two sessions that share `.traces/index.redb`.
- Tickets 24, 25, 32 and 33 own cross-process coordination, canonical path
  identity and store locking.
- `ConfigService::admit(&Config)` is the admission primitive. It returns
  current, changed or an error.
- A nearer config that appears later makes a different workspace. The host
  decides the response.
- Immediate cancellation stays a host-policy question. IWE supports no
  `$/cancelRequest` [V].

## Inputs from the journal and replay decisions

See [ticket 26's Answer](./26-measure-index-recovery-journal.md) and [ticket
27's Answer](./27-index-replay-prototype-and-fault-tests.md).

- Never write a fact from an unsaved editor buffer. This ticket owns the buffer
  overlay.
- A replay must change the revision. The generation pair after a replay is this
  ticket's rule to state.
- The journal is an optional optimization. Ticket 36 owns adoption and the
  `Durability::None` versus `Durability::Immediate` choice.
- A pure read-only operation never opens the journal.
- redb rejects the second process at open while a writer holds the file.
- In the tested stack, acknowledged `Durability::None` rows vanished at an
  `ENOSPC` failure boundary. `Durability::Immediate` rows survived. This is an
  adoption blocker for ticket 36. It is not a general redb guarantee.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE's LSP
has no overlay. `didOpen` and `didChange` write the buffer into the one
in-memory graph (full-document sync). `open_documents` records open keys.
Watcher events for an open key are dropped until `didClose`, when the file is
re-read from disk (`crates/iwes/src/router/server.rs`) [V]. A `didClose` of an
unsaved new document drops the content
(`crates/iwes/tests/reconcile_test.rs`) [V].
The CHANGELOGs show why. Last-write-wins let disk changes overwrite unsaved
buffers (fixed in v0.16.0 with editor ownership). Rebuilding the BM25 index per
keystroke stalled the server (v0.18.0, now batched after 300 ms idle) [V]. It
supports a single root and no `$/cancelRequest` [V]. The IWE MCP crate writes
mutations to disk and updates its own graph [V]. Two writers can touch one vault
through separate processes. [H] Because IWE has no persisted index, this settles
only the in-process view of buffers. It does not test the persisted-index
question of this ticket.
The primary changelog records the ownership fix under v0.16.0
(`docs/digests/iwe-org-iwe-src-digest.txt`) [V].
The 300 ms constant is in the same digest. Research 12 attributes the
ownership fix to v0.17.0; that version attribution is incorrect.

## Evidence limits

These limits describe the pre-grilling evidence. The Answer below records the
later prototype measurements and the limits that remain.

- The IWE ownership rule covers one process, one root and no cancellation [V].
  It is evidence for the in-process view only [H].
- The v0.16.0 and v0.18.0 fixes are changelog facts [V]. They are not
  measurements of a persisted-index design.
- No Traces host code exists. `.scratch/md-pkm-lsp/` states intent only.
- No measurement exists for overlay costs: parse per keystroke, debounce, or
  memory. The ticket 23 prototype scan took about 29% of parse time [V].
  The real `NoteDoc` builder cost is unmeasured. The 50,000-note figures are
  linear extrapolations [H] (ticket 23).
- The redb facts hold for redb 4.3.0 [V]. The experimental multiprocess mode
  refuses `Durability::None` [V].
- The replay and recovery figures come from Linux VM runs (ticket 27). They are
  not native latency or power-loss proof.
- Markdown-oxide is the nearest LSP precedent. It has an in-memory whole-vault
  lock, no persisted cache and a 300 ms debounce. Sources:
  `research/23-prep/h1-precedent-rumdl-iwe.md` and
  `research/24-prep/c1-digests-lsp-zk.md`, findings 12, 13 and 20 [V]. It shows
  the lock model, not the persisted-generation model.
- A change counter does not make a snapshot atomic
  (`research/24-prep/h1-stress-test-designs.md`) [V]. Examples: the `revision`
  of Dataview and the `diag_gen` of markdown-oxide.
- rust-analyzer and Tantivy give each request an immutable snapshot
  (`research/24-prep/z-grilling-guide.md`) [V]. A generation clone costs 120 µs
  at 50,000 entries (`research/24-prep/h1-stress-test-designs.md`, design G2)
  [V].
- The template engine refreshes per render and holds a `WorkspaceIndex` in its
  state (findings ledger). That is current-code evidence, not a decided host
  design.

## Open questions

Group 1: overlay truth and scope

- Which host reads observe unsaved content? Decide the set: all queries in the
  host, or a named subset.
- A CLI process cannot observe the buffer. State the accepted divergence between
  host answers and CLI answers.
- State where the overlay lives: per workspace, per root or per request. Ticket
  33 owns the composition. This ticket owns the semantics.

Group 2: overlay on a pinned generation

- State what a request pins besides the generation: the buffer revision, the
  buffer text, or nothing.
- State which generation an overlay entry attaches to. State what replaces the
  entry after a refresh publishes a new generation.
- State the coherence rule between an overlay entry and the generation row for
  the same path.
- State the generation pair that a buffer-affected result names. Tickets 24, 26
  and 27 set the pair and the revision rule.

Group 3: edits, parse and positions

- State which buffer revision an admitted request observes. State the rule for
  edits during a running request.
- Ticket 23 promises no incremental reparse. State the parse trigger rule that
  the host obeys. Ticket 33 owns the debounce wiring.
- IWE stalled on a per-keystroke index rebuild (v0.18.0) [V]. Use that evidence.
  Do not invent a cost figure.
- State how UTF-16 conversions stay correct across edits. Ticket 23 gives the
  contract. This ticket gives the overlay rule.

Group 4: save and close

- State the invariant that joins a save to the store. State what proves the
  store holds the saved content before the overlay entry is removed.
- State the rule for `didClose` without save. IWE drops the unsaved new document
  [V].
- State whether the host adopts, changes or refuses the IWE close rule: re-read
  the file from disk at close [V].

Group 5: deletion and rename

- State the overlay rule for a path that the generation no longer contains.
- State the query result for an overlay path whose file is gone.
- State the overlay key behavior across a rename. Ticket 32 owns the portable
  path form. Do not preset that form here.
- State how positions stay invalid across a rename or an edit (ticket 23).

Group 6: external change

- State whether the host observes external changes to an open key before close.
  IWE drops those events [V].
- State what the host reports when disk content and buffer content differ.
  Ticket 34 owns the notice text.
- The IWE MCP crate and an editor can write one vault through separate processes
  [V]. State the overlay rule for that race.

Group 7: freshness and the staleness bound

- State whether an overlay-answered query checks the sources for buffered paths.
- State what the staleness bound means for an overlay query. This ticket and
  ticket 33 set the bound (ticket 25).
- State the overlay coherence rule after a replay advances the revision (tickets
  26 and 27).
- State the overlay rule for a host that meets a higher-epoch store while
  buffers are open (ticket 25).

Group 8: the open-buffer schema check

- State which content feeds `parse_with_doc` for diagnostics and which buffer
  revisions trigger a check.
- Hosts set gating policy (ticket 20). Core carries no protocol types (ticket
  20).

Group 9: multi-process

- Record the settled constraints from the inputs above.
- State any overlay rule that those constraints force. Do not relax a store lock
  rule here.

## Scenarios the Answer must resolve

State the outcome for each scenario. Cite the rule that decides it.

- S1: unsaved edit versus CLI. A note is open with an unsaved change from
  `status: draft` to `status: done`. The host runs `--where 'status == "done"'`.
  A terminal runs the same query. State both answers and the accepted
  divergence.
- S2: save race. The editor saves a note while the host is mid-refresh. The
  refresh read the file before the save. State how the next query avoids the
  stale row. Use the `racy` window `mtime >= scan_start - 2 s` in the answer.
- S3: close without save. An unsaved new note gets `didClose`. IWE drops the
  content [V]. State the overlay removal and the store state. No store row and
  no journal may hold the dropped content (ticket 26).
- S4: external delete. Another process deletes an open note. State the overlay
  rule and the query result for that path. Use the path-set change cost as
  evidence: 18.5 ms note deserialization at 5k, 97% of the cliff
  (`research/11-performance-evidence.md`).
- S5: rename. `a.md` becomes `b.md` while the buffer is open. The watcher
  reports the change. Inbound links re-resolve (ticket 24). State the overlay
  key outcome and the position rule.
- S6: external edit to an open file. IWE drops watcher events for an open key
  [V]. A terminal process persists the external change. State the divergence
  window and what the host reports (ticket 34 owns the text).
- S7: edit during a request. A query runs with generation G pinned. The buffer
  changes during the request. State which content the request reports (ticket
  21: the admitted request keeps its generation).
- S8: replay revision. A replay after a crash advances the revision (tickets 26
  and 27). A host holds a buffer and an overlay from the pre-crash generation.
  State how the overlay attaches to the post-replay generation and what the
  result names.
- S9: two processes. A host holds an overlay. A terminal runs `traces index`.
  redb rejects a second open while a writer holds the file [V]. A host holds the
  store only during refresh and persist (ticket 25). State which process waits,
  what each reports, and that the overlay never enters the store.

## Owner boundaries

- Ticket 24 owns the generation, the facets and the write batch. This ticket
  owns only the overlay on a pinned generation.
- Ticket 23 owns `NoteDoc::scan`, `parse_with_doc` and the host-owned text
  contract. This ticket decides which revision a query observes.
- Ticket 25 owns the freshness rules, the `racy` bit and the store identity.
  This ticket and ticket 33 set the staleness bound.
- Ticket 33 owns the host composition: watcher wiring, debounce wiring, the held
  workspace and writer admission. This ticket owns the semantics that the host
  obeys.
- Ticket 32 owns the portable path form. The rename rule waits for it.
- Ticket 34 owns notice text and severities.
- Ticket 36 owns journal adoption and the durability choice.
- Ticket 29 owns the row type, the plan and membership. This ticket decides what
  content the plan reads for a buffered path.
- Ticket 20 owns the check rule and the layering. No protocol types enter core.

## Required resolution output

The Answer must name, for each open question:

- The alternatives, including retain.
- The chosen rule and its invariant owner.
- The code that the rule makes removable, or "none".
- What stays unresolved and the downstream tickets.

The Answer must also name:

- The outcome for every scenario S1 to S9, with the deciding rule.
- The generation pair and the revision rule for buffer-affected results (tickets
  24, 26 and 27).
- The staleness-bound semantics that this ticket and ticket 33 set (tickets 25
  and 27).
- The change-table rows for observable behavior (ticket 16). A host answer that
  includes unsaved content is a new surface. Record it. It is not a CLI
  regression.
- The evidence limits that carry into the Answer, with every claim that stays
  [H].

## Completion bar

- A grilling session resolves each open question with the user.
- Each open question names its rule or its owning downstream ticket.
- Every scenario S1 to S9 has a stated outcome.
- Every adopted rule names its invariant owner.
- The evidence limits survive into the Answer.
- The ticket keeps the [V], [H] and [P] marks and the exact paths and figures.
- The ticket states semantics only. It does not implement code and does not
  prescribe ticket 33 host internals.

## Answer

### Decisions recorded in the grilling session (2026-11-14 and 2026-11-15)

The maintainer confirmed every rule below after two adversarial rounds. The
final review (2026-11-16) then amended several rules. The last section lists
those amendments. The project is pre-release. The `.redb` layout is Free.
Ticket 16 freezes stdout payload shapes, exit codes and template and query
capabilities.

The evidence is in `../research/28-prep/`. The files are `g1-probes.md` (probe
measurements), `h1-stress-designs.md` (candidate designs), `h2-redteam.md`
(first red team) and `h3-redteam-round2.md` (second red team and three
audits). The prototype source and raw logs are in `/tmp/rt28-proto/`
(`proto28.rs`, `fuzz.rs`, `rt28-run1.txt`, `rt28-run2.txt`). That directory is
not in the repository.

This Answer states semantics only. It builds no code. It does not prescribe the
host internals of ticket 33.

Confirmed design rules below carry [P]. They are not current-code claims.
Measurements and current-code claims carry [V]. Unchecked limits carry [H].

Decision index:

| Section | Subject |
| --- | --- |
| Group 1 | Overlay truth and scope |
| Group 2 | Derived generation, resolution context and identity |
| Group 3 | Ordered edits, snapshot work and position selection |
| Group 4 | Save, close and the scan certificate |
| Group 5 | Deletion, rename, admission and overlay keys |
| Group 6 | External change to an open file |
| Group 7 | Freshness and the staleness bound |
| Group 8 | The open-buffer schema check |
| Group 9 | Multi-process use and the write boundary |
| S1 to S9 | Scenario outcomes |
| Later sections | Change table, evidence limits, code delta and final review |

Terms used below:

- The base generation is the pinned generation of ticket 24. It is persisted or
  held in memory.
- A derived generation is an in-memory generation. The owner builds it from the
  validated pinned base and the open buffers. The host never persists it.
- An overlay entry is a derived row for a client document. It can retain a
  saved snapshot after close until the scan certificate (Group 4).

Each group below gives four items. First, the alternatives (including retain).
Second, the chosen rule and its invariant owner. Third, the code that the rule
makes removable. Fourth, what stays unresolved.

#### Group 1: overlay truth and scope

Questions: which host reads see unsaved content, what divergence from the CLI
is accepted, and where the overlay lives.

Alternatives:

- Transparent overlay. Every query in a host process sees the buffers. This is
  the chosen rule. md-pkm-lsp chose it (issue 13) [V].
- Opt-in for each call. Only marked reads see buffers. The template default
  must then notice or fail, because ticket 21 forbids silent empty rows.
- Retain. The host reads the persisted generation only. Buffers feed
  diagnostics only.
- IWE model. One in-memory graph with no overlay [V].

Rule:

- Every query in a host process sees the content of each open buffer that
  passes admission (Group 5).
- The CLI sees saved content only, through the store or a fresh parse.
- The divergence is documented in one help paragraph. It is a new observable
  surface. It is not a CLI regression (change row 16).
- A query result in an open editor view can change while the user types. This
  is intended. The help paragraph states it.
- The semantics do not depend on where the overlay lives. An overlay is a set of
  entries keyed by client URI. It attaches to the base generation of one root.
  Ticket 33 picks the container (workspace or root).

Invariant owner: this ticket owns what a host query reads. Ticket 24 owns the
base. Ticket 29 owns the row type and the plan.

Code made removable: none. The overlay is new code.

Unresolved: the container of the overlay (ticket 33). The text of the help
paragraph (tickets 16 and 35).

#### Group 2: overlay on a pinned generation

Questions: what a request pins and which generation an entry attaches to. Also
how an entry and a base row for one path stay coherent, and which pair a
result names.

Alternatives:

- Entry keyed by path, checked at row assembly. After a refresh the entry
  re-attaches by path or drops.
- Overlay swaps the row list before memoization (the ticket 29 seam). Rows stay
  immutable.
- Wrapper generation (h1 design D3). The host merges buffer rows into an
  in-memory generation. An enum row (Generation or Overlay) is one variant.
- Retain. No overlay in the generation. Hosts read the base only.

Rule:

- The mechanism is a derived in-memory generation. The owner builds it from the
  validated pinned base. Its rows are immutable. It is never persisted.
- `QuerySet` and `GroupedQuerySet` read one pinned generation. They know
  nothing about buffers. The plan sees "any pinned generation" (the ticket 29
  seam). The enum row is not adopted.
- A request selects its generation before it pins the triple for execution.
  The pin never changes. Position selection also checks the document session
  and received version (Group 3).
- Coherence rule. An open buffer replaces the base note facts for its path.
  A row shares the base entry only when all effective row facts stay unchanged,
  including inlinks.
- Invariant: a derived generation always uses the current validated base and
  every current buffer. The owner never derives from a previous derived result.
  A buffer close removes that replacement and rebuilds from the base.
- One owner serializes every generation change: a buffer change, a base
  publish and a replay. Compare-and-swap on the current base id is one allowed
  implementation. It is not a requirement.
- A replay advances the revision. A `cp -R` fork changes the incarnation
  (ticket 32). The owner discards in-flight derived work in both cases.
  Ticket 27's macOS-suite result shows a weakened incarnation-plus-revision
  check accepting a same-revision forked journal [V]. It does not test a
  revision-only check.
- Core supplies a pure `derive(base, current_buffers)`. The host (ticket 33)
  owns the worker. The result equals a full build over the effective paths and
  notes.
- `current_buffers` contains every current overlay entry. This includes a
  closed saved snapshot that waits for proof.

Edit path. A content derive uses the targeted patch:

- Core uses a private immutable resolution context. Its key names the exact
  validated base snapshot and the effective path set.
- The effective path set is the union of the base paths and the admitted
  overlay paths, including ghosts.
- The context holds the effective resolver. It also holds the base-note edges
  resolved through that resolver.
- A content derive removes the edges of each replaced base note. It adds the
  current buffer edges through the same resolver. It includes every current
  buffer, not only the buffer from the latest event.
- The patch replaces only affected `Arc<FileEntry>` entries. It clones the
  `Vec<Arc<FileEntry>>` spine.
- The context is private. `QuerySet` still reads one ordinary pinned generation.
- A link resolution depends only on the source path, the link text and the
  path set (`resolve` in `src/index/inlinks.rs`) [V]. A content edit therefore
  changes only the edges of the edited note.

Original baseline measurements [V] (RedTeam28 prototype, bench profile with
opt-level 3 and thin LTO, medians of 9 after 2 warmups, 12 rayon threads).
Every result was asserted equal to a from-scratch build. A randomized check
of 300 iterations at 5k was equal in all iterations.

These measurements do not test the amended resolution-context lifecycle.
In the original baseline, `resolve_note` took 5.2 us for the base note and
5.4 us for the buffer note.

| Operation | 50k | 5k |
| --- | --- | --- |
| Literal full rebuild | 113.8 ms | 12.7 ms |
| Gather `&[Note]` from Arc entries, extra | +59.3 ms | +6.1 ms |
| Existing batch path: rebuild map from entries, fresh resolver, `patch_and_diff` | 13.1 ms | 1.09 ms |
| Same, resolver cached | 4.83 ms | 0.42 ms |
| Targeted patch, one note | 0.148 ms | 0.027 ms |
| Targeted patch, an edit that adds a hub link (4,787 inlinks at 50k) | 0.218 ms | 0.037 ms |
| Targeted patch, 20 buffers | 0.620 ms | 0.355 ms |

- At 50k, about 90% of the targeted cost is the spine clone (134 us).
  The spine clone takes 13 us at 5k. The hub inlink-list clone costs 69 us
  at 50k.
- Parse of a typical note, about 0.7 KB, takes 20 us. Parse of an 88 KB note
  takes 3.25 ms. Parse plus patch takes 171 us at 50k and 45 us at 5k.
- The heap retained for each derived generation, at 50k / 5k, is 400 KB / 49 KB
  (one note), 655 KB / 80 KB (hub) and 919 KB / 352 KB (20 buffers).
- The corpus has five link slots per note. Its nominal 30% generator class
  is not a measured resolution rate. Unique resolved edges total 48,927 at 50k
  and 4,861 at 5k [V per Stress28]. Dense-link workloads remain unmeasured.

Per-base cache [V for measurements, P for lifecycle rules]:

- `LinkResolver::new` takes 6.4 ms and 10.5 MB net at 50k (0.60 ms and 1.0 MB at
  5k).
- The baseline resolver lives inside its base generation object.
- The owner discards the resolution context after every base publish or replay.
  An exact immutable base identity is needed. A persisted revision alone cannot
  identify a fresh-parse fallback snapshot.
- A new or removed effective path changes the context key.
- A close removes the buffer replacement. It changes the context key only if
  the effective path set changes. A close of a base path needs no full
  re-resolve.
- A watcher rename never changes the client URI identity. A validated base
  publish that reflects the rename invalidates the context.
- A host-global resolver cache is banned. A resolver for another effective
  path set can corrupt inlinks without an error.
- An immutable resolver component can serve a new context when the path set
  stays unchanged. `RefreshPlan::is_paths_unchanged` supplies the existing
  predicate (`src/index/refresh.rs`). The old context does not survive.
- Each exact-base change can need a full context rebuild, even when an
  overlay-only path remains open. No cheap rebuild bound is established.

Path-set change in the overlay (a new unsaved document or a ghost row):

- The owner does one full re-resolve before the changed context can serve
  content derives. It runs off the request thread with latest-wins.
- The original full-build baseline is 113.8 ms at 50k (about 173 ms with the
  note gather) and 12.7 ms at 5k. The amended lifecycle is not measured.
- Queries keep the previous derived generation until the new one publishes. A
  position request follows the selection rule of Group 3.
- The full re-resolve occurs once for each new effective path set under the
  exact base. Later same-path edits reuse that context for the targeted patch.
- The stem-bucket merge (0.8 to 12 ms) and `nearest_raw` are a recorded later
  optimization.
- This rule REPLACES the line that said a path-set change uses
  `patch_and_diff`. That line was wrong. The existing refresh calls
  `patch_and_diff` only when `is_paths_unchanged` holds
  (`src/index/refresh.rs`). A path add or
  remove runs `LinkResolver::new` plus `InlinkMap::with_resolver` over all
  notes. An added file can change the wikilink resolution of other
  notes (nearest match, ties) [V].

Code preconditions [V for current code, P for requirements]:

- `WorkspaceIndex.entries` is `Box<[FileEntry]>`
  (`src/index/entry.rs`). Ticket 24 decides that rows own
  `Arc<FileEntry>`. That decision is not built.
- `FileEntry` is `{ file, note: Option<Box<Note>>, inlinks: Box<[PathBuf]> }`
  (`src/index/entry.rs`). Replacing an entry whose inlinks changed
  deep-clones the `Note`.
- Requirement [P]: `FileEntry` shares `Note` and `inlinks` by `Arc`. The
  `Arc<Note>` variant is not measured. The 0.148 ms figure comes from
  `/tmp/rt28-proto/`, not from the repository.
- `LinkResolver<'a>` borrows `&'a [FileMeta]` (`src/index/inlinks.rs`).
  `InlinkMap::with_resolver` takes `&[Note]`.
- `WorkspaceIndex` owns no `InlinkMap` and no resolver. `assemble_internal`
  drops the map (`src/index/entry.rs`).
- `get_by_path` is a `Path::cmp` binary search (`src/index/inlinks.rs`).
  The resolver must become owned before it can live in a generation object.
  That is a redesign [P].
- `InlinkMap::patch_and_diff` is crate-internal (`src/index/inlinks.rs`).
  It scans every target with `retain`.
- The shipped profile is `opt-level = "z"` (`Cargo.toml`). Measure
  again [P].

Invariant owner: the owner of generation changes (ticket 33 wires it) enforces
"built from the current base". Ticket 24 owns the base and the `Arc<FileEntry>`
rows. Ticket 29 owns the seam.

Code made removable: none.

Unresolved: the owned resolver redesign and the `Arc<Note>` sharing (ticket
24). The amended context-lifecycle measurement. The measure under opt-level
"z". The heavy-note entry clone. Ticket 24 relabels "trie build 224 ms at
5,000 same-stem candidates" (the trie build alone takes 2.0 to 2.3 ms).

#### Group 3: edits, parse and positions

Questions: which buffer revision an admitted request sees, which parse trigger
the host obeys, what the IWE stall means, and how UTF-16 conversions stay
correct.

Alternatives:

- Parse and publish on a debounce, never on a keystroke (first-round
  proposal).
- Debounce, then do a full derive, with no targeted patch (AuditSimplicity item
  23). This costs 113.8 ms at 50k for each derive.
- Apply every edit in order and publish bounded snapshot work, with no timer.
  This is the chosen rule.
- Retain. No parse before save. Buffers feed diagnostics only.

Rule:

- Apply every text-change notification in arrival order. Apply changes within
  each notification in their given order. Never discard an incremental edit.
- Keep the current text separate from derived parse work.
- The worker coalesces complete versioned buffer snapshots, not individual
  notifications. Unchanged text and entries stay shared.
- Keep at most one running job and one latest pending snapshot. Use no
  parse timer. Debounce only schema checks and diagnostics.
- Each `didOpen` starts a new document session. Each job records the exact base,
  overlay-state token, document sessions and versions.
- A change, close, reopen, admission change or base publish advances that token.
  Reject obsolete results before publication. Check diagnostics the same way.
- A position request captures its document session and received version.
  Select a generation containing that exact parsed version before pinning it.
- Selection can wait for relevant work or return `Stale`. The host chooses
  the wait budget. A newer version never silently replaces the requested version.
- Once execution starts, the request never changes its generation pin.
- `Stale` is a core outcome. The host maps it to its protocol code.
  `ContentModified` is an LSP term and does not belong in core.
- A request NEVER answers with positions from an older version.
- The host owns the text, the version, the revision gate and the UTF-16
  conversion (ticket 23). A stored position never drives an edit.
- Ticket 23 promises no incremental reparse. Its prototype scan took about 29%
  of parse time [V]. The real `NoteDoc` builder cost is unmeasured.
- Precedent [V]. rust-analyzer applies changes in batches before the next
  snapshot (`.scratch/md-pkm-lsp/research/14-buffer-overlay-model.md`).
  Its non-retrying requests return `ContentModified` after cancellation
  (`.scratch/md-pkm-lsp/research/x-rust-lsp-implementation-patterns.md`).
  markdown-oxide runs `Vault::update_vault` on each `didChange` under the write
  lock. It debounces only diagnostics, at 300 ms
  (`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt`).
  IWE writes the buffer into its graph at once. It batches only the BM25
  rebuild after 300 ms (v0.18.0).
- Original baseline: parse plus targeted patch takes 171 us at 50k and 45 us
  at 5k. It does not measure the amended context lifecycle. IWE stalled on a
  heavy per-keystroke index rebuild. Traces has no such rebuild.

Invariant owner: ticket 23 owns the position contract. This ticket owns which
version a request observes. Ticket 33 owns the worker wiring.

Code made removable: none.

Unresolved: the LSP re-push rule, that is, whether the client gets a result
again after a publish (ticket 33). The `ContentModified` mapping (hosts).

#### Group 4: save and close

Questions: which invariant joins a save to the store, and what proves that the
store holds the saved content. Also what `didClose` without save does, and
whether the host adopts the IWE close rule.

Alternatives:

- Earlier rule Q9. The entry stays until a refresh publishes a row that matches
  the saved file (stat triple).
- Earlier rule Q16. An edited existing file drops at close.
- Drop the entry at save, then let the racy rescan read the file.
- A close drops the entry. A save marks it clean (AuditSimplicity item 25).
- IWE. Re-read the file from disk at close for every file [V].
- Chosen rule: the scan-epoch proof below.

Rule:

- An open buffer keeps its entry across a save.
- Clean means that the text equals a known saved text snapshot. `didOpen`
  alone proves no disk equality. Treat unknown cleanliness as dirty.
- At save receipt, capture the ordered saved text before later edits replace it.
  Record its observation time on the host's monotonic clock.
- At close, retain the known clean snapshot or the last saved snapshot only
  while its proof is pending. Otherwise remove the replacement.
- A new unsaved document drops at close. No store row or journal fact exists.
- Every retained closed entry has a `proof_time`. For a save, this is its
  receipt time. Without a save, use the known disk snapshot's observation time.
- An eligible scan starts at `scan_start >= proof_time + 2 s`.
  Record scan start before reading sources, on the same monotonic clock.
- Release the entry only after validation succeeds and the owner installs the
  coherent replacement base. Install the base and release the entry atomically.
- Failed, incomplete or discarded scans release nothing. A no-change scan can
  prove the entry without publishing a new persisted revision.
- Failed persistence does not prevent proof when the validated in-memory base
  is installed. Schedule an eligible validation for every pending proof.
- The row need not equal the snapshot. Disk wins after the buffer closes.
- Remove the early `Note`-equality shortcut. Equal parsed facts do not prove
  equal byte size, text or parse notices.
- A validated deletion supersedes older saved snapshots. Its scan starts after
  the relevant save observation. Never restore that old snapshot at close.
- A later save creates a new proof obligation. An older deletion cannot clear it.
- The 2 s constant is the racy margin of ticket 25
  (`mtime >= scan_start - 2 s`). It is borderline on FAT. Clock skew on network
  volumes defeats it [H].
- Divergence from IWE. IWE re-reads the file from disk at close for every file
  [V]. Traces drops a new unsaved document, as IWE does. Traces keeps a known
  clean or saved snapshot until the scan certificate.
- This rule REPLACES the earlier rules Q9 and Q16. Those rules let a
  saved-then-closed file flip back to the stale persisted row.

Invariant owner: this ticket owns the rule. Ticket 25 owns the racy bit, the
margin and the scan epoch.

Code made removable: none. The earlier rules were never built.

Unresolved: ticket 33 wires the atomic validation-and-install event. Ticket 25
owns its acceptance test. Whether a no-change refresh clears a racy bit with a
commit stays [H] in ticket 25; proof does not need that commit.

#### Group 5: deletion, rename and overlay keys

Questions: what the overlay does for a path that the generation no longer
contains. What a query returns for an overlay path whose file is gone. What
key survives a rename. How positions stay invalid.

Alternatives:

- Re-key the entry on a watcher rename (first-round rule Q23, design D1 and
  the first-round S5). The client owns the document identity, so this is
  unsound under LSP [H].
- Drop the entry when its path leaves the generation (first-round S4). The
  chosen rule keeps this behavior for a clean buffer only.
- Retain. The row drops in all cases, as in the CLI view.
- IWE. Drop watcher events for an open key until `didClose` [V].

Rule for identity and admission:

- The overlay identity is the client document URI.
- At `didOpen`, `ProjectRoot` maps the URI ONCE to a `RelativePath`. It looks up
  the listed spelling in the base.
- These documents get diagnostics only and NO query overlay:
  - a document outside the root;
  - a document with a non-UTF-8 name;
  - an existing file that fails scanner node-kind or symlink checks
    (`src/index/service.rs`, test `skips_symlinks_entirely`);
  - a path whose lexical admission excludes an ignored or metadata component;
  - a non-`.md` document (`FileFormat::Other`);
  - an `untitled:` URI;
  - a document that Windows rejects.
- The reason: an entry for such a document would show in queries and no refresh
  would ever prove it. That is a leak.
- An unsaved new file uses its URI spelling until the scan proof drops it.
- A rename never re-maps the entry until close. After an external
  `mv a.md b.md`, the client still sends `didChange` for `a.md`.
- The Comparison Key is never applied to a URI or to an overlay key
  (`src/GLOSSARY.md`).
- Admission has lexical and existing-node checks. A nonexistent final component
  can enter as a new buffer when its parent passes admission.
- Recheck admission after policy or validated node-kind changes. An excluded
  buffer remains available for diagnostics but leaves the query overlay.
- Give each admitted `RelativePath` one active buffer owner. A second URI
  receives diagnostics and a notice, not another replacement row.
- A watcher event requests validation. It does not prove deletion or re-key
  the document.

Rule for deletion:

- A validated external deletion keeps a dirty or unknown buffer as a ghost
  until close. Give one notice: "file deleted on disk; showing unsaved buffer".
  Ticket 34 owns the final text.
- A known clean buffer drops only with atomic installation of the validated
  absent base row. Raw watcher events never remove it.
- A ghost joins link resolution as an added path. Links that fell to another
  same-stem candidate flip back to the ghost. The CLI shows the opposite. The
  help paragraph documents this.
- A ghost never persists.
- The cost for the derived generation is one full re-resolve (Group 2).
- The saved-index added-file baseline is 499 ms and 394.9 MB at 50,000 notes
  (`research/24-prep/g-probe-results.md`) [V].
- At 5k, note decoding takes 18.48 ms, or 97.3% of measured in-memory
  reconciliation work (`research/11-performance-evidence.md`) [V].
  This denominator excludes scanning and persistence. Neither figure measures
  deletion, ghost removal or the amended retained-generation implementation.
- Today's reconciliation reads stored notes on path-set changes
  (`src/index/refresh.rs`) [V].
- The derived generation does not pay the cliff. It builds from in-memory
  entries that already hold notes. The cliff would apply if host entries were
  lazy facets [H]. The stem bucket needs every note's outlinks.
  See ticket 24, "Inputs from the note document model decision".

Rule for rename:

- A watcher rename affects the disk only.
- A rename of an open file shows a duplicate row or a ghost row until close,
  with one notice.
- Inbound links re-resolve on the base.
- Positions stay invalid after a rename or an edit (ticket 23). A stored
  position never drives an edit.

Invariant owner: this ticket owns the overlay rules. Ticket 32 owns the
`RelativePath` form and the `ProjectRoot` mapping function. Ticket 34 owns the
notice text.

Code made removable: none.

Unresolved: the URI mapping function for a host (tickets 32 and 33). The lint
code for Windows device names (ticket 34). The deletion and ghost removal are
not timed [H].

#### Group 6: external change to an open file

Questions: whether the host sees external changes before close. What it reports
when disk and buffer differ. What the rule is for a second writer.

Alternatives:

- IWE rule. Drop the events for an open key. The editor owns the truth until
  close [V].
- Adopt the disk content into the buffer. IWE's v0.16.0 changelog records the
  earlier last-write-wins behavior. Disk changes overwrote unsaved buffers.
  That release changed the rule to editor ownership [V].
- Observe and warn. This is the chosen rule.

Rule:

- The buffer stays the truth. The host never adopts disk content into the
  buffer.
- The host observes the divergence and warns.
- The host emits one notice for each (document, divergence kind). It keeps
  that notice until the state changes or the buffer closes.
  The document identity is the client URI. Ticket 34 owns the text.
- Closed-buffer divergence ends at Group 4's atomic proof transition, not at
  an arbitrary next scan. An unknown unsaved replacement drops under its close rule.
- A second writer is possible. The IWE MCP crate can write the vault through
  another process [V]. The overlay never enters the store. The store stays
  consistent through the lock rules of tickets 24 and 25.

Invariant owner: this ticket owns the rule. Ticket 34 owns the notice text and
severity. Ticket 33 owns the watcher wiring. A watcher only signals the need
for revalidation (ticket 22).

Code made removable: none.

Unresolved: the notice text and the dedup mechanism (ticket 34). The
rust-analyzer behavior for this case is [H] and is not verified.

#### Group 7: freshness and the staleness bound

Questions: whether an overlay-answered query checks the sources. What the bound
means. How the overlay stays coherent after a replay. What happens with a
higher-epoch store.

Alternatives:

- Check the sources of buffered paths inside the bound. This adds no
  information for an open buffer.
- IWE behavior. Drop external events for an open key [V]. This is the opposite
  choice.
- Retain. The bound applies to every path in the same way.
- Chosen rule: the rules below.

Rule:

- A path with an open buffer is its own truth. No disk check adds information.
- The staleness bound (ticket 25) applies only to paths without a buffer.
- The racy rescan still runs at the next refresh.
- The numeric bound stays with ticket 33. This ticket states the overlay rules
  for a parameter B and leaves the number to ticket 33. The handoff goes one
  way.
- After a replay or any new base generation, the owner rebuilds the derived
  generation from the new base. It re-attaches the buffers by URI.
- A host that meets a higher-epoch store never writes it. It serves from its
  in-memory generation plus the buffers. It parses fresh when it has none.
  Results name the new triple.

Invariant owner: ticket 25 owns the freshness rules and the racy bit. This
ticket owns the overlay rules. Ticket 33 owns the number B.

Code made removable: none.

Unresolved: the number B (ticket 33, with the figures of ticket 27).
Ticket 25 now names tickets 33 and 28 under "Confirmed decisions" [V].
Its higher-epoch rule already permits an in-memory host generation [V].

#### Group 8: the open-buffer schema check

Questions: which content feeds `parse_with_doc` for diagnostics, and which
buffer revisions trigger a check.

Alternatives:

- Run the check on each keystroke.
- Run the check on a save only.
- Check the debounced parse output of the buffer. This is the chosen rule.
- Retain. No open-buffer check.

Rule:

- `check_note(before, after)` runs on the DEBOUNCED parse output of the buffer.
  It never runs on each keystroke.
- The host maps the violations to protocol diagnostics. Core carries no
  protocol types.
- Diagnostics are feedback only. `didChange` cannot deny an edit.
  `willSaveWaitUntil` cannot abort a save (ticket 20).
- Each host sets its gating policy over the stable violation codes (ticket 20).

Invariant owner: ticket 20 owns the check rule and the layering. This ticket
owns the placement.

Code made removable: none.

Unresolved: the debounce wiring and the interval (ticket 33). The IWE and
markdown-oxide precedent is 300 ms [V]. This Answer does not set a figure.

#### Group 9: multi-process

Questions: which constraints are settled, and which overlay rules those
constraints force.

Settled constraints [V for measured redb behavior, P for design rules]:

- redb rejects a second writer at open. The facts hold for redb 4.3.0. The
  experimental multiprocess mode refuses `Durability::None`.
- A host holds the store only during a refresh and a persist (ticket 25).
- A CLI process in another terminal can query when it can parse the sources
  (ticket 24).
- Never write a fact from an unsaved buffer (ticket 26).
- A pure read-only operation never opens the journal (ticket 26).
- Ticket 36 owns the journal adoption and the `Durability::None` against
  `Durability::Immediate` choice. In the tested stack, `Durability::None` rows
  vanished at an `ENOSPC` boundary (ticket 27).

Alternatives:

- A cross-process overlay. No design exists for it [V, first red team].
- Relax a store lock rule. This ticket cannot do it.
- Retain. Each process keeps its own view.

Rule:

- The terminal waits or parses fresh. The host holds the store only during a
  refresh and a persist.
- A query generation cannot create a persisted batch. Only validated disk facts
  can create the batch accepted by the store or journal.
- Derived generations and closed pending-proof snapshots have no write
  capability. Their read representation can still match the base representation.
- No lock rule is relaxed.
- A fork (`cp -R`) changes the incarnation. Derived generations never carry
  over.
- Two hosts on one vault keep independent overlays. They never see each other.
  The help paragraph documents this.

Invariant owner: tickets 24 and 25 own the lock rules. Ticket 26 owns the
journal rule. This ticket owns the overlay rule.

Code made removable: none.

Unresolved: two machines on one file system (a network mount). No overlay, no
lock and no racy margin can make the bound hold. That case is out of scope for
the single-machine tier of ticket 16. Ticket 36 owns the adoption blockers.

### Scenario outcomes S1 to S9

| Scenario | Outcome | Deciding rule |
| --- | --- | --- |
| S1 Unsaved edit against the CLI | The host answers `status == "done"` from the derived generation: the open note matches. The CLI answers `draft` from the store: the note does not match. The divergence is accepted. It is a new surface (change row 16). | Group 1 (transparent overlay) and Group 2 (derived generation) |
| S2 Save race | Keep the saved snapshot until an eligible scan validates sources and installs the coherent base atomically with entry removal. Use `scan_start >= proof_time + 2 s`; a save sets `proof_time` to its receipt time. An open buffer keeps its entry. Failed scans release nothing. A no-change scan can prove it without a new persisted revision. | Group 4 and ticket 25 |
| S3 Close without save | An unsaved new note closes. The entry drops. The owner rebuilds the derived generation from the base. No store row exists. No journal fact exists. IWE also drops the content [V]. | Group 4 and Group 9 (the overlay never enters the store or the journal) |
| S4 External delete | A validated deletion keeps a dirty or unknown buffer as a ghost until close. A known clean buffer drops with installation of the absent base row. Never restore a saved snapshot superseded by that deletion. Rebuild the effective-path context as needed. The original full-build baseline is 113.8 ms at 50k; the amended lifecycle is unmeasured. The 499 ms / 394.9 MB saved-index baseline measured an added file, not deletion. | Groups 2, 4 and 5 |
| S5 Rename | The overlay stays keyed by the client URI. The watcher rename affects the disk only. A duplicate row or a ghost row shows until close, with one notice. Inbound links re-resolve on the base. Positions stay invalid. | Group 5 (rename) and ticket 23 (positions) |
| S6 External edit of an open file | The buffer stays truth. Give one notice for each document and divergence kind. After close, pending saved snapshots end at the atomic proof transition. Discard unknown unsaved replacements under Group 4. Ticket 34 owns notice text. | Groups 4 and 6 |
| S7 Edit during a request | Select the exact document session and received version before pinning a position request. Wait within host policy or return `Stale`. Never repin during execution or silently use another version. Non-position requests can pin the current published generation immediately. | Groups 2 and 3 |
| S8 Replay revision | The replay advances the revision. The owner discards in-flight derived work. It rebuilds the derived generation from the new base and re-attaches the buffers by URI. The result names the new triple. | Group 2 (one owner), Group 7 and the generation pair |
| S9 Two processes | redb rejects a second writer at open. The host holds the store only during a refresh and a persist. The terminal waits or parses fresh. Each side reports its own result. The overlay never enters the store. Two hosts keep independent overlays. | Group 9 |

### The generation pair and the revision rule

- The base pair is (incarnation, revision) (ticket 25). Every commit advances
  the revision (ticket 24). A replay advances the revision (ticket 27). A
  `cp -R` fork changes the incarnation (ticket 32).
- A result that reads a derived generation names three values: the base
  incarnation, the base revision and `derived_id`. A result without an overlay
  has `derived_id` 0. It names the base pair, as before.
- A buffer edit changes `derived_id` only. It never changes the base revision.
- `derived_id` is a process-wide monotonic `u64`. The value 0 means "no
  overlay". The value is never reused. A close of all buffers does not reset
  it. A new base does not reset it.
- The LSP version is not part of the pair. It restarts at `didOpen`. A digest
  of (path, version) is not unique.
- The triple identifies derived state within one host lifetime only. External
  cross-process consumers also need a host-session identity.
- A memo belongs to its immutable query family. It retains the generation
  object while using its identity. A raw address alone proves no identity.
- Keep no process-global generation history. Retain old contexts only while
  active queries own them. Ticket 33 bounds queued work and concurrent requests.
- A revision-only identity is not enough [H]. Ticket 27's macOS-suite result
  defeats a weakened incarnation-plus-revision check with a same-revision
  forked journal [V]. That probe does not isolate a revision-only check.
- After a replay or a new base, the owner builds a new derived generation. The
  results name the new triple.

### Staleness-bound semantics

Ticket 25 sets the bound for paths without a buffer. This ticket states the
overlay rules for the parameter B. Ticket 33 sets the number.

- B governs disk paths without buffers. Open buffer text is its own truth.
  Disk metadata, admission and divergence still need validation.
- The racy rescan still runs at the next refresh. A refresh can find that the
  disk file differs from the buffer. Group 6 then applies.
- A retained closed entry needs an eligible scan with
  `scan_start >= proof_time + 2 s` and the atomic install transition of Group 4.
  The host schedules that validation. A failed scan does not end the wait.
- After a replay or any new base generation, the owner rebuilds the derived
  generation from the new base. It re-attaches the buffers by URI.
- A host that meets a higher-epoch store never writes it. It serves from its
  in-memory generation plus the buffers, or it parses fresh when it has none.
- Ticket 27 proposes a numeric bound. The next command produces a queryable
  store within one completed publish cycle of the same host. Recovery plus the
  first query takes 361 to 450 ms at 5k against a 770 ms cycle. At 50k it takes
  6.2 to 6.3 s against a 10.8 s cycle. These are VM figures. Ticket 33 measures
  them again on real hosts.
- The overlay adds no rule that relaxes the bound for a path without a buffer.

### Overlay row contents

- `file.size` is the buffer bytes.
- `mtime` and `created` stay from the base row. A new file has none.
- The parse notices of the buffer replace the persisted notices of the base for
  that path in the derived entry.
- A template run inside the host sees the overlay (earlier rule Q8). It can
  write output that derives from unsaved text. The index sees that output only
  through its saved file.
- Two hosts on one vault keep independent overlays (S9).

### Adversarial review and required corrections

Two red-team rounds and three audits tested the first-round recommendation. The
details are in `../research/28-prep/h2-redteam.md` and
`../research/28-prep/h3-redteam-round2.md`. The list below records each
reversal.

1. Earlier rule Q9 (the entry stays until a refresh publishes a matching row)
   became the scan-epoch proof with the 2 s margin. The old rule had three
   holes. A terminal edit leaks the entry. A racy row matches with stale
   content. A refresh with no change publishes nothing.
2. Earlier rule Q16 (an edited existing file drops at close) became: a known
   clean snapshot or the last saved snapshot stays until its proof, and a
   dirty or unknown buffer otherwise drops. The old rule flipped a
   saved-then-closed file back to the stale row.
3. The first-round proposal to drop the entry at save became: open buffers keep
   their entry across a save.
4. Earlier rule Q22 (`overlay_seq`) became `derived_id`. The new counter is
   process-wide and monotonic. The old counter could repeat and collide memo
   keys.
5. Earlier rule Q23 (re-key the entry on a rename) became URI identity. The
   client keeps sending `didChange` for the old name.
6. The first-round proposal to parse on a debounce became ordered edits with
   bounded snapshot work and no parse timer. Only the schema check and the
   diagnostics use a debounce.
7. The first-round rebuild of the whole generation for each edit became the
   targeted patch. The literal full rebuild costs 113.8 ms at 50k, plus 59.3 ms
   for the gather. That is about 173 ms. The targeted patch costs 0.148 ms.
8. The line "a path-set change uses `patch_and_diff`" became one full
   re-resolve. The existing refresh calls `patch_and_diff` only for an
   unchanged path set (`src/index/refresh.rs`). The 13 ms figure
   belongs to that other path.
9. `ContentModified` became the core outcome `Stale`. The host maps it.
10. The first-round S4 outcome became a ghost for dirty or unknown text after
    validated deletion. Known clean text drops with the absent base installation.
11. The rule "publish only by compare-and-swap" became: one owner serializes
    every generation change. Compare-and-swap is one allowed implementation.
12. The first design lacks an admission rule and a URI conversion. The overlay
    now applies the scanner predicate. `ProjectRoot` maps the URI once at
    `didOpen`.
13. The enum row of the wrapper design is not adopted. The derived generation
    holds normal `Arc<FileEntry>` rows.
14. The rule "one notice" lacks a key. The key is now (document or path,
    divergence kind) until the state changes or the buffer closes. For a
    buffer, the document identity is the client URI.
15. The base-only targeted patch became `derive(base, current_buffers)` with
    a private immutable resolution context. The context uses the exact base
    snapshot and effective path set. A prior derived result never becomes the
    base. The original timings do not prove this amended lifecycle.
16. The scan proof became a certificate. A scan validates sources and installs
    the coherent base atomically with entry removal. Every retained closed
    entry has a proof time. Failed scans release nothing. The `Note`-equality
    shortcut is removed.
17. The request pin became selection first, then one fixed pin. A request never
    repins to a newer version.
18. A clean flag became a known saved text snapshot. Unknown cleanliness is
    dirty. A validated deletion supersedes older saved snapshots.
19. Admission became a lexical check plus an existing-node check, so that a new
    unsaved file can enter. Each path has one active buffer owner.

Other corrections:

- The first-round probe measured an overlay check at query time. The final
  design has no per-row check, so those figures are context only (see the
  evidence limits).
- Ticket 24 says "trie build 224 ms at 5,000 same-stem candidates".
  The trie build alone takes 2.0 to 2.3 ms. The 224 ms figure is probably the
  whole `InlinkMap::new` [H]. Ticket 24 relabels the number. The trie decision
  stays.

Audit proposals that the grilling did not adopt:

- AuditSimplicity item 23: debounce, then do a full derive. The original
  baseline costs about 173 ms at 50k with the gather for each publish [V].
  Its targeted patch costs 0.148 ms [V]. Path-set changes still need a full
  re-resolve. An exact-base change can also need a full context rebuild.
- AuditSimplicity item 25: close drops the entry and save marks it clean. This
  reopens the flip back to the stale base row after a save and a close.
- AuditSimplicity item 24: drop `derived_id` and compare-and-swap. The Answer
  keeps `derived_id`. It keeps one owner and lets the owner choose how to hold
  the invariant.

Audit findings that the Answer accepted:

- The 2 s constant needs its source: the racy margin of ticket 25.
- Document that a query result in an open view can change while the user
  types.
- Define the notice dedup key.
- Name the core outcome `Stale`, not `ContentModified`.
- State the full re-resolve for a path-set change.
- State the code preconditions (the `Arc<FileEntry>` spine, shared `Note` and
  `inlinks`, an owned resolver).

### Resolution and handoffs

The Answer above settles the semantics. The handoffs below keep the open work
explicit.

Alternatives considered: opt-in for each call and retain (the host reads the
persisted generation only). Also an entry keyed by path at row assembly, the
wrapper generation with an enum row, a debounced full derive and "close drops
the entry". Also the IWE model (one graph, no overlay). See the groups above.

Invariant owners:

- This ticket: what a host query reads, the coherence rule for an overlay entry
  and a base row, the save, close and proof rule, the deletion, rename and
  external-change rules, the generation pair and the overlay rules for B.
- Ticket 24: the base generation, the facets, the write batch and the
  `Arc<FileEntry>` rows.
- Ticket 25: store identity, the racy bit, the scan epoch and the bound for
  paths without a buffer.
- Ticket 29: the row type, the plan and membership. The seam is "any pinned
  generation".
- Ticket 32: the `RelativePath` form and the `ProjectRoot` mapping.
- Ticket 33: the worker wiring, the watcher wiring, the debounce wiring and the
  number B.
- Ticket 20: the check rule and the layering.
- Ticket 23: the position contract.

Duties for other tickets:

- Ticket 24: relabel "trie build 224 ms" (the trie build alone takes 2.0 to
  2.3 ms). Record the spine clone cost (134 us at 50k). `FileEntry` shares
  `Note` and `inlinks` by `Arc`. The resolver is owned.
  A private resolution context names the exact validated base snapshot and
  effective path set. It holds base-note edges under the effective resolver.
- Ticket 25: add the scan-epoch proof and a test for it. The newer-epoch rule
  and the downstream-ticket reference already agree with this Answer [V].
- Ticket 26 and ticket 27: the journal never holds a buffer fact.
- Ticket 29: the seam stays "any pinned generation". A memo owns its
  generation. The triple scopes to one host lifetime.
- Ticket 32: `ProjectRoot` maps a URI once to a `RelativePath`. The Comparison
  Key never applies to an overlay key.
- Ticket 33: one owner and a bounded snapshot worker with session and version
  fences. The LSP re-push rule. The number B. The ban on a host-global resolver
  cache. The container of the overlay. The wiring of the atomic proof event.
- Ticket 34: the notices of this ticket (ghost row, divergence, rename
  duplicate) with the dedup key (document or path, divergence kind). The notice
  text "file deleted on disk; showing unsaved buffer".
- Ticket 35: the bench home for the overlay benchmarks. The visibility of the
  derive function.
- Ticket 36: adoption of the journal. The overlay adds no journal fact.
- Blocked-by repairs are in place [V]. Ticket 31 waits for 7, 16, 18, 28 and 29.
  Ticket 34 waits for 8, 12, 13, 20, 28, 29 and 31. Ticket 35 waits for 8, 12,
  16, 22, 33 and 34. Ticket 33 includes 28 and 32. Ticket 34 does not list
  itself.
- Map: add a "Decisions so far" line for ticket 28.

Fog: none new. The unmeasured items are in the evidence limits.

### Change table (ticket 16)

| # | Change | Category | Note |
| --- | --- | --- | --- |
| 15 | Bracket-mismatch advisory notice | New surface | stderr only. Ticket 29 owns the row. The notice dedup key of this ticket applies to it. |
| 16 | Host queries see unsaved buffers. The CLI does not. `file.size` is the buffer bytes in a host. | New surface | Documented divergence. A host result can include a ghost row and links that flip back to the ghost. It is not a CLI regression. |
| 17 | The quoted and unquoted divergence of ADR-0010 is fixed by ticket 19 | Migratable | Ticket 19 owns the row. The overlay adds no second rule. A host and the CLI use the same rule. |

Row 16 is the only row that ticket 28 creates. Rows 15 and 17 are listed
because the consolidated list assigns rows 15 to 17 to ticket 28. Their owners
stay tickets 29 and 19.

### Evidence limits carried into the Answer

- Traces has no host code. `.scratch/md-pkm-lsp/` states intent only [V].
- The IWE ownership rule covers one process, one root and no cancellation [V].
  It is evidence for the in-process view only [H].
- The v0.16.0 and v0.18.0 fixes of IWE are changelog facts [V]. They are
  not measurements of a persisted-index design.
- The redb facts hold for redb 4.3.0. The experimental multiprocess mode refuses
  `Durability::None` [V].
- The replay and recovery figures are Linux VM runs. They are not native
  latency and not power-loss proof [V].
- The derived-generation figures come from a prototype in a throwaway worktree.
  The worktree is removed. The source is in `/tmp/rt28-proto/`.
  The synthetic corpus has five link slots per note, not a measured 30%
  resolution rate. The build used opt-level 3 and `Box<Note>` entries only [V].
- Not measured: the amended resolution-context lifecycle, the heavy-note entry
  clone and the `Arc<Note>` variant.
- Also unmeasured: the single-thread full rebuild (`RAYON_NUM_THREADS=1`),
  deletion and ghost removal, equal-distance ties in the fuzz, and the
  flat-to-trie crossing (63 to 64 candidates).
- The repository does not hold the source of the 0.148 ms figure. Keep every
  claim that carries [H].
- The shipped profile is opt-level "z" (`Cargo.toml`). One audit
  measured that "z" slows iterator-heavy comparators by about 2x. The overlay
  figures are not measured again under "z" [V].
- The 50,000-note figures of ticket 23 are linear extrapolations [H].
- The first-round probe (`../research/28-prep/g1-probes.md`) measured an
  overlay check at query time with redb 4.5.0. The `Arc` handout took less than
  0.1 us. A naive miss-check over a 50k-row result took 0.69 ms. The candidate
  step with 10 overlay paths added about 64 us. The final design has no
  per-row check. These figures are context only [V].
- An audit judged that a 3x error in the overlay figures still leaves 0.5 ms for
  one buffer and 2 ms for 20 buffers [H].
- markdown-oxide is the nearest LSP precedent. It has an in-memory whole-vault
  lock, no persisted cache and a 300 ms debounce. It shows the lock model, not
  the persisted-generation model [V]
  (`research/23-prep/h1-precedent-rumdl-iwe.md`,
  `research/24-prep/c1-digests-lsp-zk.md`, findings 12, 13 and 20).
- A change counter does not make a snapshot atomic
  (`research/24-prep/h1-stress-test-designs.md`) [V]. Dataview `revision` and
  markdown-oxide `diag_gen` are examples.
- rust-analyzer and Tantivy give each request an immutable snapshot
  (`research/24-prep/z-grilling-guide.md`). A generation clone costs 120 us at
  50,000 entries (design G2 in `research/24-prep/h1-stress-test-designs.md`)
  [V].
- The template engine refreshes for each render and holds a `WorkspaceIndex` in
  its state (`src/template/engine/query.rs`) [V]. That is current-code
  evidence, not a decided host design.
- Two machines on one file system (a network mount) are out of scope. No
  overlay, lock or racy margin can make the bound hold there [H].

### Code delta

The code does not implement this ticket. The table lists what the refactor
changes. The Today column is [V]. The Target column is [P].
Locations are the working tree on 2026-11-15.

| Area | Today (file and symbol) | Target (ticket decision) | Depends on |
| --- | --- | --- | --- |
| Generation rows | `WorkspaceIndex.entries: Box<[FileEntry]>` in `src/index/entry.rs` | A spine of `Arc<FileEntry>`. A derived generation clones the spine and replaces only affected entries. | Ticket 24, ticket 29 |
| Entry sharing | `FileEntry { file, note: Option<Box<Note>>, inlinks: Box<[PathBuf]> }` in `src/index/entry.rs` | Share `Note` and `inlinks` by `Arc`. The `Arc<Note>` variant is not measured. | Ticket 24 |
| Resolver ownership | `LinkResolver<'a>` borrows `&'a [FileMeta]` in `src/index/inlinks.rs` | An owned baseline resolver and a private immutable resolution context for the exact base snapshot and effective path set. | Ticket 24, ticket 32 (key form), ticket 35 |
| Map at assembly | `assemble_internal` moves the inlinks into entries and drops the `InlinkMap` in `src/index/entry.rs` | The private context retains base-note edges under its effective resolver. | Ticket 24 |
| Link patch | `patch_and_diff` is crate-internal in `src/index/inlinks.rs`. It scans every target with `retain`. `resolve_edges_for`. | A pure `derive(base, current_buffers)` in core. It replaces base-note edges with all current buffer edges under one effective resolver. | Ticket 24, ticket 29 |
| Path-set change | The `else` branch runs `LinkResolver::new` plus `InlinkMap::with_resolver` over all notes in `src/index/refresh.rs` | One full re-resolve per exact-base/effective-path context, off the request thread. A base publish can repeat this cost. | Group 2 |
| Resolver reuse | `is_paths_unchanged` in `src/index/refresh.rs` | Reuse an immutable resolver component when paths stay unchanged. A new base still invalidates the private context. | Ticket 25 |
| Admission predicate | `scan` in `src/index/service.rs` and `is_ignored_dir` in `src/env_vars.rs` | Shared lexical admission plus existing-node checks. A new final component needs an admitted parent. | Ticket 32, ticket 33 |
| Store lifecycle | `IndexStore::open` in `src/index/refresh.rs` and `src/index/service.rs` | The host opens the store only for a refresh and a persist. The overlay reads memory. | Ticket 24, ticket 25, ticket 33 |
| Generation pair | None. `src/` has no `incarnation` and no `revision` (grep finds no match). | The base pair in `meta` plus a process-wide `derived_id`. Results name the triple. | Ticket 24, ticket 25 |
| Buffer parse | None. `src/` has no `parse_with_doc` and no `NoteDoc`. | A public `parse_with_doc` for an unsaved buffer (ticket 23). | Ticket 23 |
| Schema check | None. `src/` has no `check_note`. | The host calls `check_note(before, after)` on the debounced parse. | Ticket 20, ticket 33 |
| Template index cache | The per-render `WorkspaceIndex` cache in `src/template/engine/cache.rs` and `src/template/engine/query.rs` | A template run inside a host reads the pinned derived generation. The cache must not outlive its generation. | Ticket 31, ticket 33 |
| Overlay row facts | `FileMeta` in `src/file.rs`, `size` | The derived entry has `file.size` as buffer bytes. `mtime` and `created` come from the base. | Ticket 29 |
| Overlay notices | No notice emission outside the stderr count lines (grep finds no notice prefix in `src/`) | Overlay notices ride the ticket 34 mechanism with the dedup key. | Ticket 34 |
| Release profile | `[profile.release]` is `opt-level = "z"` in `Cargo.toml` | Measure the overlay and sort costs again under "z". | Ticket 32 (row 19) |

Not in code:

- The derived generation, the pure `derive(base, current_buffers)` and the
  owner. The private resolution context and its lifecycle.
- The URI mapping in `ProjectRoot` and the shared admission predicate.
- The `derived_id` counter and the generation pair.
- The scan certificate: proof time, atomic validation-and-install, and the 2 s
  margin.
- The `Stale` outcome, the position selection and the single pin.
- The overlay notices and their dedup key.
- The document-session and version fences, and the disk-fact-only write
  capability.
- Any host code (watcher, debounce, worker).

Unclear in the ticket:

- Where the overlay lives: per workspace or per root. Ticket 33 decides.
- The number B. Ticket 33 decides.
- The LSP re-push rule after a publish. Ticket 33 decides.
- Whether a refresh with no change commits when it clears a racy bit. Ticket 25
  leaves this [H].
- The cost of the heavy-note entry clone and of the `Arc<Note>` variant.
- The cost of the full re-resolve with one thread.

### Final review amendments (2026-11-16)

The maintainer confirms the resolution-context, scan-certificate and bounded
snapshot rules above. The review also adds session fencing, native admission
handoffs and a disk-fact-only write capability.

Ordered LSP input and bounded parse work serve different jobs. The host applies
every edit, then coalesces obsolete parse jobs. It retains no event history.
The [LSP specification][review-lsp] defines ordered application. The
[rust-analyzer architecture][review-ra] separates mutable host changes from
immutable analysis snapshots. These precedents do not measure Traces latency.

Required regressions include persistent overlay paths, save-edit-close,
no-save close, failed scans, ghost recreation, two dependent incremental edits
and close/reopen with a restarted version. Compare every derived graph with a
full build. Measure the amended context's memory and shipped-profile latency
before adoption.

[review-lsp]: https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/textDocument/didChange.md
[review-ra]: https://rust-analyzer.github.io/book/contributing/architecture.html
