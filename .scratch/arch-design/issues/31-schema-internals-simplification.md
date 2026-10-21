# What schema internals survive now that schema produces a checker and a class hierarchy?

Type: grilling
Status: open
Blocked by: 16, 28, 34, 35

## Question

[Ticket 16's Answer](./16-schema-note-typing-and-validation.md) fixes what the
schema subsystem must produce: resolved effective fields with a pure
`check`, a plain class hierarchy keyed by `ClassName`, field descriptors
(`origin`, `description`, `parent_order`) and violation codes. Decide which of
the current internals still earn their cost:

- `fields/select.rs` external select values (about 2.0k lines) and the deep
  clone of each entry list into every inheriting schema (a shared lookup would
  avoid it; unmeasured);
- imperative option parsing against serde, given the strict
  `deny_unknown_fields` format;
- the five nested field-error enums and `RawSchemaFieldType` /
  `SchemaFieldTypeTag` / `SchemaFieldType` overlap;
- the CSR inheritance graph against a vault-scale need (the author's vault is
  one example, not a bound);
- the test-only `SchemaService::new` and the template `Schema` handle;
- where the resolved-field set and `ClassHierarchy` are built, and how a host
  replaces the snapshot.

Facts: `survey/schema.md`; scouts `FG-schema-model.md` and
`B-class-expander-critique.md` in `research/16-prep/round3/` (claims tagged [S]
were not re-read); the decision's brief `research/16-prep/brief.md`.

Alternatives: keep as is; collapse the field-type enums; replace the graph;
split parsing from resolution. Include the keep-as-is option and what it means
for the 12k lines (62 percent test code).

Inputs: ticket 16 Answer, crate topology (28).

Graduated from fog: "Schema internals" in the map.

Also from 16: a strict `[binding]` table (`tags`, `paths` with `!`
exclusions) on class schemas, not inherited; `global` becomes a lowest-
precedence layer for every note (update the invariant at
`src/schema/mod.rs:48-51`); new schema keys `description` and an `origin`
record; multiple `extends` parents are kept.

## Inputs from the IWE and Fileclass research

See [research 34](../research/34-iwe-query-schema-and-agent-write-gating.md)
and [35](../research/35-fileclass-model-binding-and-validation.md). Fileclass:
28 field types in the AST, of which 22 take input, 2 are dead stubs (`Formula`,
`Lookup`) and 3 are canvas sync; `Cycle` validates exactly like `Select` and
only about ten are structurally distinct (`src/schema/field.ts`,
`src/fields/support.ts`, `validate.ts`) [V]. Nested fields are identified by
`path\0name`, so an object property never collides with a root field
(`src/schema/inheritance.ts`) [V]. A child redefinition replaces the ancestor's
field in the ancestor's position; `excludes` accumulate toward ancestors; the
global class is the first entry of the merge list, so it has the lowest
precedence for every note [V]. `step` is a UI helper only and `required`
treats `[]` as present (`validate.ts:58`) [V]. IWE binds by globs in host
config with gitignore-style `!` (the last matching pattern decides), has no
inheritance, and composes overlapping schemas order-free [V]. Both tools check
bindings that can never bind at design time (Fileclass strikes through a tag
containing a space; IWE exits 2 for unreachable entries) [V], a candidate for
`traces check` schema errors. IWE delegates validation to an external crate
(`schematter-validator` 0.2.0); see [ticket
36](./36-research-schematter-validator-and-document-schema.md).
