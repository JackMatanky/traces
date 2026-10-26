# Inventory: Type Placement, Config Crossings, and Ambient Primitives

Investigation of current tree (HEAD `8064cf76`) for wayfinder ticket 22 sub-questions (a), (b), (f-part: importers of `dirs`/`env_vars`/`file_tracker`), and whole-crate configuration orientation.
Methodology: Direct file inspections, AST regex/symbol scans, and git history analysis (`git log --follow --oneline`).
Claim verification tags: `[V]` verified by direct read (`path:line`), `[H]` inference, `[P]` proposal.

---

## 1. Whole-Crate Orientation

### 1.1 Module Tree and Sizes (`src/lib.rs:110-134`)

The crate defines 25 crate-root modules in `src/lib.rs:110-134` [V]. The tree encompasses 94,829 LoC total (72 single/multi-file source files, excluding `tests/` and benches) [V]:

| Module | Form | Files | LoC | Subsystem Role |
|---|---|---|---|---|
| `cli` | dir | 13 | 8,175 | Command-line dispatch, Clap parsing, command handlers [V: `src/lib.rs:6-7, 110`] |
| `config` | dir | 15 | 6,033 | Discovery, TOML parsing, two-tier merging, trust/tracking [V: `src/lib.rs:8-9, 111`] |
| `delimiter` | file | 1 | 498 | Delimiter stack and paired character validation [V: `src/lib.rs:112`] |
| `dialog` | dir | 4 | 1,147 | Object-safe interactive/preset prompt abstraction [V: `src/lib.rs:10, 113`] |
| `dirs` | file | 1 | 218 | Platform XDG/OS config/state path resolution [V: `src/lib.rs:11-12, 114`] |
| `dirtree` | file | 1 | 1,098 | Directory traversal with classified walk errors [V: `src/lib.rs:13, 115`] |
| `env_vars` | file | 1 | 68 | Ambient environment variable parsing and walk filters [V: `src/lib.rs:116`] |
| `field` | file | 1 | 1,845 | Exact/forgiving field identities and dynamic values [V: `src/lib.rs:14-15, 117`] |
| `file` | file | 1 | 667 | File metadata and filename newtypes [V: `src/lib.rs:16-17, 118`] |
| `file_class_expander` | file | 1 | 87 | Adapter implementing query expander for schema [V: `src/lib.rs:18-19, 119`] |
| `file_tracker` | file | 1 | 1,189 | BLAKE3 hash-keyed path tracking store [V: `src/lib.rs:20-21, 120`] |
| `hash` | file | 1 | 419 | Injective length-prefixed BLAKE3 hashing [V: `src/lib.rs:22, 121`] |
| `index` | dir | 13 | 10,228 | Redb persistent index, metadata scanner, inlink graph [V: `src/lib.rs:23, 122`] |
| `lexer` | file | 1 | 657 | Spanned token streams and lexical unquoting [V: `src/lib.rs:123`] |
| `note` | dir | 15 | 10,171 | Markdown note parser, YAML frontmatter, lists/tasks [V: `src/lib.rs:26-27, 124`] |
| `path` | file | 1 | 788 | Root-relative path validation and confinement [V: `src/lib.rs:28, 125`] |
| `position` | file | 1 | 799 | Source line, byte span, and line index primitives [V: `src/lib.rs:126`] |
| `query` | dir | 14 | 11,419 | Read-side query language, planner, and execution [V: `src/lib.rs:24-25, 127`] |
| `schema` | dir | 20 | 12,126 | Schema registry, inheritance DAG, field definitions [V: `src/lib.rs:29-31, 128`] |
| `strsim` | file | 1 | 81 | Levenshtein edit distance and closest match helper [V: `src/lib.rs:129`] |
| `tag` | file | 1 | 600 | Validated `#`-prefixed hierarchical tag primitive [V: `src/lib.rs:39, 130`] |
| `task` | file | 1 | 1,911 | Task status, status map, date set, priority, emojis [V: `src/lib.rs:42-43, 131`] |
| `template` | dir | 18 | 13,345 | Minijinja rendering, template loader, file writer [V: `src/lib.rs:32-33, 132`] |
| `temporal` | dir | 10 | 10,264 | Jiff-backed dates, datetimes, and durations [V: `src/lib.rs:133`] |
| `yaml` | file | 1 | 30 | Frontmatter YAML parser wrapper over `noyalib` [V: `src/lib.rs:134`] |
| `lib.rs` | file | 1 | 897 | Crate facade, visibility probes, `testing` exports, test fixtures [V] |
| `main.rs` | file | 1 | 69 | Binary entrypoint and exit code mapping [V] |
| **Total** | | **72** | **94,829** | Crate codebase |

### 1.2 Re-exports in `src/lib.rs`

- **Public API (`pub`)**:
  - `cli::*` (`pub mod cli;` [V: `src/lib.rs:110`])
  - `dialog::{DialogError, DialogProvider, DialogResult, PresetDialogProvider, TerminalDialogProvider}` [V: `src/lib.rs:138-141`]
  - `file::FileMeta` [V: `src/lib.rs:146`]
  - `note::{Heading, ListItem, ListItemType, ListText, MarkdownParserInput, Note, NoteFieldValue, NoteFieldValueRef, TaskListItem, parse_markdown}` [V: `src/lib.rs:157-160`]
  - `position::{PositionError, SourceLine}` [V: `src/lib.rs:162`]
  - `tag::{Tag, TagError}` [V: `src/lib.rs:163`]
  - `task::{TaskDate, TaskDateSet, TaskDateSetIter, TaskDateType, TaskError, TaskPriority, TaskStatus, TaskStatusMap, TaskStatusSymbol, TaskStatusType}` [V: `src/lib.rs:164-167`]
  - `temporal::{DateError, DateTimeValue, DateValue, DurationError, DurationValue}` [V: `src/lib.rs:174-176`]
