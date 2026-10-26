# Multi-root / multi-workspace behavior vs the single Project Root model

Type: grilling
Status: open
Blocked by: 10
Assigned: agent

## Question

Traces today uses a single Project Root. It anchors on `.traces/` and discovers the root by walking up from cwd (`src/config/discovery.rs`). It has a two-tier Configuration Scope: global defaults, overridden by local project config. Trust Verification checks a companion hash before the config loads. LSP's `workspace/workspaceFolders` supports multiple simultaneous roots in one client session. Decide this after the analysis-host design (ticket 10) is settled.

Decide these points:

- Decide whether the LSP server runs one analysis host per workspace folder. That means N independent `WorkspaceIndex`/`SchemaService` instances, each with its own Trust Verification and Config Scope. Or decide whether the server needs exactly one workspace folder and errors or degrades otherwise.
- Decide how Trust Verification works with LSP initialization. The CLI today refuses outright to load an Untrusted or Stale local config. Choose the LSP-mode equivalent: block initialization for that folder; show a diagnostic or notification that prompts the user to run `traces trust`; or never auto-trust from the LSP path, for security reasons.
- Handle `workspace/didChangeWorkspaceFolders`: adding a root at runtime starts that root's analysis host; removing a root tears it down.
- Decide cross-root reference resolution. May a wikilink, tag, or query in one workspace folder resolve against another open workspace folder? Or does each folder get a fully independent analysis host with no cross-root visibility, matching the single-Project-Root model's isolation? Decide on the merits: consider whether real PKM usage (a personal vault and a shared team vault open together) benefits from cross-root resolution. Do not default to isolation only because it is the smaller change from today's single-root CLI.

## Inputs from 26 (2026-10-18)

- **Confinement predicate**: prepareRename returns `Ok(null)` when the resolved target fails the confinement check. The check is one predicate threaded through `resolve`, and the nav spine consults it. Pre-30 single root: no special case beyond the predicate existing. Multi-root is where it bites. `Ok(null)` (not an error) is the settled refusal form.
- **Uri/path multi-root matching**: match folder to document by **decoded path**, not by string equality. Percent-encoding case variants, drive-letter case, and symlink variants are documented distinct-key hazards. 26 pins per-spelling keys plus path-based cross-referencing as the v1 answer; gopls-style normalization is the upgrade.
