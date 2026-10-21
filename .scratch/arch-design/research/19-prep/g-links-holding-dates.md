# Investigation: Links Holding Dates in Traces

- **Topic:** Semantics, comparison, coercion, sorting, and precedent for links whose targets are dates (periodic/daily notes).
- **Target File:** `.scratch/arch-design/research/19-prep/g-links-holding-dates.md`.
- **Status:** Complete.

---

## 1. Executive Summary & Empirical Ground Truth

In personal knowledge management (PKM) vaults, periodic notes (daily, weekly, monthly) are titled by date. Consequently, frontmatter and inline fields like `due: "[[2025-01-15]]"` or `day:: [[2025-01-15]]` parse as `NoteFieldValue::Link`, not `Date` [V: `src/note/field.rs:553`, `src/note/parser/inline.rs:135`].

Empirical probe results across heterogeneous fields (`due: "[[2025-01-15]]"`, `due: 2025-01-15`, `due:: "2025-01-15"`, `[[1990-01-01]]`, `[[2030-01-01]]`):

| Expression / Operation | `due` is Link `[[2025-01-15]]` | `due` is Date `2025-01-15` | `due` is String `"2025-01-15"` | Empirical Root Cause |
| :--- | :--- | :--- | :--- | :--- |
| `due == 2025-01-15` | **False** [V] | **True** [V] | **False** [V] | `is_equal_to_literal` has no cross-kind coercion (`field.rs:404`) [V]. |
| `due < 2025-02-01` | **False** [V] | **True** [V] | **False** [V] | Link rank 6 > Temporal rank 4; leaks rank lattice (`field.rs:310`) [V]. |
| `due > 2025-02-01` | **True (even for 1990!)** [V] | **False** [V] | **True** [V] | `[[1990-01-01]] > 2025-02-01` matches because rank 6 > rank 4 [V]. |
| `date(due) == 2025-01-15` | **False** [V] | **True** [V] | **True** [V] | `date()` rejects `Link` and returns `None` (`filter.rs:726`) [V]. |
| Sort by `due` (Ascending) | Rank 5 (`Text(target)`) [V] | Rank 4 (`DateTime`) [V] | Rank 4 (`DateTime`) [V] | String is sniffed to DateTime; Link is NOT sniffed (`sort.rs:381`) [V]. |

### Three Critical Findings

1. **Relational filters leak the rank lattice aggressively on Links:** `due < 2025-02-01` is NEVER satisfied by any link date (rank 6 is never < 4). Conversely, `due > 2025-02-01` is satisfied by EVERY link, even `[[1990-01-01]]` [V].
2. **`date(due)` rejects Links:** The query function `date()` accepts `Date`, `DateTime`, and `String`, but returns `None` for `Link`, dropping rows silently [V: `src/query/grammar/filter.rs:705-728`].
3. **Sort segregates Dates and Links into different rank bands:** `SortKey` sniffs `String` into `DateTime` (rank 4), but maps `Link` directly to `Text(target)` (rank 5) [V: `src/query/sort.rs:381, 394`]. Consequently, ALL real Dates sort before ALL Link dates, regardless of chronological time [V].

---

## 2. Traces Codebase Inventory

### 2.1 Query Function Registry (`date()` and relatives)

- `date(args)` (`src/query/grammar/filter.rs:705-728`) [V]:
  - `[Date(d)] | [Date(d), String(_)] => Some(Date(*d))`
  - `[DateTime(dt)] | [DateTime(dt), String(_)] => Some(DateTime(*dt))`
  - `[String(s)] => DatePoint::parse(s).ok().map(Into::into)`
  - `[String(s), String(fmt)] => DatePoint::parse_with_pattern(s, fmt).ok().map(Into::into)`
  - `[Null] => Some(Null)`
  - `_ => None` (rejects `Link`, `Number`, `Bool`, `Duration`, `List`, `Object`).
  - When `evaluate` returns `None`, `ComparisonExpr::is_matching` drops the evaluation to `false` [V: `filter.rs:219-225`].
- `as_date_point()` (`src/note/field.rs:125-130`) [V]: Only extracts `DatePoint` from `Date` and `DateTime`. Comment explicitly documents: *"Strings are not parsed: query evaluation only treats typed date values as dates"* [V: 122-123].

