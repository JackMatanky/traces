# What is the single dynamic value model, and who owns it?

Type: grilling
Status: resolved
Blocked by: 2, 16, 1

## Question

Should the four value enums (`FieldValue`, `FieldValueRef`, `NoteFieldValue`,
`NoteFieldValueRef`) collapse into one value model, and if so what are its
numeric model, extension variants (Duration, Link, Date, DateTime),
ownership/borrowing story, serialization form, and home module? If not, what is
the explicit contract between the schema-side and note-side representations?

**Pending outcome (see Round 1 outcomes):** not one model and not four. Two
owned enums: a runtime `FieldValue` for notes and queries, and a schema-local
literal type, with no borrowed mirror enum in either.

Verified facts (`research/01` claims 1-2; current source checked):
`FieldValue` and `NoteFieldValue` differ, and their reference enums have
separate ownership shapes. Frontmatter currently parses to `noyalib::Value`,
then converts through `FieldValueRef` to `NoteFieldValue`
(`src/note/metadata.rs:56-74`). The first conversion maps every
`noyalib::Value::Number` through `as_f64()` to `FieldValueRef::Float`
(`src/field.rs:769-797`); the later conversion also casts `Int(i64)` to f64
(`src/note/field.rs:532-570`). Thus the current frontmatter path loses integer
identity/precision before `NoteFieldValue` construction, not only at the
final cast. `FieldValueRef`'s Serde visitor can retain `i64` (`src/field.rs:
891-905`), but `RawFrontmatter::parse` does not use it. Raw YAML is held during
parsing, then `Frontmatter` stores only structured values; note values have no
source spans (`src/note/metadata.rs:14-18,56-74,95-98`; survey/note.md).
Current note `Link` mixes target and display text with source syntax kind and
embed state (`src/note/links.rs:22-34`); `LinkTarget` is only a borrowed
target decomposition. Query equality currently uses derived `Link` equality
(including syntax/embed fields), while ordering compares target then display
text and ignores syntax/embed (`src/note/field.rs:302-304,393-405,1082-1092`).
This mismatch makes the semantic-value/occurrence split consequential.
Postcard persists the parsed `Note` in the rebuildable index cache
(`src/index/store.rs`; compatibility-envelope decision), not a standalone
value wire format.

Alternatives: unify; keep two with a defined mapping; unify the data model but
keep a borrowed read view; deserialize directly with serde into the final type.
Include the keep-as-is option.

Inputs: `survey/root-primitives.md`, `survey/note.md`, `survey/schema.md`,
`survey/cross-cutting.md` H2, `research/02`, `research/11` (sizes), compat
decision.

Unlocks: coercion and comparison policy, schema-note typing, note document
model, query core, template boundary.

## Tentative responses

- **Agreed:** preserve `Int(i64)` and `Float(f64)` separately; conversions are
  explicit.
- **Agreed:** retain typed `Date`, `DateTime`, `Duration`, and `Link` values in
  the semantic model.
- **Agreed:** an owned runtime model with no parallel borrowed mirror enum in
  the model. Refined in Round 1: any borrowed view enum is query-local and is
  decided by ticket 26 after a benchmark.
- **Agreed direction:** share only runtime value semantics that mean the same
  thing across consumers. Keep schema declarations, note source occurrences,
  query projections, and template-facing representations separate where their
  responsibilities differ.
- **Agreed direction:** convert parsed frontmatter into the final owned
  semantic model at the boundary where structure and type are known, while
  retaining source/editor information separately. Keep an intermediate
  representation only when it carries information another consumer needs.
  **Fact checked:** current code does not satisfy this yet: it converts
  `noyalib::Value` through `FieldValueRef`, and its number bridge converts all
  numbers to `f64` before note-value construction. The source parser's integer
  distinction is available per `research/02`; a direct AST-to-owned-value
  conversion can avoid both the borrowed mirror and precision loss. A direct
  Serde visitor from raw YAML would be a broader parser change, not required
  merely to remove the intermediate value enum.
- **Agreed direction — owner:** choose the lowest existing layer that can own
  the shared semantic variants without depending upward on note parsing,
  editing, or presentation. Decide exact module/crate placement only after
  validating the dependency boundary and considering the crate-topology
  decision; do not create a generic value module or absorb adjacent root domain
  types without a demonstrated need.
- **Agreed direction — schema literals:** keep schema declarations distinct
  from runtime values. Reuse shared literal variants only where the data has
  identical semantics; avoid conversion machinery unless actual schema fields
  require it.
- **Agreed direction — Link split:** use distinct `Link` and `NoteLink` types
  with fields belonging to each type. The base semantic `Link` carries the
  destination and optional display text. Occurrence-specific syntax and embed
  state belong to `NoteLink`; source spans depend on the note source-retention
  decision. Equality/order semantics remain open for the comparison decision.
- **Schema evidence:** the reviewed
  [Metadata Menu source digest](../../../docs/digests/obsidian_mdelobelle-metadatamenu-src-digest.txt)
  treats field kinds such as Date and File as editing/acceptance behavior with
  options, not necessarily runtime value variants. In Traces, raw schema
  options are generic `FieldValue`s; a date `format` is a string, numeric
  bounds accept integers and convert to floats, and select `value`/`label`
  resolve to strings while extra entry data is passed to templates. This
  supports keeping declarations/options separate while testing selective
  sharing for actual value payloads.
- **Superseded — owner and dependency shape:** see "Owner and persistence
  decisions" as revised by the two-enum decision.
- **Resolved — parsing and persistence:** see "Noyalib direct-deserialization
  review" and "Owner and persistence decisions". Serialization of temporal
  values in the cache remains a handoff (research result 5).
- **Resolved — frontmatter conversion:** direct AST-to-owned conversion; no
  custom Serde visitor on the YAML path.
- **Superseded — schema literals:** the two-enum decision (Round 1) gives
  schema its own literal type, so the earlier hybrid is replaced.

### Noyalib direct-deserialization review

Reviewed the latest Noyalib release, 0.0.57, through rust-docs MCP (the
project currently depends on 0.0.53). `from_str_with_config<T>` supports custom
Serde targets while retaining a `ParserConfig`, but it is not unconditionally a
streaming/raw-input visitor. The compatibility config sets
`MergeKeyPolicy::AsOrdinary`, while the streaming path requires `Auto`; for
that config Noyalib parses a `Value` AST and span tree, then deserializes the
target from them. Its `Value` target has a special span-free path that avoids
re-walking the AST. Thus a custom target may add a span tree and another
traversal; it is not automatically faster or lower-allocation than parsing to
`noyalib::Value` and consuming that tree directly.

**Agreed:** keep Noyalib's compatibility parser as the frontmatter boundary,
then convert its owned AST directly into the shared semantic model, removing
`FieldValueRef` from that conversion. Do not add a custom `Deserialize` target
to this YAML path unless typed diagnostics or validation justify its extra
span-tree/traversal cost and it preserves current invalid-key skipping. There
is no field-level recovery today: any YAML parse error, including an integer
above `u64::MAX`, drops the whole frontmatter block with a warning
(`note/metadata.rs:181`). The shared model may still implement `Deserialize` for
other boundaries.

**Agreed:** preserve Noyalib's current behavior for integers above `i64::MAX`
and through `u64::MAX`: explicitly convert them to `f64` at the parser-to-model
boundary. This can lose precision above 2^53; exactness would require rejecting
those values or adding an unsigned variant, neither of which is part of the
agreed model. Values beyond `u64::MAX` continue to error under the compatibility
parser.

Source references: [Noyalib 0.0.57 `from_str_with_config`](https://docs.rs/noyalib/0.0.57/noyalib/fn.from_str_with_config.html),
[ParserConfig::serde_yaml_compat](https://docs.rs/noyalib/0.0.57/noyalib/struct.ParserConfig.html#method.serde_yaml_compat),
[Number](https://docs.rs/noyalib/0.0.57/noyalib/enum.Number.html).

### Owner and persistence decisions

**Agreed:** choose placement by dependency evidence, not by maximizing reuse.
Only shared runtime semantics belong in the common layer. Keep note
occurrences, schema declarations, and query projections local; do not move
adjacent root concepts such as paths, tags, or tasks without an independent
semantic and dependency justification. Placement is settled below.

**Agreed:** do not define an independent stable wire format now. Keep cache
serialization internal, and ensure incompatible model changes invalidate or
rebuild the index cache. Revisit a stable contract if values become durable
user data or are exchanged with independent consumers.

**Agreed — schema boundary:** schema declarations/options remain schema-local.
Share a resolved payload only when it has the same runtime meaning; schema
field types do not silently reinterpret literals. Any such interpretation must
be explicit at the consumer boundary.

**Placement evidence (before the two-enum decision; see revised placement):**
this is a single-crate design. `src/field.rs` is
already the root-level shared home for `FieldValue`, field identifiers, and
keys; it depends on temporal types but not on note parsing. `src/note/field.rs`
depends on both `crate::field::FieldValueRef` and note-local `Link`, while
schema code consumes the root field value. This makes `field` the selected
dependency floor for the shared model. The semantic `Link` must not depend on
note-local link parsing or `NoteLink`.

**Superseded by `## Answer`:** the two-enum decision removed the reason for a
root-level shared `FieldValue`. The runtime value stays in `note`; only the
semantic `Link` is root-level.

