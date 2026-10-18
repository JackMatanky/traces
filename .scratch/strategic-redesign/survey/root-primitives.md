# Architecture Survey: Root Primitives & Crate Infrastructure

## 1. Responsibilities & seams

- **Scope & Scale**: 19 root files (11,642 LoC: 6,888 prod / 4,754 test across slice). Spans crate facade (`lib.rs`, `main.rs`), domain models hoisted to root (`task.rs`, `field.rs`, `tag.rs`), spatial/text primitives (`position.rs`, `lexer.rs`, `delimiter.rs`, `strsim.rs`, `yaml.rs`), filesystem/identity leaf types (`file.rs`, `path.rs`, `dirtree.rs`, `hash.rs`), OS configuration leaf stores (`dirs.rs`, `env_vars.rs`, `file_tracker.rs`), and an inter-subsystem adapter (`file_class_expander.rs`).
- **Public Surface (`src/lib.rs:137-175`)**:
  - `pub` (library API): `FileMeta` (`file.rs:45`); `PositionError`, `SourceLine` (`position.rs:270, 472`); `Tag`, `TagError` (`tag.rs:28, 242`); `TaskDate`, `TaskDateSet`, `TaskDateSetIter`, `TaskDateType`, `TaskError`, `TaskPriority`, `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType` (`task.rs:51-1049`); `DialogProvider`, `Note` items, `temporal` values.
  - `pub(crate)` (internal engine): `DelimiterType` (`delimiter.rs`); `DirNode`, `DirTree`, `DirTreeError` (`dirtree.rs:37, 83`); `FieldKey`, `FieldKeyRef`, `FieldName`, `FieldNameError`, `FieldNameRef`, `FieldValue` (`field.rs:46-596`); `BaseName`, `BaseNameRef`, `FileName` (`file.rs:281-344`); `CleanMode`, `FilePathTracker`, `FilePathTrackerError` (`file_tracker.rs:32, 167, 378`); `Blake3Digest`, `Blake3Hasher`, `Blake3PathHash` (`hash.rs:24, 69, 133`); `LexError`, `SpannedTokenStream`, `TokenSpec`, `lexical_unquote` (`lexer.rs:19-172`); `BytePos`, `ByteSpan`, `LineIndex`, `Spanned` (`position.rs:33, 79, 190, 359`).
  - `pub mod testing` (`src/lib.rs:177-210`): Exactly 45 frozen exports under `#[cfg(any(test, feature = "test-utils"))]`.
  - `mod test_support` (`src/lib.rs:230-892`): 662 lines (74% of `lib.rs`) of test fixtures (`TestProject`, `fixture_service`).
- **Callers & Call Protocols**:
  - `main.rs:10-35`: Minimal process shell mapping `traces::cli::run()` to POSIX exit codes (0 for `Completed`/`Cancelled`, 130 for `Interrupted`, 1 for `Err`).
  - `config`: Consumes `TaskConfig` / `TaskStatusMap` (`config/model/tasks.rs:16`), `Tag` (`config/model/tasks.rs:17`), `FieldName` (`field.rs:46`), `FilePathTracker` (`file_tracker.rs:32`), `Blake3Hasher` (`hash.rs:69`), `dirs` (`dirs.rs:24`), `env_vars` (`env_vars.rs:8, 20`).
  - `note`: Consumes `FieldKey`, `FieldValueRef` (`note/metadata.rs:71`), `TaskListItem` (`note/lists.rs:347`), `TaskDateSet`, `TaskPriority` (`note/parser/task.rs:12`), `SourceLine`, `LineIndex`, `ByteSpan` (`note/parser.rs:111`), `Tag` (`note/lists.rs:27`), `DelimiterType` (`delimiter.rs`), `yaml::parse` (`yaml.rs:28`).
  - `schema`: Consumes `FieldName`, `FieldValue`, `FieldValueRef` (`schema/raw.rs:16`, `schema/fields/parser.rs:17`), `SafeRelativePath` (`schema/fields/select.rs:22`), `closest_match` (`schema/model.rs`).
  - `query`: Consumes `FieldKey`, `FieldKeyRef` (`query/grammar/field.rs`), `TaskStatusSymbol` (`query/results.rs:25`), `FileMeta` (`query/results.rs:26`), `SpannedTokenStream` (`query/grammar/`), `FileClassExpander` (`query/grammar/source.rs:353`).
  - `index`: Consumes `FileMeta` (`file.rs:45`), `HasPath`, `codec` (`path.rs:55, 340`), `DirTree` (`dirtree.rs:83`), `SourceLine` (`position.rs:270`).
