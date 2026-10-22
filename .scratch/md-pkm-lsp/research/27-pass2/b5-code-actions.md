# B5. Cluster 5: Code actions beyond quick-fixes + documentHighlight (F7)

Status: complete (pass-2, research only).
Scope: F7.1-F7.8 per `a3-decomposition.md` §F7. Deliverable shape per a3 §3: findings with
citations, option spaces, leanest-maximally-capable recommendations, matrix/perf/registration
rows for consolidation, premise-check results.

**Evidence labels** used throughout:
- **[S]** = source/API demand: verified in digest source, upstream source, the LSP spec, or
  `a2-crates.md`'s ls-types/tower-lsp-server inventory.
- **[E]** = ecosystem practice: client behavior, docs, issue threads, editor plugins.

**Independence disclosure.** One `grep` call in this session was given a file path but the tool
searched the parent `research/` directory instead, incidentally returning match excerpts from
`27-structural-symbols-editor-intelligence.md` (the pass-1 file this cluster is barred from
reading). No claim below is sourced to that file; everything load-bearing was re-verified
against allowed inputs (digests, `tool-*.md`, `27-pass2/a*`, issues 23/25/26/27/28, LSP spec,
upstream/web). Where an early working note overlapped, the citation given is the primary source
re-checked in this session.

---

## 1. F7.1: Peer action inventory (source-verified)

### 1.1 Summary table