- **Internal API (`pub(crate)`)**:
  - `config::{Config, ConfigService, TaskConfig, TrustRequest}` [V: `src/lib.rs:136`]
  - `delimiter::DelimiterType` [V: `src/lib.rs:137`]
  - `dirtree::{DirNode, DirTree, DirTreeError}` [V: `src/lib.rs:142`]
  - `field::{FieldKey, FieldKeyRef, FieldName, FieldNameError, FieldNameRef, FieldValue}` [V: `src/lib.rs:143-145`]
  - `file::{BaseName, BaseNameRef, FileName}` [V: `src/lib.rs:147`]
  - `file_tracker::{CleanMode, FilePathTracker, FilePathTrackerError}` [V: `src/lib.rs:148-150`]
  - `hash::{Blake3Digest, Blake3Hasher, Blake3PathHash}` [V: `src/lib.rs:151`]
  - `index::IndexerService` [V: `src/lib.rs:152`]
  - `lexer::{LexError, SpannedTokenStream, TokenSpec, lexical_unquote}` [V: `src/lib.rs:153-155`]
  - `note::NoteFieldType` [V: `src/lib.rs:156`]
  - `position::{BytePos, ByteSpan, LineIndex, Spanned}` [V: `src/lib.rs:161`]
  - `template::TemplateService` [V: `src/lib.rs:169`]
  - `temporal::{DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN, TimeUnit, UNIT_SPELLINGS_HINT}` [V: `src/lib.rs:170-173`]
- **Frozen Test Export (`pub mod testing`)**:
  - 46 identifiers under `#[cfg(any(test, feature = "test-utils"))]` [V: `src/lib.rs:178-212`].
- **Test Support Fixture Submodule (`mod test_support`)**:
  - Lines 214-897 (684 lines, 76% of `src/lib.rs`) containing `TestProject`, `fixture_service`, and test builders [V: `src/lib.rs:214-897`].

### 1.3 External Consumers of Config Types Outside `src/config/`

The table below lists every caller consuming `Config`, `ConfigService`, `TaskConfig`, `FrontmatterConfig`, `SchemasConfig`, or `ConfigDigests` outside `src/config/`.

| Consumed Type | Consumer File & Line | Invocation Signature / Usage | Takes `&Config` or Projection? |
|---|---|---|---|
| `Config` | `src/template/service.rs:39, 58` [V] | `TemplateService::new(config: &'a Config, ...)` | Takes `&'a Config` (stored in struct field `config`) [V] |
| `Config` | `src/template/service.rs:81` [V] | `TemplateService::list_available(config: &Config)` | Takes `&Config` (passes to `TemplateLoader::from(config)`) [V] |
| `Config` | `src/template/loader.rs:307-308` [V] | `impl From<&Config> for TemplateLoader` | Projection (`config.local_template_dir()`, `config.global_template_dir()`) [V: `src/template/loader.rs:309-310`] |
| `Config` | `src/template/engine.rs:112` [V] | `TemplateEngine::new(..., config: &Config)` | Projection (`root_arc()`, `class_field_arc()`, `schemas().directory()`, `IndexerService::from(config)`) [V: `src/template/engine.rs:125-135`] |
| `Config` | `src/index/service.rs:47-49` [V] | `impl From<&Config> for IndexerService` | Projection (`root`, `tasks`, `frontmatter`, `class_field_name`, `digests`) [V: `src/index/service.rs:50-58`] |
| `Config` | `src/cli/mod.rs:273` [V] | `with_config(service, handler: impl FnOnce(&Config))` | Passes `&Config` to CLI command runners [V] |
| `Config` | `src/cli/list.rs:61, 83` [V] | `ListCmd::run(&self, config: &Config)` | Takes `&Config` (projects `config.root()`, `config.schemas()`) [V] |
| `Config` | `src/cli/index.rs:38` [V] | `IndexCmd::run(&self, config: &Config)` | Takes `&Config` (projects `IndexerService::from(config)`) [V] |
| `Config` | `src/cli/task.rs:191, 215` [V] | `TaskCmd::run(&self, config: &Config)` | Takes `&Config` (projects `config.root()`, `config.tasks()`, `config.schemas()`) [V] |
| `Config` | `src/cli/table.rs:60, 82` [V] | `TableCmd::run(&self, config: &Config)` | Takes `&Config` (projects `config.root()`, `config.schemas()`) [V] |
| `Config` | `src/cli/template.rs:121, 133, 161, 194, 237` [V] | `TemplateCmd::run`, `render`, `list_templates` | Takes `&Config` (passes to `TemplateService::new`) [V] |
| `Config` | `src/cli/mod.rs:490` [V] | `load_schema_service(config: &Config)` | Projection (`config.root()`, `config.schemas().directory()`) [V: `src/cli/mod.rs:491-492`] |
| `ConfigService` | `src/cli/mod.rs:123, 183` [V] | `Cli::run(&self, service: &ConfigService, ...)` | Takes `&ConfigService` [V] |
| `ConfigService` | `src/cli/mod.rs:227` [V] | `Cli::parse().run(&ConfigService::default(), ...)` | Constructs default service in `main.rs` dispatch [V] |
| `ConfigService` | `src/cli/mod.rs:248` [V] | `load_config(service: &ConfigService) -> Result<Config>` | Calls `service.load(&cwd)` [V] |
| `ConfigService` | `src/cli/mod.rs:272` [V] | `with_config(service: &ConfigService, ...)` | Calls `load_config(service)` [V] |
| `ConfigService` | `src/cli/mod.rs:538` [V] | `ConfigService::trust_subjects(path, scope)` | Static method call [V] |
| `ConfigService` | `src/cli/trust.rs:49, 134` [V] | `TrustCmd::run(self, service: &ConfigService)` | Calls `service.trust(&request)` [V] |
| `ConfigService` | `src/cli/untrust.rs:38` [V] | `UntrustCmd::run(self, service: &ConfigService)` | Calls `service.untrust(&request)` [V] |
| `ConfigService` | `src/cli/completions.rs:47, 99, 117` [V] | `CompletionsCmd::run(self, service: &ConfigService)` | Calls `load_config(service)` to list templates [V] |
| `ConfigService` | `src/dirs.rs:173` [V] | Doc comment reference only [V] | N/A |
| `TaskConfig` | `src/index/service.rs:41, 52` [V] | `IndexerService.tasks: TaskConfig` | Stored field (cloned from `config.tasks()`) [V] |
| `TaskConfig` | `src/note/parser/input.rs:31, 44, 85` [V] | `MarkdownParserInput.tasks: &'a TaskConfig` | Borrowed field (`input.tasks()`) [V] |
| `TaskConfig` | `src/note/parser.rs:527-528` [V] | `input.tasks().tag_filters()`, `statuses()` | Projection (`&[Tag]`, `&TaskStatusMap`) [V] |
| `TaskConfig` | `src/note/parser.rs:619, 1794` [V] | `parse_with_tasks(src, tasks: &TaskConfig)` (tests) | Test helper borrowing `&TaskConfig` [V] |
| `TaskConfig` | `src/template/engine/query.rs:1387` [V] | `TaskConfig::from_tags(&["#task"])` (test) | Test builder [V] |
| `TaskConfig` | `src/query/results.rs:1062, 1143` [V] | `tasks: &crate::TaskConfig` (test helper) | Test helper [V] |
| `TaskConfig` | `src/query/service.rs:1185` [V] | `TaskConfig::from_tags(&["#task"])` (test) | Test builder [V] |
| `FrontmatterConfig` | `src/index/service.rs:42, 53` [V] | `IndexerService.frontmatter: FrontmatterConfig` | Stored field (cloned from `config.frontmatter()`) [V] |
| `FrontmatterConfig` | `src/note/parser/input.rs:33, 45, 92` [V] | `MarkdownParserInput.frontmatter: &'a FrontmatterConfig` | Borrowed field (`input.frontmatter()`) [V] |
| `FrontmatterConfig` | `src/note/parser.rs:381` [V] | `input.frontmatter().tags_name()` | Projection (`&str`) [V] |
| `FrontmatterConfig` | `src/note/parser.rs:632, 1769` [V] | `frontmatter: &FrontmatterConfig` (tests) | Test helper [V] |
| `SchemasConfig` | `src/index/service.rs:1771, 1963` [V] | `SchemasConfig::for_test("kind")` (tests only) | Zero production callers outside `src/config/` [V] |
| `SchemasConfig` | `src/template/engine/query.rs:622, 688` [V] | `SchemasConfig::for_test(class_field)` (test only) | Zero production callers outside `src/config/` [V] |
| `ConfigDigests` | `src/index/fingerprint.rs:158-159` [V] | `IndexFingerprint::new(digests: &ConfigDigests)` | Destructures 10 setting digests [V: `src/index/fingerprint.rs:159-170`] |

