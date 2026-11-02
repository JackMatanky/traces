# Does the landed index freshness model stand?

Type: grilling
Status: resolved
Blocked by: 5, 22, 24, 12

## Question

The index must return the same results as a fresh parse of trusted files and
config. Config drift detection landed at `207cc3fb`.
Use that design as the keep-as-is baseline.
Use [config ownership](./22-config-ownership-and-lifecycle.md) and
[the index read model](./24-index-read-model-and-query-execution.md).
Do not reopen their ownership or store-shape decisions.

### A. Config drift

- Check the landed `IndexFingerprint` and the full rebuild on drift.
- Compare the full rebuild with per-axis invalidation and manual `traces index`.
- Decide which invalidation granularity gives correct results for each
  index-time input.
- Keep `IndexPolicy::fingerprint()` as the only fingerprint source. Ticket 22
  decided this.
- Include the effective ignore policy. A changed ignore set needs a rescan and a
  rebuild.
- Exclude `title`, `aliases` and the date keys that the index does not read.
- Include a new config field only if an index-time read uses it.
- Exclude the host time zone and schema files from the fingerprint.
- Keep ticket 24's fresh-parse or validated-generation read fallback. Decide how
  to avoid repeated rebuilds on an unwritable store.

The landed code builds the fingerprint from 10 of 15 `ConfigDigests` settings.
It writes the fingerprint and rows in one redb transaction.
See `src/index/fingerprint.rs:158-201` and `src/index/store.rs`.
The drift check runs at each refresh. Drift has priority over file staleness.
`src/index/service.rs` rebuilds on drift. An unreadable fingerprint produces
`DriftCause::Unrecorded`.

Ticket 22 removes `ConfigDigests`. One exhaustive projection builds
`IndexPolicy`. Do not retain the old probe that treats `title`, `aliases` and
date keys as necessary fingerprint inputs.

### B. Engine, parser and store format

- Decide how the fingerprint identifies the store engine and format. Ticket 24
  needs this identity.
- Decide when stored facts, parser rules, value serialization or facets
  invalidate the store.
- Compare a manual `META_VERSION` bump, a derived version and a separate parser
  identity.
- Decide how a missing, unreadable or incompatible meta value triggers a
  rebuild.
- Rebuild the index once after an incompatible format change.
- Keep the schema `[binding]` and `global` rules outside the store identity.
  Queries resolve those rules at read time.
- If an engine uses a different cache filename, exclude that file from the
  source scan. Today the scan excludes only `.traces/index.redb`. Check how the
  new engine handles an old engine's unreadable bytes.

Today `META_VERSION = b"1"` changes by hand (`src/index/store.rs:41`).
`IndexStore::open` wipes the store after a version mismatch.
It does not use the `Drifted` path that ADR-0009 describes.
[H] A parser change without a version bump can serve stale facts.
Tickets 18, 19 and 23 change stored values, civil dates and the `Note` shape.
Ticket 24 changes the store to independent facets, task copies and secondary
keys. These changes must invalidate old bytes, parser notices and derived
keys.

### C. Source freshness and crash recovery

- Detect same-size edits even when file size and mtime do not change.
- Compare a racy-timestamp content check, a full content check and the existing
  stat shortcut.
- Compare Markdown content where parsing supplies stored facts. Check path
  membership and query-relevant metadata for all scanned files. Do not hash
  attachments that supply no parsed facts.
- Define the scan and commit checks when a source file changes during a refresh.
- Decide how to check source freshness after a crash loses the last
  `Durability::None` commit.
- Check the format, policy and source freshness on reopen. A matching
  fingerprint alone is insufficient.
- Keep the source snapshot coherent with the committed facets and meta values.
- Treat a missing or corrupt required facet as a cache miss. Ticket 24 decided
  this.
- Use a fresh parse for a read query after a cache miss. Make `traces index`
  fail closed.

The current delta has no racy-timestamp guard (`src/index/delta.rs`).
The existing `service.rs` tests sleep to let mtime advance.
Research 05 describes Git's guard for a file whose mtime reaches the index
timestamp. IWE avoids needless mtime changes with `write_file_if_changed`
(diwe v0.24.1). Do not rewrite unchanged source bytes to refresh a host
view. Ticket 23 requires a full source-snapshot comparison at commit for a
closed-file edit.

