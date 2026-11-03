# 14: Adoption roadmap

Type: grilling
Status: resolved
Blocked by: 10, 12, 13

## Question

In what order should BDD be adopted, and what is the concrete first slice? This
is the last decision ticket. Its answer feeds `spec.md` directly.

**Inputs**:

- tickets 06 (framework verdict and caveats), 07 (tiering rule), 08 (layer
  architecture), 09 (e2e prototype results), 10 (invariance list), 11
  (organization and authoring conventions), 12 (quality/gap method), and 13
  (migration rules)
- the five inventories (which domains carry the highest-value, least-pinned
  guarantees)

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Write prioritized adoption slices. Each slice names its domain, the
  guarantees it pins, and its dependency on infrastructure. The first slice must
  bootstrap layout, fixtures, and phase wiring.
- (b) Write the pilot slice in enough detail that an implementing session starts
  cold. Name candidate domains, which existing tests it first interacts with,
  and its acceptance criteria.
  Use only guarantees whose doc/code contradictions have a recorded disposition
  from ticket 12, decision (f). Later slices resolve the contradictions that
  they need as ordinary doc fixes inside the slice.
- (c) Give the sequencing rationale: risk, value of guarantees pinned, and
  learning payback. Early slices should validate the framework verdict from 06
  in real conditions.
- (d) Write the explicit "not now" list: the domains deliberately deferred, with
  reasons. Candidates for the map's fog graduate into this list, not into more
  tickets.

