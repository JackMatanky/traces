# Stress test and conflict report for ticket 23

This document stress tests decisions Q1 through Q7 against all tickets. It
compares decisions against tickets 16 to 22 and 24 to 36 [V]. It also checks the
redesign map [V: .scratch/arch-design/map.md:1-405]. Every claim has a tag: [V]
for checked text, [H] for inference, [P] for a proposal.

## Conflict table

| Decision | Ticket | Quote | Severity |
|---|---|---|---|
| Q1 | 18 | `NoteLink (syntax kind, span) is the editor-side type` [V: issues/18-value-model.md:260-261] | Low |
| Q2 | 16 | `Frozen: vault-format parsing, index result equivalence with a fresh parse` [V: issues/16-grilling-compatibility-envelope.md:237-238] | High |
| Q3 | 20 | `The app layer owns any merge of inline fields into that map` [V: issues/20-schema-note-typing-and-validation.md:122-123] | High |
| Q3 | 19 | `Across brackets, == is false and != is true` [V: issues/19-coercion-and-comparison-policy.md:78-79] | Medium |
| Q4 | 22 | `Make note own its parser input, so note no longer imports config` [V: issues/22-config-ownership-and-lifecycle.md:354-355] | None |
| Q5 | 16 | `Frozen: vault-format parsing... The same files must parse to the same meaning` [V: issues/23-note-document-model.md:76-77] | None |
| Q6 | 20 | `schema owns the pure, span-free rule over resolved effective fields` [V: issues/20-schema-note-typing-and-validation.md:298-299] | None |
| Q7 | 31 | `A parse-time NaN-to-Null warning is a third` [V: issues/31-error-and-diagnostic-architecture.md:33] | None |

## Decision Q1: Stored body spans and long-lived host

The decision stores body spans in the shared `Note` [V:
issues/23-note-document-model.md:17-22]. The LSP host loads all notes from
`index.redb` at start-up [H].

### Conflicts with resolved tickets

Ticket 18 states that `NoteLink` is the editor-side type [V:
issues/18-value-model.md:260-261]. If `Note` stores spans, `NoteLink` is no
longer editor-side only [H]. The shared `Note` holds `NoteLink` in persistence
[P]. This conflict is low severity and needs only a name update [P].

Ticket 16 classifies the `index.redb` layout as Free [V:
issues/16-grilling-compatibility-envelope.md:239-240]. Adding spans to stored
notes obeys Ticket 16 [V].

### Assumptions in open tickets

Ticket 24 asks if the store keeps the whole `Note` or extracted facts [V:
issues/24-index-read-model-and-query-execution.md:20-21]. Q1 answers this
question: the store persists `Note` with spans [P].

Ticket 25 asks about note deserialization costs [V:
issues/25-index-freshness-and-invalidation.md:36-39]. Q1 increases the
serialized record size by 4.3 percent [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:8-9].

Ticket 34 needs the editor shape from an unsaved buffer [V:
issues/34-live-buffer-and-persisted-index-consistency.md:9-14]. Q1 fixes this
need [P]. The parser builds the shared `Note` directly from an in-memory buffer
[P].

### Hidden costs

Loading 50,000 notes into memory uses large memory [H]. The probe shows 28 list
items per note on average [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:96-97]. 50,000 notes
hold about 1,400,000 list items [H]. The decoded notes consume between 500
megabytes and 1 gigabyte of RAM [H]. Cold CLI queries also pay to decode body
spans when reading only frontmatter [H].

### Edge cases

1. Vault of 50,000 notes. Decoding takes 0.31 seconds [V:
   .scratch/arch-design/research/23-prep/z-grilling-guide.md:61-63]. Holding all
   notes in RAM can cause memory pressure [H]. Smallest fix: Ticket 24 must
   support lazy note decoding or paged loading [P].
2. Version change of the stored record. Old redb databases cannot deserialize
   notes that gain spans [H]. Smallest fix: Bump `META_VERSION` in
   `src/index/store.rs:41` to wipe old caches on start-up [P].
