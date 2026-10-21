# What are `schematter-validator` and the document-schema spec, and do they fit Traces' body-section needs?

Type: research
Status: resolved
Blocked by: 34

## Question

[Ticket 34's research](../research/34-iwe-query-schema-and-agent-write-gating.md)
found that IWE delegates schema parsing, compilation and validation (frontmatter
as JSON Schema 2020-12, body as an ordered section matcher with token budgets)
to an external crate, `schematter-validator = "0.2.0"` (`Cargo.toml:20` of
`iwe-org/iwe`), implementing a published spec
(`https://document-schema.org/draft/2026-06/schema`). The IWE digests contain
only its callers (`crates/liwe/src/schema.rs`, `schema/document.rs`,
`crates/diwe/src/schema.rs`), not the crate.

[Body-section schemas](./32-body-section-schemas.md) and [schema
internals](./31-schema-internals-simplification.md) cannot weigh "adopt a crate
for the body matcher" against "write a small heading matcher over persisted
`Note` headings" without knowing the crate. Find out, from primary sources
(crates.io / docs.rs page text, the source repository, the spec text):

- who maintains it, repository, licence, release history, downloads, MSRV,
  edition, whether it is the same author or organisation as IWE;
- size (LoC), direct and transitive dependencies, compile-time weight, whether
  it pulls `serde_yaml`/`serde_json`, a regex engine, `jsonschema`, a tokenizer;
- its input model: does it validate a generic value tree (serde_json::Value?),
  a document tree built by the caller (as `liwe::schema::document` does), or
  both; could Traces feed it `NoteFieldValue`-shaped data and a heading list
  without converting through YAML;
- semantics the spec fixes: section matcher (ordered, greedy, non-backtracking),
  `minContains`/`maxContains`, `additionalSections`, terminal wildcards, token
  budgets (how tokens are counted; is the tokenizer pluggable or a hard
  dependency), error and violation structure (breadcrumb, `schemaPath`,
  keyword, hint), schema-error versus violation separation;
- spec maturity and stability (draft status, version policy, independent
  implementations, conformance suite);
- what it would cost Traces to vendor, wrap or reimplement the section matcher
  alone, measured against the heading data already persisted in `Note`.

Informs: [ticket 32](./32-body-section-schemas.md) (primary),
[ticket 31](./31-schema-internals-simplification.md), reconciliation ticket 29.

Deliverable: `.scratch/arch-design/research/36-schematter-validator-and-document-schema.md`
(<= 200 lines). Cite URLs and file paths for every claim; mark [V], [H], [S].
Do not decide; state fit, cost and risk.

## Answer

Deliverable: [research 36](../research/36-schematter-validator-and-document-schema.md),
from docs.rs, the spec site and the `iwe-org/schematter` repository (crate
internals are [S]: read by the research agent, not re-read by the owner).

- `schematter-validator` 0.2.0: Apache-2.0, same author and organisation as
  IWE, split out of IWE in 2026-07; two releases, about 2,600 downloads, one
  implementation, a draft spec (`draft/2026-06`). About 1,200 lines; depends on
  `jsonschema` 0.47, `serde_json`, `serde_yaml_ng` and `regex`; no markdown
  parser and no tokenizer (token counts are caller-supplied integers).
- Input: frontmatter as `serde_json::Value`, body as a caller-built recursive
  `Document { sections, blocks, token counts }`. Violations carry string
  breadcrumbs, a JSON-Pointer `schemaPath`, keyword and `hint`; no source lines.
  Schema errors are separated from violations.
- Fit: poor for Traces. It brings a second schema dialect (JSON Schema 2020-12
  in YAML) next to ticket 16's strict TOML, a recursive section tree that
  Traces would rebuild from its flat `Heading { level, text, line }` slice, and
  no spans for diagnostics. A native flat heading matcher is estimated at
  150 to 250 lines [H] and can report the heading line directly. Reusing the
  spec's matcher semantics (ordered, greedy, `minContains`/`maxContains`,
  `additionalSections`, terminal wildcard, schema error for duplicate entries)
  does not require the crate.

Unresolved: none for ticket 32 to wait on. Downstream: [body-section
schemas](./32-body-section-schemas.md) (the "adopt a crate" alternative is
answered), [schema internals](./31-schema-internals-simplification.md).
