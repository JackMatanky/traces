# Traces: strategic architectural redesign — mission brief

Verbatim-in-spirit copy of the user's brief. Every session on this effort orients to it.

## Objective

Conduct a comprehensive, adversarial, research-driven architectural investigation of Traces to find
opportunities for fundamental design improvement across the whole codebase.

Maximise architectural depth: cohesive, composable components that encapsulate substantial
complexity behind simple interfaces, enforce well-defined invariants, and minimise unnecessary
coupling and coordination. Apply Ousterhout's strategic programming: long-term reduction of
complexity, change amplification, cognitive load, and hidden dependencies over tactical refactors.

Treat the existing architecture as an implementation to investigate, not a design to preserve.
Seek fundamental simplification, not redistributed complexity. Do not equate design quality with
smaller files, fewer lines, more types, or more abstraction layers. A decomposition is valuable only
when its benefit outweighs the conceptual/coordination/implementation complexity it adds; do not
preserve large components merely because they were historically written together.

## Investigation lenses

- Recursive deepening: find coherent components hidden in larger implementations, examine their
  responsibilities/invariants, reassess relations to surrounding components.
- Invariant ownership: classify invariants (consistency, lifecycle, state transitions); decide which
  need cohesive ownership and which boundaries minimise coordination while preserving correctness.
- Decomposition and consolidation: extract where separation improves cohesion; consolidate shallow
  abstractions, fragmented responsibilities, duplicated domain knowledge, indirection. Ask whether
  boundaries reflect domain concepts or accidental implementation choices.
- Domain modeling: structs, enums, newtypes, traits, typestate, ownership models; make invariants
  explicit and invalid states hard to represent.
- Architecture and algorithms: subsystem boundaries, dependency direction, data flow, state,
  persistence, caching, indexing, query execution, parsing, errors, algorithms, data structures.
- Strategic leverage: foundational improvements that remove recurring complexity, localise future
  change, enable further deepening. Information hiding over abstraction for its own sake.

## Comparative research

Study actual implementations (source, invariants, representations, algorithms, boundaries,
tradeoffs), not just published descriptions. Bidirectional: Traces findings pick the comparables;
comparables expose weaknesses/alternatives in Traces. Illustrative (not mandatory) comparables:

- Values & validation: SQLite, DuckDB, Polars, serde_json, jsonschema, Obsidian Dataview, Metadata Menu.
- Configuration & composition: Cargo, Ruff, jj, gitoxide, ripgrep, Jiff, Templater.
- Indexing, caching, freshness: rustc incremental, Cargo fingerprints, Git, Mercurial, rumdl,
  Dataview, Tasks, advanced-metadata-cache.
- Query & template architecture: DataFusion, Polars, jaq, cargo-generate, Copier, Cookiecutter, Templater.
- Errors & component topology: rustc, Ruff, Cargo, uv, gitoxide, rumdl.

Discover further comparables independently. Extract transferable principles; evaluate against
Traces' actual requirements, constraints, and performance. Use rust-docs-mcp for crate research
(fall back to other sources only after two failures for the crate). Repository digests are
complementary to upstream research; neither has automatic precedence.

## Investigation standards

Thorough, critical, independent, adversarial. Ground in implementation evidence and first-principles
reasoning. Docs, comments, ADRs, and design intent are fallible context. Distinguish **verified
observations**, **architectural hypotheses**, and **proposed solutions**. Investigate root causes
before turning smells into refactors. For significant candidates: (1) establish responsibilities,
invariants, underlying complexity; (2) alternatives including retaining the current design;
(3) relevant external implementations; (4) try to disprove the preferred solution; (5) name the
decision that remains unresolved. Judge by invariant ownership, abstraction depth, cohesion,
information hiding, coupling, cognitive complexity, change localisation, correctness, performance,
long-term flexibility, using realistic change scenarios. Validate consequential performance claims
with evidence. Importance is not implementation size or number of affected files.

## Wayfinder execution

Planning only: do not implement the redesign. Decision tickets, research tickets for consequential
external unknowns, preserved fog where questions cannot yet be stated. Prioritise foundational
decisions with large downstream impact. Independent investigations stay separate; genuine
cross-subsystem dependencies stay linked. Before completion every major subsystem and significant
cross-subsystem interaction must have received meaningful scrutiny; distinguish areas needing
redesign from boundaries that withstand investigation; reconcile decisions codebase-wide (competing
invariants, inconsistent boundaries, coupling, duplicated responsibilities, missed deepening).

## Destination

A coherent, evidence-backed set of architectural decisions sufficient to produce a comprehensive
redesign specification for Traces, establishing: boundaries and component responsibilities;
invariant ownership and enforcement; components to decompose/consolidate/redesign/introduce/
eliminate; improved domain models, representations, abstractions, interfaces; algorithmic,
data-structure, performance improvements; transferable comparative lessons; foundational
investments that reduce long-term complexity; dependencies, tradeoffs, implementation constraints.

Preserve externally observable capabilities and correctness. Hand off to specification once the
significant architectural decisions are resolved.
