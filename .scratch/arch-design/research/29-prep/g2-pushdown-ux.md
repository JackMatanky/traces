# Pushdown UX measurement for ticket 29

[H] The measured time savings suggest no visible change on vaults of 1k to 5k
notes.
It saves about 60 ms per CLI query at 50k notes. [V]
A perfect pushdown still leaves 50k at about 140 ms. [V]
A fixed per-invocation cost dominates at every size. [V]

Marks: [V] means a measurement or source read by the original author.
[H] means an inference. [P] means a proposal or an adopted target design.
The method, measurement tables and query-shape totals below are [V] unless
marked otherwise.
No new timing run was made in this copy-edit pass.

## Method and state

- **Machine:** macOS 27.2.1 arm64, 12 logical cores, hot file cache.
  Peer agents kept the load average at 4-6. The report gives medians and
  50k min-max.
- **Binary:** the shipped `target/release/traces` (Nov 9, `opt-level="z"`,
  lto, `strip`, `panic=abort`). The author did not edit the main tree.
- **The profiling build:** a separate build of HEAD `d80d0d89` with the same
  profile plus line tables. It lives in `.worktrees/pushdown-ux` with its
  target dir in `/tmp`.
  - It runs about 12% slower (230 vs 204 ms for `list`).
  - It serves for sampling only. The report uses shares, not its absolute
    times.
- **The timing method:** wall time of the whole process, with stdout and stderr
  sent
  to `/dev/null`.
  - 2 warmups, then the median of 11 timed runs (15 for floors, 9 for
    templates).
  - `HOME`, `TRACES_STATE_DIR`, `XDG_CONFIG_HOME` and `TZ=UTC` are
    isolated, with `traces trust` on each temp vault.
- **State:** Fresh (index built, no file changed). One stale probe is in
  the floor section.
- **The sample method:** macOS `sample`, 1 ms interval. A small C wrapper spawns
  the process suspended, so the sample covers the whole run. There are 6-8
  runs per variant.
- **Vaults:** synthetic and deterministic (seed 7), at 1k, 5k and 50k.
  The author added 20k to find where each curve crosses 100 ms.
  - 70 areas with 3-7 subfolders each, some nested one more level.
  - `projects/active` holds exactly 10% and `archive/2024` exactly 1%.
    Both are leaf folders, so `file.folder == "x"` equals `--from x/`.
  - 10% of notes have no frontmatter.
  - `rating` is none 20%, 1: 10%, 2: 20%, 3: 20%, 4: 20%, 4.5: 9.9%,
    5: 0.1%.
  - `status` is one of 4 values on 70% of frontmatter notes.
  - Tags number 0-4 per note over 78 tags plus nested ones. `#common` is
    on 10% and `#rare` on 1%.
  - Dates are frontmatter `created` and `due`, inline `key::` fields on
    15% of notes, wikilinks, and tasks on 40% of notes.

## Index build

| Notes | `traces index` | Store size |
|---|---|---|
| 1k | 90 ms | 1.06 MB |
| 5k | 152 ms | 4.21 MB |
| 20k | 413 ms | 16.8 MB |
| 50k | 981 ms | 33.7 MB |

## End-to-end wall time

Median ms, Fresh state.

