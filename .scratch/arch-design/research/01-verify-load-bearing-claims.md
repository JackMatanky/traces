# Research: Adversarial Verification of Load-Bearing Scout Claims

> **Re-verification note (main @ `207cc3fb`, after this file was written at `2dc3250d`).** Claim 3, the ADR-0009 matrix row, the "Config Staleness Safety" invariant row and the "Dead Config Hashing Infrastructure" fact are **superseded**: config-drift detection landed (see `../reverification.md`). Claims 1, 2, 4, 6-7, 9-14 are untouched by the diff and stand. Claims 5 and 8 stand in substance with shifted lines: `reconcile` `src/index/refresh.rs:410`, `is_paths_unchanged` `:475`, `merge_refreshed_notes` `:555`, `read_all_notes` `:560`; `visit_values` `src/index/store.rs:1749`; the fileClass tests are `src/index/service.rs` `mod class_field` (`:1732`). Claim 14 ADR-0009 (index refresh) is now **TRUE in code with one deviation** (format-version mismatch wipes the store at open rather than producing a `Drifted` state; the ADR file still says `proposed`).

**Ticket:** `.scratch/arch-design/issues/01-research-verify-load-bearing-claims.md`  
**Deliverable:** `.scratch/arch-design/research/01-verify-load-bearing-claims.md`  
**Status:** Complete

---

## 1. Summary of Claim Verdicts

| # | Topic | Scout Claim | Verdict | Exact Source Citation |
|---|---|---|---|---|
| 1 | Value models | `FieldValueRef` coerces ISO strings at deserialization; 3-stage frontmatter conversion; 4 value enums exist | **Partially Confirmed** | `src/field.rs:781,838-854,976`; `src/note/metadata.rs:71-72`; `src/note/field.rs:43,179` |
| 2 | Temporal cmp | `NoteFieldValueRef::compare` promotes Date to local midnight, diverging from `NoteFieldValue` derived `PartialEq` | **Confirmed** | `src/note/field.rs:297-300`; `src/temporal/datetime.rs:340-345` |
| 3 | Config hash | `ConfigDigests`/`Config::hash` dead code; config drift pinned as stale in tests | **Confirmed at `2dc3250d`; SUPERSEDED at `207cc3fb`** (10 of 15 digests now feed `IndexFingerprint`; `Config::hash`/`compose` still dead; stale test replaced by `heals_class_axes_when_class_field_config_changes`) | `src/config/model.rs:156-158`; `src/index/service.rs:1749-1780` |
| 4 | Query paths | `run_from_store` partial index breaks totality; `Not` scans all paths; `run` is linear scan | **Confirmed** | `src/query/service.rs:162-166,323,395`; `src/index/entry.rs:15` |
| 5 | Index refresh | Path add/delete forces reading all notes from redb and re-resolving all links; content edits do not | **Confirmed** | `src/index/refresh.rs:343-344,361-386,487` |
| 6 | Document order | `QueryRow::cmp_document_order` ignores `item_idx` | **Confirmed** | `src/query/results.rs:427-432` |
| 7 | Template engine | Unconfined absolute paths; DryRun skips confinement; query stringification; row vec cloned on chain | **Partially Confirmed / Refuted** | `src/template/engine/path.rs:162`; `src/template/service.rs:225`; `src/template/engine/query.rs:398,604` |
| 8 | Schema isolation | `src/schema` never reads note values; index extracts `fileClass` with no schema involvement | **Confirmed** | `src/schema/`; `src/index/store.rs:1653-1662`; `src/index/service.rs:1646-1780` |
| 9 | Markdown parser | Dual pulldown-cmark pass; byte spans discarded; list metadata duplicated; only `first()` read | **Confirmed** | `src/note/parser.rs:108,121,490-497`; `src/note/model.rs:170-173`; `src/query/results.rs:328` |
| 10 | Doc drift | Slugs/footnotes/callouts drift; Occurrence/NavTarget drift; "Avoid relative path"; duplicate ADR 0009 | **Confirmed** | `src/note/GLOSSARY.md:45,84,98`; `src/GLOSSARY.md:20,72`; `docs/adr/0009-*.md` |
| 11 | Isolated deps | `speedate` and `winnow` have 1 call site each; `indextree` used only by 64-candidate trie | **Confirmed** | `src/temporal/date.rs:296`; `src/temporal/lexer.rs:7`; `src/index/trie.rs:12,33` |
| 12 | CLI domain leak | Scaffolding in CLI; filesystem-probing `normalize_source_input`; DSL synthesis; unbuffered `print!` EPIPE | **Confirmed** | `src/cli/init.rs:52-83`; `src/cli/mod.rs:436-468`; `src/cli/task.rs:42-64`; `src/cli/list.rs:64` |
| 13 | Module topology | 10-tier DAG acyclic; `note -> config` upward edge; `file_tracker` has 2 callers; edge table accurate | **Confirmed** | `src/note/parser/input.rs:9`; `src/config/tracker.rs:9`; `src/config/trust.rs:16` |
| 14 | ADR invariants | Audit 10 ADRs (0001–0009, two 0009s) for constraints and code truth | **Confirmed & Audited** | `docs/adr/0001` through `0009` |

