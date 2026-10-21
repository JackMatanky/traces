# How do markdown/document tools model positions, metadata caches, and heading/link structure?

Type: research
Status: resolved
Blocked by: none

## Question

`Note` discards byte spans (only `SourceLine` on headings/list items), parses with pulldown-cmark twice per note, duplicates list-item fields/tags into note-level collections, stores multi-valued inline fields but reads only the first, carries `String` status names per task, and is serialized whole into redb (`survey/note.md` H2/H3/H5/H6/H8, `survey/root-primitives.md` H6). The glossary claims slugs/footnotes/callouts that code lacks.

Study: Obsidian's `CachedMetadata` shape (headings, links, sections, listItems with `position`), Dataview's `markdown-page`/`ListItem` model + inline-field extraction (`src/data-import/markdown-file.ts`, `inline-field.ts`), Tasks plugin's `Task` model, `rumdl` (single-pass markdown rule engine: how it gets heading slugs, positions, link structure without a second parse), `markdown-rs` mdast positions, `pulldown-cmark` `into_offset_iter` + `OffsetIter`, `rowan`/rust-analyzer lossless trees (for contrast, not recommendation), rustc_span `Span` + ruff `TextRange`/`TextSize` and `ruff_source_file::LineIndex`, `tower-lsp`/`lsp-types` position encoding requirements (UTF-16 vs UTF-8 offsets).

Focus: minimum position information that keeps later features cheap (edit-in-place, diagnostics, LSP) vs persisted-size cost; whether the document model is an AST, a flat extracted "facts" table, or both; where list/task items belong (child of note vs separate indexed entity); multi-value metadata semantics; how an editor-overlay (unsaved buffer) flows through the same model.

Informs: note document model, index read model, forward consumers, value model.

Deliverable: `.scratch/arch-design/research/10-document-model-spans-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at `.scratch/arch-design/research/10-document-model-spans-precedent.md`.