### 2.2 DatePoint and Temporal Parsing Entry Points

- `DatePoint::parse(s: &str)` (`src/temporal/point.rs:77-139`) [V]:
  - Requires `starts_like_date` (digit/`-`, byte 4/5 is `-`) and `starts_with_year` (4 ASCII digits) [V: 79-88].
  - Parses ISO 8601 / RFC 3339: `YYYY-MM-DD` (Day), `YYYY-MM` (Month), or `YYYY-MM-DD[T ]HH:MM[:SS]` (DateTime).
  - Rejects any leading directory path (e.g. `daily/2025-01-15`) or surrounding wiki brackets (`[[...]]`) [V: 79].
- `DatePoint::parse_prefix(s: &str)` (`src/temporal/point.rs:146-148`) [V]: Parses 10-byte `YYYY-MM-DD` or 7-byte `YYYY-MM` prefix from string start.
- `DatePoint::parse_with_pattern(text, pattern)` (`src/temporal/point.rs:158-181`) [V]: Parses arbitrary formats using Jiff `strtime` broken-down time [V: 755].

### 2.3 Periodic Notes and `file.day` Derivation

- `FileMeta::derive_day(path, created_at)` (`src/file.rs:88-98`) [V]:
  - Reads `BaseNameRef::from_path(path)` (the file stem without extension or parent folders) [V: 92].
  - Calls `DatePoint::parse_prefix(name)` to extract date from the prefix of the file name [V: 93].
  - Falls back to `created_at` timestamp if name parsing fails [V: 97].
  - Comment: *"filename-derived date winning over created_at (the Dataview rule)"* [V: 87].
- No periodic note plugin or configuration exists in Traces (`src/config/` has discovery, tracker, trust, error; no daily/periodic settings) [V]. Periodic note handling in Traces relies purely on `file.day` and field values [V].

### 2.4 Template Engine Link and Date Handling

- `src/template/engine/query.rs:603` [V]: In the minijinja bridge, `NoteFieldValue::Link(link)` is converted via `Value::from(link.target().to_owned())`.
- In templates, links lose their link wrapper and become raw target strings [V]. Consequently, `{{ note.due | date_format("%Y") }}` already works on links today, because minijinja receives the target string and pipes it to `DatePoint::parse` [V: `date.rs:19`].

---

## 3. Precedent in the Obsidian / PKM Ecosystem

### 3.1 Dataview

- `date(any)` constructor (`src-digest.txt:6865-6909`) [V]:

  ```ts
  .add1("link", (link, c) => {
      if (link.display) {
          let parsedDate = EXPRESSION.date.parse(link.display);
          if (parsedDate.status) return parsedDate.value;
      }
      let parsedDate = EXPRESSION.date.parse(link.path);
      if (parsedDate.status) return parsedDate.value;
      let resolved = c.linkHandler.resolve(link.path);
      if (resolved && resolved.file?.day) return resolved.file.day;
      return null;
  })
  ```

  - Priority chain: (1) `link.display`, (2) `link.path`, (3) Target file's `file.day` via index resolution [V: 6874-6887].
  - Tests explicitly verify: `date([[2020-04-18]])` -> `2020-04-18`, and `date([[Place|2021-04]])` -> `2021-04-01` [V: 9863-9864].
- `findDate(file, fields)` (`src-digest.txt:3631-3648`) [V]:
  - If field `date` or `day` is a Link, it extracts date via `extractDate(link.path) ?? extractDate(link.subpath) ?? extractDate(link.display)` [V: 3642].
  - Falls back to `extractDate(getFileTitle(file))` [V: 3647].
- `extractDate(str)` (`src-digest.txt:13868-13879`) [V]: Regex scan for `(\d{4})-(\d{2})-(\d{2})` or `(\d{4})(\d{2})(\d{2})` anywhere in the string.

### 3.2 Metadata Menu