---

## 2. Adversarial Re-Verification of Claims 1–14

### Claim 1: Value Coercion and Enum Duplication

- **[V] Verdict: Partially Confirmed (Corrected).**
- **Evidence:** Default `Deserialize for FieldValueRef` (`src/field.rs:971-977`) uses `DateTimeCoercion::Passthrough`, treating strings as text. Only explicit `deserialize_with(..., DateTimeCoercion::Classify)` (`src/field.rs:980-1002`) and `From<noyalib::Value>` (`src/field.rs:781`) invoke `FieldStringValue::classify` (`src/field.rs:838-854`, line 843 `DatePoint::parse`), coercing ISO strings to `FieldValueRef::Date` or `DateTime`.
- **[V]** Frontmatter 3-stage conversion is verified: YAML bytes parse via `noyalib::Value`, converted to `FieldValueRef::from(raw_value)`, then to `NoteFieldValue::from(field_value)` (`src/note/metadata.rs:71-72`).
- **[V]** Four distinct value enums exist: `FieldValue` (`src/field.rs:596`), `FieldValueRef` (`src/field.rs:717`), `NoteFieldValue` (`src/note/field.rs:43`), and `NoteFieldValueRef` (`src/note/field.rs:179`).
- **[V] Adjacent Fact:** `FieldValue` preserves `Int(i64)` vs `Float(f64)` (`src/field.rs:599-601`); `NoteFieldValue` collapses all numbers to `f64` (`src/note/field.rs:48`), losing integer precision at the note boundary.

### Claim 2: Temporal Comparison Asymmetry

- **[V] Verdict: Confirmed.**
- **Evidence:** `NoteFieldValueRef::compare` (`src/note/field.rs:297-300`) promotes `Date` to `DateTime` via `DateTimeValue::try_from(*a)`. `try_from` (`src/temporal/datetime.rs:340-345`) queries the host environment's `local_zone()?` to attach local midnight.
- **[V]** `NoteFieldValue`'s derived `PartialEq` (`src/note/field.rs:42`) treats `Date` and `DateTime` as distinct enum variants, never equal.
- **[V] Adjacent Fact:** If a `DateTime` equals local midnight, `a.as_ref().compare(&b.as_ref()) == Ordering::Equal`, yet `a == b` is false. Cross-type sorting is non-deterministic across machines in different time zones.

### Claim 3: Config Hash Dead Code & Unwitnessed Drift

- **[V] Verdict: Confirmed.**
- **Evidence:** `Config::hash` (`src/config/model.rs:156-158`) and `ConfigDigests` (`src/config/model/digest.rs:32`) have zero production call sites across the entire crate; they are only invoked in unit tests under `src/config/`.
- **[V]** In `src/index/service.rs:1749-1780`, test `fresh_refresh_serves_persisted_class_axes_unchanged_when_class_field_config_changes` explicitly asserts that altering `class_field` leaves persisted redb class axes unchanged and reports zero rows touched.
- **[V] Adjacent Fact:** ADR-0009 (`docs/adr/0009-index-refresh.md`) was proposed to store a config fingerprint in redb to detect drift, but was never implemented.

