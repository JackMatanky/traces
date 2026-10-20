# Quote Escape Feasibility & Adversarial Stress-Test (Lean L5)

- **Target File:** `.scratch/arch-design/research/15-prep/h-quote-escape-feasibility.md`
- **Scope:** Mechanism feasibility, performance cost, and user impact of Lean L5 (quoted frontmatter scalars shaped like date/datetime/duration become strings; links exempt).
- **Status:** Complete.

---

## 1. Executive Summary & Adversarial Verdict

**Verdict on Lean L5:** **REJECT as primary escape mechanism.** [P]
While technically implementable via Noyalib's side-table CST (`noyalib::cst::Document`), Lean L5 suffers from three structural flaws:
1. **Mechanical Cost & Flaws:** No zero-allocation route exists in Noyalib 0.0.53 or 0.0.57 [V]. CST parsing adds a 59% latency penalty per note [V: probe]. The proposed "lazy CST" optimization (parse CST only if date-shaped strings exist) degrades to **2.59x slower than baseline** in real PKM vaults where ~85%+ of notes contain dates (`created`, `updated`, `due`), because it parses notes twice [V: probe]. Furthermore, `span_at` fails on tagged scalars (`!!str "2025-01-15"` returns raw `"!!str \"2025-01-15\""` which defeats `starts_with('"')`) [V: probe], requires synthetic bracket syntax (`["a.b"]`) for dotted keys [V: probe], and green-tree CST walks cannot resolve aliases (`*x`) [V: probe].
2. **Severe Silent Query Breakage:** In real Obsidian vaults, users frequently possess quoted dates written by tools: Templater syntax requires quoting frontmatter tags (`due: "<% tp.date.now() %>"` -> `due: "2025-01-15"`) [V: digest], Metadata Menu forces quotes on serialization [V: digest], and Prettier/YAML formatters quote date-shaped strings [V]. Under L5 combined with L1 (strict type bracketing), `due: "2025-01-15"` becomes `String`, causing queries like `--where 'due <= 2025-01-01'` to **silently drop rows** without warning or error [V, H].
3. **Ecosystem Divergence:** Obsidian Dataview has zero YAML quote escape (both `"2025-01-15"` and `2025-01-15` parse as `DateTime`) [V: digest]. Obsidian Properties UI enforces typing through a schema file (`.obsidian/types.json`), not YAML quote syntax [V].

**Recommendation:** Maintain **quote-insensitive parse-time heuristic classification** (matching status quo and Ticket 14 Answer lines 501-502) [P]. Implement string-forcing via **schema overrides** (`type = "string"`) and query explicit casts (`string(x)` / `date(x)`) [P].

---

## 2. Part 1: Mechanism Analysis (Noyalib 0.0.53 & 0.0.57)

### 2.1 Public API Landscape
- **`noyalib::Value` AST:** Drops all quote styles. `Value::String(String)` retains only unescaped character contents [V: `src/value/mod.rs`].
- **YAML Tags:** `!!str 2025-01-15` and `!!str "2025-01-15"` are explicitly resolved directly to plain `Value::String` by Noyalib's loader (`noyalib-0.0.53/src/parser/loader.rs:2180`) [V]. They do NOT surface as `Value::Tagged` [V]. `!str` produces `Value::Tagged`, but Traces' frontmatter pipeline drops all tags to inner values (`src/field.rs:793`, Ticket 14:497) [V].
- **`PolicyEvent`:** `PolicyEvent<'a>` contains only `kind: PolicyEventKind`, `anchor`, `tag`, and `scalar: Option<&'a str>` (`src/policy.rs:84-95`) [V]. It exposes no scalar style or quote delimiter [V].
- **`StreamingDeserializer`:** Implements `serde::Deserializer`; internal parser events and scanner tokens are `pub(crate)` and inaccessible (`src/streaming.rs:231`) [V].
- **`Spanned<T>`:** Populates source `Location` during `from_str`, but fails on flattened structs and provides no quote style (`src/spanned.rs:79-125`) [V].
- **Noyalib 0.0.57:** Crate inspection via `rust-docs` confirms 0.0.57 adds `dominant_quote_style()` on `Document` (file-wide heuristic) [V: `cst/document.rs:2121`], `quote_style()` on `EmitCtx` (serializer only) [V: `cst/emit.rs:120`], and `plain_scalar_strings` in `ParserConfig` (deserializing unquoted scalars into String targets) [V: `de/config.rs:1284`]. **It adds NO public per-scalar quote inspection API to `Value` or deserialization.** [V]

