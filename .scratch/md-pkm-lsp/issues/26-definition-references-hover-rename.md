# Definition, references, hover, rename & workspace-edit behavior

Type: grilling
Status: resolved
Blocked by: 11, 15, 16, 19, 22, 23
Assigned: agent

Research: [consolidated
findings](../research/26-definition-references-hover-rename.md) (six subagent
tracks: navigation substrate, entity abstraction, WorkspaceEdit mechanics,
position/encoding/Uri plumbing, hover, prepareRename/rename semantics).

## Question

Once link/reference (15), tag (16), and frontmatter/inline-field (19) semantics
are settled. The span model (11) is in place. Decide the shared mechanics that
every entity kind's navigation/edit capability rides on:

- Decide whether a single internal "resolvable entity" abstraction is warranted,
  per the "introduce abstractions only for real boundaries" preference. Do
  links/tags/headings/fields genuinely share enough resolution/edit shape to
  warrant one? Or are four independent, simpler implementations actually less
  code? The abstraction covers locate-at-position, list-references, and
  compute-rename-edit.
- Decide `WorkspaceEdit` construction for rename: does Traces use
  `documentChanges` (versioned, LSP 3.16+ preferred form) or the plain `changes`
  map? Check `docs/refs/lsp_spec.md`, WorkspaceEdit section. Decide the
  `TextDocumentEdit` version-check policy, since one rename may touch multiple
  files (for example renaming a tag across 40 notes).
- Decide hover content: the `MarkupContent` kind (`plaintext` vs `markdown`),
  and what each entity kind shows (resolved path + first heading for a link
  target; resolved File Class fields for a schema-bound note; etc.). Negotiate
  against client `hoverProvider`/`markdown` capability.
- Decide prepare-rename (`textDocument/prepareRename`) validity checks. For
  example: refuse to offer rename when the cursor is not actually on a renamable
  entity, or when the target is outside any confined root.

## Answer

Resolved by four grilling rounds (Q1-Q5, Q6-Q8, Q9-Q11, Q12). Each round was
followed by a three-track stress test (conflict audit against the settled map,
internet benchmark audit, codegraph design/alternatives audit). Each round was
consolidated in the [research
file](../research/26-definition-references-hover-rename.md) Parts VII, VIII, IX,
X. Where research Parts I-VI contradict VII-X, the later parts win. Twelve
decisions locked:

1. **Entity abstraction (Q1): one thin protocol-free nav spine, two enums, no
behavior trait.** Core owns `nav::Occurrence { kind: OccurrenceKind, span:
ByteSpan }` where variants carry domain payloads (`Wikilink(Link)`,
`Embed(Link)`, `FrontmatterLink(Link)`, `Heading(Heading)`, `Tag(Tag)`,
`FieldKey(FieldKey)`, ...). Core also owns `nav::NavTarget { kind: TargetKind,
doc: PathBuf, name_range: Option<ByteSpan>, scope_range: Option<ByteSpan> }`
with the focus-containment invariant (`name_range` within `scope_range`), which
applies only when both exist. `TargetKind = Note | Heading | Tag | Footnote |
Field | SchemaField | LinkDef | File` (LinkDef covers 15's reference-definition
label spans; File covers images and schema TOMLs). A `NavContext` facade (a
borrowed index/schema/config/content-resolver bundle, following the
QueryService/SchemaService idiom) exposes
`locate`/`resolve`/`candidates`/`references`/`rename`/`highlight`. Use enum
dispatch and exhaustive matches; no `ResolvableEntity` trait (rejected: forced
stub arms for legitimate None-kinds, recreates Markdown Oxide's cross-product
tax). `OccurrenceKind != TargetKind` (the load-bearing peer lesson). Per-kind
logic stays in the domain modules that own the semantics. Boundary: no ls-types
in core, and the module home is pending 34 (same status as 25's fold).
`nav::locate` classifies **navigation entities only**: 24's dispatcher, 23's raw
scan, and 21/22 classifiers keep their own position logic, and all of them feed
ONE renderer. Two named changes to existing code are budgeted: expose
`LinkResolver` via a facade method, and expose `paths_with_tag` the same way.
The confinement predicate is threaded through `resolve` (A7).
Frontmatter-embedded wikilinks are first-class occurrences (A16).

2. **Resolution semantics (Q9): three-state, distance-nearest with explicit
ties.** One `match_key`-keyed request-time `candidates(from, target) ->
Vec<Match { path, distance }>` (candidates-with-distance; the `(nearest,
alternatives)` shape re-encodes the tie bug into the type). The spine's
`classify` derives `Resolution = Unresolved | Unique(target) | Ambiguous { kind:
AmbiguityKind, candidates: Vec<NavTarget> }`: empty goes to Unresolved, strict
argmin to Unique, tie to Ambiguous. "Distance" is the LCA **folder distance**
(`FolderRef::distance_to`); `resolve()` is unchanged for index build; the
`nearest_wins`/`ties_resolve_to_none` test suites remain the executable spec for
the Unique side. AmbiguityKind = `StemTie | HeadingAnchor | PathTie` (PathTie
added by Q12). Heading-anchor Ambiguous needs 15 D2's anchor matcher pinned as
multi-match against pre-dedup slugs (written as an Amended-by-26 into 15).
`RenameError::Ambiguous { kind, candidates: Vec<NavTarget> }` (thiserror,
protocol-free). Hover gets candidates on demand via `candidates_at`; there is no
`alternatives` side-channel on `Resolution`. Graph-vs-nav divergence recorded:
the inlink graph keeps nearest-picked edges (backlink counts, 25's
change-summary soundness, 27's counts, 28's rewrite targeting all consume them);
navigation reports ties. Q9(ii) (revoking distance-picking) is rejected as a
26-local switch.

3. **Ambiguity choke point and refusal taxonomy (Q6).** The ambiguity check
lives in the spine's position-level entry points
(`rename_at`/`prepare_rename_at`), never in handlers. It is structurally not in
the single-target driver (MCP (36) gets it for free). Two-tier refusal taxonomy:
`Ok(null)` = cursor off entity, non-renamable kind, alias/display side, or
confinement failure; `ResponseError(-32803 RequestFailed, human message)` =
entity present but unsafe (ambiguity) or illegal `new_name`. `Ok(null)` covers
only cursor-off-entity plus the four refusals; -32803 is never mixed for the
same operation class (this amends 16's InvalidParams for tag rename). The -32803
scope is rename/prepareRename **only**, never `textDocument/references`
(erroring the read path contradicts 15's "ambiguity is normal in PKM"). Consumer
matrix (written into this ticket): definition returns all candidates uncapped;
hover renders a bounded list (first 5 + "+N more"); references returns the
capped union; prepareRename/rename refuse with -32803; documentHighlight is
document-local span matching and never errors on target ambiguity; unresolved
hover is informational. Only link-shaped occurrences (stem ties, heading-anchor
multi-match, path ties) plus DailyDate (18's folder/both/path modes produce stem
ties, M3) can be Ambiguous; tags, fields, schema, and footnotes resolve
Unique/Unresolved only (a negative pin is written into 16). Use distinct message
text for heading ambiguity ("3 headings match in this note"). prepare checks the
capability gate before ambiguity (gopls ordering).

4. **Comparison keys (Q10 + Q12): tiered `match_key`, byte-exact-first,
fold-fallback.** One `match_key(s) -> Cow<str>` helper (quick-check borrow
fast-path) in `src/path.rs`: NFC, then `char::to_lowercase`, then trailing NFC
(idempotence pinned by property test; full casefold rejected, sigma divergence
documented; a simple-casefold alternative was considered and declined for
codebase consistency with `FieldKey::canonicalize`). Path keys fold `\` to `/`
inside the same helper; non-UTF-8 paths take no fold step (byte-level, fixture
owed). Apply it at exactly two boundaries. Index build: basename keys, an
exact-path hash map replacing the binary search on the resolver's lookup path
only. Query side: link path/stem, request path, build. Watcher-event paths
normalize at intake. **Every lookup tier is byte-exact-first**: an exact-byte
hit is final; the fold key is consulted only on miss; fold-tier ties, including
multi-raw-per-folded-key collisions at the exact-path tier, become `Ambiguous {
kind: PathTie }` (never raw-sorted-first; raw-sorted-first survives only for the
NFC-equal exotic case). Rename self-exclusion rule: the file being renamed is
excluded from its own ambiguity candidates; a case-only file rename =
`RenameFile` with zero text edits (input to 28). Emit side: the hash-map entry
is (key, original path); all emits read the raw `FileMeta.path`; original
spellings appear everywhere in output. Guard-rail verbatim: comparison keys
only, never spans, never Uri/overlay keys, never edit payloads. Never redb keys,
never sort order, never I/O paths, never slug generation. New direct dep:
`unicode-normalization 0.1` only, behind the one helper. Normalize at every
intake boundary (ecosystem pattern: normalize-once-at-build, retain originals
for output). Cost sold as correctness (12 to 300ns per key, ~10ms one-time index
build, zero budget impact).

5. **References and the response cap (Q7 + Q11).** The union is computed at
`references_at` orchestration as a union of **resolved occurrences**. Per
candidate, scan its inlink-implicated files for occurrences whose own
file-relative resolution lands on that candidate. Dedupe by (doc path,
ByteSpan). Never a file-set union: that rewrites sibling same-stem notes on
rename. `includeDeclaration` unions every candidate's declaration. The
single-target driver stays the shared primitive, so rename reuses it directly,
which structurally guarantees a cap-free `EditSet`. Heading-tie union is
complete and essential; stem-tie union is best-effort (tie-er notes produce no
inlink edge today). Document it with the multi-target-edge inlink-graph upgrade
path recorded as its own future ticket, not a 26 rider. Cap: 5,000 soft cap
scoped structurally to the `textDocument/references` response path only (two
spine entry points; `EditSet` computation is never truncated). Bounded
collection in canonical order (files in path order, occurrences ascending,
`take(5000)` with per-file early stop, capping work not just payload).
Truncation notice via `window/logMessage` (Info), a named constant fed to 27/29,
and a `partialResultToken` streaming upgrade path recorded for 29 (deliberate
non-advertisement until then). Rationale is client-side (Neovim bufloads every
referenced file: the measured 2.93s stall class), anchored honestly between
clangd's 1,000 and VS Code's 20,000.

6. **WorkspaceEdit construction (Q2).** Core computes a protocol-free `EditSet`
(per-doc `(DocPath, Vec<Replacement { span, text }>)`, sorted and
overlap-checked). The edge alone owns Uri/TextEdit conversion, capability
gating, and `metadata` (map standing constraint). Two-tier serializer: emit
`documentChanges` iff `advertised_documentChanges OR ops_present`
(rust-analyzer-proven deviation; `changes` structurally cannot carry resource
ops, and Neovim advertises ops but not documentChanges). Versions:
`Some(current_version)` for open/overlay docs, `null` for closed, **never 0 as a
closed-file sentinel** (a client may legitimately open at version 0). Resource
ops gate on `resourceOperations` alone; offer them in prepareRename only when
genuinely applicable (gopls "client does not support file renaming"). Full
server-side pre-validation is mandatory (no mainstream client rolls back;
atomicity is a server obligation). Set `metadata.isRefactoring` on multi-file
cascades; skip change annotations in v1. Use an in-house edit helper with
apply-and-assert tests (no mature edit-application crate exists). Ownership: 26
owns server-initiated `RenameFile` for textDocument/rename on note targets
(ordering: old-path edits, RenameFile, new-path edits); 28 owns client-initiated
workspace/will*Files and the delete path. Post-rename refresh rides didChange +
watched-file events into T1's 100ms coalesced refresh; ticket 13's single-file
RefreshPlan constructor needs a **batch/multi-file constructor** (seam noted for
the implementation spec). Uri policy: gate `scheme == "file"` at every boundary,
absolute file URIs, echo client-spelled URIs verbatim, path-based
cross-referencing, skip+log non-file schemes, no canonicalization machinery in
v1. iwe's rename (prepareRename -> WorkspaceEdit with create+delete file ops,
link text preserved) is cited as the PKM-native precedent for this serializer
shape. (amended 2026-10-21 by iwe/fileclass research,
[tool-iwe](../research/tool-iwe.md) Part 6).

7. **Position encodings (Q5).** Negotiate utf-8-if-offered-else-utf-16 (the
first encoding in the client's ordered preference that Traces implements; never
utf-32; default utf-16), and echo it via `capabilities.positionEncoding`. One
total `convert` module sits at the protocol boundary (`PositionEncoding`,
`negotiate`, `to_lsp_position`, `to_lsp_range`, `to_byte_pos`); all functions
clamp and never panic; inbound semantics = VS Code `offsetAt`. The same module
is the miette bridge (miette SourceSpan is byte offset + len; no adapter crate
needed). Whole-document ASCII fast path plus naive walk in v1; the
`rust-analyzer` line-index crate upgrade route is documented if hot. Keep
`byte_to_utf16_cu` as a thin `LineIndex` method so the 19/23 prerequisite text
stays true; 11's ropey branch is deleted (14 resolved `BufferState { content:
String, version: i32 }`, no rope; one LineIndex-backed helper serves disk and
overlay via `ContentResolver::text_of`). Strip BOM once at ContentResolver.
Encoding is an explicit parameter threaded from the session, never a global.
This closes 11's open negotiation question and the 11/14 ropey inconsistency.

8. **Hover (Q3).** Kind: negotiated Markdown iff `contentFormat` contains
Markdown (absent -> PlainText); PlainText = stripped rendering of the same
composed body, one composer two formatters, never raw markdown in a plaintext
field. Architecture: one shared renderer plus per-entity producers over 24's
per-(uri,version) content snapshot; `locate` is shared with navigation, and
rendering is never shared. Safe subset: 19's cross-IDE set plus strikethrough;
banned: raw HTML, images, tables, `command:` links, theme icons, and
**headings**. A12: schema-bound field hover renders the field name as bold, not
H2, amending 19's internal inconsistency. Use one escaping helper (MS
`escapeMarkdownSyntaxTokens` convention). Always set `range` to the token span
when ticket-11 spans exist. Template hover renders 22's `Doc` and `Identified`
tiers, suppresses only `None`, and shows an explicit degradation banner on
construction failure (never silently empty). Caps are producer-owned: footnote
preview ≤200 chars (17), inlinks ≤20, previews bounded, task hover one line;
`isTrusted` is client-side only, and the server never sets it. Wikilink/embed
producer decided (A20): title + resolved path + bounded first-lines preview +
inlink count. It is sourced from 19's refresh-time preview cache for disk files
and ContentResolver for overlays (staleness ≤ T1 100ms), ambiguity labelling per
the bounded candidate list. Heading-as-entity producer (N1): title + level +
inlink count. Ambiguity lists bounded candidates rather than silently picking
first (markdown-oxide's known wart). Non-resolvable states still hover,
informational, never a prompt or code action (creation belongs to diagnostics).
Budget: tightened to **<2ms target / <1ms stretch** (SOTA warm markdown hover is
<1ms; 33's <10ms/<5ms row is superseded here).

9. **prepareRename and per-entity rename (Q4).** Advertise `RenameOptions {
prepare_provider: true }` iff the client announced prepareSupport
(spec-mandated, Marksman pattern), else plain `true`; handlers are registered
unconditionally. Use a uniform `{ range, placeholder }` response. Refusal
taxonomy per decision 3; rename **re-validates eligibility even after prepare
succeeded** (clients may skip prepare, and buffers may change). Per-kind matrix:
wikilink occurrence rewrites the path/title/slug portion only (alias display
preserved); standard-link ref/definition sites rewrite url/fragment only, and
display-text refuses (seam to 32); tags cascade prefixes via `replacen`, body
`#`-aware and frontmatter syntax-aware, and the definition itself gets an edit;
headings rewrite title text plus **all anchor occurrences** (cross-file
`file.md#h`, same-file wikilink anchors, same-file standard-link fragments;
rumdl MD051 line noted for 32; a duplicate collision uniquifies `slug-1` and
never refuses); footnotes are file-local find-replace (safe floor, implement
first); frontmatter/inline field keys are note-local tier 1, and
FileClass-schema tier 2 defers to 20's schema-TOML span work (amends 19); embeds
ride the wikilink path with `!` outside the token range; RefLabel rename is
added (definition site + all three use forms, file-local); DailyDate refuses
rename in prepare regardless of target-note existence; template/query
identifiers are non-renamable in v1 (receipts into 21/22). Validate `new_name`
legality per kind at rename time (tag: `Tag::parse`; heading: reject `\n` and
leading `#`; footnote: reject spaces; field key: `FieldKey` rules, with
case-only rename legal and author casing preserved; wikilink target:
structural-resolver shape, and reject collisions with existing stems). Cursor
edges: the alias side refuses; the heading range excludes `#`s; the fragment
interior renames the target heading; code blocks are excluded by AST
construction. prepare range pins (A11): body tag includes `#` with placeholder
`#projects/active`; frontmatter excludes `#` with placeholder `projects/active`.
Errors: -32803 RequestFailed with human messages (never rust-analyzer's
self-described-wrong -32602). Staleness: `Some(v)` for open docs keeps VS Code's
`validateWorkspaceEdit` guard live; under sequential dispatch the
prepare->rename pair is atomic server-side. Out-of-root: `Ok(null)` when the
resolved target fails the confinement predicate (seam to 30).

10. **Concurrency and performance (A2, round-1).** Keep `concurrency_level(1)`;
rename is the **sole handler permitted to exceed 100ms**; implement workDone
`$/progress` for rename (this sides with 33's allowance and amends 12's
deferral; both are written into those tickets). The blocking consequence (a 1s
rename makes subsequent hovers wait) is recorded with 41 as the structural fix.
Budgets are restated under 33's binding rows, byte-tiered not file-count-tiered:
prepare <5ms (it is a locate); rename scales by total bytes/occurrences (≤100ms
@ ≤1MB, ≤1s @ ≤10MB); no refuse threshold in v1 stands only because progress
lands. Rename-at-scale evidence: tsserver 6s on 300-edit/190-file class,
client-side apply ~0.5s; markdown notes are KB-scale, so precomputed-span
renames beat code LSPs. Zed rename smoke-test (prepare + rename error path) is
owed at implementation time; markdown-oxide documents "(not zed)" next to its
rename features as the hazard evidence.

11. **NFKC rejected, slugger locked (Q8).** NFKC is rejected everywhere: slug
generation would produce rumdl MD051 false positives on Traces' own output and
GitHub-rendered anchor mismatches; path matching would resolve links the
OS/editor cannot open; and 15 revoked title matching, so the Obsidian-title
rationale was stale. Slug generation stays byte-exact GitHub (15 D2 + rumdl
default + vendored github-slugger triple-lock). Live divergences are named
instead: filename/stem normalization (Obsidian, macOS) and NFD heading anchors.
The Unicode fixture set is owed to 25. NFD heading vs NFC anchor, fullwidth,
circled, ligature, emoji-in-title, NFC-equal duplicate filenames, CJK width
tags, non-UTF-8 path, sigma, case-only rename. Home = 15's slugger acceptance
tests.

12. **Process ledger.** Amended-by-26 blocks are written into 11, 12, 14, 15,
16, 18, 19, 21, 22, 25. Inputs-from-26 blocks are written into 27, 28, 29, 30,
32, 33, 34, 41. Glossary terms: Occurrence, NavTarget, Resolution, Comparison
Key (never the prohibited phrase "normalized path"), Ambiguity (with
AmbiguityKind variants), Folder Distance. Plus the Slug/Normalization terms owed
from round 2. Use "inlinks" not "backlinks" in glossary prose. The ls-types <->
tower-lsp-server pairing is pinned (feeds 09/34). Schema goto-definition
provenance is consumed from 20's `SchemaFieldDef::origin` (no second provenance
field).

## Sign-off from 27 (2026-10-22)

- Per 26's consumer matrix extension: documentLink emits Ambiguous links with
  first-candidate target plus count tooltip (deterministic first candidate from
  `candidates()` order); documentHighlight stays document-local span matching,
  never errors on ambiguity, and needs no 26 spine (it is a filter over the
  walk's link spans); the Ambiguous semantic-token modifier ships in 27's Wave
  2, gated on 26's tie-reporting candidates facade (Wave 1 ships two-valued
  `unresolved` only).
- Ceiling amendment receipt: 27 D13' restates 26's REFERENCE_CEILING as two-tier
  (default soft cap 1,000 with visible truncation, hard ceiling 5,000
  configurable, clangd-shaped); this amends 26's flat 5,000 soft cap, with 5,000
  becoming the hard ceiling. Proposed here for 26's owner to confirm.
