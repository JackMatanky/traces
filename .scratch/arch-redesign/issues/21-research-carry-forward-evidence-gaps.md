# Research: carry-forward evidence gaps from the first research wave

Type: research
Status: resolved

## Question

The first research wave left two facts that decision tickets depend on explicitly unchecked: (a) whether `NoteFieldValueRef::compare` (`src/note/field.rs:297,299`) reaches `local_zone()` on every cross-kind date comparison, and (b) which of the minijinja 3.0 capabilities cited by the template/query precedent research exist in the pinned 2.24.0. Establish both from source. **Do not modify production code, tests, benches, or docs.**

## Output

`.scratch/arch-redesign/research/21-carry-forward-evidence-gaps.md`, graded verified / unmeasured / unknown. Dependents are updated by the orchestrator, not by this ticket.

## Answer

Detailed in `.scratch/arch-redesign/research/21-carry-forward-evidence-gaps.md`.

1. **Cross-kind date compare is zone-dependent and uncached**: `Date` vs `DateTime` in `compare` calls `DateTimeValue::try_from(DateValue)` → `local_zone()` on every comparison (`field.rs:297-300`, `datetime.rs:340-345`, `zone.rs:13-19`), reachable from `<,<=,>,>=` filters (`filter.rs:807,832`) and nested objects (`field.rs:518`); a zone-lookup failure silently becomes `Less`/`Greater`. Per-call cost is unmeasured.
2. **minijinja 2.24.0 already has `Template::undeclared_variables` and `Expression::undeclared_variables`** (static template analysis needs no bump), **but not `State::get_or_insert_extension`**, which exists only in 3.0.0. Render-scoped storage on 2.24.0 is string-keyed `State::get_temp`/`set_temp`. `machinery` is behind `unstable_machinery` in both versions.
3. Cost of a 2.24.0 → 3.0.0 bump is unknown (changelog, MSRV, breaking changes unchecked).
