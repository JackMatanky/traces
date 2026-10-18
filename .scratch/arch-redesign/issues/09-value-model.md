# Value model: one metadata value tree or several?

Type: grilling
Blocked by: 01, 03, 04

## Question

Traces represents dynamically-typed metadata in several parallel forms: `FieldValue`/`FieldValueRef` (`src/field.rs` — generic, `Int(i64)`/`Float(f64)`, used by schema option maps and select-value files), `NoteFieldValue`/`NoteFieldValueRef` (`src/note/field.rs` — `Number(f64)` plus `Duration` and `Link`, and the owner of the cross-kind total order), and `QueryFieldValueRef` (`src/query/value.rs` — wraps the note form, adds `Tags`/`Inlinks`/`Owned`). Frontmatter load is hypothesized to convert `noyalib::Value → FieldValueRef → NoteFieldValue`, with separate TOML/JSON/minijinja paths. Decide the **target value model**: how many trees exist, which module owns each, owned vs borrowed forms, numeric representation (`Int`+`Float` vs one `Number`), where equality/ordering/hash semantics live, and where the YAML/TOML/JSON → value conversion boundary sits.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md) lands; cite its ledger, not the survey): [root primitives §9](../survey/root-primitives.md), [note §9/§11.1](../survey/note.md), [query §9.2](../survey/query.md), [schema §9](../survey/schema.md). Counter-argument already visible: schema options and config are format-agnostic data that should not carry Markdown-only variants (`Duration`, wikilink `Link`).

**Verified (ledgers landed 2026-10-18; supersedes the UNVERIFIED wording above)** — [01 §2](../research/01-verify-core-findings.md): the chain `noyalib::Value → FieldValueRef → NoteFieldValue` is real (`note/metadata.rs:67-73`). `FieldValue` is consumed by `schema` (raw options, select entries) and by template schema conversion; `FieldValueRef` only by `field.rs`, `note/field.rs`, `note/metadata.rs`; `FieldStringValue` is private to `field.rs`; `NoteFieldType` is used only for sort ranking (`note/field.rs`, `query/sort.rs`). `DateTimeCoercion::Classify` is enabled in exactly one production place (`impl From<noyalib::Value> for FieldValueRef`, `field.rs:772`); serde paths hardcode `Passthrough`. Lossy hops: YAML numbers go through `as_f64` before the note type exists (`field.rs:770`), `Int→f64` at `note/field.rs:545`, month/year precision dropped (`note/field.rs:161-173`), markdown (non-wiki) links stay strings, durations become strings in minijinja, link aliases dropped in sort keys. Schema and note value trees never meet (no `schema → note` edge). Perf context ([03 §4](../research/03-performance-evidence.md)): frontmatter conversion costs ~0.56 µs/field, so the double hop is a modeling/ownership problem, not a measured CPU problem.

## Alternatives (not exhaustive, none preferred)

- Keep two trees, make the conversion boundary explicit and single-hop.
- One `Value` in a low-level module with PKM variants; format-agnostic consumers ignore them.
- One raw, format-agnostic tree plus a typed *interpretation* layer produced at a single boundary (parse, don't validate), rather than variants for every interpretation.
- Borrowed-first (`Cow`/arena) vs owned values given redb persistence and the `Note` size/allocation facts from [Performance evidence](03-research-performance-evidence.md).

## Constraints and inputs

`Note` is persisted via serde into redb today (see ticket [Note document model](14-note-document-model.md)); the value model's serialized shape is therefore a storage format. Template rendering converts values to `minijinja::Value`; query sort/filter rely on the total order. The LSP map's schema-validator and completion tickets need a typed view of frontmatter (md-pkm-lsp tickets 19/20).

## Done when

The answer states: the set of value types and their owning modules; the representation of numbers, dates, durations, links, lists, objects; who owns ordering/equality; the conversion boundary and its allocation behavior; what is deleted; the evidence for each choice; and which of [Coercion policy](10-coercion-policy.md), [Schema and note typing](11-schema-note-typing-and-validation.md), [Note document model](14-note-document-model.md) are affected and how.
