# What do IWE's query language, document schema, search and MCP write path teach Traces?

Type: research
Status: resolved
Blocked by: none

## Question

The data-language side of IWE (`iwe-org/iwe`), complementing
[ticket 33](./33-research-iwe-core-and-hosts-architecture.md) (core and hosts).
Round 4 of the ticket 16 prep read only `crates/iwe/docs/schema.md`. Its query
language (`docs/query-language.md`, `liwe/src/query/*`: filter, project, sort,
block eval, block update, execute, cli), frontmatter handling
(`model/frontmatter.rs`), schema subsystem (`liwe/src/schema/*`,
`crates/diwe/src/schema.rs`, `docs/document-schema.md`, `docs/cli-schema.md`,
schema inference `schema/infer.rs`), search and ranking (`diwe/src/search*.rs`,
`docs/search-ranking.md`, `liwe/src/collation.rs`), CLI verbs
(`docs/cli-find|retrieve|count|update|create|new|stats|tree`), MCP tool set
(`crates/iwec`, `docs/mcp.md`, tests) and write gating, config bindings
(`docs/configuration.md`), and agent integration (`iwe/src/internal/claude/*`,
`init/*`, `docs/feature-ai-commands.md`) were not studied.

Focus, against decisions taken or open in this map:

- [Coercion and comparison](./15-coercion-and-comparison-policy.md) and the
  [value model](./14-value-model.md): how IWE types frontmatter values in
  queries and sorts (collation, numeric, dates), cross-type rules, nulls.
- [Schema typing](./16-schema-note-typing-and-validation.md): class binding by
  glob and `type`, multi-class composition, violation shape (`schemaPath`,
  keyword, `hint`), schema-error versus violation exit codes, inference of
  schemas from a vault, strict MCP write gating, delta handling, what IWE
  checks after a write and how it reports it to the agent. Re-verify the
  round-4/5 claims about IWE against source; correct any that were wrong.
- [Query core](./22-query-core-and-plan.md): predicate model, projection,
  update operators on frontmatter and blocks, how filters compose, pushdown
  or not, result shapes, `find` versus `retrieve`.
- [Body-section schemas](./32-body-section-schemas.md): the ordered, greedy
  section matcher, header patterns, blocks, token budgets (`diwe/src/tokens.rs`)
  and how violations locate in the body.
- [Schema internals](./31-schema-internals-simplification.md): format choice
  (JSON Schema), size of the implementation, what it costs.
- Agent write path: which MCP tools exist, which write operations are gated,
  atomicity, what the agent sees on failure; how this compares to the
  guardrail channels in ticket 16.

Informs: tickets 15, 16 (re-test), 22, 31, 32, 26 (write use cases), 29.

Deliverable: `.scratch/arch-design/research/34-iwe-query-schema-and-agent-write-gating.md`
(<= 300 lines). Source: `docs/digests/iwe-org-iwe-*digest.txt`; use the
pre-split tree at `/tmp/iwe/` (recreate if missing). Docs first, then source by
symbol, tests as behaviour specs. Cite repo path plus symbol or file-relative
line for every claim. Structure: (1) what IWE actually does, (2) invariants and
owners, (3) tradeoffs and costs, (4) transferable lessons, (5) lessons that do
NOT transfer and why, (6) what this changes or challenges in the named Traces
decisions, including a list of round-4/5 claims about IWE now confirmed or
corrected. Mark [V] and [H]. Do not propose the final decision.

## Answer

Deliverable: [research 34](../research/34-iwe-query-schema-and-agent-write-gating.md),
with the claims below spot-checked against the digest by the owner.

- Query semantics converge with ticket 15 (strict scalar types, cross-type
  equality false, ICU natural collation, nulls first, rank-based total order).
  The query is a serialisable YAML or JSON document with a published JSON
  Schema: input to ticket 22 (does Traces' IR need a serde form for hosts?).
- Schema: bound by host-config globs with gitignore-style `!`, no inheritance,
  order-free composition; violations carry breadcrumb, hint, `schemaPath` and
  keyword; exit 0, 1, 2 as ticket 16 already adopts.
- MCP gating is always on and validates the whole pending document, with no
  delta rule, so a pre-existing violation blocks an edit [V]; all multi-document
  changes are all-or-nothing, and query mutations require `expect` count guards.
  This supports ticket 16's delta rule and adds a bulk-guard idea for the MCP
  host.
- Body schema: ordered greedy non-backtracking matcher, occurrence bounds,
  `additionalSections`, terminal wildcards, token budgets via a model-specific
  tokenizer: input to ticket 32. Implemented by an external crate; follow-up
  [ticket 36](./36-research-schematter-validator-and-document-schema.md).
- Claims about IWE in earlier research, checked at source: it does NOT bind by
  a frontmatter `type` (round 4 file D was wrong; the round-4 report was
  right); the MCP gate is always on and "strict" means `expect` guards; fields
  with a reserved prefix are validated since iwe 0.22.0 (the doc round 4 read,
  `crates/iwe/docs/schema.md:83`, is stale, and the claim in round 4's report is
  withdrawn); a Claude Code post-tool hook also exists, which ticket 16 already
  treats as a recipe.
- Corrections made by the owner to the agent's draft: "Traces has an SQL query
  engine and 100k-note target" (neither is true) and "hooks are an effective
  guardrail" (no evidence in the digest) were removed.

Unresolved: crate fit (ticket 36); `schematter` semantics beyond IWE's callers.
Downstream: tickets 22, 26, 27, 31, 32 carry an "Inputs" section; ticket 16 has
a comment.