| id | Query | Rows 1k / 5k / 50k | Sel. | 1k | 5k | 50k | 50k min–max |
|---|---|---|---|---|---|---|---|
| a | `list` | 1000 / 5000 / 50000 | 100% | 28 | 48 | 204 | 199–214 |
| b1 | `--from archive/2024/` (range pushdown) | 10 / 50 / 500 | 1% | 27 | 39 | **142** | 131–275 |
| c1 | `--where 'file.folder == "archive/2024"'` (scan) | 10 / 50 / 500 | 1% | 29 | 47 | **202** | 195–229 |
| b10 | `--from projects/active/` | 100 / 500 / 5000 | 10% | 28 | 41 | **144** | 139–161 |
| c10 | `--where 'file.folder == "projects/active"'` | 100 / 500 / 5000 | 10% | 37 | 46 | **206** | 196–228 |
| d1 | `--from '#rare'` | 10 / 50 / 500 | 1% | 34 | 38 | 132 | 125–155 |
| d1s | `--where 'contains(tags, "#rare")'` (scan) | 10 / 50 / 500 | 1% | 30 | 47 | 206 | 197–221 |
| d10 | `--from '#common'` | 100 / 500 / 5000 | 10% | 27 | 42 | 146 | 136–151 |
| d10s | `--where 'contains(tags, "#common")'` | 100 / 500 / 5000 | 10% | 31 | 48 | 205 | 194–215 |
| e1 | `rating == 5` | 1 / 5 / 50 | 0.1% | 32 | 48 | 207 | 199–225 |
| e2 | `rating >= 4.5` | 100 / 500 / 5000 | 10% | 33 | 47 | 206 | 198–220 |
| e3 | `rating == 4` | 200 / 1000 / 10000 | 20% | 32 | 46 | 202 | 195–216 |
| e4 | `rating >= 3` | 500 / 2500 / 25000 | 50% | 30 | 49 | 215 | 196–523 |
| f1 | Or: `rating == 5 or rating == 4.5` | 100 / 500 / 5000 | 10% | 31 | 49 | 199 | 190–208 |
| f2 | Or: `status == "doing" or status == "done"` | 312 / 1557 / 15678 | 31% | 32 | 47 | 212 | 199–237 |
| f3 | Not: `not (status == "done")` | 743 / 3759 / 37453 | 75% | 31 | 47 | 207 | 198–210 |
| f4 | Not: `not (rating >= 3)` | 500 / 2500 / 25000 | 50% | 31 | 46 | 207 | 197–223 |
| f5 | quoted: `rating >= "4.5"` | 0 / 0 / 0 | 0% | 30 | 47 | 204 | 194–214 |
| f6 | `status != "done"` | 743 / 3759 / 37453 | 75% | 30 | 48 | 211 | 197–219 |
| f7 | `rating != 3` | 800 / 4000 / 40000 | 80% | 31 | 48 | 207 | 201–215 |
| f8 | `status == "doing"` (quoted string) | 55 / 316 / 3131 | 6% | 31 | 46 | 207 | 198–235 |
| f9 | And: `rating >= 4.5 and status == "doing"` | 4 / 39 / 362 | 0.7% | 30 | 50 | 209 | 198–220 |
| f10 | `--from projects/active/ --where 'rating >= 4.5'` | 11 / 57 / 499 | 1% | 28 | 43 | 153 | 142–159 |
| g1 | `task` | 999 / 5134 / 49938 | ~100% | 30 | 48 | 211 | 203–223 |
| g2 | `table --column file.name --column rating` | 1000 / 5000 / 50000 | 100% | 30 | 48 | 216 | 207–270 |
| h1 | `task --count` | 999 / 5134 / 49938 | ~100% | 28 | 47 | 201 | 193–237 |
| h2 | `task --todo --count` | 473 / 2550 / 24846 | 50% | 29 | 46 | 209 | 197–218 |
| h3 | `task --todo` (list) | 473 / 2550 / 24846 | 50% | 30 | 46 | 208 | 200–257 |

- **Real-binary pushdown gain (c - b):** 2 ms at 1k (within noise), 8 ms
  at 5k, 60 ms at 50k, 26 ms at 20k. [V]
- **Tag source:** it already pushes down. The saving has the same size as
  for folders. [V]
- **Scan cost is flat:** every scan costs the same from 0.1% to 80%
  selectivity. [V]
  - Output formatting of 50k rows is negligible (`list` 204 ms vs 0-row
    scan 205 ms).
  - This is the argument for pushdown. The floor below caps the saving.
- **Quoted numeric literal:** `rating >= "4.5"` returns 0 rows because it
  is a string comparison. This matches the decided rule that quoted
  literals are `Unsupported`. [V]

## Floor and ceiling

**Wall-time floors** (ms):

| Run | 1k | 5k | 20k | 50k |
|---|---|---|---|---|
| `traces --version`, `trust --show` | 4 | 4 | 4 | 4 |
| 0-row range query (`--from zzz-none/`) = floor | 26 | 39 | 72 | 140 |
| Scan, same size | 29 | 48 | 103 | 205 |
| Perfect-pushdown saving (scan − floor) | 3 ms (about 10%) | 9 ms (19%) | 31 ms (30%) | 65 ms (32%) |

**Breakdown by phase.** Values are sampled ms per invocation.
The sampler inflates totals by about 10%:

| Phase | 1k | 5k | 50k, 0-row range | 50k, `list` | 50k, scan (0 rows) |
|---|---|---|---|---|---|
| Freshness walk and stat of all files (`refresh_store`) | 14 | 25 | **110** | 112 | 122 |
| Store close (redb commit and flush, even when nothing changed) | 10 | 11 | 11 | 11 | 12 |
| Source resolve (`paths_in_folder`) | 0.4 | 1 | 12 | 8 | 0 |
| Row decode (rayon) | 0–2 | 0–5 | 0 | 31 | 45 |
| `WorkspaceIndex::assemble` | 0–0.2 | 0–1 | 0 | 12 | 15 |
| Filter evaluation | 0.5 | 0.8 | 0 | 0 | **10** |
| Drop `WorkspaceIndex` at exit | 0.1 | 2 | 0 | 22 | 31 |

- **The floor is mostly the freshness walk.** At 50k it is about 110 of
  the 140 ms. A plain `find` over the same 50k files takes 125 ms and
  `fd` takes 60 ms. Pushdown cannot remove this. [V]
- **Filter share at 50k:** filter evaluation is about 9-10 ms of 205.
  [V]
  - In the scan-attributable bill (decode + assemble + drop + filter,
    65-90 ms) it is about 10-13%.
  - The other 87-90% is materialising and freeing rows. Pushdown can skip
    that part.
- **Stale state:** the first run after an edit of one file costs
  59 / 70 / 249 ms at 1k / 5k / 50k. That is +26 / +21 / +47 ms over
  Fresh. The run applies the delta and commits. An edit of 10 files costs
  about the same. [V]
- **The perfect-pushdown floor stays above 100 ms at 50k.** [V]
  - Scan crosses 100 ms near **19k notes** (3.7 µs per note).
  - The floor crosses near **33k**.
  - Pushdown changes the perceived category only for about 19k-33k notes.

## Template scenario

Ten queries, `traces template -i … --dry-run --no-input`. Times are in ms.

| Template | 5k | 50k |
|---|---|---|
| No query | 5 | 5 |
| 1 query (any shape, 10 shapes tried) | 49–56 | 244–268 |
| `query.from("projects/active/")` alone vs `rating >= 4.5` alone | 51 vs 52 | **254 vs 253** |
| 10 mixed queries | 55 | 290 |
| 10 copies of one query | 53 | 279 |

- **No store pushdown in templates.** The first query loads the whole
  generation. The load takes about 50 ms at 5k and 255 ms at 50k, once
  per run. [V]
  - A folder-source query costs the same as a scan. The CLI range
    pushdown does not apply on this path.
  - At 50k the load is single-threaded decode (about 43% of samples)
    plus the freshness walk (about 32%).
- **Marginal cost per extra query at 50k** (10-variant template minus
  1-query template, divided by 9): [V]

  | Shape | ms per query |
  |---|---|
  | numeric `>=` | 3.3 |
  | numeric `==` | 3.1 |
  | string `==` | 4.2 |
  | Or of two `==` | 4.1 |
  | Not | 3.3 |
  | `!=` | 3.3 |
  | `file.folder ==` scan | 3.1 |
  | folder source | 2.3 |
  | tag source | 1.0 |

  At 5k every shape costs 0.0-0.4 ms, within noise.
- **Share pushdown could remove:** 10 queries cost about 33-36 ms of
  290 ms at 50k (about 11%). At 5k they cost about 5 ms of 55 ms (about
  9%). A query that runs 10× faster saves about 30 ms at 50k (10%) and
  about 4.5 ms at 5k (8%). [V]
- **In-memory scan cost:** 3-4 ms per query measured from templates, and
  8.5-10 ms in the CLI plan stage. A long-lived host (LSP or MCP) pays
  none of the walk per query. [V]

## Query shape distribution

**Repo-authored traces filters.** These are distinct strings. They are
heavy on grammar tests and biased toward edge cases (invalid expressions
excluded).

| Feature | Unit/integration tests, n=75 | Features and templates, n=9 | Docs, `.scratch`, ADRs, n=31 |
|---|---|---|---|
| `==` | 71% | 44% | 58% |
| Range (`< <= > >=`) | 23% | 44% | 39% |
| `!=` | 4% | 0% | 0% |
| `or` / `\|\|` | 7% | 0% | 3% |
| `not` / `!` | 3% | 0% | 0% |
| `and` / `&&` | 8% | 11% | 6% |
| `contains()` | 5% | 11% | 3% |
| Quoted rhs literal | 43% | 33% | 52% |
| Bare rhs literal | 57% | 56% | 55% |
| Single comparison only | 81% | 78% | 84% |
| `file.*` field | 5% | 0% | 3% |

