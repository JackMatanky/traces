# Query and template consumers of note facts

This file lists how the query layer reads `Note` facts. I checked the claims
marked "checked now" in the code. The other claims come from the first agent and
keep its tag.

## Summary

- [V] A query row holds an index position and an optional item index. It does
  not copy a `Note`. List and task rows iterate `Note::lists` in document order
  (`src/query/results.rs:31-85`; `src/query/service.rs:246-305`).
- [V] The `tags` field of a list row reads the item tags. The `tags` field of a
  page row reads `Note::tags`. `file.tags` always reads `Note::tags`
  (`src/query/results.rs:200-206,313-316`). Checked now.
- [V] A metadata key on an item row reads the first value of the item field. If
  the item has no such field, it reads `Note::get(key)`
  (`src/query/results.rs:320-333`). Checked now.
- [V] `Note::get` returns the frontmatter value first. Then it returns the first
  inline value (`src/note/model.rs:160-173`). Checked now.
- [H] The parser copies item fields and item tags into the note-level
  collections (`src/note/parser.rs:488-498`). So `Note::get` can return a field
  that only one item has. A page query can match on that field. Test this before
  ticket 23 decides the rollup.
- [V] `Cargo.toml` has no LSP dependency. It has no `lsp-types` and no
  `tower-lsp`. It has only a `tokio_unstable` lint setting (`Cargo.toml:255`).
  Checked now.

## Read set

| Consumer | Headings | List items | Outlinks | Tags | Inline fields | Frontmatter | Positions |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| Query | no | yes | no | yes | yes (note and item) | yes | `list.line`, `list.parent`, `list.depth` |
| Template | not checked | yes | not checked | yes | yes | yes | not checked |

- [V] The query layer does not read `headings` or `outlinks` in the files the
  agent read. Outlinks feed the `LINKS` table and the inlink map in the index.
- [H] The agent says the CLI file `src/cli/template.rs` is the only direct
  consumer for template dispatch. I did not check this.

## Query row model

- [V] A row is `QueryRow`: an index reference, a file position and a row type.
  The row type is a page row or a list row with `item_idx`
  (`src/query/results.rs:31-85`).
- [V] The task query filters the list items by task kind. The row exposes
  `task_text`, `task_completed` and `task_priority_rank`.
- [V] Document order comes from the `Note::lists` order. The sort is stable
  (`std::slice::sort_by`), so tied rows keep their order
  (`src/query/sort.rs:79-145`).
- [V] `size_of::<QueryRow>()` has a test bound of 112 bytes
  (`src/query/results.rs:787-791`).
- [P] If items become separate indexed entities, the row identity changes from
  the pair (note index, `item_idx`) to a stable item id. Define the id before
  ticket 26 starts.
- [P] If the note exposes multi-valued inline fields, `Note::get` needs a new
  return type. It returns `Option<&NoteFieldValue>` today.

## Editor consumers

- [V] No LSP code or LSP dependency exists today.
- [H] Ticket 34 needs an unsaved buffer to override the stored note. This can
  need a `Note` built from the buffer. IWE keeps an in-memory graph overlay.

## Public surface

- [V] `Note`, `Heading`, `ListItem`, `ListItemType`, `ListText`, `TaskListItem`,
  `NoteFieldValue`, `NoteFieldValueRef`, `parse_markdown` and
  `MarkdownParserInput` are public (`src/lib.rs:157-160`). Checked now.
- [H] The agent says `pub(crate)` is the usual visibility in `src/note`,
  `src/query` and `src/index`.
- [V] `parse_note` exists only under the `test-utils` feature
  (`src/lib.rs:559-561`). Checked now.

## Lessons for ticket 23

- C4, C5: An item row reads item fields first, then note-level fields. Removing
  the rollup changes the page-row result. Record the change in the ticket 16
  change table.
- C6: Multi-valued fields need a new accessor type. The only reader of a
  multi-value today uses `.first()` (`src/query/results.rs:328`).
- C4: Row identity needs a stable id if items leave the `Note`.

## Not checked

- The template engine internals beyond `src/template/engine/query.rs`.
- The `pub-tokens.txt` baseline entries for the note types.
- The changes in `src/query` after `207cc3fb`.

## Inconsistencies found while rewriting

- The first version says "LSP deps (`lsp-types`) are present in `Cargo.toml`".
  This is false [V]. `Cargo.toml` has no LSP dependency.
- The first version says `src/lib.rs` exports `parse_note`. `parse_note` is
  available only with the `test-utils` feature [V].
- The first version has a table row for the CLI with no content in every column.
- The first version says `item_idx` is brittle if the order changes. The index
  rebuilds the rows from the stored `Note` on every query, so this is a risk
  only if ticket 23 stores item ids [H].
