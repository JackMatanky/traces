use std::{
    env,
    error::Error,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::MetadataExt,
    path::Path,
    process::ExitCode,
};

use redb::{
    Database, Durability, ReadableDatabase, ReadableTable, TableDefinition,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MIB: usize = 1024 * 1024;
const ROWS: u64 = 128;
const HEADER: &str = "enospc-probe-v1 redb=4.3.0 rows=128 value_bytes=1048576";
const TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("rows");

fn payload() -> Vec<u8> {
    let mut bytes = vec![0; MIB];
    let mut state = 0x1234_5678_9abc_def0_u64;
    for chunk in bytes[8..].chunks_exact_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes());
    }
    bytes
}

fn space(path: &Path, phase: &str) -> Result<()> {
    let stats = rustix::fs::statvfs(path)?;
    let metadata = fs::metadata(path)?;
    println!(
        "space phase={phase} available_bytes={} free_bytes={} total_bytes={} \
         logical_bytes={} allocated_bytes={}",
        u128::from(stats.f_bavail) * u128::from(stats.f_frsize),
        u128::from(stats.f_bfree) * u128::from(stats.f_frsize),
        u128::from(stats.f_blocks) * u128::from(stats.f_frsize),
        metadata.len(),
        u128::from(metadata.blocks()) * 512,
    );
    Ok(())
}

fn record(receipt: &mut File, line: &str) -> Result<()> {
    writeln!(receipt, "{line}")?;
    receipt.sync_data()?;
    Ok(())
}

fn insert(
    db: &Database,
    durability: Durability,
    key: u64,
    value: &[u8],
    stage: &mut &'static str,
) -> std::result::Result<(), redb::Error> {
    *stage = "begin_write";
    let mut transaction = db.begin_write()?;
    *stage = "set_durability";
    transaction.set_durability(durability)?;
    {
        *stage = "open_table";
        let mut table = transaction.open_table(TABLE)?;
        *stage = "insert";
        table.insert(key, value)?;
    }
    *stage = "commit";
    transaction.commit()?;
    Ok(())
}

fn write(
    db_path: &Path,
    receipt_path: &Path,
    mode: &str,
    journal_mib: u64,
) -> Result<u8> {
    let durability = match mode {
        "none" => Durability::None,
        "immediate" => Durability::Immediate,
        _ => return Err("use none or immediate for durability".into()),
    };
    if journal_mib > 32 {
        return Err("use 0 through 32 for journal_mib".into());
    }
    let mut receipt =
        OpenOptions::new().write(true).create_new(true).open(receipt_path)?;
    let database_file =
        OpenOptions::new().write(true).create_new(true).open(db_path)?;
    if receipt.metadata()?.dev() == database_file.metadata()?.dev() {
        return Err("put the receipt outside the database filesystem".into());
    }
    drop(database_file);
    record(&mut receipt, HEADER)?;
    record(
        &mut receipt,
        &format!("durability={mode} journal_mib={journal_mib}"),
    )?;
    let db = Database::builder().set_cache_size(MIB).create(db_path)?;
    let mut seed = db.begin_write()?;
    seed.set_durability(Durability::Immediate)?;
    drop(seed.open_table(TABLE)?);
    seed.commit()?;
    space(db_path, "seed")?;

    let mut value = payload();
    if journal_mib != 0 {
        // This file controls space use. It is not a replay journal.
        let journal_path = db_path.with_extension("retained-journal");
        let mut journal = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&journal_path)?;
        for _ in 0..journal_mib {
            journal.write_all(&value)?;
        }
        journal.sync_all()?;
        space(&journal_path, "retained_journal")?;
    }
    space(db_path, "before_rows")?;
    println!(
        "write redb=4.3.0 durability={mode} cache_bytes={MIB} \
         attempted_rows={ROWS} journal_mib={journal_mib}"
    );
    let mut committed = 0;
    let mut outcome = "complete";
    let mut status = 0;
    for key in 0..ROWS {
        value[..8].copy_from_slice(&key.to_le_bytes());
        let mut stage = "begin_write";
        if let Err(error) = insert(&db, durability, key, &value, &mut stage) {
            let enospc = matches!(&error, redb::Error::Io(error) if error.raw_os_error() == Some(libc::ENOSPC));
            eprintln!(
                "write_error stage={stage} key={key} enospc={enospc} \
                 error={error:?}"
            );
            outcome = if enospc {
                "enospc"
            } else {
                "error"
            };
            status = if enospc {
                3
            } else {
                2
            };
            break;
        }
        committed += 1;
        record(&mut receipt, &format!("commit_ok={committed}"))?;
        println!("commit_ok={committed}");
        space(db_path, "after_commit")?;
    }
    space(db_path, "before_drop")?;
    // Database::drop has no error result. The check process tests row survival.
    drop(db);
    space(db_path, "after_drop")?;
    record(&mut receipt, &format!("writer_finished={outcome}"))?;
    println!(
        "write outcome={outcome} committed_rows={committed} \
         database_drop=returned_unchecked"
    );
    Ok(status)
}

