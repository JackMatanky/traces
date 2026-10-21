# Scout C: Agent Schema Guardrails and Editing Constraints

## 1. Sub-problem Restated

Ticket 20 addresses whether and how schemas validate note data. This
sub-problem (C) specifically addresses how an autonomous agent—whether
operating through Model Context Protocol (MCP) tools or as a coding-agent
harness (Claude Code, oh-my-pi/omp, Cursor, Roo Code, Aider)—must be
constrained by schemas when modifying markdown notes.

The user's latest steer (round 3) poses a specific architectural hypothesis:
> "Edits could just be done by the agent directly on files, bound by the LSP,
> which already has diagnostics from the schemas; changing a note's class is
> denied or requires asking the user; a delta rule: only newly introduced
> violations block."

This inquiry requires stress-testing that hypothesis and resolving six
technical questions:

1. **LSP Protocol Semantics**: Can a Language Server Protocol (LSP) server
   actually "bind" or block an agent's write? How do current coding agents
   consume LSP diagnostics, and what happens when diagnostics are advisory
   warnings?
2. **Write-Blocking Enforcement**: What architectural mechanisms (harness
   pre-tool hooks, permission engines, filesystem sandboxes) can physically
   prevent or reject an invalid disk write?
3. **MCP Tool-Based Editing**: How do specialized MCP servers (Filesystem MCP,
   Obsidian MCP, Basic Memory, Notion) validate frontmatter, handle errors via
   `isError: true`, and leverage MCP elicitation for user confirmations?
4. **LSP Structural Limits**: Diagnostics are push/pull;
   `textDocument/didChange`
   is a notification that cannot reject edits; `workspace/willSaveWaitUntil`
   cannot deny saves. How can a "delta rule" or "class-change denial" be
   expressed if LSP cannot enforce them?
5. **CLI / Hook Integration**: How would a `traces check <files> --format json`
   engine operate across git hooks, harness hooks, and MCP servers?
6. **Pre-Write Schema Projection**: Can schema constraints be delivered to
   the model *before* it generates text (via MCP resources, system prompts, or
   tool `inputSchema` JSON Schema projections)?

---

## 2. What Traces Does Today

Traces currently has no active validation or agent-gating infrastructure:

- `src/schema/model.rs:11-28`: `Schema` represents a resolved schema definition
  with fields, `extends` ancestors, and child relations [V]. It contains no
  validation methods.
- `src/schema/fields.rs:52-57`: `SchemaFieldDef` stores `kind: SchemaFieldType`,
  `required: bool`, `multi: bool` [V]. The options `min`, `max`, `step`, and
  `format` are parsed into data structures but completely unread in production
  logic (`brief.md:39-41` [V]).
- `docs/adr/0006-file-class-schemas.md:110-111`: Explicitly notes that
  `required` and `multi` "are declared now but inert until the deferred MCP
  guardrail stage; LSP and MCP stages are deferred to later phases" [V].
- `src/note/metadata.rs:95-98`: Frontmatter stores
  `IndexMap<FieldKey, NoteFieldValue>` with zero byte spans [V].
- `src/index/service.rs:1732-1923`: The indexer parses `class` as an arbitrary
  string and expands class hierarchy at query time without schema validation;
  modifying schemas does not invalidate index fingerprints (ADR-0009:76-77 [V]).
- `src/cli/`: No `traces check` or `traces lint` command exists [V].
- External agent tooling: Traces has no built-in MCP server, no LSP server,
  and no hook runner under `src/` [V].

---

## 3. Precedents: Real Systems and Observed Failures

### 3.1 Claude Code (Anthropic)

- **LSP Integration** [V, code.claude.com/docs/en/plugins/code-intelligence]:
  Claude Code supports official LSP plugins (`typescript-lsp`, `pyright-lsp`,
  `rust-analyzer-lsp`). When Claude edits a file, diagnostics are gathered
  *after* the edit is written to disk:
  `Found N new diagnostic issues in M files (ctrl+o to expand)`.
  The LSP tool itself is strictly read-only navigation (definitions,
  references, symbols).
