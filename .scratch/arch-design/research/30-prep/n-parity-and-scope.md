# Speedate parity and replacement scope

## Evidence and conclusion

[V] This report checks source and existing assertions. It runs no tests or
probes. [V] marks inspected source or preserved observations. [P] marks a
proposed gate. The report does not resolve ticket 30 or its human choices.

[V] Datetime, duration, and zone differential comparisons are not missing
speedate-removal comparisons. The production tree has one speedate call:
`DateShape::parse_year_month_day`, `src/temporal/date.rs:353-399`. It calls
`speedate::Date::parse_bytes_rfc3339` at line 370. The call handles only the
unsigned, four-digit-year, full-date branch. The manifest declares speedate
at `Cargo.toml:143`. No production code calls speedate's datetime or duration
APIs. A search of `src` supplies this inventory; CodeGraph supplies the
function body and its callers.

[V] This conclusion is narrower than general temporal parity. Ticket 30's
civil-datetime carrier, duration boundary, and frame changes still need their
own acceptance evidence. The existing field probe does not establish those
contracts. It also does not execute repository wrappers. Calling these
limits uncovered *speedate replacement paths* confuses two different changes.

## Exact replacement boundary

[V] `DateShape::parse_year_month_day` first selects the signed or unsigned
length. It returns the existing short-input, trailing-input, and wrong-length
reasons before it reaches speedate (`date.rs:353-368`). Speedate returns
fields. The wrapper constructs `jiff::civil::Date` from those fields
(`date.rs:370-386`). The signed branch already parses fields directly and
constructs `civil::Date` (`date.rs:388-399`).

[V] The eligible replacement is the unsigned field branch, not the whole
parser. Keep its outer checks, error mapping, and final civil value.
`DateShape::parse_year_month` remains unchanged. It accepts signed or unsigned
year-month input and supplies day 1 (`date.rs:418-439`).

[V] The existing candidate in `probes/parity/src/main.rs:16-31` checks ten
bytes, ASCII digits, and separators. It constructs `jiff::civil::Date::new`
from numeric fields. It does not call Jiff's unrestricted date parser.
The latter accepts a broader grammar, including compact dates and datetime
text; the preserved examples are in `m-gap-closure.md:39-59`.

[V] The recorded probe checks 4,635,370 inputs. It reports zero acceptance,
value, and mapped-error differences (`m-gap-closure.md:28-59`). Its numeric
matrix uses years 0000 through 9999, months 00 through 13, and days 00 through
32. It also mutates bytes and supplies malformed shapes. This is bounded
field-branch evidence, not exhaustive coverage of all byte strings.

## Frozen contract and returned diagnostics

[V] `DateValue::parse_iso` trims whitespace, checks the four-digit ASCII year,
and calls `DateShape::parse_any` (`date.rs:87-100`). `FromStr` delegates to
that parser (`date.rs:198-200`). `DateShape::parse_any` tries full date and
then year-month. It returns the month reason only for seven-byte input;
otherwise it preserves the full-date reason (`date.rs:291-309`).

[V] Existing assertions freeze these representative results:

- Leap day, year zero, negative years: `parses_supported_full_date_years`,
  `date.rs:700-711`. Constructor boundary assertions also check -9999 and
  9999, display, and reparsing (`date.rs:586-602`).
- `2026-02-29`, `2026-13-01`, and `2026-01-00` return
  `DateError::InvalidValue` with `a field is out of range`.
  `2026-07-2x` returns `invalid input`
  (`rejects_hostile_full_dates_with_existing_reason`, `date.rs:713-728`).
- Unpadded dates return `input is too short` (`date.rs:731-747`).
  Year-month defaults to day 1 (`date.rs:749-760`).
- Two-digit years return `YearNotFourDigits` (`date.rs:762-775`).
  Five-digit years return `InvalidValue` with `invalid input`
  (`date.rs:777-792`). Non-ASCII years return `YearNotFourDigits`
  (`date.rs:845-858`).
- Datetime text passed to `DateValue` returns `InvalidValue` with
  `unexpected trailing input` (`date.rs:810-826`). Blank input returns
  `YearNotFourDigits` with the empty trimmed input (`date.rs:828-843`).
- Surrounding whitespace is accepted (`date.rs:860-873`). The seven-byte
  `2026-ab` case selects `not enough input`; `2026-7` selects
  `input is too short` (`date.rs:875-887`, case declarations).

[V] `DateError` retains the input and reason, not an error byte span
(`src/temporal/error.rs:37-57`). The guarded probe compares the inner reason
category. It does not compare this outer enum or its retained input.

## Prefix behavior and shared consumers

[V] The date prefix scanner asks Logos for one token. It validates that
complete token once (`src/temporal/lexer.rs:93-111`). An invalid full-date
token does not make the scanner try a second, shorter token. If Logos
recognizes only a seven-byte month followed by `-`, the scanner rejects it at lines
105-106. Thus `2026-02-30` cannot become `2026-02`, and `2026-02-x` cannot
become `2026-02`. This is source-backed behavior, not a new runtime result.

[V] Do not generalize this rule into a ban on intentional prefix parsing.
`DatePoint::parse_prefix` returns a spanned date prefix
(`src/temporal/point.rs:146-153`). `NoteFieldValue::as_date` deliberately
uses it for strings (`src/note/field.rs:109-115`). Inline date atoms add an
end, whitespace, or comma boundary (`src/note/parser/inline.rs:220-232,
266-279`). A `T` suffix fails that boundary. Task emoji parsing rejects an
alphanumeric suffix (`src/note/parser/lexer.rs:238-267`). These distinct
consumer boundaries remain unchanged by a field-branch replacement.

[V] The shared chain is `scan_date_prefix` to `DateShape::parse_any` to the
same field branch. Inline atoms store `NoteFieldValue::Date`. Task emoji
callbacks store `TaskDate`. Task field fallback calls
`TaskScan::first_date_in_field`, which uses `NoteFieldValue::as_date`
(`src/note/parser/task.rs:58-88,209-217`). Thus the speedate branch affects
shared date validation, not only a standalone DateValue entry point.

[V] Existing prefix assertions check positive and negative full dates,
month prefixes, leading whitespace, and consumed lengths
(`src/temporal/lexer.rs:157-179`). Existing task assertions check all six
lifecycle dates from supported syntax
(`extracts_all_lifecycle_dates_from_supported_syntax`,
`src/note/parser/task.rs:1218-1230`). The assertion
`preserves_unparsed_invalid_dates` checks that `2026-13-45` remains in task
text (`src/note/parser/task.rs:481-486`). These are assertion-backed
regression cases, not proof that a future replacement already passes.

## Unchanged datetime, duration, zone, and cache paths

[V] `DateTimeValue::parse_iso` calls `DateTimeShape::parse_any` and then
`DateTimeInput::resolve` (`src/temporal/datetime.rs:176-190`). The shape
parser validates its own date prefix and grammar. It uses
`DATETIME_PARSER.parse_pieces`, not `DateShape::parse_any`
(`datetime.rs:463-510`). Offset input becomes an absolute timestamp.
Naive input becomes `DateTimeInput::LocalClock`. The speedate branch is not
part of this successful datetime parse chain.

[V] `DatePoint::parse` sends short input to `DateShape`. A fully matched
datetime goes directly to `DateTimeShape` and resolution
(`src/temporal/point.rs:82-117`). Malformed longer input can reach
`DateShape` for its final rejection reason (`point.rs:118-143`). Those
outer date length checks reject extended text before the speedate call.
Do not replace this dispatch with unrestricted Jiff date parsing.

[V] `DurationValue::parse` uses `jiff::Span` for ISO and normalized friendly
input. It validates friendly syntax and nominal range itself
(`src/temporal/duration.rs:244-278`). The duration prefix scanner uses
Winnow to locate the candidate, then validates the complete candidate
(`src/temporal/lexer.rs:118-147`). It has no speedate dependency.

[V] Local datetime resolution uses `local_zone` and `TimeZone::to_timestamp`
(`src/temporal/datetime.rs:137-159`). Date-to-midnight conversion constructs
a civil midnight directly (`datetime.rs:360-365`). Removing the unsigned
date field parser changes neither operation. Zone comparisons remain
necessary for the frame redesign, not for removing this speedate call.

[V] Date serialization emits its canonical string. Deserialization calls
`DateValue::parse_iso` (`src/temporal/date.rs:225-258`). Thus cached Date
reads reach the replacement branch; cached Date writes do not change.
`NoteFieldValue` derives serde and contains separate Date, DateTime, and
Duration variants (`src/note/field.rs:42-64`). The row codec calls
`postcard::to_extend` and `postcard::from_bytes`
(`src/index/codec.rs:26-54`). There is no independent cache date parser.

[V] Preserve the existing serde coverage review rather than claim new
inspection of every assertion. `j-test-coverage-review.md:60-78` records
exact Date JSON/postcard fixtures, instant fixtures, and the row property
test's Date-only temporal generator. Its lines 96-112 distinguish nominal
duration equality from field-preserving coverage. The absent datetime and
duration row generators remain coverage limits for the broader redesign.
They do not expose more speedate calls or replacement algorithms.

## Decision-relevant remaining work

[V] The narrow speedate replacement needs no datetime, duration or zone
differential experiment.
The preserved probe and inspected call graph support that limited judgment.
They do not justify a whole-parser replacement, a grammar expansion, or a
claim of complete temporal parity.

[P] A production change must retain wrapper assertions and exact diagnostic
categories. It must also retain prefix boundaries and Date serde fixtures.
This is an implementation gate, not a missing pre-grill upstream comparison.
Use the current guarded candidate. Do not compare speedate's unused datetime
or duration APIs with Jiff APIs.

[P] If the parent needs to reproduce existing field evidence, run this from
`.scratch/arch-design/research/30-prep/probes/parity`:

```sh
cargo +nightly-2026-10-01 run --release --locked
```

The acceptance criteria are zero acceptance, value, and error-category
differences. The count must remain 4,635,370. This command does not establish
wrapper, cache, datetime, duration, or zone parity. No new experiment is
requested for unchanged paths. The broader carrier and frame acceptance
work remains separate and must not inherit this limited closure.
