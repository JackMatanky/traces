# Research: IWE Core and Hosts Architecture

Study of `iwe-org/iwe` (Rust workspace: `liwe`, `diwe`, `iwe`, `iwes`, `iwec`).
Focus: crate topology, document model, host layering, in-memory state lifecycle,
live buffers, configuration, diagnostics, benchmarks, and design reversals.

## 1. What IWE Actually Does

### Crate Topology and Public Surface

IWE is organised as a 5-crate Cargo workspace [Cargo.toml:1-4]:

- `liwe` [V: crates/liwe/Cargo.toml:2]: Pure core library. Zero filesystem I/O.
  Exposes domain models (`Graph`, `Arena`, `Tree`, `Document`, `Inlines`),
  keys and indexing (`Key`, `KeyIndex`, `RefIndex`), parsing (`Parser`,
  `MarkdownEventsReader`), and pure mutations (`operations::rename`, `extract`,
  `inline`, `attach_reference`, `delete`). Feature-gated `djot` support [V:
  crates/liwe/Cargo.toml:19]. Depends only on general libraries (pulldown-cmark,
  rayon, rustc-hash, serde, serde_yaml).
- `diwe` [V: crates/diwe/Cargo.toml:2]: Engine library ("IWE engine library").
  Depends on `liwe` [V: crates/diwe/Cargo.toml:18]. Owns I/O operations (`fs`,
  `loader`), workspace boundaries (`key_escapes_workspace`), configuration
  loading and migrations (`config`), search (`bm25`, `fuzzy-matcher`), and
  filesystem watching (`notify` via `watcher`) [V: crates/diwe/Cargo.toml:34].
- `iwe` [V: crates/iwe/Cargo.toml:2]: CLI binary. Depends on `liwe` and `diwe`.
  Parses clap arguments, invokes use cases via `diwe::find`, `diwe::retrieve`,
  or `liwe::operations`, applies disk changes via `diwe::fs::apply_changes`,
  and formats text or JSON output.
- `iwes` [V: crates/iwes/Cargo.toml:2]: LSP server binary. Depends on `liwe`
  and `diwe`. Single-threaded event loop over `lsp-server` and
  `crossbeam-channel` [V: crates/iwes/src/router.rs:94-135]. Translates
  operations to `WorkspaceEdit`.
- `iwec` [V: crates/iwec/Cargo.toml:2]: MCP server binary. Depends on `liwe`
  and `diwe`. Exposes stdio and streamable HTTP via `rmcp` [V:
  crates/iwec/src/main.rs:7-10]. Executes mutations directly to disk and
  updates its in-memory graph.

### Shared Use Cases (The App Layer)

Use cases live primarily in `diwe` and `liwe::operations`, not in the CLI or
hosts [V]:

- Mutating operations (`rename`, `delete`, `extract`, `inline`, `attach`) live
  in `liwe::operations` [V: crates/liwe/src/operations.rs:11-20]. They take
  `&Graph` and return a pure changeset `Changes { creates, updates, removes }`
  [V: crates/liwe/src/operations/changes.rs:6-10]. No filesystem access occurs.
- Disk execution lives in `diwe::fs::apply_changes` [V:
  crates/diwe/src/fs.rs:175], which writes through `write_file_if_changed` [V:
  crates/diwe/src/fs.rs:48] and validates workspace boundaries [V:
  crates/diwe/src/fs.rs:24].
- Read queries live in `diwe`: `DocumentFinder` [V: crates/diwe/src/find.rs:51]
  and `DocumentReader` [V: crates/diwe/src/retrieve.rs:76]. They wrap `&Graph`
  and execute pure filtering, sorting, expansion, and token budgeting.
- Hosts are thin adaptors: CLI maps clap args -> query/operation -> terminal
  output; LSP maps RPC -> operation -> `WorkspaceEdit`; MCP maps JSON-RPC ->
  operation -> disk write -> tool response. Zero one-implementor traits exist
  across the boundary [V].

