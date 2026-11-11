//! Throwaway ticket-27 replay-fault probe. It journals parsed facts or a
//! delta and replays them into the REAL `.traces/index.redb` store layout
//! through the real `traces` types and persistence path.
//!
//! Parser notices are out of scope: `parse_markdown` is infallible today, so
//! there is no notices facet to journal (tickets 34/29).
//!
//! DECIDED-CONTRACT MODELING: production META holds only `version` and
//! `fingerprint`. The probe models the decided identity (engine, platform,
//! incarnation, revision, epoch, policy hash, extractor fingerprint) in its
//! journal header and mirrors it into the sidecar file
//! `.traces/probe-identity`. This is the decided contract, not the current
//! production META.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use redb::{
    Database, Durability, MultimapTableDefinition, ReadableDatabase,
    ReadableMultimapTable, ReadableTable, TableDefinition,
};
use serde::{Deserialize, Serialize};
use traces::{
    FileMeta, Note,
    testing::{
        INDEX_REDB_PATH, IndexStore, IndexerService, QueryBuilder, QueryRow,
        QueryService, SourceSelector,
    },
};
use walkdir::WalkDir;

#[path = "../../../../../../benches/common/cache.rs"]
mod cache;

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;
const EDITED_NOTE_0: &str = "---\nclass: Area\nrating: 99\n---\n# Edited \
                             #updated\n[[hub]]\n- [x] changed task\n";

/// Real store table names in declaration order. The first three are plain
/// tables; the rest are multimaps.
const TABLES: [&str; 8] = [
    "files",
    "notes",
    "meta",
    "links",
    "paths_by_tag",
    "paths_by_file_class",
    "tags_by_path",
    "classes_by_path",
];
const PLAIN_TABLES: [&str; 3] = ["files", "notes", "meta"];
const MAGIC: &[u8; 8] = b"T27FACT!";
const MAX_JOURNAL: u64 = 64 * 1024 * 1024;
const HEADER: usize = 8 + 4 + 4 + 8 + 32 + 32;
const EPOCH: u32 = 1;
const ENGINE: &str = "traces-index-redb-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Identity {
    engine: String,
    platform: String,
    incarnation: String,
    revision: u64,
    epoch: u32,
    policy: [u8; 32],
    extractor: [u8; 32],
}
impl Identity {
    fn first() -> Self {
        Self {
            engine: ENGINE.into(),
            platform: env::consts::FAMILY.into(),
            incarnation: format!(
                "{}-{}",
                process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ),
            revision: 0,
            epoch: EPOCH,
            policy: *blake3::hash(
                b"default parser/class policy; probe 27; epoch 1",
            )
            .as_bytes(),
            extractor: *blake3::hash(b"traces test-utils extractors v1")
                .as_bytes(),
        }
    }

