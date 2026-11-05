Feature: Trust, parsing and validation failures stop before use
  Source: src/config/GLOSSARY.md, sections Config File, Configuration Scope, Config Locator, and Template Directory.
  Source: src/cli/init.rs, default initialization and transactional scaffolding guarantees.

  Scenario: Untrusted local configuration fails at trust verification
    Given an untrusted project configuration
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_untrusted"
    And stdout is empty

  Scenario: Malformed trusted configuration fails at parsing
    Given a malformed project configuration that is trusted
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_config_file_failed"
    And stdout is empty

  Scenario: Changed trusted configuration fails before its malformed content is parsed
    Given a trusted project whose configuration was changed to malformed TOML
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_untrusted"
    And stdout is empty

  Scenario: A malformed global configuration fails without global trust
    Given a trusted local project with malformed global configuration
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_config_file_failed"
    And stdout is empty

  Scenario: Invalid trusted configuration value fails at validation
    Given a project configuration with an invalid field key that is trusted
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_invalid_field_key"
    And stdout is empty

  Scenario: A section written as an array is rejected
    Given a project configuration that writes a section as an array and is trusted
    When I run "traces list"
    Then the exit code is 1
    And stderr contains diagnostic code "traces::cli::config_build_config_file_failed"
    And stdout is empty