fn receipt(text: &str) -> Result<(u64, &str)> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("invalid receipt header".into());
    }
    let config = lines.next().ok_or("missing receipt configuration")?;
    if !(config.starts_with("durability=none journal_mib=")
        || config.starts_with("durability=immediate journal_mib="))
    {
        return Err("invalid receipt configuration".into());
    }
    let mut committed = 0;
    for line in lines.by_ref() {
        if let Some(count) = line.strip_prefix("commit_ok=") {
            let count: u64 = count.parse()?;
            if count != committed + 1 || count > ROWS {
                return Err("receipt has a gap or an invalid row count".into());
            }
            committed = count;
        } else if let Some(outcome) = line.strip_prefix("writer_finished=") {
            if !matches!(outcome, "complete" | "enospc" | "error")
                || lines.next().is_some()
            {
                return Err("invalid receipt outcome".into());
            }
            if outcome == "complete" && committed != ROWS {
                return Err(
                    "complete receipt does not acknowledge all rows".into()
                );
            }
            return Ok((committed, outcome));
        } else {
            return Err("invalid receipt record".into());
        }
    }
    Err("incomplete receipt: do not classify an interrupted probe as ENOSPC"
        .into())
}

fn rows(
    db: &impl ReadableDatabase,
    committed: u64,
    phase: &str,
) -> Result<bool> {
    let transaction = db.begin_read()?;
    let table = transaction.open_table(TABLE)?;
    let mut seen = [false; ROWS as usize];
    let mut expected = payload();
    let mut actual = 0;
    let mut invalid = 0;
    let mut unacknowledged = 0;
    for entry in table.iter()? {
        let (key, value) = entry?;
        let key = key.value();
        actual += 1;
        if key >= ROWS {
            invalid += 1;
            continue;
        }
        seen[usize::try_from(key)?] = true;
        expected[..8].copy_from_slice(&key.to_le_bytes());
        if value.value() != expected.as_slice() {
            invalid += 1;
        }
        if key >= committed {
            // A failed commit can still become durable. Do not call this loss.
            unacknowledged += 1;
        }
    }
    let missing = seen[..usize::try_from(committed)?]
        .iter()
        .filter(|seen| !**seen)
        .count();
    println!(
        "rows phase={phase} acknowledged={committed} actual={actual} \
         missing={missing} invalid={invalid} unacknowledged={unacknowledged}"
    );
    Ok(missing == 0 && invalid == 0)
}

fn check(db_path: &Path, receipt_path: &Path) -> Result<u8> {
    let text = fs::read_to_string(receipt_path)?;
    let (committed, outcome) = receipt(&text)?;
    println!("check writer_outcome={outcome} acknowledged_rows={committed}");
    space(db_path, "before_read_only_open")?;
    let mut survival = true;
    let mut read_only_error = false;
    let mut inspection_error = false;
    match Database::builder().set_cache_size(MIB).open_read_only(db_path) {
        Ok(db) => match rows(&db, committed, "read_only_pristine") {
            Ok(ok) => survival &= ok,
            Err(error) => {
                read_only_error = true;
                eprintln!("check_error phase=read_only_rows error={error:?}");
            }
        },
        Err(error) => {
            read_only_error = true;
            eprintln!("check_error phase=read_only_open error={error:?}");
        }
    }
    // Writable open can repair the file. Read the pristine file first.
    let mut db = Database::builder().set_cache_size(MIB).open(db_path)?;
    match rows(&db, committed, "writable_reopen_before_integrity") {
        Ok(ok) => survival &= ok,
        Err(error) => {
            inspection_error = true;
            eprintln!(
                "check_error phase=writable_rows_before_integrity \
                 error={error:?}"
            );
        }
    }
    let clean = db.check_integrity()?;
    println!(
        "integrity phase=writable_reopen clean={clean} repaired={}",
        !clean
    );
    match rows(&db, committed, "writable_reopen_after_integrity") {
        Ok(ok) => survival &= ok,
        Err(error) => {
            eprintln!(
                "check_error phase=writable_rows_after_integrity \
                 error={error:?}"
            );
            return Ok(2);
        }
    }
    drop(db);
    space(db_path, "after_writable_drop")?;
    println!(
        "check acknowledged_rows_survived={survival} integrity_clean={clean} \
         pristine_read_unavailable={read_only_error} writer_outcome={outcome}"
    );
    Ok(if !survival || !clean {
        1
    } else if read_only_error || inspection_error {
        2
    } else {
        0
    })
}

fn run() -> Result<u8> {
    let mut args = env::args_os().skip(1);
    let mode = args.next().ok_or("missing mode: write or check")?;
    let db = args.next().ok_or("missing database path")?;
    let receipt = args.next().ok_or("missing receipt path")?;
    match mode.to_str() {
        Some("write") => {
            let durability =
                args.next().ok_or("missing durability: none or immediate")?;
            let journal = args.next().ok_or("missing journal_mib")?;
            if args.next().is_some() {
                return Err("too many arguments".into());
            }
            write(
                Path::new(&db),
                Path::new(&receipt),
                durability.to_str().ok_or("invalid durability")?,
                journal.to_str().ok_or("invalid journal_mib")?.parse()?,
            )
        }
        Some("check") if args.next().is_none() => {
            check(Path::new(&db), Path::new(&receipt))
        }
        _ => Err("usage: enospc-probe write DB RECEIPT none|immediate \
                  JOURNAL_MIB; enospc-probe check DB RECEIPT"
            .into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            eprintln!("probe_error error={error:?}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_receipt_when_writer_did_not_finish() {
        let text =
            format!("{HEADER}\ndurability=none journal_mib=0\ncommit_ok=1\n");
        assert!(receipt(&text).is_err());
    }

    #[test]
    fn rejects_receipt_when_acknowledgements_have_a_gap() {
        let text = format!(
            "{HEADER}\ndurability=none \
             journal_mib=0\ncommit_ok=2\nwriter_finished=enospc\n"
        );
        assert!(receipt(&text).is_err());
    }
}
