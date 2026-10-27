# Evidence for ticket 23 from tickets 20 to 22

This file lists what tickets 20, 21 and 22 decide about the note model. It also
reviews the four candidates of
`research/20-prep/h-arch-design-note-split-review.md`. Citations of the form
`issues/NN:LINE` come from the agent. I did not check every line number.
Citations of the form `23:NN` point to an earlier version of ticket 23 and are
stale.

## Summary

- [V] The schema check is pure and span-blind (`issues/20:52-56,76-79`). It
  takes a field map and returns violations without spans or severities.
- [V] Adapters attach spans after the check (`issues/20:104,158-160`). The
  ticket 23 text lists five needs (R1 to R5). They are the key span, the list
  element span, the heading extent, the inline field span and the link span.
- [V] Ticket 20 says the app layer owns any merge of inline fields into the
  field map. It also says ticket 23 handles it (`issues/20:122-123`).
- [V] Class binding uses the tags and the path. It never enters the index or its
  fingerprint (`issues/20:53-55,140-153`).
- [V] The note keeps no raw lexeme (`issues/20:284-294`). Consumers get a text
  view rendered from the typed value.
- [V] Path-derived facts stay out of `Note` (`issues/21:68-69,158-178`). These
  are the filename period, `file.cdate` and `file.mdate`.
- [V] `note` defines its own parser-input type and imports nothing from `config`
  (`issues/22:85-90`). `src/note/parser/input.rs:9` imports `FrontmatterConfig`
  and `TaskConfig` from `crate::config` today.
- [V] `index` owns `IndexPolicy`. `IndexPolicy::fingerprint()` is the only
  source of the fingerprint (`issues/22:94-107`).
- [V] Ticket 22 hands the final place of `TaskStatus*` and `TaskListItem` to
  ticket 23 (`issues/22:84,292-294`).
- [V] `TaskStatus` lives at the crate root (`src/task.rs:51-55`). It holds an
  owned `name: String`.

## Ticket 20: schema typing and validation

| Decision | Source | Concepts | Effect |
|----------|--------|----------|--------|
| `ResolvedFields::check(&FieldMap) -> Vec<Violation>`. No spans, no severity. | `issues/20:54-55,76-79` | C1, C2, C3 | BINDS. The check needs no span on the note. Spans matter only for an adapter such as the LSP. |
| Adapters need a span for each field key, and for each list element where known. | `issues/20:104,382-383` | C2, C3 | BINDS. Ticket 23 decides how the adapter gets the spans. |
| The check takes a field map. The app layer owns any merge of inline fields. Ticket 23 handles it. No merge is promised. | `issues/20:122-123` | C6 | BINDS. Ticket 23 decides if the note gives one effective field map and which source wins. |
| Class binding reads the tags and the path. | `issues/20:53-55,138-160` | C1, C8 | INFORMS. The parser needs no schema knowledge. It keeps raw tags and the path. |
| No parse-time override and no quote convention for date-shaped text. Consumers get a rendered text view with a tested round trip. | `issues/20:284-294` | C6 | BINDS. The `Note` keeps no raw lexeme. |

## Ticket 21: execution context

| Decision | Source | Concepts | Effect |
|----------|--------|----------|--------|
| Path-derived facts stay outside the note. `file.date` is the filename period. The code must not use `file.cdate` as a fallback. | `issues/21:68-69,158-165,245-248` | C8, C17 | BINDS. The `Note` is a document model. The filename period belongs to the file metadata or the query context. |
| Civil dates do not use the system zone. Resolve timestamp-derived dates during evaluation. | `issues/21:95-97,167-169` | C6, C8 | INFORMS. The parser needs no zone and no clock. |
| The index-freshness decision chooses to persist derived periods or to reparse file names. | `issues/21:176-178` | C8 | INFORMS. Ticket 25 decides. |

## Ticket 22: config ownership

| Decision | Source | Concepts | Effect |
|----------|--------|----------|--------|
| `note` owns its parser-input type. It holds the status map, the tag filters and the tags field name. The parser borrows it. Ticket 23 decides the final field set. | `issues/22:85-90` | C8 | BINDS. Ticket 23 decides the fields and the borrow or own choice. |
| Do not move `TaskStatus`. Ticket 23 decides the place of `TaskStatus*` and `TaskListItem`. | `issues/22:80-84,292-294` | C7 | BINDS. `TaskStatus` is in `src/task.rs:51-122`. `TaskListItem` is in `src/note/lists.rs:348`. |
| `index` owns `IndexPolicy`. Ticket 22 removes `title`, `aliases` and the date keys from the fingerprint. An index-time read must add its key. | `issues/22:94-107` | C8 | BINDS. `src/index/fingerprint.rs:158` fingerprinted these keys before. The parser reads only `tags_name()` today (`src/note/parser.rs:381`). |

