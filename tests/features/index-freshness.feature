Feature: Queries reflect project files and settings across runs
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

  Scenario: Deleting an ambiguous link target resolves the surviving target
    Given an indexed project with one of two link targets deleted
    When I run "traces table --column file.name --column inlinks"
    Then the exit code is 0
    And the row for "foo" contains "a.md"

  Scenario: Deleting a note removes its path from the next query
    Given an indexed project with one of two link targets deleted
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      - notes/foo.md
      """

  Scenario: Renaming a note removes the former path and resolves its link
    Given an indexed project with a link target renamed
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      - archive/foo.md
      """

  Scenario: A renamed target retains its inlink
    Given an indexed project with a link target renamed
    When I run "traces table --column file.name --column inlinks"
    Then the exit code is 0
    And the row for "foo" contains "a.md"

  Scenario: Adding a link target resolves its inlink on the next query
    Given an indexed project with a newly added link target
    When I run "traces table --column file.name --column inlinks"
    Then the exit code is 0
    And the row for "new" contains "a.md"

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

  Scenario: Inlinks render after a separate index run
    Given an indexed project with a link to foo
    When I run "traces table --column file.name --column inlinks"
    Then the exit code is 0
    And the row for "foo" contains "a.md"

  Scenario: Inlinks are sorted and deduplicated with empty sources omitted
    Given an indexed project with repeated and empty link sources
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "target.md" are "a.md, z.md"
    And the inlinks for "empty.md" are empty

  Scenario: Exact and implied paths resolve before a nearby basename
    Given an indexed project with exact implied and nearby links
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "notes/foo.md" are "notes/a.md, remote/a.md"
    And the inlinks for "remote/foo.md" are "root.md"

  Scenario: A rendered note appears in the next list query
    Given a trusted project with a rendered tagged note
    When I run "traces list --from #fromtemplate"
    Then the exit code is 0
    And stdout is:
      """
      - daily.md
      """

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

  Scenario: Indexing records the project configuration as tracked
    Given a trusted project that was indexed
    When I run "traces tracked list"
    Then the exit code is 0
    And stdout contains the project configuration path

  Scenario: Hidden Git files and symlink notes are absent from queries
    Given an indexed project with hidden Git files and a symlink note
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """
