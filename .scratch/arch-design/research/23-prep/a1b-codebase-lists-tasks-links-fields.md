# Code facts: lists, tasks, links and fields

I replaced the first version of this file. It had filler and one passage that
made no sense. This version lists only facts that I read in the code.

## Summary

- [V] `ListItem` is 160 bytes. `TaskListItem` is 72 bytes. `Heading` is 32
  bytes. `NoteFieldValue` is 80 bytes (`g-probe-results.md`).
- [V] Every task owns a `String`. `TaskStatus` holds `symbol`, `name: String`
  and `kind` (`src/task.rs:50-55`).
- [V] The status symbol is a `char` newtype: `TaskStatusSymbol(char)`
  (`src/task.rs:370-371`).
- [V] Hierarchy is flat. Each item stores `depth` and `parent` as a source line
  (`src/note/lists.rs:1-7,40-50`).
- [V] `ListText` stores the raw text. It stores the clean text only when it
  differs (`src/note/lists.rs:493-502,517-524`).
- [V] The parser copies item fields and item tags into the note-level
  collections (`src/note/parser.rs:488-498`).
- [V] `Link` holds `target` and an optional `text` (`src/note/links.rs:27-31`).
  The constructor call at `src/note/parser.rs:439` also passes a kind.

## Types

`ListItem` (`src/note/lists.rs:40-50`):

| Field | Type |
|-------|------|
| `text` | `ListText` |
| `kind` | `ListItemType` |
| `depth` | `u8` |
| `line` | `SourceLine` |
| `parent` | `Option<SourceLine>` |
| `is_ordered` | `bool` |
| `fields` | `Option<Box<ListFieldMap>>` |
| `tags` | `Box<[Tag]>` |

- `ListFieldMap` is `IndexMap<FieldKey, Box<[NoteFieldValue]>>`
  (`src/note/lists.rs:30`). `Note.inline_fields` has the same type
  (`src/note/model.rs:31`).
- `ListItemType` is `Plain`, `Checkbox` or `Task(TaskListItem)`
  (`src/note/lists.rs:222-241`).
- `TaskListItem` holds `dates`, `priority` and `status`
  (`src/note/lists.rs:347-352`). The doc comment says it also holds a
  precomputed subtree completion state (`src/note/lists.rs:15-17`).
- `TaskStatusMap` holds three `HashMap` values: by symbol, by name and by kind
  (`src/task.rs:118-122`).
- `ListItem` does not derive `Eq`. The code comment says
  `NoteFieldValue::Number` holds an `f64` (`src/note/lists.rs:35-39`). Ticket 18
  decides `Int` and `Float`. [H] The comment is stale after ticket 18.

## Lessons for ticket 23

- C7: A status symbol is a `char` today. The `name: String` is what costs an
  allocation for each task. A symbol-only task needs a lookup from symbol to
  name in the status map. [P]
- C4: A task and a plain item share one `ListItem` type. Items link to a parent
  by source line. A stable item id needs a new field. [P]
- C5: Item fields live in two places: `ListItem.fields` and
  `Note.inline_fields`. Removing one copy is a change to query results
  (`a2b-query-template-consumers.md`).
- C6: `inline_fields` stores all values of a key as a boxed slice. The query
  reads the first value only.

## Not checked

- The size of `TaskStatus` and the parse path of task dates.
- The `Logos` lexer and `MarkerAccumulator` internals
  (`src/note/parser/lexer.rs`, `marker.rs`).
- The `GLOSSARY.md` claims about slugs, footnotes and callouts. The findings
  ledger says the code has none of them. I did not check this again.
- The link kinds (wikilink, markdown, embed) and the heading-fragment syntax.

## Inconsistencies found while rewriting

- The first version had two copies of the sections "Logos Item Lexing" and
  "Callers". One copy had a passage that made no sense ("Revision Details
  Addressing Former Em Pane"). I removed it.
- The first version asked if `TaskStatus` contains an owned `String`. The answer
  is yes (`src/task.rs:50-55`).
- The first version tagged `Size ... approx 160 B [H]`. The probe measures 160
  bytes [V].
- The first version said "Allocation ... is allocated within `TaskListItem` [V]"
  without a citation.
- The first version had an empty `NoteLink` entry and no `Link` fields.