### Note input normalization

**Agreed direction (narrowed in Round 1):** keep text-to-domain coercion at the
note boundary and keep the runtime `FieldValue` representation-neutral.
Frontmatter and inline fields share the date, duration, link and blank
parsers and one precedence order; other differences are syntax-native and
recorded as intentional.

**Verified divergence:** frontmatter converts every parsed YAML string through
date classification, then the note conversion maps blank strings to `Null`,
wikilinks to `Link`, and duration-shaped strings to `Duration`
(`src/field.rs:769-797`; `src/note/metadata.rs:67-73`;
`src/note/field.rs:526-570`). Inline parsing has its own atom grammar;
quoted strings bypass link/duration/date recognition, while unquoted strings
are recognized (`src/note/parser/inline.rs:39-45,117-140,148-189,218-230`).
For example, frontmatter `"4h15m"` becomes a duration, while inline
`"4h15m"` remains a string. Noyalib's owned `Value::String` does not retain
quote style, so a shared classifier alone cannot preserve that distinction
for both syntaxes.

**Superseded:** the rule that explicitly quoted strings bypass recognition in
both syntaxes was agreed, then withdrawn for frontmatter in Round 1 (YAML
requires quotes to write a link, and the style-recovery mechanism below is
unsound). It remains true for inline fields only.

