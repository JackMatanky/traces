# Who owns config-derived types, trust/tracking state, and the config lifecycle?

Type: grilling
Status: resolved
Blocked by: 21, 12

## Question

Resolve the boundary of the config subsystem:

- (a) Check why `task.rs`/`tag.rs`/`field.rs` live at the crate root. Decide
  where `TaskStatusMap`, the tag filters and the status/emoji parsing belong to
  config, note, or a leaf.
- (b) Check the upward `note -> config` edge (`src/note/parser/input.rs:9`).
  Decide whether the parsers take primitives or a typed parser-config.
- (c) Decide the residual `ConfigDigests` surface now that drift detection
  landed (main @ `207cc3fb`). 10 of 15 per-setting digests feed the index-owned
  `IndexFingerprint` (`src/index/fingerprint.rs:158`). `Config::hash`/`compose`
  (pinned `traces.config.v1` framing), the root/templates/schema-directory
  digests, and the 380-line `syn` source-scan test (`syn` dev-dep) still have no
  production consumer. Every `IndexerService` construction computes and discards
  5 root/templates/schema-directory digest values. Choose one option: retire,
  keep as a general config fingerprint, or fold into the index.
- (d) Decide the design of the twin `TrustStore`/`TrackerStore` over
  `FilePathTracker`. Decide the two-store ordering invariant.
- (e) Choose between the `ConfigFile` phantom typestate and a direct pipeline.
- (f) Decide where `file_tracker`/`dirs`/`env_vars` live.
- (g) Decide the ownership of the scaffolding (`cli::init`).

Facts: `survey/config-dialog.md`, `survey/root-primitives.md` §3-5,
`survey/note.md` H4, `src/file_tracker.rs` has exactly two callers (verify in
`research/01`), ADR-0002.

Alternatives: List the alternatives for each sub-question, including keep-as-is.
Record which sub-questions are independent so the spec can sequence them.

Inputs: execution-context decision, `research/04`, `research/05` (its
digest-vs-fingerprint lesson; `207cc3fb` realized that lesson; see
`reverification.md`), `research/01`.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE keeps
one `.iwe/config.toml` with `deny_unknown_fields`, a `version` key and in-place
migrations through `toml_edit` that preserve user comments
(`crates/diwe/src/config.rs`, `migrate`) [V]; it still had to rename a whole
table (`[library]` to `[workspace]`, v0.25.0). Schema binding lives in this host
config (`[schemas.<name>].match`), not in the schema files; [ticket
20](./20-schema-note-typing-and-validation.md) chose class-owned `[binding]`.
The only transferable point for this ticket is the comment-preserving migration
technique if the config format ever changes.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). The
adapter snapshots typed OS paths, discovery inputs and overrides at workspace
open; specify precedence, an explicit base for relative paths (or reject
relative overrides), and where config-derived policy lives. Do not use a
Unicode-only environment map for paths. Hash-verify local config before parsing
and revalidate trust/content at each new held-workspace request; admitted
requests retain their immutable generation. Global config freshness is separate.
Supply one workspace's policy to its services, not a process-global value, so
different roots can coexist. The config surface also owns `DateConfig`
integration (zone source, week rules and opt-in bare-year filenames); decide its
syntax with ticket 27, not here.

## Answer

A grilling session resolved this ticket on main @ `8064cf76`. The evidence is in
[brief.md](../research/22-prep/brief.md) and
[o-stress-test-findings.md](../research/22-prep/o-stress-test-findings.md). Do
not cite these agent files in `22-prep/`: `n1-conflicts-resolved-tickets.md`,
`n2b-conflicts-open-28-36.md`, `n5-attack-trust-init-typestate.md` and
`n6-holistic-alternatives.md`. Their content is unreliable.

### Decision: shared types and the parser input (a, b)

- Keep `task.rs`, `tag.rs` and `field.rs` at the crate root. They are shared
  vocabulary. Neither `config` nor `note` owns them.
