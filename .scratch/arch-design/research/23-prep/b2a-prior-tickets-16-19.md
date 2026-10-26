# Evidence for ticket 23 from tickets 16 to 19

This file lists what tickets 16, 17, 18 and 19 decide about the note model.
Citations of the form `issues/NN-*.md:LINE` come from the agent. I did not check
every line number. Citations of the form `23:NN` point to an earlier version of
ticket 23 and are stale. The text of a decision is in the cited ticket. This
file gives a paraphrase.

## Summary

- [V] Vault-format parsing is Frozen (`issues/16-*.md:72`). The same files must
  parse to the same meaning.
- [V] Index results must equal the results of a fresh parse. This is Frozen
  (`issues/16-*.md:74`).
- [V] Dataview and Templater parity is Free (`issues/16-*.md:73`). They are
  historical influence only.
- [V] The Rust library surface is Free (`issues/16-*.md:87`). The visibility
  rule still applies (`issues/16-*.md:89-95`): use the narrowest visibility.
- [V] Known defects are not preserved (`issues/16-*.md:86`). They are the
  closed-pipe panic, the `i64` to `f64` loss at the note boundary
  (`src/note/field.rs:545`) and the `.first()`-only read of multi-valued inline
  fields. Each fix is a change-table row, not a regression.
- [V] The LSP is a committed second consumer (`issues/17-*.md:37-44`). The core
  stays synchronous. Editor features need source positions: diagnostics,
  navigation and completion.
- [V] General note write-back is not required (`issues/17-*.md:44,73,98`). A
  future writer splices spans into the original bytes.
- [V] Ticket 17 leaves the split to ticket 23
  (`issues/17-*.md:45-48,70-72,82-84`). The options are one shared span-aware
  `Note` or separate editor and index representations.
- [V] The measurement points are 50, 200, 1,000, 5,000, 10,000, 20,000 and
  50,000 notes (`issues/17-*.md:50-54`). Operations have their own budgets.
- [V] Ticket 18 adopts two owned value enums (`issues/18-*.md:447-451`):
  `note::NoteFieldValue` and a schema-local literal type. Neither has a borrowed
  mirror.
- [V] Ticket 18 splits `Link` and `NoteLink` (`issues/18-*.md:486-492`). The
  root `Link` holds `target`, `display: Option<String>` and `embedded: bool`.
  `NoteLink` holds the syntax kind and the source span. Ticket 23 defines it.
- [V] Ticket 18 rejects the Noyalib CST (+59% parse time) and keeps the compat
  parser (`issues/18-*.md:467-470,494-505`). Ticket 19 makes a `NaN` become
  `Null` and reports a notice. Inline fields gain `DateTime`.

## Ticket 16: compatibility envelope

| Decision | Source | Concepts | Effect on ticket 23 |
|----------|--------|----------|---------------------|
| Vault-format parsing is Frozen | `issues/16-*.md:72` | C15, C6, C1 | BINDS. A deliberate semantic change, such as multi-value retention or `NaN` to `Null`, needs a change-table row. |
| Index result equals a fresh parse; Frozen | `issues/16-*.md:74` | C1, C12 | BINDS. What the index stores must give the same query results as a fresh parse. |
| Dataview and Templater parity; Free | `issues/16-*.md:73` | C5, C6 | INFORMS. Ticket 23 may depart from the Dataview rollup and multi-value rules. |
| Known defects; not preserved | `issues/16-*.md:86` | C6, C15 | BINDS. Storing and querying all values of a multi-valued field is a sanctioned fix. |
| Library surface; Free, with the visibility rule | `issues/16-*.md:87,89-95` | C17, C1 | BINDS. A new type in `src/note/` must be private or `pub(crate)`, unless a crate boundary needs it. |

- [V] The ticket inputs are `survey/cli.md`, `survey/cross-cutting.md`, the
  ADRs, `tests/e2e/*` and the baseline files `tests/baselines/public-api-*.txt`
  and `pub-tokens.txt`.
- [V] The change table needs a surface, a tier and a migration for each change.
  An observable change that is not in the table is a regression (`issues/16`).

## Ticket 17: forward consumers

| Decision | Source | Concepts | Effect on ticket 23 |
|----------|--------|----------|---------------------|
| The LSP is the next consumer. Editor features need source positions. | `issues/17-*.md:37,43-44` | C1, C2, C10 | BINDS. The model must give source positions. |
| General write-back is not required. | `issues/17-*.md:44,73,98` | C1, C2 | BINDS. No tree re-synthesis is required. |
| The split is left to ticket 23. | `issues/17-*.md:45-48,70-72,82-84` | C1, C2, C12 | BINDS. Ticket 23 chooses: one shared span-aware `Note`; a `Note` and a `NoteDocument`; or a span-free `Note` with an on-demand reparse. |
| The core is synchronous. | `issues/17-*.md:56-57` | C8, C13 | BINDS. Note parsing has no `async` signature. |
| Seven vault sizes are measurement points. | `issues/17-*.md:50-54` | C16 | BINDS. Check span cost and `ListItem` size at these points. Do not require equal optimization at every size. |

## Ticket 18: value model

| Decision | Source | Concepts | Effect on ticket 23 |
|----------|--------|----------|---------------------|
| Two owned value enums; no borrowed mirror. | `issues/18-value-model.md:447-450,471-477` | C1, C6 | BINDS. `Note` stores owned `NoteFieldValue`: `Null`, `Bool`, `Int(i64)`, `Float(f64)`, `String`, `Date`, `DateTime`, `Duration`, `Link(Link)`, `List` and `Object`. |
| Keep the 80-byte size test. Do not box more variants. | `issues/18-value-model.md:390-396,475-476` | C6, C16 | BINDS. `NoteFieldValue` stays at 80 bytes or less. |
| `Link` and `NoteLink` are separate. | `issues/18-value-model.md:433-435,486-492` | C1, C2, C10 | BINDS. `Link` holds `target`, `display` and `embedded`. `NoteLink` is the editor-side occurrence type with the syntax kind and the span. Ticket 23 defines it. Ticket 32 chooses the module name. |
| Keep the Noyalib compat parser. Convert its `Value` directly to `NoteFieldValue`. | `issues/18-value-model.md:201-224,467-470,494-501` | C3, C8 | BINDS. Ticket 23 must not use the Noyalib CST for frontmatter. `Document::span_at(path)` is unsound for dotted keys, tags and block scalars. |
| Use one classification order for frontmatter and inline fields. Frontmatter ignores quotes. Inline quotes escape. | `issues/18-value-model.md:502-508` | C6, C8 | BINDS. The date, duration, link and blank parsers are shared. |
| Use `Int(i64)` and `Float(f64)`. | `issues/18-value-model.md:509-515` | C4, C6 | BINDS. The inline parser gives `Int` when the text fits `i64`. |
| Handoff: editor-side `NoteLink` and the indexing and query shape against the editing shape. | `issues/18-value-model.md:562-563` | C1, C2 | BINDS. Ticket 23 defines the shape and the place of `NoteLink`. |

- [V] Measured sizes: `NoteFieldValue` 80 B, `Link` 56 B, `DurationValue` 72 B
  and `IndexMap` 72 B (`issues/18`). The probe measures `NoteFieldValue` as 80
  B.

## Ticket 19: coercion and comparison

| Decision | Source | Concepts | Effect on ticket 23 |
|----------|--------|----------|---------------------|
| The parser classifies a value once. No code classifies it again. A consumer gets a text view of the typed value. | `issues/19-*.md:55-59,104-108` | C1, C6 | BINDS. The note stores typed values and no raw lexeme. |
| Inline fields can yield `DateTime`. A naive date-time is a civil value. | `issues/19-*.md:101-103,575-577` | C6 | BINDS. `parse_atom_at` in `src/note/parser/inline.rs` must parse `DateTime`. |
| A `NaN` becomes `Null` and the parser reports a notice. | `issues/19-*.md:90-91,188-193` | C11, C15 | BINDS. Parsing stays infallible. Ticket 31 owns the mechanism that shows the notice. |
| A multi-valued inline field classifies each element. | `issues/19-*.md:226-227` | C6 | BINDS. Each element of `key:: a, b, 2025-01-15, [[link]]` goes through the atom grammar. |
| `Link` identity is the folded target. `display` and `embedded` are attributes. | `issues/19-*.md:154-156` | C1, C10 | INFORMS. Equality ignores the display and the embed syntax. |