**Mechanism reviewed and rejected (Round 1):** Noyalib 0.0.57 exposes
`cst::parse_document_with_config` and `Document::span_at(path)`. Its CST
preserves source bytes while its typed view uses the supplied `ParserConfig`;
the span API can recover the exact source slice for a resolved value. This
could provide YAML scalar style without implementing a second YAML scanner.
Tradeoffs: `Document` retains a green tree plus typed-value/span data, and
path-based span lookup must handle quoted/special mapping keys, aliases, and
duplicate keys consistently with the typed map. The crate docs say duplicate
key lookup resolves to the same last occurrence as the typed view.

The project pins Noyalib 0.0.53, whose registry source was checked directly:
it provides `cst::parse_document_with_config` and `Document::span_at` (behind
`std`, which the project enables), and the parser's event API is private, so
the CST is the only public route to scalar style. The project does not enable
`lossless-u64`, so integers in `(i64::MAX, u64::MAX]` fall to `f64` as agreed.
Behavior of the streaming gate in 0.0.53 was not re-verified.

**Cost and scope caveat:** the CST path builds a green tree, a typed `Value`
and a `SpanTree` per note, which cuts against the span-free rationale above;
aliases, merge keys, and tags (currently dropped: `Tagged` becomes its inner
value, `field.rs:793`) are unresolved by it. A lazy variant (consult style only
for plain-eligible strings that are date, duration, link, or blank shaped) is
an alternative to evaluate.

### Review findings (status)

1. **Shared `FieldValue` vs schema-local literals:** resolved by the two-enum
   decision.