### 1.4 Orientation Summary Findings

1. `ConfigService` has **zero callers outside `src/cli/`** (and test harness in `src/lib.rs`). Core subsystems (`note`, `schema`, `index`, `query`, `template`) never interact with `ConfigService` [V].
2. `SchemasConfig` has **zero production callers outside `src/config/`** [V]. Outside config, only two test modules import it. Production consumers (`index`, `schema`, `cli`) only query `config.schemas().class_field_name()` or `config.schemas().directory()` [V].
3. `ConfigDigests` is consumed **strictly by `src/index/fingerprint.rs`** [V].
4. `TemplateService` is the **only subsystem struct holding an owned reference `&'a Config`** [V: `src/template/service.rs:39`]. `IndexerService` decomposes `&Config` upon construction into owned projections (`PathBuf`, `TaskConfig`, `FrontmatterConfig`, `IndexDimensions`, `Box<IndexFingerprint>`) [V: `src/index/service.rs:50-58`].

---

## 2. (a) Root Domain Types (`src/task.rs`, `src/tag.rs`, `src/field.rs`)

### 2.1 Inventory, Sizes, and Public Surface

| File | Total LoC | Prod LoC | Test LoC | Test Start | Public Items (`pub` / `pub(crate)`) |
|---|---|---|---|---|---|
| `src/task.rs` [V] | 1,911 | 1,152 | 759 | `1153` [V] | `TaskStatus`, `TaskStatusMap`, `TaskStatusType`, `TaskStatusSymbol`, `TaskDateSet`, `TaskDateSetIter`, `TaskDate`, `TaskDateType`, `TaskPriority`, `TaskError` [V: `src/task.rs:51-1132`] |
| `src/tag.rs` [V] | 600 | 264 | 336 | `265` [V] | `Tag`, `TagError` [V: `src/tag.rs:28, 242`] |
| `src/field.rs` [V] | 1,845 | 1,008 | 837 | `1009` [V] | `FieldName`, `FieldNameRef`, `FieldKey`, `FieldKeyRef`, `FieldValue`, `FieldValueRef`, `DateTimeCoercion`, `FieldStringValue`, `FieldNameError`, `FieldKeyError` [V: `src/field.rs:46-997`] |

### 2.2 Classification and Module Usage of Each Item

Each item is classified as:

- **Config-shaped**: Configuration model, lookup map, or settings error.
- **Note-shaped**: Document AST node, inline parser helper, or markdown token.
- **Query-shaped**: Projection target, filter operand, or ordering comparator.
- **Schema-shaped**: Exact identifier or schema option value.
- **Generic**: Domain newtype or primitive shared across 3+ layers.

#### `src/task.rs` Items

1. **`TaskStatus`** (`pub struct`, line 51) [V]:
   - *Shape*: Hybrid Config / Note domain record (`symbol: TaskStatusSymbol`, `name: String`, `kind: TaskStatusType`) [V: `src/task.rs:51-55`].
   - *Importers*: 40 references across `config(6)`, `lib(4)`, `note(30)` [V].
     - `src/config/model/tasks.rs:5, 48`
     - `src/note/lists.rs:351` (field on `TaskListItem`), `src/note/model.rs:227`
     - `src/note/parser/list.rs:420`
2. **`TaskStatusMap`** (`pub struct`, line 119) [V]:
   - *Shape*: Config-shaped lookup table (indexing by symbol char, status name, and workflow kind) [V: `src/task.rs:119-125`].
   - *Importers*: 30 references across `config(6)`, `lib(2)`, `note(22)` [V].
     - `src/config/model/tasks.rs:5, 16, 33`
     - `src/note/parser/list.rs:15, 146, 418` (used in `ListNestingTracker::end_item`)
     - `src/note/parser/task.rs:13`
3. **`TaskStatusType`** (`pub enum`, line 292) [V]:
   - *Shape*: Generic workflow enum (`Todo`, `Done`, `InProgress`, `Cancelled`) [V: `src/task.rs:292-297`].
   - *Importers*: 36 references across `config(8)`, `index(3)`, `lib(2)`, `note(23)` [V].
     - `src/config/model/tasks.rs:6, 62`
     - `src/index/fingerprint.rs:35`
     - `src/note/lists.rs:657`, `src/note/model.rs:227`
4. **`TaskStatusSymbol`** (`pub struct`, line 371) [V]:
   - *Shape*: Note-parsing & query literal matching newtype (`char`) [V: `src/task.rs:371`].
   - *Importers*: 19 references across `config(4)`, `lib(1)`, `note(10)`, `query(4)` [V].
     - `src/config/model/tasks.rs:6, 50`
     - `src/note/lists.rs:657, 665`, `src/note/model.rs:227`
     - `src/query/results.rs:25`
5. **`TaskDateSet`** (`pub struct`, line 498) [V]:
   - *Shape*: Note-shaped AST storage (4-slot fixed array for `due`, `scheduled`, `completion`, `start`) [V: `src/task.rs:498-502`].
   - *Importers*: 37 references across `lib(1)`, `note(35)`, `query(1)` [V].
     - **Zero references in `config/`** [V].
     - `src/note/lists.rs:27, 249` (field on `TaskListItem`)
     - `src/note/parser/task.rs:12`
     - `src/query/results.rs:420`
6. **`TaskDateSetIter`** (`pub struct`, line 649) [V]:
   - *Shape*: Note-shaped iterator over `TaskDateSet`.
   - *Importers*: 1 reference (`src/lib.rs:165` re-export) [V].
7. **`TaskDate`** (`pub struct`, line 690) [V]:
   - *Shape*: Note-shaped record `(TaskDateType, DatePoint)` [V: `src/task.rs:690-694`].
   - *Importers*: 14 references across `lib(1)`, `note(13)` [V].
     - **Zero references in `config/`** [V].
     - `src/note/lists.rs:657, 893`
8. **`TaskDateType`** (`pub enum`, line 745) [V]:
   - *Shape*: Note-shaped parser token & query date selector (`Due`, `Scheduled`, `Completion`, `Start`). Contains markdown emoji constants `EMOJIS: [(char, TaskDateType); 4]` [V: `src/task.rs:767-772`].
   - *Importers*: 64 references across `lib(1)`, `note(53)`, `query(10)` [V].
     - **Zero references in `config/`** [V].
     - `src/note/lists.rs:657, 894`
     - `src/note/parser/task.rs:13`
     - `src/query/grammar/source.rs:32`
9. **`TaskPriority`** (`pub enum`, line 959) [V]:
   - *Shape*: Note-shaped parser token & query priority rank. Contains markdown emoji constants `EMOJIS: [(char, TaskPriority); 5]` [V: `src/task.rs:962-968`].
   - *Importers*: 60 references across `lib(1)`, `note(56)`, `query(3)` [V].
     - **Zero references in `config/`** [V].
     - `src/note/lists.rs:27, 330` (field on `TaskListItem`)
     - `src/note/parser/task.rs:13`
     - `src/query/results.rs:430`
10. **`TaskError`** (`pub enum`, line 1132) [V]:
    - *Shape*: Config/CLI error enum (`InvalidPriority`, `InvalidStatusSymbol`, `DuplicateStatusSymbol`) [V: `src/task.rs:1132-1148`].
    - *Importers*: 7 references across `cli(2)`, `config(4)`, `lib(1)` [V].
      - `src/config/error.rs:14, 144`
      - `src/cli/error.rs:24`

#### `src/tag.rs` Items

1. **`Tag`** (`pub struct`, line 28) [V]:
   - *Shape*: Generic document primitive (`Tag(Box<str>)`) validating `#`-prefix, body characters, segments (`/`), and nested matches (`is_contained_in`) [V: `src/tag.rs:28, 56-76`].
   - *Importers*: 135 references across `config(14)`, `index(9)`, `lib(8)`, `note(82)`, `query(22)` [V].
     - `src/config/model/tasks.rs:5, 17` (`tasks.tag_filters: Vec<Tag>`)
     - `src/note/metadata.rs:12`, `src/note/lists.rs:49` (`ListItem.tags: Box<[Tag]>`)
     - `src/index/inlinks.rs:22`
     - `src/query/grammar/source.rs:35`
2. **`TagError`** (`pub enum`, line 242) [V]:
   - *Shape*: Generic validation error (`Empty`, `MissingHash`, `LeadingCharNotAlphabetic`, `EmptySegment`, `InvalidCharacter`) [V: `src/tag.rs:242-262`].
   - *Importers*: 4 references across `config(2)`, `lib(2)` [V: `src/config/error.rs:13, 140`].

#### `src/field.rs` Items

1. **`FieldName`** (`pub(crate) struct`, line 46) [V]:
   - *Shape*: Schema-shaped & Config-shaped exact identifier (`FieldName(String)`: case-sensitive, whitespace-trimmed, no slashes) [V: `src/field.rs:46, 78-87`].
   - *Importers*: 69 references across `config(29)`, `lib(1)`, `schema(39)` [V].
     - **Zero references in `note/` or `query/`** [V].
     - `src/config/builder.rs:19, 120`
     - `src/config/model/frontmatter.rs:5, 30`
     - `src/config/model/schemas.rs:6, 17`
     - `src/schema/fields/parser.rs:17`
2. **`FieldNameRef`** (`pub(crate) struct`, line 203) [V]:
   - *Shape*: Schema-shaped borrowed counterpart (`&'a str`).
   - *Importers*: 21 references across `lib(1)`, `schema(20)` [V: `src/schema/fields/address.rs:9`].
     - **Zero references in `note/`, `query/`, or `config/`** [V].
3. **`FieldKey`** (`pub(crate) struct`, line 257) [V]:
   - *Shape*: Note-shaped & Query-shaped forgiving note metadata key (stores original `name` for display + `canonical` for lowercase/punctuation-stripped lookup) [V: `src/field.rs:257-262`].
   - *Importers*: 68 references across `lib(1)`, `note(55)`, `query(3)`, `schema(9)` [V].
     - **Zero references in `config/`** [V].
     - `src/note/metadata.rs:4, 12`, `src/note/model.rs:31` (`Note.inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>`)
     - `src/query/grammar/field.rs:14`
     - `src/schema/resolve.rs:42` (used only to detect canonical field name collisions)
4. **`FieldKeyRef`** (`pub(crate) struct`, line 536) [V]:
   - *Shape*: Note-shaped borrowed zero-copy lookup counterpart.
   - *Importers*: 10 references across `lib(1)`, `note(8)`, `query(1)` [V: `src/note/metadata.rs:12, 130`].
     - **Zero references in `config/` or `schema/`** [V].
