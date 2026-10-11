# Date

Date and Date-Time parsing, local-zone resolution, and calendar arithmetic.

## Language

### Temporal Values

<!-- rumdl-disable-next-line MD024 -->
#### Date

A parsed civil calendar date with no time-of-day component. `src/date/` is the
crate's single owner of date recognition, parsing, and formatting; every
date-shaped string — including serde deserialization — funnels through it.
Enforces the four-digit year invariant (rejecting two-digit ambiguous years) and
retains month/day precision on date points, reducing `YYYY-MM` to day 1.
*Avoid*: NaiveDate, calendar value

#### Date-Time

A parsed date-time point, stored as UTC and rendered as the reader's local wall
clock. Shares `src/date/`'s parser with Date, where naive input represents the
local zone, resolved to UTC at parse time. DST ambiguities resolve to the
earliest occurrence (RFC 5545 / Temporal `'compatible'`), spring-forward gaps
advance across the gap, and zone lookup failures surface as explicit errors.
Comparing across Date and Date-Time promotes the Date at local midnight, with a
declared exception falling back to the neutral UTC frame if zone lookup fails.
*Avoid*: Timestamp, instant

#### Calendar Owner & Regime

`src/date/` is the single calendar owner for shift, diff, and duration
application; query and template engines are callers over it, owning no duplicate
calendar arithmetic. Operates under a dual regime: identity equals total magnitude
in seconds, while application treats day, week, month, and year as calendar units
applied left-to-right on the local wall clock, and sub-day units as exact duration
seconds. Consequently, equal duration values (such as `1d` and `24h`) may shift
dates differently across calendar boundaries and DST transitions.
*Avoid*: calendar engine, custom date math