2. **Serialization consumers:** resolved in structure (derived tagged form for
   the runtime value, natural serde for schema literals, two template
   adapters). The cache encoding of temporal values is a handoff (research
   result 5).
3. **Integer policy per boundary:** resolved (Round 1, Numbers).
4. **Inline `Int`/`Float`:** resolved (Round 1, Numbers).
5. **Classifier divergence:** resolved as intentional, syntax-native
   differences (Round 1, Classifier scope; table below).
6. **Quoted-literal undercut downstream:** deferred to ticket 19.
7. **Scope vs ticket 19:** resolved. Ticket 18 settles only schema-free
   parse-time classification.
8. **Borrowed views:** deferred to ticket 26 with a benchmark gate.
9. **Link payload in the runtime value:** open (Open question 1).
10. **Visibility:** deferred to ticket 32; evidence in research result 6.
11. **Size budget:** resolved by research result 1 (keep 80 B, no boxing).
12. **Behavior change from the literal rule:** obsolete, since quoted
    frontmatter values stay typed. Replaced by the behavior changes this
    ticket does introduce: inline integers become `Int`; the values-file
    `visit_u64` converts instead of saturating; computed query fields become
    `Int`; template numbers render `5` not `5.0`; `Link` display text is
    normalized; native TOML dates in schema data get explicit handling.
13. **Whole-frontmatter drop on any YAML error:** open (Open question 3).

### Evidence gathered after the review (probe run, since removed)

Throwaway test feeding the same text to the frontmatter route
(`yaml::parse` then `FieldValueRef` then `NoteFieldValue`) and the inline
parser (`parse_inline_value`):

| Text | Frontmatter | Inline |
|---|---|---|
| `007` | String | Number(7.0) |
| `0x1F` | Number(31.0) | String |
| `~` | Null | String("~") |
| `-0` | Number(0.0) | Number(-0.0) |
| `""` | Null | String("") |
| `"4h15m"` (quoted) | Duration | String |
| `"2025-01-15"` (quoted) | Date | String |
| `'4h'` (single-quoted) | Duration | String (quotes kept) |
| `2025-01-15T12:00` | DateTime (zone-shifted: printed as 10:00Z on this machine) | String |
| `2025-01-15T12:00:00Z` | DateTime | String |
| `[[note]]` (unquoted) | List([List(["note"])]) (YAML flow sequence) | Link |
| `"[[note]]"` (quoted) | Link | String |
| `![[img.png]]` | YAML parse error | Link(embedded) |
| `#tag` | Null (YAML comment) | String("#tag") |
| `1, 2` | String | List([1, 2]) |
| `[1, 2]` | List | String |

Consequences:

- **A wikilink in YAML must be quoted to be a value at all.** Unquoted `[[x]]`
  is a flow sequence. The agreed rule "quoted strings bypass link recognition
  in both syntaxes" would therefore make frontmatter links impossible. The
  rule needs a link exemption in frontmatter or a different rule.
- **Path-based CST span lookup is unsound for some keys.** With `Document`,
  `get("a.b")` resolves to nested `a: {b: ...}`, not the literal key `"a.b"`,
  and `get("\"a.b\"")` returns `None`; `!!str 4h` reports an unquoted span;
  block scalars include the `|` header; aliases resolve to the anchor's span;
  last duplicate wins (consistent with the typed map). A correct design would
  need a custom tree walk, not `span_at(path)`.
- **No test fixture relies on quoted frontmatter dates or durations**
  (only `link: "[[Project Alpha|Alpha]]"` in `note/field.rs`).
- **`FieldValue` (non-note) has no consumer outside schema today.** It is used by
  raw schema options, select entries (`value`, `label`, `extra`), values-file
  loading, and the template adapter `select_entry_value`. Its `Date` and
  `DateTime` variants are produced in production only via `FieldValueRef`'s
  classify path (frontmatter) and in tests; schema loading uses the passthrough
  deserializer. The note side has its own minijinja adapter
  (`template/engine/query.rs:604`), so templates already see two hand-written
  natural forms.