5. **`FieldValue`** (`pub(crate) enum`, line 568) [V]:
   - *Shape*: Schema-shaped owned dynamic value (`Null`, `Bool`, `Int(i64)`, `Float(f64)`, `String`, `Date`, `DateTime`, `List`, `Object`) [V: `src/field.rs:568-580`].
   - *Importers*: 279 references across `lib(1)`, `schema(274)`, `template(4)` [V].
     - **Zero references in `note/`, `config/`, `index/`, or `query/`** [V].
6. **`FieldValueRef`** (`pub(crate) enum`, line 689) [V]:
   - *Shape*: Note frontmatter zero-copy deserialization bridge [V: `src/field.rs:689-701`].
   - *Importers*: 20 references across `lib(1)`, `note(20)` [V].
     - **Zero references in `config/`, `schema/`, `index/`, or `query/`** [V].
     - `src/note/metadata.rs:12, 71`, `src/note/field.rs:24, 532`
7. **`DateTimeCoercion`** & **`FieldStringValue`** (`pub(crate)`, lines 775, 791) [V]:
   - *Shape*: Internal parsing helpers for string-vs-date classification in `field.rs`.
   - *Importers*: 0 references outside `src/field.rs` [V].
8. **`FieldNameError`** (`pub(crate) enum`, line 979) [V]:
   - *Shape*: Schema/Config error (`Empty`, `ContainsSlash`, `NotScalar`) [V: `src/field.rs:979-994`].
   - *Importers*: 10 references across `config(7)`, `lib(1)`, `schema(2)` [V: `src/config/error.rs:13, 131`].
9. **`FieldKeyError`** (`pub(crate) enum`, line 997) [V]:
   - *Shape*: Note key error (`Name`, `EmptyCanonical`) [V: `src/field.rs:997-1007`].
   - *Importers*: 0 references outside `src/field.rs` [V].

### 2.3 Dependency Edges Hidden by Crate-Root Types

Hoisting these types to `src/` obscures real architectural coupling and conflates separate subsystem concerns [H]:

1. **`FieldName` hides the `config <-> schema` edge**:
   `config` parses `[schemas].class_field` (`src/config/builder.rs:120`) and `[frontmatter]` keys (`src/config/model/frontmatter.rs:30-34`) into `FieldName`. `schema` uses `FieldName` for its exact field names (`src/schema/model.rs:35`). The note subsystem has **0 references to `FieldName`** [V]. `FieldName` exists at the root solely to share exact identifier syntax between `config` and `schema`.
2. **`TaskStatusMap` hides the `config <-> note` edge**:
   `config` builds `TaskStatusMap` from `[tasks]` TOML (`src/config/model/tasks.rs:16`). `note` consumes `TaskStatusMap` during list item flushing (`src/note/parser/list.rs:15, 528`).
3. **`task.rs` hides note-internal parsing logic at crate root**:
   `TaskPriority`, `TaskDateSet`, `TaskDate`, `TaskDateType`, and the parsing emoji maps (`TaskDateType::EMOJIS`, `TaskPriority::EMOJIS`) are **100% note-parsing and query concepts with zero consumers in `config`** [V]. Placing them in `task.rs` at the crate root was done merely because they contain the word "Task".
4. **`field.rs` bundles two disjoint type hierarchies**:
   - Hierarchy A (Schema & Config): `FieldName`, `FieldNameRef`, `FieldNameError`, and `FieldValue` (used 274 times in `schema`, 0 in `note`).
   - Hierarchy B (Note frontmatter & Query): `FieldKey`, `FieldKeyRef`, `FieldValueRef`, and temporal coercion (`DatePoint::parse`).
   These two hierarchies share neither methods nor data structures; they are colocated in a single 1,845-line root file.
5. **`FieldValueRef` creates a 3-stage intermediate AST during frontmatter parsing**:
   `src/note/metadata.rs:71-72` parses YAML via `noyalib::Value`, converts to `field::FieldValueRef<'static>`, and then converts `FieldValueRef` into `note::NoteFieldValue` [V]. This intermediate heap-allocating hop exists solely because `FieldValueRef` was parked in `src/field.rs`.

### 2.4 Why They Are at the Root: Git History & Historical Intent

1. **`src/task.rs`**:
   - `b3c2cba8` (*2026-09-01, feat(task): add task status model and [tasks] config section*) [V]: Created `src/task.rs` containing `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`. Root placement was chosen because both `config` (loading `[tasks]`) and `note` (parsing checkboxes) needed the status lookup map without creating a cycle between `config` and `note`.
   - `f60cc1b3` (*2026-09-18, refactor(task): move TaskPriority and TaskDates to task.rs, privatize fields*) [V]: Moved `TaskPriority` and `TaskDates` (`TaskDateSet`) out of `src/note/lists.rs` into `src/task.rs` under the rationale of consolidating all "task domain types" in one file. This inadvertently hoisted note-specific markdown emoji tables (`EMOJIS`) into the crate root.
2. **`src/tag.rs`**:
   - `07b9f7de` (*2026-08-27, feat: add validated Tag, delete note/tag.rs*) [V]: Deleted `src/note/tag.rs` and created `src/tag.rs`. Extracted because `query` and `index` needed to filter by tag without depending on `note`. Later (`b3c2cba8`), `config` added `tasks.tag_filters: Vec<Tag>`, deepening root placement.
3. **`src/field.rs`**:
   - `5c2cc3db` (*2026-08-09, refactor(field): extract validated FieldName/FieldKey primitives*) [V]: Moved `FieldKey` and `FieldName` out of `src/note/metadata.rs` into `src/field.rs` because `schema` needed exact field identity (`FieldName`) and `config` needed to validate field names in TOML.
   - `dcac8c4c` (*2026-08-14, refactor(field): merge zero-copy FieldValue/FieldValueRef from ticket-07 worktree*) [V]: Merged `FieldValue` and `FieldValueRef` into `src/field.rs`.
   - `20912ac7` (*2026-08-25, feat(date): consolidate date/datetime parsing into src/date.rs*) [V]: Coupled `field.rs` to `temporal` by adding `DatePoint::parse` coercion into string deserialization.

### 2.5 Mentions in ADRs and GLOSSARY

