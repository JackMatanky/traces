# N1: Conflicts Resolved Tickets: Stress Test and Verdict

This report stress-tests decisions Q1–Q15 against resolved tickets 01–21.

## Executive Summary

This document maps candidate decisions from `m-decision-candidates.md` (Q1–Q15) against the technical resolved tickets (01–21). It surfaces conflicts, gaps, and refinements required before finalizing the Answer for ticket 22.

## Methodology

- **Classification**: Findings are classified as *Hard Conflict* (direct contradiction), *Soft Conflict* (mismatch of intent/direction), *Gap* (missing definition/boundary), or *Fine* (aligned).
- **Citations**: All findings reference specific tickets/ADRs via line-number anchored quotes.

## Targeted Hunts

- **Ticket 18/19/20**: `FieldName`/`FieldValue` (runtime vs schema).
- **Ticket 20**: Layering principle (config obligations).
- **Ticket 21**: Ambient environment/context, freshness, and global config.
- **Ticket 16**: Indexing/trust boundary and visibility.

## Stress Test Matrix (Q1–Q15 vs Tickets 01–21)

| Decision | Ticket(s) | Status | Finding |
| :--- | :--- | :--- | :--- |
| **Q1** | 23 | Gap | `note` config type definition needs sync with ticket 23 status model. |
| **Q2** | 18 | Fine | `FieldName` as a leaf remains consistent. |
| **Q3** | 21 | Fine | `HostInputs` projection aligns with ticket 21 context scoping. |
| **Q5** | 16 | Hard Conflict | Ticket 16 requires specific index result equivalence which conflicts with the "two stores" design (Q5). |
| **Q8** | ADR-0009 | Fine | `IndexPolicy` approach aligns with ADR-0009-index-refresh. |

## Detailed Analysis of Conflicts

### Q5 vs Ticket 16 (Hard Conflict)

- **Ticket 16 context**: "frozen = vault-format parsing, index result equivalence with fresh parse..."
- **Q5 proposal**: Keep two stores (trust keyed by canonical ROOT; tracker keyed by config path).
- **Conflict**: The requirement for index result equivalence (Ticket 16) is harder to guarantee with separate tracker/trust stores (Q5).

## Alternatives Analysis

1. **Trust/Tracker Stores**
   - *Option A*: Single unified store (per root).
   - *Option B*: Decoupled (as per Q5).

## Verdict Table

[To be completed]

## Summary of Findings (Last 10 lines)

[To be completed]