- **`NoteFieldValue(Ref)` are public API** (`tests/baselines/public-api-*.txt`,
  `benches/query_sort.rs`, one integration test). Schema `FieldValue` is
  `pub(crate)`.
- **`Link` has two spellings of "no alias".** `parse_wikilink_prefix` stores
  `Some(target)` (`split_wikilink_text` returns `(s, s)`), while `Link::new`
  with empty text stores `None`; derived equality distinguishes them, though
  `text()` hides the difference.
- **Frontmatter `Date` and `DateTime` parsing is zone-dependent** for naive
  datetimes, which is ticket 19's determinism concern, not this ticket's.

### Candidate designs compared

- **A. One shared enum** (agreed direction): `FieldValue` gains `Duration` and
  `Link`; schema, notes, queries all use it. 80 bytes. Schema literals can then
  represent values they never produce.
- **B. Two owned enums, explicit contract:** a schema/config literal type
  (`Null/Bool/Int/Float/String/List/Object`, natural serde form, no postcard)
  and a note runtime value (adds `Date/DateTime/Duration/Link`, derived
  tagged postcard form). One-way widening conversion only where schema data
  reaches note-level comparison (none today).
- **C. Generic container over scalars:** `Value<S>` with `List`/`Object`
  shared. Rejected: generic burden on every caller for seven shared variants.
- **D. Lexical storage, typing on demand** (Obsidian/Metadata Menu style):
  note values stay YAML-native; Date/Duration/Link appear only through schema
  or query-time casts. Belongs to ticket 19; not decided here.
- **E. Reuse an external value type** (`serde_json::Value`, `noyalib::Value`):
  rejected: no `Date`, `Duration`, `Link`; `noyalib::Value` carries `Tagged`
  and `Number` variants the model does not want.

### Round 1 outcomes (confirmed; placement and link details revised in the Answer)

Stress tests covered: absence of schema consumers, the `007`/`~`/`""` and
timezone divergences, template number rendering, link consumers, and borrowed
view allocation. Caller counts alone were rejected as evidence for deleting
capabilities, since the crate has end users and a public surface.

- **Topology (two owned enums, confirmed):** the runtime value becomes
  `field::FieldValue` (`Null, Bool, Int(i64), Float(f64), String, Date,
  DateTime, Duration, Link, List(Box<[Self]>), Object`), keeping the derived
  tagged form for postcard. The current schema `FieldValue`, its visitor,
  `DateTimeCoercion` and `FieldStringValue` move into the schema module as a
  schema-local literal type (tentatively `SchemaLiteral`, same variant names,
  no `Date`/`DateTime`/`Duration`/`Link`, natural serde). Two explicit
  template adapters remain: natural serde for schema literals
  (`select_entry_value`) and the hand-written runtime adapter
  (`template/engine/query.rs`). User input: select values may reference links
  but must be readable as plain strings on the schema side, and date
  min/max/default are not planned. So schema literals stay strings, and a
  literal is interpreted as a link only at the consumer boundary (ticket 20),
  via the shared `Link` wikilink parser. Wikilink rendering and parsing must
  round-trip at root level for wikilinks only; a Markdown `[t](x)` link does
  not round-trip as a wikilink. Revisit trigger: if schema literals ever need
  typed dates, durations or links, collapse to one enum.
- **Quote rule (revised):** frontmatter classification stays quote-insensitive
  (links need quotes in YAML; the CST path is unsound for dotted keys, tags and
  block scalars). Inline quotes remain the escape hatch. The literal-string
  escape for frontmatter belongs to ticket 19.
- **Classifier scope (revised):** share the date/duration/link/blank parsers and
  one precedence order. Inline does not gain `DateTime`: naive datetimes are
  zone-shifted at parse time and persisted in the cache, so the zone question
  goes to tickets 19 and 21. Bool, null, number, list, `#tag`, `~`, `""` stay
  syntax-native, recorded as intentional divergences.
