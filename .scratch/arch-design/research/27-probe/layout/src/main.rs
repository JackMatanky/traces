// Throwaway ticket-27 layout measurement, not a store adapter or recovery
// protocol. Run in release with no RUSTFLAGS; stdout is newline-delimited
// key=value records.
use std::{
    env, fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

use indexmap::IndexMap;
use redb::{
    Database, Durability, ReadableDatabase, ReadableTable, TableDefinition,
    WriteTransaction,
};
use serde::{Deserialize, Serialize};
use traces::{
    Heading, ListItem, Note, NoteFieldValue, Tag,
    testing::{
        IndexerService, QueryBuilder, QueryService, SourceSelector, parse_note,
        path_codec,
    },
};

const FILES: usize = 0;
const FIELDS: usize = 1;
const TAGS: usize = 2;
const CLASSES: usize = 3;
const HEADINGS: usize = 4;
const OUTLINKS: usize = 5;
const LINKS: usize = 6;
const TASKS: usize = 7;
const LISTS: usize = 8;
const NOTICES: usize = 9;
const META: usize = 10;
const NAMES: [&str; 11] = [
    "files", "fields", "tags", "classes", "headings", "outlinks", "links",
    "tasks", "lists", "notices", "meta",
];
const NOTE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("notes");
const TAG_KEYS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("tag_keys");
const CLASS_KEYS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("class_keys");
const RANGE_KEYS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("range_keys");

fn def(i: usize) -> TableDefinition<'static, &'static [u8], &'static [u8]> {
    TableDefinition::new(NAMES[i])
}
fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

// Mirror of Note's serialized *parsed* fields. The private FieldKey serializes
// as its original name; the private Link serializes as these four fields.
// The postcard byte-equality check below aborts if this mirror drifts.
#[derive(Serialize, Deserialize)]
struct Parsed {
    #[serde(with = "path_codec")]
    path: PathBuf,
    frontmatter: Option<IndexMap<String, NoteFieldValue>>,
    lists: Vec<ListItem>,
    headings: Vec<Heading>,
    outlinks: Vec<ParsedLink>,
    inline_fields: IndexMap<String, Vec<NoteFieldValue>>,
    tags: Vec<Tag>,
}
#[derive(Serialize, Deserialize)]
struct ParsedLink {
    target: String,
    text: Option<String>,
    kind: LinkKind,
    embedded: bool,
}
#[derive(Serialize, Deserialize)]
enum LinkKind {
    Markdown,
    Wikilink,
}
#[derive(Serialize)]
struct FieldFacet<'a> {
    frontmatter: &'a Option<IndexMap<String, NoteFieldValue>>,
    inline_fields: &'a IndexMap<String, Vec<NoteFieldValue>>,
}
#[derive(Deserialize)]
struct OwnedFields {
    frontmatter: Option<IndexMap<String, NoteFieldValue>>,
    inline_fields: IndexMap<String, Vec<NoteFieldValue>>,
}
#[derive(Deserialize)]
struct TaskCopy {
    item_idx: u32,
    item: ListItem,
}
#[derive(Serialize)]
struct TaskRef<'a> {
    item_idx: u32,
    item: &'a ListItem,
}

struct Encoded {
    path: Vec<u8>,
    rows: [Vec<u8>; 11],
    tags: Vec<String>,
    classes: Vec<String>,
    inbound: Vec<String>,
    tasks: Vec<(u32, Vec<u8>)>,
    rating: Option<f64>,
}

fn note_source(i: usize, n: usize) -> String {
    let classes = ["Project", "Area", "Resource", "Archive"];
    let mut body = format!(
        "---\nrating: {}\nclass: {}\n---\n# Note {i} #common #topic/{} \
         #rare_{}\n## Section {}\nstate:: active\n",
        i % 10,
        classes[i % 4],
        i % 5,
        i % 50,
        i % 3,
    );
    for j in 0..10 {
        if j == 4 && i % 4 != 0 {
            body.push_str(&format!(
                "- [ ] Task {j} for note {i} [due:: 2026-01-05] \
                 #task/topic_4\n"
            ));
        } else {
            body.push_str(&format!("- item {j} text {i}\n"));
        }
    }
    body.push_str(&format!("Related: [[note-{}]].\n", (i + 1) % n));
    body
}

fn fixture(dir: &Path, n: usize) -> u64 {
    fs::create_dir(dir).expect("new fixture directory required");
    let mut bytes = 0;
    for i in 0..n {
        let body = note_source(i, n);
        bytes += body.len() as u64;
        fs::write(dir.join(format!("note-{i}.md")), body)
            .expect("write fixture note");
    }
    bytes
}

