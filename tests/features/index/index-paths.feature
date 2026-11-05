Feature: Queries reflect added, deleted and renamed notes between runs
  Source: src/index/GLOSSARY.md, src/cli/GLOSSARY.md, ADR-0009.

  Scenario: Deleting an ambiguous link target resolves the surviving target
    Given an indexed project with one of two link targets deleted
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "notes/foo.md" are "a.md"

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
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "archive/foo.md" are "a.md"

  Scenario: Adding a link target resolves its inlink on the next query
    Given an indexed project with a newly added link target
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "new.md" are "a.md"

  Scenario: A link indexed in a separate run appears on its target
    Given an indexed project with a link to foo
    When I run "traces table --column file.path --column inlinks"
    Then the exit code is 0
    And the inlinks for "notes/foo.md" are "a.md"

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

  Scenario: Hidden Git files and symlink notes are absent from queries
    Given an indexed project with hidden Git files and a symlink note
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - a.md
      """
