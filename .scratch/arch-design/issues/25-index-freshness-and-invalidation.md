# Does the landed index freshness model stand?

Type: grilling
Status: open
Blocked by: 5, 22, 24, 12, 37

## Question

The index must return the same results as a fresh parse of trusted files and
config. Config drift detection landed at `207cc3fb`.
Use that design as the keep-as-is baseline.
Use [config ownership](./22-config-ownership-and-lifecycle.md) and
[the index read model](./24-index-read-model-and-query-execution.md).
Do not reopen their ownership or store-shape decisions.

### A. Config drift

- Check the landed `IndexFingerprint` and the full rebuild on drift.
- Compare the full rebuild with per-axis invalidation and manual
  `traces index`.
- Decide which invalidation granularity gives correct results for each
  index-time input.
- Keep `IndexPolicy::fingerprint()` as the only fingerprint source.
  Ticket 22 decided this.
- Include the effective ignore policy. A changed ignore set needs a rescan
  and a rebuild.
- Exclude `title`, `aliases` and the date keys that the index does not read.
- Include a new config field only if an index-time read uses it.
- Exclude the host time zone and schema files from the fingerprint.
- Keep ticket 24's fresh-parse or validated-generation read fallback.
  Decide how to avoid repeated rebuilds on an unwritable store.

The landed code builds the fingerprint from 10 of 15 `ConfigDigests`
settings. It writes the fingerprint and rows in one redb transaction.
See `src/index/fingerprint.rs:158-201` and `src/index/store.rs`.
The drift check runs at each refresh. Drift has priority over file staleness.
`src/index/service.rs` rebuilds on drift. An unreadable fingerprint produces
`DriftCause::Unrecorded`.

Ticket 22 removes `ConfigDigests`. One exhaustive projection builds
`IndexPolicy`. Do not retain the old probe that treats `title`, `aliases`
and date keys as necessary fingerprint inputs.

### B. Engine, parser and store format

- Decide how the fingerprint identifies the store engine and format.
  Ticket 24 needs this identity.
- Decide when stored facts, parser rules, value serialization or facets
  invalidate the store.
- Compare a manual `META_VERSION` bump, a derived version and a separate
  parser identity.
- Decide how a missing, unreadable or incompatible meta value triggers a
  rebuild.
- Rebuild the index once after an incompatible format change.
- Keep the schema `[binding]` and `global` rules outside the store identity.
  Queries resolve those rules at read time.
- If an engine uses a different cache filename, exclude that file from
  the source scan. Today the scan excludes only `.traces/index.redb`.
  Check how the new engine handles an old engine's unreadable bytes.

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
- Compare a racy-timestamp content check, a full content check and the
  existing stat shortcut.
- Compare Markdown content where parsing supplies stored facts.
  Check path membership and query-relevant metadata for all scanned files.
  Do not hash attachments that supply no parsed facts.
- Define the scan and commit checks when a source file changes during a
  refresh.
- Decide how to check source freshness after a crash loses the last
  `Durability::None` commit.
- Check the format, policy and source freshness on reopen.
  A matching fingerprint alone is insufficient.
- Keep the source snapshot coherent with the committed facets and meta
  values.
- Treat a missing or corrupt required facet as a cache miss.
  Ticket 24 decided this.
- Use a fresh parse for a read query after a cache miss.
  Make `traces index` fail closed.

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
- Discard the delta and replan or rebuild on a base mismatch.
  Never retag rows from another policy.
- Decide how a refresh checks source freshness before it publishes a new
  generation.
- Check the two-process store-open and lock behavior on Linux and Windows.
- Decide which invariant controls workspaces with different policies
  at one canonical root.
- Leave host writer admission to
  [the workspace-host ticket](./30-workspace-host-and-composition.md).
- Leave canonical root identity to
  [the path ticket](./29-path-and-capability-model.md).

Ticket 24 requires one read snapshot for candidates, facets and fingerprint
validation. Its host reads a pinned immutable generation.
Do not mix fresh rows with unverified store rows.
Do not assume that a redb file lock enforces policy consistency.
Leave open-buffer overlays and save/close transitions to
[the buffer ticket](./34-live-buffer-and-persisted-index-consistency.md).

### E. Path-set change cost

- Decide whether the facet split removes enough decoding cost on add,
  delete and rename.
- Compare on-demand facet reads, a host note cache and the remaining
  full-decode path.
