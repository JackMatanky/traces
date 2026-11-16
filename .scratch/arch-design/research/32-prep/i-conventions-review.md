# Ticket 32: metadata indirection and derived-cache relocation

Retrieved: 2026-11-15. Sources below are official documentation or its upstream
source. These precedents are evidence, not votes or portability guarantees.
This research summary preserves risk caveats rather than claiming full STE.

## Checked precedents

### Git permits explicit metadata indirection

The [repository layout manual][git-layout] permits a plain text `.git` file:

> containing `gitdir: <path>` to point at the real directory that has the repository.

It names this mechanism a *gitfile*. Submodules and worktrees use it. The manual
explains that removing a submodule working tree must not lose its repository.
This is a durable-history requirement, not a disposable-cache requirement.

The [`git init` manual][git-init], under `--separate-git-dir`, says:

> This file acts as a filesystem-agnostic Git symbolic link to the repository.

It also says reinitialization moves the repository to the specified path.
This is an application-defined pointer file, not an OS symlink. It shows that
metadata indirection can be deliberate product design. It does not establish
that arbitrary metadata symlinks satisfy another application's confinement rule.

The [`git worktree` manual][git-worktree] says linked worktrees share everything
except per-worktree files such as `HEAD` and `index`. Its `repair` command can
reconnect manually moved main and linked worktrees. Thus, indirection introduces
relationships that the application sometimes has to repair.

The same manual describes `--relative-paths`:

> Link worktrees using relative paths or absolute paths (default).

The [worktree configuration documentation][git-worktree-config] identifies moves
between locations or environments as a use for relative links. Enabling them
also enables `extensions.relativeWorktrees`, making the repository incompatible
with older Git versions. Relocation support has a compatibility cost even here.
These documents do not promise that all Git metadata or derived state remains
fresh after every move, copy, filesystem change, or synchronization operation.

### Obsidian documents metadata-symlink risks, not a universal ban

The official [symlink documentation][obsidian-symlinks] says:

> We strongly advise against using symbolic links.

It specifically warns about symlinking things under `.obsidian/` to share them
between vaults:

> **has a high chance of corrupting your settings**

This advice concerns shared settings and update/synchronization behavior.
It is not proof that one confined, unshared metadata link causes corruption.
The documentation does not promise safe support for symlinking `.obsidian`
itself. This source establishes neither a guarantee nor a ban for that layout.

Obsidian also ignores in-vault folder links to avoid duplicate files and
ambiguous links. That addresses its note-tree semantics. It does not decide how
Traces should treat a metadata subtree that the scanner must exclude entirely.
The documentation permits external targets in some configurations. Traces'
root-confinement contract does not; copying that permission would change scope.

### Cargo separates derived artifacts from relocation promises

The Cargo Book's [build-cache chapter][cargo-cache] describes incremental output
as a cache that speeds subsequent builds. For exported `.d` dependency files:

> The paths in the file are absolute by default.

The [`build.dep-info-basedir` documentation][cargo-basedir] describes conversion:

> convert absolute paths to relative paths

This option strips a configured path prefix. The `.d` files serve external build
systems. Their path format is not a promise about Cargo's complete internal
freshness protocol. Neither statement proves that Cargo always rebuilds after
a root move, or that relative dependency files make all caches relocatable.

The useful precedent is narrower: derived artifacts can contain location-bound
state, while selected path formats offer explicit relative-path support.
Portable note keys alone do not prove that a store or replay journal can move.

### A Git repository is not a derived note index

The layout manual calls `objects` the repository's object store. Its submodule
example explicitly preserves the repository when removing the working tree.
The [Git FAQ][git-faq] separately warns that synchronization can corrupt objects
and refs. It requires quiescence for its suggested whole-repository transfer.
It also notes that `git status` usually rereads every working-tree file after
cross-system synchronization. Durable history portability does not imply reuse
of every filesystem-derived observation.

For Traces, reparsing notes can recreate the derived index. It cannot recreate
user-authored configuration, templates, notes, or Git history. That difference
makes rebuilding an acceptable policy option, not a user-data deletion policy.

## Recommendation 1: retain contained metadata symlinks

**Retain `.traces` symlinks whose resolved directory stays inside the admitted
root. Reject outside-root targets and unverifiable targets.**

This recommendation preserves the supplied current behavior. Rejecting all
metadata symlinks would be a new restriction, not a description of the binary.
Git supports deliberate metadata indirection, but it does not obligate Traces
to add Git's external metadata locations, pointer-file format, or repair tools.

A symlink alone is not a security problem when confinement holds. Outside-root
access, mistaken trust keys, and concurrent replacement are separate problems.
A blanket symlink ban does not remove the acknowledged check/use race elsewhere.
Do not claim it provides a hostile-concurrency security boundary.

