# Template-language intelligence

Type: grilling
Blocked by: 08
Status: resolved

## Question

Grounding: this is the sharpest constraint in the whole map. **Templates cannot
be safely executed for analysis purposes.** `TemplateEngine` owns one shared,
`debug`-enabled minijinja `Environment` (`src/template/engine.rs:71,117`);
helpers are registered as `Object`-trait globals (e.g. `ui` at
`src/template/engine/ui.rs:61-63`). The `ui.*` namespace's "No-Declaration
Format" means templates call interactive prompts *synchronously mid-render*,
blocking on a live `DialogProvider` (`src/template/engine/ui.rs:82-87`).
Attempting to render a template with no human present either hangs or
immediately errors (`DialogError::NotInteractive`). **There is no existing
static-analysis capability**: no code path extracts "which helpers/variables
does this template reference" without a full execution pass. There is no
root-template AST caching today (every render re-parses via
`env.template_from_named_str`, `src/template/engine.rs:172-175`). Minijinja's
own parser (`env.parse()`) is available but unused for this purpose today. This
was confirmed via rust-docs-mcp against the `minijinja` crate if deeper API
confirmation is needed.

Decide:

- The static-analysis approach: build a new (LSP-only, or shared-and-reusable)
  code path using minijinja's `Environment::parse`/AST inspection. It answers
  "what helper calls, variables, and includes does this template reference, and
  where (spans)" without executing `ui.*`/`query.*`/etc. side effects at all.
- Completion: helper-namespace and function-name completion (`ui.`, `file.`,
  `date.`, `query.`, `tasks.`, `schema.` then member completion). Does this need
  typed knowledge of each namespace's method signatures (a hand-maintained
  metadata table, since minijinja `Object`s do not expose a reflectable schema)?
  Can it be derived some other way?
- Hover: showing a helper function's signature/doc on hover. Same
  "hand-maintained metadata" question.
- Diagnostics: minijinja's own parse errors already preserve spans
  (`env.set_debug(true)`, `TemplateError::Render` preserves the source
  `minijinja::Error`, `src/template/error.rs:89-96`). Decide whether these get
  surfaced live (on every keystroke, via the static-parse-only path) as LSP
  diagnostics, distinct from the CLI's render-time error diagnostics.
- Whether `{% include %}`-referenced templates get "go to definition"/completion
  for template names, reusing `TemplateLoader::find`'s existing local-to-global
  resolution precedence (`src/template/loader.rs:85-99`).

## Answer

24 decisions locked across four grilling rounds (Q1 to Q6, Q7 to Q16, Q17 to
Q22, Q23 to Q24). Each round was stress-tested by a 2-to-3-subagent
adversarial/technical pass (14 source reports total, all merged into the
research file and deleted).

### Grounding supersession (needed by ticket graph MUST#5)

- **`env.parse()` becomes `machinery::parse`**: analysis uses minijinja's
  machinery API behind the `unstable_machinery` feature gate, exact-pinned
  `=2.24.0`. The free fn returns `ast::Stmt` (top node
  `Stmt::Template(Spanned<Template>)`, `parser.rs:1392-1399`), with companion
  `parse_expr`. Every node is `Spanned<T>` (a struct over a private boxed tuple,
  with `.span()`/`Deref`; no pattern-match on `(node, span)`). Offsets are UTF-8
  bytes composing with ticket 11's `BytePos` ladder.
- **Names derived, docs hand-authored** (not reflected): minijinja `Object`s
  expose no reflectable schema. Filter/test enumeration is Rank-1 derived via
  `env.state().iter_{filters,tests}()` probes, **filtered through `find`**
  (offered means find-resolvable, D5/S1), cross-checked in tests against
  hand-written docs. Use golden-span tests, an exact version pin, and ban
  `_`-match arms so a new minijinja variant breaks loudly.
