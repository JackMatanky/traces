Feature: Queries reflect edits to notes and settings between runs
  Source: src/index/GLOSSARY.md, src/cli/GLOSSARY.md, ADR-0009.

  Scenario: An edited task appears in the next query
    Given an indexed project with the original task
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] rewritten task text (a.md)
      """

  Scenario: A removed tag no longer matches after an edit
    Given an indexed project with a changed tag
    When I run "traces list --from #removed"
    Then the exit code is 0
    And stdout is empty

  Scenario: A new tag matches after an edit
    Given an indexed project with a changed tag
    When I run "traces list --from #added"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """

  Scenario: An index setting edit changes the next query without changing notes
    Given an indexed project with its class field changed
    When I run "traces list --from @Novel"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """

  Scenario: An old class no longer matches after a class field edit
    Given an indexed project with its class field changed
    When I run "traces list --from @Book"
    Then the exit code is 0
    And stdout is empty

  Scenario: A rendered note appears in the next list query
    Given a trusted project with a rendered tagged note
    When I run "traces list --from #fromtemplate"
    Then the exit code is 0
    And stdout is:
      """
      - daily.md
      """

  Scenario: Bare and prefixed frontmatter tags both match a tag query
    Given an indexed project with bare and prefixed frontmatter tags
    When I run "traces list --from #project"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      - b.md
      """
