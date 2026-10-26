# Attack Lifecycle Host Inputs: n3

## Analysis of Current Behaviour (src/dirs.rs, src/env_vars.rs)

| Variable | Behaviour | Issue with Q12 Proposal |
| :--- | :--- | :--- |
| `TRACES_STATE_DIR` | Used if set and non-empty. Defaults to `STATE_HOME/traces`. | If relative, it joins with `/` or current process working directory (depending on implementation, `dirs.rs` currently joins it with `STATE_HOME` implicitly or `PathBuf`). If relative, it must be rejected or made absolute. |
| `TRACES_CEILING_DIRS` | `split_paths` used. Filters out empty components. | `PathBuf` can be relative. Should be absolute for security. |
| `XDG_*` | Follows XDG spec: used if set and non-empty. | Consistent with proposal. |
| Non-Unicode | `var_os` used, `PathBuf` handles non-Unicode paths correctly on Unix. | Acceptable. |
| `HOME` (unset) | Defaults to `/` (Unix) or `C:\` (Windows). | Acceptable/Standard. |

### Required Changes for Q12

1. **Strict Path Resolution**: `ConfigService` must normalize all `TRACES_*` paths to absolute upon startup (`canonicalize`).
2. **Rejection**: Adapter must return a hard error if any `TRACES_*` path is relative *and* `ConfigLocator` is used in a context that requires absolute roots.

## Minimal Projection Types

`HostInputs` should be an opaque aggregate struct projected into specific services:

- **`TrustLocator`**: `ceilling_dirs`, `ignored_dirs`.
- **`ConfigStore`**: `state_dir`, `tracker_store`, `trust_store`.
- **`ConfigLocator`**: `ceiling_dirs`, `global_config_dir`.

## ConfigService::check(&Config) Semantics

**Algorithm**:

1. **Re-verify Local Config**: Perform `trust_check` (read local config bytes, re-hash, compare with recorded Companion Hash).
2. **Verify Trust Status**: Ensure the workspace root is still recorded in `TrustStore`.
3. **Global Freshness**: Hash current global config file bytes.
4. **Result**: `current` (if hash matches), `changed` (if local/global hash differs), `error` (if trust revoked or config gone).

- **TOCTOU**: `check` verifies *bytes*, but the workspace might have changed templates *after* `check`. ADR-0002 root perimeter bounds this by limiting file I/O to the trusted root. The stamp *cannot* be forged because the Companion Hash is based on the verified config file content (BLAKE3).
- **Staleness**: The stamp includes the config file hash. If the config changes, the stamp is stale.

## Alternative Designs

### Alternative 1: `ConfigSource` Value (Cloneable/Reloadable)

- **Shape**: `struct ConfigSource { bytes: Arc<String>, hash: Blake3Digest }`
- **Benefit**: Decouples the config *source* from the `ConfigService` loader. The host holds `ConfigSource` and passes it to builders.

### Alternative 2: Functional `resolve(HostInputs, Snapshot)`

- **Shape**: `fn resolve(inputs: &HostInputs, snapshot: &FileSystem) -> Result<Config>`
- **Benefit**: Makes resolution pure/deterministic, avoiding ambient state mutations.

### Comparison

*Proposed "Stamp + Check"* is best for performance, as it avoids re-parsing if bytes match. Functional resolution is best for testing but potentially slower if it requires full re-parsing on every check.

---

### Summary

1. Confirmed `src/dirs.rs` uses `std::env::var_os` and `PathBuf::join` for `TRACES_STATE_DIR`, which is unsafe if relative.
2. `ConfigService::check` relies on content hashing, which is robust against TOCTOU if `check` is called *before* workspace access.
3. Recommended design: Host holds `HostInputs` as a configuration object, `ConfigService` takes it by reference to perform discovery, and the stamp includes hashes of both local and global configs.

### Verdict Table

| Decision Id | Verdict | Required Change | Conflicts |
| :--- | :--- | :--- | :--- |
| Q12 | Holds with change | Reject relative paths | None |
| Q10 | Holds | None | None |
| Q11 | Holds | None | None |

Path: `.scratch/arch-design/research/22-prep/n3-attack-lifecycle-host-inputs.md`
