# H2 VCS, Storage, and Workflow Guardrails

Deliverable:
`.scratch/arch-design/research/20-prep/round6/H2-vcs-and-workflow.md`  
Target: Ticket 16 (Schema Note Typing and Validation)  
Scope: Guardrails at version-control, storage, and workflow layers, and how
other domains gate untrusted writers without relying on agent harnesses.  
Tags: `[V]` verified from primary source or local empirical experiment; `[H]`
hypothesis / architectural deduction; `[S]` secondary citation; `UNREACHABLE`
source inaccessible.

---

## 1. Executive Summary & Architectural Axiom

Ticket 16 established an advisory `check_note(before, after)` contract, noting
that "the portable guarantee is detect-and-feed-back; true prevention only where
traces owns the write or the harness denies native writes."

This investigation evaluated 18 mechanisms across Git, alternative VCSs (jj,
Pijul), database engines (Dolt, SQLite), workflow architectures (Terraform,
Kubernetes, Gerrit, sync inboxes, APFS copy-on-write clones), shell integrations,
and store-authority designs.

### The Storage Authority Axiom

A system cannot prevent invalid writes if an untrusted writer possesses direct,
unmediated write access to the canonical storage files `[H]`.

- In Kubernetes, admission webhooks prevent invalid states because clients have
  no network route to `etcd` `[V]`.
- In SQLite, `CHECK` constraints prevent invalid states because writers must go
  through SQL transactions; raw disk bytes cannot be written directly `[V]`.
- In Git repositories, server `pre-receive` hooks prevent invalid commits
  because clients cannot bypass the server's ref-update transaction `[V]`.

Conversely, when an untrusted process (AI agent, shell script, or human editor)
has write access to `vault/note.md`:

1. Every in-place gate (watcher daemon, shell hook, clean filter) operates either
   asynchronously after disk bytes are flushed or at discrete VCS command
   boundaries `[V]`.
2. True **prevention** without harness cooperation is possible if and only if
   the writer is isolated to a sandbox copy (Git worktree/branch or APFS CoW
   clone) or a quarantine folder (`Inbox/`), and promotion to canonical storage
   is mediated by a validating tool (`traces merge` / `traces promote`) `[V]`.
3. For in-place modifications to canonical files, **detection and feedback**
   via exit-code linters, read-time diagnostics, and staged-blob checks is the
   only universally portable guarantee `[V]`.

---

## 2. Empirical Experiments Run Under `/tmp`

To ground all claims in verified behavior rather than documentation assumptions,
four empirical experiments were executed on macOS Darwin arm64:

### Experiment 1: Git Clean Filter Driver (`filter.<driver>.clean`)

- **Setup:** Initialized a Git repo in `/tmp/test_git_clean_filter`. Configured
  `.gitattributes` with `*.md filter=notecheck`. Defined a clean filter script
  rejecting files containing `INVALID` with exit code 1. Tested both
  `filter.notecheck.required = true` and `required = false` `[V]`.
- **Observations:**
  1. With `required = true`: `git add invalid.md` failed immediately with exit
     code 128: `fatal: invalid.md: clean filter 'notecheck' failed`. The file
     was NOT staged and remained untracked `??` `[V]`.
  2. `git commit -a` on a modified tracked file failed with exit code 128,
     blocking commit creation `[V]`.
  3. `git commit -a --no-verify` was ALSO BLOCKED with exit code 128! Clean
     filters execute during tree hashing before hooks run; `--no-verify` has no
     effect on filter drivers `[V]`.
  4. With `required = false` (Git default): filter errors printed to stderr, but
     `git add` exited 0 and staged the invalid file anyway `[V]`.
  5. Critical pitfall: When `required = true` was configured without an explicit
     `filter.<driver>.smudge cat`, subsequent `git checkout HEAD -- file.md`
     crashed with exit code 128 (`smudge filter failed`) and deleted the working
     tree file `[V]`. Furthermore, `git diff` on an invalid file failed with exit
     code 128 `[V]`.

### Experiment 2: Git `reference-transaction` Hook

- **Setup:** Configured `.git/hooks/reference-transaction` in
  `/tmp/test_git_reftxn` logging all states (`preparing`, `prepared`,
  `committed`, `aborted`) and input tuples `oldvalue newvalue refname` `[V]`.