3. Unsaved editor buffer. An unsaved buffer changes spans and content in memory
   [H]. The on-disk `index.redb` remains stale [V:
   issues/34-live-buffer-and-persisted-index-consistency.md:9-13]. Smallest fix:
   Ticket 34 routes LSP queries through an in-memory document overlay [P].

### Verdict

Needs change. The model is sound, but memory use at 50,000 notes needs lazy
loading in Ticket 24 [P].

## Decision Q2: Accessor-derived note view of item fields and tags

The decision removes stored item fields and item tags from note-level fields
[P]. An accessor derives the note-level view from `lists` [P]. Tasks remain a
kind of list item [V: src/note/lists.rs:46-47].

### Conflicts with resolved tickets

Ticket 16 sets `Frozen` status for vault-format parsing and index results [V:
issues/16-grilling-compatibility-envelope.md:237-238]. If an accessor changes
field order or tag lookup, it breaks `Frozen` equivalence [H].

### Call site audit for Q2

1. `inline_fields`:
   - `src/note/model.rs:122`: `pub(crate) fn inline_fields(&self)` returns
     `&IndexMap<FieldKey, Box<[NoteFieldValue]>>` [V].
   - `src/note/model.rs:148`: `Note::fields()` iterates `self.inline_fields`
     [V].
   - `src/note/model.rs:170`: `Note::get()` looks up keys in
     `self.inline_fields` [V].
   - `src/index/service.rs:1054,1083,1496`: Tests assert loaded inline fields
     equal built fields [V].
   - `src/note/parser.rs:768,780,803,1181,1204`: Tests inspect
     `note.inline_fields()` [V].
   - `src/query/service.rs:823`: Test checks `note.inline_fields()` [V].

2. `fields()`:
   - `src/note/model.rs:142`: `pub(crate) fn fields(&self)` iterates frontmatter
     then inline fields [V].
   - `src/note/model.rs:348,374,397`: Unit tests check `note.fields()` [V].
   - `src/query/results.rs:326`: Calls `item.fields()`, not `note.fields()` [V].

3. `get(key)`:
   - `src/note/model.rs:164`: `pub(crate) fn get(&self, key: &str) ->
     Option<&NoteFieldValue>` [V].
   - `src/query/results.rs:332`: `self.note().and_then(|note| note.get(key))`
     resolves query metadata [V].

4. `tags`:
   - `src/note/model.rs:179`: `pub fn tags(&self) -> &[Tag]` returns a borrowed
     slice [V].
   - `src/query/grammar/source.rs:221`: `note.tags().iter().any(...)` filters
     `FROM #tag` [V].
   - `src/query/results.rs:202`: Resolves `FieldPath::Tags` using `Note::tags`
     for notes [V].
   - `src/query/results.rs:314`: Resolves `FileField::Tags` using `Note::tags`
     [V].
   - `src/index/store.rs:1754`: Secondary index indexes
     `note.tags().iter().flat_map(...)` [V].
   - `src/index/service.rs:1116`: Test checks `loaded_note.tags()` [V].
   - `src/note/parser.rs:767,779,804,1556,1610`: Parser tests check
     `note.tags()` [V].

### Breaking problems with accessor derivation

1. Borrowed slice contract of `Note::tags(&self) -> &[Tag]`.
   `QueryFieldValueRef::Tags(&'a [Tag])` in `src/query/value.rs:25` needs a
   borrowed slice [V]. If `Note` does not store item tags in a single slice, it
   cannot return `&[Tag]` [H]. Deriving a `Vec<Tag>` on every call allocates
   memory on every row [H]. Storing pre-merged tags costs only 4,822 bytes
   across 2,711 notes (0.03 percent of the store) [V:
   .scratch/arch-design/research/23-prep/g-probe-results.md:93].