- **Numbers (revised):** `Int(i64)`/`Float(f64)`. Frontmatter: integer to `Int`,
  values above `i64::MAX` to `Float`. Values-file visitor: `visit_u64` converts
  to `Float` instead of saturating. Inline: integer-shaped text fitting `i64`
  to `Int`, else `Float`. `Int` and `Float` share one numeric rank and compare
  by value (ticket 19 inherits this, including whether `PartialEq` agrees).
  Computed query fields (`file.size`, `list.line`, `list.depth`,
  `list.parent`) use `Int`. Template number rendering changes from `5.0` to
  `5` (confirmed below).
- **Link (payload still open):** root-level `Link { target, display:
  Option<String> }` with `None` when the trimmed, unescaped display is absent,
  empty or equal to the trimmed, unescaped target. `NoteLink` wraps `Link` with
  `kind`, `embedded` and optionally a span; the user wants these
  distinguishable in queries, and `NoteLink` is also needed for splitting note
  types. Which type a runtime `FieldValue::Link` carries is open (Open
  question 1). Equality and order stay with ticket 19.
- **Borrowed views (refines the "no mirror" line above):** the model exports
  `&FieldValue` and no mirror enum. Any view enum is query-local. Ticket 26
  benchmarks owned vs view over `file.name` sort and filter before deleting
  `NoteFieldValueRef`; `resolve_file_ref` currently returns borrowed `&str`
  for `file.name` and borrowable paths without allocation.
- **Handoffs:** ticket 19 (zone-dependent naive datetimes, literal-string
  escape, `Int` vs `Float` equality, `Link` equality, schema-select string vs
  note `Link` matching); ticket 20 (explicit string-to-link interpretation at
  the schema boundary); ticket 21 (zone input); tickets 24 and 25 (cache
  format change and temporal encoding); ticket 32 (public visibility of the
  runtime value and `Link`/`NoteLink`).

### Research results (items 1 to 6)

1. **Sizes (measured):** runtime `NoteFieldValue` 80 B, schema `FieldValue`
   72 B, `Link` 56 B, `DurationValue` 72 B (a `jiff::Span` plus an `i64`),
   `IndexMap<String, _>` 72 B, `Box<[T]>` 16 B, `Option<String>` 24 B,
   `DateTimeValue` 16 B. `IndexMap` and `DurationValue` both set the 80 B
   total, so boxing one leaves it unchanged; shrinking needs both boxed. The
   normalized `Link` (two strings, about 48 B, not built) does not change the
   enum size. Recommendation: keep the 80 B test and do not box.
2. **TOML native dates in schema data (existing bug):**
   `toml::from_str::<FieldValue>("a = 1979-05-27")` yields
   `Object({"$__toml_private_datetime": String("1979-05-27")})`. Native TOML
   dates in schema options or values files silently become a bogus object.
   The schema literal type must handle TOML datetimes explicitly (reject, or
   read as string).
3. **Template numbers (measured):** `minijinja::Value::from(5.0_f64)` renders
   `5.0`, `from(5_i64)` renders `5`, `from(2.5_f64)` renders `2.5`. The runtime
   adapter currently uses `Value::from(f64)`, so integer frontmatter values
   render as `5.0` today. The change to `Int` is a visible, intentional
   improvement.
4. **Nested keys:** `FieldPath` variants are `File`, `List`, `Tags`,
   `Inlinks`, `Metadata`; `resolve_metadata_ref` looks up one top-level key
   through case-insensitive `FieldKeyRef`. I found no nested-object path
   navigation in `results.rs`. Nested `Object` keys are plain `String` and are
   only compared, serialized or rendered. Not checked: how `FieldPath::parse`
   treats dotted metadata keys.
5. **Postcard encoding (measured):** `DateValue`, `DateTimeValue` and
   `DurationValue` serialize as text (`1h 30m` is the 6-byte string `1h 30m`,
   a datetime is a 20-byte ISO string) and are re-parsed on decode. A
   `Number` is a tag plus 8 bytes. Every cached temporal value is therefore
   parsed again at index load, while research 11 attributes about 97 percent
   of path-set-change cost to note deserialization. A compact binary form
   chosen through `is_human_readable()` is a candidate; unmeasured, and
   handed to tickets 24 and 25.