### D. Store generations and concurrent policies

- Apply ticket 24's writer-transaction check of the plan's base identity,
  revision and fingerprint.
- Discard the delta and replan or rebuild on a base mismatch. Never retag rows
  from another policy.
- Decide how a refresh checks source freshness before it publishes a new
  generation.
- Check the two-process store-open and lock behavior on Linux and Windows.
- Decide which invariant controls workspaces with different policies at one
  canonical root.
- Leave host writer admission to [the workspace-host
  ticket](./32-workspace-host-and-composition.md).
- Leave canonical root identity to [the path
  ticket](./31-path-and-capability-model.md).

Ticket 24 requires one read snapshot for candidates, facets and fingerprint
validation. Its host reads a pinned immutable generation.
Do not mix fresh rows with unverified store rows.
Do not assume that a redb file lock enforces policy consistency.
Leave open-buffer overlays and save/close transitions to
[the buffer ticket](./27-live-buffer-and-persisted-index-consistency.md).

### E. Path-set change cost

- Decide whether the facet split removes enough decoding cost on add, delete and
  rename.
- Compare on-demand facet reads, a host note cache and the remaining full-decode
  path.
- Decide whether to accept the remaining cost instead of adding a cache.
- Measure assembled results and memory at the target vault sizes before
  selecting an optimization.
- Distinguish facet scans from assembled `QueryService` results. The earlier
  50,000-note facet probe did not build result rows.
- Keep `CandidateTrie` and `indextree`, as ticket 24 decided.

Research 11 measured 18.5 ms of note deserialization on a 5,000-note path
change. That is 97% of the measured work.
Link re-resolution took 0.5 ms, or 2.7%.
Ticket 24 replaces whole-note rows with facets. It keeps the trie for large
same-stem candidate sets. The inverted unresolved-stem index from research
05 targets link work, not the measured decoding cost.

### F. Heal visibility and check state

- Decide if users see successful drift heals.
- Keep ticket 24's single stderr cache notice for a fresh-parse fallback.
- Decide how to report repeated failed heals without hiding a stale or failed
  index.
- Give notice presentation to [the diagnostic
  ticket](./33-error-and-diagnostic-architecture.md).
- Keep any check watermark and prior violation keys outside the index and its
  fingerprint.
- Leave the decision to create that optional check store to the check and host
  owners.

Today production only logs `RefreshReport`: `tracing::info!` for drift and
`debug!` for file changes. Count accessors are test-only.
The cost of a drift heal has not been measured.
A cold query can wait for a full rebuild [H].
An unwritable store can repeat that work [H].

### Evidence and handoffs

Read `docs/adr/0009-index-refresh.md`, `reverification.md`, `research/05` and
`research/11`.
Check `src/index/fingerprint.rs`, `src/index/refresh.rs`, `src/index/service.rs`
and `src/index/store.rs`.
Use `tests/e2e/index.rs` (`config_edit_heals_the_index_on_the_next_query`) as a
behavior check.
Preparatory research: [Index freshness evidence and open proof](../research/25-index-freshness-prep.md).
Do not treat ADR-0009 as unimplemented. It has landed and remains `proposed`.

- [Ticket 18](./18-value-model.md) changes `Number` to `Int`/`Float` and changes
  `Link` serialization.
- [Ticket 19](./19-coercion-and-comparison-policy.md) stores naive datetimes as
  civil values without a host zone.
- [Ticket 20](./20-schema-note-typing-and-validation.md) keeps schema binding
  and validation out of the index.
- [Ticket 21](./21-ambient-environment-and-execution-context.md) keeps
  zone-derived `cdate` and `mdate` out of persisted facts.
- [Ticket 22](./22-config-ownership-and-lifecycle.md) owns the exhaustive
  `IndexPolicy` projection and removes unused digests.
- [Ticket 23](./23-note-document-model.md) requires a store-format change when
  stored `Note` facts change. The format epoch in "Store identity" meets this
  rule.