    /// Reports whether the store's identity is format-compatible with the
    /// journal's. Incarnation and revision are excluded: they say WHICH
    /// store state, not WHICH format produced it.
    fn is_compatible_with(&self, other: &Self) -> bool {
        self.engine == other.engine
            && self.platform == other.platform
            && self.epoch == other.epoch
            && self.policy == other.policy
            && self.extractor == other.extractor
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Source {
    path: String,
    size: u64,
    mtime: u128,
    change: i128,
    digest: [u8; 32],
    racy: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Snapshot {
    scan_start: u128,
    files: Vec<Source>,
}
/// One journaled file: postcard payloads of the real `FileMeta` and
/// `Option<Note>`. These are the decided journal facts, not a shadow model.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Fact {
    path: String,
    file: Vec<u8>,
    note: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    base: Identity,
    /// Whole-store digest over all eight raw tables at the journal's base.
    base_hash: [u8; 32],
    /// Digest over the base fact set (files + notes rows).
    base_facts: [u8; 32],
    /// Digest over the fact set the replay must produce.
    target_facts: [u8; 32],
    snapshot: Snapshot,
    paths: Vec<String>,
    /// One entry per path: the new fact, or None for a deletion.
    changes: Vec<Option<Fact>>,
}
fn err(message: &'static str) -> Box<dyn Error> {
    message.into()
}
fn table(
    name: &'static str,
) -> TableDefinition<'static, &'static [u8], &'static [u8]> {
    TableDefinition::new(name)
}
fn mmtable(
    name: &'static str,
) -> MultimapTableDefinition<'static, &'static [u8], &'static [u8]> {
    MultimapTableDefinition::new(name)
}
fn store_path(root: &Path) -> PathBuf {
    root.join(INDEX_REDB_PATH)
}
fn journal_path(root: &Path) -> PathBuf {
    root.join(".traces/index.redb.journal")
}
fn temp_path(root: &Path) -> PathBuf {
    root.join(".traces/index.redb.journal.tmp")
}
fn identity_path(root: &Path) -> PathBuf {
    root.join(".traces/probe-identity")
}
fn stop(point: &str) {
    if env::var("PROBE_PANIC").ok().as_deref() == Some(point) {
        panic!("probe panic at {point}");
    }
    if env::var("PROBE_STOP").ok().as_deref() == Some(point) {
        eprintln!("probe_stop={point}");
        process::abort();
    }
}
fn fail(point: &str) -> Result<()> {
    if env::var("PROBE_FAIL").ok().as_deref() == Some(point) {
        return Err(format!("injected ENOSPC at {point}").into());
    }
    Ok(())
}
/// Park mid-recovery on stdin so the parent can deliver a signal class.
fn pause(point: &str) -> Result<()> {
    if env::var("PROBE_PAUSE").ok().as_deref() == Some(point) {
        eprintln!("probe_pause={point}");
        std::io::stderr().flush()?;
        let _ = std::io::stdin().read(&mut [0u8; 1])?;
    }
    Ok(())
}
/// Racing-edit failpoint: called after the final source check but before the
/// journal is removed; mutates a journaled source, then aborts. The next run
/// must detect the drift and fail safe (reparse).
fn race_edit(root: &Path, journal: &Journal) {
    if env::var("PROBE_RACE").ok().as_deref() != Some("1") {
        return;
    }
    if let Some(path) = journal.paths.first() {
        let target = root.join(path);
        let mut body = fs::read(&target).unwrap_or_default();
        body.extend_from_slice(b"\n# raced edit\n");
        let _ = fs::write(&target, &body);
        eprintln!("probe_race_edit={path}");
        process::abort();
    }
}
fn unix_nanos(time: SystemTime) -> Result<u128> {
    Ok(time.duration_since(UNIX_EPOCH)?.as_nanos())
}
#[cfg(unix)]
fn change_time(meta: &fs::Metadata) -> i128 {
    use std::os::unix::fs::MetadataExt;
    i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec())
}
#[cfg(not(unix))]
fn change_time(_: &fs::Metadata) -> i128 {
    0
}
fn snapshot(root: &Path, scan_start: u128) -> Result<Snapshot> {
    let mut files = Vec::new();
    for item in WalkDir::new(root).follow_links(false).into_iter().filter_entry(
        |entry| {
            !entry.file_type().is_dir()
                || !matches!(
                    entry.file_name().to_str(),
                    Some(".git" | ".traces" | "target" | "node_modules")
                )
                || entry.depth() == 0
        },
    ) {
        let item = item?;
        let relative = item.path().strip_prefix(root)?;
        if item.file_type().is_dir() {
            continue;
        }
        if !item.file_type().is_file() {
            return Err(err("unsupported non-regular source"));
        }
        let path = relative
            .to_str()
            .ok_or_else(|| err("non-UTF8 source path unsupported by probe"))?
            .replace('\\', "/");
        let file = File::open(item.path())?;
        let before = file.metadata()?;
        let mut bytes = Vec::new();
        (&file).read_to_end(&mut bytes)?;
        let meta = file.metadata()?;
        let mtime = unix_nanos(meta.modified()?)?;
        if before.len() != meta.len()
            || before.modified()? != meta.modified()?
            || change_time(&before) != change_time(&meta)
            || bytes.len() as u64 != meta.len()
        {
            return Err(err("source changed during snapshot"));
        }
        let racy = mtime >= scan_start.saturating_sub(2_000_000_000);
        files.push(Source {
            path,
            size: meta.len(),
            mtime,
            change: change_time(&meta),
            digest: *blake3::hash(&bytes).as_bytes(),
            racy,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Snapshot {
        scan_start,
        files,
    })
}
fn is_same_sources(original: &Snapshot, root: &Path) -> Result<bool> {
    // Recompute each racy bit against the original scan start, never recovery
    // time.
    Ok(snapshot(root, original.scan_start)? == *original)
}
/// Canonical fact set keyed by path, as parsed from the current sources.
type FactMap = BTreeMap<String, Fact>;

/// Parses the vault with the REAL indexer and returns the source snapshot and
/// the full fact set derived from real `FileMeta`/`Note` rows.
fn parse_current(root: &Path) -> Result<(Snapshot, FactMap)> {
    let start = unix_nanos(SystemTime::now())?;
    let before = snapshot(root, start)?;
    let index = IndexerService::for_tests(root).build()?;
    let after = snapshot(root, start)?;
    if before != after {
        return Err(err("source changed during parse"));
    }
    let mut facts = BTreeMap::new();
    for entry in index.entries() {
        let path = entry
            .file()
            .path()
            .to_str()
            .ok_or_else(|| err("non-UTF8 indexed path"))?
            .to_owned();
        facts.insert(path.clone(), Fact {
            path,
            file: postcard::to_stdvec(entry.file())?,
            note: entry.note().map(postcard::to_stdvec).transpose()?,
        });
    }
    if facts.len() != before.files.len() {
        return Err(err("index scan and source snapshot disagree"));
    }
    Ok((before, facts))
}
/// Reads the current sidecar identity (decided-contract mirror).
fn read_identity(root: &Path) -> Result<Identity> {
    Ok(postcard::from_bytes(&fs::read(identity_path(root))?)?)
}
fn write_identity(root: &Path, identity: &Identity) -> Result<()> {
    fs::write(identity_path(root), postcard::to_stdvec(identity)?)?;
    Ok(())
}
/// Opens (or creates) the real store database file.
fn open_db(root: &Path) -> Result<Database> {
    fs::create_dir_all(root.join(".traces"))?;
    Ok(Database::create(store_path(root))?)
}
/// Digests every raw byte of all eight real store tables, table name first.
fn store_digest(db: &impl ReadableDatabase) -> Result<[u8; 32]> {
    let txn = db.begin_read()?;
    let mut h = blake3::Hasher::new();
    for name in TABLES {
        h.update(name.as_bytes());
        if PLAIN_TABLES.contains(&name) {
            if let Ok(t) = txn.open_table(table(name)) {
                for entry in t.iter()? {
                    let (key, value) = entry?;
                    h.update(&(key.value().len() as u64).to_le_bytes());
                    h.update(key.value());
                    h.update(&(value.value().len() as u64).to_le_bytes());
                    h.update(value.value());
                }
            }
        } else if let Ok(t) = txn.open_multimap_table(mmtable(name)) {
            for entry in t.iter()? {
                let (key, values) = entry?;
                h.update(&(key.value().len() as u64).to_le_bytes());
                h.update(key.value());
                for value in values {
                    let value = value?;
                    h.update(&(value.value().len() as u64).to_le_bytes());
                    h.update(value.value());
                }
            }
        }
    }
    Ok(*h.finalize().as_bytes())
}
/// Decodes the full fact set from the real `files` and `notes` rows.
///
/// Fails when a stored row is undecodable (poisoned), so the strong base
/// check treats the store as unreadable.
fn store_facts(db: &Database) -> Result<BTreeMap<String, Fact>> {
    let txn = db.begin_read()?;
    let mut facts = BTreeMap::new();
    {
        let files = txn.open_table(table("files"))?;
        let notes = txn.open_table(table("notes"))?;
        for entry in files.iter()? {
            let (key, value) = entry?;
            let path = String::from_utf8_lossy(key.value()).into_owned();
            let bytes = value.value();
            // Decoding validates the row; the value itself is unused here.
            let _file: FileMeta = postcard::from_bytes(bytes)?;
            let note = notes
                .get(key.value())?
                .map(|row| postcard::from_bytes::<Note>(row.value()))
                .transpose()?;
            facts.insert(path.clone(), Fact {
                path,
                file: bytes.to_vec(),
                note: note.map(|_| ()).and(
                    notes.get(key.value())?.map(|row| row.value().to_vec()),
                ),
            });
        }
    }
    Ok(facts)
}
/// Canonical encoding of a postcard row for hashing: postcard map fields
/// are not order-stable, so hash the serde_json Value (BTreeMap-ordered)
/// instead of the raw bytes. Raw bytes are the fallback for undecodable
/// rows so poisoned rows still yield a stable mismatch hash.
fn canonical_row<T>(bytes: &[u8]) -> Vec<u8>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    postcard::from_bytes::<T>(bytes)
        .ok()
        .and_then(|row| serde_json::to_value(&row).ok())
        .and_then(|value| serde_json::to_vec(&value).ok())
        .unwrap_or_else(|| bytes.to_vec())
}
fn facts_hash(facts: &BTreeMap<String, Fact>) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    for (path, fact) in facts {
        h.update(&(path.len() as u64).to_le_bytes());
        h.update(path.as_bytes());
        let file = canonical_row::<FileMeta>(&fact.file);
        h.update(&(file.len() as u64).to_le_bytes());
        h.update(&file);
        match &fact.note {
            Some(note) => {
                let note = canonical_row::<Note>(note);
                h.update(&(note.len() as u64).to_le_bytes());
                h.update(&note);
            }
            None => {
                h.update(&0u64.to_le_bytes());
            }
        }
    }
    *h.finalize().as_bytes()
}
/// Compares decoded facts without postcard map-order differences.
fn is_same_fact(left: Option<&Fact>, right: Option<&Fact>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.path == right.path
                && canonical_row::<FileMeta>(&left.file)
                    == canonical_row::<FileMeta>(&right.file)
                && left.note.as_deref().map(canonical_row::<Note>)
                    == right.note.as_deref().map(canonical_row::<Note>)
        }
        (None, None) => true,
        _ => false,
    }
}
/// Persists the facts with native Immediate durability.
fn persist_facts(root: &Path, facts: &BTreeMap<String, Fact>) -> Result<()> {
    let store = IndexStore::open(root)?;
    persist_facts_into(root, &store, facts, Durability::Immediate)
}
/// Uses the real rebuild transaction while the caller retains the store lock.
fn persist_facts_into(
    root: &Path,
    store: &IndexStore,
    facts: &BTreeMap<String, Fact>,
    durability: Durability,
) -> Result<()> {
    let mut files = Vec::with_capacity(facts.len());
    let mut notes = Vec::new();
    for fact in facts.values() {
        files.push(postcard::from_bytes::<FileMeta>(&fact.file)?);
        if let Some(bytes) = &fact.note {
            notes.push(postcard::from_bytes::<Note>(bytes)?);
        }
    }
    let index = traces::testing::WorkspaceIndex::for_probe_facts(files, notes);
    IndexerService::for_tests(root)
        .persist_into_for_probe(store, &index, durability)?;
    Ok(())
}
fn initialize(root: &Path, count: usize) -> Result<()> {
    fs::create_dir_all(root)?;
    for i in 0..count {
        let tasks = if i % 4 == 0 {
            0
        } else {
            1
        };
        let mut text = format!(
            "---\nclass: Project\nrating: {}\n---\n# Note {i} \
             #topic/{i}\n[[hub]]\n",
            i % 10
        );
        for _ in 0..tasks {
            text.push_str("- [ ] task 📅 2026-01-01\n");
        }
        fs::write(root.join(format!("note-{i}.md")), text)?;
    }
    fs::write(root.join("hub.md"), "# Hub #hub\n")?;
    let (snap, facts) = parse_current(root)?;
    persist_facts(root, &facts)?;
    write_identity(root, &Identity::first())?;
    println!(
        "seed notes={count} tasks={} sources={} revision=0",
        count * 3 / 4,
        snap.files.len()
    );
    Ok(())
}
fn encode(journal: &Journal) -> Result<Vec<u8>> {
    let body = postcard::to_stdvec(journal)?;
    if body.len() as u64 > MAX_JOURNAL {
        return Err(err("journal exceeds 64 MiB"));
    }
    let mut bytes = Vec::with_capacity(HEADER + body.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&EPOCH.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.extend_from_slice(blake3::hash(&bytes).as_bytes());
    bytes.extend_from_slice(blake3::hash(&body).as_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}
#[cfg(unix)]
fn open_journal(path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let flags = rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK;
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(flags.bits())?)
        .open(path)?;
    if !f.metadata()?.is_file() {
        return Err(err("journal is not a regular file"));
    }
    Ok(f)
}
#[cfg(not(unix))]
fn open_journal(path: &Path) -> Result<File> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(err("journal is not a regular file"));
    }
    Ok(File::open(path)?)
}
/// Rejects a leading varint length that exceeds the remaining body. The
/// journal body starts with the identity's `engine` String, so a hostile
/// body with valid checksums cannot drive a huge allocation before postcard
/// even begins.
fn leading_length_is_bounded(body: &[u8]) -> bool {
    let mut len = 0u64;
    let mut shift = 0u32;
    for (index, byte) in body.iter().enumerate().take(9) {
        len |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return u64::try_from(body.len().saturating_sub(index + 1))
                .map(|remaining| len <= remaining)
                .unwrap_or(false);
        }
        shift += 7;
    }
    false
}
fn decode(path: &Path) -> Result<Option<Journal>> {
    let mut file = match open_journal(path) {
        Ok(f) => f,
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let size = file.metadata()?.len();
    if size < HEADER as u64 || size > MAX_JOURNAL + HEADER as u64 {
        return Err(err("invalid journal length"));
    }
    let mut header = [0; HEADER];
    file.read_exact(&mut header)?;
    if &header[..8] != MAGIC {
        return Err(err("invalid journal magic"));
    }
    if blake3::hash(&header[..24]).as_bytes() != &header[24..56] {
        return Err(err("header checksum mismatch"));
    }
    let version = u32::from_le_bytes(header[8..12].try_into()?);
    let epoch = u32::from_le_bytes(header[12..16].try_into()?);
    if version != 1 || epoch > EPOCH {
        return Ok(None);
    }
    let length = u64::from_le_bytes(header[16..24].try_into()?);
    if length > MAX_JOURNAL || length + HEADER as u64 != size {
        return Err(err("journal body length mismatch"));
    }
    let mut body = vec![0; usize::try_from(length)?];
    file.read_exact(&mut body)?;
    if blake3::hash(&body).as_bytes() != &header[56..88] {
        return Err(err("journal body checksum mismatch"));
    }
    if !leading_length_is_bounded(&body) {
        return Err(err("journal body declares an impossible length"));
    }
    let journal: Journal = postcard::from_bytes(&body)?;
    if journal.base.epoch != epoch {
        return Err(err("journal epoch and identity differ"));
    }
    Ok(Some(journal))
}
fn publish(root: &Path, bytes: &[u8]) -> Result<()> {
    let staging = temp_path(root);
    if journal_path(root).exists() {
        return Err(err("unresolved journal occupies final path"));
    }
    let _ = fs::remove_file(&staging);
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staging)?
    };
    #[cfg(not(unix))]
    let mut file =
        OpenOptions::new().write(true).create_new(true).open(&staging)?;
    stop("before_write");
    fail("journal_write")?;
    file.write_all(&bytes[..bytes.len() / 2])?;
    stop("partial_write");
    file.write_all(&bytes[bytes.len() / 2..])?;
    stop("before_file_sync");
    fail("journal_file_sync")?;
    file.sync_all()?;
    stop("after_file_sync");
    drop(file);
    fs::rename(&staging, journal_path(root))?;
    stop("before_dir_sync");
    fail("journal_dir_sync")?;
    sync_dir(root.join(".traces"))?;
    stop("after_publication");
    Ok(())
}
#[cfg(target_os = "macos")]
fn sync_dir(path: PathBuf) -> Result<()> {
    let dir = File::open(path)?;
    use std::os::fd::AsRawFd;
    // SAFETY: fcntl reads the live file descriptor and F_FULLFSYNC takes no
    // argument.
    if unsafe { libc::fcntl(dir.as_raw_fd(), libc::F_FULLFSYNC) } == -1 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::EINVAL) {
            dir.sync_all()?;
            return Ok(());
        }
        return Err(error.into());
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn sync_dir(path: PathBuf) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
/// Applies the journal's changed paths onto the base fact set.
fn apply_changes(
    mut facts: BTreeMap<String, Fact>,
    journal: Journal,
) -> Result<BTreeMap<String, Fact>> {
    if journal.paths.len() != journal.changes.len() {
        return Err(err("journal path/changes mismatch"));
    }
    for (path, change) in journal.paths.into_iter().zip(journal.changes) {
        match change {
            Some(fact) if fact.path == path => {
                facts.insert(path, fact);
            }
            None => {
                facts.remove(&path);
            }
            _ => return Err(err("journal fact path mismatch")),
        }
    }
    Ok(facts)
}
fn stage(root: &Path, journaled: bool, defer_commit: bool) -> Result<()> {
    let store = IndexStore::open(root)?;
    if journal_path(root).exists() {
        return Err(err("resolve prior journal before stage"));
    }
    let base_hash = store_digest(store.database_for_probe())?;
    let previous = store_facts(store.database_for_probe())?;
    let path = root.join("note-0.md");
    fs::write(
        &path,
        "---\nclass: Area\nrating: 99\n---\n# Edited #updated\n[[hub]]\n- [x] \
         changed task\n",
    )?;
    let parse_start = Instant::now();
    let (snapshot, current) = parse_current(root)?;
    let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    let mut paths = Vec::new();
    let mut changes = Vec::new();
    let mut union = previous.keys().cloned().collect::<BTreeSet<_>>();
    union.extend(current.keys().cloned());
    for path in union {
        if !is_same_fact(previous.get(&path), current.get(&path)) {
            paths.push(path.clone());
            changes.push(current.get(&path).cloned());
        }
    }
    let journal = Journal {
        base: read_identity(root)?,
        base_hash,
        base_facts: facts_hash(&previous),
        target_facts: facts_hash(&current),
        snapshot,
        paths,
        changes,
    };
    let started = Instant::now();
    let encoded = if journaled {
        match encode(&journal) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                eprintln!("optional journal encoding failed: {error}");
                None
            }
        }
    } else {
        None
    };
    let (published, journal_bytes) = if let Some(bytes) = encoded {
        let length = bytes.len();
        match publish(root, &bytes) {
            Ok(()) => (true, length),
            Err(error) => {
                eprintln!("optional journal publication failed: {error}");
                if temp_path(root).exists() {
                    fs::remove_file(temp_path(root))?;
                }
                // A failure after rename leaves an untrusted final journal;
                // the immediate store commit supersedes it, so drop it.
                if journal_path(root).exists() {
                    fs::remove_file(journal_path(root))?;
                }
                (false, length)
            }
        }
    } else {
        (false, 0)
    };
    let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
    pause("after_publication")?;
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err("source changed before normal apply"));
    }
    let mut next = journal.base.clone();
    next.revision += 1;
    let actual_durability = if published {
        Durability::None
    } else {
        Durability::Immediate
    };
    if defer_commit && published {
        // Deferred mode: the journal is the only durable artifact; the
        // store commit is performed later by `checkpoint`.
        println!(
            "stage parse_ms={parse_ms:.3} encode_ms={encode_ms:.3} \
             commit_ms=0.000 journal_bytes={journal_bytes} changed={} \
             revision={} durability=None publication={}",
            journal.paths.len(),
            journal.base.revision,
            if published {
                "synced"
            } else {
                "fallback"
            }
        );
        return Ok(());
    }
    let start = Instant::now();
    stop("before_commit");
    persist_facts_into(root, &store, &current, actual_durability)?;
    stop("after_commit");
    let commit_ms = start.elapsed().as_secs_f64() * 1000.0;
    let checkpoint_start = Instant::now();
    if matches!(actual_durability, Durability::None) {
        store.checkpoint_for_probe()?;
    }
    let checkpoint_ms = ms(checkpoint_start);
    stop("after_checkpoint");
    write_identity(root, &next)?;
    if matches!(actual_durability, Durability::None)
        && journal_path(root).exists()
    {
        stop("before_unlink");
        fs::remove_file(journal_path(root))?;
        stop("after_unlink");
        sync_dir(root.join(".traces"))?;
    }
    println!(
        "stage parse_ms={parse_ms:.3} encode_ms={encode_ms:.3} \
         commit_ms={commit_ms:.3} checkpoint_ms={checkpoint_ms:.3} \
         journal_bytes={journal_bytes} changed={} revision={} \
         durability={actual_durability:?} final_durability=Immediate \
         publication={}",
        journal.paths.len(),
        next.revision,
        if published {
            "synced"
        } else if journaled {
            "fallback"
        } else {
            "disabled"
        }
    );
    Ok(())
}
fn checkpoint(root: &Path) -> Result<()> {
    let store = IndexStore::open(root)?;
    let path = journal_path(root);
    let journal = decode(&path)?
        .ok_or_else(|| err("no compatible journal to checkpoint"))?;
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err(
            "source changed before checkpoint; recover from source"
        ));
    }
    let identity = read_identity(root)?;
    let snapshot = journal.snapshot.clone();
    let digest = store_digest(store.database_for_probe());
    let facts_result = store_facts(store.database_for_probe());
    let base_facts_ok = match &facts_result {
        Ok(facts) => facts_hash(facts) == journal.base_facts,
        Err(_) => false,
    };
    if !(base_facts_ok
        && identity.is_compatible_with(&journal.base)
        && identity.incarnation == journal.base.incarnation
        && identity.revision == journal.base.revision
        && digest? == journal.base_hash)
    {
        return Err(err("checkpoint base check failed"));
    }
    stop("before_checkpoint_commit");
    pause("before_checkpoint_commit")?;
    if read_identity(root)? != identity {
        return Err(err("checkpoint identity changed; journal retained"));
    }
    let facts = apply_changes(facts_result?, journal)?;
    persist_facts_into(root, &store, &facts, Durability::Immediate)?;
    let mut next = identity;
    next.revision += 1;
    write_identity(root, &next)?;
    stop("after_checkpoint_commit");
    if !is_same_sources(&snapshot, root)? {
        return Err(err("source changed during checkpoint; journal retained"));
    }
    fs::remove_file(path)?;
    sync_dir(root.join(".traces"))?;
    println!("checkpoint outcome=durable revision={}", next.revision);
    Ok(())
}
fn fresh_query(root: &Path, reason: &str) -> Result<&'static str> {
    let fresh = std::sync::Arc::new(IndexerService::for_tests(root).build()?);
    let query = QueryService::new("class");
    let pages =
        query.run(&fresh, QueryBuilder::pages(SourceSelector::All)).len();
    let lists =
        query.run(&fresh, QueryBuilder::lists(SourceSelector::All)).len();
    let tasks =
        query.run(&fresh, QueryBuilder::tasks(SourceSelector::All)).len();
    println!(
        "query outcome=fresh-query reason={reason} parse_count={} \
         pages={pages} lists={lists} tasks={tasks} revision=unpersisted",
        fresh.entries().iter().filter(|e| e.note().is_some()).count()
    );
    Ok("fresh-query")
}
/// Flattens one query mode into comparable accessor tuples.
fn query_rows(store: &IndexStore, mode: &str) -> Result<Vec<(String, String)>> {
    let query = QueryService::new("class");
    let builder = match mode {
        "pages" => QueryBuilder::pages(SourceSelector::All),
        "lists" => QueryBuilder::lists(SourceSelector::All),
        _ => QueryBuilder::tasks(SourceSelector::All),
    };
    let rows: Vec<QueryRow> =
        query.run_from_store_for_probe(store, builder)?.into_iter().collect();
    Ok(rows
        .iter()
        .map(|row| {
            (
                row.file().path().to_string_lossy().into_owned(),
                format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                    row.task_completed(),
                    row.task_text(),
                    row.task_priority_rank(),
                    row.status_symbol(),
                    row.depth(),
                    row.line(),
                    row.parent()
                ),
            )
        })
        .collect())
}
/// Order-stable JSON view of a serializable row.
fn canonical_value<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}
/// Compares the full assembled entries (file metadata, note facts, and
/// inbound links) plus the selected query accessors against a fresh parse
/// of the same sources.
fn verify(root: &Path) -> Result<()> {
    let identity = read_identity(root)?;
    let service = IndexerService::for_tests(root);
    let fresh = service.build()?;
    let loaded = service.load()?;
    // `load` opens its own store handle, so the replayed handle below must
    // not be open yet.
    let replayed_store = IndexStore::open(root)?;
    // The query-row comparison needs a persisted store. A copy changes file
    // ctime values, so only the full-entry comparison uses the same root.
    let fresh_dir = tempfile::tempdir()?;
    for item in WalkDir::new(root).follow_links(false).into_iter().filter_entry(
        |entry| {
            !entry.file_type().is_dir()
                || entry.file_name().to_str() != Some(".traces")
        },
    ) {
        let item = item?;
        if !item.file_type().is_file() {
            continue;
        }
        let relative = item.path().strip_prefix(root)?;
        let target = fresh_dir.path().join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(item.path(), &target)?;
    }
    IndexerService::for_tests(fresh_dir.path()).persist(&fresh)?;
    let fresh_store = IndexStore::open(fresh_dir.path())?;
    let fresh_entries = fresh.entries();
    let loaded_entries = loaded.entries();
    if fresh_entries.len() != loaded_entries.len() {
        return Err(err(
            "entry counts differ between replayed and fresh stores"
        ));
    }
    for (fresh_entry, loaded_entry) in
        fresh_entries.iter().zip(loaded_entries.iter())
    {
        if fresh_entry.file().path() != loaded_entry.file().path()
            || canonical_value(fresh_entry.file())
                != canonical_value(loaded_entry.file())
            || fresh_entry.note().map(canonical_value)
                != loaded_entry.note().map(canonical_value)
            || fresh_entry.inlinks() != loaded_entry.inlinks()
        {
            return Err(err("entries differ between replayed and fresh store"));
        }
    }
    for mode in ["pages", "lists", "tasks"] {
        let replayed = query_rows(&replayed_store, mode)?;
        let fresh = query_rows(&fresh_store, mode)?;
        if replayed != fresh {
            return Err(format!(
                "{mode} query rows differ between replayed and fresh stores"
            )
            .into());
        }
    }
    println!(
        "verify result=equal store=real entries=full \
         query_modes=pages,lists,tasks incarnation={} revision={}",
        identity.incarnation, identity.revision
    );
    Ok(())
}
fn query(root: &Path) -> Result<()> {
    if env::var("PROBE_DB_RO").ok().as_deref() == Some("1") {
        // Read only the files table. This is not a full query-engine path.
        let db = redb::ReadOnlyDatabase::open(store_path(root))?;
        let txn = db.begin_read()?;
        let pages = txn.open_table(table("files"))?.iter()?.count();
        let revision = read_identity(root)?.revision;
        println!("query outcome=ro-read pages={pages} revision={revision}");
        return Ok(());
    }
    match IndexStore::open(root) {
        Ok(store) => {
            let query = QueryService::new("class");
            let pages = query
                .run_from_store_for_probe(
                    &store,
                    QueryBuilder::pages(SourceSelector::All),
                )?
                .len();
            let revision = read_identity(root)?.revision;
            println!(
                "query outcome=store-read pages={pages} revision={revision}"
            );
            Ok(())
        }
        Err(_) => {
            fresh_query(root, "store-open-failed")?;
            Ok(())
        }
    }
}
fn hold(root: &Path) -> Result<()> {
    let _db = open_db(root)?;
    println!("hold ready");
    std::io::stdout().flush()?;
    let _ = std::io::stdin().read(&mut [0u8; 1])?;
    Ok(())
}
/// Drops a stale or invalid journal so recovery falls through to reparse.
fn discard_journal(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
fn rebuild_from_source(root: &Path, new_incarnation: bool) -> Result<()> {
    let (snapshot, facts) = parse_current(root)?;
    stop("before_rebuild_commit");
    persist_facts(root, &facts)?;
    let mut identity = if new_incarnation {
        Identity::first()
    } else {
        read_identity(root)?
    };
    identity.revision += 1;
    write_identity(root, &identity)?;
    println!(
        "recover outcome=rebuild-from-source parse_count={} revision={} \
         incarnation={}",
        snapshot.files.iter().filter(|f| f.path.ends_with(".md")).count(),
        identity.revision,
        if new_incarnation {
            "new"
        } else {
            "same"
        }
    );
    Ok(())
}
fn recovery(root: &Path, no_replay: bool) -> Result<&'static str> {
    let path = journal_path(root);
    let store_file = store_path(root);
    let opened = Instant::now();
    if !store_file.exists() {
        match decode(&path) {
            Ok(None) if path.exists() => {
                fresh_query(root, "future-journal-preserved")?;
                return Ok("fresh-query");
            }
            _ => discard_journal(&path)?,
        }
        rebuild_from_source(root, true)?;
        println!(
            "recover outcome=missing-store-rebuilt store_reopen_ms={:.3}",
            opened.elapsed().as_secs_f64() * 1000.0
        );
        return Ok("missing-store-rebuilt");
    }
    // Run the optional repair diagnostic before the retained store lock.
    if env::var("PROBE_CHECK_INTEGRITY").ok().as_deref() == Some("1")
        && let Ok(mut db) = open_db(root)
    {
        let started = Instant::now();
        match db.check_integrity() {
            Ok(ok) => {
                eprintln!("integrity_ok={ok} integrity_ms={:.3}", ms(started))
            }
            Err(error) => eprintln!("integrity_skipped={error}"),
        }
    }
    let opened = Instant::now();
    let store = match IndexStore::open(root) {
        Ok(store) => store,
        Err(_) => {
            fresh_query(root, "store-open-failed")?;
            return Ok("fresh-query");
        }
    };
    let reopen_ms = ms(opened);
    let base_started = Instant::now();
    let digest_result = store_digest(store.database_for_probe());
    let facts_result = store_facts(store.database_for_probe());
    let base_snapshot_ms = ms(base_started);
    let weak_base = env::var("PROBE_WEAK_BASE").ok().as_deref() == Some("1");
    let identity = read_identity(root)?;
    let decode_started = Instant::now();
    let candidate = match decode(&path) {
        Ok(Some(j)) => Some(j),
        Ok(None) if path.exists() => {
            fresh_query(root, "future-journal-preserved")?;
            return Ok("fresh-query");
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("optional journal discarded: {e}");
            discard_journal(&path)?;
            None
        }
    };
    let decode_ms = ms(decode_started);
    if let Some(journal) = candidate {
        if no_replay {
            println!(
                "recover outcome=no-replay-control revision={}",
                identity.revision
            );
            return Ok("no-replay-control");
        }
        // Decided epoch rules: a journal from a lower epoch is stale, so it
        // is discarded and the source is reparsed. A higher epoch is
        // preserved untouched and served from a fresh query (decode already
        // routes epoch > EPOCH to the preserve path).
        if journal.base.epoch > identity.epoch {
            // Decided rule: a journal ahead of the stored epoch is preserved
            // untouched and served from a fresh query.
            fresh_query(root, "higher-epoch-journal-preserved")?;
            return Ok("fresh-query");
        }
        if journal.base.epoch < identity.epoch {
            eprintln!("journal epoch below store epoch; discarded");
            discard_journal(&path)?;
            drop(store);
            rebuild_from_source(root, false)?;
            return Ok("rebuild-from-source");
        }
        // Ticket-25 WEAK contract (PROBE_WEAK_BASE=1): compare only
        // incarnation+revision. A forked store with the same
        // incarnation+revision passes this base check, and the replay then
        // applies onto the forked facts. The target hash check rejects the
        // derived result and keeps the journal, so the fork cannot silently
        // commit. Only the base check stays weak; this is the negative
        // control for the missing digest backstop.
        let base_ok_weak = weak_base
            && identity.incarnation == journal.base.incarnation
            && identity.revision == journal.base.revision;
        let base_facts_ok = match &facts_result {
            Ok(facts) => facts_hash(facts) == journal.base_facts,
            Err(_) => false,
        };
        let base_ok_strong = !weak_base
            && base_facts_ok
            && identity.is_compatible_with(&journal.base)
            && identity.incarnation == journal.base.incarnation
            && identity.revision == journal.base.revision
            && digest_result? == journal.base_hash;
        if base_ok_weak || base_ok_strong {
            let started = Instant::now();
            // The negative control weakens only the base check.
            // Keep the target digest check for both base policies.
            let mut facts = if base_ok_weak {
                facts_result.unwrap_or_default()
            } else {
                facts_result?
            };
            // Stop between per-facet writes: the real replay commit is one
            // redb transaction, so an abort here can only precede it. The
            // rollback assertion (store unchanged, journal retained) is
            // therefore tautological for single-transaction commits.
            stop("mid_replay");
            if !is_same_sources(&journal.snapshot, root)? {
                // Raced edit detected after the final source check window.
                // Fail safe: drop the journal and reparse from source.
                eprintln!("journal discarded: source changed while replaying");
                discard_journal(&path)?;
                let (_, current_facts) = parse_current(root)?;
                persist_facts_into(
                    root,
                    &store,
                    &current_facts,
                    Durability::Immediate,
                )?;
                let mut raced = identity;
                raced.revision += 1;
                write_identity(root, &raced)?;
                println!(
                    "recover outcome=parsed store_reopen_ms={reopen_ms:.3} \
                     revision={}",
                    raced.revision
                );
                return Ok("parsed");
            }
            let target_facts = journal.target_facts;
            race_edit(root, &journal);
            facts = apply_changes(facts, journal)?;
            if facts_hash(&facts) != target_facts {
                return Err(err("journal target differs from derived facts"));
            }
            // Signal-class failpoint: park mid-replay so the parent can
            // deliver SIGKILL or SIGINT and test committed-state recovery.
            pause("replay")?;
            stop("before_commit");
            persist_facts_into(root, &store, &facts, Durability::Immediate)?;
            stop("after_replay_commit");
            let mut next = identity;
            next.revision += 1;
            write_identity(root, &next)?;
            fs::remove_file(&path)?;
            sync_dir(root.join(".traces"))?;
            println!(
                "recover outcome=replayed parse_count=0 \
                 store_reopen_ms={reopen_ms:.3} \
                 base_snapshot_ms={base_snapshot_ms:.3} \
                 decode_ms={decode_ms:.3} replay_ms={:.3} revision={}",
                started.elapsed().as_secs_f64() * 1000.0,
                next.revision
            );
            return Ok("replayed");
        }
        // A stale or invalid candidate cannot contribute partial facts.
        fs::remove_file(&path)?;
    }
    let (_, facts) = parse_current(root)?;
    if let Ok(previous) = facts_result
        && facts_hash(&facts) == facts_hash(&previous)
    {
        println!(
            "recover outcome=unchanged store_reopen_ms={reopen_ms:.3} \
             revision={}",
            identity.revision
        );
        return Ok("unchanged");
    }
    persist_facts_into(root, &store, &facts, Durability::Immediate)?;
    let mut next = identity;
    next.revision += 1;
    write_identity(root, &next)?;
    println!(
        "recover outcome=parsed store_reopen_ms={reopen_ms:.3} revision={}",
        next.revision
    );
    Ok("parsed")
}
/// Rewrites one decided identity part in the sidecar for mutation tests.
fn mutate_identity(root: &Path, part: &str) -> Result<()> {
    let mut identity = read_identity(root)?;
    match part {
        "engine" => identity.engine = "future-engine".into(),
        "platform" => identity.platform = "other".into(),
        "policy" => identity.policy[0] ^= 0xff,
        "extractor" => identity.extractor[0] ^= 0xff,
        "incarnation" => identity.incarnation = "mutated-incarnation".into(),
        "revision" => identity.revision += 5,
        "epoch-up" => identity.epoch += 1,
        "epoch-down" => identity.epoch = identity.epoch.saturating_sub(1),
        _ => return Err(err("unknown identity part")),
    }
    write_identity(root, &identity)
}
/// Measures matched policies with native redb transaction durability.
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn rss_peak_kb() -> u64 {
    let mut usage = libc::rusage {
        ru_utime: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_stime: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_maxrss: 0,
        ru_ixrss: 0,
        ru_idrss: 0,
        ru_isrss: 0,
        ru_minflt: 0,
        ru_majflt: 0,
        ru_nswap: 0,
        ru_inblock: 0,
        ru_oublock: 0,
        ru_msgsnd: 0,
        ru_msgrcv: 0,
        ru_nsignals: 0,
        ru_nvcsw: 0,
        ru_nivcsw: 0,
    };
    // SAFETY: getrusage reads the live process's own accounting.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return 0;
    }
    #[cfg(target_os = "macos")]
    let kb = (usage.ru_maxrss / 1024) as u64;
    #[cfg(not(target_os = "macos"))]
    let kb = usage.ru_maxrss as u64;
    kb
}
fn store_traces_dir(root: &Path) -> PathBuf {
    root.join(".traces")
}
/// Publishes the journal and separates the write and synchronization costs.
fn publish_measured(root: &Path, bytes: &[u8]) -> Result<(f64, f64, f64, f64)> {
    let total = Instant::now();
    let staging = temp_path(root);
    if journal_path(root).exists() || staging.exists() {
        return Err(err("measurement needs an empty journal slot"));
    }
    let mut file =
        OpenOptions::new().write(true).create_new(true).open(&staging)?;
    let phase = Instant::now();
    file.write_all(bytes)?;
    let write_ms = ms(phase);
    let phase = Instant::now();
    file.sync_all()?;
    let file_sync_ms = ms(phase);
    drop(file);
    fs::rename(&staging, journal_path(root))?;
    let phase = Instant::now();
    sync_dir(store_traces_dir(root))?;
    Ok((write_ms, file_sync_ms, ms(phase), ms(total)))
}
/// Builds the same journal `stage` would, for a single edited note.
fn measure_journal(root: &Path, edits: &[(String, String)]) -> Result<Journal> {
    let (base_hash, previous) = {
        let db = open_db(root)?;
        let state = (store_digest(&db)?, store_facts(&db)?);
        drop(db);
        state
    };
    for (path, body) in edits {
        fs::write(root.join(path), body)?;
    }
    let (snapshot, current) = parse_current(root)?;
    let mut paths = Vec::new();
    let mut changes = Vec::new();
    let mut union = previous.keys().cloned().collect::<BTreeSet<_>>();
    union.extend(current.keys().cloned());
    for path in union {
        if !is_same_fact(previous.get(&path), current.get(&path)) {
            paths.push(path.clone());
            changes.push(current.get(&path).cloned());
        }
    }
    Ok(Journal {
        base: read_identity(root)?,
        base_hash,
        base_facts: facts_hash(&previous),
        target_facts: facts_hash(&current),
        snapshot,
        paths,
        changes,
    })
}
/// One complete pass, including the deferred checkpoint and journal removal.
#[derive(Default)]
struct PassTiming {
    total_ms: f64,
    base_ms: f64,
    parse_ms: f64,
    encode_ms: f64,
    journal_bytes: u64,
    publish_write_ms: f64,
    publish_file_sync_ms: f64,
    publish_dir_sync_ms: f64,
    publish_other_ms: f64,
    validate_ms: f64,
    commit_ms: f64,
    durability_sync_ms: f64,
    removal_ms: f64,
    removal_sync_ms: f64,
}
impl PassTiming {
    fn print(&self, policy: &str) {
        println!(
            "components policy={policy} total_ms={:.3} base_ms={:.3} \
             parse_ms={:.3} encode_ms={:.3} publish_write_ms={:.3} \
             publish_file_sync_ms={:.3} publish_dir_sync_ms={:.3} \
             publish_other_ms={:.3} validate_ms={:.3} native_commit_ms={:.3} \
             native_checkpoint_ms={:.3} removal_ms={:.3} \
             removal_dir_sync_ms={:.3} journal_bytes={} rss_peak_kb={} \
             redb_sync_split=unavailable model=current8tables \
             updates=full-rebuild parse_passes=1",
            self.total_ms,
            self.base_ms,
            self.parse_ms,
            self.encode_ms,
            self.publish_write_ms,
            self.publish_file_sync_ms,
            self.publish_dir_sync_ms,
            self.publish_other_ms,
            self.validate_ms,
            self.commit_ms,
            self.durability_sync_ms,
            self.removal_ms,
            self.removal_sync_ms,
            self.journal_bytes,
            rss_peak_kb()
        );
    }
}
fn policy_pass(root: &Path, policy: &str) -> Result<PassTiming> {
    if !matches!(policy, "a" | "b" | "c") {
        return Err(err("unknown measure policy"));
    }
    fs::write(root.join("note-0.md"), EDITED_NOTE_0)?;
    let total = Instant::now();
    let mut timing = PassTiming::default();
    let phase = Instant::now();
    let store = IndexStore::open(root)?;
    if journal_path(root).exists() {
        return Err(err("measurement needs an empty journal slot"));
    }
    let previous = store_facts(store.database_for_probe())?;
    let base_hash = store_digest(store.database_for_probe())?;
    let identity = read_identity(root)?;
    timing.base_ms = ms(phase);
    let phase = Instant::now();
    let (snapshot, current) = parse_current(root)?;
    timing.parse_ms = ms(phase);
    let phase = Instant::now();
    let mut paths = Vec::new();
    let mut changes = Vec::new();
    for (path, fact) in &current {
        if !is_same_fact(previous.get(path), Some(fact)) {
            paths.push(path.to_owned());
            changes.push(Some(fact.to_owned()));
        }
    }
    for path in previous.keys() {
        if !current.contains_key(path) {
            paths.push(path.to_owned());
            changes.push(None);
        }
    }
    let target_facts = facts_hash(&current);
    let journal = Journal {
        base: identity,
        base_hash,
        base_facts: facts_hash(&previous),
        target_facts,
        snapshot,
        paths,
        changes,
    };
    if journal.paths.as_slice() != ["note-0.md"] {
        return Err(err("policy pass needs exactly one changed source"));
    }
    timing.validate_ms = ms(phase);
    if policy != "a" {
        let phase = Instant::now();
        let bytes = encode(&journal)?;
        timing.encode_ms = ms(phase);
        timing.journal_bytes = bytes.len() as u64;
        let (write, file_sync, dir_sync, publication) =
            publish_measured(root, &bytes)?;
        timing.publish_write_ms = write;
        timing.publish_file_sync_ms = file_sync;
        timing.publish_dir_sync_ms = dir_sync;
        timing.publish_other_ms = publication - write - file_sync - dir_sync;
    }
    let phase = Instant::now();
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err("source changed during policy pass"));
    }
    timing.validate_ms += ms(phase);
    let phase = Instant::now();
    let durability = if policy == "c" {
        Durability::None
    } else {
        Durability::Immediate
    };
    persist_facts_into(root, &store, &current, durability)?;
    timing.commit_ms = ms(phase);
    if policy == "c" {
        let phase = Instant::now();
        store.checkpoint_for_probe()?;
        timing.durability_sync_ms = ms(phase);
    }
    let phase = Instant::now();
    if facts_hash(&store_facts(store.database_for_probe())?) != target_facts
        || !is_same_sources(&journal.snapshot, root)?
    {
        return Err(err("policy target check failed"));
    }
    let mut next = journal.base;
    next.revision += 1;
    write_identity(root, &next)?;
    timing.validate_ms += ms(phase);
    if policy != "a" {
        let phase = Instant::now();
        fs::remove_file(journal_path(root))?;
        timing.removal_ms = ms(phase);
        let phase = Instant::now();
        sync_dir(store_traces_dir(root))?;
        timing.removal_sync_ms = ms(phase);
    }
    drop(store);
    timing.total_ms = ms(total);
    timing.print(policy);
    Ok(timing)
}
/// Reads the complete raw store digest without a writable database handle.
fn control_pass(root: &Path) -> Result<f64> {
    let start = Instant::now();
    let db = redb::ReadOnlyDatabase::open(store_path(root))?;
    std::hint::black_box(store_digest(&db)?);
    drop(db);
    Ok(ms(start))
}
/// Source-check placement A (before writer hold) versus B (inside hold).
fn measure_source_check_placement(
    root: &Path,
    placement: &str,
) -> Result<(f64, f64)> {
    let total = Instant::now();
    let scan_start = unix_nanos(SystemTime::now())?;
    if placement == "A" {
        std::hint::black_box(snapshot(root, scan_start)?);
    } else if placement != "B" {
        return Err(err("unknown placement"));
    }
    let db = open_db(root)?;
    let hold = Instant::now();
    let txn = db.begin_write()?;
    if placement == "B" {
        std::hint::black_box(snapshot(root, scan_start)?);
    }
    std::hint::black_box(store_digest(&db)?);
    drop(txn);
    drop(db);
    Ok((ms(total), ms(hold)))
}
fn set_all_mtimes(root: &Path, now: SystemTime) -> Result<()> {
    for item in WalkDir::new(root).follow_links(false).into_iter().filter_entry(
        |entry| {
            !entry.file_type().is_dir()
                || entry.file_name().to_str() != Some(".traces")
        },
    ) {
        let item = item?;
        if item.file_type().is_file() {
            File::open(item.path())?.set_modified(now)?;
        }
    }
    Ok(())
}
fn first_and_repeat_query(root: &Path) -> Result<(f64, f64)> {
    let start = Instant::now();
    let store = IndexStore::open(root)?;
    let query = QueryService::new("class");
    let pages = query
        .run_from_store_for_probe(
            &store,
            QueryBuilder::pages(SourceSelector::All),
        )?
        .len();
    let _ = pages;
    let first = ms(start);
    let start = Instant::now();
    let pages = query
        .run_from_store_for_probe(
            &store,
            QueryBuilder::pages(SourceSelector::All),
        )?
        .len();
    let _ = pages;
    Ok((first, ms(start)))
}
/// Runs serial samples in new processes with the existing per-file cache hint.
fn measure(root: &Path, notes: usize, runs: usize) -> Result<()> {
    if notes < 9 || runs == 0 {
        return Err(err("measurement needs nine notes and positive runs"));
    }
    fs::create_dir_all(root)?;
    println!(
        "measure fixture_notes={notes} runs_per_case={runs} \
         process=fresh-child storage_cache=best-effort-file-hint \
         durability_immediate_mode=native-redb policy_order=a,b,a,c,a \
         model=current8tables full11facets=false"
    );
    let executable = env::current_exe()?;
    for (block, case) in [
        "a",
        "b",
        "a",
        "c",
        "a",
        "source-A",
        "source-B",
        "snapshot-stable",
        "snapshot-racy",
        "ro-no-journal",
        "ro-retained-journal",
        "recover-no-change",
        "recover-edit-1",
        "recover-edit-100",
        "recover-links-add-delete-rename",
        "recover-class-source-change",
        "fallback-no-change",
        "fallback-edit-1",
        "fallback-edit-100",
        "fallback-links-add-delete-rename",
        "fallback-class-source-change",
        "stop-pre-journal",
        "stop-post-publish",
        "stop-post-commit",
    ]
    .into_iter()
    .enumerate()
    {
        if env::var("T27_STOP_ONLY").is_ok() && !case.starts_with("stop-") {
            continue;
        }
        if env::var("T27_PHASES_ONLY").is_ok()
            && case != "a"
            && case != "b"
            && case != "c"
        {
            continue;
        }
        for run in 0..runs {
            let fixture = tempfile::tempdir_in(root)?;
            let dir = fixture.path().join("vault");
            initialize(&dir, notes)?;
            if case.starts_with("recover-") || case.starts_with("fallback-") {
                prepare_recovery_measurement(&dir, case, notes)?;
            } else if case == "ro-retained-journal" {
                let journal = measure_journal(&dir, &[])?;
                publish(&dir, &encode(&journal)?)?;
            } else if case == "stop-pre-journal" {
                fs::write(dir.join("note-0.md"), "# changed\n")?;
            } else if case == "stop-post-publish" {
                let journal = measure_journal(&dir, &[(
                    "note-0.md".into(),
                    "# changed\n".into(),
                )])?;
                publish(&dir, &encode(&journal)?)?;
            } else if case == "stop-post-commit" {
                prepare_stop_post_commit(&dir)?;
            }
            hint_fixture_cache(&dir)?;
            println!(
                "sample block={block} case={case} run={run} \
                 fixture_notes={notes} process=fresh-child"
            );
            std::io::stdout().flush()?;
            let status = process::Command::new(&executable)
                .arg("measure-case")
                .arg(&dir)
                .arg(case)
                .status()?;
            if !status.success() {
                return Err(format!(
                    "measurement child failed: case={case} run={run} \
                     status={status}"
                )
                .into());
            }
        }
    }
    Ok(())
}

