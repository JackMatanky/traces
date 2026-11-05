Feature: Calendar and clock values compare across time zones and calendar changes
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