2. Linear scan overhead in `note.get(key)`. If `note.get(key)` scans
   `self.lists`, missing field lookups take linear time [H]. A query testing a
   missing field over 10,000 notes scans every list item [H].
3. Loss of document order across paragraphs and list items. In
   `src/note/parser.rs:490-498`, the parser flushes paragraph fields and list
   fields in document order [V]. If a list item field comes before a paragraph
   field, it appears first in the old map [V]. An accessor that inspects
   paragraph fields before list items changes the result [H].

### Edge cases

1. Tags on tasks only. A note has `#urgent` only inside `- [ ] #urgent task`.
   `QueryFieldValueRef::Tags` cannot borrow a merged slice if `Note` does not
   store it [H]. Smallest fix: Keep the stored merged `tags: Box<[Tag]>` on
   `Note` [P].
2. Nested items. An item at depth 2 holds `[rating:: 5]`. `self.lists` stores
   items in a flat document-order list [V: src/note/lists.rs:6-9]. The accessor
   can find it by scanning `self.lists` [V].

### Verdict

Needs change. Keep the stored `tags: Box<[Tag]>` because it is cheap and gives
`&[Tag]` [P]. For item fields, cache the effective view in memory to prevent
repeated linear scans [P].

## Decision Q3: Effective field map and merge ownership

The note model stores two maps: frontmatter fields and inline fields [P]. It
offers an effective-view accessor [P]. Frontmatter wins on key collision [V:
src/note/model.rs:165-169]. Multiple values become a list [P]. The note model
owns the merge [P].

### Conflicts with resolved tickets

Ticket 20 states: `The check takes a field map. The app layer owns any merge of
inline fields into that map` [V:
issues/20-schema-note-typing-and-validation.md:122-123]. Q3 says the note model
owns the merge [P]. This is a direct contradiction [V]. Severity is High because
it affects module boundaries [H].

Ticket 19 states: `Across brackets, == is false and != is true` [V:
issues/19-coercion-and-comparison-policy.md:78-79]. If multiple values always
become a `List`, scalar equality filters fail [H]. For example, `WHERE author =
"Alice"` evaluates to false when `author` is `List(["Alice", "Bob"])` [H]. Under
`.first()`, it evaluated to true [V: issues/23-note-document-model.md:48-49].

### Assumptions in open tickets

Ticket 35 expects `ResolvedFields::check(&FieldMap) -> Vec<Violation>` [V:
issues/35-schema-internals-simplification.md:12]. The note model's effective map
supplies this input directly [P].

Ticket 26 needs row identity and projection for multi-value lists [V:
issues/26-query-core-and-plan.md:17-18]. Ticket 26 must format lists into joined
strings for tables [V: src/query/value.rs:36-40].

### Hidden costs

Queries with equality filters on multi-value fields will stop matching rows [H].
Users must change `WHERE key = "val"` to `WHERE contains(key, "val")` [H].
Ticket 16 treats fixing `.first()` as an accepted change-table row [V:
issues/23-note-document-model.md:85-86].

### Edge cases

1. Key in frontmatter as null. Frontmatter has `author: null`, and body has
   `author:: Bob`. `fm.get("author")` returns `Some(&Null)` [V:
   src/note/model.rs:165-167]. Frontmatter wins, so `author` becomes `Null` and
   suppresses `Bob` [V]. Smallest fix: Document this behavior in the change
   table [P].
2. Multi-value inline field with mixed types. Body contains `count:: 10` and
   `count:: "ten"`. The accessor produces `List([Int(10), String("ten")])` [P].
   Ticket 19 permits heterogeneous lists [V:
   issues/19-coercion-and-comparison-policy.md:77]. Relational comparisons on
   the list fail as expected [V:
   issues/19-coercion-and-comparison-policy.md:78-79].

### Verdict

Needs change. Amend Ticket 20 line 122 to state that the note model owns the
merge accessor [P]. Record the list filter behavior in the change table for
Ticket 26 [P].

## Decision Q4: Task status representation

