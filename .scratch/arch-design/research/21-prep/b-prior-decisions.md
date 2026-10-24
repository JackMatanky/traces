# Prior decisions and dependent questions for the execution-context grilling

[V] The binding decisions are narrower than “put everything in an `ExecutionContext` ”: evaluation needs an explicit zone and `now`
/`today`, while use cases take explicit inputs rather than a held context object. [H] Configuration, workspace lifetime and temporal
sampling meet at the host boundary; deciding one omnibus object here would silently decide later tickets. [P] Grill the independent
choices separately, including retaining narrowly controlled ambient state. Tags: [V] verified in the cited artifact, [H] inference, [P]
proposal. This is research, not a decision.

## 1. Constraints already decided

1. **Compatibility and visibility.** [V] The sole current consumer is the author; no published Rust crate API is owed compatibility.
Vault parsing, fresh-index query equivalence, trust-before-parse and CLI output/exit behavior are Frozen; configuration and diagnostic
formats are Migratable with a behavior-change table; index layout and Rust `pub` surface are Free. Prefer narrow visibility and justify
outward-facing `pub` ([compatibility envelope](../../issues/16-grilling-compatibility-envelope.md):20-53). [H] Replacing directory
discovery cannot move the trust gate after config reading or silently change stderr/exit treatment. A giant public context is not
required by compatibility.
2. **Real consumers, not imagined ones.** [V] The LSP is a committed next consumer; MCP is likely, but speculative general
multi-workspace or agent write-back infrastructure is not a requirement. Synchronous shared semantics, source-aware diagnostics, and
protocol-specific hosts are the declared direction
([forward consumers](../../issues/17-grilling-forward-consumers-and-design-pressure.md):33-101). [H] An invocation-only global snapshot
must explain an LSP's later requests and configuration changes, but designing full MCP session machinery now exceeds the commitment.
3. **Core's composition rule.** [V] CLI, LSP and MCP are thin hosts of `core`; core's app layer composes domain modules over foundation
types. Domain modules expose data and pure functions. Use cases are **functions with explicit inputs, not a held context object**; do
not invent one-implementor traits between modules
([schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):66-72). [H] A host-owned snapshot or small
function parameters can satisfy this; an app service holding `ExecutionContext` cannot be assumed compatible. Ticket 21's own
one-context alternative must be tested against this newer binding principle
([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):7-22).
4. **Zone is evaluation input, not cache identity.** [V] Naive datetimes are civil; explicit-offset datetimes are instants.
Civil/instant comparisons use an explicit zone when sort keys and filter operands are built; neither ordering nor comparison reads
ambient time/zone, and the cache holds no machine state. Inline fields may now be datetimes
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):94-100,169-188). [V] Ticket 19 explicitly hands off
zone, `today` /`now`, and the default zone here (:190-201). [H] Capture `now` once for one evaluation and derive `today` from that
instant in the chosen zone, lest the pair disagree across midnight; exact host scope remains open.
5. **Other temporal consumers.** [V] Query's relative/natural date grammar and template date functions must use the same evaluation
inputs; `DateConfig` coordination goes to temporal and config ownership
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):190-214;
[temporal boundary and dialects](../../issues/27-temporal-boundary-and-dialects.md):30-66). [H] A process-start `today` makes a
long-lived server stale after midnight; conversely, sampling per row creates intra-query inconsistency. The grammar itself is not
decided here.
6. **Schema and trust are independent of the clock.** [V] Schema checking is advisory, on demand, pure and span-free over resolved
fields; it never types at parse, gates indexing, or enters the index/fingerprint. Hosts choose `ignore | warn | reject` by violation
code ([schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):49-95,242-253). [V] A class binding may
depend on a note's path, and checks of proposed edits receive before/after path **and** content (:134-149,176-182). [H] The execution
context must not erase path identity or smuggle schema validation into index construction.
7. **Host-owned write authority.** [V] A portable agent guard is detect-and-feedback; prevention exists only where Traces owns the write
or external native writes are denied. The proposed MCP guard belongs in its host; protocol Roots cannot be trusted as its session
authority, and there are no reliable protocol sessions
([schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):176-223,242-253). [H] A process-global
“current root” or “current trust” cannot safely stand in for an explicitly selected workspace and per-call destination. None of this
commits ticket 21 to implementing the MCP host.
8. **Existing ADR constraints.** [V] Config trust is checked before parsing/executing untrusted workspace config; state/tracking and
trust stores are separate, and their state location already has an injectable temporary-directory seam
([trust and tracking ADR](../../../../docs/adr/0002-config-trust-and-tracking.md):18-34,56-71,105-116). [V] Interactive functions are
lazy and use `DialogProvider`; headless `Preset` is supported
([interactive functions ADR](../../../../docs/adr/0001-minijinja-interactive-functions.md):28-42,76-90). CLI presentation and exit codes
stay in CLI ([error presentation ADR](../../../../docs/adr/0004-cli-error-presentation.md):35-45,63-67). [H] A temporal snapshot should
not itself grant trust, own terminal I/O, or swallow typed domain errors.
9. **Cwd already has an audited seam.** [V] The question describes `Cwd` /`CwdGuard` as sound at a single audited CLI boundary, not a
proven defect to delete ([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):15-22;
[CLI survey](../../survey/cli.md)). [V] The research precedent proposes removing `CwdGuard`, but that is a research recommendation, not
a ruling ([execution-context precedent](../04-execution-context-precedent.md):99-103). [H] A design can preserve this localized guard
while making the workspace root an explicit input; weigh the actual failure mechanism before replacing it.
10. **Cost evidence is bounded.** [V] The precedent distinguishes ripgrep's boundary snapshots, jj's small parsing clock, Cargo's broad
global context, and uv's global-state test locks; Jiff's system-zone discovery has an internal RwLock/TTL cache rather than a fresh OS
call on every leaf ([execution-context precedent](../04-execution-context-precedent.md):12-141). [V] The performance research measures
indexing/query paths, not execution-context startup, Jiff-lock overhead or zone-snapshot cost
([performance evidence](../11-performance-evidence.md):13-24,37-70,204-245). [H] Neither “per-leaf syscall bottleneck” nor “explicit
context is free” follows from measurements.
11. **Path capability remains separate.** [V] Research locates root confinement at I/O boundaries and portable relative index paths,
while treating a cap-std conversion as unjustified without evidence for this local CLI
([capability/path precedent](../09-capability-path-precedent.md):88-145). [H] Capturing cwd/root in an immutable context is not a
filesystem capability or a guarantee against symlink races; [path and capability model](../../issues/29-path-and-capability-model.md)
retains that security decision.