- [Ticket 24](./24-index-read-model-and-query-execution.md) owns facet
  atomicity, read snapshots and the writer base check.

Ticket 24 derives filename periods at query time.
Do not fingerprint filename-period parsing rules for fields the store
does not persist. Persist source creation timestamps only when available.
Never substitute mtime for `cdate`.
If another persisted fact starts to use filename-period rules, include
those rules in `IndexPolicy`.

Record the alternatives, invariant owner, removable code, unresolved questions
and downstream handoffs in the answer.
Record each observable change in the specification's change table (ticket 16).

## Comments

### Grilling round: freshness and recovery

These answers are confirmed. Ticket 26 (journal measurement) is decoupled from
this ticket's resolution.

- Use per-axis repair only when every dependent facet and key has a complete,
  atomic rewrite. Otherwise, rebuild.
- Rescan and rebuild after an ignore-policy change.
- Keep `IndexPolicy::fingerprint()` as the policy fingerprint source.
- Use one format epoch for parser facts, serialization, facets and derived keys.
  See "Store identity".
- Keep the engine identity separate from the policy fingerprint. Use one cache
  file per engine.
- Exclude every recognized engine cache file from source scans.
- Compare parsed Markdown content even when size and mtime match.
- Compare path membership and query-relevant metadata for all scanned files. Do
  not hash attachment content without parsed facts.
- Check the complete source snapshot before commit and before each host query.
- Do not claim an atomic filesystem snapshot while external editors write files.
- Validate the format, policy, required facets and source snapshot on reopen.
- Keep one policy-bound disk generation per canonical root. Do not retag rows
  from another policy.
- Repair class keys from stored frontmatter only when that source is complete.
  Reparse notes for parser-dependent policy changes.
- Recheck the store incarnation and revision after an uncertain commit result.
- Use fresh parsing or a revalidated host generation after a cache failure. Make
  `traces index` fail closed.
- Do not add a host note cache before assembled-query and memory measurements.
- Keep successful automatic heals quiet. Give one stderr notice for each read
  fallback.
- Keep the current host failure state visible. Leave notice presentation to the
  diagnostic ticket.

The durable recovery journal remains open. Measure it in
[What does a durable index repair journal cost and
save?](./26-measure-index-recovery-journal.md).
Compare it with a validated incremental replan and an in-memory host retry
before deciding.

### Stress test, round two

Five reviewers attacked the answers above. The details are in [Index freshness
evidence and open proof](../research/25-index-freshness-prep.md), section G.
These changes replace the matching answers above.

- Replace "Exclude every recognized engine cache file from source scans". Remove
  the dead `path != index_db` check. Test that nothing under `.traces/` enters
  the scan.
- Replace the single racy rule. Store a `racy` bit in each file row. Set it when
  `mtime >= scan_start - 2 s`.
- Read each changed file from one open handle. Stat the handle before and after
  the read.
- After `NotFound` during a refresh, rescan once. A second failure is a
  classified error.
- Drop the 128-bit `commit_nonce`. After an uncertain commit, drop the handle
  and the plan.
- Remove the unwritable latch. Try the write-open at each persist.
- Treat a failed store replacement as a cache miss for reads.
- Classify late `META` corruption errors as a rebuild.
- Replace the manual `FORMAT_VERSION` with a format epoch and an extractor
  fingerprint. See "Store identity" below. Keep the engine tag and the policy
  fingerprint.
- Make a store with a higher epoch an unusable cache for reads. Do not wipe it.
- Hold the store in a host only during refresh and persist.

Decisions the maintainer made in this round:

- A host validates sources within a staleness bound. It does not check before
  each query.
- Keep the landed rule for alternating policies: the last writer rebuilds.
- Rebuild fully on an older epoch or on any fingerprint change. A higher epoch
  is a read miss, not a rebuild. Per-axis repair waits for a measured rebuild
  cost.
- Cover a restored-mtime edit with `ctime` on Unix and `size` plus `mtime` on
  Windows. Add `traces index --verify`. Record the Windows gap in the change
  table. Hashing every file costs 395 ms at 50,000 notes. See section G14.