Retaining contained links costs resolution, identity checks, and clear errors.
A blanket ban reduces supported layouts but breaks existing contained layouts.
The user benefit of a contained link is modest: it rearranges metadata inside
the vault; it cannot place metadata on an unrelated external disk.

Regardless of policy, exclude the opened metadata directory by filesystem
identity, including aliases reached through another name. Text case conversion
is not a filesystem-equivalence test. The supplied APFS `.traceſ` observation
establishes a macOS counterexample, not Windows or Linux behavior. This exclusion
is not deduplication of all note aliases and is not a complete mount sandbox.

Reject loops, dangling links, non-directory targets, and uncheckable confinement
with actionable errors. Do not remove their targets as a repair action. A target
that contains the vault root cannot become a metadata subtree without excluding
the note tree; containment and subtree admission must reject that arrangement.

**Reverse this recommendation** if supported-platform identity/exclusion checks
cannot reliably prevent indexing metadata aliases, or if product scope forbids
all metadata indirection. Record the compatibility change in either case.
If hostile concurrent mutation becomes a requirement, revisit the filesystem
access design rather than treating a symlink ban as sufficient protection.

## Recommendation 2: retain canonical-root binding and rebuild after relocation

**Keep canonical-root identity for the store and journal. Rebuild derived state
after a whole-vault move changes that identity. Do not add root rebinding now.**

This is a maintainability recommendation, not a claim that reuse is unsafe by
nature or that Cargo requires this policy. Relative paths can support validated
reuse. Current evidence does not establish its complete validation contract.

Rebinding must check more than portable row keys. It must reconcile the source
snapshot, path set, configuration/schema/parser/format identities, and replay
base. It also needs writer exclusion and consistent publication of the new
store/journal identity. Otherwise a partially completed rebind creates another
recovery state. Existing freshness checks can supply some of these checks;
rebinding is not automatically a second full parser pass.

A rebuild trades one cold refresh after relocation for fewer recovery states.
Preserve ticket 25's quiet rebuild heal. Record the root-change cause through
the existing `RefreshReport` and log channels, without unconditional stderr.

Do not call the user's notes corrupt. Invalidate only recognized
derived store/journal artifacts. Never recursively delete `.traces`, since it
also contains user files. Cache rebuild and user-file deletion are different.

A within-vault note rename does not change the canonical root. Handle it through
the normal path-set/freshness rules, not a whole-root relocation policy.
A different spelling that resolves to the same canonical root is also not the
root change described here. A copy is not proof of an unchanged source snapshot.

Cache reuse never transfers trust grants. Admit the new canonical root through
the existing root-keyed trust gate, even if a future cache rebind succeeds.
Conversely, a trust decision does not establish cache freshness. Rebuilding a
cache does not revoke or recreate grants for unrelated roots.

**Reverse this recommendation** if measured relocation rebuild costs materially
harm supported vault workflows, and validated reuse is cheaper in those cases.
Then define one atomic rebinding contract for both store and journal, test move
and copy failures on supported platforms, and keep trust grants separate.
A recurring portable-vault workflow would be stronger motivation than analogy
to Git history. No such workload measurement is established by these sources.

Ticket 27's supplied `mv` replay result remains valid prototype evidence.
Canonical-root binding would deliberately decline that optimization. Ticket 32
must record this policy change; it must not claim the old probe proved rebinding
or that moving notes necessarily changes their relative keys.

## Scope and checks

No source changes, runtime probes, builds, or test suites belong to this review.
The quoted upstream pages can change; the retrieval date identifies this review.
No cross-platform filesystem behavior was measured here.

The STE checker and `rumdl check` provide mechanical review, not certification.
STE warnings about sentence structure need contextual judgment. Research
caveats and checked source quotes take priority over controlled-language style.
The final check results accompany the handoff. This is research analysis,
not a controlled-language procedure.

[git-layout]: https://git-scm.com/docs/gitrepository-layout
[git-init]: https://raw.githubusercontent.com/git/git/master/Documentation/git-init.adoc
[git-worktree]: https://raw.githubusercontent.com/git/git/master/Documentation/git-worktree.adoc
[git-worktree-config]: https://raw.githubusercontent.com/git/git/master/Documentation/config/worktree.adoc
[obsidian-symlinks]: https://raw.githubusercontent.com/obsidianmd/obsidian-help/master/en/Files%20and%20folders/Symbolic%20links%20and%20junctions.md
[cargo-cache]: https://doc.rust-lang.org/cargo/reference/build-cache.html#dep-info-files
[cargo-basedir]: https://doc.rust-lang.org/cargo/reference/config.html#builddep-info-basedir
[git-faq]: https://raw.githubusercontent.com/git/git/master/Documentation/gitfaq.adoc
