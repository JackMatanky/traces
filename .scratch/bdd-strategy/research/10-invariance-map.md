# Research 10: Invariance map

This file serves [ticket 10](../issues/10-refactor-resilience.md), decision (a).
It lists the guarantees that pass ticket 07 conditions 1 and 2: observable
behavior, and an observable channel. Each entry cites its inventory item and its
tier in
[arch ticket 16](../../arch-design/issues/16-grilling-compatibility-envelope.md).

## How to read the tiers

- **F**: Frozen. A change is a regression (arch ticket 16, line 55).
- **M**: Migratable. A change needs a row in the redesign spec (line 56).
- **F+M**: the capability is Frozen and its spelling is Migratable. Arch ticket
  16 lines 84 and 85 give this split for query and template guarantees.
- **D**: arch ticket 16 does not list the surface. Its line 67 says that an
  observable change absent from the change table is a regression. Treat D as
  Frozen.
- **Free**: arch ticket 16 line 75 or 83 frees the mechanism. No Free item
  appears in the list. The class-level map holds them.

Ticket 07 conditions 3 to 6 apply when an author writes a scenario. This map does
not apply them.

A scenario never pins a known defect (ticket 10 decision (b)):

- The closed-pipe panic. It sits under 01-G1 and 01-G6. G6 is class-level.
- `i64` to `f64` loss at the note boundary. It sits under 03-N07 and 03-N15. N15 is
  class-level.
- Multi-valued inline fields that the code reads with `.first()`. It sits under
  03-N05 and 03-N16.

## Invariance list

### Process and surface (`research/01`)

| ID | Guarantee | Tier | Status |
|---|---|---|---|
| 01-G1 | Exit codes 0, 1, 2; never 101. Exit 130 is a class-level exception. | F | covered: Unknown subcommand exits with code two and prints usage (reviewed 2026-11-03); the 130 part reads `pinned: src/main.rs::ctrl_c_abort_exits_130` (slice 8 dropped, decided 2026-11-03) |
| 01-G2 | Stdout holds the payload only. Stderr holds diagnostics. | F | covered: Cleaning trust store exits with code zero and writes status to stderr (reviewed 2026-11-03) |
| 01-G3 | Count summaries go to stderr | F (channel) | covered: Task list writes count summary to stderr (reviewed 2026-11-03) |
| 01-G4 | Stable diagnostic codes with per-stage help | M | covered: Missing command exits with code one and reports diagnostic code (reviewed 2026-11-03) |
| 01-G7 | Non-TTY fallback values (the non-TTY half) | D | covered: Non-TTY template prompts use their defined fallback answers (reviewed 2026-11-03) |
| 01-G11 | The template picker refuses to run non-interactively | D | covered: The template picker refuses to run without an interactive terminal (reviewed 2026-11-03) |
| 01-G12 | `--no-input` swaps the provider wholesale | D | pinned: src/cli/template.rs::tests::no_input_ignores_the_injected_provider_and_uses_defaults (the non-TTY providers produce identical answers) |
| 01-G13 | Output collision: refuse without a prompt when non-interactive | D | covered: A non-interactive output collision preserves the existing file (reviewed 2026-11-03) |
| 01-G14 | `--dry-run` never touches disk | D | covered: Dry-run previews rendered content without writing a file (reviewed 2026-11-03) |
| 01-G15 | Non-TTY `init` scaffolds with defaults | D | covered: Non-interactive initialization scaffolds the default configuration (reviewed 2026-11-03) |
| 01-G16 | `init` is transactional | D | pinned: src/cli/init.rs::tests::run::cleans_up_after_a_mid_run_failure (the non-TTY CLI cannot induce a portable mid-run failure) |
| 01-G17 | `init` does not grant trust | F (trust gate) | covered: Init does not grant trust (reviewed 2026-11-03) |
| 01-G18 | Nearest-local discovery, global alongside | D | covered: Nearest local configuration is discovered alongside global configuration (reviewed 2026-11-03) |
| 01-G21 | Two-level configuration scope, merged field by field | M (config TOML) | covered: Local and global configuration fields combine independently (reviewed 2026-11-03) |
| 01-G22 | Four ordered config failure stages with distinct codes | F (stages), M (codes) | covered: Changed trusted configuration fails before its malformed content is parsed (reviewed 2026-11-03) |
| 01-G23 | Four trust statuses with distinct meaning | F | covered: Untrust revokes trust for later runs (reviewed 2026-11-03) |
| 01-G24 | No config loads without trust, across processes | F | covered: Untrust revokes trust for later runs (reviewed 2026-11-03) |
| 01-G25 | The global config layer is never trust-gated | F | covered: A malformed global configuration fails without global trust (reviewed 2026-11-03) |
| 01-G26 | Grant tracks before it trusts; untrust keeps the track | F | covered: Trust grant fails before trusting when tracking cannot save (reviewed 2026-11-03) |
| 01-G27 | `clean` reports counts. The store layout is Free and not listed. | D | covered: Cleaning a stale tracked path reports its removal (reviewed 2026-11-03) |
| 01-G29 | Queries re-sync the index; `index` rebuilds | F (equivalence) | covered: An edited task appears in the next query (reviewed 2026-11-03) |
| 01-G30 | Query output shapes are stable Markdown | F | covered: A page table renders one Markdown row per note (reviewed 2026-11-03) |

