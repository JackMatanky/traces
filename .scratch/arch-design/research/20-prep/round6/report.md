# Ticket 16 round 6: harness-independent guardrails

Inputs: `H1-filesystem-layer.md`, `H2-vcs-and-workflow.md`,
`H3-prior-art-and-in-band.md`. Tags: [V] read or observed by a worker (I did
not re-run the experiments), [H] hypothesis, [P] proposal.

## 1. Finding

Across 17 surveyed systems and 18 mechanisms, nothing prevents an invalid write
by a process that can write the canonical files directly [H, supported by every
precedent in H3 and by the experiments in H1 and H2]. Systems that do prevent
(mem0, Keystatic, Decap, the Anthropic memory server, Kubernetes admission) all
remove direct write access: the store, not the writer, is the authority.
Harness-independent prevention therefore means placement (keep the writer away
from the canonical store) or mediation (a filesystem that checks), not a
filter on the existing files.

## 2. Candidates and stress test

| Candidate | Prevents | Stress-test result | Verdict |
| --- | --- | --- | --- |
| Isolated working copy: git worktree or branch; APFS `clonefile` clone (under 1 ms, unprivileged [V]) plus gated `traces merge` | Yes, by placement | Holds only if the writer is launched inside the copy; same-user processes still reach the canonical vault; concurrent edits need a base hash per file; user must learn a workflow | Opt-in workflow; best preventive option |
| `_inbox/` folder plus `traces promote` | Yes, by convention | Class binding by path needs the destination path at check time; unprotected folder means convention only | Opt-in workflow; cheapest |
| Validated CLI write commands | Path, not barrier | Works for any agent with a shell; bypassed by native writes | Adopt (shares the MCP use case) |
| Loopback NFS (`nfsserve`) mount | At `rename` only | Unprivileged `mount_nfs` worked and a rename was rejected with `EACCES` [V]; in-place `>` is not gateable and exits 0 [V]; backing directory stays writable; daemon death hangs processes unless soft-mounted; FSEvents-based watchers (Obsidian) on NFS unverified [H] | Documented recipe at most |
| `chflags uchg`, ACLs | Yes | Same UID can undo it [V] | Reject |
| Git clean filter | Yes at `add` | Breaks `checkout` and `diff` when the schema or filter fails [V] | Reject |
| `reference-transaction` hook | No | Sees ref updates and hashes only, not staged blobs [V] | Reject |
| Pre-commit on staged blobs; server `pre-receive` | Commit or push time | `--no-verify` defeats the first, not the second [V] | Backstop (already adopted) |
| FUSE/macFUSE/FSKit, WebDAV, 9P, ProjFS, Endpoint Security, fanotify | Varies | Kernel extension, Apple entitlement, fragile mounts, or no write-content gate [V] | Reject |
| Store as authority (SQLite projection) | Yes | Gives up plain files | Reject |
| In-note hints (YAML comments, `$schema`) | No | Obsidian strips comments [V]; `$schema` adds a key to every note | Reject |
| Per-folder `AGENTS.md` generated from `[binding].paths` | No | Lazy load verified for Claude Code only [V]; other harnesses unknown | Adopt as soft steering |
| Tolerant reader | No | Already the design: schemas never type values at parse or query time | Nothing to add |

## 3. Lessons

1. Where the writer cannot be controlled, the only reliable lever is where it
   writes. Placement workflows are enforced by launch convention, not by the
   OS, unless combined with a sandbox or deny rule.
2. The two filesystem gates that do work (NFS at rename, `uchg`) each leave an
   escape that matters for agents: shell redirects, or same-UID bypass.
3. Every gate that rejects a write needs a stated failure mode: fail closed for
   a broken schema (exit code), never silently pass.
4. The workflow options push one requirement into core: `check_note` over a
   change set (many notes) with a per-file base, plus the destination path.

## 4. Open items

- No experiment re-run by me; NFS and clone results are worker observations.
- Windows and Linux behaviour of the placement workflows is unmeasured.
- Product features (session, inbox, write commands) are outside the map's
  destination; only their design pressure on core is recorded in ticket 16.

## 5. Outcome

Folded into [ticket 16's Answer](../../../issues/20-schema-note-typing-and-validation.md)
(agent guardrail): placement workflows, CLI write commands, per-folder
instruction snippets, the rejected list. MCP-server guards:
[addendum I](./I-mcp-server-guards.md).
