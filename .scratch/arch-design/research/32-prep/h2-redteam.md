# Red-team pass: ticket 32 designs under attack

Goal: break the h1-recommended design (`ProjectRoot` authority plus a
portable path newtype through persisted structures) before the grill
adopts it. [P] marks a proposed defense; [H] a hypothesis.

## Against the portable UTF-8 form

1. Attack: a vault contains a Latin-1 or Shift-JIS filename on macOS.
   The UTF-8 form cannot represent it. The proposed skip-plus-notice
   rule drops real user files from the index silently-by-notice. The
   grill must weigh: skip (one notice, file invisible to queries) versus
   OS-byte keys (store stays machine-local, ticket 25 keeps the platform
   family). This is the highest-cost open decision; do not let the grill
   treat it as settled. [P]
2. Attack: NFC write normalization changes the name the file system
   reports back. A file created as NFD through git stays NFD on disk;
   the store row spells it NFC. Every reconciliation (mtime check,
   watcher compare, rescan) must normalize both sides before comparing,
   or the row looks deleted and re-added each refresh. This is exactly
   the Obsidian regression class (e2-web-deep.md). Defense: normalize at
   every comparison site, owned by one function; forbid ad-hoc
   normalization. Cost measured: a check-only NFC pass is 1.3 ms per
   50k paths; a full normalize pass is 16.4 ms; the check-first shape
   is affordable (g1-probes.md). [P]
3. Attack: the new epoch wipes every existing store. Users with 50k-note
   vaults pay a full reparse (measured cold cost 117 to 127 ms by shape
   plus parse time, so seconds). Defense: pair the epoch bump with the
   wipe-or-migrate row in the change table, and let ticket 36 time the
   migration decision. [P]

## Against `ProjectRoot` ownership

1. Attack: the authority holds a canonical root string; a rename of the
   vault root directory invalidates every derived path at once. Trust
   keying already re-derives (ticket 22 settled); the new failure is the
   tracker store holding absolute paths that now point at the old root.
   Defense: state-home stores must key on root identity, never absolute
   paths. The root-scoping guard in h1 already scopes them, but the key
   form decision must say identity, not path. [P]
2. Attack: two vault roots, one nested inside the other. The authority
   resolves each path against its own root; a config opened from the
   inner root must not accept paths that escape through `..` into the
   outer. Defense: the boundary check is lexical per root plus
   canonicalization at IO; the nested case is the existing
   `nearest_local` behavior, keep it tested. [H]
3. Attack: the newtype enters serde, clap, and minijinja boundaries. Each
   needs a lossless display and parse. The clap `PathBuf` acceptance in
   `normalize_source_input` currently probes the file system during
   argument parsing (survey); moving it behind the newtype moves the
   probe. Defense: parse as `String`, validate through the authority
   once, and record the moved-probe behavior change in the change table.
   [P]

## Against byte-order ranges

1. Attack: the probe proved the hyphen sibling is excluded by the
   `[prefix, prefix+1)` byte range, but only when the end bound
   increments the final byte (`/` to `0`). An implementation that uses
   `prefix + \x01` excludes every child. The grill must pin the
   increment-the-last-byte rule as the encoding law, with the probe as
   evidence. [P]
2. Attack: case-insensitive file systems (macOS default, Windows): two
   stored keys `Note.md` and `note.md` are one file. Keys never fold, so
   both spellings can exist as rows; the rescan alternates them.
   Defense: the comparison key (NFC, lowercase, NFC) stays the
   name-matching rule for links. Row identity needs a case-stability
   answer: keep the first-seen spelling and emit a notice on flip.
   Cheapest; matches the case-only-rename scenario answer. [P]

## Against the two-namespace inspection rule

1. Attack: template authors pass user input into the raw-path test type
   and reintroduce the escape. Defense: the raw type takes an absolute
   path only; a relative input is an error, not a confinement-relative
   join. [P]
2. Attack: `DryRun` and commit disagree on namespace. The writer's
   confinement path and the inspection namespace must cite the same
   rule text, or the documented exception drifts. One invariant
   sentence, owned by this ticket. [P]

## Attacks no design survives

1. A malicious editor that writes through a different path spelling
   every save: normalization handles Unicode, not arbitrary aliasing
   (hard links, bind mounts). Out of scope; single-user tool boundary.
2. TOCTOU-closed writes without handles: accepted gap, documented per
   boundary; `openat2`-scale machinery stays rejected (research 09).

## New duties

1. The grill must answer the non-UTF-8 name question explicitly; both
   h1 options and this pass now give it priced alternatives.
2. Pin the end-bound increment rule and the case-stability rule in the
   Answer; both have probe or precedent evidence behind them.
