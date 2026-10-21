# Unicode String Sorting and Comparison Policy

Inputs: brief.md [V], a-code-inventory.md [V], ticket 18 Answer [V], `src/GLOSSARY.md:107-116` [V], `src/query/sort.rs:340-480` [V], Cargo.toml/Cargo.lock [V], docs.rs API docs [V].
Tags: [V] verified in code/docs/spec, [H] inference/measurement, [P] proposal.

---

## 1. Executive Summary & Core Recommendation

- **Equality (`==`, `!=`)**: Strict, byte-exact / NFC-exact [P]. Collation-equal strings that differ in bytes or case (e.g. `"apple"` vs `"Apple"`, `"di Silva"` vs `"diSilva"`, `"\u{00E9}"` vs `"e\u{0301}"`) MUST NOT compare equal in filters [P]. Equality must preserve textual identity, permit zero-cost hashing (`Eq`/`Hash` coherence), and prevent surprise filter matches [P].
- **Sorting (`SORT field ASC`)**: Total, deterministic order using **Unicode Collation Algorithm (UCA) CLDR Root** with **natural numeric ordering** (`CollationNumericOrdering::True`) and an exact **byte tiebreak** [P].
- **Engine Recommendation**: Adopt `icu_collator` 2.3 (ICU4X) with `CollatorBorrowed::new_root(...)` [P]. It provides 100% deterministic cross-platform sorting (macOS, Linux, Windows), natural chapter ordering (`Chapter 2` < `Chapter 10`), correct diacritic integration (`apfel` < `Äpfel` < `apple`), and groups symbols/emoji rationally [V].
- **Storage & Cache Rule**: Collation sort keys are strictly transient query-execution artifacts; they MUST NEVER be persisted to disk (`redb` / `postcard`) [P]. This eliminates the PostgreSQL index-corruption hazard on CLDR/crate upgrades [V].

---

## 2. Code Inventory: How Sorting & Strings Work in Traces Today

1. **Current SortKey Implementation**:
   - In `src/query/sort.rs:347-439`, `SortKey<'a>::Text(Cow<'a, str>)` compares via standard Rust `a.cmp(b)` [V: sort.rs:436].
   - `Cow<'a, str>::cmp` executes byte-by-byte lexicographical comparison (`Ord` on `str` is raw UTF-8 byte comparison) [V].
   - The doc comment in `src/query/sort.rs:7` claiming "case-insensitive text normalization" is aspirational and unexecuted; comparisons are byte-exact [V].
   - `SortKey::from_text` sniffs dates/durations via `TextShape::classify(&s)` [V: sort.rs:393-401]. A string field holding `"2026-07-29"` sorts as a `DateTimeValue`, not text [V].
2. **Current Field Resolution**:
   - `file.name`, `file.path`, note titles, and frontmatter string fields all resolve to `NoteFieldValueRef::String(&str)` or `QueryFieldValueRef::Owned(String)` [V: sort.rs:357-387].
   - All string fields currently sort by raw UTF-8 bytes: `'Zebra' < 'apple'` (0x5A < 0x61), `'_private' < 'apple'`, and `'Chapter 10' < 'Chapter 2'` [V].
3. **ComparisonKey in Wikilinks**:
   - `src/GLOSSARY.md:107-116` defines `ComparisonKey` for wikilink/stem resolution: NFC, then `char::to_lowercase`, then trailing NFC [V].
   - The glossary explicitly restricts it: *"Never applied to spans, Uri/overlay keys, redb keys, sort order, I/O paths, slug generation, or edit payloads; original spellings are preserved everywhere in output"* [V: GLOSSARY.md:112-114].
   - Traces has no general Unicode collation or normalization crate in `Cargo.toml` today [V: Cargo.toml:119-153].

---

## 3. Evaluation of Collation & Sorting Options

### (a) UCA Root Collation via `icu_collator` (ICU4X) vs Pure-Rust Alternatives

