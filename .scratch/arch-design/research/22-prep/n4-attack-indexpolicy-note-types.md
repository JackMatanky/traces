# Stress Test: IndexPolicy and Note Types (Q1, Q2, Q8, Q9)

This analysis evaluates the design where `note` owns the parser input (Q1, Q9), root vocabulary stays shared (Q2), and `index` owns the `IndexPolicy` fingerprint (Q8).

## 1. Conflict Audit

| Decision | Verdict | Conflicts |
| :--- | :--- | :--- |
| Q1 (note-owned parser) | Holds | Fine (no hard conflict; separation of concern) |
| Q2 (root vocabulary) | Holds | Fine (neutral) |
| Q8 (index-owned policy) | Holds with change | Gap (needs clear projection of HostInputs for period policy) |
| Q9 (parser-borrowed input) | Holds | Fine |

- **Soft Conflict (Q8 vs Q3):** Q3 (`HostInputs`) and Q8 (`IndexPolicy`) must agree on the definition and propagation of filename-period policy. If `index` owns `IndexPolicy`, `IndexPolicy` must receive the period policy via a `HostInputs` projection.
- **Gap (Q8):** The mechanism for `IndexPolicy` to receive inputs from the `template` engine (which also uses `Config`) is undefined.

## 2. Attack Analysis

### (1) IndexerService Visibility

The assumption that `IndexerService` can "never" see `Config` is correct for production. However, `IndexerService::for_tests` and `template/engine.rs` (which builds an indexer, per `template/engine.rs:125-135`) are legitimate *exceptions*. The seam should be: `Config` -> `Projected Policy` (in test or init) -> `IndexerService`. `IndexerService` itself should only hold the `IndexPolicy`.

### (2) Exhaustive Destructure Seam

To enforce compile errors for nested additions, the `IndexPolicy` projection must use `let IndexPolicy { field1, field2, .. } = config;` *without* `..`. This is only possible if we *know* all fields. The `syn` scan test must verify that the *seam function* in `config` has no `..` in the `IndexPolicy` construction.

### (3) Ignore Policy

If `TRACES_IGNORED_DIRS` changes:

1. `IndexerService` sees a different `IndexPolicy`.
2. `IndexPolicy::fingerprint()` changes.
3. `Store` detects the fingerprint mismatch.
4. Current design: Delta or rebuild? A change in ignore policy *requires* a full rebuild (or at least a full re-scan) because it changes the set of candidate files, not just the content of already-indexed files. A simple delta is insufficient unless the `Store` tracks file existence relative to the *old* ignore policy. Ticket 25 should specify if the re-scan is forced.

### (4) Type Conflicts (18-20, 23, 34)

- **18-20:** `FieldName`/`FieldKey` must remain root-level.
- **23:** If `Note` identity depends on parser config, then parser config *cannot* just be a borrow; it must be part of `Note`'s `Hash`. Q1 BORROWing seems to conflict with `Note` identity needing the config.
- **34:** LSP needs to parse unsaved text with the same policy. `IndexPolicy` (derived from `Config`) is correct, provided the LSP can projection the same policy from `HostInputs`.

### (5) Root Vocabulary Neutrality (Q2)

The root types `TaskStatus`, `FieldKey`, etc., are foundational. Moving them to `note` creates a dependency from `config` to `note` (if `config` needs them), which is a cycle. Root-ownership is the only stable design.

### (6) Template Engine Usage

`TemplateService` holding `&'a Config` means `TemplateService` has access to the full `Config`. This is fine because `TemplateService` acts as a consumer, not a producer of the index. The inconsistency is acceptable.

## 3. Alternative Designs

### Fingerprint Ownership Alternatives

- **Alt A (Fingerprint Trait):** Each config type (`ParserConfig`, `IgnorePolicy`, etc.) implements a `Fingerprint` trait. `IndexPolicy` aggregates them. *Benefit:* Distributed ownership; fingerprint logic is localized to the type. *Drawback:* Boilerplate for `IndexPolicy` to aggregate them.
- **Alt B (Builder-Driven Fingerprint):** `IndexPolicy` is constructed via a builder that *automatically* collects hashes of all inputs as they are added. *Benefit:* Eliminates drift between fingerprint and fields. *Verdict:* Alt B is superior (less drift).

### Status/Tag Ownership Alternatives

- **Alt C (Config-Owned Vocabulary):** Move `TaskStatus` and `Tag` types to `config`. *Benefit:* Clearer ownership. *Drawback:* Likely creates cycles (as analyzed in Q2).

## 4. Summary Table

| decision id | verdict | required change | conflicts |
| :--- | :--- | :--- | :--- |
| Q1 | holds | none | none |
| Q2 | holds | none | none |
| Q8 | holds with change | explicit projection seam definition | soft-conflict (Q3) |
| Q9 | holds | none | none |

*Summary:* The design of `IndexPolicy` as a projection of `Config` is robust, provided the projection is explicit and strictly managed by the `Config` service seam. The primary risk is fingerprint drift, which should be mitigated by making the projection builder automatically hash all fields added to it.
*File:* `.scratch/arch-design/research/22-prep/n4-attack-indexpolicy-note-types.md`