fn keys(path: &[u8], prefix: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(prefix.len() + path.len() + 1);
    key.extend_from_slice(prefix.as_bytes());
    key.push(0);
    key.extend_from_slice(path);
    key
}
fn task_key(path: &[u8], idx: u32) -> Vec<u8> {
    let mut key = keys(path, "task");
    key.extend_from_slice(&idx.to_be_bytes());
    key
}
fn effective<'a>(p: &'a Parsed, name: &str) -> Option<&'a NoteFieldValue> {
    p.frontmatter
        .as_ref()
        .and_then(|f| f.get(name))
        .or_else(|| p.inline_fields.get(name).and_then(|values| values.first()))
}
fn scalar_rating(p: &Parsed) -> Option<f64> {
    match effective(p, "rating") {
        Some(NoteFieldValue::Number(v)) if v.is_finite() => Some(*v),
        _ => None,
    }
}
fn range_key(path: &[u8], rating: f64) -> Vec<u8> {
    let bits = rating.to_bits();
    let ordered = if bits >> 63 != 0 {
        !bits
    } else {
        bits ^ (1 << 63)
    };
    keys(path, &format!("rating/{ordered:016x}"))
}
fn encode(
    note: &Note,
    size: u64,
    costs: &mut [f64; 11],
    bridge_ms: &mut f64,
) -> Encoded {
    let t = Instant::now();
    let blob = postcard::to_allocvec(note).expect("serialize parsed note");
    let p: Parsed =
        postcard::from_bytes(&blob).expect("decode parser facts bridge");
    assert!(p.path == note.path());
    assert!(
        postcard::to_allocvec(&p).expect("mirror re-encode") == blob,
        "shadow parser-fact mirror differs from real parser"
    );
    *bridge_ms += elapsed(t);
    let path = note.path().as_os_str().as_encoded_bytes().to_vec();
    let mut rows: [Vec<u8>; 11] = std::array::from_fn(|_| Vec::new());
    macro_rules! row {
        ($id:expr, $value:expr) => {{
            let t = Instant::now();
            rows[$id] = postcard::to_allocvec($value).expect("encode facet");
            costs[$id] += elapsed(t);
        }};
    }
    // A file facet with the real path and source byte count. The production
    // FileMeta timestamps and filename-derived day are not represented.
    row!(FILES, &(&p.path, size));
    row!(FIELDS, &FieldFacet {
        frontmatter: &p.frontmatter,
        inline_fields: &p.inline_fields
    });
    row!(TAGS, &p.tags);
    let classes: Vec<String> = match effective(&p, "class") {
        Some(NoteFieldValue::String(class)) => vec![class.clone()],
        _ => Vec::new(),
    };
    row!(CLASSES, &classes);
    row!(HEADINGS, &p.headings);
    row!(OUTLINKS, &p.outlinks);
    let t = Instant::now();
    let inbound: Vec<String> = p
        .outlinks
        .iter()
        .map(|link| format!("{}.md", link.target.trim_end_matches(".md")))
        .collect();
    costs[LINKS] += elapsed(t);
    let mut tasks = Vec::new();
    let t = Instant::now();
    for (item_idx, item) in p.lists.iter().enumerate() {
        if item.kind().is_task() {
            tasks.push((
                item_idx as u32,
                postcard::to_allocvec(&TaskRef {
                    item_idx: item_idx as u32,
                    item,
                })
                .expect("encode task copy"),
            ));
        }
    }
    costs[TASKS] += elapsed(t);
    row!(LISTS, &p.lists);
    // The valid fixture emits no parse-time notices.
    // A single meta record is written when the shadow schema is created.
    let tags = p.tags.iter().map(|tag| tag.as_str().to_owned()).collect();
    let rating = scalar_rating(&p);
    Encoded {
        path,
        rows,
        tags,
        classes,
        inbound,
        tasks,
        rating,
    }
}

