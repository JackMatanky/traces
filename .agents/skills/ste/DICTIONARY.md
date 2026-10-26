# Approved-word substitutions

ASD-STE100 gives each approved word one meaning and one part of speech. When you write, replace the common word on the left with the approved word on the right.

This is a starter list, not the full ASD-STE100 dictionary. It covers the words that appear most in software and agent output.

Do not edit the marked table manually. Regenerate it with `.agents/skills/ste/scripts/check_ste.py render-dict`. Check for drift with `.agents/skills/ste/scripts/check_ste.py render-dict --check`.

<!-- BEGIN GENERATED: substitutions (from scripts/dictionary.json) -->
| Common word / phrase | Use instead | Note |
| --- | --- | --- |
| follow | obey | "follow" means "come after" only |
| ensure | make sure |  |
| utilize | use |  |
| leverage | use |  |
| facilitate | help |  |
| initiate | start |  |
| terminate | stop / end |  |
| approximately | about |  |
| prior to | before |  |
| subsequent to | after |  |
| following | after | context dependent |
| in order to | to |  |
| strip out | remove |  |
| get rid of | remove |  |
| fix up | repair |  |
| attempt | try |  |
| additional | more |  |
| commence | begin / start |  |
| modify | change |  |
| require | need |  |
| verify | check |  |
| obtain | get |  |
| perform | do / make | "make a test," not "perform a test" |
| via | with / by / through |  |
| due to the fact that | because |  |
| in the event that | if |  |
| a number of | some / many | state the number if you know it |
<!-- END GENERATED -->

## One meaning per word

Do not use a word with two meanings. Examples:

- **"fall"** is a verb only ("the value falls"). Do not write "in the fall."
- **"test"** is a noun. Use "make a test of," not "to test."
- **"close"** is a verb ("close the file"). For distance, use "near."
