# Research: `schematter-validator` and the `document-schema` Spec

- **Ticket:** [ticket 36](../issues/36-research-schematter-validator-and-document-schema.md)
- **Primary Inputs:**
  - `schematter-validator 0.2.0` on docs.rs [V: `https://docs.rs/crate/schematter-validator/0.2.0`]
  - Spec site `document-schema.org` [V: `https://document-schema.org/`]
  - Spec & validator repository `iwe-org/schematter` [V: `https://github.com/iwe-org/schematter`]
  - IWE callers in `/tmp/iwe/crates/liwe/src/schema.rs`, `document.rs`, `crates/diwe/src/schema.rs`

---

## 1. Provenance, Maintenance, and Distribution

- **Maintainer & Author:** Dmytro Halichenko (`gimalay`), Director of Engineering [V: crates.io, LinkedIn].
- **Organisation:** `iwe-org` on GitHub (`https://github.com/iwe-org/schematter`).
- **Relationship to IWE:** Same author and organisation as `iwe-org/iwe` (`Cargo.toml:6`). `schematter` was split out from IWE's codebase into a dedicated repository [V].
- **Licence:** `Apache-2.0` [V: `Cargo.toml.orig`].
- **Releases:**
  - `0.1.0` (2026-07-12): initial release [V: `CHANGELOG.md`].
  - `0.2.0` (2026-08-30): added dialect `$ref` (`$id`, `$defs`, `resolve` module, external schema resolution) [V: `CHANGELOG.md`].
- **Downloads:** ~2,603 total downloads across 2 versions; ~702 for `0.2.0` [V: crates.io metadata].
- **MSRV & Edition:** Rust `1.82`, Edition `2021` [V: `Cargo.toml.orig`].

## 2. Size, Dependency Weight, and Compiles

- **Size:** Very small (~13-25 KiB source archive, ~1,200 LoC in `schematter-validator/src/`) [V].
- **Direct Dependencies:**
  - `serde = "1.0"` (with derive) [V]
  - `serde_json = "1.0"` [V]
  - `serde_yaml_ng = "0.10"` [V]
  - `regex = "1"` [V]
  - `jsonschema = "0.47"` (with `default-features = false`) [V]
- **What it Omits:**
  - Zero markup parsers (no `pulldown-cmark`, no markdown parser) [V].
  - Zero tokenizers (no `tiktoken-rs`; token counts are precomputed integer fields on `Document`/`Section`/`Block`) [V].
- **Weight in Traces:**
  - Traces already depends on `regex` and `serde`.
  - Adding `schematter-validator` pulls `jsonschema 0.47` (and its transitive dependencies) and `serde_yaml_ng 0.10` into core. If Traces compiles schemas from YAML strings, `serde_yaml_ng` is required [V].

## 3. Input Model: Value Tree and Document Model

- **Dual-Model Contract:**
  1. **Frontmatter:** Takes `serde_json::Value` (specifically `Value::Object(Map)`). In-memory frontmatter can be passed directly as JSON values without serializing to YAML text [V: `document.rs:18`].
  2. **Body:** Does **not** take markdown strings. It takes an explicit, pre-parsed domain struct:

     ```rust
     pub struct Document {
         pub frontmatter: Value,
         pub frontmatter_error: Option<String>,
         pub body_tokens: usize,
         pub blocks: Vec<Block>,
         pub sections: Vec<Section>,
     }
     ```

- **Can Traces Feed It Persisted Data?**
  - **Yes, but with transformation.** Traces stores frontmatter and a flat list of `Heading { level, text, line }` in `Note`.
  - To call `schematter-validator`, Traces must:
    1. Map `IndexMap<FieldKey, Box<[NoteFieldValue]>>` or `Frontmatter` into `serde_json::Value::Object`.
    2. Nest flat `Heading`s into a recursive `Vec<Section>` tree.
    3. Fill `Block` arrays (Traces does not persist paragraphs, code blocks, or quotes; empty `blocks: vec![]` passes unless the schema specifies `blocks`) [V: `document.rs:37`].
    4. Provide token counts (`usize`); passing `0` works if schemas omit `maxTokens` [V: `document.rs:50`].

## 4. Spec Semantics (`document-schema.org/draft/2026-06/schema`)