- **Observations:**
  1. `git add` did NOT trigger `reference-transaction` at all `[V]`. Index
     updates are not ref transactions.
  2. `git commit` triggered `preparing`, `prepared`, and `committed` phases for
     `HEAD` and `refs/heads/main` `[V]`.
  3. Rejecting the transaction in the `prepared` phase (exit 1) aborted `git
     commit` with `fatal: in 'prepared' phase, update aborted by the
     reference-transaction hook` `[V]`.
  4. `git commit --no-verify` DID NOT bypass `reference-transaction`; it was
     aborted identically (exit 128) `[V]`.
  5. Crucially: `reference-transaction` receives only commit object hashes on
     stdin (`$oldvalue $newvalue $refname`). It does NOT see staged index blobs
     prior to commit creation and cannot see unstaged working files `[V]`.

### Experiment 3: Server-Side Pre-Receive Hook vs `--no-verify`

- **Setup:** Created a bare Git repo in `/tmp/test_bare_origin.git` with a
  `hooks/pre-receive` script validating incoming blobs via `git show
  $newrev:$file`. Cloned locally to `/tmp/test_client_vault` `[V]`.
- **Observations:**
  1. Client created an invalid commit using `git commit --no-verify`
     (bypassing client-side pre-commit hooks) `[V]`.
  2. Client ran `git push origin main`.
  3. Server rejected the push: `remote: Server rejection: note.md contains
     INVALID`, `! [remote rejected] main -> main (pre-receive hook declined)`,
     exiting 1 `[V]`. Client `--no-verify` cannot bypass server hooks `[V]`.

### Experiment 4: macOS APFS Directory `clonefile(2)`

- **Setup:** Created directory `/tmp/test_cf_dir_src` containing nested markdown
  notes. Executed the Darwin `clonefile()` syscall via `libc` `[V]`.
- **Observations:**
  1. `clonefile(dir_src, dir_dst, 0)` cloned the entire nested directory tree
     in <1 millisecond `[V]`.
  2. Inodes in `dir_dst` differed from `dir_src`, but disk storage was shared via
     copy-on-write `[V]`.
  3. Modifying files in `dir_dst` left `dir_src` completely untouched `[V]`.

---

## 3. Comprehensive Mechanism Stress Tests

### Group 1: Git-Layer Mechanisms

#### 1. Git Pre-Commit Hook on Staged Blobs (`git diff --cached`)

