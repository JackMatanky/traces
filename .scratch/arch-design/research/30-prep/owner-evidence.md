# Owner-verified evidence notes

Small checks the integration owner ran itself. All [V] unless marked.

## Jiff system-zone cache (supports Concurrency H4, Scaling H1, questions 30/33/34)

- jiff 0.2.38 `TimeZone::try_system` (`.rust-docs/cache/crates/jiff/0.2.38/source/src/tz/timezone.rs:391-417`) delegates to `crate::tz::system::get(crate::tz::db())` under feature `tz-system`; without the feature it always errors.
- `src/tz/system/mod.rs:69`: `static TTL: Duration = Duration::new(5 * 60, 0)`. The cached system zone expires after five minutes.
- `src/tz/system/mod.rs:123-133`: `get` returns the cached zone while unexpired; after expiry it re-reads the host (`TZ`, then `/etc/localtime`, then TZif).
- Consequence: an operation longer than five minutes, or any operation that straddles an expiry, can observe two different zones. Each `local_zone()` call also takes a process-global `RwLock` read (`mod.rs:87`). A pinned frame that captures the zone once removes both hazards and the per-row lock traffic.
- Feature check for the project [V: `Cargo.toml:128`]: `jiff = { version = "=0.2.38", default-features = false, features = ["std", "tz-system", "tzdb-bundle-always"] }`. The project enables `tz-system` by name, so `try_system` is live. A correction: an earlier note of this file said "default features". That note was wrong.

## DatePoint shape (supports Layout H1/H2, question 19)

- `src/temporal/point.rs:41-52`: `DatePoint` is a two-variant `Copy` enum: `Calendar { date: DateValue, date_precision: DatePrecision }` and `DateTime(DateTimeValue)`. No `Ord` derive.
- `src/note/field.rs:42-65`: `NoteFieldValue` keeps `Date(DateValue)` and `DateTime(DateTimeValue)` variants; ticket 18 measured the enum at 80 bytes (`src/note/field.rs:579-581` static test).
- Consequence: the outer-variant shape already separates civil from instant and needs no growth in `NoteFieldValue`. Q19's realistic choice is ratify-plus-pin (static size and no-`Ord` asserts), not a redesign.
