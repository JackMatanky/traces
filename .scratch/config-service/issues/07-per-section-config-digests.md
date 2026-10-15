# 07: Per-Section Config Digests and the Config Fingerprint

**What to build:** The config module can produce per-section digests of its own
settings, and a whole-config fingerprint derived from them, so downstream
consumers (starting with the index) can detect config changes byte-exactly.
Encoding only — no behavior change, no index knowledge anywhere in this ticket.

**Blocked by:** None (can start immediately).

**Category:** enhancement

**Status:** ready-for-agent

- [ ] `Config::digests()` returns `ConfigDigests` aggregating inherent
      per-section `digests()` methods: frontmatter (title, aliases, tags,
      date_created, date_modified), tasks (statuses, tag_filters), schemas
      (directory, class_field), templates, plus `root`. Each section type
      destructures exhaustively (`..` forbidden) so adding a setting to
      `Config` breaks the constructor and forces a placement decision.
- [ ] Each per-setting digest is framed by a section-qualified `const` label
      (e.g. `b"schemas.class_field"`, `b"frontmatter.tags"`) before the
      value's bytes enter the hasher — stored digests are self-describing
      and collision-free outside their bundle field. Framing lives outside
      the existing `feed_into` methods; label consts, not `stringify!`.
- [ ] `FieldName` gains `feed_into(&self, fp: &mut Blake3Hasher)` owning the
      contract "a field name enters digests as exact UTF-8 bytes"; the
      existing `as_str().as_bytes()` reach-throughs in the config digest
      chain migrate to it (encoding-neutral — gated by the pin test).
      `hash.rs` stays std-only; dependency direction `field → hash`.
- [ ] `Config::hash()` is re-expressed as compose-all of the bundles under
      `b"traces.config.v1"` (the Config Fingerprint); the existing pin test
      is re-pinned once against the new encoding. Single live encoding —
      the old `feed_into` aggregate chain does not survive as a second
      persisted form.
- [ ] Unit tests pin the encoding: per-setting digest framing (label +
      value bytes), bundle composition, and `Config::hash()` full encoding
      round-trip.
- [ ] Exhaustive-destructuring is a pinned convention: the "every section
      type destructures exhaustively, `..` forbidden" rule is documented in
      the config module docs, and a compile-fail test (trybuild or
      equivalent) pins it — this convention is the fail-closed guarantee
      for every future config setting, so review-by-convention alone is
      not acceptable.
- [ ] Glossary: the **Config Fingerprint** entry is amended to describe the
      compose-all-per-bundles encoding (single live encoding, one-time
      re-pin) instead of the old aggregate chain.

## Comments

> *Filed from a grill-me design session on config ↔ index invalidation
> (see index-query/17 and ADR 9). The index-facing half of that design
> lives in index-query/17; this ticket is the encoding-only prefactor so
> the change is easy before the easy change.*
