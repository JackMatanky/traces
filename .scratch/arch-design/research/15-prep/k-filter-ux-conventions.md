# Filter UX Conventions for CLI & PKM: Beyond the PostgreSQL Unknown-Literal Rule

- **Target File:** `.scratch/arch-design/research/15-prep/k-filter-ux-conventions.md`
- **Scope:** Survey of date, duration, link, and string filtering conventions across PKM and CLI ecosystems to evaluate Proposal Q11 (operand-adaptive string literals) and identify high-leverage query UX patterns for Traces.
- **Status:** Complete.
- **Tags:** `[V]` verified (file:line, official doc URL, or digest probe), `[H]` inference, `[P]` proposal.

---

## 1. Executive Summary & The Core Tension

Proposal Q11 replaces parse-time sniffing (`ComparisonExpr::classify_literal`, `src/query/grammar/filter.rs:238` [V]) with PostgreSQL-style operand-adaptive string literal coercion: an untyped quoted string literal in a comparison adopts the type of the other operand per row when valid, else stays a string [P].

**The Core Tension:** Relational databases execute queries against fixed schemas where `'2025-01-15'` resolves against a known column type at plan time, throwing immediate parse errors on invalid inputs (`'2025-02-30'::date`) [V]. In contrast, personal knowledge management (PKM) vaults are open-world, heterogeneous, and schemaless:
1. **The Silent-False Typo Trap:** If adaptive coercion silently falls back to `String` when an invalid date or typo is supplied (e.g. `due == "2025-02-30"`), type bracketing (Lean L1) evaluates `Date == String` as `false` [V, H]. The query silently returns 0 rows instead of alerting the user to the bad literal [H].
2. **Point vs Interval Mismatch:** Dates in markdown are often granular (e.g. `due: 2025-01` for a month goal). An adaptive literal rule coerces `"2025-01"` to a single `civil::Date` point (Jan 1), causing `due == "2025-01"` to miss Jan 15 [V, H].
3. **Shell Friction:** PKM users query via CLI arguments (`--where '...'`). Forcing nested quoting (`--where 'due >= "2025-01-01"'`) creates high syntactic noise compared to bare literals (`due >= 2025-01-01`) [V, H].

---

## 2. Cross-Ecosystem Filter Ergonomics Matrix

| Ecosystem / Tool | Date Literals | Duration Literals | Relative Dates | Range / Membership | Link Filtering | Null / Missing Trap |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Traces Today** [V: `filter.rs`] | Sniffed string or bare ISO [V:1216] | Word `4h15m` [V:72] or sniffed | Funcs `date_add`, `dur`, `sow` [V:258] | None (need `&&`/`\|\|`) | None (lexical target text) | Relational leaks rank [V:807] |
| **Obsidian Tasks** [V: `digest:609`] | Bare `YYYY-MM-DD`, `YYYY-mm` | Direct offsets `in 7 days` | Natural: `today`, `tomorrow`, `next week` | `YYYY-MM-DD YYYY-MM-DD` | None | Inverted sort: `non-null < null` [V] |
| **Obsidian Dataview** [V: `digest:4858`] | `date(today)`, `date(2025-01-15)` | `dur(1 day)`, `dur(2yr8mo)` | `date(today)`, `date(now)`, `date(sow)` | None (needs `x >= a AND x <= b`) | `[[Note]]` literals [V:4835] | `null <= date(today)` is True [V:3795] |
| **Obsidian Bases** [V: `bases-syntax.md`] | `date("2025-01-15")`, `today()` | String `now() - "1 week"` [V:288] | Global funcs `today()`, `now()` [V] | `and`/`or` list blocks | `file.hasLink("Note")` [V] | JS null/truthy semantics [V] |
| **zk** [V: `zk-docs-digest:1509`] | Bare ISO, human string `Feb 3` | Flags `--created-after '2 weeks'` | `yesterday`, `last tuesday` [V:1498] | Flag pairs: `--created-after/-before` | `--link-to`, `--linked-by` [V:1526] | Ignored in search flags |
| **Jira JQL** [V: `jql-guide.md`] | Quoted `"2025/01/15"` | Direct offsets `-7d`, `2w`, `-1M` [V:104] | `startOfDay()`, `endOfMonth()` [V:118] | `in (a, b)`, `DURING (d1, d2)` [V:33] | N/A | `IS EMPTY` / `IS NOT EMPTY` [V] |
| **Elasticsearch** [V: `common-options`] | Formatted string with `\|\|` | Math `+1d`, `-2w`, `/M` [V] | `now/d`, `now-1d/d` [V] | Range `gte`/`lte` with rounding [V] | N/A | `must_not` / `exists` query |
| **Nushell** [V: `nushell.sh`] | Bare RFC3339 `2025-01-15` [V] | Bare tokens `1hr`, `3day`, `2wk` [V] | `(date now) - 1day` | `a..b` range operator [V] | N/A | Type errors on mismatched types |
| **DuckDB** [V: `duckdb.org`] | `DATE '2025-01-15'` | `INTERVAL 1 DAY`, `INTERVAL '1 mo'` | `current_date`, `today()` | `BETWEEN a AND b`, `IN (...)` [V] | N/A | 3VL SQL logic (`NULL` falsy) |
| **Taskwarrior** [V: `taskwarrior.org`] | Bare `YYYY-MM-DD`, `2025-01` | Modifiers `due:2d`, `due:-1w` | `today`, `yesterday`, `eom`, `sow` [V] | Modifier `.after:`, `.before:` [V] | N/A | Attributes absent if unset |
| **fd / git** [V: `man fd`] | `--changed-within 2weeks` | N/A (relative interval arg) | `--since='2 weeks ago'` (git) | Date/interval flag arguments | N/A | N/A |