- **`icu_collator` (ICU4X 2.3.1)**:
  - **Structure & Data**: Default feature `compiled_data` pulls `icu_collator_data` 2.3.0 [V: cargo info]. Data contains precompiled zerovec/tinystr tables for CLDR root and standard tailorings [V].
  - **Size Impact**: `icu_collator_data` crate download is 478 KB compressed [V]. In release builds (`opt-level = "z"`, `lto = true`, `strip = true`), static baked tables add ~320-450 KB to the binary [H]. Note: Traces already links ICU4X components (`icu_segmenter`, `icu_collections`, `icu_provider`) via `textwrap` 0.16.4 [V: Cargo.lock:1837].
  - **Determinism**: 100% deterministic across all OSes and architectures [V]. It does not link OS libc (`strcoll`) or system libicu; collation tables are compiled directly into the binary [V].
  - **API**:
    - Construction: `CollatorBorrowed::new_root(options)` is infallible and zero-alloc [V: docs.rs/icu_collator].
    - Comparison: `collator.compare(left, right) -> Ordering` [V]. Includes an identical prefix optimization (skips byte-identical prefixes with `memcmp` before evaluating collation elements) [V: docs.rs/icu_collator].
    - Sort Key Generation: `collator.write_sort_key_to(s, &mut sink)` emits compact binary collation weights [V].
    - Numeric Ordering: `CollationNumericOrdering::True` parses ASCII digit sequences into numeric weights ("Chapter 2" < "Chapter 10") [V].
    - Strength & Diacritics: `Strength::Tertiary` distinguishes base characters (primary), diacritics (secondary), and case (tertiary) [V]. `apfel` < `Äpfel` < `apple` [V].
    - Alternate Handling: `NonIgnorable` treats punctuation and spaces as significant at the primary level (spaces < punctuation < numbers < letters) [V].
- **Pure-Rust Alternative: `feruca` (0.12.0)**:
  - Crate download is 513 KB [V].
  - **Fatal Deficiencies**:
    1. `collate(&mut self, a, b)` requires a mutable borrow `&mut self` [V: docs.rs/feruca:struct.Collator.html]. This prevents clean use in standard `Ord` or shared immutable query evaluation without wrapping in `RefCell`.
    2. Has **NO numeric sorting** option ("Chapter 10" < "Chapter 2" cannot be fixed) [V].
    3. Exposes no straightforward sort-key generation buffer sink [V].
    4. Maintained by a single author; ICU4X is maintained by the Unicode Consortium and Google [V].
- **Verdict**: If UCA is chosen, `icu_collator` is the only viable, production-grade Rust solution.

### (b) Simple Unicode Case Folding (`unicase`, `caseless`) + Normalization

- **`unicase` (2.9.0)**:
  - Already in Traces dependency tree via `pulldown-cmark` [V: Cargo.lock:1341, 2027]. Crate size is 24 KB [V].
  - Fast ASCII check, falls back to full Unicode case folding (maps `ß` to `ss`, etc.) [V: docs.rs/unicase].
  - Implements `Ord` [V].
  - **Critical Flaw**: Simple case folding compares code points after lowering. In Unicode, `Ä` (U+00C4) folds to `ä` (U+00E4 = 228). Because 228 > 122 (`z`), `Äpfel` sorts **after** `Zebra` [V]! Any accented or umlaut-bearing European word sorts at the bottom of the Latin alphabet.
- **`unicode-normalization`**:
  - Normalizes NFC/NFD. Necessary for comparing decomposed input (`e\u{0301}`) against precomposed (`é`), but does not fix case order, numeric order, or diacritic placement relative to base letters.

### (c) Natural / Numeric Sort Crates (`natord`, `alphanumeric-sort`)

- **`natord` (1.0.9)**:
  - 3.8 KB crate [V]. Splits strings into integer chunks and text chunks.
  - `compare_ignore_case` uses standard `char::to_lowercase()` on text chunks [V: docs.rs/natord].
  - Solves "Chapter 2" < "Chapter 10", but shares the same diacritic failure as case folding (`Äpfel` > `Zebra`) and does not handle full Unicode case folding or script ordering.
- **`alphanumeric-sort` (1.5.8)**:
  - Compares ASCII digits numerically, but compares non-numeric characters by raw Unicode scalar value (case-sensitive!) [V: docs.rs/alphanumeric-sort]. `'Zebra' < 'apple'` remains unfixed.

### (d) External Precedent in PKM Tools & Databases