6. **Public surface:** no default-feature public function returns a
   `NoteFieldValue`; `Frontmatter` accessors and `Note::get` are `pub(crate)`.
   Externally the enum is constructible and exposes `as_str`, `as_date`,
   `as_ref`, and `NoteFieldValueRef::compare`. `Note::outlinks()` returns
   `&[Link]` whose accessors are `pub(crate)`, so it is opaque. The value
   types' public surface is thin; ticket 32 should keep public only what is
   meant for end users. The `test-utils` baseline was not checked.

### Open questions (answered)

1. **Link in the runtime value:** agreed, `embedded` on the root `Link`.
2. **"Splitting note types":** the earlier ticket's idea of separate note
   shapes for indexing/querying and for editing. So `NoteLink` (syntax kind and
   source span) belongs to the editor-side shape and is defined by ticket 23.
3. **Out-of-range integers:** agreed, frontmatter-only config copy with
   `integer_overflow_errors` off.
4. **Equality:** agreed, derived structural `PartialEq`; semantic cross-kind
   equality lives in the query layer (ticket 19).
5. **Placement:** agreed, `NoteFieldValue` stays in `note`; only `Link` goes
   to root; schema type renamed and moved into schema.
6. **YAML tags, aliases, merge keys:** agreed, recorded as intentional.
7. **TOML datetimes in schema data:** agreed, read as strings.

## Answer

**Decision:** two owned value types and no borrowed mirror enum. A note
runtime value (`NoteFieldValue`) for notes, queries and templates, and a
schema-local literal type for schema options and values files. They share only
the root-level `Link`.

**Alternatives considered:**

- *Keep as is* (four enums, three-stage conversion
  `noyalib::Value` to `FieldValueRef` to `NoteFieldValue`): rejected.
  `FieldValueRef` is always immediately owned (zero-copy gain is nil), the
  chain costs about 41 percent of frontmatter parse time, integers lose
  identity at the first hop, and the duplicate visitors diverge.
- *One shared enum:* rejected. Schema literals would hold variants they never
  produce, and the schema boundary must read select values as strings.
- *Generic container over scalars:* rejected, generic burden for seven shared
  variants.
- *Lexical storage with typing on demand:* not decided here, belongs to the
  coercion and comparison ticket.
- *External value type* (`serde_json::Value`, `noyalib::Value`): rejected, no
  `Date`/`Duration`/`Link`.
- *Direct Serde visitor or CST for quote style on the YAML path:* rejected.
  The compat config builds a `Value` AST and span tree anyway, and
  `Document::span_at(path)` is unsound for dotted keys, tags and block scalars.

**Runtime value (`note::NoteFieldValue`):** `Null, Bool, Int(i64), Float(f64),
String, Date, DateTime, Duration, Link(Link), List(Box<[Self]>),
Object(IndexMap<String, Self>)`. It replaces `Number(f64)`. Derived tagged
serde for the postcard cache; derived structural `PartialEq`; the 80-byte size
test stays and nothing is boxed (`IndexMap` and `DurationValue` are both 72 B).

**Schema literal (schema module, tentatively `SchemaLiteral`):** the current
`FieldValue` renamed and moved: `Null, Bool, Int, Float, String, List(Vec),
Object`, natural serde, no `Date`/`DateTime`/`Duration`/`Link`. Native TOML
datetimes are read as strings (today they silently become a bogus object).
Select values that name links stay strings; ticket 20 interprets them at the
consumer boundary with the shared wikilink parser.

**Root `Link`:** `Link { target, display: Option<String>, embedded: bool }`.
`display` is `None` when the trimmed, unescaped display is absent, empty or
equal to the trimmed, unescaped target (this removes the two current spellings
of "no alias"). It owns a wikilink render and parse that round-trip for
wikilinks only. Module name is chosen in the specification (ticket 32).
`NoteLink` (syntax kind and source span) is the editor-side occurrence type and
is defined by ticket 23.

**Frontmatter conversion:** keep the Noyalib compat parser and convert its
`Value` directly into `NoteFieldValue`. Use a frontmatter-specific copy of the
compat config with `integer_overflow_errors` off, so an integer above
`u64::MAX` becomes an approximate `Float` and does not discard the whole
frontmatter block. Integers convert to `Int`; those above `i64::MAX` convert
to `Float`. YAML tags are dropped and merge keys are ordinary keys, as
intentional behavior. Keys failing `FieldKey` validation are still skipped.

