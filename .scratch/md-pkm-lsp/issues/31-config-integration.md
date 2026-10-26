# Config integration: .traces/config.toml vs LSP initializationOptions/workspace configuration

Type: grilling
Status: open
Blocked by: 09
Assigned: agent

## Question

Decide how the existing two-tier `Config File` system integrates with LSP-native configuration surfaces. The two tiers are `.traces/config.toml` (local) and `~/.config/traces/config.toml` (global), both under `src/config/`. Decide this once the framework choice (ticket 09) is settled.

Decide these points:

- Decide whether the LSP reads `.traces/config.toml` directly through the existing `ConfigService`/discovery pipeline. That treats LSP mode as just another `Traces` entrypoint that reuses config loading verbatim, per the "reuse existing services" constraint. If so, limit LSP `initializationOptions`/`workspace/configuration` to editor-integration concerns that have no `.traces/config.toml` equivalent. Examples: `enableLinkNavigation`-style per-feature toggles, which mirror rumdl's own settings surface per ticket 03.
- Decide `workspace/didChangeConfiguration` handling. Does a client-side settings change (LSP-only toggles) apply live without a restart? Apply it separately from `.traces/config.toml` changes, which go through the existing Config File Store's out-of-band-change detection (`Companion Hash`).
- Decide whether editing `.traces/config.toml` while the LSP runs needs special handling. The LSP could treat that TOML file as "just another open document". Decide whether language intelligence for the config file itself is in scope, or explicitly excluded.

## Inputs from 25 (2026-10-15)

- **`[lsp.diagnostics]` lands here (25 Q7-R/Q15-R2)**: project-level per-code severity overrides in zk's shape (dotted diagnostic-code keys map to `hint|info|warn|error|off`). Merge them into 25's `OverrideSet` (editor O1 overrides win over project TOML). 31 must **extend the `OverrideSet` type** rather than invent a parallel config path. That type is a thin code-to-override map built at the LSP boundary, and it reuses the `merge_optional` precedence idiom from `src/config/builder.rs:208-213`. `deny_unknown_fields` on raw config blocks O1-key smuggling until this lands.
- **`[tasks.diagnostics]` lands here** (23's toggle valve, per its Q4 note).
- **`[schemas.diagnostics]` stays an emission gate** (25 Q7-R): `enabled` gates whether schema diagnostics emit. Severity resolution is the registry's job, not this section's.
- The schema-dir `didChangeWatchedFiles` trigger (22's `reset_engine` note) also lands here.

## Inputs from fileclass research (2026-10-21)

- (amended 2026-10-21 by fileclass research) fileclass's audit trigger model (sessionly + on-demand + rename, explicitly not per-rebuild) with fingerprint suppression is a production-tested shape for schema-dir watching ([tool-fileclass](../research/tool-fileclass.md)).

## Inputs from 27 (2026-10-22)

- LSP feature toggles: 27's `[lsp]` table (per-feature `Option<bool>`; lens default false, rest default true) is read at initialize for capability advertisement. 31 owns runtime toggles via live config reload: handler-level empty results or method_not_found, sticky-until-restart for engine-reset-class changes per 22's precedent. 27 does not promise runtime re-advertisement of capabilities until 31 lands.
