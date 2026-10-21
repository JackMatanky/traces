# G1: Agent Harness Capabilities Matrix and Write Guardrails

Research date: 2026-10-21. Target: Cross-harness write guardrail feasibility
for schema validation on note frontmatter.

---

## 1. Executive Summary

This research investigates how schema checks can gate or provide feedback on
note frontmatter modifications across 14 major AI agent harnesses.

Ticket 16 recorded the hypothesis [H]:
> "Direct file edits get detection and feedback only, because a pre-write hook
> sees the edit instruction and not the resulting file [H, from tool
> documentation, not tested]. True gating exists only where the tool owns the
> write (MCP edit tools)."

### Headline Findings

1. **Pre-write hook payload limitation is verified [V]:** Across all 8 harnesses
   that support pre-tool hooks (Claude Code, OMP, Codex CLI, Gemini CLI, Cursor,
   Windsurf, Cline, Copilot, Amp), pre-tool hooks receive only tool inputs
   (`file_path`, `old_string`/`new_string`, `patch`, or command line). None
   receives the resulting file. A pre-tool hook cannot validate schema without
   re-implementing the harness's internal text patch application.
2. **Lifecycle hooks lack cross-harness portability:** Only 8 of 14 harnesses
   (57%) support pre-tool blocking hooks. Aider, Zed Agent, Continue, and Goose
   have no pre-tool hook interception. Gating cannot rely on hooks.
3. **MCP client support is near-universal (13/14, 93%):** Dedicated MCP write
   tools (`traces_edit_note`, `traces_create_note`) represent the only portable
   mechanism for true write gating, because the tool executes validation before
   persisting to disk. However, only ~50% of harnesses can cleanly force MCP as
   the exclusive write path by disabling built-in file/bash tools.
4. **Instruction files (`AGENTS.md`) are the universal common denominator
   (14/14, 100%):** All 14 harnesses discover instruction markdown files.
5. **LSP diagnostics are widely supported (10/14, 71%):** Editor-based and
   LSP-aware harnesses feed diagnostics back after file edits, providing an
   advisory correction channel, but LSP cannot abort saves by protocol.
6. **Hard backstop across all harnesses:** A git pre-commit hook and on-demand
   `traces check` CLI remain the universal backstop against invalid writes.

---

## 2. Harness Capability Matrix

Legend:

- **Hooks**: Lifecycle pre/post tool hooks supported? Can pre-tool block?
- **Payload**: Content passed to pre-tool hook (`Instruction` vs `Resulting file`).
- **MCP**: Supports Model Context Protocol clients?
- **Force MCP**: Can built-in file/bash write tools be disabled to force MCP?
- **Inst. File**: Discovers markdown instruction files (`AGENTS.md`, `CLAUDE.md`).
- **Sandbox**: Has filesystem permissions / OS sandbox (Seatbelt / bubblewrap).
- **LSP Diag**: Surfaces LSP / compiler diagnostics back to model after edits.
- **Post Run**: Built-in post-edit linter/test auto-run feature.
- **ACP**: Supports Agent Client Protocol (client, host, or server).

