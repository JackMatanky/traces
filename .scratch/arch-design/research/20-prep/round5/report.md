# Ticket 16 round 5: claim checks and harness-independent guardrails

Inputs: `N-noyalib.md`, `R1-ecosystem-claims.md`, `R2-spec-claims.md`,
`G1-harness-capabilities.md`, `G2-mechanism-catalogue.md`. Tags: [V] read at
the primary source, [H] hypothesis, [P] proposal, [S] scout claim not re-read.

## 1. Scout claims: outcome

| Claim | Verdict | Matters to the Answer? |
| --- | --- | --- |
| +59% CST cost (Noyalib) | Our own 10k-iteration measurement ([V] in `19-prep/h-quote-escape-feasibility.md:57-73`), not an upstream number; cause is two internal parses (`cst/builder.rs:41-44`) | No: the quote convention is rejected on API grounds too |
| Noyalib upstream benchmarks | 17 suites exist; none compares `parse_document` with plain `Value` parsing | Gap stays; design in `N-noyalib.md`, not run |
| Dendron path binding "fatal" | Refuted: schemas matched dotted hierarchy, optional | No (not relied on) |
| Tana, SilverBullet | Verified: fields coexist, validation is non-gating; SilverBullet gates only with `mustValidate` | Supports advisory default |
| Metadata Enforcer v2 left JSON Schema | Verified (Python classes; expressiveness) | Supports "JSON Schema is export only" |
| Obsidian rewrites property YAML | Verified (forum 65851): quotes, comments, flow lists lost | Confirms rejection of quote convention |
| Logseq "users broke trees" | Refuted: multi-parent is a documented feature with cycle detection | Supports keeping multi-`extends` |
| LDAP MUST/MAY union, MUST wins | Verified (RFC 4512) | Supports order-independent rule |
| CUE unification commutative | Verified | Same |
| Kubernetes pruning backlash | Verified (#111074, KEP-2885) | Supports not dropping unknown keys |
| Plone behaviour field clobbering | Verified | Same family as `class-conflict` |
| Letta/Graphiti delta validation | Refuted: conflation by scout F | Delta rule rests on linters, not these |
| ADR-0009 vs Answer | Consistent | Confirms |
| `survey/schema.md` | Five statements superseded (see `R2`) | Add a banner when specified |

Remaining [S]: none that the Answer depends on.

## 2. The agent-hook claim, corrected

The Answer said a pre-write hook sees the edit instruction, not the resulting
file. G1 "verified" that for every harness. A spot check shows it is overstated:

- Claude Code `PreToolUse` for `Write` carries the full `content`
  (<https://code.claude.com/docs/en/hooks>, PreToolUse input) [V]. A whole-file
  write can be validated before it lands.
- `Edit` carries `old_string`/`new_string`; the result is a deterministic
  replace on the current file, so simulating it is cheap for string-replace
  tools [H]; patch-format tools (`apply_patch`) are harder [H].
- `Bash` edits (`sed`, redirects) carry only a command line [V]; Claude Code's
  PostToolUse `bashEditDiff` is best effort and beta, "not to enforce a policy"
  [V].

So the real limit is not "payload lacks content". It is that hooks are absent
in some harnesses, differ in every harness, and never see shell writes. The
user's ruling (do not design around hooks) stands, and is better grounded than
the old claim.

G1's other cells are mostly tagged [V] without quotes; I re-read only Claude
Code and Aider. Treat the matrix as [S] except where this report says [V].
Its counts are also inconsistent (the text says 8 hook harnesses and lists 9).

## 3. Mechanisms and stress test

Adversaries used: (A1) agent with shell and native edit tools, MCP present but
not forced; (A2) harness with no hooks, no MCP (Aider); (A3) human editing in
an editor; (A4) vault not under git; (A5) agent stops right after writing;
(A6) legacy notes with existing violations; (A7) a move or tag edit changes the
class set; (A8) sync tool or second process writes concurrently.

| Mechanism | Prevents | Survives | Breaks on | Verdict |
| --- | --- | --- | --- | --- |
| MCP validated edit tools | Yes, pre-write | A6, A7 (ask) | A1 unless native writes denied; A3 | Keep; only portable gate |
| MCP `check_note` dry-run tool | No | MCP-only harnesses | A5 unless called | Add; cheap |
| `traces check` CLI as lint command (Aider `--lint-cmd`, any post-edit command) | No, loop feedback | A2 [V] | A3, A5 | Keep; the A2 answer |
| Git pre-commit/CI on staged blobs | Commit time | A1, A2, A3 | A4, A5, `--no-verify`, never commits | Keep as backstop only |
| Validate on read plus changed-notes notice | No | A1, A2, A4 | A5 until next use | Add [P], see section 4 |
| LSP diagnostics | No | A3, editor agents | Terminal agents; no abort by protocol | Keep as feedback |
| Instruction snippet (`AGENTS.md`) | No | All harnesses | Probabilistic compliance | Generate it; never rely on it |
| Watcher daemon | No | A3 | Needs a running process; no channel to the agent; atomic renames | Reject as a product feature; LSP already watches |
| Staging dir plus `traces commit` | Only if the vault is unwritable | None alone | Without enforcement it is an instruction | Reject |
| OS sandbox / harness deny rules plus MCP | Yes | A1 | Not portable; heavy for desktop users | Document as opt-in recipe |
| ACP write interception | Yes | ACP clients | Custom client; shell writes bypass | Reject now |
| FUSE/overlay | Yes | None portable | macFUSE kernel dependency [H] | Reject |
| Auto-revert after a bad write | n/a | None | Destroys user data | Reject |
| Hooks (pre/post) | Partly | Some harnesses | Absent in several; shell writes invisible | Documented recipe, no design dependency |
| `traces new` / MCP create | Creation only | All | Later edits | Keep |
| `traces fix` | No | All | Cannot invent values | Defer; not a guardrail |

Lessons:

1. A gate is only as strong as the weakest open write path. An MCP gate that
   covers frontmatter but not body edits pushes agents back to native edit
   tools, which can change frontmatter too. A real gate needs every edit
   operation, or denied native writes.
2. No portable preventive gate exists for an agent that can write files. Only
   post-write detection is portable; prevention is opt-in per harness.
3. Detection helps only when it rides something the agent already does: a
   command it runs, a tool result it receives. A report file or daemon nobody
   reads is dead weight.
4. The delta rule needs a "before". Git is optional (A4), so `before` must come
   from an explicit path, git, or a stored baseline [P].
5. Check staged blobs (`git show :path`), not the working tree [V by G2 test].
6. Staging, auto-revert and watcher daemons look like guardrails but add
   machinery without a reliable channel; reject.

## 4. Recommended design [P]

One contract, several thin adapters. The contract is `check_note(before,
after)` with the structured violation (code, field, layer, hint) and the two
exit codes already in the Answer. Hosts decide how the contract reaches the
agent.

| Tier | Needs from harness | Channel |
| --- | --- | --- |
| 0 | Nothing | `traces init` can emit an `AGENTS.md` block; `traces check <paths>` exit code |
| 1 | Shell or post-edit command | Lint loop (`aider --lint-cmd`), git pre-commit on staged blobs, CI |
| 2 | Any `traces` call | Notice on read: notes changed since the last check that have new violations, one line, bounded |
| 3 | MCP client | Validated edit/create tools (the only gate), plus `check_note` dry-run (path plus proposed content) |
| 4 | LSP client | Diagnostics (feedback only) |
| 5 | Hooks, sandbox, deny rules | Recipes in docs, not features |

Tier 2 detail [P]: ADR-0009 refresh already finds changed files; a check
watermark plus the set of violation keys `(field, code, hash(value))` at the
last check gives a delta without git. Cost: a second derived store outside the
index and its fingerprint (schema must stay out of the index). Alternative with
no store: report all violations of changed notes with no delta. Ownership:
tickets 20, 21 and 27; 16 only requires that the contract can take a stored
baseline as `before`.

Stance for the Answer: "the portable guarantee is detect and feed back;
prevention exists only where `traces` owns the write or the harness is
configured to deny native writes."

## 5. Open items

- Per-harness matrix is unaudited beyond Claude Code and Aider; an Aider-MCP
  absence rests on a third-party summary [S].
- No benchmark run; Noyalib design only.
- Tier 2 baseline store is a proposal with an unmeasured cost.
- Whether omp `--no-tools` leaves MCP tools usable is [S] from G1.

## 6. Outcome

Folded into [ticket 16's Answer](../../../issues/20-schema-note-typing-and-validation.md)
(agent guardrail): hooks withdrawn as a design basis, tiered channels, MCP gate
strength rule, rejected mechanisms.
