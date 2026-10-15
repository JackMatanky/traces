---
number: 9
title: "Refresh-driven Index Rebuilds: Freshness Sources and Drift as First-Class Refresh State"
date: 2026-10-15
status: proposed
links:
  - target: 5
    kind: relatesto
---

# 9. Refresh-driven Index Rebuilds: Freshness Sources and Drift as First-Class Refresh State

Date: 2026-10-15

## Status

Proposed

## Context

The persisted redb index (ADR-0005) treats only file metadata (path/size/mtime
deltas) as its freshness signal. A config change that affects how notes are
indexed (frontmatter field mappings, task status sets, the schemas class_field)
leaves untouched files "fresh", so queries silently serve stale axes until an
operator runs `traces index`. This was recorded as correct-as-designed in the
integration E2E investigation (issue 04), with warn-only mitigations. Refresh
also has no vocabulary for "the persisted index no longer matches the
configuration that produced it": RefreshState models only Unchanged/Stale, and
the store has no metadata linking its rows to the config that built them.

## Decision

Index refresh treats freshness as the union of independent signals: file
metadata (FileDelta), the config fingerprint of the index-relevant settings
(stored in a meta table in the same transaction as the rows), and the store
format version. Their failure is modeled as three first-class refresh states:

1. Fresh: persisted rows serve directly.
2. Stale: file deltas require an incremental apply.
3. Drifted: the stored config fingerprint or format version no longer matches;
   drift wins over staleness.

Heal for drift is the existing rebuild verb: the refresh path performs a full
rebuild and reports it, so operators never need a separate `traces index`
invocation after a config change. The report distinguishes what happened
(Unchanged / FilesChanged / ConfigDrift with the specific changed settings)
rather than a bare row count.

Boundaries this decision keeps:

- Dependency direction is one-way: the index consumes config digests; the config
  module never learns of the index.
- Only index-relevant settings enter the stored fingerprint: the index's meta
  row persists exactly what it compares, never a whole-config archive.
- Fail-closed completeness is enforced at compile time: the index's fingerprint
  constructor destructures the config digest bundles exhaustively, so adding any
  config setting forces an explicit invalidation decision.
- Drift detection is deliberately not generalized to external companions: the
  trust state dir's Companion Hash pattern does not apply; the fingerprint lives
  inside redb, atomic with the rows it protects.

## Consequences

Good, because:

- Config edits that affect indexing heal automatically on the next refresh;
  operators never need a separate `traces index` after a config change.
- The warn-and-defer mitigations from the E2E investigation (issue 04) are
  superseded by detection and heal.
- Drift is one first-class refresh state, so future drift classes (format
  version, external companions) extend the same seam instead of adding new
  ones.
- Fail-closed completeness is compile-time: adding any config setting forces
  an explicit invalidation decision at the index fingerprint constructor.
- Settings that do not affect indexing (root, templates, schema directory)
  are deliberately excluded, so moving a project does not rebuild.

Bad, because:

- RefreshState and RefreshReport change shape, breaking their test and bench
  call sites.
- The store gains an 8th table; legacy 7-table stores are wiped once via the
  existing upgrade path (full rebuild on first open after upgrade).
- Config::hash() is re-expressed as a composition of per-section digests,
  requiring a one-time re-pin of its encoding test.

Neutral:

- The load() path remains explicitly as-persisted for tests and benches; it
  participates in neither freshness nor drift detection.

Implementation tickets: config-service/07 (per-section config digests, the
encoding prefactor) and index-query/17 (drift detection and rebuild).
