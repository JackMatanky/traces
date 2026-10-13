# Temporal

Date, Date-Time, and Duration parsing, local-zone resolution, and calendar
arithmetic. `src/temporal/` is the single owner of these values and their rules.

## Language

### Temporal Values

<!-- rumdl-disable-next-line MD024 -->
#### Date

A zone-free calendar date. Every date-shaped string, including serde
deserialization, uses the temporal parser. Parsing requires a four-digit year
and retains month or day precision; `YYYY-MM` denotes the first day of the month.
*Avoid*: `NaiveDate`, calendar value

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
Date-Time values, and apply calendar units.

#### Unit

One of the eight accepted time units. Each unit defines its accepted spellings,
nominal length, application regime, and human-readable names in the shared
registry.