---

## 3. Catalogue of 10 Filter UX Conventions & Adaptive Literal Interlock

### 1. Partial-Date Granularity & Range Rounding
- **Precedent:** Obsidian Tasks supports `YYYY-mm`, `YYYY-Www`, `YYYY-Qq`, `YYYY` [V: `digest:609`]. Taskwarrior matches partial dates via prefix/granularity (`due:2025-01` matches entire month) [V: `task.1`]. Elasticsearch rounds date math: `gte` rounds *down* to start of unit (`2025-01||/M` -> `2025-01-01T00:00:00`), while `lte` rounds *up* to end of unit (`2025-01||/M` -> `2025-01-31T23:59:59.999`) [V: `query-dsl-range-query.html`].
- **Traces Today:** `DatePoint::parse` recognizes `DateShape::YearMonth` and flags `DatePrecision::Month` (`src/temporal/point.rs:58` [V]), but `TextShape::classify` strips precision, setting the day to 1 (`sort.rs:466` [V]). `due == "2026-07"` only matches notes due exactly on July 1st [V].
- **Adaptive Literal Interlock:** **FAILS.** If `"2025-01"` adapts to `civil::Date`, a single date point cannot represent a 31-day month [H]. Equality `due == "2025-01"` must desugar to an interval check: `due >= 2025-01-01 && due <= 2025-01-31` [P]. For inequalities, ES rounding rules apply: `due <= "2025-01"` means `<= 2025-01-31`, while `due >= "2025-01"` means `>= 2025-01-01` [P].

### 2. Relative Date Keywords & Injected Context
- **Precedent:** Dataview (`date(today)`, `date(sow)`) [V: `digest:4872`]. Tasks (`due before tomorrow`, `due in next week`) [V: `digest:220`]. JQL (`startOfWeek()`, `endOfMonth()`) [V: `jql-history-and-dates.md:118`]. Taskwarrior (`eom`, `sow`, `today`, `yesterday`) [V: `dates`].
- **Deterministic Pure Engine Requirement:** Ambient time (`Local::now()`) must never leak into query evaluation [V: `brief.md:90-95`]. `ExecutionContext` must inject `today: civil::Date` and `now: civil::DateTime` [P].
- **Adaptive Literal Interlock:** **HIGH SYNERGY.** If string adaptation supports relative names (`"today"`, `"yesterday"`, `"tomorrow"`, `"sow"`, `"eom"`), `--where 'due <= "tomorrow"'` adapts `"tomorrow"` against `due: Date` using `context.today + 1d` [P]. Crucially, if the target field is `String` (`title == "today"`), `"today"` remains an exact string match [P].

