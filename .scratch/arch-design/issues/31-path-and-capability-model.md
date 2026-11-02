# What owns root confinement and path identity?

Type: grilling
Status: open
Blocked by: 9, 16, 21, 12

## Question

- Define one owner for the invariant: all file access stays inside the
  project root.
- Evaluate validated newtypes at boundaries versus a capability root handle.
- Decide how `FileMeta`, index rows, query/template paths and CLI errors carry
  confined versus raw paths.
- Decide the TOCTOU stance: accepted for a single-user CLI, or closed.
- Decide template path *inspection* and `DryRun` consistency with the writer.
- Decide the persisted path form in redb (OS bytes versus normalized UTF-8) and
  cache portability.
- Decide `DirTree`/`file_tracker` path usage.

Facts: `survey/root-primitives.md` H7/§2, `survey/template.md` H4/H5,
`survey/cross-cutting.md` H4, `src/path.rs:64,146,181-190,340-437`,
`src/template/writer.rs`.

Alternatives: capability `ProjectRoot` handle; newtype propagation;
boundary-only validation with a single audited fs facade; keep as is. Include
Windows reality from `research/09`.

Inputs: `research/09`, compat envelope (cache portability irrelevant if the
cache is disposable).

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Class
binding `paths` are vault-relative globs with portable `/` separators and `!`
exclusions. They match against the carried relative path of the note. The glob
dialect compiles `books/` to `books/**`. `check_note` takes a path on each
side. A path can be absent in stdin or dry-run mode. In that case, path
bindings do not apply.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE
identifies a note by a portable, extensionless, `/`-separated key. Watcher keys
normalized to `/` on Windows (iwec CHANGELOG). IWE checks workspace escape
in one place before touching disk or graph (`crates/diwe/src/fs.rs`,
`key_escapes_workspace`, `workspace_document_path`) [V]. This matches
the portable-path lesson of research 9. IWE offers no capability handle.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
The adapter supplies a typed OS start path and a fixed base for relative
overrides. Distinguish canonical root identity for workspace admission/index
coordination from a note's portable vault-relative path. Also distinguish
it from the confined filesystem capability.

Multi-root readiness does not let two roots aliasing one directory write
independent indexes. Resolve symlink/alias identity and TOCTOU policy here and
in tickets 25/32. Do not presume a registry or new capability.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- Unix and Windows write different path bytes into the store (`src/path.rs:349-391`). Until this ticket gives one portable form, the store identity holds a platform family. A store does not move between families. Remove the platform family when the form is portable.
- This ticket has no Unicode normalization rule. Until it has one, NFC and NFD names are different files.
- The store keeps the persisted path form from this ticket. A change of the form changes the stored rows and needs a new format epoch.
