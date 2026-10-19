# Modeling: Rust Types, Invariants & Layout Architecture

---

## 1. Module & File Layout (POLS SSOT)

Structure file and directory layouts to match the cognitive map of the domain:

- **Principle of Least Surprise (POLS):** An engineer inspecting a module must find state, logic, and types exactly where semantic ownership implies.
- **SLoC Concentration (>500 SLoC):** A single `.rs` file exceeding 500 SLoC is a primary indicator of mixed lifecycles, entangled invariants, or multiple independent modules trapped in one file.
- **Disjoint Reference Clusters:** In knowledge graph analysis (`codegraph`/`rustgraph`), types and functions within the same file that have zero references to each other, but heavy references to distinct external modules, represent misplaced responsibilities grouped by accident.
- **Feature Envy:** A type or function in `module_a.rs` that references fields, methods, or errors in `module_b.rs` more frequently than its own file belongs inside `module_b.rs`.
- **Bypassed Seams (Deep Path Traversal):** Callers reaching deep into child submodules (e.g. `use crate::pipeline::stage::internal::parse_header;`) indicate an ineffective parent seam.
- **Eliminate "Junk Drawer" Files:** Files named `utils.rs`, `helpers.rs`, `common.rs`, or `misc.rs` violate predictability. Functions in utility files must be relocated to the domain type they operate upon or encapsulated behind an intentional seam.

---

## 2. Validated Types & Parse, Don't Validate

Translate discovered invariants into idiomatic Rust types:

- **Move Validity to Construction:** Ensure invalid states cannot be constructed. Downstream operations assume validity and discard runtime checks.
- **Enums over Correlated Booleans:** Replace boolean flags, incompatible `Option` pairs, and tag-plus-payload structs with enums representing semantic alternatives (`ISR = 0`).
- **Newtypes for Domain Vocabulary:** Use newtypes to prevent confusing semantically distinct values of identical underlying primitives and to own construction validation. Avoid wrappers that enforce nothing.
- **Constructor Encapsulation:** Private fields alone do not protect invariants if public constructors, `Default` derivations, or serialization bypass validation.

---

## 3. Traits, Variation & Generic Costs

Introduce a trait only when a seam needs meaningful behavioral variation or external dependency isolation, never as ornamental decoration:
- Multiple legitimate implementations exist, or an owned dependency requires a test/environment adapter.
- Treat generic parameters and trait bounds as interface cost. Prefer concrete types until variation produces demonstrable leverage.
- Do not introduce traits merely to enable mocking when in-memory test doubles or fast local implementations suffice.

---

## 4. Ownership, Borrowing & Visibility

- **Authority over State:** Code enforcing an invariant must have direct authority over the state involved.
- **Avoid Shared Ownership Crutches:** Do not use `Arc`, `Rc`, `Mutex`, or `RefCell` merely to dodge borrow checker boundaries or mask unclear ownership. If multiple components need shared mutability, find the single cohesive owner or pass messages across channels.
- **Scoped Visibility:** Visibility implements the intended seam. Restrict internal child items to `pub(crate)`, `pub(super)`, or `pub(in crate::ancestor)` to prevent internal mechanisms from leaking into public crate APIs.

---

## 5. Data-Oriented Layout & Cache Locality

When data structures reside on performance-critical paths or scale with input volume:

- **Hot/Cold Field Segregation:** Group frequently accessed fields in contiguous memory. Move cold metadata (rarely checked flags, diagnostic names) behind `Option<Box<ColdMeta>>` or separate arrays.
- **Struct-of-Arrays (SoA):** If loops iterate over only a subset of fields, prefer columnar/SoA layouts to maximize CPU cache saturation and auto-vectorization.
- **Struct Field Ordering:** Order fields from largest alignment to smallest to eliminate compiler padding holes.
- **Avoid Indirection for Small Items:** Prefer contiguous storage (slices, arrays, inline buffers) over pointer-chasing collections of `Box<T>`.

---

## 6. Zero-Copy Models & Lifecycles

Eliminate heap allocation overhead while maintaining clean seams:

- **Borrowing over Defensive Cloning:** Expose borrowed views (`&[T]`, `&str`) or `Cow<'a, T>` instead of eagerly calling `.clone()` or `.to_owned()`.
- **Scoped Arena Allocation:** For tree or graph structures with bounded lifecycles (such as parsing passes), allocate nodes into a typed arena (`bumpalo`). Nodes cross-reference via references or arena indices without per-item heap deallocations.
- **Small-Vector Optimization:** Use stack-allocated inline buffers (`SmallVec`) for collections that almost always contain fewer than 4 to 8 elements.

---

## 7. Typestate & Compile-Time State Machines

Use typestate when states are well-defined, transitions are central to correctness, and different states expose materially different legal operations (`ITE = 0`):

```rust
pub struct Session<State> {
    inner: SessionInner,
    _state: std::marker::PhantomData<State>,
}
```

Prefer ordinary enums or runtime validation when typestate would leak marker types, generic parameters, and conversion ceremony through unrelated caller interfaces.

---

## 8. Error Boundaries & Panic-Free Seams

Error types are part of the interface because callers must act on failure modes:

- **Panic-Free Seams:** Public and cross-crate entry points must never expose unhandled `.unwrap()`, `.expect()`, or `panic!` calls. Errors crossing seams must be structured `Result<T, E>` domain enums (`thiserror`).
- **Domain Error Classification:** Hide internal child errors behind cohesive domain classifications rather than mechanically mirroring every internal error variant.
- **Panic Containment:** Seams hosting worker tasks or plugins must isolate thread panics (`std::panic::catch_unwind` or tokio task join handling) to prevent cascading process failure.
- **Contract Evolution (Expand-and-Contract):** Use `#[non_exhaustive]` on public enums so variant additions are backward-compatible; use builder patterns with defaults for struct parameter expansion; employ parallel deprecation (`#[deprecated]`) over breaking signature changes.
