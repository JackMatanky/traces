Feature: Tags and links resolve to the notes that carry them
  Source: src/note/GLOSSARY.md, src/query/GLOSSARY.md, src/GLOSSARY.md.

  Scenario: A parent tag selects notes carrying its descendant tag
    Given a trusted project with a note "active.md" containing:
      """
      Work on #projects/active today.
      """
    And another note "other.md" containing:
      """
      Work on #misc today.
      """
    When I list notes with tag "#projects"
    Then the exit code is 0
    And stdout is:
      """
      - active.md
      """

  Scenario: Wikilink anchors do not change the linked note's path
    Given a trusted project with a note "target.md" containing:
      """
      # Target
      """
    And another note "linker.md" containing:
      """
      See [[target#intro#details]].
      """
    When I filter notes by "contains(inlinks, 'linker.md')"
    Then the exit code is 0
    And stdout is:
      """
      - target.md
      """

  Scenario: A tag on a list item selects its note
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Draft #projects/active
      """
    When I list notes with tag "#projects"
    Then the exit code is 0
    And stdout is:
      """
      - plan.md
      """
