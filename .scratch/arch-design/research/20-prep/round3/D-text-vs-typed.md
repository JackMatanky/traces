# Scout D: Text vs Typed Frontmatter Values and Quote-Style Semantics

## 1. Sub-problem Restated

Ticket 16 must resolve how the schema subsystem types and validates note data,
and where that capability lives. Sub-problem D addresses a concrete ergonomic
challenge at the note-parsing boundary: **how can a date- or number-shaped
frontmatter value be retained as plain text rather than eagerly promoted to a
typed scalar?**

Examples include:

- ISBN-10 codes with leading zeros (`isbn10: 0306406152`) or hyphens.
- Version strings (`version: 1.10`) where float coercion loses trailing zeros.
- Purely numeric titles (`title: 2001`).
- Date-shaped titles, codes, or identifiers (`title: 2025-01-15`, `code:
  2024-06`).
- Raw wikilink syntax meant to stay literal text (`example: "[[Note]]"`).

The user specifically suggested considering a **TOML-like convention** [H]
distinguishing single quotes (`'...'`) from double quotes (`"..."`):

- Single quotes denote literal text (preventing type classification).
- Double quotes allow classification (so wikilinks like `"[[Note]]"` and dates
  like `"2025-01-15"` can still parse into `Link` and `Date` variants).

Ticket 15 withdrew quote-sensitive frontmatter classification because YAML
requires quotes to write a wikilink (`"[[x]]"`), and the CST route to inspect
quote delimiters added a +59% parse latency penalty while remaining unsound on
dotted keys and tagged scalars (`15:64-67` [V]). This scout investigates whether
that reasoning is airtight, stress-tests the TOML-like proposal against real
tools (Obsidian, Dataview, static site generators, and YAML round-trippers),
verifies the capabilities of the pinned parser (`noyalib`), evaluates 7
concrete candidates, and delivers a definitive recommendation.

---

## 2. What Traces Does Today

### 2.1 The Frontmatter Parsing Pipeline

Frontmatter parsing in Traces proceeds through three stages:

1. **YAML Parse to AST (`src/note/metadata.rs:56-74` [V]):**
   `RawFrontmatter::parse` invokes `yaml::parse` (`src/yaml.rs:28-30` [V]),
   which
   calls `noyalib::from_str_with_config` using a static `ParserConfig`
   configured with `serde_yaml_compat` (`src/yaml.rs:18-19` [V]). This produces
   an owned `noyalib::Value`.
2. **Intermediate Mapping (`src/field.rs:769-798` [V]):**
   `FieldValueRef::from(noyalib::Value)` converts YAML nodes into
   `FieldValueRef`. `noyalib::Value::Number(n)` becomes `FieldValueRef::Float`.
   `noyalib::Value::String(s)` invokes `FieldStringValue::classify` with
   `DateTimeCoercion::Classify` (`src/field.rs:781-782, 838-854` [V]).
3. **Semantic Note Value (`src/note/field.rs:532-571` [V]):**
   `NoteFieldValue::from(FieldValueRef)` converts the intermediate value into
   the runtime `NoteFieldValue`. Strings are inspected:
   - Empty/whitespace-trimmed strings become `NoteFieldValue::Null` (line 551).
   - Wikilink syntax (`[[target]]`) becomes `NoteFieldValue::Link` (line 552).
   - Duration syntax (`4h15m`) becomes `NoteFieldValue::Duration` (line 554).
   - All other strings remain `NoteFieldValue::String` (line 557).

### 2.2 How Different Scalar Shapes Behave Today

