//! Process-stop tests for the throwaway shadow store. Use --release for
//! panic=abort.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

fn call(
    root: &Path,
    command: &str,
    extra: Option<&str>,
    stop: Option<&str>,
) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_replay-fault-probe"));
    child.arg(command).arg(root);
    if let Some(arg) = extra {
        child.arg(arg);
    }
    if let Some(point) = stop {
        child.env("PROBE_STOP", point);
    }
    child.output().expect("run probe child")
}
fn success(root: &Path, command: &str, extra: Option<&str>) -> String {
    let result = call(root, command, extra, None);
    assert!(
        result.status.success(),
        "{command} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).expect("child stdout")
}
fn injected(
    root: &Path,
    command: &str,
    extra: Option<&str>,
    fault: &str,
) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_replay-fault-probe"));
    child.arg(command).arg(root).env("PROBE_FAIL", fault);
    if let Some(arg) = extra {
        child.arg(arg);
    }
    child.output().expect("run injected child")
}
fn seeded() -> TempDir {
    let dir = tempfile::tempdir().expect("temporary vault");
    success(dir.path(), "seed", Some("12"));
    dir
}

mod publication {
    use super::*;
    #[test]
    fn replays_parsed_facts_after_published_writer_abort() {
        let dir = seeded();
        let child = call(dir.path(), "stage", None, Some("after_publication"));
        assert!(!child.status.success());
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
        let repeat = success(dir.path(), "recover", None);
        assert!(repeat.contains("outcome=unchanged"), "{repeat}");
    }
    #[test]
    fn reparses_sources_when_writer_stops_before_journal_publication() {
        let dir = seeded();
        let child = call(dir.path(), "stage", None, Some("partial_write"));
        assert!(!child.status.success());
        assert!(!dir.path().join(".traces/index.redb.journal").exists());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_after_abort_before_immediate_store_commit() {
        let dir = seeded();
        let child = call(dir.path(), "stage", None, Some("before_commit"));
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_no_replay_control_at_complete_query_check() {
        let dir = seeded();
        let child = call(dir.path(), "stage", None, Some("after_publication"));
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", Some("no-replay"));
        assert!(result.contains("no-replay-control"));
        let check = call(dir.path(), "verify", None, None);
        assert!(!check.status.success(), "no-replay control falsely passed");
        success(dir.path(), "recover", None);
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_retained_journal_after_immediate_commit_abort() {
        let dir = seeded();
        let child = call(dir.path(), "stage", None, Some("before_unlink"));
        assert!(!child.status.success());
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=unchanged"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_after_aborted_replay_before_removal_only_once() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        assert!(
            !call(dir.path(), "recover", None, Some("after_replay_commit"))
                .status
                .success()
        );
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
        let staged = success(dir.path(), "stage", Some("none"));
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
        success(dir.path(), "stage", Some("none"));
        assert!(
            !call(
                dir.path(),
                "checkpoint",
                None,
                Some("after_checkpoint_commit")
            )
            .status
            .success()
        );
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        let next = success(dir.path(), "recover", None);
        assert!(next.contains("outcome=unchanged"), "{next}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn returns_same_injected_store_commit_error_with_and_without_journal() {
        let journaled = seeded();
        let bare = seeded();
        let with_journal =
            injected(journaled.path(), "stage", None, "store_commit");
        let without_journal =
            injected(bare.path(), "stage", Some("no-journal"), "store_commit");
        assert!(!with_journal.status.success());
        assert!(!without_journal.status.success());
        let first = String::from_utf8_lossy(&with_journal.stderr);
        let second = String::from_utf8_lossy(&without_journal.stderr);
        assert!(
            first.contains("probe error: injected ENOSPC at store_commit"),
            "{first}"
        );
        assert!(
            second.contains("probe error: injected ENOSPC at store_commit"),
            "{second}"
        );
        assert!(journaled.path().join(".traces/index.redb.journal").exists());
        assert!(!bare.path().join(".traces/index.redb.journal").exists());
        success(journaled.path(), "recover", None);
        success(bare.path(), "recover", None);
        success(journaled.path(), "verify", None);
        success(bare.path(), "verify", None);
    }
}

mod validation {
    use super::*;
    #[test]
    fn discards_journal_when_unlisted_source_changes() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        fs::write(dir.path().join("hub.md"), "# New Hub #changed\n")
            .expect("mutate unlisted hub");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_journal_when_changed_source_changes_again() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        fs::write(dir.path().join("note-0.md"), "# changed twice #fresh\n")
            .expect("mutate listed note");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn discards_corrupt_journal_and_checks_fresh_rows() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        let path = dir.path().join(".traces/index.redb.journal");
        fs::write(&path, b"bad").expect("corrupt journal");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn replays_when_saved_precommit_store_image_is_restored() {
        let dir = seeded();
        let store = dir.path().join(".traces/replay-shadow.redb");
        let pristine = tempfile::NamedTempFile::new().expect("saved image");
        fs::copy(&store, pristine.path()).expect("save store image");
        success(dir.path(), "stage", Some("none"));
        assert!(dir.path().join(".traces/index.redb.journal").exists());
        fs::copy(pristine.path(), &store).expect("restore old base image");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=replayed parse_count=0"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_same_revision_fork_with_different_note_rows() {
        use redb::{Database, TableDefinition};
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        let db =
            Database::create(dir.path().join(".traces/replay-shadow.redb"))
                .expect("open fork");
        let mut tx = db.begin_write().expect("write fork");
        {
            let mut notes = tx
                .open_table(TableDefinition::<&str, &[u8]>::new("notes"))
                .expect("notes");
            notes
                .insert("hub.md", b"forked rows".as_slice())
                .expect("fork row");
        }
        tx.set_durability(redb::Durability::Immediate).expect("set durability");
        tx.commit().expect("commit fork");
        drop(db);
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=parsed"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn rejects_journal_when_store_is_missing() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        fs::remove_file(dir.path().join(".traces/replay-shadow.redb"))
            .expect("remove store");
        let result = success(dir.path(), "recover", None);
        assert!(result.contains("outcome=missing-store-rebuilt"), "{result}");
        success(dir.path(), "verify", None);
    }
    #[test]
    fn preserves_future_epoch_journal() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
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
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
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
            assert!(
                !call(dir.path(), "stage", None, Some("after_publication"))
                    .status
                    .success()
            );
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
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
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
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
        let path = dir.path().join(".traces/index.redb.journal");
        let mut bytes = fs::read(&path).expect("read journal");
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        let checksum = *blake3::hash(&bytes[..24]).as_bytes();
        bytes[24..56].copy_from_slice(&checksum);
        fs::write(&path, bytes).expect("write false length");
        let output = call(dir.path(), "recover", None, None);
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
    fn retains_unknown_header_version_and_serves_fresh_query() {
        let dir = seeded();
        assert!(
            !call(dir.path(), "stage", None, Some("after_publication"))
                .status
                .success()
        );
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
    #[cfg(unix)]
    #[test]
    fn rejects_fifo_journal_without_blocking_or_aborting() {
        use std::{
            process::Stdio,
            thread,
            time::{Duration, Instant},
        };
        let dir = seeded();
        let path = dir.path().join(".traces/index.redb.journal");
        let create = Command::new("mkfifo")
            .arg(&path)
            .status()
            .expect("mkfifo executable");
        assert!(create.success(), "mkfifo failed");
        let mut child = Command::new(env!("CARGO_BIN_EXE_replay-fault-probe"))
            .arg("recover")
            .arg(dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn recovery");
        let started = Instant::now();
        loop {
            if child.try_wait().expect("check recovery child").is_some() {
                break;
            }
            if started.elapsed() > Duration::from_secs(20) {
                child.kill().expect("kill blocked recovery");
                child.wait().expect("reap blocked recovery");
                panic!("recovery blocked on FIFO");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().expect("read recovery result");
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
    #[cfg(unix)]
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
}

mod concurrency {
    use super::*;
    #[test]
    fn uses_source_query_while_another_process_holds_writer_handle() {
        use std::{io::BufRead, process::Stdio};
        let dir = seeded();
        success(dir.path(), "stage", Some("none"));
        let journal = dir.path().join(".traces/index.redb.journal");
        let mut holder = Command::new(env!("CARGO_BIN_EXE_replay-fault-probe"))
            .arg("hold")
            .arg(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn holder");
        let mut ready = String::new();
        std::io::BufReader::new(holder.stdout.take().expect("holder stdout"))
            .read_line(&mut ready)
            .expect("wait for holder");
        assert_eq!(ready, "hold ready\n");
        let fallback = success(dir.path(), "query", None);
        assert!(fallback.contains("reason=store-open-failed"), "{fallback}");
        assert!(fallback.contains("pages=13"), "{fallback}");
        assert!(journal.exists(), "pure query opened the journal");
        drop(holder.stdin.take());
        assert!(holder.wait().expect("holder exit").success());
        let read = success(dir.path(), "query", None);
        assert!(read.contains("outcome=shadow-read"), "{read}");
        success(dir.path(), "checkpoint", None);
        success(dir.path(), "verify", None);
    }
}
