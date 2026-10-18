# Architecture Survey: Crate-Root Primitives & Cross-Cutting Leaf Modules

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size

- **Modules Covered**:
  - `src/field.rs` (1,832 lines: 1,026 prod / 806 test)
  - `src/file.rs` (667 lines: 373 prod / 294 test)
  - `src/path.rs` (788 lines: 439 prod / 349 test)
  - `src/dirtree.rs` (824 lines: 364 prod / 460 test)
  - `src/hash.rs` (372 lines: 178 prod / 194 test)
  - `src/delimiter.rs` (498 lines: 344 prod / 154 test)
  - `src/strsim.rs` (81 lines: 23 prod / 58 test)
  - `src/yaml.rs` (30 lines: 30 prod / 0 test)
  - `src/lexer.rs` (657 lines: 310 prod / 347 test)
  - `src/position.rs` (799 lines: 483 prod / 316 test)
  - `src/dirs.rs` (219 lines: 200 prod / 19 test)
  - `src/env_vars.rs` (67 lines: 46 prod / 21 test)
  - Total: 6,834 lines (3,816 prod / 3,018 test, ~44.1% test)
- **Crate-Root Re-export Surface (`src/lib.rs:135-210`)**:
  - **`pub` tier** (external API of the `traces` crate):
    - `dialog`: `DialogError`, `DialogProvider`, `DialogResult`, `PresetDialogProvider`, `TerminalDialogProvider`
    - `file`: `FileMeta`
    - `note`: `Heading`, `ListItem`, `ListItemType`, `ListText`, `MarkdownParserInput`, `Note`, `NoteFieldValue`, `NoteFieldValueRef`, `TaskListItem`, `parse_markdown`
    - `position`: `PositionError`, `SourceLine`
    - `tag`: `Tag`, `TagError`
    - `task`: `TaskDate`, `TaskDateSet`, `TaskDateSetIter`, `TaskDateType`, `TaskError`, `TaskPriority`, `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`
    - `temporal`: `DateError`, `DateTimeValue`, `DateValue`, `DurationError`, `DurationValue`
  - **`pub(crate)` tier** (internal across crate modules):
    - `config`: `Config`, `ConfigService`, `TaskConfig`, `TrustRequest`
    - `delimiter`: `DelimiterType`
    - `dirtree`: `DirNode`, `DirTree`, `DirTreeError`
    - `field`: `FieldKey`, `FieldKeyRef`, `FieldName`, `FieldNameError`, `FieldNameRef`, `FieldValue`
    - `file`: `BaseName`, `BaseNameRef`, `FileName`
    - `file_tracker`: `CleanMode`, `FilePathTracker`, `FilePathTrackerError`
    - `hash`: `Blake3Digest`, `Blake3PathHash`
    - `index`: `IndexerService`
    - `lexer`: `LexError`, `SpannedTokenStream`, `TokenSpec`, `lexical_unquote`
    - `note`: `NoteFieldType`
    - `position`: `BytePos`, `ByteSpan`, `LineIndex`, `Spanned`
    - `template`: `TemplateService`
    - `temporal`: `DEFAULT_DATE_PATTERN`, `DEFAULT_DATETIME_PATTERN`, `TimeUnit`, `UNIT_SPELLINGS_HINT`
  - **Test-Only Tier (`pub mod testing` under `test-utils` / `cfg(test)`)**:
    - `file::FileFormat`
    - `hash::{Blake3Digest, Blake3PathHash}`
    - `path::codec as path_codec`
    - Plus services, models, and test-helpers (`TestProject`, `create_trusted_project`, etc.).

---

## 2. Responsibilities

Verified responsibilities and owning types/functions:

1. **Schema Field Identity (Exact)**:
   - `FieldName`, `FieldNameRef` (`src/field.rs:45-250`): Non-empty, whitespace-trimmed, slash-free exact string (`status` != `Status`). Used in schema definitions (`src/schema/`).
2. **Note Metadata Key (Forgiving & Canonical)**:
   - `FieldKey`, `FieldKeyRef` (`src/field.rs:263-577`): Case-insensitive, punctuation-stripped (`_`, `-` preserved, spaces mapped to `-`), display-preserving key with O(1) `IndexMap` lookup via `FieldKeyRef: Equivalent<FieldKey>`.