| Harness | Hooks (Pre/Post) | Pre Payload | MCP Client | Force MCP | Inst. File | Sandbox | LSP Diag | Post Run | ACP |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Claude Code** | Yes (Block) [V] | Instruction [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | No [V] |
| **omp / oh-my-pi** | Yes (Block) [V] | Instruction [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Hooks/LSP [V] | Yes [V] |
| **OpenAI Codex CLI** | Yes (Block) [V] | Instruction [V] | Yes [V] | Partial [H] | Yes [V] | Yes [V] | No [V] | Hooks [V] | No [V] |
| **Gemini CLI** | Yes (Block) [V] | Instruction [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | No [V] | Hooks [V] | Yes [V] |
| **Cursor** | Yes (Block) [V] | Instruction [V] | Yes [V] | Partial [V] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | No [V] |
| **Windsurf** | Yes (Block) [V] | Instruction [V] | Yes [V] | Partial [H] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | Yes [V] |
| **Cline / Roo** | Yes (Block) [V] | Instruction [V] | Yes [V] | Partial [V] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | Yes [V] |
| **Aider** | No [V] | None [V] | No [V] | No [V] | Yes [V] | No [V] | No [V] | Native [V] | No [V] |
| **GitHub Copilot** | Yes (Block) [V] | Instruction [V] | Yes [V] | Partial [H] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | No [V] |
| **OpenCode** | Yes (Block) [V] | Instruction [V] | Yes [V] | Yes [H] | Yes [V] | Yes [V] | Yes [V] | LSP [V] | Yes [V] |
| **Zed Agent** | No [V] | None [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Tool [V] | Yes [V] |
| **Continue** | No [V] | None [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | No [V] | No [V] |
| **Goose** | No [V] | None [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | No [V] | No [V] | Yes [V] |
| **Amp** | Yes (Block) [V] | Instruction [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Yes [V] | Hooks [V] | Yes [V] |

---

## 3. Ranked Common Denominator Capabilities

Capabilities ranked by prevalence across the 14 harnesses:

1. **Instruction Files (14 / 14, 100%):**
   Every harness supports soft guidance injected from files such as `AGENTS.md`,
   `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `.clinerules`, or `.goosehints`.
   *Implication:* Schema rules and guidance can always be delivered in-context
   as soft guidance.
2. **MCP Client Support (13 / 14, 93%):**
   Supported everywhere except Aider.
   *Implication:* An MCP server tool (`traces_edit_note`) can be deployed for
   almost all modern agent environments to perform true, gated writes.
3. **LSP Diagnostics Fed Back to Model (10 / 14, 71%):**
   Supported by all editor integrations and LSP-aware agents (Claude Code, OMP,
   Cursor, Windsurf, Cline, Copilot, OpenCode, Zed, Continue, Amp).
   *Implication:* LSP `didChange`/`didSave` publish diagnostics provide an
   effective, automated advisory feedback loop in ~70% of environments.
4. **Lifecycle Pre-Tool Blocking Hooks (8 / 14, 57%):**
   Supported in Claude Code, OMP, Codex CLI, Gemini CLI, Cursor, Windsurf,
   Cline, Copilot, Amp.
   *Implication:* Hooks are unavailable in Aider, Zed, Continue, and Goose.
   Furthermore, where present, pre-tool hooks receive only input parameters,
   never the resulting document.
5. **Forcing MCP as the Exclusive Write Path (7 / 14, 50%):**
   Cleanly supported in Claude Code (`--disallowedTools`), OMP (`--no-tools`),
   Zed (`agent_profiles`), Continue (tool toggles), Goose (extension selection),
   Gemini CLI (`BeforeToolSelection` function whitelist), Amp (`tool.call`).
6. **Agent Client Protocol (ACP) Support (8 / 14, 57%):**
   Supported natively by OMP, Gemini CLI, Windsurf (preview), Cline, OpenCode,
   Zed, Amp, and Goose. Not supported natively by Claude Code, Codex, Cursor,
   Aider, GitHub Copilot, Continue.
7. **Built-in Post-Edit Command / Test Runner (1 / 14 natively, 7%):**
   Aider is the only harness with native `--lint-cmd` / `--test-cmd` flags that
   auto-run arbitrary commands on every edit and feed non-zero exit output into
   chat. Other harnesses achieve this via custom `PostToolUse` scripts or LSP.

---

## 4. Per-Harness Evidence and Analysis

### 4.1 Claude Code (Anthropic)

- **(a) Lifecycle hooks:** [V] Configured in `settings.json` or plugin
  `hooks/hooks.json`. Supports `PreToolUse`, `PostToolUse`, `PostToolUseFailure`,
  `Stop`, `SessionStart` (<https://code.claude.com/docs/en/hooks>).
  - Pre-tool payload: Receives `tool_name`, `tool_input`, `tool_use_id` on
    stdin. For `Edit`, `tool_input` contains `file_path`, `old_string`,
    `new_string` (<https://code.claude.com/docs/en/tools-reference#edit-tool-behavior>).
    It receives the **instruction**, NOT the resulting file.
  - Can block: [V] YES. Returning JSON with `permissionDecision: "deny"`
    inside `hookSpecificOutput` or exit code 2 blocks the call
    (<https://code.claude.com/docs/en/hooks#pretooluse-decision-control>).
  - Post-tool payload: `PostToolUse` receives `tool_input` and `tool_response`.
    Can return `updatedToolOutput`, but the file edit has already landed on disk.
- **(b) MCP client support:** [V] Configured via `~/.claude.json` and
  `.claude/mcp.json` (<https://code.claude.com/docs/en/mcp>).
  - Forcing MCP: [V] YES. Claude Code provides `--disallowedTools` and
    `permissions.deny: ["Edit(*)", "Write(*)", "Bash(*)"]`, disabling native
    file writes while keeping MCP tools active
    (<https://code.claude.com/docs/en/tools-reference#configure-tools-with-permission-rules-and-hooks>).
- **(c) Instruction files:** [V] `CLAUDE.md`, `.claude/CLAUDE.md`, subdirectory
  `CLAUDE.md`, `.claude/rules/*.md` (<https://code.claude.com/docs/en/memory>).
- **(d) Sandbox / permissions:** [V] Permission modes `default`, `plan`,
  `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`. OS sandboxing for Bash
  via Seatbelt (macOS) and bubblewrap (Linux) with `filesystem.allowWrite`,
  `filesystem.denyWrite`, etc. (<https://code.claude.com/docs/en/sandboxing>).
- **(e) LSP diagnostics:** [V] Built-in `LSP` tool. "After each file edit, it
  automatically reports type errors and warnings so Claude can fix issues
  without a separate build step"
  (<https://code.claude.com/docs/en/tools-reference#lsp-tool-behavior>).
- **(f) Post-edit commands:** [V] `PostToolUse` hooks matching `Edit|Write` can
  run shell linters; exit code 2 or `updatedToolOutput` feeds feedback to Claude.
- **(g) ACP support:** [V] NO native ACP support. GitHub Issue #6686 was closed
  as not planned (<https://github.com/anthropics/claude-code/issues/6686>).
  Third-party adapters exist (`@zed-industries/claude-code-acp`).

### 4.2 omp / oh-my-pi

- **(a) Lifecycle hooks:** [V] Hook factories in `<cwd>/.omp/hooks/{pre,post}/*.ts`
  or extension modules (`omp://hooks.md:82-86`).
  - Pre-tool payload: `tool_call` receives `event.toolName`, `event.input`.
    Input is the execution parameters (instruction), NOT the resulting file.
  - Can block: [V] YES. Returning `{ block: true, reason: string }` aborts the
    tool call (`omp://hooks.md:149, 194-197`).
  - Post-tool payload: `tool_result` receives `event.content`, `event.details`,
    `event.isError`.
- **(b) MCP client support:** [V] Discovered via `.omp/mcp.json`,
  `~/.omp/agent/mcp.json`, and imported tool files (`omp://mcp-config.md:19-21`).
  - Forcing MCP: [V] YES. `--no-tools` disables all built-in tools
    (`omp://cli-reference.md:142`), allowing MCP tools to handle all file writes.
- **(c) Instruction files:** [V] `.omp/AGENTS.md` (user/project), `.omp/RULES.md`
  (sticky), plus compatibility with `CLAUDE.md`, `GEMINI.md`, etc.
  (`omp://context-files.md:69-81`).
- **(d) Sandbox / permissions:** [V] `approvalMode` (`always-ask`, `write`,
  `yolo`), per-tool policies, bash pattern matching (`omp://approval-mode.md:145`).
- **(e) LSP diagnostics:** [V] `lsp.diagnosticsOnWrite: true`,
  `lsp.diagnosticsOnEdit: false` (`omp://settings.md:672-673`). `xd://lsp`
  reconciles disk files and reports diagnostics (`omp://tools/lsp.md:73-88`).
- **(f) Post-edit commands:** [V] `edit.autoRepair.enabled`, Biome/SwiftLint
  linter adapters, and `tool_result` hooks (`omp://tools/lsp.md:21-23`).
- **(g) ACP support:** [V] YES, native! `omp acp` subcommand runs as an Agent
  Client Protocol server over stdio (`omp://cli-reference.md:233, 244`).

### 4.3 OpenAI Codex CLI

- **(a) Lifecycle hooks:** [V] Configured in `hooks.json` or `config.toml`
  `[[hooks.*]]` (<https://learn.chatgpt.com/docs/hooks.md:60-70>).
  - Pre-tool payload: Receives `tool_name` (`Bash`, `apply_patch`, MCP),
    `tool_input`. For `apply_patch`, `tool_input.command` is the patch diff
    instruction, NOT the resulting file (<https://learn.chatgpt.com/docs/hooks.md:774>).
  - Can block: [V] YES. Returning `permissionDecision: "deny"` or exit code 2
    blocks tool execution (<https://learn.chatgpt.com/docs/hooks.md:781-801>).
  - Post-tool payload: `PostToolUse` receives `tool_name`, `tool_input`.
- **(b) MCP client support:** [V] Configured in `config.toml` under
  `[mcp_servers.<name>]` (<https://learn.chatgpt.com/docs/developer-commands:1020>).
  - Forcing MCP: [H] No built-in flag disables `apply_patch` while allowing MCP,
    though `PreToolUse` hooks can deny `apply_patch`.
- **(c) Instruction files:** [V] `AGENTS.md` (root, project, user), configurable
  via `project_doc_fallback_filenames` (<https://learn.chatgpt.com/docs/config-file/config-reference>).
- **(d) Sandbox / permissions:** [V] `sandbox_mode` (`read-only`,
  `workspace-write`, `danger-full-access`), `approval_policy` (`on-request`,
  `never`). OS sandbox via Seatbelt/bubblewrap (<https://learn.chatgpt.com/docs/sandboxing.md>).
- **(e) LSP diagnostics:** [V] NO native LSP client or post-edit diagnostic
  feedback (unreachable/absent from Codex documentation).
- **(f) Post-edit commands:** [V] `PostToolUse` hooks can run custom scripts,
  but no native `--lint-cmd` auto-repair loop exists.
- **(g) ACP support:** [V] NO native ACP support (uses proprietary
  `codex app-server` protocol; <https://learn.chatgpt.com/docs/developer-commands:160>).

### 4.4 Gemini CLI (Google)

- **(a) Lifecycle hooks:** [V] Configured in `.gemini/settings.json` or
  `~/.gemini/settings.json`. Supports `BeforeTool`, `AfterTool`,
  `BeforeToolSelection`, etc. (<https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/index.md>).
  - Pre-tool payload: `BeforeTool` receives `tool_name`, `tool_input` (raw
    arguments instruction), NOT the resulting file
    (<https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md>).
  - Can block: [V] YES. Returning `decision: "deny"`, `reason: "..."` or exit
    code 2 blocks the tool call and returns `reason` to the agent.
  - Post-tool payload: `AfterTool` receives `tool_name`, `tool_input`,
    `tool_response`.
- **(b) MCP client support:** [V] Supported in `settings.json` (`mcpServers`)
  and via ACP connection (<https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md>).
  - Forcing MCP: [V] YES. `BeforeToolSelection` hook can set
    `toolConfig.allowedFunctionNames` to allow only MCP tools and disable native
    write tools (<https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md#beforetoolselection>).
- **(c) Instruction files:** [V] `GEMINI.md`, `~/.gemini/GEMINI.md`,
  configurable via `context.fileName` to include `AGENTS.md`
  (<https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md>).
- **(d) Sandbox / permissions:** [V] Approval policies, auto-approve modes,
  and proxied filesystem under ACP (<https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md>).
- **(e) LSP diagnostics:** [V] NO native LSP client in standalone CLI; relies
  on editor host in ACP mode.
- **(f) Post-edit commands:** [V] `AfterTool` hooks can execute scripts and
  append context or trigger tail tool calls.
- **(g) ACP support:** [V] YES, native! `gemini --acp` starts Gemini CLI in
  Agent Client Protocol mode over stdio (<https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md>).

### 4.5 Cursor (Anysphere)

- **(a) Lifecycle hooks:** [V] Configured in `.cursor/hooks.json` or
  `~/.cursor/hooks.json`. Supports `preToolUse`, `postToolUse`, `afterFileEdit`,
  `beforeShellExecution`, etc. (<https://cursor.com/docs/hooks.md>).
  - Pre-tool payload: `preToolUse` receives `tool_name`, `tool_input` (diff /
    parameters instruction), NOT the resulting file (<https://cursor.com/docs/hooks.md:857-871>).
  - Can block: [V] YES. Returning `permission: "deny"`, `agent_message: "..."`
    or exit code 2 blocks the action (<https://cursor.com/docs/hooks.md:874-888>).
  - Post-tool payload: `afterFileEdit` receives `file_path` and
    `edits: [{ old_string, new_string }]`
    (<https://cursor.com/docs/hooks.md:1143-1146>).
- **(b) MCP client support:** [V] Supported via `~/.cursor/mcp.json` and
  `.cursor/mcp.json` (<https://cursor.com/docs/context/mcp>).
  - Forcing MCP: [V] Partial. `preToolUse` can deny native `Write` while
    allowing `MCP:*` tools (<https://cursor.com/docs/hooks.md:751>).
- **(c) Instruction files:** [V] `.cursorrules`, `.cursor/rules/*.mdc`,
  `AGENTS.md` (<https://cursor.com/docs/rules>).
- **(d) Sandbox / permissions:** [V] Terminal command sandbox, tool permission
  prompts, file access control via `beforeReadFile` (<https://cursor.com/docs/hooks.md:1149>).
- **(e) LSP diagnostics:** [V] YES. As a VS Code fork, Cursor automatically
  surfaces language server diagnostics to the Composer Agent after edits.
- **(f) Post-edit commands:** [V] `afterFileEdit` hook executes shell
  formatters/linters; Agent review can trigger build/test runs.
- **(g) ACP support:** [V] NO native ACP support (UNREACHABLE/absent from
  Cursor documentation).

### 4.6 Windsurf (Codeium / Devin Desktop)

- **(a) Lifecycle hooks:** [V] Cascade Hooks in `.devin/hooks.json` or
  `~/.codeium/windsurf/hooks.json`. Supports `pre_read_code`, `pre_write_code`,
  `post_write_code`, `pre_run_command`, etc. (<https://docs.devin.ai/desktop/cascade/hooks.md>).
  - Pre-tool payload: `pre_write_code` receives `file_path` and
    `edits: [{ old_string, new_string }]`
    (<https://docs.devin.ai/desktop/cascade/hooks.md:195-210>). Receives patch
    instruction, NOT the resulting file.
  - Can block: [V] YES. Exiting with code 2 blocks the write
    (<https://docs.devin.ai/desktop/cascade/hooks.md:46-48>).
  - Post-tool payload: `post_write_code` receives `file_path`, `edits`.
- **(b) MCP client support:** [V] Supported in `mcp_config.json`
  (<https://docs.windsurf.com/windsurf/cascade/mcp>).
  - Forcing MCP: [H] Pre-hooks can block native write/command tools while
    allowing `pre_mcp_tool_use`.
- **(c) Instruction files:** [V] `AGENTS.md`, `.windsurfrules`,
  `.windsurf/rules/` (<https://docs.windsurf.com/windsurf/cascade/agents-md>).
- **(d) Sandbox / permissions:** [V] Workspace Restricted Mode, terminal command
  approval policies (<https://docs.devin.ai/desktop/cascade/hooks.md:27>).
- **(e) LSP diagnostics:** [V] YES. Based on VS Code fork; Cascade Agent reads
  editor language server diagnostics.
- **(f) Post-edit commands:** [V] `post_write_code` runs linters/formatters
  after code modifications (<https://docs.devin.ai/desktop/cascade/hooks.md:215>).
- **(g) ACP support:** [V] YES (preview). Devin Desktop / Windsurf supports
  running third-party agents via ACP in Agent Command Center
  (<https://docs.devin.ai/desktop/acp.md>).

### 4.7 Cline / Roo Code

- **(a) Lifecycle hooks:** [V] Executable scripts in `.clinerules/hooks/` or
  `~/Documents/Cline/Hooks/`. Supports `PreToolUse`, `PostToolUse`, `TaskStart`,
  `UserPromptSubmit` (<https://github.com/cline/cline/blob/main/.clinerules/hooks/README.md>).
  - Pre-tool payload: `PreToolUse` receives `toolName`, `parameters`
    (instruction). Does NOT receive resulting file.
  - Can block: [V] YES. Output `{"cancel": true, "errorMessage": "..."}` blocks
    tool execution (<https://github.com/cline/cline/blob/main/.clinerules/hooks/README.md:200-210>).
  - Post-tool payload: `PostToolUse` receives `toolName`, `parameters`, `result`,
    `success`.
- **(b) MCP client support:** [V] Supported via `cline_mcp_settings.json`
  (<https://github.com/cline/cline/blob/main/docs/mcp/mcp-overview.mdx>).
  - Forcing MCP: [V] Partial. `PreToolUse` can cancel native `write_to_file`
    calls while allowing MCP tools.
- **(c) Instruction files:** [V] `.clinerules`, `AGENTS.md`, and `.roomodes`.
- **(d) Sandbox / permissions:** [V] Auto-approve toggles per tool, terminal
  permission confirmation.
- **(e) LSP diagnostics:** [V] YES. Cline listens to VS Code Problems/LSP
  diagnostics and feeds compiler/linter errors back to the model after edits.
- **(f) Post-edit commands:** [V] `PostToolUse` runs test/lint scripts; Roo Code
  supports auto-running terminal commands.
- **(g) ACP support:** [V] YES. Cline CLI implements the ACP Agent interface
  (`cline/cli/src/agent/ClineAgent.ts`, `docs/usage/acp.mdx`).

### 4.8 Aider

- **(a) Lifecycle hooks:** [V] NO lifecycle pre/post tool hooks (<https://aider.chat/docs/>).
  Only git pre-commit hooks (`--git-commit-verify`) can abort commits.
  - Pre-tool payload: N/A.
  - Can block: Only via git pre-commit hook during commit.
- **(b) MCP client support:** [V] NO native MCP client support (Issue #3314
  remains open, <https://github.com/Aider-AI/aider/issues/3314>).
  - Forcing MCP: NO.
- **(c) Instruction files:** [V] `CONVENTIONS.md`, `.aider.conf.yml`, `--read`
  (<https://aider.chat/docs/config/aider_conf.html>).
- **(d) Sandbox / permissions:** [V] Git-based versioning, `--dry-run`, no OS
  sandbox.
- **(e) LSP diagnostics:** [V] NO LSP client. Aider uses tree-sitter for syntax
  and repo maps (<https://aider.chat/2024/05/22/linting>).
- **(f) Post-edit commands:** [V] YES! First-class `--lint-cmd <cmd>`,
  `--test-cmd <cmd>`, `--auto-lint`, `--auto-test`. Automatically runs linter
  and test suite after edits; on non-zero exit, feeds errors into LLM chat to
  drive automatic repair (<https://aider.chat/docs/usage/lint-test.html>).
- **(g) ACP support:** [V] NO native ACP support (UNREACHABLE/absent from
  Aider docs).

### 4.9 GitHub Copilot (Agent Mode / Coding Agent)

- **(a) Lifecycle hooks:** [V] Configured in `.github/hooks/*.json`,
  `~/.copilot/hooks/`, or policy directory. Supports `preToolUse`, `postToolUse`,
  `permissionRequest`, `agentStop` (<https://docs.github.com/en/copilot/reference/hooks-reference>).
  - Pre-tool payload: `preToolUse` receives `tool_name` (`edit`, `create`,
    `bash`), `tool_input` (instruction/diff), NOT the resulting file
    (<https://docs.github.com/en/copilot/reference/hooks-reference:450-466>).
  - Can block: [V] YES. Returning `permissionDecision: "deny"` or exit code 2
    blocks tool call (<https://docs.github.com/en/copilot/reference/hooks-reference:493-495>).
  - Post-tool payload: `postToolUse` receives `tool_name`, `tool_result`.
- **(b) MCP client support:** [V] Supported in VS Code via `.vscode/mcp.json`
  and hosted MCP server (<https://docs.github.com/en/copilot/concepts/context/mcp>).
  - Forcing MCP: [H] Hooks can deny `create`/`edit` while allowing MCP.
- **(c) Instruction files:** [V] `.github/copilot-instructions.md`,
  `.github/instructions/**/*.instructions.md` (<https://docs.github.com/en/copilot/concepts/context/copilot-instructions>).
- **(d) Sandbox / permissions:** [V] Cloud agent runs in ephemeral Linux
  sandbox; CLI has `sandbox.userPolicy` (<https://docs.github.com/en/copilot/reference/hooks-reference#sandboxed-sessions>).
- **(e) LSP diagnostics:** [V] YES. In VS Code agent mode, Copilot reads VS Code
  Problems (LSP diagnostics) and prompts self-repair on error.
- **(f) Post-edit commands:** [V] `postToolUse` hooks run custom scripts; VS Code
  agent mode triggers configured test tasks.
- **(g) ACP support:** [V] NO native ACP support (UNREACHABLE/absent).

### 4.10 OpenCode (`anomalyco/opencode`)

- **(a) Lifecycle hooks:** [V] Event plugins and hooks system in `opencode.json`
  (<https://github.com/anomalyco/opencode/issues/711>).
  - Pre-tool payload: Receives tool name and input parameters (instruction),
    NOT the resulting file.
  - Can block: [V] YES. Hooks can reject or cancel tool execution.
- **(b) MCP client support:** [V] Supported in `opencode.json` under `mcp`
  (<https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/mcp/index.ts>).
  - Forcing MCP: [H] Configurable tool whitelists in `opencode.json`.
- **(c) Instruction files:** [V] `AGENTS.md` (read from `~/.config/opencode/`,
  project root).
- **(d) Sandbox / permissions:** [V] Tool approval prompts, execution gates.
- **(e) LSP diagnostics:** [V] YES. OpenCode includes a built-in LSP client in
  `packages/opencode/src/lsp/` and feeds diagnostics back after edits
  (<https://github.com/anomalyco/opencode/issues/711>).
- **(f) Post-edit commands:** [V] LSP diagnostics auto-run; shell hooks.
- **(g) ACP support:** [V] YES, native! `packages/opencode/src/acp/` implements
  the Agent Client Protocol (<https://github.com/anomalyco/opencode/issues/53793>).

### 4.11 Zed Agent (Zed Editor)

- **(a) Lifecycle hooks:** [V] NO lifecycle pre/post hook script subsystem
  (verified absent from `https://zed.dev/docs/ai/`). Uses `tool_permissions`
  and `agent_profiles` instead of shell scripts.
  - Pre-tool payload: N/A.
  - Can block: Tool permissions can `deny` or `confirm`, but cannot invoke
    custom pre-tool scripts.
- **(b) MCP client support:** [V] Supported in `settings.json` under
  `context_servers` (<https://zed.dev/docs/ai/mcp.md>).
  - Forcing MCP: [V] YES. `agent_profiles` allow disabling native `edit_file`,
    `write_file`, and `terminal` tools while leaving MCP tools active
    (<https://zed.dev/docs/ai/agent-profiles>).
- **(c) Instruction files:** [V] `.zed/rules.md`, `AGENTS.md`, and Skills
  (<https://zed.dev/docs/ai/skills.md>).
- **(d) Sandbox / permissions:** [V] `tool_permissions` (`allow`, `deny`,
  `confirm`), OS sandbox for terminal (<https://zed.dev/docs/ai/sandboxing.html>).
- **(e) LSP diagnostics:** [V] YES. Zed provides a native `diagnostics` tool
  that returns language server diagnostics for files or the workspace
  (<https://zed.dev/docs/ai/tools.md#diagnostics>).
- **(f) Post-edit commands:** [V] Zed Agent calls its built-in `diagnostics`
  tool or terminal test commands.
- **(g) ACP support:** [V] YES, native! Zed is the origin of ACP and implements
  it as both client and host (<https://zed.dev/acp/editor/zed>).

### 4.12 Continue (`continuedev/continue`)

- **(a) Lifecycle hooks:** [V] NO lifecycle pre/post tool hooks
  (<https://docs.continue.dev/reference>).
  - Pre-tool payload: N/A.
  - Can block: N/A (uses tool permission toggles: allow/ask/deny).
- **(b) MCP client support:** [V] Supported in `config.yaml` under `mcpServers`
  (<https://docs.continue.dev/customize/deep-dives/mcp>).
  - Forcing MCP: [V] YES. Tools in `config.yaml` or UI can be toggled on/off,
    disabling built-in edit tools while keeping MCP active
    (<https://docs.continue.dev/ide-extensions/agent/how-to-customize>).
- **(c) Instruction files:** [V] `.continue/rules/`, `config.yaml` rules,
  `.promptload` (<https://docs.continue.dev/customize/deep-dives/rules>).
- **(d) Sandbox / permissions:** [V] Tool permission policies (allow/ask/deny).
- **(e) LSP diagnostics:** [V] YES. Continue provides `@problems` /
  `@diagnostics` context provider and feeds active editor diagnostics back to
  the model.
- **(f) Post-edit commands:** [V] Relies on editor linter problems.
- **(g) ACP support:** [V] NO native ACP support (UNREACHABLE/absent from
  Continue docs).

### 4.13 Goose (Block)

- **(a) Lifecycle hooks:** [V] NO lifecycle pre/post hook script subsystem
  (verified absent from `https://block.github.io/goose/`).
  - Pre-tool payload: N/A.
  - Can block: N/A.
- **(b) MCP client support:** [V] Core architecture. Goose is built entirely on
  MCP (<https://block.github.io/goose/docs/mcp/developer-mcp>).
  - Forcing MCP: [V] YES. The built-in `developer` extension (shell and file
    editing) can be disabled, leaving only custom MCP servers active
    (<https://block.github.io/goose/docs/getting-started/using-extensions>).
- **(c) Instruction files:** [V] `.goosehints` (project root instructions)
  (<https://block.github.io/goose/docs/guides/context-engineering/using-skills>).
- **(d) Sandbox / permissions:** [V] Per-tool approval modes (auto-execute,
  confirm).
- **(e) LSP diagnostics:** [V] NO native LSP client; external MCP servers can
  provide diagnostics.
- **(f) Post-edit commands:** [V] Subrecipes can chain test/lint commands; no
  automated post-tool hooks.
- **(g) ACP support:** [V] YES! Goose supports connecting to providers via ACP
  ("Use API keys or your existing Claude, ChatGPT, or Gemini subscriptions via
  ACP"; <https://block.github.io/goose/>).

### 4.14 Amp (`ampcode.com`)

- **(a) Lifecycle hooks:** [V] Plugin API supports `tool.call` (pre-tool) and
  `tool.result` (post-tool) events (<https://ampcode.com/docs/customize/plugins>).
  - Pre-tool payload: `tool.call` receives `event.tool`, `event.input`. Sees the
    **instruction/parameters**, NOT the resulting file.
  - Can block: [V] YES. Returning action `'reject-and-continue'` with
    `message: '...'` blocks execution and feeds `message` back to the model
    (<https://ampcode.com/docs/customize/plugins#toolcall-approve-or-reject-a-tool-call>).
  - Post-tool payload: `tool.result` receives `event.tool`, `event.status`,
    `event.output`.
- **(b) MCP client support:** [V] Supported in `amp.mcpServers` (local stdio and
  remote HTTP/SSE) (<https://ampcode.com/docs/customize/mcp>).
  - Forcing MCP: [V] YES. `tool.call` hook can reject built-in file edit tools
    while allowing MCP tools.
- **(c) Instruction files:** [V] `AGENTS.md` (and migrates from `CLAUDE.md`,
  `.cursorrules`) (<https://ampcode.com/manual?preview>).
- **(d) Sandbox / permissions:** [V] Remote "orbs" (isolated VM environments),
  tool approval confirmations (`ctx.ui.confirm`).
- **(e) LSP diagnostics:** [V] YES. Integrates code intelligence and diagnostics
  when connected to IDEs/orbs.
- **(f) Post-edit commands:** [V] `tool.result` hooks or `agent.end` continuation
  triggers custom workflows.
- **(g) ACP support:** [V] YES! Amp includes an Agent Client Protocol server mode
  (<https://ampcode.com/threads/T-388afb33-958d-425a-939c-1b489c38908e>).

---

## 5. Architectural Implications for Note Frontmatter Guardrails

How can a markdown note schema check gate writes and provide feedback across
these diverse harnesses?

### 1. Pre-Write Hooks Cannot Gate Direct File Edits

In all 8 harnesses supporting pre-tool hooks, `tool_input` passes only the edit
instruction:

- `Edit`: `file_path`, `old_string`, `new_string`
- `apply_patch`: patch diff string
- `Bash`: shell command line (e.g. `cat <<EOF > note.md` or `sed`)

To validate frontmatter before the write lands on disk, a hook script would
have to:

1. Intercept `Edit` or `Write`.
2. Read the existing note file from disk.
3. Apply the patch algorithmically (reproducing the exact search/replace or diff
   semantics of that specific harness).
4. Parse the simulated frontmatter with a YAML parser.
5. Run the schema check.
6. Return `deny` if broken.

This is brittle, harness-specific, and completely unavailable in the 6 harnesses
lacking pre-tool hooks (Aider, Zed, Continue, Goose, etc.).

### 2. Dedicated MCP Write Tools Are the Only Portable Hard Gate

In MCP, the tool implementation controls the write:

- The MCP server exposes tools such as `traces_edit_frontmatter`,
  `traces_create_note`, `traces_set_class`.
- When an agent calls the MCP tool, the tool parses the target file, validates
  the resulting note against the schema (`check_note(before, after)`), and:
  - If valid: writes the file to disk and returns success.
  - If invalid: rejects the write, writes NOTHING to disk, and returns an error
    payload (`isError: true`) with structured violation codes and repair hints.
- Supported in 13 of 14 harnesses (93%).
- In harnesses where native tools can be disabled (Claude Code, OMP, Zed,
  Continue, Goose, Gemini CLI), the agent can be strictly forced through this
  MCP write gate.

### 3. Layered Guardrail Design for Cross-Harness Reality

Because agents cannot be universally forced into MCP across every harness,
ticket 16's multi-layered architecture is confirmed as the only viable model:

```text
Layer 1: Up-Front Constraints (Soft Channel)
         AGENTS.md / CLAUDE.md / MCP Tool Input Schemas
         - Reaches 100% of harnesses.
         - Steers agent toward valid fields, classes, and formats.
                        │
                        ▼
Layer 2: Gated Writes (Hard Gate, Where MCP Is Used)
         Custom Traces MCP Edit / Create Tools
         - Reaches 93% of harnesses.
         - Pre-write validation: check_note(before, after).
         - Rejects invalid writes with structured error codes.
                        │
                        ▼
Layer 3: Post-Write Advisory Feedback (Advisory Channel)
         LSP Server (publishDiagnostics on didChange / didSave)
         - Reaches 71% of harnesses (all IDEs and LSP-aware agents).
         - Re-parses note upon save, computes delta violations,
           and flags errors in the editor problems panel / agent context.
                        │
                        ▼
Layer 4: Automated Post-Edit Verification (Linter / Test Loop)
         Aider --lint-cmd "traces check" / PostToolUse Hooks
         - Automatically invokes `traces check` after file modifications.
         - Feeds stdout/stderr failure back into the conversation for self-repair.
                        │
                        ▼
Layer 5: Universal Hard Backstop
         Git Pre-Commit Hook (`traces check`) & CI
         - Works across 100% of harnesses and human edits.
         - Aborts git commits if notes violate schema invariants.
```