- **What Callers Must Know**:
  - Root primitives are divided between pure stateless leaf utilities (`dirtree`, `lexer`, `hash`, `strsim`, `delimiter`), platform state storage (`dirs`, `file_tracker`), and domain types hoisted to break dependency cycles (`task`, `field`, `tag`, `file`, `file_class_expander`).
  - `SafeRelativePath` triggers filesystem I/O (`path.rs:181`), whereas `RelativePath` is purely lexical (`path.rs:113`).
  - `FieldValueRef` coerces ISO date strings during deserialization (`field.rs:843`), altering scalar types before callers inspect them.

## 2. Invariants

- **`FieldName` is non-empty, whitespace-trimmed, and slash-free**: Enforced at construction in `FieldName::validate` (`src/field.rs:81-91`).
- **`FieldKey` canonical form is non-empty, lowercased, and punctuation-stripped**: Enforced at construction in `FieldKey::try_new` (`src/field.rs:292-306`). Non-alphanumeric ASCII stripped, whitespace becomes `-` (`src/field.rs:359-383`).
- **`Tag` starts with `#`, second char alphabetic, body alphanumeric or `_`/`/`/`-`**: Enforced at construction in `Tag::parse` (`src/tag.rs:56-76`). Lenient parsing trims whitespace and prepends `#` if omitted (`src/tag.rs:121-131`).
- **`ByteSpan` invariant `start <= end`**: Enforced at construction via `end.max(start)` (`src/position.rs:91`). Private fields prevent mutation.
- **`SourceLine` is non-zero (1-indexed)**: Enforced by `NonZeroU32` newtype wrapper (`src/position.rs:270, 290-295`).
- **`BytePos` clamped to 4 GiB (`u32::MAX`)**: Enforced at type level (`src/position.rs:190, 195`). Saturating conversion narrows `usize` (`src/position.rs:211-213`).
- **`RelativePath` contains no `..`, root, or prefix components, and has >=1 normal component**: Enforced in `RelativePath::try_from` (`src/path.rs:113-130`).
- **`SafeRelativePath` resolves inside root boundary**: Enforced at construction via `strict_path::PathBoundary::strict_join` (`src/path.rs:181-189`). Split invariant / TOCTOU window: returned path is plain join `root.join(safe)` (`src/path.rs:190`), so intervening symlink swaps can escape (`src/path.rs:159-161`).
- **`DelimiterStack` bounded to depth 16**: Enforced via fixed array `[DelimiterType; 16]` (`src/delimiter.rs:8, 13`); overflow saturates silently (`src/delimiter.rs:37`).
- **Injective length-prefixed BLAKE3 framing**: Enforced by `Blake3Hasher::len` using little-endian `u64` prefixes (`src/hash.rs:95-97`) and 1-byte option presence tags (`src/hash.rs:105-115`).
- **`Blake3PathHash` is exactly 64 ASCII hex bytes**: Enforced by fixed `[u8; 64]` array (`src/hash.rs:133`).
- **`TaskDateSet` slot exclusivity (first-wins)**: Enforced in `TaskDateSet::insert` (`src/task.rs:709-725`); subsequent dates for an occupied slot return `false` and are dropped.
- **`TaskStatusMap` consistency across symbol, name, and kind**: Enforced in `TaskStatusMap::insert` (`src/task.rs:163-195`) by evicting stale index entries before inserting overrides.

