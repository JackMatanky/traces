# 14: Template capability — overwrite refusal, non-TTY, freshness, index feedback

**What to build:** The template capability's four unique process/integration boundaries:
overwrite refusal proven non-destructive at process level for the first time, the real dialog
path proven deterministic on null stdin (no CI hangs), and the two regressions flagged at
audit §10 — I-4, named in code, and I-5, untested — gain their guards.

**Blocked by:** 07 (prerequisites), 08 (spawn-only invariant), 10 (exit-code accessors).

**Category:** enhancement

**Status:** resolved

- [x] Overwrite refusal: spawn `template -i X --no-input` over an existing file → exit 1 +
      `traces::cli::template::output_exists` diagnostic on stderr + target file byte-identical
      afterward (highest-blast-radius gap: non-destructiveness never proven at process level)
- [x] `-o` / `-f` argv paths spawned too (in-crate tests construct the struct directly today;
      only the `-n`/`-o` conflict is argv-tested)
- [x] Non-TTY determinism: spawned template test on null stdin **without `--no-input`** — that
      flag swaps in the preset dialog provider, and the entire point is exercising the real
      terminal dialog path (which short-circuits to defaults on non-TTY); assert the
      deterministic outcome **exactly** — exit 0 with the default-path render (audit §14
      P1.16's contract) — not merely that the test terminates; bounded by a std-only harness
      watchdog (spawn + timed kill) so no hang can wedge the suite and no new dependency or
      ticket-18 timeout is required
- [x] Cross-render freshness: the cached-field regression named in code ("would wrongly persist
      across independent renders") gains a guard — all existing template integration tests are
      single-render
- [x] Template output re-enters the index: writer → fs → refresh → query feedback loop asserted
      (write units test path resolution only; index units read pre-existing files)
- [x] Each test carries executes-vs-asserts note + defect class (destructive-overwrite,
      nondeterministic-TTY, stale-cross-render-cache, untested-feedback-loop)

## Comments

Resolved in commit `c8eb4b2d` (merged into `integration-e2e-suite`).
- `tests/e2e/template_write.rs` (4 new tests on the 2 existing):
  - `overwrite_refusal_is_non_destructive_and_force_and_output_paths_spawn`:
    refusal exits 1 with `traces::cli::template::output_exists` and the
    target stays byte-identical; `-f` overwrites; `-o elsewhere.md`
    redirects (destructive-overwrite).
  - `non_tty_dialog_short_circuits_to_defaults_without_hanging`: real
    `TerminalDialogProvider` (no `--no-input`) on null stdin renders the
    prompt default exactly (`anon`); bounded by the new std-only watchdog
    (nondeterministic-TTY).
  - `rendered_output_reenters_the_index`: rendered note is query-visible
    through its tag on the next spawn (untested-feedback-loop). The counted
    tag is assembled dynamically in the template so the template file (which
    is itself inside the project and indexed) does not carry the literal
    tag.
  - Harness: `Sandbox::run_with_watchdog` — spawn + `try_wait` poll +
    timed kill, `Option<Run>` (None = killed); no new dependency, no
    ticket-18 runner timeout. Narrow `#[expect(clippy::disallowed_methods)]`
    on the poll sleep, mirroring the existing mtime-wait precedent.
  - Rendered-output content assertions account for minijinja stripping the
    template's trailing newline (observed: `{{ ... }}
→` renders without it).
- `tests/integration/template_render.rs::
  index_cache_does_not_persist_across_independent_renders`: two renders on
  one `TemplateService` with a note written between them; second render
  observes the new note (2 vs 1) — a `QueryOps` cache field would have
  served the stale count (stale-cross-render-cache; the constraint is named
  on `INDEX_CACHE_KEY` in `src/template/engine/query.rs`).
- Gate: `check:layers`, `check:visibility` (empty diffs), full suite
  (lib 3,151 / integration 61 / e2e 61 / doctests 83), clippy `-D warnings`,
  fmt — all green.

**Evidence:** audit §10 E-1 (overwrite refusal — highest blast radius), E-4 (non-TTY), I-4
(cross-render freshness), I-5 (output re-enters index); §14 P1.16 (non-TTY contract).
**Defect class:** destructive-overwrite; nondeterministic-TTY; stale-cross-render-cache;
untested-feedback-loop.