### Query and template (`research/02`)

| ID | Guarantee | Tier | Status |
|---|---|---|---|
| 02-G1 | Source-expression grammar: atoms, boolean operators, precedence | F+M | covered: Source expressions respect not and and before or (reviewed 2026-11-03) |
| 02-G2 | `from()` input dispatch | F+M | covered: A bare Markdown path selects only tasks from that note (reviewed 2026-11-03) |
| 02-G3 | An unknown class selector gives an empty result, with no error | F+M | covered: An unknown class selector returns no pages (reviewed 2026-11-03) |
| 02-G4 | Row granularity for `pages`, `lists` and `tasks` | F | covered: Tasks exclude plain list items from their row count (reviewed 2026-11-03) |
| 02-G5 | Filter semantics: comparisons, `contains`, temporal functions | F+M | covered: A date function filters notes by their computed due date (reviewed 2026-11-03) |
| 02-G6 | Sort default direction and CLI sort flags | D (default order), M (flags) | covered: An explicit ascending sort places missing ratings first (reviewed 2026-11-03) |
| 02-G7 | Null placement | D | covered: Default descending sort places missing ratings last (reviewed 2026-11-03) |
| 02-G8 | Terminal renderers: method form, pipeline form, parity | F | covered: A template query renders method and pipeline tables alike (reviewed 2026-11-03) |
| 02-G9 | Diagnostics: spans, typo suggestions, stable CLI codes | M | covered: Unknown sort fields report a stable diagnostic code (reviewed 2026-11-03) |
| 02-G10 | Template resolution order; validation before search | D | covered: An unsafe template identifier fails before lookup (reviewed 2026-11-03) |
| 02-G11 | Output-path precedence and root confinement | D | covered: A template output path cannot escape the project root (reviewed 2026-11-03) |
| 02-G12 | Write mode and prompts are independent axes; overwrite policy | D | covered: Dry-run previews over an existing file without replacing it (reviewed 2026-11-03) |
| 02-G13 | CLI stream contract; index freshness round-trips | F | covered: A rendered note appears in the next list query (reviewed 2026-11-03) |
| 02-G14 | Chained and single-call queries give identical output (equivalence only) | F | covered: Chained sort and limit preserve the order of equal-rated pages (reviewed 2026-11-03) |
| 02-G16 | Task and list rows keep document order (order only) | F | covered: List rows include plain items and checkboxes in document order (reviewed 2026-11-03) |
| 02-G18 | Include loader: missing include is soft, other errors are hard | D | covered: An unsafe include remains an error when marked ignore missing (reviewed 2026-11-03) |

### Index and note (`research/03`)

| ID | Guarantee | Tier | Status |
|---|---|---|---|
| 03-P01 | A query after an edit, delete or rename shows the change | F (equivalence) | covered: Renaming a note removes the former path and resolves its link (reviewed 2026-11-03) |
| 03-P02 | The query shows the delta: new tag, removed tag, rewritten task text | F | covered: An edited task appears in the next query (reviewed 2026-11-03) |
| 03-P03 | An added, deleted or renamed file updates every link resolution | F | covered: Adding a link target resolves its inlink on the next query (reviewed 2026-11-03) |
| 03-P05 | A size or mtime change makes the index fresh (observable half) | F (equivalence) | covered: An edited task appears in the next query (reviewed 2026-11-03) |
| 03-P08 | A changed index-relevant setting shows in the next query | F (equivalence) | covered: An index setting edit changes the next query without changing notes (reviewed 2026-11-03) |
| 03-P20 | Inlinks: deduplicated, sorted, no empty sources | F | covered: Inlinks are sorted and deduplicated with empty sources omitted (reviewed 2026-11-03) |
| 03-P21 | Wikilink resolution order: exact path, implied `.md`, nearest basename | F | covered: Exact and implied paths resolve before a nearby basename (reviewed 2026-11-03) |
| 03-P22 | Scan scope: `.git`, symlinks and the store skipped | D | covered: Hidden Git files and symlink notes are absent from queries (reviewed 2026-11-03) |
| 03-P25 | `index` populates the tracked-config store | D | covered: Indexing records the project configuration as tracked (reviewed 2026-11-03) |
| 03-N01 | Parsing never fails; malformed frontmatter still queries | F | gap |
| 03-N02 | Frontmatter mapping semantics | F | gap |
| 03-N03 | Headings: ATX and setext, in document order | F | gap |
| 03-N04 | Lists: pre-order, depth, parent chain | F | gap |
| 03-N05 | Inline fields: extraction and scope | F | gap |
| 03-N06 | Inline-field syntax forms | F | gap |
| 03-N07 | Typed inline values (the visible subset) | F | gap |
| 03-N08 | Tags: body and list-item extraction, hierarchy | F | gap |
| 03-N09 | Task markers (marker cases) | F | gap |
| 03-N10 | Task versus checkbox through `tag_filters` | F | gap |
| 03-N11 | `fully_complete` subtree semantics | F | gap |
| 03-N12 | Task text normalization | F | gap |
| 03-N14 | Emoji shorthands are task-item-only | F | gap |
| 03-N16 | Field collection: frontmatter wins on duplicate keys | F | gap |
| 03-N17 | Link shape: first `#` splits path and anchor | F | gap |