## 3. Verified observations

- [V] `src/lib.rs` is 892 lines, of which 662 lines (74%, `src/lib.rs:230-892`) are `mod test_support` fixture builders and fixture unit tests.
- [V] Visibility doctests in `src/lib.rs:69-107` only probe config types (`ConfigError`, `ConfigBuilder`, `SchemasConfig`, `FrontmatterConfig`), with zero compile-fail probes for root primitives (`FieldKey`, `RelativePath`, `DirTree`, etc.).
- [V] `src/task.rs` is 1,919 lines long and bundles configuration data structures (`TaskStatusMap`, `src/task.rs:119`), note-markdown emoji extractors (`TaskDateType::EMOJIS`, `src/task.rs:767`; `TaskPriority::EMOJIS`, `src/task.rs:962`), and task domain models.
- [V] `src/field.rs` has an upward/lateral coupling to `src/temporal/`, importing `DatePoint` (`src/field.rs:39`) and executing `DatePoint::parse` during string deserialization (`src/field.rs:843`).
- [V] Two parallel dynamic value ASTs exist: `field::FieldValue` (`src/field.rs:596`: Null, Bool, Int(i64), Float(f64), String, Date, DateTime, List(Vec), Object(IndexMap)) and `note::NoteFieldValue` (`src/note/field.rs:43`: Null, Bool, Number(f64), String, Date, DateTime, Duration, Link, List(Box), Object(IndexMap)).
- [V] Parsing note frontmatter executes two sequential AST conversions with intermediate heap allocations: `noyalib::Value` -> `FieldValueRef<'static>` (`src/note/metadata.rs:71`) -> `NoteFieldValue` (`src/note/metadata.rs:72`).
- [V] `src/file_tracker.rs` is 1,178 lines long, but has exactly two callers in the entire codebase: `src/config/tracker.rs:9` and `src/config/trust.rs:16`.
- [V] `src/file.rs:88-98` embeds Dataview-specific business logic (`derive_day`: filename ISO date prefix overrides `created_at`) directly into filesystem metadata construction.
- [V] `src/path.rs::codec` (`src/path.rs:340-437`) serializes `PathBuf` as raw OS bytes (`&[u8]`) on Unix (`src/path.rs:356`) and wide units (`&[u16]`) on Windows (`src/path.rs:390`), creating platform-incompatible `redb` database entries.
- [V] Note entities (`Link`, `Tag`, `ListItem`) discard `ByteSpan` during parsing (`src/note/parser.rs:111`), retaining only `SourceLine` (`src/note/lists.rs:47`).
- [V] `src/GLOSSARY.md:72-105` documents "Occurrence", "NavTarget", "Resolution", and "Ambiguity" as core domain primitives, but zero lines of code exist for them in `src/`.
- [V] `src/GLOSSARY.md:18-20` mandates "Avoid: relative path", directly contradicting `RelativePath` in `src/path.rs:64`.

## 4. Hypotheses

1. **[H] Consolidate Dual Dynamic Value Hierarchies**: `field::FieldValue` and `note::NoteFieldValue` duplicate generic dynamic value semantics. Unifying them into a single core `Value` type (or having `NoteFieldValue` deserialize directly from YAML/JSON via `noyalib`/`serde`) eliminates intermediate `FieldValueRef` allocations (`src/note/metadata.rs:71-72`) and collapses 400+ lines of duplicate conversion visitors.
   - *Alternative*: Keep as-is. `FieldValue` represents format-agnostic schema/TOML options; `NoteFieldValue` adds PKM constructs (`Link`, `Duration`).
   - *Removable*: `From<FieldValueRef> for NoteFieldValue` (`src/note/field.rs:532-571`), `FieldValueRefSeed` (`src/field.rs:857-890`).
   - *Disprove*: Measure frontmatter parse throughput and memory footprint under unified vs two-tier parser; if a unified enum exceeds `NoteFieldValue`'s 80-byte budget (`src/note/field.rs:580`) or degrades cache efficiency, hypothesis refuted.
   - *Blast Radius*: High (`field.rs`, `note/field.rs`, `note/metadata.rs`, `schema/`, `query/`).
