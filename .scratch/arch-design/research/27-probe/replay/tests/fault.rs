//! Process-stop and environment tests for the real-store replay probe. Use
//! --release for panic=abort and --test-threads=1 for serial execution.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

use tempfile::TempDir;

const CHILD_TIMEOUT: Duration = Duration::from_secs(30);

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_child(child: &mut Child) {
    let started = Instant::now();
    while child.try_wait().expect("check child status").is_none() {
        assert!(started.elapsed() < CHILD_TIMEOUT, "probe child timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn bounded_output(command: &mut Command) -> Output {
    let stdout = tempfile::NamedTempFile::new().expect("stdout file");
    let stderr = tempfile::NamedTempFile::new().expect("stderr file");
    let mut child = ChildGuard(
        command
            .stdout(Stdio::from(stdout.reopen().expect("open stdout")))
            .stderr(Stdio::from(stderr.reopen().expect("open stderr")))
            .spawn()
            .expect("spawn child"),
    );
    wait_for_child(&mut child.0);
    Output {
        status: child.0.wait().expect("reap child"),
        stdout: fs::read(stdout.path()).expect("read stdout"),
        stderr: fs::read(stderr.path()).expect("read stderr"),
    }
}

fn readiness(reader: impl std::io::Read + Send + 'static, expected: &str) {
    use std::io::BufRead;
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let mut reader = Some(std::io::BufReader::new(reader));
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = reader
            .as_mut()
            .expect("reader is present in the closure")
            .read_line(&mut line);
        let _ = send.send(result.map(|_| line));
        drop(reader);
    });
    let line = receive
        .recv_timeout(CHILD_TIMEOUT)
        .expect("child readiness timed out")
        .expect("read child readiness");
    pretty_assertions::assert_eq!(line, expected);
}