| Peer | Capability advertisement [S] | Actions shipped [S] | Kind(s) | Eager/lazy | Edit shape |
|---|---|---|---|---|---|
| MS markdown-ls | library only (no capability in this repo; consuming extension advertises) | `Extract to link definition` (2 providers total) | `refactor.extract.linkDefinition` (:7589), `quickfix` (:7790) | eager | in-house `WorkspaceEditBuilder`, single-doc `changes` |
| Marksman | `CodeActionOptions { CodeActionKinds = None; ResolveProvider = Some false }` (:11136, wired :11160) | TOC insert/update; create-missing-file | `Source` (:11937), `QuickFix` (:11949) | eager (`IsPreferred = Some false` :11919) | TOC: single-doc `changes`; create: **CreateFile-only** (`CodeActions.createFile` :11948) |
| Markdown Oxide | `CodeActionProviderCapability::Simple(true)` (:2049) | create unresolved file + append heading | **`kind: None`** on the action | eager | ordered `DocumentChanges`: CreateFile (`ignore_if_exists`, no overwrite) → `TextDocumentEdit` (version `None`) (:709-870) |
| rumdl | capability block **not in digest** (gap → C7); handlers at `get_code_actions` (:38284) | per-warning: fix (preferred), MD013 "Reflow paragraph", **MD034 "Convert to markdown link"**, "Ignore <rule> for this line" | all four `CodeActionKind::QUICKFIX` (upstream `src/lsp/types.rs`, [fetched](https://raw.githubusercontent.com/rvben/rumdl/main/src/lsp/types.rs)) | eager; **re-runs the whole lint per request** (:38285) | single-doc `changes`, `data: None` everywhere |
| rumdl (fix-all) | n/a | whole-document fix-all | `source.fixAll.rumdl` (:38387), `is_preferred: true` (:38407) | eager | TextEdit at pos 0, `diagnostics: []` |
| iwe | `Options { codeActionKinds: all_action_types(config), resolveProvider: Some(true) }` (tool-iwe Part 6, digest L39433-39490) | ~10-11 kinds: extract, inline, attach, transform, sort, delete, link, list, section, extract_all | **bare custom strings** via `CodeActionKind::new(id)` (not `refactor.`-prefixed) | **lazy** (`data: {key, range}` round-trip) | built on resolve (digest :39908 filter, :39941 resolve) |
| zk | `capabilities.CodeActionProvider = true` (no options, :4248) | extract-selection-to-note ("New note in current/top directory") | `refactor` | eager edit **via Command** (`zk.new` executeCommand) | server creates file on disk; `Changes` text edit (:4547-4600, :2938-3015) |
| obsidian.nvim | exposes LSP code actions for note ops (wiki/LSP.md) | add_property, move/delete/merge note, insert_link, **unlink**, insert_tag, link/link_new/extract_note (visual) | not stated in docs [E] | n/a (in-process) | n/a |

### 1.2 Notable mechanics (citations)

- **MS disabled-action pattern** [S]: `MdExtractLinkDefinitionCodeActionProvider` returns
  *disabled* actions with reasons instead of omitting: "Not on link" (:7591-7597),
  "Link is already a reference" (:7599-7605), so the user sees why nothing applies. Tests assert
  `disabled?.reason` (:17319, :17328).
- **MS honors `context.only` before doing work** [S]: `#isEnabled` early-returns unless
  `only` ⊆ `refactor` (extract, :7648-7657) / `only` ⊆ `quickfix` (remove, :7781). Single-target
  policy: only the first link in range gets an action (:7626-7635), since "creating actions for
  every link is overwhelming".
- **MS remove-definition reads `diag.data`** [S]: the quickfix is driven by
  `context.diagnostics`, matching `diag.code` (`link_unusedDefinition` / `link_duplicateDefinition`)
  **plus `diag.data` as `MdLinkDefinition`** (:7755-7771): the diagnostic carries the payload the
  action needs; no re-parse. This is the precedent for 25's `data`-channel quickfix.
- **MS extract action ships edit + command together** [S]: eager edit plus a follow-up
  `vscodeMarkdownLanguageservice.rename` command (:7676-7680).
- **MS also exposes two adjacent non-action surfaces** [S]: `organizeLinkDefinitions` is a
  *service method* returning `TextEdit[]` (:1612), **not** a code action; link rewrite on paste is
  `documentPaste` (:1789-1790), not a code action.
- **iwe resolve contract** (tool-iwe Part 6; digest :39908/:39941) [S]: handler filters by
  `params.only_includes(&action_kind)` and returns title/kind/data only; `handle_code_action_resolve`
  re-derives changes from `data`, **never errors** (identity return on parse failure); Helix
  1-char-range workaround; capability advertises every custom kind + `resolveProvider: true`.
- **moxide CreateFile discipline** [S]: `ignore_if_exists: true`, **no `overwrite` key**; create
  op ordered before the `TextDocumentEdit` (:822-831), the create-before-update ordering
  precedent; destination routed to `daily_notes_folder` / `new_file_folder_path` config (:756-762).
- **Marksman inert-route precedent** [S]: route wiring exists for **all three**
  (`textDocument/codeAction` :3169, `codeAction/resolve` :3170 (despite `ResolveProvider = false`),
  `textDocument/documentHighlight` :3174), yet the capability block advertises no
  `resolveProvider` and **no `DocumentHighlightProvider`** (:11060-11170 slice). A route clients
  never call because the capability is absent: advertising is the contract, not the route.
- **zk extract gating** [S]: action offered **only when the range is non-empty**
  (`!isRangeEmpty(params.Range)`, :4547+); two deterministic variants (current dir / top dir);
  command flow = `zk.new` (supports DryRun) → `workspace/applyEdit` with `Changes` (:2938-3015).
  zk creates the file server-side before handing the client a text edit (no CreateFile op).
- **rumdl conversion-as-quickfix** [S]: "Convert to markdown link" (bare URL → `[domain](dest)`,
  `mailto:` handling for bare emails, MDX-suppression guard) is a **diagnostic-tied quickfix**,
  the only LSP link-*conversion* action found anywhere.
- **rumdl ignore-line action** [S]: appends `<!-- rumdl-disable-line <rule> -->` at end of line;
  `is_preferred: false`; only when no disable comment already present.
- **rumdl diagnostic payload** [S]: `source: "rumdl"`, `code: <rule name>`, `data: None`: no data
  channel on diagnostics (Traces' 25 can do better, per MS's `diag.data` precedent).

### 1.3 Key negatives (absence results)

- **No LSP anywhere does wikilink ↔ standard-Markdown conversion as a code action** [S]:
  absent in MS, Marksman, moxide, rumdl, iwe, zk digests (grep), and confirmed by web search,
  where conversion exists only as *editor plugins*: `obsidian-links-converter` (toggle single link
  Cmd+Shift+L, bulk convert, **preserves aliases + anchors, skips YAML frontmatter and fenced code
  blocks, refuses external URLs**, image embeds both directions), `mii-key/obsidian-links`
  (convert-to-wikilink/markdown, selection + bulk, `<>` for spaces, optional `.md` append).
- **No LSP ships status/checkbox cycling as a code action** [E]: obsidian.nvim exposes
  `toggle_checkbox` / `set_checkbox` as *plugin verbs/keymaps* (`docs/Actions.md`), while
  genuinely exposing other operations (extract, link, unlink) *as LSP code actions*
  (`docs/LSP.md`). State-toggling lives outside LSP in the PKM ecosystem.
- **documentHighlight**: MS ships it (§8); rumdl, moxide, iwe, zk digests have **zero** hits;
  Marksman has the route but no capability (§8).
- **Extract-selection-to-note**: implemented by iwe, zk, obsidian.nvim; **moxide lists it as
  future work** ("Refactoring: Move headers or selections to a new file", Features Index);
  MS does not ship it (only link-definition extraction).

---

## 2. F7.2: Convert link ↔ wikilink

### 2.1 Precedent landscape

- LSP precedent: **none** (§1.3). Nearest in-LSP relative: rumdl's MD034 quickfix (URL →
  markdown link) [S]: diagnostic-tied, not cursor-scoped.
- Ecosystem precedent: Obsidian-plugin converters [E] give the concrete preservation rules
  (aliases, anchors, embeds; frontmatter/fence exclusion; external-URL refusal); these are the
  behaviors users already expect from the verb.
- rumdl interplay [S]: converting standard → wikilink **removes** the link from rumdl's
  validation domain (MD051 anchor / MD057 relative-path rules run on CommonMark links only,
  `tool-rumdl-boundary.md:50`); wikilink → standard **adds** it (new MD051/MD057 diagnostics may
  appear afterward: a user-visible, intended consequence, no toggle needed; kinds are disjoint
  from rumdl's quickfix/fixAll by construction).

### 2.2 Mechanics

Both directions reduce to: locate link span (26's `Occurrence` / documentLink ranges) → resolve
target → rebuild text. Two rendering tails over one shared path.

**Direction A: standard → wikilink**
- Unique target: `[text](path)` → `[[stem]]`, or `[[stem|text]]` when `text ≠ stem`
  (alias preservation); `![alt](img)` → `![[img]]` (bang preservation); `#anchor` links → `[[#slug]]`.
- Ambiguous target (option space):
  (a) **disabled action with reason** ("Multiple notes match; disambiguate first"), MS pattern [S];
  (b) nearest-pick conversion (26's rename-style resolution, if 26 settles it for write-path);
  (c) refuse silently (no action). Note client variance: VS Code shows disabled actions **faded**
  on explicit invoke; **Helix filters disabled actions out of its menu entirely** [E], so refusal
  renders as "no action" there either way.
- Unresolved target: no conversion (offer belongs to 25's create-note quickfix, not this action).

**Direction B: wikilink → standard**
- Unique: `[[stem]]` → `[stem](relpath)` (path style per 15's config: relative default, optional
  `.md` extension); `[[stem|disp]]` → `[disp](relpath)`; `[[stem#Head]]` → `(relpath#anchor-slug)`;
  `![[img.png]]` → `![img.png](img.png)` (or `![alt]` when alt known).
- Ambiguous/Unresolved: same option space as A (disabled-with-reason recommended).

**Refusal semantics vs 26**: read-path renders never error on ambiguity (26's pinned rule:
documentLink/highlight render all candidates); conversion is a **write-path offer**, so it aligns
with 26's rename-style *refusal* taxonomy: never error the request, express refusal as a disabled
action or absence.

### 2.3 Options + recommendation

| Decision | Leanest | Maximal | **Recommendation** |
|---|---|---|---|
| Directions | A only (ticket says "or vice versa") | A+B + bulk-convert-all-in-file | **Both directions, cursor-scoped only**: one shared resolve path, two text tails; bulk-convert out (28/file-ops territory, plugin-land covers it) |
| Ambiguity | no action | nearest-pick | **disabled-with-reason** (MS precedent; never-error preserved; revisit if 26 settles write-path nearest-pick) |
| Kind | bare custom `traces.convert.*` | `refactor.convert.toWikilink` etc. | **`refactor.convert.*` leaves**: lightbulb-visible, excluded from every fix-all path (§5) |
| Vehicle | command-with-args | n/a | **eager `CodeAction` with inline edit** (no prompt primitive; Zed lacked `workspace/executeCommand` until #63607, Sep 2026 [E]; edit-bearing actions work everywhere) |
| Payload | re-parse on apply | n/a | range-local edit from 26 spans; no `data` needed; `is_preferred: false` |

Offer **exactly one** conversion action per cursor position (the opposite of the link under the
cursor); never both directions at once.

---

## 3. F7.3: Extract selection → new note

### 3.1 Precedent

- **iwe** [S] (tool-iwe Part 6): `extract` (section under cursor → new doc, leaves a link),
  `link` (selection → link to new doc), `extract_all`; key_template (`{{slug}}`, `{{parent.*}}`,
  `{{id}}`, `{{source.*}}`, `{{title}}`, `{{today}}`) names the new file; `attach`-style creation
  reuses template machinery; dry-run/resolve round-trip; create ops `overwrite: false,
  ignore_if_exists: false` + **create-before-update ordering** in `into_lsp_changes`
  (iwe digest :41498-41560).
- **zk** [S]: `refactor` action, non-empty-range gated, command-based, two directory variants
  (current/top); selection kept + link inserted after it in "Append" mode; server writes the file
  itself then applies a text edit.
- **obsidian.nvim** [E]: `extract_note` (visual) exposed **as an LSP code action**, proving the
  verb travels well through the code-action UI.
- **MS**: not shipped (only `refactor.extract.linkDefinition`). **moxide**: explicitly future
  work (Features Index).

### 3.2 Seam analysis (Traces)

- **Vehicle**: codeAction with inline `WorkspaceEdit` (not executeCommand, since Zed #13756 only
  completed Sep 2026 [E]; edit-bearing actions are the portable floor). Kind
  **`refactor.extract.note`** (`refactor.extract` is in Zed's advertised valueSet, #10177 trace
  [E], and shows in VS Code's lightbulb on selection [E]).
- **Filename source** (LSP has no prompt primitive) option space:
  (a) slug of selection's first heading / first non-empty line; (b) key_template-style rule
  (`{{slug}}` + uniqueness suffix, iwe/zk `unique_note` timestamp precedent); (c) interactive
  command-with-args (rejects: breaks Zed-until-recently, extra client plumbing, async UI).
  → **(a) with (b)'s suffix rule as config extension later.**
- **Collision policy** option space: (i) auto-suffix (`notes-2.md`); (ii) **disabled-with-reason
  ("a note with this name exists")**; (iii) append-into-existing (moxide's append-heading pattern,
  wrong here: merges unrelated content). → **(ii) for v1** (honest, MS-pattern, avoids inventing
  downstream resolution for a silently-suffixed stem); (i) available later via key_template.
- **Edit shape** (ordered `DocumentChanges`, all-or-nothing intent):
  1. `CreateFile { ignore_if_exists: true }` (**omit `overwrite` entirely**): VS Code #198659 shows
     `overwrite:true` + `ignore_if_exists:true` is treated as overwrite-wins [E]; moxide's shape is
     the safe one [S].
  2. `TextDocumentEdit` on the new file: full content replace `0..u32::MAX`, `version: None`
     (iwe/moxide precedent [S]); frontmatter optional, v1 = selection verbatim.
  3. Edit source doc: replace selection range with leftover link (wikilink or standard per 15's
     path-style config; resolves Unique immediately since the file now exists).
  Known client caveat [E]: VS Code #146964: create-then-text-edit is **not atomic**; safest
  combination = pre-validate non-existence server-side + `ignore_if_exists: true` + no `overwrite`.
- **Undo story** [E]: VS Code applies workspace edits as a labeled undo element with **cross-file
  undo** (#91379; the observed failure was a disposed-model caveat, i.e. grouping exists;
  `undoredo.codeAction` quotable label in `codeAction.ts`); multi-file undo prompts exist in newer
  surfaces (#282004). One Ctrl+Z reverts source edit + new file in the good case; the
  disposed-file failure mode is a client limitation, documented not solved.
- **Gate**: shared `create ∈ resourceOperations` predicate (issue 27's pinned input: same
  predicate as 25's create-note quickfix and 26's create path). Absent capability →
  disabled-with-reason (VS Code) / hidden (Helix, Neovim); never a broken edit.
- **28 boundary**: plain `CreateFile` (no template) is extract's; **from-template note creation
  stays deferred to 28's file-ops boundary**. **15 interaction**: leftover link's target is
  Unique post-create.

### 3.3 Recommendation

`refactor.extract.note`, **eager**, range-gated (never compute on empty range, zk precedent;
makes VS Code's automatic lightbulb requests ~free), `context.only` checked first (MS pattern),
ordered DocumentChanges as above, disabled on create-gate/collision, same-folder landing,
deterministic filename from selection's first heading. **No resolve in v1** (§6). Selection is a
slice of one note; content cost is bounded by selection size; price it in the 33 row (§9.2)
rather than pre-building laziness.

---

## 4. F7.4: Status-cycle plumbing (23's invariant)

**Ownership**: 23 owns the invariant (*cycle order = config insertion order = completion
`sort_text` order*) and the "steps over symbols vs kinds" semantics; **27 owns plumbing only**
(issue 23:47 as reflected in a3 F7 intro; issue 27/26 inputs). This section prices mechanics.

### 4.1 Kind option space (the sub-question: "what kind do peers use for state-toggling actions?")

Ecosystem answer: **none do it via LSP** [E] (§1.3), so this is a first-principles pick against
the spec taxonomy:

| Kind | Case for | Case against |
|---|---|---|
| `quickfix` | fastest lightbulb path | **forbidden by our own non-overlap rule** (a3 F7.5): Traces quickfixes = traces-sourced diagnostics only; a diagnostic-less quickfix also pollutes "fix" menus and any client's apply-all-quickfixes [E] |
| `refactor` / `refactor.rewrite` | lightbulb-visible; not swept by any fix-all; MS precedent calls a semantics-changing rewrite (`extract.linkDefinition`) `refactor.*` [S] | "refactor" nominally means behavior-preserving (weak objection; MS already stretched it) |
| `source.traces.statusCycle` | namespace-clean | spec: "Source code actions apply to the **entire file**" [S]; **source actions do not trigger the lightbulb** [E] (jrieken/docs), killing discoverability for what 23 calls *the* primary verb; wrong scope for a one-line edit |
| bare custom (`traces.cycleStatus`) | iwe precedent [S] | only appears in unfiltered menus; excluded from lightbulb's quickfix+refactor filter [E] → silent in VS Code |

**Recommendation**: `refactor.rewrite.cycleStatus` (a `refactor.*` leaf): lightbulb + Ctrl+.
menu, structurally excluded from `source.fixAll*` sweeps, disjoint from 25's quickfix lane and
from rumdl's `quickfix`/`source.fixAll.rumdl`.

### 4.2 Mechanics

- **Gating**: only when cursor/selection intersects a task line's marker (24's dispatch seam;
  marker-state from 23's parse). Non-task / NonTask / filtered lines → no action (absence is the
  refusal; no `disabled` needed: there is no user-facing "why" beyond "not a task").
- **Action set**: option (a) single *"Set status: todo → doing"* (next-step only; obsidian's
  `toggle_checkbox` = next primary [E]); option (b) one action per non-current status (menu of
  N−1; obsidian's `set_checkbox` picker secondary [E]). → **(a) for v1**; (b) trivially additive
  later (same edit builder, different target status).
- **Edit**: single-doc `TextEdit` replacing the marker glyph span on that line. Span source: 23's
  task rows (line-granular) + lexer marker offset; if marker spans aren't byte-exact yet, v1 can
  derive column from line classification (cheap, but note the span-fidelity dependency).
- **Eager, no diagnostics, no `data`, `is_preferred: false`**; no resolve; trivially fast (line
  classify + config-table lookup + one TextEdit).
- **23's invariant is consumed, not re-derived**: server reads `TaskStatusMap` in config order;
  next = successor in insertion order (wrap at end). Completion `sort_text` agreement is 23's
  product requirement; plumbing must not sort independently.

---

## 5. F7.5: Kind taxonomy & fixAll boundary

### 5.1 Facts

- Spec taxonomy [S] (`codeAction.md`, 3.18): `quickfix` (fix problems), `refactor` (+extract/
  inline/rewrite leaves), `source` (**whole-file**: organizeImports, `source.fixAll` = "fix
  errors that have a clear fix… no user input"), Custom (`a.b` subsumed by `a`), `''` = all.
  `triggerKind`: Invoked=1 / Automatic=2 (lightbulb, autosave). Disabled actions: hidden from
  automatic lightbulbs, shown faded on explicit request (3.15+).
- VS Code behavior [E]: lightbulb + `editor.action.quickFix` show **quickfix + refactorings**;
  source actions appear via "Source Action…" menu / `editor.codeActionsOnSave` / keybind args
  (`editor.action.codeAction` with `args: {kind, apply: first|ifSingle|never}`, bindable per
  kind); `context.only = filter.include` from the invoking command (`codeAction.ts`);
  `editor.codeActionsOnSave: ["source.fixAll"]` **unions across all servers/providers** and
  `source.fixAll.<server>` namespaces let a single server be targeted; no global "Fix All"
  command (#185224, open). Sandy081: quickFix requests "actions of kind QuickFix" (#71579).
- Client literal support [E]: Zed advertises valueSet `["", "quickfix", "refactor",
  "refactor.extract", "source"]` (2024 trace, #10177, dated); Neovim's set adds
  `refactor.inline/refactor.rewrite/source.organizeImports` (lsp.txt); both send `only` on
  explicit requests.

### 5.2 Peer registration table [S]

| Peer | `codeActionKinds` advertised | resolve |
|---|---|---|
| rumdl | unknown (capability block not in digest: **C7 gap**); emits `quickfix` ×4 + `source.fixAll.rumdl` | n/a (eager) |
| iwe | **all custom kinds**, `all_action_types(config)` | `Some(true)` |
| Marksman | `None` (advertise nothing specific) | `Some(false)` (route nevertheless wired) |
| moxide | `Simple(true)` (no kinds) | no |
| zk | `true` (no kinds) | no |
| MS lib | n/a (extension advertises) | n/a |

### 5.3 Traces registration proposal

Static `CodeActionProviderCapability::Options(CodeActionOptions { … })` (ls-types:
`Simple(bool) | Options(CodeActionOptions)`: **no `RegistrationOptions` variant** in the provider
enum, a2 :155; `CodeActionRegistrationOptions` type exists at :3521 for a dynamic path C7/29
policy may or may not take):

```
codeActionKinds: [
  "refactor.convert.toWikilink",        // F7.2
  "refactor.convert.toStandardLink",    // F7.2
  "refactor.extract.note",              // F7.3
  "refactor.rewrite.cycleStatus",       // F7.4
  "quickfix",                           // ONLY if/when 25-lane create-note actions ride this provider
]
resolveProvider: false  // v1 (§6)
```

Non-overlap rules (each independently derived):
1. **Never register `source.fixAll` (bare)**: on-save `source.fixAll` unions *across servers*
   [E]; registering it would fold Traces' user-choice actions into rumdl's automatic sweep
   alongside `source.fixAll.rumdl`. A namespaced `source.fixAll.traces` is also **out**: Traces
   has no "clear fix, no user input" semantics to offer (spec's own definition [S]).
2. **`quickfix` stays 25's lane**: Traces quickfixes derive only from traces-sourced diagnostics
   (MS `diag.data` pattern, §1.2). If 25's create-note quickfix shares this provider, the kind
   is legitimate *because* it carries the diagnostic; conversion/status/extract never use it.
3. **rumdl disjointness** [S]: rumdl = `quickfix` + `source.fixAll.rumdl`; Traces =
   `refactor.*` (+ conditional `quickfix`) → zero kind overlap; clients merge multi-server menus
   anyway (Helix explicitly aggregates [E]; VS Code merges providers [E]).
4. **register only what is implemented** (29; iwe's advertised-but-empty capabilities are the
   named anti-pattern, tool-iwe Part 6).

---

## 6. F7.6: codeAction/resolve data discipline

### 6.1 Demands & precedents

- Spec [S]: `data` is opaque, preserved between `codeAction` and `codeAction/resolve` (3.16);
  clients advertise `dataSupport` + `resolveSupport.properties` (`a2-crates.md`:
  `CodeActionClientCapabilities` fields incl. `disabled_support`, `honors_change_annotations`).
- Client matrix [E] (lsp-devtools + traces): VS Code resolve since 1.65.2 (`('edit',)`);
  Neovim 0.6.1 (`('edit',)`), 0.11.0 (`('edit','command')`); Zed **sends resolve even when the
  action already carries an edit** (#24375; its guard `command.is_none() || edit.is_none()`
  fires with edit present); Helix resolve is capability-gated (`client.rs` checks
  `CodeActionProviderCapability::Options{…}`) and it skips stale applies ("document changed,
  skipping" on its code-actions-on-save path) [E].
- **Hard requirement from Zed**: a server advertising resolve must **tolerate resolve of an
  already-resolved action** → identity/echo return, never error (iwe's never-error contract is
  exactly this, digest :39941 [S]).
- iwe data shape [S]: `{key, range}`; server re-derives everything; client bytes are a pointer,
  not truth. MS's `diag.data` (§1.2) is the *diagnostic*-channel analogue feeding 25's quickfix.

### 6.2 Option space & recommendation

| Option | Description | Tradeoff |
|---|---|---|
| **A. All eager, no resolve (v1)** | Marksman-shaped: `ResolveProvider = false`; honor `only` first, gate extract on non-empty range | works on every client incl. hypothetical resolve-weak ones; automatic-lightbulb cost = filters only; extract cost = selection slice (bounded) |
| B. Lazy extract only | title-only response; `data = {uri, range, version}`; resolve builds content+ops | cheapest discovery; adds round-trip + stale-version handling; Zed may resolve eagerly (fine: identity return); needs implement-before-advertise |
| C. All lazy | iwe-shaped | overkill for µs-class convert/status |

**Recommendation: A**, with B's contract pre-specified so it can be adopted without breaking
clients if the 33 row (§9.2) ever shows extract discovery pressure:
- `data` discipline (whether used now or later): server re-derives from `{uri, range, version}`;
  range re-validated against current buffer text; any parse/validation failure → **identity
  return, never error**; version mismatch → re-derive if range still valid, else identity
  (never a guessed edit). Never trust client-mutated `kind`/`title`.
- Stale-window: align with 12's stale-result-discard: a resolved action against a superseded
  version is discarded rather than applied.

---

## 7. F7.7: Client matrix (rows for C7)

| Feature | VS Code | Zed | Neovim | Helix |
|---|---|---|---|---|
| **codeAction (UI)** [E] | lightbulb = quickfix+refactor; Source Action… menu; `editor.action.quickFix` (shows both); kind+apply keybind args; disabled = faded on invoke | `editor: toggle code actions` lightbulb; sends `only: ["","quickfix","refactor","refactor.extract","source"]` (#10177, 2024) | `gra`; `opts.only/triggerKind/filter`; **auto-applies when exactly one action survives filtering** | multi-server aggregate (FuturesUnordered); hierarchical `code_action_kind_matches`; **disabled actions filtered out entirely** |
| **codeAction resolve** [E] | yes (1.65.2, `'edit'`) | yes; **may resolve already-resolved actions** (#24375) → identity tolerance mandatory | yes (0.6.1 edit; 0.11 edit+command, #15818) | yes, capability-gated |
| **executeCommand** [E] | yes | **only since Sep 2026** (#63607/#13756) | yes | yes |
| **fix-all** [E] | `source.fixAll` unions across servers; `source.fixAll.<server>` namespaced; no global Fix All (#185224) | n/a documented | via `only:["source…"]` on save plugins | code-actions-on-save with stale-version skip |
| **documentHighlight** | built-in automatic (occurrences); dynamicRegistration 1.65.2 (lsp-devtools) | **automatic**, `lsp_highlight_debounce` default **75 ms** (settings docs; `editor.rs` debounced `provider.document_highlights`); WRITE ranges styled distinctly | **manual**: `vim.lsp.buf.document_highlight()` + `clear_references()` (user autocmds; `LspReference*` groups) | **manual** `select_references_to_symbol_under_cursor` + opt-in `auto-document-highlight` **default false** (book/editor.md) |

Implications: edit-bearing actions are the portable floor (Zed executeCommand recency); disabled
refusals are invisible in Helix (absence, not fade); automatic triggers (VS Code lightbulb,
Zed 75 ms highlight debounce, Neovim auto-apply-single) are why `only`-first + range-gating are
mandatory server-side.

---

## 8. F7.8: documentHighlight registration

### 8.1 Peer precedent

- **MS ships a full provider** [S] (`documentHighlights.ts`, digest :3630-3793): position on a
  header → `Write` at declaration + `Read` at each in-doc fragment link to it (:3685-3706);
  position on a link → path matches (`Read`, all links to same path, :3758), fragment (:3731),
  external URI (:3770), reference-style (definition = `Write`, usages = `Read`, :3781).
  Built on the shared ToC + link provider (caches), exported via
  `IMdLanguageService.getDocumentHighlights` (:1786), tests at :8813+. **`Text` kind unused**;
  matching is exact, not fuzzy.
- Everyone else: rumdl, moxide, iwe, zk = **zero** digest hits (iwe confirmed by tool-iwe Part 6
  line 372); Marksman = route :3174 **without capability advertisement** (inert: do not copy).

### 8.2 Traces behavior design

- Substrate (pinned): spine `nav::NavContext::highlight(target, doc)`, exposed "for 16's tags
  and this ticket's documentHighlight registration" (issue 27 input from 26).
- Semantics: document-local occurrence scan at the request position; **never errors on ambiguity**
  (26's pinned read-path rule) → highlight **all** candidate spans in this document (a fuzzy-true
  render, exactly what the spec says documentHighlight may be "more fuzzy" about).
- Kinds: **`Write` = declaration** (heading line, tag definition site, link-definition row),
  **`Read` = usages** (link occurrences), MS's convention; `Text` reserved for genuinely fuzzy
  matches (none in v1).
- Scope note: same-document only (spec); cross-file discovery stays 26's references feature;
  the 5,000-reference ceiling does **not** apply here (different family, per 26's "keep
  consciously distinct" instruction).

### 8.3 Decision

| Option | For | Against |
|---|---|---|
| **Register (static)** | one flagship peer (MS) + all four clients support it; spine primitive already exists; cost trivial (§9.2); implement-before-advertise satisfied | none material |
| Don't register | only Marksman-elsewhere would make it look novel | forfeits the cheapest ambient affordance of the nav spine |
| Dynamic-registration gated | client-capability predicate exists (`dynamicRegistration`) | no Markdown peer does it dynamically; static is the peer norm → C7/29 policy decides, but shape is trivial either way |

**Recommendation**: register statically: `documentHighlightProvider = Some(true)` (ls-types also
carries `DocumentHighlightRegistrationOptions` :3188 if C7's dynamic path is chosen). Rumdl
coexistence: zero (digest grep; C7's full enumeration confirms); no toggle needed.

---

## 9. Cross-cutting deliverables

### 9.1 Registration proposal (summary)

- `code_action_provider: Some(Options(CodeActionOptions { code_action_kinds: [refactor.convert.*,
  refactor.extract.note, refactor.rewrite.cycleStatus, (quickfix iff 25-lane)], resolve_provider:
  Some(false) }))`; static; dynamic path deferred to C7/29.
- `document_highlight_provider: Some(true)`; static, simple.
- Cheap-gate order inside the handler (cost model): `context.only` compatibility (MS `#isEnabled`
  pattern) → range/position locate → action build. Automatic triggers then cost near-zero.

### 9.2 Latency budget rows (ticket 33)

| Row | Budget (p95) | Rationale / peer bound |
|---|---|---|
| `textDocument/codeAction` discovery (convert + status-cycle at cursor) | **≤ 5 ms** | range-local span locate + O(1) stem/status lookup; `concurrency_level(1)` means anything slower starves hover (<2 ms class); cheap `only`/range gates keep automatic lightbulb traffic invisible. Peer expensive-bound: rumdl **re-runs the entire lint** per codeAction request (:38285) and still serves interactive UIs |
| `codeAction/resolve` (extract-to-note, if B adopted) | **≤ 20 ms** | user-triggered one-shot; matches the sanctioned find-refs ≤20 ms class; content = selection slice + ordered ops. (v1 eager: extract rides the ≤5 ms discovery row; giant selections are the risk the row must catch in measurement) |
| `textDocument/documentHighlight` | **≤ 5 ms** | position → cached occurrence scan (MS parallel: two cached providers per request); clients debounce anyway (Zed 75 ms) |

### 9.3 Matrix rows for C7

- `codeAction` row and `codeAction/resolve` row: §7 table (four editors).
- `documentHighlight` row: §7 table.
- Registration rows: §9.1 (static shapes; `CodeActionRegistrationOptions`/`DocumentHighlightRegistrationOptions`
  types exist in ls-types; dynamic adoption = C7/29 call).

### 9.4 Cross-ticket interactions

- **25**: shares the one `create ∈ resourceOperations` predicate (issue 27 pinned input) with
  this cluster's extract + 15's create-note; quickfix lane separation (§5.3 rule 2); MS `diag.data`
  is the payload precedent for 25's diagnostic-derived action.
- **23**: consumes `TaskStatusMap` insertion order verbatim; "steps over symbols vs kinds" stays
  23's semantics call; 27 prices only the marker-gated single-line replace (§4).
- **26**: Occurrence spans feed conversion; read-path never-error → highlight/convert render all
  candidates; write-path refusal taxonomy → conversion expresses refusal as disabled/absence;
  5,000-ceiling untouched (highlight is document-local).
- **28**: plain `CreateFile` inside a WorkspaceEdit is extract's (this cluster); **template-based
  note creation remains 28's**; zk-style server-side file writes are explicitly *not* the Traces
  shape (we use resource ops behind the create gate).
- **15**: leftover/converted links resolve through 15's model + path-style config; conversion
  interacts with rumdl MD051/MD057 domain by design (§2.2).
- **12 / 24**: stale-version discard applies to resolve (v1 no-op); task-line gating rides 24's
  dispatch seam.
- **29**: implement-before-advertise (iwe stub anti-pattern named); **32**: coexistence rows
  (Traces `refactor.*` vs rumdl `quickfix`/`source.fixAll.rumdl`, disjoint).
- **33**: rows in §9.2.

### 9.5 Premise-check results (a3 §4 items touching F7)

- **P3 (iwe zero documentHighlight)**: confirmed (zero digest hits; tool-iwe Part 6 line 372 [S]).
- **P12 (rumdl zero documentHighlight)**: digest grep zero [S]; full capability enumeration stays
  C7's job (rumdl's capability block is absent from the src digest; see gaps).
- **New premise (this cluster)**: "no LSP converts wikilink↔standard" (confirmed by six digests
  + web sweep, §1.3); "Marksman routes documentHighlight without advertising it" (confirmed,
  :3174 vs :11060-11170) [S].

### 9.6 Gaps / open items (honest)

1. rumdl's `codeActionProvider` **advertisement shape** (Simple vs Options; whether it lists
   `source.fixAll.rumdl` in `codeActionKinds`): capability block not in digest → **C7**.
2. VS Code's built-in markdown extension capability wiring (what it advertises for MS's library
   actions/highlights); library/adapter split; not read this pass (low stakes: MS advertises
   what it ships, per issue #71579 ecosystem behavior).
3. Zed's current `codeActionLiteralSupport` valueSet: the `["",…,"refactor.extract",…]` trace is
   from 2024 (#10177); likely superset now, unverified.
4. Neovim's handling of `disabled` actions (unspecified in docs read); assume shown or listed
   per spec; Helix-verified as hidden.
5. Exact `context.only` value list VS Code's *lightbulb* sends (verified structurally:
   `only: filter.include?.value` in `codeAction.ts`; docs state lightbulb = quickfix+refactorings,
   docs-level not source-exact).
6. Whether Marksman's documentHighlight route is ever exercised in the wild (capability absent →
   no standard client should call it; handler existence unverified beyond the route target).

---

## 10. Recommendation summary (leanest ↔ maximally capable)

| # | Decision | **Recommended (leanest-maximally-capable)** | Maximal alternative held in reserve |
|---|---|---|---|
| F7.2 | Conversion | both directions, cursor-scoped, eager inline edit, `refactor.convert.*`, Unique-only + **disabled-with-reason**, alias/anchor/bang preservation | nearest-pick on ambiguity (needs 26 write-path ruling); in-file bulk convert |
| F7.3 | Extract-to-note | `refactor.extract.note`, eager, non-empty-range + `only` gates, ordered CreateFile(`ignore_if_exists` only)→content→link edits, first-heading slug filename, disabled on create-gate/collision, same folder | auto-suffix collisions (key_template); lazy resolve (contract pre-specified, §6.2-B) |
| F7.4 | Status-cycle | `refactor.rewrite.cycleStatus`, marker-gated, single "next status" action, eager, single-doc TextEdit, config-order invariant consumed from 23 | all-targets menu (N−1 actions); `source.*` (rejected: spec scope + no lightbulb) |
| F7.5 | Registration | static `Options{…4 refactor leaves + conditional quickfix…}`, `resolve_provider: false`; **never `source.fixAll*`** | dynamic registration (C7/29) |
| F7.6 | Resolve | v1 eager-all + cheap gates; data/stale contract written now for later lazy-extract | adopt iwe `{uri,range,version}` resolve if 33 row pressures discovery |
| F7.8 | documentHighlight | **register**, static simple; Write=declaration / Read=usage (MS convention); ambiguity renders all candidates, never errors | dynamic-registration predicate (C7/29) |