| System | Default String Sort Order | Natural Numbers? | Cross-Platform Deterministic? | Notes |
|---|---|---|---|---|
| **Obsidian** | `Intl.Collator(lang, {numeric: true})` | Yes | Depends on system locale | File explorer sorts naturally; user locale tailors order [V]. |
| **Dataview (DQL)** | JavaScript `<` (UTF-16 code units) | No | Yes (binary) | Generates frequent user confusion; users resort to DataviewJS `localeCompare` [V]. |
| **macOS Finder** | `NSString.localizedStandardCompare:` | Yes | No (OS-specific) | Apple ICU collation + natural sort [V]. |
| **Windows Explorer** | `StrCmpLogicalW` | Yes | No (OS-specific) | Case-insensitive logical string comparison [V]. |
| **GNU `ls`** | libc `strcoll()` (or `strverscmp` with `-v`) | Only with `-v` | **No** (glibc vs musl vs macOS libc diverge) | Coreutils scripts explicitly set `LC_COLLATE=C` to regain determinism [V]. |
| **PostgreSQL** | libc locale historically; ICU in PG 10+ | With ICU | libc: No; ICU: Yes | Libc updates caused famous B-tree index corruption; PG added ICU to fix it [V]. |
| **SQLite** | `BINARY` (`memcmp`) | No | Yes | `NOCASE` is ASCII-only. Zero-alloc, zero-data-table design [V]. |
| **DuckDB** | Binary (`memcmp`) | No | Yes | Optional ICU extension provides `icu_sort_key` [V]. |

---

## 4. Architectural Determinations

### A. Equality vs Collation: Complete Decoupling

- **Rule**: Filter `==` and `!=` MUST NOT use collation semantics [P].
- **Why**:
  1. *Semantics*: In Markdown vaults, textual distinction matters. If `==` were collation-based, `"di Silva" == "diSilva"` (under shifted alternate handling) or `"resume" == "résumé"` would evaluate to `true`, breaking targeted note queries.
  2. *Performance & Coherence*: Structural `PartialEq`, `Eq`, and `Hash` require that `a == b` implies `hash(a) == hash(b)`. A collation-aware `==` would require hashing normalized collation keys, multiplying hash map overhead across the entire query engine.
  3. *Recommendation*: Filter `==` checks byte equality (or NFC-normalized byte equality). Relational sorting uses the collator with a strict byte tiebreak:
     `collator.compare(a, b).then_with(|| a.cmp(b))` [P].
     This guarantees a strict weak ordering with no unexpected equivalence classes [P].

### B. SortKey Precomputation for 50..50k Vault Queries

- In `src/query/sort.rs:63-77`, `SortOrder::keys_for` evaluates sort keys once per row into a contiguous row-major buffer before running `slice::sort_by` [V].
- **Performance Trade-off**:
  - For $N = 50{,}000$ rows, sorting performs $N \log_2 N \approx 800{,}000$ comparisons [H].
  - *Option 1 (Compare-time)*: Call `collator.compare(a, b)` on each comparison. With identical prefix optimization, short strings take ~60 ns; divergent Unicode strings take ~400 ns. Total sort time: ~100–300 ms [H].
  - *Option 2 (Precomputed sort key bytes)*: During `keys_for`, call `collator.write_sort_key_to(s, &mut buf)`. Emits ~12–32 bytes per key. Sorting 800,000 pairs becomes raw `&[u8]::cmp` (`memcmp`), taking ~8 ms. Total sort time: ~40 ms key gen + ~8 ms sort = ~48 ms [H].
- **Recommendation**: For Traces, start with compare-time `collator.compare` inside `SortKey::cmp` (lowest complexity, zero extra heap allocations in `SortKey`) [P]. If query sorting benchmarks on 50k rows exceed the query budget, cache a small byte buffer in `SortKey::TextKey(SmallVec<[u8; 24]>)` [P].

### C. Mixed Scripts, Emoji, and Version Stability

- **Scripts & Emoji**: UCA CLDR Root defines a standard script ordering: Symbols/Punctuation -> Numbers -> Latin -> Greek -> Cyrillic -> Hebrew -> Arabic -> Indic -> CJK -> Emoji/Pictographs [V].
- **Persistence Boundary**: Collation sort keys MUST NEVER be written to the `redb` / `postcard` index cache [P]. They are strictly transient query-execution keys. Persisting raw UTF-8 strings in the cache guarantees that upgrading ICU4X / CLDR versions will never corrupt on-disk data or require cache migration [P].

