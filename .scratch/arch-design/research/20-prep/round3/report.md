# Ticket 16 round 3: synthesis of scouts A-E, FG

Mode: Model (research only; no baseline, trial loop or patch was run). Scout
files are in this directory (`A-` to `FG-`; copied from the untracked
`.design/2026-10-21-model-schema-validation/`). Claims tagged [V] were re-read
by the orchestrator, [S] come from a scout and were not re-read, [P] proposals,
[H] hypotheses.

Status: input to [ticket 16's Answer](../../../issues/20-schema-note-typing-and-validation.md),
which supersedes this file where they differ: `traces check` takes `--before`
and `--after` (not a git baseline), the delta key is `(field, code,
hash(value))` without the element index, `step` is checked, severities belong to
ticket 27 (not scout E), and the author's vault is one example, not evidence.

## 1. What changed since brief revision 2

| Prior position | Now | Basis |
|---|---|---|
| Checker in a consumer module above schema and note | Withdrawn | Violates the map's own "invariant owner" rule: what a valid value for a field is belongs to the schema |
| Optional `LinkOracle` trait for tier-2 file checks | Withdrawn | One-implementor `dyn` trait, the same smell as `FileClassExpander` |
| `file_class_expander.rs` is a sound seam (survey) | Rejected | See section 3 |
| "LSP binds the agent" | LSP is feedback, not a gate | Section 5 |
| Date-shaped text needs an override or quote convention | Narrower than thought | Section 6 |
| Violations carry default severity (scout E) | Not adopted | Ticket 27 owns severity |
| Move `NoteFieldValue` into `src/field.rs` (scout A) | Open | Ticket 14 left its home to ticket 28 |

## 2. Layering that survives a CLI-first workspace split [P]

```text
consumers        cli, lsp, mcp (each a thin host: protocol, rendering,
                 host policy; cli is one consumer of core, not its top)
core             app layer: use cases (check_note, resolve classes, run
                 query, render template); composes the modules below
                 domain modules: note (parse, classify), schema (defs,
                 resolution, rule), query, index, template
                 foundation: Link, temporal, FieldKey/FieldName, runtime
                 value type
```

- First split is only `cli`; core stays one library crate. Module placement
  must still be crate-split-proof, because LSP and MCP hosts become further
  consumers of core's app layer (user steer: cli is one consumer of core).
- Rule: domain modules expose data and pure functions. The app layer composes
  them. No trait exists solely to let one module call another.

## 3. `FileClassExpander` verdict [V unless marked]

- Defined in `src/query/grammar/source.rs:353`, one production impl, in a
  crate-root file (`src/file_class_expander.rs`, 87 lines).
- `QueryService` holds `Option<Arc<dyn FileClassExpander>>`; `expand` mutates a
  `&mut ClassExpansionMode` out-parameter [S: B reports 7 smells].
- Orphan rule [V, reproduced in a throwaway three-crate workspace: E0117 for
  `impl query::Trait for schema::Type` in a third crate; a newtype wrapper
  compiles]: if `query` and `schema` become crates, this impl can live in
  neither a third crate nor the app layer. This is a concrete way the pattern
  fails the planned workspace direction, though not the CLI-first split.
- [V] B's fallback claim, with a correction: with no expander attached, store
  queries fall back to exact match but in-memory filters match nothing, yet
  production always attaches an expander when classes are used, so it is a
  latent hazard, not a current bug.
- Replacement [P]: `schema` produces a plain `ClassHierarchy` value (data) that
  the app layer hands to `QueryService`. No trait, no dyn, no out-param. The
  schema builder already computes `children`/`descendants` (`src/schema/
  builder.rs:346` [S]).
- Not a recognised pattern in the sense of a named idiom [H]; the legitimate
  relative is the composition root, which wires concrete types and does not
  define one-implementor traits.

## 4. Where the checker lives [P]

Recommended: the rule predicate lives in `schema`, on a resolved class set.

```text
schema.resolve(&[class names]) -> ResolvedFields      (immutable, cached, Arc)
ResolvedFields::check(&FieldMap) -> Vec<Violation>    (pure, span-free)
```

- Invariant owner: schema owns "valid value for this declaration"; note owns
  classification. Both use a runtime value type from a lower layer.
- Cost: `schema` gains a dependency on the runtime value type, so that type
  (`NoteFieldValue`: Null, Bool, Int, Float, String, Date, DateTime, Duration,
  Link, List, Object) must sit below both. Its dependencies are `Link`,
  temporal values and `IndexMap`, all already root-level [H, not traced].
  Ticket 14 rejected one enum shared with schema literals but left the home of
  the runtime value open (module placement by ticket 28), so this is a
  placement decision, not a reversal. Schema literals stay a separate type.
- Tier-2 `file` checks (folders, ext, class of the link target) need the
  index. The checker exposes the file-field constraints as data; the app layer
  performs the existence and class lookup. No oracle trait.
- Class resolution (reading the class field, unknown class) and the delta
  rule and class-change detection are app-layer use-case code.

Rejected: schema-to-note and note-to-schema dependencies (note must parse
without schemas, ADR-0009); a generic `ValueView` trait (needs a glue impl or
an inverted dependency; precedent `jsonschema`'s `Json` trait exists for FFI
only [S]); compiled generic validator (flat scalar schemas); a merged vault
model module (broad blast radius).

## 5. Agent guardrail [S: C, E]

- LSP cannot gate: `didChange` is a notification; `willSaveWaitUntil` returns
  edits and cannot abort the save; diagnostics are advisory. Agents tend to
  ignore advisory warnings [S]. A delta rule cannot live in the LSP without
  hiding legacy errors from the human.
- Real gates: a blocking pre-write hook in the harness, or MCP tools that
  return `isError: true` before touching disk. Asking the user about a class
  change maps to MCP elicitation or a hook `ask` decision [S].
- Layered proposal [P]: (1) schema constraints given to the agent up front
  (tool `inputSchema` projection and a schema resource); (2) a blocking check
  that calls the same use case as the LSP; (3) advisory LSP diagnostics;
  (4) pre-commit/CI safety net. Risk: an agent with raw shell access can bypass
  tool-level hooks.
- The blocking check needs one callable form. Either `traces check <files>
  --baseline <rev> --format json` (CLI crate, first split) or an MCP tool.
  Both call one app-layer `check_note(before, after)`.
- Delta key [P]: `(field, code, hash(value))`, not element index, which shifts
  when a list item is removed.

## 6. Date-shaped text (ticket 15 handoff) [S: D, partly V]

- Quoted numbers and versions already stay text: `"2001"` and `"1.10"` parse
  as strings; leading-zero values (`01234`, ISBN-10 with a leading zero) are
  strings under the YAML 1.2 core config [V: `src/note/metadata.rs:319`].
- Only date, datetime and duration shaped strings are promoted regardless of
  quotes. A text view rendered from the typed value is lossless for canonical
  ISO dates; durations (`90m` vs `1h30m`) may normalise [H].
- TOML-like single versus double quote convention: [S] Noyalib 0.0.53 and
  0.0.57 expose no per-scalar quote style outside the CST (+59% parse time);
  and Obsidian's property editor rewrites quote style on save (forum bug
  #65851 [S]). Not adopted.
- Recommendation [P]: no override; `input` accepts any scalar; text view from
  the typed value; a `string()` cast only if a real use appears (ticket 15).

## 7. Violation model [S: E, with one disagreement]

- `Violation { field, element, kind, expected, found }`, stable codes, no span,
  no severity. Bounded expected: count, truncated flag, top-k closest via
  `strsim`.
- Disagreement: E has 16 pin default severities. Ticket 27 blocks on 16 for the
  diagnostic envelope; 16 pins codes and the principle that type problems are
  never `Error` by default, and 27 owns severities and overrides.
- Huge enums and strftime formats cannot be projected to JSON Schema; they
  stay unconstrained there and are checked after the call.

## 8. Schema model facts [S: FG]

- No code resolves effective fields for a note with several classes today.
- All schemas in the author's vault have at most one `extends` parent; multi
  class occurs at the note's class field.
- `SchemaSelectField` stores a `Vec` of entries and inheriting schemas clone
  it (3,734 entries for `job_titles`); a shared `Arc` lookup would avoid it.
- Conflict rule between classes: first-listed wins, with an advisory.
- External select lists with `value = "link"` produce wikilinks, so they are
  relations in practice, which needs ticket 15's adaptive literal rule.

## 9. Not verified

- All [S] items above; no benchmark was run.
- That `NoteFieldValue` can move below `schema` without pulling note-parsing
  types.
- Whether any harness hook can block a direct file write for the author's
  actual agent (omp, Claude Code); B and C used documentation, not trials.