## 2. Decisions expected from this ticket, per downstream ticket

12. **[Config ownership and lifecycle](../../issues/22-config-ownership-and-lifecycle.md):7-39.** [V] Needs a contract for which
discovered XDG/config/state paths and ignored-dir variables are captured, by whom and when, and how workspace config/trust loading
receives them. It separately decides loader versus parser, config lifetime and fingerprint cleanup. [H] If 21 punts, 22 must choose its
own ambient reads, test-injection seam, global-vs-workspace lifetime, and whether zone is a config field. [P] Agree on the environment
snapshot and zone-configuration handoff with 22 without deciding the complete loader API here.
13. **[Index freshness and invalidation](../../issues/25-index-freshness-and-invalidation.md):7-87.** [V] Needs the root/index identity
and boundaries of configuration drift; existing `IndexFingerprint` covers ten of fifteen settings, schema files are excluded, and
temporal civil values remove zone from index identity
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):94-100;
[schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):49-64). [H] If 21 punts, 25 must choose
whether changing process zone/cwd/state home forces cache invalidation; the first should not, while workspace-root identity cannot
silently change. Keep cache freshness separate from request-time time freshness.
14. **[Query core and plan](../../issues/26-query-core-and-plan.md):38-79.** [V] Needs a stable evaluation input for constructing
date-dependent filter operands, adaptive literals and sort keys once, without per-row clock or zone reads. [H] If 21 punts, its planner
must introduce and name the zone/clock argument itself; divergence between query and template follows. Query remains synchronous under
the forward-consumer ruling. [P] Decide time sampling scope at least at query-evaluation granularity; leave grammar and planning to 26.
15. **[Temporal boundary and dialects](../../issues/27-temporal-boundary-and-dialects.md):30-66.** [V] Explicitly awaits 21 and 22 for
zone source, default zone and `DateConfig` ownership while it owns natural-language grammar, formatting/dialect and civil/instant type
shape. [H] If 21 punts, 27 must decide default zone and fallback policy to make civil/instant evaluation work; if 22 punts too, no owner
can state whether a config change affects the next evaluation. [P] Distinguish system-zone default from UTC, fixed override and
error/no-zone cases before arguing type names.
16. **[Template engine boundary](../../issues/28-template-engine-boundary.md):39-55.** [V] Needs `now` /`today` shared with query and
the common temporal relation/grammar, while template prompting stays lazy and post-render schema checking is app-layer work
([schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):168-174). [H] If 21 punts, template leaves
may keep reading the clock/zone separately even if query is deterministic. A dry-run lacks an output path for schema path bindings
(:249-254); do not invent one inside a context.
17. **[Path and capability model](../../issues/29-path-and-capability-model.md):7-35.** [V] Needs an unambiguous root plus relative path
for confinement and class binding; stdin and dry-run may lack output paths. [H] If 21 punts, 29 will choose how to turn process cwd into
an authorized root; if 21 overreaches, a passive context might be mislabeled a capability. The path check must still happen at each I/O
boundary ([capability/path precedent](../09-capability-path-precedent.md):88-145).
18. **[Workspace host and composition](../../issues/30-workspace-host-and-composition.md):7-77.** [V] Blocked by 21/22/24/25; owns
CLI-first assembly, service lifetimes, dialogs and host/session design. [H] If 21 punts, 30 must define environment/clock capture and
constructor arguments itself; if 21 chooses a long-lived omnibus host object, it preempts 30 and violates the explicit-input use-case
principle. [P] 21 should define immutable inputs and freshness rules, and let 30 choose a host's storage/lifecycle and 22 the config
loader.
19. **[Error and diagnostic architecture](../../issues/31-error-and-diagnostic-architecture.md):23-50.** [V] Needs typed operational
failures versus advisories and host-owned rendering/exit mapping; schema violations have stable codes and host policy. [H] If 21 punts,
31 cannot know whether unavailable zone, invalid configured zone, or directory-discovery failure is returned, warned on, or silently
defaulted. [P] Decide the semantics of discovering/sampling each input here; let 31 assign error types/codes and presentation.
20. **[Crate topology and public surface](../../issues/32-crate-topology-and-public-surface.md):7-76.** [V] Needs actual cross-crate
input types and minimum public exports; the CLI-first split and core-app layering constrain this, but a many-crate design is not settled
([map](../../map.md):19-45; [schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):66-72). [H] If 21
punts, 32 must guess whether host environment and evaluation snapshot cross the core boundary. [P] Design the minimal data shape, then
let 32 place and expose it; do not widen all leaf APIs to `pub` preemptively.
21. **[Live buffer and persisted index consistency](../../issues/34-live-buffer-and-persisted-index-consistency.md):7-37.** [V] Needs
checks/query operations to take a buffer snapshot without persisting it. [H] A clock captured only at process initialization gives
date-sensitive editor diagnostics stale results after midnight even with unchanged content; a per-request evaluation snapshot plus host
rescheduling can avoid that. If 21 punts, 34/LSP will choose sampling and invalidation rules independently of CLI.
22. **Additional neighboring ownership.** [V] [Index read model](../../issues/24-index-read-model-and-query-execution.md) gets
civil-datetime cache encoding, not system-zone cache identity
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):180-208);
[note document model](../../issues/23-note-document-model.md) gets inline datetime shape;
[schema internals](../../issues/35-schema-internals-simplification.md) gets path/tag bindings, not host clock. [H] These are reasons to
pass concrete data rather than couple parsing to a universal environment.

