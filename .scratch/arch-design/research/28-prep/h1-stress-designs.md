# Stress-test designs for ticket 28: live buffer vs persisted index

Rules: candidate designs, requirement checks, failure modes, inconsistencies.
Tags [V]/[H]/[P]. Cite `path:line` or ticket lines. Do not preset the Answer.

## Candidate designs

### D1: transparent overlay, all queries see buffers

A `RwLock<HashMap<PathKey, OverlayEntry>>` sits beside the pinned generation.
`OverlayEntry { revision, parsed: Note, inlinks_patch }`. Query path resolution
checks the overlay before the store row.

Checks:

- R-tickets 24/28: rows read `Arc<FileEntry>` from the pinned generation. The
  overlay must synthesize a full `FileEntry` per buffered path, or the row type
  needs an overlay variant. That variant is ticket 29 scope [H].
- R-ticket 26: the overlay never persists, so the journal rule holds by
  construction.
- Failure mode: a buffered path that the store does not know (new file). The row
  set grows beyond the generation path set; `cmp_document_order` needs a
  position for it. Positions come from the store sort; an overlay-only row has
  none [H]. Candidate rule: assign a stable synthetic order after the last store
  position.
- Failure mode: parse per keystroke. Ticket 23 promises `NoteDoc::scan` at about
  29% of parse time [V], and IWE stalled on per-keystroke rebuilds [V]. D1 needs
  the debounce (ticket 33) to bound parse rate; the overlay stores the last
  parsed revision, not the keystroke.
- Failure mode: link coherence. Outbound links of the buffer change inlink rows
  of other files. A full patch of the link multimap per edit is the store's job.
  The overlay can patch only reads that resolve inlinks for the buffered path.
  Cross-file inlink reads stay stale until save [H]. md-pkm-lsp issue 14
  accepted the same scope: patch on demand, full refresh at save.

### D2: overlay through an explicit parameter

Only calls that pass an overlay view see buffers. The default stays the pure
pinned generation.

Checks:

- R-ticket 24: one execution plan. A parameter adds a row source, not a plan
  shape. Sound.
- R-ticket 16: CLI answers stay pure store answers. The divergence rule of
  scenario S1 becomes "host callers choose". Simplest to state.
- Failure mode: template calls forget the parameter and silently serve stale
  rows. Ticket 21 already forbids silent empty results; a memoized error or a
  notice is needed [H].
- Failure mode: two answers for one question inside one host. md-pkm-lsp chose
  transparent (issue 13) because diagnostics and completion both need the buffer
  without plumbing an extra parameter through every call site [V].

### D3: overlay as a generation wrapper

The host rebuilds an in-memory generation that merges store rows and buffered
rows, then pins it like any generation. No overlay branch in the plan.

Checks:

- R-ticket 24: the plan is unchanged. The wrapper is a `WorkspaceIndex`
  replacement for reads.
- Failure mode: rebuild cost. A full rebuild per edit is the IWE v0.18.0 stall
  [V]. A merge of one file's delta is cheap [H]. Research 11: one added file
  costs 499 ms at 50k only because every note decodes. A merge that replaces one
  row avoids that. Needs a measured prototype.
- Failure mode: generation identity. Tickets 25/26/27 name results by
  incarnation + revision. A wrapper generation needs its own revision rule.
  Ticket 27 says a replay must change the revision; a merge must too [H].
- This design matches rust-analyzer's host/snapshot shape [V] (research
  24-prep). It makes the overlay invisible to tickets 29 and 31, which is its
  main attraction.

## Requirement table

| Req | Source | D1 | D2 | D3 |
| --- | --- | --- | --- | --- |
| rows own `Arc<FileEntry>` | t24 | needs variant | ok via param | ok if wrapper rows are full entries |
| one execution plan | t24 | ok | ok | ok |
| no unsaved fact persists | t26 | ok | ok | ok |
| result names generation | t25 | needs rule | names base | needs wrapper revision |
| template default correct | t21/31 | ok transparent | silent-stale risk | ok |
| rename across key change | t28/t32 | key remap | key remap | wrapper rebuild |

## Inconsistencies to resolve at grilling

1. S7 vs D1: ticket 21 says an admitted request keeps its generation. If the
   overlay is transparent, the request also pins buffer revisions. State which
   revisions; ticket 28 Group 2 asks exactly this.
2. S8 vs D3: after a replay the wrapper must detach from the pre-crash
   generation and re-merge. Any rule must reuse the racy rescan of ticket 25
   rather than invent a second mechanism.
3. The staleness bound (ticket 25, group 7) vs overlay-answered queries: if the
   overlay answers, does the host still owe a source check for that path inside
   the bound? The cheap answer is yes-check-for-buffered-paths-only. IWE does
   the opposite (drops external events for open keys) [V]. The grilling must
   pick and record the divergence.
4. Scenario S2 already resolves through the `racy` bit + 2 s margin. Do not add
   a save-time invalidation channel on top; one mechanism per race.
