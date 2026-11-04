Feature: Query results select and render notes through the command line
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

  Scenario: A page table renders one Markdown row per note
    Given a trusted project with a note "dune.md" containing:
      """
      ---
      rating: 9
      ---
      """
    And a note "emma.md" containing:
      """
      ---
      rating: 7
      ---
      """
    When I run "traces table --column file.name --column rating"
    Then the exit code is 0
    And stdout is:
      """
      | file.name | rating |
      |-----------|--------|
      | dune      | 9      |
      | emma      | 7      |
      """

  Scenario: A task table displays the default columns and task states
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Alpha item 📅 2026-10-01 🔺
      - [x] Beta item 📅 2026-10-05 🔽
      """
    When I run "traces task --table"
    Then the exit code is 0
    And stdout is:
      """
      | Task       | Status | Due        | Priority | File    |
      |------------|--------|------------|----------|---------|
      | Alpha item | Todo   | 2026-10-01 | highest  | todo.md |
      | Beta item  | Done   | 2026-10-05 | low      | todo.md |
      """

  Scenario: A task table displays requested columns
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Alpha item 📅 2026-10-01 🔺
      """
    When I run "traces task --table --column list.text --column list.priority"
    Then the exit code is 0
    And stdout is:
      """
      | list.text  | list.priority |
      |------------|---------------|
      | Alpha item | highest       |
      """

  Scenario: Task rows preserve their order in the note
    Given a trusted project with a note "todo.md" containing:
      """
      - [ ] Write the draft
      - [x] Send the draft
      """
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Write the draft (todo.md)
      - [x] Send the draft (todo.md)
      """

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

  Scenario: A template query renders method and pipeline tables alike
    Given a trusted project with a note "notes/dune.md" containing:
      """
      # Dune
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").table(["file.name"], ["file.name"]) }}{{ query.from("notes/") | table(["file.name"], ["file.name"]) }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is:
      """
      | file.name |
      |-----------|
      | dune      |
      | file.name |
      |-----------|
      | dune      |
      """

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

  Scenario: List rows include plain items and checkboxes in document order
    Given a trusted project with a note "notes/items.md" containing:
      """
      - Prepare a draft
      - [ ] Review the draft
      - [x] Publish the draft
      """
    And a template "report.md" containing:
      """
      {{ lists.from("notes/").list("list.text") }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is:
      """
      - Prepare a draft
      - Review the draft
      - Publish the draft
      """

  Scenario: Tasks exclude plain list items from their row count
    Given a trusted project with a note "notes/items.md" containing:
      """
      - Prepare a draft
      - [ ] Review the draft
      - [x] Publish the draft
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").count() }} page(s), {{ lists.from("notes/").count() }} list item(s), {{ tasks.from("notes/").count() }} task(s)
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is "1 page(s), 3 list item(s), 2 task(s)"

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

  Scenario: A task list renders the same output as a pipeline
    Given a trusted project with a note "notes/items.md" containing:
      """
      - [ ] Review the draft
      """
    And a template "report.md" containing:
      """
      {{ tasks.from("notes/").task_list() }}{{ tasks.from("notes/") | task_list }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Review the draft
      - [ ] Review the draft
      """

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

  Scenario: A page list renders the same output as a pipeline
    Given a trusted project with a note "notes/dune.md" containing:
      """
      # Dune
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").list("file.name") }}{{ query.from("notes/") | list("file.name") }}
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is:
      """
      - dune
      - dune
      """

  Scenario: A page count renders the same value as a pipeline
    Given a trusted project with a note "notes/dune.md" containing:
      """
      # Dune
      """
    And a template "report.md" containing:
      """
      {{ query.from("notes/").count() }} page(s), {{ query.from("notes/") | count }} page(s)
      """
    When I run "traces template -i report --dry-run --no-input"
    Then the exit code is 0
    And stdout is "1 page(s), 1 page(s)"

  Scenario: An ascending flag without a sort field is rejected
    Given a trusted project
    When I run "traces list --asc"
    Then the exit code is 2
    And stdout is empty

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
