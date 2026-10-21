# IWE Query, Document Schema, and Agent Write Gating

- **Topic:** Architectural investigation of IWE (`iwe-org/iwe`) data language,
  document schema, query core, search, and MCP write gating for Traces.
- **Ticket:** [ticket 13](../issues/13-research-iwe-query-schema-and-agent-write-gating.md).
- **Sources Examined:** Pre-split tree `/tmp/iwe/` and original digests in
  `docs/digests/iwe-org-iwe-*digest.txt` (`crates/liwe/`, `crates/diwe/`,
  `crates/iwec/`, `crates/iwe/`, `docs/`).

---

## 1. What IWE Actually Does

### 1.1. Query, Filter, Projection, and Update Operators

- **Operator Model:** MongoDB-style YAML queries over four operations: `find`,
  `count`, `update`, `delete` [V: `docs/query-language.md:11`,
  `crates/liwe/src/query/execute.rs:18`]. Filters support equality, comparison
  (`$gt`, `$gte`, `$lt`, `$lte`), sets (`$in`, `$nin`, `$all`), existence
  (`$exists`), cardinality (`$size`), and boolean logic (`$and`, `$or`, `$nor`)
  [V: `docs/query-language.md:32-63`]. Graph operators query identity (`$key`,
  `$startsWith`) and relational edges (`$includes`, `$includedBy`,
  `$references`, `$referencedBy`) with traversal bounds (`minDepth`, `maxDepth`)
  and cardinality tests [V: `crates/liwe/src/query/document.rs:25`].
