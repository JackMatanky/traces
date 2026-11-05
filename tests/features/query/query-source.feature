Feature: Query sources select the notes and tasks a query reads
  Source: ADR-0008, ADR-0005, src/query/GLOSSARY.md, src/cli/GLOSSARY.md.

  Scenario: A bare Markdown path selects only tasks from that note
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Alpha item
      - [x] Beta item
      """
    And a note "other.md" containing:
      """
      - [ ] Gamma item
      """
    When I run "traces task --from todo.md"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Alpha item (todo.md)
      - [x] Beta item (todo.md)
      """

  Scenario: A bare Markdown path limits task counts to one note
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Alpha item
      - [x] Beta item
      """
    And a note "other.md" containing:
      """
      - [ ] Gamma item
      """
    When I run "traces task --from todo.md --count"
    Then stdout is:
      """
      2
      """
    And stderr is empty

  Scenario: A missing Markdown path gives no task rows
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] real task
      """
    When I run "traces task --from missing.md"
    Then the exit code is 0
    And stdout is empty
    And stderr reports "0 task(s) from " for the project root

  Scenario: An unknown class selector returns no pages
    Given a trusted project with a note "book.md" containing:
      """
      # Book
      """
    When I run "traces list --from @horror"
    Then the exit code is 0
    And stdout is empty
    And stderr reports "0 page(s) from " for the project root

  Scenario: Source expressions respect not and and before or
    Given a trusted project with a note "books/dune.md" containing:
      """
      #fiction
      """
    And a note "books/manual.md" containing:
      """
      #book #reference
      """
    When I select pages from:
      """
      #fiction or #book and not #fiction
      """
    Then the exit code is 0
    And stdout is:
      """
      - books/dune.md
      - books/manual.md
      """

  Scenario: A folder source expression excludes notes outside the folder
    Given a trusted project with a note "notes/dune.md" containing:
      """
      # Dune
      """
    And a note "other.md" containing:
      """
      # Other
      """
    When I select pages from:
      """
      notes/
      """
    Then the exit code is 0
    And stdout is:
      """
      - notes/dune.md
      """

  Scenario: A template source expression selects only notes in the chosen folder
    Given a trusted project with a note "notes/a.md" containing:
      """
      # Alpha
      """
    And a note "notes/b.md" containing:
      """
      # Beta
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").count() }} chosen, {{ query.from().count() }} total
      """
    When I run "traces template -i report --dry-run --no-input"
    Then stdout is "2 chosen, 3 total"

  Scenario: A grouped source expression overrides default precedence
    Given a trusted project with a note "book.md" containing:
      """
      #book
      """
    And a note "fiction.md" containing:
      """
      #fiction
      """
    When I select pages from:
      """
      (#book or #fiction) and not #fiction
      """
    Then the exit code is 0
    And stdout is:
      """
      - book.md
      """

  Scenario: A descendant class selector includes notes with a child class
    Given a trusted project with a note "shelf/novel.md" containing:
      """
      ---
      class: novel
      ---
      """
    And a schema "book" containing:
      """
      # Base class
      """
    And a schema "novel" containing:
      """
      extends = ["book"]
      """
    When I run "traces list --from @book*"
    Then stdout is:
      """
      - shelf/novel.md
      """

  Scenario: A schema file field selects notes through a template query
    Given a trusted project with a note "covers/dune.md" containing:
      """
      ---
      class: book
      ---
      """
    And a schema "book" containing:
      """
      [fields.cover]
      type = "file"
      folders = ["covers"]
      ext = "md"
      class = ["book"]
      """
    And a template "report.md" containing:
      """
      {{ query.from(schema.get("book").field("cover")).list("file.path") }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then stdout is:
      """
      - covers/dune.md
      """
