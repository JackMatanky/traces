//! Shared end-to-end test harness: spawns the real `traces` binary against a
//! fully isolated [`Sandbox`].
//!
//! # Environment policy
//!
//! The child environment is an explicit policy, not a blanket wipe or an
//! inherit-and-override ([`Sandbox::apply_env_policy`]):
//!
//! - **Sandbox-owned**: `TRACES_STATE_DIR`, `XDG_CONFIG_HOME`, `TZ=UTC`, and
//!   the platform home anchors (`HOME` on Unix; `APPDATA`, `LOCALAPPDATA`,
//!   `USERPROFILE` on Windows) always carry sandbox-provided values, so every
//!   derived state/config location lands inside the sandbox.
//! - **Passthrough allowlist**: `PATH` (all platforms), `SystemRoot`, `TEMP`,
//!   `TMP` (Windows) are copied from this test process; the child cannot run
//!   tooling without them.
//! - **Opt-in**: `TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS` are never set by
//!   default; tests studying them set them explicitly via
//!   [`Sandbox::run_with_envs`].
//!
//! Everything else is dropped: a shell-exported variable cannot leak into any
//! child unless it is allow-listed or explicitly opted in.
//!
//! # Isolation
//!
//! Sandbox-owned variables are set only on the spawned child (never on this
//! test process itself) so every test stays parallel-safe with no shared
//! mutable state. `TRACES_STATE_DIR` isolates the trust and tracked-config
//! stores (`src/dirs.rs`'s dedicated override); `XDG_CONFIG_HOME` isolates
//! global config/template discovery, so a developer machine's real
//! `~/.config/traces/config.toml` (if any) never leaks into a test.
//!
//! # Diagnostic text is wrap-fragile
//!
//! Miette line-wraps long diagnostic text (such as absolute paths and
//! multi-level causal chains) at a fixed width. Causal chains deeper than one
//! level receive wrapped continuation lines prefixed with `│` or `├─▶` glyphs,
//! which can occur mid-path where no whitespace existed originally.
//!
//! Reconstructing exact wrapped text is therefore unreliable in general.
//! Assertions stick to content proven not to wrap:
//!
//! - Diagnostic codes (always printed on their own short line).
//! - Primary stdout content (printed directly via `print!`, never through
//!   Miette).
//! - [`plain`] output (used only for short single-bullet messages confirmed to
//!   wrap at real word boundaries with no glyph inside the reconstructed span).
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, ExitStatus, Output},
};

use pretty_assertions::{assert_eq, assert_ne};
use tempfile::TempDir;
/// integration tests run.
pub(crate) const TRACES_BIN: &str = env!("CARGO_BIN_EXE_traces-pkm");

/// Represents a captured process run with exit status and decoded output
/// streams.
pub(crate) struct Run {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Run {
    pub(crate) fn is_success(&self) -> bool {
        self.status.success()
    }

    pub(crate) fn is_failure(&self) -> bool {
        !self.is_success()
    }

    pub(crate) fn code(&self) -> Option<i32> {
        self.status.code()
    }

    pub(crate) fn assert_code(&self, expected: i32) {
        assert_eq!(
            self.code(),
            Some(expected),
            "expected exit code {expected}, got {:?}\nstdout: {}\nstderr: {}",
            self.code(),
            self.stdout,
            self.stderr
        );
    }

    pub(crate) fn assert_success(&self) {
        self.assert_code(0);
    }

    pub(crate) fn assert_never_exit_101(&self) {
        assert_ne!(
            self.code(),
            Some(101),
            "process panicked with exit code 101!\nstderr: {}",
            self.stderr
        );
    }

    pub(crate) fn assert_stdout_eq(&self, expected: &str) {
        assert_eq!(self.stdout, expected);
    }