**Sources** (`--from` and `query.from`):

- Tests, n=55: tag 23, folder or path 22, class 14, `not` 8, `or` 5,
  `and` 6. 37 are single-atom.
- Features and templates, n=4: tag 3, `or` 2, `and` 2, `not` 2.
- Docs, n=10: tag 4, class 4, folder 2. 9 are single-atom.

**The author's real vault** (`docs/vault`, Dataview and Tasks, 104 DQL
and 12 Tasks blocks, 45 distinct queries after normalising template
variables):

| Shape | Distinct queries (of 45) | Note |
|---|---|---|
| FROM is a single plain folder | 21 | |
| FROM has a negated source (`-"00_system/05_templates"`) | 16 | |
| FROM has `or` of folders | 5 | |
| FROM has `and` with a tag | 5 | |
| WHERE is AND-only (no OR/NOT/`!=`) | 21 | |
| WHERE has `or` | 20 | only 2 of 47 OR branches are a plain field-vs-literal; the other 45 wrap `date()`, `contains()`, `regextest()` or links |
| WHERE has `!=` | 16 | |
| WHERE has `!(...)` | 4 | |
| WHERE has a range operator | 21 | mostly two-sided date windows, `date(x) >= A AND date(x) <= B` |
| WHERE has `=` | 18 | string equality |
| WHERE has `contains()` | 36 | |
| WHERE has `regextest()` | 11 | |

- **Pushable under the decided initial scope:** only 17 of the 45 have
  even one plain field-vs-literal atom. Most of those are quoted string
  equality (`file_class = "pkm_note"`). The decided rule excludes it.
- **Likely mapping:** the date windows map to bare date literals
  (`date_created >= 2026-01-01`) in traces syntax. [H]
- **All 12 Tasks-plugin blocks** are AND-only. Each has `not done`, a
  `due` range and a `path does not include` clause.

**Dataview docs examples.** The sample has 11 pages and 64 DQL blocks.
Of those blocks, 28 have FROM and 19 have WHERE:

- WHERE: 0 with `or`, 5 with `!x`, 5 with `and`, 5 with a range, 5 with
  `=`, 1 with `!=`, 3 with `contains()`.
- FROM: `#tag` 14, quoted folder 13, `or` 3, `and` 1.

**Tasks docs** (34 fenced `tasks` blocks across Queries and Getting
Started):

- `OR` 4, explicit `AND` 3, `NOT` 1.
- 18 contain a negative primitive (`not done`, `does not include`,
  `no due date`).
- The sample is small. Many docs include query snippets that are not in
  the raw files.

Sources:

- <https://blacksmithgu.github.io/obsidian-dataview/queries/data-commands/>
- <https://blacksmithgu.github.io/obsidian-dataview/resources/examples/>
- <https://raw.githubusercontent.com/obsidian-tasks-group/obsidian-tasks/main/docs/Queries/Combining%20Filters.md>
- The Dataview and Tasks `docs/` trees on GitHub.

## UX judgement

Nielsen's limits are at
<https://www.nngroup.com/articles/response-times-3-important-limits/>.
0.1 s is the limit for a system that feels instant. 1 s is the limit for
flow of thought. 10 s is the limit for keeping attention. Every measured
query is under 1 s. [V]

- **1k-5k (typical personal vaults):** every shape takes 27-50 ms with or
  without pushdown [V]. The author infers no visible change for the user [H].
- **About 19k-33k notes:** this is the only band where pushdown moves a
  one-shot CLI query from over 100 ms to under it. At 20k, scan is 103 ms
  and pushdown is 72-82 ms. [V]
- **50k:** 205 ms drops to about 143 ms. This is still over 100 ms, a
  gain of about 30% [V]. It stays outside Nielsen's "instant" category [H].
- **Or and Not pushdown matter for no plausible shape.** [H]
  - Real queries rarely use a plain Or. Tests show 7%, Dataview docs 0 of
    19, and the author's vault 2 of 47 branches.
  - Not and `!=` queries usually return 75-80% of rows (`not done`,
    `!= "done"`). A complement pushdown still materialises most rows.
  - An Or of ranges gains the same ~60 ms at 50k as any single shape.
- **Larger levers than pushdown at 50k** (observations only): the
  freshness walk (about 110 ms), the redb close commit (about 10 ms at
  every size) and the exit-time drop of `WorkspaceIndex` (22-31 ms on
  scans).

### Proposals before the final decision [P]

