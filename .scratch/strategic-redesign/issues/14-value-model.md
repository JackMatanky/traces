# What is the single dynamic value model, and who owns it?

Type: grilling
Status: open
Blocked by: 2, 12, 1

## Question
Should the four value enums (`FieldValue`, `FieldValueRef`, `NoteFieldValue`, `NoteFieldValueRef`) collapse into one value model, and if so what are its numeric model, extension variants (Duration, Link, Date, DateTime), ownership/borrowing story, serialization form, and home module? If not, what is the explicit contract between the schema-side and note-side representations?

Verified facts (`research/01` claims 1-2): four enums, 3-stage conversion chain (`src/note/metadata.rs:71-72`), `FieldValue` separates Int(i64)/Float(f64) while `NoteFieldValue` collapses to f64 (integer precision lost at the note boundary, `src/note/field.rs:48`) (`src/field.rs:596`, `src/note/field.rs:43`), `NoteFieldValue` size budget 80 bytes (`src/note/field.rs:580`), comparison/containment semantics live inside `note/field.rs:393-447` rather than query, `src/field.rs` imports `temporal`.
Alternatives: unify; keep two with a defined mapping; unify the data model but keep a borrowed read view; deserialize directly with serde into the final type. Include the keep-as-is option.
Inputs: `survey/root-primitives.md`, `survey/note.md`, `survey/schema.md`, `survey/cross-cutting.md` H2, `research/02`, `research/11` (sizes), compat decision.
Unlocks: coercion and comparison policy, schema-note typing, note document model, query core, template boundary.
