Feature: The index store is created, rebuilt and tracked without changing results
  Source: src/index/GLOSSARY.md, src/cli/GLOSSARY.md, ADR-0009.

  Scenario: A list query creates a store without an index command
    Given a trusted project with a note "a.md" containing:
      """
      # A
      """
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """
    And the project has an index store

  Scenario: Deleting the index store preserves query results
    Given an indexed project whose index store was removed
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """
    And the project has an index store

  Scenario: A second query from the stored index matches the first
    Given an indexed project with a persisted list result
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      - b.md
      """
    And another list run has the same stdout

  Scenario: Indexing records the project configuration as tracked
    Given a trusted project that was indexed
    When I run "traces tracked list"
    Then the exit code is 0
    And stdout is the project configuration path
