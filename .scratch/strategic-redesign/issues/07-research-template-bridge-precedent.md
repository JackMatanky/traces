# How do embedded-template hosts bridge domain values and keep date formatting coherent?

Type: research
Status: resolved
Blocked by: none

## Question

The template engine (13k LoC) stringifies rich values at the minijinja boundary (`src/template/engine/query.rs:604`), then re-parses them in 2.5k lines of `engine/date.rs` that embed a Moment.js dialect translator (`:488-623`), ordinal suffixes, and calendar anchors; `Schema` needs a hidden `State` cache hack to reach its service (`src/template/engine/schema.rs:82-90`); render-scoped index caching lives in minijinja `State`. See `survey/template.md` H1/H2/H3/H7, `survey/schema.md` H6, `survey/temporal.md` H4.

Study: minijinja's `Object`/`Value::from_object`/`ObjectRepr` protocol and real downstream uses (read `minijinja/src/value/object.rs`; projects such as `cargo-generate`'s Liquid usage for contrast, `uv`/`pixi`/`zola`-style hosts if they embed minijinja/tera); Copier and Cookiecutter extension/context models; Obsidian Templater's `tp.date`/`tp.file` API and its Moment usage; Jiff `fmt::strtime` and `jiff-icu`, chrono `format` vs Moment token tables; how `typst` or `rhai` hosts pass engine handles through evaluators.

Focus: typed host objects vs plain data at the template boundary; where a date-format dialect should live (date library vs template layer) and whether Moment compatibility is a Templater-compat feature or a domain concept; how a host injects services (context handle vs ambient state); how errors from host callbacks preserve source spans.

Informs: template engine boundary, temporal boundary and dialects.

Deliverable: `.scratch/strategic-redesign/research/07-template-bridge-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Resolved by research deliverable: `.scratch/strategic-redesign/research/07-template-bridge-precedent.md`.
