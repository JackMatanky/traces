Feature: Initialization scaffolds the default configuration
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
