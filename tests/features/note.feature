Feature: Notes keep their fields, tags, and tasks available to queries
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

  Scenario: The todo shortcut includes every incomplete task
    Given a trusted project with a note "items.md" containing:
      """
      - [ ] Bravo task
      - [x] Charlie done
      - [/] Alpha in-progress
      - [!] Delta on-hold
      """
    When I run "traces task --todo"
    Then the exit code is 0
    And stdout is:
      """
      - [ ] Bravo task (items.md)
      - [/] Alpha in-progress (items.md)
      - [!] Delta on-hold (items.md)
      """

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