### Document Model: Arena, Tree, and Lossiness

IWE represents documents across three distinct forms:

1. Raw Content: `HashMap<Key, Content>` (where `Content` is `String`) stores
   verbatim file text inside `Graph` [V: crates/liwe/src/graph.rs:47,66].
2. AST / Document: `Document` holding hierarchical blocks (`Header`, `Para`,
   `BulletList`, `Table`, etc.) and inlines [V:
   crates/liwe/src/model/document.rs:11-54].
3. Structural Graph: An arena-backed binary tree using `first-child` /
   `next-sibling` pointers [V: crates/liwe/src/graph/arena.rs:21-24;
   docs/data-model.md:5-15]. `GraphNode` represents blocks [V:
   crates/liwe/src/graph/graph_node.rs:7-19] with `prev: NodeId`, `next:
   MaybeNodeId`, and `child: MaybeNodeId`. Text inlines live in `Line` structs
   in the arena.
4. Spans: `nodes_map: HashMap<Key, Vec<(NodeId, LineRange)>>` maps nodes to
   1-based line number ranges [V: crates/liwe/src/graph.rs:60;
   crates/liwe/src/model.rs:82,265]. Inlines do not carry byte offsets or line
   spans [V: crates/liwe/src/model/inline.rs:26-49].
5. Lossless? NO [V]. Round-tripping is lossy. Markdown rendering reconstructs
   syntax via `pulldown-cmark-to-cmark` [V:
   crates/liwe/src/markdown/writer.rs:2-27] according to formatting tokens
   (`emphasis_token`, `list_token`, `wrap_column`). Indentation, list markers,
   and soft breaks are synthesized, causing numerous text loss and formatting
   bugs recorded in CHANGELOGs [V: crates/iwe/CHANGELOG.md:18,52,56,99,106].

### State Lifecycle and Filesystem Synchronization

- In-memory graph rebuilt from scratch: The CLI has NO persistent cache (no
  SQLite, no redb) [V: docs/benchmark.md:8-15]. On every invocation, it walks
  the workspace via `ignore::WalkBuilder`, parses all markdown files in
  parallel using Rayon, and constructs the arena and indexes [V:
  crates/diwe/src/fs.rs:62-69; crates/liwe/src/graph.rs:567-586].
- Incremental updates in servers: LSP (`iwes`) and MCP (`iwec`) build the
  graph once at startup, then mutate it in place via
  `Graph::update_document(key, content)` [V: crates/liwe/src/graph.rs:181;
  crates/iwes/src/router/server.rs:86]. This unindexes the key, deletes the
  node subtree from `Arena`, parses the new document, and ingests new nodes [V:
  crates/liwe/src/graph.rs:529-538].
- Watcher: `diwe::watcher` runs `notify::RecommendedWatcher` or `PollWatcher`
  [V: crates/diwe/src/watcher.rs:64-110]. A `PathFilter` drops `.git`, `.iwe`,
  and ignored files [V: crates/diwe/src/fs.rs:76-115]. Events are dispatched
  as `FsChange::Update(Key, String)` or `FsChange::Remove(Key)`.

### Live Buffer Overlay in LSP

- No shadow graph or overlay fork exists [V].
- Unsaved editor buffers are written directly into `Server.graph` via
  `Graph::update_document` upon `textDocument/didOpen` and
  `textDocument/didChange` [V: crates/iwes/src/router/server.rs:140-145,
  235-244]. Synchronization uses `TextDocumentSyncKind::FULL` [V:
  crates/iwes/src/main.rs:82].
- Ownership rule: `Server` tracks `open_documents: HashSet<Key>` [V:
  crates/iwes/src/router/server.rs:48]. While a document is open, external
  filesystem change events are discarded (`apply_external_update` drops them)
  [V: crates/iwes/src/router/server.rs:115-117].