- Decide whether to accept the remaining cost instead of adding a cache.
- Measure assembled results and memory at the target vault sizes before
  selecting an optimization.
- Distinguish facet scans from assembled `QueryService` results.
  The earlier 50,000-note facet probe did not build result rows.
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
- Decide how to report repeated failed heals without hiding a stale or
  failed index.
- Give notice presentation to
  [the diagnostic ticket](./31-error-and-diagnostic-architecture.md).
- Keep any check watermark and prior violation keys outside the index and
  its fingerprint.
- Leave the decision to create that optional check store to the check
  and host owners.

Today production only logs `RefreshReport`: `tracing::info!` for drift and
`debug!` for file changes. Count accessors are test-only.
The cost of a drift heal has not been measured.
A cold query can wait for a full rebuild [H].
An unwritable store can repeat that work [H].

### Evidence and handoffs

Read `docs/adr/0009-index-refresh.md`, `reverification.md`, `research/05`
and `research/11`.
Check `src/index/fingerprint.rs`, `src/index/refresh.rs`,
`src/index/service.rs` and `src/index/store.rs`.
Use `tests/e2e/index.rs`
(`config_edit_heals_the_index_on_the_next_query`) as a behavior check.
Preparatory research: [Index freshness evidence and open proof](../research/25-index-freshness-prep.md).
Do not treat ADR-0009 as unimplemented. It has landed and remains `proposed`.

- [Ticket 18](./18-value-model.md) changes `Number` to `Int`/`Float`
  and changes `Link` serialization.
- [Ticket 19](./19-coercion-and-comparison-policy.md) stores naive
  datetimes as civil values without a host zone.
- [Ticket 20](./20-schema-note-typing-and-validation.md) keeps schema
  binding and validation out of the index.
- [Ticket 21](./21-ambient-environment-and-execution-context.md) keeps
  zone-derived `cdate` and `mdate` out of persisted facts.
- [Ticket 22](./22-config-ownership-and-lifecycle.md) owns the exhaustive
  `IndexPolicy` projection and removes unused digests.
- [Ticket 23](./23-note-document-model.md) requires a `META_VERSION`
  change when stored `Note` facts change.
- [Ticket 24](./24-index-read-model-and-query-execution.md) owns facet
  atomicity, read snapshots and the writer base check.

Ticket 24 derives filename periods at query time.
Do not fingerprint filename-period parsing rules for fields the store
does not persist. Persist source creation timestamps only when available.
Never substitute mtime for `cdate`.
If another persisted fact starts to use filename-period rules, include
those rules in `IndexPolicy`.

Record the alternatives, invariant owner, removable code, unresolved
questions and downstream handoffs in the answer.
Record each observable change in the specification's change table
(ticket 16).

## Comments

### Grilling round: freshness and recovery

These answers are provisional. Do not resolve this ticket before the journal
measurement.

- Use per-axis repair only when every dependent facet and key has a complete,
  atomic rewrite. Otherwise, rebuild.
- Rescan and rebuild after an ignore-policy change.
- Keep `IndexPolicy::fingerprint()` as the policy fingerprint source.
- Use one explicit application-format version for parser facts,
  serialization, facets and derived keys.
- Keep the engine identity separate from the policy fingerprint.
  Use one cache file per engine.
- Exclude every recognized engine cache file from source scans.
- Compare parsed Markdown content even when size and mtime match.
- Compare path membership and query-relevant metadata for all scanned files.
  Do not hash attachment content without parsed facts.
- Check the complete source snapshot before commit and before each host query.
- Do not claim an atomic filesystem snapshot while external editors write files.
- Validate the format, policy, required facets and source snapshot on reopen.
- Keep one policy-bound disk generation per canonical root.
  Do not retag rows from another policy.
- Repair class keys from stored frontmatter only when that source is complete.
  Reparse notes for parser-dependent policy changes.
- Recheck the store incarnation and revision after an uncertain commit result.
- Use fresh parsing or a revalidated host generation after a cache failure.
  Make `traces index` fail closed.
- Do not add a host note cache before assembled-query and memory measurements.
- Keep successful automatic heals quiet.
  Give one stderr notice for each read fallback.
- Keep the current host failure state visible.
  Leave notice presentation to the diagnostic ticket.

The durable recovery journal remains open. Measure it in
[What does a durable index repair journal cost and
save?](./37-measure-index-recovery-journal.md).
Compare it with a validated incremental replan and an in-memory host retry
before deciding.
