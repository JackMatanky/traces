# Noyalib CST Performance & Quote-Style Preservation Investigation

- **Target File:** `.scratch/arch-design/research/20-prep/round5/N-noyalib.md`
- **Scope:** Review pinned `noyalib` (0.0.53) and upstream (0.0.57) for CST /
  quote-style preservation APIs, track down the provenance of the "+59%" claim,
  inventory upstream benchmarks, identify measurement gaps, and propose a
  minimal local benchmark design.
- **Status:** Complete.

---

## 1. Provenance of the "+59%" Claim

### 1.1 Origin and Evidence Citation

The claim that preserving quote style or parsing CST incurs a "+59% latency
penalty" appears repeatedly across the codebase research corpus:

1. `.scratch/arch-design/research/19-prep/h-quote-escape-feasibility.md:13,57-73`
   [V: file read]
2. `.scratch/arch-design/issues/18-value-model.md:218-223` [V: file read]
3. `.scratch/arch-design/research/20-prep/h-arch-design-note-split-review.md:22,37`
   [V: file read]
4. `.scratch/arch-design/research/20-prep/round3/D-text-vs-typed.md:301-304,401-403`
   [V: file read]

The exact source and evidentiary basis for the numbers is located in
`.scratch/arch-design/research/19-prep/h-quote-escape-feasibility.md:57-73`
("2.4 Performance Measurement"):

> "Empirical timing on realistic 30-line frontmatter (10,000 iterations,
> release mode, Apple Silicon) [V: probe]:
>
> ```text
> --- Frontmatter WITH date/duration scalars (~85% of real notes) ---
> Baseline (noyalib::Value only):  35.45 µs/note  (1.00x)
> CST always (parse_document):    56.27 µs/note  (1.59x baseline, +20.8 µs)
> Lazy CST (consulted on hit):    91.97 µs/note  (2.59x baseline, +56.5 µs)
>
> --- Frontmatter WITHOUT date/duration scalars (~15% of real notes) ---
> Baseline (noyalib::Value only):  34.20 µs/note  (1.00x)
> CST always (parse_document):    54.26 µs/note  (1.59x baseline, +20.1 µs)
> Lazy CST (skipped on hit):      34.69 µs/note  (1.01x baseline, +0.49 µs)
> ```
>
> Key Finding: `parse_document` parses YAML twice internally (`parse_exactly_one`
>
> - `build_green_tree`, `cst/builder.rs:41-44`) [V]. The 'Lazy CST' strategy
> parses the document with `from_str_with_config`, scans `Value`, and then
> parses again with `parse_document`."

### 1.2 Verification Status: [V] Internal Throwaway Probe, [S] for Noyalib Upstream

- The +59% claim is **NOT** an upstream published benchmark figure from the
  Noyalib repository [V: search across upstream benches and docs]. Upstream
  does not publish a "+59%" figure.
- The claim is a **local empirical measurement** run on an Apple Silicon host
  during Round 19 preparation using a throwaway test probe that evaluated
  `parse_document` vs `from_str_with_config::<noyalib::Value>` over a 30-line
  frontmatter fixture (`h-quote-escape-feasibility.md:58` [V]).
- The ratio $\frac{56.27}{35.45} \approx 1.5873$ (+58.73%, rounded to +59%).
- The mechanical reason is verified directly in Noyalib crate source:
  `noyalib::cst::builder::parse_full` executes both
  `crate::parser::parse_exactly_one(input, cfg)` and
  `build_green_tree(&source)` (`src/cst/builder.rs:41-44` [V]).

---

## 2. Review of Pinned Noyalib (0.0.53 & 0.0.57)

- **Pinned Version:** `noyalib = "0.0.53"` with features `["fast-float",
  "fast-int", "std"]` (`Cargo.toml:132`, `Cargo.lock:1073-1076` [V]).