    pub(crate) fn assert_stderr_eq(&self, expected: &str) {
        assert_eq!(self.stderr, expected);
    }
}

/// Strips Miette box-drawing continuation glyphs and collapses whitespace runs
/// to single spaces.
///
/// Only safe for messages proven to wrap at real word boundaries with no
/// glyph inside the reconstructed span (see module docs). Never used for
/// long absolute paths, which Miette can wrap mid-path with a `│` continuation
/// prefix.
pub(crate) fn plain(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '│' | '├' | '╰' | '─' | '▶' | '×' | '·'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Manages an isolated project sandbox with dedicated state, configuration, and
/// project root directories.
///
/// Every [`Sandbox::run`] invocation runs against these isolated temporary
/// directories, never the real host environment.
pub(crate) struct Sandbox {
    state_dir: TempDir,
    config_home: TempDir,
    project: TempDir,
}

impl Sandbox {
    /// Creates three isolated temporary directories for state, configuration,
    /// and project root.
    ///
    /// Leaves project files uninitialized. Call [`Self::write_config`] or
    /// [`Self::trusted`] before running commands that require a
    /// discoverable or trusted project.
    pub(crate) fn new() -> Self {
        Self {
            state_dir: TempDir::new().expect("create state dir"),
            config_home: TempDir::new().expect("create config home dir"),
            project: TempDir::new().expect("create project dir"),
        }
    }

    /// Adopts caller-owned temporary directories (used by `golden_path`,
    /// which must create its directories before `Init` runs in-process).
    pub(crate) fn from_dirs(
        project: TempDir,
        state_dir: TempDir,
        config_home: TempDir,
    ) -> Self {
        Self {
            state_dir,
            config_home,
            project,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        self.project.path()
    }

    /// Returns the sandbox state directory (`TRACES_STATE_DIR`): the root
    /// under which the trust and tracked-config stores must land.
    pub(crate) fn state_dir(&self) -> &Path {
        self.state_dir.path()
    }

    /// Returns the sandbox global-config home (`XDG_CONFIG_HOME`): the parent
    /// directory under which global config/template discovery resolves.
    pub(crate) fn config_home(&self) -> &Path {
        self.config_home.path()
    }

    /// Passthrough allowlist entries copied from this test process.
    fn passthrough_env() -> Vec<(&'static str, OsString)> {
        let base = vec![("PATH", std::env::var_os("PATH").unwrap_or_default())];
        #[cfg(windows)]
        {
            let mut passthrough = base;
            for key in ["SystemRoot", "TEMP", "TMP"] {
                if let Some(value) = std::env::var_os(key) {
                    passthrough.push((key, value));
                }
            }
            passthrough
        }
        #[cfg(not(windows))]
        base
    }

    /// Sandbox-owned entries: every derived state/config location the binary
    /// can resolve carries a sandbox-provided value, never the host's.
    fn owned_env(&self) -> Vec<(&'static str, OsString)> {
        // Windows entries are cfg-gated because the config resolver reads
        // `APPDATA` (not `XDG_CONFIG_HOME`) there; `dirs.rs` anchors its
        // Windows `HOME` on `USERPROFILE`. On Unix, `HOME` anchors the
        // `XDG_*` fallbacks.
        #[cfg(windows)]
        {
            vec![
                (
                    "TRACES_STATE_DIR",
                    self.state_dir.path().as_os_str().to_owned(),
                ),
                (
                    "XDG_CONFIG_HOME",
                    self.config_home.path().as_os_str().to_owned(),
                ),
                ("APPDATA", self.config_home.path().as_os_str().to_owned()),
                ("LOCALAPPDATA", self.state_dir.path().as_os_str().to_owned()),
                ("USERPROFILE", self.state_dir.path().as_os_str().to_owned()),
                ("TZ", OsString::from("UTC")),
            ]
        }
        #[cfg(not(windows))]
        {
            vec![
                (
                    "TRACES_STATE_DIR",
                    self.state_dir.path().as_os_str().to_owned(),
                ),
                (
                    "XDG_CONFIG_HOME",
                    self.config_home.path().as_os_str().to_owned(),
                ),
                ("HOME", self.state_dir.path().as_os_str().to_owned()),
                ("TZ", OsString::from("UTC")),
            ]
        }
    }

    /// Returns the full declared policy table (passthrough + owned entries)
    /// applied to every spawned child.
    ///
    /// Opt-in variables (`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS`) are
    /// deliberately absent; tests studying them add them explicitly.
    pub(crate) fn env_policy_entries(&self) -> Vec<(String, OsString)> {
        Self::passthrough_env()
            .into_iter()
            .chain(self.owned_env())
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    /// Applies the sandbox environment policy to `cmd`: default-deny via
    /// [`Command::env_clear`], then the passthrough allowlist and the
    /// sandbox-owned entries.
    ///
    /// Unlisted variables are dropped, so a shell-exported variable cannot
    /// leak into any child unless it is allow-listed or explicitly opted in.
    fn apply_env_policy(&self, cmd: &mut Command) {
        cmd.env_clear();
        for (key, value) in Self::passthrough_env() {
            cmd.env(key, value);
        }
        for (key, value) in self.owned_env() {
            cmd.env(key, value);
        }
    }

    /// Writes `.traces/config.toml` pointing to a local `templates` directory
    /// and creates that directory.
    pub(crate) fn write_config(&self) {
        std::fs::create_dir_all(self.root().join("templates"))
            .expect("create templates dir");
        self.write_config_fixture(&super::common::ConfigFixture::minimal());
    }

    /// Writes a declarative [`ConfigFixture`] to `.traces/config.toml`.
    pub(crate) fn write_config_fixture(
        &self,
        fixture: &super::common::ConfigFixture,
    ) -> std::path::PathBuf {
        fixture.write_to(self.root()).expect("write config fixture")
    }

    /// Writes a Note at `rel_path` relative to the project root, creating
    /// parent directories as needed.
    pub(crate) fn write_note(&self, rel_path: &str, content: &str) {
        let path = self.root().join(rel_path);
        std::fs::create_dir_all(path.parent().expect("note parent"))
            .expect("create note parent dir");
        std::fs::write(path, content).expect("write note");
    }

    /// Writes a schema TOML file under `.traces/schemas/`.
    pub(crate) fn write_schema(&self, name: &str, toml: &str) {
        let path =
            self.root().join(".traces/schemas").join(format!("{name}.toml"));
        std::fs::create_dir_all(path.parent().expect("schemas parent"))
            .expect("create schemas dir");
        std::fs::write(path, toml).expect("write schema file");
    }

    /// Writes a schema values file under `.traces/schemas/values/`.
    pub(crate) fn write_schema_value(&self, rel_path: &str, toml: &str) {
        let path = self.root().join(".traces/schemas/values").join(rel_path);
        std::fs::create_dir_all(path.parent().expect("schema values parent"))
            .expect("create schema values dir");
        std::fs::write(path, toml).expect("write schema value file");
    }

    /// Writes a template file into the project's local template directory.
    pub(crate) fn write_template(&self, name: &str, source: &str) {
        std::fs::write(self.root().join("templates").join(name), source)
            .expect("write template");
    }

    /// Builds a `traces` [`Command`] invocation configured for this sandbox.
    ///
    /// Sets binary path, project root working directory, and applies the
    /// sandbox environment policy ([`Self::apply_env_policy`]) on the child
    /// process without mutating the test process environment.
    pub(crate) fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(TRACES_BIN);
        cmd.args(args).current_dir(self.root());
        self.apply_env_policy(&mut cmd);
        cmd
    }

    /// Executes `args` against this sandbox and captures output in a [`Run`].
    pub(crate) fn run(&self, args: &[&str]) -> Run {
        let Output {
            status,
            stdout,
            stderr,
        } = self.command(args).output().expect("spawn traces");
        Run {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }
    }

    /// Executes `args` with explicit opt-in environment entries layered on
    /// top of the sandbox policy, and captures output in a [`Run`].
    ///
    /// This is the only sanctioned way to study opt-in variables
    /// (`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS`) from a test: they are
    /// never part of the default policy table.
    pub(crate) fn run_with_envs(
        &self,
        args: &[&str],
        envs: &[(&str, &OsStr)],
    ) -> Run {
        let mut cmd = self.command(args);
        for (key, value) in envs {
            cmd.env(key, value);
        }
        let Output {
            status,
            stdout,
            stderr,
        } = cmd.output().expect("spawn traces");
        Run {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }
    }

    /// Constructs a trusted [`Sandbox`] by writing configuration and
    /// dispatching `traces trust`.
    ///
    /// Exercises the trust flow as a side effect for fixtures requiring a
    /// trusted root.
    pub(crate) fn trusted() -> Self {
        let sandbox = Self::new();
        sandbox.write_config();
        let trust = sandbox.run(&["trust"]);
        assert!(
            trust.is_success(),
            "fixture setup: `traces trust` failed: {}",
            trust.stderr
        );
        sandbox
    }
}
