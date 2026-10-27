# Alternative designs and critique for ticket 23

This document analyses alternative designs for the note document model. It
deepens the decisions from round 1.

## 1. List items and tasks (Q2 and extension)

### 1.1 Context and type sketches

Today `ListItem` is 160 bytes and `TaskListItem` is 72 bytes [V:
`g-probe-results.md:44-46`]. `ListItemType::Task` contains `TaskListItem`
directly [V: `src/note/lists.rs:240`]. Because `ListItemType` holds
`TaskListItem` inline, every list item pays for the task fields [H]. The probe
found 76,728 list items and 3,785 tasks [V: `g-probe-results.md:96`]. Non-task
list items make 95.1% of all items [H]. They still occupy 160 bytes in memory
[H].

The user proposed to model tasks as a kind of list item that holds the shared
data [V: round 1]. We sketch a unified `ListItem` with a shared core and a kind
enum [P]:

```rust
pub struct ListItem {
    text: ListText,
    span: ByteSpan,
    line: SourceLine,
    parent: Option<SourceLine>,
    depth: u8,
    is_ordered: bool,
    fields: Option<Box<ListFieldMap>>,
    tags: Box<[Tag]>,
    kind: ListItemKind,
}

pub enum ListItemKind {
    Plain,
    Checkbox,
    Task(Box<TaskData>),
}

pub struct TaskData {
    status_symbol: TaskStatusSymbol,
    status_kind: TaskStatusType,
    dates: TaskDateSet,
    priority: Option<TaskPriority>,
    fully_complete: bool,
}
```

### 1.2 Placement of task data

Task lifecycle dates live in `TaskDateSet` [V: `src/task.rs:498-505`]. Task
priority lives in `Option<TaskPriority>` [V: `src/task.rs:959`]. Task status
lives in `status_symbol` and `status_kind` [P]. Under decision Q4, the task item
drops the owned `name: String` [V: round 1]. The display name resolves on demand
from `TaskStatusMap` [V: `src/task.rs:103-122`]. These fields move into
`TaskData` [P]. Boxing `TaskData` keeps `ListItemKind` at 16 bytes [H]. The
shared core adds `span: ByteSpan` (8 bytes) [V: `src/position.rs:132-138`].
`ListItem` shrinks from 160 bytes to 112 bytes [H]. This saves 3.68 megabytes
across 76,728 list items [H].

### 1.3 Reads by the query layer

The query layer reads both list items and tasks [V:
`src/query/results.rs:335-419`]. For list rows, it reads `clean_text`,
`raw_text`, `depth`, `line`, `parent`, `is_ordered`, `fields` and `tags` [V:
`src/query/results.rs:339-375`]. All these fields remain on the shared
`ListItem` core [P]. For task rows, `resolve_task_ref` reads status, status
type, symbol, completed, priority, dates, and fully complete [V:
`src/query/results.rs:388-418`]. `TaskData` supplies all these facts directly
[P]. The status name lookup passes `status_symbol` to `TaskStatusMap` [P].

### 1.4 Accessor performance on notes with many items

Decision Q2 removes the stored copy of item fields and item tags from note-level
fields [V: round 1]. The note accessor derives the view from `lists` on demand
[P]. An item row checks its own `item.fields` first [V:
`src/query/results.rs:324-331`]. It calls `note.get(key)` only as a fallback [V:
`src/query/results.rs:332`]. Only page-level queries need the derived note view
for item fields [H]. The average note has 28 list items [V:
`g-probe-results.md:96`]. Scanning 28 items takes less than 50 nanoseconds [H].
On an outlier note with 1,000 items, a linear scan checks 1,000 pointers [H].
This scan costs about 1.5 microseconds [H]. An in-memory index would add heap
allocations for every note [H]. The note accessor does not need an index of
items [P].

### 1.5 Verdict

Verdict: Adopt [P].
Strongest argument against proposal:
Boxing `TaskData` adds one extra heap allocation for each task item during the parse [H].