`TaskListItem` stores `symbol: char` and `kind: TaskStatusKind` [P]. It removes
the `name: String` field [P]. The name resolves from the status map on demand
[P]. The parser keeps the map to compute `fully_complete` [V:
src/note/parser/list.rs:430-437].

### Conflicts with resolved tickets

Ticket 22 says `note` owns its parser input and imports no `config` [V:
issues/22-config-ownership-and-lifecycle.md:354-355]. Q4 obeys Ticket 22 by
passing a borrowed status map to `parse_markdown` [P].

### Assumptions in open tickets

Ticket 26 assumes the query executor can print task status names [V:
issues/26-query-core-and-plan.md:35-36]. The executor holds the status map from
`Config` and resolves symbols on demand [P].

Ticket 30 and Ticket 34 pass `&TaskStatusMap` to `parse_markdown` [P].

### Hidden costs

Displaying status names without `Config` (such as a standalone note dumper)
prints symbols only [H].

### Edge cases

1. Unknown status symbol. Markdown has `- [?] unknown task`. If `?` is not in
   the status map, `resolve('?')` returns default `Todo` [V:
   src/note/parser/task.rs:178]. Name resolves to `"?"` or `"Unknown"` on demand
   [P].
2. Rename a status in configuration. A user renames status `x` from `"Done"` to
   `"Finished"`. Because `TaskListItem` does not store the string `"Done"`, the
   cache stays valid [P]. Queries immediately show the new name without a cache
   rebuild [P].

### Verdict

Robust. This design cuts 24 bytes from `TaskListItem` and eliminates thousands
of string allocations [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:46].

## Decision Q5: Guarded second pass for heading text

The parser collects heading text in pass 1 [P]. It runs pass 2 with
`ENABLE_MATH` only when a heading line contains `$` [P]. A differential test
protects the math tests [V: src/note/parser.rs:725-748,772-785].

### Conflicts with resolved tickets

Ticket 16 requires exact parsing parity for `Frozen` vault parsing [V:
issues/16-grilling-compatibility-envelope.md:237-238]. The probe found 0
mismatches across 2,688 notes without `$` in headings [V:
.scratch/arch-design/research/23-prep/z-grilling-guide.md:24-26]. Notes with `$`
in headings still run pass 2 [P]. Parity remains 100 percent [V].

### Assumptions in open tickets

Speeds up parse times by 21 percent on 98.8 percent of notes [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:14-15]. Directly
benefits Ticket 24, Ticket 25, Ticket 30, and Ticket 34 [P].

### Hidden costs

Scanning heading lines for `$` costs 1.05 milliseconds across 33,464 headings
(0.2 percent of parse time) [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:63]. The cost is
negligible [V].

### Edge cases

1. Escaped dollar in heading. A heading contains `Price is \\$50`. The line
   contains `$`, so pass 2 runs and unescapes the dollar correctly [V].
2. Setext multiline headings. The scan must check both lines of a Setext heading
   [P].

### Verdict

Robust.

## Decision Q6: Slugs derived on demand, footnotes and callouts added

The model stores no slugs [P]. Slugs derive on demand from heading text [P]. The
model adds footnotes and callouts [P].

### Conflicts with resolved tickets

Ticket 16 permits additions to internal models under `Free` tier [V:
issues/16-grilling-compatibility-envelope.md:239-241]. No conflict exists [V].

### Design of footnotes and callouts

1. Footnotes: Add `FootnoteDefinition` with `label: String`, `line: SourceLine`,
   `span: ByteSpan` [P]. Add `FootnoteReference` with `label: String`, `line:
   SourceLine`, `span: ByteSpan` [P]. `Note` holds `footnotes:
   Box<[FootnoteDefinition]>` and `footnote_refs: Box<[FootnoteReference]>` [P].
   `pulldown-cmark` already emits footnote events [V: src/note/parser.rs:69].
2. Callouts: Add `Callout` with `kind: String`, `title: Option<String>`, `line:
   SourceLine`, `span: ByteSpan`, `is_collapsible: Option<bool>` [P]. `Note`
   holds `callouts: Box<[Callout]>` [P]. The parser detects callout markers `>
   [!TYPE]` in blockquotes [V: src/note/GLOSSARY.md:100-104].
3. Slugs: Derive slugs with `slug_from_heading(text: &str) -> String` [P].
   Deduplicate duplicate headings with `-1`, `-2` suffixes by iterating headings
   in document order [V: src/note/GLOSSARY.md:50-51].

### Hidden costs

Adds 48 bytes (three `Box` pointers) to `Note` in memory [H]. Saves 31,433 slug
string allocations across 2,711 notes [V:
.scratch/arch-design/research/23-prep/g-probe-results.md:96].

### Edge cases

1. Duplicate heading text. Two headings have the text `# Summary`. On-demand
   derivation gives `summary` and `summary-1` in document order [V:
   src/note/GLOSSARY.md:50-51].
2. Unresolved footnote reference. A note references `[^1]` with no definition.
   The parser accepts the note without error and emits an advisory notice [P].

### Verdict

Needs change. Specify the exact struct shapes for footnotes and callouts in the
ticket resolution [P].

## Decision Q7: Parser notices channel

The parser returns `(Note, Vec<NoteNotice>)` [P]. `Note` does not store notices
[P]. Ticket 31 owns notice presentation [V:
issues/31-error-and-diagnostic-architecture.md:30-36].

### Conflicts with resolved tickets

Ticket 16 requires infallible parsing [V:
issues/16-grilling-compatibility-envelope.md:237]. Returning notices beside
`Note` preserves infallibility [V].

Ticket 19 requires a notice when `NaN` becomes `Null` [V:
issues/19-coercion-and-comparison-policy.md:90-91]. Q7 implements this
requirement [P].

### Assumptions in open tickets

Ticket 31 expects a notice channel for parser diagnostics [V:
issues/31-error-and-diagnostic-architecture.md:33-36]. Ticket 24 and Ticket 25
avoid storing repeated notice strings in redb [P].

### Hidden costs

Generating notices adds tiny memory overhead when notices occur [H]. Most notes
produce zero notices [H].

### Edge cases

1. Unsaved buffer diagnostics in LSP. An unsaved buffer with `NaN` returns a
   notice with a `ByteSpan` [P]. The LSP translates it immediately into an
   editor diagnostic [V:
   issues/34-live-buffer-and-persisted-index-consistency.md:21-23].
2. High volume of notices during index rebuild. Ticket 31 must throttle or group
   stderr notices during bulk indexing [P].

### Verdict

Robust.

## List of edge-case failures and smallest fixes

1. Failure: `QueryFieldValueRef::Tags` cannot borrow a slice from an
   accessor-derived view. Smallest fix: Keep `tags: Box<[Tag]>` stored on `Note`
   [P].
2. Failure: `note.get(key)` causes $O(N)$ linear scans across list items.
   Smallest fix: Cache the effective field map in memory or store item fields in
   `Note` [P].
3. Failure: Ticket 20 contradicts Q3 on who owns the field merge. Smallest fix:
   Amend Ticket 20 line 122 to assign the accessor to the note model [P].
4. Failure: Multi-value `List` breaks scalar equality queries (`key = "val"`).
   Smallest fix: Record the change in the change table and add list membership
   functions in Ticket 26 [P].
5. Failure: Deserialization fails when `Note` layout changes. Smallest fix: Bump
   `META_VERSION` in `src/index/store.rs:41` to invalidate old redb caches [P].
6. Failure: RAM exhaustion at 50,000 notes in long-lived hosts. Smallest fix:
   Implement lazy row decoding in Ticket 24 [P].

## Not checked

- Personal vaults with deep footnote trees.
- Performance of on-demand slug generation on queries filtering thousands of
  headings.
- Memory usage of callout title strings on large vaults.