## 3. Couplings and sequencing risks

23. **The 21–22–30 triangle.** [V] Ticket 21 explicitly unlocks config ownership and workspace host
([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):24-28), while
[config ownership and lifecycle](../../issues/22-config-ownership-and-lifecycle.md):7-39 and
[workspace host and composition](../../issues/30-workspace-host-and-composition.md):7-77 each depend on its answer. [H] Decide *where
inputs enter* in 21, *how configuration is loaded/reloaded* in 22, and *what hosts retain and when they reconstruct* in 30. These must
be reviewed as one boundary, not silently decided three times.
24. **Time is not process environment alone.** [V] Jiff `TimeZone::system` caches internally; jj pins parse time, whereas Cargo's
GlobalContext gathers many process facts ([execution-context precedent](../04-execution-context-precedent.md):12-141). [H] Three
lifetimes can differ: invocation or server startup for stable directory discovery, workspace/config revision for chosen zone, and
request or operation for `now`. A single immutable process-start snapshot cannot meet all three without recreation; recreating
everything each request needlessly entangles trust/config state with time.
25. **The 21–27–26–28 temporal seam.** [V] Ticket 19 delegates zone/default here, dialect to 27, binder to 26, template behavior to 28
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):190-214). [H] An unspecified choice between system
local, UTC and config zone changes civil/instant results. `today` from a different zone or tick than `now` creates a semantic split even
though each function has explicit arguments. [P] State sampling unit, pairing rule, default-zone rule and change visibility; postpone
syntax.
26. **Trust and mutable homes.** [V] The trust ADR stores trust and tracking separately under a discoverable state dir and requires
trust before parsing workspace config
([trust and tracking ADR](../../../../docs/adr/0002-config-trust-and-tracking.md):18-34,56-71,105-116). [H] If a long-lived host
re-reads environment variables halfway through a trust/config transaction, the grant may be read from one home and tracking written to
another. Conversely a forever-frozen home ignores deliberate reconfiguration. [P] Specify an atomic discovery scope, then 22/30 choose
reload triggers and trust revalidation; do not make directory strings an authorization proof.
27. **LSP's prior host design is pressure, not a new mandate.** [V]
[LSP analysis host](../../../md-pkm-lsp/issues/10-analysis-host-and-index-integration.md) chooses an immutable `Arc<WorkspaceIndex>`
snapshot in a thin analysis-host facade;
[LSP concurrency](../../../md-pkm-lsp/issues/12-concurrency-and-cancellation-model.md):20-46,61-67 chooses sequential request dispatch,
stale-result discard and a later single-lane diagnostics publisher. [H] Request-local time and workspace-revision config can be
introduced without general concurrency locks in that initial design. Avoid assuming those LSP choices dictate CLI types or every future
host.
28. **Date rollover is already recognized.** [V]
[LSP daily notes and date references](../../../md-pkm-lsp/issues/18-daily-notes-and-date-references.md):20-54 proposes one
local-midnight timer that reevaluates date-dependent diagnostics;
[LSP diagnostics](../../../md-pkm-lsp/issues/25-diagnostics-architecture.md):19-47 separates publication and per-code behavior. [H]
Re-evaluation is host scheduling, not a comparison reaching out for a clock. Its “local” zone needs the new default-zone decision. [P]
Treat timer scheduling and date-dependent invalidation as 30/34/LSP concerns, not as a generic background task in core.
29. **LSP config and workspace status must not be mistaken for decisions.** [V]
[LSP config integration](../../../md-pkm-lsp/issues/31-config-integration.md):1-23,
[multi-root versus project root](../../../md-pkm-lsp/issues/30-multi-root-vs-project-root.md):1-18, and
[crate packaging](../../../md-pkm-lsp/issues/34-crate-and-binary-packaging.md):6-25 have open questions rather than binding Answers. [H]
Earlier prep's shorthand “34 resolved LSP tickets” must not confer decided status on these. [V]
[LSP schema intelligence](../../../md-pkm-lsp/issues/20-schema-and-fileclass-intelligence.md):14-57 also precedes and is superseded on
span-free checker and host policy by
[schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):49-72,176-253.
30. **Analogies have limits.** [V] IWE's `liwe` pure domain, `diwe` application engine and thin CLI/LSP/MCP adapters are a pertinent
real precedent, but its MCP guard semantics and host setup are its own
([IWE core and hosts](../12-iwe-core-and-hosts-architecture.md):9-58,85-131,273-297). [V] Error research favors typed errors and
CLI-owned miette/exit presentation, not a forced exact crate count
([error-topology precedent](../08-errors-topology-precedent.md):108-164). [H] Neither authorizes transplanting an omnibus Traces
context.