### Schema and temporal (`research/04`)

| ID | Guarantee | Tier | Status |
|---|---|---|---|
| 04-S3 | A cycle in `extends` is a hard error (one case in a broken-vault story) | M (schema TOML) | covered: A cycle in file class inheritance stops selection (reviewed 2026-11-03) |
| 04-S4 | One malformed schema excludes only itself | M (schema TOML) | covered: An invalid file class does not block a valid file class (reviewed 2026-11-03) |
| 04-S7 | Structural and predicate references: the user-facing error split | M | covered: An unknown schema in a template fails with a diagnostic (reviewed 2026-11-03) |
| 04-T2 | Naive input uses the local zone; storage is UTC | M (coercion) | covered: An offset-dated note matches the same instant to a nanosecond (reviewed 2026-11-03) |
| 04-T4 | Equality versus shift across DST (the pair) | F | covered: Equal durations shift differently over the spring clock change (reviewed 2026-11-03) |
| 04-T6 | A month-end clamp in a compound apply (one case) | F | covered: Adding one month to January's final day clamps to February's final day (reviewed 2026-11-03) |
| 04-T7 | Calendar diff counts local boundaries | F | covered: Calendar differences count the local day and elapsed hours separately (reviewed 2026-11-03) |
| 04-T8 | A Date equals the midnight DateTime in filters | M (coercion) | covered: A date equals the midnight clock value in a filter (reviewed 2026-11-03) |
| 04-B1 | Schema to query source expansion | F | covered: A descendant file class selects notes from later generations (reviewed 2026-11-03) |
| 04-B2 | Schema to query and template through `class_field` | M | covered: A configured file class field selects the matching note (reviewed 2026-11-03) |
| 04-B3 | Schema to the template `schema` namespace | M | covered: A template reads the labels and values of a file-sourced select field (reviewed 2026-11-03) |
| 04-B4 | Temporal literals in filters | M | covered: A bare date filter selects only the due note (reviewed 2026-11-03) |
| 04-B5 | RFC3339 offsets through ingestion to a CLI filter | F | covered: An offset-dated note matches the same instant to a nanosecond (reviewed 2026-11-03) |
| 04-B6 | One calendar rule for query and template | F | covered: A query and a template agree on the same month-end clamp (reviewed 2026-11-03) |
| 04-B7 | Temporal registry functions over indexed notes | M | covered: Calendar functions filter notes from indexed dates (reviewed 2026-11-03) |

### Cross-cutting (`research/05`)

| ID | Guarantee | Tier | Status |
|---|---|---|---|
| 05-G1 | Numeric exit-code contract | F | covered: Unknown subcommand exits with code two and prints usage (reviewed 2026-11-03) |
| 05-G2 | Stream split | F | covered: Listing templates exits with code zero and writes payload to stdout (reviewed 2026-11-03) |
| 05-G3 | Stable diagnostic codes | M | covered: Missing project configuration exits with code one and reports discovery diagnostic (reviewed 2026-11-03) |
| 05-G7 | Root confinement of every filesystem path | D | gap |
| 05-G8 | One YAML config for frontmatter and template filters (the parity rule) | D | gap |
| 05-field | `FieldKey` matches forgivingly; `FieldName` stays exact | D | gap |
| 05-task | Unknown status symbol gives an incomplete task | F | gap |
| 05-file_tracker | `clean` removes stale entries, never live ones | D | covered: Cleaning preserves a live tracked configuration path (reviewed 2026-11-03) |
| 05-dirtree | Traversal errors classified by where they occur | D | pinned: src/dirtree.rs::tests (no portable CLI channel distinguishes root versus node errors) |
| 05-tag | Containment by segment prefix | F | gap |
| 05-env_vars | `TRACES_CEILING_DIRS` bounds upward discovery | D | covered: A discovery ceiling stops the upward configuration search (reviewed 2026-11-03) |