- [H] The red-team notes in `research/22-prep/h-redteam-and-interface-check.md`
  (lines 76-89 and 381-391) warn against two moves. First, do not freeze a
  parser input with a lifetime `'a` before ticket 23 decides the status model.
  Second, do not make the parser borrow `&TaskStatusMap`. I did not read those
  lines.

## Review of the split candidates

The agent reviews `h-arch-design-note-split-review.md`, section 3.2 (lines
108-135). The verdicts below are the agent's. I did not check them against the
`h` file.

| Candidate | Description | Verdict | Reason |
|-----------|-------------|---------|--------|
| A | One shared `Note` with spans in the `NOTES` table. | Wrong [H] | Note deserialization is 97.3% of the sum of decoding and link resolution (18.48 ms and 0.52 ms for 5k notes, `research/11:91-95`). A whole path-set change takes 32 ms to 35 ms. Spans add bytes. The agent says it also mixes editor layout into stored data. |
| B | `NoteFacts` for the index and `NoteDocument` for the editor. | Partly overturned [H] | Ticket 20 says the validator takes a `&FieldMap`, not a `&NoteDocument`. The editor needs a document only for spans. The agent adds that two models need two hierarchies and projection code. |
| C | Lossless tree (Rowan) and facts. | Rightly rejected [H] | A Rowan tree is heavy and hard to persist (`research/10:53,98-99`). The Noyalib CST costs +59% parse time (`issues/18:201-224`). |
| D | Span-free `Note`. Reparse the buffer when the editor needs spans. | Undervalued [H] | The `h` file rejected D because it assumed the schema check works on raw text. Ticket 20 made the check take a field map. The agent says D is now a strong candidate. Taplo and rumdl use this pattern. |

- The probe changes the argument against A. Spans add 4.3% bytes on this corpus
  (`g-probe-results.md`). The list item text is 93% of the item bytes.

## Conflicts and handoffs

1. Persisted or transient spans. Ticket 17 rejected the zero-span design for the
   editor. Ticket 20 made the schema check span-blind. Does the index store any
   spans?
2. Borrowed or owned parser input. Ticket 22 says the parser borrows it. The
   red-team notes warn about lifetimes in async hosts (tickets 30 and 34).
3. Status interning and the status map. If the parser stores only the marker
   `char`, it needs no `TaskStatusMap` at parse time.

Exact handoff text:

- Ticket 20: "The check takes a field map. The app layer owns any merge of
  inline fields into that map; ticket 23 handles this, and no merge is promised
  here." (`issues/20:122-123`)
- Ticket 20: "Field-key spans for adapters; inline-field merge: note document
  model (ticket 23)." (`issues/20:382-383`)
- Ticket 21: "Keep path-derived facts, such as the filename period, out of
  `Note` (ticket 21)." (`issues/21:68-69`)
- Ticket 21: "Let the index-freshness decision choose whether to persist derived
  periods or reparse filenames. Do not store a zone for that reason."
  (`issues/21:176-178`)
- Ticket 22: "Ticket 23 decides the final place of `TaskStatus*` with
  `TaskListItem`." (`issues/22:84`)
- Ticket 22: "The parser-input fields are provisional. Decide the status model
  and the place of `TaskListItem`. Add each index-time read to `IndexPolicy`."
  (`issues/22:292-294`)

## Not checked

- [H] The cost of a reparse of a 500-line editor buffer on arm64 macOS.
- [H] The cost of the Noyalib `SpanTree`. `research/20-prep/brief.md:72-78` says
  Noyalib builds `(Value, SpanTree)` on every parse. `g-probe-results.md`
  measures `Spanned<Value>`.
- [H] Whether Dataview rolls up multi-valued inline fields as nested arrays or
  flattens them.

## Inconsistencies found while rewriting

- The first version says the review of candidate B is "partially overturned /
  refined" and the review of D is "ignored / undervalued (now strong
  contender)". These are the agent's opinions. They are not decisions. The user
  decides.
- The first version cites `issues/23:30-31,47-48,104,158-162`. These lines
  belong to an earlier version of the ticket.
- The first version says ticket 20 "imposes zero span overhead on ... persisted
  index facts". Ticket 20 says the check is span-blind. It does not say what the
  index stores.
- The first version tags the red-team claims [V]. I did not read those lines. I
  tagged them [H].
- The first version says the schema check does not need a `NoteDocument`.
  Adapters still need the spans. The claim is true only for the check.
- Ticket 20 says ticket 23 handles the merge. It also says the app layer owns
  the merge. The two statements conflict (`issues/20:122-123`).
