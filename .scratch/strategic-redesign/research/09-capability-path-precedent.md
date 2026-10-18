# Research: Enforcing Root Confinement and Portable Path Identity

Label: `wayfinder:research`. Informs: path and capability model, index read model, template writer, and error topology.
Context tickets: `survey/root-primitives.md` (H7), `survey/cross-cutting.md` (H4), `survey/template.md` (H4, H5).

---

## 1. What Each Comparable Actually Does

### cap-std & cap-primitives

- **Kernel-level resolution**: On Linux >= 5.6, `cap_primitives::fs::open_beneath` calls `rustix::fs::openat2` with `ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS` (`cap-primitives/src/rustix/linux/fs/open_impl.rs:56-124`) [V]. It retries up to 4 times on `EAGAIN` (handling concurrent filesystem renames) and falls back to manual resolution on `ENOSYS` or `EPERM` [V]. FreeBSD 13+ passes `OFlags::RESOLVE_BENEATH` to `openat` (`cap-primitives/src/rustix/freebsd/fs/open_impl.rs:17`) [V].
- **Userspace fallback (macOS & older Linux)**: `cap_primitives::fs::manually::open` executes manual component-by-component traversal (`cap-primitives/src/fs/manually/open.rs:23-28`) [V]. It maintains an open directory descriptor stack `dirs: Vec<MaybeOwnedFile>` (`:36`). On `..`, it pops `dirs`; if empty, it errors with an escape attempt without asking the OS to resolve `..` (`:184-207`) [V]. Intermediate directories are opened with `O_NOFOLLOW` / `O_PATH` (`:216-235`), and symlinks are expanded manually with absolute targets rejected (`:247-285`) [V].
- **Windows strategy**: Uses Windows NT syscall `NtCreateFile` directly, passing the parent directory handle in `OBJECT_ATTRIBUTES.RootDirectory` (`cap-primitives/src/windows/fs/create_file_at_w.rs:156-160,195-209`) [V]. Directory handles are opened with `FILE_FLAG_BACKUP_SEMANTICS` and omit `FILE_SHARE_DELETE` to freeze directories against rename/deletion during resolution (`cap-primitives/src/windows/fs/dir_utils.rs:63-65`) [V]. Syntactic `..` reduction occurs before lookup (`open.rs:81-95`) [V].
- **Encapsulation**: `cap_std::fs::Dir` wraps an owned `std::fs::File` descriptor (`cap-std/src/fs/dir.rs:50`) [V]. All filesystem operations require passing `&Dir`; paths cannot touch the filesystem without their parent directory handle [V].

### strict-path

- **Validation mechanism**: `PathBoundary<Marker>` wraps an `Arc<PathHistory<((Raw, Canonicalized), Exists)>>` (`strict-path/src/validator/path_boundary.rs:91`) [V]. `canonicalize_and_enforce_restriction_boundary` joins candidates to the boundary, delegates to `soft-canonicalize` (resolving existing prefixes and symlinks, then appending uncreated tail components syntactically), and checks `canonicalized.boundary_check(boundary)` (`:41-63`) [V].
- **Output & limits**: Returns `StrictPath<Marker>` holding the boundary and canonicalized `PathBuf` (`path/strict_path/mod.rs:1`) [V]. It is a point-of-validation check, not an ongoing handle [V]. It does not retain open file descriptors, leaving a TOCTOU window if intervening symlinks swap before subsequent I/O [V].
- **Traces misuse**: In Traces, `SafeRelativePath::parse` runs `boundary.strict_join(...)` (`src/path.rs:181-189`) [V], but immediately discards the resulting `StrictPath` and returns a plain join `root.join(safe.as_ref())` (`:190`) [V], discarding the canonicalized path and retaining an acknowledged TOCTOU window (`:159-161`) [V].

### gitoxide (gix-path, gix-fs, gix-worktree, gix-validate)

- **Lexical component validation**: `gix_validate::path::component` validates path components against repository escapes (`gix-validate/src/path.rs:90-146`) [V]. It rejects empty names, `.`, `..`, `/`, `\`, and Windows drive prefixes (`:99-114`) [V].
- **Filesystem-specific evasion guards**:
  - `protect_hfs` (`:115-122,213-253`): Rejects `.git` matches that exploit macOS HFS+ case-folding and ignorable Unicode characters (e.g. ZWNJ `0x200C`, directional formatting `0x202A-0x202E`, BOM `0xFEFF`) [V].
  - `protect_ntfs` (`:124-135,157-206,255-328`): Rejects 8.3 short names (`git~1`), Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`), forbidden characters (`<>:"|?*`, trailing dots/spaces), and NTFS Alternate Data Streams (`::$DATA`) [V].
