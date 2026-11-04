//! Shell completion generation and template-name listing.
//!
//! Serves `traces completions` by emitting a static completion script for a
//! supported [`Shell`] or printing available template names for dynamic
//! tab-completion.

use clap::{ArgGroup, Args, CommandFactory as _};
use clap_complete::{Shell, generate};

use super::error::{CliError, CliResult};
use crate::{ConfigService, template::TemplateService};

/// Arguments for `traces completions`.
///
/// Exactly one of `--shell` or `--list-templates` must be provided.
#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("completions_mode")
        .args(["shell", "list_templates"])
        .required(true)
        .multiple(false)
))]
pub(super) struct Completions {
    /// Generate a static shell completion script for `traces`.
    #[arg(long, value_enum)]
    shell: Option<Shell>,
    /// Print available template names for dynamic tab-completion.
    #[arg(long)]
    list_templates: bool,
}

impl Completions {
    /// Runs `traces completions`.
    ///
    /// With `--shell`, writes a shell completion script to stdout. With
    /// `--list-templates`, prints available template names.
    ///
    /// # Errors
    ///
    /// - [`CurrentDirectory`] if `--list-templates` requires reading the
    ///   current directory and it cannot be read.
    /// - [`ConfigLoad`] if loading configuration for `--list-templates` fails.
    ///
    /// [`CurrentDirectory`]: CliError::CurrentDirectory
    /// [`ConfigLoad`]: CliError::ConfigLoad
    #[inline]
    pub(super) fn run(self, service: &ConfigService) -> CliResult {
        self.shell.map_or_else(
            || Self::list_templates(service),
            |shell| {
                Self::print_script(shell);
                Ok(())
            },
        )
    }

