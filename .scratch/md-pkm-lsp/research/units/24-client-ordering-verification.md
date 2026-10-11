# Ticket 24: Helix / Neovim-core / nvim-cmp completion ordering verification (Q6, ST-K4)

Verification pass closing ST-K4 (`24-stress-test-resolve-ranking.md:187-201`): Q6's contract —
"clients re-sort completions by their own fuzzy score; our `sortText` is absolute only at empty
input; `preselect` honored where supported; `filterText` word-initial invariant" — was verified
only for VS Code + Zed (`24-zed-client-reality.md`). This unit verifies (or corrects) it for the
remaining three rungs: **Helix**, **Neovim core**, **nvim-cmp**.

- **Date**: 2026-10-07 (sources fetched live this date; no local digests exist for any of the
  three — `docs/digests/` has none).
- **Snapshots** (GitHub HEAD at fetch time; citations are upstream-relative path + line against
  these):
  - helix-editor/helix `master` @ `ba40e547426b0f9896c8bdc699a4ab11f2b37dbc` (2026-09-29)
  - neovim/neovim `master` @ `7e1ae5115f1f7088abe329892948708fc809d45e` (2026-10-09) — **and**
    tag `v0.12.6` (latest release, 2026-10-09) where master has diverged
  - hrsh7th/nvim-cmp `main` @ `2ffe79f1f021def8dd1fcd81deb16f1bb0d989f3` (2026-07-09);
    hrsh7th/cmp-nvim-lsp `main` (fetched same date) for the client-capability side
- **Legend**: **V** = verified against the fetched source this pass; **[correction]** = Q6/ST-K6
  wording must change.

---

## 1. Helix (sparse-floor client)

All ordering lives in one place: `Completion::score`
(`helix-term/src/ui/completion.rs:323-380`). The menu (`Menu<T>`, `helix-term/src/ui/menu.rs:21-39`)
never sorts; it only renders `self.matches` in the order `score()` produced
(`menu.rs:346-353`).

### The sort key

```rust
matches.sort_unstable_by_key(|&(i, score)| {
    let option = &options[i as usize];
    (
        score <= min_score,            // 1. weak-match bucket (demoted to bottom)
        Reverse(option.preselect()),   // 2. LSP preselect=true hoisted to top
        option.provider_priority(),    // 3. per-language-server bucket
        Reverse(score),                // 4. nucleo fuzzy score, DESC
        i,                             // 5. response array index (original order)
    )
});
```
(`helix-term/src/ui/completion.rs:370-379`; cutoff `min_score = (7 + needle_len * 14) / 3` at
`:369`.)

### Ordering at empty input — **sortText is never read; array order is absolute**

- **V** `sortText` appears **nowhere** in Helix's completion path. The only filter-text accessor is
  `filter_text()` = `item.filter_text` else `label` (`helix-term/src/handlers/completion/item.rs:61-67,79-84`);
  the sort key contains no `sort_text` field, and grepping the whole helix-term completion
  surface finds zero `sort_text`/`sortText` references.
- **V** The filter string is the word-char fragment before the cursor
  (`helix-term/src/ui/completion.rs:300-307`) — empty on trigger-char/manual open (`[` is not a
  word char), so `[[` opens the menu at empty input.
- **V** Empty needle → nucleo scores every item `Some(0)`
  (nucleo-matcher 0.3.1, `src/lib.rs:220-223`, `fuzzy_matcher_impl`; Helix pins
  `nucleo = "0.5.0"` → nucleo-matcher 0.3.1 per helix `Cargo.lock`). `min_score` = 2 at len 0, so
  `0 <= 2` puts **all** items in the demoted bucket uniformly — the cutoff is a no-op, not a
  reorder.