- `validateValue` (`src-digest.txt:12406-12424`) [V]: Unwraps link strings before validating against Moment date format:
  `value.replace(/^\[\[/g, "").replace(/\]\]$/g, "").split("|").first()?.split("/").last()`.
  - Strips brackets, isolates target before `|`, isolates leaf basename after `/`, then parses via `moment(leaf, dateFormat).isValid()` [V: 12413, 12419].

### 3.3 Other Tools

- **Obsidian Periodic Notes / Daily Notes:** Users define Moment format strings (`YYYY-MM-DD`, `gggg-[W]ww`, `YYYY-MM`). Notes frequently reside in subfolders (`daily/2025-01-15.md`). Links in frontmatter/body take shapes: `[[2025-01-15]]`, `[[daily/2025-01-15]]`, `[[2025-01-15|Today]]`, `[[2025-W03]]`.
- **Tasks Plugin:** Tasks uses emojis (`📅 2025-01-15`). When Dataview format is used (`[due:: [[2025-01-15]]]`), users expect Dataview queries to filter tasks by date.
- **Logseq:** Journal pages are named by date (`2025-01-15` or `Jan 15th, 2025`). All date references are wikilinks. Logseq has no separate Date primitive; dates are page references. Built-in Datalog queries use `(between ?d ?start ?end)` to parse journal titles into timestamps.
- **Foam / Dendron:** Foam uses `journal/YYYY-MM-DD.md`. Dendron uses dot-notation `daily.2025.01.15.md`.

---

## 4. Evaluation of Four Design Options for Traces

| Evaluation Dimension | (a) Explicit `date(link)` | (b) Implicit Link-vs-Temporal Coercion | (c) Derived `.date` Attribute | (d) Index Link Resolution |
| :--- | :--- | :--- | :--- | :--- |
| **Purity & Determinism** | Pure function, 0 I/O [P]. | Pure comparator, 0 I/O [P]. | Pure accessor, 0 I/O [P]. | **Impure:** requires index lookup [H]. |
| **Type-Bracket Rule (L1)** | **Preserved:** Link is in Link bracket; `date()` produces Temporal [P]. | **Breached:** Cross-bracket coercion pollutes Link & Temporal brackets [H]. | **Preserved:** Link stays in Link bracket; `.date` is Temporal [P]. | **Preserved:** Link stays in Link bracket; resolution produces Date [P]. |
| **Sort / Filter Consistency** | Consistent: `sort date(due)` matches `where date(due) < ...` [P]. | **Inconsistent:** Filters coerce, but does SortKey sniff? If yes, violates L2; if no, sorts divergence [H]. | Consistent: `sort due.date` matches `where due.date < ...` [P]. | Inconsistent: Sort cannot resolve links without query-wide index join [H]. |
| **Folder-qualified (`[[daily/2025-01-15]]`)** | Handled via target leaf extraction [P]. | Handled via target leaf extraction [P]. | Handled via target leaf extraction [P]. | Handled by resolver [V]. |
| **Non-ISO (`2025-W03`, `15-01-2025`)** | Handled via `date(link, "%d-%m-%Y")` [P]. | **Fails:** Implicit coercion cannot accept format string [H]. | **Fails:** Static attribute cannot accept format string [H]. | Depends on target file frontmatter [H]. |
| **Missing / Future Daily Notes** | Works: Parses link target string directly [P]. | Works: Parses link target string directly [P]. | Works: Parses link target string directly [P]. | **Fails:** Target note does not exist yet; yields Null [H]. |
| **Display Aliases (`[[2025-01-15\|Today]]`)** | Target-authoritative: ignores alias (L6) [P]. | Target-authoritative: ignores alias [P]. | Target-authoritative: ignores alias [P]. | Ignores alias [P]. |

### Option (a): Explicit `date(link)` Function (No Implicit Coercion)

- **Mechanism:** Expand query function `date(args)` (`filter.rs:705`) to accept `NoteFieldValue::Link`.
  1. Extract link target leaf: strip anchor (`#...`), strip folder prefixes (take text after last `/`), strip optional `.md` extension.
  2. One-arg form `date(link)` parses leaf via `DatePoint::parse(leaf)`.
  3. Two-arg form `date(link, format)` parses leaf via `DatePoint::parse_with_pattern(leaf, format)`.
  4. Returns `NoteFieldValue::Date` (or `DateTime`), or `NoteFieldValue::Null` on failure (DuckDB `TRY_CAST` style).