## 2. Footnotes and callouts in the model (Q6 and extension)

### 2.1 Pulldown-cmark 0.13.4 events

For footnotes, `pulldown-cmark` 0.13.4 emits dedicated events [V:
`pulldown-cmark-0.13.4/src/lib.rs:214,580`]:

- `Tag::FootnoteDefinition(CowStr<'a>)` marks the start of a definition block
  [V: `lib.rs:214`].
- `TagEnd::FootnoteDefinition` marks the end of a definition block [V:
  `lib.rs:396`].
- `Event::FootnoteReference(CowStr<'a>)` marks an inline reference [V:
  `lib.rs:580`]. The parser already turns on `Options::ENABLE_FOOTNOTES` [V:
  `src/note/parser.rs:70`].

For callouts, Obsidian uses blockquote syntax with a type marker [V:
`src/note/GLOSSARY.md:98-105`]: `> [!note]+ Title` or `> [!warning]- Title`.
`pulldown-cmark` emits `Tag::BlockQuote(Option<BlockQuoteKind>)` [V:
`lib.rs:182`]. `BlockQuoteKind` supports only five GitHub alert types under
`Options::ENABLE_GFM` [V: `lib.rs:135-141`]. Obsidian supports 13 canonical
types and custom types [V: `src/note/GLOSSARY.md:101-104`]. `pulldown-cmark`
does not parse the `+` or `-` fold state [V: `lib.rs:135-141`]. It does not
parse custom callout titles [V: `lib.rs:182`]. It emits the marker as plain text
inside the first paragraph of the blockquote [V: `lib.rs:182,536`]. The traces
parser must inspect the first line of each blockquote to extract callout data
[P].

### 2.2 Model representations

We sketch the model types for footnotes and callouts [P]:

```rust
pub struct FootnoteDefinition {
    label: CompactString,
    span: ByteSpan,
    line: SourceLine,
    content_span: ByteSpan,
}

pub struct FootnoteReference {
    label: CompactString,
    span: ByteSpan,
    line: SourceLine,
}

pub enum CalloutFold {
    None,
    Expanded,
    Collapsed,
}

pub struct Callout {
    kind: CompactString,
    title: Option<CompactString>,
    fold: CalloutFold,
    span: ByteSpan,
    line: SourceLine,
}
```

Most notes have zero footnotes and zero callouts [H]. To keep `Note` lean, we
wrap them in an optional structure [P]:

```rust
pub struct BodyExtras {
    footnote_definitions: Box<[FootnoteDefinition]>,
    footnote_references: Box<[FootnoteReference]>,
    callouts: Box<[Callout]>,
}
```

`Note` stores `extras: Option<Box<BodyExtras>>` [P]. This adds only 8 bytes to
`Note` when empty [H].

### 2.3 Consumers and cost

Three consumers need these facts:

1. LSP navigation: jump from reference to definition, show definition text on
   hover, and provide folding ranges for callouts [P].
2. Diagnostics (ticket 31): report undefined footnote references, duplicate
   definitions, and unused definitions [V: `b3-open-ticket-demands.md:53`].
3. Body schemas (ticket 36): check mandatory callouts and exclude footnote text
   from section token budgets [V: `b3-open-ticket-demands.md:58`].

Cost: Parsing callout headers needs a simple prefix check on blockquotes [P].
The storage cost is 8 bytes in memory for notes without extras [H]. Postcard
serializes an empty `Option` as one byte [H].

### 2.4 Verdict

Verdict: Adopt as option [P].
Strongest argument against proposal:
No production query or index feature reads footnotes or callouts today [V: `docs/digests/plans/arch-redesign-digest/research/01-verify-core-findings.md:215-216`].
Adding them now increases parse time before the LSP exists [H].

## 3. Effective field map comparisons (Q3)

### 3.1 Comparison of candidates

We compare three candidates for multi-value and metadata field merges:

1. Candidate A: Stored effective map.
The parser merges frontmatter and inline fields into one `IndexMap` [P].
It discards the source distinction [P].
Tradeoffs:
Query reads are fast [H].
However, schema validation in ticket 20 and ticket 35 cannot tell frontmatter from body fields [V: `20-schema-note-typing-and-validation.md:196-200`].
Diagnostics in ticket 31 cannot pinpoint the authored source line [V: `31-error-and-diagnostic-architecture.md:30-37`].
Storing both the effective map and the source maps would duplicate memory [H].

2. Candidate B: Two maps with an accessor (Q3 leaning proposal).
`Note` stores `frontmatter: Option<Frontmatter>` and `inline_fields` [V: `src/note/model.rs:27,31`].
An accessor merges the maps on demand [P].
Precedence: Frontmatter wins when a key appears in both [V: `src/note/model.rs:164-173`].
Multi-value keys: Several values always form a list [V: round 1].
Null keys: A frontmatter null masks an inline field of the same key [P].
Tag-like keys: Frontmatter `tags` populates `Note.tags` and stays in frontmatter fields [P].
Provenance remains intact for schema rules [H].

3. Candidate C: App-layer merge.
`Note` exposes raw maps without an accessor [P].
Each consumer merges fields on its own [P].
Tradeoffs:
The query engine, template engine, and LSP can implement different merge rules [H].
This breaks the Frozen compatibility guarantee of ticket 16 [V: `16-compatibility-and-support-tiers.md`].

### 3.2 Verdict

Verdict: Adopt candidate B (two maps with accessor) [P].
Strongest argument against proposal:
Iterating effective fields creates a dynamic iterator chain with hash checks on every call [H].

## 4. Representation split and spans (Q1)

### 4.1 Comparison of candidates

We compare four candidates for spans and persistence:

1. Candidate 1: Shared `Note` with stored spans (Q1 leaning proposal).
`Note` holds byte spans on headings, items, links, and extras [P].
`redb` stores the whole `Note` in the `NOTES` table [V: `src/index/tables.rs:26-28`].
Probe measurement: Spans add 4.3% bytes to the serialized `Note` [V: `g-probe-results.md:8-9,113`].
Decoding takes 6.1 microseconds per note [V: `g-probe-results.md:118`].
Startup time for 50,000 notes takes 0.31 seconds [H].

2. Candidate 2: Split types (`IndexNote` and `EditorNoteDocument`).
`IndexNote` stores no spans [P].
The editor builds `EditorNoteDocument` by reparsing open files [P].
Tradeoffs:
`IndexNote` saves 4.3% in postcard bytes [H].
However, a workspace query for all references must reparse unopened files [H].
Reparsing 1,000 files takes 0.22 seconds (1,000 times 217 microseconds) [H].
Also, the codebase must maintain two distinct note types [H].

3. Candidate 3: Stored spans in a compact side table.
`NOTES` stores span-free notes [P].
A second table `NOTE_SPANS` stores byte spans by path [P].
Tradeoffs:
The CLI skips `NOTE_SPANS` during metadata queries [H].
It saves 0.7 milliseconds across 2,711 notes [H].
However, the second table adds `redb` b-tree keys and page headers [H].
Total database size grows by about 5% [H].
The index must update two tables on every edit [H].

4. Candidate 4: Persisted facts with lazy item text.
`Note` stores metadata and spans, but omits raw and clean item text strings from storage [P].
Probe measurement: List item text is 93% of `ListItem` bytes and 72.2% of the entire database [V: `g-probe-results.md:10,89`].
The stored database shrinks from 15.89 megabytes to 4.42 megabytes (-72%) [H].
For 50,000 notes, the database shrinks from 293 megabytes to 82 megabytes [H].
Decode time for 50,000 notes shrinks from 0.31 seconds to 0.08 seconds [H].
Tradeoffs:
Any query that displays task text must open the source file from disk and slice the bytes [H].
Matching 500 tasks across 200 files needs 200 disk file reads [H].
This destroys cold CLI query speed [H].

### 4.2 Verdict