- Store identity (replaces the earlier manual `FORMAT_VERSION` and lock-pin
  decision): see "Store identity" below.

Confirmed decisions:

- Settle the host staleness bound with downstream tickets 32 and 27.
- Decouple ticket 26: crash recovery is a no-op refresh of uncommitted files
  (12 to 65 ms). The journal is an optional optimization that does not block
  this ticket's invalidation rules.

### Store identity

The maintainer chose a fingerprint with a version floor. Round three replaced
the version floor. A check against earlier decisions then corrected the engine
tag owner, the `META` fields and the rollback path. The maintainer confirmed this
revision. The evidence is in [the research brief](../research/25-index-freshness-prep.md), section H.

`META` holds these identity parts:

- **Engine tag.** It names the storage engine and its major version, such as
  `redb4`. Ticket 24 gives this rule to this ticket (`24:299-300`). The tag is
  part of the cache file name, so one file serves one engine. The current file
  keeps its name for the first engine.
- **Platform family.** Unix and Windows serialize arbitrary vault paths
  differently (`src/path.rs:349-391`). A store does not move between families.
- **Format epoch.** An ordered `u32`. It is the "format version" of ticket 24
  (`24:309`). It decides only "newer" or "older".
- **Extractor fingerprint.** A 32-byte hash. It decides "same behavior".
- **Policy fingerprint.** `IndexPolicy::fingerprint()` supplies it. The glossary
  term "Index Fingerprint" is stale. See "Stale ADR and glossary text" below.
- **Written-by version.** The binary version. Notices use it. No rule compares
  it.

`META` also keeps the store incarnation and the revision (`24:431-438`). They
guard the writer transaction. They are not identity parts.

The extractor fingerprint hashes two inputs:

1. The resolved versions of the crates that feed persisted values:
   `pulldown-cmark`, `logos`, `noyalib`, `jiff`, `speedate` and `postcard`.
   `build.rs` reads them from `Cargo.lock`. The build fails if a listed crate is
   missing. Whoever prunes a crate updates the list. Tickets 29 and 34 may prune
   `speedate` or `noyalib`.
2. The encoded bytes of probe records built from an embedded corpus and
   synthetic facet records.

The probe rules are:

- There is one probe record for each facet in ticket 24's list, except `meta`.
- There is one range key for each scalar key encoding.
- The `files` probe uses a synthetic file record with distinct values.
- A test fails if a facet table in ticket 24's list other than `META` has no
  probe record.

The corpus rules are:

- It has a maximal note. That note fills every optional field with a distinct
  value and uses every enum variant that exists.
- It has a minimal note.
- It uses a fixed ASCII path for corpus notes. For ASCII, the Unix and Windows
  path encodings write the same bytes, because postcard writes a `u16` below 128
  as one byte (`src/path.rs:349-391`). This makes the committed extractor
  fingerprint identical across platforms. The identity test runs on Windows CI
  too. This is unverified on Windows.
- It uses explicit-offset datetimes only. An unzoned value depends on the host
  zone today (`src/temporal/datetime.rs:133-136`). When ticket 19 lands the
  civil variant, add it to the corpus. That bumps the epoch.

The process computes the fingerprint once, on the first store open. It does not
compute it at startup.

A committed identity file holds the current epoch and fingerprint. The first
epoch is 1. A test fails when the computed fingerprint differs from the file.
The message names `mise run index:bless`. That task rewrites the file and adds
one to the epoch. The task is the only step a person runs. The file path and
syntax are implementation choices.

Store-open rules:

- A different platform family, a lower epoch, or the same epoch with a different
  fingerprint is a rebuild. The last case means someone skipped the bless task,
  so the rule fails safe. These rebuilds are automatic heals. They stay quiet.
- A row that fails to decode, or a damaged `META`, is a rebuild.
- A higher epoch is an unusable cache under ticket 24 (`24:374-384`). A read
  parses the source files and gives one stderr notice. It exits 0 if the parse
  and the query succeed. It does not write and does not wipe. A host that starts
  cold against such a store parses fresh the same way.
