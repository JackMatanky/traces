# What owns index freshness: config drift, path-set change, format version?

Type: grilling
Status: open
Blocked by: 5, 18, 20

## Question
Define one freshness model for the persistent index: how inputs that affect derived rows (config tasks/frontmatter/class_field, parser version, path set, file content) are enumerated and fingerprinted; where the fingerprint lives (inside redb `meta` atomically vs companion file); granularity of invalidation (whole cache vs per-axis vs per-entry); handling of the path-set change cliff for link resolution (all-or-nothing `is_paths_unchanged` vs unresolved-stem inverse index); format-version policy; failure policy (fail-open fallback retained?).

Facts: ADR-0009 and `.scratch/index-query/issues/17-...`, `survey/index.md` H2/H3, hardcoded cache format `b"1"` (`src/index/tables.rs:73`), pinned stale test `src/index/service.rs:1749-1780`, config digests (`survey/config-dialog.md` H1).
Alternatives: implement ADR-0009 as written; derive-style `CacheKey` over consumed inputs; whole-cache invalidation on any drift (simplest); keep manual `traces index`. Challenge ADR-0009 explicitly.
New fact to verify (`research/05`): the delta uses bare mtime/size without a racy-timestamp guard like git's `is_racy` (`gix-index/src/entry/stat.rs`); decide whether content hashing fallback is part of the freshness model.
Measured (`research/11`): at 5k notes the path-change cliff is 97% note deserialization (18.5 ms) and 2.7% link re-resolution (0.5 ms); an inverted unresolved-stem index (suggested by `research/05` precedent) would attack the wrong cost. Decide on note caching/laziness vs accepting the cost.
Inputs: `research/05`, `research/11` (#2), config-ownership decision, read-model decision.
