Feature: Query filters select notes by field, tag and date
  Source: ADR-0008, ADR-0005, src/query/GLOSSARY.md, src/cli/GLOSSARY.md.

  Scenario: Invalid filter expressions report a stable diagnostic code
    Given a trusted project with a note "a.md" containing:
      """
      # A
      """
    When I filter pages where:
      """
      not a valid expression
      """
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::query::failed
      """
    And stdout is empty

  Scenario: Filtering by a numeric field selects matching pages
    Given a trusted project with a note "high.md" containing:
      """
      ---
      rating: 9
      ---
      """
    And a note "low.md" containing:
      """
      ---
      rating: 3
      ---
      """
    When I filter pages where:
      """
      rating >= 8
      """
    Then the exit code is 0
    And stdout is:
      """
      - high.md
      """

  Scenario: Filtering by a tag includes nested tags
    Given a trusted project with a note "dune.md" containing:
      """
      #book/fiction
      """
    And a note "manual.md" containing:
      """
      #manual
      """
    When I select pages from:
      """
      #book
      """
    Then the exit code is 0
    And stdout is:
      """
      - dune.md
      """

  Scenario: A filter matches nested tags by prefix
    Given a trusted project with a note "dune.md" containing:
      """
      #book/fiction
      """
    And a note "manual.md" containing:
      """
      #manual
      """
    When I filter pages where:
      """
      contains(tags, "#book")
      """
    Then the exit code is 0
    And stdout is:
      """
      - dune.md
      """

  Scenario: A date function filters notes by their computed due date
    Given a trusted project with a note "due.md" containing:
      """
      ---
      due: 2026-07-29
      ---
      """
    And a note "later.md" containing:
      """
      ---
      due: 2026-08-01
      ---
      """
    When I filter pages where:
      """
      date_add(due, 17, "days") == "2026-08-15"
      """
    Then the exit code is 0
    And stdout is:
      """
      - due.md
      """

  Scenario: Chained filters match one combined filter expression
    Given a trusted project with a note "notes/high.md" containing:
      """
      ---
      rating: 9
      ---
      """
    And a note "notes/low.md" containing:
      """
      ---
      rating: 3
      ---
      """
    And a note "notes/over.md" containing:
      """
      ---
      rating: 11
      ---
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").filter("rating >= 5").filter("rating < 10").list("file.path") }}{{ query.from("notes/").filter("rating >= 5 and rating < 10").list("file.path") }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then stdout is:
      """
      - notes/high.md
      - notes/high.md
      """

  Scenario: An obsolete task field reports a stable diagnostic code
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Review the draft
      """
    When I filter tasks where:
      """
      task.completed == true
      """
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::query::failed
      """
    And stdout is empty