- **Incremental prefix caching**: `gix_fs::Stack` caches directory traversal state (`gix-fs/src/stack.rs:228-241,113-225`) [V]. As paths are processed in order, it finds the longest matching prefix with `current`, pops differing components, and pushes new components through a `Delegate` (`:136-224`) [V].
- **Worktree symlink protection**: `gix_worktree::StackDelegate` checks `symlink_metadata()` on newly pushed directory components to ensure leading path components are real directories, never followed symlinks (`gix-worktree/src/stack/delegate.rs:86-108,165-195`) [V]. Terminal entries are re-validated on Windows (`:172-184`) [V].
- **Path typing**: Internal repository paths are strictly byte strings (`&BStr` / `BString`) with forward slashes (`gix-path/src/convert.rs:18-98`) [V]. Conversion to OS `Path` occurs only at the I/O boundary [V].

### Cargo (cargo-util::paths & cargo-util-schemas::restricted_names)

- **Lexical normalization without canonicalization**: `cargo_util::paths::normalize_path` normalizes `.`, `..`, and prefixes purely syntactically (`cargo-util/src/paths.rs:84-116`) [V]. The source explicitly warns against `std::fs::canonicalize` because it fails on nonexistent paths, resolves symlinks unexpectedly, and introduces Windows verbatim `\\?\` prefixes (`:78-83`) [V].
- **Atomic file writes**: `write_atomic` writes to a sibling tempfile via `tempfile::Builder::new().tempfile_in(path.parent())` and calls `persist(path)` (`:193-225`) [V].
- **Name validation**: `cargo_util_schemas::restricted_names` enforces Unicode XID start/continue rules and rejects Windows reserved words and build keywords (`cargo-util-schemas/src/restricted_names.rs:42-83,109-177`) [V].

### Ruff (ruff_db::system::SystemPath vs SystemPathBuf)

- **UTF-8 path newtype**: `SystemPath` (`crates/ruff_db/src/system/path.rs:11`) is `#[repr(transparent)] Utf8Path` (camino) [V]. `SystemPathBuf` (`:518`) is `Utf8PathBuf` [V]. The entire compilation and query database operates on validated UTF-8 paths [V].
- **Windows UNC simplification**: `SystemPath::simplified()` uses `dunce::simplified` to strip Windows `\\?\` verbatim prefixes safely without breaking UTF-8 guarantees (`:57-60`) [V].
- **Virtual vs System separation**: `SystemVirtualPath(str)` (`:781`) models in-memory/notebook documents, explicitly decoupled from physical `SystemPath` (`:11`) [V].
- **System boundary**: The `System` trait (`crates/ruff_db/src/system.rs:52-123`) abstracts filesystem I/O, isolating path manipulation from disk syscalls [V].

### camino & relative-path

- **camino**: Replaces `Path`/`PathBuf` with UTF-8 equivalents `Utf8Path`/`Utf8PathBuf`. Uses OS-native separators (`/` on Unix, `\` on Windows) [V].
- **relative-path**: `RelativePath`/`RelativePathBuf` models portable UTF-8 relative paths with fixed `/` separators across all platforms (`relative-path/src/lib.rs:5-18`) [V]. It cannot represent an absolute path and requires an explicit base path (`to_path(&base)`) to interact with the filesystem (`:12-14`) [V].

---

## 2. Invariants and Invariant Owners

| Invariant | cap-std | strict-path | gitoxide | Ruff / Cargo | Traces Current |
|---|---|---|---|---|---|
| **Root Confinement** | Owned by OS kernel / `Dir` fd handle at every syscall [V]. | Owned by `PathBoundary` at validation time via canonical check [V]. | Owned by `gix_validate` (lexical) + `gix_fs::Stack` (leading directory checks) [V]. | Owned by lexical sanitization (`normalize_path`) and caller convention [V]. | Fragmented: `SafeRelativePath` checks `strict-path` then unwraps to `root.join()` [V]; template inspect accepts absolute [V]; dry-run skips check [V]. |
| **Symlink Non-Traversal** | Invariant: syscalls cannot traverse symlinks outside `Dir` [V]. | Invariant: canonical target is within boundary at check time [V]. | Invariant: leading path components must be directories, never symlinks [V]. | Invariant: none; symlinks followed during traversal unless caller opts out [V]. | Partial: `strict-path` resolves symlinks at check; TOCTOU window acknowledged [V]. |
| **Path Portability** | Native OS paths only; not portable [V]. | Native OS paths; boundary check is machine-local [V]. | Invariant: internal tree paths are `/`-separated `BStr`; portable across OS [V]. | Ruff: UTF-8 `SystemPath` [V]. `relative-path`: `/`-separated UTF-8 [V]. | Broken: `src/path.rs::codec` serializes `&[u8]` (Unix) vs `&[u16]` (Windows) [V]. |
| **Atomic Creation** | `Dir::open` options [V]. | `StrictPath` delegates to `std::fs` [V]. | `gix_fs` atomic lock/write helpers [V]. | `cargo_util::paths::write_atomic` via tempfile persist [V]. | `CommitPolicy::CreateNew` uses `File::create_new` (`src/template/writer.rs:68`) [V]. |

---

## 3. Tradeoffs and Costs

### Capability Handles (cap-std)

- **Costs**: Pervasive handle passing (`&Dir`); high architectural viral footprint [H]. High syscall overhead on macOS/Linux fallback ($O(K)$ descriptor opens per path component) [V]. Windows requires undocumented NT syscalls (`NtCreateFile`) and intrusive directory locking (`FILE_SHARE_DELETE` unset) [V].
- **Benefits**: True kernel-enforced immunity against symlink race attacks (TOCTOU) [V].

### Validated Lexical + Boundary Newtypes (relative-path + PathBoundary / ProjectRoot)

- **Costs**: Does not prevent concurrent background processes from swapping symlinks between check and read/write (TOCTOU) [V].
- **Benefits**: Zero file descriptor leaks, zero Windows NT syscall tricks, negligible CPU overhead, clean thread/async sharing, simple serialization, zero allocations when borrowed [H].

### Prefix Stack (gix-fs::Stack)

- **Costs**: Requires sequential/tree-ordered traversal to amortize prefix validation calls [V].
- **Benefits**: Enforces that no intermediate path component can be a symlink during checkout/traversal with amortized $O(1)$ stat calls [V].

### UTF-8 Portable Paths (camino, relative-path)

- **Costs**: Rejects non-UTF-8 paths on Unix (non-UTF-8 paths cannot be represented) [V].
- **Benefits**: Eliminates `OsStr` conversion boilerplate (`to_str().unwrap()`), prevents platform-incompatible serialization, and enables uniform `/` separators [H].

---

## 4. Transferable Lessons for Traces

1. **Vault-Relative Path as a First-Class Domain Type** [H]:
   - Traces is a PKM vault tool. Every note and asset within a vault has an identity strictly relative to the vault root [V].
   - Storing `PathBuf` in `FileMeta` (`src/file.rs:47`) and index structures is an anti-pattern that permits absolute paths, Windows backslashes, and non-portable serialization [H].
   - A portable, UTF-8 relative path type (following `relative-path` / `camino` / `gix-path`) with `/` separators should represent note paths across the entire internal pipeline (`FileMeta`, `WorkspaceIndex`, `redb`, query results, template models) [H].

2. **Root Confinement Belongs at the I/O Boundary, Not as a Viral Type** [H]:
   - In `cap-std`, directory capability handles must be passed into every leaf function [V]. In contrast, Ruff and Cargo keep internal path types pure and apply confinement/filesystem operations at the service boundary [V].
   - Traces query evaluation, AST parsing, and in-memory indexing do not perform arbitrary disk I/O; they operate on cached or scanned vault notes [V].
   - Confinement needs cohesive enforcement in exactly two places: (a) initial scanner discovery, and (b) `TemplateWriter` / `TemplateService` destination resolution [H].

3. **Eliminate Non-Portable Index Codec via UTF-8 Relative Paths** [V]:
   - `src/path.rs::codec` (`:340-437`) serializes Unix paths as raw bytes (`&[u8]`) and Windows paths as wide units (`&[u16]`) [V]. A database created on Unix cannot be opened on Windows [V].
   - Serializing normalized UTF-8 relative paths with `/` separators eliminates the platform-conditional codec entirely and makes redb databases cross-platform portable [H].

4. **Harmonize Template Inspection and Dry-Run Confinement** [V]:
   - Template inspection (`src/template/engine/path.rs:162`) accepting absolute paths violates the workspace security boundary established in ADR-0002 [V].
   - `TemplateService::write` (`src/template/service.rs:225-227`) bypassing output resolution during `DryRun` creates an invariant discrepancy where invalid escape paths succeed in preview and fail only on commit [V].

---

## 5. Lessons that Do NOT Transfer

1. **Full `cap-std` OS Capability Handles Do NOT Transfer** [H]:
   - *Reason*: Traces is a local CLI tool run by a single user on personal Markdown vaults [V]. It is not a multi-tenant setuid daemon, container sandbox, or web server hosting untrusted code [H]. The threat of a malicious local process racing symlink swaps inside a user's Obsidian vault during a 20ms CLI command is outside Traces' realistic threat model [H]. Paying the architectural complexity of `cap_std::fs::Dir` across macOS, Linux, and Windows is disproportionate [H].
2. **Linux `openat2(RESOLVE_BENEATH)` Does NOT Transfer as a Universal Primitive** [H]:
   - *Reason*: Traces supports macOS, Linux, and Windows [V]. `openat2` is Linux-only (kernel >= 5.6) [V]. Relying on it provides zero protection on macOS developer laptops without maintaining complex userspace fallbacks [H].
3. **`gix-fs::Stack` Incremental Directory Stack Does NOT Transfer to Single-File Template Writes** [H]:
   - *Reason*: Git checkout writes hundreds of thousands of files in sorted order, where amortizing `mkdir` and symlink checks over a shared prefix yields huge performance wins [V]. Traces writes one rendered template note at a time [V]. Simple parent validation via `ProjectRoot` suffices [H].
4. **Git's Exotic HFS Ignorable Unicode Checks Do NOT Transfer** [H]:
   - *Reason*: Git needs to prevent hostile repositories from overwriting `.git/config` on clone via codepoints like `0x200C` [V]. Traces does not parse untrusted foreign repository trees [H].

---

## 6. Impact on Traces Architectural Hypotheses

### `survey/root-primitives.md` [H7] (Capability `ProjectRoot` vs `strict-path`)

- **Survey thesis**: Encapsulate path confinement into an OS-handle capability `ProjectRoot` to eliminate the TOCTOU symlink traversal window (`src/path.rs:159-161`) [V].
- **Finding**: Refutes full `cap-std` capability handles as over-engineering for a PKM CLI [H].
- **Refined orientation**: Confirms that a `ProjectRoot` domain boundary object is needed, but as a cohesive **I/O boundary authority** owning canonical root resolution, safe path joining, and file writing—not an OS file descriptor wrapper [H].

### `survey/cross-cutting.md` [H4] (End-to-End Type-Safe Root Confinement)

- **Survey thesis**: Carrying `SafeRelativePath` throughout persistent structs (`FileMeta::path`) guarantees root confinement at the type level [V].
- **Finding**: Challenges carrying `SafeRelativePath` in persistent structs [H]. In Traces, `SafeRelativePath` joins the candidate to `root` (`src/path.rs:190`), producing an absolute path that couples persisted data to host-specific filesystem roots [V].
- **Refined orientation**: Persistent structs and index tables should carry a **purely relative, normalized UTF-8 path newtype** (e.g. `VaultPath` / `RelativePath`) [H]. Confinement is enforced at the I/O boundary where relative paths meet host paths [H].

### `survey/template.md` [H4] (Confine Path Inspection to Project Root)

- **Survey thesis**: Harmonize path inspection (`path_exists`, `is_file_path`, `is_dir_path`) with `file.include` and `writer` by strictly confining all path arguments to `Config::root` (`src/template/engine/path.rs:162-170`) [V].
- **Finding**: Strongly affirmed [V]. Allowing unconfined absolute paths in template inspection is an architectural leak [H]. Inspection should route through `ProjectRoot` confinement [H].

### `survey/template.md` [H5] (Validate Output Paths during DryRun)

- **Survey thesis**: Resolving and validating `file.write_to()` output paths during `WriteMode::DryRun` (`src/template/service.rs:225-227`) ensures path confinement errors are caught during preview [V].
- **Finding**: Strongly affirmed [V]. Target resolution and boundary validation must execute identically in both modes; only the actual disk commit should be suppressed [H].
