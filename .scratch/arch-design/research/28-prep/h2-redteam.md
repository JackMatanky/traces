# Red-team pass: ticket 28 designs under attack

Goal: break the h1-recommended designs before the grill adopts them. Each attack
lists the design it targets, the break, and the cheapest defense. Verdict marks:
[P] proposed defense, [H] hypothesis.

## Against the transparent overlay

1. Attack: save the buffer, then edit again before refresh runs. The overlay
   still holds the post-save content. A query between the second edit and the
   next debounced refresh returns stale text even though the file on disk is
   newer than the generation. Defense: on host save, drop the overlay entry for
   that path and let the racy rescan pick the file up. Costs one window of store
   staleness. The bound of tickets 25/33 already covers that window. [P]
2. Attack: the buffer holds a path the generation does not know (new unsaved
   file). Any plan that enumerates `files` misses it; a facet scan misses it;
   only an overlay-aware candidate step can admit it. This is the strongest
   argument for overlay awareness inside the candidate step, not only at row
   assembly. [P]
3. Attack: the user renames the file externally while the buffer is open. The
   overlay keys on the old path; the generation now holds the new path. A query
   sees the file twice (old path from overlay, new from generation) or zero
   times, depending on drop rules. Defense: the overlay entry must re-key on the
   watch rename event, which makes the overlay depend on ticket 32's portable
   key form. Record as a cross-ticket duty. [P]

## Against the wrapper generation

1. Attack: the wrapper clones the 50k row list to swap one entry. The probe says
   the `Arc` handout is free (below 0.1 µs), but a per-row copy of the swapped
   slice is not. The design must swap by re-wrapping a copied-slice only for the
   touched range or by an enum row (Generation | Overlay) instead of copying.
   Cheapest: enum row, one allocation-free branch per lookup. [P]
2. Attack: two consecutive queries with different buffers open must not observe
   different orderings. The wrapper preserves the base order (position then
   `item_idx`), so ordering holds. The risk is only a result naming an
   incarnation that no longer exists after refresh. Defense: the wrapper carries
   the same incarnation and bumps the revision, matching ticket 27's replay
   rule. [P]

## Against "overlay drops at save"

1. Attack: the user saves and immediately queries from the CLI in another
   terminal. Dropping at save means the CLI reads the store before the debounced
   refresh lands; the CLI answer misses the save. This is inherent: no
   cross-process overlay exists (S9). The defense is documentation: save-to-CLI
   visibility is bounded by the refresh cycle, and ticket 33 states the bound.
   [P]
2. Attack: the editor writes the file through a different path spelling
   (symlinked vault root). The store row exists under the real path; the overlay
   key under the symlink spelling. After drop-at-save the rescan must reconcile
   the two spellings; ticket 32's NFC-plus-realpath rule for trust keying must
   extend to row keying, or the row duplicates. [P] Record for ticket 32.

## Attacks no design survives (accept and document)

1. Two machines, one file system (network mount): no overlay, no lock, no racy
   margin can make the bound hold. Out of scope by ticket 16's single-machine
   tier; state it in the Answer.
2. A second process writing the store concurrently: redb's single-writer open
   already rejects it; the terminal path reads fresh parse. S9 stands.

## New evidence duties this pass creates

1. Measure nothing new: the probe already bounds the overlay cost (g1-probes.md,
   0.69 ms naive per-pass at 50k; the candidate step with 10 overlay paths adds
   about 64 µs).
2. The grill must decide the save-drop rule explicitly; both h1 options assumed
   it silently.