- **Tooling Inspection:** `xd://mcp__rust_docs_structure` succeeded on both
  `0.0.53` and `0.0.57` [V]. `cache_crate` failed due to a missing source path
  in `.rust-docs/cache/` for 0.0.53, so crate source was read directly from
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/noyalib-0.0.53/` [V].

### 2.1 CST and Round-Trip API

Noyalib provides `noyalib::cst`:

- `Document`: Byte-faithful document wrapper. Calling `doc.to_string()`
  reproduces the input byte-for-byte [V: `src/cst/mod.rs:8-12`].
- Read access: `doc.as_value()` yields `&noyalib::Value`; `doc.span_at(path)`
  yields `Option<(usize, usize)>` representing the byte offset range of the
  value [V: `src/cst/document.rs:374`]; `doc.key_span(path)` yields the byte
  offset range of the mapping entry's key token [V: `src/cst/document.rs:405`];
  `doc.syntax()` yields the raw `GreenNode` [V: `src/cst/mod.rs:26`].
- Surgical edits: `doc.set(path, value)` or `doc.replace_span(s, e, fragment)`
  splices edits and re-validates the CST [V: `src/cst/document.rs:479-569`].

### 2.2 Quote-Style Preservation Capabilities

1. **`noyalib::Value` AST:** Drops all quote styles. `Value::String(String)`
   holds unescaped string contents only [V: `src/value/mod.rs`]. No quote
   delimiter metadata exists on `Value`.
2. **`cst::SyntaxKind`:** Defines `PlainScalar`, `SingleQuotedScalar`,
   `DoubleQuotedScalar`, `LiteralScalar`, `FoldedScalar`
   [V: `src/cst/syntax.rs:68-75`].
3. **Per-Scalar Querying in 0.0.53:** No public function exists on `Document`
   or `Value` to return `ScalarStyle` for a given field path. The caller must
   call `doc.span_at(path)`, index into `doc.source()[start..end]`, and inspect
   the boundary delimiter bytes (`"` or `'`) [V: `h-quote-escape-feasibility.md:31-50`].
4. **Flaws of `doc.span_at` for Quote Inspection:**
   - On tagged scalars (`date: !!str "2025-01-15"`), `span_at` returns the span
     covering the tag and scalar (`!!str "2025-01-15"`), causing naive prefix
     checks (`starts_with('"')`) to fail [V: `src/cst/path.rs:396-401`].
   - Keys with dots require synthetic bracket paths (`["a.b"]`) [V].
   - Aliases (`*x`) resolve to the anchor target's span [V: `src/cst/document.rs:634`].
5. **Additions in 0.0.57:**
   - Adds `Document::dominant_quote_style()` [V: `src/cst/emit.rs:72`]. This is
     a file-wide aggregate heuristic, not a per-scalar query API.
   - Adds `EmitCtx::quote_style()` [V: `src/cst/emit.rs:120`], used when
     formatting newly inserted entries.
   - `ParserConfig::plain_scalar_strings` [V: `src/de/config.rs:1284`] controls
     whether plain scalars can deserialize into strings.
   - **Still NO public API** exists to inspect individual scalar quote styles
     during `Value` parsing or deserialization.

---

## 3. Upstream Benchmarks Inventory

Upstream repository: `https://github.com/sebastienrousseau/noyalib`
[V: `Cargo.toml:52`]. The `benches/` directory contains 17 benchmark suites
[V: directory listing]:

| Benchmark File | Focus / Targets Measured |
|:---|:---|
| `benches/architecture.rs` | Compares zero-copy deserialization, AST (`Value`) vs streaming, and `bench_span_overhead` comparing `no_spans/from_str`, `with_spans/from_str_with_config`, `no_spans/value_from_slice`, and `with_spans/value_from_str_config`. Security benchmarks (Billion Laughs DoS limits). |
| `benches/benchmarks.rs` | Core throughput: `from_str`, `from_str_with_config` (default vs strict), `to_string`, large mappings, deeply nested collections, duplicate keys. |
| `benches/comparison.rs` | Head-to-head comparison against other crates: `serde_yaml_ng`, `yaml-rust2`, `serde-saphyr`. |
| `benches/incremental_repair.rs` | CST surgical edit cost: compares `phase_a_set` (`Document::set` with localized green-tree splice) against `baseline_full_reparse` (`parse_document(new_source)` full re-parse). |
| `benches/numeric_parse.rs` | SWAR decimal parsing vs standard library parser. |
| `benches/structural_bitmask.rs` | SIMD structural byte bitmasking. |
| `benches/streaming_vs_value.rs` | `StreamingDeserializer` vs `Value` AST deserialization. |
| `benches/validation_overhead.rs` | JSON Schema 2020-12 validation and schema coercion overhead. |
| `benches/large_doc_soak.rs` | Multi-megabyte document parsing soak test. |
| `benches/borrowed_vs_value.rs` | `BorrowedValue` vs owned `Value`. |
| `benches/lossless_u64.rs` | `lossless_u64_integers` configuration overhead. |
| `benches/mapping_key_clone.rs` | Key string deduplication/cloning in mapping loads. |
| `benches/parallel.rs` | Rayon-based multi-document stream parsing. |
| `benches/simd.rs` | SIMD vector scanning optimizations. |
| `benches/v006_features.rs` | Lenient recovery parsing vs strict parsing throughput. |
| `benches/interner.rs` | String interning microbenchmarks. |

### Upstream CI & Published Numbers

- Published documentation (`docs/BENCHMARKS.md`, README.md) focuses on
  **deserialization throughput** (claiming 1.69×–2.00× faster than
  `serde-saphyr`) and **serialization speed** (3×–4× faster than forks).