- `traces index` already rebuilds in full (`src/cli/index.rs:38-48`). It
  replaces a newer store. This is the way out after a rollback. The notice names
  `traces index`. This ticket adds no new flag for it.
- Only a writer that has decided to rebuild may delete a store file. A read that
  meets an engine error before `META` is readable treats the store as a miss.
  Today `create_db` deletes the file first (`src/index/store.rs:903-931`). This
  must change.
- A changed engine tag uses another file. A newer `redb` major version never
  opens an older engine's file.
- A rebuild heal reports its cause. Rename `RefreshReport::ConfigDrift` and
  `DriftCause` so they do not say "config". Add identity causes: engine,
  platform, epoch, extractor and damaged `META`. Keep one rebuild state. The
  heal is the same full rebuild for every cause.

A long-lived host keeps its old binary until its editor restarts. It then meets
a newer store. It never writes that store. It serves from its in-memory
generation, or parses fresh if it has none. Tickets 32 and 27 own that behavior.

Why the version floor is gone:

- The repository has no version tags. The version is still `0.1.0` after many
  breaking commits.
- For `0.x`, the minor version is the breaking one. "Major or minor" does not
  fit.
- Developer builds and `cargo install --git` never change the version.
- The crate list in the fingerprint covers the crate part of the same gap. It
  needs no release discipline.

Boundary rules:

- Confine `pulldown-cmark` to `src/note/parser.rs` and `src/note/parser/`.
  Confine `redb` and `postcard` to `src/index/`. A search of `src/` found
  `pulldown_cmark` only in `src/note/parser.rs`. The task pins today's state.
- Enforce this with a `check:boundaries` mise task. Use `git grep`, as
  `check:layers` does. Do not write a Rust test that reads source text. The task
  is provisional. Ticket 34 may replace it with crate boundaries.
- Do not confine `logos`, `noyalib`, `jiff` or `speedate`. Four subsystems use
  `logos`: the shared lexer, the note parser, the query grammar and the temporal
  module. `noyalib` is in `src/yaml.rs`, `src/field.rs`, `src/note/metadata.rs`
  and the template engine. The crate list covers them.
- Domain types keep their derived serde for the cache (tickets 18 and 24). The
  probe records make a shape change visible. Do not add store-only record types.

Invariant owners and removable code:

- `src/index/store.rs` owns identity validation. `src/index/service.rs` owns the
  rebuild or miss decision. The index module owns the probe builder and the
  identity file.
- Remove `META_VERSION` and `META_VERSION_KEY` from `src/index/store.rs`.

Known gaps:

- A change in Traces code on an input that the corpus lacks. Rule: a fix to the
  extractor adds a regression note to the corpus. That changes the fingerprint.
  `traces index --verify` is the backstop.
- A `cargo install` without `--locked` can resolve other crate versions than the
  packaged `Cargo.lock`. Ticket 38 requires `--locked` in the install
  instructions.
- An enum variant that no corpus note uses. Spike `serde-reflection` in the
  implementation to close it. This is unverified for the tagged serde types.
- Cache files of engines that are no longer used stay on disk. Who removes them
  is open.
- The runtime cost of the corpus pass is unmeasured.
- A host-zone value stays outside the identity until tickets 21 and 24 remove
  `FileMeta.day` (`src/file.rs:88-98`). A zone change leaves stale `day` values.
  The synthetic file probe sets `day` by hand.
- A future-dated mtime stays racy. The store re-reads that file on every
  refresh. This is correct but costs time. Measure it before you add a clamp.
- A read-only vault cannot `create` the store. Check whether a read-only open of
  a valid store avoids the parse fallback in `redb`. This is unverified.
- The rebuild cost figures (797 ms at 20,000 notes, 1,801 ms at 50,000) are full
  builds with persistence. The parse-only fallback latency is unmeasured.

Handoffs:

- Ticket 31: the store is not portable across platform families until paths are
  portable. Then drop the platform part of the identity. Ticket 31 also has no
  Unicode normalization rule. Until it has one, NFC and NFD names are different
  files.
- Tickets 32 and 27: the host rules above.
- Ticket 33: the notice for a higher epoch. Rebuild heals stay quiet.
- Ticket 34: the `build.rs` path to `Cargo.lock` after a workspace split, and
  the replacement of `check:boundaries`.
