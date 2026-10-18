# 17: Detect Config Drift at Refresh and Auto-Rebuild

**What to build:** Editing an index-relevant setting in `traces.toml` and
re-trusting heals the index on the next refresh — no `traces index` needed.
Refresh gains config drift as a first-class state alongside file staleness,
with a rebuild as the heal verb and a report that names what changed.

**Blocked by:** None. config-service/07 (Per-Section Config Digests and the
Config Fingerprint) is merged to `main` (`1b1d9ff1`); this ticket consumes
`ConfigDigests` and must not build its own encoding.

**Category:** enhancement

**Status:** ready-for-agent

- [x] `IndexFingerprint` (index-owned type, `Copy`) holds the 10 index-relevant
      framed digests: frontmatter title/aliases/tags,
      date_created name+format, date_modified name+format; tasks
      statuses/tag_filters; schemas class_field. Built by exhaustive
      destructures at both levels (`root`, `templates`, and `schemas.directory`
      explicitly ignored): adding any new config setting breaks the
      constructor and forces an invalidation decision. Never use `..` there.
      `IndexerService::from(&Config)` computes it once from
      `Config::digests()` and keeps it beside `class_field`. Title, aliases and
      the date keys are not read at index time today; they stay in on purpose
      (fail-closed, one harmless rebuild per edit), recorded in the type docs.
      Drop the now-live `expect(dead_code)` on `FrontmatterDigests`,
      `DateFieldDigests`, and `TaskDigests`; keep it where accessors stay
      unused (`root`, templates, `schemas.directory`).
- [x] `IndexSetting` enum (index-owned; derives `Copy`/`PartialEq`; one variant
      per fingerprint digest, named after the raw.toml key; qualified `Display`
      such as `[frontmatter] tags` and `[frontmatter.date_created] name`) for
      diagnosis only: config never learns of it. It is `pub` and re-exported
      through the `testing` facade because `RefreshReport` is public there.
- [x] The existing `cache_format` table is renamed `meta` (const `CACHE_FORMAT`
      → `META`, redb table name `"meta"`, `write_cache_format` → `write_meta`,
      version consts renamed to match) and becomes the store's single metadata
      table: no extra `TableSpec`. It gains a `fingerprint` key beside
      `version` (value = the 10 raw 32-byte digests concatenated;
      `Blake3Digest` gains a `pub(crate)` bytes codec). The version stays
      `"1"`: every legacy store has no `meta` table, so `rebuild_needed` sees a
      partial schema and wipes it at open (free one-time rebuild). `write_meta`
      writes both keys in the txn that already writes the index rows, so it
      takes the `IndexFingerprint` from `PersistRequest` beside `dimensions`,
      at both write sites (service persist and PendingApply; the first build
      into a virgin store goes through the incremental path, so both need it).
      The row persists exactly the `IndexFingerprint`, never the whole config.
- [x] `RefreshPlan::collect` takes the expected fingerprint and reads the
      `fingerprint` key in the same read txn as the file listing (a combined
      store read; the current `read_all_files` opens its own txn).
      `prepare_pass` gains `Drifted { index, store }` (drift wins over
      staleness). Virgin means the `meta` table is absent. `open`
      guarantees all tables or none, so there is no "tables exist, zero rows"
      case. A virgin store stays on the honest first-build `FilesChanged`
      path. A present table with a missing or wrong-length `fingerprint` is
      drift (fail-closed); with nothing stored to diff, `changed` lists every
      `IndexSetting`.
- [x] Heal for drift is the existing rebuild verb, handing the plan's own
      store back to the refresh path: no second `IndexStore::open` (redb
      rejects a second handle on the file; today's `persist()` opens its own).
      Factor a `build_from(files)` out of `build()` so the heal reuses the
      plan's scan instead of rescanning. `refresh()` stays fail-open (log the
      persist failure, return the correctly built index; the store keeps the
      old fingerprint, so the next refresh heals again). `refresh_store()`
      propagates the persist error. `load()` remains untouched (documented
      as-persisted for tests/benches).
- [x] `RefreshReport` collapses into
      `enum RefreshReport { #[default] Unchanged, FilesChanged { upserted,
      deleted, links_modified }, ConfigDrift { changed: Box<[IndexSetting]> } }`
      with count accessors kept as documented methods (they return 0 for
      `Unchanged` and `ConfigDrift`: a heal reports settings, not row counts);
      no file-path lists. Loses `Copy`: the test/bench comparison sites are
      updated, `log_report` handles all three variants, and the `tests/baselines`
      public-API and pub-token files are regenerated.