- **Block Selection and Updates:** Blocks are AST nodes (node: `$header`,
  `$paragraph`, `$item`, `$code`, `$table`, `$ref`, `$hr`; tree: `$section`,
  `$quote`, `$list`). `$content` tests block presence in filters; `$blocks`
  (data entries) and `$matches` (grep lines) project blocks [V:
  `docs/query-language.md:270-358`]. Mutations use unit block operators
  (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`,
  `$delete`) combined with frontmatter `$set`/`$unset` [V:
  `crates/liwe/src/query/block_update.rs:36`].
- **Evaluation & Pushdown:** Zero database index pushdown. The engine runs
  entirely in-memory over `Graph` using Rayon parallelism [V:
  `crates/liwe/src/query/eval.rs:43-100`]. `eval_and` partitions clauses into
  graph-walk generators and scalar predicates, intersecting candidate key sets
  before predicate filtering [V: `eval.rs:60`].
- **Result Shapes & `find` vs `retrieve`:** `find` returns `Vec<FindMatch>`
  carrying projected frontmatter, system selectors (`$key`, `$title`,
  `$content`), and block slices [V: `crates/liwe/src/query/execute.rs:24`].
  `retrieve` is an LLM context assembler: takes search seeds, expands graph
  relations up to depth N, and trims document bodies against token budgets [V:
  `crates/diwe/src/retrieve.rs:20`].

### 1.2. Frontmatter Value Typing, Comparison, and Collation

- **Scalar Typing:** YAML scalars maintain strict types; no implicit
  string-number or string-boolean coercion [V:
  `crates/liwe/src/query/filter.rs:137`]. Cross-type equality (`deep_eq`)
  evaluates to `false` [V: `filter.rs:60`]; cross-type comparison
  (`cmp_ordered`) returns `None` [V: `filter.rs:85`].
- **Collation and Sorting:** `sort_in_place` sorts single keys stably,
  breaking ties by ascending document key [V: `crates/liwe/src/query/sort.rs:13`].
  Text fields use ICU natural collation (`TitleCollator` via `icu_collator`),
  ordering numerics naturally ("Note 2" < "Note 10") and ignoring emoji except
  to break ties [V: `crates/liwe/src/collation.rs:20-60`]. Supports `binary`
  and locale tags (`sv-SE`). Nulls sort first in ascending, last in descending
  [V: `sort.rs:44`]. Cross-type sorting defines a total order by `type_rank`
  (Null=0 < Bool=1 < Number=2 < String=3 < Sequence=4 < Mapping=5) [V:
  `sort.rs:75`].

### 1.3. Document Schema, Composition, Violations, and Inference

- **Format:** YAML documents (`https://document-schema.org/draft/2026-06/schema`).
  Frontmatter is validated via literal JSON Schema 2020-12 (only in-document
  `$ref` allowed) [V: `docs/document-schema.md:21-114`]. Body schema defines
  structural tree constraints.
- **Binding & Composition:** Schemas bind strictly via path globs in
  `.iwe/config.toml` (`[schemas.<name>].match`) [V:
  `crates/diwe/src/schema.rs:20`]. There is NO frontmatter class binding and NO
  inheritance [V: `docs/document-schema.md:68`]. Multiple schemas matching a
  path compose order-free like `allOf`; each yields an independent `KeyReport`
  in JSON output [V: `crates/iwe/tests/schema_validate_test.rs:104`].
- **Violations & Exit Codes:** Violations carry `breadcrumb` (header path),
  `message`, `hint` (nearest `description` walking outward), `schemaPath`
  (JSON Pointer), and failing `keyword` [V:
  `crates/diwe/src/schema.rs:render_reports_text`,
  `crates/iwe/tests/schema_validate_test.rs:60`]. Exit codes: `0` (clean), `1`
  (document violations), `2` (schema syntax or config error) [V:
  `document-schema.md:432-438`].
- **Inference:** `iwe schema` scans frontmatter across documents, reporting
  field coverage, YAML type breakdown, distinct count, and enum values (<100
  distinct alphanumeric values) [V: `crates/liwe/src/schema/infer.rs:15-115`].
  It does not infer body schemas or generate JSON Schema files.

### 1.4. Body-Section Matcher and Token Budgets

- **Matching Algorithm:** Ordered, sequential, greedy, non-backtracking walk
  [V: `docs/document-schema.md:197`]. Evaluates `sections` against header
  schemas (`const`, `enum`, `pattern`, `minLength`, `maxLength`, `maxTokens`).
  Advances a pointer into the schema entries; entries before the pointer are
  closed. Wildcard entries greedily consume all remaining sections and must be
  terminal (non-terminal wildcards trigger exit 2) [V:
  `document-schema.md:230`].
- **Occurrences & Leftovers:** `minContains` (default 1) and `maxContains`
  (default unbounded) enforce occurrences across the whole node, not consecutive
  runs. Unmatched sections fall to `additionalSections` (`true` open, `false`
  forbidden, or subschema) [V: `document-schema.md:124-188`].
- **Token Budgets:** Evaluated using `tiktoken_rs` with `o200k_base`
  (GPT-4o) [V: `crates/diwe/src/tokens.rs:8`]. `maxTokens` bounds headers,
  section subtrees, and overall document bodies [V:
  `document-schema.md:97-158`].

### 1.5. MCP Tool Set, Write Gating, and Agent Integration

- **MCP Surface:** 14 tools (`iwe_find`, `iwe_retrieve`, `iwe_tree`,
  `iwe_stats`, `iwe_squash`, `iwe_create`, `iwe_update`, `iwe_delete`,
  `iwe_query`, `iwe_rename`, `iwe_extract`, `iwe_inline`, `iwe_normalize`,
  `iwe_attach`) [V: `docs/mcp.md:35-124`].
- **Gated Operations:** All mutating tools (`create`, `update`, `delete`,
  `query`, `rename`, `extract`, `inline`, `attach`) execute
  `validate_pending_documents` before writing [V:
  `crates/iwec/CHANGELOG.md:177`, `crates/diwe/src/schema.rs:255`]. If any
  touched document violates its bound schema, the entire change aborts, leaving
  disk and graph untouched [V: `crates/iwec/tests/schema_test.rs:17`].
  `iwe_normalize` and `iwe_squash` are exempt [V: `schema_test.rs:105`].
- **Failure Reporting & Strictness:** Failures return a JSON-RPC `McpError`
  with human message plus structured JSON `data.violations` [V:
  `schema_test.rs:32`]. Mutation operations in `iwe_query` require strict
  `expect` count guards (pinning document and block match counts) [V:
  `crates/iwec/tests/query_test.rs:80`].
- **Delta Handling:** IWE has NO delta rule [V]. Writes are validated against
  the entire document after edit; any pre-existing violation aborts the write.
- **Claude Code Hooks:** `crates/iwe/src/internal/claude/hook/post_tool.rs`
  intercepts Claude Code `Write`, `Edit`, `MultiEdit`, and `Bash` commands,
  running `schema_report` and injecting violation notices into agent context [V].

---

## 2. Invariants and Owners

- **Invariants:**
  1. *Schema Errors Precede Document Checks:* Schema syntax errors and invalid
     globs fail with exit 2 before any document is evaluated [V:
     `docs/cli-schema.md:10`].
  2. *Terminal Wildcards:* Wildcards in body section schemas must be the final
     entry; any preceding wildcard is a fatal schema error [V:
     `docs/document-schema.md:231`].
  3. *All-or-Nothing Mutation:* If any document in a multi-document mutation
     fails schema validation or `expect` guards, zero documents are written [V:
     `crates/liwe/src/query/execute.rs:105`].
  4. *Pairwise Disjoint Updates:* Concurrent block updates within a document
     must have disjoint extents after target coalescing [V:
     `docs/query-language.md:479`].
- **Subsystem Owners:**
  - `schematter-validator` (external crate): Schema parsing, compilation, AST
    validation, and violation structure [V: `crates/liwe/src/schema.rs:4`].
  - `liwe::schema`: Document tree extraction from `Graph` and schema inference
    [V: `crates/liwe/src/schema/document.rs`, `infer.rs`].
  - `diwe::schema`: Config-level glob compilation (`SchemaBindings`), disk
    schema loading, and pending document validation [V:
    `crates/diwe/src/schema.rs`].
  - `liwe::query`: Query document parsing, block evaluation, collation, and
    in-memory execution [V: `crates/liwe/src/query/`].
  - `iwec`: MCP tool registration, write gating, and structured RPC error
    mapping [V: `crates/iwec/`].

---

## 3. Tradeoffs and Costs

- **No Delta Rule:** Without a delta rule, agents cannot edit clean
  sections of legacy documents that hold existing schema violations [H]. Every
  edit requires fixing the entire document, causing write lockouts on legacy
  vaults.
- **Full In-Memory Execution:** Field filters scale linearly (12 ms at 20k
  notes) and `$includedBy`/`--roots` are super-linear (60 ms at 20k) per
  IWE's own benchmark [V: `docs/benchmark.md:185-208`]; the earlier "poor past
  10,000" reading is [H] and unmeasured for Traces.
- **No Class Inheritance:** Pure path glob matching avoids class conflict
  resolution and MRO complexity, but cannot model polymorphic frontmatter types
  (e.g. `book` extending `work`) [H].
- **Proprietary Tokenizer:** Body token budgeting hardcodes OpenAI's BPE
  (`tiktoken_rs` `o200k_base`), creating an unneeded external model bias [H].

---

## 4. Transferable Lessons

1. **Exit Code Distinction:** Adopting exit 2 for broken schema definitions
   and exit 1 for note violations prevents invalid schemas from masking as
   clean notes [V: `crates/iwe/tests/schema_validate_test.rs:135`].
2. **Actionable Violation Hints:** Walking outward to the nearest `description`
   to supply a `hint` enables LLM self-correction during tool loops [V:
   `crates/diwe/src/schema.rs:235`].
3. **Structured Diagnostics:** Providing `schemaPath` (JSON Pointer) alongside
   breadcrumb and keyword allows programmatic error handling [V:
   `crates/iwec/tests/schema_test.rs:40`].
4. **`expect` Mutation Guards:** Pinning expected target counts (`expect: 1`)
   protects against runaway agent updates [V:
   `crates/liwe/src/query/block_update.rs`].
5. **Separation of Search vs Retrieval:** Retaining `find` for structured
   projection/queries and `retrieve` for agent reading context (with BFS
   expansion and token budgeting) is clean and effective [V:
   `crates/diwe/src/retrieve.rs`].
6. **Post-Tool Agent Hooks:** IWE ships a Claude Code `post_tool.rs` hook that
   validates the written file and injects a notice into agent context [V:
   `crates/iwe/src/internal/claude/hook/post_tool.rs`, `schema_report`]. It is
   one harness integration, not a general guardrail; ticket 20 already records
   it as a recipe and refuses to depend on hooks. Evidence of effectiveness is
   not in the digest [H].

---

## 5. Lessons that do NOT Transfer and Why

1. **Lack of Delta Rule:** Traces' open-world PKM model requires the delta rule
   `(field, code, hash(value))` [ticket 20]. Refusing writes due to
   pre-existing unrelated violations would break incremental PKM adoption.
2. **Pure Path-Glob Binding:** Traces requires frontmatter class binding with
   single/multiple inheritance (`extends`). Path-only globs cannot represent
   semantic note ontologies.
3. **In-Memory Query Graph (as a replacement for an index):** IWE queries run
   over the whole in-memory graph (12 ms for a field filter at 20k notes,
   `docs/benchmark.md:185-190`) after a 705 ms cold load. Traces' choice of redb
   is ticket 24's decision on ticket 11's measurements; this does not settle
   it. (Editor note: the original line said Traces has an "SQL query engine" and
   targets 100k notes; Traces has neither. Ticket 17 lists 50 to 50,000 notes.)
4. **Hardcoded BPE Tokenizer:** Traces should avoid hardcoding model-specific
   BPE encodings in core schema validators.

---

## 6. Challenges to Traces Decisions and Round-4/5 Claims Ledger

### 6.1. Verification of Round 4/5 Claims

| Claim & Location | Source Reality in IWE | Verdict |
|---|---|---|
| IWE validates per-type schemas (`round4/report.md:20`, `round4/D:502`) | Schemas bind strictly by path globs in config, NOT frontmatter `type` [V: `crates/diwe/src/schema.rs:20`, `docs/document-schema.md:55`]. | **Corrected** |
| Frontmatter uses literal JSON Schema 2020-12, body has structural schema (`round4/report.md:26`) | Verified directly in docs and validator integration [V: `docs/document-schema.md:10`, `crates/liwe/src/schema.rs:4`]. | **Confirmed** |
| Binding is config globs, order-free `allOf`, no inheritance (`round4/report.md:28`) | Verified; globs compose order-free, no class inheritance exists [V: `crates/diwe/src/schema.rs:40`, `document-schema.md:68`]. | **Confirmed** |
| Unknown schema keywords exit 2, document violations exit 1 (`round4/report.md:32`) | Verified in CLI tests; unknown keywords or missing schema files exit 2 [V: `crates/iwe/tests/schema_validate_test.rs:135`]. | **Confirmed** |
| Violations carry `schemaPath`, keyword, and `hint` from `description` (`round4/report.md:35`) | Verified in violation structures and report rendering [V: `crates/diwe/src/schema.rs:230`, `crates/iwec/tests/schema_test.rs:40`]. | **Confirmed** |
| Reserved-prefix fields (`_`, `$`, `.`, `#`, `@`) invisible to schema (`round4/report.md:38`) | **Refuted.** All fields are presented to validator [V: `crates/liwe/src/schema/document.rs:40`, `docs/document-schema.md:88`, CHANGELOG 0.22.0]. | **Corrected** |
| YAML dates presented to validator as ISO strings (`round4/report.md:39`) | Verified; YAML dates converted to ISO-8601 strings [V: `docs/document-schema.md:90`, `crates/liwe/src/query/filter.rs:125`]. | **Confirmed** |
| Write gating occurs only under "strict mode" (`round4/D:505`) | **Refuted.** MCP schema write gating is ALWAYS active; strict mode refers only to `expect` guards [V: `crates/iwec/CHANGELOG.md:177`]. | **Corrected** |
| IWE gates MCP writes only, direct edits unprevented (`ticket 20:55`, `round4:55`) | MCP writes are hard-gated, but Claude Code hooks (`post_tool.rs`) ALSO catch direct edits post-write [V: `crates/iwe/src/internal/claude/hook/post_tool.rs`]. | **Expanded** |

### 6.2. Architectural Challenges to Named Traces Decisions

- **Ticket 20 (Schema Note Typing & Validation):**
  - Confirms the exit code split (0 clean, 1 violations, 2 schema error), `hint`
    derivation from `description`, and machine `schemaPath` JSON Pointers.
  - Vindicates Traces' decision to design a **delta rule**: IWE's lack of a
    delta rule causes severe write lockouts on legacy files, proving Traces must
    not omit it.
  - IWE's Claude Code post-tool hook (`post_tool.rs`) is a harness-specific
    second line of defense; it does not change ticket 20's rule that no design
    depends on hooks.
- **Ticket 26 (Query Core) & Ticket 30 (Write Use Cases):**
  - Demonstrates value of `expect` count guards for precision agent mutations.
  - Shows that separating read query projection (`find`) from LLM context
    assembly with graph expansion (`retrieve`) is a superior agent interface.
- **Ticket 36 (Body Section Schemas):**
  - IWE's ordered, greedy, non-backtracking matcher with header constraints and
    occurrence bounds (`minContains`/`maxContains`) provides a concrete,
    field-tested blueprint for Traces body schemas.