- Tickets 19 and 29: a civil datetime must deserialize without a zone lookup.
  Tickets 29 and 34: update the crate list if they prune a crate.
- Ticket 35: replace ADR-0009 and the glossary text named in "Stale ADR and
  glossary text" when the implementation lands.
- Ticket 38: the release process. It no longer gates this ticket.

Change-table rows for ticket 16:

- A store from an older epoch rebuilds once on upgrade. A patch release that
  changes no extractor output does not rebuild.
- A store from a higher epoch is a read miss with one stderr notice and exit
  0. It is no longer wiped.
- `traces index` from an older binary replaces a newer store. It gives one
  stderr notice first.
- A store from another platform family rebuilds.
- The cache file name carries the engine tag.
- `traces index --verify` is new.
- On a held store, a read query falls back to a fresh parse and exits 0 instead
  of 1.
- A transient `NotFound` during refresh retries the scan once.
- Ticket 16 correction: see "Audit of all tickets".

### Stale ADR and glossary text

An ADR or a glossary entry can be wrong. This section tests them. It does not
defer to them.

- ADR-0009 is `proposed`. No one accepted it. It does not bind this ticket.
- ADR-0009 says a format-version mismatch is a `Drifted` refresh state. The code
  does not do this. `IndexStore::open` wipes the store before a refresh runs
  (`src/index/store.rs:936-988`, and the premise at `25:62-63`). `DriftCause`
  has only `Settings` and `Unrecorded` (`src/index/fingerprint.rs`).
- ADR-0009 names the state "config drift". Format and engine changes are not
  config changes. The name hides the cause. Rename it, as above.
- ADR-0009 enforces completeness by destructuring the config digest bundles.
  Ticket 22 deletes those bundles. That consequence is dead.
- ADR-0009 says a heal means an operator never needs `traces index`. Policy
  alternation breaks this. Two policies at one root rebuild the store at each
  switch (brief G12). Ticket 25 keeps the landed rule only until a measurement
  justifies another.
- The glossary says the "Index Fingerprint" includes every setting "even those
  no index-time code reads today". This ticket decides the opposite (`25:28`).
  The glossary also describes per-setting config digests that ticket 22 deletes.
  Rename the term "Policy Fingerprint". Rewrite its definition.
- The glossary defines `Note` as one persisted form, and `Incremental Delta` as
  a timestamp comparison. Ticket 24 decomposes the `Note` into facets. Round two
  adds `ctime` and a racy bit. Both entries are stale.
- Not verified: ADR-0005, which ADR-0009 links, does not appear in `docs/adr/`.
  I did not find it.

### Audit of all tickets

Four reviewers audited tickets 01 to 38, the ADRs and the glossaries. They
treated every ticket as fallible, this one included. The parent verified each
finding that could change this ticket. The full table is in section H5 of the
research brief.

Ticket 16 overstates its guarantee. Row 74 freezes "index result equivalence
with a fresh parse of files and config". No stat-based check can guarantee that.
The landed canary test pins stale output for a same-size edit with a restored
mtime (`tests/e2e/index.rs:129-180`). Replace the row with a stated contract: a
file with an unchanged size, mtime and (on Unix) ctime counts as unchanged,
outside the racy window. `traces index --verify` checks the rest. Record the
Windows gap in the change table.

Findings logged for other tickets, not adopted here:

- Ticket 24: a reviewer reports that the 11-facet split costs 85% more write
  time and doubles the file size. I did not re-derive the figures. A slower
  write raises the rebuild cost that this ticket relies on.
- Ticket 32: a host that serves from memory pays 467 MB of RSS at 50,000 notes.
- Ticket 33: a notice channel that does not travel as an error. A higher epoch
  exits 0.
- Ticket 26: a reviewer called the measurement plan large. The maintainer
  decided to keep the range of 50 to 50,000 notes. The range comes from the
  maintainer's vaults and from other Obsidian vaults. Earlier data
  (`research/11`) suggests a journal adds more fixed latency than it saves.
