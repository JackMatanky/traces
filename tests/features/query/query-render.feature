Feature: Query output renders the same rows as tables, lists, counts and pipelines
  Source: ADR-0008, ADR-0005, src/query/GLOSSARY.md, src/cli/GLOSSARY.md.

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