## 4. Contradictions and stale premises to surface explicitly

31. **Most consequential reversal.** [V] The [value model](../../issues/18-value-model.md):445-573 and ticket 21's old input
([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):30-34) say naive datetimes are zone-shifted
into the persisted cache. The later [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):94-100,180-208
and the explicit correction in ticket 21 :36-43 replace this with civil cached values and evaluation-only zone. [H] Any recommendation
to put timezone in index fingerprint or reindex on timezone change relies on the stale premise.
32. **Old temporal specification versus newer decisions.** [V] The historical
[temporal-core specification](../../../temporal-core/spec.md):13-25,40-45,62-80,98-105 describes local naive input converted to UTC and
system-clock `today`; its :122-137 asks for isolated time tests and rejects unnecessary Clock/TimeZone trait abstractions. The
[temporal divergence register](../../../temporal-core/divergence-register.md):140-160 records `ZoneGuard` in place of mutating process
`TZ` and failure on unavailable zone rather than an unnoticed UTC fallback. [H] Treat the old spec as implementation history, not the
governing zone representation after ticket 19; do not casually invent a UTC fallback without reconciling the failure rule.
33. **Survey claim corrected by research.** [V] The [temporal survey](../../survey/temporal.md) describes leaf-level `local_zone()` and
frames it as repeated OS zone lookup. The [execution-context precedent](../04-execution-context-precedent.md):124-130 shows Jiff
system-zone caching and its TTL. [H] Removing per-leaf calls may still improve determinism and repeated lock work, but an OS
syscall-per-row claim is unsound and no benchmark measured the saving ([performance evidence](../11-performance-evidence.md):204-245).
34. **Config fingerprint is not dead.** [V] The [config/dialog survey](../../survey/config-dialog.md) marks `ConfigDigests` dead, but
[load-bearing claim verification](../01-verify-load-bearing-claims.md):11-28 and [reverification](../../reverification.md):18-25
identify a live index fingerprint over 10/15 settings and a distinct unused `Config::hash`. [H] Do not propose deleting all config
digests while relocating discovery; separate the live fingerprint from dead residue. The
[proposed index-refresh ADR](../../../../docs/adr/0009-index-refresh.md):31-61,74-91 is not an accepted source of new context rules and
has implementation drift.
35. **Prior prep proposed since-rejected semantics.** [V] [19 prior decisions](../19-prep/d-prior-decisions.md) and
[19 brief](../19-prep/brief.md):8-34 include old zone-persisted assumptions; the brief itself corrects a supposed public Noyalib
`ScalarStyle` and other scout claims (:36-52). [V] [20 prior decisions](../20-prep/d-prior-decisions.md):23-27,58-75 treats ticket 19's
proposed schema string override as binding; ticket 20 explicitly withdraws it
([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):218-223). [H] Do not add an override, schema-aware
parse, or cache-zone coupling as a context feature.
36. **Accepted ADR versus newer schema Answer.** [V] The accepted
[schema ADR](../../../../docs/adr/0006-file-class-schemas.md):58-88,94-111 calls `required` /`multi` inert until MCP and describes
schema as a value provider; [schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):49-96,398-414 now
requires advisory checking and leaves the ADR/glossary update for specification. [H] Citations to the ADR for “schema has no checks” are
stale; the schema's validation still does not require clock injection.
37. **Glossaries and surveys describe the present, not the selected future.** [V] The
[cross-cutting survey](../../survey/cross-cutting.md) argues for many crates and depicts the old `FileClassExpander` coupling; ticket
20's functions-with-explicit-inputs layering and the [map](../../map.md):19-45 supersede that as a prescription. [V] The
[config/dialog survey](../../survey/config-dialog.md) reports no default templates directory, while the
[config glossary](../../../../src/config/GLOSSARY.md):74-80 describes a `.traces/templates` default; verify the actual config behavior
before a design treats either as user-visible fact. [H] This conflict belongs to 22, not an excuse to quietly choose one here.
38. **Current-tree drift caveat.** [V] Comparing baseline `207cc3fb` to current HEAD names changes in `src/env_vars.rs`,
`src/temporal/mod.rs`, `src/temporal/zone.rs`, config discovery/service, query service, template engine/date/query, index refresh and
file tracker, among others (`git diff --name-only 207cc3fb..HEAD -- src docs tests`, read for this note). [V] Ticket 21 itself describes
the old HOME hack and guards ([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):15-18). [H]
Survey inventories are starting points only: current code inventory must check the thread-scoped `ZoneGuard` and Minijinja 3 migration
rather than treating pre-baseline call sites or cross-test zone leaks as still present.
39. **Other effort drift is bounded.** [V] Sibling directories under `.scratch/` include `md-pkm-lsp`, `arch-design`, `temporal-core`,
`integration-e2e-suite`, `template-service`, `task-system`, `query-expansion`, `prompt-service`, `metadata-schemas`, `index-query`,
`config-service` and `note-parse` (directory listing reviewed). The old
[config-service specification](../../../config-service/spec.md):9-11,38-46 embeds obsolete trust/home and prompt-provider expectations
relative to [trust and tracking ADR](../../../../docs/adr/0002-config-trust-and-tracking.md):18-34 and
[interactive functions ADR](../../../../docs/adr/0001-minijinja-interactive-functions.md):28-42; the
[temporal-core specification](../../../temporal-core/spec.md):13-25 is superseded on naive values by ticket 19. [H] Those historical
sibling designs are stress-test material, not requirements to recreate them.

