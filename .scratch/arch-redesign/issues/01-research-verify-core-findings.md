# Research: verify value/note/schema/config/temporal findings

Type: research
Status: resolved

## Question

The area surveys in `.scratch/arch-redesign/survey/` were written by single-pass subagents and are unverified (see map Notes). Produce a **verification ledger** for every decision-critical claim below: re-check each against the source on `main` (use `codegraph_explore` and direct reads; do NOT trust the survey, glossaries, ADRs, or the older `.design/` reports), and record `VERIFIED` / `PARTIAL` / `REFUTED` / `UNVERIFIABLE` with `file:line` evidence and a corrected one-sentence statement when the survey was wrong or imprecise. Do not propose designs. Do not modify any production code, test, bench, or doc; do not run cargo/mise tasks.

Claims to check (add any adjacent discrepancy you notice, clearly marked as new):

1. **note → config edge.** `src/note/parser/input.rs` imports `config::{FrontmatterConfig, TaskConfig}`; list every other non-test `note → config` and `index → config` edge, and every production caller that constructs `MarkdownParserInput` (is `IndexerService` the only one?).
2. **Value trees.** Enumerate every production consumer of `FieldValue`, `FieldValueRef`, `DateTimeCoercion`, `FieldStringValue`, `NoteFieldValue`, `NoteFieldValueRef`, `NoteFieldType`, and `QueryFieldValueRef`. Confirm the frontmatter conversion chain `noyalib::Value → FieldValueRef → NoteFieldValue` and list every distinct conversion path from YAML, TOML, and JSON into these types (including `serde_json::Value`/`toml` paths and minijinja `Value` conversion). Where is `DateTimeCoercion::Classify` actually enabled? Which conversions are lossy (Int→f64, month precision, durations, links)?
3. **Temporal coupling and ambient zone on `main`.** Does `QuerySet` hold a `TimeZone`? List every non-test call site of `local_zone()` (and any other ambient clock/`now` read) with file:line; does `compare_temporal` exist; how often is `local_zone()` reached per comparison in sort/filter paths; where are `ZoneGuard`-style test overrides required. Confirm `field.rs` and `file.rs` import `temporal` and why.
4. **Schema ↔ note.** Does anything under `src/schema/` reference `note` types? Does any runtime validator of note values against a schema exist anywhere (CLI, template, index)? What does `SchemaDateField`'s `format` attribute do? Confirm the four type taxonomies (`RawSchemaFieldType`, `SchemaFieldTypeTag`, `SchemaFieldType`, `NoteFieldType`) and any mapping between them.
5. **Config.** For each `Config` section (`root`, `templates`, `schemas`, `frontmatter`, `tasks`), list its production consumers. Record the current state of `Config::hash()` and of per-section digests: `config-service/07` (per-section `ConfigDigests`, compose-all `hash()`) is **IN-FLIGHT user work** (WIP in `.worktrees/config-digest-sections`), followed by `index-query/17` (index-owned `IndexFingerprint`, drift detection); tag these claims `MOVING (07/17)`, report what has landed on `main` vs the worktree, and do NOT report their absence as a defect. Count how many files must change to add one new `tasks` setting (`raw.rs`, `model.rs`, `builder.rs`, `task.rs`, …) on `main`.
6. **Note parse pipeline.** Confirm `parse_markdown` runs pulldown-cmark twice (second pass `collect_headings`, with different options); count how many times list-item text is lexed; confirm spans are discarded (`ByteSpan` retained anywhere in `Note`?); confirm `Note` is serialized by serde derive straight into the redb `NOTES` table; check the GLOSSARY claims (heading slug dedup at parse time, footnote/callout nodes) against code.
7. **Ambient statics.** Enumerate `LazyLock`/`static`/`OnceLock` process-global state in `src/` (`dirs.rs`, `env_vars.rs`, `yaml.rs`, others) and `std::env::*` reads outside `cli`; for each, who consumes it and how tests override it.
8. **Root-primitive homes.** For `strsim`, `yaml`, `dirs`, `env_vars`, `delimiter`, `dirtree`, `hash`, `lexer`, `position`: list production consumers (modules + counts) to confirm which are single-consumer.

## Output

Write `.scratch/arch-redesign/research/01-verify-core-findings.md` (table-first, dense, <= ~3000 words). On finishing, append an `## Answer` section to THIS ticket file (a 5-8 line summary: how many claims verified/partial/refuted, and the three most consequential corrections) and set `Status: resolved`. Do NOT edit `map.md` (the orchestrator records the decision line).

## Answer

Verified all 8 claims against `main` (`0c43149d`) with active worktree comparison (`.worktrees/config-digest-sections` at `49c5ac39`): 6 claims are `VERIFIED`, 2 are `PARTIAL` (Claim 6: pipeline mechanics verified, but GLOSSARY claims refuted; Claim 8: `delimiter` and `lexer` have more than one consumer, see the ledger's Errata); Claim 5 is tagged `MOVING (07/17)`. Output written to `.scratch/arch-redesign/research/01-verify-core-findings.md`.
The three most consequential corrections to earlier survey assumptions are:

1. **GLOSSARY documentation fiction**: Heading slug dedup at parse time, footnote nodes, and callout nodes do not exist in code (`Heading` only stores level/text/line without slugs; footnote events only push text into the scan buffer without AST nodes; callouts have zero code).
2. **`DateTimeCoercion::Classify` is frontmatter-only**: It is enabled exclusively in `FieldValueRef::from(noyalib::Value)` (`src/field.rs:772`); all serde deserialization (`FieldValue::deserialize`) hardcodes `Passthrough`, and `deserialize_with(..., Classify)` is called only in unit tests.
3. **Complete note/schema decoupling and unused frontmatter config**: `src/schema/` has zero note references, runtime schema validation does not exist anywhere, `SchemaDateField.format` is dead code (`#[cfg(test)]` getter), and schema field taxonomies have zero mapping to `NoteFieldType`. In `FrontmatterConfig`, only `tags_name()` is read in production (`title`, `aliases`, `date_created`, `date_modified` are completely unread outside tests).