3. **Format-Agnostic Structured Value**:
   - `FieldValue`, `FieldValueRef`, `DateTimeCoercion`, `FieldStringValue` (`src/field.rs:585-994`): Generic scalar/composite tree (Null, Bool, Int, Float, String, Date, DateTime, List, Object) parsed from YAML/TOML/JSON, with string-to-temporal coercion during deserialization.
4. **Filesystem Metadata & Name Decomposition**:
   - `FileMeta`, `FileFormat`, `FileName`, `BaseName`, `BaseNameRef`, `FileNameError` (`src/file.rs:44-370`): File metadata (path, created/modified timestamps, size, Dataview-style day derivation) and extension-stripping stem newtypes for link resolution.
5. **Path Validation, Confinement & Codec**:
   - `RelativePath`, `SafeRelativePath`, `FolderRef`, `HasPath`, `PathError`, `codec` (`src/path.rs:33-435`): Lexical path safety (`RelativePath`), OS filesystem boundary confinement via external `strict_path` crate (`SafeRelativePath`), folder distance metric (`FolderRef`), path-sorted row abstraction (`HasPath`), and platform-native OS-byte/UTF-16 path serde (`codec`).
6. **Filesystem Traversal with Classified Errors**:
   - `DirTree`, `DirNode`, `DirNodeRef`, `DirTreeError` (`src/dirtree.rs:37-360`): Single-directory and recursive walk generator wrapping `walkdir`, classifying failures into `MissingRoot`, `RootInaccessible`, and `NodeInaccessible`.
7. **Cryptographic Hashing & Path Encoding**:
   - `Blake3Digest`, `Blake3Hasher`, `Blake3PathHash`, `HashError` (`src/hash.rs:23-170`): BLAKE3 content hashing, length-prefixed injective serialization for cache keys, and fixed `[u8; 64]` hex path hash for filesystem store entry names.
8. **Balanced Delimiter & Wikilink Tracking**:
   - `DelimiterType`, `DelimiterStack`, `QuoteType` (`src/delimiter.rs:10-340`): Fixed-depth stack (max 16) scanning paired brackets `()`, `[]`, `{}`, `[[]]` and quotes, with escape handling.
9. **Fuzzy String Matching**:
   - `closest_match` (`src/strsim.rs:11-21`): Levenshtein-distance candidate search with length/2 threshold for typo diagnostics (`file.<field>`, schema fields).
10. **Shared YAML Configuration**:
    - `parse`, `YAML_CONFIG` (`src/yaml.rs:18-30`): Singleton YAML 1.2 `serde_yaml_compat` `noyalib::ParserConfig` ensuring frontmatter and minijinja template filters parse YAML identically.
11. **Lexer Plumbing & Unquoting**:
    - `lex_prefix`, `LexTokenStream`, `SpannedTokenStream`, `TokenSpec`, `lexical_unquote`, `LexError` (`src/lexer.rs:19-305`): Single-token lookahead stream over `logos::Logos`, expectation checkers, and quote/backslash unescaping.
12. **Source Positioning & Span Plumbing**:
    - `BytePos`, `ByteSpan`, `LineIndex`, `SourceLine`, `Spanned`, `PositionError` (`src/position.rs:32-480`): Precomputed O(log n) line-starts lookup, `u32` byte positions/spans, 1-indexed `NonZeroU32` line numbers, and spanned wrapper.
13. **OS Directory & Store Paths**:
    - `StateDirRoot`, `CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS` (`src/dirs.rs:24-191`): Platform XDG/macOS/Windows directory resolution for config and state stores.
14. **Discovery Environment Variables**:
    - `CEILING_DIRS`, `IGNORED_DIRS`, `is_ignored_dir` (`src/env_vars.rs:8-44`): Global discovery ceiling paths and ignored directory set (`.git`, `target`, `node_modules`, `.traces`).

---

## 3. Invariants

1. **`FieldName` is non-empty and slash-free**:
   - *Enforced*: In constructor `FieldName::validate` (`src/field.rs:71-81`). Safe code cannot construct an invalid `FieldName`.