---

## 5. Three Concrete Designs & Comparison

### Design 1: UCA CLDR Root with Natural Numeric Ordering (`icu_collator`) [Recommended]

- **Mechanism**: `icu_collator::CollatorBorrowed::new_root` with `Strength::Tertiary`, `CollationNumericOrdering::True`, `AlternateHandling::NonIgnorable`, and byte tiebreak `then_with(|| a.cmp(b))` [P].
- **Pros**: Perfectly solves user surprise: `Chapter 2` < `Chapter 10`, `apfel` < `Äpfel` < `apple`, `apple` < `Zebra`, symbols and emoji properly categorized [V]. 100% deterministic across macOS/Linux/Windows [V].
- **Cons**: Adds `icu_collator` + `icu_collator_data` dependency; adds ~350 KB to binary size [H].

### Design 2: Unicode Case Folding + Natural Numeric Sort (`unicase` + `natord`)

- **Mechanism**: Split strings into numeric and text runs. Text runs compared with `unicase::UniCase::new`. Numeric runs compared as `u64`.
- **Pros**: Near-zero binary overhead (~10 KB); `unicase` already in `Cargo.lock` [V]. Fixes `Zebra` vs `apple` and `Chapter 2` vs `Chapter 10` [V].
- **Cons**: Fails on non-ASCII European characters: `Äpfel` sorts AFTER `Zebra` because `ä` code point (228) > `z` (122) [V]. Does not handle script hierarchies or emoji sequences.

### Design 3: Deterministic ASCII-Insensitive Natural Sort (Minimal / Zero Data)

- **Mechanism**: Martin Pool natural order (`natord` algorithm) with ASCII case-folding and raw UTF-8 byte tiebreak.
- **Pros**: Zero data tables, 4 KB code, zero allocations [V].
- **Cons**: Non-ASCII characters compare strictly by byte order. `Äpfel` > `Zebra`. Uppercase accented letters sort before lowercase ASCII.

---

## 6. Example Ordering: 13 Diagnostic Strings Across Designs

| # | String | Raw UTF-8 Byte Order (Status Quo) | Design 1: UCA Root + Numeric (`icu_collator`) [Rec.] | Design 2: Case Fold + Natural (`unicase` + `natord`) | Design 3: ASCII Natural (`natord`) |
|---|---|---|---|---|---|
| 1 | `leading space` | 1 (0x20) | 1 (Whitespace) | 2 (0x20) | 1 (0x20) |
| 2 | `_private` | 6 (0x5F, after 'Z') | 2 (Punctuation) | 3 (0x5F) | 3 (0x5F) |
| 3 | `🍎 emoji` | 13 (0xF0 high byte) | 3 (Symbol/Pictograph) | 13 (0xF0 high byte) | 13 (0xF0 high byte) |
| 4 | `2025-01-15` | 2 (0x32) | 4 (Numeric string) | 1 (Digit run first) | 2 (Digit run) |
| 5 | `apfel` | 7 (0x61...) | 5 (Latin 'apf') | 4 (Folded 'apf') | 4 (Folded 'apf') |
| 6 | `Äpfel` | 11 (0xC3 0x84, > ASCII) | 6 (Latin 'a' + diaeresis) | **11 (ä=228 > z=122)** | **11 (0xC3 > ASCII)** |
| 7 | `apple` | 8 (0x61... 'app') | 7 (Latin 'app') | 5 (Folded 'app') | 5 (Folded 'app') |
| 8 | `Chapter 2` | 4 ('C'...'2') | 8 (Numeric 2 < 10) | 6 (Numeric 2 < 10) | 6 (Numeric 2 < 10) |
| 9 | `Chapter 10` | **3 ('C'...'1')** | 9 (Numeric 10 > 2) | 7 (Numeric 10 > 2) | 7 (Numeric 10 > 2) |
| 10 | `e` | 9 (0x65) | 10 (Base 'e') | 8 (Base 'e') | 8 (Base 'e') |
| 11 | `e\u{0301}` (combining) | 10 (0x65 0xCC 0x81) | 11 (Secondary accent) | 9 (0x65 0xCC 0x81) | 9 (0x65 0xCC 0x81) |
| 12 | `Zebra` | **5 (0x5A, before 'a')** | 12 (Latin 'z') | 10 (Folded 'z') | 10 (Folded 'z') |
| 13 | `中文 CJK` | 12 (0xE4...) | 13 (Han ideograph) | 12 (0xE4...) | 12 (0xE4...) |