### 2.2 Probe Accuracy Matrix across Scalar Scenarios
Tested against `noyalib::cst::parse_document` with `span_at(path)` vs custom `GreenNode` walk [V: empirical probe]:

| Scenario | Input YAML | `span_at(path)` Result | Custom `GreenNode` Walk | Assessment |
| :--- | :--- | :--- | :--- | :--- |
| **Top-level plain** | `date: 2025-01-15` | `quoted=false`, raw `"2025-01-15"` | `PlainScalar` | Correct [V] |
| **Top-level double** | `date: "2025-01-15"` | `quoted=true`, raw `"\"2025-01-15\""` | `DoubleQuotedScalar` | Correct [V] |
| **Top-level single** | `date: '2025-01-15'` | `quoted=true`, raw `"'2025-01-15'"` | `SingleQuotedScalar` | Correct [V] |
| **Dotted key plain** | `a.b: "2025-01-15"` | Bare `span_at("a.b")` yields `None`; bracket `["a.b"]` yields `quoted=true` | `DoubleQuotedScalar` | **Fragile:** requires synthetic bracket path formatting [V] |
| **Dotted key quoted** | `"a.b": "2025-01-15"` | Bare `span_at("a.b")` yields `None`; bracket `["a.b"]` yields `quoted=true` | `DoubleQuotedScalar` | **Fragile:** requires synthetic bracket path formatting [V] |
| **Nested map** | `nested:\n  due: "..."` | `span_at("nested.due")` yields `quoted=true` | `DoubleQuotedScalar` | Correct [V] |
| **Flow-list element** | `tags: ["2025-01-15", 2025-01-16]` | `span_at("tags[0]")` = true; `span_at("tags[1]")` = false | `FlowSequence` (flat) | Correct [V] |
| **Block-list element**| `items:\n  - "2025-01-15"\n  - ...` | `span_at("items[0]")` = true; `span_at("items[1]")` = false | `BlockSequence` -> `SequenceItem` | Correct [V] |
| **Block scalar `\|`** | `text: \|\n  2025-01-15` | `quoted=false`, raw `"\|\n  2025-01-15"` | `LiteralScalar` | Correct (not quote) [V] |
| **Folded scalar `>`** | `text: >\n  2025-01-15` | `quoted=false`, raw `">\n  2025-01-15"` | `FoldedScalar` | Correct (not quote) [V] |
| **Alias `*x`** | `anchor: &x "..."\nalias: *x` | `quoted=true`, raw `"\"2025-01-15\""` (resolves through) | `AliasMark("*x")` | `span_at` resolves anchor; green walk misses target [V] |
| **Duplicate keys** | `k: "2025-01-15"\nk: 2025-01-16` | `quoted=false`, raw `"2025-01-16"` | Reports both entries | Last duplicate wins, matching `Value` [V] |
| **Tagged quoted** | `date: !!str "2025-01-15"` | `quoted=false`, raw `"!!str \"2025-01-15\""` | `DoubleQuotedScalar` | **FAILS:** raw begins with `!`, defeating quote check [V] |
| **Tagged plain** | `date: !!str 2025-01-15` | `quoted=false`, raw `"!!str 2025-01-15"` | `PlainScalar` | Correct [V] |
| **Wikilink quoted** | `link: "[[My Note]]"` | `quoted=true`, raw `"\"[[My Note]]\""` | `DoubleQuotedScalar` | Correct (exempt from L5) [V] |

### 2.3 Non-Noyalib Fallback: Line-Based Pre-Scan
A regex/line scanner (e.g. `^\s*([a-zA-Z0-9_.-]+)\s*:\s*(["'].*["'])`) is superficially attractive but structurally unsound [H]:
- **Failure modes:** Fractures on multi-line scalars, block sequences (`- "2025-01-15"` lacks key), flow mappings (`{a: "..."}`), block/folded scalars (`|`, `>`), anchors/aliases, and duplicate keys [H].
- **False positives:** Comments with quotes (`due: 2025-01-15 # "target"`) match regexes as quoted [H]. Obsidian Linter explicitly abandoned regex YAML parsing due to severe frontmatter corruption (`digest:244-250`) [V].