### Claim 4: Query Execution & Partial WorkspaceIndex

- **[V] Verdict: Confirmed.**
- **Evidence:** `QueryService::run_from_store` (`src/query/service.rs:162-166`) constructs `WorkspaceIndex::assemble` using only notes, files, and inlinks for candidate paths returned by `SourceResolver::resolve`. This breaks `WorkspaceIndex`'s documented contract (`src/index/entry.rs:15`): "Every regular file under the project root contributes one `FileEntry`".
- **[V]** `BooleanExpr::Not(_)` (`src/query/service.rs:395`) falls back to `self.store.paths_in_folder(Path::new(""))`, scanning the entire vault.
- **[V]** `matched_file_rows` (`src/query/service.rs:323-328`) iterates `0..index.entries().len()` with linear filtering.

### Claim 5: Index Refresh Path Invariant & Bulk Read

- **[V] Verdict: Confirmed.**
- **Evidence:** `RefreshPlan::reconcile` (`src/index/refresh.rs:343-344`) evaluates `Self::is_paths_unchanged` (`:403-411`). If any file is added or deleted (`!delta.deleted().is_empty()` or new path added), it executes `merge_refreshed_notes` (`:363, 482-518`), which calls `store.read_all_notes()?.into_vec()` at line 487 and re-resolves all inlinks across the vault (`:377`). Content-only updates take the `Modified` branch and skip full re-resolution.
- **[V] Adjacent Fact:** Adding a single empty `.md` file to a 50,000-note vault forces full redb deserialization of all 50,000 notes and complete backlink recomputation.

### Claim 6: Document Order Ignores Item Index

- **[V] Verdict: Confirmed.**
- **Evidence:** `QueryRow::cmp_document_order` (`src/query/results.rs:427-432`) executes `self.position.cmp(&other.position)`. `self.position` is a `RowIndex` into the file entries array. The doc comment (`:421-424`) claims it orders "by document order for note rows, and item order within their note for list and task rows", but `self.kind` (`RowType::List { item_idx }` / `RowType::Task { item_idx }`) is completely ignored.
- **[V] Adjacent Fact:** Multiple list items or tasks within the same document evaluate as `Ordering::Equal`, resulting in unstable or arbitrary sorting when sorted by document order in minijinja.

### Claim 7: Template Path Confinement & QuerySet Row-Vec Refutation

- **[V] Verdict: Partially Confirmed / Crucially Refuted on Row Vec Cloning.**
- **Evidence (Confirmed):** `PathInspection::resolve` (`src/template/engine/path.rs:161-163`) immediately returns `Ok(Self(candidate.to_owned()))` for absolute paths without verifying containment in `root`.
- **Evidence (Confirmed):** `TemplateService::write` (`src/template/service.rs:225-227`) short-circuits on `WriteMode::DryRun`, returning `Ok(WriteOutcome::Previewed)` before constructing `TemplateWriteTarget`, skipping target directory root-confinement checks.
- **Evidence (Confirmed):** `field_value` (`src/template/engine/query.rs:604-607`) converts `Duration`, `Date`, `DateTime`, and `Link` to plain `String` when crossing into minijinja. `Bool`, `Number`, `String`, `List`, `Object` retain native types.
- **Evidence (Refuted):** The scout claimed "every chained `QuerySet` method clones the row vec (`:398`)". In code, `src/template/engine/query.rs:398` executes `let rows = self.as_ref().clone();`. `query::QuerySet` (`src/query/results.rs:463-468`) stores `base: Arc<Vec<QueryRow>>` and an `ExecutionPlan`. Cloning `QuerySet` merely clones the atomic reference-counted pointer and the small `ExecutionPlan` (`:521-528`). Chained operations (`filter`, `where`, `sort`, `limit`, `group_by`, `flatten`) push transforms to `ExecutionPlan` without allocating or cloning rows. Rows are cloned only upon terminal materialization (`:484-490`).

### Claim 8: Schema Isolation & FileClass Extraction

