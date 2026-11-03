Feature: Spawned process contracts for exit codes and streams
  Source: ADR-0004, src/cli/GLOSSARY.md.

  Scenario: Unknown subcommand exits with code two and prints usage
    When I run "traces nonexistent_command"
    Then the exit code is 2
    And the first line of stderr is:
      """
      error: unrecognized subcommand 'nonexistent_command'
      """
    And stdout is empty

  Scenario: Missing command exits with code one and reports diagnostic code
    When I run "traces"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::no_command
      """
    And stdout is empty

  Scenario: Missing project configuration exits with code one and reports discovery diagnostic
    Given an unconfigured project
    When I run "traces list"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::config_discovery_failed
      """
    And stdout is empty

  Scenario: Listing templates exits with code zero and writes payload to stdout
    Given a trusted project with a template "alpha.md"
    When I run "traces template --list"
    Then the exit code is 0
    And stdout is:
      """
      alpha
      """
    And stderr is empty

  Scenario: Cleaning trust store exits with code zero and writes status to stderr
    Given a trusted project
    When I run "traces trust clean"
    Then the exit code is 0
    And stdout is empty
    And stderr is:
      """
      removed 0 stale trust entries
      """

  Scenario: Task list writes count summary to stderr
    Given a trusted project with a note "note.md" containing:
      """
      - [ ] my task
      """
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] my task (note.md)
      """
    And stderr reports "1 task(s) from " for the project root

  Scenario: Task count flag suppresses summary on stderr
    Given a trusted project with a note "note.md" containing:
      """
      - [ ] my task
      """
    When I run "traces task --count"
    Then the exit code is 0
    And stdout is:
      """
      1
      """
    And stderr is empty
