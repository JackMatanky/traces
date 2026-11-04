# Scenario ownership

The reverse gate pairs every scenario title in `tests/features/` with a
`covered:` row in `.scratch/bdd-strategy/research/10-invariance-map.md`
or with a row in the table below. This file is the second half of the
`check:layers` scenario gate.

| Scenario title | Owner row IDs or reason |
|---|---|
| A bare Markdown path limits task counts to one note | 02-G2 |
| A missing Markdown path gives no task rows | 02-G2 |
| A task table displays the default columns and task states | 01-G30 |
| A task table displays requested columns | 01-G30 |
| Task rows preserve their order in the note | 02-G4 |
| Conflicting sort flags reject the query | 02-G6 |
| Invalid filter expressions report a stable diagnostic code | 02-G9 |
| Filtering by a numeric field selects matching pages | 02-G5 |
| Filtering by a tag includes nested tags | 02-G5 |
| A folder source expression excludes notes outside the folder | 02-G1 |
| A template source expression selects only notes in the chosen folder | 02-G1 |
| A filter matches nested tags by prefix | 02-G5 |
| A task list renders the same output as a pipeline | 02-G8 |
| A grouped source expression overrides default precedence | 02-G1 |
| A descendant class selector includes notes with a child class | 04-B1 |
| Explicit descending sort places larger ratings before smaller ratings | 02-G6 |
| A prefixed sort key overrides the default direction | 02-G6 |
| Chained filters match one combined filter expression | 02-G5 |
| A page list renders the same output as a pipeline | 02-G8 |
| A page count renders the same value as a pipeline | 02-G8 |
| An ascending flag without a sort field is rejected | 02-G6 |
| A schema file field selects notes through a template query | 04-B2 |
| An obsolete task field reports a stable diagnostic code | 02-G9 |
| A local clock field matches its UTC instant in another time zone | 04-T2 |
| An exact file class excludes its descendants | 04-B1 |
| An unknown file class selects no unrelated notes | 02-G3 |
| A configured file class field does not invent another class | 04-B2 |
| An unknown schema in a template fails with a diagnostic | 04-S7 |
| An unknown schema field fails with a diagnostic | 04-S7 |
| An unknown template schema class falls back to exact matching | 04-S7 |
| No-input preview uses the defined prompt defaults | 01-G7 |
| The template picker reports when no templates are available | 01-G11 |
| The default template output creates a rendered Markdown file | 01-G30 |
| Force replaces an existing template output file | 02-G12 |
| A local template takes precedence over a global template | 02-G10 |
| An exact global filename wins over a local stem-only match | 02-G10 |
| An ambiguous template stem refuses to select a file | 02-G10 |
| The command output path overrides a template declaration | 02-G11 |
| A declared output path takes precedence over the default path | 02-G11 |
| A missing include marked ignore missing renders empty | 02-G18 |
| Indexed configuration is listed as a tracked path | 03-P25 |
| Revoking trust leaves the configuration tracked | 01-G26 |
| Cleaning preserves a live tracked configuration path | 05-file_tracker |
| Repeat initialization leaves the existing configuration untouched | 01-G15 |
| An absent local task filter uses the global filter | 01-G21 |
| A local task filter replaces the global filter | 01-G21 |
| Missing local configuration fails at discovery | 01-G22 |
| Untrusted local configuration fails at trust verification | 01-G22 |
| Malformed trusted configuration fails at parsing | 01-G22 |
| Invalid trusted configuration value fails at validation | 01-G22 |
| A removed tag no longer matches after an edit | 03-P02 |
| A new tag matches after an edit | 03-P02 |
| Deleting an ambiguous link target resolves the surviving target | 03-P03 |
| Deleting a note removes its path from the next query | 03-P01 |
| A renamed target retains its inlink | 03-P03 |
| A list query creates a store without an index command | 03-P01 |
| An old class no longer matches after a class field edit | 04-B1 |
| A link indexed in a separate run appears on its target | 03-P01 |
| Deleting the index store preserves query results | 03-P01 |
| A second query from the stored index matches the first | 03-P01 |
| Bare and prefixed frontmatter tags both match a tag query | 03-N08 |
| Indexing records the project configuration as tracked | 03-P25 |
| Parent coordinates distinguish siblings from nested tasks | 03-N04 |
| Task coordinates keep every known and unknown marker's source line | 03-N09 |
| Task date shorthand appears as a query field | 03-N12 |
| Field keys match without regard to casing or punctuation | 05-field |
| The todo shortcut includes every incomplete task | 05-task |
| The done shortcut includes only completed tasks | 05-task |
| The status shortcut matches the exact task marker | 03-N09 |
| Inline fields in code blocks do not select a note | 03-N05 |
| A tag on a list item selects its note | 03-N08 |
| Bracketed inline fields accept multiword keys | 03-N06 |
| Small numbers and dates in inline fields retain their types | 03-N07 |
| Task count flag suppresses summary on stderr | 01-G3 |
| Missing template exits with code one and reports template not found diagnostic | 01-G4 |