fn parked(root: &Path, command: &str, point: &str) -> ChildGuard {
    let mut child = ChildGuard(
        probe_command(root, command, None)
            .env("PROBE_PAUSE", point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parked child"),
    );
    readiness(
        child.0.stderr.take().expect("pause stderr"),
        &format!("probe_pause={point}\n"),
    );
    child
}

fn probe_command(root: &Path, command: &str, extra: Option<&str>) -> Command {
    let mut child = Command::new(env!("CARGO_BIN_EXE_replay-fault-probe"));
    child.arg(command).arg(root);
    if let Some(arg) = extra {
        child.arg(arg);
    }
    child
}
/// Returns the signal helper. Linux images without a `kill` binary (the
/// Debian rust image ships none) provide `T27_SIGNAL_HELPER` pointing at a
/// `kill(signal, pid)`-compatible executable.
fn env_signal_helper() -> String {
    if let Ok(helper) = std::env::var("T27_SIGNAL_HELPER") {
        return helper;
    }
    "kill".to_string()
}
fn call(root: &Path, command: &str, extra: Option<&str>) -> Output {
    bounded_output(&mut probe_command(root, command, extra))
}
fn call_envs(
    root: &Path,
    command: &str,
    extra: Option<&str>,
    envs: &[(&str, &str)],
) -> Output {
    let mut child = probe_command(root, command, extra);
    for (key, value) in envs {
        child.env(key, value);
    }
    bounded_output(&mut child)
}
fn injected(
    root: &Path,
    command: &str,
    extra: Option<&str>,
    fault: &str,
) -> Output {
    bounded_output(probe_command(root, command, extra).env("PROBE_FAIL", fault))
}
fn success(root: &Path, command: &str, extra: Option<&str>) -> String {
    let result = call(root, command, extra);
    assert!(
        result.status.success(),
        "{command} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).expect("child stdout")
}
fn seeded() -> TempDir {
    let dir = tempfile::tempdir().expect("temporary vault");
    success(dir.path(), "seed", Some("12"));
    dir
}
fn aborted_stage(dir: &Path) {
    let output = bounded_output(
        probe_command(dir, "stage", None)
            .env("PROBE_STOP", "after_publication"),
    );
    assert!(!output.status.success());
    assert!(dir.join(".traces/index.redb.journal").exists());
}

fn stopped(root: &Path, command: &str, point: &str) {
    let result = call_envs(root, command, None, &[("PROBE_STOP", point)]);
    assert!(!result.status.success(), "{command} did not stop at {point}");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains(&format!("probe_stop={point}")), "{stderr}");
}

mod publication {
    use super::*;
    #[test]
    fn replays_parsed_facts_after_published_writer_abort() {
        let dir = seeded();
        aborted_stage(dir.path());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
        let repeat = success(dir.path(), "recover", None);
        assert!(result.contains("store_reopen_ms="), "{result}");
        assert!(repeat.contains("outcome=unchanged"), "{repeat}");
    }
    #[test]
    fn reparses_sources_when_writer_stops_before_journal_publication() {
        let dir = seeded();
        let child = bounded_output(
            probe_command(dir.path(), "stage", None)
                .env("PROBE_STOP", "partial_write"),
        );
        assert!(!child.status.success());
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_after_abort_before_store_commit() {
        let dir = seeded();
        let child = bounded_output(
            probe_command(dir.path(), "stage", None)
                .env("PROBE_STOP", "before_commit"),
        );
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_no_replay_control_at_complete_query_check() {
        let dir = seeded();
        aborted_stage(dir.path());
        let result = success(dir.path(), "recover", Some("no-replay"));
        assert!(result.contains("no-replay-control"));
        let check = call(dir.path(), "verify", None);
        assert!(!check.status.success(), "no-replay control falsely passed");
        success(dir.path(), "recover", None);
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_stale_journal_after_durable_stage_commit_abort() {
        let dir = seeded();
        let child = bounded_output(
            probe_command(dir.path(), "stage", None)
                .env("PROBE_STOP", "before_unlink"),
        );
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=unchanged"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_after_aborted_replay_before_removal_only_once() {
        let dir = seeded();
        aborted_stage(dir.path());
        let child = bounded_output(
            probe_command(dir.path(), "recover", None)
                .env("PROBE_STOP", "after_replay_commit"),
        );
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=unchanged"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn falls_back_to_immediate_store_write_when_optional_publication_fails() {
        for point in ["journal_write", "journal_file_sync", "journal_dir_sync"]
        {
            let dir = seeded();
            let output = injected(dir.path(), "stage", None, point);
            assert!(
                output.status.success(),
                "{point}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout).expect("stdout");
            assert!(
                stdout.contains("publication=fallback"),
                "{point}: {stdout}"
            );
            assert!(
                stdout.contains("durability=Immediate"),
                "{point}: {stdout}"
            );
            assert!(
                !dir.path().join(".traces/index.redb.journal").exists(),
                "{point}: orphan final journal"
            );
            assert!(
                !dir.path().join(".traces/index.redb.journal.tmp").exists(),
                "{point}: orphan temp journal"
            );
            success(dir.path(), "verify", None);
        }
    }
    #[test]
    fn retains_deferred_journal_until_checked_immediate_checkpoint() {
        let dir = seeded();
        let staged = success(dir.path(), "stage", Some("deferred"));
        assert!(staged.contains("durability=None"), "{staged}");
        let journal = dir.path().join(".traces/index.redb.journal");
        assert!(journal.exists());
        let checkpoint = success(dir.path(), "checkpoint", None);
        assert!(
            checkpoint.contains("outcome=durable revision=1"),
            "{checkpoint}"
        );
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
        let next = success(dir.path(), "recover", None);
        assert!(next.contains("outcome=unchanged"), "{next}");
    }
    #[test]
    fn keeps_journal_after_checkpoint_commit_abort_and_discards_stale_copy() {
        let dir = seeded();
        success(dir.path(), "stage", Some("deferred"));
        let child = bounded_output(
            probe_command(dir.path(), "checkpoint", None)
                .env("PROBE_STOP", "after_checkpoint_commit"),
        );
        assert!(!child.status.success());
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let next = success(dir.path(), "recover", None);
        assert!(next.contains("outcome=unchanged"), "{next}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn reparses_after_abort_before_any_journal_bytes() {
        let dir = seeded();
        stopped(dir.path(), "stage", "before_write");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn reparses_after_abort_before_journal_file_sync() {
        let dir = seeded();
        stopped(dir.path(), "stage", "before_file_sync");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn reparses_after_abort_after_journal_file_sync() {
        let dir = seeded();
        stopped(dir.path(), "stage", "after_file_sync");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn replays_after_abort_before_journal_directory_sync() {
        let dir = seeded();
        stopped(dir.path(), "stage", "before_dir_sync");
        let journal = dir.path().join(".traces/index.redb.journal");
        assert!(!fs::read(&journal).expect("published journal").is_empty());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
    }

    #[test]
    fn keeps_store_commit_after_abort_before_identity_update() {
        let dir = seeded();
        stopped(dir.path(), "stage", "after_checkpoint");
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=unchanged"), "{result}");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        success(dir.path(), "verify", None);
    }

    #[test]
    fn serves_committed_query_after_abort_after_journal_removal() {
        let dir = seeded();
        stopped(dir.path(), "stage", "after_unlink");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=unchanged"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn replays_after_abort_before_checkpoint_commit() {
        let dir = seeded();
        success(dir.path(), "stage", Some("deferred"));
        stopped(dir.path(), "checkpoint", "before_checkpoint_commit");
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn replays_after_abort_before_replay_commit() {
        let dir = seeded();
        aborted_stage(dir.path());
        stopped(dir.path(), "recover", "before_commit");
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn advances_revision_once_when_replay_is_repeated() {
        let dir = seeded();
        aborted_stage(dir.path());
        let first = success(dir.path(), "recover", None);
        assert!(first.contains("outcome=replayed"), "{first}");
        assert!(
            first.split_whitespace().any(|field| field == "revision=1"),
            "{first}"
        );
        let verified = success(dir.path(), "verify", None);
        assert!(verified.contains("incarnation="), "{verified}");
        assert!(
            verified.split_whitespace().any(|field| field == "revision=1"),
            "{verified}"
        );
        let repeat = success(dir.path(), "recover", None);
        assert!(repeat.contains("outcome=unchanged"), "{repeat}");
        assert!(
            repeat.split_whitespace().any(|field| field == "revision=1"),
            "{repeat}"
        );
        success(dir.path(), "verify", None);
    }
}

mod validation {
    use pretty_assertions::assert_eq;

    use super::*;
    #[test]
    fn discards_journal_when_unlisted_source_changes() {
        let dir = seeded();
        aborted_stage(dir.path());
        fs::write(dir.path().join("hub.md"), "# New Hub #changed\n")
            .expect("mutate unlisted hub");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_journal_when_changed_source_changes_again() {
        let dir = seeded();
        aborted_stage(dir.path());
        fs::write(dir.path().join("note-0.md"), "# changed twice #fresh\n")
            .expect("mutate listed note");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_corrupt_journal_and_checks_fresh_rows() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        fs::write(&path, b"bad").expect("corrupt journal");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_when_saved_precommit_store_image_is_restored() {
        let dir = seeded();
        let store = dir.path().join(".traces/index.redb");
        let pristine = tempfile::NamedTempFile::new().expect("saved image");
        fs::copy(&store, pristine.path()).expect("save store image");
        let stopped = call_envs(dir.path(), "stage", None, &[(
            "PROBE_STOP",
            "after_commit",
        )]);
        assert!(
            !stopped.status.success(),
            "writer did not stop before the barrier"
        );
        assert!(
            String::from_utf8_lossy(&stopped.stderr)
                .contains("probe_stop=after_commit")
        );
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        // The abort preceded the durability barrier, so the None commit
        // is not guaranteed on disk. The restored image makes that loss
        // exact.
        fs::copy(pristine.path(), &store).expect("restore old base image");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
    pub(super) fn fork_note_rows(dir: &Path) {
        use redb::{Database, Durability, TableDefinition};
        let db = Database::create(dir.join(".traces/index.redb"))
            .expect("open fork");
        let mut tx = db.begin_write().expect("write fork");
        {
            let mut notes = tx
                .open_table(TableDefinition::<&[u8], &[u8]>::new("notes"))
                .expect("notes");
            notes
                .insert(&b"hub.md"[..], &b"forked rows"[..])
                .expect("fork row");
        }
        tx.set_durability(Durability::Immediate).expect("set durability");
        tx.commit().expect("commit fork");
        drop(db);
    }
    #[test]
    fn rejects_same_revision_fork_with_different_note_rows() {
        let dir = seeded();
        aborted_stage(dir.path());
        fork_note_rows(dir.path());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_journal_when_store_is_missing() {
        let dir = seeded();
        aborted_stage(dir.path());
        fs::remove_file(dir.path().join(".traces/index.redb"))
            .expect("remove store");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=missing-store-rebuilt"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn preserves_higher_epoch_journal_and_serves_fresh_query() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&path).expect("read journal");
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
        let checksum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&checksum);
        fs::write(&path, &bytes).expect("future epoch");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("reason=future-journal-preserved"), "{result}");
        assert!(
            result.contains("pages=13")
                && result.contains("revision=unpersisted"),
            "{result}"
        );
        assert_eq!(fs::read(&path).expect("journal retained"), bytes);
    }
    #[test]
    fn rejects_same_size_and_mtime_when_contents_change() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join("hub.md");
        let modified =
            fs::metadata(&path).expect("metadata").modified().expect("mtime");
        let mut body = fs::read(&path).expect("read hub");
        body[2] = b'b';
        fs::write(&path, &body).expect("same-size write");
        fs::File::open(&path)
            .expect("reopen")
            .set_modified(modified)
            .expect("restore mtime");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_added_deleted_and_renamed_sources() {
        for change in ["add", "delete", "rename"] {
            let dir = seeded();
            aborted_stage(dir.path());
            match change {
                "add" => {
                    fs::write(dir.path().join("new.md"), "# new [[hub]]\n")
                        .expect("add note")
                }
                "delete" => fs::remove_file(dir.path().join("note-1.md"))
                    .expect("delete note"),
                _ => fs::rename(
                    dir.path().join("note-1.md"),
                    dir.path().join("moved.md"),
                )
                .expect("rename note"),
            }
            let result = success(dir.path(), "recover", None);
            assert!(result.contains("outcome=parsed"), "{change}: {result}");
            success(dir.path(), "verify", None);
        }
    }
    #[test]
    fn rejects_oversized_journal_before_body_allocation() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open journal")
            .set_len(64 * 1024 * 1024 + 89)
            .expect("extend sparse journal");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_false_journal_body_length_without_allocating_claimed_size() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&path).expect("read journal");
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        let checksum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&checksum);
        fs::write(&path, bytes).expect("write false length");
        let output = call(dir.path(), "recover", None);
        assert!(
            output.status.success(),
            "release child aborted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("outcome=parsed")
        );
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_hostile_checksummed_body_without_large_allocation() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&path).expect("read journal");
        // Hostile body: postcard declares a ~1 TiB engine String, with
        // valid blake3 checksums and a matching declared body length.
        let body = vec![0x80, 0x80, 0x80, 0x80, 0x20, b'x'];
        bytes[16..24].copy_from_slice(&(body.len() as u64).to_le_bytes());
        let body_sum = *blake3::hash(&body).as_bytes();
        bytes.truncate(56);
        bytes.extend_from_slice(&body_sum);
        bytes.extend_from_slice(&body);
        let header_sum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&header_sum);
        fs::write(&path, &bytes).expect("write hostile body");
        let output = call(dir.path(), "recover", None);
        assert!(
            output.status.success(),
            "release child aborted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("outcome=parsed"), "{stdout}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn retains_unknown_header_version_and_serves_fresh_query() {
        let dir = seeded();
        aborted_stage(dir.path());
        let path = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&path).expect("read journal");
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        let checksum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&checksum);
        fs::write(&path, &bytes).expect("write future format");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("reason=future-journal-preserved"), "{result}");
        assert!(result.contains("pages=13"), "{result}");
        assert_eq!(fs::read(&path).expect("journal retained"), bytes);
    }
    #[test]
    fn rejects_fifo_journal_without_blocking_or_aborting() {
        let dir = seeded();
        let path = dir.path().join(".traces/index.redb.journal");
        let create = bounded_output(Command::new("mkfifo").arg(&path));
        assert!(create.status.success(), "mkfifo failed");
        let output = call(dir.path(), "recover", None);
        assert!(
            output.status.success(),
            "release child aborted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("outcome=unchanged")
        );
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_symlink_journal_without_following_target() {
        use std::os::unix::fs::symlink;
        let dir = seeded();
        let target = dir.path().join("valuable.txt");
        fs::write(&target, b"valuable").expect("write target");
        symlink(&target, dir.path().join(".traces/index.redb.journal"))
            .expect("symlink journal");
        success(dir.path(), "recover", None);
        assert_eq!(fs::read(target).expect("target retained"), b"valuable");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn reparses_when_published_journal_loses_all_data() {
        let dir = seeded();
        success(dir.path(), "stage", Some("deferred"));
        let journal = dir.path().join(".traces/index.redb.journal");
        fs::OpenOptions::new()
            .write(true)
            .open(&journal)
            .expect("open journal image")
            .set_len(0)
            .expect("lose journal data");
        assert!(journal.exists(), "directory entry must survive");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
    }

    #[test]
    fn reparses_when_journal_header_or_body_is_truncated() {
        for length in [8, 88, 89] {
            let dir = seeded();
            aborted_stage(dir.path());
            let journal = dir.path().join(".traces/index.redb.journal");
            fs::OpenOptions::new()
                .write(true)
                .open(&journal)
                .expect("open journal")
                .set_len(length)
                .expect("truncate journal");
            let result = success(dir.path(), "recover", None);
            assert!(
                result.contains("outcome=parsed"),
                "length={length}: {result}"
            );
            assert!(!journal.exists());
            success(dir.path(), "verify", None);
        }
    }

    #[test]
    fn discards_lower_header_epoch_without_using_recorded_facts() {
        let dir = seeded();
        aborted_stage(dir.path());
        let journal = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&journal).expect("read journal");
        bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
        let checksum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&checksum);
        fs::write(&journal, bytes).expect("write lower epoch");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
    }

    #[test]
    fn ignores_traces_files_when_checking_complete_source_snapshot() {
        let dir = seeded();
        aborted_stage(dir.path());
        let nested = dir.path().join(".traces/nested");
        fs::create_dir_all(&nested).expect("create excluded directory");
        fs::write(nested.join("generated.md"), "# Excluded [[hub]] #ignored\n")
            .expect("write excluded note");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn rebuilds_after_abort_before_full_rebuild_commit() {
        let dir = seeded();
        aborted_stage(dir.path());
        fs::remove_file(dir.path().join(".traces/index.redb"))
            .expect("remove store");
        stopped(dir.path(), "recover", "before_rebuild_commit");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=missing-store-rebuilt"), "{result}");
        success(dir.path(), "verify", None);
    }
}

mod identity {
    use pretty_assertions::assert_eq;

    use super::*;
    #[test]
    fn rejects_forked_target_after_weak_base_check_admits_candidate() {
        // The weak base check admits the candidate. The target digest
        // check must reject the fork before any commit or generation
        // update.
        let dir = seeded();
        aborted_stage(dir.path());
        validation::fork_note_rows(dir.path());
        let journal = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&journal).expect("read fork journal");
        let identity = dir.path().join(".traces/probe-identity");
        let generation = fs::read(&identity).expect("read fork identity");
        let output =
            call_envs(dir.path(), "recover", None, &[("PROBE_WEAK_BASE", "1")]);
        assert!(!output.status.success(), "weak fork unexpectedly committed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("journal target differs from derived facts"),
            "{stderr}"
        );
        assert_eq!(fs::read(&journal).expect("retained fork journal"), before);
        assert_eq!(
            fs::read(&identity).expect("unchanged fork identity"),
            generation
        );
        // The rejected fork remains different from the sources until strong
        // recovery.
        let check = call(dir.path(), "verify", None);
        assert!(
            !check.status.success(),
            "rejected fork should still differ from the fresh parse"
        );
        // Strong mode repairs from source.
        let repaired = success(dir.path(), "recover", None);
        assert!(repaired.contains("outcome=parsed"), "{repaired}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_mutated_policy_hash() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("policy"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_mutated_extractor_fingerprint() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("extractor"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_mutated_incarnation() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("incarnation"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_mutated_revision() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("revision"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_mutated_engine() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("engine"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        success(dir.path(), "verify", None);
    }

    #[test]
    fn rejects_mutated_platform() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("platform"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_lower_epoch_journal_and_reparses() {
        // Store epoch is bumped above the journal's epoch: the journal is
        // stale, so it is discarded and the source reparsed.
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("epoch-up"));
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=rebuild-from-source"), "{result}");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        success(dir.path(), "verify", None);
    }
    #[test]
    fn preserves_journal_when_store_epoch_is_lower() {
        let dir = seeded();
        aborted_stage(dir.path());
        success(dir.path(), "mutate-identity", Some("epoch-down"));
        let path = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&path).expect("read journal");
        let result = success(dir.path(), "recover", None);
        assert!(
            result.contains("reason=higher-epoch-journal-preserved"),
            "{result}"
        );
        assert_eq!(fs::read(&path).expect("journal retained"), before);
    }
}

mod environment {
    use pretty_assertions::assert_eq;

    use super::*;
    fn chmod_vault(root: &Path, dir_mode: u32, file_mode: u32) {
        let traces = root.join(".traces");
        for entry in fs::read_dir(&traces).expect("list traces") {
            let entry = entry.expect("traces entry");
            let path = entry.path();
            let mut perms = fs::metadata(&path).expect("meta").permissions();
            perms.set_mode(file_mode);
            fs::set_permissions(&path, perms).expect("chmod file");
        }
        for dir in [root, &traces] {
            let mut perms = fs::metadata(dir).expect("meta").permissions();
            perms.set_mode(dir_mode);
            fs::set_permissions(dir, perms).expect("chmod dir");
        }
    }

    struct RestorePermissions<'a>(&'a Path);

    impl Drop for RestorePermissions<'_> {
        fn drop(&mut self) {
            chmod_vault(self.0, 0o755, 0o644);
        }
    }

    fn should_skip_permission_case() -> bool {
        // SAFETY: geteuid has no arguments or memory requirements.
        if unsafe { libc::geteuid() } == 0 {
            eprintln!("skipped: root can bypass Unix write permissions");
            return true;
        }
        false
    }
    #[test]
    fn read_only_vault_serves_fresh_query_without_writes() {
        if should_skip_permission_case() {
            return;
        }
        let dir = seeded();
        aborted_stage(dir.path());
        let traces = dir.path().join(".traces");
        let before = fs::read_dir(&traces)
            .expect("list traces")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>();
        chmod_vault(dir.path(), 0o555, 0o444);
        let restore = RestorePermissions(dir.path());
        let result = success(dir.path(), "recover", None);
        // Recovery cannot write, so it serves the fresh-query path and keeps
        // the journal for the next writable run.
        assert!(result.contains("outcome=fresh-query"), "{result}");
        let after = fs::read_dir(&traces)
            .expect("list traces read-only")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>();
        assert_eq!(after, before, "read-only recovery changed the vault");
        drop(restore);
        let replay = success(dir.path(), "recover", None);
        assert!(replay.contains("outcome=replayed parse_count=0"), "{replay}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn read_only_redb_handle_serves_query_without_opening_journal() {
        let dir = seeded();
        let journal = dir.path().join(".traces/index.redb.journal");
        assert!(!journal.exists());
        let result =
            call_envs(dir.path(), "query", None, &[("PROBE_DB_RO", "1")]);
        assert!(
            result.status.success(),
            "ro query failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("outcome=ro-read pages=13"), "{stdout}");
        assert!(!journal.exists(), "ro query created a journal");
    }

    #[test]
    fn serves_bounded_fresh_queries_when_store_stays_unwritable() {
        if should_skip_permission_case() {
            return;
        }
        let dir = seeded();
        aborted_stage(dir.path());
        let journal = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&journal).expect("read journal");
        let store = fs::read(dir.path().join(".traces/index.redb"))
            .expect("read store");
        chmod_vault(dir.path(), 0o555, 0o444);
        let restore = RestorePermissions(dir.path());
        for _ in 0..3 {
            let result = success(dir.path(), "recover", None);
            assert!(result.contains("outcome=fresh-query"), "{result}");
            assert!(result.contains("pages=13"), "{result}");
            assert_eq!(
                fs::read(&journal).expect("read retained journal"),
                before
            );
            assert_eq!(
                fs::read(dir.path().join(".traces/index.redb"))
                    .expect("read store"),
                store,
            );
            assert!(
                !dir.path().join(".traces/index.redb.journal.tmp").exists()
            );
        }
        drop(restore);
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn copied_root_discards_journal_and_reparses() {
        let dir = seeded();
        aborted_stage(dir.path());
        let copy = tempfile::tempdir().expect("copy root");
        let vault = copy.path().join("vault");
        let status = bounded_output(
            Command::new("cp").arg("-R").arg(dir.path()).arg(&vault),
        );
        assert!(status.status.success(), "cp -R failed");
        let result = success(&vault, "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(&vault, "verify", None);
    }
    #[test]
    fn moved_root_replays_from_content_digests() {
        let outer = tempfile::tempdir().expect("outer root");
        let dir = seeded();
        aborted_stage(dir.path());
        let moved = outer.path().join("moved-vault");
        let status =
            bounded_output(Command::new("mv").arg(dir.path()).arg(&moved));
        assert!(status.status.success(), "mv failed");
        let result = success(&moved, "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(&moved, "verify", None);
    }
    #[test]
    fn racing_edit_after_final_check_fails_safe_and_reparses() {
        let dir = seeded();
        aborted_stage(dir.path());
        let output =
            call_envs(dir.path(), "recover", None, &[("PROBE_RACE", "1")]);
        assert!(!output.status.success(), "racing run should abort mid-replay");
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        // Fail safe: the next run detects the raced source, drops the
        // journal, and reparses.
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        success(dir.path(), "verify", None);
    }
    #[test]
    fn recovers_after_precommit_replay_abort() {
        let dir = seeded();
        aborted_stage(dir.path());
        let child = bounded_output(
            probe_command(dir.path(), "recover", None)
                .env("PROBE_STOP", "mid_replay"),
        );
        assert!(!child.status.success());
        // The stop precedes the transaction. This does not test a partial
        // rebuild. Raw bytes can change when redb opens the store.
        let check = call(dir.path(), "verify", None);
        assert!(
            !check.status.success(),
            "aborted mid-replay store should not verify"
        );
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
}

mod signals {
    use pretty_assertions::assert_eq;

    use super::*;

    fn checked_recovery(root: &Path) -> String {
        let output =
            call_envs(root, "recover", None, &[("PROBE_CHECK_INTEGRITY", "1")]);
        assert!(output.status.success(), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("integrity_ms="), "{stderr}");
        String::from_utf8(output.stdout).expect("recovery stdout")
    }
    /// Parks the child mid-replay, delivers the signal, and returns once the
    /// child is reaped.
    fn kill_parked_child(dir: &Path, signal: &str) {
        use std::os::unix::process::ExitStatusExt;
        let mut child = parked(dir, "recover", "replay");
        let status = bounded_output(
            Command::new(env_signal_helper())
                .arg(signal)
                .arg(child.0.id().to_string()),
        );
        assert!(status.status.success(), "kill {signal} failed");
        wait_for_child(&mut child.0);
        let status = child.0.wait().expect("reap signaled child");
        assert!(!status.success());
        let expected = match signal {
            "-9" => libc::SIGKILL,
            "-2" => libc::SIGINT,
            _ => panic!("unsupported signal: {signal}"),
        };
        assert_eq!(status.signal(), Some(expected));
    }
    #[test]
    fn recovers_to_committed_state_after_sigkill_mid_replay() {
        let dir = seeded();
        aborted_stage(dir.path());
        kill_parked_child(dir.path(), "-9");
        let journal = dir.path().join(".traces/index.redb.journal");
        assert!(journal.exists(), "SIGKILL lost the journal");
        let result = checked_recovery(dir.path());
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        assert!(result.contains("store_reopen_ms="), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn recovers_to_committed_state_after_sigint_mid_replay() {
        let dir = seeded();
        aborted_stage(dir.path());
        kill_parked_child(dir.path(), "-2");
        let journal = dir.path().join(".traces/index.redb.journal");
        assert!(journal.exists(), "SIGINT lost the journal");
        let result = checked_recovery(dir.path());
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        assert!(result.contains("store_reopen_ms="), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn prints_repair_timing_and_integrity_after_abort() {
        let dir = seeded();
        aborted_stage(dir.path());
        let output = call_envs(dir.path(), "recover", None, &[(
            "PROBE_CHECK_INTEGRITY",
            "1",
        )]);
        assert!(
            output.status.success(),
            "abort-class recover failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stdout.contains("store_reopen_ms="), "{stdout}");
        assert!(stderr.contains("integrity_ms="), "{stderr}");
        success(dir.path(), "verify", None);
    }

    #[test]
    fn recovers_after_panic_before_replay_commit() {
        use std::os::unix::process::ExitStatusExt;
        let dir = seeded();
        aborted_stage(dir.path());
        let output = call_envs(dir.path(), "recover", None, &[(
            "PROBE_PANIC",
            "before_commit",
        )]);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("probe panic at before_commit"), "{stderr}");
        if !cfg!(debug_assertions) {
            assert_eq!(
                output.status.signal(),
                Some(libc::SIGABRT),
                "release panic must abort"
            );
        }
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = call_envs(dir.path(), "recover", None, &[(
            "PROBE_CHECK_INTEGRITY",
            "1",
        )]);
        assert!(result.status.success(), "{result:?}");
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("outcome=replayed parse_count=0"), "{stdout}");
        assert!(stdout.contains("store_reopen_ms="), "{stdout}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("integrity_ms="),
            "{result:?}"
        );
        success(dir.path(), "verify", None);
    }
}

mod concurrency {
    use pretty_assertions::assert_eq;

    use super::*;
    #[test]
    fn uses_source_query_while_another_process_holds_writer_handle() {
        let dir = seeded();
        success(dir.path(), "stage", Some("deferred"));
        let journal = dir.path().join(".traces/index.redb.journal");
        let mut holder = ChildGuard(
            probe_command(dir.path(), "hold", None)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("spawn holder"),
        );
        readiness(
            holder.0.stdout.take().expect("holder stdout"),
            "hold ready\n",
        );
        let fallback = success(dir.path(), "query", None);
        assert!(fallback.contains("reason=store-open-failed"), "{fallback}");
        assert!(fallback.contains("pages=13"), "{fallback}");
        assert!(journal.exists(), "pure query opened the journal");
        drop(holder.0.stdin.take());
        wait_for_child(&mut holder.0);
        assert!(holder.0.wait().expect("holder exit").success());
        let read = success(dir.path(), "query", None);
        assert!(read.contains("outcome=store-read"), "{read}");
        success(dir.path(), "checkpoint", None);
        success(dir.path(), "verify", None);
    }

    #[test]
    fn refuses_replay_while_publishing_writer_is_alive() {
        let dir = seeded();
        let mut writer = parked(dir.path(), "stage", "after_publication");
        let journal = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&journal).expect("read live journal");
        for command in ["query", "recover", "recover"] {
            let result = success(dir.path(), command, None);
            assert!(result.contains("reason=store-open-failed"), "{result}");
            assert!(result.contains("pages=13"), "{result}");
            assert_eq!(
                fs::read(&journal).expect("retained live journal"),
                before
            );
        }
        writer.0.kill().expect("kill publishing writer");
        wait_for_child(&mut writer.0);
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
        let repeat = success(dir.path(), "recover", None);
        assert!(repeat.contains("outcome=unchanged"), "{repeat}");
    }

    #[test]
    fn refuses_second_replay_while_first_replayer_is_alive() {
        let dir = seeded();
        aborted_stage(dir.path());
        let mut replayer = parked(dir.path(), "recover", "replay");
        let journal = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&journal).expect("read replay journal");
        let concurrent = success(dir.path(), "recover", None);
        assert!(
            concurrent.contains("reason=store-open-failed"),
            "{concurrent}"
        );
        assert_eq!(
            fs::read(&journal).expect("retained replay journal"),
            before
        );
        replayer.0.kill().expect("kill first replayer");
        wait_for_child(&mut replayer.0);
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
        let repeat = success(dir.path(), "recover", None);
        assert!(repeat.contains("outcome=unchanged"), "{repeat}");
    }

    #[test]
    fn retains_journal_when_checkpoint_writer_base_changes() {
        let dir = seeded();
        success(dir.path(), "stage", Some("deferred"));
        let mut checkpoint =
            parked(dir.path(), "checkpoint", "before_checkpoint_commit");
        let journal = dir.path().join(".traces/index.redb.journal");
        let before = fs::read(&journal).expect("read checkpoint journal");
        success(dir.path(), "mutate-identity", Some("revision"));
        drop(checkpoint.0.stdin.take());
        wait_for_child(&mut checkpoint.0);
        assert!(!checkpoint.0.wait().expect("checkpoint exit").success());
        assert_eq!(
            fs::read(&journal).expect("retained checkpoint journal"),
            before
        );
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        assert!(!journal.exists());
        success(dir.path(), "verify", None);
    }
}

mod enospc {

    fn podman(args: &[&str]) -> std::process::Output {
        super::bounded_output(std::process::Command::new("podman").args(args))
    }
    fn podman_ok(args: &[&str]) -> String {
        let output = podman(args);
        assert!(
            output.status.success(),
            "podman {args:?} failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
    fn probe_in_container(args: &str) -> (bool, String) {
        let output = podman(&[
            "exec",
            "t27-enospc",
            "sh",
            "-c",
            &format!("/tmp/probe {args} 2>&1"),
        ]);
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
        )
    }
    // NOTE: requires the container-prepared /tmp/probe binary. The binary
    // must be a Linux build of this crate. A macOS build would not run in
    // t27-enospc.
    //
    // GATE THAT WE CANNOT CLOSE AS SPECIFIED, with the exact reason: redb
    // 4.3.0 maps the database file with mmap. On tmpfs (and any
    // sparse-capable filesystem), an ENOSPC commit with Durability::None
    // does NOT return an error. redb ftruncates the file to its 2x growth
    // target and the mmap writes land in a sparse hole. The commit then
    // reports success while the data is silently lost. Verification: the
    // file grew to size=2109440 with blocks=352, and query serves the
    // stale pre-write state. With a hard file-size cap (ulimit -f), the
    // process dies by SIGXFSZ/SIGBUS. Again there is no catchable error.
    // The decided reopen-and-classify path in `enospc-write` therefore has
    // no ENOSPC error surface to exercise on this stack. The test below
    // pins the OBSERVED behavior instead. A silent-loss commit is a
    // stronger adoption blocker than a new failure class.
    #[test]
    #[ignore = "requires podman container t27-enospc and the in-container \
                Linux build; run with T27_ENOSPC=1"]
    fn classifies_real_enospc_store_write_and_recovers() {
        if std::env::var("T27_ENOSPC").ok().as_deref() != Some("1") {
            eprintln!("skipped: T27_ENOSPC not set");
            return;
        }
        // Precondition: the container must hold a Linux build of the probe.
        // Copy the worktree to /work. Run `cargo build --release
        // --manifest-path /work/.scratch/.../replay/Cargo.toml` inside
        // t27-enospc. Then run `cp target/release/replay-fault-probe
        // /tmp/probe`.
        let probe = podman(&["exec", "t27-enospc", "test", "-x", "/tmp/probe"]);
        assert!(probe.status.success(), "/tmp/probe missing in t27-enospc");

        // Probe-only leak check. After every scenario, drain the tmpfs so
        // later rounds start from a known state.
        let drain = |extra: &str| {
            podman_ok(&[
                "exec",
                "t27-enospc",
                "sh",
                "-c",
                &format!("rm -f /spill/vault/extra-*.md /spill/filler {extra}"),
            ]);
        };
        let seed_and_cap = |with_journal: bool| -> String {
            podman_ok(&[
                "exec",
                "t27-enospc",
                "sh",
                "-c",
                "rm -rf /spill/vault /spill/filler && /tmp/probe seed \
                 /spill/vault 200",
            ]);
            let store_bytes: u64 = podman_ok(&[
                "exec",
                "t27-enospc",
                "sh",
                "-c",
                "stat -c %s /spill/vault/.traces/index.redb",
            ])
            .parse()
            .expect("store size");
            if with_journal {
                // Deferred stage: journal publishes while space is ample.
                let (ok, out) =
                    probe_in_container("stage /spill/vault deferred");
                assert!(ok, "deferred stage failed: {out}");
            }
            // 4000 structured notes force the commit to roughly double the
            // store (~1 MiB of new rows).
            podman_ok(&[
                "exec",
                "t27-enospc",
                "sh",
                "-c",
                "for i in $(seq 1000 4999); do printf -- '---\\nclass: \
                 Project\\nrating: 3\\n---\\n# Extra %s\\n[[hub]]\\n' \"$i\" \
                 > /spill/vault/extra-$i.md; done",
            ]);
            // Cap free space at 8 KiB: far below the growth the commit needs.
            podman_ok(&[
                "exec",
                "t27-enospc",
                "sh",
                "-c",
                "AV=$(df --output=avail --block-size=1 /spill | tail -1); dd \
                 if=/dev/zero of=/spill/filler bs=1024 count=$(( (AV - 8192) \
                 / 1024 )) 2>/dev/null",
            ]);
            format!("{}", store_bytes)
        };

        // --- Case 1: pure store-write ENOSPC (no journal) ---
        seed_and_cap(false);
        let avail: u64 = podman_ok(&[
            "exec",
            "t27-enospc",
            "sh",
            "-c",
            "df --output=avail --block-size=1 /spill | tail -1",
        ])
        .trim()
        .parse()
        .expect("avail bytes");
        assert!(avail < 16 * 1024, "filler failed, avail={avail}");
        let (ok, out) = probe_in_container("enospc-write /spill/vault");
        // OBSERVED: the commit does NOT error. It reports success and the
        // store stays logically doubled but physically sparse.
        assert!(ok, "unexpected error surface: {out}");
        assert!(out.contains("outcome=written"), "{out}");
        let logical: String = podman_ok(&[
            "exec",
            "t27-enospc",
            "sh",
            "-c",
            "stat -c %s /spill/vault/.traces/index.redb",
        ]);
        let physical: u64 = podman_ok(&[
            "exec",
            "t27-enospc",
            "sh",
            "-c",
            "stat -c %b /spill/vault/.traces/index.redb",
        ])
        .parse()
        .expect("blocks");
        let logical_bytes: u64 = logical.parse().expect("logical size");
        assert!(
            logical_bytes > physical * 512,
            "expected a sparse hole (silent loss), got \
             logical={logical_bytes} physical={}",
            physical * 512
        );
        // The parsed set was NOT durable: the store still serves only the
        // seeded 201 pages.
        let (_ok, out) = probe_in_container("query /spill/vault");
        assert!(
            out.contains("pages=201"),
            "silent-loss store should serve stale seed state: {out}"
        );
        drain("");

        // --- Case 2: journal-induced ENOSPC (journal retained, store write
        // needs the space). The failure surface must be the SAME as case 1
        // (silent success, no error, no new class). ---
        seed_and_cap(true);
        let (ok, out) = probe_in_container("enospc-write /spill/vault");
        assert!(ok, "journal-induced surface differs from case 1: {out}");
        assert!(out.contains("outcome=written"), "{out}");
        assert!(
            !out.contains("classify="),
            "journal case produced an error surface case 1 lacks"
        );
        drain("");
        eprintln!(
            "ENOSPC finding: redb 4.3 mmap + Durability::None silently \
             succeeds ENOSPC commits as sparse holes on tmpfs; no error text \
             exists to capture."
        );

        // --- Signal-kill surface: a hard file-size cap kills the process
        // instead of returning an error. ---
        podman_ok(&["exec", "t27-enospc", "sh", "-c", "true"]);
        let output = podman(&[
            "exec",
            "t27-enospc",
            "sh",
            "-c",
            "ulimit -f 2000; /tmp/probe enospc-write /spill/vault 2>&1; echo \
             probe_exit=$?",
        ]);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            combined.contains("File size limit exceeded")
                || combined.contains("probe_exit=153"),
            "expected a signal death surface: {combined}"
        );
    }
}