1. **Numbers and Versions:**
   - Unquoted `title: 2001` parses as `noyalib::Value::Number(2001)`, converting
     to `NoteFieldValue::Number(2001.0)` (`src/note/field.rs:545` [V]).
   - Unquoted `version: 1.10` parses as `noyalib::Value::Number(1.1)`. The
     trailing zero is permanently discarded by `f64` conversion.
   - **Quoted `title: "2001"` or `'2001'`:** Parses as `Value::String("2001")`.
     `DatePoint::parse("2001")` fails because `"2001"` is neither `YYYY-MM` nor
     `YYYY-MM-DD` (`src/temporal/point.rs:90-97` [V]). Traces **never** sniffs
     numbers from strings (`src/field.rs:843-853` [V]).
     **Result:** Quoted numbers **already stay as text** today
     (`NoteFieldValue::String("2001")` [V]).
   - **Quoted `version: "1.10"` or `'1.10'`:** Parses as
     `Value::String("1.10")`.
     It does not match Date, Link, or Duration.
     **Result:** Quoted version strings **already stay as text** today
     (`NoteFieldValue::String("1.10")` [V]).
   - **Leading-Zero Numbers (`zip: 01234`):** Under YAML 1.2 Core schema
     (`ParserConfig::serde_yaml_compat`), integers with leading zeros are not
     valid decimal or octal numbers (octal requires `0o`). They resolve as
     `Value::String("01234")` and stay `NoteFieldValue::String("01234")`
     (verified by test `src/note/metadata.rs:319` [V]).
   - **ISBN-10 (`isbn10: 0306406152`):** Starts with `0`, so under YAML 1.2
     Core schema it resolves as `Value::String("0306406152")` and stays
     `NoteFieldValue::String` [V].
2. **Dates, DateTimes, and Durations:**
   - Unquoted `due: 2025-01-15` parses as `Value::String("2025-01-15")`.
     `DatePoint::parse` succeeds (`src/field.rs:843` [V]). It becomes
     `NoteFieldValue::Date`.
   - **Quoted `due: "2025-01-15"`:** Parses as `Value::String("2025-01-15")`.
     Because string classification is quote-insensitive, it **also** becomes
     `NoteFieldValue::Date` [V].
   - **Single-quoted `due: '2025-01-15'`:** Also becomes `NoteFieldValue::Date`
     [V].
   - **Crucial finding:** In Traces today, quotes **do** preserve text for
     numbers and version strings, but **cannot** preserve text for date-shaped,
     datetime-shaped, or duration-shaped strings.
3. **Wikilinks:**
   - Unquoted `link: [[Note]]` is a YAML syntax error (unquoted `[` begins a
     flow
     sequence).
   - Quoted `link: "[[Note]]"` or `'[[Note]]'` parses as
     `Value::String("[[Note]]")`.
     `Link::parse_wikilink` (`src/note/link.rs:30-45` [V]) matches and converts
     it to `NoteFieldValue::Link`.
4. **YAML 1.1 / 1.2 Ambiguities (`src/note/metadata.rs:309-335` [V]):**
   - `draft: yes` -> `NoteFieldValue::String("yes")` (YAML 1.2 booleans are
     strictly `true`/`false`).
   - `meeting: 10:30` -> `NoteFieldValue::String("10:30")` (YAML 1.2 removed
     sexagesimal numbers).
5. **YAML Tags (`!!str`):**
   - `noyalib::Value::Tagged` is stripped to its inner value in
     `src/field.rs:793-795`
     [V].
   - In `noyalib-0.0.53/src/parser/loader.rs:2180` [V], `resolve_tagged_scalar`
     explicitly resolves `!!str` directly into plain `Value::String`, dropping
     the
     tag wrapper entirely. Thus `!!str 2025-01-15` arrives at Traces as plain
     `Value::String("2025-01-15")` and is sniffed as `Date`.

### 2.3 Verification in Pinned Noyalib Source (0.0.53 and 0.0.57)

Direct inspection of `noyalib-0.0.53` in the cargo registry and `noyalib-0.0.57`
via `rust-docs` confirms:

- **`ScalarStyle` is private:** `enum ScalarStyle` (`Plain`, `SingleQuoted`,
  `DoubleQuoted`, `Literal`, `Folded`) is `pub(crate)` in
  `noyalib-0.0.53/src/parser/scanner.rs:21` and `src/parser/mod.rs:19` [V]. It
  is
  never exposed in the public parser API.
- **`noyalib::Value` drops quote style:** `Value::String(String)` retains only
  the unescaped payload (`src/value.rs:28-45` [V]). Both single and double
  quotes produce identical `Value::String`.
- **`BorrowedValue` drops quote style:** Zero-copy `BorrowedValue`
  (`src/borrowed.rs:109-170` [V]) also represents strings as
  `BorrowedValue::String(Cow<str>)`, with no quote style metadata.
- **`PolicyEvent` lacks quote style:** `PolicyEvent` (`src/policy.rs:84-95` [V])
  contains only `kind`, `anchor`, `tag`, and `scalar: Option<&str>`.