**Answer records**: the ordered slice list, the pilot slice dossier, and the
not-now list. Resolution of this ticket is followed immediately by writing
`spec.md` (that writing is the map's destination act, not a new ticket).

## Answer

Resolved in a grilling session on 2026-11-03. The user confirmed each decision.
The full text is in [spec.md](../../bdd-testing/spec.md), section 9 (roadmap) and section 8
(agent policy). This Answer records the decisions and the evidence.

Evidence: eight external research files, tags A to H:
[A adoption](../research/14-bdd-adoption-evidence.md),
[B CLI test architecture](../research/14-rust-cli-test-architecture.md),
[C CI and flakes](../research/14-ci-crossplatform-flakes.md),
[D sequencing](../research/14-sequencing-pilot-evidence.md),
[E foundation gaps](../research/14-rust-foundation-gap-analysis.md),
[F spec quality](../research/14-executable-spec-quality.md),
[G framework currency](../research/14-framework-currency.md),
[H test strategy](../research/14-test-strategy-foundations.md).
The research found no reason to reopen ticket 06.

### Mechanics

- One worktree and one branch per slice. Merge locally when the slice closes.
- CI runs only on push to `main` and on `pull_request`. Push the slice branch and
  open a draft PR as a CI trigger only. In ticket 13, "PR" reads "slice branch
  with its draft PR".
- Slice 1 has two commits. A three-OS gate sits between them.
- Foundation extras: only a nextest `[profile.ci]` with JUnit output and no
  retries, wired in `ci.yml`. All other research gaps are on the not-now list.

### (a) Ordered slices

| # | Domain | Pilot or gate |
|---|---|---|
| 1 | Trust and init lifecycle | Pilot. Bootstrap. Three-OS gate. First deletion. |
| 2 | Process contract | Exact-equality steps |
| 3 | Query surface | Sort-test rename |
| 4 | Index freshness | After the three-OS result |
| 5 | Config scope, tracking and template | May split at entry |
| 6 | Schema and temporal at the boundary | Entry condition: time zone on Windows |
| 7 | Note-model rules through query output | None |
| 8 | Spawned-signal exit 130 (Unix) | Entry condition: platform-gating spike |

The row list of each slice, with namespaced IDs, is in spec.md section 9.3. The
accepted list named some rows. The roadmap placed the other rows by domain
affinity. The user may reassign them. Outside the slices: U1 (yaml.rs pins), U2
(three proptests and `proptest-regressions/`), a doc pass, and a hygiene backlog
file that the user creates at slice 1 close.

### (b) Pilot dossier

Spec.md section 9.4 holds the full dossier. Summary:

- Rows: 01-G17 (new coverage), 01-G23, 01-G24, and 01-G4 and 05-G3 for the
  `config_build_untrusted` code. ADR-0002 dispositions 1 and 6 apply.
- S1: after `init`, `list` exits 1 with the untrusted code and an empty stdout.
  S2 converts `trust.rs::untrust_then_list_fails_with_the_untrusted_diagnostic`.
  Both use a one-line docstring step to test docstring comparison on Windows.
- Commit 1 adds the framework, `"Zlib"`, the layout, the `check:layers` fix and
  five feature-text checks, `.gitattributes`, the CI profile, and two task
  fixes. It deletes nothing.
- Commit 2 deletes the old test after the kill demonstration and a relative
  mutation compare on `src/config/trust.rs`. It adds the first ledger, the Status
  columns, the sixth gate check, the first gap recompute and the profraw check.

### (c) Sequencing rationale

Learn on stable, high-value rows first. Then take volume. Then take timing-
sensitive rows after the three-OS result. Then take host-dependent rows. The
pilot checks the framework verdict in real conditions on three operating
systems. No arch ticket is near implementation, so no refactor deadline puts a
moving domain first. The research supports a vertical first slice [D].

### (d) Not-now list

Spec.md section 9.9 lists twelve items with reasons and owners. Real-TTY dialogs,
the mtime canary, plan and store internals, gate-owned rows, glossary-only
claims, known defects, 04-B1 until arch ticket 20 is implemented, facade-seam
scenarios, new workspace entry points, mutation and coverage in CI, unit tests
on three OS, and the foundation gaps. The fog items of the map graduated into
this list.

### Policies

- **Agent policy (human-owned contract).** An agent may add scenarios. Editing,
  weakening or deleting an existing scenario needs a `spec change: <reason>`
  line in the PR, confirmed at review. An agent never deletes a scenario outside
  ticket 13's die rule. A behavior change is red-first. A characterization
  scenario records one break demonstration. Review enforces this. No gate can see
  authorship.
- **Pilot rubric.** Three yes or no questions, no numbers. Questions 1 and 2 are
  due at slice 1 close. Question 3 (a second cold session writes a scenario from
  the spec alone) is due at slice 2 close. The review checklist of ticket 12
  stays binding. A red OS with a framework cause reopens tickets 06 and 08. The
  user decides.
- **Mutation compare.** A relative compare: record the escape sets before and
  after, and fail on a new escape. The baseline stays untouched.

### Stress test record

Two adversarial reviews, a smoke run and a mutation run tested the draft. They
found and fixed these items:

- Rows 01-G4 and 05-G3 were missing from slice 1.
- 04-B2 is not blocked by a seam. Only 04-B1 is.
- The exact-equality steps and the Windows docstring check belong in slice 1.
- The cold-session question could not run at slice 1 close.
- `[profile.ci]` does nothing until `ci.yml` selects it.
- Six obligations were missing. They are in the register of spec.md section 10.
- Arch ticket 38 rejects `cargo-semver-checks`. Ticket 20 owns the
  `FileClassExpander` removal.

The smoke run on macOS confirmed both scenario premises: exit 1, empty stdout and
the code on the first stderr line. The mutation run found that `-f
src/config/trust.rs` has 20 mutants and runs longer than 30 minutes, because the
task runs the whole suite per mutant.

### Open and unverified

- Linux and Windows results: unknown until the slice 1 draft PR runs.
- Whether `src/config/trust.rs` passes `--fail-on-escaped` at base.
- The `audit` CI job failed on 2026-10-29 at a tool install. Its state now is
  unknown.
- The agent policy is review-enforced.
- The candidate table is a static agent judgment. It maps the init test to
  01-G17 wrongly. The slice 1 commit 2 fixes the row.

## Comments

- 2026-11-03: Claimed. Eight research files written and committed (`dd163371`).
  Round 1: shape of slice 1, foundation scope, agent policy, rubric. Round 2: gate
  evidence, worktree scope, pilot domain, order, hygiene owner. Round 3: dossier,
  not-now list, unit items. A final stress test applied the fixes above. Status:
  resolved. `spec.md` written.
