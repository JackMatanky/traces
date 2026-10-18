# Carry-forward evidence gaps: cross-kind compare and minijinja 2.24.0 capabilities

**Ticket:** `.scratch/arch-redesign/issues/21-research-carry-forward-evidence-gaps.md`
**Date:** 2026-10-18
**Baseline:** `main` @ `17f0d585` (only change since `0c43149d` is a doc-comment edit in `src/query/grammar/filter.rs`, 8 lines in, 8 out, so all cited line numbers hold).
**Method:** read the code and the vendored crate sources in `~/.cargo/registry/src/`; nothing was run, nothing in the repo was modified.

## Gap A: does cross-kind `NoteFieldValueRef::compare` reach `local_zone()` per comparison?

**Verdict: VERIFIED yes, by static reading. Per-call cost UNMEASURED.**

| Step | Evidence |
|---|---|
| The `Date`/`DateTime` cross-kind arms of `compare` convert the date side every call | `src/note/field.rs:297-300`: `DateTimeValue::try_from(*a)` / `(*b)` inside both arms, with no caching |
| That conversion reads the ambient zone | `src/temporal/datetime.rs:340-345`: `TryFrom<DateValue> for DateTimeValue` calls `local_zone()` then `from_local_in` |
| `local_zone()` is uncached | `src/temporal/zone.rs:13-19`: under `cfg(test)`/`test-utils` an override, otherwise `resolve(TimeZone::try_system)` on every call; no `OnceLock` or memo |
| Cross-kind is reachable from filters | `src/query/grammar/filter.rs:807` (`value.compare(&literal.as_ref())`) and `:832` (`lhs.as_ref().compare(&rhs.as_ref())`) are the only production callers for `<`, `<=`, `>`, `>=`; `Eq`/`Ne` take `is_equal_to_literal` instead and are not covered by this finding |
| Cross-kind is reachable from nested values | `src/note/field.rs:518` (`compare_objects`) recurses with `compare` on values of equal keys |
| Same-kind pairs never touch the zone | `field.rs:295-296`: `Date`/`Date` and `DateTime`/`DateTime` use `Ord` directly |
| Failure mode is silent | on `try_from` error the arms return `Less` (`:298`) or `Greater` (`:300`) instead of propagating `DateError::LocalZoneLookup`, so a zone-discovery failure turns into a fixed ordering |

What this does **not** establish:

- Per-call cost of `TimeZone::try_system`. Research 03 measured whole-filter cost (0.8 ms for a plain date filter vs 2.2–2.7 ms with calendar arithmetic, at 20K notes) but did not isolate the cross-kind comparison; a micro-measurement is needed and was not added to the repo.
- Sorting. Research 03 found that sort keys are precomputed, so the sort compare loop does not call `compare` on cross-kind pairs; this ticket did not re-verify that and relies on research 03.
- Prevalence: whether realistic vaults mix bare dates with datetimes in one field, and whether queries compare a date field against a datetime literal.

**Design relevance.** It strengthens the evidence for the *Ambient environment* ticket (the zone is read in a *comparison function*, not just in parsing/formatting paths), and it gives *Query core* a concrete semantic fact: ordering between a `Date` and a `DateTime` is not a pure function of the two values; it also depends on the process zone and swallows lookup failure.

## Gap B: which minijinja capabilities cited by research 07 exist in the pinned 2.24.0?

**Verdict: VERIFIED by reading both crate sources.** `Cargo.toml:131` pins `minijinja 2.24.0` with features `adjacent_loop_items, builtins, debug, macros, multi_template, serde`; 3.0.0 is also present in the local registry.

| Capability (research 07, Lesson 7) | 2.24.0 | 3.0.0 |
|---|---|---|
| `Template::undeclared_variables(nested)` | present, stable (`src/template.rs:425`) | present (`src/template.rs:317`) |
| `Expression::undeclared_variables(nested)` | present (`src/expression.rs:95`) | present (`src/expression.rs:93`) |
| `State::get_or_insert_extension` / `_with` (render-local typed storage) | **absent**: no match in `src/vm/state.rs`, and no match for "extension" anywhere in `src/vm` | present (`src/vm/state.rs:537`, `:547`) |
| String-keyed render-scoped storage | present: `State::get_temp` / `set_temp` (`src/vm/state.rs:357,364`, `Mutex`-guarded `Value` map), which is the facility the current code's `query.index_cache` / `schema.service_cache` entries are built on | not re-checked |
| `minijinja::machinery` (AST access) | gated behind the `unstable_machinery` feature, "no semver guarantees" (`src/lib.rs:193-196, 254-260`); not enabled in `Cargo.toml` | gated behind `unstable_machinery` (`src/lib.rs:257-258`) |

Consequences for the template-engine boundary:

- Static variable discovery needed by the LSP (`undeclared_variables`) is available **today** without a version bump or the unstable feature. Research 07's "requires 3.0" reading is wrong for this half of Lesson 7.
- Typed render-local extensions (`get_or_insert_extension`) are available **only after a bump to 3.0**; on 2.24.0 the only render-scoped store is string-keyed `Value` via `get_temp`/`set_temp`, so a typed cache would have to be held by the host and reached some other way (for example a custom `Object` or a captured `Arc`).
- Raw AST walking (`machinery`) needs the unstable feature in either version, so it is a risk-bearing choice rather than a free default.
- Not checked: what else changed between 2.24.0 and 3.0.0 (breaking API changes, MSRV, behaviour of existing filters), so whether a bump is cheap is UNKNOWN. A bump is also not a template-boundary decision by itself; flag it for the *Template engine boundary* grilling as a cost to price, not a premise.

## What this ledger changes in earlier findings

| Earlier statement | Status now |
|---|---|
| Research 07, caveat line: availability of the 3.0 items on 2.24.0 "must be checked" | Checked: `undeclared_variables` present, `get_or_insert_extension` absent (table above) |
| Research 07 Lesson 7: "`State::extensions` replaces custom downcasting caches" | Holds only on 3.0; on 2.24.0 it is not an available replacement |
| Research 01 / ticket *Ambient environment* "indirect: `note/field.rs:297,299` (cross-kind compare)" | Confirmed as a per-comparison `local_zone()` call, uncached, error-swallowing |
| Research 03: "ambient lookups hit filter evaluation, not sorting" | Consistent: the filter comparison path (`filter.rs:807,832`) is a call site of the cross-kind arms |

## Not investigated (still open)

- Per-call cost of `local_zone()` (micro-benchmark would be needed).
- Whether the `index-query/17` "8th table" wording and the "7-table legacy store" wording can be reconciled with `main`'s 8 tables; the in-flight ticket owns that and it lands separately.
- Whether rumdl-style content-hash freshness is needed for the cache; this is a decision, not evidence.