2. **[H] Decompose `task.rs` into Config Schema vs Note Parser Domains**: `src/task.rs` (1,919 LoC) hoists note-specific emoji lexing (`TaskDateType::EMOJIS`, `src/task.rs:767`; `TaskPriority::EMOJIS`, `src/task.rs:962`) and date slot storage (`TaskDateSet`, `src/task.rs:689`) to the root solely because `config` needs `TaskStatusMap` (`src/config/model/tasks.rs:16`). Moving `TaskStatusMap` to `config::tasks` or a leaf `task_status` primitive allows `TaskPriority`, `TaskDateSet`, and emoji parsing to relocate to `note::task`.
   - *Alternative*: Keep as-is. Colocating all task domain types in one file avoids multiple smaller task modules.
   - *Removable*: ~1,200 LoC of note-specific parsing logic from crate root.
   - *Disprove*: Check if `query` or `cli` require `TaskPriority` and `TaskDateSet` without `note`. If they do, moving them into `note` would create an upward dependency from `query` to `note`.
   - *Blast Radius*: Medium (`task.rs`, `config/model/tasks.rs`, `note/lists.rs`, `note/parser/task.rs`, `query/results.rs`).
3. **[H] Consolidate Crate-Root "Junk Drawer" into Owning Subsystems**: 17 loose files live at `src/` (11,642 LoC). Modules with 1-2 callers should be re-homed: `file_tracker.rs` (1,178 LoC) and `dirs.rs` (219 LoC) into `config/`, `delimiter.rs` (498 LoC) into `note/parser/`, `file_class_expander.rs` (87 LoC) into `schema/`, `env_vars.rs` (67 LoC) into `config/`.
   - *Alternative*: Keep as-is. Flat crate root prevents deep directory nesting.
   - *Removable*: ~2,000 LoC of internal implementation from crate root; clarifies that `file_tracker` is not a generic engine primitive.
   - *Disprove*: Trace cross-subsystem imports (e.g. if `index` requires `is_ignored_dir` from `env_vars.rs:42`, re-homing `env_vars` to `config` forces `index` to depend on `config`).
   - *Blast Radius*: Medium (module declarations in `lib.rs`, import paths across `config`, `index`, `note`).
4. **[H] Extract Test Support from `lib.rs` into Dedicated Fixture Module**: 662 of 892 lines in `src/lib.rs` (74%) are test fixtures (`TestProject`). Moving them to `src/test_support.rs` isolates the crate facade and clarifies public re-exports.
   - *Alternative*: Keep as-is. Keeps test-utils gating localized in `lib.rs`.
   - *Removable*: 662 lines from `src/lib.rs`.
   - *Disprove*: None; `mod test_support` is already an internal submodule (`src/lib.rs:231`).
   - *Blast Radius*: Low (extracting file, updating `pub mod testing` re-exports).
5. **[H] Remove Temporal Coercion from Root `field.rs`**: `src/field.rs:843` coerces strings to `DateValue`/`DateTimeValue` via `DatePoint::parse` during deserialization. Moving temporal classification to callers (`note` frontmatter and `schema` validation) decouples root field primitives from the `temporal` subsystem.
   - *Alternative*: Keep as-is. Automatic coercion ensures YAML/JSON dates are uniformly typed without requiring caller post-processing.
   - *Removable*: `crate::temporal::DatePoint` import in `src/field.rs:39`, `DateTimeCoercion` (`src/field.rs:803`), `FieldStringValue` (`src/field.rs:819`).
   - *Disprove*: Check if schema option validation relies on dates being pre-classified during raw TOML deserialization.
   - *Blast Radius*: Medium (`field.rs`, `note/metadata.rs`, `schema/fields/`).
