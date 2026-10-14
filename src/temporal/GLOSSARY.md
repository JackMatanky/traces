# Temporal

Date, Date-Time, and Duration parsing, local-zone resolution, and calendar
arithmetic. `src/temporal/` is the single owner of these values and their rules.

## Language

### Temporal Values

<!-- rumdl-disable-next-line MD024 -->
#### Date

A zone-free calendar date. Every date-shaped string, including serde
deserialization, uses the temporal parser. Parsing requires a four-digit year;
`YYYY-MM` normalizes to the first day of the month. `DateValue` represents
a whole calendar date using Jiff's zone-free civil date type; written precision
(month or day) is preserved separately by `DatePoint`.

#### Date-Time

A point on the timeline stored as UTC and rendered using the process-local clock.
Unzoned input resolves in the process's local zone. A repeated local time takes
the earlier occurrence; a skipped local time shifts forward by the gap length.
Comparisons with Date promote the Date to local midnight. If the local offset
cannot be applied at the representable range edge, the UTC reading is used.
*Avoid*: timestamp, instant

#### Duration

A validated duration expression. Nominal seconds define its identity, so one
month equals 30 days and one year equals 365 days for equality and ordering. A
parsed Duration retains its written spelling; a synthesized or computed Duration
uses canonical spelling. Calendar application uses its component sequence, while
an exact-only Duration applies as elapsed time.
*Avoid*: interval, period

### Calendar Regime

Calendar units (day, week, month, and year) apply left-to-right on the local
clock. Exact units (hour and smaller) apply as elapsed time. Thus equal Durations
such as `1d` and `24h` can move a Date-Time differently across calendar
boundaries and daylight-saving transitions.

#### Zone

The process-local zone used to resolve unzoned Date-Time input, render stored
Date-Time values, and apply calendar units. System zone lookup errors propagate
as `DateError::LocalZoneLookup`.

#### Time unit

`TimeUnit` is one of the eight accepted duration quantities. Its shared
registry defines spellings, nominal length, calendar-versus-elapsed application,
and English names. A week here is a seven-day calendar amount.

#### Date unit

`DateUnit` selects a field from a date point's local civil reading: year,
month, day, hour, minute, second, ISO weekday number, or ISO week number. It
does not represent an amount that can be added to a date.

#### Date-time input

`DateTimeInput` is a temporary recognition result. An explicit offset has
already been applied to produce an absolute timestamp; a local clock reading
still needs zone resolution before it becomes `DateTimeValue`.

#### Date difference

`DateDiff` distinguishes whole calendar measurements from fractional
clock/elapsed measurements; query consumers convert it to a number and
template consumers retain integer-versus-float output.
