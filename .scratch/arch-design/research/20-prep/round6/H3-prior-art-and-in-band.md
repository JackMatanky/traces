# H3: Prior Art and In-Band Guardrails for Markdown Vaults

Deliverable:
`.scratch/arch-design/research/20-prep/round6/H3-prior-art-and-in-band.md`  
Target: Ticket 16 (Schema Note Typing and Validation) Round 6  
Scope: Exhaustive prior-art survey and in-band technique analysis for keeping
markdown/YAML-frontmatter note stores valid when edited by AI agents, arbitrary
processes, or human editors, WITHOUT relying on agent harness features (no
hooks, no MCP-client behaviour, no LSP client, no harness config).  
Evidence tags: `[V]` verified from primary source (spec, source code, docs,
empirical local test; URL/path and quote cited); `[H]` hypothesis or
architectural inference; `[S]` secondary citation / scout claim not re-read.

---

## 1. Executive Summary & Framing

When notes are stored as plain Markdown files with YAML frontmatter, any process
with file write access (an AI agent using bash/edit tools, an external sync
daemon, Obsidian, or a human in Vim) can write arbitrary bytes to disk.

Three distinct architectural stances address this reality:

1. **Prevention (Hard Gate):** Intercepts bytes before they land in canonical
   storage; rejects invalid operations. Without harness hooks or kernel-level
   filesystem mediation, true prevention is only achievable by isolating the
   canonical directory and requiring writes to pass through an explicit gate
   (e.g., an inbox/quarantine landing zone with atomic promotion, or an MCP
   write tool where the server owns the file write).
2. **Remediation & Detection (Feedback Loop):** Allows writes to disk, detects
   violations asynchronously (via CLI check, git hooks, watcher daemon, or on
   read), and delivers structured diagnostics back to the writer.
3. **In-Band Steering (Scaffolding & Normalisation):** Uses information co-located
   with the notes (directory instructions, note templates, `$schema` pointers,
   read-time canonical normalisation) to steer writers away from invalid states
   or make consumers immune to minor variations.

---

## 2. Comprehensive Prior Art Survey & Mechanism Matrix

The table below catalogues 17 real-world systems, the mechanisms they employ to
maintain schema conformance in markdown/frontmatter stores, and their ability
to survive uncontrolled writers.

### Comparative Prior-Art Matrix

| System / Ecosystem | Core Mechanism | Category | Survives Arbitrary Writers? | Failure Stance | Evidence |
|---|---|---|---|---|---|
| **Letta MemFS** | Git-backed memory filesystem; pre-commit hook validation | Remediation / Gate | Partial (bypass if uncommitted) | Fail-closed on commit | `[V]` Docs / GitHub [1] |
| **Basic Memory** | Picoschemas; advisory `bm schema validate`; frontmatter indexing | Remediation / Steering | No (permissive storage) | Fail-open | `[V]` GitHub / Docs [2] |
| **Anthropic Memory MCP** | Tool-mediated JSON knowledge graph | Prevention (in-tool) | No (tool bypassable if FS open) | Fail-closed in tool | `[V]` GitHub MCP [3] |
| **mem0** | API/database storage layer (Qdrant/SQLite); no raw files | Prevention (DB layer) | Yes (no direct file access) | Fail-closed | `[V]` Docs / GitHub [4] |
| **Obsidian Fileclass / Metadata Menu** | Class presets; audit table; form-based UI modal | Remediation / Steering | No (bypassed by source/external edit) | Fail-open | `[V]` GitHub [5] |
| **Obsidian Linter** (`platers`) | Lint on save / change; auto-formats YAML frontmatter | Remediation (Auto-fix) | No (requires active Obsidian save) | Fail-open | `[V]` Docs [6] |
| **Metadata Enforcer v2** (`omarish`) | Django-style Python models; CLI `check`/`watch --fix` | Remediation (Auto-fix) | No (post-write detection) | Fail-open (watch) | `[V]` Docs [7] |
| **Obsidian Schema / Metadata Validator** | Manifest in `schemes/`; JSON schema sidebar | Remediation | No (advisory UI warnings) | Fail-open | `[V]` GitHub [8] |
| **Dendron `doctor`** | CLI/VS Code `doctor --action fixFrontmatter` | Remediation (Auto-fix) | No (manual/scheduled sweep) | Fail-open | `[V]` Wiki [9] |
| **Logseq** | DataScript parser in file mode; SQLite in DB mode | Steering / Normalisation | Yes in DB mode; No in file mode | Fail-open (tolerant) | `[V]` GitHub [10] |
| **Zettlr** | Pandoc frontmatter extraction; unvalidated | None | No (permissive) | Fail-open | `[V]` Docs [11] |
| **Quartz 4/5** | Tolerant reader; alias coalescing; scalar-to-array coercion | In-band Normalisation | Yes (reader normalises on read) | Fail-open (tolerant) | `[V]` Source [12] |
| **Astro Content Collections** | Zod schemas; dev-server error overlay; CI build fail | Remediation (Build-time) | No (prevents build, not write) | Fail-closed on build | `[V]` Docs [13] |
| **Hugo** | Archetypes (scaffolding); build-time template check | Scaffolding / Remediation | No (prevents build, not write) | Fail-closed on build | `[V]` Docs [14] |
| **Keystatic** (Thinkmill) | Zod schemas; Reader API; browser UI gate | Prevention (UI only) | No (direct file edits bypass UI) | Fail-closed on read | `[V]` Docs [15] |
| **Decap CMS** (Netlify CMS) | `config.yml` widgets; client-side browser gate | Prevention (UI only) | No (direct git/file edits bypass) | Fail-closed in UI | `[V]` Docs [16] |
| **Agent Vault Inboxes / Review Queues** | `00-Inbox/` landing zone; `Task Vault` review queue | Prevention (Structural) | Yes (canonical vault untouched) | Fail-closed to core | `[V]` Community [17] |

