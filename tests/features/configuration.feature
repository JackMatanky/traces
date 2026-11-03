Feature: Configuration discovery, scope and initialization preserve project behavior
  Source: src/config/GLOSSARY.md, sections Config File, Configuration Scope, Config Locator, and Template Directory.
  Source: src/cli/init.rs, default initialization and transactional scaffolding guarantees.

  Scenario: Non-interactive initialization scaffolds the default configuration
    Given an unconfigured project
    When I run "traces init"
    Then initialization creates the default configuration files
    And stderr reports successful initialization for the project

  Scenario: Repeat initialization leaves the existing configuration untouched
    Given a project already initialized with a custom configuration
    When I run "traces init"
    Then the exit code is 1
    And the existing configuration is unchanged

  Scenario: Local and global configuration fields combine independently
    Given a trusted project with a global template directory and a local output directory
    When I run "traces template -i daily --no-input"
    Then the rendered template is written under the local output directory

  Scenario: An absent local task filter uses the global filter
    Given a trusted project with a global task filter and no local task filter
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] global task (todo.md)
      """

  Scenario: A local task filter replaces the global filter
    Given a trusted project with both local and global task filters
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] local task (todo.md)
      """

  Scenario: Nearest local configuration is discovered alongside global configuration
    Given a nested project with outer, nearest, and global templates
    When I list templates from the nested project
    Then only the nearest local and global templates are listed

  Scenario: Missing local configuration fails at discovery
    Given an unconfigured project
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_discovery_failed"

  Scenario: Untrusted local configuration fails at trust verification
    Given an untrusted project configuration
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_untrusted"

  Scenario: Malformed trusted configuration fails at parsing
    Given a malformed project configuration that is trusted
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_config_file_failed"

  Scenario: Changed trusted configuration fails before its malformed content is parsed
    Given a trusted project whose configuration was changed to malformed TOML
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_untrusted"

  Scenario: A malformed global configuration fails without global trust
    Given a trusted local project with malformed global configuration
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_config_file_failed"

  Scenario: Invalid trusted configuration value fails at validation
    Given a project configuration with an invalid field key that is trusted
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_invalid_field_key"

  Scenario: A discovery ceiling stops the upward configuration search
    Given a trusted local configuration at the project root
    When I run "traces list" with the project root as the discovery ceiling
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_discovery_failed"
