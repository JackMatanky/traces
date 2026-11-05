Feature: Frontmatter fields take part in queries despite malformed or duplicate input
  Source: src/note/GLOSSARY.md, src/query/GLOSSARY.md, src/GLOSSARY.md.

  Scenario: Malformed frontmatter does not prevent a note from appearing
    Given a trusted project with a note "broken.md" containing:
      """
      ---
      invalid: [
      ---
      # Still a note
      """
    When I run "traces list"
    Then the exit code is 0
    And stdout is:
      """
      - broken.md
      """

  Scenario: Duplicate frontmatter fields use the last value
    Given a trusted project with a note "draft.md" containing:
      """
      ---
      status: pending
      status: approved
      ---
      """
    When I filter notes by "status == 'approved'"
    Then the exit code is 0
    And stdout is:
      """
      - draft.md
      """
    And filtering notes by "status == 'pending'" yields no results

  Scenario: Frontmatter takes priority over a duplicate inline field
    Given a trusted project with a note "plan.md" containing:
      """
      ---
      owner: Ada
      ---
      Owner:: Bea
      """
    When I filter notes by "owner == 'Ada'"
    Then the exit code is 0
    And stdout is:
      """
      - plan.md
      """
    And filtering notes by "owner == 'Bea'" yields no results

  Scenario: Frontmatter and template YAML interpret leading zero values alike
    Given a trusted project with a note "notes/a.md" containing:
      """
      ---
      code: 012
      ---
      """
    And a template "parity.md" containing:
      """
      {{ query.from("notes/").where("code == '012'") | length }}|{{ ('code: 012' | from_yaml).code }}
      """
    When I run "traces template -i parity --dry-run --no-input"
    Then the exit code is 0
    And stdout is "1|012"
