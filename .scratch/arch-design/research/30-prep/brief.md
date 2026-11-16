# Preparation brief: temporal decisions

## Purpose

Prepare evidence for [Where do temporal, format dialects and calendar helpers belong?](../../issues/30-temporal-boundary-and-dialects.md).
Do not resolve the ticket. Do not implement the redesign.
The ticket remains claimed for preparation and the later human discussion.

## Baseline

- Source revision: `971bdd5729d9d5f51a1bd6fed46bab6947df936a`.
- CodeGraph reports an up-to-date index.
- The worktree already has changes in six architecture tickets.
- Preserve those changes. Only the integration owner changes the temporal ticket.
- Read the map and the local tracker rules before interpreting ticket status.
- The map ends at architectural decisions for a specification.

## Question groups and research routes

| Questions | Concepts | Evidence routes |
| --- | --- | --- |
| 1 to 5 | Format dialects, compatibility, incomplete formats | Moment, strftime, schema checks, owner templates |
| 6 to 8 | Calendar operations, filename periods, week years | Jiff, ISO week rules, Unicode week data, existing parser |
| 9 to 12 | Parser roles and dependency costs | CodeGraph callers, local crate documentation, accepted input shapes |
| 13 to 18 | Closed grammar, relative values, membership syntax | Candidate parser source, Dataview, Templater, nldates, query grammar |
| 19 to 25 | Civil values, instants, periods, order, duration, links | Accepted decisions, Jiff, temporal designs, path contracts |
| 26 to 32 | Config policy, zones, week options | Config ownership, Jiff zone discovery, strict parsing |
| 33 to 34 | Request frames, explicit inputs, lazy failure | Execution-context decision, query and template callers |
| 35 to 36 | Range bounds, residual predicates, notices | Query-core decision, store keys, counterexamples |

## Accepted constraints to check at their source

This list guides research. It does not replace the accepted Answers.

- Preserve frozen vault-format parsing and temporal capabilities.
- Treat target decisions and current implementation as different evidence.
- Keep naive datetimes civil. Explicit offsets identify instants.
- Keep period membership separate from equality, grouping and total order.
- Use the civil date for membership of a zoned instant. Do not need a next-day midnight.
- Keep one pinned clock and week rule per operation.
- Get the zone only when needed. Retain a failed lookup in that frame.
- Do not use UTC as an implicit fallback.
- Keep `DateConfig` in config. Temporal receives a temporal-owned policy.
- Keep filename-period recognition separate from frozen note parsing.
- Derive filename periods during evaluation, not in the persisted note.
- Keep schemas out of the index fingerprint.
- Keep unproved range mappings Unsupported. Always run the residual predicate.
- Keep the 80-byte runtime value limit as a design constraint, not a measured result for a new design.
- Do not treat the truncated schema date-check sentence as an accepted rule.

## Parallel work and file ownership

- Ten read-only scouts inspect the ten Rust-design lenses. They return findings only.
- The prior-decision researcher owns `b-prior-decisions.md`.
- The digest researcher owns `c-digest-lessons.md`.
- The crate researcher owns `d-crate-evidence.md`.
- The external-source researcher owns `e-external-designs.md`.
- The integration owner writes the code synthesis, design comparison and grilling guide.
- Only the integration owner changes this brief or the temporal ticket.

## Evidence contract

Use these tags:

- [V] Source inspection or an observed result. State which one.
- [H] A hypothesis that still needs evidence.
- [P] A proposal for human approval.

Rules for evidence:

- Cite the primary source for each material claim.
- Use source symbols and paths for code citations. Add exact locations in scout findings.
- State the crate version and feature configuration for crate claims.
- Use rust-docs tools only with project-local `.rust-docs/` sources.
- Do not claim that a cached version matches a candidate without checking it.
- Keep prototype facts separate from production behavior.
- Mark unmeasured binary size and runtime cost explicitly.

## Research rules

Rules for research prose:

- Write all research prose in ASD-STE100.
- Keep instructions to 20 words. Keep descriptions to 25 words.
- Use short active sentences. Preserve caveats, numbers, identifiers and source links.
- Use the same term for the same concept.
- Do not change earlier Answers, current-code glossaries or production files.
- Do not run builds, tests, benchmarks, formatters, linters or diagnostic scripts in an agent.
- The integration owner runs the final checks.
- Do not claim a design is optimal from line count, dependency count or unmeasured cost.
- Compare retain, consolidation, adapters and removal where each applies.
- Give each candidate a counterexample and a condition that rejects it.

## Integration requirements

Map every numbered question to evidence, alternatives and a proposed answer.
Identify inherited rules, genuine choices and evidence blockers separately.
Name each invariant owner and each downstream contract.
Record conflicts without silently replacing an accepted decision.
Keep decision evidence separate from implementation gates.
The preparation does not supply the human side of the grilling.
