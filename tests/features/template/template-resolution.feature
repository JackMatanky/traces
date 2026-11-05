Feature: Template names and includes resolve within project boundaries
  Source: src/template/GLOSSARY.md (Template Resolution, Template Output Path, Write Mode, Commit Policy); src/config/GLOSSARY.md (Template Directory); docs/adr/0003-index-based-dialog-selection.md (non-TTY defaults); src/template/loader.rs (include resolution).

  Scenario: A local template takes precedence over a global template
    Given a trusted project with local and global daily templates
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "local"

  Scenario: An exact global filename wins over a local stem-only match
    Given a trusted project with local and global daily templates
    When I run "traces template -i daily.md --dry-run"
    Then the exit code is 0
    And stdout is "global"

  Scenario: An unsafe template identifier fails before lookup
    Given a trusted project with a daily template
    When I run "traces template -i ../daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::invalid_identifier
      """

  Scenario: An ambiguous template stem refuses to select a file
    Given a trusted project with ambiguous daily templates
    When I run "traces template -i daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::ambiguous
      """

  Scenario: A missing include marked ignore missing renders empty
    Given a trusted project with a template that ignores a missing include
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "beforeafter"

  Scenario: An unsafe include remains an error when marked ignore missing
    Given a trusted project with a template that ignores an unsafe include
    When I run "traces template -i daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::render_failed
      """