- **⇒ Visible order at empty input** = preselect (see §1 preselect — effectively absent) →
  provider-priority bucket (`-(config_list_index as i8)`, `helix-term/src/handlers/completion/request.rs:203,239`;
  uniform for a single attached server) → **response array order** via the `i` tiebreak.
  **[correction]** "sortText is absolute at empty input" is not Helix's mechanism — Helix never
  computes it. What is absolute is *the order the server returned items in*. For Traces (single
  server, items already emitted in group order) the observable result is identical; the
  difference is that a client-side merge that changes array position cannot be repaired by
  `sortText` the way it can in VS Code/Zed.

### Ordering under typed query — fuzzy score sits above array order; sortText still unread

- **V** Same key. nucleo `AtomKind::Fuzzy` with `prefer_prefix = true`
  (`helix-term/src/ui/completion.rs:326-335`) matched against `filterText` (`:341,354`). Score
  DESC (rung 4) outranks array index (rung 5); below-cutoff matches fall under all good matches
  of any provider (`:360-369` — explicit comment that this lets server-2's good matches escape
  below server-1's bad ones). `sortText` remains unread at every keystroke.
- **V** Quirk (not load-bearing): incremental rescoring halves scores (`:345`, `/2`) while full
  rescoring divides by 3 (`:357`), shifting the cutoff boundary after each keystroke; relative
  order within a pass is unaffected.

### preselect — honored in the sort, but not advertised; menu opens unselected

- **V** `preselect()` reads the LSP item field (`item.rs:121-126`); rung 2 hoists
  `preselect = true` items **above the fuzzy score** (within the good-score bucket,
  `completion.rs:374`).
- **V [correction]** Helix does **not** advertise `preselectSupport` — client capabilities set
  only `snippet_support`, `resolve_support`, `insert_replace_support`, `deprecated_support`,
  `tag_support` (`helix-lsp/src/client.rs:638-652`, `..Default::default()`). Spec-gating servers
  will therefore never send it; un-gating servers do (rust-analyzer sets
  `res.preselect = Some(true)` on its max-relevance item unconditionally —
  `crates/rust-analyzer/src/lsp/to_proto.rs:480-486` — so RA's best item is hoisted to Helix's
  row 0 regardless of capability).
- **V** Even when hoisted, preselect does not auto-accept: the menu opens with `cursor: None`
  (`menu.rs:56`; `set_completion` never calls `move_down`, `helix-term/src/ui/editor.rs:1101-1121`)
  — nothing is selected until Tab/C-n/Down. Preselect = "sorts first", not "preselected".
- ⇒ ST-K6's "preselect no-ops in Zed/Helix" is half right: it no-ops *by capability*, but if a
  server ignores the capability Helix will honor it in ordering (helpful for the 23 flow, since
  order-then-select-by-user still lands on row 0).

### filterText

- **V** Matched by nucleo fuzzy (sequence match, prefix preferred) against the typed fragment
  (`completion.rs:341,354`; `item.rs:79-84`). A non-word-initial split word in `filterText` is
  *not* demoted the Zed/`OtherMatch` way — any subsequence matches; word-initial merely raises
  the score. The R-SHARED-FILTER-WORD invariant is a ranking aid in Helix, not a visibility
  requirement.

---

## 2. Neovim core (`vim.lsp.completion` + in-sexpand C core)

The pipeline: `vim.lsp.completion` (Lua) converts + sorts LSP items, then hands them to
`vim.fn.complete(start_col, matches)` → `set_completion()` (C) which links them into the popup
list **in list order** (`src/nvim/insexpand.c:3505-3575`). The Lua comparator:

```lua
local function compare_by_sortText_and_label(a, b)
  local itema = a.user_data.nvim.lsp.completion_item
  local itemb = b.user_data.nvim.lsp.completion_item
  return (nonempty(itema.sortText) or itema.label) < (nonempty(itemb.sortText) or itemb.label)
end

local use_fuzzy_sort = has_completeopt('fuzzy')
  and not has_completeopt('nosort')
  and not result.isIncomplete
  and #prefix > 0

local compare_fn = use_fuzzy_sort
    and function(a, b)          -- fuzzy score DESC, tie -> sortText/label
      ...
    end
  or compare_by_sortText_and_label
table.sort(candidates, compare_fn)
```
(`runtime/lua/vim/lsp/completion.lua:635-657`; identical logic in tag `v0.12.6` at `:463-485`.)

### Ordering at empty input — **sortText is absolute (holds; stronger than Q6)**

- **V** `#prefix > 0` is a hard gate on the fuzzy comparator (`completion.lua:644`) — at empty
  input the comparator is **always** `sortText ASC, fallback label ASC`, `completeopt` flags
  notwithstanding. Additionally the item filter passes everything when the prefix has no word
  char (`:523-526`) and `item_matcher` returns `true, nil` on empty prefix (`:334-337`).
- **V** The C core adds no reorder for `complete()`-fed lists: `set_completion` inserts matches
  in list order (`insexpand.c:3546-3549` via `ins_compl_add_list`, `:3447-3459`; FORWARD-linking
  at `:1148-1163`) and the only post-hoc sorts run in the ^X-scan path
  (`ins_compl_get_exp`, `:5013-5019`), which this path never enters.
- **⇒ Q6's "sortText absolute at empty input" is verified for Neovim core — and with the
  default `completeopt` ("menu,popup", `runtime/doc/options.txt:1659`) it is absolute *even
  under a typed query***, because the fuzzy rung requires the opt-in `fuzzy` flag.

### Ordering under typed query — score-above-sortText only behind explicit flags

- **V** `completeopt+=fuzzy` (opt-in): primary = Lua fuzzy score from `matchfuzzypos` on
  filterText/label (`completion.lua:334-343, 530-533, 646-654`), tie → sortText. The rung
  reverts to sortText when the result `isIncomplete` (`:643`) or `nosort` is set (`:642`).
- **V** `completeopt+=nearest`: C-side `sort_compl_match_list(cp_compare_nearest)`
  (`insexpand.c:5013-5016`, compare at `:1514-1522`) sorts by proximity score — but scores are
  computed only for current-buffer matches (docs: "applies only to matches from the current
  buffer", `runtime/doc/options.txt:1696-1700`); LSP items keep `FUZZY_SCORE_NONE`
  (`insexpand.c:3441`) so the comparator returns 0 for any LSP pair and stable mergesort
  (`:1631-1663`) preserves the Lua sortText order among them.
- **V** `completeopt+=fuzzy` *and* continued typing inside an active session: C re-scores the
  whole list with `fuzzy_match_str` and re-sorts (`ins_compl_new_leader` →
  `ins_compl_fuzzy_sort`, `insexpand.c:2445-2463`, scores at `:1598-1627`) — until the next
  `vim.fn.complete()` rebuild replaces it. Momentary, real.
- **V** Multi-client: each client's batch is sorted internally, then batches are concatenated in
  `pairs(responses)` iteration order with **no global merge sort**
  (`completion.lua:1126,1163,1177-1194`). sortText is absolute *within* a server's batch, not
  across servers. A user-supplied per-buffer `cmp` comparator replaces both sorts entirely
  (`:632, :1183`; field doc `:1280`).

### preselect — version-split; opt-in even on master

- **V (0.12.x, current release)**: client capability is `preselectSupport = false`
  (`runtime/lua/vim/lsp/protocol.lua` tag v0.12.6 `:476-479`) and the C core has **zero**
  preselect handling (tag v0.12.6 `insexpand.c`: no `preselect` symbol; the Lua layer does not
  pass the field either). preselect is a complete no-op on every shipped 0.12.
- **V (master / 0.13-dev)**: `preselectSupport = true` (`protocol.lua:489-492`);
  `item.preselect` is copied into the complete-item dict (`completion.lua:611`); the C core
  honors it **only when `completeopt` contains the new `preselect` flag** —
  `compl_preselect_match` is gated on `kOptCotFlagPreselect` (`insexpand.c:1075-1078`), applied
  at `set_completion` (`:3554-3557`), docs state it "Takes precedence over 'noselect'"
  (`runtime/doc/options.txt:1721-1729`). Default `completeopt` does not include the flag.
- **V Fallback that preserves ticket-23's flow**: without `noselect`, the C core auto-selects the
  **first** item (`insexpand.c:3563-3565`) = our sortText-first row; with `noselect` — which
  `vim.lsp.completion`'s own header recommends (`completion.lua:9-11`) — nothing is selected
  unless the `preselect` flag is also set. **[correction]** ST-K6's "preselect honored by
  VS Code/Neovim" must split: honored by Neovim ≥0.13 *with* an opt-in completeopt flag; on the
  installed-base 0.12 it is capability-refused + unimplemented; the landing-on-row-0 behavior on
  0.12 comes from implicit first-item selection, not from preselect.

### filterText — word-initial invariant is load-bearing here

- **V** Matching runs on `filterText` (fallback label) with `vim.startswith`
  (`completion.lua:530-541`, matcher `:324-346`; case-folded only under `ignorecase`). At a
  non-empty prefix, an item whose `filterText` does not **start with** the typed prefix is
  dropped from the popup entirely (unless the item has a `textEdit` whose `newText` is absent —
  `:536-539` server-filtered). With `completeopt+=fuzzy` it relaxes to a sequence match
  (`matchfuzzypos`, `:339-341`). ⇒ R-SHARED-FILTER-WORD (natural typed form word-initial in
  `filterText`) is required for *visibility* on default Neovim — the strongest form of the
  invariant among the five clients.

---

## 3. nvim-cmp (plugin, primary Neovim completion UI for many users)

### The default comparator chain

```lua
sorting = {
  priority_weight = 2,
  comparators = {
    compare.offset, compare.exact, compare.score,
    compare.recently_used, compare.locality, compare.kind,
    compare.sort_text,          -- 7th!
    compare.length, compare.order,
  },
}
```
(`lua/cmp/config/default.lua:67-81`.) Applied by `table.sort` over each source-group's merged
entries (`lua/cmp/view.lua:108-117`).

- **[correction]** There is **no `compare_timestamps`** comparator in nvim-cmp. The
  timestamp-based one is `compare.recently_used`, keyed on `vim.loop.now()` records of accepted
  labels, default `-1` (`lua/cmp/config/compare.lua:52-67`).

### Ordering at empty input — **sortText does NOT control order**

- **V** The matcher short-circuits empty input with a constant score for every entry —
  `PREFIX_FACTOR + NOT_FUZZY_FACTOR` (= 14) (`lua/cmp/matcher.lua:86-91`) — so `compare.score`
  ties and is a no-op on the base score.
- **V** But source priority is **added into `entry.score`** before sorting
  (`view.lua:96-100`: `e.score = e.score + priority`, default bonus
  `(#source_group - i + 1) * priority_weight`, or the source's configured `priority`). ⇒
  across sources, empty-input order = source priority, decided *before* any comparator
  (LSP-vs-buffer etc.).
- **V** Within a source at empty input, first differentiators in chain order:
  1. `recently_used` — an LSP item accepted earlier this session jumps above everything
     (`compare.lua:52-67`) regardless of `sortText`;
  2. `locality` — items whose inserted `word` appears near the cursor in the buffer are boosted
     (`compare.lua:136-197`, 10-line window);
  3. `kind` — CompletionItemKind ordinal, Snippet forced first, Text forced last
     (`compare.lua:73-93`). **Our per-kind display choice reorders the menu here, above
     sortText** (e.g. a `Function`-kind row outranks a `Reference`-kind row whatever its
     sortText);
  4. only then `sort_text` — and only if **both** entries carry a (non-nil) `sortText`, compared
     case-insensitively via `vim.stricmp` (`compare.lua:97-107`);
  5. `length` (shorter label), `order` (`entry.id` ≈ arrival order) finish.
- **⇒ [correction]** For nvim-cmp, "sortText is absolute at empty input" is **false**. It is the
  7th comparator; recency, locality, kind, and source priority dominate. Q7
  (`24-completion-item-shape-and-ranking.md:854`) closes as: nvim-cmp's own comparators —
  not sortText — decide empty-input order; sortText only resolves ties among equally-scored,
  equally-kinded, equally-recency'd items.

### Ordering under typed query — matcher score dominates, sortText stays 7th

- **V** Non-empty input: each entry gets a real matcher score (word-boundary/prefix-weighted,
  `matcher.lua:8-13, 86+`), so `compare.score` (which includes the source-priority bonus)
  separates entries first; `recently_used`/`locality`/`kind`/`sort_text` only order equal scores.
  The chain order never changes with input.

### filterText / matching

- **V** Entries match on `filter_text = item.filterText or trim(label)`
  (`lua/cmp/entry.lua:92, 447-449`); zero-score entries are dropped (`lua/cmp/source.lua:123-135`).
- **V** Default matching config allows non-prefix (sequence) matches —
  `disallow_prefix_unmatching = false`, `disallow_fuzzy_matching = false`,
  `disallow_partial_fuzzy_matching = true` (`default.lua:58-65`). ⇒ the word-initial invariant
  is a ranking aid here (matcher prefers word-boundary/prefix hits, `matcher.lua:11-13`), not a
  visibility requirement — same latitude as Helix, unlike Neovim core.

### preselect — honored by default (via cmp-nvim-lsp capability)

- **V** Default `preselect = PreselectMode.Item` (`default.lua:29`); on menu open the **first
  preselected entry in the sorted list** is selected, taking precedence over cmp's own forced
  `completeopt = 'menu,menuone,noselect'` (`default.lua:45`;
  `lua/cmp/view/custom_entries_view.lua:148-159, 224-225, 250-253`).
- **V** The capability is advertised by the companion source plugin, not cmp itself:
  `cmp_nvim_lsp.default_capabilities` sets `preselectSupport = true`
  (`hrsh7th/cmp-nvim-lsp/lua/cmp_nvim_lsp/init.lua:45-48`).

---

## 4. Verdict — does Q6's contract hold?

Q6's three-part contract, per client (VS Code + Zed carried from
`24-completion-item-shape-and-ranking.md` S-4 / `24-zed-client-reality.md`):

| Client | sortText absolute at empty input | score-above-sortText under query | preselect honored | filterText word-initial needed |
|---|---|---|---|---|
| VS Code | yes (run above) | yes | yes | ranking aid (Zed: visibility-critical) |
| Zed | yes (via response order + empty-score tie) | yes | no (capability absent) | yes (visibility, OtherMatch) |
| **Helix** | **not by mechanism** — sortText *never read*; response **array order** absolute (`completion.rs:370-379`) | yes (nucleo score above array order) | sort-rung only, capability **not advertised** (`client.rs:638-652`); no auto-select (`menu.rs:56`) | ranking aid (fuzzy sequence match) |
| **Neovim core** | **yes — strongest**: absolute at empty input *and* (default cot) under query (`completion.lua:635-657`) | only with opt-in `completeopt+=fuzzy` (`:641-644`) | 0.12: **no** (`preselectSupport=false` v0.12.6 `protocol.lua:479`, no C support); 0.13-master: yes **iff** `completeopt+=preselect` (`insexpand.c:1075-1078`); fallback = implicit first-item select (`:3563-3565`) | **visibility-critical** (startswith, `completion.lua:530-541`) |
| **nvim-cmp** | **no** — sortText is 7th comparator; source priority / recently_used / locality / kind dominate (`default.lua:69-80`, `view.lua:96-117`) | cmp's own matcher score, sortText still 7th | yes by default (`custom_entries_view.lua:250-253`) | ranking aid (prefix unmatching allowed, `default.lua:63`) |

### What must change before Q6 locks

1. **Q6's core sentence keeps its shape but needs a per-client footnote.** "sortText absolute
   only at empty input" is exactly right for VS Code, Zed, and Neovim core (Neovim: absolute even
   under query by default). For **Helix** it must read "response *array* order absolute at empty
   input — sortText is never computed client-side; the server's emitted order is the only order
   Helix keeps" (⇒ Traces must emit display order in the array; our current
   group-ordered emission already does). For **nvim-cmp** the claim is false and Q6 must say so:
   "clients with their own ranking stacks (nvim-cmp) may cross our order at any time via source
   priority, recency, locality, or kind; sortText survives only as a late tiebreak."
