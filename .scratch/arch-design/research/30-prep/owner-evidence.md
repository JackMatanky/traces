# Owner-checked evidence notes

The integration owner records these source checks as [V].
They describe the inspected preparation baseline, not a new execution.
The cache checks inform Concurrency H4 and Scaling H1. The carrier check
informs Layout H1/H2 and Q19.

## Jiff system-zone cache (questions 30, 33 and 34)

Jiff 0.2.38 `TimeZone::try_system` delegates to
`crate::tz::system::get(crate::tz::db())` when `tz-system` is enabled.
Without that feature, it returns an error. Source:
`.rust-docs/cache/crates/jiff/0.2.38/source/src/tz/timezone.rs:391-417`.

The system-zone module defines `static TTL: Duration = Duration::new(5 * 60, 0)`
at `src/tz/system/mod.rs:69`. The cache expires after five minutes.
While the cached value is valid, `get` returns it. After expiry, `get`
reads the host again (`src/tz/system/mod.rs:123-133`). Host inputs include
`TZ`, `/etc/localtime` and TZif data.

An operation that crosses an expiry can observe different zones if the
host inputs change. Each `local_zone()` call also takes a process-global
`RwLock` read (`mod.rs:87`). A frame that retains one acquisition result
avoids repeated lookups and zone changes within that operation.

The project explicitly enables `tz-system` and disables default features.
The exact dependency at `Cargo.toml:128` is:

```toml
jiff = { version = "=0.2.38", default-features = false, features = ["std", "tz-system", "tzdb-bundle-always"] }
```

An earlier version of this report incorrectly said that the project uses
default features.

## DatePoint shape (question 19; corrected conclusion)

`DatePoint` is a two-variant `Copy` enum at `src/temporal/point.rs:41-52`.
Its variants are `Calendar { date: DateValue, date_precision: DatePrecision }`
and `DateTime(DateTimeValue)`. It does not derive `Ord`.
`DateTimeValue` is a transparent `Timestamp` wrapper with a derived `Ord`
(`src/temporal/datetime.rs:74-78`).

`NoteFieldValue` contains `Date(DateValue)` and `DateTime(DateTimeValue)`
variants (`src/note/field.rs:42-65`). Ticket 18 records its size as 80 bytes.
The static size assertion is at `src/note/field.rs:579-581`.

Parse-time `LocalClock` values resolve through the process-local zone
before storage (`src/temporal/datetime.rs:622-657`,
`src/temporal/datetime.rs:133-154`). `DatePoint` uses that resolution path
(`src/temporal/point.rs:105-114`).

Thus, the inspected runtime does not retain a civil datetime. Its outer
`Calendar`/`DateTime` split separates civil dates from instants, not civil
datetimes from instants. The earlier recommendation to retain that shape
is withdrawn. Question 19 must select a carrier that retains civil
datetimes. See [i-leakage-review.md](./i-leakage-review.md), L5.
