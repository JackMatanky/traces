# What is the explicit execution context, and what ambient state is eliminated?

Type: grilling
Status: open
Blocked by: 4

## Question

Should process-ambient state (XDG/state dirs and ignored-dir env via `LazyLock`,
system timezone and `now`, cwd) be replaced by an explicit context constructed
at the composition root? If so what are its fields, who constructs it, how do
test and long-lived-host determinism work, and how is the god-object risk
avoided?

Facts: `src/dirs.rs:85-190`, `src/env_vars.rs:8-37`, test HOME hack
`src/dirs.rs:81-83`, `local_zone()` per leaf and `ZoneGuard`
(`src/temporal/mod.rs:58-255`), `Cwd`/`CwdGuard` (`src/cli/cwd.rs`, judged sound
as a single audited boundary).

Alternatives: explicit `ExecutionContext`/`Environment` passed down; split
contexts (time/zone vs filesystem roots vs env policy); keep ambient with
narrower guards. Include keep-as-is.

Inputs: `survey/config-dialog.md`, `survey/temporal.md`, `survey/cli.md`,
`research/04`, forward-consumer decision (not blocking but consulted).

Unlocks: config ownership, temporal boundary, query/template evaluation
signature, workspace host.

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). Naive frontmatter datetimes are
zone-shifted at parse time and persisted in the postcard cache, so the zone
input also affects index contents, not only evaluation.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Naive
datetimes are now civil values, so the zone no longer affects index contents;
it is needed only at evaluation. The context must supply the zone and
`today`/`now` when sort keys and filter operands are built (civil vs instant,
day literals against instants, relative and natural-language dates). No
comparison may read the clock or system zone. Decide the default zone.
