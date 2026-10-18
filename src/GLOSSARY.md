# Core

Shared domain primitives, security boundaries, and cross-cutting types for
Traces.

## Language

### Workspace & Boundaries

#### Project Root

The workspace directory containing notes and `.traces/` configuration, against
which all file operations, template outputs, and queries are strictly confined.
*Avoid*: working directory, vault root, base folder

#### Root-Confined Path

A filesystem path guaranteed to reside within the Project Root, rejecting
absolute paths, root escapes, and parent directory (`..`) traversal.
*Avoid*: relative path, sanitized path, normalized path

### Metadata Primitives

#### Field Key

The canonical, case-insensitive identifier for note and schema metadata fields,
preserving author casing for display.
*Avoid*: property name, attribute key, column

#### Tag

A `#`-prefixed identifier supporting hierarchical sub-tag prefix matching.
*Avoid*: label, category, keyword

#### File Metadata

The universal filesystem metadata captured for every regular file in a project:
relative path, size, timestamps, and format classification.
*Avoid*: fs entry, file record

#### Duration

An elapsed duration carrying both its total seconds magnitude and written source
components. `src/temporal/` is the sole owner of duration recognition, parsing,
and the unit registry (`TEMPORAL_UNITS`, `TimeUnit`, `nominal_seconds`). Compares and
hashes canonically by total seconds, so `1h 30m` and `90m` are equal. Negative
zeros normalize (`-0m` equals `0m`) at construction; raw float comparisons follow
a unified signed-zero doctrine (ticket 09 extraction home). The unit vocabulary
maintains a strict sync contract between `UNIT_SPELLINGS_HINT` and `TEMPORAL_UNITS`
(invariant test in `unit.rs`).
*Avoid*: elapsed time, time span, interval

#### Temporal Strictness & Resolution

Temporal parsing strictness follows a strictness-by-seam rule: user input in notes
and templates is lenient with actionable error diagnostics; serde deserialization
is strict; query filters evaluate strict typed expressions. Week math and
numbering pin strictly to ISO Monday via standard calendar primitives, never
locale-dependent offsets or epoch-day division. Avoid terms listed across this
glossary are strictly prohibited from appearing as concept names or definitions.
*Avoid*: locale week, lenient parser

#### Path Codec

The platform-native (de)serialization of `PathBuf` fields crossing a
serialized format. `src/path.rs` is the crate's single owner; every such
field funnels through `crate::path::codec`.
*Avoid*: path serde, path serialization, filesystem encoding

### Navigation & Resolution

#### Occurrence

A position-tagged instance of a navigable entity inside a note: a byte span
plus a kind carrying its domain payload (wikilink, embed, frontmatter link
value, heading, tag, footnote, field key, inline field, schema field, daily
date). Produced by the navigation spine's locate step; transient, never
persisted.
*Avoid*: entity instance, use site, reference record

#### NavTarget

The resolved destination of an Occurrence: target document path, target kind,
and optional name and scope byte spans (name contained within scope when both
exist). Protocol-free; converts to LSP Locations only at the protocol edge.
*Avoid*: destination node, goto target, resolved link

#### Resolution

The three-state outcome of resolving an Occurrence against the workspace:
Unresolved (no candidate), Unique (one best candidate), or Ambiguous (two or
more equally best candidates, carrying an AmbiguityKind and the candidate
list). Each feature consumes the states per the settled consumer matrix
(definition returns all candidates, hover lists them bounded, references
returns the capped union, rename refuses).
*Avoid*: resolution result, target option, link outcome

#### Ambiguity

The state where two or more candidates are equally best for one Occurrence:
StemTie (same-basename notes at equal folder distance), HeadingAnchor (duplicate
pre-dedup heading slugs within a note), or PathTie (case/folded path
collisions). Ambiguity is normal in PKM: never a diagnostic, navigation reports
it, rename refuses it with a RequestFailed error naming the candidates.
*Avoid*: conflict, ambiguity error, tie failure

#### Comparison Key

The derived form of a wikilink path, file stem, or request path used only for
equality comparisons at index-build and query boundaries: NFC, then
`char::to_lowercase`, then trailing NFC, with byte-exact matching attempted
first at every tier (the fold consulted only on a miss). Never applied to
spans, Uri/overlay keys, redb keys, sort order, I/O paths, slug generation, or
edit payloads; original spellings are preserved everywhere in output.
*Avoid*: normalized path, canonical path, fold key

### Traversal & State

#### Directory Tree

The shared filesystem traversal model for scanning directories, discovering
templates, and loading schemas with classified error handling.
*Avoid*: walker, walk adapter, walkdir

#### File Path Tracker

The user-level persistent tracker managing tracked configuration paths and
trusted project roots.
*Avoid*: state cache, local database, cache dir

#### User

The human operating Traces, or an automated agent acting on their behalf.
*Avoid*: Client, operator, caller
