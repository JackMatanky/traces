# Multi-root / multi-workspace behavior vs the single Project Root model

Type: grilling
Blocked by: 10

## Question

Traces' existing model is a single Project Root (`.traces/`-anchored, discovered by walking up from cwd — `src/config/discovery.rs`) with a two-tier Configuration Scope (global defaults overridden by local project config, verified by Trust Verification against a companion hash before loading). LSP's `workspace/workspaceFolders` supports multiple simultaneous roots in one client session. Decide, once the analysis-host design (ticket 10) is settled:

- Whether the LSP server runs one analysis host per workspace folder (N independent `WorkspaceIndex`/`SchemaService`/etc. instances, each with its own Trust Verification and Config Scope) or requires/assumes exactly one workspace folder and errors/degrades otherwise.
- How Trust Verification interacts with LSP's initialization flow: the CLI today refuses to load an Untrusted/Stale local config outright — decide the LSP-mode equivalent (block initialization for that folder? show a diagnostic/notification prompting the user to run `traces trust`? never auto-trust from the LSP path for security reasons?).
- `workspace/didChangeWorkspaceFolders` handling: adding/removing a root at runtime spins up/tears down that root's analysis host.
- Cross-root reference resolution: should a wikilink/tag/query in one workspace folder ever resolve against another open workspace folder, or does each folder get a fully independent analysis host with no cross-root visibility (matching the existing single-Project-Root model's isolation)? Decide this on its own merits — e.g. whether real PKM usage patterns (a personal vault plus a shared team vault open together) would benefit from cross-root resolution — rather than defaulting to isolation because that's the smaller change from today's single-root CLI.

## Inputs from 26 (2026-10-18)

- **Confinement predicate**: prepareRename returns `Ok(null)` when the resolved target fails the confinement check; it is one predicate threaded through `resolve`, consulted by the nav spine (pre-30 single root: no special case beyond it existing; multi-root is where it bites). `Ok(null)` (not an error) is the settled refusal form.
- **Uri/path multi-root matching**: folder-to-document matching by **decoded path**, not string equality; percent-encoding case variants, drive-letter case, and symlink variants are documented distinct-key hazards (26 pins per-spelling keys + path-based cross-referencing as the v1 answer, gopls-style normalization as the upgrade).