- **ADR-0002 (`docs/adr/0002-workspace-trust-boundary.md`)**: Defines workspace root as the single security perimeter. Explains why config trust gates the whole vault, but does not dictate where domain AST models live.
- **ADR-0003 (`docs/adr/0003-index-based-dialog-selection.md`)**: Mandates index-based `usize` selection for `DialogProvider`.
- **`src/GLOSSARY.md:24-34` [V]**:
  - Defines `Field Key` as "The canonical, case-insensitive identifier for note and schema metadata fields, preserving author casing for display." *Observation*: The glossary conflates note and schema metadata under `Field Key`, whereas the code split them into `FieldName` (schema) and `FieldKey` (note).
  - Defines `Tag` as a `#`-prefixed identifier supporting hierarchical sub-tag prefix matching.
- **`src/config/GLOSSARY.md:106-114` [V]**:
  - Defines `Task Configuration` as the `[tasks]` table configuring checkbox status symbols and tag filters.
- **`src/note/GLOSSARY.md:66-82` [V]**:
  - Defines `Task`, `Task Status`, and `Tag`.

---

## 3. (b) The Upward `note -> config` Edge

### 3.1 Trace of Cross-Subsystem Edges

#### Edges from `note` to `config`

There is **exactly one production import edge from `note` to `config`** across the entire codebase [V]:

- `src/note/parser/input.rs:9` [V]:

  ```rust
  use crate::config::{FrontmatterConfig, TaskConfig};
  ```

  `MarkdownParserInput<'a>` stores:
  - `tasks: &'a TaskConfig` [V: `src/note/parser/input.rs:31`]
  - `frontmatter: &'a FrontmatterConfig` [V: `src/note/parser/input.rs:33`]

(In tests, `src/note/parser.rs:620, 634, 1769, 1798` and `src/note/parser/task.rs:941-942` use `TaskConfig` / `FrontmatterConfig` test constructors).

#### Edges from `note` to Root Types

- `note -> crate::Tag`: 82 occurrences across metadata, list items, and tag parsing [V].
- `note -> crate::TaskStatusMap`: 22 occurrences in list item status lookup (`src/note/parser/list.rs:15, 146, 418, 528`) [V].
- `note -> crate::TaskStatusSymbol`: 10 occurrences (primarily test fixtures) [V].
- `note -> crate::FieldName`: **0 occurrences across all of `src/note/`** [V]. `note` uses `FieldKey` exclusively.

### 3.2 Exact Configuration Values Received by the Parsers

`parse_markdown(&input)` (`src/note/parser.rs:106`) consumes `MarkdownParserInput` [V]. Tracking every read of `input.tasks()` and `input.frontmatter()` inside `src/note/` reveals:

| Category | Setting | Config Origin | Consumed in `note` | Parser Usage & Purpose |
|---|---|---|---|---|
| **Tasks** | `tag_filters: &[Tag]` | `TaskConfig.tag_filters` (`src/config/model/tasks.rs:26`) [V] | `src/note/parser.rs:527` [V] | Passed to `ListNestingTracker::end_item`. If non-empty, a status-marked item is only classified as a `TaskListItem` if it carries a matching tag [V: `src/note/parser/list.rs:417-422`]. |
| **Tasks** | `statuses: &TaskStatusMap` | `TaskConfig.statuses` (`src/config/model/tasks.rs:33`) [V] | `src/note/parser.rs:528` [V] | Passed to `ListNestingTracker::end_item`. Looks up the checkbox character (e.g. `' '`, `'x'`) to obtain the `TaskStatus` (`name`, `kind`) [V: `src/note/parser/list.rs:420-424`]. |
| **Frontmatter** | `tags_name: &str` | `FrontmatterConfig.tags` (`src/config/model/frontmatter.rs:55`) [V] | `src/note/parser.rs:381` [V] | `tags.extend(frontmatter.tags(self.input.frontmatter().tags_name()))`. Extracts tags from the configured YAML frontmatter key (default `"tags"`) so `Note::tags()` includes them [V: `src/note/parser.rs:381`]. |
| **Frontmatter** | `title_name` | `FrontmatterConfig.title` | **NOT READ BY PARSER** [V] | Unused during parsing. Frontmatter keys are preserved verbatim in `RawFrontmatter` [V]. |
| **Frontmatter** | `aliases_name` | `FrontmatterConfig.aliases` | **NOT READ BY PARSER** [V] | Unused during parsing [V]. |
| **Frontmatter** | `date_created` | `FrontmatterConfig.date_created` | **NOT READ BY PARSER** [V] | Unused during parsing [V]. |
| **Frontmatter** | `date_modified` | `FrontmatterConfig.date_modified` | **NOT READ BY PARSER** [V] | Unused during parsing [V]. |
| **Inline Fields** | Inline field settings | **DOES NOT EXIST IN `Config`** [V] | `src/note/parser/inline.rs:51` [V] | Syntax `[key:: value]` and `(key:: value)` is hardcoded in the lexer and parser. Zero config settings exist for inline fields in `RawConfig` or `Config` [V]. |

**Key Finding**: Out of the entire `Config` and the two sub-config structs passed into `MarkdownParserInput`, the note parser consumes **only three scalar values**:

1. `statuses: &TaskStatusMap`
2. `tag_filters: &[Tag]`
3. `tags_field_name: &str`

### 3.3 Construction and Passing Pipeline

```text
Config File (.traces/config.toml)
  │
  ▼
ConfigBuilder (src/config/builder.rs:163-230)
  ├── resolve_frontmatter -> FrontmatterConfig (tags: FieldName("tags"), ...)
  └── resolve_tasks -> TaskConfig (statuses: TaskStatusMap, tag_filters: Vec<Tag>)
  │
  ▼
Config (src/config/model.rs:44)
  │
  ▼
IndexerService::from(&Config) (src/index/service.rs:47-60)
  ├── clones tasks: TaskConfig
  └── clones frontmatter: FrontmatterConfig
  │
  ▼
IndexerService::parse_note (src/index/service.rs:340-356)
  └── constructs MarkdownParserInput::new(path, content, &self.tasks, &self.frontmatter)
  │
  ▼
parse_markdown(&input) (src/note/parser.rs:106)
  ├── line 381: tags.extend(frontmatter.tags(input.frontmatter().tags_name()))
  └── lines 527-528: list_nesting.end_item(input.tasks().tag_filters(), input.tasks().statuses())
```

### 3.4 Proposal: Inverting the Edge with `NoteParserConfig` [P]

The upward dependency `note -> config` violates hierarchical layering: the core Markdown document parser (`src/note/`) imports the high-level configuration management subsystem (`src/config/`).