- [V] The inline parser `parse_atom_at` parses atoms in a strict order.
  `parse_inline_value` parses one atom or a comma-separated list of atoms
  (`src/note/parser/inline.rs:39-141`).

## Conflicts and handoffs that the ticket needs

1. Inline `DateTime` and the grammar.
   - Ticket 18 excluded inline `DateTime` (`issues/18:357-358,505-506`). Ticket
     19 reversed this (`issues/19:101-103,575-577`).
   - Ticket 23 must place `parse_datetime_at` in `parse_atom_at`
     (`src/note/parser/inline.rs:131-141`). Questions: does `YYYY-MM-DDTHH:MM`
     come before `YYYY-MM-DD`? Which separator does the date-time use, `T` or a
     space? A space separator can consume the next token in an inline list. [P]
2. The notice channel for an infallible parse.
   - Ticket 19 requires a notice for `NaN` (`issues/19:90-91,188-193`). Ticket
     16 requires that vault parsing is Frozen (`issues/16:72`).
   - Ticket 23 must give the parser signature: a `(Note, Vec<NoteNotice>)`
     result, or a notice sink. Ticket 31 owns the presentation.
3. Frontmatter spans and the direct conversion.
   - Ticket 18 rejected the CST and converts `noyalib::Value` directly.
   - Ticket 20 says adapters need a span for each field key and each list
     element where known.
   - `noyalib::Value` has no byte offsets. Ticket 23 must say how adapters get
     spans. `g-probe-results.md` measures a line scan and `Spanned<Value>`.
4. Multi-value storage.
   - Ticket 16 lists the `.first()` read as a known defect (`issues/16:86,99`).
   - In the code, `inline_fields` is `IndexMap<FieldKey, Box<[NoteFieldValue]>>`
     (`src/note/model.rs:31`). `frontmatter` wraps `IndexMap<FieldKey,
     NoteFieldValue>` (`src/note/metadata.rs:95-98`).
   - Ticket 23 must decide the effective field map, the collision rule and the
     storage of multi-values: `NoteFieldValue::List` or several entries.

## Not checked

- [H] A byte scanner over the raw frontmatter against the Noyalib CST on 50,000
  notes. `g-probe-results.md` gives a smaller measurement.
- [H] The memory cost of `NoteLink` in a shared `Note` against an editor-only
  `NoteDocument`.
- [H] Whether one unrecognized atom makes the whole inline field a string.
- The unit-test wording pins in `tests/unit/*`. Ticket 16 did not scan them
  (`issues/16-grilling-compatibility-envelope.md:101`).

## Inconsistencies found while rewriting

- The first version cites `issues/23-note-document-model.md:46,150-154`. These
  lines belong to an earlier version of the ticket.
- The first version says ticket 16 "mandates that vault parsing is Frozen and
  infallible (`issues/16:72,99`)". The Frozen row of ticket 16 says the same
  files parse to the same meaning. I did not find "infallible" in that row. [H]
  Check `issues/16:99`.
- Ticket 18 lists `Int(i64)` and `Float(f64)`. The code comment at
  `src/note/lists.rs:35-39` still says `NoteFieldValue::Number` holds an `f64`.
  The code has not caught up with ticket 18 [H].
- The first version tags "Not checked: unit-test wording pins ..." as [V]. It is
  a gap, not a verified fact.
- The first version labels the multi-value item "Multi-Value Field
  Representation". It sits under "conflicts", but ticket 16 already decides that
  the fix is sanctioned. The open part is the storage and the collision rule
  only.
