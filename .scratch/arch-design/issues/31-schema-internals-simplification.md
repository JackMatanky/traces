# What schema internals survive now that schema produces a checker and a class hierarchy?

Type: grilling
Status: open
Blocked by: 16, 28

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
