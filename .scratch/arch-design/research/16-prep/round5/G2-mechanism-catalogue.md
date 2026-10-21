# G2 Mechanism Catalogue: Frontmatter Schema Enforcement & Feedback

Deliverable:
`.scratch/arch-design/research/16-prep/round5/G2-mechanism-catalogue.md`  
Target: Ticket 16 (Schema Note Typing and Validation)  
Scope: Catalogue of every plausible mechanism to provide schema validation
feedback or enforcement when notes are edited or created by AI agents (or
human editors), without relying on agent-harness hooks.  
Evidence tags: `[V]` verified from primary source (spec, source code, docs,
empirical local test; URL/path cited); `[H]` hypothesis or architectural
inference; `[S]` secondary citation / scout claim not re-read.

---

## 1. Executive Summary & Mechanism Matrix

When AI agents or human editors modify markdown notes in a vault, schema
validation can operate at three distinct intervention points:

1. **Pre-write blocking (Prevention):** Intercepts the write before bytes hit
   the target file on disk; rejects the operation if schema violations exist.
2. **Post-write detection & feedback (Remediation loop):** Allows disk write,
   evaluates violations, and feeds structured diagnostics back to the agent or
   editor for iterative self-correction.
3. **By-construction validity (Scaffolding):** Guides note creation via
   templates, normalizers, or constrained schemas so invalid states are avoided
   or automatically rectified.

### Mechanism Comparison Matrix

| ID | Mechanism | Mode | Intervention Point | Harness Needs | Real Precedent / Reference |
|---|---|---|---|---|---|
| M01 | Validated MCP Edit/Write Tools | Hard Gate | Pre-write block | MCP client support | IWE (`iwec`), Obsidian Local REST API [V] |
| M02 | Direct CLI Wrapper / Staging Area | Hard Gate | Pre-write block | None (Tool/Shell) | Git staging index, `git commit` [V] |
| M03 | Agent Client Protocol (ACP) FS Interception | Hard Gate | Pre-write block | ACP client | ACP `fs/write_text_file` [V] |
| M04 | OS Sandbox / FS Access Controls | Hard Gate | Pre-write block | OS / Container | Linux Landlock, macOS sandbox-exec [V] |
| M05 | Git Pre-commit / Pre-push Hooks & CI | Hard Gate | Commit-time block | Git / VCS | `check-jsonschema`, `pre-commit` [V] |
| M06 | MCP Input Schema & Elicitation | Advisory / Constrained | Pre-write guide | MCP client support | MCP 2026-07-28 Spec, SEP-2106 [V] |
| M07 | LSP Diagnostics Server | Advisory | Post-write detect | LSP client support | IWE (`iwes`), Roo Code `getDiagnostics` [V] |
| M08 | Agent Lint Feedback Loop (`aider --lint`) | Remediation | Post-write loop | Harness CLI config | Aider `lint-cmd`, AgentLint [V] |
| M09 | Filesystem Watcher Daemon (`notify`) | Advisory | Post-write detect | Background process | Obsidian `MetadataCache.on('changed')` [V] |
| M10 | Vault Report File (`AGENTS.md` protocol) | Advisory | Post-write detect | None (File IO) | Letta MemFS, Vale SARIF output [V] |
| M11 | Validate on Read (Index Interception) | Advisory | Post-write (Next read) | None | Obsidian Properties, Wikidata constraints [V] |
| M12 | Template / Scaffold by Construction | Prevention | Creation-time | None (CLI tool) | Dendron schema templates, `traces new` [V] |
| M13 | Normalization / Auto-fix (`traces fix`) | Remediation | Post-write auto-fix | None (CLI tool) | Prettier, `markdownlint --fix`, Vale [V] |

---

## 2. Detailed Mechanism Analyses

### M01. Validated MCP Edit/Write Tools (`traces` as MCP Server)

