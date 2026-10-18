# Research Precedent: Template Host Value Bridging, Date Formatting, and Service Injection

Research ticket: `issues/07-research-template-bridge-precedent.md`
Deliverable: `.scratch/strategic-redesign/research/07-template-bridge-precedent.md`

Investigates how embedded template engines (Minijinja, Liquid, Jinja2) and domain evaluators (Typst, Rhai, Obsidian Templater) bridge domain values, dialect formatting, service injection, and error spans.

---

## 1. What Each Comparable Actually Does

### Minijinja (Rust)

- **Object protocol [V]**: `minijinja::value::Object` (`minijinja-2.24.0/src/value/object.rs:174-301`) provides dynamic dispatch for Rust host types via `repr()` (`ObjectRepr::Plain | Map | Seq | Iterable`, `:866-898`), `get_value`, `get_value_by_str`, `call`, `call_method`, `custom_cmp`, and `render`. `ObjectRepr::Plain` formats via `render()` or `Debug` without exposing iterable/map semantics (`:875-879`).
- **Render-scoped state [V]**: `State` (`minijinja-2.24.0/src/vm/state.rs:43-77`) exposes `get_temp(name)` / `set_temp(name, val)` (`:357-367`) backed by `Mutex<BTreeMap<FastString, Value>>`. In Minijinja 3.0 (`src/vm/state.rs:496-503`), `state.get_extension<T>()` introduces type-keyed `Any + Send` extensions.
- **Source span preservation [V]**: `minijinja-2.24.0/src/vm/mod.rs:1140-1158` (`process_err`): When a host callback (`call_method`, `call`, function, filter) returns `Err(err)`, the VM checks `if err.line().is_none()`. If unset, it automatically stamps the template filename and source span from the bytecode instruction program counter (`pc`). Host callbacks never need to capture template spans manually.
- **Minijinja Contrib datetime [V]**: `minijinja-contrib-3.0.0/src/filters/datetime.rs:166-190` provides `datetimeformat`, `timeformat`, `dateformat`. All use POSIX strftime format strings (e.g., `short` -> `"%Y-%m-%d %H:%M"`). Contrib contains zero Moment.js or custom token translation logic.

### Cargo-Generate (Rust Liquid Host)

- **Context model [V]**: `cargo-generate/src/template.rs:58-82` (`create_liquid_object`): Context is a heap-allocated dictionary `LiquidObjectResource = Arc<Mutex<RefCell<liquid_core::Object>>>`.
- **Eager pre-collection [V]**: `cargo-generate/src/lib.rs:281-365`: Strict two-phase lifecycle. Phase 1 interactively prompts for every variable defined in config, inserting raw scalar values (`Value::Scalar`). Phase 2 renders files with no host queries.
- **Filter limitations [V]**: `cargo-generate/src/template_filters.rs:144-146`: Liquid filters cannot cleanly fail template rendering; errors fall back to leaving the substitution text untouched. Variables lack method dispatch.

### Copier (Python Jinja2 Host)

- **Data context & sandbox [V]**: `copier/copier/_jinja_ext.py:31-48` (`SandboxedEnvironment`): Restricts access to unsafe host model attributes. Filesystem loader strictly confines path inclusion to template root (`:76-80`).
- **Flat answers map [V]**: `copier/copier/_user_data.py:111-120`: Interactive questions evaluate via a DAG into a flat dictionary `Mapping[str, Any]` before render.
- **Standardized formatting [V]**: `copier/copier/_user_data.py:43-46`: Deprecated ad-hoc date helpers (`_now()`) with a warning directing users to Jinja's standard strftime filter (`{{ '%Y-%m-%d %H:%M:%S' | strftime }}`).

### Obsidian Templater (TypeScript Plugin)

- **Per-render context injection [V]**: `Templater/src/core/functions/FunctionsGenerator.ts:37-63`: `generate_object(config: RunningConfig)` generates the `tp` object per template run, wrapping internal modules (`date`, `file`, `system`, `web`).
- **Moment.js delegation [V]**: `Templater/src/core/functions/internal_functions/date/InternalModuleDate.ts:1-47`: `tp.date.now`, `tomorrow`, `yesterday`, `weekday` delegate 100% to Obsidian's runtime `moment` (`import { moment } from "obsidian"`). Templater performs no pattern parsing or token translation.
- **Service binding [V]**: `Templater/src/core/functions/internal_functions/file/InternalModuleFile.ts:19-48`: File functions (`tp.file.include`, `exists`, `creation_date`) close over `this.plugin.app` (vault API) and `RunningConfig` (active note).

### Jiff & Chrono (Rust Datetime Libraries)

