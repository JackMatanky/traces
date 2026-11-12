# Crate evidence for ticket 28: buffer, watcher and store crates

Crate facts come from `rust-docs-mcp` against the cached versions. Each finding
names the concept that it informs.

## A. Editor text and positions (C3)

1. **ropey 1.6.1.** [V] A rope gives cheap edits and offset mapping for live
   buffers. It is cached locally with generated docs.
   Lesson for 28: C3 — a rope beats a plain `String` for repeated edits.
   Ticket 23 already gives the host the text and the version. The crate choice
   is a host detail, not core semantics.

2. **line-index 0.1.2 and text-size 1.1.1.** [V] rust-analyzer's extracted
   line-index and text-size crates model byte offsets and line conversion.
   Lesson for 28: C3 — ticket 23 gives the UTF-16 conversion contract. It can
   ride these crates or the existing `src/position.rs` `LineIndex`. Compare
   them before the grilling.

## B. Watching and debounce (C6, C7)

3. **notify 8.2.0 and notify-debouncer-full 0.7.0.** [V] The debouncer merges
   rename-from and rename-to pairs and queues events; `DebounceDataInner`
   handles rename and remove edges (`notify-debouncer-full` structure,
   `DebounceEventHandler`).
   Lesson for 28: C6 — external-change coalescing is a solved crate problem.
   Ticket 33 wires it. Ticket 28 states the semantics of a coalesced external
   event against an open key.

4. **salsa 0.28.5.** [V] Revision-based incremental recomputation with
   cancellation (`Cancelled`), durability levels, and cycle recovery.
   Lesson for 28: C2 — salsa's revision model matches the incarnation and
   revision pair naming from tickets 24 and 25. Adopting salsa is ticket 33+
   scope. The md-pkm-lsp map already deferred it.

## C. Store locking (C9)

5. **redb 4.3.0 `ConcurrencyMode`.** [V] `ExclusiveWriter` (default),
   `SingleWriter` and `MultiWriter` exist; the two shared modes need byte-range
   locks and are gated behind `experimental-multiprocess` (`redb` docs,
   `ConcurrencyMode`, `Builder::set_concurrency_mode`).
   Lesson for 28: C9 — the default mode enforces the one-writer rule that
   tickets 24, 25 and the S9 scenario assume. Cite the enum docs as the source
   of the lock behavior.

6. **redb `Durability`.** [V] `Durability::None` rows vanished at an `ENOSPC`
   failure boundary in the ticket 27 probe; `Immediate` rows survived.
   Lesson for 28: C9 — the overlay rule must never rely on a `None` commit for
   buffer facts; ticket 36 owns the durability choice.

## D. Protocol layer (C1, C8)

7. **tower-lsp-server 0.23.0.** [V] JSON-RPC framing, a `LanguageServer` trait
   with generated method registration, `Client::publish_diagnostics`, and
   progress helpers; transport is tokio-confined.
   Lesson for 28: C8 — diagnostics publishing and capability registration stay
   at the protocol edge; core carries no protocol types (ticket 20 rule).

8. **lsp-types 0.97.0.** [V] `TextDocumentSyncKind`, `didChange` payloads with
   version numbers, and `Position` in UTF-16 code units by default.
   Lesson for 28: C3 — the version guard and UTF-16 conversion in ticket 23 map
   directly onto the LSP payload shape.