- **Execution Lifecycle & Hooks** [V, code.claude.com/docs/en/hooks]:
  Claude Code provides `PreToolUse` and `PostToolUse` hooks. A `PreToolUse`
  hook matching `Edit` or `Write` receives `tool_input` on stdin and can:
  (a) Exit with code 2 to block the tool call;
  (b) Return JSON `hookSpecificOutput: { permissionDecision: "deny",
  permissionDecisionReason: "..." }`;
  (c) Return `permissionDecision: "ask"` to halt and prompt the user.
- **Observed Failures / Regrets**: Anthropic's documentation emphasizes that
  post-edit diagnostics are advisory feedback loops, not guardrails. If a model
  is in an autonomous loop, advisory diagnostics often lead to multi-turn
  "thrashing" (repeated failed edits attempting to fix linter errors) or are
  deprioritized when the model believes its task is complete. Hard write
  prevention required introducing the `PreToolUse` hook subsystem.

### 3.2 Cursor Agent Harness

- **Model** [V, zenml.io/llmops-database/
  engineering-and-optimizing-an-agent-harness-
  for-production-ai-coding-assistants]:
  Cursor's 2026 harness case study documents the evolution of their coding
  agent. In 2024, Cursor automatically surfaced lint and type diagnostics after
  every edit.
- **Observed Failures / Regrets**: Cursor identified "context rot": accumulated
  tool errors and post-edit lint warnings degrade model reasoning over long
  sessions. Even when an agent can self-correct, errors remaining in context
  waste tokens and increase subsequent error rates. Passive post-edit feedback
  without upfront constraints resulted in high error rates, leading Cursor to
  invest in tight harness guardrails and dynamic context pruning.

### 3.3 Roo Code / Cline (VS Code Extension Agents)

- **Diagnostics Integration** [V,
  roocodeinc.github.io/Roo-Code/features/diagnostics-integration]:
  Roo Code captures VS Code diagnostics before editing, waits for updates after
  editing, and diffs them to identify newly introduced problems (delta rule).
- **Severity Filtering & Warning Behavior** [V]:
  Roo Code explicitly filters out warnings in auto-detection:
  *Error (level 0)* is included; *Warning (level 1)*, *Information (2)*, and
  *Hint (3)* are **excluded**. Roo Code documents the rationale:
  "Only includes errors to avoid interrupting your workflow with non-critical
  issues."
- **Observed Failures / Regrets**: Because warnings are stripped to prevent
  agent distraction, deprecation notices, schema field warnings, and soft
  constraints
  are completely ignored by the agent. Furthermore, because edits occur directly
  in VS Code text buffers before diagnostics are evaluated, syntax breakage
  leaves the workspace in an unbuildable state if the agent exhausts its turns.

### 3.4 Aider (Aider-AI)

- **Model** [V,
  github.com/Aider-AI/aider/blob/main/aider/website/docs/usage/lint-test.md]:
  Aider runs an automated `--lint-cmd` and `--test-cmd` loop after every file
  edit. If the command exits non-zero, Aider feeds stdout/stderr back into the
  LLM prompt to attempt a repair.
- **Observed Failures / Regrets**: Aider's documentation explicitly warns about
  code formatters: tools returning non-zero status on style changes confuse the
  agent into hallucinating errors. Crucially, edits are already committed to
  disk; if repair attempts fail, user intervention via `git reset` is necessary.

### 3.5 Model Context Protocol (MCP 2025-11-25) & Domain Servers

- **Tool Error Standard** [V,
  modelcontextprotocol.io/specification/2025-11-25/server/tools]:
  MCP tools distinguish between JSON-RPC Protocol Errors and Tool Execution
  Errors. Tool execution errors are returned inside the normal result payload
  with `isError: true` and descriptive text in `content`. When `isError: true`
  is returned, the model is informed of failure and can self-correct. Crucially,
  the server does *not* execute the write when it returns `isError: true`.