- **[V] Verdict: Confirmed.**
- **Evidence:** `src/schema/` contains no imports or references to `Note` or `NoteFieldValue`. In `src/index/store.rs:1653-1662`, `visit_values` for `IndexDimension::FileClass` queries `note.frontmatter().into_iter().flat_map(|fm| fm.get_values(key))` using the unvalidated raw string `class_field` key from config. The test suite in `src/index/service.rs:1646-1780` confirms schema-free extraction.
- **[V] Adjacent Fact:** Schema definitions govern select values and field typing in templates, but Traces has zero schema validation for note content. Notes can violate schema definitions without diagnostics.

### Claim 9: Dual Markdown Pass & Discarded Spans & Multi-Value Blindness

- **[V] Verdict: Confirmed.**
- **Evidence:** `parse_markdown` (`src/note/parser.rs:106-115`) runs `Parser::new_ext(..., MARKDOWN_OPTIONS)` at line 108, followed by `collect_headings` running `Parser::new_ext(..., HEADING_OPTIONS)` at line 121. Byte spans from the pulldown-cmark offset iterator are translated to line numbers and discarded; `Note` stores zero byte spans (`src/note/model.rs:24-33`).
- **[V]** Flushed list-item metadata (`src/note/parser.rs:490-497`) is copied into `self.inline_fields` and `self.tags`, duplicating item-level metadata into note-level collections.
- **[V]** Multi-valued inline fields are stored as `Box<[NoteFieldValue]>` (`src/note/model.rs:31`), but `Note::get` (`src/note/model.rs:170-173`) and `QueryRow::resolve_metadata_ref` (`src/query/results.rs:328`) only ever read `.first()`. Elements `1..n` are completely inaccessible via query or note lookup APIs.

### Claim 10: Documentation Drift & ADR 0009 Duplicate

- **[V] Verdict: Confirmed.**
- **Evidence:** `src/note/GLOSSARY.md:45,84,98` documents `Slug`, `Footnote Reference/Definition`, and `Callout`. None of these exist in code (`Note` has no slug generation or callout extraction).
- **[V]** `src/GLOSSARY.md:72-106` defines `Occurrence`, `NavTarget`, `Resolution`, and `Ambiguity` from an unbuilt navigation spine; none exist in `src/`.
- **[V]** `src/GLOSSARY.md:20` lists `*Avoid*: relative path` under "Root-Confined Path", while `src/path.rs:64` names the core type `pub(crate) struct RelativePath(PathBuf)`.
- **[V]** Two ADRs share number 0009: `docs/adr/0009-index-refresh.md` and `docs/adr/0009-filter-date-literals.md`.

### Claim 11: Single-Site Dependencies & Trie Threshold

- **[V] Verdict: Confirmed.**
- **Evidence:** `speedate` is called exclusively at `src/temporal/date.rs:296` (`speedate::Date::parse_bytes_rfc3339`). `winnow` is imported exclusively at `src/temporal/lexer.rs:7` for `duration_part` (`:139-148`). `indextree` is imported exclusively at `src/index/trie.rs:12` and used only by `CandidateTrie` when `candidates.len() >= TRIE_THRESHOLD` (`TRIE_THRESHOLD = 64` at line 33).
- **[V] Adjacent Fact:** Jiff's `jiff::civil::Date` and existing `logos` lexers make `speedate` and `winnow` redundant.

### Claim 12: CLI Domain Logic Leakage & EPIPE Panic

- **[V] Verdict: Confirmed.**
- **Evidence:** `src/cli/init.rs:52-83` owns directory scaffolding, config writing, and rollback. `src/cli/mod.rs:436-468` (`normalize_source_input`) performs filesystem probes (`root.join(path).is_file()`) to auto-append `.md` or quote strings. `src/cli/task.rs:42-64` synthesizes query DSL filter strings (`"list.completed == false"`).
- **[V]** Unbuffered `print!` without SIGPIPE/EPIPE suppression is used in `src/cli/list.rs:64`, `src/cli/table.rs:63`, `src/cli/task.rs:194`, `src/cli/template.rs:223`, and `src/cli/completions.rs:65`. Piping output to `head -n 1` triggers a broken pipe panic.

### Claim 13: 10-Tier DAG & Tracker Isolation