6. **[H] Elevate `ByteSpan` to First-Class AST Coordinate for LSP Readiness**: Note parsing currently drops `ByteSpan` (`src/note/parser.rs:111`), retaining only `SourceLine` (`src/position.rs:270`), while query syntax uses `miette::SourceSpan` (`src/query/error.rs:12`). Adopting `ByteSpan` (`src/position.rs:79`) across `Note` and `Query` ASTs provides byte-accurate span coordinates without reparsing documents.
   - *Alternative*: Keep as-is. Discarding byte spans minimizes in-memory AST footprint in `redb` (`src/index/store.rs`).
   - *Removable*: Re-parsing text buffers in CLI/LSP diagnostics; bridge conversions in `SpannedTokenStream` (`src/lexer.rs:36`).
   - *Disprove*: Measure `redb` database size expansion when `ByteSpan` (8 bytes per item) is stored in note records. If database size expands >20% and cache hit rate drops, hypothesis challenged.
   - *Blast Radius*: High (`position.rs`, `note/`, `query/`, `index/store.rs`).
7. **[H] Encapsulate Path Confinement into a Capability `ProjectRoot`**: `path.rs` separates `RelativePath` (lexical, `src/path.rs:64`) and `SafeRelativePath` (boundary check, `src/path.rs:146`), but `SafeRelativePath` returns a plain join (`src/path.rs:190`) with an acknowledged TOCTOU window (`src/path.rs:159-161`). Replacing raw path joins with a capability-oriented `ProjectRoot` eliminates ad-hoc path confinement checks (`src/lib.rs:709, 734`).
   - *Alternative*: Keep as-is. `strict_path` boundary checking is sufficient for single-user synchronous CLI runs.
   - *Removable*: Scattered `SafeRelativePath::parse` calls across `template/writer.rs`, `schema/`, and `config/`.
   - *Disprove*: Evaluate Windows filesystem capability support (directory handles / `openat` emulation). If Windows compatibility requires fallback to joined paths, capability guarantees degrade.
   - *Blast Radius*: Medium (`path.rs`, `template/writer.rs`, `index/service.rs`).

## 5. Cross-slice couplings

- **`field.rs` -> `temporal`**: `src/field.rs:39` imports `DatePoint`, `DateValue`, `DateTimeValue`. Deserializing any `FieldValueRef` runs `DatePoint::parse` (`src/field.rs:843`), coupling root field primitives to the temporal calendar parser.
- **`file.rs` -> `temporal`**: `src/file.rs:36` imports `DatePoint`. `FileMeta::derive_day` (`src/file.rs:88-98`) runs `DatePoint::parse_prefix` on filenames to implement Dataview calendar rules inside filesystem metadata scanning.
- **`note` -> `field.rs` -> `note`**: `note/metadata.rs:71-72` converts `noyalib::Value` -> `field::FieldValueRef` -> `note::NoteFieldValue`, traversing two AST hierarchies to parse frontmatter.
- **`config` -> `task.rs`**: `src/config/model/tasks.rs:5` imports `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`. Root `task.rs` was created to allow `config` to load tasks without depending on `note`.
- **`config` -> `tag.rs`**: `src/config/model/tasks.rs:17` stores `tag_filters: Vec<Tag>`, forcing `tag.rs` to live at crate root.
- **`file_class_expander.rs` (Root Glue)**: `src/file_class_expander.rs:15` implements `query::FileClassExpander` for `schema::SchemaService` at the crate root to keep `query` and `schema` decoupled.
- **`file_tracker.rs` -> `dirs.rs` & `hash.rs`**: `src/file_tracker.rs:20` consumes `dirs::StateDirRoot` and `hash::Blake3PathHash` exclusively on behalf of `config/tracker.rs` and `config/trust.rs`.

## 6. Comparables worth studying

