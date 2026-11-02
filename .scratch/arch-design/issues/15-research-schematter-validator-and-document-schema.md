# What are `schematter-validator` and the document-schema spec, and do they fit Traces' body-section needs?

Type: research
Status: resolved
Blocked by: 13

## Question

[Ticket 13's
research](../research/13-iwe-query-schema-and-agent-write-gating.md) showed an
external crate for schemas. IWE delegates schema parsing, compilation and
validation to `schematter-validator = "0.2.0"` (`Cargo.toml:20` of
`iwe-org/iwe`). Frontmatter uses JSON Schema 2020-12. The body uses an ordered
section matcher with token budgets. The crate implements a published spec
(`https://document-schema.org/draft/2026-06/schema`). The IWE digests contain
only its callers (`crates/liwe/src/schema.rs`, `schema/document.rs`,
`crates/diwe/src/schema.rs`), not the crate.

[Body-section schemas](./37-body-section-schemas.md) and [schema
internals](./36-schema-internals-simplification.md) need crate details. They
compare two choices: adopt a crate for the body matcher, or write a small
heading matcher over persisted `Note` headings.

Find out these facts from primary sources (crates.io or docs.rs page text, the
source repository, the spec text):

- Maintainer, repository, licence, release history, downloads, MSRV, edition.
- Identity: confirm if the author or organisation is the same as IWE.
- Size in lines of code, direct and transitive dependencies, and compile-time
  weight.
- Dependencies: check if it pulls `serde_yaml`/`serde_json`, a regex engine,
  `jsonschema`, or a tokenizer.
- Input model: does it validate a generic tree (serde_json::Value), a caller
  document tree (`liwe::schema::document`), or both.
- Data input: could Traces feed it `NoteFieldValue`-shaped data and a heading
  list without conversion through YAML.
- Spec section matcher semantics: ordered, greedy, and non-backtracking.
- Spec rules: `minContains`/`maxContains`, `additionalSections`, and terminal
  wildcards.
- Token budgets: how tokens are counted. Is the tokenizer pluggable or a hard
  dependency.
- Error and violation structure: breadcrumb, `schemaPath`, keyword, and hint.
- Separation: check separation between schema errors and document violations.
- Spec maturity and stability: draft status, version policy, independent
  implementations, and conformance suite.
- Reimplementation cost: what it would cost Traces to vendor, wrap or
  reimplement the section matcher alone. Measure this against heading data in
  `Note`.

Informs: [ticket 37](./37-body-section-schemas.md) (primary), [ticket
36](./36-schema-internals-simplification.md), reconciliation ticket 35.

Deliverable:
`.scratch/arch-design/research/15-schematter-validator-and-document-schema.md`
(<= 200 lines).

Instructions:

- Cite URLs and file paths for every claim.
- Mark [V], [H], and [S].
- Do not decide. State fit, cost and risk.

## Answer

Deliverable: [research
15](../research/15-schematter-validator-and-document-schema.md). Sources are
docs.rs, the spec site, and the `iwe-org/schematter` repository. Crate internals
are [S] (read by the research agent, not re-read by the owner).

- The crate `schematter-validator` 0.2.0 uses Apache-2.0 and shares IWE's
  author and organization. The author split it from IWE in 2026-07. It has two
  releases, about 2,600 downloads, one implementation, and a draft spec
  (`draft/2026-06`). The crate has about 1,200 lines and depends on
  `jsonschema` 0.47, `serde_json`, `serde_yaml_ng` and `regex`. It lacks a
  markdown parser and a tokenizer. Caller-supplied integers provide token
  counts.
- Input: frontmatter as `serde_json::Value`. The body is a caller-built
  recursive `Document { sections, blocks, token counts }`. Violations carry
  string breadcrumbs, a JSON-Pointer `schemaPath`, keyword and `hint`.
  Violations carry no source lines. Schema errors are separated from violations.
- Fit: poor for Traces. It brings a second schema dialect (JSON Schema 2020-12
  in YAML) next to ticket 20's strict TOML. Traces would rebuild a recursive
  section tree from its flat `Heading { level, text, line }` slice. Diagnostics
  lack spans. A native flat heading matcher needs 150 to 250 lines [H] and
  reports the heading line directly. The spec's matcher semantics (ordered,
  greedy, `minContains`/`maxContains`, `additionalSections`, terminal wildcard,
  duplicate-entry schema error) do not need the crate.

Unresolved: none for ticket 37 to wait on. Downstream: [body-section
schemas](./37-body-section-schemas.md) (the "adopt a crate" alternative is
answered), [schema internals](./36-schema-internals-simplification.md).
