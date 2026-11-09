# How do embedded-template hosts bridge domain values and keep date formatting coherent?

Type: research
Status: resolved
Blocked by: none

## Question

The template engine (13k LoC) stringifies rich values at the minijinja boundary
(`src/template/engine/query.rs:604`). Then 2.5k lines of `engine/date.rs`
re-parse them. That file embeds a Moment.js dialect translator (`:488-623`),
ordinal suffixes, and calendar anchors. `Schema` needs a hidden `State` cache
hack to reach its service (`src/template/engine/schema.rs:82-90`). Render-scoped
index caching lives in minijinja `State`. See `survey/template.md` H1/H2/H3/H7,
`survey/schema.md` H6, `survey/temporal.md` H4.

Study these comparables:

- minijinja's `Object`/`Value::from_object`/`ObjectRepr` protocol and real
  downstream uses (read `minijinja/src/value/object.rs`)
- hosts embedding minijinja/tera (`uv`/`pixi`/`zola`-style), plus
  `cargo-generate`'s Liquid usage for contrast
- Copier and Cookiecutter extension/context models
- Obsidian Templater's `tp.date`/`tp.file` API and its Moment usage
- Jiff `fmt::strtime` and `jiff-icu`, chrono `format` vs Moment token tables
- how `typst` or `rhai` hosts pass engine handles through evaluators

Focus:

- Typed host objects vs plain data at the template boundary?
- Where should a date-format dialect live: date library or template layer?
- Is Moment compatibility a Templater-compat feature or a domain concept?
- How does a host inject services: context handle vs ambient state?
- How do errors from host callbacks preserve source spans?

Informs: template engine boundary, temporal boundary and dialects.

Deliverable: `.scratch/arch-design/research/07-template-bridge-precedent.md` (<=
300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Resolved by research deliverable:
`.scratch/arch-design/research/07-template-bridge-precedent.md`.