- Do not move `TaskStatus`. `TaskListItem` stores it, and `index` and `query`
  use it.
- Keep `FieldName` as a leaf. `config` and `schema` both use it.
- Put the file splits in the findings ledger. They change no owner.
- Ticket 23 decides the final place of `TaskStatus*` with `TaskListItem`.
- Make `note` define its own parser-input type. It holds the status map, the tag
  filters and the tags field name.
- Make `note` import nothing from `config`.
- Build the parser input one time for each indexing run. The parser borrows it.
  Do not clone it for each file.
- Ticket 23 decides the final field set.

### Decision: index policy and digests (c)

- Delete `ConfigDigests`, `Config::hash` and the digests that no code uses.
- Make `index` own `IndexPolicy`. It holds the parser input, `class_field` and
  the ignore policy. It holds the filename-period policy only if the index
  stores parsed periods.
- Make `IndexerService` take `IndexPolicy`. Do not give it `&Config`.
- Make `IndexPolicy::fingerprint()` the only source of the fingerprint.
- Use one projection from `Config` and `HostInputs` to `IndexPolicy`.
  Destructure `Config` there without a rest pattern. Name each ignored field
  with `_`. A new setting then fails to compile until a developer classifies it.
- Let `src/index/**` import `Config` in the projection only. Add a test that
  scans the imports.
- Remove `title`, `aliases` and the date keys from the fingerprint. No
  production code reads them. An index-time read must add its key to
  `IndexPolicy`.
- Keep one `syn` scan test. It guards the single destructure against a rest
  pattern.
- Do not put the time zone in the fingerprint.
- This decision sets ownership and direction only. Ticket 25 decides the
  content, the granularity, the parser version, the format version and the racy
  timestamps.
- The next run after the format change rebuilds the index one time. The rebuild
  is automatic.

### Decision: trust and tracking stores (d)

- Keep the Trust Store and the Tracker Store as two stores.
- Key the Trust Store by the canonical root. A config-path key makes two
  repositories collide when their `.traces` links point to one directory.
- Make `ConfigService` the only writer. Make the store write methods private.
  Give the CLI read-only views.
- Remove root-only grants and `MissingBaseline`. Three trust states remain:
  Untrusted, Stale and Trusted.
- Make every grant track the config path first. Then the Trust Store records the
  grant.
- Keep this invariant: the tracker holds the config path of each trusted root,
  unless the config file is gone. Add a regression test.
- Keep `traces tracked`.
- Repair `FilePathTracker::record`. `exists()` follows symlinks, so a dangling
  entry fails with `EEXIST`. Use `symlink_metadata`.
- Write each Companion Hash file atomically: write a temporary file, then rename
  it.
- Existing config grants stay valid.

### Decision: typestate (e)

- Keep `ConfigFile<Source, State>`.
- Make `into_trusted` take the store. A store method records the path
  best-effort. Then it runs Trust Verification.
- Make that store method the only producer of `Trusted { content, digest }`.
  Keep the field private in `trust.rs`.
- Canonicalize the root one time. Carry it in the file type.
- Add `compile_fail` doctests for two cases: parse without trust, and trust a
  global file. A scratch crate compiled both.
- Do not add a runtime provenance enum. `IsLocal` and `IsGlobal` already encode
  the source.

### Decision: host inputs (f)

- Make each adapter build one `HostInputs` when it opens a workspace. It holds
  the start path, the state home, the config home, the ceiling directories and
  the ignored directories. It stores them as OS strings.
- Give each service only the projection it needs.
- Make `ConfigService` own its locator and its stores.
- Change `trust_subjects` to a `&self` method.
- Give trust discovery and the index walker one shared ignore value.
- Make `file_tracker` private to `config`.
- Remove the `LazyLock` globals in `dirs` and `env_vars`.
- Use one audited constructor to read the OS. Enforce this with clippy
  `disallowed-methods` and exact paths.
- Keep the path conventions (XDG, and macOS Application Support).

