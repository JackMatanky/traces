# What do IWE's query language, document schema, search and MCP write path teach Traces?

Type: research
Status: resolved
Blocked by: none

## Question

The data-language side of IWE (`iwe-org/iwe`), complementing [ticket 12](./12-research-iwe-core-and-hosts-architecture.md) (core and hosts). Round 4 of the ticket 20 prep read only `crates/iwe/docs/schema.md`.

Previous work did not study these areas:

- Query language (`docs/query-language.md`, `liwe/src/query/*`: filter, project, sort, block eval, block update, execute, cli).
- Frontmatter handling (`model/frontmatter.rs`).
- Schema subsystem (`liwe/src/schema/*`, `crates/diwe/src/schema.rs`, `docs/document-schema.md`, `docs/cli-schema.md`, schema inference `schema/infer.rs`).
- Search and ranking (`diwe/src/search*.rs`, `docs/search-ranking.md`, `liwe/src/collation.rs`).
- CLI verbs (`docs/cli-find|retrieve|count|update|create|new|stats|tree`).
- MCP tool set (`crates/iwec`, `docs/mcp.md`, tests) and write gating.
- Config bindings (`docs/configuration.md`).
- Agent integration (`iwe/src/internal/claude/*`, `init/*`, `docs/feature-ai-commands.md`).

Focus, against decisions taken or open in this map:

- [Coercion and comparison](./19-coercion-and-comparison-policy.md) and the [value model](./18-value-model.md): how IWE types frontmatter values. Queries and sorts (collation, numeric, dates), cross-type rules, nulls.
- [Schema typing](./20-schema-note-typing-and-validation.md): class binding by glob and `type`, and multi-class composition. Violation shape (`schemaPath`, keyword, `hint`), and exit codes for schema errors versus violations. Inference of schemas from a vault, strict MCP write gating, and delta handling. What IWE checks after a write, and how it reports violations to the agent. Re-verify round-4/5 claims about IWE against source. Correct any wrong claims.
- [Query core](./28-query-core-and-plan.md): predicate model, projection, and update operators on frontmatter and blocks. How filters compose, pushdown or not, result shapes, `find` versus `retrieve`.
- [Body-section schemas](./38-body-section-schemas.md): the ordered, greedy section matcher, header patterns, blocks, token budgets (`diwe/src/tokens.rs`) and how violations locate in the body.
- [Schema internals](./37-schema-internals-simplification.md): format choice (JSON Schema), size of the implementation, what it costs.
- Agent write path: which MCP tools exist, and which write operations are gated. Atomicity and what the agent sees on failure. How this compares to the guardrail channels in ticket 20.

Informs: tickets 19, 20 (re-test), 28, 33 (write use cases), 37, 38.

Deliverable:
`.scratch/arch-design/research/13-iwe-query-schema-and-agent-write-gating.md` (<= 300 lines).
Source: `docs/digests/iwe-org-iwe-*digest.txt`.

Strategy:

- Use the pre-split tree at `/tmp/iwe/` (recreate if missing).
- Read docs first, then source by symbol, tests as behaviour specs.
- Cite repo path plus symbol or file-relative line for every claim.
- Mark [V] and [H].
- Do not propose the final decision.

Structure:
(1) what IWE actually does.
(2) invariants and owners.
(3) tradeoffs and costs.
(4) transferable lessons.
(5) lessons that do NOT transfer and why.
(6) what this changes or challenges in the named Traces decisions, including a list of round-4/5 claims about IWE now confirmed or corrected.

## Answer

Deliverable: [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md), with the claims below spot-checked against the digest by the owner.

- Query semantics converge with ticket 19 (strict scalar types, cross-type equality false, ICU natural collation, nulls first, rank-based total order). The query is a serialisable YAML or JSON document with a published JSON Schema. This is input to ticket 28 (does Traces' IR need a serde form for hosts?).
- Schema: bound by host-config globs with gitignore-style `!`, no inheritance, order-free composition. Violations carry breadcrumb, hint, `schemaPath` and keyword. Exit codes are 0, 1, 2 as ticket 20 already adopts.
- MCP gating is always on. It validates the whole pending document with no delta rule. A pre-existing violation blocks an edit [V]. All multi-document changes are all-or-nothing, and query mutations need `expect` count guards. This supports ticket 20's delta rule and adds a bulk-guard idea for the MCP host.
- Body schema: ordered greedy non-backtracking matcher, occurrence bounds, `additionalSections`, and terminal wildcards. Token budgets through a model-specific tokenizer: input to ticket 38. Implemented by an external crate. Follow-up [ticket 15](./15-research-schematter-validator-and-document-schema.md).
- Claims about IWE in earlier research, checked at source: it does NOT bind by a frontmatter `type`. Round 4 file D was wrong, and the round-4 report was right. The MCP gate is always on and "strict" means `expect` guards. Fields with a reserved prefix are validated since iwe 0.22.0. The doc read in round 4 (`crates/iwe/docs/schema.md:83`) is stale, and the claim in round 4's report is withdrawn. A Claude Code post-tool hook also exists, which ticket 20 already treats as a recipe.
- Corrections made by the owner to the agent's draft: two claims were removed. "Traces has an SQL query engine and 100k-note target" (neither is true). "hooks are an effective guardrail" (no evidence in the digest).

Unresolved: crate fit (ticket 15). The `schematter` semantics beyond IWE's callers.
Downstream: tickets 28, 33, 34, 37, 38 carry an "Inputs" section. Ticket 20 has a comment.
