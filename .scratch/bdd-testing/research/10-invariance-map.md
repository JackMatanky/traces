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
  Frozen until arch-design records a row.
- **Free**: arch ticket 16 line 75 or 83 frees the mechanism. Only the
  observable half of such a guarantee appears below.

Ticket 07 conditions 3 to 6 apply when an author writes a scenario. This map does
not apply them.

A scenario never pins a known defect (ticket 10 decision (b)):

- The closed-pipe panic. It sits under 01 G1 and G6.
- `i64` to `f64` loss at the note boundary. It sits under 03 N07 and N15.
- Multi-valued inline fields that the code reads with `.first()`. It sits under
  03 N05 and N16.

## Invariance list

### Process and surface (`research/01`)

| ID | Guarantee | Tier |
|---|---|---|
| G1 | Exit codes 0, 1, 2; never 101. Exit 130 is a class-level exception. | F |
| G2 | Stdout holds the payload only. Stderr holds diagnostics. | F |
| G3 | Count summaries go to stderr | F (channel) |
| G4 | Stable diagnostic codes with per-stage help | M |
| G7 | Non-TTY fallback values (the non-TTY half) | D |
| G11 | The template picker refuses to run non-interactively | D |
| G12 | `--no-input` swaps the provider wholesale | D |
| G13 | Output collision: refuse without a prompt when non-interactive | D |
| G14 | `--dry-run` never touches disk | D |
| G15 | Non-TTY `init` scaffolds with defaults | D |
| G16 | `init` is transactional | D |
| G17 | `init` does not grant trust | F (trust gate) |
| G18 | Nearest-local discovery, global alongside | D |
| G21 | Two-level configuration scope, merged field by field | M (config TOML) |
| G22 | Four ordered config failure stages with distinct codes | F (stages), M (codes) |
| G23 | Four trust statuses with distinct meaning | F |
| G24 | No config loads without trust, across processes | F |
| G25 | The global config layer is never trust-gated | F |
| G26 | Grant tracks before it trusts; untrust keeps the track | F |
| G27 | `clean` reports counts. The store layout is Free and not listed. | D |
| G29 | Queries re-sync the index; `index` rebuilds | F (equivalence) |
| G30 | Query output shapes are stable Markdown | F |

### Query and template (`research/02`)

| ID | Guarantee | Tier |
|---|---|---|
| G1 | Source-expression grammar: atoms, boolean operators, precedence | F+M |
| G2 | `from()` input dispatch | F+M |
| G3 | An unknown class selector gives an empty result, with no error | F+M |
| G4 | Row granularity for `pages`, `lists` and `tasks` | F |
| G5 | Filter semantics: comparisons, `contains`, temporal functions | F+M |
| G6 | Sort default direction and CLI sort flags | D (default order), M (flags) |
| G7 | Null placement | D |
| G8 | Terminal renderers: method form, pipeline form, parity | F |
| G9 | Diagnostics: spans, typo suggestions, stable CLI codes | M |
| G10 | Template resolution order; validation before search | D |
| G11 | Output-path precedence and root confinement | D |
| G12 | Write mode and prompts are independent axes; overwrite policy | D |
| G13 | CLI stream contract; index freshness round-trips | F |
| G14 | Chained and single-call queries give identical output (equivalence only) | F |
| G16 | Task and list rows keep document order (order only) | F |
| G18 | Include loader: missing include is soft, other errors are hard | D |

### Index and note (`research/03`)

| ID | Guarantee | Tier |
|---|---|---|
| P01 | A query after an edit, delete or rename shows the change | F (equivalence) |
| P02 | The query shows the delta: new tag, removed tag, rewritten task text | F |
| P03 | An added, deleted or renamed file updates every link resolution | F |
| P05 | A size or mtime change makes the index fresh (observable half) | F (equivalence) |
| P08 | A changed index-relevant setting shows in the next query | F (equivalence) |
| P20 | Inlinks: deduplicated, sorted, no empty sources | F |
| P21 | Wikilink resolution order: exact path, implied `.md`, nearest basename | F |
| P22 | Scan scope: `.git`, symlinks and the store skipped | D |
| P25 | `index` populates the tracked-config store | D |
| N01 | Parsing never fails; malformed frontmatter still queries | F |
| N02 | Frontmatter mapping semantics | F |
| N03 | Headings: ATX and setext, in document order | F |
| N04 | Lists: pre-order, depth, parent chain | F |
| N05 | Inline fields: extraction and scope | F |
| N06 | Inline-field syntax forms | F |
| N07 | Typed inline values (the visible subset) | F |
| N08 | Tags: body and list-item extraction, hierarchy | F |
| N09 | Task markers (marker cases) | F |
| N10 | Task versus checkbox through `tag_filters` | F |
| N11 | `fully_complete` subtree semantics | F |
| N12 | Task text normalization | F |
| N14 | Emoji shorthands are task-item-only | F |
| N16 | Field collection: frontmatter wins on duplicate keys | F |
| N17 | Link shape: first `#` splits path and anchor | F |

