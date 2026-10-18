# 07: Per-Section Config Digests and the Config Fingerprint

**What to build:** The config module can produce per-section digests of its own
settings, and a whole-config fingerprint derived from them, so downstream
consumers (starting with the index) can detect config changes byte-exactly.
Encoding only: no behavior change, no index knowledge anywhere in this ticket.

**Blocked by:** None (can start immediately).

**Category:** enhancement

**Status:** ready-for-agent

## Acceptance Criteria

- [ ] `Config::digests()` returns bundles covering: frontmatter (title,
      aliases, tags, date_created name+format, date_modified name+format),
      tasks (statuses, tag_filters), schemas (directory, class_field),
      templates (local, global, output), and root: every setting currently
      in the fingerprint is represented. Each section type destructures
      exhaustively in its digest constructor (`..` forbidden) so adding a
      setting to `Config` breaks compilation and forces a placement decision.
- [ ] Each per-setting digest hashes its section-qualified `const` label
      (e.g. `b"schemas.class_field"`, `b"frontmatter.tags"`) before the
      value's bytes, so each stored per-setting digest is self-describing and
      cannot collide with a sibling. Labels are `const` byte strings, not
      `stringify!`; framing lives outside the existing value-feeding methods.
      A unit test pins the exact framing.
- [ ] `FieldName` gains `feed_into(&self, fp: &mut Blake3Hasher)` owning the
      contract "a field name enters digests as exact UTF-8 bytes"; the
      existing `as_str().as_bytes()` reach-throughs in the config digest
      chain migrate to it (encoding-neutral, gated by the pin test), and no
      such reach-throughs remain in that chain. `hash.rs` stays std-only;
      dependency direction `field → hash`, never the reverse.
- [ ] `Config::hash()` is re-expressed as compose-all of the bundles under
      `b"traces.config.v1"`. The old aggregate `feed_into` chain is deleted,
      not kept alongside: single live encoding (grep-verified: the old chain
      no longer exists as a second persisted form, and the old pinned digest
      value no longer appears anywhere).
- [ ] The existing pin test is re-pinned once against the new encoding; the
      one-time change is intentional and accepted.
- [ ] Unit tests pin the encoding: per-setting framing (label + value bytes),
      bundle composition, and the full `Config::hash()` round-trip.
- [ ] The exhaustive-destructuring convention is documented in the config
      module docs and pinned by an automated test: a `syn`-based
      source-scanning unit test that parses the config model source and fails
      if any digest constructor uses a rest pattern (`..`). (Amendment to the
      original "trybuild or equivalent": the config types are crate-private,
      so trybuild's external-crate harness cannot observe them; the `syn`
      test is the equivalent that actually works. Optionally also deny
      `clippy::rest_pat_in_fully_bound_structs` workspace-wide as a cheap
      supplement; it only catches redundant `..`, so the `syn` test remains
      the real pin.)
- [ ] Adding a dummy field to a section type breaks its digest constructor at
      compile time (manual verification during review; demonstrates the
      fail-closed property the convention exists for).
- [ ] Glossary: the **Config Fingerprint** entry is amended to describe the
      compose-all-per-bundles encoding (single live encoding, one-time
      re-pin) instead of the old aggregate chain.
- [ ] `mise run verify` passes.

## Current / Desired Behavior

**Current:** A loaded `Config` exposes a single `hash()` that feeds every
resolved value into one `Blake3Hasher` via an aggregate `feed_into` chain:
root path, then templates, schemas, frontmatter, and tasks each dumping raw
values with no per-setting framing. The encoding is pinned by a unit test, but
the stored digest is opaque: a consumer cannot tell *which* setting changed
without recomputing and diffing blind. The aggregate chain also means there is
no granularity available to any future invalidation consumer.

**Desired:** The config module owns a `ConfigDigests` type produced by
`Config::digests()`. Each section type (`TemplateConfig`, `SchemasConfig`,
frontmatter config, task config) gains an inherent `digests()` method
returning its own bundle. Within a bundle, every individual setting's digest
is framed by a section-qualified label constant so each stored per-setting
digest is self-describing. `Config::hash()` becomes compose-all of the bundles
under `b"traces.config.v1"`. The exhaustive-destructuring convention is the
fail-closed guarantee for every future config setting.

## Key Interfaces

- `Config::digests() -> ConfigDigests`: new; aggregates the per-section bundles
- `ConfigDigests`: new type; one framed digest per setting, plus root
- Per-section inherent `digests()` on each config section type: new; must
  destructure exhaustively
- `Config::hash() -> Blake3Digest`: same signature; body becomes compose-all
  under `b"traces.config.v1"`; old aggregate chain deleted
- `FieldName::feed_into(&self, fp: &mut Blake3Hasher)`: new; absorbs the
  UTF-8-bytes contract
- Section label consts (e.g. `b"frontmatter.tags"`): new `const` byte strings,
  one per setting

## Out of Scope

- Any index knowledge: no `IndexFingerprint`, no drift detection, no store
  changes; that is index-query/17, which consumes `ConfigDigests` but must
  not be built here
- Runtime behavior changes of any kind; this is encoding-only
- Persisting digests anywhere; no consumer exists yet outside tests
- Keeping the old aggregate chain alive in any form (dual encodings forbidden)

## Comments

> *Filed from a grill-me design session on config ↔ index invalidation
> (see index-query/17 and ADR 9). The index-facing half of that design
> lives in index-query/17; this ticket is the encoding-only prefactor so
> the change is easy before the easy change.*

> *This was generated by AI during triage.*
> Triaged to `ready-for-agent` with an integrated agent brief. One amendment
> made during triage review with the maintainer: the compile-fail pin is a
> `syn`-based source-scanning unit test instead of trybuild; the config types
> are crate-private, so trybuild's external-crate harness cannot observe them,
> making trybuild architecturally incapable of pinning this convention.