- **Noyalib 0.0.57 additions:** 0.0.57 adds `dominant_quote_style()` on
  `Document` (a file-wide aggregate), `quote_style()` on `EmitCtx` (for
  serialization), and `plain_scalar_strings` in `ParserConfig`. It adds **no**
  per-scalar quote inspection API to `Value` or deserialization [V].
- **CST is the sole route:** The only way to access quote styles in Noyalib
  0.0.53 or 0.0.57 is via the Concrete Syntax Tree (`noyalib::cst::Document`),
  which builds a green tree and span index per note (`cst/builder.rs:41-44`
  [V]).

---

## 3. Precedents

### 3.1 TOML (v1.0.0 and v1.1.0 Specifications)

- **What it does:** TOML defines basic strings (`"..."`) and literal strings
  (`'...'`). Basic strings process backslash escape sequences (`\n`, `\t`,
  `\uXXXX`). Literal strings treat all characters verbatim (no escapes).
- **The typing model:** In TOML, dates/times (`1979-05-27`), numbers (`42`), and
  booleans (`true`) are **bare tokens**. Strings **must** be quoted.
- **Critical distinction:** In TOML, `d = 2025-01-15` is a Date. Both
  `d = "2025-01-15"` and `d = '2025-01-15'` are Strings. TOML never uses quote
  styles to distinguish "auto-typed" from "literal" values; quotes simply mean
  string, while lack of quotes means typed bare token. Transferring this
  distinction to YAML frontmatter misapplies TOML's semantics.

### 3.2 YAML 1.2 Specification & The Norway Problem

- **What it does:** In YAML 1.2 Core Schema (sections 7.3 and 9.1 [V]), implicit
  schema tag resolution (to bool, int, float, null) applies **only** to plain
  (unquoted) scalars. Quoted scalars (`'`, `"`, `|`, `>`) have resolved tag
  `tag:yaml.org,2002:str`.
- **The Norway Problem:** In YAML 1.1, unquoted `NO` (the ISO country code for
  Norway) resolved as boolean `false`, and `yes` resolved as `true`. YAML 1.2
  resolved this by restricting core booleans strictly to `true` and `false`.
- **The lesson:** In standard YAML, quote delimiters have **no semantic typing
  difference**. Single quotes and double quotes differ only in escape
  processing. Creating custom semantics where single quotes mean "string" and
  double quotes mean "evaluate as date/link" invents an ad-hoc YAML dialect that
  no other parser understands.

### 3.3 Obsidian Properties UI & `processFrontMatter`

- **What it does:** Obsidian 1.4+ introduced the Properties UI. Behind the
  scenes, Obsidian parses frontmatter into a JavaScript object using `js-yaml`
  and re-serializes it on save (`app.fileManager.processFrontMatter`).
