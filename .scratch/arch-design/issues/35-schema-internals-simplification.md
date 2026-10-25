# What schema internals survive now that schema produces a checker and a class hierarchy?

Type: grilling
Status: open
Blocked by: 20, 32, 13, 14

## Question

[Ticket 20's Answer](./20-schema-note-typing-and-validation.md) fixes what the
schema subsystem must produce:

- Produce resolved effective fields with a pure `check`.
- Produce a plain class hierarchy keyed by `ClassName`.
- Produce field descriptors (`origin`, `description`, `parent_order`).
- Produce violation codes.

Decide which of the current internals still earn their cost:

- Decide the `fields/select.rs` external select values (about 2.0k lines).
- Decide the deep clone of each entry list into every inheriting schema.
  Unmeasured: a shared lookup avoids the clone.
- Decide the imperative option parsing against serde, given the strict
  `deny_unknown_fields` format.
- Decide the five nested field-error enums and the `RawSchemaFieldType` /
  `SchemaFieldTypeTag` / `SchemaFieldType` overlap.
- Decide the CSR inheritance graph against a vault-scale need. The author's
  vault is one example, not a bound.
- Decide the test-only `SchemaService::new` and the template `Schema` handle.
- Decide where the resolved-field set and the `ClassHierarchy` are built.
- Decide how a host replaces the snapshot.

Facts: `survey/schema.md`; scouts `FG-schema-model.md` and
`B-class-expander-critique.md` in `research/20-prep/round3/` (claims tagged [S]
were not re-read); the decision's brief `research/20-prep/brief.md`.

Alternatives:

- Keep as is.
- Collapse the field-type enums.
- Replace the graph.
- Split parsing from resolution.

Include the keep-as-is option and what it means for the 12k lines (62 percent
test code).

Inputs: ticket 20 Answer, crate topology (32).

This ticket graduates these fog items: "Schema internals" in the map.

Also from 20: the schema subsystem has a strict `[binding]` table (`tags`,
`paths` with `!` exclusions) on class schemas. The table is not inherited.
`global` becomes a lowest-precedence layer for every note. Update the invariant
at `src/schema/mod.rs:48-51`. The schema subsystem gets new schema keys
`description` and an `origin` record. The schema subsystem keeps multiple
`extends` parents.

## Inputs from the IWE and Fileclass research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md). Fileclass: 28
field types in the AST, of which 22 take input, 2 are dead stubs (`Formula`,
`Lookup`) and 3 are canvas sync; `Cycle` validates exactly like `Select` and
only about ten are structurally distinct (`src/schema/field.ts`,
`src/fields/support.ts`, `validate.ts`) [V]. Nested fields are identified by
`path\0name`, so an object property never collides with a root field
(`src/schema/inheritance.ts`) [V]. A child redefinition replaces the ancestor's
field in the ancestor's position; `excludes` accumulate toward ancestors; the
global class is the first entry of the merge list, so it has the lowest
precedence for every note [V]. `step` is a UI helper only and `required` treats
`[]` as present (`validate.ts:58`) [V]. IWE binds by globs in host config with
gitignore-style `!` (the last matching pattern decides), has no inheritance, and
composes overlapping schemas order-free [V]. Both tools check bindings that can
never bind at design time (Fileclass strikes through a tag containing a space;
IWE exits 2 for unreachable entries) [V], a candidate for `traces check` schema
errors. IWE delegates validation to an external crate (`schematter-validator`
0.2.0); see [ticket
15](./15-research-schematter-validator-and-document-schema.md).