- **`serde_json::Value` vs `jiff`** (Informs Hypotheses 1 & 5): How generic tree deserialization avoids embedding temporal parsing, keeping string classification as an explicit boundary step.
- **`rustc_span` (`compiler/rustc_span/src/lib.rs`)** (Informs Hypothesis 6): `BytePos` (`u32`), `Span`, and `SourceFile` precomputed line-start binary search (`LineIndex`); shows how to maintain compact 8-byte spans throughout parsing, AST, and error diagnostics without heap churn.
- **`ruff_source_file` (`crates/ruff_source_file`)** (Informs Hypotheses 3 & 6): Minimalist, leaf-tier source positioning and line index primitives decoupled from Python AST and linter config.
- **`gitoxide` (`gix-path`, `gix-config`)** (Informs Hypotheses 2, 3 & 7): How Gitoxide decomposes path validation, config tracking, and shared primitives into cohesive leaf modules without creating flat root junk drawers.
- **`cap-std` (`cap-std/src/fs/dir.rs`)** (Informs Hypothesis 7): Operating-system capability handles for directory-confined operations, eliminating TOCTOU symlink traversal races.

## 7. Open decisions

- **Decision: Unified `Value` vs Specialized Domain Values**: Should `field::FieldValue` and `note::NoteFieldValue` merge into a single crate-wide `Value` enum, or should `NoteFieldValue` deserialize directly from YAML/JSON and leave `FieldValue` solely for schema options? (Candidate ticket: `unify-metadata-value-hierarchy`).
- **Decision: Re-home `file_tracker`, `dirs`, and `task` configuration**: Should `file_tracker.rs` and `dirs.rs` be absorbed into `src/config/`, and `src/task.rs` split into `config::tasks::status` and `note::task`? (Candidate ticket: `rehome-root-config-primitives`).
- **Decision: AST ByteSpan Adoption Policy**: Should `ByteSpan` be stored directly on `Note` items (`Link`, `Tag`, `ListItem`) for LSP readiness, or should `Note` remain line-only (`SourceLine`) to minimize `redb` storage footprint? (Candidate ticket: `adopt-bytespan-in-note-ast`).
- **Decision: Cross-Platform Path Codec in Redb**: Should `src/path.rs::codec` continue persisting platform-native OS bytes/wide chars in `redb` (making database files non-portable across OSes), or migrate to UTF-8 normalized `/`-separated paths? (Candidate ticket: `portable-index-path-codec`).
- **Preserved Fog**: How LSP document overlays and incremental reparsing will interact with `LineIndex` and `ByteSpan` when buffers change faster than redb index flushes.

## 8. Withstands scrutiny

- **`FieldKey` Canonicalization & O(1) Zero-Allocation Lookup (`src/field.rs:359, 582`)**: `FieldKeyRef` implementing `Equivalent<FieldKey>` using `FieldKey::is_match` enables zero-copy lookups in `IndexMap<FieldKey, V>`. Preserving the author's original casing for display while hashing normalized canonical text is robust and architecturally sound.
- **`FolderRef::distance_to` Algorithm (`src/path.rs:242-279`)**: Calculating folder tree distance by iterating components in lockstep to find LCA and counting remaining hops is $O(\text{depth})$ with zero heap allocations. Sound and optimal for link ambiguity resolution.
- **`LineIndex` Line Conversion (`src/position.rs:36-61`)**: Precomputing `\n` byte offsets into `Box<[BytePos]>` and performing $O(\log L)$ binary search via `slice::partition_point` is optimal, memory-compact, and panic-safe.
- **`Blake3Hasher` Injective Length-Prefixed Framing (`src/hash.rs:95-115`)**: Prepending variable-length items with little-endian `u64` lengths and optional items with 1-byte presence tags mathematically prevents delimiter collision attacks across cached digests.
- **`DirTree` Error Classification (`src/dirtree.rs:38-57`)**: Clean separation between `MissingRoot`, `RootInaccessible`, and `NodeInaccessible` enables callers to implement distinct policies (degrade vs fail) without inspecting raw OS errno codes.
- **`Tag` Memory & Parsing Invariants (`src/tag.rs:28, 56, 184`)**: Compact `Tag(Box<str>)` storing `#`-prefixed strings, computing `segments()` and `is_contained_in()` on demand without secondary heap structures, is robust and performant.