The table preserves the author's proposals. The final decision below excludes
quoted text equality at first.

| Shape | Size at which pushdown changes UX | Ship first or defer |
|---|---|---|
| Single `==`, `<`, `<=`, `>`, `>=` on an unquoted numeric/bool/date literal | about 19–33k; at 50k only −30% | **Ship first.** It is the base case and the other shapes build on it. |
| And-intersection of ranges (date window) and source plus filter (f10: 153 vs 209 ms) | same band | **Ship first.** It is the dominant real shape: 21 of 45 real queries have a range and 21 are AND-only. |
| Quoted text equality (`status == "doing"`, `file_class = "x"`) | same band | **Decide before the initial slice.** It is the most common equality in every corpus (43–52% of test filters have a quoted rhs; 18 of 45 real queries use `=` on strings), and the decided rule excludes it. Admitting quoted literals whose text shape is plain text would cover it. |
| Or of equalities or ranges | none below about 19k | **Defer.** About 0–7% of shapes, and branches are usually function-wrapped. |
| Not, `!=`, complement | none | **Defer.** Selectivity is about 75–80%, so there is little to skip. |
| `contains()` and `date()`-wrapped atoms | none | **Defer.** Unsupported by design, and the tag source already covers `contains(tags)`. |
| `--count` (task) | same as list, about 60 ms at 50k | Defer to the same slice as the filter it uses. |
| Template multi-query | about 10% at best at 50k | **Defer.** The first-query load dominates, and a store range pushdown does not apply because the whole generation is loaded. |

## Caveats

- **Synthetic data:** frontmatter, ratings and folder sizes are the
  author's own. Real note size, link density and the walk cost on
  non-APFS or cold-cache machines differ. Only the walk cost scales with
  file count, not with query shape.
- **Noise:** other load runs on the machine. Outliers are visible (`e4` max
  523 ms, `b1` max 275 ms). The 1k column is within a few ms of noise.
- **Shares, not exact:** phase shares come from sampling, not from
  instrumented timing.
- **Floor-table percentage:** the source report gives 8% beside 29 ms and 26 ms.
  The table now computes about 10% from those displayed rounded medians. Raw
  timing samples are unavailable. This repairs arithmetic, not a measurement.

**Not measured:** sort, limit and top-K pushdown, `@Class` sources, and
cold-cache runs.

**Leftovers:**

- Removed: `/tmp/pux`. It held the temp vaults 1k/5k/20k/50k, the
  isolated HOME, state and trust links, and the profiling build output.
  The real HOME and trust store were never touched. The main tree is
  clean.
- The remaining git worktree `.worktrees/pushdown-ux` is clean and
  detached at `d80d0d89`. `.gitignore:43` ignores it. A repo rule needs
  your approval before the author removes it. The command is
  `git worktree remove .worktrees/pushdown-ux`.

These cleanup statements record the original author's state. They are not a
current worktree audit.

## Decisions this evidence fed [P]

- The user set the first pushdown scope to exact range keys.
- The keys cover `==`, `<`, `<=`, `>` and `>=` on an unquoted numeric,
  temporal or bool single-bracket literal.
- The scope also covers And-intersection and source-plus-filter.
- The first scope has no text range table.
- Or, Not, `!=`, quoted literals, `contains()`, function-wrapped atoms and
  arithmetic scan.
- The gate: add a case only when a measured one-shot query exceeds 100 ms
  on a vault of 20,000 notes or fewer.
- Quoted text equality is excluded at first.

## Observations handed to tickets 24, 25 and 33 (not decided) [P]

- Skip the `WorkspaceIndex` drop at process exit. It costs 22-31 ms at
  50k on scans.
- Skip the redb commit when nothing changed. It costs about 10-11 ms at
  every size.
- The freshness walk is the largest lever. It costs about 110 ms at 50k.
  Plain `find` takes 125 ms and `fd` takes 60 ms.
- The template path loads the whole generation per run. The load costs
  about 50 ms at 5k and 255 ms at 50k. Its decode is single-threaded.
- Stale state after one edit costs +26 / +21 / +47 ms at 1k / 5k / 50k.

## Final review update (2026-11-16)

The maintainer approved removal of both probe worktrees. Both are removed.
The untracked `probe29` source survives at `/tmp/rt29-probe-src`.

The 100 ms gate remains necessary, not sufficient. Measure whole-process benefit
with diagnostics included. Exact scoped mismatch notices can add work that the
original floor experiment did not measure.