- **Corrected line refs** are recorded in research §0.
- **Supersession block**: ticket 08's "20's territory" attribution for
  schema-change rebuild is superseded (the gap = an un-owned open item, owner
  candidate 31; see §3.5). **Q7's "eager = startup-only" is NOT superseded**:
  Q23(a') reinstates it with a non-fatal qualifier, and only the Q17-lazy
  knock-on claimed otherwise (withdrawn).

### Static analysis approach (Q1-Q6)

- **(a) The `unstable_machinery` gate**: full-parser reuse over a hand-rolled
  scanner. A hand-rolled full parser was rejected for grammar divergence
  (minijinja's upstream COMPATIBILITY.md). A hand-rolled *token scanner* remains
  the gate-failure fallback (the no-AST tier).
- **Placement: shared, engine-adjacent, in `src/template/analysis.rs`** (D2
  restated as a protocol-free invariant: analysis depends on minijinja only,
  with no LSP types). Concrete placement is a ticket-34 input (workspace split /
  `pub(super)` widening / gate cost; the inputs line applies to 34's Question).
- **Metadata = derived names + hand-authored docs + cross-check** (filters/tests
  included, per Rank-1).
- **Diagnostics live-debounced** (not on-save): recompute on `didChange` through
  the single-lane publisher (Q19 β), timing provisional to 25 (`map.md:31`).
- **D5 soundness invariant**: completion-offered implies `find`-resolvable,
  scope S1 (exact-name forms).
- **No persistent AST cache**: parse per recompute. The derived-analysis memo
  lives in 33's LRU tier within existing caps, with caller-side invalidation via
  `clear_templates()`; env-handle/invalidation ownership is decided in Q19.

### Completion & hover (Q11, Q12, Q21)

- **Q11/Q21: tiered hover returns `Doc(entry)` / `Identified(kind, name)` /
  `None`**. Which tiers render is 26's call. `Identified` appears only for
  kind-confirmed entities (a probe-derived kind or a table entry); unknown-root
  attributes give `None` by construction. The kind rule: a non-empty `Str`
  enumerator gives "namespace", an empty enumerator that is
  registered-in-globals gives "function", else "value". Markdown safety = an
  inline-code wrap (identifiers are ASCII + `_`).
- **Q12: a narrow fix, SURVIVES**. Drop ambiguous stems in the shared
  `list_available` via `retain(|stem| find(stem).is_ok())` **after the
  local/global merge** (naive per-dir filtering is a no-op and cross-dir
  unsound). Zero test changes + one new regression test; update doc comments
  (`loader.rs:237-246`, `service.rs:77-84`). **CLI delta: listing behavior
  changes. User sign-off is recorded here (no CLI ticket exists).**

### Diagnostics & include navigation (Q9, Q10, Q13, Q14)

- **Q9(c): the phase-1 four classes**: syntax + name-resolution + typo (probe
  with close match) + undefined-variable, **iff** Q18's span mechanism.
  Conditions: `undeclared_variables(false)` (nested=true defeats subtraction),
  subtract `env.globals()` keys, record the empty-render-context soundness
  condition (`engine.rs:195`), and record the includer-context FP as a known
  limitation. Severity proposed Information, sent to 25.
- **Q18: span mechanism**: a scope-correct span-collecting mini-tracker re-walk
  (`nested=false` only, about 100-150 lines on top of D1's existing adapter
  walk). The `undeclared_variables` name set is demoted to a gate + `#[test]`
  oracle only and dropped from production (one parse per recompute).
- **Q10: fallback range = span-precedence ladder** (owned by 22): (1) nearest
  AST-node span, (2) `Error::range()`, (3) `0..len` only as a tripwire. All
  `BytePos`, boundary-mapped; the cosmetic choice is proposed to 25.
- **Q13: buffer-authoritative content reading** whenever content reading lands
  (transitive analysis/preview), served by 14's `ContentResolver::text_of` with
  the overlay handle injected at engine construction. Phase-1 reads no include
  content today, so overlay-vs-disk is moot for 22 now.
- **Q14: extends/import/from in discovery scope**: uniform across
  runtime/codegen/loader. Dynamic targets (`{% extends layout %}`) are an honest
  false-negative, named so nobody "fixes" it with string-sniffing.
  `file.include(...)` is named as excluded (a different resolution path).

### Data source, cadence & delivery (Q7, Q8, Q15, Q16, Q19, Q20)

- **Q7: one env per analysis host** (30); the eager cost is accepted as
  startup-only (confirmed under Q23(a')).
- **Q8: no edge to 34**: conditional re-open triggers are recorded (analysis
  leaves `src/template/`; the gate must ride an LSP-only feature;
  `derived_names()` visibility under the chosen crate graph). The inputs-pending
  line applies to 34's Question.
- **Q15(c): the data-source guarantee is scoped as an API contract**:
  env-derived tier total, doc-derived tier `Option`, diagnostics exempt (owned
  rule 2), cursor-context parse-dependent. The degrade call goes to 24. A
  **third explicit state** (construction-failure/intelligence-disabled) must be
  distinguishable from empty-by-success. It is keyed off the **stored `Err`
  record**, not engine shape (`Ok(full)` / `Ok(empty-registry, Err recorded)` /
  `Err(no env)`), threaded to 24 + 26.
- **Q16: three confirmations**: (a) class+content belong to 22's, severity goes
  to 25; (b) the must-carry sentence goes into 25's inputs; (c) drift risk is
  accepted with a proximity forcing function.
- **Q19: β confirmed**: compute-on-lane + a long-lived background sleep/publish
  task holding owned payloads + a `Client` clone, find-only analysis resolution,
  single-owner `&mut` env (N=1). FIFO messages `Compute/Clear/Opened` with
  coalesce-by-URI, publish-gate on open/version/no-newer-pending, and per-URI
  reopen version reset (prevents ghost diagnostics). A single-publisher-per-URI
  invariant is proposed to 25. Amendment text: `Amends 12 §1` (cite `12:46`; §3
  refresh exclusions are unaffected), `Amends 14 §2` scope-limited to the
  template-diagnostics publication path (14's 150ms sleep stands), and 09 is a
  one-line clarification, not an amends.
- **Q20(b): overlay-aware discovery**: existence = the set union of disk entries
  and open-buffer paths. The virtual-child rule lives inside `find`'s
  per-directory two-phase check (precedence unchanged). **`find` and
  `list_available` adopt the seam together**. One code path serves two configs:
  an `Option<Arc<RwLock<…>>>` overlay handle is injected only by the LSP, while
  the CLI constructs `None` (disk-only, zero delta). `didClose` removes the
  name; untitled buffers never participate (a stated limit).

### Construction policy (Q17, with Q23(a') chosen)

- **Eager attempt at `initialize`, non-fatal**: store `Err`, degrade, record.
  The failure ladder has three rungs: **(0)** config absent/untrusted
  (`ConfigService::load` fails non-interactively) gives no Config, no loader,
  parse-only tier, env off; **(1)** schema dir resolve/load fails gives an
  empty-schema-registry fallback engine (env alive, registry empty) + a failure
  flag; **(2)** full. **State exposure is mandatory**: the failure flag
  accompanies the fallback data ("degraded, schema registry unavailable, data =
  empty"). Depth A = schema-tier-empty + flag; depth B = env-tier-off list
  (syntax/resolution/loader features survive; filter/test/global/hover/undef-var
  off).
- **The fallback needs two additive APIs**: `SchemaService::empty()` + an
  engine-level seam (no `Option` plumbing through ops).
- **Surface**: `window/showMessage` + `window/logMessage` (the spec's named
  pre-`InitializeResult` exception, framework-legal) **and** `tracing::warn!`.
  The toast carries the schema-dir path + disabled scope + "until restart".
  Optional v2 = a range-less schema-file diagnostic, owed to **25's inputs row**
  (20 is resolved).
- **`reset_engine()`**: one named API sharing initialize's full construction
  path (Config → loader → engine), re-injecting the same overlay `Arc` from
  state, called only by lane handlers. v1 = sticky-until-restart. The schema-dir
  `didChangeWatchedFiles` trigger is owed to 31 (a flagged line, not an edge).
- **Two-policy note + seam**: `ConstructionPolicy { FailClosed,
  DegradeEmptyRegistry }` at the shared constructor. The CLI passes `FailClosed`
  (three fail-closed tests stay untouched); the LSP passes `Degrade` (**zero
  existing test coverage: new tests are needed**). **CLI delta: the CLI stays
  fail-closed. No CLI ticket/owner exists, and change is out of scope: user
  sign-off is recorded here.** If the A3-split (schema-free env build +
  registration step) is adopted later, it subsumes this fallback. Record that,
  and do not build both.

### Wiring (Q22/Q24 + research §5)

- **Edges: exactly {24, 25, 26} from 22** (their Questions contain items
  unanswerable without 22). **27 gets no edge (Q24(b))**: a flagged line is
  appended verbatim to 27's Question (include/extends spans as documentLink
  candidates + overlay-aware existence + re-evaluate-the-edge clause). **Timing
  fallback: if 27 has been claimed/resolved before this lands, restate it at
  27's resolution.** 34 gets the sibling inputs-pending line (Q8). The
  32-from-22 edge is never added (the flag rides 22 to 25 to 32).
- **Edge-vs-line policy (stated once)**: edges = inputs
  answerable-today-but-blocked; flagged lines = optional/deferred inputs (27,
  31, 34).
- **Downstream owed rows** (full text in research §5): 24 (data source + staging
  - trigger + ordering + degrade line incl. disabled state), 25 (one inputs
  list: class list, source/code set, fallback-ladder rule, severities incl.
  undef-var FP rationale, cadence proposal, single-publisher/β coordination,
  SchemaLoad class, construction-failure degradation note, which is env-free
  classes only, silence-not-ranges), 26 (hover content + tiered contract +
  degrade line incl. disabled state), 12/14/09 timer amendment texts, 13
  (`Amends 13`), 33 (`Amends 33`: eager KB-scale construction on the cold-start
  path + failure-degrade axis), 20 (construction-failure surface: v1 vehicle
  named, two-SchemaService fact), 30/31/18 lines.
- **Checked clean**: 10's facade is consistent with D2; 09's
  runtime/DialogProvider is consistent with Q7/Q9; 19's hover/kind split is
  intact; map standing constraints `:22/:23/:24/:29/:31` are all respected; no
  ticket loses an edge.

### Research

Consolidated: `research/22-template-intelligence.md` (637 lines, §0 to §9):
grounding corrections, four round-decision sections with stress verdicts,
downstream wiring table, Round-4 answers. Fourteen source reports (4 grounding +
10 stress) were merged here and deleted.

### Amended by 25 (2026-10-15)

- **β generalized beyond templates (25 Q3-R)**: 22's β single-lane publisher is
  the publication path for **all** diagnostics (links, schema, query, footnotes,
  tasks), not only template diagnostics. It is adopted as-is for MVP (generation
  counters arrive with scope B, inside the diagnostics module; a separate lane
  and facade DirtySet were rejected). Workspace warm-up bypasses the β FIFO.
- **Timing (Q19 line)**: "timing provisional to 25" is resolved: T1 100ms, T2
  ~50ms (adaptive clamp `[50ms, compute, 500ms]`), T3 event-driven.
- **Citation fix (22:34)**: the debounce bullet lives at `map.md:32`, not
  `map.md:31`.
- **14 §2 wording**: 22's proposed "Amends 14 §2 scope-limited (14's 150ms sleep
  stands)" is **superseded**: 14 §2 is superseded outright. There is no
  in-handler sleep anywhere; debounce = spawn + generation counter (14 amended
  directly by 25).
- **22:72 receipt**: 25 accepts the owed inputs list with per-item pointers.
  Class list goes to Q15-R2 `DiagnosticRule` enum (`template.*` codes);
  source/code set goes to Q6-R Scheme C (`source: "traces"`, hierarchical
  codes); the fallback-ladder rule stays untouched by 25's severity calibration
  (construction-failure stays 22's degrade design); severities incl. the
  undef-var FP rationale: the four phase-1 classes are undeclared, so Q5-R
  applies, defaulting them to Information as 22 proposed (FP rationale recorded;
  taxonomy = registry); cadence proposal goes to Q3-R (live-debounced, T2 lane);
  single-publisher/β coordination goes to Q3-R (β adopted, generalized);
  SchemaLoad class goes to registry code `template.schema-load` with 22's
  proposed severity standing under Q5-R; the construction-failure degradation
  note (env-free classes only, silence-not-ranges) is accepted.

### Amended by 26 (2026-10-18)

- **Which hover tiers render (22's open handoff to 26) is resolved**: render
  `Doc` and `Identified`, and suppress only `None`. The construction-failure
  state gets an explicit degradation banner (never silently empty: failed-load
  is byte-identical to schema-less).
- **Receipt: template identifiers are non-renamable v1** (prepareRename returns
  `Ok(null)`). Template hover rides 26's negotiated-markdown renderer with
  producer-owned caps.

## Input from 27 (2026-10-22)

- Q24(b) answer from 27: template references appear on BOTH channels.
  Goto-definition over template refs ships as soon as 22's resolution logic
  exists (not span-gated); documentLink emission for template refs waits for
  22's exact-span source (no line-granular underlines); dynamic targets are
  dropped from emission; existence is gated on 22's Q20(b) overlay set. Risk
  note: Traces ships minijinja 3.0.1 while 22 designs `machinery::parse` behind
  exact-pin `=2.24.0`, a genuine dep mismatch that 22 must reconcile before
  template documentLinks can exist.