/// Applies the benchmark cache hint to the store, sidecars, and sources.
fn hint_fixture_cache(root: &Path) -> Result<()> {
    if cache::index_db_path(root) != store_path(root) {
        return Err(err("benchmark and probe store paths differ"));
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            cache::drop_page_cache(entry.path())?;
        }
    }
    Ok(())
}

/// Prepares the post-commit stop state: the store already holds the target
/// facts and the bumped identity, and the journal still occupies its final
/// path because the stop landed before the unlink barrier.
fn prepare_stop_post_commit(root: &Path) -> Result<()> {
    let journal =
        measure_journal(root, &[("note-0.md".into(), "# changed\n".into())])?;
    let bytes = encode(&journal)?;
    println!(
        "prepared case=stop-post-commit changed={} journal_bytes={}",
        journal.paths.len(),
        bytes.len()
    );
    publish(root, &bytes)?;
    let (_, facts) = parse_current(root)?;
    let store = IndexStore::open(root)?;
    persist_facts_into(root, &store, &facts, Durability::Immediate)?;
    let mut next = read_identity(root)?;
    next.revision += 1;
    write_identity(root, &next)?;
    Ok(())
}

/// Prepares recovery work before the measured child starts.
fn prepare_recovery_measurement(
    root: &Path,
    case: &str,
    notes: usize,
) -> Result<()> {
    let scenario = case
        .split_once('-')
        .map(|(_, tail)| tail)
        .ok_or_else(|| err("missing recovery scenario"))?;
    let mut edits = Vec::new();
    match scenario {
        "no-change" => {}
        "edit-1" => edits.push(("note-0.md".into(), "# changed\n".into())),
        "edit-100" => {
            for i in 0..notes.min(100) {
                edits.push((format!("note-{i}.md"), "# bulk\n".into()));
            }
        }
        "links-add-delete-rename" => {
            fs::write(root.join("added.md"), "# added [[note-5]]\n")?;
            fs::remove_file(root.join("note-7.md"))?;
            fs::rename(root.join("note-8.md"), root.join("renamed.md"))?;
        }
        "class-source-change" => {
            for i in 0..notes {
                let path = format!("note-{i}.md");
                let body = fs::read_to_string(root.join(&path))?
                    .replace("class: Project", "class: Area");
                edits.push((path, body));
            }
        }
        _ => return Err(err("unknown recovery measurement scenario")),
    }
    let journal = measure_journal(root, &edits)?;
    let bytes = encode(&journal)?;
    println!(
        "prepared case={case} changed={} journal_bytes={}",
        journal.paths.len(),
        bytes.len()
    );
    publish(root, &bytes)?;
    if case.starts_with("fallback-") {
        fs::write(root.join("hub.md"), "# Hub changed #hub\n")?;
    }
    Ok(())
}