fn create_shadow(path: &Path) -> Database {
    let db = Database::create(path).expect("create shadow store");
    let mut tx = db.begin_write().expect("schema transaction");
    for i in 0..11 {
        tx.open_table(def(i)).expect("create facet table");
    }
    tx.open_table(def(META))
        .expect("meta")
        .insert(&b"format-epoch"[..], &1_u64.to_le_bytes()[..])
        .expect("meta epoch");
    for d in [TAG_KEYS, CLASS_KEYS, RANGE_KEYS] {
        tx.open_table(d).expect("create derived table");
    }
    tx.set_durability(Durability::Immediate).expect("schema durability");
    tx.commit().expect("schema commit");
    db
}
fn insert(tx: &WriteTransaction, e: &Encoded) {
    for i in 0..11 {
        if i == TASKS || i == META || i == LINKS || i == NOTICES {
            continue;
        }
        tx.open_table(def(i))
            .expect("open facet")
            .insert(e.path.as_slice(), e.rows[i].as_slice())
            .expect("write facet");
    }
    for (idx, bytes) in &e.tasks {
        let key = task_key(&e.path, *idx);
        tx.open_table(def(TASKS))
            .expect("tasks table")
            .insert(key.as_slice(), bytes.as_slice())
            .expect("write task");
    }
    for tag in &e.tags {
        let key = keys(&e.path, &format!("tag/{tag}"));
        tx.open_table(TAG_KEYS)
            .expect("tag keys")
            .insert(key.as_slice(), &b""[..])
            .expect("tag key");
    }
    for class in &e.classes {
        let key = keys(&e.path, &format!("class/{class}"));
        tx.open_table(CLASS_KEYS)
            .expect("class keys")
            .insert(key.as_slice(), &b""[..])
            .expect("class key");
    }
    if let Some(rating) = e.rating {
        let key = range_key(&e.path, rating);
        tx.open_table(RANGE_KEYS)
            .expect("range keys")
            .insert(key.as_slice(), &b""[..])
            .expect("range key");
    }
    for target in &e.inbound {
        let key = keys(
            target.as_bytes(),
            &format!("in/{}", String::from_utf8_lossy(&e.path)),
        );
        tx.open_table(def(LINKS))
            .expect("links table")
            .insert(key.as_slice(), &b""[..])
            .expect("inbound edge");
    }
}

// Replacement removes old derived keys before adding the new ones. It uses
// fixture-owned old facts; reading these from the store is not measured.
fn replace(tx: &WriteTransaction, old: &Encoded, fresh: &Encoded) {
    for tag in &old.tags {
        let key = keys(&old.path, &format!("tag/{tag}"));
        tx.open_table(TAG_KEYS)
            .expect("tag keys")
            .remove(key.as_slice())
            .expect("remove tag");
    }
    for class in &old.classes {
        let key = keys(&old.path, &format!("class/{class}"));
        tx.open_table(CLASS_KEYS)
            .expect("class keys")
            .remove(key.as_slice())
            .expect("remove class");
    }
    if let Some(rating) = old.rating {
        let key = range_key(&old.path, rating);
        tx.open_table(RANGE_KEYS)
            .expect("range keys")
            .remove(key.as_slice())
            .expect("remove range");
    }
    for (idx, _) in &old.tasks {
        let key = task_key(&old.path, *idx);
        tx.open_table(def(TASKS))
            .expect("tasks table")
            .remove(key.as_slice())
            .expect("remove task");
    }
    for target in &old.inbound {
        let key = keys(
            target.as_bytes(),
            &format!("in/{}", String::from_utf8_lossy(&old.path)),
        );
        tx.open_table(def(LINKS))
            .expect("links table")
            .remove(key.as_slice())
            .expect("remove edge");
    }
    insert(tx, fresh);
}

