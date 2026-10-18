# Index

Persistent file indexing, metadata caching, storage, and inbound link graph
construction.

## Language

### Index Model

#### Workspace Index

An immutable in-memory snapshot of indexed files, parsed notes, and derived
inbound links across a project root.
*Avoid*: File Index, NoteIndex, database handle, cache, vault

#### Index Store

The durable store of file entries, parsed notes, and derived
relationships for one project root, distinct from the in-memory Workspace Index.
*Avoid*: database, persistence layer, cache handle

#### Indexer Service

The service driving the index lifecycle: building fresh indexes, persisting to
disk, loading cached data, and performing differential refreshes.
*Avoid*: index manager, scanner, indexer facade

#### Index Update

The computed change set and recomputed state from one incremental
synchronization, not yet applied.
*Avoid*: sync outcome, reconciliation result, sync delta

#### RefreshState

The state from comparing current project files and the stored Index Fingerprint
with the persisted index: `Fresh` when the opened store remains current,
`Stale` when an Index Update awaits persistence, or `Drifted` when the store was
built under different index-relevant config and must be rebuilt in full. Drift
wins over staleness; a virgin store never drifts.
*Avoid*: sync pass, scan pass, refresh transaction

#### Refresh Report

What one refresh found, as one of three variants: `Unchanged` for a `Fresh`
RefreshState, `FilesChanged` with the changed-file and changed-link counts for a
`Stale` one, or `ConfigDrift` carrying the drift cause for a `Drifted` one: the
settings that changed, or `Unrecorded` when the store holds rows but no
readable fingerprint. A heal reports its cause, never row counts.
*Avoid*: sync report, update result, refresh outcome

#### Index Fingerprint

The digests of the index-relevant config settings, persisted beside the rows
they shaped and compared at every refresh to detect drift. It is a subset of
the config's per-setting digests: root, template directories, and the schema
directory are excluded, so moving a project does not rebuild its index, while
every setting that could change a derived row is included, even those no
index-time code reads today.
*Avoid*: config hash, cache key, config stamp

### Indexed Data

#### File Metadata

The filesystem metadata captured for every regular file regardless of document
type: relative path, size, timestamps, and format classification.
*Avoid*: fs entry, raw record

#### File Entry

One indexed file's File Metadata together with its optional parsed Note and derived
inbound links in a Workspace Index.
*Avoid*: file record, indexed note

#### Note

The indexed form of a Markdown note: the parsed frontmatter, inline fields,
tags, lists, tasks, and outgoing links persisted for querying, distinct from
the File Metadata carrying its file metadata.
*Avoid*: note metadata, page data, document info, note payload

#### Inlink

A derived inbound reference mapping a note's path to all other notes linking to
it via Markdown links or wikilinks.
*Avoid*: backlink, incoming link, reverse ref

#### Folder Distance

The separation between two notes measured by their location in the directory
tree (distance to their lowest common ancestor folder), used to pick the
nearest candidate when one basename maps to several notes and to detect
equal-distance ties, which remain undisambiguated rather than resolved.
*Avoid*: path distance, LCA metric, resolution distance

#### Incremental Delta

The differential change set inside an Index Update that compares timestamps and
patches only modified files, notes, and affected link targets.
*Avoid*: index patch, sync delta