### Decision: scaffolding (g)

- Make `init` a core use case with two functions. `plan` checks the request.
  `apply` writes the files and keeps an undo journal.
- Make both functions take plain data. Do not pass `DialogProvider` into core.
  The CLI prompts again when `plan` fails.
- Write the default config text as a constant. A test parses it into `RawConfig`
  and resolves it to the builder defaults. This repairs the templates default
  mismatch.
- Do not add starter templates.

### Decision: admission, global config and path variables

- Do not hold a workspace object in core. Ticket 30 owns it.
- Make `Config` carry a stamp. The stamp holds the local digest and an optional
  global digest.
- Name the admission function `ConfigService::admit(&Config)`. Do not name it
  `check`. `traces check` and `check_note` use that word.
- Make `admit` run Trust Verification again for the bound root and the local
  path. It reads the content again. It does not discover again. It does not use
  a stat shortcut.
- Make `admit` return current, changed or an error.
- Treat a nearer config that appears later as a different workspace. The host
  decides what to do.
- Create a new `ConfigService` when a host input changes. Host policy is not in
  the stamp.
- Let an admitted request keep its `Arc<Config>`.
- Do not gate the global config. Hash its bytes into the stamp. Use a fixed
  value when the file is absent.
- Review the global gate again if one daemon host serves many repositories.
- Make each relative value in a host path variable an error that names the
  variable. The variables are `TRACES_STATE_DIR`, `TRACES_CEILING_DIRS` and
  `XDG_*`. Today the code accepts them and resolves them against the current
  directory.

### Decision: trust after init

Init trusts the new workspace only if both conditions are true:

1. This run created `.traces/` with an atomic `create_dir`, and init wrote each
   file in it.
2. This run created the template directory, or the template directory already
   existed and is empty.

The template directory can be anywhere under the root. Trust covers the whole
root. If a condition is false, init leaves the workspace untrusted. Init then
prints the `traces trust` command.

- Make core return what it created.
- Make the host apply trust with `ConfigService::trust`. That function tracks
  the path first, then trusts it.
- If trust fails after a successful init, warn the user. Do not roll back.
- Precedent: `direnv edit` allows the file after the user edits it.

### Decision: format, dates and merge

- Keep `deny_unknown_fields`. Add no `version` key until the first breaking
  change. Then use a migration that keeps comments, as IWE does.
- Make `config` own and parse the `[date]` table into `DateConfig`. Ticket 27
  decides the syntax.
- Use this precedence for the time zone: host override, then config, then
  system.
- Keep the merge. Apply the global config first, then the local config, field by
  field. A non-empty local task list replaces the global task list. Then add the
  built-in statuses.
- Resolve local paths against the project root. Resolve global paths against the
  global config directory.

### Alternatives considered

Each sub-question kept "keep as is" as an option.

- Root types: move the status types into `note` or `config`. Reject: it adds an
  edge or pre-empts ticket 23. Put the types in a core crate. Reject: ticket 32
  decides it.
- Parser input: pass primitives, or pass `&Config`. Reject both.
- Digests: trim the dead digests only (the fallback). Let `config` own an
  `IndexView`. Use one combined hash. Reject all three.
- Stores: delete the Tracker Store. Add a `TrustManager` type. Merge both into
  one root-keyed store. Key one store by config path. Reject all four. The
  merged store is viable but abandons existing grants and rewrites 18 tests. The
  config-path key is unsafe.
- Typestate: use a sealed type without typestate, or plain functions. Reject
  both.
- Admission: hold a `WorkspaceConfig` in core, reload without state, or gate
  admission with `stat`. Reject all three. Equal mtime does not prove equal
  content, and a `stat` never sees revoked trust.
- Scaffolding: keep `init` in the CLI. Reject it.

### Owners, removable code and open items

