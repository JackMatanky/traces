# Round-2 defect hunt: ambient state, composition root, input ownership

Evidence tags: [V] = I read the cited source this round; [H] = inference from verified
mechanism; [P] = proposal. Scenario letters map to the task brief (a)-(l). ADRs and
ticket Answers are treated as (D) derived unless the user's own words ratify them; none
of the entries below depends on a (D) claim being true.

A recurring shape: the codebase has three date domains that look like one. (1)
Index-time derivation (`FileMeta.day`, naive frontmatter instants) frozen into the
store; (2) query-time derivation (`file.cdate`, `file.mdate`, filter literals,
Date-to-midnight promotion, sort keys) sampled per read; (3) render-time sampling
(template `date.*`, `Display`). The defects below are what happens where the three
meet a zone change or a deferred materialization.

## Ranked defect register

### D1: Zone change is invisible to the index fingerprint; stale zone-derived rows are served indefinitely (a, f)

Severity: High. Exists today in the CLI; worse for a long-lived host.

Reproduction reasoning:

1. In `TZ=America/New_York`: a note with `due: 2026-07-29T00:30` is parsed on a Rayon
   worker; the naive string resolves to the UTC instant `04:30Z` at parse time
   (`src/field.rs:769-798` classifies the string; `src/temporal/datetime.rs:120-135`
   resolves unzoned input against `local_zone()`; called under `par_iter` in
   `IndexerService::parse_notes`, `src/index/service.rs:329-337`). [V]
2. Persisted rows include `FileMeta.day` (`src/file.rs:45-53`), derived from
   `created_at`'s local calendar day when the filename has no date prefix
   (`src/file.rs:88-98`, `DateValue::try_from(SystemTime)` →
   `src/temporal/date.rs:178-192`). [V]
3. Fly to `TZ=Asia/Tokyo` (or sync the vault from a second machine: `traces init`
   scaffolds `.traces/` and writes no ignore rule for it, `src/cli/init.rs:29-32`;
   `docs/adr/` and repo conventions do not exclude `.traces/index.redb`). [V]
4. Next `traces list`: nothing changed on disk → `RefreshState::Fresh` returns the
   opened store without rewriting rows (`src/index/service.rs:280-286,195-207`);
   `run_from_store` then reads persisted `FileMeta` and note rows
   (`src/query/service.rs:130-170`; wired at `src/cli/mod.rs:335-366`). The note's
   `due` is still the New-York interpretation; `file.day` is still the New-York day. [V]
5. No heal: the fingerprint is exactly the 10 config-file digests
   (`src/index/fingerprint.rs:37-38,142-143`); zone is not a config setting and is in
   no digest. Only editing the file re-parses it under the new zone, so two identical
   strings in one vault can carry different instants depending on when each file was
   last touched. [V]

The doc comment at `src/index/service.rs:107-121` says unchanged notes reuse persisted
parses "so a config edit heals on the next refresh"; a zone change is exactly the edit
class that does not heal. Eliminating element: ticket 19/24-26 civil-values cutover
(stop persisting zone-derived values), or, weaker, adding the derivation zone to the
fingerprint (a zone with 500 entries is one rebuild per travel day; also fine).

### D2: One query runs in two zones: persisted `file.day` vs query-time `file.cdate`/`file.mdate` (a, g)

Severity: High. Live in the CLI.

`FileField::Day` serves the persisted store value (`src/query/results.rs:307-312`,
store read per D1), while `CreatedDate`, `ModifiedDate`, `CreatedDateTime`,
`ModifiedDateTime` are converted from `SystemTime` at row-resolution time
(`src/query/results.rs:271-306`), i.e. under the *current* process zone
(`DateValue::try_from` → local date, `src/temporal/date.rs:178-192`). After a zone
change with an unchanged file set, `file.day` and `file.cdate` disagree inside a single
result row, and a filter/sort touching both mixes interpretations. Sort keys promote a
civil `Date` to a local-zone midnight at key-materialization time
(`src/query/sort.rs:373-376`, `TextShape` path :393-398) while `DateTime` keys carry
frozen index-time instants; the promotion is query-zone-dependent, the instants are
not. `NoteFieldValueRef::compare` documents the same dependence for filters
(`src/note/field.rs:273-300`: "a `Date`'s exact ordering against a same-day `DateTime`
depends on the process's local zone"). Eliminating element: one explicit evaluation
input per query reaching every row resolution (brief T1/T2), plus D1's cutover.

### D3: Zone-dependent code runs on Rayon workers that tests cannot pin; the whole non-UTC production surface is unverified (f, h)

Severity: High as a test-validity defect (it silently invalidates future index tests).

- `ZoneGuard` is thread-local by design: "Readers on other threads observe their own
  zone" (`src/temporal/mod.rs:96-99,198-202`). Rayon workers spawned by
  `IndexerService::scan`/`parse_notes` (`src/index/service.rs:331-337,436-440`) and
  `QueryService::run_from_store` (`src/query/service.rs:139-159`) therefore resolve the
  *real* system zone even when the test thread pinned one. [V]
- No index test sets a zone: `grep ZoneGuard src/index` → no matches. [V] So
  `derive_day` and naive-instant resolution have zero zone-pinned coverage; any such
  test written naively will pass on UTC CI and fail or (worse) pass vacuously on a
  developer machine in another zone.
- e2e pins `TZ=UTC` for every child run (`tests/e2e/support.rs:266,281` with
  `env_clear`, :293). Every end-to-end date behavior is exercised under exactly one
  zone. [V]

A host design that keeps ambient zone reads anywhere below the acquisition seam inherits
this: tests can pin the main thread but not the pool. Eliminating element: worker
closures receive zone as a value (or never touch zone, post-cutover); e2e gains one
non-UTC zone run (the `run_with_ambient_env` helper already exists,
`tests/e2e/support.rs:63-73`). [P]

### D4: Unit tests are not hermetic: cfg(test) swaps HOME but leaves every other ambient key live (b, j)

Severity: Medium-High (outcomes vary by developer machine; possible real-store writes).

The test static replaces HOME with `<manifest>/test` (`src/dirs.rs:81-83`) but
`CONFIG_HOME` still reads the real `XDG_CONFIG_HOME` first
(`src/dirs.rs:102-106`), and `TRACES_STATE_DIR`, `STATE_HOME` (`XDG_STATE_HOME`),
`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS` are real-env `LazyLock`s
(`src/dirs.rs:162-164`, `src/env_vars.rs:8-16,25-38`). `ConfigService::at` overrides
only the two store roots; `load` and the static `trust_subjects` still build
`ConfigLocator::from_env()` (`src/config/service.rs:60-65,97-100,194-199`;
`src/config/discovery.rs:43-48`). Consequences, all mechanism-verified:

1. A developer (or CI image) with `XDG_CONFIG_HOME` exported loads their real global
   `traces/config.toml` into every unit test that reaches `ConfigService::load` or
   in-process `Cli::run`: outcomes depend on the machine. [V]
2. A developer with `TRACES_STATE_DIR` exported points in-process `ConfigService::new()`
   tests at the real trust/tracker stores: a test that grants trust writes into the
   developer's real state. [V]
3. A file placed at `<repo>/test/Library/Application Support/traces/config.toml` (the
   test-HOME path; directory does not exist today) would silently become the global
   config for all unit tests. [V]

Actual divergence found (j): production HOME resolution `$HOME` → `/`
(`src/dirs.rs:61-63`) and the empty-string-is-unset rule (`non_empty_var`,
:192-194) are dead code under cfg(test): no unit test can exercise the unset-HOME
fallback because the static bypasses the env entirely; a regression there is invisible
until e2e. CI scrubs nothing (`.github/workflows/ci.yml` sets only `CARGO_TERM_COLOR`),
so the leak surface exists on runners that export XDG variables. Eliminating element:
the host acquisition seam supplies an env-map to tests (brief H2/H7); the
`nextest`-per-process model does not save in-process `Cli::run` tests.

### D5: A template render samples the clock 3 times and the zone 11 times; lazy query sets materialize later still (e)

Severity: Medium (narrow midnight window in a CLI; structural once a host caches).

Every date callback resolves `local_zone()` independently, at 11 sites in
`src/template/engine/date.rs` (:121,132,184,254,346,449,474,698,799,928,987), and
`Timestamp::now()` is sampled at three (`:161-163` `date.now()`, `:1007-1011`
`is_past`, `:1013-1027` `is_future`). A render straddling midnight yields a header
`date.today()` from yesterday and per-row `is_past` answers from today, plus
`date.today()`/`date.now()` disagreeing with each other. Nested `query` calls construct
fresh runs per verb (`src/template/engine/query.rs:100-124`), and a `filter` verb
appends a transform that executes on first read through a `OnceLock`
(`src/query/results.rs:456-491,538-541`); so even the eager/lazy split inside one
render samples at two different times. There is no carrier today; minijinja
`render_captured` builds its own `State` (brief-verified). Eliminating element: one
sampled `(now, zone)` per render handed to every date callback and query leaf (brief
T2), which also forces the lazy path to carry a resolved value.

### D6: Production installs no tracing subscriber: drift heals, refresh reports, and persist failures are silent (l)

Severity: Medium (observability amplifier for D1/D2).

`tracing` is a std-only production dependency and `tracing-subscriber` is a
dev-dependency (`Cargo.toml:148,164`); `src/main.rs` installs nothing. Therefore
`RefreshReport::ConfigDrift` (`tracing::info!`,
`src/index/service.rs:310-327`), per-run refresh stats (`:313-322`), and the
failed-persist warning that leaves "the stored fingerprint stale" (`:185-188`) are
discarded in production. The one event that would tell a user "your index was just
rebuilt under different assumptions" is compiled out of their visibility. Today's CLI
gets away with it because every command re-syncs; a host that skips syncs (ticket 34
scheduling) loses the only signal that staleness healing ever happened. Eliminating
element: host owns the subscriber as part of process-adapter setup, or the CLI prints
drift reports at normal verbosity — a ticket-31/30 handoff.

### D7: Non-TTY silently selects defaults; a future host inherits the policy by accident (k)

Severity: Medium.

`TerminalDialogProvider` returns `Ok(0)` for `select`, default-or-`""` for `text`,
default-or-`false` for `confirm`, and `[]` for `multi_select` whenever stdin is not a
TTY, with no marker in the output (`src/dialog/terminal.rs:43-108`). Piping any
command today (or running under an IDE) makes interactive template instantiations
silently take the first choice. There is no "headless" mode distinct from "piped by a
human"; an MCP/LSP host that reuses this provider gets silent first-choice behavior as
its answer policy (brief H5 flags it; the defect is that the current behavior is
indistinguishable from a deliberate answer: a rendered document gives no evidence a
question was skipped). Eliminating element: host-level dialog policy as an explicit
input to the template/Init paths (H5), with the silent fallback made visible in
output metadata at minimum.

### D8: Unrepresentable local midnight silently degrades to Less/Null in comparisons and sorts (g)

Severity: Medium-Low (exotic zones/dates; wrong answer, not an error).

`NoteFieldValueRef::compare` maps a failed Date→DateTime promotion to
`Ordering::Less` (`src/note/field.rs:297-300`); `SortKey::from_value_ref` maps the
same failure to `SortKey::Null` (`src/query/sort.rs:375-376,395-396`), which sorts
below everything and changes null-placement semantics. `DateTimeValue::try_from(Date)`
resolves midnight via `local_zone()` and fails on gaps/out-of-range
(`src/temporal/date.rs:341-344`; zone shifts at midnight and range edges exist, e.g.
whole-day skips). Both sites silently return a *ranking*, so a filter like
`due > X` can exclude a row without any diagnostic. Eliminating element: temporal owns
a fallible `day_interval`/promotion primitive (brief T4) whose error propagates as a
query error (ticket 31), instead of an ordering guess.

### D9: `Display for DateTimeValue` reads the ambient zone at write time and fails as a formatting error (h, l)

Severity: Low.

`Display` converts to local civil time and maps zone failure to `fmt::Error`
(`src/temporal/datetime.rs:297-301`), reached from output paths via
`append_text` (`src/note/field.rs:337-339`) and template interpolation. Two
consequences: rendering the same value in two processes can print different clock
times by design (documented at `src/temporal/mod.rs:75-84`), and a zone-lookup failure
manifests as a broken/empty field mid-render rather than a named diagnostic. With the
bundled tzdb (`jiff` `tzdb-bundle-always`, `Cargo.toml:128`) resolution failure is
rare, so the practical risk is the *coupling* (output format depends on where the
process runs), not availability. Eliminating element: render paths take the evaluation
zone explicitly (D5's carrier); `Display` keeps default-zone behavior only at the
process adapter boundary.

### D10: Trust/tracker stores accumulate grants under stale env policies; `trust --all` and the index agree only because both re-read the same globals (c, d)

Severity: Low (hygiene today; policy-freeze hazard for hosts).

The index fingerprint does not cover `TRACES_IGNORED_DIRS` (it is the 10 config
digests, `src/index/fingerprint.rs:142-143`); consistency is preserved only because
every refresh does a full scan with the current set and any path-set difference forces
a recompute (`src/index/service.rs:209-231,422-441`). So a changed ignore set *does*
heal the index on the next run, but a host that caches a scan or a `WorkspaceIndex`
across requests has no invalidation signal for env-policy changes at all (the
`LazyLock`s freeze the first read for the process: `src/env_vars.rs:8-38`). Trust has
the mirror problem with no heal: grants recorded under one ignore set persist for
paths the scanner would now skip (`src/config/discovery.rs:141-176` prunes ignored
names; the trust store has no revalidation pass). Grant-then-load under two different
state homes cannot happen inside one process today (one command per process, and
Rust 2024 forbids `set_var`), but `ConfigService::at` + static `trust_subjects`
already mix an injected store with env discovery in tests (D4), which is the same
mixed-authority shape an LSP would get wrong. Eliminating element: one captured
host policy (ignored dirs, ceilings, state home) passed to both walkers and both
stores (brief H3/F4), making "what the index sees" and "what trust granted" one value.

### D11: `.traces/index.redb` has no default ignore rule, making cross-machine zone alternation a normal deployment (a, supports D1/D2)

Severity: Low on its own; it is what upgrades D1/D2 from "travel" to "two laptops".

`traces init` scaffolds `.traces/` and nothing writes an ignore rule for it
(`src/cli/init.rs:29-32`; repo `.gitignore` covers build/editor artifacts only). A
vault synced by git or a sync tool carries a binary index whose zone-derived rows were
computed on the *other* machine; the local run then serves them under Fresh
(D1 step 4). Eliminating element: init writes `.traces/` to the project ignore file
(or the path-model ticket decides the store is machine-local by construction);
fundamentally D1's cutover.

### D12: Dependency-owned ambient reads are unaudited and partly dead-lettered (l)

Severity: Info.

Verified: `inquire` (crossterm backend, `Cargo.toml:127`) reads terminal state;
`miette` fancy rendering (`Cargo.toml:130`) reads terminal width/color conventions
(display-only). No `NO_COLOR`, `RUST_LOG`, or env-filter handling exists anywhere in
`src` (grep: no hits), so users cannot enable the logging that D6 shows is already
absent. No custom panic hook (`src/main.rs` has none); a Rayon worker panic
propagates by pool semantics and aborts the command with the default hook. `jiff`
resolves the system zone per call with its own 5-minute cache (brief; research 04/11),
so zone changes are observed only after TTL: irrelevant to one-shot commands,
relevant to a host that would otherwise assume live zone tracking. These are inputs no
design seam currently names; the acquisition seam should list them even if the decision
is "out of scope".

## Looks bad but is fine (do not spend grilling time here)

- `std::fs::read_dir(".")` at `src/index/entry.rs:360-365` is inside `#[cfg(test)]
  mod tests` (module at :217) and asserts that `WorkspaceIndex::for_test` creates no
  files. Not a production path. (The task brief's citation of this as production
  relative-path use is wrong.) [V]
- `env::current_dir` occurs only in `Cwd::new` (`src/cli/cwd.rs:31-33`); the
  `clippy.toml` ban holds. No production `read_dir` on relative paths: the index
  resolves against `self.root` from config (`src/index/service.rs:341-343`), and
  config roots come from `dir.ancestors()` of an absolute cwd
  (`src/config/discovery.rs:98-114`). [V]
- `Etc/Unknown`-on-failure (jiff's silent fallback) is not reachable: the code uses
  `TimeZone::try_system` and maps failure to `DateError::LocalZoneLookup`
  (`src/temporal/zone.rs:13-36`). [V]
- `DateValue`'s own `Display` is civil and zone-free (y-m-d; `src/temporal/date.rs:
  141-152`); only `DateTimeValue` renders zone-dependently (D9). [V]
- Mixed Date/DateTime *ordering logic* is internally consistent and pinned by a
  cross-check test (`src/query/sort.rs:1020-1067`); the hazard is the zone sampled
  during promotion (D2/D8), not the comparator. [V]
- CLI filters given on the command line are eager: they enter the builder's plan and
  run inside `QueryService::run`/`run_from_store` (`src/cli/mod.rs:352-366`,
  `src/query/service.rs:110-114,169`). Only template `filter` verbs are deferred
  (D5). [V]
- `ConfigService::at` does isolate the trust/track stores; the leak is bounded to
  discovery inputs (global config home, ceilings): store cross-talk is not possible
  through `at` (`src/config/service.rs:54-65,118-136`). [V]
- A changed `TRACES_IGNORED_DIRS` does not leave a stale index in today's CLI: the
  always-full scan turns the new set into a path-set diff and a recompute (D10). The
  stale-index claim would be true only for a future host caching scans. [V]
- ZoneGuard/CwdGuard never touch production state (both `#[cfg(test)]`,
  `src/temporal/mod.rs:92`, `src/cli/cwd.rs:55-126`). [V]
- e2e hermeticity is real for child processes: `env_clear` + owned/passthrough tables
  (`tests/e2e/support.rs:247-300`). The env gap is unit tests only (D4). [V]

## What the evidence says the design must guarantee (invariants, one sentence each)

1. No zone-derived value (`day`, naive-instant) is persisted without either being
   zone-free by construction or covered by a fingerprint that changes when the
   derivation zone changes (checkable: change TZ, run twice unchanged, assert no
   stale row is served).
2. All date-derived values consumed by one query (filter literals, row fields, sort
   keys, promotions) derive from one explicit `(now, zone)` input, and two
   evaluations of the same query in one process under different `(instant, zone)`
   produce identical results.
3. Anything evaluated after its creating call returns (lazy `QuerySet` transforms,
   Rayon workers) receives its evaluation inputs as values, never by ambient read
   (checkable: materialize a filtered set after the creator returns under a replaced
   zone and observe the creator's zone).
4. Environment-derived policy (ignored dirs, ceilings, state home) is captured once
   into a value that index scan, trust discovery, and both stores share, with no
   `env::var*` outside the acquisition module (enforceable by the existing
   `disallowed-methods` mechanism).
5. Unit tests read no developer environment: every ambient key production reads is
   sandboxed under cfg(test), verified by a canary (point `XDG_CONFIG_HOME`/
   `TRACES_STATE_DIR` at a canary and assert no test outcome changes).
6. Non-interactive execution is a host policy that leaves evidence (a rendered
   document or error states which prompts were defaulted), never an unmarked
   first-choice.
7. A failed date resolution at comparison, sort, or display time is a named error at
   a named site, never a silent ordering (`Less`/`Null`) or a blank field.
8. Refresh, drift, and persist-failure reports reach a sink in production builds
   (subscriber or stderr), so staleness healing is observable.
9. Cross-kind date ordering never depends on the process zone: a civil date compares
   to a datetime by a rule fixed at evaluation-entry, not by a per-call `local_zone()`
   (today contradicted by `src/note/field.rs:279-283`, the code's own admission).

## Claims from earlier artifacts corrected this round

- Brief/evidence pointed at `src/index/entry.rs:360-365` as a production
  relative-path hazard: it is a test-only `read_dir(".")` (see fine-list). [V]
- Brief cites `src/note/metadata.rs:56-72` for naive-datetime resolution; that range
  is YAML parsing. The resolution is `FieldValueRef::from`
  (`src/field.rs:769-798`) → `DateTimeValue::parse_iso`
  (`src/temporal/datetime.rs:120-135`). Citation drift, conclusion unchanged. [V]
- Round-1 framing "the index persists a zone-derived `FileMeta.day`" is right but
  incomplete in one direction: `day` is *re-derived on every full scan*, so today's
  staleness window for `day` is specifically the Fresh/persisted-store read path
  (D1 step 4); the whole surface freezes anyway once any host caches a scan or
  index across requests. [V]