**Proposal [P]**:

1. Define `NoteParserConfig` (or `NoteConfig`) in `src/note/config.rs` (or `src/note/parser/config.rs`):

   ```rust
   // In src/note/config.rs [P]
   #[derive(Clone, Debug)]
   pub struct NoteParserConfig<'a> {
       pub statuses: &'a TaskStatusMap,
       pub tag_filters: &'a [Tag],
       pub tags_field: &'a str,
   }
   ```

   Or an owned version stored on `IndexerService`:

   ```rust
   // Stored inside IndexerService [P]
   pub struct NoteConfig {
       pub statuses: TaskStatusMap,
       pub tag_filters: Vec<Tag>,
       pub tags_field: String,
   }
   ```

2. `MarkdownParserInput<'a>` borrows `&'a NoteParserConfig` (or the 3 fields directly).
3. `TaskStatusMap` lives in `task.rs` (or a leaf `task_status` module), `Tag` lives in `tag.rs`. Neither requires `config`.
4. `IndexerService` constructs `NoteParserConfig` by projecting from `&Config` in `IndexerService::from(&Config)`.
5. **Result**: `src/note/` has **zero imports from `src/config/`**. The upward dependency edge is completely removed [P].

---

## 4. (f-part) Ambient Primitives (`src/file_tracker.rs`, `src/dirs.rs`, `src/env_vars.rs`)

### 4.1 Importer Matrix and Items Used

| Source File | Public / Crate-Visible Items | Importers Outside Self (`path:line`) | Items Consumed | Consumers Outside `config`/`cli`? |
|---|---|---|---|---|
| `src/file_tracker.rs` (1,189 LoC) [V] | `FilePathTracker`, `CleanMode`, `FilePathTrackerError` [V: `src/lib.rs:148-150`] | `src/config/trust.rs:16, 165, 307, 318, 328`<br>`src/config/tracker.rs:9, 17, 72, 82, 92`<br>`src/config/error.rs:13, 87, 104`<br>`src/lib.rs:148-150` [V] | `FilePathTracker::from`<br>`FilePathTracker::for_test`<br>`CleanMode::WithCompanions`<br>`CleanMode::EntriesOnly`<br>`FilePathTrackerError` | **NO**. Exactly 0 callers outside `src/config/` [V]. |
| `src/dirs.rs` (218 LoC) [V] | `StateDirRoot`, `CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS` [V: `src/dirs.rs:25, 91, 176, 188`] | `src/config/discovery.rs:17, 46`<br>`src/config/tracker.rs:9, 92`<br>`src/config/trust.rs:16, 328`<br>`src/file_tracker.rs:20`<br>`src/lib.rs:114` [V] | `CONFIG_HOME`<br>`TRACKED_CONFIGS`<br>`TRUSTED_CONFIGS`<br>`StateDirRoot` | **NO**. Exactly 0 callers outside `src/config/` and `src/file_tracker.rs` [V]. |
| `src/env_vars.rs` (68 LoC) [V] | `CEILING_DIRS`, `DEFAULT_IGNORED_DIRS`, `IGNORED_DIRS`, `is_ignored_dir` [V: `src/env_vars.rs:8, 20, 25, 43`] | `src/config/discovery.rs:17, 45, 143, 158`<br>`src/index/service.rs:424, 745, 748`<br>`src/lib.rs:116` [V] | `CEILING_DIRS`<br>`is_ignored_dir`<br>`DEFAULT_IGNORED_DIRS` | **YES**. `src/index/service.rs:424` consumes `env_vars::is_ignored_dir` in `IndexerService::scan` [V]. |

### 4.2 Detailed Analysis of `file_tracker.rs`

- `src/file_tracker.rs` is an 1,189-line generic persistence engine that writes BLAKE3-hashed symlinks (Unix) or text files (Windows) under a state root directory [V].
- It has **only two production callers**: `src/config/tracker.rs:9` (`TrackerStore`) and `src/config/trust.rs:16` (`TrustStore`) [V].
- It is NOT used by `index`, `template`, `note`, `query`, or `schema`.
- It currently lives at `src/file_tracker.rs` in the crate root, giving the false impression that it is a general-purpose repository tracker, when in reality it is private storage plumbing for `config`.

### 4.3 Detailed Analysis of `dirs.rs`

- `src/dirs.rs` defines static `LazyLock`s resolving XDG / platform paths: `CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS` [V: `src/dirs.rs:91, 176, 188`].
- In tests, line 81 hardcodes `HOME` to `CARGO_MANIFEST_DIR/test` [V: `src/dirs.rs:81-83`].
- Consumers:
  - `src/config/discovery.rs:46`: reads `dirs::CONFIG_HOME` to locate global config file (`~/.config/traces/config.toml`).
  - `src/config/tracker.rs:92`: passes `dirs::TRACKED_CONFIGS` to `FilePathTracker::from(...)`.
  - `src/config/trust.rs:328`: passes `dirs::TRUSTED_CONFIGS` to `FilePathTracker::from(...)`.
  - `src/file_tracker.rs:20`: imports `dirs::StateDirRoot`.
- **Zero consumers outside `config` and `file_tracker`**.

### 4.4 Detailed Analysis of `env_vars.rs`

- `src/env_vars.rs` defines:
  - `CEILING_DIRS: LazyLock<Vec<PathBuf>>`: parsed from `$TRACES_CEILING_DIRS` [V: `src/env_vars.rs:8`].
  - `DEFAULT_IGNORED_DIRS: [&str; 4]`: `[".git", "target", "node_modules", ".traces"]` [V: `src/env_vars.rs:20`].
  - `IGNORED_DIRS: LazyLock<HashSet<String>>`: default ignored dirs + `$TRACES_IGNORED_DIRS` [V: `src/env_vars.rs:25`].
  - `is_ignored_dir(file_name: &OsStr) -> bool`: checks membership in `IGNORED_DIRS` [V: `src/env_vars.rs:43`].
- Consumers:
  - `src/config/discovery.rs:45, 158`: uses `CEILING_DIRS` to bound upward config search and `is_ignored_dir` to skip walking ignored directories during discovery [V].
  - `src/index/service.rs:424` [V]:

    ```rust
    let entries = DirTree::descendants(root)
        .filter(|node| crate::env_vars::is_ignored_dir(node.file_name()))
        .walk()
    ```

    `IndexerService::scan` uses `crate::env_vars::is_ignored_dir` to prune ignored directories when scanning vault notes [V].