- **Strftime specifiers [V]**: `jiff-0.2.38/src/fmt/strtime/mod.rs:1-80`: Jiff implements POSIX-style `strftime`/`strptime` (`%Y`, `%m`, `%d`). Chrono does the same in `chrono::format::strftime`.
- **No Moment dialect [V]**: Neither Jiff nor Chrono supports Moment tokens (`YYYY`, `DD`, `dddd`, `Do`, bracket literals). `jiff-0.2.38/src/fmt/strtime/mod.rs:255-275`: `jiff-icu` provides Unicode ICU4X localization, not Moment dialect parsing.

### Typst (Rust Document Evaluator)

- **Explicit Engine handle [V]**: `typst/crates/typst-library/src/engine.rs:18-36`, `typst-eval/src/vm.rs:18-42`: Typst passes an explicit `Engine<'a>` handle containing `world: Tracked<'a, dyn World + 'a>` and `introspector: Protected<Tracked<'a, dyn Introspector + 'a>>`. Evaluators have zero ambient thread-local state.
- **Exact spans [V]**: AST nodes carry `Span`; errors return `SourceResult<T>` with precise span diagnostics.

### Rhai (Rust Embedded Scripting)

- **Context handle [V]**: `rhai/src/func/native.rs:75-86, 219-259`: Native Rust functions take `context: NativeCallContext<'a>`, exposing `context.engine()`, `context.call_position()` (script callsite position), and `context.tag()` (custom host state stored in `GlobalRuntimeState`).

---

## 2. Invariants and Invariant Ownership

| Invariant | Owner in Comparables | Traces Current Owner | Traces Defect / Smells |
|---|---|---|---|
| **Domain Value Integrity** | Value Object (`minijinja::Object`, Rhai `CustomType`) [V] | Degraded to `String` at boundary (`query.rs:604-607`) [V] | `DurationValue`, `DatePoint`, `Link` lose identity; date filters repeatedly re-parse strings in loops (`date.rs:473`) [V]. |
| **Service Lifecycle** | Explicit Context (`Typst::Engine`, `Rhai::NativeCallContext`) [V] | Ambient Minijinja `State::set_temp` (`schema.rs:82-90`) [V] | `Schema` lacks backpointer to `SchemaService`; relies on side-effect seeding in `schema.get()`; panics/errors if accessed out-of-order [V]. |
| **Render Cache Scope** | Template host runtime / engine extensions [V] | Ambient Minijinja `State::set_temp` (`cache.rs:21-48`) [V] | Dynamic string-keyed temp storage with downcasting wrappers rather than cohesive host service handles [V]. |
| **Date Dialect Grammar** | Platform date library (Obsidian `moment`) [V] | Hand-rolled translator in template engine (`date.rs:488-623`) [V] | 2,486 lines of temporal logic in template engine; `src/temporal/` has no Moment translator, duplicating temporal concepts [V]. |
| **Error Span Attribution** | VM instruction dispatcher (`minijinja::process_err`) [V] | VM instruction dispatcher [V] | Works as intended; callbacks returning naked `Error::new` automatically receive callsite spans from Minijinja [V]. |

---

## 3. Tradeoffs and Costs

- **`minijinja::value::Object` vs String Degradation**:
  - `Object` cost: `Arc<dyn Object>` allocation and dynamic virtual dispatch for method calls.
  - String degradation cost: Heap `String` allocation for every cell in `query.rs:604`, plus full string parsing (`parse_date_point`, regex, timezone lookup) inside every loop iteration in `date.rs:473`. In benchmarks (`benches/template_render.rs`), string parsing dominates render time [H].
- **Eager Pre-Collection (Cargo-Generate, Copier) vs Lazy Dynamic Render (Traces, Templater)**:
  - Pre-collection cost: Requires static declaration (`cargo-generate.toml`, `copier.yml`); cannot execute template-driven queries (`traces.query.from(...)`) or dynamic note inclusions during prompt collection.
  - Lazy render cost: Requires injecting live host capabilities (`WorkspaceIndex`, `SchemaService`, `DialogHandler`) into the template execution runtime.
- **Explicit Context Handles vs Ambient `State` Scratchpad**:
  - Ambient `State` scratchpad cost: Minijinja's `set_temp`/`get_temp` relies on mutex-locked `BTreeMap` lookups, string keys, and runtime type downcasting. Misses compile-time lifecycle guarantees.
  - Bound context handles cost: Requires passing an explicit handle (e.g. `SchemaHandle<'a>`) or bundling shared references (`Arc<SchemaHierarchy>`) inside the model object.

---

## 4. Transferable Lessons for Traces

1. **Implement `minijinja::value::Object` on Domain Primitives [H]**:
   Wrapping `DatePoint`, `DurationValue`, and `Link` in custom `Object` implementations (`ObjectRepr::Plain` or `Map`) allows them to cross the Minijinja boundary without stringification (`query.rs:604`). Methods like `.format(...)`, `.add(...)`, and attributes like `.target` can be invoked directly via `Object::call_method` or `get_value_by_str`.
