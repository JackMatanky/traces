# Config integration: .traces/config.toml vs LSP initializationOptions/workspace configuration

Type: grilling
Blocked by: 09

## Question

Decide how the existing two-tier `Config File` system (`.traces/config.toml` local, `~/.config/traces/config.toml` global — `src/config/`) integrates with LSP-native configuration surfaces, once the framework choice (ticket 09) is settled:

- Does the LSP read `.traces/config.toml` directly via the existing `ConfigService`/discovery pipeline (treating LSP-mode as just another `Traces` entrypoint that reuses config loading verbatim, per the "reuse existing services" constraint), with LSP `initializationOptions`/`workspace/configuration` limited to purely editor-integration concerns (e.g. `enableLinkNavigation`-style per-feature toggles, mirroring rumdl's own settings surface per ticket 03) that have no `.traces/config.toml` equivalent?
- `workspace/didChangeConfiguration` handling: does a client-side settings change (LSP-only toggles) get applied live without a restart, separately from `.traces/config.toml` changes (which go through the existing Config File Store's out-of-band-change detection, `Companion Hash`)?
- Does editing `.traces/config.toml` itself while the LSP is running need special handling (it's a TOML file the LSP could also treat as "just another open document" — decide if any language intelligence for the config file itself is in scope, or explicitly excluded)?

## Inputs from 25 (2026-10-15)

- **`[lsp.diagnostics]` lands here (25 Q7-R/Q15-R2)**: project-level per-code severity overrides in zk's shape (dotted diagnostic-code keys → `hint|info|warn|error|off`), merged into 25's `OverrideSet` (editor O1 overrides win over project TOML). 31 must **extend the `OverrideSet` type** (thin code→override map built at the LSP boundary, reusing the `merge_optional` precedence idiom from `src/config/builder.rs:208-213`) rather than invent a parallel config path; `deny_unknown_fields` on raw config blocks O1-key smuggling until this lands.
- **`[tasks.diagnostics]` lands here** (23's toggle valve, per its Q4 note).
- **`[schemas.diagnostics]` stays an emission gate** (25 Q7-R): `enabled` gates whether schema diagnostics emit; severity resolution is the registry's job, not this section's.
- Schema-dir `didChangeWatchedFiles` trigger (22's `reset_engine` note) also lands here.

## Inputs from fileclass research (2026-10-21)

- (amended 2026-10-21 by fileclass research) fileclass's audit trigger model (sessionly + on-demand + rename, explicitly not per-rebuild) with fingerprint suppression is a production-tested shape for schema-dir watching ([tool-fileclass](../research/tool-fileclass.md)).
