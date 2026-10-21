# What owns root confinement and path identity?

Type: grilling
Status: open
Blocked by: 9, 12

## Question

One owner for the invariant "all file access stays inside the project root":
validated newtypes at boundaries vs a capability root handle; how `FileMeta`,
index rows, query/template paths and CLI errors carry confined vs raw paths;
TOCTOU stance (accepted for a single-user CLI, or closed?); template path
*inspection* and `DryRun` consistency with the writer; persisted path form in
redb (OS bytes vs normalised UTF-8) and portability of the derived cache;
`DirTree`/`file_tracker` path usage.

Facts: `survey/root-primitives.md` H7/§2, `survey/template.md` H4/H5,
`survey/cross-cutting.md` H4, `src/path.rs:64,146,181-190,340-437`,
`src/template/writer.rs`.

Alternatives: capability `ProjectRoot` handle; newtype propagation;
boundary-only validation with a single audited fs facade; keep as is. Include
Windows reality from `research/09`.

Inputs: `research/09`, compat envelope (cache portability irrelevant if the
cache is disposable).

## Inputs from the schema typing decision

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). Class
binding `paths` are vault-relative globs with portable `/` separators and `!`
exclusions, matched against the note's carried relative path (the existing glob
dialect, `books/` compiles to `books/**`). `check_note` takes a path on each
side; a path may be absent (stdin, dry-run), in which case path bindings do not
apply.
