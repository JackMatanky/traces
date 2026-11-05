Feature: Query sorting orders rows and rejects invalid sort requests
  Source: ADR-0008, ADR-0005, src/query/GLOSSARY.md, src/cli/GLOSSARY.md.

  Scenario: An explicit ascending sort places missing ratings first
    Given a trusted project with a note "rated.md" containing:
      """
      ---
      rating: 9
      ---
      """
    And a note "unrated.md" containing:
      """
      # Unrated
      """
    When I run "traces list --sort rating --asc"
    Then the exit code is 0
    And stdout is:
      """
      - unrated.md
      - rated.md
      """

  Scenario: Default descending sort places missing ratings last
    Given a trusted project with a note "zz-rated.md" containing:
      """
      ---
      rating: 9
      ---
      """
    And a note "unrated.md" containing:
      """
      # Unrated
      """
    When I run "traces list --sort rating"
    Then the exit code is 0
    And stdout is:
      """
      - zz-rated.md
      - unrated.md
      """

  Scenario: Conflicting sort flags reject the query
    Given a trusted project
    When I run "traces list --sort file.name --asc --desc"
    Then the exit code is 2
    And stdout is empty

  Scenario: Unknown sort fields report a stable diagnostic code
    Given a trusted project with a note "a.md" containing:
      """
      # A
      """
    When I run "traces list --sort file.nam"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::query::failed
      """
    And stdout is empty

  Scenario: Chained sort and limit preserve the order of equal-rated pages
    Given a trusted project with two notes rated nine and one rated three
    And a template "report.md" containing:
      """
      {{ query.from("notes/").sort("rating").limit(2).list("file.path") }}{{ query.from("notes/").sort("rating").list("file.path") }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is:
      """
      - notes/a.md
      - notes/b.md
      - notes/a.md
      - notes/b.md
      - notes/c.md
      """

  Scenario: Explicit descending sort places larger ratings before smaller ratings
    Given a trusted project with a note "alpha.md" containing:
      """
      ---
      rating: 3
      ---
      """
    And a note "zulu.md" containing:
      """
      ---
      rating: 9
      ---
      """
    When I run "traces list --sort rating --desc"
    Then the exit code is 0
    And stdout is:
      """
      - zulu.md
      - alpha.md
      """

  Scenario: A prefixed sort key overrides the default direction
    Given a trusted project with a note "low.md" containing:
      """
      ---
      rating: 3
      ---
      """
    And a note "high.md" containing:
      """
      ---
      rating: 9
      ---
      """
    When I run "traces list --sort +rating"
    Then the exit code is 0
    And stdout is:
      """
      - low.md
      - high.md
      """

  Scenario: An ascending flag without a sort field is rejected
    Given a trusted project
    When I run "traces list --asc"
    Then the exit code is 2
    And stdout is empty