- **Matching Algorithm:**
  - Ordered, sequential, greedy, non-backtracking walk [V: `document-schema.md:197`].
  - For each section, matches against `sections` entry list. Matches header identity (`const`, `enum`, `pattern`, `minLength`, `maxLength`, `maxTokens`). Advances pointer; prior entries are closed forever [V].
  - Wildcards (headerless entries) greedily absorb all remaining sections and **must** be terminal; non-terminal wildcards trigger exit 2 (schema error) [V: `eval.rs:unreachable_wildcard`].
  - Two entries with identical match identity trigger a load-time schema error [V: `duplicate_entry`].
- **`minContains` and `maxContains`:**
  - `minContains` defaults to `1`; `maxContains` defaults to unbounded [V: `document-schema.md:210`].
  - Counts total occurrences across the node, not consecutive runs [V: `document-schema.md:222`].
- **`additionalSections`:**
  - Boolean or reduced schema. `true` (default) allows unmatched sections. `false` rejects any section not matched to an open entry. A reduced schema validates each leftover section [V: `document-schema.md:185`].
- **Token Budgets:**
  - Token counting is completely externalized: the validator takes `body_tokens`, `header_tokens`, `subtree_tokens`, `text_tokens` as raw `usize` fields on the structs [V: `document.rs`].
  - The spec dictates OpenAI `o200k_base` (tiktoken) convention on the CLI, but the validator crate has zero tokenizer dependencies [V].
- **Violations & Schema Errors:**
  - Separation: `SchemaError` (exit 2, invalid schema/syntax/unreachable rule) vs `Violation` (exit 1, document fails schema) [V: `violation.rs`, `document-schema.md:435`].
  - Violation struct: `breadcrumb: Vec<Crumb>` (`Frontmatter`, `Header(String)`, `Position(usize)`, `Field(String)`, `Block(usize)`, `Item(usize)`), `message: String`, `hint: Option<String>` (nearest `description`), `schema_pointer: String` (JSON Pointer), `keyword: String` [V: `violation.rs:36`].

## 5. Spec Maturity and Stability

- **Draft Status:** `draft/2026-06/schema` [V: `document-schema.org`].
- **Independent Implementations:** Exactly **one** (`schematter` / `schematter-validator` in Rust, with a WASM build on the spec website) [V].
- **Conformance Suite:** Metaschema validation tests (`tests/metaschema.rs`, `tests/metaschema_agreement.rs`) inside `schematter-validator` [V]. No cross-language or standalone test suite.
- **Maturity Assessment:** High alignment with JSON Schema 2020-12 dialect conventions, but tightly coupled to the author's vision for IWE and OKF [H].

## 6. Fit and Costs for Traces

### Options for Traces

1. **Adopt `schematter-validator` crate:**
   - *Pros:* Complete implementation of section ordering, heading depth, regex/enum matching, breadcrumbs, hints, and JSON pointer tracking; zero markdown parser bloat.
   - *Cons:* Pulls `jsonschema` (heavy compile time) and `serde_yaml_ng`; expects full recursive `Section` + `Block` tree; mismatch with Traces' proposed strict-TOML frontmatter schema architecture ([ticket 16](../issues/16-schema-note-typing-and-validation.md)).
2. **Reimplement Section Matcher Alone (~150-250 LoC, [H] estimate):**
   - *Pros:* Traces only persists `Heading { level, text, line }` in `Note`. A custom matcher in Traces would directly walk flat `headings: Box<[Heading]>`, check required headings in order, depth limits, and report `SourceLine` directly (which `schematter-validator` cannot do, as it only reports crumb strings). Zero dependency weight.
   - *Cons:* Deviates from `document-schema.org` spec; requires maintaining the sequential pointer logic in Traces.
3. **Vendor or Adapt:** Not recommended over a native 200-line heading matcher given Traces' persisted data model [H].

### Adversarial Assessment: Where it is a Poor Fit for Traces

- **No Source Spans / Line Numbers:** `schematter-validator` has no concept of byte offsets or line numbers. Violations carry only string crumbs (`bad › Jon visited Paris › Details`). Traces diagnostics require precise lines/spans (`SourceLine` from `Heading::line()`). Feeding Traces diagnostics from `schematter-validator` requires fuzzy re-grepping of lines [H].
- **Mismatched Data Models:** Traces stores flat headings and list items. Building a recursive `Section` tree with empty `Block` vectors on every validation run creates useless allocations [H].
- **Schema Dialect Mismatch:** `document-schema` pairs section matching with full JSON Schema 2020-12 frontmatter in YAML. Traces ticket 16 decided on schema validation for frontmatter using Rust/TOML idioms; pulling JSON Schema 2020-12 into Traces for body schemas creates dual schema paradigms [H].