fn scan_shadow(db: &Database, n: usize) -> (usize, usize, f64, f64) {
    let tx = db.begin_read().expect("shadow read");
    let start = Instant::now();
    let tasks = tx.open_table(def(TASKS)).expect("tasks facet");
    let mut task_rows = 0;
    for row in tasks.iter().expect("task scan") {
        let (_, value) = row.expect("task row");
        let task: TaskCopy =
            postcard::from_bytes(value.value()).expect("decode task");
        black_box((task.item_idx, task.item));
        task_rows += 1;
    }
    drop(tasks);
    let task_ms = elapsed(start);
    let start = Instant::now();
    let mut full_rows = 0;
    for i in 0..n {
        let path = format!("note-{i}.md");
        let b = path.as_bytes();
        let fields = tx.open_table(def(FIELDS)).expect("fields");
        let decoded: OwnedFields = postcard::from_bytes(
            fields.get(b).expect("fields get").expect("fields row").value(),
        )
        .expect("decode fields");
        let lists = tx.open_table(def(LISTS)).expect("lists");
        let items: Vec<ListItem> = postcard::from_bytes(
            lists.get(b).expect("lists get").expect("lists row").value(),
        )
        .expect("decode lists");
        let headings = tx.open_table(def(HEADINGS)).expect("headings");
        let h: Vec<Heading> = postcard::from_bytes(
            headings.get(b).expect("heading get").expect("heading row").value(),
        )
        .expect("decode headings");
        let tags = tx.open_table(def(TAGS)).expect("tags");
        let t: Vec<Tag> = postcard::from_bytes(
            tags.get(b).expect("tags get").expect("tags row").value(),
        )
        .expect("decode tags");
        let links = tx.open_table(def(OUTLINKS)).expect("outlinks");
        let l: Vec<ParsedLink> = postcard::from_bytes(
            links.get(b).expect("links get").expect("links row").value(),
        )
        .expect("decode links");
        black_box((decoded.frontmatter, decoded.inline_fields, items, h, t, l));
        full_rows += 1;
    }
    (task_rows, full_rows, task_ms, elapsed(start))
}
fn scan_control(db: &Database) -> (usize, usize) {
    let tx = db.begin_read().expect("control read");
    let notes = tx.open_table(NOTE_TABLE).expect("control notes");
    let mut tasks = 0;
    let mut rows = 0;
    for row in notes.iter().expect("control scan") {
        let (_, bytes) = row.expect("control row");
        let note: Note =
            postcard::from_bytes(bytes.value()).expect("decode control note");
        tasks += note.tasks().count();
        black_box(note);
        rows += 1;
    }
    (tasks, rows)
}

