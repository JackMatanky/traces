# Red-team pass: ticket 29 designs under attack

Goal: break the h1-recommended design (two grammars + candidate step +
per-predicate Exact-or-scan) before the grill adopts it. [P] marks a
proposed defense; [H] a hypothesis.

## Against the candidate-superset gate

1. Attack: a task-row filter `list.status_symbol == "x"` where note rows
   also carry a `list.` field of the same name. The probe gate proves a
   superset over note rows; task rows can disagree (the verified override
   at `src/query/results.rs:320-333`). A range key built from note values
   excludes a task row whose effective value matches. Defense: restrict
   range keys to note-mode rows. That is already the settled ticket 24
   wording; the grill must confirm the wording covers task-only queries
   (they fall to facet scan). [P]
2. Attack: `Not(filter)` with a proven superset. A range key on the inner
   atom proves the matching set; `Not` needs the complement, which the
   key cannot express. Current code falls back to full scan for `Not`.
   Defense: the protocol must state that `Not` above an Exact atom is
   Inexact (scan), never Exact. One sentence in the protocol. [P]

## Against range-key encoding

1. Attack: two fields, one table. The probe key form
   (`tag + value + NUL + path`) lets a folder scan on field A also match
   field B rows with the same value prefix if tags collide. Defense:
   include the field tag in the key before the value (already in the
   probe form). The grill must pin tag uniqueness in the encoding
   decision, since a collision silently mis-scans. [P]
2. Attack: values containing the NUL byte or the tag prefix. Text values
   come from frontmatter and can contain NUL in principle. Defense: keep
   the length-prefix variant (SQLite4 0x25-style) as the documented
   alternative, or reject NUL at admission. Cheapest: reject at
   admission, one notice. [P]
3. Attack: size numbers from different runs do not subtract. The
   three-table layout costs 33.7 MB at 50k in one commit [V]. A
   stage-per-commit build costs the same total [V]. Stages: files
   16.8 MB, text facet +16.8 MB, numeric facet +0 MB (reuses freed
   pages) [V]. The 24-prep case grew the file to 135 MB with three
   range tables over large text values. The grill can wrongly
   extrapolate any of the numbers. Defense: state totals and the
   commit-pattern dependence in the Answer's evidence section. [P]
4. Attack: the measured numeric encoding (sign-bit set for positives,
   all bits inverted for negatives) has edge values: NaN sorts above
   every number, and `-0.0` and `0.0` encode differently. A range query
   `>= 0` must match both spellings of zero. Defense: ticket 19 already
   excludes NaN (Null, never matches), so NaN never enters a key. At
   write time, normalize `-0.0` to `0.0` (add `0.0`) so one zero has
   one key. Cost: one branch per numeric value, already in the encode
   path. [P]

## Against the lazy QuerySet with overlay seam

1. Attack: the overlay swaps rows before memoization; a later chained
   terminal reuses memoized rows that no longer match the live overlay.
   Defense: the seam lives in the QuerySet construction (base rows
   before `Arc`), not after memoization; a new overlay state means a new
   QuerySet. One construction-site rule. [P]
2. Attack: memoized typed errors (ticket 21) memoize a failure caused by
   a transient overlay state. Defense: errors memoize per QuerySet, and
   an overlay change creates a new QuerySet, so the stale error cannot
   outlive its cause. Same rule as attack 1; write it once. [P]

## Against sort and GroupBy equivalence

1. Attack: `cmp_document_order` fixes ordering to position then
   `item_idx`. But GroupBy collapses rows and re-numbers. A filter
   inside a group over task rows after the fix needs a fresh proof.
   The equivalence claim "filter before group == filter after group"
   must be re-proved for task rows whose group key comes from the
   overridden metadata. The 24-prep list flags task layouts as
   unmeasured. Scope the claim to note rows, or add the proof to
   ticket 36's list. [P]
2. Attack: NaN and unlike-kind sort ranks interact with the proposed
   `member_of` predicate: a period value never equals Null, so rows
   without dates drop out of `member_of` silently. Defense: advisory
   notice when membership excludes Null-date rows, consistent with
   ticket 19's mismatch rule. [P]

## Attacks no design survives

1. Arbitrary user grammar extensions: any fixed IR admits a query the
   planner cannot prove; the facet-scan fallback is the ceiling, not a
   gap. State it as the design's boundary in the Answer.
2. Cross-facet joins (link targets as filterable rows): out of scope by
   ticket 24's settled shape; reject re-opening.

## New duties

1. The grill must decide tag-uniqueness and NUL admission for the
   range-key encoding; the probe does not settle them.
2. The task-row GroupBy equivalence stays unmeasured; either scope the
   claim or add it to ticket 36's measurement list.