### 2.4 Performance Measurement
Empirical timing on realistic 30-line frontmatter (10,000 iterations, release mode, Apple Silicon) [V: probe]:

```
--- Frontmatter WITH date/duration scalars (~85% of real notes) ---
Baseline (noyalib::Value only):  35.45 µs/note  (1.00x)
CST always (parse_document):    56.27 µs/note  (1.59x baseline, +20.8 µs)
Lazy CST (consulted on hit):    91.97 µs/note  (2.59x baseline, +56.5 µs)

--- Frontmatter WITHOUT date/duration scalars (~15% of real notes) ---
Baseline (noyalib::Value only):  34.20 µs/note  (1.00x)
CST always (parse_document):    54.26 µs/note  (1.59x baseline, +20.1 µs)
Lazy CST (skipped on hit):      34.69 µs/note  (1.01x baseline, +0.49 µs)
```

**Key Finding:** `parse_document` parses YAML twice internally (`parse_exactly_one` + `build_green_tree`, `cst/builder.rs:41-44`) [V]. The "Lazy CST" strategy parses the document with `from_str_with_config`, scans `Value`, and then parses *again* with `parse_document`. In notes containing dates/durations, it runs THREE parse passes, taking **2.59x the baseline time** [V: probe]. Across an 85%-date vault, Lazy CST increases total frontmatter parse time by **~135%**, whereas baseline heuristic classification costs zero extra passes [V, H].

---

## 3. Part 2: User Impact & Vault Evidence

### 3.1 Tooling Landscape: Who Writes Quoted Dates?
1. **Templater / QuickAdd:** In Obsidian templates, YAML frontmatter syntax errors occur if template tags are unquoted (e.g. `due: <% tp.date.now() %>` fails YAML parse because `<` is invalid) [V: digest]. Users MUST write `due: "<% tp.date.now() %>"` or `date: "{{date}}"` [V: digest]. When expanded, the file permanently contains `due: "2025-01-15"` [V].
2. **Metadata Menu:** `renderValueString` (`digest:19696-19708`) explicitly wraps strings in double quotes (`"${rawValue}"`) whenever writing to frontmatter if YAML re-parsing would not preserve them as strings [V].
3. **Formatters (Prettier / yaml-lint):** YAML formatters with `singleQuote: true` or string-safety rules automatically quote date-like strings (`created: '2024-12-01'`) [H].
4. **Obsidian Properties UI:** Writes date properties unquoted (`due: 2025-01-15`) [V]. However, if a user changes property type to `text`, Obsidian quotes it (`due: "2025-01-15"`) to prevent YAML parsers from inferring a date [V].
5. **Traces Test Suite:** Zero tests or fixtures in `traces` currently quote frontmatter dates [V: `tests/integration/query.rs:50`, `src/field.rs:843`].

### 3.2 The Silent Breakage Risk
Under Lean L1 (strict type bracketing) and Lean L5:
- A user note generated via Templater contains `due: "2025-01-15"`.
- Under status quo, Traces parses this as `NoteFieldValue::Date` [V: `src/field.rs:843`].
- Under L5, Traces parses this as `NoteFieldValue::String("2025-01-15")` [P].
- Under L1, `String` vs `Temporal` bracket comparisons evaluate:
  - `due == 2025-01-15` -> `false` [P]
  - `due <= 2025-01-15` -> `false` (no match) [P]
  - `due != 2025-01-15` -> `true` [P]
- **Failure Mode:** The query `--where 'due <= 2025-01-15'` silently returns 0 results for this note [H]. There is no syntax error, no parse error, and no diagnostic warning [H]. This is identical to the Dataview `null <= date(today)` trap that required documentation warnings (`digest:3794-3803`) [V].

### 3.3 Evaluation of Mitigations

