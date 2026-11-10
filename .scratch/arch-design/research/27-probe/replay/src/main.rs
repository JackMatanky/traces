//! Throwaway ticket-27 fact journal. This is a shadow index, not the production
//! adapter.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use redb::{
    Database, Durability, ReadableDatabase, ReadableTable, TableDefinition,
};
use serde::{Deserialize, Serialize};
use traces::{
    FileMeta, ListItem, Note,
    testing::{
        IndexerService, InlinkMap, QueryBuilder, QueryService, SourceSelector,
    },
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TABLES: [&str; 11] = [
    "files",
    "notes",
    "lists",
    "tasks",
    "tags_by_path",
    "paths_by_tag",
    "classes_by_path",
    "paths_by_class",
    "links",
    "ranges",
    "notices",
];
const MAGIC: &[u8; 8] = b"T27FACT!";
const MAX_JOURNAL: u64 = 64 * 1024 * 1024;
const HEADER: usize = 8 + 4 + 4 + 8 + 32 + 32;
const EPOCH: u32 = 1;
const ENGINE: &str = "redb-shadow-v1";
const META: TableDefinition<&str, &[u8]> = TableDefinition::new("identity");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Identity {
    engine: String,
    platform: String,
    incarnation: String,
    revision: u64,
    epoch: u32,
    policy: [u8; 32],
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
                b"default parser/class policy; fixture 27; epoch 1",
            )
            .as_bytes(),
        }
    }

    fn is_compatible(&self, other: &Self) -> bool {
        self.engine == other.engine
            && self.platform == other.platform
            && self.incarnation == other.incarnation
            && self.epoch == other.epoch
            && self.policy == other.policy
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
#[derive(Clone, Serialize, Deserialize)]
struct Fact {
    path: String,
    file: Vec<u8>,
    note: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    base: Identity,
    base_hash: [u8; 32],
    target_hash: [u8; 32],
    snapshot: Snapshot,
    changes: Vec<Option<Fact>>,
    paths: Vec<String>,
    old_keys: Vec<(String, Vec<String>)>,
    affected_edges: Vec<(String, Vec<String>)>,
    notices: Vec<String>,
}
#[derive(Default)]
struct Rows {
    tables: BTreeMap<&'static str, BTreeMap<String, Vec<u8>>>,
}
impl Rows {
    fn put(
        &mut self,
        table: &'static str,
        key: String,
        value: impl Serialize,
    ) -> Result<()> {
        self.tables
            .entry(table)
            .or_default()
            .insert(key, postcard::to_stdvec(&value)?);
        Ok(())
    }

    fn get(&self, table: &str, key: &str) -> Option<&[u8]> {
        self.tables.get(table)?.get(key).map(Vec::as_slice)
    }
}
fn err(message: &'static str) -> Box<dyn Error> {
    message.into()
}
fn table(
    name: &'static str,
) -> TableDefinition<'static, &'static str, &'static [u8]> {
    TableDefinition::new(name)
}
fn db_path(root: &Path) -> PathBuf {
    root.join(".traces/replay-shadow.redb")
}
fn journal_path(root: &Path) -> PathBuf {
    root.join(".traces/index.redb.journal")
}
fn temp_path(root: &Path) -> PathBuf {
    root.join(".traces/index.redb.journal.tmp")
}
fn stop(point: &str) {
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
#[cfg(target_os = "macos")]
fn sync_file(file: &File) -> Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: fcntl reads the live file descriptor and F_FULLFSYNC takes no
    // argument.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_FULLFSYNC) } == -1 {
        let error = std::io::Error::last_os_error();
        if file.metadata()?.is_dir()
            && error.raw_os_error() == Some(libc::EINVAL)
        {
            eprintln!(
                "probe sync: directory F_FULLFSYNC unsupported; using fsync \
                 instead"
            );
            file.sync_all()?;
            return Ok(());
        }
        return Err(error.into());
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn sync_file(file: &File) -> Result<()> {
    Ok(file.sync_all()?)
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
    for item in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_type().is_dir()
                || !matches!(
                    entry.file_name().to_str(),
                    Some(".git" | ".traces" | "target" | "node_modules")
                )
                || entry.depth() == 0
        })
    {
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
        let bytes = fs::read(item.path())?;
        let meta = fs::metadata(item.path())?;
        let mtime = unix_nanos(meta.modified()?)?;
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
fn parsed(root: &Path) -> Result<(Snapshot, Vec<Fact>)> {
    let start = unix_nanos(SystemTime::now())?;
    let before = snapshot(root, start)?;
    let index = IndexerService::for_tests(root).build()?;
    let after = snapshot(root, start)?;
    if before != after {
        return Err(err("source changed during parse"));
    }
    let mut facts = Vec::with_capacity(index.entries().len());
    for entry in index.entries() {
        facts.push(Fact {
            path: entry
                .file()
                .path()
                .to_str()
                .ok_or_else(|| err("non-UTF8 indexed path"))?
                .to_owned(),
            file: postcard::to_stdvec(entry.file())?,
            note: entry.note().map(postcard::to_stdvec).transpose()?,
        });
    }
    if facts.len() != before.files.len() {
        return Err(err("index scan and source snapshot disagree"));
    }
    Ok((before, facts))
}
fn load(db: &Database) -> Result<(Identity, Rows)> {
    let tx = db.begin_read()?;
    let meta = tx.open_table(META)?;
    let identity: Identity = postcard::from_bytes(
        meta.get("current")?.ok_or_else(|| err("missing identity"))?.value(),
    )?;
    drop(meta);
    let mut rows = Rows::default();
    for name in TABLES {
        let t = tx.open_table(table(name))?;
        let mut data = BTreeMap::new();
        for entry in t.iter()? {
            let (k, v) = entry?;
            data.insert(k.value().to_owned(), v.value().to_vec());
        }
        rows.tables.insert(name, data);
    }
    Ok((identity, rows))
}
fn hash_rows(rows: &Rows) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    for name in TABLES {
        h.update(name.as_bytes());
        if let Some(table) = rows.tables.get(name) {
            for (key, value) in table {
                h.update(&(key.len() as u64).to_le_bytes());
                h.update(key.as_bytes());
                h.update(&(value.len() as u64).to_le_bytes());
                h.update(value);
            }
        }
    }
    *h.finalize().as_bytes()
}
fn facts_from(rows: &Rows) -> Result<BTreeMap<String, Fact>> {
    let mut all = BTreeMap::new();
    for (key, file) in
        rows.tables.get("files").ok_or_else(|| err("files missing"))?
    {
        all.insert(key.to_owned(), Fact {
            path: key.to_owned(),
            file: postcard::from_bytes(file)?,
            note: rows
                .get("notes", key)
                .map(postcard::from_bytes)
                .transpose()?,
        });
    }
    Ok(all)
}
fn derived(all: &BTreeMap<String, Fact>) -> Result<Rows> {
    let mut rows = Rows::default();
    for name in TABLES {
        rows.tables.insert(name, BTreeMap::new());
    }
    let mut notes = Vec::new();
    let mut files = Vec::new();
    for (path, fact) in all {
        let file: FileMeta = postcard::from_bytes(&fact.file)?;
        if file.path().to_str() != Some(path.as_str()) {
            return Err(err("fact file path differs from key"));
        }
        rows.put("files", path.to_owned(), &fact.file)?;
        if let Some(bytes) = &fact.note {
            let note: Note = postcard::from_bytes(bytes)?;
            if note.path() != file.path() {
                return Err(err("fact note path differs from key"));
            }
            rows.put("notes", path.to_owned(), bytes)?;
            rows.put("lists", path.to_owned(), note.lists())?;
            let tasks: Vec<_> = note.tasks().collect();
            rows.put("tasks", path.to_owned(), tasks)?;
            let tags: Vec<_> =
                note.tags().iter().map(|t| t.as_str().to_owned()).collect();
            rows.put("tags_by_path", path.to_owned(), &tags)?;
            for tag in tags {
                rows.put("paths_by_tag", format!("{tag}\0{path}"), ())?;
            }
            let fm = serde_json::to_value(note.frontmatter())?;
            let class = fm.get("fields").and_then(|v| v.get("class"));
            let values =
                class.and_then(|v| v.get("List")).and_then(|v| v.as_array());
            let classes: Vec<String> = values
                .into_iter()
                .flatten()
                .chain(class.filter(|_| values.is_none()))
                .filter_map(|v| v.get("String").and_then(|s| s.as_str()))
                .map(str::to_lowercase)
                .collect();
            rows.put("classes_by_path", path.to_owned(), &classes)?;
            for class in classes {
                rows.put("paths_by_class", format!("{class}\0{path}"), ())?;
            }
            // A file-level range key; finer date/number keys need the decided
            // facet schema.
            let size = serde_json::to_value(&file)?
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| err("file size missing"))?;
            rows.put("ranges", path.to_owned(), size)?;
            notes.push(note);
        }
        files.push(file);
    }
    let graph = InlinkMap::new(&notes, &files);
    for file in &files {
        rows.put(
            "links",
            file.path().to_string_lossy().into_owned(),
            graph.inlinks_of(file.path()),
        )?;
    }
    Ok(rows)
}
fn persist(
    db: &Database,
    base: Option<&Identity>,
    base_hash: Option<&[u8; 32]>,
    next: &Identity,
    rows: &Rows,
    durability: Durability,
) -> Result<()> {
    let mut tx = db.begin_write()?;
    if let Some(base) = base {
        let recorded = tx.open_table(META)?;
        let actual: Identity = postcard::from_bytes(
            recorded
                .get("current")?
                .ok_or_else(|| err("store base disappeared"))?
                .value(),
        )?;
        if &actual != base {
            return Err(err("writer base identity changed"));
        }
        drop(recorded);
        // The database is exclusively open by this process; verify affected
        // rows, not only revision.
        if Some(&hash_rows(&load(db)?.1)) != base_hash {
            return Err(err("writer base rows changed"));
        }
    }
    for name in TABLES {
        match tx.delete_table(table(name)) {
            Ok(_) | Err(redb::TableError::TableDoesNotExist(_)) => (),
            Err(error) => return Err(error.into()),
        }
        let mut t = tx.open_table(table(name))?;
        if let Some(data) = rows.tables.get(name) {
            for (key, value) in data {
                t.insert(key.as_str(), value.as_slice())?;
            }
        }
    }
    {
        let mut t = tx.open_table(META)?;
        t.insert("current", postcard::to_stdvec(next)?.as_slice())?;
    }
    tx.set_durability(durability)?;
    fail("store_commit")?;
    stop("before_commit");
    tx.commit()?;
    stop("after_commit");
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
    let (snap, facts) = parsed(root)?;
    let all =
        facts.into_iter().map(|fact| (fact.path.to_owned(), fact)).collect();
    fs::create_dir_all(root.join(".traces"))?;
    let db = Database::create(db_path(root))?;
    let identity = Identity::first();
    persist(
        &db,
        None,
        None,
        &identity,
        &derived(&all)?,
        Durability::Immediate,
    )?;
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
    let journal: Journal = postcard::from_bytes(&body)?;
    if journal.base.epoch != epoch {
        return Err(err("journal epoch and identity differ"));
    }
    Ok(Some(journal))
}
fn publish(root: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temp_path(root);
    if journal_path(root).exists() {
        return Err(err("unresolved journal occupies final path"));
    }
    let _ = fs::remove_file(&tmp);
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?
    };
    #[cfg(not(unix))]
    let mut file =
        OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    stop("before_write");
    fail("journal_write")?;
    file.write_all(&bytes[..bytes.len() / 2])?;
    stop("partial_write");
    file.write_all(&bytes[bytes.len() / 2..])?;
    stop("before_file_sync");
    fail("journal_file_sync")?;
    sync_file(&file)?;
    stop("after_file_sync");
    drop(file);
    fs::rename(&tmp, journal_path(root))?;
    stop("before_dir_sync");
    fail("journal_dir_sync")?;
    sync_file(&File::open(root.join(".traces"))?)?;
    stop("after_publication");
    Ok(())
}
fn stage(root: &Path, durability: Durability, journaled: bool) -> Result<()> {
    let db = Database::create(db_path(root))?;
    let (base, previous) = load(&db)?;
    if journal_path(root).exists() {
        return Err(err("resolve prior journal before stage"));
    }
    let path = root.join("note-0.md");
    fs::write(
        &path,
        "---\nclass: Area\nrating: 99\n---\n# Edited #updated\n[[hub]]\n- [x] \
         changed task\n",
    )?;
    let parse_start = Instant::now();
    let (snapshot, current) = parsed(root)?;
    let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    let all = facts_from(&previous)?;
    let mut new = BTreeMap::new();
    for fact in current {
        new.insert(fact.path.to_owned(), fact);
    }
    let mut paths = Vec::new();
    for path in all.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        let old = all.get(path);
        let fresh = new.get(path);
        if old.map(|x| (&x.file, &x.note)) != fresh.map(|x| (&x.file, &x.note))
        {
            paths.push(path.to_owned());
        }
    }
    let changes: Vec<_> = paths
        .iter()
        .map(|path| {
            new.get(path).map(|x| Fact {
                path: x.path.to_owned(),
                file: x.file.to_vec(),
                note: x.note.clone(),
            })
        })
        .collect();
    let old_keys = paths
        .iter()
        .map(|path| {
            let keys = previous
                .tables
                .iter()
                .flat_map(|(name, entries)| {
                    entries
                        .keys()
                        .filter(move |key| {
                            *key == path
                                || key.rsplit_once('\0').is_some_and(
                                    |(_, suffix)| suffix == path.as_str(),
                                )
                        })
                        .map(move |key| format!("{name}:{key}"))
                })
                .collect();
            (path.to_owned(), keys)
        })
        .collect();
    let mut affected_edges = Vec::new();
    for (target, bytes) in &previous.tables["links"] {
        let sources: Vec<PathBuf> = postcard::from_bytes(bytes)?;
        if paths.iter().any(|path| {
            path == target
                || sources.iter().any(|src| src.to_str() == Some(path.as_str()))
        }) {
            affected_edges.push((
                target.to_owned(),
                sources
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            ));
        }
    }
    let target_rows = derived(&new)?;
    let journal = Journal {
        base: base.clone(),
        base_hash: hash_rows(&previous),
        target_hash: hash_rows(&target_rows),
        snapshot,
        changes,
        paths,
        old_keys,
        affected_edges,
        notices: Vec::new(),
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
                (false, length)
            }
        }
    } else {
        (false, 0)
    };
    let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err("source changed before normal apply"));
    }
    let mut next = base.clone();
    next.revision += 1;
    let actual_durability = if published {
        durability
    } else {
        Durability::Immediate
    };
    let start = Instant::now();
    persist(
        &db,
        Some(&base),
        Some(&journal.base_hash),
        &next,
        &target_rows,
        actual_durability,
    )?;
    let commit_ms = start.elapsed().as_secs_f64() * 1000.0;
    if matches!(actual_durability, Durability::Immediate)
        && journal_path(root).exists()
    {
        stop("before_unlink");
        fs::remove_file(journal_path(root))?;
        stop("after_unlink");
    }
    println!(
        "stage parse_ms={parse_ms:.3} encode_ms={encode_ms:.3} \
         commit_ms={commit_ms:.3} journal_bytes={journal_bytes} changed={} \
         revision={} durability={actual_durability:?} publication={}",
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
    let path = journal_path(root);
    let journal = decode(&path)?
        .ok_or_else(|| err("no compatible journal to checkpoint"))?;
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err(
            "source changed before checkpoint; recover from source"
        ));
    }
    let db = Database::create(db_path(root))?;
    let (current, rows) = load(&db)?;
    if !current.is_compatible(&journal.base)
        || current.revision != journal.base.revision + 1
        || hash_rows(&rows) != journal.target_hash
    {
        return Err(err("checkpoint target does not match journal"));
    }
    let mut tx = db.begin_write()?;
    {
        let meta = tx.open_table(META)?;
        let observed: Identity = postcard::from_bytes(
            meta.get("current")?
                .ok_or_else(|| err("missing checkpoint identity"))?
                .value(),
        )?;
        if observed != current
            || hash_rows(&load(&db)?.1) != journal.target_hash
        {
            return Err(err("checkpoint writer target changed"));
        }
    }
    tx.set_durability(Durability::Immediate)?;
    stop("before_checkpoint_commit");
    tx.commit()?;
    stop("after_checkpoint_commit");
    if !is_same_sources(&journal.snapshot, root)? {
        return Err(err("source changed during checkpoint; journal retained"));
    }
    fs::remove_file(path)?;
    println!("checkpoint outcome=durable revision={}", current.revision);
    Ok(())
}
fn fresh_query(root: &Path, reason: &str) -> Result<&'static str> {
    let fresh = Arc::new(IndexerService::for_tests(root).build()?);
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
fn query(root: &Path) -> Result<()> {
    match Database::create(db_path(root)) {
        Ok(db) => {
            let (identity, rows) = load(&db)?;
            println!(
                "query outcome=shadow-read pages={} revision={}",
                rows.tables["files"].len(),
                identity.revision
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
    let _db = Database::create(db_path(root))?;
    println!("hold ready");
    std::io::stdout().flush()?;
    let _ = std::io::stdin().read(&mut [0u8; 1])?;
    Ok(())
}
fn recovery(root: &Path, no_replay: bool) -> Result<&'static str> {
    let path = journal_path(root);
    if !db_path(root).exists() {
        match decode(&path) {
            Ok(None) if path.exists() => {
                fresh_query(root, "future-journal-preserved")?;
                return Ok("fresh-query");
            }
            Ok(Some(_)) | Err(_) => {
                if path.exists() {
                    fs::remove_file(&path)?;
                }
            }
            Ok(None) => (),
        }
        let (snap, facts) = parsed(root)?;
        let all = facts
            .into_iter()
            .map(|fact| (fact.path.to_owned(), fact))
            .collect();
        let db = Database::create(db_path(root))?;
        persist(
            &db,
            None,
            None,
            &Identity::first(),
            &derived(&all)?,
            Durability::Immediate,
        )?;
        println!(
            "recover outcome=missing-store-rebuilt parse_count={}",
            snap.files.iter().filter(|f| f.path.ends_with(".md")).count()
        );
        return Ok("missing-store-rebuilt");
    }
    let db = Database::create(db_path(root))?;
    let (base, previous) = load(&db)?;
    let candidate = match decode(&path) {
        Ok(Some(j)) => Some(j),
        Ok(None) if path.exists() => {
            drop(db);
            fresh_query(root, "future-journal-preserved")?;
            return Ok("fresh-query");
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("optional journal discarded: {e}");
            let _ = fs::remove_file(&path);
            None
        }
    };
    if let Some(j) = candidate {
        if no_replay {
            println!(
                "recover outcome=no-replay-control revision={}",
                base.revision
            );
            return Ok("no-replay-control");
        }
        let source_ok = is_same_sources(&j.snapshot, root)?;
        let base_ok = base == j.base
            && hash_rows(&previous) == j.base_hash
            && base.is_compatible(&j.base);
        if source_ok && base_ok {
            stop("before_replay");
            let started = Instant::now();
            let mut all = facts_from(&previous)?;
            if j.paths.len() != j.changes.len() {
                return Err(err("journal path/changes mismatch"));
            }
            for (path, change) in j.paths.iter().zip(&j.changes) {
                match change {
                    Some(fact) if fact.path == *path => {
                        all.insert(path.to_owned(), Fact {
                            path: fact.path.to_owned(),
                            file: fact.file.to_vec(),
                            note: fact.note.clone(),
                        });
                    }
                    None => {
                        all.remove(path);
                    }
                    _ => return Err(err("journal fact path mismatch")),
                }
            }
            if !is_same_sources(&j.snapshot, root)? {
                return Err(err("source changed while replaying"));
            }
            let rows = derived(&all)?;
            if hash_rows(&rows) != j.target_hash {
                return Err(err("journal target differs from derived facts"));
            }
            let mut next = base.clone();
            next.revision += 1;
            persist(
                &db,
                Some(&base),
                Some(&j.base_hash),
                &next,
                &rows,
                Durability::Immediate,
            )?;
            stop("after_replay_commit");
            fs::remove_file(path)?;
            println!(
                "recover outcome=replayed parse_count=0 \
                 decode_and_replay_ms={:.3} revision={}",
                started.elapsed().as_secs_f64() * 1000.0,
                next.revision
            );
            return Ok("replayed");
        }
        // A stale or invalid candidate cannot contribute partial facts.
        fs::remove_file(path)?;
    }
    let (snap, facts) = parsed(root)?;
    let all: BTreeMap<_, _> =
        facts.into_iter().map(|fact| (fact.path.to_owned(), fact)).collect();
    let rows = derived(&all)?;
    if hash_rows(&rows) == hash_rows(&previous) {
        println!(
            "recover outcome=unchanged parse_count={} revision={}",
            snap.files.iter().filter(|f| f.path.ends_with(".md")).count(),
            base.revision
        );
        return Ok("unchanged");
    }
    let mut next = base.clone();
    next.revision += 1;
    persist(
        &db,
        Some(&base),
        Some(&hash_rows(&previous)),
        &next,
        &rows,
        Durability::Immediate,
    )?;
    println!(
        "recover outcome=parsed parse_count={} revision={}",
        snap.files.iter().filter(|f| f.path.ends_with(".md")).count(),
        next.revision
    );
    Ok("parsed")
}
fn verify(root: &Path) -> Result<()> {
    let db = Database::create(db_path(root))?;
    let (identity, stored) = load(&db)?;
    let (_, facts) = parsed(root)?;
    let all: BTreeMap<_, _> =
        facts.into_iter().map(|f| (f.path.to_owned(), f)).collect();
    let trusted = derived(&all)?;
    for name in TABLES {
        if stored.tables.get(name) != trusted.tables.get(name) {
            return Err(format!(
                "facet {name} differs from fresh parsed facts"
            )
            .into());
        }
    }
    let fresh = Arc::new(IndexerService::for_tests(root).build()?);
    let query = QueryService::new("class");
    for (mode, build) in [
        ("pages", QueryBuilder::pages as fn(SourceSelector) -> QueryBuilder),
        ("lists", QueryBuilder::lists),
        ("tasks", QueryBuilder::tasks),
    ] {
        let expected = query.run(&fresh, build(SourceSelector::All));
        let expected_paths: Vec<_> = expected
            .iter()
            .map(|row| row.file().path().to_string_lossy().into_owned())
            .collect();
        let mut actual_paths = Vec::new();
        let mut actual_tasks = Vec::new();
        if mode == "pages" {
            actual_paths.extend(stored.tables["files"].keys().cloned());
        } else {
            let facet = if mode == "lists" {
                "lists"
            } else {
                "tasks"
            };
            for (path, bytes) in &stored.tables[facet] {
                let items: Vec<ListItem> = postcard::from_bytes(bytes)?;
                for item in items {
                    actual_paths.push(path.to_owned());
                    if mode == "tasks" {
                        actual_tasks.push(item.clean_text().to_owned());
                    }
                }
            }
        }
        if expected_paths != actual_paths {
            return Err(format!("{mode} query row paths differ").into());
        }
        if mode == "tasks" {
            let expected_tasks: Vec<_> = expected
                .iter()
                .map(|r| r.task_text().unwrap_or_default().to_owned())
                .collect();
            if expected_tasks != actual_tasks {
                return Err(err("task query rows differ"));
            }
        }
    }
    println!(
        "verify result=equal facets={} query_modes=pages,lists,tasks \
         incarnation={} revision={} (projected rows, not complete query \
         semantics)",
        TABLES.len(),
        identity.incarnation,
        identity.revision
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("probe error: {error}");
        process::exit(2);
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let command = args.get(1).ok_or_else(|| {
        err("usage: replay-fault-probe \
             seed|stage|recover|checkpoint|query|hold|verify ROOT \
             [count|none|no-journal]")
    })?;
    let root = Path::new(args.get(2).ok_or_else(|| err("missing ROOT"))?);
    match command.as_str() {
        "seed" => initialize(
            root,
            args.get(3).map(String::as_str).unwrap_or("12").parse()?,
        ),
        "stage" => stage(
            root,
            if args.get(3).is_some_and(|s| s == "none") {
                Durability::None
            } else {
                Durability::Immediate
            },
            !args.get(3).is_some_and(|s| s == "no-journal"),
        ),
        "recover" => {
            recovery(root, args.get(3).is_some_and(|s| s == "no-replay"))?;
            Ok(())
        }
        "checkpoint" => checkpoint(root),
        "query" => query(root),
        "hold" => hold(root),
        "verify" => verify(root),
        _ => Err(err("unknown command")),
    }
}