- **Quote normalization behaviour:**
  - In Obsidian Forum bug report #65851 (`processFrontMatter removes / alters
    string quotes, comments, types, formatting` [V]), Obsidian lead developer
    Licat stated:
    > "The decision we made is for plugins to be able to interact with
    > frontmatter
    > easily. We do this by converting YAML into a JS object, which means it
    > loses
    > all of its YAML specific formatting and comments."
  - When re-serializing, `js-yaml` **strips single quotes** from strings and
    tags. When quoting is required (e.g. for strings containing colons or
    digits), it **always emits double quotes (`"..."`)**, never single quotes
    [V: forum #65851, #63824].
- **Fatality for TOML-like convention:** If a user writes `title: '2025-01-15'`
  or `isbn: '0306406152'` in Obsidian, the moment they edit **any** property in
  Obsidian's UI, Obsidian rewrites the file, stripping the single quotes or
  converting them to double quotes. Under a TOML-like convention, Traces would
  then parse the value as a `Date` or `Number` on the next index run.

### 3.4 Obsidian Dataview

- **What it does:** Dataview parses frontmatter via Obsidian's metadata cache.
  Classification is entirely quote-insensitive: `due: 2025-01-15`, `due:
  "2025-01-15"`,
  and `due: '2025-01-15'` all parse into Luxon `DateTime` objects.
- **Escape mechanism:** Dataview provides **no** frontmatter quote escape. If a
  field looks like a date, it becomes a date. To treat it as text in queries,
  users must invoke the explicit `string(field)` function.
- **Regret:** Community forums document frequent user confusion when dates
  silently fail string comparisons. Dataview solved this in its query engine via
  functions, never by inspecting YAML quote delimiters.

### 3.5 Static Site Generators: Astro, Hugo, Jekyll

- **Jekyll:** Uses Ruby's `Psych` (YAML 1.1). Unquoted dates become
  `Time`/`Date`
  objects; quoted dates become `String`. Users frequently suffer Liquid
  template crashes when formatters or tools alter quote styles.
- **Astro Content Collections:** Astro explicitly abandoned YAML quote-sniffing
  in favor of Zod schemas (`defineCollection({ schema: z.object({ ... }) })`)
  [V: Astro docs]. If the schema declares `z.string()`, the frontmatter value is
  coerced or validated as string. If `z.coerce.date()`, it becomes a Date.
  **The schema dictates the type, not the presence of quotes.**
- **Hugo:** Distinguishes types based on the frontmatter format (TOML vs YAML)
  rather than quote styles inside YAML.

### 3.6 Python `ruamel.yaml` Round-Tripping

- **What it does:** `ruamel.yaml` is the standard round-tripping YAML library in
  Python.
- **Lessons learned:** Python's native `str` cannot store quote styles.
  `ruamel.yaml` was forced to subclass `str` into `SingleQuotedScalarString`,
  `DoubleQuotedScalarString`, `LiteralScalarString`, and `FoldedScalarString`.
  Any normal string operation (concatenation, formatting, json serialization)
  drops the subclass, losing the quote style. This demonstrates that carrying
  syntax-level quote style into runtime value models causes extreme friction.

### 3.7 Rust YAML Ecosystem (`serde_yaml`, `serde_norway`, `yaml-rust2`)

- Across `serde_yaml::Value`, `serde_norway::Value`, and `yaml_rust2::Yaml`,
  string variants are universally `String(String)` [V].
- None of the established Rust YAML ASTs expose quote style. Doing so requires
  falling back to low-level token scanners or full CSTs.

---

## 4. Menu of Candidate Solutions

| # | Candidate | Summary |
|---|---|---|
| **C1** | **Keep Status Quo [P] (Schema-Free Quote-Insensitive + Canonical Text View)** | Parse-time classification is quote-insensitive. Values are typed as Date/Duration/Link/Number. Consumers convert to string via canonical projection (`as_str()`). Lossy for non-canonical formats (`1.10` -> `1.1`). |
| **C2** | **Quote Style = Text [P] (All Quotes Force String)** | Any scalar with single or double quotes parses as `NoteFieldValue::String`. Unquoted scalars undergo normal YAML resolution and date/duration sniffing. |
| **C3** | **TOML-Like Convention [P] (Single Quote = Text, Double Quote = Classified)** | `'...'` forces `NoteFieldValue::String`. `"..."` and unquoted allow classification into Date, Duration, and Link. |
| **C4** | **Standard YAML `!!str` [P] Explicit Tag** | Users write `title: !!str 2025-01-15` or `version: !!str 1.10`. The `!!str` tag forces string type. |
| **C5** | **Raw-Lexeme Retention [P] for Ambiguous Scalars** | `NoteFieldValue::Number` and `Date` retain the original raw source string slice (e.g. `Number { value: f64, raw: Box<str> }`). |
| **C6** | **Schema `type = "input"` [P] with On-Demand Frontmatter Re-read** | Frontmatter parses quote-insensitively into the standard value model. When a consumer (LSP, template, validator) needs raw text for an `input` field, it re-reads `RawFrontmatter::as_str()`. |
| **C7** | **Schema-Declared Text Intent [P] at Query & Validation Boundary** | Frontmatter parsing stays schema-free and quote-insensitive. Schema `type = "input"` defines text intent. Ticket 15's adaptive literals handle queries; schema validator warns on unquoted numbers; explicit `string()` function handles query casts. |

---

## 5. Strongest Argument Against Each Candidate

### Against C1 (Keep Status Quo / Lossy Canonical View)

Non-canonical text shapes cannot be preserved. Writing `version: 1.10` unquoted
permanently degrades to float `1.1`, losing the minor version zero. Writing
`title: 2025-01-15` permanently parses as a `Date`, meaning string operations
like `startswith(title, "2025-01")` or case formatting require explicit casting.

### Against C2 (Quote Style = Text: All Quotes Force String)

1. **Breaks wikilinks:** In YAML, wikilinks must be quoted (`link: "[[Note]]"`).
   If all quotes force string, wikilinks become strings unless an ad-hoc
   exception is carved out for `[[...]]`.
2. **Silent query breakage:** Templater (`due: "<% tp.date.now() %>"`), Metadata
   Menu, and Prettier quote dates (`due: "2025-01-15"`). Under strict type
   bracketing (Ticket 15), these notes silently drop out of date queries
   (`--where 'due <= 2025-01-01'`) without warning (`19-prep/h:85-95` [V]).
3. **Parse cost:** Noyalib's `Value` drops quotes. Inspecting quotes requires
   the
   CST, adding +59% parse latency across the vault.

### Against C3 (TOML-Like: Single Quote = Text, Double Quote = Classified)

1. **Destroyed by Obsidian Properties:** Obsidian's `processFrontMatter` and
   Properties editor parse YAML to JS objects and re-serialize using `js-yaml`.
   `js-yaml` strips single quotes and converts required quotes to double quotes
   (`"..."`) [V: forum #65851]. The moment a user modifies a note in Obsidian,
   their `'2025-01-15'` is rewritten to `"2025-01-15"` or bare `2025-01-15`,
   silently turning back into a Date in Traces!
2. **High parse latency penalty:** As measured in Ticket 15 prep
   (`19-prep/h:57-73`
   [V]), accessing quote styles in Noyalib requires the CST
   (`noyalib::cst::Document`).
   A full CST parse adds a **+59% latency penalty**. A "lazy CST" parse (parsing
   CST only when a date string is found) is **2.59x slower** because notes are
   parsed twice or thrice.
3. **Ecosystem & formatter friction:** Linters and formatters (Prettier with
   `singleQuote: true`) arbitrarily convert double quotes to single quotes.
   Under C3, running Prettier across a vault would silently convert valid dates
   (`"2025-01-15"`) into strings (`'2025-01-15'`), breaking all date queries.
4. **Violates YAML spec:** In YAML 1.2, single and double quotes have identical
   typing semantics. Inventing a distinction confuses authors and tools.

### Against C4 (Standard YAML `!!str` Tag)

1. **Ignored by Noyalib 0.0.53:** In `noyalib-0.0.53/src/parser/loader.rs:2180`
   [V],
   `resolve_tagged_scalar` resolves `!!str` directly into plain `Value::String`
   and discards the tag. It is invisible to Traces' conversion pipeline.
2. **Stripped by Obsidian:** Obsidian Properties UI strips YAML explicit tags
   (`d: !!float 123` becomes `d: "123"` [V: forum #65851]).
3. **Hostile ergonomics:** Requiring PKM authors to write `title: !!str
   2025-01-15`
   is ugly, error-prone, and rejected by the markdown community.

### Against C5 (Raw-Lexeme Retention for Ambiguous Scalars)

1. **Noyalib does not retain raw lexemes:** `noyalib::Number` is an enum of
   `Integer(i64)` and `Float(f64)` (`src/value/number.rs:21-30` [V]). It does
   not keep the raw string slice. Acquiring it requires the +59% CST pass.
2. **Memory layout bloat:** `NoteFieldValue` is strictly budgeted at <= 80
   bytes (`src/note/field.rs:579-581` [V]). Adding `Option<Box<str>>` to scalar
   variants increases memory footprint across millions of indexed field values.
3. **Contradicts ADR-0009 and Ticket 14:** Settled facts state that notes keep
   no byte spans and no raw scalar text (`local://20-prep-context-3.md:30-31`
   [V]).

