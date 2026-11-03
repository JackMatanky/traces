Feature: Schema selection and calendar values at the project boundary
  Source: ADR-0006, ADR-0007, ADR-0009, src/temporal/GLOSSARY.md.

  Scenario: A local clock field matches its UTC instant in another time zone
    Given a trusted project with a note "meeting.md" containing:
      """
      ---
      due: 2026-07-29T14:30:00
      ---
      """
    When I run "traces list --where due==2026-07-29T18:30:00Z" in time zone "America/New_York"
    Then the exit code is 0
    And stdout is:
      """
      - meeting.md
      """

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
    And file "topic_note.md" contains:
      """
      Category: Rust Programming (rust)
      """

  Scenario: An offset-dated note matches the same instant to a nanosecond
    Given a trusted project with two offset-dated notes one nanosecond apart
    When I run "traces list --where due==2026-07-29T12:30:00.123456789Z"
    Then the exit code is 0
    And stdout is:
      """
      - matching.md
      """

  Scenario: A bare date filter selects only the due note
    Given a trusted project with a date, a different date, and an undated note
    When I run "traces list --where due==2026-01-01"
    Then the exit code is 0
    And stdout is:
      """
      - due.md
      """

  Scenario: A date equals the midnight clock value in a filter
    Given a trusted project with a date, a different date, and an undated note
    When I run "traces list --where due==2026-01-01T00:00:00Z"
    Then the exit code is 0
    And stdout is:
      """
      - due.md
      """

  Scenario: Adding one month to January's final day clamps to February's final day
    Given a trusted project with a template "clamp.md" containing:
      """
      {{ "2026-01-31" | date_add(1, unit="months") }}
      """
    When I run "traces template -i clamp --no-input --dry-run"
    Then the exit code is 0
    And stdout is "2026-02-28"

  Scenario: A query and a template agree on the same month-end clamp
    Given a trusted project with two month-end notes and a calendar template
    When I run "traces template -i calendar --no-input --dry-run"
    Then the exit code is 0
    And stdout is "1|2026-02-28"

  Scenario: Calendar functions filter notes from indexed dates
    Given a trusted project with a date, a different date, and an undated note
    When I list notes with filter:
      """
      date_add(due,1,"months")==2026-02-01
      """
    Then the exit code is 0
    And stdout is:
      """
      - due.md
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

  Scenario: An unknown schema in a template fails with a diagnostic
    Given a trusted library with a template that names an unknown schema
    When I run "traces template -i missing --no-input --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::render_failed
      """
    And stdout is empty

  Scenario: Equal durations shift differently over the spring clock change
    Given a trusted project with a calendar comparison across the spring clock change
    When I run "traces template -i spring --no-input --dry-run" in time zone "America/New_York"
    Then the exit code is 0
    And stdout is "1|1|0"

  Scenario: Calendar differences count the local day and elapsed hours separately
    Given a trusted project with a calendar comparison across the spring clock change
    When I run "traces template -i boundaries --no-input --dry-run" in time zone "America/New_York"
    Then the exit code is 0
    And stdout is "1|1"
