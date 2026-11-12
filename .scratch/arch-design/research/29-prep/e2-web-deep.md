# Deepened web evidence for ticket 29

The shared round-2 file lives at research/32-prep/e2-web-deep.md. Sections
that load this ticket:

1. redb MVCC and read-transaction pinning: a read transaction pins every
   page it references until it completes. The plan runner's settled
   "one read transaction per cold query" rule is a file-growth constraint,
   not only a consistency rule. Any design that runs a query in several
   transactions pays reclaim pressure.
2. redb commit strategies: writes cost one `fsync` at the default
   durability. This bounds the cost side of the range-key table
   decision. The probe (research/29-prep/g1-probes.md) measured the
   table size and scan costs; this source explains what the commit
   itself guarantees.
3. Numeric range keys are measured now: `rating >= 4.5` range scans
   take 17 µs @5k and 171 µs @50k, with path order falling out of the
   key tail. The key uses an order-preserving f64 transform: sign-bit
   set for positives, all bits inverted for negatives. This is a
   measured candidate for the ticket 24 open question "exact range-key
   encoding + costs". Details and the table in g1-probes.md.

No query-language-specific new claims beyond these pointers. The first
pass already covered DataFusion pushdown, Polars predicate pushdown,
Dataview, Tasks, SQLite `whereSplit`, and SQLite4 key encoding.