2. **ST-K6's preselect line splits three ways**: VS Code + nvim-cmp honor preselect today
   (nvim-cmp via cmp-nvim-lsp's `preselectSupport = true`); Zed no-ops (capability absent);
   Helix honors-if-sent but never asks (rust-analyzer sends unconditionally, others may not);
   Neovim core is version-split — no-op on installed 0.12 (`preselectSupport = false`), opt-in
   via `completeopt+=preselect` on 0.13+. 23's "config order + preselect on item 0" survives
   everywhere **as long as item 0 is the best candidate** (VS Code preselect, nvim-cmp preselect,
   Helix order, Neovim implicit first-item select) — the design should keep betting on order, not
   on the preselect field.
3. **filterText invariant hardened, not weakened**: word-initial `filterText` is *visibility*-
   critical on Neovim core (startswith) and Zed; ranking-aid-only on Helix, nvim-cmp, VS Code.
   Keep R-SHARED-FILTER-WORD as the builder-enforced invariant — it is the only counterweight
   that matters on two of five clients.
4. **ST-K4 closes**: the Helix/Neovim-core/nvim-cmp sweep is done; Q6 may move from *proposed*
   to lockable with the three corrections above folded in (array-order wording for Helix,
   nvim-cmp's "never absolute" carve-out, preselect version/flag split for Neovim).

### Sources

- Helix: <https://github.com/helix-editor/helix> @ `ba40e54` —
  `helix-term/src/ui/completion.rs:300-380,410-425`, `helix-term/src/ui/menu.rs:21-76,346-353`,
  `helix-term/src/handlers/completion/item.rs:61-67,79-84,106-126`,
  `helix-term/src/handlers/completion/request.rs:203-241`, `helix-term/src/ui/editor.rs:1101-1121`,
  `helix-lsp/src/client.rs:637-659`;
  nucleo-matcher 0.3.1 (crates.io tarball, pinned by helix `Cargo.lock`) `src/lib.rs:200-223`,
  `src/pattern.rs:302-320`.
- Neovim: <https://github.com/neovim/neovim> @ `7e1ae511` (master) + tag `v0.12.6` —
  `runtime/lua/vim/lsp/completion.lua:9-11,324-347,500-660,1100-1194,1280`,
  `runtime/lua/vim/lsp/protocol.lua:489-492` (master) vs `:476-479` (v0.12.6),
  `src/nvim/insexpand.c:946-953,998-1078,1145-1163,1485-1522,1598-1663,2444-2463,3388-3459,3505-3575,5013-5019`
  (master) vs v0.12.6 (`preselect`: 0 occurrences),
  `runtime/doc/options.txt:1658-1740` (completeopt flags incl. `fuzzy`/`nearest`/`preselect`).
- nvim-cmp: <https://github.com/hrsh7th/nvim-cmp> @ `2ffe79f` —
  `lua/cmp/config/default.lua:29,41-48,58-81`, `lua/cmp/config/compare.lua:17-131,133-197`,
  `lua/cmp/view.lua:55-142`, `lua/cmp/entry.lua:92,132-172,430-482`,
  `lua/cmp/matcher.lua:8-13,86-91`, `lua/cmp/source.lua:105-140`,
  `lua/cmp/view/custom_entries_view.lua:148-159,224-264`;
  cmp-nvim-lsp `main` `lua/cmp_nvim_lsp/init.lua:37-72`.
- Server behavior cited: rust-analyzer `crates/rust-analyzer/src/lsp/to_proto.rs:479-495`
  (unconditional `preselect`, relevance-inverted zero-padded sortText) @ master.