/// Measures one prepared fixture without a warmup pass.
fn measure_case(root: &Path, case: &str) -> Result<()> {
    match case {
        "a" | "b" | "c" => {
            policy_pass(root, case)?;
        }
        "source-A" | "source-B" => {
            let placement = if case == "source-A" {
                "A"
            } else {
                "B"
            };
            let (total, hold) =
                measure_source_check_placement(root, placement)?;
            println!(
                "components case={case} total_ms={total:.3} \
                 owner_hold_ms={hold:.3} updates=none source_hash=all-files"
            );
        }
        "snapshot-stable" | "snapshot-racy" => {
            let now = SystemTime::now();
            let racy = case == "snapshot-racy";
            let mtime = if racy {
                now
            } else {
                now.checked_sub(std::time::Duration::from_secs(5)).ok_or_else(
                    || err("clock cannot represent stable fixture"),
                )?
            };
            set_all_mtimes(root, mtime)?;
            hint_fixture_cache(root)?;
            let scan_start = unix_nanos(SystemTime::now())?;
            let started = Instant::now();
            let sources = snapshot(root, scan_start)?;
            let elapsed = ms(started);
            if sources.files.iter().any(|source| source.racy != racy) {
                return Err(err(
                    "snapshot racy classification differs from fixture"
                ));
            }
            println!(
                "components case={case} snapshot_ms={elapsed:.3} sources={} \
                 racy={} source_hash=all-files",
                sources.files.len(),
                racy
            );
        }
        "ro-no-journal" | "ro-retained-journal" => {
            let before = fs::read(journal_path(root)).ok();
            let elapsed = control_pass(root)?;
            if fs::read(journal_path(root)).ok() != before {
                return Err(err("read-only control changed the journal"));
            }
            println!(
                "components case={case} read_only_digest_ms={elapsed:.3} \
                 scope=raw-eight-table-digest query_engine=false writes=0"
            );
        }
        case if case.starts_with("recover-")
            || case.starts_with("fallback-")
            || case.starts_with("stop-") =>
        {
            let started = Instant::now();
            let outcome = recovery(root, false)?;
            let elapsed = ms(started);
            let (first, repeat) = first_and_repeat_query(root)?;
            println!(
                "components case={case} outcome={outcome} \
                 recovery_ms={elapsed:.3} first_query_ms={first:.3} \
                 repeat_query_ms={repeat:.3} rss_peak_kb={}",
                rss_peak_kb()
            );
            verify(root)?;
        }
        _ => return Err(err("unknown measurement case")),
    }
    Ok(())
}
/// Decided ENOSPC path: attempt the store write; on failure drop the handle,
/// reopen, read identity and revision, classify, and return the existing
/// error. No retry.
fn enospc_write(root: &Path) -> Result<()> {
    let (_, facts) = parse_current(root)?;
    if let Err(error) = persist_facts(root, &facts) {
        match Database::open(store_path(root)) {
            Ok(db) => {
                let revision = read_identity(root)
                    .map(|identity| identity.revision)
                    .unwrap_or(u64::MAX);
                drop(db);
                eprintln!(
                    "enospc classify=reopen-ok revision={revision} \
                     store_error={error}"
                );
            }
            Err(reopen_error) => {
                eprintln!(
                    "enospc classify=reopen-failed \
                     reopen_error={reopen_error} store_error={error}"
                );
            }
        }
        return Err(error);
    }
    println!("enospc outcome=written");
    Ok(())
}
fn main() -> process::ExitCode {
    if let Err(error) = run() {
        eprintln!("probe error: {error}");
        return process::ExitCode::from(2);
    }
    process::ExitCode::SUCCESS
}
fn run() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let command = args.get(1).ok_or_else(|| {
        err("usage: replay-fault-probe \
             seed|stage|recover|checkpoint|query|hold|verify|mutate-identity|measure \
             ROOT [count|none|deferred|no-journal|no-replay|part] [runs]")
    })?;
    let root = Path::new(args.get(2).ok_or_else(|| err("missing ROOT"))?);
    match command.as_str() {
        "seed" => initialize(
            root,
            args.get(3).map(String::as_str).unwrap_or("12").parse()?,
        ),
        "stage" => stage(
            root,
            !args.get(3).is_some_and(|s| s == "no-journal"),
            args.get(3).is_some_and(|s| s == "deferred"),
        ),
        "recover" => {
            recovery(root, args.get(3).is_some_and(|s| s == "no-replay"))?;
            Ok(())
        }
        "checkpoint" => checkpoint(root),
        "query" => query(root),
        "hold" => hold(root),
        "verify" => verify(root),
        "mutate-identity" => mutate_identity(
            root,
            args.get(3).map(String::as_str).unwrap_or("policy"),
        ),
        "measure" => measure(
            root,
            args.get(3).map(String::as_str).unwrap_or("5000").parse()?,
            args.get(4).map(String::as_str).unwrap_or("20").parse()?,
        ),
        "measure-case" => measure_case(
            root,
            args.get(3).ok_or_else(|| err("missing measurement case"))?,
        ),
        "enospc-write" => enospc_write(root),
        _ => Err(err("unknown command")),
    }
}