- **Behavior:**
  - `where date(due) == 2025-01-15` -> matches both `due: 2025-01-15` (Date identity) and `due: "[[2025-01-15]]"` (Link parsed).
  - `where date(due) < 2025-02-01` -> evaluates inside the Temporal bracket.
  - `sort date(due)` -> sorts inside the Temporal bracket (rank 4), perfectly chronological across mixed Date and Link rows.
  - Raw `where due == 2025-01-15` remains `false` (strict type bracket).
  - Raw `sort due` sorts inside the Link bracket (rank 6) by folded target.

### Option (b): Implicit Coercion on Link vs Temporal Comparison

- **Mechanism:** When comparing `Link` against `Date`/`DateTime` in `CompareOp` or `is_equal_to_literal`, parse the link target leaf. If valid, evaluate as `Date` vs `Date`.
- **Fatal Flaws:**
  1. *Transitivity Failure:* If `due` is `[[2025-01-15]]`, `due == 2025-01-15` is true, and `due == [[2025-01-15]]` is true, but `2025-01-15 == [[2025-01-15]]` is an asymmetrical cross-bracket check.
  2. *Filter vs Sort Split:* If filters coerce `Link < Date`, what does `sort due` do? If `SortKey` does NOT sniff links, sorting by `due` puts `[[1990-01-01]]` (rank 6) AFTER `2025-01-15` (rank 4), contradicting the filter `where due < 2025-01-15`. If `SortKey` DOES sniff links, sort-time sniffing is resurrected (violating Lean 2).
  3. *Inability to handle non-ISO formats:* Vaults using `[[15-01-2025]]` or `[[2025-W03]]` cannot pass a format parameter to an operator like `<`.

### Option (c): Derived `.date` Attribute on Link (`due.date`)

- **Mechanism:** Expose a virtual attribute `.date` on links.
  - When accessed (`due.date`), it parses the target leaf as an ISO date.
  - Query: `where due.date == 2025-01-15`, `where due.date < 2025-02-01`, `sort due.date`.
- **Pros & Cons:** Elegant property syntax, zero size impact if virtual (evaluated on property access), preserves L1. However, cannot handle non-ISO formats without a format argument (which attributes cannot take), so it still requires Option (a) as a fallback.

### Option (d): Resolve Link to Target Note (`file.day` / frontmatter)

- **Mechanism:** Query engine resolves `link.target` against `LinkResolver`, fetches the target note's `FileMeta::day` or frontmatter `date`.
- **Fatal Flaws:**
  1. *Future Dates Broken:* In PKM, linking to future daily notes (`due: "[[2025-12-31]]"`) before the file exists is standard practice. Uncreated notes fail to resolve, returning `Null`.
  2. *Impurity & Performance:* Turns every comparison into an index lookup. Value comparison must remain pure and context-free (Lean 1).

---

## 5. Recommendation & Sharp Counterexamples

### 5.1 Recommendation: Option (a) with Target-Leaf Extraction

1. **Keep strict type bracketing (L1):** Do NOT introduce implicit cross-bracket coercion between `Link` and `Temporal`. `due == 2025-01-15` on a Link remains `false`.
2. **Extend `date(any)` in `src/query/grammar/filter.rs`:**
   - Add handling for `NoteFieldValue::Link(link)`.
   - Define canonical target leaf: strip anchor fragment (`#...`), take substring after the last `/` (ignoring folder paths), and strip trailing `.md`.
   - `date(link)` parses leaf via `DatePoint::parse(leaf)`.
   - `date(link, fmt)` parses leaf via `DatePoint::parse_with_pattern(leaf, fmt)`.
   - Return `Null` on parse failure (DuckDB `TRY_CAST` semantics).
3. **Target-Authoritative (Ignore Display Alias):**
   - Dataview parsed `link.display` before `link.path` [V: `src-digest.txt:6874-6881`].
   - Traces MUST NOT parse `display`. In Traces (Lean 6, ticket 18), link identity is the target. Display text is an alias (`[[2025-01-15|Tomorrow]]`, `[[2025-01-15|Meeting]]`). Extracting dates from display aliases causes semantic collision and alias-dependent date instability.