2. **`FieldKey` canonical form is never empty and preserves original name**:
   - *Enforced*: In constructor `FieldKey::try_new` (`src/field.rs:282-300`). Canonicalization (`src/field.rs:359-374`) strips non-alphanumeric punctuation and lowercases. If canonical form is empty, errors with `FieldKeyError::EmptyCanonical`. Equality and Hash compare `canonical`, ignoring case and whitespace/punctuation variations.
3. **`ByteSpan` invariant `start <= end`**:
   - *Enforced*: In constructor `ByteSpan::new` (`src/position.rs:88-92`) via `end.max(start)`. Struct fields are private to `src/position.rs`.
4. **`SourceLine` is non-zero**:
   - *Enforced*: By `NonZeroU32` newtype wrapper (`src/position.rs:270`). `SourceLine::new` returns `Option<Self>`.
5. **`RelativePath` has at least one normal component and no `..` or root components**:
   - *Enforced*: In `RelativePath::try_from` (`src/path.rs:113-130`). Rejects absolute paths, `..`, and prefix/root-dir components.
6. **`SafeRelativePath` resolves within root boundary**:
   - *Enforced*: In `SafeRelativePath::parse` (`src/path.rs:176-191`) via `strict_path::PathBoundary::strict_join`.
   - *Violation window*: As documented in `src/path.rs:159-161`, TOCTOU symlink race can theoretically bypass boundary if the filesystem changes between parse and subsequent read/write.
7. **`DelimiterStack` bounds**:
   - *Enforced*: `MAX_DELIMITER_DEPTH = 16` (`src/delimiter.rs:8`). Pushes beyond depth 16 are silently saturated without panic (`saturating_add`, `src/delimiter.rs:37`).
8. **Injective Structured Hash Framing**:
   - *Enforced*: `Blake3Hasher::len` prefixes all variable-length bytes with little-endian `u64` (`src/hash.rs:95-97`); optional fields carry a 1-byte presence tag (`src/hash.rs:105-115`).
9. **`Blake3PathHash` is exactly 64 ASCII bytes**:
   - *Enforced*: Fixed `[u8; 64]` array (`src/hash.rs:133`). `as_str()` does `std::str::from_utf8` and panics only if corrupted (`src/hash.rs:158`).

---

## 4. State & Lifecycle

- **Stateless Modules**:
  - `delimiter.rs`, `strsim.rs`, `position.rs`, `lexer.rs`: Pure functions, mathematical algorithms, stack buffers.
  - `path.rs`: Functional validators and path codecs (delegating FS checks to `strict-path`).
  - `hash.rs`: Pure cryptographic hashing functions and incremental stateful hashers.