### 3. Range Syntax (`..`) and Set Membership (`in`)
- **Precedent:** GitHub search (`created:2025-01-01..2025-01-31`) [V: `syntax`]. Nushell (`due in 2025-01-01..2025-01-31`) [V: `types_of_data`]. SQL / JQL (`status in ("todo", "in-progress")`) [V: `jql-history-and-dates.md:143`].
- **Adaptive Literal Interlock:** **COMPLEMENTARY.** An adaptive literal rule only helps single comparisons (`left == right`). It cannot express set membership: writing `status == "a" || status == "b" || status == "c"` is tedious in CLI filters [H]. `status in ("a", "b", "c")` desugars cleanly. For ranges, `due in 2025-01-01..2025-01-31` allows adapting both bounds [P].

### 4. String Operators & Case Folding
- **Precedent:** Dataview provides `contains(str, sub)`, `icontains(str, sub)`, `startswith(str, pre)`, `regexmatch(pat, str)` [V: `digest:4239, 4548, 4485`]. Bases provides `string.containsAny()` and `string.lower()` [V: `bases/functions.md:237, 275`]. zk provides `--match-strategy re` and `exact` [V: `zk-docs-digest:1318`].
- **Adaptive Literal Interlock:** **INDEPENDENT.** Q11 only adapts strings to non-string types. It does nothing for string-vs-string matching [H]. Case-insensitive matching (`tag == "rust"` matching `"Rust"`) requires either a case-folding operator (`~=` / `=~`) or ICU-aware string collation [V: `e-query-engine-conventions.md:87`].

### 5. Existence & Type Predicates (The Null Trap)
- **Precedent:** In Dataview, `null < date(today)` is `true`, forcing users to guard every query with `typeof(due) = "date" AND due <= date(today)` [V: `digest:3795`]. Tasks inverts this so `non-null < null` [V: `digest:5151`]. JQL provides `field IS EMPTY` / `IS NOT EMPTY` [V: `jql-history-and-dates.md:157`].
- **Traces Lean L1 Defense:** Traces avoids Dataview's trap by design: under Lean L1 (MongoDB type bracketing), relational ops on `Null` always evaluate to `false` (no match) [V: `e-query-engine-conventions.md:58`]. Missing or null fields never match `<= "today"`.
- **Adaptive Literal Interlock:** **NEUTRAL.** `exists(field)` or `field != null` is still necessary when a user wants to find notes where a field was explicitly defined regardless of its value [P].

### 6. Duration Literals & Arithmetic Ergonomics
- **Precedent:** Dataview (`dur(1 day)`, `dur(2yr8mo)`) [V: `digest:1012`]. Bases directly allows string durations in arithmetic: `file.mtime > now() - "1 week"` [V: `bases-syntax.md:288`]. Nushell uses bare tokens `1hr`, `3day` [V: `duration.html`]. DuckDB uses `INTERVAL 1 DAY` [V: `interval`].
- **Traces Today:** Tokenizes unquoted duration words (`4h15m`) via `DurationValue::parse` (`filter.rs:72` [V]).
- **Adaptive Literal Interlock:** **EXPANSION NEEDED.** Q11 was specified for binary comparisons (`==`, `<`), but users frequently write arithmetic: `file.mtime >= now() - "2w"` [P]. In `DateTime - String`, the string literal must adapt to `Duration` based on arithmetic operator expectations (`-` on DateTime expects Duration) [P].

### 7. Link Literals & Wikilink-vs-String Matching
- **Precedent:** Dataview parses bare `[[Note]]` as link literals [V: `digest:4835`]. Metadata Menu strips brackets and aliases when matching link targets against date/select fields (`AbstractDate.validateValue`) [V: `digest:12406`]. zk uses `--link-to <target>` matching path or filename [V: `digest:1527`].
- **Adaptive Literal Interlock:** **EXCELLENT.** If a note frontmatter holds `project: "[[Alpha]]"` (parsed as `NoteFieldValue::Link`), a query `--where 'project == "Alpha"'` adapts the string literal `"Alpha"` to a `Link` with target `"Alpha"`, matching the field cleanly [P]. Symmetrically, if note frontmatter holds `project: Alpha` (plain string), `"Alpha"` stays a string, matching string equality [P]. This eliminates the need for special link wrapper functions in standard queries [P].

