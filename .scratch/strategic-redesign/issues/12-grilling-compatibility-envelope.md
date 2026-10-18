# What counts as the externally observable surface the redesign must preserve?

Type: grilling
Status: open
Blocked by: none

## Question
The destination says "preserve externally observable capabilities and correctness". Which surfaces are frozen, and which may change shape?

Candidates to classify (frozen / may change with migration / free): CLI subcommands, flags, exit codes and stdout/stderr contract (`tests/e2e/`); template function API and minijinja object protocol users write against (`query`, `lists`, `tasks`, `schema`, `ui`, `file`, `date`, string helpers); query DSL syntax for `--from`/`--where` including heuristic string-to-date promotion; schema TOML format incl. external select-values files; config TOML shape and defaults; on-disk `.traces/index.redb` (derived cache, currently disposable); trust/tracking symlink layout under XDG dirs; diagnostic codes (~35 dotted codes) and message text; the Rust library surface (`pub` items in `src/lib.rs`, `traces::testing`, `tests/baselines/public-api-*.txt`).

Evidence already in hand (main @ `207cc3fb`): the merge that landed config-drift healing rewrote `RefreshReport` from a `Copy` struct into an enum, which changed `traces::testing`, added 2 re-exports (45 to 46 identifiers), and regenerated `tests/baselines/pub-tokens.txt` (1,528 to 1,550 lines) and `public-api-test-utils.txt` (4,306 to 4,444 lines): the Rust-library baselines behave as change detectors refreshed per commit, not as a consumer compatibility contract. The same merge also wiped legacy stores once (`cache_format` to `meta`) and made a config edit heal the index on the next query (new e2e canary), which is an observable behaviour change inside an envelope the user may call 'disposable cache'.
Evidence to bring: `survey/cli.md`, `survey/cross-cutting.md` (visibility freeze), `docs/divergence-register.md`, existing ADRs, `tests/e2e/*`. Include whether Obsidian/Dataview/Templater compatibility is a requirement or a historical influence (it explains much of the temporal and inline-field behavior).

Why it is a decision: it bounds every redesign option (e.g. removing heuristic coercion, simplifying select values, changing diagnostic codes, splitting the crate).

Type: HITL. Grill the user; do not answer for them.
