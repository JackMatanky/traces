# PKM precedent: Obsidian, Dataview and markdown-oxide

This file covers how Obsidian, Dataview and markdown-oxide model a parsed note.
The agent read Dataview and markdown-oxide in the digests. It did not open
Obsidian docs pages. I did not check these claims, except where I say so. The
`[V]` tags are the agent's. Logseq, Foam, Marksman and zk are not covered.

## Summary

- [V] Obsidian `CachedMetadata` holds `sections`, `headings`, `listItems`,
  `tags`, `links`, `embeds`, `frontmatter`, `frontmatterLinks`, `blocks` and
  `frontmatterPosition`. Most entries have a `Pos` range.
- [V] Dataview builds a two-layer model on this cache: `PageMetadata` for the
  page and `ListItem` for the list item and the task.
- [V] Dataview merges the frontmatter fields and the inline fields of a page
  into one map. Checked in `c2-digests-obsidian.md`: the merge concatenates the
  values of a key.
- [V] Dataview keeps `Map<string, Literal[]>` while it parses. It then stores a
  single value as a scalar and several values as an array.
- [V] Dataview keeps a `FullIndex` with `pages`, `tags`, `etags` and `links`. A
  `revision` number marks freshness. Events from the metadata cache and the
  vault trigger a reload. A `LocalStorageCache` persists the data.
- [V] markdown-oxide has an option for GitHub-style heading slugs in
  completions. Go-to-definition and diagnostics resolve both the dash form and
  the space form.

## Obsidian `CachedMetadata`

- [V] `MetadataCache.getCache(path)` returns it. `MetadataCache.on('changed',
  ...)` delivers it.
- [V] `sections` are root-level markdown blocks.
- [V] `frontmatterPosition?: Pos` is the anchor of the YAML frontmatter.
- [H] A host can give a pre-parsed structural cache with source ranges. This
  lets a PKM layer avoid a full reparse.
- Gap: the agent did not find the semantics of nested constructs, the stability
  of `Pos` across edits, or the granularity of `blocks`. All are [H].

## Dataview

`PageMetadata` holds (agent, [V]):

- `path`, `ctime`, `mtime`, `size`, `day` and `title`.
- `fields: Map<string, Literal>`: the merge of inline fields and frontmatter.
- `tags`, `aliases`, `links` and `lists`.
- `frontmatter: Record<string, Literal>`.

`ListItem` holds (agent, [V]):

- `position`, `line`, `lineCount`, `list` (the list start line) and `section` (a
  link to the containing header).
- `fields` for the inline fields of the item.
- `task?: { status; checked; completed; fullyCompleted }`.

Dataview serialises tasks as `lists.filter(l => !!l.task)`.

Multi-valued fields (agent, [V]):

- Frontmatter: `addInlineField(key, value, fields)`. Inline: each parsed value
  goes into `Map<string, Literal[]>`.
- `mergeFieldGroups(target, source)` concatenates the arrays of one key.
- `finalizeInlineFields` stores `value[0]` when the length is 1. Otherwise it
  stores the array.

Inline field parsing (agent, [V]):

- `findSeparator(line, start)` finds `::`. `findSpecificInlineField` checks the
  key and the closing wrapper with nesting. `extractInlineFields` sorts by start
  and removes overlapping fields.
- Dataview skips very long lines and lines without `::`.

Task reconstruction (agent, [V]):

- The code iterates over `metadata.listItems`. It uses `LIST_ITEM_REGEX` for the
  marker and the checkbox.
- It joins the lines from `position.start.line` to `position.end.line` for the
  item text.
- For a task: `status` is the raw marker. `checked` is true when the marker is
  neither empty nor a space. `completed` is true for `X` or `x`.

Header links (agent, [V]):

- `normalizeHeaderForLink` collapses whitespace, handles emoji and normalises
  control characters. Tests cover markup and `[[link|display]]` forms.

## markdown-oxide

From `docs/digests/lsp_feel-ix-343-markdown-oxide-digest.txt` (agent):

- [V] The `heading_slug` option says "Use GitHub-style heading slugs in
  completions (spaces replaced with dashes)".
- [V] Go-to-definition and diagnostics resolve both forms.
- [V] Heading links with dashes and heading links with spaces both resolve
  ("lenient matching").
- [H] A `Referenceable` abstraction unifies items for rendering and completion.
- [H] The agent did not extract the footnote and callout syntax rules.

## Lessons for ticket 23

- C4, C5: Dataview separates page facts from item facts. [P] Dataview also rolls
  up the fields of non-task items to the page, and not those of task items
  (`c2-digests-obsidian.md`). The separation is not complete.
- C6: Dataview shows an internal array and an external scalar or array. [P] A
  uniform list avoids two shapes for one field. Ticket 16 says Dataview parity
  is Free.
- C7: Dataview keeps the task as a sub-structure of the list item. The status is
  the raw marker. Traces keeps a `TaskListItem` in a `ListItemType::Task`.
- C10: Fragment resolution needs a canonical form and lenient matching. [P]
  Decide the heading slug rule when ticket 23 models slugs.
- C13: Dataview uses events and a `revision`. It does not reparse every file for
  each query.
- C14: `sections` are root-level blocks. Dataview uses them to decide which
  sections to scan. [H] A two-stage model fits ticket 36: host section spans,
  then item facts inside.

## Not checked

- YAML span handling in editors (how a diagnostic maps to a YAML key).
- Stability and granularity of Obsidian `blocks`.
- Dataview issues about multi-valued fields, task tags and large vaults.
- Performance budgets (C16) and naming drift (C17).
- Logseq, Foam, Marksman and zk.

## Inconsistencies found while rewriting

- The title of the first version names Logseq, Foam, Marksman and zk. The file
  covers none of them.
- The first version ends with a section "Final reply (for Main)". It is a chat
  message, not part of the evidence. I removed it.
- The first version says a note schema and an item schema "is likely to reduce
  ambiguity" in the same paragraph as a [V] claim. It is an inference. I tagged
  it [H] and added the rollup caveat.
- The first version describes Dataview as a clean split of note facts and item
  facts. The digest shows that the fields of non-task items roll up to the page
  (`c2-digests-obsidian.md`).
- The first version calls the scalar-or-array collapse "a strong model" for
  ticket 23. It gives no source for user satisfaction. Ticket 16 makes parity
  Free.
- The first version tags "Obsidian provides span-aware caches" [V] in the
  summary and [H] in the detail.
