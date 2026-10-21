# Agent & MCP Vault Tools: Frontmatter Guardrails & Schema Enforcement

Deliverable: `round4/F-agent-and-mcp-vault-tools.md`  
Target: Ticket 16 (Schema Note Typing and Validation)  
Scope: Tools, specifications, and empirical precedent for LLM agents reading
and writing markdown notes, knowledge bases, and structured frontmatter.  
Evidence tags: `[V]` verified from primary documentation, source code,
changelog, or research paper (with URL); `[H]` hypothesis or architectural
inference based on domain patterns.

---

## 1. Scope

This report examines 13 distinct real-world projects, protocols, and empirical
benchmarks that address how software systems and AI agents read, write,
validate, and evolve structured data in markdown notes and knowledge bases:

1. **Model Context Protocol (MCP) Spec & Tool Annotations** (2024-2026):
   The protocol governing AI tool invocation, `inputSchema`, `outputSchema`,
   `ToolAnnotations` (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
   `openWorldHint`), `roots`, `resources`, `prompts`, and `elicitation`.
   Examined via official specification, schema repository, and maintainer blog
   posts [V: <https://modelcontextprotocol.io/specification/2026-07-28>,
   <https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations>].
2. **Anthropic Basic Memory Server** (`@modelcontextprotocol/server-memory`):
   The reference MCP knowledge graph memory server managing entities,
   observations, and relations. Examined via TypeScript implementation source,
   Zod schemas, concurrency locking, and atomic file persistence [V:
   <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts>].