### Against C6 (Schema `type = "input"` with On-Demand Frontmatter Re-read)

1. **Two parse passes during queries/templates:** Re-reading raw frontmatter
   text
   during template rendering or query evaluation breaks the fast synchronous
   evaluation path and requires ad-hoc string searching or re-parsing YAML.
2. **Query index mismatch:** If the index stores
   `NoteFieldValue::Date(2025-01-15)`,
   index-accelerated queries cannot know the original text without loading and
   slicing the source file.

### Against C7 (Schema-Declared Text Intent at Query & Validation Boundary)

Requires the author to declare a schema if they want strict enforcement for
unusual fields (e.g. a title that is exactly an ISO date). In a zero-config
vault without schemas, an unquoted `title: 2001` remains a Number and
`title: 2025-01-15` remains a Date (though quoted `title: "2001"` is already a
String).

---

## 6. Deep Analysis of the TOML-Like Convention

The prompt directs a specific analysis of five aspects of the TOML-like
proposal:

### 6.1 Does it solve the wikilink-needs-quotes problem?

**Superficially yes, but with a trap.** Under the rule where double quotes allow
classification, `link: "[[Note]]"` parses as `Link`. However, if an author
writes `link: '[[Note]]'` using single quotes (a standard habit in many YAML
vaults), it would become a literal `String`, breaking outgoing link indexing.

