# Config ownership: section types and reload lifecycle

Type: grilling
Blocked by: 01, 02, 05

## Question

`config` defines raw serde structs (`raw.rs`), validated model structs (`model.rs`, ~1.3K lines) and a builder (`builder.rs`), including domain settings such as `TaskConfig` and `FrontmatterConfig` whose domain types live elsewhere (`task.rs`, `note`). Lower layers therefore import `config`: `note::parser::input` takes `&FrontmatterConfig`/`&TaskConfig`; `IndexerService` takes `&Config` and clones three fields from it; `TemplateLoader` takes `&Config` for two paths. Trust verification (`ConfigService`, typestate `Discovered → Trusted → Parsed`, `FilePathTracker`) is single-shot by design. Decide: **who owns each configuration section's type and validation**, how consumers receive only what they need (without the upward edge from `note`), and what the lifecycle is for reload and trust re-verification in a long-lived host.

## Fixed inputs (in-flight, NOT open here)

The maintainer is implementing [config-service/07](../../config-service/issues/07-config-digest-sections.md) then [index-query/17](../../index-query/issues/17-config-drift-detection-and-rebuild.md) (see map Notes, *In-flight implementation baseline*). Per-section digests, their framing, `Config::hash()` as compose-all, the index-owned `IndexFingerprint`, and `index → config` as the sole invalidation edge are decided. This ticket must **not** re-decide digest granularity or drift policy; it must find an ownership design that is *compatible* with them. The live constraint to resolve: [config-service/07](../../config-service/issues/07-config-digest-sections.md) requires each config section type to destructure exhaustively in its own `digests()` constructor so that adding a setting breaks compilation. If section types move out of `config` into their domain modules, that fail-closed property must travel with them (and [index-query/17](../../index-query/issues/17-config-drift-detection-and-rebuild.md)'s index-side exhaustive destructure must still see every section). Any real conflict is recorded as an *amendment candidate* for the maintainer.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md) lands; `MOVING (07/17)` claims are tagged there): [config/trust §9–§11](../survey/config-trust.md), [cross-cutting §5 upward edges](../survey/cross-cutting.md). Existing ADRs to treat as claims: `docs/adr/0002-config-trust-and-tracking.md`, `docs/adr/0009-index-refresh.md` (ADR-0009, index refresh; decided direction, being implemented).

**Verified (ledgers landed 2026-10-18)** — [01 §1, §5](../research/01-verify-core-findings.md): `note/parser/input.rs:9` is the sole non-test `note → config` edge and `index/service.rs:22-23,42-51` the sole `index → config` edge; `IndexerService::parse_note` (`service.rs:270`) is the only production constructor of `MarkdownParserInput`. Section consumers: `root` (cli, index, template, config), `templates` (loader, service, writer), `schemas` (cli, index, template engine), `frontmatter` — **only `tags_name()` is read in production** (`note/parser.rs:381`; `title`, `aliases`, `date_created`, `date_modified` have no production reader), `tasks` (`tag_filters()`, `statuses()` in list parsing, `parser.rs:527-528`). Adding one `tasks` setting touches 4–5 files. `config → temporal` (`DEFAULT_DATETIME_PATTERN`, `config/model.rs:925,954`) is an edge the survey missed ([02 §7](../research/02-verify-system-findings.md)). In-flight state: `Config::hash()` is unused outside tests on `main`; the `config-digest-sections` worktree (`49c5ac39`) has `src/config/model/digest.rs` with `ConfigDigests` (config-service/07 landing). Ownership consequence to weigh: digests include settings nothing reads yet (`frontmatter.title`, …), which is deliberate under config-service/07's fail-closed rule.

## Alternatives (not exhaustive)

- Domain modules own their settings types and validation from raw TOML; `config` becomes discovery + trust + raw loading + composition; the digest convention is enforced per domain type (and the `syn` test scans each).
- `config` keeps all section types (single view of the file format, trivial fail-closed digests) but exposes narrow, independently-constructible projections to consumers.
- Consumer-defined option types (e.g. `note` defines the parser options it needs; the composition root maps config into them), removing the upward `note → config` edge without moving the section types.
- Trust/reload: re-verify on a change event with an atomically swapped config vs restart-only; behavior of in-flight work when trust is revoked.

## Done when

The answer states: where each section type lives and who validates it; dependency direction for `note`, `index`, `template`, `schema`, `query` (with `index → config` accepted as given); how the config-service/07 and index-query/17 fail-closed guarantees are preserved under the chosen ownership; the reload/trust lifecycle contract; the keep/change verdict on `FilePathTracker` vs `config::tracker`; and any amendment candidates for config-service/07, index-query/17, ADR-0002, or ADR-0009 (index refresh).
