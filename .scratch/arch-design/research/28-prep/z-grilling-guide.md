# Grilling guide for ticket 28: live buffer and persisted index

Purpose: question order, evidence, options, and proposals per open question.
Do not answer for the user. Keep STE rules in the Answer.

## Order

Ask the questions in this order. Later groups reuse earlier answers.

1. Group 1 (scope): which host reads see buffers. This decides the overlay shape
   for every later group.
2. Group 2 (pinning): what a request pins; the attach point; the named
   generation pair.
3. Groups 3 and 4 (edits, save, close): revision and parse triggers, then the
   save and close invariants.
4. Groups 5 and 6 (deletion, rename, external change).
5. Group 7 (staleness bound interplay).
6. Group 8 (schema check placement).
7. Group 9 (multi-process recording).
8. Scenarios S1 to S9 as the wrap-up check.

## Measured and adversarial evidence

Two deeper evidence files back this guide:

1. `g1-probes.md` (shared): the `Arc` generation handout costs below 0.1 µs at
   50k; a naive overlay miss-check over a 50k-row result costs 0.69 ms per pass,
   about 0.6% of a cold query. The overlay is cheap at any of the recommended
   shapes. The overlay-aware candidate step adds about 64 µs for 10 overlay
   paths over a 50k candidate list, under 0.1% of a cold query.
2. `h2-redteam.md`: six attacks with defenses. Three decide questions. First,
   the save-drop rule: h1 assumed it silently, so the grill must decide it.
   Second, the new-unsaved-file candidate gap: the strongest case for overlay
   awareness inside the candidate step. Third, the rename re-key duty, which
   ties the overlay to ticket 32's portable key form. Ask the save-drop question
   in group 3.

## Per-group evidence and options

### Group 1: overlay truth and scope

Evidence: md-pkm-lsp chose a transparent overlay (issue 13) [V]. IWE has no
overlay and drops external events for open keys [V]. The CLI cannot see the
buffer [H].

Options:

- A. Transparent in-host: every query in a host process sees buffers. The CLI
  divergence is accepted and documented (S1).
- B. Opt-in per call: only overlay-marked reads see buffers. The template
  default must then error or notice (ticket 21 forbids silent empty rows).
- C. Wrapper generation (D3 of h1): the host merges buffers into an in-memory
  generation. The plan stays unchanged. Costs a merge prototype and a wrapper
  revision rule.

Proposal: A or C. B matches no committed consumer need. The stall evidence (IWE
v0.18.0) applies to A and C equally, so the debounce (ticket 33) is required
either way.

### Group 2: pinning and the attach point

Evidence: rows own `Arc<FileEntry>` from the pinned generation (ticket 24) [V].
Each result names incarnation + revision (ticket 25) [V]. A replay must change
the revision (ticket 27) [V]. Rust-analyzer gives each request an immutable
snapshot, generation clone 120 µs at 50k [V].

Options for the attach point:

- A. Overlay entry keyed by path, checked at row assembly. After a refresh
  publishes a new generation, entries re-attach by path or drop when the path
  left the generation.
- B. Overlay swaps the row list before memoization (the ticket 29 seam). Rows
  stay immutable.

The generation pair rule: buffer-affected results name the pinned pair.
Whether the overlay entry carries its own revision is the open sub-question.
The cheapest rule: one pair per result; the buffer revision rides the entry
and shows only in notices [P].

### Groups 3 and 4: edits, parse, save, close

Evidence: `NoteDoc::scan` is about 29% of parse (ticket 23) [V]. The host owns
text, version, debounce, revision gate, UTF-16 (ticket 23) [V]. IWE drops
unsaved new documents at close [V]. The `racy` bit with the 2 s margin covers
the save race (ticket 25) [V].

Proposals:

- Parse trigger: on debounce, into the overlay; never per keystroke.
- Save invariant: the overlay entry stays until the next refresh proves the
  store row matches the saved bytes. Proof is the stat triple plus racy rescan;
  no new mechanism.
- Close without save: adopt the IWE drop rule for new documents; for edited
  existing files, keep the overlay until close ends, then drop and let the
  watcher see the disk state. State the divergence from IWE explicitly.

### Groups 5 and 6: deletion, rename, external change

Evidence: rename needs the portable path form (ticket 32, open) [V]; positions
never drive edits (ticket 23) [V]; ticket 34 owns notice text.

Options for external edit of an open key:

- A. IWE rule: drop events, editor owns truth until close. Divergence window =
  close-to-refresh.
- B. Observe and warn: the host reports disk-vs-buffer difference through a
  notice; it never adopts disk content into the buffer.

The race with a second writer: the IWE MCP crate writes disk. The overlay never
enters the store, so the store stays consistent by the lock rules of tickets 24
and 25. The user-visible divergence is a notice, not a merge.

### Group 7: staleness bound interplay

Proposal to test with the user: an overlay-answered path checks sources inside
the bound only when the buffer is closed. An open buffer is its own truth, so no
disk check adds information. The racy rescan still runs at refresh. Record the
answer as the staleness-bound semantics ticket 25 handed here.

### Group 8: schema check placement

Evidence: `check_note(before, after)` takes path and content (ticket 20) [V];
diagnostics are feedback only; `didChange` cannot deny [V].

Proposal: the check consumes the debounced parse output, not the keystroke. The
host maps violations to publish_diagnostics; core stays protocol-free.

### Group 9: multi-process

Recording duty only: redb rejects the second writer at open [V]. The host holds
the store only during refresh and persist (ticket 25) [V]. `S9` resolves by "the
terminal waits or reads fresh parse, the overlay stays in-memory". No new rule
is legal here.

## Scenario quick answers to check with the user

| S | Deciding rule (candidate) |
| --- | --- |
| S1 | Group 1 choice; CLI sees store only; divergence documented |
| S2 | racy bit + 2 s rescan at next refresh (ticket 25) |
| S3 | drop overlay entry; no store row; no journal fact (ticket 26) |
| S4 | overlay path absent from generation: row drops; path-set cost cited |
| S5 | rename: re-key entry; positions invalid; wait on ticket 32 form |
| S6 | Group 6 choice; divergence window stated; notice to ticket 34 |
| S7 | admitted request keeps its pinned revisions (ticket 21) |
| S8 | replay advances revision; entries re-attach or drop; result names new pair |
| S9 | redb rejects second writer; overlay never enters store |

## Cross-ticket consistency checks

1. 28-29 seam: the overlay swap happens before memoization; rows immutable.
2. 28-32: rename re-keying uses the portable form; do not preset bytes.
3. 28-33: debounce and watcher wiring stay in 33; semantics here.
4. 28-25: one mechanism per race; racy bit covers save; no second channel.
5. 28-36: unsaved facts never reach the journal or the store.

## Change-table candidates to record in the Answer

- Host answers include unsaved content: new observable surface (not a CLI
  regression).
- Close-without-save drop rule (if adopted from IWE): host behavior row.
- Any divergence notice from S6: ticket 34 owns the text row.