### 6.2 What about authors who write `title: '2025-01-15'` today?

In existing Obsidian and markdown vaults, single and double quotes are used
interchangeably. Many authors write `due: '2025-01-15'`. Under the TOML-like
convention, this note's `due` field would become a `String`, while a neighbor's
`due: "2025-01-15"` would become a `Date`. Under Ticket 15's type bracketing
(where comparisons across different type brackets do not match),
`--where 'due <= 2025-01-01'` would silently exclude the single-quoted note.
This silent breakage is unacceptable in a PKM tool.

### 6.3 Does Obsidian normalise quote styles in Properties editor?

**Yes, aggressively.** Verified directly in Obsidian Forum threads #65851 and

## 63824 [V]. Obsidian parses frontmatter to a JavaScript object and dumps it

via

`js-yaml`. On save:

- Unnecessary quotes are removed.
- Single quotes are converted to unquoted text or double quotes.
- `js-yaml` never outputs single quotes.
Therefore, any convention in Traces that relies on single quotes to preserve
text is inherently unstable in an Obsidian vault: the user's single quotes will
be destroyed by Obsidian's Properties editor.

### 6.4 Cost and feasibility via Noyalib

As established in Section 2.3, Noyalib 0.0.53 and 0.0.57 do not expose scalar
quote styles in `Value` or deserializer events. Feasibility is blocked unless
Traces runs `cst::parse_document_with_config`. Empirical benchmarks
(`19-prep/h:57-73` [V]) prove that:

- Always parsing CST adds **+59% parse time** per note.
- Parsing CST lazily when dates appear adds **+159% (2.59x)** because real PKM
  vaults contain dates in ~85%+ of notes, causing duplicate parsing.
- Path-based span lookup (`span_at`) is fragile on dotted keys (`a.b: 'val'`)
  and
  block scalars.

#### 6.5 Consistency with Ticket 15's Adaptive-Literal Rule

Ticket 15 established that query filters use **adaptive literals** (`15:107-135`
[V]): a quoted literal in a query (e.g. `due == "2025-01-15"`) adapts at
evaluation time to match the row's field type.

- If the field is a `Date`, it matches as a Date.
- If the field is a `String`, it matches as a String.
This already eliminates query-side friction without mutating frontmatter
classification. Reintroducing quote-sensitivity at parse time directly conflicts
with Ticket 15's settled rule:
> "Frontmatter classification stays quote-insensitive (links stay recognised
> when quoted); inline quotes remain the escape." (`15:102-105` [V])

---

### 7. Verdict Matrix and Recommendation

#### 7.1 Verdict Matrix

| Criterion | C1 (Status Quo) | C2 (All Quotes = Text) | C3 (TOML-Like Quotes) | C4 (`!!str` Tag) | C7 (Schema Text Intent) |
|---|---|---|---|---|---|
| **Zero Parse Overhead** | Yes (1.0x) | No (+59% CST) | No (+59% to 2.59x) | Yes (1.0x) | **Yes (1.0x)** |
| **Obsidian Safe (No Quote Mangle)** | Yes | Fragile | **NO (Fatal)** | No (Stripped) | **Yes** |
| **Wikilinks Work Naturally** | Yes (`"[[x]]"`) | Needs Exception | Yes (`"[[x]]"`) | Yes | **Yes** |
| **No Silent Query Breakage** | Yes | No (Drops dates) | No (Drops dates) | Yes | **Yes (Adaptive)** |
| **Standard YAML 1.2 Compliant** | Yes | Divergent | Divergent | Yes | **Yes** |
| **Preserves Version `1.10` / `2001`** | Partial (Quotes) | Yes | Yes | Yes | **Yes (Quotes + Schema)** |
| **Preserves Date-Shaped Title** | No (Becomes Date) | Yes | Yes (Single quote) | Yes | **Yes (Schema override)** |

#### 7.2 Ranked Recommendation

##### Rank 1: Candidate 7 (Schema-Declared Text Intent at Query & Validation

