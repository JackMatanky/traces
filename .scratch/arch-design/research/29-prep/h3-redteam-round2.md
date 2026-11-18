# Red-team round 2 and audits: ticket 29

This file condenses the second-round evidence for ticket 29. It continues
[h2-redteam.md](./h2-redteam.md), the first round. The first round attacked
the h1 design. This round attacks the grilling decisions v2 and v3.

Tags: [V] means measured, run, or read in a cited source. [H] means an
inference. [P] means a proposed defense that the user adopted or rejected in
the grilling. Adopted rules below are [P] unless they carry an explicit evidence
mark.
The [V] marks retain the original agents' evidence provenance. They do not
report a new probe run.

## Sources

- `agent://RedTeam29`: probes in `.worktrees/rt29/probe29` (the worktree is
  removed; the source is in `/tmp/rt29-probe-src`). Release mode,
  median of 11 runs after 2 warmups, no RUSTFLAGS. The agent used the
  existing `target/release/traces` (built Nov 9) for the CLI probes.
- `agent://AuditSimplicity`: hostile audit, complexity lens. It read the
  v2 text. It ran no measurement.
- `agent://AuditUxConventions`: second-round audit, UX and performance lens.
  It ran the Nov 9 binary in a temp vault with a fresh HOME.
- `agent://AuditImplementability`: implementation map of the v3 text. It ran
  one clippy compile check in a scratch crate.
- `agent://GroupPrecedent`: survey of grouping in nine systems. It read
  primary documents and sources. It made no repo edit.
- `agent://GroupLocal`: local digests of Dataview and Tasks, plus the CLI and
  formatter code of Traces.
- `agent://NuRender` and `agent://NuPipeline`: Nushell `main` at `b4993b5b`
  (2026-11-14, workspace 0.117.1). They read source, tests and book. They ran
  no `nu` binary.

## 1. Range-key soundness (W29a)

Verdict: [V] the first-round rule was unsound as written. A numeric,
temporal or string range key can miss rows that the residual filter accepts.

| # | Predicate | Row the key misses | Evidence |
| --- | --- | --- | --- |
| 1 | `code == '012'` (quoted literal) | String `"012"`: ticket 19 raw-text match, while the literal also parses as Int 12 | [V] `tests/features/note/note-frontmatter.feature:51-64` pins this row as a match. A numeric key `[12,12]` misses it. |
| 2 | `rating == "5"` | String `"5"` rows (`str.md`, inline `"5"`) when the key is numeric. Int and Float rows when the key is string. | [V] binary: today `rating == "5"` returns exactly the String rows and not `rating: 5`. Ticket 19 adds the Int candidate, so a union of brackets is needed. |
| 3 | `due == "2026-07-29"` | Inline `due:: "2026-07-29"` is a String, and raw text matches. A Date key misses it. | [V] binary and ticket 19 lines 118-121 |
| 4 | `due == 2026-07-29` | Period-valued row `due: 2026-07` matches by membership (the point is inside the period), but starts at 07-01 | [V] ticket 19 lines 126-136 |
| 5 | Same | Civil `DateTime` at any time that day (a day literal covers the whole day), and instants whose local date in the pinned zone is that day | [V] ticket 19 |
| 6 | `project == "alpha"` | Link `[[Alpha]]` matches by folded `ComparisonKey`, while a String key is byte-exact | [V] ticket 19 lines 148-164 |
| 7 | `spent == 90m` | Duration rows spelled `1h 30m`. A text key on the spelling misses them. | [V] ticket 19 durations |
| 8 | `rating != 5`, `rating == null`, `not (rating < 5)` | Missing, null and other-bracket rows. A key holds only present values. | [V] binary: `rating != 5` returns 10 of 12 rows, `rating == null` returns 6 |
| 9 | `rating == 5.0` | Int 5 | [H] A split Int and Float key space misses one side. A lossy but monotone key needs closed probe bounds. Bounds alone do not prove Exact. |
| 10 | Any predicate, in Pages mode | Attachment rows such as `img.png`: `traces list` prints `- img.png`, and `matched_file_rows` iterates all entries | [V] binary, `src/query/service.rs:289-302` |

[H] Rows 1 to 7 and 9 rely on ticket 19 semantics. Ticket 19 is not implemented.
The author derived those rows from the spec text plus today's behavior.
Rows 2, 3, 8 and 10 also ran on the binary in today's form.

### Rules proposed by the red team (R1-R7)

Protocol definitions [P]:

Exact means that the candidate set equals the matching set for the predicate,
row mode and pinned frame.
Inexact means that the candidate set contains every matching row and can contain
extra rows.
Unsupported means that the planner cannot produce a proved candidate set.
A lossy encoding is Inexact unless a predicate-specific proof establishes
equality of the sets.
Closed bounds prevent omissions on monotone encodings. Closed bounds alone do
not establish Exact.
R6 may complement only a set that satisfies the Exact definition.

The grilling adopted these rules as the final protocol. The ticket Answer
states them in full.

- R1: A range key is Exact only with a matching-set equality proof and all five
  conditions below.
  The final Answer also requires the ticket 23 effective accessor.
  - The literal is an unquoted token of bracket B: bare Int or Float, bare
    Date or DateTime, or bare Duration. The grilling adds bare bool.
  - The operator is `==`, `<`, `<=`, `>` or `>=`.
  - The left side is a plain field path. It has no arithmetic, call or
    field-versus-field form.
  - The field is a note-level scalar on a Pages row.
  - The key covers all of bracket B, including period-valued rows.
- R2: Under ticket 19, across-bracket `==` is false and relational operators
  do not match. No other kind can match.
- R3: Any quoted literal is Unsupported and scans. A later optimization may
  probe a union of brackets.
- R4: `!=`, `null` and missing are never range keys.
  A `Not` needs the complete-set proof in R6; otherwise it scans.
- R5: Probe bounds are closed on lossy but monotone encodings.
  This prevents omissions, not extra candidates. Temporal probes are interval
  overlap, not point lookup.
- R6: `Not(Exact S)` is `Exact(U \ S)`. `U` is all `files` rows, attachments
  included. `Not(Inexact)` and `Not(Unsupported)` are Unsupported.
- R7: `And` intersects its supported children. It is Exact only if every
  child is Exact. `Or` is Unsupported if any branch is Unsupported.

Reading hazard for R6 [V]: if an implementer reads the first-round line "Not
over Exact = Inexact" as "same S, flagged Inexact", the rule is unsound. The
reason is that `not (rating > 5)` matches rows outside S.

Today Pages filters read frontmatter before inline fields [V].
Frontmatter wins: `rating: 3` plus `rating:: 9` shows `3` and does not match
`rating > 5`. The key writer must use the ticket 23 effective accessor.

[H] A closed lossy-key probe can contain extra rows.
For example, Int `9007199254740992` and Int `9007199254740993` share an f64 key.
A closed equality probe returns both. The residual equality accepts only the
first.
The probe is Inexact. Its complement would omit the second row under Not of that
equality.

### Plan-dependent diagnostics (new finding)

The ticket 19 bracket-mismatch notice reports rows that the filter
excluded. A pruned run never evaluates string-valued rows. The notice would
appear only on scans [V]. The red team proposed per-field bracket counts in
the facet, so the notice does not depend on the plan [P]. The grilling
adopted this proposal and placed the counts in the `meta` facet.

### First-round defenses that this round changed