- **Elicitation Standard** [V,
  modelcontextprotocol.io/specification/2025-11-25/client/elicitation]:
  Introduced in the 2025-11-25 specification, `elicitation/create` allows an MCP
  server to prompt the user for input or confirmation mid-tool-execution
  via Form
  Mode or URL Mode. If a tool requires user confirmation to change a note class,
  the server can elicit confirmation dynamically. If the user declines, the tool
  aborts and returns `isError: true` without touching disk.
- **Observed Failures / Regrets**: Generic MCP servers like
  `@modelcontextprotocol/server-filesystem`
  allow unrestricted raw writes. Markdown notes edited via generic filesystem
  tools frequently suffer from YAML truncation, unquoted wikilink syntax errors,
  and schema corruption because the filesystem server has no domain awareness.

### 3.6 Oh-My-Pi (omp) Runtime Hooks

- **Model** [V, github.com/can1357/oh-my-pi/blob/main/docs/hooks.md]:
  `pi.on("tool_call")` runs before tool execution. If any handler returns
  `{ block: true, reason: "..." }`, tool execution halts immediately and throws
  a blocking error to the model. `tool_call` can also return `additionalContext`
  or mutate `input`. Furthermore, `HookUIContext` exposes interactive user
  prompts (`confirm`, `select`, `input`).
- **Observed Failures / Regrets**: Legacy post-execution hooks (`tool_result`)
  could not undo disk mutations if a write failed validation, proving that
  safety invariants must be enforced at the pre-execution boundary.

---

## 4. Stress-Testing the User's Proposal

The user proposes:
> "the agent edits files directly but is bound by the LSP, which already has
> diagnostics from the schemas; changing a note's class is denied or needs user
> confirmation; a delta rule: only newly introduced violations block."

An honest, rigorous engineering evaluation reveals that this proposal relies
on an architectural fallacy: **LSP is structurally incapable of binding an
agent or denying a write.**

### 4.1 Structural Reality of the LSP Protocol

1. **LSP is an Observational Notification Stream, Not a Firewall** [V]:
   Under LSP 3.17, document updates are sent from the client to the server via
   `textDocument/didChange`. In JSON-RPC, `didChange` is a **Notification**
   (it has no `id` and expects no response) [V]. The server cannot reject, deny,
   or fail a `didChange` message. The edit has already happened in the client
   buffer.
2. **`willSaveWaitUntil` Cannot Deny Saves** [V]:
   The only pre-save hook in LSP is `textDocument/willSaveWaitUntil`. Its
   protocol response type is `TextEdit[] | null` [V]. It allows a server to
   format text or organize imports before saving; it *cannot* reject the save,
   throw an abort exception, or prevent the file from landing on disk.
3. **Diagnostics Are Purely Advisory** [V]:
   `textDocument/publishDiagnostics` emits diagnostics with severities:
   Error (1), Warning (2), Information (3), Hint (4). An LSP server has no
   mechanism to enforce that a client, IDE, or agent reacts to these
   diagnostics.
4. **LSP Cannot Implement a Delta Rule Without Corrupting the Editor** [V]:
   LSP diagnostics represent the complete diagnostic state of a document
   version. If an LSP server suppresses legacy violations to implement an agent
   "delta rule", human users opening the file in VS Code or Zed would see
   pre-existing frontmatter errors vanish from their Problems panel!
   Conversely, if the LSP server emits all diagnostics, the LSP protocol has no
   standard field to designate a diagnostic as "newly introduced in this turn".
   Every system implementing a delta rule (such as Roo Code [V]) computes the
   diff in the *client agent harness*, not inside the language server.
5. **LSP Cannot Block Class Changes or Elicit Confirmation** [V]:
   When an agent writes `class: project` over `class: task`, the LSP server
   receives a notification after the fact. It cannot revert the file, cannot
   block the save, and cannot pause the agent's turn to elicit modal user
   confirmation.

### 4.2 Do Agents Ignore Advisory Warnings?

Empirical evidence from modern agent harnesses answers: **Yes.**

- Roo Code's automated post-edit error loop explicitly **drops all warnings**
  from auto-detection to prevent infinite agent loops and distraction [V].