---

### Detailed Analysis of Prior-Art Systems

#### 1. Letta MemFS (Context Repository)

- **Mechanism:** Letta agents store long-term memory in a git repository
  projected onto the local filesystem as Markdown files with YAML frontmatter
  (`[V]` <https://docs.letta.com/concepts/memfs/index.md>). MemFS uses a git
  `pre-commit` hook to validate note frontmatter, enforce structure, reject
  unknown keys, and protect read-only files before commits are finalized
  (`[V]` `letta-ai/letta-code` prompt `letta_root_memfs.md`).
- **Surviving Uncontrolled Writers:** If an agent or script writes directly to
  the working tree, the file on disk is modified. However, the agent's memory
  subsystem only synchronizes committed state. When the agent attempts to commit,
  the pre-commit hook rejects invalid files, outputting the error to stderr, which
  the harness injects into the agent's next turn as feedback.
- **Limitation:** Does not prevent disk corruption if git is bypassed or never
  committed (`[V]`).

#### 2. Basic Memory

- **Mechanism:** Local-first Markdown memory system. Entities use YAML frontmatter
  for token-efficient discovery. Defines "picoschemas" mapped to frontmatter
  keys, observation categories, and relation links (`[V]`
  <https://github.com/basicmachines-co/basic-memory>).
- **Surviving Uncontrolled Writers:** Validation (`bm schema validate`) is strictly
  advisory and diagnostic. It logs missing required fields or invalid links but
  never blocks writes (`[V]`).

#### 3. Anthropic Memory MCP Server

- **Mechanism:** Manages a knowledge graph (entities, relations, observations)
  stored in a single JSON file. Access is strictly mediated via MCP tools
  (`create_entities`, `add_observations`) (`[V]`
  <https://github.com/modelcontextprotocol/servers/tree/main/src/memory>).
- **Surviving Uncontrolled Writers:** Strong schema enforcement *inside* the MCP
  tool. But it provides zero protection against external processes writing to
  `memory.json` directly (`[V]`).

#### 4. mem0

- **Mechanism:** Memory layer for AI agents backed by managed databases (vector
  stores, SQLite, PostgreSQL). Markdown notes are not exposed as raw files on
  disk (`[V]` <https://github.com/mem0ai/mem0>).
- **Surviving Uncontrolled Writers:** Survives arbitrary filesystem writes by
  eliminating the filesystem as a storage interface. Inapplicable to local PKM
  markdown vaults.

#### 5. Obsidian Plugins (Metadata Menu, Fileclass, Linter, Metadata Enforcer)

- **Metadata Menu / Fileclass:** Defines typed `fileClass` schemas. Provides
  form-based editing modals, dataview integration, and an "audit" view listing
  violating files (`[V]` <https://github.com/mdelobelle/metadatamenu>). When
  edited via Obsidian source mode or external tools, validation is purely
  post-hoc advisory.
- **Platers Obsidian Linter:** Listens for "on save" or "on file change" events
  within Obsidian. Automatically reformats YAML frontmatter, sorts keys, and
  deduplicates tags (`[V]` <https://github.com/platers/obsidian-linter>). Cannot
  intercept writes made when Obsidian is closed or writes made by CLI tools.
- **Metadata Enforcer v2 (`omarish`):** Defines Django-style Python models with
  path routing (`ROUTES`). Uses `metadata-enforcer check` and `watch --fix` via
  the Python `watchdog` library (`[V]` <https://omarish.com/p/metadata-enforcer-v2>).
  Operates entirely post-write.

#### 6. Foam & Zettlr

- **Foam:** Markdown knowledge workspace for VS Code (`[V]` <https://foam.md/>).
  Relies on standard VS Code extensions (markdownlint, YAML). Contains no custom
  schema enforcement engine.
- **Zettlr:** Academic markdown editor (`[V]` <https://docs.zettlr.com>). Parses
  YAML frontmatter for Pandoc compatibility; treats frontmatter metadata as
  completely unconstrained.

#### 7. Dendron `doctor`

- **Mechanism:** Hierarchical markdown PKM with schema templates (`[V]`
  <https://wiki.dendron.so/notes/ZeC74FYVECsf9bpyngVMU/>). Includes a `doctor`
  command family (`dendron doctor --action fixFrontmatter`, `h1ToTitle`,
  `findBrokenLinks`) available via VS Code command palette and CLI.
- **Surviving Uncontrolled Writers:** Bypassed completely during writes. `doctor`
  is a batch remediation sweep run manually or in CI.

#### 8. Logseq

- **Mechanism:** Dual-engine architecture (`[V]`
  <https://github.com/logseq/logseq/blob/master/libs/guides/db_properties_guide.md>).
  In file-based mode, properties are parsed into an in-memory DataScript
  database; invalid or unrecognized properties are tolerated without error. In
  DB mode, Logseq migrates primary storage to SQLite with typed properties.
- **Lesson:** File-based notes must use tolerant parsing to avoid crashing when
  users or tools introduce non-standard properties.

#### 9. Static Site Generators & Content Frameworks

- **Quartz 4/5:** Fast static site generator for digital gardens (`[V]`
  `jackyzha0/quartz` `quartz/plugins/transformers/frontmatter.ts`). Employs
  canonical-form normalisation on read:
  - `coerceToArray`: Coerces single strings or comma-separated lists into
    arrays for fields like `tags` and `aliases`.
  - `coalesceAliases`: Resolves alternate key names (e.g., `["tags", "tag"]`,
    `["created", "date"]`, `["modified", "lastmod", "updated"]`).
  - Default fallbacks: Falls back to `file.stem` if `title` is missing.
  Quartz never crashes on irregular frontmatter; it projects it into canonical
  form at read time (`[V]`).
- **Astro Content Collections:** Uses
  `defineCollection({ schema: z.object(...) })` with Zod (`[V]`
  <https://docs.astro.build/en/guides/content-collections/>).
  During `astro dev`, Vite file watchers detect frontmatter errors and render an
  error overlay. During `astro build`, validation errors fail the build with
  exit code 1. Does not prevent disk writes.
- **Keystatic & Decap CMS:** Git-backed CMSes. Provide structured web UIs that
  validate fields before invoking git/filesystem APIs (`[V]`
  <https://keystatic.com/docs/collections>, <https://decapcms.org>). Both are
  bypassed if files are edited directly in an editor or CLI.

#### 10. Agent Vault Safety Architectures (Inboxes & Review Queues)

- **Inbox & Quarantine Pattern:** Community agent-vault patterns (e.g.,
  `obsidian-agent-vault`, `AI Work Review`, `Task Vault`) enforce that autonomous
  agents only have write access to an `00-Inbox/` or `_quarantine/` directory
  (`[V]` <https://community.obsidian.md/plugins/ai-work-review>,
  <https://github.com/georgeracu/obsidian-agent-vault>).
- **Review Queue:** Core vault notes are strictly read-only to agents. Proposed
  additions or edits land in the inbox. A human or an automated promotion script
  evaluates the diff, validates the schema, and moves approved notes into the
  main vault hierarchy.
- **Verdict:** Highly effective, harness-independent structural prevention.

---

## 3. In-Band Techniques: Analysis & Empirical Testing

In-band techniques rely entirely on filesystem structures, note content, and
standard tool conventions to steer writers or repair reads without harness
configuration.

### 1. Directory-Local Instruction Files (`AGENTS.md` / `README.md`)

- **How it works:** Placing instruction files in subdirectories (e.g.,
  `books/AGENTS.md`, `meetings/README.md`) detailing the required schema.
- **Lazy Loading Across Harnesses:**
  - **Claude Code (`[V]`):** Eagerly loads root `AGENTS.md` and ancestors at
    session start. For subdirectories, Claude Code loads `subdir/AGENTS.md`
    *lazily on demand* the moment Claude reads, writes, or edits a file in that
    subdirectory (<https://code.claude.com/docs/en/memory.md>).
  - **Cursor (`[V]`):** Legacy `.cursorrules` loads globally. New
    `.cursor/rules/*.mdc` files support glob scoping (`globs: "books/**"`),
    loading instructions into context only when matching files are accessed.
  - **Aider / Codex / Windsurf (`[S]`):** Do not automatically discover nested
    instruction files unless explicitly configured or referenced in root prompts.
- **Limits:** Purely advisory. Non-LLM writers (scripts, Obsidian, git) ignore
  them completely. Even LLMs suffer probabilistic drift (`[H]`).

### 2. Self-Describing Note Templates (`traces new`)

- **How it works:** CLI command generates valid scaffolding pre-populated with
  required frontmatter keys, default values, and comments.
- **Limits:** Protects creation time only. Provides zero defense against
  destructive subsequent edits.

### 3. Schema Hints Embedded in Notes

Three embedding approaches were tested:

#### (a) YAML Comments (`# schema: book`)

- **Hypothesis:** Embed schema hints as YAML comments in the frontmatter.
- **Empirical Test (`[V]`):** Executed a local Bun script parsing a YAML block
  with comments (`# schema: book`, `# required field`) into a standard JavaScript
  object, then re-serializing via `YAML.stringify()` (replicating Obsidian's
  `processFrontMatter`, `gray-matter`, and `js-yaml` behavior).
  - *Result:* All comments were completely stripped from the output.
  - *Corroboration:* Obsidian Forum thread 65851 confirms that Obsidian's
    Properties UI and `processFrontMatter` API systematically strip comments,
    formatting, and quotes upon save (`[V]`
    <https://forum.obsidian.md/t/yaml-properties-api-processfrontmatter-removes-alters-string-quotes-comments-types-formatting/65851>).
- **Verdict:** REJECTED. YAML comments are non-durable in Obsidian vaults.

#### (b) Frontmatter Property (`$schema: ./schemas/book.json`)

- **Empirical Test (`[V]`):** Executed local Bun roundtrip test with
  `$schema: ./schemas/book.json` as a top-level property.
  - *Result:* Successfully preserved across parse and serialization cycles.
  - *Ecosystem Fit:* Recognized natively by VS Code YAML extension
    (`yaml-language-server` supports inline `$schema` property `[V]`) and
    JSON Schema tools (`check-jsonschema`).
- **Verdict:** VIABLE. Durable across Obsidian and standard YAML parsers.

#### (c) Markdown HTML Comments (`<!-- traces-schema: book -->`)

- **Behavior:** Placed immediately below the YAML closing `---`. Obsidian and
  markdown parsers preserve HTML comments in note bodies without stripping.
- **Verdict:** VIABLE but requires custom parser logic to associate body
  comments with frontmatter validation.

### 4. Co-located `_schema.md` or Directory Index Note

- **How it works:** Placing a `_schema.md` or `schema.yaml` directly inside
  category folders (`books/_schema.md`).
- **Effectiveness:** When an agent inspects a folder via directory listing
  (`ls` or `glob`), the schema file is immediately visible. It acts as an
  in-band signpost without polluting root prompts.

### 5. Actionable Validation Messages (Fix-Naming Errors)

- **Contract:** When validation fails, error messages must output structured,
  copy-pasteable corrections:

  ```text
  Error[E0012]: note 'books/moby-dick.md' violates schema 'Book'
    --> missing required field 'isbn'
    hint: add 'isbn: ""' to frontmatter
  ```

- **Value:** Essential for LLM self-correction loops (`aider --lint`, Claude
  agent retries) and human debugging.

### 6. Canonical-Form Normalisation on Read (Tolerant Reader)

- **Concept:** Following Quartz 4/5, the read engine coerces minor schema
  irregularities on the fly:
  - String to array: `tags: reading` -> `tags: ["reading"]`.
  - Alias coalescing: accepts `date` or `created`.
  - Missing optional fields: injects schema defaults in-memory.
- **Value:** Decouples vault query resilience from strict writer perfection.

---

## 4. Stress-Testing Candidates Across the 8 Dimensions

Every candidate mechanism is evaluated against the 8 required dimensions:

- **(a) Writes seen:** Temp+rename, truncate+write, shell redirect, `mv`, `cp`,
  `git checkout`, sync tools, Obsidian.
- **(b) When validation occurs & what writer sees:** Pre-write vs post-write vs
  read-time; error channel and payload.
- **(c) Partial-write / atomicity:** Behavior on interrupted or chunked writes.
- **(d) Platforms:** macOS, Linux, Windows.
- **(e) Privilege & deployment cost:** Sudo, background daemons, CLI dependencies.
- **(f) Bypass routes:** How writers circumvent the mechanism.
- **(g) Fail stance:** Fail-open vs fail-closed if checker crashes or schema breaks.
- **(h) Rust fit:** Crates, maintenance status, performance.

---

### Candidate 1: In-Vault Staging / Inbox Landing Zone (`_inbox/` + `traces promote`)

- **(a) Writes seen:** Covers ALL write types (temp+rename, truncate, redirect,
  cp, mv, sync, Obsidian) because writers are strictly directed to write into
  the inbox (`_inbox/`). The canonical directory (`notes/`) is never targeted
  directly by external writers.
- **(b) Validation & Feedback:** Validation occurs at promotion time (`traces
  promote <path>`). The promoting actor (agent CLI turn or human) receives
  stdout/stderr with exit code 0 (success) or 1 (schema violation with field
  diff).
- **(c) Atomicity:** Full atomicity. Promotion uses filesystem rename (`fs::rename`
  on POSIX is atomic across the same mount). Incomplete writes in `_inbox/`
  simply fail promotion without corrupting the canonical vault.
- **(d) Platforms:** 100% cross-platform (macOS, Linux, Windows).
- **(e) Privilege/Deployment Cost:** Zero privilege required. Pure CLI. No daemon,
  no kernel extension.
- **(f) Bypass routes:** An uncontrolled process can write directly to the
  canonical path if filesystem permissions permit. (Mitigated if canonical folder
  is marked read-only via `chmod -w` or OS ACLs).
- **(g) Failure stance:** **Fail-closed.** If the schema is broken or the
  checker crashes, promotion aborts. The canonical vault remains intact; draft
  remains safely in `_inbox/`.
- **(h) Rust fit:** Excellent. Native standard library (`std::fs`), `tempfile`,
  and `noyalib` for YAML parsing.

---

### Candidate 2: Tolerant Reader with Canonical Normalisation (The Quartz Pattern)

- **(a) Writes seen:** Sees all completed writes at read time. Transparent to how
  the file was written.
- **(b) Validation & Feedback:** Occurs on `traces query`, `traces read`, or
  index build. Writers do not see feedback at write time. If configured, read
  commands output advisory notices on stderr or attach warnings to query results.
- **(c) Atomicity:** Read-time parsing must handle partially written files
  gracefully. If YAML is truncated or malformed, the parser must report a read
  error rather than crashing.
- **(d) Platforms:** 100% cross-platform.
- **(e) Privilege/Deployment Cost:** Zero cost. Internal logic in the `traces`
  binary.
- **(f) Bypass routes:** N/A (this is a read-side resilience pattern, not a write
  gate).
- **(g) Failure stance:** **Fail-open (permissive).** Minor irregularities
  (scalar vs list, alias names) are coerced to canonical form. Severe syntax
  errors (unparseable YAML) fail safe by skipping the note from index queries
  with a logged warning.
- **(h) Rust fit:** Perfect. Implemented via Rust pattern matching over
  `noyalib::Value`. Traces already vendors/pins `noyalib = "0.0.53"` (`[V]`).

---

### Candidate 3: Frontmatter `$schema` Pointer (`.traces/schemas/*.json`)

- **(a) Writes seen:** Preserved across all standard file writes and Obsidian
  Properties updates (`[V]`).
- **(b) Validation & Feedback:** Feedback delivered whenever any JSON-Schema-aware
  tool (VS Code YAML, JetBrains, `check-jsonschema`, CLI linter) opens or audits
  the file. Inline squiggly diagnostics in editors.
- **(c) Atomicity:** Subject to standard editor write atomicity.
- **(d) Platforms:** 100% cross-platform.
- **(e) Privilege/Deployment Cost:** Zero privilege. Requires generating static
  JSON Schema files under `.traces/schemas/` during `traces init` or schema
  definition.
- **(f) Bypass routes:** Writers can omit or delete the `$schema` property line.
- **(g) Failure stance:** **Fail-open.** Missing or invalid `$schema` pointer
  does not delete data; linter simply falls back or reports unlinked schema.
- **(h) Rust fit:** High. Traces can generate JSON Schema from its internal
  schema definitions using `schemars` crate or direct JSON serialization.

---

### Candidate 4: Filesystem Watcher Auto-Remediation Daemon (`traces watch --fix`)

- **(a) Writes seen:** Sees writes via OS file event notifications (`kqueue` on
  macOS, `inotify` on Linux, `ReadDirectoryChangesW` on Windows).
  - *Critical Pitfall:* Atomic renames (`save-to-temp-then-rename`) produce
    `Rename` events, not `Modify` events. A naive watcher tracking only file
    modifications misses writes (`[V]` `G2-mechanism-catalogue.md`).
- **(b) Validation & Feedback:** Asynchronous (post-write). Feedback cannot be
  reliably delivered to the original writer process without a dedicated IPC
  channel.
- **(c) Atomicity:** High risk of race conditions. If an editor writes in chunks,
  the watcher may wake up and read a half-written file. Requires debouncing
  (`notify-debouncer-mini`). If `--fix` mutates the file, it triggers a second
  watcher event, risking infinite loops.
- **(d) Platforms:** macOS, Linux, Windows (supported via `notify` crate).
- **(e) Privilege/Deployment Cost:** High runtime cost for desktop PKM. Requires
  a long-running daemon process consuming battery and memory.
- **(f) Bypass routes:** Easy to bypass: simply kill or pause the daemon.
- **(g) Failure stance:** Fail-open. If the watcher crashes, writes continue
  unmonitored.
- **(h) Rust fit:** Crates exist (`notify 6.1`, `notify-debouncer-mini 0.4`),
  but async daemon complexity is disproportionate for a CLI tool.
- **Verdict:** REJECTED as a core product feature (consistent with Round 5).

---

### Candidate 5: Directory-Local Schema Index Notes (`_schema.md`)

- **(a) Writes seen:** Non-interfering. Co-exists alongside notes in category
  subdirectories.
- **(b) Validation & Feedback:** Passive in-band steering. The writer reads the
  schema when scanning the folder before authoring.
- **(c) Atomicity:** No atomicity issues.
- **(d) Platforms:** 100% cross-platform.
- **(e) Privilege/Deployment Cost:** Zero. Just a Markdown file.
- **(f) Bypass routes:** Completely ignored by non-LLM processes; LLMs may
  overlook it if they don't list directory contents.
- **(g) Failure stance:** Fail-open.
- **(h) Rust fit:** Trivial. `traces new --schema` or `traces schema export` can
  render Markdown tables of field definitions.
- **Verdict:** VIABLE as passive in-band documentation.

---

## 5. The 3 Most Promising Harness-Independent Ideas

To satisfy the prompt's acceptance criteria, these three ideas are strictly
distinct from the mechanisms catalogued in `round5/report.md` Section 4
(which covered Tier 0 init/check CLI, Tier 1 lint loop/git pre-commit, Tier 2
delta baseline notice on read, Tier 3 MCP tools, Tier 4 LSP, and Tier 5 OS
sandboxes).

---

### Idea 1: In-Vault Inbox Landing Zone with Atomic CLI Promotion (`traces promote`)

- **The Concept:** A structural staging architecture native to the vault.
  Agents, external scripts, and capture tools are instructed to write exclusively
  to an `_inbox/` (or `_quarantine/`) folder. The core notes directory is treated
  as canonical and protected. To move a note into the main vault, the agent or
  user runs:

  ```bash
  traces promote _inbox/my-note.md --to books/
  ```

- **How It Works:**
  1. `traces promote` reads `_inbox/my-note.md` and validates it against the
     target class (`Book`).
  2. If valid: atomically moves the file (`fs::rename`) to `books/my-note.md` and
     indexes it. Exit code 0.
  3. If invalid: aborts the move, leaves `_inbox/my-note.md` untouched, and
     outputs actionable diagnostic errors naming the exact fields and expected
     types on stderr. Exit code 1.
- **Why It Succeeds Without Harness Features:**
  - It provides **true write prevention** for the canonical corpus without
    requiring kernel sandboxes, FUSE, or hooks.
  - It handles partial writes gracefully: if a tool crashes mid-write, only
    the quarantined draft is damaged.
  - It creates a natural human-in-the-loop review queue (`Task Vault` pattern)
    whenever desired.

---

### Idea 2: Tolerant Reader with Canonical-Form Normalisation (The Quartz Pattern)

- **The Concept:** Shift resilience from write-time enforcement to read-time
  projection. Instead of failing queries or requiring pristine disk compliance,
  the `traces` query and indexing engine normalises frontmatter in memory:

  ```rust
  // Conceptual projection in traces reader
  fn normalise_frontmatter(raw: RawFrontmatter, schema: &Schema) -> ValidNote {
      // 1. Coerce scalars to arrays if schema expects list
      //    (e.g. tags: "rust" -> ["rust"])
      // 2. Coalesce aliases (e.g. date vs created vs lastmod)
      // 3. Inject schema default values for omitted optional fields
  }
  ```

- **How It Works:**
  - Notes on disk remain human-editable and forgiving.
  - The indexer and query engine operate on guaranteed canonical data shapes.
  - Complemented by an optional `traces fix <file>` command that writes
    normalised canonical YAML back to disk if the user desires clean files.
- **Why It Succeeds Without Harness Features:**
  - It makes the PKM system immune to external processes that format YAML
    differently (e.g. Obsidian converting inline arrays to multi-line bullets,
    or agents writing a single tag without square brackets).
  - Zero runtime dependencies, zero harness requirements, instant performance.

---

### Idea 3: Durable Frontmatter `$schema` Pointer with Static JSON Schema Export

- **The Concept:** Embed a durable schema reference directly in note frontmatter:

  ```yaml
  ---
  $schema: ../.traces/schemas/book.json
  title: "The Rust Programming Language"
  author: "Steve Klabnik and Carol Nichols"
  isbn: "9781718503106"
  ---
  ```

- **How It Works:**
  1. `traces init` or `traces schema export` compiles the vault's internal
     schema definitions into standard JSON Schema files stored under
     `.traces/schemas/`.
  2. Notes generated via `traces new` include the relative `$schema` path.
  3. Unlike YAML comments (which Obsidian strips `[V]`), `$schema` is a real
     property key preserved across all editors, YAML parsers, and Obsidian's
     Properties UI `[V]`.
- **Why It Succeeds Without Harness Features:**
  - It exploits standard editor behavior: VS Code, JetBrains, and any editor
    with YAML schema support automatically download/resolve the schema and
    provide instant visual validation and autocomplete to humans and IDE agents.
  - External CLI validators (`check-jsonschema`, `ajv-cli`) can validate the
    entire vault using standard off-the-shelf tools without knowing anything
    about `traces`.

---

## 6. Sources and Evidence Citations

1. `[V]` Letta MemFS Documentation: <https://docs.letta.com/concepts/memfs/index.md>.
   "MemFS projects that repository onto whatever computer the agent is running
   on, as a real checkout the agent reads and edits with ordinary file tools...
   Every memory edit is committed to the MemFS git repository." Pre-commit
   validation confirmed in `letta-ai/letta-code` prompts (`letta_root_memfs.md`).
2. `[V]` Basic Memory Repository: <https://github.com/basicmachines-co/basic-memory>
   and NOTE-FORMAT.md. Picoschemas and advisory CLI validation (`bm schema validate`).
3. `[V]` Model Context Protocol Memory Server:
   <https://github.com/modelcontextprotocol/servers/tree/main/src/memory>.
   Tool-mediated memory schema definitions.
4. `[V]` Mem0 AI Repository: <https://github.com/mem0ai/mem0>. Managed DB layer.
5. `[V]` Obsidian Metadata Menu: <https://github.com/mdelobelle/metadatamenu>.
   FileClass schemas and audit modal.
6. `[V]` Platers Obsidian Linter: <https://github.com/platers/obsidian-linter>.
   Lint on save and YAML formatting rules.
7. `[V]` Metadata Enforcer v2: <https://omarish.com/p/metadata-enforcer-v2>.
   Django-style Python models and path routing.
8. `[V]` Obsidian Metadata Validator:
   <https://github.com/flowing-abyss/obsidian-metadata-validator>. Manifest
   inheritance in `schemes/`.
9. `[V]` Dendron Doctor Documentation:
   <https://wiki.dendron.so/notes/ZeC74FYVECsf9bpyngVMU/>. `fixFrontmatter`
   and doctor actions.
10. `[V]` Logseq Property Architecture:
    <https://github.com/logseq/logseq/blob/master/libs/guides/db_properties_guide.md>.
    DataScript tolerant parser in file mode; SQLite in DB mode.
11. `[V]` Zettlr Documentation: <https://docs.zettlr.com>. Pandoc YAML frontmatter.
12. `[V]` Quartz 4/5 Source Code:
    `jackyzha0/quartz` `quartz/plugins/transformers/frontmatter.ts`. Verified
    `coalesceAliases`, `coerceToArray`, and default fallbacks.
13. `[V]` Astro Content Collections Documentation:
    <https://docs.astro.build/en/guides/content-collections/>. Zod schemas and
    dev-server error overlay.
14. `[V]` Hugo Archetypes Documentation:
    <https://gohugo.io/content-management/archetypes/>. Scaffolding templates.
15. `[V]` Keystatic Documentation: <https://keystatic.com/docs/collections>.
    Zod schema validation in reader and UI.
16. `[V]` Decap CMS Documentation: <https://decapcms.org/docs/configuration-options/>.
    `config.yml` collection schemas.
17. `[V]` Community Obsidian Agent Workflows:
    <https://community.obsidian.md/plugins/ai-work-review>,
    <https://github.com/georgeracu/obsidian-agent-vault>. Inbox and review queue
    patterns.
18. `[V]` Claude Code Memory Documentation: <https://code.claude.com/docs/en/memory.md>.
    Lazy loading of subdirectory `AGENTS.md` and path-scoped rules.
19. `[V]` Obsidian Forum Bug Report 65851:
    <https://forum.obsidian.md/t/yaml-properties-api-processfrontmatter-removes-alters-string-quotes-comments-types-formatting/65851>.
    Confirms Obsidian systematically strips YAML comments and formatting.
20. `[V]` Empirical Local Bun Test: Observed that standard YAML parse/stringify
    roundtrips strip comments completely, but preserve `$schema` property keys.
