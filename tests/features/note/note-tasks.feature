Feature: Tasks keep their order, nesting, coordinates and filters
  Source: src/note/GLOSSARY.md, src/query/GLOSSARY.md, src/GLOSSARY.md.

  Scenario: Nested tasks retain their document order and parent coordinates
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Plan
        - [x] Draft
          - [ ] Review
      - [ ] Release
      """
    When I run "traces task -l"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Plan (plan.md:1)
        - [x] Draft (plan.md:2)
          - [ ] Review (plan.md:3)
      - [ ] Release (plan.md:4)
      """

  Scenario: Parent coordinates distinguish siblings from nested tasks
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Plan
        - [x] Draft
          - [ ] Review
      - [ ] Release
      """
    When I count tasks with "list.parent == 2"
    Then the exit code is 0
    And stdout is:
      """
      1
      """

  Scenario: A task's inline field does not leak to its sibling
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Draft [Owner:: Ada]
      - [ ] Review [Owner:: Bea]
      """
    When I filter tasks by "owner == 'Ada'"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Draft [Owner:: Ada] (plan.md)
      """

  Scenario: Task output keeps all known and unknown markers at their nesting depth
    Given a trusted project with a note "tasks.md" containing:
      """
      - [ ] Root task
      - [x] Done plain
        - [/] In progress
          - [-] Cancelled
        - [!] On hold
      - [?] Unknown marker
      """
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Root task (tasks.md)
      - [x] Done plain (tasks.md)
        - [/] In progress (tasks.md)
          - [-] Cancelled (tasks.md)
        - [!] On hold (tasks.md)
      - [?] Unknown marker (tasks.md)
      """

  Scenario: Task coordinates keep every known and unknown marker's source line
    Given a trusted project with a note "tasks.md" containing:
      """
      - [ ] Root task
      - [x] Done plain
        - [/] In progress
          - [-] Cancelled
        - [!] On hold
      - [?] Unknown marker
      """
    When I run "traces task -l"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Root task (tasks.md:1)
      - [x] Done plain (tasks.md:2)
        - [/] In progress (tasks.md:3)
          - [-] Cancelled (tasks.md:4)
        - [!] On hold (tasks.md:5)
      - [?] Unknown marker (tasks.md:6)
      """

  Scenario: Configured tag filters exclude untagged checklist items from tasks
    Given a trusted project with task tag filter "todo" and a note "a.md" containing:
      """
      # A
      - [ ] one #todo
      - [ ] two #other
      - [ ] three #todo/active
      """
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] one (a.md)
      """

  Scenario: A parent task is not fully complete while a descendant remains open
    Given a trusted project with a note "plan.md" containing:
      """
      - [x] Plan
        - [x] Draft
          - [ ] Review
      - [x] Publish
      """
    When I filter tasks by "list.fully_complete == true"
    Then the exit code is 0
    And stdout is:
      """
          - [ ] Review (plan.md)
      - [x] Publish (plan.md)
      """

  Scenario: Task output removes date priority and inline field annotations
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Submit report 📅 2026-12-01 🔺 [Due:: 2026-12-02]
      """
    When I run "traces task"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Submit report (plan.md)
      """

  Scenario: Task date shorthand appears as a query field
    Given a trusted project with a note "plan.md" containing:
      """
      - [ ] Submit report 📅 2026-12-01
      """
    When I filter tasks by "list.due == '2026-12-01'"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Submit report (plan.md)
      """

  Scenario: The todo shortcut includes every incomplete task
    Given a trusted project with a note "items.md" containing:
      """
      - [ ] Bravo task
      - [x] Charlie done
      - [/] Alpha in-progress
      - [!] Delta on-hold
      - [?] Echo unknown
      """
    When I run "traces task --todo"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Bravo task (items.md)
      - [/] Alpha in-progress (items.md)
      - [!] Delta on-hold (items.md)
      - [?] Echo unknown (items.md)
      """
    And filtering tasks by "status == 'x'" yields no results

  Scenario: The done shortcut includes only completed tasks
    Given a trusted project with a note "items.md" containing:
      """
      - [ ] Bravo task
      - [x] Charlie done
      - [/] Alpha in-progress
      - [!] Delta on-hold
      """
    When I run "traces task --done"
    Then the exit code is 0
    And stdout is:
      """
      - [x] Charlie done (items.md)
      """

  Scenario: The status shortcut matches the exact task marker
    Given a trusted project with a note "items.md" containing:
      """
      - [ ] Bravo task
      - [x] Charlie done
      - [/] Alpha in-progress
      - [!] Delta on-hold
      """
    When I run "traces task --status /"
    Then the exit code is 0
    And stdout is:
      """
      - [/] Alpha in-progress (items.md)
      """

  Scenario: Date shorthand in a plain list item is not a task date
    Given a trusted project with a note "notes/plan.md" containing:
      """
      - Plain note 📅 2026-12-01
      - [ ] Submit report 📅 2026-12-01
      """
    And a template "dates.md" containing:
      """
      {{ lists.from("notes/").where("list.due == '2026-12-01'") | length }}
      """
    When I run "traces template -i dates --dry-run --no-input"
    Then the exit code is 0
    And stdout is "1"