## Class-level map

These guarantees fail condition 1 or 2, or have a better owner. Each class has
one disposition. Ticket 12 uses this list as input.

| Class | Members | Disposition |
|---|---|---|
| Real-TTY and abort | 01-G5, 01-G8, 01-G9, 01-G10 | Exception class (ticket 07). Unit test. |
| Exit 130 | 01-G1 (the 130 part), 01-G5 | The unit test stays. Slice 8 was dropped (2026-11-03): numeric 130 needs a real-TTY prompt and the scenario would need a PTY harness, which spec section 4 bans. Known limitation; the pin is `pinned: src/main.rs::ctrl_c_abort_exits_130`. |
| Test harness | 01-G28 | Not a product guarantee. |
| Gate-owned | 05-G4, 05-G5, 05-G6 | Compile-time gates and lints own them. |
| Doc convention | 05-G9 | Ticket 12 and ticket 13. |
| Never 101 | 05-G10 | A Then-clause default, not its own scenario. |
| Internal mechanism | 02-G14 (plan shape), 02-G15, 02-G16 (size), 02-G17, 02-G18 (symlink) | No scenario. |
| Store internals | 03-P04, 03-P06, 03-P07, 03-P09, 03-P10, 03-P12, 03-P13, 03-P16, 03-P18, 03-P19, 03-P23, 03-P24 | Unit or integration test. |
| Free strategy | 03-P11, 03-P14, 03-P15, 03-P17 | Free (arch ticket 16 line 75). |
| Parser tables | 03-N09 (chunk splits), 03-N13, 03-N15, 03-N18 | Unit or proptest. |
| Pure tables | 04-S1, 04-S2, 04-S5, 04-S6, 04-S8, 04-S9, 04-S10, 04-S11, 04-S12, 04-T1, 04-T3, 04-T5, 04-T9, 04-T10 | Unit test. |
| Root-module helpers | 05: delimiter, hash, lexer, position, strsim, `file_class_expander`, `path` codec | Unit test. |

## Seam dispositions (decision (c))

Each seam gets a scenario at the higher boundary that it feeds, or no scenario.
Unit tests stay as they are. Items refer to `research/03` section C, and G14 to
G18 refer to `research/02`.

| Seam | Disposition |
|---|---|
| 1 redb error taxonomy | No scenario. Byte-level store surgery is an exception class. |
| 2 `META_VERSION` wipe | Scenario: results stay identical after the index file is deleted. No scenario for the wipe. |
| 3 Fingerprint source scan | No scenario. It is a `syn` scan. |
| 4 Secondary tables and pushdown | Scenario: bare and prefixed tags give the same results. Frontmatter-only tags are found. |
| 5 Whole-`Note` postcard persistence | Scenario: a second run from the persisted index equals the first. |
| 6 Debug-only sort witness | No scenario. |
| 7 Same-size mtime canary | No scenario. Ticket 07 sends timing to unit tests, tagged as a known limitation. |
| 8 `Note` model shapes | Scenario: query output for frontmatter-wins and mirrored tags. |
| 9 Parser mechanism tests | No scenario. Scenarios address the output through query results. |
| 10 YAML crate behavior | Scenario only for outcomes with intent evidence (ticket 07 condition 3). |
| 11 `traces::testing` facade | No scenario. See the facade seams below. |
| 12 Disk-is-truth refresh | Keep the scenarios. The CLI has no editor buffers. |
| 02-G14 Plan fusion | Scenario for equivalence, including tie order. None for plan shape. |
| 02-G15 Memoization | No scenario. `QuerySet` is a library API. |
| 02-G16 Row layout | Scenario for document order. None for the size budget. |
| 02-G17 Store versus memory | No scenario. It stays an integration test. |
| 02-G18 Include loader | Scenario for the soft and hard split. The symlink quirk stays a unit test. |

## Proptest list and facade seams (decisions (d) and (e))

- Add now: table escaping, tag containment and `NoteFieldValue` total order.
- Keep on the list: path codec and note wire format.
- Drop the schema DAG sort.
- Query-plan idempotence stays excluded (ticket 08).
- `RefreshReport`, `DriftCause`, `InlinkMap` and `load()` get no scenario. They
  stay integration-test seams outside BDD.

## Flagged contradictions

Ticket 12 decision (f) owns each disposition. These touch the invariance list:

- `src/cli/mod.rs:54` claims exit 0 for an abort. The code exits 130. (01-G1)
- ADR-0002 on trust: 01-G17 and 01-G23 to 01-G26.
- ADR-0009 on format version: 03-P15 (class-level) and 04-B4.
- ADR-0008 names `SourceAtom` variants that do not exist (02 C1): 02-G1.
- The slug, callout and footnote glossary claims have no code (03 section D).
  They get no scenario.