Verdict: Adopt candidate 1 (shared `Note` with stored spans) [P].
Strongest argument against proposal:
Persisting spans invalidates the stored row on any whitespace edit, even when semantic data does not change [H].

## 5. Five gaps in the seven round-1 decisions

### 5.1 Format version and configuration fingerprint

Adding spans, callouts, or changing `TaskStatus` alters postcard serialization bytes [V: `a2a-index-persistence.md:59-62`].
The format version `META_VERSION = b"1"` in `src/index/store.rs:38-41` must increase to `b"2"` [P].
Without this bump, opening an existing database causes decode failures or data corruption [V: `a2a-index-persistence.md:46-58`].
Decision Q1 to Q7 did not name this required change [H].

Verdict: Adopt version bump in ticket 23 resolution [P].
Strongest argument against proposal:
Increasing `META_VERSION` forces a complete reindex of the vault on the first run after upgrade [H].

### 5.2 Row identity for ticket 26

Ticket 26 needs a stable identity for list and task rows [V: `b3-open-ticket-demands.md:17-18,50`].
Today the row identity is `(file_index, item_idx)` [V: `src/query/results.rs:31-85`].
When a user edits an item above another item, `item_idx` shifts [H].
The editor cannot use `item_idx` to toggle a task status [H].
Ticket 23 must define an anchor using `(note_path, BytePos)` or `SourceLine` [P].

Verdict: Adopt stable position anchor for row identity [P].
Strongest argument against proposal:
Line and byte positions also shift when text changes earlier in the document [H].

### 5.3 Unsaved-buffer build for ticket 34

Ticket 34 needs to build the editor note from an in-memory buffer without disk access [V: `b3-open-ticket-demands.md:29-31`].
Today `MarkdownParserInput::for_test` takes in-memory text, but it is gated by `test-utils` [V: `src/lib.rs:559-561`].
The production constructor takes `&Path` and reads the file [V: `src/note/parser/input.rs`].
The model needs a public constructor `MarkdownParserInput::from_buffer(path, text)` [P].
The LSP needs an in-memory overlay that does not write to `redb` [P].

Verdict: Adopt in-memory buffer constructor in production [P].
Strongest argument against proposal:
An unsaved buffer note in memory can drift from persisted index state if not tracked carefully [H].

### 5.4 Host memory footprint for 50,000 notes

Decision Q1 assumes the LSP host loads all 50,000 notes into memory on startup [V: round 1].
The probe measures `size_of::<Note>()` at 232 bytes [V: `g-probe-results.md:44`].
In RAM, each note owns list items, headings, links, fields, tags, and heap strings [V: `src/note/model.rs:24-33`].
The average note consumes about 9 kilobytes of heap memory in RAM [H].
Holding 50,000 notes in RAM takes about 450 megabytes of memory [H].
Decision Q1 did not budget this memory usage [H].

Verdict: Adopt as option with a lazy eviction policy for large vaults [P].
Strongest argument against proposal:
A lazy eviction policy complicates the LSP memory model and reintroduces disk reads [H].

### 5.5 Section extents and preambles for ticket 36

Ticket 36 needs section boundaries and text slices for body schemas [V: `b3-open-ticket-demands.md:34-35,58`].
`Heading` stores `level`, `text`, `line`, and `span` [V: `src/note/heading.rs:10-14`].
It does not store the full section extent [V: `src/note/heading.rs:10-14`].
Also, notes often contain preamble text before the first heading [H].
The model must define whether `Heading` derives its section extent from the next heading, and how to address preambles [P].

Verdict: Adopt derived section extents and an explicit preamble span [P].
Strongest argument against proposal:
Storing section extents or preambles adds more fields to `Note` and `Heading` [H].

## Not checked

- Memory footprint measurement of 50,000 notes on a real machine.
- Benchmark of the parse time of callout prefix scanning across the corpus.
- Test of redb commit times with a separate NOTE_SPANS table.
- Test of how TaskData boxing affects cache misses during task queries.