- `ConfigService` owns the trust gate and the grant order.
- The typestate owns the rule "no parse before trust".
- `IndexPolicy` owns the index freshness inputs.
- The `HostInputs` constructor owns the reads of the operating system.
- Removable: `ConfigDigests`, the per-section `.digests()` methods,
  `Config::hash`, `compose`, `traces.config.v1`, root-only grants,
  `MissingBaseline`, the `LazyLock` globals and one `syn` scan.
- Open: the figures for a full load (about 106 µs) and a hash check (about 20
  µs) come from an agent harness. Nobody re-ran them.
- Open: nobody knows if a user runs `traces trust` on a directory without a
  config.

### Change-table rows for the spec

1. `traces trust <dir without config>` becomes an error. It points to `traces
   init`. The exit code changes from 0 to 1. Ticket 16 freezes exit codes, so
   the spec needs an explicit row.
2. `MissingBaseline` and the root-only `trust --show` state are removed.
3. A relative path variable becomes an error.
4. The index fingerprint format changes. The index rebuilds one time.
5. `traces init` can trust the workspace it created.
6. The code reads `TRACES_IGNORED_DIRS` as an OS string. A non-Unicode value is
   no longer dropped.

The Trust Store layout does not change. Existing grants stay valid.

### Independence for the spec

- Decisions (a) and (b) do not depend on the others.
- Decide (f) first. Decision (c) and the admission function use `HostInputs`.
- Decision (c) depends on (b) and (f).
- Decisions (d) and (g) share only the init-then-trust step.
- Decision (e) shapes the admission function.

### Handoffs

- [Note document model](./23-note-document-model.md): The parser-input fields
  are provisional. Decide the status model and the place of `TaskListItem`. Add
  each index-time read to `IndexPolicy`.
- [Index read model](./24-index-read-model-and-query-execution.md) and
  [freshness](./25-index-freshness-and-invalidation.md): Ticket 22 fixed only
  the owner of `IndexPolicy` and the source of the fingerprint. Ticket 25
  decides the content, the granularity, the parser version, the format version
  and the racy timestamps. Ticket 25 also decides if a period-policy field
  exists. The "fingerprint wider than consumed" probe in ticket 25 no longer
  applies. A changed ignore set needs a rescan and a rebuild.
- [Temporal](./27-temporal-boundary-and-dialects.md): Decide the `[date]`
  syntax. The filename-period subset enters `IndexPolicy` if the index stores
  periods.
- [Template boundary](./28-template-engine-boundary.md): `TemplateService` holds
  `&Config`, and the template engine builds an indexer
  (`template/engine.rs:125-135`). `Config` no longer holds the ignore and period
  policy. The composition root builds `IndexPolicy` one time for the CLI and the
  template engine.
- [Paths](./29-path-and-capability-model.md): The host rejects relative path
  variables. Do not assume a base. Decide if `.traces` can be a symlink. Decide
  how a root alias gets an identity.
- [Host](./30-workspace-host-and-composition.md): Decide the held workspace, the
  watcher, the debounce, the composition of `IndexPolicy`, the dialog threading
  and who applies trust after init. `ConfigService::admit` is the admission
  primitive.
- [Errors](./31-error-and-diagnostic-architecture.md): Decide the new error sets
  for config, trust and init. Decide the error for `traces trust` without a
  config.
- [Crate topology](./32-crate-topology-and-public-surface.md): Ticket 22 fixes
  the logical owners and the module graph. Ticket 32 decides the crate cuts and
  the `pub` items. After a CLI split, `ConfigService`, `HostInputs`, `admit` and
  the init use case are the first outward-facing surface. The constructor that
  reads the OS belongs with the adapters.
- [Schema internals](./35-schema-internals-simplification.md) and [body
  sections](./36-body-section-schemas.md): The schema files own class
  `[binding]` (ticket 20). Ticket 22 adds no config key for it. `FieldName`
  stays a leaf.
- [Reconciliation](./33-reconcile-decisions-and-audit-coverage.md): Test the
  `IndexPolicy` boundary again. Test the shared ignore value between trust
  discovery and the index walker.