- When warnings are fed to LLMs in conversational context without a hard tool
  failure, models frequently rationalize them as "acceptable formatting" or
  "pre-existing linter warnings" and conclude their task without fixing them.
- If schema constraints (e.g. missing required field, type mismatch) are
  emitted only as advisory LSP warnings, direct-editing agents will
  consistently leave dirty, invalid notes on disk.

**Verdict on User Proposal**: "Bound by the LSP" is physically non-binding.
LSP diagnostics are a vital developer feedback mechanism, but they cannot
serve as an enforcement guardrail for autonomous agents editing files directly.

---

## 5. Candidate Solutions

| Candidate | Mechanism | Where Enforced | Blocks Write? | Delta Rule Support | Class Change Gating |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1. LSP-Only Advisory (User's Proposal)** | `publishDiagnostics` | LSP Client/IDE | No (advisory feedback only) | No (pollutes human editor if filtered) | No (cannot intercept or pause) |
| **2. LSP + Blocking Harness Hook** | Pre-tool hook + `traces check` | Harness (`PreToolUse`, `tool_call`) | Yes (aborts tool before write) | Yes (diffs candidate vs disk) | Yes (`ask` user or `deny`) |
| **3. MCP Domain Tools (`isError` + Elicitation)** | Dedicated note editing tools | Traces MCP Server | Yes (aborts before file write) | Yes (evaluates before write) | Yes (`elicitation/create` prompt) |
| **4. Pre-Commit / CI Check** | Git hook / `mise run verify` | Git / CI pipeline | Delayed (blocks git commit, not edit) | Yes (git diff vs HEAD) | Yes (git diff inspection) |
| **5. Pre-Write Schema Projection** | MCP resources + JSON Schema `inputSchema` | Model Inference Boundary | Soft / Constrained | N/A (prevents error generation) | Soft (prompts model not to change) |
| **6. Git-Level Transactional Revert** | Optimistic edit + post-tool git rollback | Harness / Wrapper Script | Yes (reverts dirty working tree) | Yes (git diff against baseline) | Yes (reverts if class changed) |
| **7. Shadow-Staging Virtual Buffer (Unconventional)** | Two-phase commit in scratch worktree | Traces VFS / Staging Proxy | Yes (promotes only on valid check) | Yes (staged diff evaluation) | Yes (gated promotion) |

### 5.1 Candidate 1: LSP-Only Advisory Feedback (Keep-as-is / User Baseline)

- **Description**: The planned Traces LSP server publishes schema diagnostics
  over `textDocument/publishDiagnostics`. Agents edit files directly using
  generic `Edit`/`Write` tools. Diagnostics appear in the agent's context
  window on subsequent turns.
- **Checker Requirements**: `check_note(note: &Note) -> Vec<Violation>`.

### 5.2 Candidate 2: LSP Diagnostics + Blocking Harness Pre-Tool Hook

- **Description**: The LSP provides developer diagnostics for human editing.
  For agents, a harness hook (e.g. Claude Code `.claude/hooks/PreToolUse` or
  omp `.omp/hooks/pre/`) intercepts `Edit` and `Write` calls targeting note
  files. The hook executes `traces check --stdin --path <file>` (or evaluates
  candidate frontmatter via a fast IPC socket).
- If a note's `class` field is modified, the hook returns
  `permissionDecision: "ask"` (prompting the user) or `"deny"`.
- If newly introduced schema violations exist (delta rule), the hook exits 2
  or returns `permissionDecision: "deny"`, preventing the file write.
- **Checker Requirements**:
  `check_delta(baseline: Option<&Note>, candidate: &Note) -> ValidationDelta`.

### 5.3 Candidate 3: MCP Domain Tools with `isError: true` and Elicitation

- **Description**: The agent does *not* edit raw markdown files using generic
  filesystem tools. Instead, Traces provides an MCP server exposing domain
  tools: `traces_update_note`, `traces_set_field`, `traces_create_note`.
- When called, the tool validates proposed values against `SchemaService`.
- If validation fails, the tool aborts and returns
  `{ isError: true, content: [...] }`.
  The file on disk is never touched.
- If `class` is changed, the server uses MCP Form Mode Elicitation
  (`elicitation/create`) to prompt the user: `"Change class from task to
  project? [Confirm/Deny]"`. If declined, returns `isError: true`.
- **Checker Requirements**:
  `validate_fields(schema: &Schema,
   fields: &IndexMap<FieldKey, NoteFieldValue>) -> Vec<Violation>`.

### 5.4 Candidate 4: CLI Check in Pre-Commit and CI Only

- **Description**: Agents edit files completely unconstrained. A git pre-commit
  hook (via `mise` or `lefthook`) runs
  `traces check --staged --deny-class-change`.
- If violations or class changes exist, `git commit` fails.
- **Checker Requirements**:
  `traces check <files> --against HEAD --format json`.

### 5.5 Candidate 5: Pre-Write Schema Projection (MCP Resource + InputSchema)

- **Description**: Schema constraints are projected into the agent's context
  *before* it writes.
  (a) Active schemas are exposed as MCP Resources (`traces://schemas/{class}`).
  (b) Note-editing tools expose dynamic JSON Schema `inputSchema` generated from
  the TOML schema (e.g. field types, enums for `select`, required properties).
- The LLM's tool calling is constrained at generation time by structured
  outputs.
- **Checker Requirements**:
  `export_json_schema(schema: &Schema) -> serde_json::Value`.

### 5.6 Candidate 6: Git-Level Transactional Revert (Optimistic + Rollback)

- **Description**: The agent writes files directly. A post-write harness hook
  or wrapper runs `traces check <file>`. If the check fails or violates delta
  rules, the hook runs `git checkout -- <file>` to revert disk state immediately
  and reports the error to the agent.
- **Checker Requirements**:
  `traces check <file> --against HEAD`.

### 5.7 Candidate 7: Shadow-Staging Virtual Buffer (Unconventional)

- **Description**: Two-phase commit. Agents write to a virtual workspace or
  hidden scratch worktree (`.traces/staging/`). An automated daemon checks the
  staged files. If valid (and approved), changes are promoted to the main
  vault. If invalid, the transaction is aborted.
- **Checker Requirements**:
  `traces transaction check <staging-id>`.

---

## 6. Strongest Arguments Against Each Candidate

### Against Candidate 1 (LSP-Only Advisory)

- **Fatal Flaw**: It does not work. LSP notifications cannot block file
  writes. Agents consistently ignore or misinterpret advisory warnings, leaving
  corrupted YAML, invalid dates, and broken required fields across the user's
  vault. It provides zero security boundary and zero guarantee of data
  integrity.

### Against Candidate 2 (LSP + Blocking Harness Hook)

- **Harness Fragmentation**: Every coding agent has a different hook mechanism:
  Claude Code has `PreToolUse` in `.claude/settings.json`; omp has `tool_call`
  in `.omp/hooks/`; Cursor, Zed, and Aider have no standardized pre-tool write
  blocking hook API. Maintaining hooks across disparate agent runners creates
  ongoing packaging and portability overhead.
- **Partial Edit Inspection**: In `Edit` tools (patch/diff tools), the hook
  receives a string diff or search-and-replace block, not the full candidate
  file. The hook must reconstruct the candidate buffer in memory before running
  `traces check`, which is computationally non-trivial for complex patches.

### Against Candidate 3 (MCP Domain Tools)

- **Agent Bypasses**: If the agent has access to raw shell (`bash`) or generic
  filesystem tools (`Write`, `Edit`, `write_file`), an LLM may choose to bypass
  `traces_update_note` and edit the file directly unless the harness strictly
  restricts available tools.

### Against Candidate 4 (Pre-Commit / CI Only)

- **Too Late in the Feedback Loop**: Pre-commit catches errors only when the
  agent attempts to commit. If an agent executes an 8-turn editing task, it
  discovers on turn 8 that turn 2 broke a schema constraint. Backtracking and
  reverting across 7 intervening steps causes massive context rot and frequently
  fails.

### Against Candidate 5 (Pre-Write Schema Projection Only)

- **Schema Expressiveness Gap**: TOML schemas support advanced PKM constraints
  (e.g. `file` fields filtering by folder/class, external select files
  with 3,734 values, wikilink format validation) that cannot be
  cleanly expressed inside a standard JSON Schema without blowing past
  LLM token context limits.

### Against Candidate 6 (Git Transactional Revert)

- **Working Tree Thrashing**: Reverting via `git checkout` discards all edits
  in the file, including valid changes to body text that accompanied the invalid
  frontmatter change. The agent loses its valid prose work and gets frustrated
  by full-file rollbacks.

### Against Candidate 7 (Shadow-Staging Virtual Buffer)

- **Extreme Complexity**: Imposes heavy filesystem virtualization, git
  worktree management, and synchronization overhead for what should be a fast
  local PKM note update.

---

## 7. Ranked Recommendation: Layered Defense-in-Depth

No single mechanism satisfies all constraints across all agent environments.
The optimal architecture is **Layered Defense-in-Depth**, combining upfront
guidance, domain tool gating, harness interception, and terminal CI
verification.

```text
               Layer 1: Upfront Guidance (Prevent Generation)
               MCP Resources + Schema Prompts + JSON Schema
                                   │
                                   ▼
        ┌─────────────────────────────────────────────────────┐
        │                 Agent Execution Path                │
        └──────────────────────────┬──────────────────────────┘
                                   │
          ┌────────────────────────┴─────────────────────────┐
          ▼                                                  ▼
[MCP Domain Tool Path]                            [Direct File Edit Path]
Layer 2: MCP Tool Gating                          Layer 3: Harness Hook Gate
- traces_update_note                              - PreToolUse / tool_call
- validate_fields()                               - traces check --stdin
- isError: true on failure                        - Deny/Ask on class change
- Elicitation for class change                    - Delta rule evaluation
          │                                                  │
          ▼                                                  ▼
──────────────────────────────────────────────────────────────────────────
                             DISK WRITE / VAULT
──────────────────────────────────────────────────────────────────────────
                                   │
                                   ▼
                Layer 4: Advisory LSP Diagnostics
                - Real-time Problems panel for human & post-edit agent
                - Full document health (no artificial suppression)
                                   │
                                   ▼
                Layer 5: Pre-Commit & CI Verification
                - traces check --staged (mise run verify)
                - Final non-bypassable git boundary
```

### Ranking of Strategies

1. **Primary Agent Boundary: Layer 2 (MCP Domain Tools + Elicitation)**
   *Rationale*: When an agent operates via MCP, domain tools
   (`traces_update_note`)
   provide the cleanest, most robust guardrail. The server validates before
   touching disk, returns actionable `isError: true` messages for
   self-correction,
   and natively leverages MCP Elicitation (`elicitation/create`) to prompt the
   user when a note class changes.
2. **Direct-Editing Guardrail: Layer 3 (Harness Hooks calling `traces check`)**
   *Rationale*: For coding agents (Claude Code, omp) that insist on direct file
   editing via `Edit`/`Write`, a standardized `PreToolUse`/`tool_call` hook
   calling `traces check --stdin --path <file>` provides the only mechanism that
   can physically intercept and block a write before disk corruption occurs.
3. **Interactive Developer Layer: Layer 4 (LSP Server)**
   *Rationale*: The LSP server remains essential, but its role is correctly
   scoped to **advisory developer intelligence** (syntax highlighting, hover,
   completion, diagnostics in VS Code/Zed). It should report *all* current
   violations without trying to act as an enforcement proxy or hiding legacy
   errors.
4. **Upfront Prevention: Layer 1 (Schema as MCP Resource & Prompt Context)**
   *Rationale*: Providing schema definitions in context minimizes the likelihood
   that the agent generates invalid frontmatter in the first place.
5. **Safety Net: Layer 5 (Git Pre-Commit Hook)**
   *Rationale*: Ensures broken notes never enter version history, catching any
   bypasses from unhooked terminal agents.

### What Evidence Would Change This Ranking?

- If the LSP working group standardized an LSP request method for pre-save
  validation with denial capability (e.g. `textDocument/willSaveCanDeny`),
  Layer 3 could move into the LSP client.
- If major coding agent harnesses standardized on a universal filesystem
  proxy rather than bespoke hook configs, the hook layer could be simplified.

---

## 8. Requirements for the Core Checker API (Sub-problems A & E)

To support this layered architecture without duplicating validation logic, the
core validation kernel designed in Sub-problems A and E must satisfy these
contractual requirements:

### 8.1 API Shape

The core validation engine must expose three evaluation tiers:

1. **In-Memory Field Map Validator (for MCP Tools & Post-Render Templates)**:

   ```rust
   pub fn validate_fields(
       schema: &Schema,
       fields: &IndexMap<FieldKey, NoteFieldValue>,
   ) -> Vec<ValidationViolation>;
   ```

   Pure function; zero span dependency; runs in <10 microseconds.

2. **Full Note Validator (for LSP & CLI)**:

   ```rust
   pub fn validate_note(
       schemas: &SchemaService,
       note: &Note,
   ) -> Vec<ValidationViolation>;
   ```

   Parses frontmatter, resolves effective schema, returns structured violations.

3. **Delta / Diff Validator (for Harness Hooks & Delta Rules)**:

   ```rust
   pub fn check_delta(
       schemas: &SchemaService,
       baseline: Option<&Note>,
       candidate: &Note,
   ) -> ValidationDelta;
   ```

   Where `ValidationDelta` provides:

   ```rust
   pub struct ValidationDelta {
       pub class_change: Option<ClassChange>,
       pub new_violations: Vec<ValidationViolation>,
       pub retained_violations: Vec<ValidationViolation>,
       pub fixed_violations: Vec<ValidationViolation>,
   }

   pub struct ClassChange {
       pub from: Option<String>,
       pub to: Option<String>,
   }
   ```

### 8.2 Structured Violations

`ValidationViolation` must be structured (machine-actionable enum), not a
rendered string, so MCP tools can serialize it to JSON and LSP adapters can
map it to `Diagnostic` codes:

- `field: FieldKey`
- `kind: ViolationKind` (`MissingRequired`, `TypeMismatch`,
  `InvalidSelectValue`,
  `OutOfRange`, `InvalidDateFormat`, `ClassChangeDenied`)
- `expected: ExpectedConstraint` (with bounded enum representations for large
  external select lists)
- `found: FoundValueSummary`

### 8.3 CLI Contract

A fast CLI subcommand for hooks:

```text
traces check [--stdin] [--path <file>] [--against <git-ref>]
             [--deny-class-change] [--format json]
```

Exits `0` on clean/accepted delta, exits `2` on blocking violation with JSON
diagnostic payload on stdout.

---

## 9. Lessons Transferable to Other Sub-Problems

1. **Keep the Kernel Span-Free and Transport-Agnostic (Sub-problem A)**:
   The validation kernel must not know about LSP `Range`, MCP `isError`, or CLI
   `miette` diagnostics. It should only evaluate
   `(&Schema, &IndexMap<FieldKey, NoteFieldValue>) -> Vec<ValidationViolation>`.
   LSP and MCP adapters handle protocol mapping.
2. **Severity Belongs to the Presentation Envelope (Sub-problem E)**:
   A violation is a factual discrepancy (`field X expected date, found string`).
   Whether that violation is an advisory LSP warning, a blocking MCP tool error,
   or an ignored legacy issue is determined by the host policy, not the
   validation
   primitive.
3. **Bounded Expected Values Are Critical for Agents**:
   When validating external select fields with thousands of entries
   (e.g. `job_titles.json` with 3,734 values), the violation object must
   summarize or truncate the expected list, otherwise agent context windows
   will be swamped by error serialization.
4. **Quote Sensitivity & Coercion (Sub-problem B)**:
   Agents writing frontmatter often output unquoted strings or unexpected YAML
   scalar types. The coercion and comparison policy (Ticket 19) must remain
   permissive and quote-insensitive so that syntactically valid edits are not
   falsely blocked.