### 8. Explicit Casts: Why Users Still Need Them
- **Precedent:** Dataview provides `string(x)`, `date(x)`, `number(x)`, `dur(x)` [V: `digest:6865-6971`]. DuckDB provides `TRY_CAST(x AS DATE)` [V: `duckdb.org`].
- **User Critique Addressed:** "If literals adapt, why is `string()` needed at all?"
  - **Reason 1: Field-to-Field Comparisons.** If note field `a` is Date and note field `b` is String, neither operand is a literal [H]. Q11 does NOT adapt fields [P]. Comparing them requires `a == date(b)` or `string(a) == b`.
  - **Reason 2: Overriding Frontmatter Heuristic Classification.** An unquoted frontmatter scalar `isbn: 9780131103627` or `title: 2026-07-29` is heuristically parsed as `Int` or `Date` at ingestion (Ticket 14 Answer [V: `14:501`]). To perform string operations (e.g. `startswith(string(isbn), "978")`), the user must cast [H].
  - **Reason 3: Formatting & Output.** MiniJinja template rendering and table column formatting require converting typed temporals/durations into formatted strings (`date.format("%Y-%m-%d")`) [V: `bases/functions.md:144`].
  - **Conclusion:** Adaptive literals eliminate 90% of casts in interactive CLI filters, but explicit casts (`date()`, `string()`) remain mandatory for expressions, field comparisons, and formatting [P].

### 9. CLI Shell-Quoting Ergonomics for `--where`
- **Precedent:** CLI tools suffer severe usability friction when users must escape inner quotes inside shell arguments (e.g. `traces find --where "due <= \"2025-01-15\""` or `'due <= "2025-01-15"'`) [H].
- **Mitigations across tools:**
  - Bare ISO dates without quotes: Nushell (`2025-01-15`), Traces today (`FilterToken::DateLiteral` regex in `filter.rs:1216` [V]).
  - Bare duration units without quotes: Traces today (`1h30m`), Nushell (`3day`), JQL (`-7d`).
  - Flag-based alternatives: zk (`--created-after 2025-01-01`), fd (`--changed-within 2weeks`).
- **Adaptive Literal Interlock:** **POTENTIAL TRAP.** If Traces *requires* quotes for literals to adapt, users will constantly trip over shell quote escaping. Bare tokens (`2025-01-15`, `2w`, `today`) MUST be first-class tokens that bypass quotes entirely [P].

### 10. Diagnostic UX & The Silent-False Typo Trap
- **Precedent:**
  - Dataview: Silent false/null traps require prominent warning callouts in official documentation (`digest:3794-3803` [V]).
  - Taskwarrior: Emits explicit warnings when filter syntax fails to parse or matches no tasks [V: `faq`].
  - Jira JQL: Hard syntax errors with exact column pointers when a date format or function parameter fails [V: `jql-history-and-dates.md:168`].
- **The Typo Trap under Q11:**
  - Query: `--where 'due == "2025-02-30"'` (invalid leap year date) or `--where 'due == "2025-01-15T"'`.
  - Under naive Q11: Literal fails to parse as Date -> falls back to `NoteFieldValue::String("2025-02-30")` -> Type bracketing evaluates `Date == String` as `false` [V, H].
  - Result: 0 matches returned, no error, user assumes no tasks are due [H].
- **Diagnostic UX Requirement:** If an operand on a row is `Date`, and a string literal in comparison with it *resembles* a date (`^\d{4}-\d{2}`) but fails validation, query evaluation should emit an advisory stderr diagnostic: `warning: literal "2025-02-30" looks like a Date but is invalid calendar date; compared as String (evaluates to false)` [P].

---

## 4. Implications for Ticket 15

### 4.1 Top 5 Recommended Options for the User

1. **Operand-Adaptive String & Link Literals (Refined Q11)**
   - *Syntax:* `--where 'due >= "2025-01-01" && project == "Alpha"'`
   - *Cost:* Low-Medium (Query binder / comparator check).
   - *Impact:* Quoted literals adopt Date/DateTime/Duration/Link when comparing against that type. Eliminates explicit `date()`/`link()` boilerplate in 90% of user queries.