- **[V] Verdict: Confirmed.**
- **Evidence:** Module tiers 0 through 9 form an acyclic 10-tier DAG matching the table in `survey/cross-cutting.md:94-121`.
- **[V]** `src/note/parser/input.rs:9` imports `FrontmatterConfig` and `TaskConfig` from `crate::config`, forming an upward dependency edge from domain model (tier 5) to application config (tier 4).
- **[V]** `file_tracker` is imported in `src/lib.rs:147` and called exclusively by `src/config/tracker.rs:9` and `src/config/trust.rs:16`.

### Claim 14: ADR 0001–0009 Status & Invariant Matrix

- **[V] Verdict: Audited.**
  - **ADR-0001 (Minijinja Interactive Functions):** Constrains template engine & dialog seam. **Invariant TRUE in code** (Object global dispatch, DialogProvider split, dry-run independence).
  - **ADR-0002 (Symlink Config Trust & Tracking):** Constrains config security and storage. **Invariant TRUE in code** (BLAKE3 hash symlinks in XDG state dirs with companion `.hash`).
  - **ADR-0003 (Index-Based Dialog Selection):** Constrains dialog provider methods. **Invariant TRUE in code** (`select`/`multi_select` take `&[String]` and return `usize`/`Vec<usize>`).
  - **ADR-0004 (CLI Error Presentation & Outcomes):** Constrains CLI error handling. **Invariant TRUE in code** (`CliError` centralization, `CommandOutcome::UserAbort` exit 0/130).
  - **ADR-0005 (redb Index & QueryOps):** Constrains index storage and query API. **Invariant TRUE in code** (8 redb tables, `.traces/index.redb`, `QueryOps.from`).
  - **ADR-0006 (File Class Schemas):** Constrains schema model. **Invariant TRUE in code** (`.traces/schemas/*.toml`, `extends` DAG, bounded `$ref`, `deny_unknown_fields`).
  - **ADR-0007 (Field Inheritance & extends-as-is-a):** Constrains schema inheritance & query matching. **Invariant TRUE in code** (Kahn topological sort, multi-parent, `global.toml`).
  - **ADR-0008 (Source Expression Grammar):** Constrains query DSL. **Invariant TRUE in code** (`from()` accepting `#tag`, `folder/`, `@Class`, `"exact.md"`, boolean ops).
  - **ADR-0009 (Refresh-driven Index Rebuilds / Drift Detection):** Constrains index freshness. **Invariant FALSE at `2dc3250d`; TRUE at `207cc3fb`** (`meta` table with fingerprint, drift heals on refresh; deviation: format version mismatch wipes at open, ADR file still `proposed`).
  - **ADR-0009 (Filter date literals):** Constrains query parser. **Invariant TRUE in code** (unquoted ISO dates lexed as date literals and number-only arithmetic rejected in `src/query/grammar/filter.rs:84-107,301-325`).

---

## 3. Invariants and Invariant Ownership

| Invariant | Owner | Status in Code | Architectural Risk |
|---|---|---|---|
| **WorkspaceIndex Totality** | `index::entry` | Broken by `query::service` [V] | `run_from_store` passes partial slices to `WorkspaceIndex::assemble`, breaking backlink queries |
| **Numeric Model Precision** | `field` vs `note` | Inconsistent [V] | `field::FieldValue` preserves `i64`; `note::NoteFieldValue` casts to `f64`, losing integer fidelity |
| **Temporal Cross-Kind Equality** | `note::field` | Inconsistent [V] | `compare` equates Date and DateTime at local midnight; derived `PartialEq` never equates them |
| **Root Confinement** | `path` | Bypassed in Template [V] | `PathInspection::resolve` accepts absolute paths; `WriteMode::DryRun` skips target checks |
| **Document Ordering** | `query::results` | Violated for Items [V] | `QueryRow::cmp_document_order` ignores `item_idx`, making list/task sorting unstable |
| **Config Staleness Safety** | `index::fingerprint` + `index::refresh` | Missing at `2dc3250d`; **Present at `207cc3fb`** [V] | Fail-closed completeness is compile-time (exhaustive destructure); remaining gap: parser version is not fingerprinted (hand-bumped `META_VERSION`) |

