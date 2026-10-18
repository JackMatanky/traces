# Does the schema subsystem validate and type note data, and where does that capability live?

Type: grilling
Status: open
Blocked by: 14, 15

## Question
`src/schema` (12k LoC; ~4.7k production) models field definitions and inheritance but never touches note values; the index extracts `fileClass` independently. Should schemas drive validation/typing of note frontmatter and inline fields (advisory diagnostics vs gating vs none), and if so which module owns the validator, the diagnostic type, and the typed-field representation shared with templates (`ui` prompts) and query?

Facts: `survey/schema.md` §2-3, `src/index/service.rs:1656-1840`, four field-type enums (`RawSchemaFieldType`, `SchemaFieldTypeTag`, `SchemaFieldType`, `NoteFieldType`), `FileClassExpander` seam (judged sound), global schema semantics, `SchemaService` `expect(dead_code)` constructor.
Alternatives: keep schema passive (template/prompt metadata only); add advisory validation pass producing structured diagnostics; schema-typed ingestion at index time. Include the keep-as-is option and what it means for 12k lines.
Inputs: `survey/schema.md`, `research/03`, value model and coercion decisions, compat envelope.
Graduates fog: schema internals simplification (see map).
