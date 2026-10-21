# Ticket 20 round 4: external precedent, challenge ledger

Six scouts (`A-` to `F-` in this directory) searched Obsidian plugins, PKM class
systems, CMS frontmatter schemas, schema languages and linters, wikis and graph
systems, and agent/MCP vault tools. This file records what the orchestrator
re-checked, where the scouts disagree or over-reach, and what it means for
[ticket 20's Answer](../../../issues/20-schema-note-typing-and-validation.md).
Tags: [V] read by the orchestrator (URL given), [S] scout claim not re-read, [H]
recollection. The author's vault is not evidence anywhere in this file.

## 1. Verified by the orchestrator

| Claim | Result |
|---|---|
| Fileclass (successor to Metadata Menu, same file format) resolves two classes declaring the same key by "the last one wins"; `extends` takes one parent | [V] <https://mdelobelle.github.io/fileclass/schema/> |
| Fileclass binds a note to classes by frontmatter key, tag, folder path, bookmark group, Base view, then a global "preset" class (lowest precedence) | [V] same URL |
| Logseq DB tags can `extends` several parents; a root tag is the default parent | [V] <https://github.com/logseq/docs/blob/master/db-version.md>. Scout B's "multi-parent is an anti-pattern, users broke trees" is [S], unverified, and the docs support multi-parent as a feature |
| arXiv 2603.13404 shows agents ignore advisory warnings | [V] **Not supported.** It is a pilot (one local model, three seeds), success was zero in every condition, and it compares free-form docs vs JSON Schema vs schema plus structured diagnostics. Structured diagnostics reduced interface misuse, not semantic misuse. Scout F's headline claim is a mis-citation |
| Sitecore: ordered inheritance "first wins" caused silent bugs | [V] **Not supported.** Duplicate field names coexist (both shown); no winner. Best practice is unique names. Umbraco instead rejects the composition (`InvalidCompositionException`, <https://issues.umbraco.org/issue/U4-8631>) [V] |
| IWE (Rust: CLI `iwe`, LSP `iwes`, MCP `iwec`) validates documents against per-type schemas; a schema-violating MCP write is rejected with the violation named | [V] <https://github.com/iwe-org/iwe>, `crates/iwe/docs/schema.md` |
| MCP elicitation exists, so class-change confirmation has an in-band channel | [H] strong recollection; scout F wrongly says ticket 20 assumed none (the Answer already asks the user, denying only with no channel) |
| `step` has no precedent | [H] **Wrong.** JSON Schema `multipleOf` and HTML `step` are direct precedents (F's claim stands only for document-metadata tools) |

## 2. IWE, the closest analogue (read in full for schema.md)

- Frontmatter validated by literal JSON Schema 2020-12; the schema file also
  covers body structure (ordered sections, headers, blocks, token budgets).
- Binding is config-level globs (`[schemas.note] match = "notes/**"`), not a
  frontmatter class. Order-free: a document is validated against every schema
  whose glob matches, composing like `allOf`. A document matching nothing is
  unvalidated. There is no inheritance.
- Unknown keywords in a schema are errors ("a typo cannot silently validate
  nothing"); unreachable entries are rejected at load. Schema errors exit 2,
  document violations exit 1.
- Violations carry a `schemaPath`, the failing keyword and a `hint` taken from
  the nearest `description` walking outward. JSON output is one report per
  (document, schema).
- Reserved-prefix fields (`_`, `$`, `.`, `#`, `@`) are invisible to the schema.
  YAML dates are presented to the validator as ISO strings.

## 3. Challenge ledger

| Ticket 20 decision | Evidence | Verdict |
|---|---|---|
| Advisory on read, gating by host on writes | Wikidata (advisory constraint reports), SHACL, Fileclass ("a statement about your model, not a gate"), Obsidian Properties, Velite default non-strict, Sanity severities; gating only at tool-owned writes (IWE MCP, Notion API) [S for most] | **Keep** |
| Frontmatter class as the only binding | Fileclass and every Obsidian plugin add tag/path binding; IWE binds by glob only; Dendron path binding is reported fatal [S]. Notes without a class are unvalidated, including agent-created ones | **Keep, but record limits** (see Q3) |
| Schema in TOML, strict, custom (not JSON Schema) | IWE chose JSON Schema for pure validation; Traces schemas also drive prompts, completion, hover, file-link and label/value mapping that JSON Schema cannot express [H]; strictness matches IWE's reject-unknown-keywords | **Keep**; JSON Schema stays a projection |
| First-listed class wins | Fileclass: last wins [V]. IWE, JSON Schema `allOf`, SHACL, LDAP MUST/MAY: all classes apply, no winner [V for IWE, H for the rest]. CUE: unification is commutative, conflicts fail [H]. Umbraco: duplicates rejected [V]. Python MRO: first wins [H] | **Change or justify** (see Q1) |
| `extends` with several parents, first-listed wins | Fileclass: one parent [V]. Logseq: several [V]. Plone moved from class inheritance to behaviours (mixins) [H]. Existing behaviour is implemented and documented (`src/schema/raw.rs:31`) | **Leave; apply the same conflict surfacing** (see Q2) |
| `required` kept | Payload/Sveltia relax `required` for drafts [S]; Sanity warning vs error [S]; standard in JSON Schema, Notion, Airtable | **Keep**; host policy covers drafts |
| `step`, `min`, `max`, `format` | `multipleOf` and HTML `step` are precedents [H] | **Keep** |
| External value lists | Metadata Menu `valuesListNotePath`, Front Matter CMS `dataFile`, SMW and Plone vocabularies [S] | **Keep** |
| Open-world notes; unknown fields returned with a `closest` match, hosts filter | Kubernetes silent pruning incident [S]; Docusaurus/Quarto open metadata [S]; IWE ignores reserved-prefix fields [V] | **Keep**; consider a reserved-key convention |
| Delta rule | Letta/Graphiti per scout F [S]; ruff/mypy baselines [H] | **Keep** |
| Direct file edits detected, not prevented | IWE gates MCP writes only [V]; scouts C, D, F agree [S] | **Keep** |
| Violation carries field, code, expected, found | IWE adds `schemaPath`, failing keyword and a `hint` from `description` | **Add `hint`** (see Q4) |

## 4. Things Traces has not considered

- **Schema errors vs document violations.** IWE separates exit code 2 (broken
  schema) from 1 (violations), so a typo in a schema never reads as "notes are
  fine".
- **Design-time conflict checks.** IWE rejects unreachable entries at load and
  Umbraco rejects conflicting compositions at save. Traces could flag
  conflicting declarations once per class set instead of once per note.
- **Body-structure schemas** (required sections, header patterns, token
  budgets): IWE and mdschema validate them. Ticket 20 is frontmatter-only.
- **Violation locator.** If body schemas ever arrive, `Violation.field` needs a
  locator that is not only a frontmatter key.
- **Surgical edits.** Obsidian Local REST API and the Anthropic memory tool show
  agents damaging formatting with whole-file writes [S]; relevant to ticket 30
  and the MCP edit mechanism.

## 5. Scout reliability

- Scouts A and B disagree about Metadata Menu's multi-class behaviour; the only
  verified statement is Fileclass's own page. Scout A's "JSON Schema abandoned
  in Metadata Enforcer v2" and scout B's Dendron, Tana and SilverBullet claims
  are [S].
- Scout F's two headline claims are wrong or unsupported (section 1).
- Scouts C, D, E and F recommend changing multi-class resolution on the basis of
  CUE, LDAP, SHACL, Plone and Sitecore; of these only Umbraco and IWE were
  checked here.
- No scout ran a tool; LDAP semantics (MUST/MAY union) and CUE commutativity are
  [H], standard but unread here.

## 6. Outcome

Recorded in [ticket 20's Answer](../../../issues/20-schema-note-typing-and-validation.md):
order-independent multi-class rule, multiple `extends` parents kept, `global`
as a lowest-precedence layer with its own code family, class binding by tag and
path (an explicit known class suppresses bindings), `hint`, the exit-code
split, and body-section schemas deferred to ticket 36.