**Classification:** frontmatter and inline share the date, duration, link and
blank parsers and one precedence order. Frontmatter classification is
quote-insensitive (a link must be quoted in YAML); inline quotes remain the
literal escape. Inline does not gain `DateTime`. Bool, null, number, list,
`#tag`, `~` and `""` stay syntax-native and are recorded as intentional
divergences (comparison table above).

**Numbers:** `Int(i64)` and `Float(f64)` everywhere. Inline integer-shaped text
fitting `i64` becomes `Int`, else `Float`. The schema values-file visitor
converts `u64` above `i64::MAX` to `Float` and no longer saturates. Computed
query fields (`file.size`, `list.line`, `list.depth`, `list.parent`) use `Int`.
`Int` and `Float` share one numeric rank and compare by value.

**Persistence and templates:** no independent wire format; cache serialization
stays internal and a model change invalidates the cache. Two explicit
template adapters remain: natural serde for schema literals and the
hand-written runtime adapter (`Int` renders `5` instead of `5.0`).

**Borrowed views:** the model exports `&NoteFieldValue` and no mirror enum. Any
view enum is query-local. `NoteFieldValueRef` is not deleted until ticket 26
benchmarks owned vs view over `file.name` sort and filter.

**Invariant owners:** note-boundary classification rules (note parser, one
precedence order); numeric identity (the runtime value's constructors); `Link`
normalization and wikilink round-trip (root `Link`); literal-vs-link
interpretation of schema strings (the schema consumer, ticket 20).

**Becomes removable:** `FieldValueRef`, its seed and visitor, `DateTimeCoercion`,
`FieldStringValue`, both `From<FieldValueRef>` conversions, the
`Number(f64)` variant, the values-file saturation test, and the double
frontmatter conversion. `NoteFieldValueRef` is removable pending ticket 26.

**Deliberate behavior changes:** inline and frontmatter integers become `Int`;
template numbers render `5`; values-file `u64` overflow converts; huge
frontmatter integers no longer drop the block; `Link` display is normalized;
native TOML dates in schema data are read as strings; computed query fields
become `Int`. Quoted frontmatter values keep their current classification.

**Remains unresolved (handed off):**

- [Coercion and comparison policy](./19-coercion-and-comparison-policy.md):
  frontmatter literal-string escape, `Int`/`Float` equality, `Link` equality
  and order (including `embedded`), zone-shifted naive datetimes, schema-select
  string vs note `Link` matching.
- [Schema note typing](./20-schema-note-typing-and-validation.md): explicit
  string-to-link interpretation; schema literal type boundary.
- [Execution context](./21-ambient-environment-and-execution-context.md): the
  zone input for naive datetimes persisted in the cache.
- [Note document model](./23-note-document-model.md): editor-side `NoteLink`
  and the indexing/query vs editing shapes.
- [Index read model](./24-index-read-model-and-query-execution.md) and
  [freshness](./25-index-freshness-and-invalidation.md): temporal values encode
  as text and re-parse on every cache load; cache format change.
- [Query core](./26-query-core-and-plan.md): view enum shape and benchmark gate.
- [Template boundary](./28-template-engine-boundary.md): the two adapters and
  number rendering.
- [Crate topology and public surface](./32-crate-topology-and-public-surface.md):
  visibility of the value types, `Link` and `NoteLink`.

## Superseded in part by ticket 19

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). It revokes
"Inline does not gain `DateTime`" (the reason was the persisted zone shift;
naive datetimes become civil values). It settles the handoffs from this ticket:
no frontmatter literal-string escape (classification stays quote-insensitive; a
schema override covers text), exact `Int`/`Float` comparison outside the
structural `PartialEq`, `Link` equality on the folded target only, and schema
select strings matching links by literal adoption.

## Superseded in part by ticket 20

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The
"schema override covers text" handoff is not built (no override). Select values
that name links are parsed with the root `Link` parser at the schema check.