- Reconciliation on close: When `textDocument/didClose` fires, the document is
  removed from `open_documents`, re-read from disk, and replaced in `Graph` [V:
  crates/iwes/src/router/server.rs:152-170]. Unsaved new documents closed
  without saving are dropped [V: crates/iwes/tests/reconcile_test.rs:69-78].
- Single-root only: `workspace_folders` is `None` [V:
  crates/iwes/src/main.rs:106]. Multi-root is unsupported. Cancellation
  requests (`$/cancelRequest`) are not handled; requests process sequentially
  in the event loop [V: crates/iwes/src/router.rs:104-134].

### Configuration Model

- Config lives at `.iwe/config.toml` [V: crates/diwe/src/config.rs:21-22].
- Managed by `diwe::config::load_config` [V: crates/diwe/src/config.rs:470].
- Migration in place: `migrate(&raw)` checks `version` and rewrites TOML
  in-place using `toml_edit` to preserve user layout and comments [V:
  crates/diwe/src/config.rs:505-525].
- Strict schema: Tables use `#[serde(deny_unknown_fields)]` [V:
  crates/diwe/src/config.rs:25,80], with validated fallback for formatting.

### Error and Diagnostic Types

- No central monolithic error type [V].
- `liwe` defines narrow domain enums: `OperationError` (NotFound,
  AlreadyExists, InvalidTarget, NoParentSection) [V:
  crates/liwe/src/operations/changes.rs:88] and `SelectError<T>` [V:
  crates/liwe/src/operations/select.rs:22].
- `diwe` uses `std::io::Result` for file operations and `Result<T, String>` for
  validation checks like `require_documents` [V: crates/diwe/src/keys.rs:5].
- LSP adapts errors to `lsp_server::ResponseError` [V:
  crates/iwes/src/router/server.rs:572-588]. CLI formats strings to stderr and
  sets process exit code.

### Test and Benchmark Approach

- Benchmarks: Criterion harness in `crates/iwe/benches/` measuring Stage 1
  (load plain vs BM25 indexed) and Stage 2 (queries) [V:
  docs/benchmark.md:29-38].
- Corpus: Synthetic, deterministic (`seed_from_u64(42)`), ~700 B/doc, 5
  frontmatter fields, 3 inclusion links, 2 headers, 6 paragraphs, 6 inline
  links [V: docs/benchmark.md:47-84].
- Scale tested: 5k, 10k, and 20k documents [V: docs/benchmark.md:86]. On Apple
  M3 Pro: 5k cold load = 144 ms plain / 204 ms indexed; 10k = 329 ms / 435 ms;
  20k = 705 ms / 958 ms [V: docs/benchmark.md:155-162]. BM25 adds ~35% overhead.
- Query scaling: Linear in corpus size for field filters (~12 ms for 20k), but
  super-linear for `--roots` (60 ms for 20k) and backlinks due to linear edge
  scans [V: docs/benchmark.md:185-208].