### Schema and temporal (`research/04`)

| ID | Guarantee | Tier |
|---|---|---|
| S3 | A cycle in `extends` is a hard error (one case in a broken-vault story) | M (schema TOML) |
| S4 | One malformed schema excludes only itself | M (schema TOML) |
| S7 | Structural and predicate references: the user-facing error split | M |
| T2 | Naive input uses the local zone; storage is UTC | M (coercion) |
| T4 | Equality versus shift across DST (the pair) | F |
| T6 | A month-end clamp in a compound apply (one case) | F |
| T7 | Calendar diff counts local boundaries | F |
| T8 | A Date equals the midnight DateTime in filters | M (coercion) |
| B1 | Schema to query source expansion | F |
| B2 | Schema to query and template through `class_field` | M |
| B3 | Schema to the template `schema` namespace | M |
| B4 | Temporal literals in filters | M |
| B5 | RFC3339 offsets through ingestion to a CLI filter | F |
| B6 | One calendar rule for query and template | F |
| B7 | Temporal registry functions over indexed notes | M |

### Cross-cutting (`research/05`)

| ID | Guarantee | Tier |
|---|---|---|
| G1 | Numeric exit-code contract | F |
| G2 | Stream split | F |
| G3 | Stable diagnostic codes | M |
| G7 | Root confinement of every filesystem path | D |
| G8 | One YAML config for frontmatter and template filters (the parity rule) | D |
| field | `FieldKey` matches forgivingly; `FieldName` stays exact | D |
| task | Unknown status symbol gives an incomplete task | F |
| file_tracker | `clean` removes stale entries, never live ones | D |
| dirtree | Traversal errors classified by where they occur | D |
| tag | Containment by segment prefix | F |
| env_vars | `TRACES_CEILING_DIRS` bounds upward discovery | D |

## Class-level map

These guarantees fail condition 1 or 2, or have a better owner. Each class has
one disposition. Ticket 12 uses this list as input.

| Class | Members | Disposition |
|---|---|---|
| Real-TTY and abort | 01 G5, G8, G9, G10 | Exception class (ticket 07). Unit test. |
| Exit 130 | 01 G1 (the 130 part), 01 G5 | One spawned-signal case on Unix. The unit test stays. |
| Test harness | 01 G28 | Not a product guarantee. |
| Gate-owned | 05 G4, G5, G6 | Compile-time gates and lints own them. |
| Doc convention | 05 G9 | Ticket 12 and ticket 13. |
| Never 101 | 05 G10 | A Then-clause default, not its own scenario. |
| Internal mechanism | 02 G14 (plan shape), G15, G16 (size), G17, G18 (symlink) | No scenario. |
| Store internals | 03 P04, P06, P07, P09, P10, P12, P13, P16, P18, P19, P23, P24 | Unit or integration test. |
| Free strategy | 03 P11, P14, P15, P17 | Free (arch ticket 16 line 75). |
| Parser tables | 03 N09 (chunk splits), N13, N15, N18 | Unit or proptest. |
| Pure tables | 04 S1, S2, S5, S6, S8, S9, S10, S11, S12, T1, T3, T5, T9, T10 | Unit test. |
| Root-module helpers | 05 section 3: delimiter, hash, lexer, position, strsim, `file_class_expander`, `path` codec | Unit test. |

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
| G14 Plan fusion | Scenario for equivalence, including tie order. None for plan shape. |
| G15 Memoization | No scenario. `QuerySet` is a library API. |
| G16 Row layout | Scenario for document order. None for the size budget. |
| G17 Store versus memory | No scenario. It stays an integration test. |
| G18 Include loader | Scenario for the soft and hard split. The symlink quirk stays a unit test. |

## Proptest list and facade seams (decisions (d) and (e))

- Add now: table escaping, tag containment and `NoteFieldValue` total order.
- Keep on the list: path codec and note wire format.
- Drop the schema DAG sort.
- Query-plan idempotence stays excluded (ticket 08).
- `RefreshReport`, `DriftCause`, `InlinkMap` and `load()` get no scenario. They
  stay integration-test seams outside BDD.

## Flagged contradictions

Ticket 12 decision (f) owns each disposition. These touch the invariance list:

- `src/cli/mod.rs:54` claims exit 0 for an abort. The code exits 130. (01 G1)
- ADR-0002 on trust: 01 G17 and G23 to G26.
- ADR-0009 on format version: 03 P15 (class-level) and 04 B4.
- ADR-0008 names `SourceAtom` variants that do not exist (02 C1): 02 G1.
- The slug, callout and footnote glossary claims have no code (03 section D).
  They get no scenario.