- [x] Tests: inverted pinned test (`index::service::tests::class_field::
      fresh_refresh_serves_persisted_class_axes_unchanged_when_class_field_config_changes`
      becomes: config `class_field` edit → `ConfigDrift` with the exact changed
      settings and axes re-derived from the new config, no `traces index`);
      legacy-store rebuild test (a store whose metadata table is still named
      `cache_format` wipes and rebuilds at open; model on the existing
      outdated-version test in `index/store.rs`); missing-`fingerprint`-key →
      drift with all settings;
      framing/encoding pins (fingerprint value round-trip); an E2E canary:
      edit config → re-trust →
      query healed without `traces index`. Fixtures that persist by hand with
      `PersistRequest::rebuild(IndexDimensions::for_class_field(..), ..)`
      (`index/service.rs` ~L1204, `index/store.rs` helpers and tests) must
      persist the matching fingerprint or they now observe drift.
- [x] Glossary: new **Index Fingerprint** entry; extend **RefreshState**
      and **Refresh Report** (the Config Fingerprint amend lives in
      config-service/07, which defines the new encoding). Amend ADR 0009
      (index-refresh) consequences to "`meta` table; legacy stores". Add
      a supersession comment to integration-e2e-suite issue 04 (config
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

> *This was generated by AI during triage.*
> Re-triaged against `main` (`def3429b`). State stays `ready-for-agent`, with
> corrections where the ticket had drifted from the code:
>
> **Verified.** Staleness is real on `main`: the pinned test
> `fresh_refresh_serves_persisted_class_axes_unchanged_when_class_field_config_changes`
> passes, so a `class_field` edit gives an empty report and old-field axes.
> config-service/07 is on `main`: `Config::digests()` and the per-section
> `*Digests` accessors exist. Ticket 07's own file is still open (its manual
> dummy-field compile-fail check and the final `mise run verify` were never
> observed). Run `mise run verify` on `main` before starting.
>
> **Corrected premises.**
> - The store already has a metadata table (`cache_format`: format version
>   row, written in the same txn as the index rows, checked by
>   `rebuild_needed`). It is renamed `meta` and the fingerprint joins it as a
>   second key, instead of adding a table. The rename itself wipes legacy
>   stores at open. The ticket's "8th table / 7-table legacy store" premise
>   was stale.
> - The fingerprint has 10 digests, not 8: `DateFieldDigests` carries name and
>   format separately. Maintainer chose to keep all 10, including title,
>   aliases and the date keys that no index-time code reads today.
> - Heal cannot call `rebuild()`/`persist()` as is: both open a second
>   `IndexStore` while `RefreshPlan` still holds the first.
> - `RefreshReport` is public via `testing`, so `IndexSetting` is public too
>   and API baselines change.
>
> **In-flight work.** `rust-design-revision` (dirtree refactor, test-only
> hunk in `index/service.rs`) and its `improve/*/trial-N` branches touch
> `src/index/service.rs` only in the `scan` tests; no overlap with refresh or
> store code. Branch from `main`; no sequencing constraint. Note ADR numbering
> collision: `docs/adr/` has two `0009-*` files; this ticket means
> `0009-index-refresh.md`.
>
> **Brief.** Seams: `IndexerService::{from, prepare_pass, refresh_with_report,
> refresh_store, persist, rebuild}` (`src/index/service.rs`);
> `RefreshPlan::collect`, `RefreshState`, `RefreshReport`, `PendingApply::apply`
> (`src/index/refresh.rs`); `PersistRequest`, `IndexStore::{open,
> rebuild_needed, persist}` (`src/index/store.rs`); `TABLES`
> (`src/index/tables.rs`); `Config::digests` (`src/config/model/digest.rs`).
> Gate: `mise run verify`.
>
> ### Implementation notes
>
> - The digest bundles' fields are now `pub(crate)` and their accessors are
>   gone (except `TemplateDigests`, whose accessors stay test-only): a
>   constructor outside `config` can only destructure exhaustively if the
>   fields are visible. `IndexFingerprint::new` is that constructor, pinned
>   against `..` by a source-scanning test.
> - `IndexFingerprint` is 320 bytes, so `PersistRequest` and `PendingApply::apply`
>   borrow it, `StoredFingerprint::Recorded` boxes it, and `IndexerService`
>   keeps it boxed (clippy `large_types_passed_by_value`,
>   `large_enum_variant`, and the bench stack-frame lint).
> - Found, not fixed (out of scope): the incremental persist path only opens
>   the tables it writes, so a first build into a virgin store of notes with no
>   links, tags, or classes leaves a partial schema. The next `open` sees
>   "present and missing" and wipes it, so the second refresh rebuilds again.
>   Results stay correct, and drift cannot be misreported (a wiped store is
>   virgin), but the "all tables or none" premise above does not hold for that
>   case.