- Integration tests: Single `tests/main.rs` per crate (`liwe`, `iwe`, `iwes`,
  `iwec`) to avoid Cargo per-test binary link overhead [V: crates/*/Cargo.toml].

### Design Reversals in CHANGELOGs

- Normalization clobbering external edits (v0.26.0) [V:
  crates/iwe/CHANGELOG.md:52]: Whole-file re-synthesis on editor tool writes
  reformatted markdown and broke links in READMEs. Reverted so external edits
  are not rewritten into canonical form.
- Mtime thrashing from blind normalization (v0.24.1) [V:
  crates/diwe/CHANGELOG.md:59]: Normalizing the workspace touched all files,
  wiping edit history. Introduced `write_file_if_changed` to check bytes first.
- Search index thrashing (v0.18.0) [V: crates/iwes/CHANGELOG.md:128-132]:
  Rebuilding BM25 on every keystroke stalled the server. Switched to
  batch-rebuilding on idle delay (300 ms) or lazy query execution.
- Open buffer clobbering (v0.17.0) [V: crates/iwes/CHANGELOG.md:144-148]:
  Initially last-write-won caused disk changes to overwrite unsaved buffers.
  Reversed to strict editor ownership (`open_documents`).
- Knowledge filter abstraction deleted (v0.23.0) [V:
  crates/iwe/CHANGELOG.md:123]: Removed `knowledge_filter`, `recency_field`,
  and git status coupling; session start now treats memory slices as queries.
- Library to Workspace rebranding (v0.25.0) [V: crates/iwe/CHANGELOG.md:63]:
  Renamed `[library]` config table and `--library` flags to `[workspace]`.

## 2. Invariants and Owners

1. Filesystem boundary invariant [V]: No operation may read, write, or rename a
   file outside the workspace.
   Owner: `diwe::fs::key_escapes_workspace` and `workspace_document_path` [V:
   crates/diwe/src/fs.rs:15,24]. Checked before touching disk or graph.
2. In-memory mutation isolation [V]: Domain operations never touch the
   filesystem.
   Owner: `liwe::operations`. All mutations return pure `Changes` records [V:
   crates/liwe/src/operations/changes.rs:6].
3. Editor buffer authority [V]: An open editor buffer is authoritative over
   on-disk file content.
   Owner: `iwes::server::Server` via `open_documents: HashSet<Key>` [V:
   crates/iwes/src/router/server.rs:48, 115]. External disk events are ignored
   for open keys until closed.
4. Mtime preservation invariant [V]: A document write that produces identical
   content must not touch disk or update filesystem mtime.
   Owner: `diwe::fs::write_file_if_changed` [V: crates/diwe/src/fs.rs:48].
5. Key resolution determinism [V]: Wikilinks resolve by shortest unambiguous
   path suffix, breaking ties lexicographically by path.
   Owner: `liwe::model::KeyIndex` [V: crates/liwe/src/model/key_index.rs:11;
   docs/keys.md:51-56]. Cached in `RwLock<Resolutions>`, invalidated by bucket.

## 3. Tradeoffs and Costs It Pays

- Cold rebuild cost for CLI [H]: By having no persistent cache, `iwe` pays
  140-700 ms CPU on every command. For vaults up to 10k notes, this is
  acceptable interactively, but prevents instant sub-10ms CLI responsiveness.
- Lossy markdown rewrite hazard [H]: Because the internal representation is an
  AST rather than a lossless CST, formatting or modifying a note rewires the
  syntactic layout via `pulldown-cmark-to-cmark`. The project suffered repeated
  regressions where text inside lists or complex formatting was mangled.
- Memory overhead in daemon [H]: Storing raw markdown `String` in `content:
  HashMap<Key, Content>` alongside the structural `Arena` duplicates text data
  in RAM.
- Unbounded memory in single-thread event loop [H]: The LSP server runs on a
  single crossbeam channel loop. Long query operations or index updates block
  subsequent RPC handling, relying on a 300ms idle delay to batch search index
  rebuilds.

## 4. Transferable Lessons

1. Clean core/engine separation (`liwe` vs `diwe`) [H]: Isolating pure domain
   types and transformations (`liwe`) from filesystem I/O and server
   infrastructure (`diwe`) cleanly prevents I/O leakage into domain algorithms.
2. Changeset abstraction for mutations [H]: Returning `Changes { creates,
   updates, removes }` from domain operations allows CLI (disk writer), LSP
   (WorkspaceEdit generator), and MCP (direct applier) to execute the exact same
   use case without duplication.
3. Editor buffer authority via `open_documents` [H]: Tracking open keys and
   ignoring filesystem watcher events until buffer close is dramatically simpler
   than complex branch-and-merge algorithms and completely solves editor/disk
   races for PKM vaults.
4. Fast cold parallel parsing [V for the numbers, H for the reading]:
   Rayon-parallel parsing reaches ~28,000 notes/s (705 ms for 20k synthetic
   ~700 B notes, Apple M3 Pro). The same table [docs/benchmark.md:155-162] puts
   5k at 144 ms. Traces' own cold 20k query is <75 ms on an M4 Pro through redb
   (ticket 11), so the two are not comparable without a re-run on one machine
   and corpus; IWE shows only that a persistent cache is an optimisation at
   <=10k notes, not that it is dispensable at 20k-50k (the ticket 13 sizes).
   [Editor note: the original line claimed ~30,000 notes/s and "not a hard
   prerequisite" without the cross-machine caveat.]
5. In-place TOML migrations via `toml_edit` [H]: Preserving comments and
   formatting while automatically updating configuration schema across
   versions avoids breaking user config files on upgrades.
6. Single integration test binary [H]: Grouping tests under a single
   `tests/main.rs` dramatically reduces linker strain during development
   compared to dozens of separate integration test binaries.

## 5. Lessons That Do NOT Transfer and Why

1. Lack of persistent index cache [H]: IWE relies on pure in-memory state and
   pays 144-705 ms per CLI run at 5k-20k notes (~200 ms at 5k, which is what
   a hook-driven agent loop would pay per call). Whether Traces keeps redb is
   ticket 20/21's decision, to be taken on tickets 11's measurements (redb
   halves peak RSS and answers a cold 20k query in <75 ms), not on this
   comparison; this research only shows the cost of the alternative.
2. Lossy AST markdown re-generation [H]: IWE's strategy of parsing to AST and
   re-synthesizing markdown via `pulldown-cmark-to-cmark` is unacceptable for
   Traces. Traces promises non-destructive editing of user notes; edits must be
   surgical (span-based or CST-preserving), never rewriting unedited sections.
3. Full document sync in LSP [H]: `TextDocumentSyncKind::FULL` sends entire
   document text on every keystroke. While workable for small notes, incremental
   sync is preferable for large documents and low-latency diagnostic updates.
4. Single-root workspace assumption [H]: IWE does not support multi-root
   workspaces (`workspace_folders: None`). Traces designs for flexible vault
   roots and inclusion across directories.

## 6. What This Changes or Challenges in Traces Decisions

- Confirms [ticket 16](../issues/16-schema-note-typing-and-validation.md) Layering: IWE
  strongly validates that CLI, LSP, and MCP should be thin hosts of an
  engine/core layer. Domain operations should return pure data (`Changes`), and
  use cases should be explicit functions or thin structs (`DocumentFinder`),
  not heavy context objects.
- Informs [ticket 28](../issues/28-crate-topology-and-public-surface.md): IWE
  demonstrates the value of a two-tier core: a zero-I/O domain crate (`liwe`
  analogue) and an engine/app-layer crate (`diwe` analogue) that owns I/O,
  config, and watcher, consumed equally by CLI, LSP, and MCP.
- Informs [ticket 30](../issues/30-live-buffer-and-persisted-index-consistency.md):
  IWE needs no shadow graph because it has no persisted index: the single
  in-memory graph is the only state, and an open buffer is written into it
  (`open_documents` makes the editor authoritative and drops watcher events for
  that key until close). Traces has a persisted redb index and a CLI that may
  run beside the LSP, so the rule transfers only for the LSP process's own
  in-memory view, not for the index [H]. IWE also shows the ownership rule's
  cost: it handles only a single root and no cancellation [V].
- Informs [ticket 19](../issues/19-note-document-model.md): IWE confirms the separation
  of raw content from structural arena nodes, but serves as a cautionary tale
  against whole-document AST re-serialization.
- Informs [ticket 27](../issues/27-error-and-diagnostic-architecture.md): IWE avoids
  monolithic error crates, using localized operation error enums and lightweight
  string diagnostics, proving that complex error hierarchies across the
  core/host seam are unnecessary overhead.
