Feature: A project is trusted only after the user runs trust
  Source: ADR-0002, src/config/GLOSSARY.md.

  Scenario: Init does not grant trust
    Given a project initialized with init
    When I run "traces list"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::config_build_untrusted
      """
    And stdout is empty

  Scenario: Untrust revokes trust for later runs
    Given a trusted project that was then untrusted
    When I run "traces list"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::config_build_untrusted
      """
    And stdout is empty
