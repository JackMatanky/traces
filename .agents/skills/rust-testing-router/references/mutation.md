# Mutation Testing (`mutarust` and `cargo-mutants`)

> Coverage proves which lines a test executed. Mutation testing proves whether the test's assertions fail when the code breaks.

## 1. Operational Guardrails (Prevent Disk and RAM Exhaustion)

Mutation engines compile and test dozens of mutated copies of the crate. Obey these five operational rules before starting a run:

1. **Scope by module or git diff:** Never run an unscoped workspace-wide mutation build during iterative development. Scope to the changed file, module (`-m <module>`), or git diff (`--git-diff`).
2. **Stage new files before diff-scoped runs:** Git-diff mutation filters ignore untracked files. Run `git add -N <path>` on new source files before invoking `--git-diff`.
3. **Cap parallel workers:** Each worker maintains its own build directory. Cap concurrency (`--workers 4` or `-j 4`) to prevent multi-gigabyte target directory bloat and RAM exhaustion.
4. **Exclude Criterion benchmarks from mutant runs:** Pass `--lib --tests` so the engine skips slow `benches/` targets on every mutant.
5. **Account for `test-utils` and `skip_with_cfg`:** In `mutarust`, `skip_with_cfg: false` or `--features test-utils` mutates `test-utils` code unless excluded.

## 2. Running `mutarust`

 Check whether the repository provides a `mise` task first (for example, `mise run test:mutants`).

- **Diff-scoped run:**

  ```bash
  cargo mutarust --git-diff --workers 4
  ```

- **Single-mutant verification (bypasses global score gate):**
  When verifying a new test written to kill one escaped mutant, run that mutant ID directly so you get feedback in seconds:

  ```bash
  cargo mutarust --run-mutant-id <id>
  ```

  Note that each `mutarust` invocation overwrites its JSON/report artifact (`mutarust-report.json`). Save or inspect the report before starting a single-mutant re-run.

## 3. Running `cargo-mutants`

When the project uses `cargo-mutants`:

```bash
# Scope to a single file with 4 jobs
cargo mutants -f src/parser.rs -j 4 -- --lib

# Scope to lines touched in the current branch diff
cargo mutants --in-diff git-diff-file.patch -j 4
```

## 4. Triage Workflow for Escaped Mutants

Classify every surviving (escaped) mutant into one of three buckets:

| Classification | Meaning | Action |
|---|---|---|
| **True Escape (Weak Test)** | Mutant changes observable behavior (`<` to `<=`, deleted error check), and tests still pass | Add or sharpen a unit/integration assertion that kills the mutant |
| **Equivalent Mutant** | Mutant cannot change observable behavior (for example, pre-allocation capacity hint `Vec::with_capacity(n)` mutated to `0`) | Document why it is equivalent and add to baseline or exclusion rules |
| **Dead / Redundant Code** | Branch cannot be reached by any valid input | Delete the dead branch or replace it with a type invariant |

## Completion

Completion: output includes a triage table of killed, escaped, and equivalent mutants for the target scope, with zero unaddressed true escapes.