| First-round item (h2-redteam.md) | Round-2 result |
| --- | --- |
| Restrict range keys to note-mode rows | Too coarse. `file.*` fields have no item override (`src/query/results.rs:230-311`), so keys on `file.folder`, `file.path` and `file.name` may prune Task and List rows. Metadata and `tags` cannot (`results.rs:313-326`). Filename-derived `file.date` keys stay Unsupported until ticket 30 defines the filename-period parser. `file.day` is retired. |
| `Not` above an Exact atom is Inexact | Replaced by R6: `Not(Exact S)` is `Exact(U \ S)`, and `Not(Inexact)` is Unsupported. |
| Reject NUL in text values at admission | Moot at first: the first implementation builds no text range table. |
| Normalize `-0.0` at write time | Kept: normalize `-0.0` at key build. NaN is Null and has no key. |
| Advisory notice when membership excludes Null-date rows | Not adopted here. `member_of` is an IR name only. Tickets 30 and 34 decide the notice. |
| Task-row GroupBy equivalence: scope the claim or list it for ticket 36 | Listed for ticket 36 (the grilling's task-row group equivalence item). |

## 2. Sort comparator and group equality (W29b)

Verdict: partly confirmed. Today's `SortKey::cmp` is a total preorder [V]
(`src/query/sort.rs:405-419`). In 2M random triples (Int and Float near 2^53
and 2^63, signed zero, infinity) it had 0 transitivity violations.
Today's comparator is still wrong as a grouping equivalence, in six ways.

| Today it merges | Evidence |
| --- | --- |
| Int above 2^53 (the value is already lossy at the note boundary) | [V] binary: `big: ...993` renders as `...992` and ties with `...992` |
| Link with String, through `Text(link.target())` | `sort.rs:363-365` |
| List, Object, Tags, Inlinks and missing into one Null (`group_by("tags")` is a no-op today) | `sort.rs:346-367` |
| Date-shaped or duration-shaped Strings with real Dates or Durations, through `TextShape::classify` | `sort.rs:374-380` |
| Date at ambient-zone midnight with DateTime | `sort.rs:359-360` |
| `-0.0` with `0.0` (handled by `normalize_zero`) | `sort.rs:357,412` |

Ticket 19 changes the value and comparison rules. These projections cannot
define the new grouping relation.
The replacement comparator has separate risks.

### Comparator probes (`probe29 cmp` and `probe29 temporal`)

| Comparator | Result |
| --- | --- |
| Naive: Int-Int exact, Int-Float through `as f64` | 563 violating triples per 2M. Hand case: Int(2^53+1) = Float(2^53), Float(2^53) = Int(2^53), but Int(2^53+1) > Int(2^53). |
| Exact number line | 0 violations |
| Naive, sorted 3000 values x 300 runs | 300 of 300 outputs not non-decreasing |
| Naive, split into runs on 20,000 values | 315 groups against 29 true classes |
| Today (f64-only) | 20 groups, merging 9 classes |
| Civil `DateTime` against instant, one mixed comparator, `America/New_York` 2026-03-08 | Cycle: civil 02:30 < civil 03:00 < instant 07:15Z < civil 02:30 |
| Same mixed comparator, `sort_by` over 200 x 2000 values | 15 panics and 45 order violations |
| Same values mapped to one instant line, then tie-broken by kind and fields | Total, no cycle |

- Fix proposed: project every temporal value to one key once, with the
  pinned zone. Compare only keys. Never compare raw civil and instant values.
- The grilling decided a variant: two lines, civil `(date, time)` and instant
  epoch nanoseconds. The two classes stay apart (AuditImplementability B18).
- Link mismatch: ticket 19 sorts Links by collator on the target only. Link
  equality uses the folded target plus subpath. `[[A#x]]` and `[[A#y]]` would
  merge into one group while `==` says they differ.
- Strings: collator Equal plus the byte tiebreak is identical to byte
  equality. NFC and NFD spellings, and `a10` against `a010`, land in
  different groups. This agrees with `==`. The first-round "Equal" is safe.

### Invariant tests that the Answer needs

The test pool holds these values:

- Int and Float near 2^53 and 2^63, signed zero and infinity.
- Date, civil DateTime, and instants across a gap and a fold.
- Duration (`1mo` and `30d`) and Link (case, subpath).
- Strings (NFC and NFD, digits), Bool and Null.

1. Transitivity of `<=` and `~` on all triples, and `sort_by` never panics.
2. Group equality is equality of canonical `GroupKey` values. It is reflexive,
   symmetric and transitive.
   Scalar filter equality agrees within each grouping domain.
   Date and civil-midnight DateTime share a key when their civil values agree
   (`19:137`).
   Civil values and instants remain separate group classes, even when a filter
   relates them through the pinned zone.
   Membership matching never defines group equality. Test these exceptions
   separately from scalar equality.
   The behavior for period, object and nested-list group values stays open.
3. Equivalence classes are contiguous after sorting.
4. A mixed Int and Float test above 2^53 and 2^63.
5. `-0.0` is normalized at key build.
6. No `i64 as f64` in any comparison.

## 3. Fan-out, `--count` and stderr (W29c)

Verdict: confirmed. The definitions need to be exact.

- R is the rows after filter, sort and limit. Each row has one pre-group
  ordinal. D is the number of distinct rows in R.
- Fan-out membership M is the sum of the key counts per row. An empty list
  counts as one null membership. M is at least D.
- Group sizes sum to M, not D. `limit(n)` before grouping yields at most n
  distinct rows but up to M memberships.
- The first-round stderr summary is `D page(s) in G group(s)`.
  The final summary adds `from ROOT` (section 11b).
- `--count` with `--group-by` prints per-group lines. Without `--group-by` it
  stays a single number, which is frozen.
- `list` and `table` have no `--count` today [V]. Only `task` has it, and the
  stderr summary is off with it (`src/cli/task.rs:195`).
- The CLI has no `--limit` [V, grep]. Limit before grouping affects templates
  only.

Weak point in the first-round row identity [V]: `(note position, item_idx)`
collides after `flatten`. `flatten("tags")` on one note yields two rows with
the same `(position, kind)` and different `flattened` values
(`src/query/results.rs:36-43`, `src/query/plan.rs:255-272`). `QueryRow::eq`
also deep-compares the whole `FileEntry` (`results.rs:428-434`). Use the
pre-group ordinal for distinct counts and dedup. Add a flatten ordinal.

`--count` line format: `key: n` is not safe for keys that hold a newline or
`: `. Escape control characters in count lines. The markdown heading escape
does not apply to non-markdown output [H].

## 4. Uses of `group_by` (W29d)

Verdict: confirmed. The red team found every use below.

| Where | Detail |
| --- | --- |
| `src/query/plan.rs:146,212-221,250-254` | `GroupBy` variant and sort alias |
| `src/query/results.rs:599-608` | `QuerySet::group_by` |
| `src/query/results.rs:1250-1311` | three tests: `clusters_records_with_equal_values_in_ascending_key_order`, durations clustering, `rejects_malformed_field_path` |
| `src/template/engine/query.rs:30-33,325-338,422-425` | docs and dispatch |
| `src/template/engine/query.rs:928-941` | test `group_by_clusters_equal_values_ascending`, expects `"b a "` |
| `src/query/GLOSSARY.md:65`, `docs/adr/0005-template-query.md:67` | docs |
| `tests/baselines/pub-tokens.txt:1020,1046` | public-API baseline |
| `docs/traces-digest.txt`, `docs/traces-query-digest.txt`, `.scratch/**` | generated and design docs, stale |

- No `.feature` file, e2e test, example, bench or `docs/vault` template uses
  `group_by`. The search used `gitignore:false`.
- The user's own vault templates sit outside the repo. Nobody can search them
  [H].

What breaks in a template (minijinja 3.0.1 probe with a group-shaped value):

- `{{ n.file.name }}` fails loudly with `undefined value`. `| default("?")`
  does not help.
- `{{ n.file }}` prints empty. `{% if n.file %}` is false.
- `{{ g | length }}` changes from row count to group count.
- A terminal such as `.group_by(x).list(...)` changes shape.

Least-breaking migration: old `group_by(x)` equals `sort(x, false)`.
Templates default `sort` to descending [V]
(`src/template/engine/query.rs:411-415`).
The migration must pass `false` explicitly. The change row says so.

Typed error against group-level limit: the red team judged a typed error on
`.limit`, `.sort`, `.where` or `.count` of a grouped set best. The message
names the replacement. Group-level operations would be new surface with no
demonstrated user. Add `take_groups(n)` only when a template needs it. Give
`GroupedQuerySet` a `len` that equals the group count. Define `.row_count`
(distinct rows) and `.entry_count` (memberships).

## 5. Group headings (W29e)

Verdict: confirmed. A `##` prefix plus the raw key breaks on keys that hold
a newline, a trailing `#`, markup, HTML, entities or `{#id}`. The test used
pulldown-cmark 0.13 with GFM, math, heading attributes and wikilinks.

| Form | Fuzz failures, 400k keys |
| --- | --- |
| naive | 64,587 |
| fold line breaks and trim | 20,922 |
| fold plus minimal escape | 0 |
| escape all ASCII punctuation | 0 |

Naive failures include `x\n# injected` (a second heading), `x\n\n- [ ] fake
task` (a list), `foo ###` (the closing sequence strips the text), `"#"` (an
empty heading) and `Title\n===`.

Safest form: `##` plus a space plus `heading_escape(display(key))`. The
function:

1. Replaces `\n \r U+2028 U+2029 U+0085 \v \f` with a space and trims.
2. Backslash-escapes `` \ ` * _ [ ] < > & ~ $ { } ^ ! ``.
3. Escapes a trailing run of `#` when it comes after a space or starts the
   text.

Do not escape `- + = |`. Dates, tags and paths then read cleanly:
`2026-07-29` and `#book/fiction` stay unchanged. Add the fuzz property test.

Display and labels:

- Show a canonical class spelling, not "first row's key". The first-round
  "group shows first row's key" depends on tie order, and tie order depends
  on file path. Renaming a file changes the heading (`big1` against `big2`
  today) [V].
- Use the byte-minimal member for Links and strings. Use the normalized decimal
  for numbers (`5`, not `5.0`), ISO for dates, and canonical form for
  durations.
- The null label must not collide with a real key. Use one fixed label. Prefix
  a real key that equals it with `\`.
- The heading level has no conflict if the output stands alone. A template owns
  the
  case where output goes under a note heading.

## 6. Default order (W29f)

Verdict: confirmed, and fixable without a change row.

Today's order is `Path::cmp`, component by component [V]. The store scan is
raw bytes but `paths.sort()` re-sorts it (`src/index/store.rs:1168`).
`SortedByPath` uses `Path::cmp` (`src/index/sort.rs`). `cmp_document_order`
uses only the position. A unit test pins the order: `index/service.rs:488-505`
expects `["notes/note.md", "notes-x.md"]`.

Observed order from `traces list`, `task` and `table`. The three commands
agree. `--sort title` with all ties also keeps document order.

| Position | Today (Path order, observed) | Plain byte order |
| --- | --- | --- |
| 1 | B.md | B.md |
| 2 | Z.md | Z.md |
| 3 | _u.md | _u.md |
| 4 | a.md | a.md |
| 5 | notes/a.md | notes b.md |
| 6 | notes/b.md | notes!.md |
| 7 | notes b.md | notes-x.md |
| 8 | notes!.md | notes.d/q.md |
| 9 | notes-x.md | notes.md |
| 10 | notes.d/q.md | notes/a.md |
| 11 | notes.md | notes/b.md |
| 12 | notes0.md | notes0.md |
| 13 | ä.md | ä.md |

- 8 of 13 rows move, in all three commands [V].
- Uppercase before lowercase, Unicode and underscore are identical in both
  orders. Only separators against bytes below `/` differ. Those bytes are
  space ! " # $ % & ' ( ) * + , - and `.`.
- Query results come out in the order of the sorted source. Stable ties and
  TopK tie-breaks use input order. A re-sort at the presentation edge is not
  enough.
- A byte-order change is an observable stdout change. Under ticket 16, an
  observable change that is not in the change table is a regression.
- On APFS the `Ä.md` and `ä.md` pair collide (case-insensitive). The vault has
  13 files.

Cheapest fix: keep the byte compare, and make the key the path with `/`
replaced by `0x00`. `0x00` cannot occur in a file name.

- Probe: 3M random pairs, 0 mismatches against `Path::cmp` [V]. Plain UTF-8
  byte compare mismatched on 73,891 pairs.
- The folder range becomes `[prefix\0, prefix\x01)`.
- Store order, document order and Path order coincide. No change row and no
  re-sort are needed.

Sort cost, 50,000 realistic paths (median of 11) [V]:

| Method | ms |
| --- | --- |
| `Vec<PathBuf>.sort()` from random order | 31.6 |
| `Vec<String>.sort()` (byte) | 4.75 |
| map `/` to `0x00`, then sort `Vec<Vec<u8>>` | 4.54 |
| `Vec<PathBuf>.sort()` when already byte-sorted | 1.85 |

The random realistic set had 0 positions that differ. A difference needs a
sibling file name that starts with a directory name plus a byte below `/`, as
in the 13-file vault above.

The UX audit re-measured this at 50k (median of 9, 1 to 4 levels deep, a mix
of `-`, `.`, space and `_`) [V]:

| Case | Measured | First claim |
| --- | --- | --- |
| `Vec<PathBuf>` sort | 31-32 ms at both opt levels | 31.6 ms |
| Sort of pre-built NUL keys | 4.75 ms at opt-level z, 3.4 ms at opt-level 3 | 4.5 ms |
| Key build plus sort at opt-level z | 13.5 ms | not stated |
| `Vec<String>` sorted with a comparator that maps `/` to 0 per byte | 10 ms at opt-level 3, 22 ms at opt-level z with a naive iterator comparator | not stated |

Order matched `Path::cmp` in every case (0 differing positions). The claim
of 4.5 ms is the pre-built key case. A String-backed `Ord` is the comparator
row. The ratio against `Path::cmp` is 1.4x to 3x, not 7x. The grilling kept
the rule and recorded both costs.

## 7. Spans and grammar lowering (W29g and Q4, Q15)

Verdict: confirmed, partly open.

Spans today [V]:

- Spans exist only inside `QuerySyntaxError`. It stores the full input string
  and the span (`src/query/error.rs:110-123`). The parser creates it.
- The evaluator has no spans. `FieldPathError` carries no span.
- A bad field path in a filter gives an error without a span. Lexical errors
  show a span in miette.

Needed after lowering:

- An invalid date-shaped quoted literal (a build-time advisory).
- A bracket mismatch.
- A NaN-to-Null notice.

Each of these notices comes after the parse. Give each IR atom
`Option<Span>` plus a clause id. Repeatable `--where` fuses into one `And`, so
an input string id is needed. Typed builder calls carry `None`.

A real bug that a typed builder removes [V]: `traces task --status é` and
`--status ✓` return 0 tasks while `traces task` lists them.
`--where 'list.status_symbol == "✓"'` finds them. `char::escape_default`
produces `\u{e9}`, and the DSL does not decode it (`src/cli/task.rs:46`).

Hidden differences between source text and filter text [V]:

- Filter tokens parse bare words as `f64` or Duration first.
  `--where 'file.name == inf'` (and `nan`, `infinity`) fails with a lex error.
  `--from inf.md`, `--from 1d` and `--from nan` work as paths.
- `file.name == "1d"` returns 0 for `1d.md` today. The quoted literal becomes
  a Duration. Ticket 19's raw-text rule for String rows fixes this.
- Source atoms resolve a `Class` atom when `resolve_classes` changes its
  `ClassExpansionMode` set after the parse. `with_children` changes it in
  templates (`src/template/engine/query.rs:500-537`). The IR must keep names
  and depth immutable and hold the resolved set elsewhere.
- A `Path` atom is a case-sensitive regex glob over `display_portable`.
  `file.path ==` is byte equality. The planner needs a separate candidate rule
  for each.

One IR is not one predicate. The source predicate runs first. Each filter
stage sits at its own plan position. The proof obligation:

- A Filter may move before a Sort or another Filter.
- A Filter never moves before a Limit.
- A Filter moves before a Flatten only if it does not read the flattened path.
- TopK moves only under the conditions in section 9, item 9.

## 8. Measurements

### 8a. Grouping one key, 50,000 rows (ms) [V]

Each variant includes key projection as `Cow<str>`. The second sort term is a
numeric rating. Variants:

- alias-sort: today's alias, a single sort.
- sort+split: the first-round composite sort plus a run split.
- hash-group: sort by `--sort` only, hash on the canonical key, then sort
  the G distinct keys.

All three variants produced identical groups.

| Key | Groups | alias-sort | sort+split | hash-group |
| --- | --- | --- | --- | --- |
| number | 10 | 1.35 | 5.09 | 2.94 |
| number | 100 | 1.00 | 5.07 | 5.04 |
| number | 5,000 | 2.30 | 5.11 | 4.10 |
| number | 50,000 | 2.46 | 4.64 | 6.71 |
| text, bytes | 10 | 1.63 | 6.97 | 2.03 |
| text, bytes | 100 | 2.63 | 7.13 | 2.18 |
| text, bytes | 5,000 | 5.31 | 6.95 | 3.64 |
| text, bytes | 50,000 | 5.51 | 7.49 | 8.43 |
| text, ICU | 10 | 34.71 | 50.26 | 2.00 |
| text, ICU | 100 | 73.96 | 90.84 | 2.34 |
| text, ICU | 5,000 | 165.19 | 174.16 | 18.77 |
| text, ICU | 50,000 | 200.94 | 210.67 | 132.19 |

- Sort+split costs 1.2x to 5x the alias for numbers and bytes. It costs 1.2x
  to 1.5x on top of an ICU sort that already takes 35 to 200 ms.
- With a collator, sort+split pays ICU comparisons across the whole n log n
  sort. Hash-group pays them only for the G distinct keys. That is about 17x
  cheaper at 100 groups.
- The first-round "no hash type" costs 50 to 200 ms at 50k with text keys.
- A hash on a canonical key is safe only if the canonical form equals the
  invariant in section 2.
- The red team recommended sort-then-split for numbers and temporal keys and
  hash-group for text and Link keys. The grilling chose one code path:
  hash-group for every key type (see section 13).

### 8b. Grouped markdown render, 100 groups x 500 rows, 4 columns [V]

| Variant | Median ms | Peak heap MiB | Allocated MiB | Stdout KiB |
| --- | --- | --- | --- | --- |
| per-group widths (`render_table` x100) | 21.54 | 8.59 | 17.11 | 4,425 |
| shared widths, cells materialized | 21.00 | 17.10 | 17.10 | 4,609 |
| shared widths, two-pass, nothing stored | 27.23 | 8.50 | 14.25 | 4,609 |
| with one 184-char outlier title in group 7: per-group | 21.28 | 8.59 | 17.11 | 4,503 |
| with the outlier: shared, materialized | 25.66 | 25.60 | 25.60 | 12,208 |
| with the outlier: shared, two-pass | 33.37 | 17.00 | 22.75 | 12,208 |

- Shared widths cost 2x peak heap on typical data, and 3x with an outlier.
- With the outlier, stdout grows 2.7x. All 50,000 rows pad to the width of
  the outlier.
- Markdown viewers align tables themselves.
- Use the per-group widths. Stream the groups to the renderer.
  Its grid then holds one group, not all groups.
  This bounds incremental render-grid memory, not memory for hash grouping or
  retained rows.
  The "typed rows to plain cell grid" step must take an iterator.
- AuditImplementability adds: flat tables need two passes for global widths,
  so streaming applies to grouped output only. The empty table form `"||\n"`
  (`src/query/format.rs:126`) needs a grouped equivalent.

## 9. More weaknesses, ranked by cost of being wrong

1. A mixed civil and instant comparator is not transitive. `sort_by`
   panicked in 15 of 200 runs (section 2). Project temporal values to one key
   per request.
2. Row identity collides after `flatten`. Distinct counts and dedup are wrong,
   and `QueryRow::eq` deep-compares the entry. Use a pre-group ordinal.
3. "Not over Exact = Inexact" is ambiguous. The universe must be all `files`
   rows, attachments included. `Not(Inexact)` is Unsupported.
4. Plain byte order changes frozen stdout and tie order. Use the `0x00` key
   (section 6) or add a change-table row.
5. Quoted literals make range keys unsound. A repo-pinned counterexample
   exists in `tests/features/note/note-frontmatter.feature:51-64`.
6. Bracket-mismatch notices depend on the plan. Count them from per-field
   bracket counts in the facet.
7. The Link group relation is wrong. Sort by `ComparisonKey` first, with the
   collator and the raw tiebreak after it.
8. Collator cost under sort+split is 35 to 200 ms at 50k rows (section 8a).
9. TopK by index is sound only if three conditions hold:
   - The key encoding order equals the comparator order, including
     cross-kind rank and null placement.
   - The descending ties keep stage input order. The executor uses a stable sort
     with a reversed compare (`src/query/sort.rs:115-117`). A reverse index
     scan would reverse ties.
   - The string keys use collation, but a byte-ordered key does not.

   The safest rule: TopK pushdown for numeric and temporal keys only at first.
10. "Task rows never prune from note keys" is too cautious. `file.*` fields
    have no item override (`src/query/results.rs:230-311`). `file.folder`,
    `file.path` and `file.name` can prune Task and List rows. Metadata and
    `tags` cannot (`results.rs:193-199,313-326`). Filename-derived `file.date`
    keys stay Unsupported
    until ticket 30 defines the filename-period parser. `file.day` is retired.
11. Flat task output indents by raw item depth. `traces task --todo` with a
    done parent prints an orphaned child at depth 1 [V, binary]. Sorting does
    the same. The first-round decision fixed depth only inside groups, so
    flat and grouped output would disagree. Apply nearest-ancestor-in-output
    to both, with a change-table row.
12. Group direction. The CLI sort defaults to descending, and `--asc` and
    `--desc` need `--sort` (`src/cli/mod.rs:294-302`). The first-round rule
    "group order follows `--asc` only if the first sort key is the group
    field" is hard to explain. Reuse the `+` and `-` prefix grammar of
    `SortOrder::parse` for `--group-by -rating`.
13. Lazy errors through minijinja. `Object::get_value` returns `Option` and
    `enumerate` returns an `Enumerator`
    (`src/template/engine/query.rs:316-323`).
    Neither can fail. Ticket 21's "memoized error, not silently no rows" has
    no channel for `GroupedQuerySet`. Handoff to ticket 31.
14. Date and midnight civil DateTime are `==`-equal and group together.
    Temporal group classes are civil and instant, never merged across the two.
15. Pinned triple memo. Today `QuerySet` memoizes per instance.
    `GroupedQuerySet` is a target type, not current code [P].
    `#[derive(Clone)]` copies the `OnceLock` state, and `push` resets it, so
    sibling chains recompute the plan. Put the memo behind a shared `Arc`
    keyed by plan.
16. Handoffs. Ticket 30's `today` and `eom` must fold into key bounds under
    the pinned frame at bind time. The plan memo must key on that frame.
    Ticket 34 notices need the clause id and atom span (section 7).

## 10. Not verified by the red team

- Ticket 19 semantics have a check against the spec text plus today's
  behavior only. They are not implemented. Cases in section 1 that rely on
  adaptive literals or periods come from the spec and have no run.
- Section 8 and the section 2 probes use replica comparators, not the real
  `SortKey`. They use real jiff and real `icu_collator`, with the
  numeric-ordering and tertiary-strength settings from ticket 19.
- The minijinja probe used a map-shaped group, not the real
  `GroupedQuerySet` object.
- The Windows `Path::cmp` ordering (UTF-16 compare) has no check.
- The agent left the worktree `.worktrees/rt29` in place. It holds one
  untracked directory, `probe29/`, and about 82 MB of build output. A repo
  rule needs user approval before removal. The command is
  `git worktree remove --force .worktrees/rt29`. The `/tmp` vaults are
  deleted.

## 11. Audits: the ticket 29 parts

Three audits read the v2 and v3 decision sets. Each audit judged the
ticket 29 decisions. The table lists each verdict and the grilling result.

### 11a. AuditSimplicity (complexity and consistency lens, v2 text)

The audit read `local://grill-decisions-v2.md`. It ran no measurement. It did
not re-read the red-team reports. Its lines 19-20 were cut, so it did not
check the GroupBy tail or the pushdown tail [V].

| Item | Audit verdict | Grilling result |
| --- | --- | --- |
| 13. Two grammars lower to one IR; builder is core; spans on atoms | KEEP. The `traces task --status é` bug is real. Serde deferral is correct. | Kept. |
| 14. Filter move rules (never before Limit; before Flatten only if the path is not read) | KEEP. | Kept. |
| 15. Exact, Inexact, Unsupported with the residual always run | SIMPLIFY. Ship Exact for equality on a scalar note-level field. Defer ranges, Not, Or and temporal probes. Compute the bracket-mismatch notice in the residual pass. | Partly adopted. The first scope is range keys for `==` and the four relational operators on unquoted numeric, temporal and bool literals, plus And-intersection and source-plus-filter. The full protocol stays in the Answer. Bracket counts stay in the `meta` facet. |
| 16. Distinct `GroupedQuerySet` | KEEP the type. Typed errors for misuse are a mild overclaim, because a type without the methods removes the need. The template adapter is an open hole. | Kept. The Rust type has no `sort`, `limit`, `where` or `count` method. The template object returns a typed error, because templates dispatch dynamically (`src/template/engine/query.rs:402-433`). The adapter goes to ticket 31. |
| 17. Canonical key; sort-then-split for numbers and temporal; hash-group for text and Link | SIMPLIFY to one path: hash-group on a canonical key, then sort the distinct keys. Reuse the filter's own equality as the group key. | Adopted: one grouping code path for every key type. Group equality agrees with filter `==` within each grouping domain. |
| 18. Output module, one renderer, heading escape, fuzz test | SIMPLIFY. One renderer. One escape function. The fuzz test and the width comparison are over-built. Choose per-group widths. | Per-group widths and one renderer adopted. The heading property test stays. |
| 19. Task depth from the nearest ancestor in the same list | KEEP. It touches frozen stdout, so the change row is necessary. | Kept for flat and grouped output (change row 12). |
| 20. `member_of` predicate | DEFER. Do not land IR support without a frontend. | Adopted in part: `member_of` is reserved as an IR node name only. No frontend and no implementation. |
| 21. Memo keyed by (plan, frame, generation triple) behind a shared `Arc` | SIMPLIFY. Key on the base id only. | Not adopted. The triple stays. |

Contradictions the audit found for ticket 29 [V]:

- Group equality against ticket 19. Grouping dates by instant contradicts
  `19:139`, where period equality is the `(start, unit)` identity. The audit
  said the group key should reuse the filter's own equality. The grilling
  settled this with two temporal key lines and the rule that a period is never
  a key.
- Template `group_by` against ticket 16 line 89 ("capabilities must stay
  reachable"). The template adapter that flattens a grouped set to `{key,
  rows}` is an open hole. The Answer commits to the adapter and hands it to
  ticket 31.

Cost without user-visible benefit, as the audit listed it for ticket 29:

- The full Exact and Inexact algebra for Not, Or and temporal pushdown.
- The two grouping paths.
- Typed errors for grouped-set misuse.
- The fuzz test and the shared-width comparison.

Holes an implementer would meet [V]:

- The template adapter for grouped results (ticket 31).
- A bracket-mismatch notice that depends on facet counts. The audit found no
  rebuild trigger after an epoch wipe.

Minimal decision set that the audit proposed for ticket 29:

- One IR with two frontends. The builder is the core. Atoms carry spans.
- Exact equality only for pushdown.
- A one-key `GroupedQuerySet` with hash-group.
- One renderer with per-group widths.
- The task-depth fix. `member_of` deferred.

Cheap to defer: multi-key group-by, `member_of`, serde form of the IR,
Range, Not and Or pushdown. Expensive to retrofit: group equality that uses
the filter's equality (stdout grouping is frozen) and the template group
adapter.

### 11b. AuditUxConventions (UX, conventions and performance lens)

The audit ran the Nov 9 `target/release/traces` in a temp vault with a fresh
HOME [V]. It measured these facts about today's CLI:

- `list` and `table` print rows on stdout and `N page(s) from ROOT` on
  stderr. `task` prints `N task(s) from ROOT` on stderr.
- `--count` exists only on `task`. It prints a bare number and suppresses the
  stderr summary. `list --count` and `table --count` fail with a clap error,
  exit 2.
- `--sort` defaults to descending. `--asc` and `--desc` need `--sort`.
- `--sort nosuch` exits 0 and does nothing.
- A `--where` syntax error exits 1 with a miette span and a `help:` line.
- An empty result exits 0.
- The source emits no notice outside the stderr count lines. `grep` finds no
  "warning:" in `src/`. Ticket 34 is open.

| Decision | Audit verdict | Grilling result |
| --- | --- | --- |
| Two grammars to one IR; spans plus clause id | KEEP. | Kept. |
| Pushdown with Exact, Inexact, Unsupported | KEEP. The red-team rules are tighter than v1. Quoted literals as Unsupported costs speed only. | Kept. First scope narrowed (override 1). |
| `GroupedQuerySet` distinct type; typed error | KEEP. The error text names the replacement. | Kept. |
| Canonical key and group equality | KEEP. The cycle and panic evidence is strong. | Kept. |
| Sort-then-split for numbers; hash-group for text and Link | KEEP (replica comparators, [H]). | Replaced by one hash-group path. |
| `--group-by FIELD` and `-FIELD`, no `--group-desc` | SIMPLIFY. Define the bare direction. Option: same default as `--sort`, with the prefix as an override. | Bare `--group-by FIELD` is ascending. The `-` prefix is descending. The help text states the asymmetry. `--sort` orders rows inside each group. |
| Heading per group, then the existing renderer | KEEP for terminals. Headings suit `grep` and `awk` badly. Machine users use `--count`. State the fixed `##` level. | Kept. The help text says the heading can collide under a note heading. No `--format` flag yet. |
| `--count` prints a line per group | REVISIT `key: n`. Use `n<TAB>key`. Escape TAB, newline and backslash. Document the null label. | Adopted. `--count` is new on `list` and `table`. The help text says the form changes with `--group-by`. |
| stderr `D page(s) in G group(s)` | SIMPLIFY. Keep `from ROOT`. Add `(M listings)` only when M is not D. | Adopted. |
| Fan-out; empty list to a null group | KEEP. | Kept. |
| Fixed null label; real key prefixed with a backslash | KEEP. Name the label in the help text. | Label is `(none)`. |
| Output module; per-group widths; streamed groups | KEEP. | Kept. |
| Task depth from the nearest ancestor in the output list | KEEP. The change row is needed. | Kept for flat and grouped output. |
| `member_of` is never a key | KEEP. | Kept. |
| Pinned generation; memo behind a shared `Arc` | KEEP. | Kept. |
| Template `group_by` returns groups | KEEP, but flag a breaking template change. | Kept. Change row 11 says so. |

Ranked defects of the audit for ticket 29 [V unless marked]:

- The direction of bare `--group-by FIELD` is unspecified. `--sort` defaults to
  descending.
- `key: n` is hard to parse and unlike `uniq -c`.
- The summary wording dropped `from ROOT`.
- `--count` on `list` and `table` is new. The help must carry it.
- No machine-readable group output exists. Scripts get only `--count`.
- Notice dedup keys are missing. The Answer sets one notice per
  (document or path, kind) until the state changes or the buffer closes.
- `--sort nosuch` exits 0 and does nothing. An unknown `--group-by` field
  would silently give one null group.
- Error message wording is open. Fix the shape now: one `help:` line that
  names the replacement.
- `task --table` with `--group-by`: does each group get its own header?
- `task --count` with `--group-by` is a new shape under one flag.
- The exit code of an empty grouped output needs a statement.
- The heading level and nesting need a statement.
- `Object::get_value` and `enumerate` have no error channel (ticket 31 handoff).
  [H] Without an adapter, these callbacks cannot propagate lazy errors from the
  target `GroupedQuerySet`.
- A separate type is right for grouped sets. A shared trait buys nothing
  without a second implementer.

Overclaims the audit found [V unless marked]:

- The sort claim of "4.5 ms against 31.6 ms" is the pre-built key case
  (section 6). A String-backed `Ord` costs 9 to 10 ms at opt-level 3 and 10 to
  22 ms at opt-level z.
- The shipped profile is `opt-level = "z"` (`Cargo.toml:269-281`). It slows
  iterator-heavy comparators 2x. Other numbers are not re-measured under `z`.
- Grouped render numbers use a 100x500x4 shape and a median of 11. The shape
  fits a render test, not a real vault.
- ICU collator cost (35 to 200 ms) comes from replica comparators [V per
  RedTeam29]. A 3x error could make text grouping noticeable. The hash-group
  path caps it.

### 11c. AuditImplementability (implementation map of v3)

The audit mapped the code sites and found 21 ambiguities (B1-B21). The rows
below cover ticket 29.

Code facts [V]:

- Today the builder is `QueryBuilder::{pages,lists,tasks}(SourceSelector)`
  plus `.filter(&str)`. `ExecutionPlan { ops: Vec<QueryTransform> }` is at
  `src/query/plan.rs:30`.
- `FilterExpr` and `SourceExpr` are separate parsers under
  `query/grammar/{filter,source,expr}.rs`. Source atoms carry no spans.
- `ClassExpansionMode` holds the resolved set. `visit_atoms_mut` changes it in
  place (`source.rs:81`). That contradicts "class names immutable in the IR".
- [P] Atoms would live in a new module such as `query/ir.rs`. The existing
  `Spanned` and `SpannedTokenStream` carry spans. The IR needs `Option<Span>`
  plus a clause id.
- `refresh_query` (`src/cli/mod.rs:339`) builds a `QueryBuilder` from strings.
  `TaskFilterArgs::status_filter` (`task.rs:44`) formats
  `list.status_symbol == "{}"` with `escape_default()`.
- The template engine calls `QueryService::run(&Arc<WorkspaceIndex>, builder)`.
- The store has 8 tables (`tables.rs`), no facet tables and no range keys.
  `SourceResolver` scans all paths for Not. And intersects child candidate sets
  (`src/query/service.rs:340-370`).
- `QueryTransform::GroupBy(FieldPath)` is a sort alias (`plan.rs:146/250`).
  `QuerySet::group_by` is at `results.rs:606`. The template dispatch is at
  `src/template/engine/query.rs:422`. Tests sit at `results.rs:1250` and
  `src/template/engine/query.rs:929`.
- `--count` exists only on `task` (`TaskPresentationArgs`, `task.rs:112`).
- `format.rs` pads flat tables to the widest cell (`:147-158`). Golden files
  pin this (`tests/baselines/output/*.txt` through `e2e/golden.rs`).
- [P] Help-surface assertions in `tests/e2e/help.rs` change.
- [P] The grammar tests that move include builder and plan tests
  (`src/query/builder.rs:324`,
  `plan.rs`) and the fuzz targets `source_expression` and `filter_expression`.

| # | Ambiguity | Resolution proposed, and the grilling result |
| --- | --- | --- |
| B10 | Clause id and span: relative to what text? | A builder-owned clause table `(id, Arc<str>)`; the span is a byte range into that clause. Adopted. |
| B11 | Pushdown scope. | Decide before the Answer. Recommended the small subset. The user decided (override 1). |
| B12 | Where do per-field bracket counts live, and how do they decrement? | Proposed an 11th `field_stats` table. The grilling chose the `meta` facet, in the same write transaction as the facet change (old derived facts are deleted first). |
| B13 | Group label "canonical spelling, not first row". | Byte-minimal raw spelling among class members. Adopted. |
| B14 | Null label `(none)` against a real key `(none)` in `n<TAB>key` lines. | Same backslash escape in count lines. Adopted. |
| B15 | `--group-by` takes one key through the `SortOrder::parse` grammar. | Exactly one term; a comma is an error. Adopted. |
| B16 | `--count` with `--group-by` and `--table`: which wins? | `--count` wins, as in flat `task`. The summary stays suppressed. Adopted. |
| B17 | Group invariant "except Date and midnight civil DateTime". | Not an exception: ticket 19 (`19:137`) says they are equal. The real exception is civil against instant. Adopted. |
| B18 | "One instant line in the pinned zone" against "civil kept separate". | Two lines: civil `(date, time)` with Date as midnight, and instant epoch nanoseconds. Adopted. |

Handoffs, ADR and glossary conflicts the audit found for ticket 29 [V]:

- Ticket 33 already owns `normalize_source_input` and task-filter synthesis.
  Ticket 29's "CLI synthesis moves to the builder" is a duplicate decision.
  Tell ticket 33 that ticket 29 settled synthesis.
- The `member_of` spelling goes to ticket 30. Ticket 30's Question lists no
  query operators yet.
- The EPIPE writer contract in ticket 33 and the output module in ticket 35
  share one location. Name it once.
- ADR 0005 (accepted): the eight tables and the `group_by` chain wording
  (`:38`, `:45`, `:67`) are superseded by tickets 24 and 29.
- ADR 0008 (accepted): `SourceAtom::Path(GlobPattern)` (`:77-84`) and "no
  introspection of parsed expressions" (`:104`) change with the IR and spans.
- ADR 0010 (proposed): "quoted and unquoted forms must mean the same thing"
  (`:30`, `:42`) is false after ticket 19's adaptive quoted literals. The
  pushdown rule relies on the difference. Add "superseded in part".
- Glossaries: `src/query/GLOSSARY.md:65` (`group_by` becomes a terminal),
  `:43-47` (Sort Order default against `--group-by`), `:3-4` and
  `GLOSSARY-MAP.md:23-24` (output rendering moves).
- The DSL field `ctime` is the birth-time alias
  (`src/query/grammar/field.rs:205`).
  Ticket 25's "ctime" is the Unix inode-change time. Add an Avoid line.

Defects of the audit for ticket 29, ranked [V]:

- Medium: the IR cannot express path-literal atoms. Atoms are `GlobPattern`
  regexes (`source.rs:190`, ADR-0008). Add a Literal, Folder and Glob split.
- Medium: the flat task-depth fix changes frozen stdout. It is not on ticket
  16's not-preserved list. Add a row and get the user's acknowledgement.
- Medium (UX): `--group-by FIELD` ascending against `--sort FIELD`
  descending (`src/cli/mod.rs:293-302`). Flip the bare form or state the
  asymmetry in the help.
- Medium: `LinkResolver<'a>` borrows `&[FileMeta]` and cannot sit in a
  generation object. This belongs to ticket 28.
- Low-medium: the group invariant exception list is wrong (B17).
- Low: the pushdown scope is open and bracket counts have no table.
- Low: flat tables need two passes for global widths, so only grouped output
  streams. `"||\n"` (`format.rs:126`) needs a grouped equivalent.
- Low: DSL `ctime` and ticket 25 `ctime` differ.

Not checked by this audit:

- The 0.148 ms and 13 ms patch timings. The artifact is not in the repo.
- Windows behavior.
- Whether `public-api-default.txt` lists `InlinkMap` or `FileEntry` types
  beyond the 16 matches.

## 12. Grouping precedent (GroupPrecedent and GroupLocal)

GroupPrecedent read primary documents and sources. It made no repo edit.
`web_search` was not mounted. The Tasks site failed TLS, so the agent read
the Tasks docs markdown from the project's GitHub repo. DuckDB `stable`
redirects to the `docs/2.0/...` pages. [V] means the agent read the claim in
a cited source. [H] means inference. GroupLocal read the local digests under
`docs/digests/` (gitignored: read them with `gitignore:false` or a direct
path). Digest line numbers below are file-line numbers.

### 12a. Comparison table

| Precedent | Output shape | Group-order rule | Null key | Key equality | Later stages after grouping | Row order inside group |
| --- | --- | --- | --- | --- | --- | --- |
| PostgreSQL `GROUP BY` | One row per group. Non-key columns only inside aggregates. [V] | Unspecified without `ORDER BY` [V] | One group not stated for `GROUP BY` [H]. `DISTINCT` treats nulls as equal [V]. In `ORDER BY`, nulls are "larger" by default [V]. | Not documented for `GROUP BY`. 1 against 1.0 is [H]. | `HAVING`, `SELECT`, window functions, `ORDER BY`, `LIMIT` apply to group rows [V] | Unspecified for `array_agg` unless `ORDER BY` is inside the aggregate [V] |
| PostgreSQL `PARTITION BY` | Row-preserving. Adds a column per input row. [V] | None (no groups emitted) | n/a | n/a | The normal row pipeline continues [V] | Per window `ORDER BY`. Ties are unspecified. [V] |
| SQLite | Same as SQL. One row per group. | `ORDER BY` absent: "undefined" [V] | "For the purposes of grouping rows, NULL values are considered equal" [V] | Collation rules for text [V]. Int against float is stated for `DISTINCT` only [V]. For `GROUP BY` it is [H]. | Same as PostgreSQL | Not applicable (collapsed) |
| DuckDB `GROUP BY` plus `list()` | Collapsed row. `list(x)` carries members. | "`GROUP BY` (neither in- nor output order are guaranteed)" [V] | Default `NULLS LAST` on `ORDER BY`. Set by `default_null_order`. [V] | Not documented. [H] | Same as SQL. `GROUP BY ALL` groups by every non-aggregate select column. [V] | `list` is "affected by ordering" and "can have non-deterministic results without ordering". [V] |
| jq `group_by(f)` | Array of arrays. The key is not emitted. [V] | "sorted by the value of the `.foo` field", with the `sort` order [V] | `null` sorts first (null, false, true, numbers, strings, arrays, objects) [V] | Comparison equality under that total order. 1 against 1.0 is [H]. | Ordinary jq. The result is a plain array, so `map` and `.[]` continue. [V] | Stable. `sort_items` breaks ties by index. `group_by` is `_group_by_impl(map([f]))`. [V] |
| Obsidian Tasks `group by` | Headings drawn over the flat task list. No data structure. [V] | "Headings are displayed in case-sensitive alphabetical order, not the original order." `reverse` flips them. Built-ins can override the order. [V] | Built-in headings: "No due date", "(No tags)". A custom `group by function` that returns `null`, `''` or `[]` gives no heading. [V] | Equal heading strings share a group. [H] | Fixed order: filter, sort, `limit`, group, `limit groups`. [V] | `sort by` controls the order inside each group. [V] |
| Obsidian Dataview `GROUP BY` | "one row per unique field value", with the key field plus a `rows` array [V]. Source shape: `{key, rows, [name]: key}`. [V] | Sorted by key ascending (comparator `Values.compareValue`) [V] | Null is smallest. `compareValue`: `val1 === null` gives -1. [V] Renders as `\-` (`renderNullAs`). [V] | Sort, then compare adjacent keys with `=`. For numbers `==` is used, so 1 equals 1.0. [V] | Later commands run on the group rows: `WHERE`, `SORT`, `LIMIT`, `FLATTEN`. A duplicate `GROUP BY` is allowed. [V] | Prior order, through a JS `Array.sort`. Stability is [H] (ES2019). |
| Nushell `group-by` | Default: a record keyed by the key string, each value a list of rows. `--to-table`: a table with one column per grouper plus `items`. [V] | The doc examples show first-appearance order. [V] | Record form: "null groups are omitted". `--to-table`: "null groups are kept". [V] | Record form: keys are stringified, so "true" and true, and 1 and "1", share a group. `--to-table`: "same type and the same value", so "1 and 1.0 are separate groups". [V] | Ordinary Nushell values. | Not stated. The doc examples show prior order. |
| pandas `groupby` | A `GroupBy` object, not a DataFrame. Iteration yields `(key, sub-DataFrame)`. A multi-key name is a tuple. [V] | "By default the group keys are sorted". `sort=False` gives order of appearance. [V] | "By default `NA` values are excluded from group keys"; `dropna=False` keeps them. [V] | Not documented. [H] | `agg`, `apply`, `transform`, `filter`. | "`groupby` will preserve the order in which observations are sorted within each group" [V] |
| Python `itertools.groupby` | Iterator of `(key, sub-iterator)` | Runs of consecutive equal keys. "Generally, the iterable needs to already be sorted on the same key function." [V] | n/a | n/a | The previous group is invalid when the iterator advances. [V] | Source order [V] |
| DataFusion `LogicalPlan` | `Aggregate` and `Window` are separate plan variants. The `Aggregate` schema is "the group expressions followed by the aggregate expressions". The `Window` schema is "the input schema followed by the window function expressions". [V] | Not set by the node | n/a | n/a | Both are mid-plan nodes. `Sort`, `Filter`, `Limit` can sit above. | n/a |

### 12b. Source notes (URLs)

- PostgreSQL docs 18:
  <https://www.postgresql.org/docs/current/queries-table-expressions.html>
  (7.2.3, 7.2.5), <https://www.postgresql.org/docs/current/sql-select.html>,
  <https://www.postgresql.org/docs/current/tutorial-window.html>,
  <https://www.postgresql.org/docs/current/functions-aggregate.html>.
- DuckDB: <https://duckdb.org/docs/2.0/sql/query_syntax/groupby>,
  <https://duckdb.org/docs/2.0/sql/functions/aggregates>,
  <https://duckdb.org/docs/2.0/sql/dialect/order_preservation>,
  <https://duckdb.org/docs/2.0/sql/query_syntax/orderby>.
- jq: <https://jqlang.org/manual/>, and the sources
  `src/builtin.jq` and `src/jv_aux.c` in `jqlang/jq`.
- Tasks: `Queries/Grouping.md` and `Scripting/Custom Grouping.md` in the
  `obsidian-tasks-group/obsidian-tasks` repo (`docs/`).
- Dataview:
  <https://blacksmithgu.github.io/obsidian-dataview/queries/data-commands/>
  and, in `blacksmithgu/obsidian-dataview`, `src/query/engine.ts`,
  `src/data-model/value.ts`, `src/expression/binaryop.ts`, `src/settings.ts`.
- Nushell: <https://www.nushell.sh/commands/docs/group-by.html>. pandas:
  <https://pandas.pydata.org/docs/user_guide/groupby.html>. Python:
  <https://docs.python.org/3/library/itertools.html#itertools.groupby>.
- DataFusion:
  <https://docs.rs/datafusion-expr/latest/datafusion_expr/logical_plan/enum.LogicalPlan.html>
  and <https://datafusion.apache.org/user-guide/sql/select.html>. SQLite:
  <https://www.sqlite.org/lang_select.html> (2.4).

### 12c. Where grouping sits across precedents

| Reading | Precedents |
| --- | --- |
| Mid-plan relational node; later stages see group rows | SQL, DataFusion `Aggregate`, DuckDB, Dataview |
| Presentation step after sort and limit | Tasks |
| Value-level function that returns plain data | jq, Nushell, pandas (object), `itertools` |

### 12d. Design pressure on the candidate

The candidate: `group_by` ends the plan and returns an ordered list of
`{key, rows}`. Each `rows` is a normal `QuerySet`. Group order is the sort
rank of the key. Row order inside a group is the prior order.

| Question | Supports | Contradicts or complicates |
| --- | --- | --- |
| (a) Terminal, ends the plan | Tasks is the closest text precedent: grouping is a presentation step after sort and `limit`, and only `limit groups` and per-group limits follow [V]. pandas returns a separate `GroupBy` type. jq returns a plain value. Tasks order protects "grouping must not change which rows appear" [V]. | SQL, DataFusion and DuckDB treat grouping as a mid-plan node [V]. Dataview allows `WHERE`, `SORT`, `LIMIT`, `FLATTEN` after `GROUP BY`, and repeated `GROUP BY` [V]. Nushell and jq outputs are ordinary values [V]. No precedent that keeps rows has a typed "ends here" marker except Tasks. A terminal-by-type form is the closest reading. |
| (b) Shape: list of `{key, rows}` | Dataview is almost this shape, with a recursive `Grouping<{key, rows}>` [V]. pandas yields `(key, sub-frame)` pairs [V]. Nushell `--to-table` is key columns plus `items` [V]. | jq returns an array of arrays with no key [V]. SQL collapses to one row per group [V]. The Nushell default record loses key type and null keys [V]. Tasks has no data shape. |
| (c) Group order is the key sort rank | jq, Dataview (`compareValue`, nulls first) and pandas (`sort=True`) [V]. Tasks sorts by heading text [V]. | SQL gives no order without `ORDER BY` [V]. Nushell, pandas `sort=False` and `itertools` use appearance or run order [V]. Tasks sorts rendered text, so rank and text can diverge [V]. Null position differs: first in jq, Dataview and SQLite ascending, last in PostgreSQL and DuckDB ascending [V]. Ticket 19's nulls-first matches jq, Dataview and SQLite. Only pandas and Tasks offer a reversal switch [V]. |
| (d) Numeric-equality key (Int 1 equals Float 1.0) | Dataview (JS numbers, `compareValue` with `==`) [V]. SQLite ("An integer is equal to a floating point number if they represent the same quantity", stated for `DISTINCT`) [V]. jq and PostgreSQL probably agree [H]. | Nushell `--to-table`: "keys share a group only when they have the same type and the same value, so 1 and 1.0 are separate groups" [V]. The Nushell record form puts 1 and "1" in one group by display text [V]. |
| (e) Multiple keys | Tasks nests headings; the first `group by` has top priority [V]. Nushell nests, or flattens with `--to-table` [V]. pandas uses a tuple key [V]. PostgreSQL uses a flat composite key [V]. jq builds a composite key with `map([f])` [V]. | Dataview takes one key and nests by repeating `GROUP BY` [V for syntax, [H] for shape]. Nested forms make `rows` of an outer group a list of groups, not a `QuerySet`. |
| Multi-valued keys | Tasks shows a task under every value of an array key [V]. | Nothing else surveyed does this. Dataview compares the array as one key. Nushell groups the whole list. Today's Traces sort key turns lists to Null (`sort.rs:316-317`). |

Observation [H]: numeric-equality keys need a canonical key. The docs do not
say whether the group shows 1 or 1.0. Dataview keeps the first row's key after
sorting (`finalGroupData` takes `groupData[0].key`) [V].

### 12e. GroupLocal: Dataview and Tasks sources, and the local code

Dataview (digest `DV-src` is
`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`)
[V]:

- Each group row is `{ key, rows: DataObject[], [op.field.name]: key }` with
  `id = key` (DV-src:8332-8356). The JS API type is
  `{ key: U; rows: DataArray<T> }` (DV-src:1350).
- `groupBy` in the JS API is a stable sort, then a run split
  (DV-src:1611-1631). `groupIn` groups inside each existing group
  (DV-src:1353-1354, 1634-1645).
- After a group op, every later `WHERE`, `SORT`, `LIMIT`, `FLATTEN` runs on
  the group rows. Docs "swizzling": `rows.test` is the array of `test` from
  each member (DV-docs:2735).
- `FLATTEN` on the group name pops the group meaning (DV-src:8378). A repeated
  `GROUP BY` stacks the meaning (DV-src:8357).
- A row whose key errors is dropped (DV-src:8312-8315). If every row errors,
  the op fails the query (8384-8393).
- Keys sort with the binary `<` and `>` operators, then adjacent keys merge
  when `=` is true (DV-src:8321-8343). `compareValue`: null first; different
  types by `type.localeCompare`; strings by `localeCompare`; numbers by `<`
  and `==` (DV-src:5472-5558). Integers and floats are both `number`.
- No descending `GROUP BY` form exists. A separate `SORT` after grouping
  gives direction.
- The render form. A grouped `LIST` shows only the group keys by default
  (DV-docs:3046). A grouped `TABLE` has one row per group, and the first
  column is the key (DV-src:8540-8541). Task groups render as headings plus
  nested bullets (DV-src:12414-12420) or an `<h4>` per group
  (DV-src:13518-13535). `nestGroups` removes tasks that already sit as a child
  in the same group (DV-src:13656-13667). Dataview's changelog says task-group
  semantics "will likely also be revisited" (DV-digest:1329-1331). `GROUP BY`
  has no effect in `CALENDAR` (DV-docs:3436).

Tasks (digests `T-src` and `T-docs`) [V]:

- Pipeline order: filter, sort, limit, group, `limit groups`
  (T-docs:10501-10507). Group order inside a group is the sort order
  (T-docs:10474). `TaskGroups` is built from tasks that are "already sorted"
  (T-src:17513).
- Several `group by` lines nest. Headings are `h4`, `h5`, then `h6` for level
  3 and deeper (T-docs:10437-10445; T-src:17345 `Math.min(4 + nestingLevel,
  6)`).
- The heading order uses `localeCompare(..., {numeric:true})` per level
  (T-src:17602). Docs call it "case-sensitive alphabetical" (T-docs:10450).
  `reverse` flips one level (T-src:17604). Users force a numeric order with a
  hidden `%%...%%` prefix (T-docs:14399-14405).
- A grouper returns `string[]`, so one task can land in several groups
  (T-src:17110, 17429-17438, 17551-17555). Zero names become the group `''`,
  shown with no
  heading (T-src:17431-17436).
- Output is a flat `TaskGroup[]` with `groups`, `groupHeadings` and `tasks`
  (T-src:17254-17276, 17546-17547). `limit groups` caps tasks per group
  (T-src:17630-17640).

Local code (Traces) [V]:

- `--from` and `--where` are repeated per command (`list.rs:28-33`,
  `table.rs:22-27`, `task.rs:150-155`). `--sort`, `--asc` and `--desc` are the
  shared `SortArgs` (`src/cli/mod.rs:289-303`, flattened into `list.rs:35-36`,
  `table.rs:29-30`, `task.rs:160-161`; `resolve` at `src/cli/mod.rs:315-326`).
- `--limit` does not exist on the three commands. `--column` is on `table`
  (`table.rs:33-34`). Task presentation uses `--table`, `--column`, `--count`
  and `-l` (`task.rs:70-76, 104-112`).
- Each command calls `refresh_page_query` or `refresh_task_query`
  (`list.rs:90`, `table.rs:89`, `task.rs:222`), then `refresh_query`
  (`src/cli/mod.rs:339-370`). The CLI does not call `QuerySet::group_by`. Only
  the
  template method reaches it (`src/template/engine/query.rs:422-425`).
- The plan transforms are `QueryTransform::{Filter, Sort, Limit, GroupBy,
  Flatten, TopK}` (`plan.rs:140-153`). `GroupBy` builds
  `SortOrder::single(path, Ascending)` and calls `order.sort(rows)`
  (`plan.rs:250-254`). The `fuse_sort_limit` optimizer (`plan.rs:110-130`)
  fuses adjacent `Sort` and `Limit` stages into `TopK`. A `GroupBy` between them
  breaks the fusion.
- `QueryTransform::apply` maps `Vec<QueryRow>` to `Vec<QueryRow>`
  (`plan.rs:235`). `QuerySet` caches one `Arc<Vec<QueryRow>>`
  (`results.rs:455-482`). The formatter takes `&[QueryRow]` (`format.rs:83`).
  Grouped output needs a new type.
- Formatter: `QueryDisplayFormat::{Table, List, TaskList}` (`format.rs:25-39`).
  `render_table` pads every column to the widest cell over all rows
  (`format.rs:112-161`, `147-153`). `render_list` emits `- <field>\n`
  (`format.rs:197-206`). `render_task_list` emits `- [x] text (path)` and
  indents by `row.depth()` (`format.rs:214-264`, `225-227`). A nested child can
  land in another group than its parent [H].
- `--count` applies in `format_rows` before formatting (`task.rs:126-128`).
  The stderr summary uses `rows.len()` (`list.rs:96`, `table.rs:95`,
  `task.rs:228`), printed as `"{count} page(s)"` or `"{count} task(s)"`
  (`list.rs:65`, `table.rs:64`, `task.rs:196`).
- Frozen shapes: five golden baselines under `tests/baselines/output/`
  (`list-20-rows.txt`, `page-table-20-rows.txt`, `task-list-20-rows.txt`,
  `task-table-20-rows.txt`, `task-count-20-rows.txt`). They load through
  `tests/e2e/golden.rs:34-57`.
- Help contracts: `tests/e2e/help.rs`, `help_exposes_command_contract`.
  A `--group-by` flag changes its expected list, table and task options.
- Feature scenarios: `tests/features/query/query-render.feature` (lines 17,
  33, 48, 63). Unit render tests: `list.rs:121-135` and `table.rs:126-145`.
- Public-API baselines: `tests/baselines/pub-tokens.txt:1020` and `:1046`.
- Tests that assert the sort-alias behavior:
  `src/template/engine/query.rs:928-941`
  (`"b a "`), `results.rs:1256-1274` and `results.rs:1277-1299`, and
  `results.rs:1302-1311` (rejects `file.zzzz`).
- The duration test expects three separate rows for `1h 30m`, `10m` and
  `90m`. Real grouping with ticket 19 equality probably merges `1h 30m` and
  `90m` [H].
- Today's key projection (`sort.rs:316-317, 346-349`) turns lists, objects,
  Tags and Inlinks into Null. Null placement is `Auto`: first ascending, last
  descending (`sort.rs:269-292`). The `First` and `Last` variants are dead
  code outside tests.
- Ticket 19 hand-off: the "`GroupBy` key relation" is unresolved and handed
  to ticket 29 (`19:225`). Ticket 19 says `List` and `Object` "sort last"
  (`19:217`).

Facts that constrain the design (GroupLocal) [V]:

1. Dataview lets later stages run on `{key, rows}`. Tasks groups last and
   permits no later filter or sort. The candidate matches Tasks.
2. Both keep the earlier sort order inside a group. Tasks sorts groups by
   heading text. Dataview sorts by typed value. Ticket 19 settled typed
   equality, so the Dataview model fits better.
3. Both render task groups as headings with nested bullets. Dataview renders
   table groups as one row per group. Neither prints one sub-table per group.
4. Both treat null by "null sorts first" (Dataview) or "empty heading" (Tasks).
5. Tasks puts a task under every value of an array key. Dataview compares the
   array as one key. Today's Traces projection nulls lists.
6. No CLI path calls `group_by` and no flag exists. A new flag needs a clap
   definition, a hook in `refresh_query` and a formatter for grouped output.
7. Default stdout is pinned by five golden baselines, three help baselines
   and feature scenarios. A new group flag must be opt-in.
8. Not verified: the ticket 16 envelope (the agent did not open it) and
   `builder.rs`.

Gaps (GroupPrecedent): Notion and Logseq were not read. PostgreSQL, DuckDB and
pandas docs do not state int and float key equality for `GROUP BY`. Dataview's
docs page does not describe null keys (read from `compareValue` and
`renderNullAs`). jq's manual does not state `group_by` stability.

## 13. Nushell lessons (NuRender and NuPipeline)

Both agents read Nushell `main` at `b4993b5b` (workspace 0.117.1, 2026-11-14).
They ran no `nu` binary, so every behavior below comes from source, tests and
the book. Anchors are `crate path:line` [V]. [H] marks inference.
Source paths below are relative to `crates/` in
<https://github.com/nushell/nushell/tree/b4993b5b>.
Short anchors name Nushell files, not similarly named Traces files.
The book-only line anchor is unresolved in this pass; the source-code anchors
remain the evidence for sort behavior.

Typed group keys are brand new [V]. PR #18933, "impl Hash for Value", merged
2026-10-07 into v0.117.0. The release date was 2026-11-10. Its description
says it should "later allow Value to be used as a key in group-by". Treat
`group-by --to-table` as an unseasoned design. The anchors hold at tag
`0.117.0` (`where_.rs` is off by 2 lines).

### 13a. Value model and comparison

| Topic | Nushell fact | Anchor |
| --- | --- | --- |
| Enum shape | `Value` has 17 variants. A "table" is a `List` of `Record`s, labeled only at display time. | `nu-protocol/src/value/mod.rs:80-222`, `:1167-1181` |
| Cross-type order | The variant order is the sort order ("do not reorder"). The book guarantees only "null last". | `value/mod.rs:73-74`; book `sorting.html` lines 363-369 |
| Spans | Every variant carries `internal_span`, marked "spans are being refactored out of Value". Hash and `strict_eq` ignore spans. Plugin values get `call.head`. | `value/mod.rs:84-87`, `:2823`; `nu_plugin_query/src/query_json.rs:121-123` |
| `==` and ordering | `PartialEq` is `partial_cmp == Equal`. `Int == Float` is true. Float equality uses an epsilon, which is not transitive. `i64 as f64` loses precision above 2^53. Nushell states that `Value` must never implement `Eq`. | `value/mod.rs:2787-2791`, `:2474,2492`, `:2441-2449`, `:2815-2821,2897` |
| Strict equality for keys | `strict_eq`: same variant and payload, NaN equals NaN, so `1 != 1.0`. `Hash` follows `strict_eq` (`-0.0` collapsed to `0.0`). Lists hash the first and last 5 items. Dates compare by instant. The docs say to wrap `Value` in a newtype for map keys. | `value/mod.rs:2903-2925`, `:2800-2813,2824-2878`, `:2793-2798,2849-2865`, `:2838`, `:2893-2895`; `group_by.rs:426-470` |
| Sort | `sort` and `sort-by` never error on type mismatch. Int and Float pairs use `f64::total_cmp`, so `sort` and `==` use different comparators. Everything else uses `partial_cmp().unwrap_or(Equal)`. | `nu-command/src/sort_utils.rs:180-189`, `:210-220` |
| Sort options | `--ignore-case` folds strings and globs only. `--natural` uses `alphanumeric_sort`. `--reverse` is `vec.reverse()` after a stable sort. By code reading, it puts nulls first and reverses ties [H]. No test found. | `sort_utils.rs:225-240`, `:231`; `sort_by.rs`; `sort.rs:188-189` |
| Multi-key sort and errors | Multi-key `sort-by` stops at the first non-Equal key. A missing field fails the whole sort. A key closure runs twice per comparison, with no key cache. | `sort_utils.rs:137-170`, `:245-255`, `:72-90`, `:257-271` |
| Nulls in operators | `<`, `<=`, `>`, `>=` return null if either side is null. `==` and `!=` are two-valued (`null == 1` is false). `where` keeps a row only if the result is exactly `Bool(true)`. | `value/mod.rs:3826-3850,4538-4546`, `:3990-4020`, `:1852-1854`; `where_.rs:75` |
| Errors as values | `Value::Error` is a first-class stream item. `unwrap_error` turns it back into `Err`. | `value/mod.rs:1840-1850` |

### 13b. `group-by` (`nu-command/src/filters/group_by.rs`)

| Topic | Nushell fact | Anchor |
| --- | --- | --- |
| Grouper | A cell path or a closure. Several groupers are allowed. | `:310-311,541-557` |
| Input | Fully materialized first. The first `Value::Error` aborts. Empty input gives an empty record or list. | `:325`, `:326-333`; `pipeline_data.rs:1013-1015` |
| Record mode (default) | The key is `to_expanded_string(", ")`. `1` and `"1"` share a group. `true` and `"true"` share a group. Null groups are omitted. | `:416-454`, `:435-443`, `:672-690` |
| `--to-table` | The key is the value, compared with `strict_eq`. `1`, `1.0` and `"1"` are three groups. Columns are one per grouper plus `items`. Null groups are kept. A grouper named `items`, or a duplicate name, is an error. | `:319-323`, `:355-414`, `:456-470`, `:638-670` |
| Order | First-seen order (`IndexMap`). Rows keep input order inside each group. No group sort. | `:485,514,528,572` |
| List-valued keys | No fan-out. The key is one list value. In record mode it is the string `[a, b]`. Read from code, not run. | `tests/commands/group_by.rs:95-105`; `value/mod.rs:1126-1132` |
| Optional paths | `foo?` that yields null drops the row. A required path with an explicit null keeps a null group. A missing required field errors. | `:491-495`; `value/mod.rs:2338-2348` |
| Multiple groupers | A tree. Record mode nests records. Table mode flattens to one row per leaf. | `:559-570,610-670` |
| Errors | A key closure error aborts the whole command (`?`, no per-row isolation). | `:532` |
| Validation | Table column names are checked before any closure runs. `items` and duplicate names are rejected with a help message. | `:316-320,355-414` |
| `--prune` | Removes the key column, and its parent if empty. Best effort. | `:499-511` |
| Other identity | `uniq` and the set commands serialize a key-sorted value to NUON. | `filters/utils.rs:142-187` |

### 13c. Streaming, laziness and errors

- `PipelineData` is `Empty | Value | ListStream | ByteStream`
  (`nu-protocol/src/pipeline/pipeline_data.rs:49-55`). `ListStream` is a boxed
  iterator plus a span and an interrupt wrapper
  (`list_stream.rs:9-19,24-33,177-195`).
- `where` is lazy (`filter_map`). A closure error becomes an error value in
  the output, not an abort (`where_.rs:71-79`). `take` and `first`
  short-circuit (`nu-command/src/filters/take/take_.rs:114-116`;
  `first.rs:322-326`). `first 0` never
  touches the source (`first.rs:176-182`).
- `sort-by` and `group-by` force full materialization (`group_by.rs:325`).
- Error items are never dropped silently ("An error buried in a list wouldn't
  be noticed", `filters/utils.rs:21-50,42-47,71-140`). A closure error is
  wrapped with a "source value" label at the span of the input value
  (`utils.rs:9-19`; `shell_error/eval_block_with_input.rs:23-57`).
- `ShellError` is a `thiserror` and `miette::Diagnostic` enum with a code,
  `#[label]` spans and `#[help]` (`shell_error/mod.rs:24-58`). `ErrorStyle`
  has Short, Plain, Fancy and Nested renderers (`report_error.rs:239-262`;
  `short_handler.rs:22-50`). The book describes `error_style` and
  `error_lines`.
- A separate non-fatal channel exists: `ShellWarning` with a `ReportMode` of
  `FirstUse` (dedup by a hash log) or `EveryUse`. Warnings print to stderr as
  `Warning: ...` (`shell_warning.rs:10-34,75-83`; `report_error.rs:54-90`).
  Data-level notices ride on `Value::Error` items, which are fatal when
  consumed.
- Pushdown. `QueryPlan` has one variant, `Sqlite(SQLiteQueryBuilder)`, with
  `with_limit`, `with_offset`, `with_distinct`, `with_order_by`,
  `project_output_columns` and `count`
  (`database/query_plan/mod.rs:15-96`). `select`, `last`, `skip`, `uniq`,
  `length` and `first` use it. `where` and `sort-by` do not. The reason is
  [H]: a row condition is a closure and an AST, not a translatable IR.
- `project_output_columns` returns `Option` and says "so callers can safely
  fall back" (`nu-command/src/database/query_plan/mod.rs:57-66`;
  `nu-command/src/database/values/sqlite.rs:1002-1007`). This matches
  the Exact, Inexact, Unsupported split.
- The Polars plugin has a separate expression wrapper, `NuExpression`, around
  `polars::Expr`
  (`nu_plugin_polars/src/dataframe/values/nu_expression/mod.rs:19-26`).
  `polars collect` materializes. `==` on a lazy frame collects it first
  (`nu_plugin_polars/src/dataframe/values/nu_lazyframe/custom_value.rs:77-87`).
- The `where` language: a bare word on the left of an operator becomes
  `$it.<word>` (`parse_expressions.rs:1464-1480`). `in` uses loose `==`, so
  `1 in [1.0]` is true (`value/mod.rs:4028`). `=~` is an unanchored regex. A
  regex runtime error returns true for both `=~` and `!~`
  (`value/mod.rs:4165-4185,4198-4208`).

### 13d. Rendering (`nu-table`, `viewers/table.rs`)

- Layers. `Value` is the only pipeline type. At the end
  `PipelineData::print_table`
  calls the `table` command (`pipeline_data.rs:729-758`). `to md` and `to json`
  are separate commands that return strings.
- `NuTable` is a grid of already-rendered `Text<String>` cells plus styles. It
  has no `Value` knowledge (`nu-table/src/table.rs:46-62`). The `Value` to
  cell step lives in `types/general.rs`, `types/expanded.rs` and
  `types/collapse.rs`. It sits on the `tabled 0.22` crate (root
  `Cargo.toml:273`) with Nushell's own width planner.
- Width. `string_width` delegates to tabled
  (`nu-table/src/util.rs:17-19`). The claim that it strips ANSI and applies
  `unicode_width`
  to the widest line was not checked against the pinned dependency [H].
  `clean_charset` turns a tab into 4
  spaces and drops `\r` (`util.rs:64-95`). The tabled match is [H] (read on
  master, not the pinned 0.22).
- Fit ladder (`nu-table/src/table.rs:1088-1196`, `:40`, `:1258-1278`): use the
  natural
  width if it fits. Else give each column its floor (natural width, capped at
  10 characters) and hide the rest behind a trailing `...` column. Shrink in
  tiers, widest first (`:1295-1337`, `:1353-1388`). Wrap when content width is
  at least 4 (`:38`, `:802-869`).
- Failure. `NuTable::draw` returns `None` when not even one column fits. The
  command then prints `Couldn't fit table into {N} columns!` as ordinary
  stdout with a success result (`viewers/table.rs:694-702`, `:1288-1297`).
- Batches. A list streams in batches of 1000 rows or 1 s
  (`nu-protocol/src/config/table.rs:399-441`; `viewers/table.rs:922-1046`).
  Column widths are planned per batch, so they can differ between batches [H].
- Null and missing. Null renders as an empty string (`value/mod.rs:1141`). A
  missing key renders `missing_value_symbol` (default `❎`), centered
  (`common.rs:95-100`; `types/general.rs:207-218`). Empty input renders a dim
  "empty list" placeholder (`viewers/table.rs:1244-1271`).
- Nested cells show `[table N rows]`, `[list N items]` or `{record N fields}`
  (`value/mod.rs:1161-1189`). `--expand` draws nested tables
  (`types/expanded.rs:477-529`). Grouped output has no heading layer.
- `to md` (`nu-command/src/formats/to/md.rs`). A table comes from `table()`
  (`:497-588`). A missing key becomes an empty cell (`:539`). `|` is escaped to
  `\|` in cells (`:399`). `\r\n` becomes `<br>` (`:389-394`). `--pretty` pads
  with `String::len()`, which is bytes (`:487`, `:551`, `:730-760`). A second
  Markdown path exists: `table --theme markdown`
  (`nu-table/src/table_theme.rs:39-41`).
- Tests. No snapshot library. Expected output is inline literals (`indoc!`)
  (`tests/commands/table/display.rs:8-61`). Every `nu-table` case asserts
  `string_width(&table) <= termwidth` (`crates/nu-table/tests/common.rs`).
  Matrix tests cover 19 themes and assert properties
  (`tests/commands/table/config_matrix.rs:68-114`). Command `examples()` run
  as tests and generate the docs (`group_by.rs:696-700`). The agent found no
  stated stability policy for table output [H].

### 13e. What Traces could copy [H]

1. A cell grid with no domain types between the row-to-text step and the
   drawing step. This matches the Traces `&[QueryRow]` renderers.
2. Two equality relations: a loose `==` for predicates, and a separate
   `strict_eq` plus `Hash` newtype for group keys. Never `Eq` on the loose
   type. The grilling uses exact numeric equality, not Nushell's numeric-variant
   split. Group equality agrees with filter `==` within each grouping domain.
3. Use order-preserving group maps. Validate group-by names before execution.
4. Distinct renderings for missing and null.
5. Typed pushdown results with an explicit `None` fallback (the ternary), and
   a plan type behind a single seam.
6. Escape `|` and newlines in Markdown cells.
7. Group output as key columns plus items, a deterministic empty-input shape,
   reserved column names.
8. Measure with `unicode_width` on ANSI-stripped text. Normalize tabs and
   `\r` before measuring.
9. Stream-level error items that are never dropped silently. A wrapper
   labeled "source value".
10. Warnings as a separate severity channel with first-use dedup.
11. Tests: a render function that takes a width, an invariant
    `width <= termwidth`, a matrix with property assertions, and examples that
    double as docs and tests.
12. Cache sort keys once. Plan widths once when the result set is collected.

### 13f. What Traces must not copy [H]

1. Loose epsilon `==` next to a `total_cmp` sort. The two comparators
   disagree. Traces uses exact numeric equality.
2. Group keys as strings (record mode). They collide, and null groups vanish.
3. Null groups dropped in record mode.
4. A hard abort on a key-closure error.
5. "Couldn't fit" printed as success stdout. This is bad for frozen stdout
   shapes.
6. `to md --pretty` padding by bytes.
7. Two Markdown renderers (`to md` and `table --theme markdown`).
8. Spans on every value. Nushell is removing them, and `call.head` does not
   locate data.
9. Silent `where` drops of non-bool results, and fail-open regex errors.
10. A list key that stays one group. Traces decided fan-out. Confirm it stays
    deliberate.
11. Null order. Nushell sorts null last ascending and `--reverse` flips the
    whole vector. Traces sorts nulls first. Decide what `--desc` does
    explicitly.
12. Scale. Nushell has 19 themes, width priorities, nested-table planning and
    a magic `index` column name. `table.rs` has 1,646 lines.
13. Per-batch widths. Traces collects its result set and can plan once. The
    grilling chose per-group widths for grouped output.

Open gaps: `nu-explore` was not read. The pinned `tabled 0.22` source was not
read. The `ErrorStyle` enum definition was not found. Polars `filter`
internals, the `Casing` implementation and `ByteStream` laziness were not read.
`--reverse` null placement and upstream short-circuit through several stages
are inferred. `group-by --to-table` has existed for one release only.

## 14. Reversals of the first-round recommendation

The first round is `z-grilling-guide.md`, `h1-stress-designs.md` and
`h2-redteam.md`. The table lists each place where the final decision set
differs, and the evidence that caused the change. Items marked "v2" come from
the grilling decisions v2. Items marked "override" come from the user's last
answers.

| # | First-round position | Final decision | Evidence or reason |
| --- | --- | --- | --- |
| 1 | Pushdown option A: a candidate step from the source atom only. Option B (per-predicate judgments) as a follow-up if the facet scan hurts at 50k. | The Answer states the full protocol (Exact, Inexact, Unsupported; R1-R7). The first implementation pushes range keys for `==`, `<`, `<=`, `>`, `>=` on unquoted numeric, temporal and bool literals, plus And-intersection and source-plus-filter. | Override 1. The wall-time data in `g2-pushdown-ux.md`: a scan crosses 100 ms near 19k notes. |
| 2 | Range keys are sound under "Exact-or-scan" per predicate. | Quoted literals, `!=`, `null` and missing never use a range key. `Or` scans at first. R1-R7 state the full protocol. | Section 1: ten counterexamples. `note-frontmatter.feature:51-64` pins one. |
| 3 | `Not` above an Exact atom is Inexact (h2). | R6: `Not(Exact S)` is `Exact(U \ S)` with `U` equal to all `files` rows. `Not(Inexact)` is Unsupported. In the first implementation, `Not` scans. | Section 1, hazard note. `not (rating > 5)` matches rows outside S. |
| 4 | Restrict range keys to note-mode rows (h2). | Keys on `file.folder`, `file.path` and `file.name` may prune Task and List rows. Other fields prune Pages rows only. Filename-derived `file.date` keys stay Unsupported until ticket 30. `file.day` is retired. | `src/query/results.rs:230-311` (no item override for `file.*`), `:313-326` (metadata), and `:193-199` (item tags). |
| 5 | Reject NUL in text values for the text range encoding (h2). | No text range table at first. | Override 1. A text facet table adds 16.8 MB at 50k (`g1-probes.md`). |
| 6 | AuditSimplicity: compute the ticket 19 bracket-mismatch notice in the residual pass. | Per-field bracket counts live in the `meta` facet, in the same write transaction. | Section 1: a pruned run never evaluates string-valued rows, so a residual-pass notice would vanish. |
| 7 | Keep the `GroupBy` alias and document it as a sort (z-guide, h1: "accepted debt"). | A distinct `GroupedQuerySet` with real grouping, after filter, sort and limit. | Dataview, Tasks and pandas keep members (section 12). h1 says that keeping the alias keeps the lie [H]. |
| 8 | Sort-then-split for grouping (v2, Q26). The red team then suggested a hybrid: sort-then-split for numbers and temporal, hash-group for text and Link. | ONE code path: hash-group on the canonical key, then sort the distinct keys. | Section 8a: hash-group costs 2 to 8 ms for numbers and byte keys. ICU sort+split costs 50 to 210 ms. The audit asked for one path (11a). |
| 9 | Group temporal keys on one instant line in the pinned zone (v2). | Two lines: civil `(date, time)` with Date as midnight, and instant epoch nanoseconds. The classes stay apart. A period is never a key. | Section 2: a mixed comparator has a cycle and 15 panics in 200 runs. AuditImplementability B17 and B18. |
| 10 | Link keys sort by the collator on the target only. | Link keys sort by `ComparisonKey`, then target plus subpath. | Section 2: `[[A#x]]` and `[[A#y]]` would merge. |
| 11 | Row identity is `(note position, item_idx)` (v2, Q7). | Document order is `(note position, item_idx)`. The row also carries a pre-group ordinal and a flatten ordinal. Distinct counts and dedup use the ordinal. | Section 3: `flatten("tags")` collides. `QueryRow::eq` deep-compares the entry. |
| 12 | Task depth is fixed only inside groups (v2, Q31). | Depth counts from the nearest ancestor present in the same output list or group, for flat and grouped output. Change row 12. The user acknowledged it. | Section 9, item 11: `traces task --todo` prints an orphaned child at depth 1. |
| 13 | Plain byte order for the default sort (v2, Q10). | The key is the path with `/` replaced by `0x00` (ticket 32). No change row for order. | Section 6: plain byte order moves 8 of 13 rows. 73,891 mismatches in 3M pairs. |
| 14 | Group order follows `--asc` only if the first sort key is the group field (v2, Q6b). | Bare `--group-by FIELD` is ascending. A `-` prefix is descending. The help text states the asymmetry with `--sort`. `--sort` orders rows inside each group. | Section 9, item 12. The `+` and `-` grammar of `SortOrder::parse` already exists. |
| 15 | The group heading shows the first row's key (v2, Q6b). | The heading shows the canonical spelling: byte-minimal among class members for strings and Links, normalized decimal for numbers, ISO for dates. | Section 5: the first row's key changes when a file is renamed. |
| 16 | Naive `##` prefix plus the raw key. | `##` plus a space plus `heading_escape(display(key))`. A property test with pulldown-cmark 0.13. | Section 5: 64,587 failures in 400k keys. |
| 17 | `--count` prints `key: n`. | `n<TAB>key`, with TAB, newline and backslash escaped. | UX audit: `key: n` is ambiguous when a key holds `: `. |
| 18 | stderr summary `D page(s) in G group(s)`. | `D page(s) in G group(s) from ROOT`, with `(M listings)` only when M is not D. | UX audit: the house style is `N page(s) from ROOT` (`src/cli/list.rs:65`). |
| 19 | Shared widths for grouped tables (v2, Q27). | Per-group widths, with groups streamed. | Section 8b: peak heap 8.59 MiB against 17.10 MiB. With an outlier, stdout grows 2.7x. |
| 20 | `format.rs`: record a preference and decide with ticket 35. | Rendering moves to an output module (typed rows or groups, then a plain cell grid, then ONE markdown renderer). Ticket 35 picks the path. | Section 13: Nushell has two Markdown paths and pads by bytes. |
| 21 | `member_of` predicate in the IR, with a notice for Null-date rows. | `member_of` is an IR node name only. No frontend, no implementation. Never a range key, never a group key. | AuditSimplicity item 20. The spelling waits on ticket 30. |
| 22 | Overlay swaps the row list before memoization (z-guide, h1). | `QuerySet` and `GroupedQuerySet` read one pinned generation and know nothing about buffers. Ticket 28 owns the derived generation. | Ticket 28 decision: a derived generation is one more pinned generation, so the seam is "any pinned generation". |
| 23 | Memoize per `QuerySet` instance (v2, Q28). | The memo lives behind a shared `Arc`, keyed by `(plan, frame, generation triple)`. | Section 9, item 15: `#[derive(Clone)]` copies `OnceLock` state and `push` resets it. |
| 24 | z-guide option C: builder in core and a thin CLI string helper. | The typed builder is the core surface. CLI synthesis moves to the builder. | The `--status é` bug (section 7). Ticket 33 still decides who calls the core. |
| 25 | Pushdown scope open in v3. | Decided in override 1, with the gate of 100 ms on 20,000 notes or fewer. | `g2-pushdown-ux.md`. |
| 26 | Path-set change uses `patch_and_diff` (v3). | One full re-resolve, off the request thread. This is ticket 28's decision. | Override 2. `src/index/refresh.rs:413-421`. |

## 15. Leftovers

These statements record the original authors' state, not a current worktree
audit.

- Worktree `.worktrees/rt29` stays (section 10).
- Worktree `.worktrees/pushdown-ux` is clean and detached at `d80d0d89`. It is
  ignored by `.gitignore:43`. A repo rule needs approval before removal. The
  command is `git worktree remove .worktrees/pushdown-ux`.
- Throwaway probe code: `/tmp/traces-probe`, `.worktrees/rt29/probe29`.
  Ticket 35 decides a durable bench home.

## 16. Final adversarial review (2026-11-16)

This section supersedes conflicting round-2 recommendations above.
The maintainer confirms the group, notice, task-depth and label corrections.
The review runs small counterexample models, not the target query engine.

### Candidate proofs

`9007199254740992` and `9007199254740993` share an f64 encoding [V per
Stress29]. An equality probe can therefore return both while the exact predicate
matches one. That set is Inexact. Its complement can lose a valid row. Exact now
means matching-set equality under the pinned row mode and frame. Closed lossy
bounds prove only a superset.

Temporal probes need operator-specific bounds for every eligible temporal kind.
Point-period membership is not period identity order. Unproved mappings scan.
The initial scope still admits only proved Exact cases and And.

### Complete group keys

Canonical equality is reflexive, symmetric and transitive. Civil values and
instants remain separate even when a filter relates them through a pinned zone.
Periods group by normalized `(start, unit)` identity, never membership.
Temporal subclasses order Civil, Instant, then Period.

Fan out only the outer list. Deduplicate canonical memberships per row.
Nested lists become ordered sequence keys. Objects use byte-sorted names and
recursively canonical values. Empty nested lists and objects remain distinct.
Tags project String keys. Inlinks project Link keys.

Stable sorting preserves each stage's input ordinal, not unconditional document
order. Hash grouping retains rows, keys and membership references.
The one-group memory bound applies only to the temporary rendering grid.

### Notices and output

Global bracket totals cannot establish a restricted query's mismatch total.
Each atom uses its logical stage input, independently of physical optimization.
Use an exact diagnostic pass when metadata cannot prove that result.
Include its cost in the whole-process optimization gate.

Task display depth uses active preceding ancestors. Orphans start at zero.
Do not reorder explicitly sorted rows. Keep source `list.depth` unchanged.

Group labels escape terminal controls and directional formatting controls.
Ordinary RTL letters and combining marks stay intact. Normalize before marking a
non-null `(none)` or leading-backslash label. Then apply output escaping once.
Typed templates retain complete keys. Printed Number `5` and String `5` labels
can coincide; labels are not serialization.

Tests must separate terminal safety from Markdown structure. Cover control
characters, bidi formatting, sentinel collisions, nested keys, scoped notices
and child-before-parent tasks. These are acceptance duties, not completed tests.

### Cleanup state

Both probe worktrees are removed with maintainer approval.
The untracked `probe29` source is copied to `/tmp/rt29-probe-src`.
Historical leftover statements above describe the authors' earlier state.