- **How it works:** `traces` runs an MCP (Model Context Protocol) server. Rather
  than granting the agent arbitrary filesystem write access, the vault exposure
  provides domain-specific tools: `create_note`, `update_frontmatter`, and
  `patch_note`. Before committing any modification to disk, the tool invokes
  `check_note(before, after)`. If violations occur, the write is aborted, the
  file remains untouched, and the tool returns `isError: true` with a structured
  payload containing the field, error code, expected schema, and remediation
  hint [V: <https://github.com/iwe-org/iwe>, `crates/iwe/docs/schema.md`].
- **Intervention point:** Pre-write hard gate. Invalid content never touches
  the file system.
- **Harness prerequisites:** MCP client support (supported across Claude Code,
  Cursor, Cline, Roo Code, Zed, Windsurf, Neovim via mcphub).
- **Failure modes:**
  - If the agent has access to generic filesystem write tools (`write_file`,
    `bash` shell redirection, `edit`), the agent may bypass the MCP vault tools
    unless instructed or restricted.
  - Granular patch drift: multi-paragraph text edits might fail if the tool only
    accepts whole-file text or rigid frontmatter key-value dictionaries.
- **Precedent & Verification:**
  - `[V]` **IWE** (`iwe-org/iwe`): IWE exposes `iwec` (MCP client/server) that
    validates markdown documents against per-type schemas. When an agent attempts
    a schema-violating MCP write, the call is rejected with the specific violation
    named in the MCP tool response (<https://github.com/iwe-org/iwe>).
  - `[V]` **Obsidian Local REST API** (`coddingtonbear/obsidian-local-rest-api`):
    Provides an MCP interface (`vault_write`, `vault_patch`) supporting JSON
    patches and optimistic locking via ETags
    (<https://github.com/coddingtonbear/obsidian-local-rest-api>).

### M02. Copy-on-Write Staging Area & CLI Wrapper (`traces commit` / `stage`)

- **How it works:** Notes are kept in a protected directory or git-managed
  working tree. Edits by an agent take place in a designated draft or staging
  directory (or git index). To apply the changes to the primary vault, the agent
  must execute a CLI command (`traces commit` or `traces promote`). The command
  executes `check_note(before, after)` on all changed files; if any file
  violates its schema, the promotion is aborted.
- **Intervention point:** Pre-write hard gate relative to the canonical vault.
- **Harness prerequisites:** None. Operates via standard CLI or bash execution
  tools already available to coding agents.
- **Failure modes:**
  - Requires agent discipline or workflow structuring to write to the staging area
    rather than directly editing live notes.
  - Merging changes back to live storage can cause conflicts if multiple
    actors edit concurrently.
- **Precedent & Verification:**
  - `[V]` **Git Staging & Index**: Git separates the working tree from the
    staged index (`git add` then `git commit`).
  - `[V]` **Empirical Test**: Verified via local Python test that a gate running
    against staged content (`git show :file`) can inspect proposed changes
    without being tricked by dirty working-tree state.

### M03. Agent Client Protocol (ACP) Filesystem Interception

- **How it works:** The Agent Client Protocol (ACP) standardizes communication
  between IDEs/editors and coding agents. In ACP, file operations are mediated
  by the client via methods such as `fs/write_text_file` and `fs/read_text_file`
  [V: <https://agentclientprotocol.com/protocol/v1/file-system>]. A custom ACP
  client proxy or editor plugin intercepts `fs/write_text_file` calls targeting
  markdown files in the vault, evaluates `check_note`, and rejects or approves
  the RPC call.
- **Intervention point:** Pre-write hard gate.
- **Harness prerequisites:** ACP-compliant harness and client (e.g., Zed,
  JetBrains ACP connectors, Coder ACP Go/Rust SDKs).
- **Failure modes:**
  - ACP is an emerging protocol; widespread support outside of Zed and JetBrains
    is still developing compared to LSP and MCP.
  - Agents invoking sub-shells (terminal commands) bypass the ACP filesystem RPC
    methods entirely unless terminal access is sandboxed.
- **Precedent & Verification:**
  - `[V]` **ACP Specification v1**: ACP defines `fs/write_text_file` as an
    optional capability where clients track and mediate file modifications
    (<https://agentclientprotocol.com/protocol/v1/file-system>).

### M04. Operating System Sandbox & Kernel Restrictions (Landlock / sandbox-exec)

- **How it works:** The agent harness process runs under OS-level access control
  rules. On Linux, unprivileged processes invoke the Landlock LSM
  (`landlock_create_ruleset`) to enforce read-only access to the vault directory,
  allowing write access only to `/tmp` or through a dedicated Unix socket connected
  to a validating daemon [V: <https://man7.org/linux/man-pages/man7/landlock.7.html>].
  On macOS, `sandbox-exec` (Seatbelt) enforces a `.sb` profile denying
  `(file-write* (subpath "/path/to/vault"))` except via an authorized IPC shim.
- **Intervention point:** Pre-write hard gate (enforced by OS kernel).
- **Harness prerequisites:** OS privileges/kernel support; external sandbox
  wrapper around the agent binary (e.g., Docker, Bubblewrap, Sandlock, ai-jail).
- **Failure modes:**
  - Non-portable across operating systems (Linux Landlock vs macOS Seatbelt vs
    Windows AppContainer).
  - Heavy deployment footprint; difficult to require for typical desktop PKM users.
- **Precedent & Verification:**
  - `[V]` **Sandlock / ai-jail**: Sandlock (arXiv:2605.26298) and `ai-jail`
    demonstrate Landlock and `sandbox-exec` to restrict AI agent file access to
    read-only states on developer filesystems
    (<https://github.com/akitaonrails/ai-jail>).

### M05. Git Hooks (Pre-commit / Pre-push) & CI Workflows

- **How it works:** A `.git/hooks/pre-commit` script is installed in the vault
  repository. Whenever an agent or user invokes `git commit`, the hook runs:
  `traces check --staged` or `traces check --before <rev> --after <staged>`.
  If validation fails, the hook exits with non-zero status (1 or 2), aborting
  the commit.
- **Intervention point:** Pre-commit hard gate. (Post-disk write, but pre-VCS
  persistence).
- **Harness prerequisites:** Vault must be a git repository; agent must use git
  to record its work.
- **Failure modes:**
  - Hook inspection pitfall: naive pre-commit hooks check the file on the working
    tree (`cat note.md`), which fails if the agent partially staged changes
    (`git add -p`) or modified the working tree after staging [V]. The hook
    MUST inspect the index blob via `git show ":$file"`.
  - Agents can bypass hooks if they invoke `git commit --no-verify` or never
    commit changes to git.
- **Precedent & Verification:**
  - `[V]` **`markdownlint-cli2` / `check-jsonschema`**: Widely used in
    pre-commit frameworks to validate frontmatter and markdown structure
    (<https://github.com/DavidAnson/markdownlint-cli2>).
  - `[V]` **Empirical Verification**: Executed a test verifying that `git show
    ":note.md"` properly isolates staged content from unstaged working tree
    modifications, correctly aborting invalid staged commits while allowing valid
    staged commits even when the working tree has invalid edits.

### M06. MCP Input Schema, Elicitation & System Prompts

- **How it works:** Rather than checking frontmatter after arbitrary text
  generation, MCP tools declare structured `inputSchema` (JSON Schema 2020-12)
  for frontmatter fields. For ambiguous or class-altering changes, MCP
  **Elicitation** (`client/elicitation`, `ctx.mcpReq.elicitInput`) pauses
  execution to request user confirmation in-band [V:
  <https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation>].
- **Intervention point:** Pre-write guidance and interactive confirmation.
- **Harness prerequisites:** MCP client with elicitation support.
- **Failure modes:**
  - Complex custom note validation rules (e.g. cross-field constraints, file-path
    existence, wikilink validation) cannot be fully expressed in JSON Schema
    primitives alone and require server-side code validation.
- **Precedent & Verification:**
  - `[V]` **Model Context Protocol Specification**: Tools declare input schemas
    and interactive elicitation primitives
    (<https://modelcontextprotocol.io/specification/2026-07-28>).

### M07. Language Server Protocol (LSP) Diagnostics Server

- **How it works:** `traces` runs an LSP server (`traces lsp`). When files are
  opened or changed in the editor/harness (`textDocument/didChange`,
  `textDocument/didSave`), the server evaluates schemas and publishes
  `textDocument/publishDiagnostics` with precise line/column spans, severity
  levels (Error, Warning, Information, Hint), and remediation hints.
- **Intervention point:** Post-write advisory feedback.
- **Harness prerequisites:** Harness must support LSP client connections and
  relay diagnostics to the LLM context.
  - Harnesses supporting diagnostic relay: **Cursor** (reads LSP diagnostics
    into context), **Cline / Roo Code** (implements `getDiagnostics` to query
    problems and pass them to the model [V:
    <https://github.com/cline/cline/issues/5566>]), **Zed** (native LSP),
    **Continue.dev**.
- **Failure modes:**
  - LSP cannot prevent a save: `didChange` and `didSave` are notifications
    (one-way fire-and-forget). `willSaveWaitUntil` only allows formatting edits;
    it cannot cancel or reject a save operation.
  - Terminal-only agent harnesses (e.g., bare Claude Code CLI, Aider) do not run
    an LSP client by default without an intermediary bridge.
- **Precedent & Verification:**
  - `[V]` **IWE Language Server** (`iwes`): Validates documents against schemas
    and publishes diagnostics directly to LSP-enabled editors (<https://github.com/iwe-org/iwe>).
  - `[V]` **Roo Code / Cline**: Actively exposes `getDiagnostics` to collect
    diagnostics from language servers and auto-feed them to the LLM prompt.

### M08. Agent Lint / Self-Correction Loop (`aider --lint`, AgentLint)

- **How it works:** The agent harness executes a linter after every modification.
  If the command exits with non-zero status, the linter's stderr/stdout
  is automatically injected into the agent's next conversation turn as an error
  message with an explicit prompt to fix it (e.g., `traces check <file>`).
- **Intervention point:** Post-write automated remediation loop.
- **Harness prerequisites:** Harness configuration supporting auto-linting
  commands (e.g., Aider's `--lint-cmd`, Claude Code `/review` hooks, custom agent
  evaluation loops).
- **Failure modes:**
  - If the agent fails to converge or enters an edit loop, it can exhaust
    token/tool limits.
  - Requires the harness to have an automated retry mechanism on non-zero exit.
- **Precedent & Verification:**
  - `[V]` **Aider**: Aider implements `--lint-cmd` and auto-linting loops:
    "Aider can automatically lint and test your code every time it makes changes.
    This helps identify and repair any problems introduced by the AI edits"
    (<https://aider.chat/docs/usage/lint-test.html>).
  - `[V]` **AgentLint**: CI gating and local linter for agent workflows
    (<https://github.com/marketplace/actions/agentlint>).

### M09. Filesystem Watcher Daemon (`notify` crate / `fswatch`)

- **How it works:** A background daemon (`traces watch`) listens for filesystem
  events across the vault using OS notifications (kqueue on macOS, inotify on
  Linux, ReadDirectoryChangesW on Windows) via the Rust `notify` crate [V:
  <https://docs.rs/notify/latest/notify/>]. When a markdown file is created or
  modified, the daemon runs schema validation and writes a diagnostic summary or
  triggers an OS notification / local socket message.
- **Intervention point:** Post-write asynchronous advisory.
- **Harness prerequisites:** Background process execution; filesystem access.
- **Failure modes & Reliability Considerations:**
  - **Atomic renames:** Editors and agents frequently save files by writing to
    a temporary file (e.g. `note.md.tmp.1234`) and atomically renaming it over
    the destination (`rename()` syscall). A naive watcher tracking only file
    modifications misses writes; watchers MUST monitor directory events and
    track rename/create event pairs.
  - **Event coalescing & storms:** Rapid multi-chunk writes generate bursts of
    events. Watchers must debounce events using crates like `notify-debouncer-mini`
    to prevent redundant validations.
  - Asynchronous: the agent may have already moved on to reading or acting before
    the watcher finishes validation.
- **Precedent & Verification:**
  - `[V]` **Obsidian Vault & MetadataCache**: Obsidian listens to file system
    events using internal file watchers, emitting `vault.on('modify')` and
    `metadataCache.on('changed')` to re-parse frontmatter asynchronously
    (<https://docs.obsidian.md/Reference/TypeScript+API/MetadataCache/on('changed')>).
  - `[V]` **Rust `notify` crate**: Documents support for debouncing and cross-
    platform handling of file events (<https://docs.rs/notify/latest/notify/>).

### M10. Agent Instruction & Vault Report File (`AGENTS.md` / `CLAUDE.md`)

- **How it works:** The repository provides an instruction file (`AGENTS.md` or
  `CLAUDE.md`) instructing the agent to adhere to vault rules and read a live
  status report file (e.g. `.traces/problems.json` or `.traces/report.md`).
  After making note edits, the agent is instructed to run `traces check` or
  inspect `.traces/problems.json` before completing its task.
- **Intervention point:** Post-write advisory inspection guided by system prompt.
- **Harness prerequisites:** Agent must read repo instruction files (`AGENTS.md`,
  `CLAUDE.md`, or system prompts). Standard across modern coding agents.
- **Failure modes:**
  - Probabilistic compliance: agents may overlook system prompt instructions,
    skip running the check command, or hallucinate that the file is valid.
  - Cannot strictly block invalid writes.
- **Precedent & Verification:**
  - `[V]` **Claude Obsidian Skills & `CLAUDE.md`**: Widely used to instruct
    agents on frontmatter formatting and schema rules
    (<https://github.com/jykim/claude-obsidian-skills>).
  - `[V]` **Vale CLI / SARIF**: Vale outputs structured lint reports to JSON/SARIF
    files for consumption by other tools (<https://docs.vale.sh/topics/templates>).

### M11. Validate on Read (Index Interception)

- **How it works:** Validation is deferred until a file or index query is read.
  When an agent or tool invokes a search, list, or read command (`traces query`,
  `traces read <note>`), the index inspects the note's schema conformance.
  If violations exist, the read tool attaches an advisory diagnostic section
  to the returned content: `[WARNING: Note violates schema 'Book': field 'isbn'
  missing]`.
- **Intervention point:** Post-write detection surfaced at the subsequent read.
- **Harness prerequisites:** Agent must read notes through `traces` query/read
  APIs or MCP tools rather than raw filesystem reads.
- **Failure modes:**
  - Silent latency: errors remain invisible until someone or something reads
    the note.
  - If the agent finishes a session immediately after writing an invalid note,
    the feedback is never received during that session.
- **Precedent & Verification:**
  - `[V]` **Obsidian Properties UI**: Renders warnings next to invalid property
    fields when a note is opened in the editor view.
  - `[V]` **Wikidata Constraint Violations**: Evaluates semantic schema
    constraints on demand when inspecting entity items.

### M12. Template & Scaffold by Construction (`traces new`)

- **How it works:** Agents are instructed or configured to create new notes
  exclusively via scaffolding commands: `traces new --class <class> <title>`.
  The scaffolding command generates a pre-populated, syntactically valid
  frontmatter block containing all `required` fields with their default values
  and type-appropriate placeholders.
- **Intervention point:** Prevention by construction (creation time).
- **Harness prerequisites:** CLI execution or MCP tool exposure.
- **Failure modes:**
  - Only protects note creation; does not prevent subsequent destructive edits
    or invalid field additions.
- **Precedent & Verification:**
  - `[V]` **Dendron Schema Templates**: Dendron automatically applied template
    frontmatter matching schema patterns when creating notes via lookup
    (<https://github.com/dendronhq/dendron>).
  - `[V]` **IWE `iwe new`**: Proposed feature to instantiate valid document
    templates matching document type schemas (<https://github.com/iwe-org/iwe/issues/192>).

### M13. Normalization & Auto-fix Commands (`traces fix`)

- **How it works:** A deterministic normalization command (`traces fix <file>`)
  repairs common schema mistakes: coercing dates to standard format, reordering
  fields to match schema declarations, stripping unregistered keys if configured,
  and formatting scalar strings.
- **Intervention point:** Post-write automatic remediation.
- **Harness prerequisites:** CLI or MCP tool call.
- **Failure modes:**
  - Cannot auto-fix missing required values or ambiguous enum values without
    guessing or user interaction.
- **Precedent & Verification:**
  - `[V]` **`markdownlint --fix` / `prettier`**: Automatically transforms
    markdown and frontmatter syntax into canonical compliance.
  - `[V]` **Ruff (`ruff check --fix`)**: The standard for auto-fixing code
    and metadata lint errors in developer tooling.

---

## 3. Analysis & Architectural Recommendations for Traces

1. **Defense-in-Depth Model:** No single mechanism covers all harnesses and
   modalities. A complete harness-independent strategy requires layering:
   - **Layer 1 (The Primary Gate):** Expose validated MCP tools (`M01`) where
     `traces` owns the write path. This provides deterministic pre-write gating
     whenever an agent interacts via MCP.
   - **Layer 2 (The CLI & Git Gate):** Provide `traces check --staged` and git
     pre-commit hooks (`M05`) for VCS-based vaults. This catches direct file
     writes before commits occur.
   - **Layer 3 (The Feedback Loop):** Expose `traces check` as an exit-code-
     compliant linter suitable for agent lint loops (`M08`, like `aider --lint`)
     and publish advisory diagnostics via LSP (`M07`) for IDE-based agents
     (Cursor, Roo Code, Zed).
   - **Layer 4 (Creation by Construction):** Provide `traces new` (`M12`) to
     ensure notes begin life conforming to their bound schema.

2. **Delta Rule & Self-Correction:**
   - As established in Ticket 16's Answer, validation errors must provide clear
     `hint` data (taken from field descriptions) and report `(field, code,
     expected, found)` to enable LLM self-correction loops (`M08`).
   - The delta rule ensures an agent editing an existing legacy note with pre-
     existing violations is not blocked from making valid progress unless its
     own edit introduced new violations.
