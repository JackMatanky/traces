# YAML spans, interning and persistence crates

I replaced the first version. Its recommendations (`compact_str`, `saphyr`) had
no evidence. This version keeps only what I read or measured.

## Summary

- [V] The repository uses `noyalib` 0.0.53, `postcard` 1.1.3, `redb` 4.3.0,
  `indexmap` 2.14.2 and `logos` 0.16.1 (`Cargo.toml:125-138`).
- [V] The repository has no interning crate and no `smallvec`.
- [V] `noyalib::Spanned<T>` holds `value`, `start` and `end`
  (`noyalib-0.0.53/src/spanned.rs:94-125`). It works through a thread-local span
  context (`noyalib-0.0.53/src/lib.rs:246-249`).
- [V] `Spanned<T>` gives the span of a value. A map key is a plain `String` in
  the probe. A key span needs another method [H].
- [V] A `Spanned<Value>` parse is 2.8x to 3.3x slower than a plain parse. See
  `g-probe-results.md`.
- [V] The end offset of a `Spanned` value includes trailing text up to the next
  token (`g-probe-results.md`).
- [V] A line scan for top-level keys costs 0.5% of the full parse time on the
  175 frontmatter files of the corpus (`g-probe-results.md`).

## Frontmatter span options

| Option | Key span | Value span | Cost | Source |
|--------|----------|-----------|------|--------|
| Keep the span-free parse and add a line scan | yes, top level only | no | 0.5% of parse time | probe |
| `Spanned<Value>` | not tested | yes, loose end | +181% to +225% of the YAML parse | probe |
| Noyalib CST | rejected by ticket 18 | yes | +59% parse time | ticket 18 |

- [H] A line scan can miss quoted keys, complex keys and flow mappings.
- [H] A line scan gives no span for a list element on a later line. The probe
  gives list element spans with `Vec<Spanned<String>>`: `work` 10..17 and
  `project/alpha` 19..33.

## Not checked

- `lasso`, `string-interner`, `ustr`, `smol_str`, `compact_str`, `ecow`,
  `smallvec` and `thin-vec`.
- `rkyv`, `bitcode` and the borrowed value types of `redb`.
- `saphyr`, `yaml-rust2` and `marked-yaml`.
- Persistence of an interned symbol id across runs.

## Inconsistencies found while rewriting

- The first version recommended `saphyr` "due to proposed performance savings".
  It gave no measurement and no source.
- The first version recommended `compact_str` for small strings without a check
  of the status use case. A task status is one `char` in the map today
  (`src/task.rs:370-371`). A `char` symbol needs no string crate.
- The first version lists `Lasso`, `String-Interner` and `SmallVec` with
  statements such as "focus on durability" that no source supports.
- The first version ties "C12" to YAML spanning and "C3" to memory models. The
  brief assigns C3 to frontmatter spans and C12 to persistence.