## 5. Vocabulary already claimed

40. **Root and workspace.** [V] [root glossary](../../../../src/GLOSSARY.md):8-20 calls the *Project Root* the vault boundary and warns
against conflating it with working directory; [index glossary](../../../../src/index/GLOSSARY.md):10-25 uses *Workspace Index* for
indexed data, not an LSP host/session. [H] Name a host's chosen root separately from process cwd and distinguish workspace-owned
services from `WorkspaceIndex`.
41. **Config and trust.** [V] [config glossary](../../../../src/config/GLOSSARY.md):10-21,25-80 claims *Configuration Scope*, *Config
Locator*, *TrustStore*, *TrackerStore*, *Template Directory* and *Config Fingerprint*. A `Config Locator` already names a discovery
concern involving more than one home. [H] A proposed `Environment` must not collapse trust state and a path locator, nor imply the
fingerprint contains zone.
42. **Time, render and UI.** [V] [temporal glossary](../../../../src/temporal/GLOSSARY.md):19-26,44-48,62-66 defines *Date-Time*, *Zone*
and *DateTimeInput*, but its process-local promotion descriptions predate ticket 19. [V]
[template glossary](../../../../src/template/GLOSSARY.md):34-38 uses render “environment” for template variable binding, and
[dialog glossary](../../../../src/dialog/GLOSSARY.md):10-14 owns *Dialog Provider*. [H] `HostEnvironment`, `WorkspaceContext` and
`TemporalContext` in [execution-context precedent](../04-execution-context-precedent.md):91-103 are research labels, not glossary terms.
Avoid ambiguous unqualified `Environment` and do not hide dialog inside a clock/zone value.
43. **Execution and core.** [V] [query glossary](../../../../src/query/GLOSSARY.md):30-48 already uses *Query Builder*, *Execution Plan*
and *Sort Order*; [CLI glossary](../../../../src/cli/GLOSSARY.md):56-66 uses *Command Outcome* and *User Abort*.
[glossary map](../../../../GLOSSARY-MAP.md):9-67 maps existing module language but still depicts CLI root composition and
`FileClassExpander`; ticket 20 marks Core/App Layer/consumer vocabulary for later revision
([schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):398-414). [H] A `QueryExecutionContext` name
might be mistaken for planner/execution mode, whereas a narrow time/zone evaluation input describes the need; exact name belongs to
design, not current glossary authority.