    /// Writes the completion script for `shell` to stdout.
    #[expect(
        clippy::print_stdout,
        reason = "completion scripts are data meant to be sourced by the \
                  shell, not diagnostic text; mirrors the dry-run precedent \
                  in crate::cli::template"
    )]
    fn print_script(shell: Shell) {
        print!("{}", Self::script(shell));
    }

    /// Generates the shell completion script for `shell`.
    ///
    /// # Panics
    ///
    /// Panics if generated output is not valid UTF-8.
    #[expect(
        clippy::expect_used,
        reason = "clap_complete::generate always emits valid UTF-8 \
                  shell-script text; a failure here means the invariant \
                  broke, which should panic loudly rather than silently print \
                  an empty script"
    )]
    fn script(shell: Shell) -> String {
        let mut command = super::Cli::command();
        let name = command.get_name().to_owned();
        let mut buf = Vec::new();
        generate(shell, &mut command, name, &mut buf);
        String::from_utf8(buf)
            .expect("clap_complete generates valid UTF-8 shell-script text")
    }

    /// Loads configuration and retrieves available template names.
    ///
    /// # Errors
    ///
    /// - [`CurrentDirectory`] if reading the current directory fails.
    /// - [`ConfigLoad`] if configuration discovery or loading fails.
    ///
    /// [`CurrentDirectory`]: CliError::CurrentDirectory
    /// [`ConfigLoad`]: CliError::ConfigLoad
    fn template_names(
        service: &ConfigService,
    ) -> Result<Vec<String>, CliError> {
        let config = super::load_config(service)?;
        Ok(TemplateService::list_available(&config))
    }

    /// Prints available template names to stdout, one per line.
    ///
    /// # Errors
    ///
    /// - [`CliError::CurrentDirectory`] if reading the current directory fails.
    /// - [`CliError::ConfigLoad`] if configuration discovery or loading fails.
    #[expect(
        clippy::print_stdout,
        reason = "template names are data meant to be consumed by shell \
                  completion scripts, not diagnostic text; mirrors the \
                  dry-run precedent in crate::cli::template"
    )]
    fn list_templates(service: &ConfigService) -> CliResult {
        for name in Self::template_names(service)? {
            println!("{name}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestProject, fixture_service};

    mod script {
        use rstest::rstest;

        use super::*;

        #[test]
        fn zsh_output_starts_with_the_zsh_compdef_directive() {
            let output = Completions::script(Shell::Zsh);

            assert!(output.starts_with("#compdef traces"));
        }

        #[test]
        fn bash_output_names_the_traces_completion_function() {
            let output = Completions::script(Shell::Bash);

            assert!(output.contains("_traces()"));
        }

        #[test]
        fn fish_output_targets_the_traces_command() {
            let output = Completions::script(Shell::Fish);

            assert!(output.contains("-c traces"));
        }

        /// Every non-hidden subcommand name on the root `Cli`.
        fn cli_subcommand_names() -> Vec<String> {
            let command = crate::cli::Cli::command();
            command
                .get_subcommands()
                .filter(|sub| !sub.is_hide_set())
                .map(|sub| sub.get_name().to_owned())
                .collect()
        }

        /// Every long flag declared by the named subcommand, derived from
        /// the same `Cli` tree `Completions::script` renders, so this test
        /// tracks the CLI definition instead of a hand-written list.
        fn cli_long_flags(subcommand: &str) -> Vec<String> {
            let command = crate::cli::Cli::command();
            let sub = command
                .find_subcommand(subcommand)
                .expect("subcommand exists on Cli");
            sub.get_arguments()
                .filter_map(|arg| arg.get_long())
                .map(ToOwned::to_owned)
                .collect()
        }

        /// How `shell` spells the long flag `long` in its script: fish
        /// declares it as `-l long`, the others keep `--long`.
        fn flag_token(shell: Shell, long: &str) -> String {
            match shell {
                Shell::Fish => format!("-l {long}"),
                _ => format!("--{long}"),
            }
        }

        #[rstest]
        #[case::bash(Shell::Bash)]
        #[case::elvish(Shell::Elvish)]
        #[case::fish(Shell::Fish)]
        #[case::powershell(Shell::PowerShell)]
        #[case::zsh(Shell::Zsh)]
        fn mentions_every_subcommand_in_the_generated_script(
            #[case] shell: Shell,
        ) {
            let output = Completions::script(shell);

            for name in cli_subcommand_names() {
                assert!(
                    output.contains(&name),
                    "{shell:?} script lacks subcommand {name:?}"
                );
            }
        }

        #[rstest]
        #[case::bash(Shell::Bash)]
        #[case::elvish(Shell::Elvish)]
        #[case::fish(Shell::Fish)]
        #[case::powershell(Shell::PowerShell)]
        #[case::zsh(Shell::Zsh)]
        fn mentions_the_long_flags_of_list_task_and_template(
            #[case] shell: Shell,
        ) {
            let output = Completions::script(shell);

            for subcommand in ["list", "task", "template"] {
                for long in cli_long_flags(subcommand) {
                    let flag = flag_token(shell, &long);
                    assert!(
                        output.contains(&flag),
                        "{shell:?} script lacks {flag:?} of {subcommand:?}"
                    );
                }
            }
        }
    }

    mod parse {
        use clap::Parser as _;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use crate::cli::Cli;

        #[rstest]
        #[case::neither(&["traces", "completions"][..], clap::error::ErrorKind::MissingRequiredArgument)]
        #[case::both(
            &["traces", "completions", "--shell", "bash", "--list-templates"][..],
            clap::error::ErrorKind::ArgumentConflict
        )]
        fn rejects_anything_but_exactly_one_of_shell_and_list_templates(
            #[case] argv: &[&str],
            #[case] kind: clap::error::ErrorKind,
        ) {
            let error = Cli::try_parse_from(argv)
                .expect_err("exactly one of --shell/--list-templates");

            assert_eq!(error.kind(), kind);
        }
    }

    mod dispatch {
        use super::*;

        #[test]
        fn generates_shell_script_and_returns_ok() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let service = fixture_service(temp.path());

            Completions {
                shell: Some(Shell::Bash),
                list_templates: false,
            }
            .run(&service)
            .expect("shell completion should succeed without config");
        }
    }

    mod template_listing {
        use std::fs;

        use super::*;
        use crate::{
            cli::{CwdGuard, error::CliError},
            config::ConfigError,
        };

        #[test]
        fn fails_with_config_discovery_when_no_config_is_found() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            fs::create_dir_all(&root).expect("create project dir");
            let service = fixture_service(temp.path());
            let _guard = CwdGuard::enter(&root);

            let error = Completions {
                shell: None,
                list_templates: true,
            }
            .run(&service)
            .expect_err("no config discoverable");

            assert!(matches!(error, CliError::ConfigLoad {
                source: ConfigError::Discovery(_),
                ..
            }));
        }
    }

    mod template_names {
        use std::fs;

        use pretty_assertions::assert_eq;

        use super::*;
        use crate::{
            cli::{CwdGuard, error::CliError},
            config::ConfigError,
        };

        #[test]
        fn lists_every_available_template_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_template("daily.md", "content");
            let _guard = CwdGuard::enter(project.root());

            let names = Completions::template_names(project.service())
                .expect("template_names");

            assert_eq!(names, vec!["daily".to_owned()]);
        }

        #[test]
        fn fails_with_config_build_when_project_root_is_not_trusted() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::untrusted(temp.path().join("project"));
            let templates_dir = project.root().join("templates");
            fs::create_dir_all(&templates_dir).expect("create templates dir");
            let _guard = CwdGuard::enter(project.root());

            let error = Completions::template_names(project.service())
                .expect_err("untrusted root fails");

            assert!(matches!(error, CliError::ConfigLoad {
                source: ConfigError::Trust(_),
                ..
            }));
        }
    }
}