- **Precedent:** Git documentation `githooks(5)`
  (<https://git-scm.com/docs/githooks#_pre_commit>), Pre-commit framework
  (<https://pre-commit.com>), `check-jsonschema`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Only writes staged into the Git index (`git add`, `git
    commit -a`). Completely blind to unstaged working tree edits, editor
    auto-saves, direct shell redirects, or sync writes until staged `[V]`.
  - *(b) Timing & Feedback:* Runs at `git commit`. If validation fails, `git
    commit` aborts with stderr diagnostics. Staged files remain staged in the
    index; working tree files are untouched `[V]`.
  - *(c) Atomicity:* When inspecting staged blobs via `git show :<path>` (or `git
    diff --cached`), it is fully isolated from unstaged working-tree dirty states
    `[V]`. The index is updated atomically.
  - *(d) Platforms:* macOS, Linux, Windows (requires shell or executable; Windows
    needs git-bash or binary `.exe`).
  - *(e) PKM Cost:* Zero for Git users. High friction for non-Git PKM users
    (requires initializing a repository and committing changes) `[V]`.
  - *(f) Bypass routes:* `git commit --no-verify`, `git commit -n`, modifying
    `.git/hooks/pre-commit`, or editing files without ever committing `[V]`.
  - *(g) Failure mode:* Must fail-closed on checker crash (exit non-zero) to
    prevent invalid commits. Emergency bypass exists via `--no-verify` `[H]`.
  - *(h) Rust fit:* Excellent. `traces check --staged` using `gix` or `git2` to
    inspect index blobs directly `[V]`.
- **Verdict: Adopt as opt-in recipe / CLI flag (`traces check --staged`).**
  Essential backstop for Git users, but cannot serve as the universal gate.

#### 2. Git Clean & Smudge Filter Drivers (`filter.<driver>.clean`)

- **Precedent:** Git documentation `gitattributes(5)`
  (<https://git-scm.com/docs/gitattributes>).
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Executed whenever Git stages a file into the object
    database (`git add`, `git commit -a`, `git stash`). Also invoked during `git
    diff` `[V]`. Does not see unstaged disk writes by editors or shell redirects.
  - *(b) Timing & Feedback:* Synchronous during `git add`. When configured with
    `required = true`, an exit 1 halts `git add` with exit code 128 `[V]`.
    Writer receives: `fatal: <file>: clean filter '<name>' failed` plus filter
    stderr `[V]`.
  - *(c) Atomicity:* Per-file stream. If any file fails during `git add .`, the
    staging operation halts immediately with exit code 128 `[V]`.
  - *(d) Platforms:* Cross-platform across all Git distributions.
  - *(e) PKM Cost:* Severe UX hazard. If `required = true` is set, a missing
    smudge filter causes `git checkout` to crash and delete files `[V]`. Invalid
    files cause `git diff` and `git stash` to crash with exit 128 `[V]`.
  - *(f) Bypass routes:* `git -c filter.<name>.clean=cat add`, modifying
    `.git/config`, or editing `.gitattributes` `[V]`. (Note: `--no-verify`
    CANNOT bypass it `[V]`).
  - *(g) Failure mode:* Git defaults to fail-open (`required = false`). Setting
    `required = true` (fail-closed) breaks basic Git porcelain commands upon any
    schema violation or checker crash `[V]`.
  - *(h) Rust fit:* `traces filter-clean %f` is trivial to implement in Rust, but
    the Git porcelain blast radius is catastrophic `[H]`.
- **Verdict: Reject.** Too brittle and hazardous; breaks checkout, stash, and
  diff when files violate schema.

#### 3. Git Pre-Push Hook

- **Precedent:** Git documentation `githooks(5)`
  (<https://git-scm.com/docs/githooks#_pre_push>).
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Only remote push operations (`git push`).
  - *(b) Timing & Feedback:* Remote synchronization time. Too late for local
    authoring loops. Invalid commits are already recorded in local history `[V]`.
  - *(c) Atomicity:* Validates commit ranges before network transmission. If
    hook exits non-zero, push is aborted; local repo is unchanged `[V]`.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Requires a remote repository setup.
  - *(f) Bypass routes:* `git push --no-verify` `[V]`.
  - *(g) Failure mode:* Fail-closed on push.
  - *(h) Rust fit:* Native CLI invocation `traces check`.
- **Verdict: Opt-in recipe for multi-device sync or team vaults; reject as local
  editing guardrail.**

#### 4. Git Pre-Receive & Update Server-Side Hooks

- **Precedent:** Git documentation `githooks(5)`
  (<https://git-scm.com/docs/githooks#pre-receive>), GitHub Branch Protection,
  Gerrit.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* All pushes to the authoritative bare repository.
  - *(b) Timing & Feedback:* Push time. Client receives server stderr: `remote:
    Server rejection: note.md contains INVALID` `[V]`. Client `--no-verify`
    cannot bypass server-side hooks `[V]`.
  - *(c) Atomicity:* Complete ref transaction rollback in the bare repo `[V]`.
  - *(d) Platforms:* Server environment (Linux/macOS).
  - *(e) PKM Cost:* Unreasonable for single-user local vaults (requires running
    a separate bare repository and pushing/pulling locally) `[H]`.
  - *(f) Bypass routes:* None from client. Only server filesystem admin `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* High; `traces` binary can run inside server hooks.
- **Verdict: Adopt as theoretical reference model for store authority; reject
  as default desktop PKM UX.**

#### 5. Git `core.fsmonitor`

- **Precedent:** Git documentation `git-fsmonitor--daemon(1)`
  (<https://git-scm.com/docs/git-fsmonitor--daemon>), `core.fsmonitor` config.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* None for validation. It is an index stat query mechanism
    designed solely to speed up `git status` via OS file watching `[V]`.
  - *(b) Timing & Feedback:* Never evaluates schemas. It cannot veto writes or
    refuse status updates `[V]`.
  - *(c) Atomicity:* N/A.
  - *(d) Platforms:* macOS (FSEvents), Windows (ReadDirectoryChangesW).
  - *(e) PKM Cost:* Low to enable, zero validation benefit.
  - *(f) Bypass routes:* N/A.
  - *(g) Failure mode:* Fail-open (falls back to full lstat crawl).
  - *(h) Rust fit:* N/A.
- **Verdict: Reject.** Not a validation or guardrail mechanism.

#### 6. Git `core.hooksPath`

- **Precedent:** Git documentation `git-config(1)`
  (<https://git-scm.com/docs/git-config#Documentation/git-config.txt-corehooksPath>).
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Same as standard Git hooks.
  - *(b) Timing & Feedback:* Redirects Git hook lookup to a versioned directory
    (e.g., `.githooks/` or `.traces/hooks/`) `[V]`.
  - *(c) Atomicity:* Same as Git hooks.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Requires one-time manual user configuration: `git config
    core.hooksPath .traces/hooks`. Git security rules strictly ignore
    repo-level `.gitattributes` or `.gitconfig` for `core.hooksPath` to prevent
    arbitrary code execution upon clone (CVE-2014-9390) `[V]`.
  - *(f) Bypass routes:* `git commit --no-verify`, or `git -c core.hooksPath=
    commit` `[V]`.
  - *(g) Failure mode:* Same as pre-commit.
  - *(h) Rust fit:* `traces init --git-hooks` can configure `core.hooksPath`
    automatically `[V]`.
- **Verdict: Adopt as opt-in setup helper for Git vaults (`traces init
  --git-hooks`).**

#### 7. Git Worktrees & Dedicated Agent Branches

- **Precedent:** Git documentation `git-worktree(1)`
  (<https://git-scm.com/docs/git-worktree>), GitHub PR merge queues, Bors.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Agent is given write access ONLY to a dedicated worktree
    and branch (e.g. `.scratch/worktree-agent` on branch `agent/session-1`).
    Canonical vault on `main` is completely untouched by agent writes `[V]`.
  - *(b) Timing & Feedback:* At merge/promotion time (`traces merge <branch>`).
    `traces` validates all differences against schema (`check_note(before,
    after)`). If valid, fast-forwards or merges into `main`. If invalid, merge
    aborts, returning structured errors `[V]`.
  - *(c) Atomicity:* Fully atomic in Git history. `main` moves only upon clean,
    validated merge commit `[V]`.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Requires Git. Modest disk overhead for worktree metadata
    `[V]`.
  - *(f) Bypass routes:* Agent writing directly to `main` directory if not
    sandboxed `[V]`.
  - *(g) Failure mode:* Fail-closed on merge command.
  - *(h) Rust fit:* Excellent. `gix` / `git2` can perform tree merges and ref
    updates programmatically `[V]`.
- **Verdict: Adopt as the primary recommended isolation pattern for Git
  vaults.**

---

### Group 2: Alternative VCS & Data Stores

#### 8. Jujutsu (jj) & Pijul

- **Precedents:**
  - Jujutsu: <https://docs.jj-vcs.dev/>, `.gitattributes: No`, `Hooks: No`
    (<https://docs.jj-vcs.dev/latest/git-compatibility/>) `[V]`. Operation log
    in `.jj/repo/op_store/` `[V]`.
  - Pijul: <https://pijul.org/>, native record hooks (`preHooks`, shared hooks,
    explicit approval via `pijul hooks approve`) `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Jujutsu snapshots working copy changes into `@`
    automatically after every CLI command `[V]`. Pijul records explicitly via
    `pijul record` `[V]`.
  - *(b) Timing & Feedback:*
    - Jujutsu: Currently NO hook support `[V]`. Working-copy snapshots cannot be
      gated. `jj fix` exists for post-hoc rewriting, but runs after commits are
      created `[V]`.
    - Pijul: `pijul record` executes approved record hooks; aborts patch creation
      if hook exits non-zero `[V]`.
  - *(c) Atomicity:* Both provide transaction rollback (`jj undo`, Pijul patch
    unrecord) `[V]`.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Very high for desktop PKM users (learning curve, small
    ecosystem, no Obsidian plugin support) `[H]`.
  - *(f) Bypass routes:* In jj, working copy is snapshot unconditionally `[V]`.
    In Pijul, unrecorded working edits sit on disk ungated `[V]`.
  - *(g) Failure mode:* jj fail-open; Pijul fail-closed on record.
  - *(h) Rust fit:* Both built in Rust (`jj-lib`, `libpijul`), but internal APIs
    are unstable `[V]`.
- **Verdict: Reject as general PKM guardrail.** jj architecturally rejects
  pre-commit gates (every state is a commit); Pijul lacks ecosystem adoption.

#### 9. Dolt (Branch-and-Merge Relational Store)

- **Precedent:** Dolt (<https://github.com/dolthub/dolt>,
  <https://www.dolthub.com/blog/2024-07-23-schema-conflicts/>) `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* SQL queries (`INSERT`, `UPDATE`), `dolt merge` `[V]`.
  - *(b) Timing & Feedback:* SQL write time (table constraints enforce types) and
    merge time (multi-phase lifecycle: schema merge -> data merge -> conflict
    resolution -> constraint validation) `[V]`.
  - *(c) Atomicity:* ACID transaction atomicity.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Extreme. Storing notes in SQL tables / Prolly trees destroys
    plain-text markdown files `[H]`.
  - *(f) Bypass routes:* None within SQL engine.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* Dolt is written in Go; requires MySQL wire client in Rust.
- **Verdict: Reject for file storage; adopt architectural principle:**
  Multi-phase merge where constraint validation is the final gate before commit.

#### 10. Database Constraints & Triggers (PostgreSQL / SQLite)

- **Precedent:** SQLite `CHECK` constraints, `BEFORE INSERT/UPDATE` triggers with
  `RAISE(ABORT, 'msg')` (<https://www.sqlite.org/lang_createtrigger.html>) `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Only writes executed through the database connection.
    Completely invisible to filesystem edits made to loose markdown files `[V]`.
  - *(b) Timing & Feedback:* Synchronously inside `BEGIN ... COMMIT`. Rejects
    with `SQLITE_CONSTRAINT` before disk write `[V]`.
  - *(c) Atomicity:* ACID rollback.
  - *(d) Platforms:* Ubiquitous.
  - *(e) PKM Cost:* Requires vault to live inside a database file rather than
    independent markdown files `[V]`.
  - *(f) Bypass routes:* Editing loose files bypasses database triggers if files
    are not synced `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* `rusqlite` is mature and fast `[V]`.
- **Verdict: Fundamental lesson:** Database gates succeed because writers lack
  direct filesystem access. In file vaults, loose writes bypass DB triggers.

---

### Group 3: Workflow & Architecture Patterns

#### 11. Terraform Plan / Apply Pattern (Speculative Execution)

- **Precedent:** HashiCorp Terraform
  (<https://developer.hashicorp.com/terraform/cli/commands/plan>), Open Policy
  Agent Conftest (<https://www.openpolicyagent.org/docs/latest/terraform/>) `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Tools/agents execute `traces plan` (or `propose`). A
    speculative change plan is generated. Target files are not touched `[V]`.
  - *(b) Timing & Feedback:* Pre-write. `traces apply <plan>` validates against
    live vault state and schemas. If invalid, apply is aborted `[V]`.
  - *(c) Atomicity:* Plan generation is zero-side-effect read-only. `apply`
    writes all changes in a validated batch `[H]`.
  - *(d) Platforms:* Cross-platform.
  - *(e) PKM Cost:* Zero extra dependencies.
  - *(f) Bypass routes:* Agent writing directly to target files via shell
    redirection instead of calling `traces plan/apply` `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* Native CLI commands in `traces`.
- **Verdict: Adopt as core CLI workflow design (`traces plan` / `traces
  apply`).**

#### 12. Kubernetes Validating Admission Webhook Pattern

- **Precedent:** Kubernetes Admission Controllers
  (<https://kubernetes.io/docs/reference/access-authn-authz/extensible-admission-controllers/>)
  `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* All writers. Clients cannot touch `etcd` directly; all
    writes route through `kube-apiserver` `[V]`.
  - *(b) Timing & Feedback:* Pre-persistence. Apiserver dispatches
    `AdmissionReview` to validating webhooks. If rejected, HTTP 403 returned,
    store is never modified `[V]`.
  - *(c) Atomicity:* Strict. Either written to store or rejected.
  - *(d) Platforms:* Linux server clusters.
  - *(e) PKM Cost:* Heavy conceptually, but maps directly to: "store is fronted
    by an API, and raw filesystem access is denied" `[H]`.
  - *(f) Bypass routes:* None as long as etcd storage is restricted `[V]`.
  - *(g) Failure mode:* Configurable (`failurePolicy: Fail` vs `Ignore`). In
    integrity systems, must fail-closed `[V]`.
  - *(h) Rust fit:* Conceptually maps to validated MCP server tools `[V]`.
- **Verdict: Architectural reference standard for mediated write boundaries.**

#### 13. Gerrit / PR Review Gates (Isolated Ref Namespace)

- **Precedent:** Gerrit Code Review
  (<https://gerrit-review.googlesource.com/Documentation/intro-quickstart.html>)
  `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Untrusted writers push to staging ref `refs/for/main`,
    never canonical `refs/heads/main` `[V]`.
  - *(b) Timing & Feedback:* CI and reviewers evaluate patches in staging
    namespace. Only approved patches merge to canonical `main` `[V]`.
  - *(c) Atomicity:* Merges are atomic Git commits `[V]`.
  - *(d) Platforms:* Cross-platform Git.
  - *(e) PKM Cost:* Requires Git conventions.
  - *(f) Bypass routes:* Denied by permissions on `refs/heads/main` `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* Git ref manipulation via `gix`.
- **Verdict: Adopt conceptual model:** Quarantine / staging namespace separated
  from canonical vault.

#### 14. Syncthing / iCloud / Obsidian Sync Inbox Pattern

- **Precedent:** Syncthing conflict handling
  (<https://docs.syncthing.net/users/sync-conflicts.html>) `[V]`, Obsidian Inbox
  / Drop Folder pattern.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Untrusted processes (agents, mobile sync, external tools)
    write exclusively into an `Inbox/` or `_incoming/` directory `[V]`.
  - *(b) Timing & Feedback:* At ingestion time (`traces ingest` / `traces
    triage`). Conforming notes are moved to canonical paths; invalid notes remain
    quarantined in `Inbox/` with diagnostic logs `[V]`.
  - *(c) Atomicity:* Per-file `rename()` within same filesystem is atomic `[V]`.
  - *(d) Platforms:* Universal cross-platform.
  - *(e) PKM Cost:* Zero tools required. Pure directory structure convention
    `[V]`.
  - *(f) Bypass routes:* Bypassed if agent writes outside `Inbox/` `[V]`.
  - *(g) Failure mode:* Fail-closed (invalid files remain quarantined) `[V]`.
  - *(h) Rust fit:* Native file scanning and validation in `traces`.
- **Verdict: Adopt as primary non-Git workflow pattern (Inbox / Quarantine).**

#### 15. APFS Clonefile Working Copies (CoW Sandbox & Promotion)

- **Precedent:** Apple APFS `clonefile(2)`
  (<https://developer.apple.com/documentation/kernel/clonefile>) `[V]`, macOS
  `sys/clonefile.h` `[V]`, Linux Btrfs/XFS `ioctl(FICLONE)`.
- **Empirical Experiment Findings `[V]`:**
  - Calling `clonefile()` on a directory clones the entire hierarchy in <1ms
    `[V]`.
  - Storage blocks are shared via copy-on-write; only edits consume new blocks
    `[V]`.
  - Modifications in clone leave the source vault 100% pristine `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Agent edits files in `vault-clone/`. All editor temp
    files, truncate writes, shell redirects, and moves occur inside the clone.
    The canonical `vault/` is completely untouched `[V]`.
  - *(b) Timing & Feedback:* At promotion time: `traces merge <clone>`.
    `traces` scans diffs and evaluates `check_note(before, after)`. If valid,
    promotes to canonical vault. If invalid, promotion aborts with diagnostics;
    canonical vault remains untouched `[V]`.
  - *(c) Atomicity:*
    - Single file: atomic `rename()` or `renameatx_np(..., RENAME_SWAP)` on macOS
      `[V]`.
    - Multi-file: Requires transactional promotion loop (recording a manifest or
      journal) to guarantee all-or-nothing atomicity `[H]`.
    - Concurrency: Must verify that canonical files were not concurrently
      modified by the user since clone creation (optimistic concurrency check)
      `[H]`.
  - *(d) Platforms:* macOS APFS native (`clonefile` syscall [V]). Linux:
    supported on Btrfs and XFS via `FICLONE` (per-file reflink; no native
    directory clone syscall). Windows: ReFS only; standard NTFS does not support
    block cloning (requires full copy) `[V]`.
  - *(e) PKM Cost:* Unprivileged and zero-cost on macOS APFS and Linux Btrfs.
    Modest copy overhead on Windows NTFS `[V]`.
  - *(f) Bypass routes:* Agent writing directly to primary vault path instead of
    clone `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* High. Rust crates `reflink-copy` or direct `libc::clonefile`
    bindings on Darwin `[V]`.
- **Verdict: Adopt as premier local sandbox pattern for non-Git macOS/Linux
  vaults.**

#### 16. Proposals & Patch Files (`traces propose`)

- **Precedent:** Unified diffs, Git patches (`git format-patch`), Linux kernel
  patch workflow.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* Agent outputs patch bundles or JSON proposals (`traces
    propose > changes.patch`). Target files are never touched directly `[V]`.
  - *(b) Timing & Feedback:* `traces apply changes.patch` validates changes
    against current vault schemas in memory before applying to disk `[V]`.
  - *(c) Atomicity:* All-or-nothing patch application.
  - *(d) Platforms:* Universal cross-platform.
  - *(e) PKM Cost:* Zero dependencies.
  - *(f) Bypass routes:* Bypassed if agent writes directly to files `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* High (`diffy` or `patch` crates in Rust) `[V]`.
- **Verdict: Adopt as standard safe interaction mode for CLI agents.**

---

### Group 4: Shell & OS Level Integrations

#### 17. Shell Shims, direnv, PROMPT_COMMAND, launchd/systemd Units

- **Precedent:**
  - PATH shims: `asdf`, `rbenv`, wrapper scripts in `$PATH`.
  - `direnv`: `.envrc` hook on directory change (<https://direnv.net/>) `[V]`.
  - `PROMPT_COMMAND` (bash) / `precmd` (zsh): shell prompt hooks.
  - `launchd` `WatchPaths` / `QueueDirectories` (<https://www.launchd.info/>)
    `[V]`.
  - `systemd.path` with `PathModified=`
    (<https://www.freedesktop.org/software/systemd/man/systemd.path.html>) `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:*
    - PATH shims: Only see commands explicitly intercepted (e.g. `git`). Blind
      to Python, subshells, node scripts, or GUI apps (Obsidian) `[V]`.
    - direnv: Only runs on directory change in interactive shells `[V]`.
    - PROMPT_COMMAND: Only runs after commands in interactive shells; blind to
      non-interactive scripts (`bash -c "..."`) `[V]`.
    - launchd / systemd: Sees all filesystem modifications via kernel
      notifications (FSEvents/inotify) `[V]`.
  - *(b) Timing & Feedback:*
    - PATH shims: Command execution time.
    - PROMPT_COMMAND: Interactive prompt time.
    - launchd / systemd: Asynchronously AFTER writes have flushed to disk. Cannot
      veto writes; writer does not receive synchronous feedback `[V]`.
  - *(c) Atomicity:* N/A (post-write detection).
  - *(d) Platforms:* Heavily fragmented (macOS launchd vs Linux systemd vs shell
    types).
  - *(e) PKM Cost:* Fragile, invasive configuration for desktop users `[H]`.
  - *(f) Bypass routes:* Absolute binary paths, non-interactive shells, GUI
    editors `[V]`.
  - *(g) Failure mode:* Must fail-open to prevent breaking user shell sessions
    `[H]`.
  - *(h) Rust fit:* CLI can be called by scripts, but daemon management is out of
    scope for `traces`.
- **Verdict: Reject shell and OS daemon integrations.** High friction, fragile,
  fragmented, and strictly post-write.

---

### Group 5: Store-as-Authority Architecture

#### 18. Store as Authority (SQLite / Database Engine with Markdown Projection)

- **Precedent:** Anytype (<https://tech.anytype.io/>), Logseq DB version
  (<https://github.com/logseq/logseq/issues/10759>), Dendron, Foam `[V]`.
- **Stress-Test Analysis:**
  - *(a) Writers seen:* If a database is the authority, writes MUST execute
    through the store API. Untrusted writers cannot produce invalid states
    because DB constraints reject them at transaction commit `[V]`.
  - *(b) Timing & Feedback:* Synchronous pre-persistence.
  - *(c) Atomicity:* ACID transaction atomicity `[V]`.
  - *(d) Platforms:* Universal cross-platform.
  - *(e) PKM Cost:* Destroys the foundational premise of Markdown PKM ("plain
    text files on disk that I own, editable with any tool") `[V]`. Markdown
    becomes a read-only projection or export format. If loose files remain
    editable, two-way sync reintroduces conflicts and destroys the gate `[H]`.
  - *(f) Bypass routes:* Editing loose files if two-way sync is attempted `[V]`.
  - *(g) Failure mode:* Fail-closed.
  - *(h) Rust fit:* `rusqlite` is world-class in Rust.
- **Verdict: Reject for `traces` core storage (ADR-0001 / ADR-0009); adopt its
  architectural lesson.** Prevention requires mediated write boundaries.

---

## 4. Synthesis: Best Harness-Independent Designs

Based on empirical evidence and stress tests, here are the recommended designs
categorized by repository type:

### 1. Vaults Under Git

| Level | Mechanism | Mode | What must be true for it to work |
|---|---|---|---|
| **Prevention** | Dedicated Branch / Worktree + `traces merge` | Hard Gate | Agent is restricted to an isolated branch/worktree (`agent/<session>`). Canonical `main` is only updated via `traces merge <branch>`, which enforces `check_note(before, after)` before merging. |
| **Commit Gate** | Pre-Commit Hook on Staged Blobs (`traces check --staged`) | Hard Gate (local) | Vault is a Git repo. Hook inspects index blobs via `git show :<file>`. (Vulnerable to `git commit --no-verify`). |
| **Server Gate** | Pre-Receive Hook on Remote/Bare Repo | Hard Gate (absolute) | Changes are pushed to a bare/remote repo. Server-side pre-receive hook enforces schema; client `--no-verify` cannot bypass. |
| **Detection** | Notice on Read (Ticket 16 Tier 2) | Advisory | Any `traces` CLI/MCP invocation checks mtime/watermark of modified notes and returns structured diagnostics for any new violations. |
| **Remediation** | Linter Exit Code (`traces check <files>`) | Remediation Loop | Exit code 1 (schema violation) or 2 (class mismatch) with field, expected, found, and hint outputs for agent auto-repair loops. |

### 2. Vaults NOT Under Git

| Level | Mechanism | Mode | What must be true for it to work |
|---|---|---|---|
| **Prevention (macOS/Linux)** | APFS / Btrfs CoW Clone + `traces merge` | Hard Gate | Vault resides on APFS or Btrfs/XFS. Agent is launched inside an instant copy-on-write clone (`traces clone`). Edits occur in clone. `traces merge <clone>` validates all differences and promotes to canonical vault only if 100% valid. |
| **Prevention (Universal)** | Inbox / Quarantine Pattern (`Inbox/`) | Hard Gate | Canonical notes directory is treated as protected. Untrusted agents/scripts only write to `Inbox/`. `traces ingest` validates notes and moves valid ones to canonical paths. |
| **Prevention (CLI)** | Speculative Proposals (`traces propose` / `apply`) | Hard Gate | Agent outputs unified diff or JSON proposal. `traces apply <patch>` validates against schemas before updating disk files. |
| **Detection** | Validate on Read (Read-Time Advisory) | Advisory | `traces query`, `read`, or LSP queries attach diagnostic warnings to returned notes if schema violations exist. |
| **Remediation** | Exit-Code Linter (`traces check`) | Remediation Loop | Exit code 1/2 with structured diagnostics for agent self-correction. |

---

## 5. Architectural Conclusions for Ticket 16

1. **Reaffirmation of the Advisory Contract:** The decision in Ticket 16 to make
   the portable contract `check_note(before, after)` with detect-and-feed-back is
   100% validated. When untrusted processes possess write access to canonical
   files on disk, no in-place mechanism can synchronously prevent byte writes
   without severe instability or harness hooks.
2. **True Prevention Requires Namespace Isolation:**
   - For Git vaults: Worktree/branch isolation with gated merge (`traces merge`).
   - For non-Git vaults: APFS CoW clone sandbox (`traces clone` + `traces merge`)
     on macOS/Linux, or Quarantine/Inbox folders on all platforms.
3. **Clean Filters Rejected:** While Git clean filters with `required = true`
   block invalid `git add` and cannot be bypassed by `--no-verify`, their
   collateral damage (crashing `git checkout`, deleting working files, and
   crashing `git diff`) makes them unsuitable for desktop note vaults.
4. **Staged-Blob Inspection Adopted:** For Git pre-commit hooks, `traces check
   --staged` inspecting staged blobs via `git show :path` is the correct, robust
   implementation immune to working-tree dirty states.