- Ticket 38: a reviewer calls it dead weight. It no longer gates this ticket.
  The maintainer asked for it.

## Code delta

The code does not implement this ticket. This table lists what the refactor changes. Locations are the working tree on 2026-10-30.

| Area | Today | Target | Depends on |
| --- | --- | --- | --- |
| Format marker | `META_VERSION = b"1"` and `META_VERSION_KEY` (`src/index/store.rs:39-41`). A mismatch makes `open` clear the store | Remove both. `META` holds the engine tag, platform family, format epoch, extractor fingerprint, policy fingerprint and written-by version | Ticket 24 `meta` table |
| Store open | `create_db` deletes the file before `rebuild_needed` decides (`src/index/store.rs:903-931`). `rebuild_needed` compares one version value (`src/index/store.rs:936-988`) | Reads never delete. Only a writer that decided to rebuild deletes. Lower epoch, other platform, other fingerprint, damaged `META` or undecodable rows: rebuild. Higher epoch: read miss with one stderr notice, exit 0 | Ticket 24 |
| Policy fingerprint | `IndexFingerprint` built from `ConfigDigests` (`src/index/fingerprint.rs`, `src/config/model/digest.rs`) | `IndexPolicy::fingerprint()`. Delete `ConfigDigests` | Ticket 22 |
| Refresh report | `RefreshReport::ConfigDrift` and `DriftCause` (`src/index/refresh.rs:44`, `src/index/service.rs:177,294,315`) | Rename. Add identity causes: engine, platform, epoch, extractor, damaged `META`. Rebuild heals stay quiet | Ticket 33 |
| Extractor fingerprint | None | New `build.rs` reads resolved versions of `pulldown-cmark`, `logos`, `noyalib`, `jiff`, `speedate` and `postcard` from `Cargo.lock`. New probe builder and embedded corpus in `src/index/`, computed once on first open | Tickets 19, 29 for the civil datetime |
| Identity file | None | A committed identity file. A test fails on drift. New task `mise run index:bless` rewrites it and adds 1 to the epoch | None |
| Boundary check | `check:layers` only (`mise.toml:148`) | New `check:boundaries` task. Use `git grep`. Confine `pulldown-cmark` to `src/note/parser.rs` and `src/note/parser/`. Confine `redb` and `postcard` to `src/index/` | Ticket 34 may replace it |
| Cache file name | `.traces/index.redb` (`src/index/mod.rs:48`) | The name carries the engine tag | Ticket 24 |
| Source scan | `scan` compares `path != index_db` (`src/index/service.rs:424-435`) | Remove the dead check. A test shows that nothing under `.traces/` enters the scan | None |
| Change detection | `FileDelta::compute` compares `FileMeta` (`src/index/delta.rs:21`). The landed canary pins stale output for a restored mtime (`tests/e2e/index.rs:129-180`) | Add a `racy` bit per file row. Set it when `mtime >= scan_start - 2 s`. Add `ctime` on Unix. Read each changed file from one handle with a stat before and after. Retry the scan once on `NotFound` | Ticket 24 file rows |
| Verify command | None | New `traces index --verify` hashes every file | Ticket 33 for the exit codes |
| Host store use | A held store fails a read with exit 1 | A read falls back to a fresh parse with one stderr notice and exit 0. A host holds the store only during refresh and persist | Tickets 27, 32 |
| Commit recovery | No `commit_nonce` and no unwritable latch exist in `src/index/`. The research brief (C5, C6) says they were proposed and drops them | Do not add them. After an uncertain commit, drop the handle and the plan. Try the write-open at each persist | Ticket 24 |
| Replacement failure | The store open and replacement path is `create_db` (`src/index/store.rs:903`) | A failed store replacement is a cache miss for reads | None |
| ADR and glossary | ADR-0009 `proposed`. The glossary "Index Fingerprint" entry is stale | Replace both when the code lands | Ticket 35 |
| Ticket 16 table | Row 78 holds the corrected contract | Add the change-table rows listed above to the redesign spec | Ticket 35 |

Not in code:

- The staleness bound (tickets 27 and 32).
- The release process and the `--locked` install rule (ticket 38).
- The journal decision (ticket 26).