4. **Sorting:**
   - Raw `sort due` sorts in the Link bracket (rank 6) by folded target. For root ISO links (`[[2025-01-15]]`), this sorts chronologically for free.
   - For mixed notes (some Date, some Link), folder-qualified links (`[[daily/2025-01-15]]`), or non-ISO links, users write `sort date(due)`, which sorts in the Temporal bracket (rank 4).

### 5.2 Sharp Counterexamples & Edge Cases

1. **Folder-Qualified Links (`[[daily/2025-01-15]]`):**
   - *Behavior in raw sort:* Sorts under `d`, not `2`.
   - *Behavior in `date(due)`:* Stripping `daily/` yields `2025-01-15`, successfully parsing into a Date.
2. **Conflicting Display Alias (`[[2025-01-15|2025-01-16]]` or `[[Project|2025-01-15]]`):**
   - Under Dataview's display-first rule, `[[Project|2025-01-15]]` yields `2025-01-15` (even though the target is a project note).
   - Under Traces target-leaf rule: `[[Project|2025-01-15]]` yields `Null` (target `Project` is not a date). `[[2025-01-15|2025-01-16]]` yields `2025-01-15` (the actual daily note referenced).
3. **Non-ISO Periodic Notes (`[[15-01-2025]]`, `[[January 2025]]`, `[[2025-W03]]`):**
   - In raw sort: `15-01-2025` sorts by day of month.
   - Implicit coercion (Option b) fails completely.
   - Explicit `date(due, "%d-%m-%Y")` parses `15-01-2025` cleanly into `Date(2025-01-15)`.
4. **Future Periodic Notes (`[[2026-12-31]]` uncreated):**
   - Resolution against index (Option d) returns `Null`.
   - Lexical parsing in `date(link)` works immediately without requiring file creation.

---

## Implications for ticket 19

- **L1 (Type Bracketing in Filters, Rank Table in Sort):**
  - **Refine wording:** Confirm that `Link` and `Temporal` are strictly separate brackets. Cross-bracket `==` is false, and cross-bracket relational operators (`<, <=, >, >=`) never match (fixing the current bug where `Link > Date` evaluates to true).
  - Explicitly document: *A Link whose target is a date remains in the Link bracket. To compare or filter a date-holding link against a date, queries use the explicit cast function `date(field)`.*
- **L2 (Delete Sort-Time Sniffing):**
  - **Keep wording:** Delete `TextShape::classify` from `SortKey` unconditionally. Do NOT introduce link-target sniffing to `SortKey`.
  - Links sort in the Link bracket by folded target string. ISO links (`[[2025-01-15]]`) sort chronologically among themselves for free. Sorting across heterogeneous Date/Link fields requires `sort date(field)`.
- **L3 (Numeric Comparison):**
  - **Keep:** Unaffected.
- **L4 (Civil Model for Naive Datetimes):**
  - **Refine wording:** `date(link)` produces a civil `DateValue` when parsing `YYYY-MM-DD` or `YYYY-MM`, and a civil `DateTime` when parsing naive timestamps, adhering to L4 without ambient zone pollution.
- **L5 (Quoted Frontmatter Scalars):**
  - **Refine wording:** Frontmatter links MUST be quoted in YAML (`due: "[[2025-01-15]]"`). Reaffirm ticket 18's classification precedence: wikilink syntax `[[...]]` is recognized before date sniffing, ensuring quoted links become `NoteFieldValue::Link` and are never swallowed as strings or dates.
- **L6 (Link Identity on Folded Target + Subpath Only):**
  - **Refine wording:** In strict adherence to L6, date extraction from links in `date(link)` operates SOLELY on the link's target leaf (stripping folders, anchors, and `.md`). It MUST NOT inspect `display` text or aliases.
- **L7 (Durations Nominal Seconds):**
  - **Keep:** Unaffected.
- **L8 (Strings Unicode-Based Sorting):**
  - **Keep:** Unaffected. Links continue to sort by folded target in the Link bracket.
