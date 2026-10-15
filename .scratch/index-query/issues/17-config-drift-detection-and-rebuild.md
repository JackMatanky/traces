# 17: Detect Config Drift at Refresh and Auto-Rebuild

**What to build:** Editing an index-relevant setting in `traces.toml` and
re-trusting heals the index on the next refresh — no `traces index` needed.
Refresh gains config drift as a first-class state alongside file staleness,
with a rebuild as the heal verb and a report that names what changed.

**Blocked by:** config-service/07 (Per-Section Config Digests and the Config
Fingerprint) — this ticket consumes `ConfigDigests`; it must not build its
own encoding.

**Category:** enhancement

**Status:** ready-for-agent

- [ ] `IndexFingerprint` (index-owned type) holds the 8 index-relevant
      framed digests — frontmatter title/aliases/tags/date_created/
      date_modified, tasks statuses/tag_filters, schemas class_field —
      built by exhaustive destructures at both levels (`root`, `templates`,
      and `schemas.directory` explicitly ignored): adding any new config
      setting breaks the constructor and forces an invalidation decision.
      Never use `..` there.
- [ ] `IndexSetting` enum (index-owned; derives `Copy`/`PartialEq`; variant
      names mirror the raw.toml keys; qualified `Display` such as
      `[frontmatter] tags`) for diagnosis only — config never learns of it.
- [ ] The store gains an 8th `meta` TableSpec in `TABLES` with
      `DeletePolicy::Required`, making every legacy 7-table store wipe at
      open via `rebuild_needed` (free one-time rebuild). Row = format
      version + the 8 raw 32-byte digests (`Blake3Digest` gains a bytes
      codec), written in the same txn as the index rows via `PersistRequest`
      beside `dimensions`, at both write sites (service persist and
      PendingApply). The row persists exactly the `IndexFingerprint` —
      never the whole config.
- [ ] `RefreshPlan::collect` reads the meta row in the same read txn as the
      file listing and compares against the expected fingerprint;
      `prepare_pass` gains `Drifted { index, store }` (drift wins over
      staleness). A missing or undecodable meta row in a store that holds
      rows is drift (fail-closed); a virgin store (no tables, no rows)
      stays on the honest first-build `FilesChanged` path.
- [ ] Heal for drift is the existing rebuild verb, handing its store back to
      the refresh path. `load()` remains untouched (documented as-persisted
      for tests/benches).
- [ ] `RefreshReport` collapses into
      `enum RefreshReport { #[default] Unchanged, FilesChanged { upserted,
      deleted, links_modified }, ConfigDrift { changed: Box<[IndexSetting]> } }`
      with count accessors kept as documented methods; no file-path lists.
      Loses `Copy` — the test/bench comparison sites are updated.
- [ ] Tests: inverted pinned test (config `class_field` edit → `ConfigDrift`
      with the exact changed settings and axes re-derived from the new
      config, no `traces index`); legacy-store rebuild test (7-table store
      wipes and rebuilds at open); framing/encoding pins; an E2E canary:
      edit config → re-trust → query healed without `traces index`.
- [ ] Glossary: new **Index Fingerprint** entry; extend **RefreshState**
      and **Refresh Report** (the Config Fingerprint amend lives in
      config-service/07, which defines the new encoding). Add a
      supersession comment to integration-e2e-suite issue 04 (config
      staleness "correct-as-designed" is now healed, not warned).

## Comments

> *Filed from a grill-me design session on config ↔ index invalidation.
> Design record: the index consumes `ConfigDigests` from config-service/07
> and owns all invalidation policy — dependency direction `index → config`
> only. Config drift is one drift class; the refresh architecture is
> recorded in ADR 9.*

> *Supersedes the mitigations recorded in
> `.scratch/integration-e2e-suite/issues/04-config-staleness-investigation.md`
> (warn-on-stale-axes): drift is now detected and auto-healed.*