---

## 4. Tradeoffs and Architectural Costs Paid

- **[H] Dual Parser Overhead:** Running `pulldown-cmark` twice per note (`src/note/parser.rs:108,121`) trades 2x Markdown parsing CPU time for decoupled heading text extraction.
- **[H] Coercion Cascades:** Storing four distinct value enums (`FieldValue`, `FieldValueRef`, `NoteFieldValue`, `NoteFieldValueRef`) incurs 3-stage frontmatter conversions and type loss (`i64` -> `f64`).
- **[H] All-or-Nothing Inlink Refresh:** Re-resolving all vault inlinks on any path addition or deletion (`src/index/refresh.rs:361-386`) trades catastrophic $O(N)$ re-reads on single file additions for simpler delta logic.
- **[H] Memory Overhead of Duplicated Metadata:** Flushed list-item inline fields and tags are duplicated into note-level arrays (`src/note/parser.rs:490-497`), doubling metadata storage per list item.

---

## 5. Transferable Lessons for Traces

1. **[H] Upstream Fingerprint Completeness (Cargo / Ruff):** Storing an explicit configuration fingerprint in the persistent index store (as proposed in ADR-0009) is mandatory to prevent silent data corruption when settings like `class_field` drift.
2. **[H] Inverted Basename Indexing (advanced-metadata-cache / rumdl):** Path additions and deletions should only re-resolve files that reference the modified basename, replacing Traces' full-vault `read_all_notes` sweep.
3. **[H] Single Canonical Domain Value Type (Polars / DuckDB):** Unifying `FieldValue` and `NoteFieldValue` into a single domain value enum eliminates conversion boilerplate and numeric distortion.

---

## 6. Lessons That Do NOT Transfer and Why

- **[H] Multi-Crate Workspace for 139 Files:** Splitting Traces' acyclic 10-tier DAG into 6+ workspace crates adds cargo dependency graph serialization overhead and packaging friction without resolving the core domain boundary issues. The monolith's compile time is fast; internal module boundaries provide sufficient encapsulation.
- **[H] Dynamic Schema Validation Engine:** Heavy jsonschema-style validation is unnecessary because notes are authored in human Markdown; strict validation would break notes on harmless typos.

---

## 7. What This Challenges in Traces Redesign Hypotheses

- **[H] Challenges Hypothesis H1 (Workspace Split):** The module DAG is already acyclic with clean boundaries; splitting into crates does not fix the upward edge (`note -> config`), which requires domain restructuring (extracting note parser options).
- **[H] Strengthens Hypothesis H2 (Value Model Consolidation):** The verification of four parallel enums, 3-stage conversions, and numeric precision loss confirms that consolidating into a single domain value type is high-leverage.
- **[H] Refutes Scout Claim on QuerySet Cloning:** QuerySet chaining does NOT clone the row vec; redesign efforts should not waste complexity optimizing `QuerySet` chaining laziness.
- **[H] Elevates Issue 04 / Index Refresh Priority:** The confirmation that a single path addition forces reading all 50k notes from redb elevates index refresh redesign to a top-tier architectural priority.

---

## 8. New Architectural Facts Discovered

- **[V] Dead Config Hashing Infrastructure (at `2dc3250d`; narrowed at `207cc3fb`):** per-setting digests (now `src/config/model/digest.rs`, 380 lines) have one production consumer, `IndexFingerprint::new`; `Config::hash`/`compose`, the root/templates/schema-directory digests and the `syn` scan remain unconsumed.
- **[V] Multi-Valued Inline Fields are Write-Only:** Notes parse and store multiple values for an inline field key (`Box<[NoteFieldValue]>`), but the lookup API (`Note::get`) hardcodes `.first()`.
- **[V] CLI Pre-Query Filesystem Probing:** `src/cli/mod.rs:436-468` intercepts user input and runs `is_file()` checks against disk to rewrite source strings before passing them to `QueryService`.
- **[V] Unhandled SIGPIPE Across Output Commands:** `traces list`, `traces table`, `traces task`, and `traces completions` all invoke unbuffered `print!` directly, panicking when piped to standard Unix tools like `head`.
