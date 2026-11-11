# How should open editor buffers relate to the persisted index?

Type: grilling
Status: open
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
- Missing or corrupt required facets mean a complete cache miss. The result
  never looks silently complete.

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

Note on a stale cross-reference: ticket 25 line 279 says "tickets 33 and 27".
That line predates the 2026-11-09 renumbering. The store-use change row at
ticket 25 line 541 names tickets 28 and 33. This ticket reads the change row as
current.

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
unsaved new document drops the content (`tests/reconcile_test.rs:69-78`) [V].
The CHANGELOGs show why. Last-write-wins let disk changes overwrite unsaved
buffers (v0.17.0, reversed to editor ownership). Rebuilding the BM25 index per
keystroke stalled the server (v0.18.0, now batched after 300 ms idle) [V]. It
supports a single root and no `$/cancelRequest` [V]. The IWE MCP crate writes
mutations to disk and updates its own graph [V]. Two writers can touch one vault
through separate processes. [H] Because IWE has no persisted index, this settles
only the in-process view of buffers. It does not test the persisted-index
question of this ticket.

## Evidence limits

- The IWE ownership rule covers one process, one root and no cancellation [V].
  It is evidence for the in-process view only [H].
- The v0.17.0 and v0.18.0 reversals are changelog facts [V]. They are not
  measurements of a persisted-index design.
- No Traces host code exists. `.scratch/md-pkm-lsp/` states intent only.
- No measurement exists for overlay costs: parse per keystroke, debounce, or
  memory. `NoteDoc::scan` is about 29% of parse time (ticket 23, open item). The
  50,000-note figures are linear extrapolations [H] (ticket 23).
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