fn rss_peak_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage writes the initialized rusage value on a successful
    // call.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return 0;
    }
    // SAFETY: the successful call initialized every rusage field.
    let rss = unsafe { usage.assume_init() }.ru_maxrss as u64;
    if cfg!(target_os = "linux") {
        rss * 1024
    } else {
        rss
    }
}
fn metric(phase: &str, millis: f64, bytes: u64, count: usize) {
    println!(
        "phase={phase} ms={millis:.3} bytes={bytes} count={count} \
         rss_peak_bytes={}",
        rss_peak_bytes()
    );
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: t27-layout-probe <note-count-positive> \
             <new-output-directory>\noutput: phase=<name> ms=<elapsed> \
             bytes=<facet-or-store-bytes> count=<rows> \
             rss_peak_bytes=<process-high-water>; all times are single \
             release-profile observations"
        );
        std::process::exit(2);
    }
    let n: usize = args[1].parse().expect("note count");
    assert!(n > 0, "note count must be positive");
    eprintln!(
        "limits: generated ring-link notes only; file facet omits FileMeta \
         timestamps/day; links resolve this fixture's simple basenames only; \
         numeric rating is the only range key; no parse notices; meta has \
         only epoch; shadow full-facet decode is not QueryRow assembly; no \
         journal, fault injection, cold-cache control, or power-loss claim"
    );
    let root = PathBuf::from(&args[2]);
    let t = Instant::now();
    let fixture_bytes = fixture(&root, n);
    metric("fixture_generate", elapsed(t), fixture_bytes, n);

    // Real current/no-journal control. The baseline indexer owns its own scan,
    // parse, link resolution and existing eight-table redb persistence.
    let indexer = IndexerService::for_tests(&root);
    let t = Instant::now();
    let index = indexer.build().expect("build current index");
    metric("control_build", elapsed(t), fixture_bytes, n);
    let t = Instant::now();
    indexer.persist(&index).expect("persist current index");
    metric(
        "control_persist_none",
        elapsed(t),
        fs::metadata(root.join(".traces/index.redb"))
            .expect("control size")
            .len(),
        n,
    );
    let service = QueryService::new("class");
    let start = Instant::now();
    let pages = service
        .sync_and_run(&indexer, QueryBuilder::pages(SourceSelector::All))
        .expect("control page query");
    metric("control_refresh_and_query_pages", elapsed(start), 0, pages.len());
    assert!(pages.len() == n, "control page count mismatch");
    drop(pages);
    let start = Instant::now();
    let tasks = service
        .sync_and_run(&indexer, QueryBuilder::tasks(SourceSelector::All))
        .expect("control task query");
    metric("control_refresh_and_query_tasks", elapsed(start), 0, tasks.len());
    assert!(tasks.len() == n - (n + 3) / 4, "control task count mismatch");
    drop(tasks);
    drop(index);

    // Parsed facts are produced by the actual public parser, not from redb
    // rows.
    let mut encoded = Vec::with_capacity(n);
    let mut enc_costs = [0_f64; 11];
    let mut bridge_ms = 0_f64;
    let mut parse_ms = 0_f64;
    for i in 0..n {
        let path = format!("note-{i}.md");
        let body =
            fs::read_to_string(root.join(&path)).expect("read generated note");
        let t = Instant::now();
        let note = parse_note(Path::new(&path), &body);
        parse_ms += elapsed(t);
        encoded.push(encode(
            &note,
            body.len() as u64,
            &mut enc_costs,
            &mut bridge_ms,
        ));
    }
    metric("shadow_parse", parse_ms, fixture_bytes, n);
    metric("parser_fact_bridge", bridge_ms, 0, n);
    for i in 0..11 {
        let bytes = if i == META {
            8
        } else if i == TASKS {
            encoded
                .iter()
                .flat_map(|e| &e.tasks)
                .map(|(_, b)| b.len() as u64)
                .sum()
        } else if i == LINKS {
            encoded
                .iter()
                .flat_map(|e| {
                    e.inbound.iter().map(move |target| {
                        target.len() as u64 + e.path.len() as u64 + 4
                    })
                })
                .sum()
        } else if i == NOTICES {
            0
        } else {
            encoded.iter().map(|e| e.rows[i].len() as u64).sum()
        };
        let count = if i == TASKS {
            encoded.iter().map(|e| e.tasks.len()).sum()
        } else if i == LINKS {
            encoded.iter().map(|e| e.inbound.len()).sum()
        } else if i == NOTICES {
            0
        } else if i == META {
            1
        } else {
            n
        };
        metric(
            &format!("facet_encode_{}", NAMES[i]),
            enc_costs[i],
            bytes,
            count,
        );
    }
    let task_count: usize = encoded.iter().map(|e| e.tasks.len()).sum();
    assert!(task_count == n - (n + 3) / 4, "fixture task ratio drifted");

    let shadow_path = root.join("shadow.redb");
    let db = create_shadow(&shadow_path);
    let t = Instant::now();
    {
        let mut tx = db.begin_write().expect("shadow write");
        for e in &encoded {
            insert(&tx, e);
        }
        tx.set_durability(Durability::Immediate).expect("shadow durability");
        tx.commit().expect("shadow commit");
    }
    metric(
        "shadow_persist_immediate",
        elapsed(t),
        fs::metadata(&shadow_path).expect("shadow size").len(),
        n,
    );
    let (shadow_tasks, shadow_rows, task_ms, full_ms) = scan_shadow(&db, n);
    metric("shadow_task_only_decode", task_ms, 0, shadow_tasks);
    metric("shadow_full_facets_decode", full_ms, 0, shadow_rows);
    assert!(
        shadow_tasks == task_count && shadow_rows == n,
        "shadow count mismatch"
    );
    drop(db);
    let db = Database::open(&shadow_path).expect("shadow reopen");
    let t = Instant::now();
    let (_, _, _, _) = scan_shadow(&db, n);
    metric("shadow_reopen_and_scan", elapsed(t), 0, n);
    drop(db);

    let control_path = root.join(".traces/index.redb");
    let t = Instant::now();
    let control = Database::open(&control_path).expect("control reopen");
    let (current_tasks, current_rows) = scan_control(&control);
    metric(
        "control_reopen_and_scan",
        elapsed(t),
        fs::metadata(&control_path).expect("control size").len(),
        current_rows,
    );
    assert!(
        current_tasks == task_count && current_rows == n,
        "parser/store task count mismatch"
    );
    drop(control);

    let old = &encoded[0];
    let mut edited_body = note_source(0, n);
    edited_body.push_str(
        "\n#replacement #changed\n- [x] completed [due:: 2026-01-06]\n",
    );
    let changed_note = parse_note(Path::new("note-0.md"), &edited_body);
    let mut changed_cost = [0_f64; 11];
    let changed = encode(
        &changed_note,
        edited_body.len() as u64,
        &mut changed_cost,
        &mut 0_f64,
    );
    let db = Database::open(&shadow_path).expect("reopen for replace");
    let t = Instant::now();
    {
        let mut tx = db.begin_write().expect("replace transaction");
        replace(&tx, old, &changed);
        tx.set_durability(Durability::Immediate).expect("replace durability");
        tx.commit().expect("replace commit");
    }
    metric(
        "shadow_replace_one_immediate",
        elapsed(t),
        fs::metadata(&shadow_path).expect("shadow size").len(),
        1,
    );
    let (tasks_after, rows_after, _, _) = scan_shadow(&db, n);
    assert!(
        rows_after == n && tasks_after == task_count + 1,
        "task copy replacement failed"
    );
    drop(db);
}