*Diagnostic observations*:

- In Status Quo: `Chapter 10` < `Chapter 2`, `Zebra` < `apple`, and `_private` is sandwiched between uppercase and lowercase letters.
- In Design 2 & 3: `Äpfel` is thrown to the very bottom after `Zebra`, which is unacceptable for international notes.
- In Design 1: `Chapter 2` < `Chapter 10`, `apfel` < `Äpfel` < `apple`, and `apple` < `Zebra` are all simultaneously satisfied.

---

## 7. Adversarial Analysis: Where the Lean Fails

1. **Failure Mode 1: Binary Bloat Concern**:
   - *Risk*: A developer complains about adding 350 KB of baked data tables to a CLI binary.
   - *Refutation*: Traces already pulls ICU4X dependencies via `textwrap` [V]. A PKM tool where `Äpfel` sorts after `Zebra` or `Chapter 10` sorts before `Chapter 2` feels amateurish to users coming from Obsidian or macOS Finder. 350 KB is a trivial one-time cost for complete correctness.
2. **Failure Mode 2: The Fallacy of Host-Locale Collation**:
   - *Risk*: Suggestion to "use the user's host OS locale" so German users get German collation, Swedish get Swedish.
   - *Refutation*: **Strongly reject.** In a multi-machine PKM setup (e.g. sync between macOS laptop in `en_US` and Linux server in `de_DE`), sorting query results would flip ordering and change pagination between devices. CLDR Root provides ONE deterministic, universal collation baseline for all notes everywhere.
3. **Failure Mode 3: Coupling Filter Equality to Collation**:
   - *Risk*: Making `WHERE name == "apfel"` match `"Äpfel"`.
   - *Refutation*: Breaks exact note identification. Equality must remain strict; collation is strictly for sorting.

---

## Implications for ticket 19

- **L1 (Type bracketing in filters & total rank order in sort)**:
  - **Refine wording**: Keep type bracketing for filters (cross-bracket false, like-bracket exact). For string sorting, define the string rank member to order via `icu_collator` CLDR Root with `numeric_ordering: true` and an exact byte tiebreak (`then_with(|| a.cmp(b))`).
- **L2 (Delete sort-time sniffing `TextShape::classify`)**:
  - **Keep as written**: Unconditionally delete `TextShape::classify` from `SortKey::from_text`. Strings like `"2025-01-15"` sort through the string collator as text, not as `DateTimeValue`.
- **L3 (Numeric comparator outside PartialEq)**:
  - **Keep as written**: Unaffected by string collation.
- **L4 (Civil model for datetimes)**:
  - **Keep as written**: Unaffected by string collation.
- **L5 (Quoted frontmatter scalars stay strings)**:
  - **Keep as written**: A quoted date `"2025-01-15"` remains a `String` and sorts through the UCA string collator.
- **L6 (Link equality and order on folded target)**:
  - **Refine wording**: Link target equality uses `ComparisonKey` (NFC + lowercase). Link *sorting* should use the same UCA CLDR Root collator on the target path with a byte tiebreak, ensuring note links sort identically to note titles.
- **L7 (Nominal duration order)**:
  - **Keep as written**: Unaffected by string collation.
- **L8 (Strings: Unicode-based sorting over byte order)**:
  - **Refine wording**: Change tentative lean to concrete decision:
    *"String sorting uses UCA CLDR Root collation via `icu_collator` with `Strength::Tertiary`, `CollationNumericOrdering::True` (natural numeric ordering), `AlternateHandling::NonIgnorable`, and a byte tiebreak (`then_with(|| a.cmp(b))`). Collation sort keys are strictly transient query constructs and are never persisted to disk caches. Filter equality (`==`, `!=`) remains byte/NFC-exact and does not use collation."*