- **Process-Lifetime Lazy State (`std::sync::LazyLock`)**:
  - `src/yaml.rs:18`: `YAML_CONFIG` (`ParserConfig::serde_yaml_compat`), allocated on first use.
  - `src/dirs.rs`: `HOME`, `CONFIG_HOME`, `STATE_HOME`, `TRACES_STATE_DIR`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS`. Initialized from environment variables or platform paths on first access.
  - `src/env_vars.rs`: `CEILING_DIRS` (parses `TRACES_CEILING_DIRS`), `IGNORED_DIRS` (parses `TRACES_IGNORED_DIRS`).
- **Disk / Persistence**:
  - `src/path.rs::codec`: Custom Serde serialization used by `src/index/store.rs` to write raw OS bytes (Unix) or `u16` wide chars (Windows) into `redb`.
  - `src/file.rs::FileMeta`: Persisted in `redb` (`FILES` table).
  - `src/hash.rs::Blake3PathHash`: Used by `src/file_tracker.rs` as the filename for symlink/text records under `~/.local/state/traces/tracked-configs/` and `trusted-configs/`.
- **Concurrency**:
  - All types implement `Send + Sync` (except `FieldKeyRef`, `DirNodeRef`, `BaseNameRef`, `FieldValueRef` which have standard lifetime borrowing rules).
  - `LazyLock` uses standard library thread-safe initialization. No mutexes or internal locks exist in any of these modules.

---

## 5. Dependency Edges

### Inbound / Outbound `crate::` Edges

1. **`src/field.rs`**:
   - *Outbound*: `crate::{DateTimeValue, DateValue, temporal::DatePoint}` (`src/field.rs:39`). Note: tightly coupled to temporal parsing for string classification!
   - *Inbound*:
     - `schema`: Uses `FieldName`, `FieldNameRef`, `FieldNameError`, `FieldValue`, `FieldValueRef`.
     - `note`: Uses `FieldKey`, `FieldKeyRef`, `FieldValueRef` (to convert to `NoteFieldValue`).
     - `config`: Uses `FieldName`, `FieldNameError`, `FieldKey`.
     - `query`: Uses `FieldKey`, `FieldKeyRef`.
2. **`src/file.rs`**:
   - *Outbound*: `crate::{DateValue, path::{FolderRef, RelativePath, HasPath, codec}, temporal::DatePoint}` (`src/file.rs:33-37, 228`).
   - *Inbound*:
     - `index`: Core row type `FileMeta`, `FileFormat`, `BaseName`, `BaseNameRef`.
     - `template`: Uses `BaseName`, `FileName`.
     - `query`: Uses `FileMeta`.
3. **`src/path.rs`**:
   - *Outbound*: None (depends only on `strict_path`, `serde`, `thiserror`).
   - *Inbound*:
     - `file`: `RelativePath`, `FolderRef`, `HasPath`, `codec`.
     - `config`: `RelativePath`, `SafeRelativePath`, `PathError`.
     - `schema`: `SafeRelativePath`, `PathError`.
     - `template`: `RelativePath`, `SafeRelativePath`, `PathError`, `FolderRef`.
     - `index`: `HasPath`, `FolderRef`, `codec`.
     - `query`: `display_portable`.
4. **`src/dirtree.rs`**:
   - *Outbound*: None (depends only on `walkdir`, `thiserror`).
   - *Inbound*:
     - `config::discovery`: `DirTree`.
     - `index::service`: `DirTree`, `DirNode`.
     - `schema::service`: `DirTree`, `DirTreeError`.
     - `template::loader`: `DirTree`, `DirTreeError`.
     - `file_tracker`: `DirTree`, `DirTreeError`.
5. **`src/hash.rs`**:
   - *Outbound*: None (depends only on `blake3`, `thiserror`).
   - *Inbound*:
     - `config::model`: `Blake3Hasher`, `Blake3Digest`.
     - `config::trust`: `Blake3Digest`, `HashError`.
     - `file_tracker`: `Blake3PathHash`.
6. **`src/delimiter.rs`**:
   - *Outbound*: None.
   - *Inbound*:
     - `task`: `DelimiterType` (checks whether status symbol violates delimiters).
     - `note::links`: `DelimiterType::DoubleBracket` for wikilink closing.
     - `note::parser::lexer`: `DelimiterType::{Bracket, Parenthesis}` for inline field wrappers.
     - `note::parser::marker`: `DelimiterType::Bracket` for task checkboxes.
7. **`src/strsim.rs`**:
   - *Outbound*: None (depends on `strsim` crate).
   - *Inbound*:
     - `query::grammar::field`: Typo suggestions for `file.<field>` and `list.<field>`.
     - `schema::model`: Typo suggestions for schema field names.
8. **`src/yaml.rs`**:
   - *Outbound*: None (depends on `noyalib`).
   - *Inbound*:
     - `note::metadata`: Parsing raw frontmatter string to `noyalib::Value`.
     - `template::engine::yaml`: minijinja `to_yaml` and `from_yaml` filter implementation.
9. **`src/lexer.rs`**:
   - *Outbound*: `crate::Spanned` (`src/lexer.rs:15`).
   - *Inbound*:
     - `query::grammar::expr`, `query::grammar::filter`, `query::grammar::source`: `SpannedTokenStream`, `TokenSpec`, `lexical_unquote`, `LexError`.
     - `temporal::duration`, `temporal::point`: `lex_prefix`.
10. **`src/position.rs`**:
    - *Outbound*: None.
    - *Inbound*:
      - `note::parser`: `BytePos`, `ByteSpan`, `LineIndex`.
      - `note::heading`, `note::lists`: `SourceLine`.
      - `note::parser::lexer`: `Spanned`.
      - `query::grammar::expr`, `query::grammar::filter`: `Spanned`.
      - `index::store`: `SourceLine`.
      - `lexer`: `Spanned`.
11. **`src/dirs.rs`**:
    - *Outbound*: None.
    - *Inbound*:
      - `config::discovery`: `dirs::CONFIG_HOME`.
      - `config::tracker`: `dirs::TRACKED_CONFIGS`.
      - `config::trust`: `dirs::TRUSTED_CONFIGS`.
      - `file_tracker`: `StateDirRoot`.
12. **`src/env_vars.rs`**:
    - *Outbound*: None.
    - *Inbound*:
      - `config::discovery`: `CEILING_DIRS`, `is_ignored_dir`.
      - `index::service`: `is_ignored_dir`.

### Layering Evaluation
- **Cycles**: No cycles detected. All 12 modules sit at the leaf tier of the crate.
- **Upward References / Leaked Internals**:
  - `src/field.rs` has an upward/lateral coupling to `src/temporal/` (`DateTimeValue`, `DateValue`, `DatePoint`). `FieldValueRef::from(noyalib::Value)` and `FieldStringValue::classify` automatically parse ISO dates into typed variants, pulling temporal logic into the basic field value type.
  - `src/file.rs` also pulls in `crate::temporal::DatePoint` and `crate::DateValue` to derive file calendar days.

---

## 6. Interfaces

- **Deep vs Shallow Analysis**:
  - `src/path.rs`: **Deep interface**. Encapsulates subtle lexical edge cases (`RelativePath`) and multi-platform OS-level path boundary validation (`SafeRelativePath` with `strict_path` resolving Windows 8.3 short paths, NTFS ADS, and symlinks). It also provides `HasPath` and the platform-specific `codec` module for `redb` path persistence.
  - `src/dirtree.rs`: **Deep interface**. Thin wrapper around `walkdir`, but deeply classifies root vs node failures and encapsulates traversal policy (`MissingRoot` vs inaccessible) cleanly.
  - `src/hash.rs`: **Deep interface**. `Blake3Hasher` encapsulates the injective length-prefixed framing and optional field protocol, preventing delimiter collision attacks across all persisted digests.
  - `src/field.rs`: **Mixed**. `FieldName` and `FieldKey` are deep: `FieldKeyRef` implements `indexmap::Equivalent<FieldKey>` with zero-allocation hashing for canonical strings. However, `FieldValue` / `FieldValueRef` is a very large (400+ lines of visitors and conversions) generic AST that largely mirrors `serde_json::Value` / `noyalib::Value`, and competes directly with `NoteFieldValue` (`src/note/field.rs`).
  - `src/strsim.rs`: **Shallow interface** (81 lines). A 10-line wrapper around `strsim::levenshtein` with a ceiling threshold `count().div_ceil(2).max(1)`. Only 2 callers in the entire codebase.
  - `src/yaml.rs`: **Shallow interface** (30 lines). Just stores a `LazyLock<ParserConfig>` and calls `noyalib::from_str_with_config`. Only 2 callers.
  - `src/env_vars.rs`: **Shallow interface** (67 lines). Two static `LazyLock`s and `is_ignored_dir()`.

---

## 7. Algorithms, Data Structures & Performance

1. **`FieldKey` Canonicalization & Hash Matching (`src/field.rs:359-447, 565-577`)**:
   - Strips non-alphanumeric punctuation and lowercases.
   - `FieldKey::is_canonical` (`src/field.rs:396-403`) checks without allocation whether a string slice is already in canonical form.
   - `FieldKeyRef` (`src/field.rs:565-570`) hashes borrowed `&str` directly via `FieldKey::to_canonical` without allocating on the common path. This enables zero-allocation, O(1) hash lookup in `IndexMap<FieldKey, V>`.
2. **`FolderRef::distance_to` (`src/path.rs:242-279`)**:
   - Calculates tree distance between two folder paths: hops up to lowest common ancestor (LCA) + hops down.
   - Iterates components in lockstep: $O(\text{depth}_A + \text{depth}_B)$ with zero heap allocation.
3. **`LineIndex` Line Conversion (`src/position.rs:36-61`)**:
   - Precomputes byte indices of `\n` into a `Box<[BytePos]>`.
   - `line_at(pos)` uses `slice::partition_point` -> $O(\log L)$ binary search where $L$ is line count. Zero allocation during lookup.
4. **`DelimiterStack` (`src/delimiter.rs:11-119`)**:
   - Fixed array `[DelimiterType; 16]`. Zero heap allocation for parsing balanced delimiters up to depth 16.
5. **Memory Footprint & Allocation Hotspots**:
   - `FieldValue` uses `IndexMap<String, FieldValue>` for objects and `Vec<FieldValue>` for lists. `FieldValueRef` uses `IndexMap<Cow<'a, str>, FieldValueRef<'a>>`.
   - `NoteFieldValue` (`src/note/field.rs:43`) has its own `Box<[NoteFieldValue]>` and `IndexMap<FieldKey, Box<[NoteFieldValue]>>`.
   - Every note parsing pass converts `noyalib::Value` -> `FieldValueRef` -> `NoteFieldValue` (`src/note/metadata.rs:71-72`), creating double conversion allocation layers.

---

## 8. Error Topology

- **Error Types**:
  - `FieldNameError`: `Empty`, `ContainsSlash`, `NotScalar` (`src/field.rs:996-1011`).
  - `FieldKeyError`: `Name(FieldNameError)`, `EmptyCanonical` (`src/field.rs:1014-1024`).
  - `PathError`: `Absolute`, `OutsideRoot`, `UnsafeComponent`, `StrictPath(StrictPathError)` (`src/path.rs:290-313`).
  - `DirTreeError`: `MissingRoot`, `RootInaccessible`, `NodeInaccessible` (`src/dirtree.rs:136-155`).
  - `HashError`: `io::Error` with path (`src/hash.rs:163-170`).
  - `LexError`: `UnexpectedToken`, `UnexpectedEndOfInput` (`src/lexer.rs:271-285`).
  - `PositionError`: `BytePosOverflow`, `ZeroSourceLine` (`src/position.rs:472-479`).
  - `FileNameError`: `Missing` (`src/file.rs:366-370`).
- **Context Handling & Loss**:
  - `PathError::fold_confinement` (`src/path.rs:322-333`): Maps `Absolute`, `UnsafeComponent`, and `OutsideRoot` to a generic "escape" handler, while preserving source `StrictPathError` for unverifiable paths.
  - In `src/template/path.rs:76-85`, `PathError` variants `UnsafeComponent`, `OutsideRoot`, and `StrictPath` are collapsed into `TemplatePathError::UnsafeComponent`, discarding the underlying `StrictPathError`.
  - In `src/index/error.rs:225-234`, `IndexError::Scan` downcasts `DirTreeError` directly to `io::Error`, bypassing `DirTreeError` wrapper context.

---

## 9. Duplicated / Conflicting Concepts

1. **`FieldValue` (`src/field.rs:586`) vs. `NoteFieldValue` (`src/note/field.rs:43`)**:
   - **Severe conceptual overlap**: The codebase has *two* distinct dynamically-typed metadata AST value enums!
   - `FieldValue`: Null, Bool, Int(i64), Float(f64), String, Date, DateTime, List(Vec), Object(IndexMap). Used by `schema` options and external select files.
   - `NoteFieldValue`: Null, Bool, Number(f64), String, Date, DateTime, Duration, Link, List(Box<[Self]>), Object(IndexMap). Used by notes, frontmatter, inline fields, and queries.
   - `NoteFieldValue` implements `From<FieldValueRef>` (`src/note/field.rs:532-564`), doing a lossy/redundant conversion step on every frontmatter load.
2. **`FieldName` vs. `FieldKey` vs. `FieldPath`**:
   - `FieldName`: Exact schema name (`src/field.rs:46`).
   - `FieldKey`: Canonical note metadata key (`src/field.rs:264`).
   - `FieldPath`: Query accessor path (`file.name`, `list.due`, `custom_field`) (`src/query/grammar/field.rs:37`).
   - While `FieldName` vs `FieldKey` is an intentional distinction (strict schemas vs forgiving note authoring), having them alongside each other in `field.rs` without clear ownership creates cognitive friction.
3. **`ByteSpan` / `BytePos` (`src/position.rs`) vs Query Spans vs Logos Spans**:
   - Logos emits `std::ops::Range<usize>`.
   - `src/position.rs` defines `BytePos(u32)` and `ByteSpan(BytePos, BytePos)`.
   - `SpannedTokenStream` bridges them with `Spanned::from_usize_range`, but `QuerySyntaxError` uses `miette::SourceSpan` (`usize, usize`).
   - The planned LSP AST span integration (`.scratch/md-pkm-lsp/map.md:78`) will require `Range<BytePos>` on `Note` items, which currently discard spans.

---

## 10. Doc / Glossary / ADR Drift

1. **`src/GLOSSARY.md` (Core)**:
   - "Root-Confined Path" is defined as avoiding "relative path, sanitized path, normalized path". Yet `src/path.rs` explicitly defines `RelativePath` (lexical only) and `SafeRelativePath` (filesystem confinement). The glossary conflates them into a single concept.
   - "Occurrence", "NavTarget", "Resolution", "Ambiguity" are documented under `src/GLOSSARY.md:72-105` as core concepts for the navigation spine, but **none** of these types exist in `src/` today! They are future LSP concepts pre-written into the glossary.
2. **`src/lib.rs` Module Documentation (`src/lib.rs:1-33`)**:
   - Documents modules like `dirs`, `dirtree`, `field`, `file`, `hash`, `path` as top-level components, but omits `position.rs`, `lexer.rs`, `delimiter.rs`, `strsim.rs`, `env_vars.rs`, and `yaml.rs` from the module listing!
3. **Visibility Probes in `src/lib.rs:63-107`**:
   - Probes claim to verify public vs private boundaries, but only test compile failures for `ConfigError`, `ConfigBuilder`, `SchemasConfig`, `FrontmatterConfig`. There are no compile-fail probes testing that root primitives like `FieldKey`, `RelativePath`, `DirTree`, etc., do not leak into the public API.

---

## 11. Candidate Architectural Questions

### 1. Dual Value Hierarchy: Merge or isolate `FieldValue` and `NoteFieldValue`?
- **Claim**: Having both `field::FieldValue` and `note::NoteFieldValue` is redundant, confusing, and forces an allocation/conversion step on every note frontmatter parse.
- **Evidence**: `src/field.rs:586` vs `src/note/field.rs:43`. Both define Null, Bool, Number/Float, String, Date, DateTime, List, Object. `NoteFieldValue` adds `Duration` and `Link`. `src/note/metadata.rs:71-72` parses YAML via `noyalib` into `FieldValueRef` and then calls `NoteFieldValue::from(field_value)`.
- **Strongest Counter-Argument**: `FieldValue` is format-agnostic structured data representing TOML/JSON configuration and schema option bags (where `Duration` and `Wikilink` do not exist). `NoteFieldValue` is a rich PKM domain entity with Obsidian links and Dataview durations. Keeping `FieldValue` clean preserves format neutrality for schemas without pulling PKM markdown link syntax into generic schema options.
- **Open Decision**: Should `FieldValue` remain the generic schema/config data representation, or should a single unified `DynamicValue` enum live in a core primitives module?
- **Touches**: `field.rs`, `note/field.rs`, `schema/`, `query/`.

### 2. Is `src/field.rs` violating single responsibility by embedding temporal coercion?
- **Claim**: `src/field.rs` is 1,832 lines long partly because it takes on date classification and temporal parsing (`DateTimeCoercion`, `FieldStringValue::classify`, calling `DatePoint::parse`), coupling root field primitives directly to `temporal::*`.
- **Evidence**: `src/field.rs:39` imports `DatePoint`. `src/field.rs:828-843` inspects every string during deserialization to coerce it to `Date` or `DateTime`.
- **Strongest Counter-Argument**: Obsidian/Dataview notes and frontmatter YAML do not have native date types in YAML 1.2; strings *must* be classified at deserialization time to prevent losing date types when parsing metadata. Centralizing this in the deserializer seed ensures schema options and note frontmatter classify dates identically.
- **Open Decision**: Should temporal string classification remain in `field.rs` during serde visitor walks, or be a deliberate post-processing pass owned by `note` and `schema`?
- **Touches**: `field.rs`, `temporal/`, `note/`, `schema/`.

### 3. Junk Drawer Elimination: Re-home single-consumer leaf modules
- **Claim**: `src/strsim.rs`, `src/yaml.rs`, `src/dirs.rs`, `src/env_vars.rs`, `src/delimiter.rs` do not belong as crate-root modules; they are orphans or single-purpose helpers that clutter the crate root.
- **Evidence**:
  - `src/strsim.rs` (81 lines) is used *only* for field typo hints in `query` and `schema`.
  - `src/yaml.rs` (30 lines) is a single `noyalib` config wrapper used only by `note` and `template`.
  - `src/dirs.rs` and `src/env_vars.rs` are used *only* by `config` (discovery/trust) and `file_tracker`.
  - `src/delimiter.rs` is essentially part of note/lexer parsing.
- **Strongest Counter-Argument**: Having them as root leaf modules prevents circular dependencies between higher-level subsystems (e.g., if `dirs` were in `config`, `file_tracker` would depend on `config` while `config` depends on `file_tracker`).
- **Open Decision**: Keep crate root as a flat collection of leaf primitives, or introduce a structured `fs::` / `util::` namespace or move single-consumer modules to their owners?
- **Touches**: All modules, `lib.rs`.

### 4. Span/Positioning Strategy for LSP readiness: Elevate `position.rs` to first-class AST carrier
- **Claim**: `src/position.rs` currently provides `BytePos`, `ByteSpan`, and `LineIndex`, but they are discarded during note parsing. If the upcoming LSP requires byte spans on every note entity (`Link`, `Tag`, inline fields, headings), `position.rs` must become the central span authority for both the note parser and query diagnostics.
- **Evidence**: `src/note/parser.rs:111` receives `ByteSpan` from pulldown-cmark but only uses it to pass line numbers to list items; spans on links and tags are dropped. `query::error` uses `miette::SourceSpan` (`usize, usize`) instead of `ByteSpan`.
- **Strongest Counter-Argument**: Adding spans to AST nodes increases the memory size of `Note` in the index and complicates `redb` storage unless spans are explicitly stripped or skipped before persistence (see `.scratch/md-pkm-lsp/map.md:91` research on `Spanned<T>` persistence).
- **Open Decision**: Should `ByteSpan` (`u32`-based) be adopted uniformly across `Note` and `Query` ASTs now, or kept minimal until the LSP crate is formally introduced?
- **Touches**: `position.rs`, `lexer.rs`, `note/`, `query/`.

### 5. Path Confinement & `strict-path` Dependency Architecture
- **Claim**: The distinction between `RelativePath` (lexical safety) and `SafeRelativePath` (filesystem confinement via `strict_path`) is well-designed, but `SafeRelativePath` performs I/O on every construction and returns a joined `PathBuf`, leaving TOCTOU symlink race windows.
- **Evidence**: `src/path.rs:176-191` creates a `PathBoundary` and validates boundaries on disk. `src/path.rs:19-22` explicitly documents the TOCTOU limitation.
- **Strongest Counter-Argument**: Traces is a synchronous single-user local CLI; full file descriptor open-at confinement (`openat2` / capsicum) is non-portable across Windows/macOS and massively over-engineered for a Markdown PKM tool. `strict-path` provides the best available cross-platform protection against directory traversal.
- **Open Decision**: Keep `path.rs` with `strict-path` as-is, or formalize root confinement into a dedicated capability-style `ProjectRoot` type that dispenses verified paths.
- **Touches**: `path.rs`, `config/`, `template/writer.rs`.

### 6. Crate-Root Re-export Coherence: `pub` vs `pub(crate)` tiers
- **Claim**: `src/lib.rs` re-exports a chaotic mix of internal implementation types (`DelimiterType`, `DirNode`, `Blake3Digest`, `SpannedTokenStream`, `LineIndex`) alongside public domain types (`Note`, `FileMeta`, `Tag`, `DialogProvider`). The `pub` vs `pub(crate)` distinction in `lib.rs` is blurred, and `testing` re-exports 45 items in a frozen whitelist.
- **Evidence**: `src/lib.rs:135-176`. `src/lib.rs:181` states: "The membership of this module is frozen: exactly the 45 identifiers required by external test targets".
- **Strongest Counter-Argument**: Since `traces` is a single binary crate, `lib.rs` serves only integration tests and benchmarks. The distinction between `pub` and `pub(crate)` is an intentional facade to keep integration tests clean.
- **Open Decision**: Keep `lib.rs` as the internal integration test harness, or structure the crate into clear public and internal layers (in anticipation of multi-crate workspace decomposition in ticket 34).
- **Touches**: `lib.rs`.
