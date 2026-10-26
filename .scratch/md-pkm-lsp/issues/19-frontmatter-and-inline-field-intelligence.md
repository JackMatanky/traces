# Metadata/frontmatter & inline-field intelligence

Type: grilling
Blocked by: 08, 11
Status: resolved

## Prerequisites (ticket 11 changes that must land first)

The following ticket 11 outcomes are assumed by this ticket's decisions but are
NOT yet implemented in the codebase. They must be completed before ticket 19 can
be implemented:

1. **`BytePos` narrows to `u32`**: Currently `pub(crate) struct BytePos(usize)`
   at `src/position.rs:33`. It must become `u32` with `From<u32>` and
   `TryFrom<usize>` (lossy). See ticket 11.
2. **`LineIndex` moves to `src/position.rs`**: Currently `pub(super)` at
   `src/note/parser/line.rs:7`. It must move to `src/position.rs`, widen to
   `pub(crate)`, and gain a `byte_to_utf16_cu` method. See ticket 11.
3. **`InlineTokenLexer` byte spans**: The lexer currently returns
   `Vec<(FieldKey, NoteFieldValue)>` without byte ranges. Inline-field
    diagnostics need spans. Options: (a) add a span-preserving variant. (b) run
    a separate position-aware scan for diagnostics. (c) scope inline-field
    diagnostics to line-level only until spans are added. See ticket 11 for the
   broader span model.

## Question

Grounding: Frontmatter is parsed via `yaml-serde` into an opaque
`IndexMap<FieldKey, NoteFieldValue>` (`src/note/metadata.rs:19,50`) with the raw
source text and spans discarded post-parse. Ticket 11 must settle whether/how
spans get reconstructed before this ticket's completion/hover/diagnostics can
attach to precise ranges. Inline fields (`Key:: Value`, `[Key:: Value]`, `(Key::
Value)`) are modeled the same way, also without location tracking today.
Informed by Metadata Menu research (ticket 08).

Decide:

- Completion: field-key completion in frontmatter YAML and inline-field syntax.
  informed by what fields exist elsewhere in the workspace (unscoped) vs what a
  bound Schema/File-Class permits (scoped; this overlaps ticket 20, so decide
  the boundary: is unscoped completion this ticket's job and scoped completion
  ticket 20's job, layered together at request time?).
- Hover: showing a field's resolved type/value, and (if bound to a schema) its
  Field Definition constraints.
- Diagnostics: malformed YAML frontmatter (syntax errors: does `yaml-serde`
  already surface span-aware errors that can be forwarded? check the
  `src/note/metadata.rs` error path), malformed inline-field syntax.
- Rename: renaming a field key across a note (frontmatter and inline-field
  consistency) and, further, across the workspace (all notes using that key).
  Decide if the latter is in scope given `Field Key` is explicitly
  "case-insensitive... preserving author casing" (`src/GLOSSARY.md`), which
  affects rename-match semantics.
- Whether inline-field value types (`NoteFieldValue`: String, Date, Link, List,
  Object per `src/note/field.rs:30`) get type-aware hover/diagnostics (e.g. a
  Link-typed field value gets the same reference-resolution treatment as a body
  wikilink).

## Answer

### Parsing strategy: replace `serde_yaml` with `noyalib`

Replace `serde_yaml` with `noyalib` (v0.0.51; `serde_yml`'s 23M downloads now
forward to it). Use noyalib's core API, `from_str_with_config` with a shared
`ParserConfig::serde_yaml_compat()`, not the `compat::serde_yaml` drop-in
feature. `noyalib` provides `Spanned<T>` with byte ranges (`line()`, `column()`,
`index()`), enabling single-pass value+position extraction. The CST module is
available for future lossless editing. Zero unsafe, YAML 1.2 (406/406 tests), 8
deps.

*Amended 2026-09-27 (by note-parse ticket 02 review): the original "Enable
`YamlVersion::V1_1` for Dataview compatibility" is corrected: **use YAML 1.2**.
Obsidian/Dataview parse via `eemeli/yaml` (1.2 default), the current yaml_serde
is already 1.2.2. V1_1 would diverge from Dataview (`Bool(true)` vs
`"yes"`), contradict this repo's own inline parser, and retype vault content
(sexagesimal/octal). Full evidence: note-parse ticket 02, "Rationale: YAML 1.2,
not 1.1". Parity comes from the `serde_yaml_compat()` config knobs instead of a
version flag.*

### Frontmatter byte-range extraction

Per ticket 11: `BytePos` narrows to `u32` with `From<u32>`/`TryFrom<usize>`.
Introduce a `ByteSpan(Range<BytePos>)` newtype for field spans. The scanner runs
over `RawFrontmatter.as_str()` via noyalib's `Spanned<T>`, producing `FieldKey →
ByteSpan` (byte ranges relative to frontmatter start). The LSP handler adds the
pulldown-cmark metadata block base offset and converts via `LineIndex` (ticket
11: it moves to `src/position.rs` and gains `byte_to_utf16_cu`) to `Position {
line, character }`. No new struct goes on `Frontmatter`: the scan output is a
separate data structure consumed by LSP handlers only. The scanner handles full
YAML structure (nesting, block scalars, anchors, aliases), not just flat
key-value.

*Reconciled 2026-09-23: this is the effort's **single** frontmatter-span
mechanism. It supersedes the hand-rolled Level-2 raw-text scanner originally
decided in [Source span/position model for the Note
AST](11-source-span-and-position-model.md) and the line-offset re-scan
originally planned in ticket 20; both now defer here.*

### Three-layer completion architecture

- **Layer 1** (highest): Fields from the note's resolved schema (already
  includes `$ref`-resolved global fields via eager resolution in
  `SchemaBuilder`).
- **Layer 2** (medium): Global schema fields NOT already in Layer 1
  (supplementary, "available but unused").
- **Layer 3** (lowest): Vault-wide inferred fields (Dataview-style, from all
  notes' frontmatter/inline fields).

Dedup by `FieldKey` canonicalization (case-folded,
hyphen/underscore-normalized). Higher layers suppress lower. Visual badges in
completion: no badge for schema fields, `⟨global⟩` for global, `⟨inferred⟩` for
vault-inferred.

**Data structure at completion time**: Build a `Vec<(FieldKey, Layer,
Option<SchemaFieldDef>)>` by: (1) collecting schema fields from the note's
fileClass schema into a `HashSet<FieldKey>`. (2) collecting global schema fields
into a second set and subtracting (1), (3) collecting vault-wide inferred fields
into a third set and subtracting (1) and (2). Each set becomes a completion
layer with `sortText` prefixed by layer number (0=schema, 1=global, 2=inferred)
for client-side ordering.

**Performance**: Layers 1+2 are O(1) lookups (schemas are in memory via
`SchemaService`). Layer 3 needs a vault-wide field-key index, the same
`VaultFieldIndex` needed for inferred hover. It is pre-built during
`IndexerService::refresh`, not computed per-request. Completion budget: <20ms
per ticket 33.

### Conflict resolution: fileClass wins (matrix)

When a field name appears in both global and fileClass schemas:

- **Type**: fileClass wins (global is suppressed).
- **Required**: stricter wins (`true` > `false`), regardless of layer.
- **Multi**: fileClass wins.
- **Exclude**: if fileClass excludes a field, it is removed from Layer 1 even if
  global defines it.

For `$ref` fields: renaming a field that is referenced via `$ref` from other
schemas needs renaming in the defining schema and all references. This is out of
scope for ticket 19 and deferred to workspace-wide rename (tier 3).

### Unbound notes (no fileClass)

Layer 1 is empty. Layer 2 shows global schema fields. Layer 3 shows vault-wide
inferred fields. Rich completion comes with clear provenance badges.

### Schema-bound field hover

Type info (from `SchemaFieldType`), `required`/`multi` flags, type-specific
options (Select values, Number bounds, Date format, File filters), and schema
attribution (`#book` schema). Format as markdown: the field name as H2, the type
in backticks, constraints in a bullet list. Future: add `description:
Option<String>` to `SchemaFieldDef` for user-authored hover text.

### Vault-wide inferred field hover

Inferred type (from the `NoteFieldValue` variant), usage count ("12 of 150
notes"), location (frontmatter/inline/both), sample values (up to 5). Hybrid
tentative/authoritative: lead with the most common type and footnote minority
uses. No "invalid value" diagnostics in unscoped mode.

### Link hover

Link values in inline fields get the same go-to-definition and hover as body
links. Use a safe markdown subset for cross-IDE compatibility: bold, italic,
inline code, fenced code blocks (with language hints), unordered lists, and
inline links. No headings, tables, images, HTML, or definition lists. Include
the target note title, a 2-3 line content preview, a link type indicator, and
the file path. First-line previews are computed during `IndexerService::refresh`
and stored in an in-memory `HashMap` on the analysis host (not in redb, per
ticket 13). `MarkupKind::Markdown` with a plain text fallback for non-markdown
editors (e.g., Visual Studio). Content is structured for progressive enhancement
(Neovim/Zed can extend via richer rendering).

### Trigger characters

`[` and `:` as trigger chars with context-dependent handling:

- `[` trigger: check whether it is a wikilink, markdown link, or inline field;
  offer completion only for inline fields.
- `:` trigger: must follow a valid YAML key pattern (bare word:
  `^[a-zA-Z_-][a-zA-Z0-9_-]*\s*$`). It must NOT be followed by another colon
  (which would be `::`), and must be in the frontmatter region. This
  distinguishes YAML value colons (`title: My Note`) from inline field triggers
  (`[key:` inside brackets).
- `CodeRegion` exclusion runs FIRST (before any trigger handling).
- `(` is dropped as a standalone trigger.

### Diagnostics: YAML frontmatter

Warning severity for malformed YAML (this allows partial intelligence on
well-formed fields). Error severity for document-root-level issues (missing
`---` delimiters, non-mapping top-level) where no intelligence is possible.
Include the error message with a byte range from noyalib's error types. Defer
code actions (YAML fix-its) to a later ticket.

### Diagnostics: inline fields

Diagnose only when the `InlineTokenLexer` has already identified a token as a
field but the structure is malformed (unbalanced brackets, empty key). Bare
fields are whole-line `Key:: Value` forms after optional blockquote/list
prefixes; their keys match `[A-Za-z][A-Za-z0-9_-]*`. Bracketed and parenthesized
forms may use multi-word keys. Do NOT flag ambiguous single-colon syntax (`[key:
value]`): the user may not be writing an inline field. Warning for syntax
errors, Information for style. Published per-file on document open/change.

### Rename scope

Three tiers:

1. **Note-local rename** (ticket 19): rename a field key across frontmatter +
inline fields within the current document. Simple text replacement with
`FieldKey` canonical matching. No index needed.
2. **FileClass-scoped rename** (ticket 19): rename a field key in the schema
   definition (`<fileClass>.toml`), limited to that schema's **own fields** (not
   inherited `$ref` targets). Renaming a `$ref` target needs renaming in the
   defining schema and all references, which is deferred to tier 3.
3. **Workspace-wide inline field rename** (NOT in scope): needs a vault-wide
   field-key index. Deferred.

### Type-aware treatment

Follow Dataview's inference cascade for unscoped mode: empty→null,
date→duration→quoted string→tag→link→bool→number→null→fallback string. Different
types are incomparable (NOT alphabetical like Dataview). Unscoped mode:
informational diagnostics only (no validation without schema). **This decision
covers unscoped mode only.** Scoped mode (schema-bound validation) is ticket
20's scope. It will use the same Dataview cascade but with the schema type as
the expected type.

### Amended by 25 (2026-10-15)

- **Frontmatter-missing-delimiter severity recorded (25 Q13-R2)**: 19's Error
  for missing `---` delimiters **stands**. The premises are corrected against a
  live rumdl 0.2.79 probe: MD041 (Warning) fires on every broken shape; MD065
  (Warning) fires only when the leading `---` parses as a horizontal rule
  (missing closing delimiter); both are suppressible only by disabling the rule
  itself (impractical for transient mid-edit breakage). Traces' diagnostic is a
  frontmatter-parse code (`frontmatter.missing-delimiter`), not a heading
  judgment, so it does not duplicate the ceded MD041/MD065 heading rules. A
  coexistence note ships to 32.

### Amended by 26 (2026-10-18)

- **Schema-bound field hover renders the field name as bold, not H2**: 26's
  heading ban (the intersection-safe subset; this ticket's own cross-IDE
  rationale is the stronger half of its internal inconsistency) wins.
- **Tier-2 field rename (FileClass-schema-own fields) deferred to 20's
  schema-TOML span work** (schema-TOML spans are forbidden in v1); tier 1
  (note-local canonical `FieldKey` match) and the `$ref` tier 3 are unchanged.
- **This ticket's refresh-time content preview cache is adopted as the
  wikilink/embed hover producer source** (disk files), with `ContentResolver`
  for overlays (staleness ≤ T1 100ms). Wikilink/embed hover content is decided
  by 26: title + resolved path + bounded first-lines preview + inlink count,
  with ambiguity labelled rather than silently first-picked.