Boundary)

**Adopt C7.** This is the cleanest, most robust, and highest-performance
architecture. It honors all settled facts from Ticket 14 and Ticket 15:

1. **Keep frontmatter parsing quote-insensitive, infallible, and fast (1.0x):**
   No CST overhead, no green-tree traversals, and no reliance on fragile quote
   delimiters.
2. **Clarify that quotes already preserve numbers and versions:**
   In Traces today, writing `version: "1.10"` or `title: "2001"` **already**
   preserves them as `NoteFieldValue::String` because Traces never sniffs
   numbers
   from strings. Educate users and document this rule clearly.
3. **Handle rare date-shaped strings via Schema and Query Casts:**
   - For fields that look like dates but represent text (e.g. `title:
     2025-01-15`):
     - In queries, Ticket 15's **adaptive literals** ensure that
       `title == "2025-01-15"` matches seamlessly regardless of whether `title`
       is stored as `Date` or `String`.
     - In templates, rendering `{{ note.title }}` outputs `"2025-01-15"`.
     - In schemas, declaring `[fields.title] type = "input"` instructs the
       validator (Ticket 16) that text is expected. If an author writes unquoted
       `title: 2001` (Number), the validator flags an advisory diagnostic:
       `"Field 'title' expects text but parsed as number; wrap in quotes"`.
     - In query expressions requiring string methods, users invoke
       `string(title)`
       or `lower(title)`.

##### Rank 2: Candidate 1 (Keep Status Quo / Lossy Canonical View) [P]

Acceptable fallback if no schema is configured in the vault. Zero mechanical
risk, but cannot preserve unquoted `1.10` or distinguish date-shaped titles
without explicit query casts.

##### Definitively Rejected

- **Candidate 3 (TOML-like convention) [P]:** Rejected.
  Mechanically blocked without +59% CST penalty; immediately corrupted by
  Obsidian Properties rewriting single quotes; creates silent query breakage
  for existing vaults.
- **Candidate 2 (All quotes = text) [P]:** Rejected. Breaks wikilinks and causes
  silent row dropping in date queries for Templater/Prettier users.
- **Candidate 4 (`!!str`) [P]:** Rejected. Stripped by both Noyalib 0.0.53 and
  Obsidian Properties.
- **Candidate 5 (Raw lexeme retention) [P]:** Rejected.
  Bloats value model beyond 80 bytes and violates ADR-0009.
- **Candidate 6 (Consumer on-demand re-read) [P]:** Rejected. Slow and complex.

#### 7.3 What Evidence Would Change This Recommendation?

This recommendation would only change if:

1. Noyalib upstream exposes scalar quote style as a zero-cost `enum QuoteStyle`
   directly on `noyalib::Value::String` without requiring CST parsing; **AND**
2. Obsidian ceases using `js-yaml` and guarantees 100% lossless preservation of
   single quotes in frontmatter.
Because neither condition is true or likely, Candidate 7 is mathematically and
architecturally dominant.

---

### 8. Lessons Transferable to Other Sub-Problems

1. **To Sub-problem A & B (Schema Seam & Violation Model):**
   Do not force note parsing to know about schemas, and do not force schemas to
   alter low-level parser tokenization. The parser must remain fast, infallible,
   and schema-free. The schema's role is to validate parsed values and provide
   semantic intent to consumers (LSP diagnostics, template helpers, and MCP
   guardrails).
2. **To Sub-problem E (Checker Diagnostics & Fixes):**
   When an author writes `title: 2001` unquoted for a schema field typed
   `input`,
   the schema checker should emit a diagnostic:
   - Severity: Warning / Advisory.
   - Code: `type-mismatch-scalar`.
   - Message: `"Expected string for 'title', found number. Add quotes:
     '\"2001\"'."`
   - Quick-fix: Wrap the scalar in standard double quotes.
3. **To Sub-problem F (MCP Agent Guardrail):**
   When an MCP agent edits frontmatter, it must format string properties using
   **standard double quotes (`"..."`)**, never single quotes or YAML explicit
   tags (`!!str`), ensuring compatibility with Obsidian Properties and standard
   YAML parsers.
4. **To Ticket 15 Integration:**
   Re-confirming quote-insensitive classification solidifies Ticket 15's
   adaptive literal model. Query literals adapt to the data; note parsers do
   not play guessing games with quote characters.