- **Crucial Architectural Implication**: If `env_vars.rs` were re-homed into `src/config/`, `src/index/` would be forced to depend on `src/config/` just to filter directory names during filesystem indexing [H]. Therefore, `is_ignored_dir` (or directory filtering policy) must either live in a leaf filesystem utility module (e.g. `dirtree` or `path`), or be injected into `IndexerService` during construction [P].

---

## 5. Surprises and Survey Corrections

### 5.1 Surprises (Code Divergence from Ticket Text / Assumptions)

1. **Note parser ignores 4 of 5 `FrontmatterConfig` fields**:
   `FrontmatterConfig` contains `title`, `aliases`, `tags`, `date_created`, and `date_modified` [V: `src/config/model/frontmatter.rs:29-35`]. Ticket 22 mentions "frontmatter settings" crossing into the note parser. In reality, `src/note/parser.rs:381` reads **only `tags_name()`** [V]. The title, aliases, and date configurations are never read during note parsing.
2. **Inline-field settings do not exist in `Config`**:
   Ticket 22 sub-question (b) asks to check "inline-field settings". Inspection of `src/config/raw.rs` and `src/config/model.rs` reveals **zero configuration settings for inline fields** [V]. Parsing of `[key:: value]` and `(key:: value)` is hardcoded with no configuration options.
3. **`note` has zero references to `FieldName`**:
   `src/note/` never imports or uses `FieldName` [V]. `note` uses `FieldKey` (forgiving case/punctuation matching) exclusively [V: `src/note/metadata.rs:12`, `src/note/model.rs:31`]. `FieldName` is strictly a `schema` and `config` concept.
4. **`task.rs` contains extensive note-only types unreferenced by `config`**:
   `TaskDateSet`, `TaskDate`, `TaskDateType`, `TaskPriority`, and markdown emoji lookup tables (`TaskDateType::EMOJIS`, `TaskPriority::EMOJIS`) have **zero references in `src/config/`** [V]. They were hoisted from `src/note/lists.rs` to crate root solely for nominal cohesion ("task"), not because `config` required them.
5. **`env_vars::is_ignored_dir` is consumed by `index`**:
   `env_vars` is not private to config discovery. `src/index/service.rs:424` relies on it directly for vault traversal [V]. Moving `env_vars` to `config/` would introduce an unwanted `index -> config` coupling.
6. **`SchemasConfig` has zero production consumers outside `src/config/`**:
   Outside `config`, `SchemasConfig` only appears in integration-style unit tests [V: `src/index/service.rs:1771`, `src/template/engine/query.rs:622`]. Production code only queries `config.schemas().class_field_name()` or `config.schemas().directory()`.
7. **No root `GLOSSARY.md`**:
   The repo root does not contain `GLOSSARY.md` (verified: file is `src/GLOSSARY.md`, alongside subsystem glossaries in `src/note/GLOSSARY.md`, `src/config/GLOSSARY.md`, etc.) [V].

### 5.2 Status of Prior Survey Claims (`survey/root-primitives.md`, `survey/config-dialog.md`, `survey/note.md`)

- `survey/root-primitives.md`:
  - Claim: "src/lib.rs is 892 lines" -> **Moved to 897 lines** [V: `src/lib.rs:897`].
  - Claim: "19 root files (11,642 LoC)" -> **18 root files (excluding lib/main: 10,955 LoC; 11,921 LoC with lib/main)** [V].
  - Claim: "src/task.rs is 1,919 lines long" -> **Moved to 1,911 lines** (1,152 prod / 759 test) [V: `src/task.rs:1911`].
  - Claim: "src/file_tracker.rs has exactly two callers: src/config/tracker.rs and src/config/trust.rs" -> **Still holds exactly** [V].
  - Claim: "src/lib.rs:177-210: Exactly 45 frozen exports" -> **Updated to 46 identifiers** in `pub mod testing` [V: `src/lib.rs:182`].
- `survey/config-dialog.md`:
  - Claim: "ConfigDigests: 10 of 15 feed IndexFingerprint" -> **Still holds exactly** (`IndexFingerprint::new` exhaustively matches 10 and discards 5) [V: `src/index/fingerprint.rs:159-170`].
  - Claim: "src/dirs.rs:85-191 resolves platform paths" -> Line numbers moved slightly to `84-190` [V].
  - Claim: "src/env_vars.rs parses walk ceilings and directory filters" -> **Still holds**, and now additionally exports `DEFAULT_IGNORED_DIRS` (`src/env_vars.rs:20`) [V].
- `survey/note.md`:
  - Claim H4: "Upward dependency edge note -> config at src/note/parser/input.rs:9" -> **Still holds exactly** [V: `src/note/parser/input.rs:9`].

---

## 6. Open Gaps for Ticket 22 Grilling

1. **Where should `is_ignored_dir` live?**
   If `dirs.rs` and `file_tracker.rs` move into `src/config/`, `env_vars.rs` cannot simply be moved into `src/config/` without forcing `index::IndexerService` to depend on `config` [H]. It either belongs in `src/dirtree.rs`, `src/path.rs`, or as an injected predicate in `IndexerService::scan`.
2. **Boundary of `task.rs` split**:
   `TaskStatusMap`, `TaskStatus`, `TaskStatusSymbol`, `TaskStatusType` are required by `config` and `note`. But `TaskDateSet`, `TaskDate`, `TaskDateType`, `TaskPriority`, and their emoji tables belong exclusively in `note` (and `query` for projections). If `task.rs` is split, does a leaf `task_status` module remain at the root, or does `config` own status mapping while `note` owns parsing?
3. **`FieldName` vs `FieldKey` consolidation**:
   `FieldName` is used by `config` and `schema`. `FieldKey` is used by `note` and `query`. `FieldValue` is used by `schema`. `FieldValueRef` is used by `note`. Can `field.rs` be decomposed so schema exact types and note dynamic types are separated?
4. **Typestate vs Direct Pipeline in `ConfigFile`**:
   `ConfigFile` typestate markers (`Discovered`, `Trusted`, `Parsed`) span 300+ lines in `src/config/file.rs`, yet `ConfigService::build` is their only caller. The grilling session must weigh compile-time trust guarantees against single-caller boilerplate.