## 6. Visible user steering and grilling lessons

44. **Adversarial evaluation and staged split.** [V] The [map](../../map.md):19-45 requires a serious keep-as-is case, tight public
surface and a CLI-first workspace split candidate; it does not authorize immediate many-crate partitioning. [V]
[error topology research](../08-errors-topology-precedent.md):108-164 cautions against a gratuitous testing backdoor and exact
crate-count prescription. [H] Offer a sound retained-ambient option alongside split snapshots, naming the actual test/long-lived-host
costs; do not sell line-count alone.
45. **Do not infer use solely from callers.** [V] [20 brief](../20-prep/brief.md):17-39 records a user correction: schema is staged for
the committed LSP, and a caller-count/pruning pitch missed prior LSP design and diagnostic-span coupling. [H] Global HOME hacks and
ambient zone calls need evidence of harmful behavior, not a blanket “globals bad” rationale; equally, `#[cfg(test)]` isolation does not
establish correctness for a server.
46. **Decompose recommendations.** [V] The [schema answer](../../issues/20-schema-note-typing-and-validation.md):66-72,176-253 separates
rule checking, host policy, trust/write gates and protocol-specific machinery; its comments explicitly correct claims about harness
hooks and unverified agent capabilities (:216-240). [V] The [20 brief](../20-prep/brief.md):11-39 says an earlier one-sided
recommendation was withdrawn after user pushback. [H] The right grilling shape is separate decisions about captured roots/env,
evaluation time/zone, guard/test seams and host lifetime—not one bundled approval of a `Context` type, new traits, crate split and MCP
server.
47. **Tactical ledger is not a context mandate.** [V] The [findings ledger](../../findings-ledger.md):7-26 records non-TTY silent
selection, template-dir asymmetry, EPIPE, residual `Config::hash`, log-only refresh healing and CLI-owned schema-loading `has_classes`.
[H] Dialog selection and EPIPE/error isolation can remain host/diagnostic tasks; only directory asymmetry, config hashing and
composition reach this context boundary. No item alone justifies global context replacement.

