Feature: Template documents resolve and render within project boundaries
  Source: src/template/GLOSSARY.md (Template Resolution, Template Output Path, Write Mode, Commit Policy); src/config/GLOSSARY.md (Template Directory); docs/adr/0003-index-based-dialog-selection.md (non-TTY defaults); src/template/loader.rs (include resolution).

  Scenario: Non-TTY template prompts use their defined fallback answers
    Given a trusted project with a daily template that asks for different prompt answers
    When I preview the template without a terminal
    Then the exit code is 0
    And stdout is "anon||False|calm|0"

  Scenario: No-input preview uses the defined prompt defaults
    Given a trusted project with a daily template that asks for different prompt answers
    When I run "traces template -i daily --no-input --dry-run"
    Then the exit code is 0
    And stdout is "anon||False|calm|0"

  Scenario: The template picker refuses to run without an interactive terminal
    Given a trusted project with a daily template
    When I run "traces template"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::picker_failed
      """
    And stdout is empty

  Scenario: The template picker reports when no templates are available
    Given a trusted project without templates
    When I run "traces template"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::no_templates
      """

  Scenario: The default template output creates a rendered Markdown file
    Given a trusted project with a daily template
    When I run "traces template -i daily --no-input"
    Then the exit code is 0
    And the rendered file "daily.md" exists with content "# Today"

  Scenario: A non-interactive output collision preserves the existing file
    Given a trusted project with a daily template and an existing output file
    When I run "traces template -i daily"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::output_exists
      """
    And the existing file "daily.md" still contains "keep me"

  Scenario: Force replaces an existing template output file
    Given a trusted project with a daily template and an existing output file
    When I run "traces template -i daily --force"
    Then the exit code is 0
    And the rendered file "daily.md" exists with content "# Today"

  Scenario: Dry-run previews rendered content without writing a file
    Given a trusted project with a daily template
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "# Today"
    And no output file "daily.md" exists

  Scenario: Dry-run previews over an existing file without replacing it
    Given a trusted project with a daily template and an existing output file
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "# Today"
    And the existing file "daily.md" still contains "keep me"

  Scenario: A local template takes precedence over a global template
    Given a trusted project with local and global daily templates
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "local"

  Scenario: An exact global filename wins over a local stem-only match
    Given a trusted project with local and global daily templates
    When I run "traces template -i daily.md --dry-run"
    Then the exit code is 0
    And stdout is "global"

  Scenario: An unsafe template identifier fails before lookup
    Given a trusted project with a daily template
    When I run "traces template -i ../daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::invalid_identifier
      """

  Scenario: An ambiguous template stem refuses to select a file
    Given a trusted project with ambiguous daily templates
    When I run "traces template -i daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::ambiguous
      """

  Scenario: The command output path overrides a template declaration
    Given a trusted project with a daily template that declares an output path
    When I run "traces template -i daily --output override.md"
    Then the exit code is 0
    And the rendered file "override.md" exists with content "# Declared"
    And no output file "declared.md" exists

  Scenario: A declared output path takes precedence over the default path
    Given a trusted project with a daily template that declares an output path
    When I run "traces template -i daily"
    Then the exit code is 0
    And the rendered file "declared.md" exists with content "# Declared"
    And no output file "daily.md" exists

  Scenario: A template output path cannot escape the project root
    Given a trusted project with a daily template that declares a path outside the project root
    When I run "traces template -i daily"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::output_escapes_root
      """
    And the escaped output file is not created

  Scenario: A missing include marked ignore missing renders empty
    Given a trusted project with a template that ignores a missing include
    When I run "traces template -i daily --dry-run"
    Then the exit code is 0
    And stdout is "beforeafter"

  Scenario: An unsafe include remains an error when marked ignore missing
    Given a trusted project with a template that ignores an unsafe include
    When I run "traces template -i daily --dry-run"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::template::render_failed
      """