3. **Obsidian Local REST API & Built-in MCP Server**
   (`coddingtonbear/obsidian-local-rest-api`):
   Local REST API and native MCP server for Obsidian vaults providing
   surgical section and frontmatter patching, optimistic concurrency with
   `If-Match` ETags, binary write guards, and OpenAPI schema exposure as an
   MCP resource [V: https://github.com/coddingtonbear/obsidian-local-rest-api].
4. **Third-Party Obsidian MCP Servers & CLIs** (`MarkusPfundstein/mcp-obsidian`,
   `Wicked-Evolutions/mcp-obsidian`, `Yakitrak/obsidian-cli`):
   Ecosystem implementations bridging agents to vault markdown files. Examined
   via GitHub repositories, tool schemas, and issue trackers
   [V: <https://github.com/MarkusPfundstein/mcp-obsidian>,
   <https://github.com/Yakitrak/obsidian-cli>].
5. **Anthropic Native Memory Tool** (`memory_20250818`): Claude Platform API
   native memory capability for client-side stateful file operations under
   `/memories`. Examined via official Anthropic Claude API documentation and SDK
   tool runners [V:
   <https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/memory-tool>].
6. **Letta / MemGPT & MemFS** (`letta-ai/letta`): Evolution of agent memory
   architectures from character-bounded in-context blocks (`persona`, `human`)
   to MemFS (a Git-backed repository of markdown files with YAML frontmatter)
   and asynchronous background auditing ("dreaming" and "memory doctor") [V:
   <https://docs.letta.com/concepts/memfs/index.md>,
   <https://docs.letta.com/configuration/memory/index.md>].
7. **Graphiti / Zep** (`getzep/graphiti`): Temporal knowledge graph engine for
   agents supporting custom entity and edge types via Pydantic models,
   extraction validation, open-world schema evolution, and edge-type mapping [V:
   <https://help.getzep.com/graphiti/core-concepts/custom-entity-and-edge-types.md>,
   <https://github.com/getzep/graphiti>].
8. **LangChain & LlamaIndex Structured Output & Self-Correction**: Agent
   framework mechanisms for enforcing schemas: `PydanticOutputParser`,
   `OutputFixingParser`, `RetryWithErrorOutputParser`, and PydanticAI. Examined
   via reference documentation and source code [V:
   <https://reference.langchain.com/python/langchain-classic/output_parsers/retry/RetryWithErrorOutputParser>,
   <https://docs.pydantic.dev/ai/core-concepts/output>].
9. **Empirical Studies on LLM Schema Adherence & Tool Misuse**:
   Controlled experimental studies measuring prompt conventions versus
   schema constraints and compiler-style validation diagnostics:
   Sigdel & Baral (arXiv:2603.13404, 2026) and Agarwal et al.
   (arXiv:2502.14905, 2025) [V: <https://arxiv.org/abs/2603.13404>,
   <https://arxiv.org/abs/2502.14905>].
10. **Prompt-Only Vault Conventions & Agent Rules** (`CLAUDE.md`, `AGENTS.md`,
    `claude-obsidian-skills`): System prompt and skill-based constraints for
    frontmatter formatting
    (`jykim/claude-obsidian-skills/obsidian-yaml-frontmatter`). Examined via
    canonical skill files and production prompt patterns [V:
    <https://github.com/jykim/claude-obsidian-skills/blob/main/obsidian-yaml-frontmatter/SKILL.md>].
11. **Docs-as-Code Prose & Frontmatter Linters** (Vale, markdownlint):
    Prose and syntax linters that parse frontmatter formats (YAML, TOML, JSON)
    into dynamic scopes for rule targeting and lint checks
    [V: https://docs.vale.sh/formats/front-matter.md].
12. **CI & Pre-Commit Frontmatter Schema Validators** (`check-jsonschema`,
    `remark-lint-frontmatter-schema`):
    Git pre-commit hooks and CI linters validating markdown frontmatter against
    JSON Schemas [V: <https://github.com/python-jsonschema/check-jsonschema>,
    <https://github.com/JulianCataldo/remark-lint-frontmatter-schema>].
13. **Structured MADR & Backstage Software Catalog**:
    Architectural Decision Records with JSON Schema frontmatter validation
    (`smadr.dev`) and Spotify Backstage catalog entity schemas
    (`catalog-model`, `entityKindSchemaValidator`)
    [V: <https://smadr.dev/reference/schema>,
    <https://github.com/backstage/backstage/blob/master/packages/catalog-model/src/validation/entityKindSchemaValidator.ts>].

All 13 projects were inspected directly through official documentation,
published specification standards, academic papers, or repository source code.
No project relies solely on unverified memory.

---

## 2. Per-Project Analysis

### 2.1 Model Context Protocol (MCP) Specification & Tool Annotations

- **What the feature is:** MCP defines standardized JSON-RPC protocols for AI
  clients to discover and invoke server capabilities: tools, resources, prompts,
  roots, and interactive elicitation [V:
  <https://modelcontextprotocol.io/specification/2026-07-28>].
- **Typing, validation, and inheritance:**
  - Tools declare an `inputSchema` conforming to JSON Schema (JSON Schema
    2020-12 per SEP-2106) [V:
    <https://modelcontextprotocol.io/seps/2106-json-schema-2020-12.md>].
  - Tools may declare an `outputSchema` for validating structured
    return data [V].
  - Tool behavior is classified via `ToolAnnotations`: `readOnlyHint`
    (default `false`), `destructiveHint` (default `true`), `idempotentHint`
    (default `false`), and `openWorldHint` (default `true`)
    [V: https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations].
  - In 2025/2026 revisions, MCP introduced **Elicitation**
    (`client/elicitation`, `ctx.mcpReq.elicitInput`), allowing a tool handler to
    pause execution and request interactive structured user input mid-flight [V:
    <https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation>].
  - Filesystem boundaries are constrained by `client/roots` [V].
- **When validation runs:** Tool input schemas are validated before execution by
  the server SDK or host (gating). Tool results indicate execution errors via
  `isError: true` on `CallToolResult` [V]. Tool annotations are non-enforcing
  hints used by client UIs to drive confirmation prompts [V].
- **Architectural location:** Transport and dispatch boundary. Schemas and hints
  are declared at registration and exposed over `tools/list` [V].
- **Typing vs storage/parsing:** MCP transmits pure JSON. Parsing and schema
  mapping are the responsibility of the server application [V].
- **Complaints, regrets, and changes:**
  - The MCP core team debated whether tool annotations can be trusted if
    untrusted servers supply them (Justin Spahr-Summers and Basil Hosmer noted:
    "clients must treat hints from untrusted servers as untrusted") [V:
    <https://github.com/modelcontextprotocol/modelcontextprotocol/pull/185#discussion_r2010043988>].
  - Simon Willison highlighted the "lethal trifecta" (private data access +
    untrusted input + external communication), leading to proposals for security
    taint tracking across tools [V:
    <https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations>].
- **Key lesson:** Input schemas provide strict structural gating, but
  behavioral guardrails (destructive actions, class changes) require
  interactive user confirmation channels (such as MCP Elicitation) or
  client-side confirmation dialogs driven by `destructiveHint`.

### 2.2 Anthropic Basic Memory Server (`@modelcontextprotocol/server-memory`)

- **What the feature is:** The reference implementation of persistent memory
  over a local knowledge graph of entities, relations, and observations, served
  over MCP as 9 tools and one subscribable resource [V:
  <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts>].
- **Typing, validation, and inheritance:**
  - Fixed entity model defined via Zod: `Entity` (`name: string`,
    `entityType: string`, `observations: string[]`) and `Relation`
    (`from: string`, `to: string`, `relationType: string`) [V: lines 721-733].
  - No subtyping or inheritance exists. `entityType` and `relationType` are
    unbounded open strings [V].
- **When validation runs:**
  - Gating on tool inputs: Zod validates arguments against JSON Schema.
  - Gating on read: parsing legacy or corrupted lines uses `safeParse`.
  - Non-gating/advisory on entity existence: `create_entities` skips entities
    that already exist rather than throwing an error, returning a text notice:
    `"Skipped N entities that already exist... use add_observations for"`
    `"existing entities"`
    and a structured `skipped` array [V: lines 791-828].
- **Architectural location:** Standalone MCP server communicating with clients
  over stdio or SSE.
- **Typing vs storage/parsing:**
  - Stored as JSONL on disk (`memory.jsonl`) [V: line 27].
  - **Unreadable line preservation:** Lines failing validation or malformed
    JSON stay out of the in-memory graph, but are appended back to `saveGraph`
    verbatim so that unrelated writes do not silently destroy unparseable or
    foreign data [V: lines 367-375].
  - Cross-process concurrency lock (`withFileLock` using `.lock` file and
    periodic heartbeat) and atomic file rename (`rename(2)`) prevent file
    truncation on crash or OOM [V: lines 249-329, 467-485].
  - Exposes `memory://knowledge-graph` as an MCP subscribable resource with
    `sendResourceUpdated` change notifications [V: lines 735, 773-783].
- **Complaints, regrets, and changes:**
  - Concurrent tool calls dispatched from a single LLM turn caused silent
    overwrites and corruption; resolved by serializing mutations behind a
    promise queue and file lock (#1819, #4797) [V: lines 338-347].
  - File permissions were wiped by atomic renames (0600 became 0644); resolved
    by explicitly preserving existing file modes (#4827) [V: lines 477-483].
- **Key lesson:** File-backed memory tools must use atomic writes, concurrency
  locks, and unreadable-line preservation so that schema validation failures
  in one record do not destroy the surrounding data on disk.

### 2.3 Obsidian Local REST API & Built-in MCP Server

- **What the feature is:** Secure local HTTPS/HTTP REST API and built-in MCP
  server running inside Obsidian, providing note CRUD, full-text and JsonLogic
  metadata search, and surgical section patching [V:
  <https://github.com/coddingtonbear/obsidian-local-rest-api>].
- **Typing, validation, and inheritance:**
  - No note schema or class system. Metadata is represented as free-form YAML
    frontmatter, tags, and document maps (headings and block references) [V].
  - `vault_patch` allows surgical targeting of a specific `heading`, `block`,
    or `frontmatter` field [V: line 449].
  - Frontmatter patch operation (`targetType: "frontmatter"`,
    `target: "status"`, `operation: "replace"`, `value: "done"`) updates only
    the targeted key while preserving comments, formatting, and other keys [V].
- **When validation runs:**
  - Gating on API boundaries: requests targeting protected directories
    (`.obsidian`) are rejected with `403` to prevent arbitrary code execution
    via plugin scripts (GHSA-66m9-r757-qvq7) [V: lines 167-178].
  - Gating on file types: `vault_read` refuses non-UTF8 binary files;
    `vault_write` refuses paths with binary extensions (PDF, images, audio) to
    prevent agents from corrupting binary files via text decoders
    [V: lines 468-469].
  - Gating on version preconditions: supports optimistic concurrency via
    `If-Match` ETags on both REST and MCP write tools (`ifMatch`), rejecting
    conflicting edits with `412 Precondition Failed` [V: lines 212-243].
  - Advisory warnings for formatting edge cases (e.g. heading depth > 6) come
    back in the `Markdown-Patch-Warnings` response header [V: line 269].
- **Architectural location:** Native Obsidian plugin running in Electron's
  renderer process, exposing HTTPS on `127.0.0.1:27124`.
- **Typing vs storage/parsing:**
  - Operates directly over vault markdown files using Obsidian's internal vault
    and metadata cache APIs.
  - Exposes `obsidian://local-rest-api/openapi.yaml` as an MCP
    resource [V: line 503].
- **Complaints, regrets, and changes:**
  - Full-file overwrites by agents caused severe data loss and destroyed
    surrounding markdown structures; resolved by introducing surgical patching
    and library-owned whitespace management [V: lines 271-275].
  - Writing attachment files as text destroyed user media; resolved by strictly
    bifurcating `vault_read`/`vault_write` (text only) from `vault_read_binary`
    (downscaling images, signed download URLs) [V: lines 468-479].
- **Key lesson:** Agents cannot be trusted with whole-file rewrites. A vault
  guardrail must provide surgical property-level and section-level patch tools
  with optimistic concurrency checks (`If-Match`).

### 2.4 Third-Party Obsidian MCP Servers (`MarkusPfundstein`, `obsidian-cli`)

- **What the feature is:** Independent MCP servers and CLI wrappers that
  interface AI assistants (Claude Desktop, Cursor) with Obsidian vaults
  either via the Local REST API plugin or directly via local disk access [V].
- **Typing, validation, and inheritance:**
  - Typically expose tools like `get_file_contents`, `patch_content`,
    `append_content`, and `search` [V:
    <https://github.com/MarkusPfundstein/mcp-obsidian>].
  - No frontmatter schema enforcement. Fields are passed as arbitrary text or
    untyped JSON [V].
- **When validation runs:**
  - Tool arguments are validated against MCP `inputSchema` (strings, arrays).
  - Note frontmatter is not validated against any domain schema; whatever
    YAML the model produces is written directly [V].
- **Architectural location:** External node/python processes interacting via
  stdio with the agent host.
- **Complaints, regrets, and changes:**
  - In `MarkusPfundstein/mcp-obsidian` (issue #167), patching frontmatter
    defaulted to string-based payloads, causing numeric and boolean fields to be
    serialized as quoted YAML strings (e.g., `rating: "5"` instead of
    `rating: 5`), corrupting Dataview queries [V:
    <https://github.com/MarkusPfundstein/mcp-obsidian/issues/167>].
  - Direct file-editing MCP servers repeatedly broke YAML delimiters and
    accidentally dropped existing frontmatter fields during append
    operations [H].
- **Key lesson:** Passing frontmatter mutations as unparsed strings through
  generic tools introduces type coercion errors on disk (quoting
  numbers/booleans).
  Field mutations must be typed and parsed by the vault backend.

### 2.5 Anthropic Native Memory Tool (`memory_20250818`)

- **What the feature is:** Anthropic's official client-side memory tool for
  Claude Opus and Sonnet models, providing persistent file operations under
  `/memories` [V:
  <https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/memory-tool>].
- **Typing, validation, and inheritance:**
  - Protocol defines 6 commands: `view`, `create`, `str_replace`, `insert`,
    `delete`, and `rename` [V: lines 413-475].
  - Zero internal schema or file format typing: memory files are arbitrary text
    (markdown, XML, JSON). Claude decides the internal layout [V].
- **When validation runs:**
  - Client handler validates paths to prevent path traversal outside
    `/memories` [V].
  - Non-gating on schema: no semantic or structural schema exists.
  - Error feedback is returned as plain strings in `tool_result` (e.g.,
    `"No replacement was performed, old_str '...' did not appear verbatim"`,
    `"Error: The destination ... already exists"`) [V: lines 441, 468].
- **Architectural location:** Defined in the Anthropic model specification and
  executed client-side by application SDK runners
  (`BetaLocalFilesystemMemoryTool`)
  [V: lines 300-346].
- **Complaints, regrets, and changes:**
  - Because files are unstructured, Claude frequently duplicates entries or
    creates inconsistent naming conventions unless heavily prompted with
    file-tree guidelines [H].
  - String replacement errors (`str_replace`) occur frequently when models fail
    to match whitespace or indentation exactly, requiring multi-turn
    retries [V].
- **Key lesson:** Plain text file memory gives agents flexibility but zero
  schema guarantees. Unstructured files inevitably suffer from semantic drift
  and redundant keys without a structured schema check.

### 2.6 Letta / MemGPT & MemFS

- **What the feature is:** The memory framework for stateful AI agents,
  evolving from MemGPT's in-context character-bounded memory blocks to Letta's
  **MemFS**: a Git-backed memory filesystem projected into local markdown
  files [V: https://docs.letta.com/concepts/memfs/index.md].
- **Typing, validation, and inheritance:**
  - MemGPT: In-context memory blocks (`persona`, `human`) with hard character
    limits, edited via tools (`core_memory_append`, `core_memory_replace`) [V].
  - MemFS: Memory projected as markdown files with YAML frontmatter containing
    `name` and `description` [V: lines 40-48].
  - Hierarchical routing: root files are loaded into context; directories
    with `MEMORY.md` index files are loaded on-demand via search or path
    [V: lines 50-68].
- **When validation runs:**
  - Advisory / background audit: Letta uses asynchronous background subagents
    called **Dreaming** (runs on context compaction or step thresholds) and
    **Memory Doctor** (audits placement, duplication, and token bloat) [V:
    <https://docs.letta.com/configuration/memory/index.md>].
  - Gating on git conflicts: memory mutations are versioned commits in Git
    worktrees, isolating background updates from active agent turns
    [V: lines 88-94].
- **Architectural location:** Core agent platform service. MemFS projects the
  cloud Git repository into a local filesystem checkout [V: lines 27-31].
- **Complaints, regrets, and changes:**
  - Rigid in-context character limits in MemGPT caused models to fail when
    appending new facts, or forced models to spend tokens constantly
    summarizing.
  - Replacing hard block schemas with a Git-backed markdown filesystem allowed
    natural scaling, but introduced the need for background reconciliation
    ("dreaming") to prevent memory clutter and fragmentation [V].
- **Key lesson:** Memory management is too complex for single-turn synchronous
  gating. Separating immediate file writes from asynchronous background
  auditing (like Letta's dreaming or git worktrees) prevents blocking
  agent turns.

### 2.7 Graphiti / Zep

- **What the feature is:** Real-time temporal knowledge graph framework for AI
  agents, transforming unstructured text into structured nodes and edges [V:
  <https://help.getzep.com/graphiti/core-concepts/custom-entity-and-edge-types.md>].
- **Typing, validation, and inheritance:**
  - Developers define custom entity and edge types using standard Pydantic
    models (`BaseModel`) [V: lines 21-65].
  - Field types include `Optional[int]`, `Optional[str]`, `Optional[datetime]`,
    `Optional[float]`, `Optional[bool]` with `Field(description=...)` [V].
  - Inheritance: standard Python class inheritance.
  - **Edge Type Mapping:** An `edge_type_map` dictionary specifies which edge
    types are permitted between specific entity type pairs
    (e.g., `("Person", "Company"): ["Employment"]`) [V: lines 77-82].
  - **Fallback / Open World:** If an entity pair lacks an edge mapping, Graphiti
    keeps the raw extracted predicate as an unclassified edge rather than
    failing [V: lines 162-167].
  - Protected attribute names (`uuid`, `name`, `group_id`, `labels`,
    `created_at`, `summary`) cannot be overwritten by custom types
    [V: lines 243-247].
- **When validation runs:**
  - Gating on ingestion: when `add_episode` extracts entities, each extracted
    instance is validated against the assigned Pydantic model (including custom
    validators) before writing to the graph [V: lines 141-158].
- **Architectural location:** Ingestion pipeline within the graph engine.
- **Typing vs storage/parsing:**
  - Schema evolution: fully backward-compatible. New attributes added to
    Pydantic models are optional (`Optional[T] = Field(None)`); historical
    nodes retain their original fields without migration [V: lines 170-179].
- **Complaints, regrets, and changes:**
  - Attempting to enforce overly granular edge types (e.g., `CEOEmployment` vs
    `EngineerEmployment`) degraded LLM extraction accuracy; docs explicitly
    advise balanced, domain-level types [V: lines 220-231].
- **Key lesson:** Schema evolution in agent knowledge bases requires an
  open-world, additive model (optional fields with defaults). Hard closed-world
  validation on historical data breaks backward compatibility.

### 2.8 LangChain, LlamaIndex & PydanticAI Structured Output

- **What the feature is:** Framework utilities that constrain LLM completions to
  structured schemas via prompt formatting, function calling, and
  self-correction parsers [V:
  <https://reference.langchain.com/python/langchain-classic/output_parsers/retry/RetryWithErrorOutputParser>,
  <https://docs.pydantic.dev/ai/core-concepts/output>].
- **Typing, validation, and inheritance:**
  - Schemas defined via Pydantic or JSON Schema.
  - LangChain `PydanticOutputParser` injects format instructions into prompts.
- **When validation runs:**
  - Post-hoc validation with retry loop: if the model outputs malformed JSON
    or violates Pydantic constraints, the parser throws a `ValidationError`.
  - `OutputFixingParser` and `RetryWithErrorOutputParser` catch the error and
    dispatch an automated self-correction prompt containing: (1) the original
    prompt, (2) the invalid completion, and (3) the exact validation error
    message, instructing the model to fix the violations [V].
- **Architectural location:** Client-side orchestration layer wrapping
  LLM calls.
- **Complaints, regrets, and changes:**
  - Prompt-only format instructions fail 15-40% of the time on complex nested
    schemas without native tool calling / JSON mode [H].
  - Returning generic error messages ("Invalid JSON") causes models to repeat
    the mistake; providing structured, field-specific compiler-style diagnostics
    dramatically improves single-turn recovery rates [V].
- **Key lesson:** When an agent produces an invalid structured output, returning
  the exact validation error (path, expected, found) into the context window
  is the most reliable mechanism for automatic error recovery.

### 2.9 Empirical Studies on Agent Schema Adherence (Sigdel 2026, Agarwal 2025)

- **What the feature is:** Rigorous empirical research isolating tool-interface
  representation and validation diagnostics as experimental variables under
  budgeted agent execution [V: <https://arxiv.org/abs/2603.13404>,
  <https://arxiv.org/abs/2502.14905>].
- **Findings from Sigdel & Baral (arXiv:2603.13404, March 2026):**
  - Evaluated three conditions under information equivalence: (A) Free-form
    natural language docs, (B) strict JSON Schema, and (C) JSON Schema with
    structured compiler-style diagnostics [V: §1, §4].
  - Structured validation errors took the exact shape:

    ```text
    {
      "error_type": "SCHEMA_VALIDATION",
      "tool": "get_metric",
      "violations": [
        {"path": "$.metric_key", "expected": "enum",
         "allowed": ["p95_latency","error_rate"], "found": "latency95"},
        {"path": "$.window.minutes", "expected": "integer >= 1", "found": 0}
      ]
    }
    ```

    [V: §5, line 214].
  - **Interface Misuse vs Semantic Misuse:** Schema conditions (B and C)
    significantly reduced *interface misuse* (malformed arguments, invalid
    types, missing required fields). However, schemas did NOT reduce *semantic
    misuse* (calling the wrong tool or unproductive queries) [V: Abstract, §1].
  - Structured diagnostics (C) significantly improved recovery probability
    under tight step budgets, preventing agents from exhausting their turn
    budget on repeated syntax errors [V: §4].
- **Findings from Agarwal et al. (arXiv:2502.14905, Feb 2025):**
  - Tested schema adherence across foundation models (DeepSeek R1, Qwen,
    Gemini 2.0 Flash) converting legacy records to JSON schemas [V: §1].
  - Success rates in producing valid schema-conforming outputs varied from
    0% to 100% depending on prompt complexity and model capability [V: §1].
  - Prompt engineering alone does not guarantee schema consistency; models
    inevitably fail on edge cases and complex constraints without formal
    reinforcement or schema-gated decoding [V: §2.4].
- **Key lesson:** Prompt instructions alone cannot guarantee schema adherence.
  Schemas eliminate interface syntax errors, and structured diagnostics enable
  recovery within budget, but semantic domain logic remains the agent's
  bottleneck.

### 2.10 Prompt-Only Vault Conventions & Rules (`claude-obsidian-skills`)

- **What the feature is:** Prompt-based instructions instructing coding agents
  (Claude Code, Cursor, Aider) how to format frontmatter and wikilinks in
  markdown vaults [V:
  <https://github.com/jykim/claude-obsidian-skills/blob/main/obsidian-yaml-frontmatter/SKILL.md>].
- **Typing, validation, and inheritance:**
  - Natural language rules: "Single YAML block at top", "lowercase property
    names", "no hashtags in YAML tags list", "wrap wikilinks in double quotes",
    "use ISO YYYY-MM-DD for dates", "quote values with colons or special chars"
    [V: lines 28-150].
- **When validation runs:**
  - Never at the platform level. Completely advisory [V].
  - Depends entirely on the model's in-context attention.
- **Observed failures and user complaints:**
  - Unquoted wikilinks: models frequently write `sources: [ [[Note A]] ]`,
    which is invalid YAML that breaks Obsidian's frontmatter parser
    [V: line 105].
  - Numeric stringification: versions like `version: 2.0` are written unquoted
    and parsed as floats, dropping trailing zeros [V: line 147].
  - Overwriting entire notes: agents using basic file-writing tools wipe
    unrelated frontmatter properties, drop blank lines after `---`, or strip
    inline Dataview fields (`key:: value`) during edits [H].
  - Context degradation: across long multi-turn conversations, models suffer
    instruction drift and stop adhering to the skill checklist [H].
- **Key lesson:** Advisory prompt rules are necessary for initial drafting,
  but completely insufficient for vault integrity. Without a machine-enforced
  validation or patching tool, agent edits degrade vault metadata over time.

### 2.11 Docs-as-Code Linters (Vale, markdownlint)

- **What the feature is:** Static prose and markup linters that parse
  frontmatter
  in Markdown, MDX, and AsciiDoc to run style and syntax checks
  [V: https://docs.vale.sh/formats/front-matter.md].
- **Typing, validation, and inheritance:**
  - Vale recognizes YAML (`---`), TOML (`+++`), and JSON (`;;;` or `---json`)
    frontmatter delimiters [V].
  - Dynamically creates hierarchical scopes for frontmatter fields:
    `text.frontmatter.title`, `text.frontmatter.description`,
    `text.frontmatter.author` [V].
  - Rules can assert capitalization, forbidden words, or regex patterns on
    specific frontmatter scopes [V].
  - Only string fields are linted; lists and nested maps are ignored [V].
- **When validation runs:** Post-hoc CLI execution, CI pipeline, or LSP
  diagnostics (`vale-ls`).
- **Architectural location:** Standalone CLI / language server reading
  disk files.
- **Complaints, regrets, and changes:**
  - If a file contains syntax-invalid frontmatter, Vale cannot parse scopes and
    reports a document-level read failure (exit code 2) [V:
    <https://docs.vale.sh/topics/cli.md>].
  - Vale provides no schema-level type enforcement (cannot verify whether a
    date is a valid date or whether a number is within a bounded range); it
    only checks prose patterns within strings [V].
- **Key lesson:** Document linters successfully use scoped field selectors
  (`frontmatter.<key>`) for targeted checks, but general prose linters cannot
  replace a dedicated frontmatter schema validator.

### 2.12 CI & Pre-Commit Frontmatter Validators (`check-jsonschema`)

- **What the feature is:** Pre-commit hooks and CI utilities that validate YAML
  frontmatter in markdown files against formal JSON Schemas
  [V: <https://github.com/python-jsonschema/check-jsonschema>,
  <https://github.com/JulianCataldo/remark-lint-frontmatter-schema>].
- **Typing, validation, and inheritance:**
  - JSON Schema Draft-07 / 2020-12: supports types (string, number, boolean,
    array, object), required fields, regex patterns, enums, numeric ranges [V].
  - `remark-lint-frontmatter-schema` integrates into the unified/remark AST
    ecosystem [V].
- **When validation runs:**
  - Gating on git commit (pre-commit hook) or gating on pull request merge
    (GitHub Actions CI) [V].
- **Complaints, regrets, and changes:**
  - Strict schemas (`additionalProperties: false`) break community workflows
    when authors add custom frontmatter fields for local plugins; maintainers
    often regret disallowing unknown fields [H].
  - Git pre-commit hooks fail abruptly when run against AI agent edits,
    aborting the commit without providing the agent with a structured correction
    payload unless the agent framework explicitly captures stderr [H].
- **Key lesson:** Pre-commit and CI validation provide an essential backstop,
  but running validation only at the commit stage creates high recovery costs
  for agents compared to tool-level gating during the edit step.

### 2.13 Structured MADR & Backstage Software Catalog

- **What the feature is:**
  - Structured MADR (`smadr.dev`): Architectural Decision Records with YAML
    frontmatter validated against JSON Schema Draft-07 [V:
    <https://smadr.dev/reference/schema>].
  - Backstage Software Catalog: Spotify's developer portal cataloging software
    entities (`Component`, `API`, `Resource`) via `catalog-info.yaml` and
    TechDocs markdown metadata [V:
    <https://backstage.io/docs/features/software-catalog/descriptor-format/>].
- **Typing, validation, and inheritance:**
  - Structured MADR: required fields (`title`, `description`, `type`,
    `category`, `tags`, `status`, `created`, `updated`, `author`, `project`).
    Tag regex `^[a-z0-9][a-z0-9-]*[a-z0-9]$`. Status enum (`proposed`,
    `accepted`, `deprecated`, `superseded`).
  - **Open-world extension:** Custom metadata is explicitly accommodated via
    `x-` prefixed extension fields (e.g. `x-team: platform`, `x-priority: high`)
    [V: https://smadr.dev/reference/schema#extension-fields].
  - Backstage: Entities declare `apiVersion`, `kind`, `metadata`, `spec`. Each
    `kind` is validated by `entityKindSchemaValidator` using Ajv compiled
    schemas [V:
    <https://github.com/backstage/backstage/blob/master/packages/catalog-model/src/validation/entityKindSchemaValidator.ts>].
- **When validation runs:**
  - Ingestion pipeline: catalog processors validate entities upon ingest. If an
    entity fails validation, it is marked with an error state in the database
    rather than crashing the catalog processor [V:
    <https://backstage.io/docs/features/software-catalog/life-of-an-entity.md>].
  - CLI: `backstage-cli repo schema:openapi` and pre-commit linting [V].
- **Complaints, regrets, and changes:**
  - Early ADR tools without frontmatter validation suffered from inconsistent
    status values and missing dates across repositories, preventing automated
    indexing and visualization [V:
    <https://smadr.dev/explanation/design-decisions>].
- **Key lesson:** Standardized frontmatter requires: (1) a single explicit
  `kind`/`type` declaration in frontmatter, (2) strict typing of standard
  fields, and (3) an explicit extension prefix (`x-`) for open-world
  extensibility
  without breaking validation.

---

## 3. Comparison Matrix Across Examined Projects

| Project / Standard | Field Type Set | Required / Optional | Constraints (min/max/enum/regex) | Inheritance / Composition | Class / Type Declaration | Validation Timing | Open vs Closed World | External Value Sources | Diagnostics Surface |
|---|---|---|---|---|---|---|---|---|---|
| **MCP Spec & SDK** | JSON Schema 2020-12 primitives & objects | Declared in `required` array | Full JSON Schema constraints | None (flat tool schemas; `allOf` composition) | Tool name in registry | Gating on tool call dispatch | Open (closed per tool schema) | Resources & dynamic tool lists | `CallToolResult` (`isError: true`, structured content) |
| **Basic Memory MCP** | String, string array (Entity, Relation) | All fields required in Zod schema | Search query `max(2048)` | None | `entityType` (unbounded string) | Gating on tool call; advisory on skip | Open (unreadable lines preserved) | None | JSON response + plain text notice of skipped entities |
| **Obsidian Local REST API** | Untyped text / JSON frontmatter | None | Path regex, relative heading levels | None | None | Gating on routes/types; advisory on formatting | Open (preserves unedited keys) | Vault index (tags, document map) | HTTP status + `Markdown-Patch-Warnings` header |
| **Third-Party Obsidian MCP** | Strings, untyped frontmatter JSON | Defined per tool inputSchema | Basic tool parameter types | None | None | Gating on tool params only | Open | None | Text error strings on tool failure |
| **Anthropic Memory Tool** | Text files under `/memories` | None | None | None | None | Gating on path boundaries only | Open | None | Plain string error messages in `tool_result` |
| **Letta / MemFS** | Markdown + YAML (`name`, `description`) | `name` required in frontmatter | Context window character limits | Directory nesting | Frontmatter `name` & folder hierarchy | Advisory background audit ("dreaming") | Open (arbitrary markdown) | MemFS search mod (QMD vector index) | Background agent logs / worktree commits |
| **Graphiti / Zep** | Pydantic types (str, int, float, datetime, bool) | Optional by default (`Optional[T] = Field(None)`) | Pydantic validators (`validate_age`, bounds) | Python class inheritance; edge-type map | Class name in `entity_types` dict | Gating on ingestion (`add_episode`) | Open (unmapped edges become raw predicates) | Search filters, temporal graph state | Ingestion exceptions / validation errors |
| **LangChain / PydanticAI** | Pydantic types & custom validators | Explicit `Optional` vs required | Full Pydantic validators & field bounds | Pydantic model inheritance | Pydantic class name | Gating post-hoc with retry parser | Configurable (Pydantic `extra="allow"` / `"forbid"`) | External API lookups via tools | Structured `ValidationError` fed back to LLM |
| **Empirical Study (Sigdel 2026)** | JSON Schema tool contracts | Explicit `required` list | Bounds (`integer >= 1`), enums (`allowed`) | Canonical contract inheritance | Tool schema name | Gating on tool call execution | Closed per tool contract | Benchmark artifacts (logs, metrics) | Structured JSON diagnostics (`path`, `expected`, `found`) |
| **Prompt Skills (Obsidian Skill)** | Markdown frontmatter conventions | Prompt checklist ("Core Properties") | Regex conventions (YYYY-MM-DD, lowercase) | Template composition | Prompt instructions | Never (purely advisory prompt rules) | Open | None | None (silent failure or user review) |
| **Vale Linter** | String prose scopes | Ignored / targeted scopes | Capitalization, regex, wordlists | Style inheritance (`extends: capitalization`) | Dynamic scope `text.frontmatter.<key>` | Post-hoc CLI / CI / LSP | Open (ignores non-string fields) | External vocabulary files | CLI / LSP diagnostic messages with line numbers |
| **check-jsonschema** | JSON Schema (Draft-07, 2020-12) | Standard JSON Schema `required` | Full JSON Schema (pattern, min, max, enum) | Schema `$ref` composition | File path pattern matching | Gating in pre-commit hook / CI | Configurable (`additionalProperties`) | Remote cached schema files | CLI error trace with JSON paths and expected rules |
| **Structured MADR / Backstage** | YAML / JSON Schema types | Explicit required ADR / catalog fields | Tag regex `^[a-z0-9-]+$`, status enum | Backstage `apiVersion`/`kind` hierarchy | Frontmatter `type: adr` / Backstage `kind` | Gating in CI / Ingestion processor | Open via explicit `x-` prefix | Backstage catalog relations & entity refs | Ingestion error entity status / CI failure logs |

---

## 4. Challenges to Traces' Design (Ticket 16 Answer)

This section systematically tests each decision of Traces' Ticket 16 design
against the external precedent gathered above.

### 4.1 "Schemas validate note data as an advisory, on-demand check"

- **Supporting Evidence:**
  - Backstage catalog processing: invalid entities do not abort ingestion or
    crash the service; they are ingested and flagged with error diagnostics in
    the catalog database [V:
    <https://backstage.io/docs/features/software-catalog/life-of-an-entity.md>].
  - Vale and markdownlint: linters run as separate advisory passes, decoupling
    document storage and parsing from validation [V:
    <https://docs.vale.sh/topics/cli.md>].
  - Anthropic Basic Memory: unparseable lines do not invalidate the entire graph
    file; they are preserved and bypassed [V:
    <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts:367-375>].
- **Contradicting Evidence:**
  - Sigdel & Baral (arXiv:2603.13404, 2026): in agent tool execution,
    advisory-only checks fail. Agents under step budgets repeat invalid actions
    unless the interface enforces contract validity at the execution
    boundary [V: §1, §4].
  - LangChain / PydanticAI: models ignore advisory formatting guidance in
    prompts; reliable agent pipelines require strict post-hoc gating that
    triggers a retry loop [V:
    <https://reference.langchain.com/python/langchain-classic/output_parsers/retry/RetryWithErrorOutputParser>].
- **Recommendation:** **KEEP Traces' design**, but enforce a strict host-level
  distinction. Core `check()` remains pure, span-free, and advisory. However,
  the planned MCP host **MUST implement gating policy** (or delta-gating policy)
  that sets `isError: true` on `CallToolResult`. If the MCP host returns
  advisory warnings on a successful tool call (`isError: false`), empirical
  precedent proves LLMs will ignore the warnings and proceed with corrupted
  state.

### 4.2 "The delta rule blocks only newly introduced violations"

- **Supporting Evidence:**
  - Graphiti / Zep: schema evolution preserves backward compatibility; existing
    unclassified nodes and historical attributes are never invalidated when new
    schema rules are introduced [V:
    <https://help.getzep.com/graphiti/core-concepts/custom-entity-and-edge-types.md>].
  - Obsidian Local REST API: surgical patching targets only the edited
    property or section, leaving all other frontmatter fields completely
    untouched [V: https://github.com/coddingtonbear/obsidian-local-rest-api].
  - Pre-commit linters in large monorepos: gating only on changed lines /
    git diffs prevents legacy files from blocking urgent updates [H].
- **Contradicting Evidence:**
  - None. No project reported regret from isolating validation to changed
    fields. Projects that enforced whole-document validation on legacy files
    consistently reported user frustration and stalled migrations.
- **Recommendation:** **KEEP and PRIORITIZE**. The delta rule is essential for
  agent integration in real-world vaults. Without it, an agent tasked with
  updating a single field in a legacy note will be permanently blocked by
  unrelated syntax violations created years prior.

### 4.3 "Several classes: extends inheritance, first-listed class wins"

- **Supporting Evidence:**
  - CSS cascading classes and Python MRO: "first wins" or left-to-right
    resolution is a well-understood programming language convention [H].
- **Contradicting Evidence:**
  - **Zero agent, PKM, or catalog tools support multi-class inheritance with
    first-wins field conflict.**
  - Backstage enforces a strict single `kind` per entity (`Component`, `API`,
    `Resource`) [V:
    <https://backstage.io/docs/features/software-catalog/descriptor-format/>].
  - Structured MADR enforces a single `type: adr` [V:
    <https://smadr.dev/reference/schema>].
  - Graphiti uses single class models or explicit edge maps; it does not allow
    an entity to inherit from multiple conflicting classes where order dictates
    typing [V:
    <https://help.getzep.com/graphiti/core-concepts/custom-entity-and-edge-types.md>].
  - For LLMs, multiple inheritance with silent first-wins resolution is a
    catastrophic source of hallucination: an agent reading frontmatter cannot
    know whether `status` is an enum, string, or number without simulating
    the schema resolver's list-order traversal.
- **Recommendation:** **CHANGE / SIMPLIFY**. Deprecate multi-class inheritance
  or disallow conflicting field declarations between classes. If multiple
  classes are declared on a note (e.g. `class = ["book", "project"]`), require
  their field sets to be disjoint, or emit a hard validation error
  (`schema.class-conflict`) rather than silently letting the first class win.
  Better yet, adopt Backstage and MADR's precedent: one primary `class` (or
  `type`), with secondary attributes handled via composition (traits or tags).

### 4.4 "Options: required, multi, min, max, step, format"

- **Supporting Evidence:**
  - `required`, `min`, `max`, and `format` are universal across JSON Schema,
    Zod, Pydantic, and Structured MADR [V].
  - Sigdel & Baral (arXiv:2603.13404): explicit `min`/`max` bounds and enums
    in tool schemas directly reduced interface misuse in agents [V: §5].
- **Contradicting Evidence:**
  - **`step` has zero precedent in document, PKM, or agent metadata schemas.**
    `step` is an HTML form input constraint for number steppers. Neither
    JSON Schema, Pydantic, Zod, Backstage, nor MADR includes `step` as a
    metadata validation constraint [V]. Checking float equality or relative
    tolerance for `step` in frontmatter adds significant code complexity for
    virtually no PKM user need.
- **Recommendation:** **CHANGE**. Keep `required`, `multi`, `min`, `max`,
  and `format`. **Drop `step`**. It is dead weight inherited from UI form
  widgets that has no place in note metadata validation.

### 4.5 "Select values possibly external JSON lists of value/label/link"

- **Supporting Evidence:**
  - Obsidian Dataview and Templater allow suggester modals to load external
    values [V].
- **Contradicting Evidence:**
  - Dynamic external value sources in schemas create severe synchronization,
    caching, and offline-readiness issues [H].
  - In MCP, dynamic lists are exposed as separate resources or query tools
    (e.g., `tag_list` in Obsidian Local REST API, `search_nodes` in Basic
    Memory),
    never as external file pointers embedded inside static schema types [V].
- **Recommendation:** **CHANGE / DEFER**. Keep select values static within
  the schema TOML. Defer external JSON file loading to the template or app
  layer. The core validator should not depend on external file I/O to evaluate
  whether a scalar is valid.

### 4.6 "Strict TOML format (unknown keys are errors)"

- **Supporting Evidence:**
  - Strict validation prevents typo bugs in schema definitions [H].
- **Contradicting Evidence:**
  - Structured MADR's explicit lesson: strict closed-world schemas broke
    real-world adoption until they introduced the `x-` prefix escape hatch for
    custom metadata [V: https://smadr.dev/reference/schema#extension-fields].
  - Anthropic Basic Memory Server: strictly rejecting unknown record types or
    fields caused accidental data loss during read-modify-write cycles; the
    server had to be modified to preserve unreadable/unknown records [V:
    <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts:367-375>].
  - Obsidian ecosystem: users and plugins constantly add experimental
    frontmatter keys. Rejecting unknown fields with hard errors causes users
    to disable the validator entirely.
- **Recommendation:** **CHANGE**. Adopt Structured MADR's compromise: standard
  declared fields are strictly typed, but unknown fields should either be
  treated as advisory info/warnings (never gating), or an explicit prefix
  (`x-` or `_`) should be reserved for open-world extension fields.

### 4.7 Essential Capabilities Traces Has NOT Considered

External projects reveal four critical capabilities missing from Ticket 16:

1. **MCP Elicitation for Class-Change Confirmation:** Ticket 16 states:
   *"Changing a note's class, or setting an unknown class, asks the user and
   denies when no confirmation channel exists."* Ticket 16 treats this as an
   unsolved problem for MCP. However, the MCP specification already standardized
   **Elicitation** (`client/elicitation`, `ctx.mcpReq.elicitInput`) for this
   exact purpose: a tool handler pauses execution and asks the user for
   confirmation directly through the MCP host UI [V:
   <https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation>].
2. **Tool Annotations for Risk Signaling:** MCP `ToolAnnotations`
   (`destructiveHint: true`, `readOnlyHint: false`) must be emitted on Traces'
   MCP write and delete tools so that client hosts (Claude Desktop, Cursor) know
   to show user confirmation dialogs before mutating vault notes [V:
   <https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations>].
3. **Preserving Unparseable / Foreign Frontmatter on Write:**
   Anthropic Basic Memory and Obsidian Local REST API prove that
   read-modify-write cycles destroy data unless the write layer explicitly
   preserves unparseable, foreign, or comment-laden frontmatter blocks
   [V: lines 367-375]. Traces must
   ensure that frontmatter validation failures do not cause the write adapter
   to overwrite or truncate raw frontmatter text.
4. **Surgical Property-Level Patch Tools for Agents:**
   Obsidian Local REST API demonstrated that giving agents full-file write tools
   (`vault_write`) inevitably leads to frontmatter corruption, lost wikilinks,
   and mangled formatting. Agents need surgical patch tools (like `vault_patch`
   targeting `frontmatter.<key>`) with optimistic concurrency checks
   (`If-Match`).

---

## 5. Failure Stories and Regrets

- **Concurrent Tool Write Race in Anthropic Memory Server (2025):** When LLMs
  invoked multiple memory mutation tools within a single conversational turn,
  concurrent processes each loaded the graph file, mutated their local copy, and
  wrote it back, silently overwriting each other's changes and corrupting the
  JSONL file. Fixed by serializing mutations behind an in-process promise queue
  and a cross-process `.lock` file with heartbeats (#1819, #4797) [V:
  <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts:338-347>].
- **Atomic File Rename Wiping Permissions in Node MCP Tools (2025):** Using
  atomic `rename(2)` over target files to prevent truncation on crash had the
  unintended consequence of resetting file permissions to the process umask
  (turning private 0600 files into public 0644 files). Fixed by explicitly
  copying existing permission bits to the temporary file before renaming (#4827)
  [V:
  <https://github.com/modelcontextprotocol/servers/blob/main/src/memory/index.ts:477-483>].
- **String Coercion Corruption in mcp-obsidian (2024):**
  In `MarkusPfundstein/mcp-obsidian`, frontmatter patch tools passed values as
  raw strings, causing numbers and booleans to be written as quoted YAML strings
  (`rating: "5"`), which broke Dataview numeric comparisons across user vaults
  (Issue #167) [V: https://github.com/MarkusPfundstein/mcp-obsidian/issues/167].
- **Rewrites Destroying Attachments in Obsidian Local REST API (2024):** AI
  agents using text-based write tools accidentally read binary attachments
  (images, PDFs) as UTF-8 text and wrote them back, permanently corrupting the
  binary files. Fixed by strictly refusing binary file extensions in text tools
  and creating a dedicated `vault_read_binary` tool with downscaled image
  previews [V:
  <https://github.com/coddingtonbear/obsidian-local-rest-api#binary-files-and-attachments>].
- **Strict Closed-World Schema Rejection in Structured MADR (2024):** Original
  MADR schemas rejected any unknown YAML frontmatter key, causing breakage
  across teams who added CI/CD metadata, team tags, or Jira tickets. The
  specification had to be revised to explicitly support open-world extension
  fields via the `x-` prefix [V:
  <https://smadr.dev/reference/schema#extension-fields>].
- **In-Context Character Bounds in MemGPT (2024):** MemGPT's hard character
  ceilings on in-context memory blocks caused frequent tool call exceptions when
  agents attempted to append facts, forcing models into expensive multi-turn
  summarization loops. Letta completely abandoned fixed in-context blocks in
  favor of a Git-backed filesystem (MemFS) with asynchronous background
  consolidation [V: https://docs.letta.com/concepts/memfs/index.md].
- **Code Execution via Vault Config Overwrite in Obsidian API (2024):** Allowing
  scripts and AI agents to write to `.obsidian` allowed attackers to drop
  malicious JavaScript into plugin files (`main.js`), achieving remote code
  execution. The API had to be locked down to strictly forbid access to the
  configuration directory by default (GHSA-66m9-r757-qvq7) [V:
  <https://github.com/coddingtonbear/obsidian-local-rest-api#the-configuration-directory-is-off-limits>].

---

## 6. Transferable Lessons for Traces

1. **Expose Schemas as MCP Resources and Prompts:**
   Traces should expose resolved class schemas as MCP resources (e.g.
   `traces://schemas/{class_name}`) and prompt context. Providing the schema
   up front prevents interface misuse before the model ever attempts a write.
2. **Implement Structured Compiler-Style Diagnostics on MCP Error:**
   When an agent tool call fails validation, Traces' MCP host must return
   `isError: true` with a structured payload:

   ```text
   {
     "error_code": "schema.out-of-range",
     "field": "rating",
     "expected": "integer between 1 and 5",
     "found": 10,
     "class": "book"
   }
   ```

   Sigdel & Baral (arXiv:2603.13404) demonstrated that structured diagnostics
   with field paths and expected/found sets are what allow budgeted agents
   to self-correct on the next turn.
3. **Provide Surgical Frontmatter Patch Tools:**
   Do not force agents to rewrite entire markdown files to update frontmatter.
   Traces' MCP layer should provide a dedicated `patch_frontmatter` tool that
   takes a note path, a field key, and a new typed value, leaving the rest of
   the document and comments completely untouched.
4. **Use MCP Elicitation for Class Changes:**
   Instead of inventing an ad-hoc out-of-band confirmation mechanism for class
   changes, Traces' MCP server should leverage the standardized MCP Elicitation
   protocol (`client/elicitation`) to prompt the user directly in the client UI.
5. **Mark Mutating Tools with Destructive Hints:**
   All note-writing, patching, and deleting tools exposed over MCP should
   declare `destructiveHint: true` (or `destructiveHint: false` for additive
   patching) and `readOnlyHint: false` per the official MCP ToolAnnotations
   standard.
6. **Drop `step` and Multi-Class First-Wins Conflict:**
   Simplify the schema core by eliminating the HTML-form `step` option and
   disallowing conflicting field definitions across multiple classes.
   Clarity beats unneeded flexibility.