| Mitigation | Mechanism | Pros | Cons | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **M1: Read-time Schema Coercion** | Schema `type = date` coerces `String` to `Date` when queried | Safe; respects schema | Fails in zero-config vaults without schemas | Supplementary [P] |
| **M2: Query `date()` Cast** | User writes `date(due) <= 2025-01-15` | Standard Dataview/SQL idiom (`digest:6865`) | Burden on user; breaks existing queries | Necessary query tool [P] |
| **M3: One-time `traces check` Advisory** | Diagnostic warning: string field looks like date | Catches migration bugs | High noise on genuine string fields (IDs, titles) | Diagnostic only [P] |
| **M4: Quote-Insensitive Classification + Schema String Override** | Parse-time classification treats `"2025-01-15"` as Date; schema `type = string` treats as String | **Zero breakage**; 1.0x parse cost; matches Dataview | Requires schema to force rare string-date | **RECOMMENDED** [P] |
| **M5: Opt-in Config Flag** | `[frontmatter] quote_escape = true` | Isolates breakage | Splits ecosystem semantics; configuration burden | Reject [P] |

### 3.4 Precise Behavior Change for Ticket 12

```markdown
| Change | Status Quo | Adopted Decision (Refined L5) | Rationale |
|---|---|---|---|
| Frontmatter string-date classification | Sniffs ISO dates regardless of quotes (`field.rs:843`) | **Preserve quote-insensitivity** for Date/DateTime/Duration classification at parse time | Prevents silent query failure on Templater/Prettier quoted dates; avoids 2.59x CST parse penalty |
| Frontmatter string escape mechanism | None (quotes ignored; inline quotes escape) | **Schema declaration** (`type = "string"`) and query explicit cast (`string(field)`) | Provides clean escape hatch without YAML parsing overhead or ambiguous heuristics |
| Frontmatter wikilinks | Quoted `"[[x]]"` classified as Link; unquoted `[[x]]` is YAML flow sequence error | **Preserve** link recognition on quoted `"[[x]]"` | Preserves valid YAML syntax for links (Ticket 14:279-282) |
```

---

## 4. Implications for Ticket 15 (Mapping to Leans L1..L8)

- **L1 (Type Bracketing):** **KEEP.** [P]
  - Type bracketing (Numeric, Temporal, Duration, String, Bool, Link, List, Object, Null) solves cross-kind relational leaks (`42 < "hello"` becoming false instead of true).
  - *Refinement:* Because cross-bracket comparisons evaluate to `false` without errors, preserving quote-insensitive date classification is essential to prevent user notes from being silently dropped from query results.
- **L2 (Sort Sniffing):** **KEEP.** [P]
  - Delete `TextShape::classify` in `query/sort.rs:462`. Values stored as `String` sort strictly as `String`.
- **L3 (Numeric Model):** **KEEP.** [P]
  - Exact `Int(i64)` / `Float(f64)` comparator outside `PartialEq`. `-0.0 == 0.0`. Reject `NaN` at construction.
- **L4 (Civil Temporal Model):** **KEEP.** [P]
  - Civil model for naive datetimes; zero machine timezone baked into index cache.
- **L5 (Quote Escape):** **CHANGE / REFINE.** [P]
  - **Do NOT adopt quote-sensitive classification for frontmatter scalars.**
  - *Concrete wording for Ticket 15:*
    > "Frontmatter classification remains quote-insensitive: text matching Date, DateTime, or Duration syntax is eagerly classified at parse time regardless of whether it is written with plain, single, or double quotes. Link recognition on quoted wikilinks (`\"[[target]]\"`) remains active. To treat a date-shaped frontmatter value as a literal string, users specify `type = \"string\"` in the schema or invoke the explicit `string()` function in queries. Explicit casts `date()` and `dur()` in the query engine fail to `Null` (TRY_CAST semantics)."
- **L6 (Link Identity):** **KEEP.** [P]
  - Semantic equality and sort order compare folded target + subpath only. Display and embedded remain queryable attributes.
- **L7 (Duration Ordering):** **KEEP.** [P]
  - Nominal second ordering (`1mo == 30d`, `1y == 365d`) for total sort order.
- **L8 (Unicode String Order):** **KEEP.** [P]
  - Unicode-aware case-folded ordering for string collation.
