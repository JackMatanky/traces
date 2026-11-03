Feature: Configuration tracking survives trust changes and cleaning removes only stale paths
  Source: src/config/GLOSSARY.md, sections Tracked Config Path and Tracker Store.

  Scenario: Indexed configuration is listed as a tracked path
    Given a trusted project whose configuration was indexed
    When I run "traces tracked list"
    Then the exit code is 0
    And stdout is the project configuration path

  Scenario: Trust grant fails before trusting when tracking cannot save
    Given a project whose configuration tracking cannot be saved
    When I run "traces trust"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::trust::failed
      """
    And the project remains untrusted

  Scenario: Revoking trust leaves the configuration tracked
    Given a trusted project that was then untrusted
    When I run "traces tracked list"
    Then the exit code is 0
    And stdout is the project configuration path

  Scenario: Cleaning a stale tracked path reports its removal
    Given a trusted project whose indexed configuration was deleted
    When I run "traces tracked clean"
    Then the exit code is 0
    And stderr is:
      """
      removed 1 stale tracked entry
      """
    And no tracked configuration paths remain

  Scenario: Cleaning preserves a live tracked configuration path
    Given a trusted project whose configuration was indexed
    When I run "traces tracked clean"
    Then the exit code is 0
    And stderr is:
      """
      removed 0 stale tracked entries
      """
    And the project configuration path remains tracked
