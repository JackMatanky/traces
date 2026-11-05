Feature: Inline fields attach typed values to notes
  Source: src/note/GLOSSARY.md, src/query/GLOSSARY.md, src/GLOSSARY.md.

  Scenario: Bare bracketed and parenthesized inline fields select notes
    Given a trusted project with a note "bare.md" containing:
      """
      Owner:: Ada
      """
    And another note "bracketed.md" containing:
      """
      [Owner:: Ada]
      """
    And another note "parenthesized.md" containing:
      """
      (Owner:: Ada)
      """
    When I filter notes by "owner == 'Ada'"
    Then stdout is:
      """
      - bare.md
      - bracketed.md
      - parenthesized.md
      """

  Scenario: Inline durations compare by elapsed time
    Given a trusted project with a note "matching.md" containing:
      """
      [duration:: 1h 30m]
      """
    And another note "different.md" containing:
      """
      [duration:: 1h 31m]
      """
    When I filter notes by "duration == '90m'"
    Then the exit code is 0
    And stdout is:
      """
      - matching.md
      """

  Scenario: Field keys match without regard to casing or punctuation
    Given a trusted project with a note "plan.md" containing:
      """
      ---
      Due!Date: 2026-12-01
      ---
      """
    When I filter notes by "duedate == '2026-12-01'"
    Then the exit code is 0
    And stdout is:
      """
      - plan.md
      """

  Scenario: Inline fields in code blocks do not select a note
    Given a trusted project with a note "real.md" containing:
      """
      Owner:: Ada
      """
    And another note "code.md" containing:
      """
      ```text
      Owner:: Ada
      ```
      """
    When I filter notes by "owner == 'Ada'"
    Then the exit code is 0
    And stdout is:
      """
      - real.md
      """

  Scenario: Bracketed inline fields accept multiword keys
    Given a trusted project with a note "plan.md" containing:
      """
      [Full Name:: Ada]
      """
    When I show the field "Full Name" for notes
    Then the exit code is 0
    And stdout is:
      """
      | Full Name |
      |-----------|
      | Ada       |
      """

  Scenario: Small numbers and dates in inline fields retain their types
    Given a trusted project with a note "matching.md" containing:
      """
      [score:: 7] [due:: 2026-12-01]
      """
    And another note "different.md" containing:
      """
      [score:: 8] [due:: 2026-12-01]
      """
    When I filter notes by "score == 7 and due == '2026-12-01'"
    Then the exit code is 0
    And stdout is:
      """
      - matching.md
      """
