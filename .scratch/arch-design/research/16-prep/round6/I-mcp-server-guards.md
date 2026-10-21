# Round 6 addendum: what an MCP server can guard on its own

Read at the source: MCP specification revision 2026-07-28
(<https://modelcontextprotocol.io/specification/2026-07-28>): index, changelog,
`server/discover`, `server/tools`, `basic/patterns/mrtr`,
`basic/patterns/subscriptions`, `basic/transports/stdio`,
`client/elicitation`, the Skills extension page and the extension client
matrix. An earlier answer in this session cited the 2025-06-18 pages; these
claims replace it.

## Spec facts [V]

- No `initialize` handshake and no protocol-level sessions. Every request
  carries version and client capabilities in `_meta`. Cross-call state is a
  server-minted handle passed as an ordinary tool argument (changelog items 1
  and 2).
- `instructions` (natural-language guidance for LLMs) is an optional field of
  the `server/discover` result; calling `server/discover` is optional for
  clients.
- Server-initiated requests (elicitation, sampling, roots) use the multi
  round-trip pattern. A server may answer `tools/call`, `resources/read` or
  `prompts/get` with `resultType: "input_required"`; the client retries with
  `inputResponses` and the opaque `requestState`. A server must not request a
  capability the client did not declare. `requestState` is attacker-controlled
  input: integrity-protect it and bind it to principal, TTL and a digest of the
  originating request. Form elicitation schemas are flat primitives only.
- `subscriptions/listen` replaced `resources/subscribe`: client opt-in, stream
  per subscription; on stdio the client must re-send it after a restart.
- Roots, Sampling and Logging are deprecated. Logging notifications are
  emitted only for requests that carry a per-request log level.
- `tools/call` results carry a `content` array and `isError`; `structuredContent`
  may be any JSON value. Tool annotations remain, and clients must treat them as
  untrusted. List results carry `ttlMs` and `cacheScope`.
- stdio: the server is a client subprocess; on unexpected exit the client should
  restart it and in-flight requests are lost.
- Skills over MCP is an extension; in the community client matrix only ChatGPT,
  fast-agent and Inspector appear, all "Partial". No coding agent is listed.

## What follows for `traces`

Held guards (each runs inside the server, needs nothing from the harness):
validate before write with the delta rule; content-hash precondition; atomic
write then re-read and re-check; undo journal; delete, move and bulk guards;
input coercion; violations plus a field summary on every read result;
confirmation through `input_required` where the client declares `elicitation`,
deny otherwise.

Best effort only: a notice of out-of-band changes (in-process watcher and
in-memory baseline). The protocol has no sessions; the process may restart and
the baseline is lost; several callers may share an HTTP server. Correctness
must never depend on it. Whether clients pass extra result content blocks to
the model is untested [H].

Not usable: Roots confinement (use server configuration), Logging as a channel,
`subscriptions/listen` as a model feed, Skills extension, annotations as
enforcement.

## Open

- No client was tested; every "reaches the model" claim is [H].
- The `traces` MCP server's transport (stdio vs HTTP) decides how meaningful
  per-caller state is.