2. **Minijinja VM Automatically Preserves Source Spans [V]**:
   As verified in `minijinja/src/vm/mod.rs:1140-1158`, host callbacks returning `minijinja::Error::new(...)` have template filenames and byte spans automatically attached by `process_err`. Traces does not need to thread AST spans through callback signatures.
3. **Replace `State::set_temp` with Bound Domain Handles [H]**:
   Typst's `Engine` and Rhai's `NativeCallContext` show that values needing engine services should either hold bound handles (e.g. `SchemaHandle<'a>` holding `&'a SchemaService` and `Arc<Schema>`) or embed shared read-only hierarchy data (`Arc<SchemaHierarchy>`). This completely removes the fragile `SCHEMA_SERVICE_CACHE_KEY` hack in `schema.rs:82-90`.
4. **Recognize Moment.js as a Templater Compat Dialect, Not a Temporal Core Concept [H]**:
   In Templater, Moment is native because Obsidian runs in Electron. In Rust, standard datetime libraries (Jiff, Chrono) strictly use strftime. Moment formatting in Traces is solely an Obsidian Templater compatibility feature.

---

## 5. Lessons That Do NOT Transfer (and Why)

1. **Cargo-Generate's Eager Pre-Collection Model [V]**:
   Cargo-Generate resolves all variables upfront from a static manifest before executing Liquid. Traces committed to ADR-0001 (No-Declaration lazy execution), allowing templates to dynamically issue queries, inspect schemas, and prompt users mid-render. Cargo-Generate's architecture cannot support dynamic note queries.
2. **Templater's Direct Platform Delegation [V]**:
   Templater can afford a 47-line date module (`InternalModuleDate.ts`) because it delegates directly to `obsidian.moment`. Traces is a pure Rust CLI; Jiff and Chrono have no built-in Moment parser. Traces must compile or translate Moment tokens if it supports them.
3. **Copier's Python Reflection Sandboxing [V]**:
   Copier must sandbox attribute access (`SandboxedEnvironment`) to prevent arbitrary Python code execution via Jinja reflection. Minijinja in Rust is memory-safe and access-confined by construction: only methods explicitly exposed via `Object` or environment globals are reachable.

---

## 6. Impact on Traces Architectural Hypotheses

### `survey/template.md` [H1] (Typed Minijinja Value Protocol vs String Degradation)

- **CONFIRMED [V]**. Upstream `minijinja::value::Object` (`src/value/object.rs:174`) directly supports `ObjectRepr::Plain` and `call_method`. Implementing `Object` for `DatePoint`, `DurationValue`, and `Link` eliminates string allocation in `query.rs:604` and eliminates string re-parsing in `date.rs:473`.

### `survey/template.md` [H2] & `survey/temporal.md` [H4] (Consolidate Moment.js into `src/temporal/`)

- **CHALLENGED [H]**. The comparables disprove the assumption that Moment.js is a general temporal concept that belongs in `src/temporal/`. Upstream date libraries (Jiff, Chrono) explicitly omit Moment tokens. Moving the 2,486 lines of `date.rs` into `src/temporal/` would pollute the pure temporal subsystem with PKM-template compatibility code.
- **Alternative Formulation [H]**: The pure temporal engine (`src/temporal/`) should only know about ISO-8601, Jiff strftime, and calendar math. Moment translation belongs in a dedicated template compatibility module (`template::compat::moment` or `compat::templater`), or a narrowly scoped parser in `temporal::format` only if schema field format definitions also require Moment syntax.

### `survey/template.md` [H3] (Zero-Copy Query Builder & Pipeline Pushdown)

- **CONFIRMED [V]**. Minijinja's `Object` can represent lazy builder objects (`QueryBuilderObject`). Chained calls (`.where()`, `.sort()`) can mutate or wrap an internal query plan without materializing or cloning `Vec<QueryRow>` until terminal rendering (`.table()`, `.list()`, or `enumerate()`).

### `survey/schema.md` [H6] (Detached `Schema` Forces Minijinja `State` Cache Hack)

- **CONFIRMED [V]**. Typst and Rhai precedent confirms that `Schema` should be returned as a bound `SchemaHandle` (or embed an `Arc<SchemaHierarchy>`). Stashing `Arc<SchemaService>` in Minijinja's `State::set_temp` is an anti-pattern caused by anemic domain models.

### `survey/template.md` [H7] (Lazy Schema Loading in TemplateEngine)

- **CONFIRMED [V]**. Decoupling `SchemaService` loading from `TemplateEngine::new` isolates template rendering from schema registry validation errors when schemas are not queried.
