Feature: Schema file classes and fields select notes at the project boundary
  Source: ADR-0006, ADR-0007, ADR-0009, src/temporal/GLOSSARY.md.

  Scenario: A descendant file class selects notes from later generations
    Given a trusted library with book, science fiction, and novel classes
    When I run "traces list --from @book*"
    Then the exit code is 0
    And stdout is:
      """
      - shelf/book.md
      - shelf/novel.md
      """

  Scenario: An exact file class excludes its descendants
    Given a trusted library with book, science fiction, and novel classes
    When I run "traces list --from @book"
    Then the exit code is 0
    And stdout is:
      """
      - shelf/book.md
      """

  Scenario: An unknown file class selects no unrelated notes
    Given a trusted library with book, science fiction, and novel classes
    When I run "traces list --from @horror"
    Then the exit code is 0
    And stdout is empty

  Scenario: A configured file class field selects the matching note
    Given a trusted library with kind as its file class field
    When I run "traces list --from @Novel"
    Then the exit code is 0
    And stdout is:
      """
      - novel.md
      """

  Scenario: A configured file class field does not invent another class
    Given a trusted library with kind as its file class field
    When I run "traces list --from @Book"
    Then the exit code is 0
    And stdout is empty

  Scenario: A template reads the labels and values of a file-sourced select field
    Given a trusted project with a file-sourced category field
    When I run "traces template -i topic_note --no-input"
    Then the exit code is 0
    And file "topic_note.md" is:
      """
      Category: Rust Programming (rust); Python Programming (python)
      """

  Scenario: A cycle in file class inheritance stops selection
    Given a trusted library with a cycle in its file classes
    When I run "traces list --from @book"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::query::schema_failed
      """
    And stdout is empty

  Scenario: An invalid file class does not block a valid file class
    Given a trusted library with one invalid file class and one valid file class
    When I run "traces list --from @book"
    Then the exit code is 0
    And stdout is:
      """
      - book.md
      """

  Scenario: An unknown schema field fails with a diagnostic
    Given a trusted library with a template that names an unknown schema field
    When I run "traces template -i missing_field --no-input --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::render_failed
      """
    And stdout is empty

  Scenario: An unknown template schema class falls back to exact matching
    Given a trusted library with a template that queries an unknown schema class
    When I run "traces template -i unknown_class --no-input --dry-run"
    Then the exit code is 0
    And stdout is "1"

  Scenario: An unknown schema in a template fails with a diagnostic
    Given a trusted library with a template that names an unknown schema
    When I run "traces template -i missing --no-input --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::render_failed
      """
    And stdout is empty