- `benches/incremental_repair.rs` measures `Document::set` mutation latency on
  synthetic mapping documents (e.g. 100 to 10,000 entries).
- `benches/architecture.rs` (`bench_span_overhead`) measures the cost of
  populating `SpanTree` for `Spanned<T>` during `from_str_with_config`, but
  does **not** run `parse_document` or compare `Value` against `Document`.

---

## 4. Gap Analysis & Minimal Local Benchmark Design

### 4.1 What Upstream Answers vs What Remains Open

- **Answered Upstream:**
  1. Populating `SpanTree` alone inside `from_str_with_config` adds a small
     overhead over `from_str` (`benches/architecture.rs:211-247` [V]).
  2. Modifying an existing `Document` via `Document::set` is faster than
     re-parsing the entire document (`benches/incremental_repair.rs:57-96` [V]).
- **The Gap (Unanswered Upstream):**
  Upstream has **no benchmark** measuring the initial parse cost of
  `noyalib::cst::parse_document` (CST + GreenTree + `SpanTree` + `Value`)
  versus `noyalib::from_str_with_config::<noyalib::Value>` on typical note
  frontmatter (10–40 lines of YAML mapping containing strings, dates, tags,
  and lists).
  Consequently, upstream benchmarks do not answer:
  1. What is the exact CPU / allocation overhead of `parse_document` over
     `from_str` on note frontmatter?
  2. What is the cost of slicing `doc.source()[span]` to sniff quotes for
     every scalar field in a note?

### 4.2 Minimal Local Benchmark Design (Proposal Only)

A self-contained criterion or micro-benchmark design in `traces`
(e.g. `benches/frontmatter_cst.rs`):

```rust
// Design: benches/frontmatter_cst.rs (Design only - DO NOT RUN)
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const TYPICAL_FRONTMATTER: &str = r#"title: "Deep Work Daily Review"
date: 2026-03-30
due: "2026-04-01"
tags:
  - work
  - "deep-focus"
status: in-progress
rating: 5
estimated_duration: 1h30m
"#;

pub fn bench_frontmatter_cst_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("frontmatter_parsing");

    // 1. Status Quo: parse noyalib::Value AST without CST
    group.bench_function("ast_value_only", |b| {
        let cfg = noyalib::ParserConfig::serde_yaml_compat();
        b.iter(|| {
            let v: noyalib::Value = noyalib::from_str_with_config(
                black_box(TYPICAL_FRONTMATTER),
                &cfg,
            )
            .unwrap();
            black_box(v)
        });
    });

    // 2. CST Eager: full parse_document
    group.bench_function("cst_parse_document", |b| {
        let cfg = noyalib::ParserConfig::serde_yaml_compat();
        b.iter(|| {
            let doc = noyalib::cst::parse_document_with_config(
                black_box(TYPICAL_FRONTMATTER),
                &cfg,
            )
            .unwrap();
            black_box(doc)
        });
    });

    // 3. CST + Quote Sniffing: parse_document + span_at inspection per field
    group.bench_function("cst_with_quote_sniffing", |b| {
        let cfg = noyalib::ParserConfig::serde_yaml_compat();
        b.iter(|| {
            let doc = noyalib::cst::parse_document_with_config(
                black_box(TYPICAL_FRONTMATTER),
                &cfg,
            )
            .unwrap();
            // Inspect spans of candidate date/string fields
            let is_quoted = |path: &str| -> bool {
                doc.span_at(path).is_some_and(|(s, e)| {
                    let slice = &doc.source()[s..e];
                    slice.starts_with('"') || slice.starts_with('\'')
                })
            };
            let q1 = is_quoted("date");
            let q2 = is_quoted("due");
            let q3 = is_quoted("title");
            black_box((doc, q1, q2, q3))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_frontmatter_cst_cost);
criterion_main!(benches);
```

### 4.3 Summary of Findings for Ticket 20 & Architecture Design

- The +59% claim originates from a local throwaway benchmark on Apple Silicon
  measuring `parse_document` (56.27 µs) vs `from_str_with_config` (35.45 µs).
  The mechanism is verified: `parse_document` runs two full passes
  (`parse_exactly_one` + `build_green_tree`).
- Noyalib does not expose per-scalar quote styles in its typed `Value` AST.
- Using Noyalib's CST for quote-style detection would mandate parsing every note
  with `parse_document`, incurring ~1.59× parse latency, and would remain
  mechanically brittle for dotted keys, YAML tags (`!!str`), and block scalars.
- The architectural decision in Tickets 18 and 19 to reject CST-based
  quote sniffing in favor of quote-insensitive parse-time heuristic
  classification plus schema overrides is solidly grounded in Noyalib's API
  limitations and mechanics.
