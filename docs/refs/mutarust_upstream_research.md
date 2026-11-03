# mutarust upstream research: verification of `mutarust.yml`

Research only; no existing file was modified. Every factual claim below was
checked against primary sources fetched live on **2026-11-02** from
`quality-gates/mutarust` branch `main` (README, `docs/`, `schema/`, and the
Rust source that parses and validates configuration). Local context files
(`docs/refs/quality_gates_mutarust.md`, `docs/refs/mutarust/*`,
`docs/refs/mutation_testing.md`) were treated as prior snapshots and
re-verified, not trusted.

Repo policy constraints from ticket 12 are treated as decided inputs and are
not relitigated: `min_msi` stays declined as a gate, `min_covered_msi` stays
unset, no multi-hour full-project runs, and the committed baseline absorbs the
noisier mutants while `exclude_dirs` ports the old cargo-mutants
`exclude_globs`.

---

## 1. Option inventory

Upstream supports exactly **eleven** YAML keys. This is confirmed three ways:
the published schema (`schema/mutarust.schema.json`, `additionalProperties:
false`), the deserialization struct (`src/configuration.rs:134-148`,
`deny_unknown_fields`), and the config guide's field table
(`docs/config.md`, "Policy Fields"). There is no twelfth key, no deprecated
key, and no renamed key. The local `mutarust.yml` uses only known keys, so it
would parse and validate without an unknown-field error.

| Key | Type | Upstream default | Local `mutarust.yml` |
| --- | --- | --- | --- |
| `skip_without_test` | bool | `false` | set `false` (explicit, equals default) |
| `skip_with_cfg` | bool | `false` | set `false` (explicit, equals default) |
| `json_output` | bool | `false` | set `true` |
| `html_output` | bool | `false` | set `true` |
| `silent_mode` | bool | `false` | set `false` (explicit, equals default) |
| `min_msi` | int 0 to 100 | absent = no gate | commented out (policy) |
| `min_covered_msi` | int 0 to 100 | absent = no gate | unset (policy) |
| `exclude_dirs` | list of strings | `[]` | `["src/cli"]` |
| `disable_mutators` | list of patterns | `[]` | `["select/*"]` |
| `enable_mutators` | list of patterns | `[]` | `[]` |
| `ignore_source_lines` | list of regexes | `[]` | `[]` |

Sources: schema defaults, https://raw.githubusercontent.com/quality-gates/mutarust/main/schema/mutarust.schema.json ; struct defaults (`unwrap_or_default`, scores `Option`), `src/configuration.rs:78-94, 134-148`; "All fields are optional. A Boolean field defaults to `false`." in https://raw.githubusercontent.com/quality-gates/mutarust/main/docs/config.md ("Policy Fields").

Notes on the inventory:

- The two unset keys (`min_msi`, `min_covered_msi`) are the only keys whose
  absence is behavioral (no score gate). They have no numeric default; the
  upstream example file sets both to 80, and `docs/ci.md` offers tier
  suggestions, but the tool's own default is "no gate".
- Everything else local either sets a non-default value (`json_output`,
  `html_output`, `exclude_dirs`, `disable_mutators`) or restates the default
  explicitly (`silent_mode`, `skip_*`, `enable_mutators`,
  `ignore_source_lines`). The explicit restatements are documentation, not
  configuration debt.
- Baseline/blacklist/match/coverage/git-diff/workers/timeouts/loggers are
  **command-only** options with no YAML key (`docs/config.md` "Command
  Priority", `docs/cli.md`). The repo's mise task supplies them on the
  command line (`docs/refs/mutation_testing.md` §3), which is correct.
- Unknown keys fail fast (`src/configuration.rs:135` `deny_unknown_fields`;
  `docs/config.md` "Command Priority"), so there is no silent-key risk.

---

## 2. Optimality verdict (set vs omit, per unset key)

**`min_msi`: OMIT. Optimal.** Ticket 12 declined it as a gate
(`.scratch/bdd-strategy/issues/12-coverage-gap-methodology.md`, decision (e):
"No universal MSI threshold"), and the local comment now records the decline.
Upstream treats an absent `min_msi` as no gate (`src/main.rs:964-967`), so
omitting it is exactly the declined-gate posture. The commented-out
`# min_msi: 60` is documentation of history only and matches ci.md's "Active
work" tier (60), so it does not mislead.

**`min_covered_msi`: OMIT. Optimal, and required.** A positive
`min_covered_msi` without `--coverage` exits 4 (`src/main.rs:1020-1029`,
`docs/config.md`, `docs/cli.md`), and neither the mise task nor any documented
local run passes `--coverage`. Setting it would break every run. Note the
fine print: `min_covered_msi: 0` would *not* exit 4 (the `minimum == 0` case
returns success at `src/main.rs:1024-1026`), but a zero gate is meaningless
anyway; leave it unset.

**No value changes recommended.** All set values are correct against upstream:

- `exclude_dirs: [src/cli]` faithfully ports the deleted
  `.cargo/mutants.toml` glob `src/cli/**/*.rs` (component-wise prefix
  matching; `src/cli` does not match a hypothetical `src/cli_extra`,
  `src/filter.rs:80-91` plus its test `directory_prefixes_match_path_components`
  at `src/filter.rs:931-953`).
- `disable_mutators: ["select/*"]` is valid syntax and matches two real
  built-ins (see §3(d)); keeping it is harmless even though it is currently a
  no-op (see §3(d) caveat).
- `json_output`/`html_output: true` are intentional and gitignored
  (`.gitignore:29-33`).
- `skip_without_test: false` / `skip_with_cfg: false` are the safe values for
  this repo (see §3(e)); the comment correctly warns against flipping them.

**Optional (non-required) cleanup, values only:** `silent_mode: false`,
`skip_without_test: false`, `skip_with_cfg: false`, `enable_mutators: []`,
and `ignore_source_lines: []` all restate upstream defaults. They could be
deleted with zero behavior change, but they are self-documenting and their
comment blocks explain *why* they are false; keeping them is defensible.
Nothing in the file must change to be optimal.

---

## 3. Comment accuracy

Each block is quoted from `mutarust.yml`, then each factual claim is marked
TRUE / FALSE / STALE with its citation.

### Block 1: lines 8-11

> `# Reports land in CWD (all gitignored): report.json (triage) and`
> `# mutarust-report.html (human). mutarust-agentic.json comes from the task's`
> `# --logger-agentic-json. NOTE: report.json is clobbered by every run,`
> `# including --dry-run — read stats immediately after a scored run.`

- "Reports land in CWD": **TRUE**. `docs/cli.md` (Reports): "writes the
  full report to `report.json` in the current directory"; HTML likewise.
  Filename constant `FULL_REPORT_FILE_NAME = "report.json"` at
  `src/report/mod.rs:26`.
- "(all gitignored)": **TRUE** for this repo: `.gitignore:29-33` lists
  `/report.json`, `/mutarust-agentic.json`, `/mutarust-summary.json`,
  `/mutarust-gitlab.json`, `/mutarust-report.html`.
- "mutarust-agentic.json comes from the task's `--logger-agentic-json`" :
  **TRUE**. `docs/cli.md` (Reports): "`--logger-agentic-json` to write the
  agent-ready escaped-mutant report to `mutarust-agentic.json`"; the task
  passes it (`docs/refs/mutation_testing.md:73`).
- "report.json is clobbered by every run, including `--dry-run`": **TRUE**
  for the dry-run claim, with one documented exception. The dry-run path
  calls `write_reports_or_error` (`src/main.rs:755-757`), which writes
  `report.json` whenever `json_output` is true (`src/main.rs:799-803`);
  `--no-exec` and normal runs do the same (`src/main.rs:759-770`). The
  exception: an `--update-baseline` run returns before report writing
  (`src/main.rs:752-754`; `docs/cli.md`: "A baseline update does not write
  these reports"). So "every run" is mildly overbroad; "including
  `--dry-run`", the part that matters, is exactly right, and the upstream
  dry-run stdout message "No files written, no tests run"
  (`src/main.rs:852-859`) is misleading about reports. The repo's own
  `docs/refs/mutation_testing.md:140-142` states the same fact more precisely.
- "read stats immediately after a scored run": sound advice; consistent with
  `docs/refs/quality_gates_mise_adoption.md:937`, which did exactly that.

### Block 2: lines 16-21

> `# Port of .cargo/mutants.toml exclude_globs "src/cli/**" (path prefix).`
> `# Accepted gaps (no file-level exclude exists): src/main.rs and the 8 non-cli`
> `# src/**/error.rs files ARE mutated, and exclude_re has no port, so impl-body`
> `# mutants appear that cargo-mutants skipped (noisier than the old baseline).`
> `# Absorbed by the committed baseline; revisit with `// mutator-disable-func``
> `# annotations only if escape noise demands it.`

- "Port of `.cargo/mutants.toml` `exclude_globs "src/cli/**"`": **TRUE**
  with a tiny imprecision: the deleted config (last content at commit
  `ffb725d6^`) actually listed `"src/cli/**/*.rs"` (alongside `src/main.rs`
  and `src/**/error.rs`). The quoted `src/cli/**` is an abbreviation of that
  glob; semantics for the port are equivalent. Path-prefix semantics are the
  documented contract (`docs/config.md`, Source Selection: "removes source
  candidates below each path prefix", relative to the Cargo workspace root)
  and the implemented behavior (`src/filter.rs:81-91`,
  `src/execution.rs:1929`).
- "no file-level exclude exists": **TRUE for the documented API** (no
  `exclude_files`, no globs, no `exclude_re`; schema has only the eleven keys)
  **but FALSE in implementation**: `exclude_dirs` entries are matched with
  `Path::starts_with` (`src/filter.rs:88-89`), and `starts_with` returns true
  for an *equal* path, so `exclude_dirs: ["src/main.rs"]` would actually
  exclude that file. This is undocumented (`docs/config.md` promises only
  directory prefixes) and should not be relied on without an upstream doc
  change, but the categorical "no file-level exclude exists" is not literally
  true of the binary. This is the one claim in the file that is wrong in the
  direction that matters (it understates available levers).
- "the **8** non-cli `src/**/error.rs` files ARE mutated": **STALE**. The
  count was correct when written (comment introduced 2026-09-27, commit
  `3c8a6b7a`), but `src/temporal/error.rs` was added 2026-10-12 (commit
  `0da0f1d0`). There are now **9** non-cli `error.rs` files
  (`src/{config,dialog,index,query,schema,fields,template,engine,temporal}/error.rs`;
  `src/temporal` is a real module, `src/lib.rs:133`). The substantive claim,
  that they are all mutated (only `src/cli` is excluded), remains **TRUE**.
- "src/main.rs ... IS mutated": **TRUE**. Discovery includes declared bin
  sources (`docs/cli.md`, Source Listing); `src/main.rs` exists and is not
  under `src/cli`.
- "`exclude_re` has no port": **TRUE**. No `exclude_re` key in schema,
  struct, or `docs/config.md`; nearest approximations are
  `ignore_source_lines` (line regexes) and in-source annotations.
- "impl-body mutants appear that cargo-mutants skipped": **TRUE**. The
  deleted `.cargo/mutants.toml` carried `exclude_re = ["fn main",
  "impl.*Debug", "impl.*Display", …]` under the comment "Exclude boilerplate
  trait implementations"; mutarust has no equivalent, so those impl bodies now
  mutate.
- "Absorbed by the committed baseline": **TRUE**. `mutarust-baseline.json`
  exists and was committed 2026-09-28 (`8a2b3ba7`).
- "revisit with `// mutator-disable-func` annotations": **TRUE**. The
  annotation is documented in `docs/config.md` (Source Annotations) with
  `mutator-disable-func`, `mutator-disable-next-line`, and
  `mutator-disable-regexp` forms; invalid annotations exit 3.

### Block 3: lines 25-27

> `# Zero-yield operator group for this repo (no tokio-style select! sites in`
> `# src/; see quality_gates_mutarust.md §9.17) — drops mutants that only burn`
> `# build time.`

- "no tokio-style `select!` sites in `src/`": **TRUE**. Zero occurrences of
  `select!` anywhere in `src/` or `tests/` (verified by repo-wide search; the
  563 hits for the bare word "select" are English prose and test names like
  `selects_the_named_weekday…`). The two select mutators only mutate
  `tokio::select!` / `::tokio::select!` forms (`docs/mutators.md`,
  `concurrency/goroutine-remove` paragraph and the `select/case-remove` /
  `select/default-remove` rows).
- "see quality_gates_mutarust.md §9.17": **TRUE**; the section exists
  (`docs/refs/quality_gates_mutarust.md:1004`) and its table row reads
  "per-mutator stats + `enable/disable_mutators`: drop zero-yield `select/*`".
- "drops mutants that only burn build time": **FALSE as a current-effect
  claim** (harmless, and the parenthetical explains why). With zero `select!`
  sites, `select/case-remove` and `select/default-remove` generate zero
  candidates, so the disable entry drops *nothing* today: there are no
  mutants to burn build time. The entry is defensive (it pre-emptively
  silences the group if `select!` code appears later), and it is valid
  configuration, but the sentence describes an effect that does not occur.

### Block 4: lines 31-34

> `# Do NOT enable without a --dry-run A/B: this repo gates many items with`
> `# #[cfg(any(test, feature = "test-utils"))] and the docs never classify`
> `# `any(test, …)` (§9.3) — enabling could suppress active test-support`
> `# mutants.`

- "this repo gates many items with `#[cfg(any(test, feature = "test-utils"))]`"
 : **TRUE**. 41 occurrences across ~20 files in `src/` (40 exact
  `#[cfg(any(test, feature = "test-utils"))]` plus one macro-string variant).
- "the docs never classify `any(test, …)` (§9.3)": **TRUE**. A search for
  `any(test` across every upstream doc (`README.md`, `config.md`, `cli.md`,
  `ci.md`, `mutators.md`, `glossary.md`, `quickstart.md`, `json-outputs.md`,
  `parity.md`, `custom-mutators.md`, `selective-ci.md`) returns zero hits.
  `docs/config.md` only says "`#[cfg(test)]` does not trigger this policy."
  §9.3 of the local research note exists
  (`docs/refs/quality_gates_mutarust.md:672`) and records the same ambiguity.
- "enabling could suppress active test-support mutants": **TRUE**, and the
  source resolves the ambiguity the docs leave: `cfg_path_is_test` accepts
  only the exact token list `test` (`src/filter.rs:448-453`), so
  `#[cfg(any(test, feature = "test-utils"))]` is classified as a **non-test**
  cfg (`src/filter.rs:444-446`) and with `skip_with_cfg: true` every mutant
  whose changed range overlaps such an item is skipped
  (`src/filter.rs:387`, `src/filter.rs:218`). Because the task always runs
  with `--features test-utils` (`docs/refs/mutation_testing.md:73-75`), those
  items are live production code, so the suppression would remove real
  mutants. The comment's "could" is, if anything, an understatement.

### Block 5: lines 38-46

> `# Score gate: declined as a gate (ticket 12). Do not enable min_msi without`
> `# a new recorded decision. Measurement stays optional; the full-project run`
> `# stays deferred under the no-multi-hour-runs constraint.`
> `# min_msi unit: integer percent 0-100 (config.md; cli.md `--min-msi PERCENT`).`
> `# The ci.md companion 75 is min_covered_msi, which we do NOT set: without`
> `# --coverage a positive min_covered_msi exits 4.`
> `# Historical scoped measurement (evidence, not a target): MSI =`
> `#   0.8333333333333334 on strsim, 2026-09-28.`
> `# min_msi: 60`

- "declined as a gate (ticket 12)": **TRUE**. Ticket 12 decision (e):
  "No universal MSI threshold. No primary source supports one."
  (`.scratch/bdd-strategy/issues/12-coverage-gap-methodology.md`); the ticket
  also records "mutarust.yml: the min_msi comment now records the decline."
- "min_msi unit: integer percent 0-100 (config.md; cli.md `--min-msi PERCENT`)"
 : **TRUE**. `docs/config.md` field table: "Integer, 0 to 100";
  `docs/cli.md`: "Use `--min-msi PERCENT` to require a total mutation score";
  source rejects values above 100 (`src/configuration.rs:175-182`, `u8`
  field at `src/configuration.rs:22`).
- "The ci.md companion 75 is min_covered_msi": **TRUE**.
  `docs/ci.md` "Recommended thresholds" row *Active work*: `--min-msi 60`,
  `--min-covered-msi 75`; the companion of 60 is 75, as a covered gate.
- "without `--coverage` a positive min_covered_msi exits 4": **TRUE**.
  `docs/config.md`: "A positive `min_covered_msi` without `--coverage` returns
  exit value 4"; `docs/cli.md` (LLVM Coverage) repeats it; implementation at
  `src/main.rs:1020-1029` (exit 4 when `minimum > 0` and no coverage; the
  `minimum == 0` case passes). The word "positive" in the comment is exactly
  right.
- "MSI = 0.8333333333333334 on strsim, 2026-09-28": **TRUE** (local
  evidence, not upstream). `docs/refs/quality_gates_mise_adoption.md:924,931,937`
  records §11.1 "Scoped baseline run (2026-09-28)" with that exact MSI and
  stats JSON; the baseline was committed the same day.
- "full-project run stays deferred under the no-multi-hour-runs constraint" :
  **TRUE** as repo policy (ticket 12 deferrals; scoped `-m`/`-f`/`--git-diff`
  is the documented cost lever in `docs/refs/mutation_testing.md` §3).

### Block 6: lines 48-49

> `# Empty allowlist = all mutators enabled (config.md: "An empty mutator`
> `# list, or `*`, means all mutators"), minus disable_mutators above.`
> `enable_mutators: []`
> `ignore_source_lines: []`

- The quoted sentence: **TRUE**, verbatim, in `docs/config.md` (Source
  Annotations section): "An empty mutator list, or `*`, means all
  mutators."
- The generalization to `enable_mutators: []`: **TRUE**. The source applies
  exactly that rule: an empty allowlist permits every mutator, and
  `disable_mutators` then subtracts
  (`src/configuration.rs:105-116`); schema default for `enable_mutators` is
  `[]`.
- `enable_mutators: []` and `ignore_source_lines: []` as keys/values :
  **TRUE** (both are supported keys at their defaults; `select/*` in
  `disable_mutators` also passes the schema pattern
  `^(?:\*|[a-z0-9-]+(?:/[a-z0-9-]+)*(?:/\*)?)$` and source validation
  `src/configuration.rs:224-235`, and matches known mutators
  `select/case-remove` / `select/default-remove`, so it does not trigger the
  "pattern does not match an available mutator" error at
  `src/configuration.rs:261-277`).

### Summary of special-attention items

| Item | Verdict |
| --- | --- |
| (a) empty list or `*` = all mutators | **TRUE** (config.md + `configuration.rs:105-116`) |
| (b) `min_covered_msi` exits 4 without `--coverage` | **TRUE** (config.md, cli.md, `main.rs:1020-1029`; "positive" qualifier correct) |
| (c) `exclude_dirs` path-prefix vs old `exclude_globs` | **TRUE** for docs/semantics; note the undocumented file-path-as-prefix behavior in §3 Block 2 |
| (d) `select/*` syntax and select mutator existence | syntax **TRUE**, mutators exist (**TRUE**); "drops mutants" claim **FALSE** (currently zero candidates, a no-op defense) |
| (e) `skip_*` semantics + docs never classify `any(test, …)` | **TRUE** (grep-verified; source classifies it as non-test cfg, so the warning is warranted) |
| (f) `report.json` clobbering incl. `--dry-run` | **TRUE** for dry-run; "every run" misses the `--update-baseline` exception (`main.rs:752-754`) |
| (g) html/json/silent_mode defaults | **TRUE**, all three default `false` (schema); local sets `true`/`true`/`false` deliberately |
| (h) referenced doc/section names | all **exist and unmoved**: `config.md`, `cli.md`, `ci.md` at the same repo paths; `§9.3` (`quality_gates_mutarust.md:672`), `§9.17` (`:1004`); `--min-msi PERCENT` still in cli.md |
| comment's "the 8 non-cli error.rs files" | **STALE**, now 9 (`src/temporal/error.rs`, added 2026-10-12) |

---

## 4. Stale snapshot deltas

Compared against live upstream `main` on 2026-11-02.

**Vendored mirror `docs/refs/mutarust/`, mostly fresh:**

- **Byte-identical to live upstream:** `README.md`, `docs/config.md`,
  `docs/cli.md`, `docs/ci.md`, `docs/mutators.md`, `docs/glossary.md`,
  `docs/parity.md`, `docs/selective-ci.md`, `docs/quickstart.md`,
  `docs/install.md`, `docs/release.md`, `docs/custom-mutators.md`,
  `schema/mutarust.schema.json`, `mutarust.yml.example`. No refresh needed;
  every config-related citation in `mutarust.yml` points at a file that has
  not drifted.
- **Changed upstream (refresh candidates):**
  - `docs/json-outputs.md`: report example `version` bumped `"0.2.0"` to
    `"0.2.1"`.
  - `docs/index.md`: upstream added a link to the
    `2026-10-03-workers-exec-scope` exploratory report.
  - `schema/{report,agentic,summary,gitlab}.schema.json`: upstream
    reformatted/compacted these (added `$schema`/`title`/`required`,
    reordered keys). A JSON-parse comparison shows them **semantically
    equal** to the local copies, so this is formatting drift only, not
    content drift.
- **New upstream files not vendored:** `CHANGELOG.md`, `Cargo.toml`,
  `docs/agents/{domain,issue-tracker,triage-labels}.md`,
  `docs/performance/workspace-manifest-index.md`,
  `docs/exploratory-testing/2026-10-03-workers-exec-scope.md` (plus its
  fixture sources). `CHANGELOG.md` is the most useful of these: it documents
  releases **0.1.10** (report-seam refactor, 2026-09-26), **0.2.0**
  (blacklist-checksum output added to result lines, `report.json`
  `blacklistChecksum` field, HTML report, 2026-09-26), and **0.2.1**
  (workspace-copy fix, 2026-09-30). None of these change config keys, exit
  codes, or gate behavior, so no `mutarust.yml` claim is invalidated by them.

**`docs/refs/quality_gates_mutarust.md` (research note, checked 2026-09-25):**

- **Version facts stale:** §1 records version `0.1.9` and "latest 0.1.9 on
  2026-09-20"; live is **0.2.1** (upstream `Cargo.toml`;
  `CHANGELOG.md` 2026-09-30; crates.io `max_version` 0.2.1, updated
  2026-09-30; the locally installed binary via mise is also 0.2.1). §1's
  adoption stats (0 stars/downloads, last push 2026-09-25) are obviously
  dated.
- **§9.3 is wrong where `mutarust.yml` is right:** it says `--dry-run` is
  "count selected mutants only, **no writes**, no tests". The source writes
  enabled reports even on dry-run (`src/main.rs:755-757`); upstream
  `docs/cli.md` only promises "without writing mutation areas or running
  tests". The `mutarust.yml` comment (Block 1) is the more accurate record.
- **§7 local sources partly deleted/moved:** `.cargo/mutants.toml` and
  `.mise/tasks/mutants/report` were removed in `ffb725d6` (cargo-mutants
  retirement); the mutation task now lives at `.mise/tasks/test/mutants`.
- **§9.13 drift findings still live (not stale):** the schema mutator pattern
  still forbids `_` while built-ins like `arithmetic/assign_invert` and
  `loop/range_break` contain it (runtime validation in `configuration.rs`
  allows `_`; only the published schema is stricter);
  `mutarust.yml.example` still sets `min_msi: 80` / `min_covered_msi: 80`
  (a trap if copied, since the covered gate needs `--coverage`); the stray
  `docs/refs/messrust/mutarust.yml.example` still differs only in
  `min_msi: 75` vs 80 (re-verified by diff).
- **§9.18 item 13 partially resolved:** the json-outputs example version has
  moved from 0.2.0 to 0.2.1 (it keeps tracking upstream releases; §1's 0.1.9
  reference remains stale).

**`docs/refs/mutation_testing.md`:**

- §6 says "`mutarust 0.1.10` (pinned in `mise.toml`)". **Stale:**
  `mise.toml:36` now reads `"cargo:mutarust" = "latest"` and the installed
  version is 0.2.1. (Commit `3d35c428` corrected the "latest pin" fact in the
  ticket file but did not touch this doc.) Everything else checked in that
  file, including report overwrite behavior on `--dry-run`,
  `--update-baseline` writing no reports, the exit-code table, and task
  flags, matches upstream source.

---

## 5. Citations and verification notes

**Upstream primary sources (fetched 2026-11-02):**

- README: https://raw.githubusercontent.com/quality-gates/mutarust/main/README.md
- Config reference: https://raw.githubusercontent.com/quality-gates/mutarust/main/docs/config.md
- CLI reference: https://raw.githubusercontent.com/quality-gates/mutarust/main/docs/cli.md
- CI guidance: https://raw.githubusercontent.com/quality-gates/mutarust/main/docs/ci.md
- Mutator list: https://raw.githubusercontent.com/quality-gates/mutarust/main/docs/mutators.md
- JSON outputs / index / glossary / parity / selective-ci / quickstart /
  install / release / custom-mutators: same raw base under `docs/`
- Config schema: https://raw.githubusercontent.com/quality-gates/mutarust/main/schema/mutarust.schema.json
- Report schemas: `https://raw.githubusercontent.com/quality-gates/mutarust/main/schema/{report,agentic,summary,gitlab}.schema.json`
- Example config: https://raw.githubusercontent.com/quality-gates/mutarust/main/mutarust.yml.example
- Source (config parsing/validation): `src/configuration.rs` (struct
  `configuration.rs:10-33`, `FileConfiguration` `:134-148`, defaults `:78-94`,
  score validation `:175-182`, mutator patterns `:213-248`, empty-allowlist
  rule `:105-116`)
- Source (exit codes / report writing): `src/main.rs` (dry-run reports
  `:755-757`, baseline-update early return `:752-754`, active reports
  `:799-820`, total gate `:964-978`, covered gate `:1020-1039`)
- Source (filters): `src/filter.rs` (exclude prefix `:80-91`, prefix test
  `:931-953`, cfg classification `:440-455`, cfg ranges `:387`,
  mutation-range overlap `:218`); call site `src/execution.rs:1929`
- Source (report filename): `src/report/mod.rs:26`
- Manifest / changelog: https://raw.githubusercontent.com/quality-gates/mutarust/main/Cargo.toml
  and .../CHANGELOG.md
- crates.io: https://crates.io/api/v1/crates/mutarust (`max_version` 0.2.1,
  updated 2026-09-30; fetched with a User-Agent header)
- GitHub tree API (file inventory): https://api.github.com/repos/quality-gates/mutarust/git/trees/main

**Local primary/first-party evidence:** `mutarust.yml`,
`docs/refs/mutation_testing.md`, `docs/refs/quality_gates_mutarust.md`,
`docs/refs/quality_gates_mise_adoption.md:924-946`,
`.scratch/bdd-strategy/issues/12-coverage-gap-methodology.md`,
`.mise/tasks/test/mutants`, `mise.toml:36`, `.gitignore:29-33`,
`mutarust-baseline.json` (commit `8a2b3ba7`),
deleted `.cargo/mutants.toml` (last content via
`git show ffb725d6^:.cargo/mutants.toml`),
git history for `src/temporal/error.rs` (`0da0f1d0`, 2026-10-12) and
`mutarust.yml` (`3c8a6b7a`, 2026-09-27), and repo-wide source searches
(`select!` count, `cfg(any(test` count, `error.rs` inventory).

**Not verified / caveats:**

- The undocumented behavior "a file path in `exclude_dirs` excludes that
  file" is derived from `Path::starts_with` semantics plus
  `src/filter.rs:80-91`; it has no doc statement and no dedicated upstream
  test (the adjacent test covers directory prefixes only). Treat as
  source-derived, not documented behavior.
- cargo-mutants' exact `exclude_re` matching semantics (file-level vs
  line-level) were not re-checked against the cargo-mutants book; the claim
  assessed here is only that mutarust has no `exclude_re` port (verified) and
  that the old config used those regexes to skip impl boilerplate (verified
  from the deleted file's own comment).
- No mutation run was executed during this research (dry-runs write
  `report.json`, which this task was forbidden to touch); zero-yield for
  `select/*` is argued from `docs/mutators.md`'s documented target forms plus
  the absence of any `select!` token in `src/`.
