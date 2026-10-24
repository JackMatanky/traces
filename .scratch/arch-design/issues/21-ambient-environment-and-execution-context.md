# What is the explicit execution context, and what ambient state is eliminated?

Type: grilling
Status: resolved
Blocked by: 4

## Question

Should an explicit context at the composition root replace process-ambient state?
This state includes XDG/state directories, ignored-directory environment values
held by `LazyLock`, the system zone, `now` and cwd.
If so, which inputs does the context contain? Who constructs it?
How do tests and long-lived hosts get deterministic behavior?
How does the design avoid a god object?

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

See [ticket 18's Answer](./18-value-model.md). Naive frontmatter datetimes are
zone-shifted at parse time and persisted in the postcard cache, so the zone
input also affects index contents, not only evaluation.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Naive
datetimes are now civil values, so the zone no longer affects index contents; it
is needed only at evaluation. The context must supply the zone and `today`/`now`
when sort keys and filter operands are built (civil vs instant, day literals
against instants, relative and natural-language dates). No comparison may read
the clock or system zone. Decide the default zone.

## Pre-decision review

The [decision ledger](../research/21-prep/decisions-ledger.md) records the
original L1-L18 hypotheses, not the final decisions. The independent
[temporal](../research/21-prep/r5-temporal-adversarial.md) and
[host](../research/21-prep/r5-host-adversarial.md) challenges, the
[synthesis](../research/21-prep/r5-synthesis.md), and the subsequent grilling
exposed contradictions in L6, L10, L13 and the same-root storage assumption. The
compiled lab's date comparison promotes to midnight; its benchmark does not
validate period membership or production speedups. This Answer supersedes the
ledger wherever they differ.

Prior constraints matter: [ticket 16](./16-grilling-compatibility-envelope.md)
freezes vault parsing, trust-before-parse, exit codes and successful stdout, but
permits recorded query-name/behavior migrations. [Ticket
17](./17-grilling-forward-consumers-and-design-pressure.md) calls for
LSP/MCP-capable core ownership without a speculative multi-root registry.
[Ticket 19](./19-coercion-and-comparison-policy.md) originally used an inclusive
period bound, a civil-midnight field rule and "filters never error"; only the
first and the blanket error wording change here. [Ticket
20](./20-schema-note-typing-and-validation.md) leaves schemas out of persisted
note facts. The accepted
[ADR-0003](../../../docs/adr/0003-index-based-dialog-selection.md) requires
selection by index *and* documents an unattended first/empty fallback; only that
fallback is revised, pending reconciliation before implementation.

## Answer

### Decision: acquire policy at the edge, bind it at the right lifetime

- The adapter gets OS-valued inputs once when it opens each workspace. These
  inputs are state/config homes, discovery ceilings, ignored directory names and
  the explicit start path. Preserve platform path encodings and path precedence.
  The config/path decisions must choose an explicit base for relative overrides
  or reject them. Do not use a Unicode-only map for production paths. Only the
  CLI may use cwd as the default start path. Do not require that path for each
  operation on an already-bound workspace.
- Core must support concurrent construction of roots with different policies,
  configs and zones. It must not change process globals to do this. A workspace
  owns its resolved policy, config and derived services for one generation. An
  adapter may share one immutable policy across roots. A future host can
  identify an instance by canonical root and policy/generation identity. Do not
  build a registry or multi-root coordinator now. Adding one must not require
  changes to domain APIs.
- One coherent index generation must supply the rows of an initial query. The
  root-local index store has authority independent of the host instance. Do not
  promise concurrent index writers with different policies for the same
  canonical root. Include the ignore policy and any persisted filename-period
  parse policy in index freshness. Do not automatically use the full host-policy
  identity as the index fingerprint. Zone changes do not change stored civil
  note values or filename periods. Resolve timestamp-derived civil dates during
  evaluation. Never persist those dates with a zone-free fingerprint. Tickets
  24, 25, 29 and 30 own cross-process coordination, canonical path identity and
  store locking. A registry key cannot isolate two sessions that share
  `.traces/index.redb`.
- Check the hash of local config before parsing it, as ticket 16 requires. At
  request admission, a held workspace rechecks local trust and content. Reject
  changed or revoked local config instead of using the old parsed value. An
  admitted request keeps its immutable generation and may finish. A watcher only
  signals the need for possible revalidation. Global config has a different
  freshness path; it does not use the local trust gate. Immediate cancellation
  and privileged-write commit checks remain separate host-policy questions.
- A query service owns a source of evaluation frames, not one frozen frame. For
  each operation, sample one `now` and one week rule. Get the system-local zone
  only when needed. Pin that zone resolution, including a failure, for the
  operation. One standalone query, one template render with nested queries, or
  one LSP request shares one frame. Resolve a reusable expression again for the
  next operation. A lazy result set keeps its creator's resolved plan and frame
  after the operation. A render-scoped state extension alone cannot supply a
  frame after that state is gone.
- Resolve relative expressions and constant operands once. The evaluator must
  not read the clock or ambient zone. A resolved plan may hold one shared
  immutable calendar capability. Do not clone a zone for each node or row.
  Adaptive literal typing for each row can still need a pinned zone. Do not
  assume that query construction can resolve every temporal operation.
  `run_with` may accept a render/request frame. Ordinary `run` samples the
  service's frame source. Keep the current evaluator. Do not adopt a relational
  engine without a demonstrated workload need.
- Report a missing required zone or temporal presentation context as a typed
  resolution, materialization or render failure. Do not drop the row, return
  `Null`, or make the cell blank. Do not get a zone for work solely on stored
  civil values. Conversion of a timestamp to a civil date needs a zone. A lazy
  `QuerySet` caches both successes and failures. Give it fallible terminal
  access. Its `len`/`get`/`iter`, formatting and trait implementations must not
  hide execution errors. Complete fallible work before successful output, or
  stage it without an unnecessary second in-memory copy. A host may explicitly
  select UTC. Never use UTC as an implicit fallback. A data-type mismatch
  remains a nonmatch or `Null` under ticket 19.

### Decision: civil periods and filesystem dates

- Make `CalendarPeriod` a validated value with a civil start date and a unit.
  Rename its current boundary-selector role `CalendarUnit`. Add `Day` and
  `Quarter` to its `Week`, `Month` and `Year` units. Keep `DatePoint` as a point
  with written precision and date arithmetic. Precision retained after a shift
  does not make a canonical period start. Do not add a second period-value type
  or allocate on the heap for this value.
- For period-versus-instant filters, compare the instant's civil date in the
  pinned zone with the period's civil dates. A skipped local day matches no
  instants. An existing day does not fail when the next day is skipped. For
  period-versus-civil-Date or civil-DateTime filters, use the point's civil date
  without a zone. Keep whole-span matching for a period literal against a point.
  Keep ticket 19's civil-midnight rule for ordinary Date field-versus-field
  comparisons. Do not use Rust `Eq` or `Ord` for membership.
- Two periods are filter-equal only if their normalized starts and units match.
  Compare `(start, unit)` keys for period-versus-period relational filters. Use
  a fixed unit tie-break. Overlap and containment are not equality; add explicit
  functions only when a query needs them. `!=` complements filter equality. Use
  a separate deterministic total sort key for mixed temporal values. Do not use
  period membership for sorting. Exclude the next day's start from period
  literal upper bounds on instants. This removes ticket 19's adjacent-day
  overlap.
- `file.date` holds a filename-derived `CalendarPeriod`. Return `Null` if the
  filename has no recognized prefix. Never read creation time or fall back to
  `file.cdate` for `file.date`. Remove the old `file.day` name and its
  filename-or-creation hybrid. Do not silently assign the old name a new
  meaning.
- `file.cdate`/`cday` and `file.ctime`/`created_at` use only filesystem creation
  time. Return `Null` when it is unavailable. Never substitute modification time
  for creation time. Creation timestamps are best-effort operating-system data.
- `file.mdate`/`mtime` use modification time. The timestamp fields remain
  instants. During evaluation, project `cdate` and `mdate` timestamps into civil
  dates with the pinned zone. If the needed zone cannot be resolved, return a
  typed error. Never persist these zone-derived dates as zone-independent facts.
- Preserve existing day/month filename prefix recognition. Add validated ISO
  week forms with Monday as the start and a four-day week one. Add quarter forms
  with clear prefix boundaries. Enable a bare year prefix only through
  `DateConfig`. Prefer the longest valid shape. Do not reinterpret a malformed
  longer date as a shorter date. Use the new forms only for filename extraction.
  Do not change the frozen shared note/frontmatter/inline parser under ticket
  16. A configurable first weekday also needs a week-one numbering rule. Let the
  index-freshness decision choose whether to persist derived periods or reparse
  filenames. Do not store a zone for that reason.

### Decision: unattended prompts and ambient enforcement

- The adapter selects interactive, unattended or answered-preset dialog
  behavior. The template and core receive a `DialogProvider` without detecting a
  TTY. Unattended prompts may use authored defaults. Return a typed prompt error
  for a missing default, a choice without an explicit answer or an exhausted
  preset. Do not substitute empty text, false, the first choice or an empty
  multi-selection. Apply this rule to CLI non-TTY, `--no-input` and new hosts.
  Treat an explicitly selected `--force` as a separate overwrite policy. Do not
  change a provider's interactivity when its queued answers run out.
- Revise only ADR-0003's non-TTY fallback. Keep its accepted index-by-position
  selection contract. Do not change the ADR's historical status here. Ticket 30
  must reconcile its documented fallback before implementation. Preserve
  authored defaults such as `init` paths.
- Get cwd, environment values, clock time and zone only at identified adapter
  boundaries. Enforce the core restriction with concrete Clippy
  `disallowed-methods` paths. Do not use a nonfunctional `std::env::var*`
  wildcard. Treat source scans as inventory, not proof. Replace test-only
  `HOME`/zone guards after injection gives deterministic fixtures. A CLI-only
  cwd guard can remain while in-process adapter tests change cwd. Prefer
  subprocess checks for real environment behavior. Treat random template UUIDs
  as a separate, explicit entropy capability, not an implicit part of the
  workspace policy.

### Alternatives considered and why not chosen

- Keep ambient reads plus narrower test guards: simpler today, but a held
  workspace can change meaning when process cwd, zone or env changes.
- One universal `ExecutionContext` passed into every operation, or an `Arc`/zone
  cloned at every row: broad coupling or avoidable hot-path work. A plan-owned
  immutable capability covers only the temporal operations that need it.
- Eagerly acquire the system zone for all queries, drop zone-failing rows, or
  treat a missing zone as UTC: either civil-only queries fail without need or
  results change silently. Fallible lazy materialization is the coherent
  boundary.
- Interpret every Date as an instant at midnight, or make ordinary Date equality
  mean interval membership: the former breaks coarse literals; the latter
  contradicts field-versus-field semantics and cannot supply an equivalence
  relation. Civil periods are distinct values.
- Keep the current `CalendarPeriod` as only a span selector and add another
  `Period` value, or overload `DatePoint` as an anchored period: the former
  duplicates the chosen vocabulary; the latter confuses point arithmetic with a
  normalized period invariant.
- Automatically pick first/empty/false answers to unattended prompts: compatible
  with existing scripts but can change rendered content without user choice.
  Rejecting *all* unattended prompts would also discard safe, explicitly
  authored defaults.
- Build a multi-root registry, per-policy index namespace, or async core now:
  ticket 17 requires readiness for LSP/MCP, not speculative machinery.
  Multi-root construction is supported by ownership, with same-root index
  contention explicitly limited.

### Invariant owners and removable code

The following components own the invariants:

- Adapter: ambient input acquisition, CLI defaults and prompt mode.
- Config: discovery, trust-before-parse and request-admission revalidation.
- Workspace host: generation publication and index-writer admission.
- Index: stored index-relevant fingerprint and consistent read generation.
- Temporal: validated periods, week rules and explicit-zone civil conversion.
- Value semantics: filter relations separate from deterministic sort.
- Query: resolution, pinned frames and fallible cached execution.
- Template: render-wide frame and error propagation.

After the replacements land, remove ambient leaf reads, `FileMeta::derive_day`'s
creation fallback, creation-to-mtime fallbacks, `file.day`, silent prompt
fallbacks, per-row zone lookups and test-only injection hacks. Do not add
compatibility shims for the retired query field.

### Deliberate behavior changes and outstanding implementation decisions

Record in the redesign specification's change table: `file.day` to filename-only
`file.date`, including the newly recognized week and quarter prefixes and any
opted-in bare year; absent date and absent creation timestamp now yield `Null`;
`file.cdate`/`file.ctime` no longer fall back to mtime; coarse period upper
bounds exclude the next day's start; unattended missing answers fail; required
zone/render failure fails the operation; query result access may become
fallible. Preserve the frozen vault-format parser, exit codes, stdout/stderr
split and successful stdout shapes from ticket 16. The specification must
migrate affected CLI/template queries and asserted diagnostics rather than
relabel these as compatibility-neutral.

Still open for downstream tickets: exact host-input path precedence and relative
override behavior; storage-lock and cross-process coordination (no claim that
redb automatically provides the needed policy invariant); the period sort unit
tie-break and compact encoding; week formats beyond ISO and the configured zone
source; explicit temporal type and template carrier shapes; whether privileged
writes recheck trust at commit; and when a long-lived host refreshes diagnostics
after midnight. None is silently implemented by this ticket.

### Handoffs

- [Coercion and comparison](./19-coercion-and-comparison-policy.md): exclusive
  period upper bound, period/point and period/period relations, typed
  environmental failures rather than the blanket no-error wording.
- [Config ownership](./22-config-ownership-and-lifecycle.md): typed OS snapshot,
  trust/config source lifetimes, relative overrides and request-admission
  verification.
- [Index read model](./24-index-read-model-and-query-execution.md) and
  [freshness](./25-index-freshness-and-invalidation.md): consistent query
  generation, policy and filename-parser fingerprint inputs, cross-process
  coordination, creation timestamp nullability and index encoding.
- [Query core](./26-query-core-and-plan.md): resolve pass, pinned plan and
  fallible memoized materialization for all terminal access.
- [Temporal boundary](./27-temporal-boundary-and-dialects.md): period and unit,
  ISO/custom weeks, skipped days, DateConfig and filename recognition.
- [Template boundary](./28-template-engine-boundary.md): render frame and
  captured escaping sets, fallible formatting, explicit dialog responses.
- [Path identity](./29-path-and-capability-model.md) and [workspace
  composition](./30-workspace-host-and-composition.md): canonical root identity,
  multi-root-ready per-workspace policy, one coherent same-root index-writer
  generation, trust admission, dialog policy.
- [Error architecture](./31-error-and-diagnostic-architecture.md): typed
  execution and prompt errors translated without changing frozen CLI exit codes
  or successful output; [crate
  topology](./32-crate-topology-and-public-surface.md): adapter-only ambient
  calls and a narrow outward core interface.
- [Final reconciliation](./33-reconcile-decisions-and-audit-coverage.md): record
  each migratable behavior and reconcile ADR-0003 before cutover.

## TODOs

At specification time, update the temporal, query, dialog, index, config, CLI
and core glossaries once code reflects the chosen vocabulary. They currently
describe running code and must not claim this redesign has already landed; see
[ticket 19's glossary TODOs](./19-coercion-and-comparison-policy.md#todos).
Reconcile the accepted ADR-0003 fallback wording at implementation time without
altering its selection-by-position ruling.