## What this means for the grilling questions

48. [P] First test **keep ambient with narrow guards**: preserve `CwdGuard`, injectable state stores and thread-scoped `ZoneGuard`;
demand a precise way to give query/template one consistent `now` /zone and to keep LSP requests fresh without accidental process-wide
reads. Ask which remaining ambient reads are actually unsafe rather than rejecting this option by label.
49. [P] Compare that with a **split-input design**: host discovers dirs/env/root once per explicit host or workspace revision, and
samples a small `now` +zone evaluation input once per operation; core use-case functions take only the inputs they consume. Challenge
reload semantics, clock/zone error handling and calls where time is not needed. A single omnibus context is a third alternative: require
evidence that it avoids parameter scattering without violating ticket 20's explicit-input functions or conflating three lifetimes.
50. [P] Ask separately: what is the default zone for civil/instant evaluation, who resolves configured zone and unavailable-system-zone
errors, how are `today` and `now` paired across midnight, and when do LSP host time-dependent results reschedule? Which tests can inject
*both* zone and instant without mutating process global state? Explicitly distinguish index state (no zone fingerprint), workspace
config state and evaluation state.
51. [P] Finally pin handoffs: 21 supplies the time/zone and process-input contract; 22 owns trusted config discovery/reload; 27 owns
temporal grammar/format; 26/28 consume the shared evaluation input; 29 owns confinement; 30 owns host construction and lifetime; 31 owns
errors/presentation; 32 owns exported placement; 34 owns buffer/time-dependent refresh. Insist on one behavior-change table if defaults
or CLI output change, not an implicit redesign of those tickets.
