# Obsidian ecosystem digests: item, page and field models

This file summarises how Dataview, Obsidian Tasks and related tools model
fields, items and positions. I checked the Dataview claims about merging and
rollup in the Dataview digest. The first version had one wrong claim, which this
version corrects. The other claims keep the tag of the first version.

## Summary

- [V] Dataview collects the inline fields of a key in a `Map<string,
  Literal[]>`. `finalizeInlineFields` then stores a single occurrence as a
  scalar and several occurrences as an array
  (`docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3719-3731`).
- [V] Dataview puts the frontmatter fields into the same map with
  `addInlineField(key, value, fields)`. It then calls `mergeFieldGroups(fields,
  markdownData.fields)` (same file, lines 3355-3358 and 3391-3393).
  `mergeFieldGroups` concatenates the arrays of a key that exists in both (lines
  3734-3737). There is no precedence rule. A key in the frontmatter and in the
  body becomes one array.
- [V] For a list item that is not a task, Dataview merges the item fields into
  the page-level fields: `mergeFieldGroups(literals, listItem.fields)` (same
  file, lines 3603-3604). The fields of a task item do not go to the page.
- [V] The code that would merge the fields of a non-task item into its nearest
  ancestor task is in a comment with a TODO (same file, lines 3606-3614). It
  does not run.
- [V] Dataview skips the `position` key of the frontmatter, because Obsidian
  sets it (same file, lines 3356-3357).
- [V] Dataview stores a `ListItem` with `position`, `line`, `lineCount`, `list`,
  `blockId`, `tags`, `text`, `fields` and `children` (agent, `parseLists`).
- [V] Obsidian Tasks reads the task lines from `fileCache.listItems` of
  Obsidian. It uses `position.start.line` as the anchor.
- [V] Obsidian Tasks reads a task line backwards from the end. It recognises
  emojis with values, tags and block links. It stops at the first value that it
  does not recognise (Tasks docs).
- [V] Obsidian Tasks reads task tags from the task lines only. It does not read
  tags from the frontmatter.
- [V] `obsidian-advanced-metadata-cache` adds indexes for questions that the
  core cache answers by a scan. Examples: backlinks and link suggestions. Its
  methods wait until the index is ready, or return `null`.
- [V] Metadata Menu finds the frontmatter range with
  `metadataCache.getFileCache(file).frontmatterPosition`. It uses the range to
  choose between inserting a `yaml` field and an `inline` field.

## Lessons for ticket 23

- C5: Dataview rolls up the fields of non-task items to the page. It does not
  roll up the fields of task items. The Traces parser copies the item fields to
  the note-level collection (`src/note/parser.rs:488-498`). [H] It copies the
  fields of task items too. Check this before ticket 23 decides.
- C6: Dataview merges frontmatter values and body values of one key into one
  array. Traces gives the frontmatter value first and ignores the body value
  (`src/note/model.rs:128-173`). The two behaviours differ. Ticket 16 makes
  Dataview parity Free.
- C6: Dataview shows a single value as a scalar and several values as an array.
  A query then has to handle both. [P] A uniform list type avoids this.
- C2: Obsidian stores positions as line, column and offset (`position` objects).
  Tasks uses only the start line.
- C14: `obsidian-advanced-metadata-cache` exists because the core cache has no
  index for some questions. An index should answer the questions that consumers
  ask. [P] The parser should keep the facts that LSP features need: headings,
  links and tags.
- C7: Obsidian Tasks parses task metadata from the end of the line. Text after
  the metadata stops the parse. This is a user-visible rule.

## Comparison

| Tool | Source of items | Scope | Multi-value behaviour | Position | Known limit |
|------|-----------------|-------|-----------------------|----------|-------------|
| Dataview | Obsidian metadata cache | page, list item, task | scalar for one value, array for several | Obsidian `position` | rescans on large vaults [H] |
| Obsidian Tasks | Obsidian `listItems` | task | not checked | start line | stops at the first unknown value |
| Advanced Metadata Cache | its own index | page, file | not applicable | link positions | extra plugin |
| Metadata Menu | Obsidian metadata cache | page, inline | not checked | frontmatter range | built for UI, not for queries |

## Not checked

- Precedence at query time between a page field and an item field.
- The handling of many tags and dates in Obsidian Tasks.
- The schemas of the `obsidian-advanced-metadata-cache` modules.
- Whether the `rust_zoni-obsidian-export`, `rust_adamisrael-obsidian-rs` and
  `vim_obsidian-nvim` digests add anything. The agent found only tag filters and
  a picker cache.

## Inconsistencies found while rewriting

- The first version says Dataview "merges metadata from the non-task list item
  tree into the nearest ancestor task". The digest shows that this code is
  commented out (lines 3606-3614). The active code merges the fields of non-task
  items into the page. The first version was wrong [V].
- The first version says "Digests did not reveal explicit precedence rules" for
  a key in both the frontmatter and the body. The digest shows a merge by
  concatenation (lines 3391-3393 and 3734-3737). The first version missed this.
- The first version labels the sections with C2, C3, C4, C5, C6, C7 and C14.
  They do not match the concept numbers of `brief.md`. For example, the section
  "C3 multi-valued inline fields" belongs to C6.
- The first version tags every claim [V], including inferences. Example: "That
  implies task-level tags are on the list item". I removed the inferences or
  tagged them [H].
- The first version lists "Not checked" items with the tag [P]. They are gaps,
  not proposals.
- The comparison table of the first version states "Array-like (tags/dates)" for
  Tasks, "Scalar-primary" for Metadata Menu and "Performance on large vaults
  (rescans)" for Dataview. No source supports these. I marked them "not checked"
  or [H].
