---
number: 9
title: Filter date literals
status: proposed
date: 2026-10-11
tags:
- query
- grammar
- date
---

# Filter date literals

## Context and Problem Statement

`--where 'due == 2026-01-01'` lexed as `due == 2026 - 01 - 01`: the identifier
token excludes `-`, so the date split into number literals and subtraction.
Number-number arithmetic evaluates to `Null`, and a note without `due` also
reads as `Null`, so the filter returned exactly the notes that had no `due`.

## Considered Options

- Quote-only: leave the lexer alone and reject number-only arithmetic loudly.
- Lex bare ISO dates as one literal and also reject number-only arithmetic.

## Decision Outcome

Chosen option: "Lex bare ISO dates as one literal and also reject number-only
arithmetic", because users naturally write unquoted dates and the quoted and
unquoted forms must mean the same thing.

A bare `YYYY-MM[-DD[THH:MM[:SS[.f]]][Z|+hh:mm]]` lexes as one literal and is
promoted to a Date or DateTime exactly like its quoted form (an invalid shape
such as `2026-13-45` stays a string, as when quoted). Arithmetic whose operands
are all number literals is a query-build error, so shapes the lexer does not
claim (`2026-1-1`) fail loudly instead of matching missing fields.

### Consequences

- Good, because `due == 2026-01-01` and `due == "2026-01-01"` agree.
- Good, because number-only arithmetic can no longer silently become `Null`.
- Bad, because `1 + 2 == 3` is now a build error rather than a `Null` compare.

### Confirmation

Unit tests in `src/query/grammar/filter.rs` and
`filters_pages_using_unquoted_iso_date_literals` in `tests/integration/query.rs`.
