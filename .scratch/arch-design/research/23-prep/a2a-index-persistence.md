# Index persistence and note decoding

This file describes how the index stores and decodes a `Note`. Citations come
from the agent that wrote the first version. I checked the claims marked
"checked now". The other claims keep the tag of the first version.

## Summary

- [V] The `NOTES` table stores the whole `Note` as one postcard blob for each
  path (`src/index/tables.rs:26-28`). Checked now.
- [V] No separate table holds items, tasks, fields or body facts. The store has
  eight tables.
- [V] The format version is `META_VERSION = b"1"` at the key `b"version"`
  (`src/index/store.rs:38-41`). A mismatch wipes the store at open. Checked now.
- [V] `IndexerService::refresh_with_report` parses only changed files. It does
  not decode every stored note on a stale pass (`src/index/service.rs:155-230`).
- [V] A fresh store decodes all stored notes with `store.read_all_notes()`
  (`src/index/service.rs:194-206`; `src/index/store.rs:513-526`).
- [H] A partial decode of a `Note` row needs a format change. The `Note` is one
  blob. No lazy-field strategy exists in these files.
- [P] A new field changes the postcard bytes of the `Note`. Increase
  `META_VERSION` when the `Note` shape changes. Ticket 16 makes the `index.redb`
  layout Free.

## Stored representation

- [V] `NOTES` maps the path bytes to the entire serialized `Note`.
- [V] `FILES` holds `FileMeta`. `LINKS` holds the target-to-source path edges.
  Four multimaps hold the tag and class axes. `META` holds the version and the
  config fingerprint (`src/index/tables.rs:16-79,102-135`;
  `src/index/store.rs:1647-1677`).
- [V] `encode_row` uses `postcard::to_extend` with a reusable `Vec`.
  `decode_row` uses `postcard::from_bytes` (`src/index/codec.rs:27-55`).
- [V] Full rebuild and incremental upserts give `&Note` to the generic row
  writer (`src/index/store.rs:642-661,1502-1529,1606-1618`).
- [V] The tag lookup, the class lookup, the folder path lookup and the inlink
  point lookup need no note decoding
  (`src/index/store.rs:235-324,1040-1053,1115-1127`).
- [V] A changed-file delta reparses the changed Markdown files. It writes whole
  note rows. It derives the tag and class axes from `Note::tags()` and from the
  configured frontmatter `get_values(key)` strings
  (`src/index/service.rs:328-355`).

## Invalidation

- [V] A missing version, a different version, a partial schema, a type mismatch
  or a recognized redb corruption wipes and recreates the database at open
  (`src/index/store.rs:38-43,166-189,870-880,894-978`).
- [V] A fingerprint that cannot be read, or a changed setting, causes a full
  reparse on refresh. It does not wipe the store at open
  (`src/index/store.rs:664-686,539-569`; `src/index/refresh.rs:337-358`;
  `src/index/service.rs:208-217`).
- [H] The first version says `META` holds ten BLAKE3 digests. Ticket 22 deletes
  `ConfigDigests` and makes `IndexPolicy::fingerprint()` the only source of the
  fingerprint. Check the current code before you rely on the number ten.
- [H] A change of the `Note` wire shape, without a `META_VERSION` increase, can
  cause decode errors on old rows. It can also read compatible bytes with a
  wrong meaning.
- [V] Temporal types and `NoteFieldValue` use `serde` through the `Note`
  derivation. Postcard writes them in a binary form. A new span field or a
  `TaskStatus` change alters the blob.

## Decode paths

- [V] A stale refresh for a content-only change calls
  `persisted.patch_and_diff(&edited, &new_edges)`. It does not decode the stored
  notes again. The inlink edges come from `plan.store().read_all_links(...)`
  (`src/index/service.rs:222-229`; `src/index/refresh.rs:417-433`).
- [V] A stale refresh for a path change that goes beyond content calls
  `store.read_all_notes()` in `merge_refreshed_notes`. This decodes all stored
  notes (`src/index/refresh.rs:435-460,557-575`).
- [V] A point read of chosen notes goes through `IndexStore::read_notes_batch`
  and `decode_row`. Most flows read in bulk
  (`src/index/store.rs:193-207,981-1013`).

## Tests

- [V] The tests `test_fresh_rebuild_equals_delta_refresh` and
  `test_incremental_persistence` assert that a full scan equals an incremental
  refresh. This protects the Frozen rule "index results equal a fresh parse"
  (ticket 16). The agent did not give a path for them. Check the names before
  you cite them.

## Changes after `207cc3fb`

- [H] The agent says the changes in `src/index` are mostly formatting. It says
  `src/index/service.rs` has a larger rework of error variants and of the
  `build` and `scan` logic. I did not check this.

## Lessons for ticket 23

- C12: The whole-`Note` blob is the unit of decoding. Any design that keeps a
  smaller index record needs a new table or a split `Note`. [P]
- C2, C12: Extra fields in the `Note` raise the bytes of every row. Measure the
  size first. The probe in `g-probe-results.md` measures +4.3% for spans.
- C8: The `Note` shape belongs to the format version. Ticket 25 owns the parser
  version and the format version.

## Not checked

- The cost of decoding in a real refresh. This file relies on research 11.
- The number of digests in `META` after ticket 22.

## Inconsistencies found while rewriting

- The first version had the headings "Wire types and format-change consequences"
  and "Baseline delta and equivalence tests" without content. The decode-path
  facts sat under the wrong heading. This version moves them.
- The first version had an empty `## Not checked` section.
- The first version lists tests without paths.
