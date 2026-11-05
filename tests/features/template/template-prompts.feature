Feature: Template prompts fall back to defaults without an interactive terminal
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