2. **First-Class Bare Date & Duration Tokens (Shell Quoting Relief)**
   - *Syntax:* `--where 'due >= 2025-01-01 && file.mtime > now() - 14d'`
   - *Cost:* Low (Keep existing `FilterToken::DateLiteral` [V:1216] and duration lexer [V:72]).
   - *Impact:* Zero nested quotes needed inside `--where '...'`. Eliminates shell-escaping friction entirely.
3. **Partial-Date Granular Equality & Relational Rounding**
   - *Syntax:* `--where 'due == "2025-01"'` (month) or `--where 'due == "2025"'` (year)
   - *Cost:* Medium (Desugar equality to `[start, end]` interval; apply ES `gte`/`lte` rounding on inequalities).
   - *Impact:* Matches natural PKM note metadata where due dates and milestones are set at month or year granularity.
4. **Context-Injected Relative Date Identifiers**
   - *Syntax:* `--where 'due <= today || due <= eom'`
   - *Cost:* Low (Resolve identifiers `today`, `yesterday`, `tomorrow`, `sow`, `eow`, `som`, `eom` from `ExecutionContext`).
   - *Impact:* Extreme ergonomic boost for task and journal management without leaking non-deterministic ambient time into the core engine.
5. **Set Membership (`in`) Operator**
   - *Syntax:* `--where 'status in ("todo", "in-progress", "blocked")'`
   - *Cost:* Low (Lexer/parser desugars `x in (a, b)` to `x == a || x == b`).
   - *Impact:* Prevents verbose, error-prone chains of `||` equality comparisons in CLI arguments.

### 4.2 Deliberate Non-Adoptions

1. **DO NOT Adopt Natural Language Date Parsing (`"due before in two weeks"`, `"last tuesday"`):**
   - *Rationale:* Obsidian Tasks and zk use heavy, non-deterministic NLP libraries (Chrono.js, dateparser) [V: `digest:849`]. They create locale ambiguity, timezone drift, and parser bloat. Strict ISO 8601 plus fixed keywords (`today`, `yesterday`, `eom`, offsets `+7d`) is deterministic, portable, and fast.
2. **DO NOT Adopt Dataview-Style Permissive Total Ordering for Filters (`42 < "hello"` is True):**
   - *Rationale:* Total ordering in filters leaks internal rank tables into queries [V: `e-query-engine-conventions.md:43`]. MongoDB type bracketing (Lean L1) is strictly superior for open-world vaults.
3. **DO NOT Adopt Dataview's Null Trap (`null <= date` is True):**
   - *Rationale:* Breaks basic user intuition. Lean L1's rule (relational operators poison `Null` to `false`) is far more defensive and prevents accidental inclusion of undated notes [V: `e-query-engine-conventions.md:58`].
4. **DO NOT Adopt Silent Coercion Failure Without Advisory Warnings:**
   - *Rationale:* Silently treating malformed dates (`"2025-02-30"`) as strings causes silent query misses under type bracketing. An advisory warning on stderr is essential for user trust [P].

### 4.3 Concrete Recommendation & Decision-Ready Position

**Position:** **Accept Refined Proposal Q11 with Bare Literals and Stderr Advisory Hints.** [P]

- **Evaluation Rule:** In a binary comparison `left OP right`:
  1. If one operand is typed (`Date`, `DateTime`, `Duration`, `Link`) and the other is a string literal (quoted or bare):
     - Attempt parsing the string as the target type (using `ExecutionContext` for relative keywords/anchors).
     - If parsing succeeds: evaluate comparison in the target type bracket.
     - If parsing fails: check if the string resembles the target type (e.g. ISO date prefix). If so, log an advisory diagnostic to stderr; then fallback to `String` (evaluating to `false` under Lean L1).
  2. If both operands are bare/quoted string literals: stay `String`.
  3. If both operands are fields: evaluate strictly within type brackets; no implicit coercion across non-literal fields (require explicit `date()`, `string()` casts).
- **Synergy:** Solves link matching (`project == "Alpha"` matches `[[Alpha]]`), date matching (`due >= "2025-01-01"`), and relative querying (`due <= "today"`), while retaining strict type safety and high performance across the Postcard index cache.
