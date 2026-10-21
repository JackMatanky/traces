# Ecosystem Claims Verification: Schemas, Types, and Vault Tooling

- **Target File:** `.scratch/arch-design/research/20-prep/round5/R1-ecosystem-claims.md`
- **Scope:** Primary-source verification of six external ecosystem claims from
  prior research rounds (Dendron, Tana, SilverBullet, Metadata Enforcer,
  Obsidian Properties, and Logseq DB) and their impact on Ticket 20's Answer.
- **Status:** Complete.

---

## 1. Dendron: Path-Binding of Schemas and Failure Modes

### 1.1 Verdict

**REFUTED / UNSUPPORTED** [V]. The claim from Round 4 Scout B that path-binding
of schemas was "proven fatal" or caused Dendron's collapse is an unsupported
scout exaggeration. Dendron did not bind schemas to directory folder paths,
schemas were explicitly optional/advisory, and project cessation was driven by
startup business realities rather than schema technical failure.

### 1.2 Primary Source Evidence

- **Dot-hierarchy matching, not filesystem folders [V]:** Dendron notes live in
  a flat folder where dot notation encodes hierarchy (`foo.bar.baz.md`). Schemas
  match these dot-delimited hierarchy identifiers via glob patterns on `id` or
  `pattern`:
  > "A schema is a YAML file with the following naming scheme `{name}.schema.yml`
  > Below is an example of a four-level hierarchy describing cli commands. The
  > `id` field of the schema is used to build a glob pattern which will be used
  > to match notes matching the schema."
  > — [Dendron Schemas Doc](https://wiki.dendron.so/notes/c5e5adde-5459-409b-b34d-a0d75cbb1052)
- **Schemas are optional and advisory [V]:**
  > "Think of schemas as an optional type system for your notes that describe
  > the hierarchy of your data and are represented as a hierarchy, themselves...
  > Dendron doesn't force you to use schemas if you don't want to. This is why
  > you can create notes that don't match any schema. Dendron will show you a `?`
  > next to these results. Sometimes rules schemas are meant to be broken."
  > — [Dendron Schemas Doc](https://wiki.dendron.so/notes/c5e5adde-5459-409b-b34d-a0d75cbb1052)
- **Namespaces allow arbitrary children [V]:**
  > "making a schema node a namespace automatically brings its immediate
  > children as part of the schema, even if they are not explicitly defined in
  > the schema definition. it will also automatically set the pattern to
  > `{namespace}.*`"
  > — [Dendron Schemas Doc](https://wiki.dendron.so/notes/c5e5adde-5459-409b-b34d-a0d75cbb1052)
- **Cross-vault precedence on conflicts [V]:**
  > "Schemas are cross vault. If you have multiple schemas with same starting
  > pattern, the last schema loaded takes precedence."
  > — [Making Your First Schema](https://wiki.dendron.so/notes/5U4eAiqshI67VxIL40KWH)
- **Refactoring support [V]:** Dendron provided built-in refactoring commands
  (`Dendron: Refactor Hierarchy`) using regex and capture groups to rename and
  reorganize hierarchies and matching schemas concurrently.
- **Cause of project shutdown [V]:** GitHub Discussion #3890 ("Development
  stopped?") and founder Kevin Lin's announcements document that Dendron ceased
  development because the commercial startup ran out of runway and the founder
  transitioned to OpenAI, not because dot-namespace schemas collapsed.
  > [GitHub Discussion #3890](https://github.com/dendronhq/dendron/discussions/3890)

### 1.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Ticket 20's Answer already
adopted path bindings (`[binding]` table with vault-relative globs) as default
fallback bindings for unclassed notes alongside tags and explicit frontmatter
classes. Refuting Scout B's alarmist claim confirms that supporting path
bindings is sound and poses no inherent architectural hazard.

---

## 2. Tana: Supertags Field Inheritance and Conflicting Field Definitions

### 2.1 Verdict

**VERIFIED** [V]. Tana supertags support template inheritance via projection
(`Extend`). At the data layer, fields are independent nodes with unique IDs;
conflicting field definitions from multiple supertags coexist side-by-side as
distinct tuple children rather than colliding or gating. Type constraints and
`required` checks are purely visual warnings with zero gating consequences.

### 2.2 Primary Source Evidence

- **Field inheritance via Extend [V]:**
  > "Pick a supertag from the list and the default content will populate with
  > template objects from that tag. Think of it as a projection. You cannot move
  > or delete content that originates from an extended tag, but you can add
  > more fields/nodes, and you can specify different default values to the
  > fields."
  > — [Tana Supertags Feature Guide](https://outliner.tana.inc/learn/features/supertags)
  > — [When to use Extend in supertags](https://outliner.tana.inc/learn/guides/when-to-use-extend-in-supertags)
- **Multi-parent inheritance in underlying schema [V]:** In Tana's JSON export,
  inheritance is modeled by an `extends` tuple (`SYS_A13`) attached to a
  `metaNode`. A single tag can reference multiple parent supertag IDs (e.g.
  `outcome-goal` inheriting from both `goal-base` and `Stream | Objectives`,
  and `meeting` resolving through an 8-ancestor chain):
  > "Child supertags inherit fields from parent supertags. A node tagged with
  > `#outcome-goal` effectively has fields from `#goal-base` and
  > `#Stream | Objectives`. Field values may be stored at any level of the
  > inheritance hierarchy."
  > — [Tana Field Structures Technical Reference](https://github.com/jcfischer/supertag-cli/blob/main/docs/TANA-FIELD-STRUCTURES.md)
- **Field identity and multi-supertag conflicts [V]:**
  - Fields are first-class nodes (`attrDef` with a unique ID).
  - If two supertags reference the *same* `attrDef` node ID, they share that
    field definition without conflict.
  - If two supertags declare *different* fields that share the same display
    name, they remain distinct `attrDef` nodes with separate IDs. In Tana's node
    structure, both fields coexist as separate sibling tuples.
  - Pinned fields across multiple supertags are accumulated additively:
    > "If you have nodes from multiple supertags in a view, and those
    > supertags each have pinned fields configured, all pinned fields from all
    > represented supertags will show."
    > — [Tana Fields Guide](https://outliner.tana.inc/learn/features/fields)
- **Non-gating advisory validation [V]:**
  > "Data Validation: Based on their type, Tana Outliner will validate their
  > contents. There are no consequences for storing information that doesn't
  > conform with the validation rules, except for the warning that will appear
  > in the field itself."
  > "Required: If switched on, will give a warning when field has no value. The
  > warning is visual only and has no consequence."
  > — [Tana Fields Guide](https://outliner.tana.inc/learn/features/fields)

### 2.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Tana's pure advisory error
reporting aligns with Ticket 20's non-gating check principle. Because Traces
notes use flat YAML frontmatter where two fields cannot share a key, Ticket 20's
decision to report `schema.class-conflict` when unrelated classes differ on the
same key is the necessary equivalent for flat key-value documents.

---

## 3. SilverBullet: Validation and Typing of Frontmatter / Tags

### 3.1 Verdict

**VERIFIED** [V]. SilverBullet validates page frontmatter and tag attributes
against standard JSON Schema definitions registered via Lua (`tag.define`). By
default, invalid data produces only advisory inline editor warnings and the
page is indexed normally. SilverBullet also offers an optional strict index-time
gate (`mustValidate: true`) which drops invalid objects from the index.

### 3.2 Primary Source Evidence

- **JSON Schema validation of attributes [V]:**
  > "SilverBullet uses JSON Schema for validation of structured data. Schemas
  > ensure that Frontmatter attributes, custom tag fields, and configuration
  > options conform to expected types and shapes."
  > — [SilverBullet Schema Documentation](https://docs.silverbullet.md/Schema)
- **Tag schema registration via Lua [V]:**
  > "When you define a custom tag with `tag.define`, you can provide a schema
  > that describes the expected attributes. SilverBullet validates objects
  > against this schema and surfaces errors in the editor:
  >
  > ```lua
  > tag.define {
  >   name = "contact",
  >   schema = {
  >     type = "object",
  >     properties = {
  >       email = schema.string(),
  >       phone = schema.nullable("string"),
  >       priority = schema.number(),
  >     }
  >   }
  > }
  > ```
  >
  > Pages or objects tagged with `#contact` will be validated against this
  > schema — if a field has the wrong type, you'll see a lint warning in the
  > editor."
  > — [SilverBullet Schema Documentation](https://docs.silverbullet.md/Schema)
- **Behavior on invalid data: default advisory vs index gate [V]:**
  > "When editing a page tagged with `#person` where this schema does not
  > validate (e.g. you set the age to be a string), you will see this error
  > being highlighted in the editor.
  > If you set `mustValidate` to `true`, schema validation will happen during
  > the index phase (in addition to linting in the editor) and non-validating
  > objects will not be indexed (with errors being reported in the JavaScript
  > console). Use this to ensure all your objects conform to your schema (it
  > does come at a slight performance penalty at the indexing phase). Errors
  > are reported to the Log."
  > — [SilverBullet API/tag Documentation](https://docs.silverbullet.md/API/tag)
- **Tag definition merging [V]:**
  > "When `tag.define` is called multiple times with the same `name`, the specs
  > will be merged. This e.g. enables overriding the schema of a built-in tag,
  > or augment it with a custom `transform` callback."
  > — [SilverBullet API/tag Documentation](https://docs.silverbullet.md/API/tag)

### 3.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Ticket 20 decided that schemas
are purely advisory on the read and index paths ("never gate the read or index
path, and never enter the index or its fingerprint"), matching SilverBullet's
default behavior while intentionally rejecting SilverBullet's `mustValidate`
index-drop gate (per ADR-0009 and Ticket 16/19 decisions).

---

## 4. Metadata Enforcer: JSON Schema Abandonment in v2

### 4.1 Verdict

**VERIFIED** [V] (with minor clarification: `metadata-enforcer` is an external
Python CLI tool for Obsidian vaults, not an Obsidian community plugin running
inside the JS app). Author Omar Bohsali explicitly abandoned JSON Schema in v2
(August 2026), replacing `columns.yaml` / JSON Schema Draft 2020-12 with
Django-style Python class models because JSON Schema was insufficiently
expressive and awkward for collection constraints and inheritance.

### 4.2 Primary Source Evidence

- **v1 used JSON Schema Draft 2020-12 [V]:**
  > "JSON-schema is a great idea, so `metadata-enforcer` uses the Draft 2020-12
  > spec under the hood. I did make a few small modifications, like being able
  > to specify column nullability in the column definition itself...
  > Principles: Remain human in the config file; get to JSON Schema at runtime."
  > — [Omar Bohsali, v1 Announcement](https://omarish.com/p/metadata-enforcer)
- **v2 rewrite to Python class models [V]:**
  > "Last week I published omarish/metadata-enforcer, a tool to keep my obsidian
  > metadata organized. I started using it but the intermediate DSL felt
  > superfluous; so I skipped a stable v1.0.0 release and went straight to
  > v2.0.0...
  > Point the CLI at a `schema.py` that defines those models and a path map:
  >
  > ```python
  > from metadata_enforcer import models
  >
  > class AbstractPage(models.Model):
  >     title = models.TextField()
  >     description = models.TextField(optional=True)
  > ```
  >
  > `metadata-enforcer check ~/vault`
  > `metadata-enforcer fix ~/vault`"
  > — [Omar Bohsali, v2 Announcement](https://omarish.com/p/metadata-enforcer-v2)
- **Explicit rationale for abandoning JSON Schema [V]:**
  > "Why not JSON Schema? Because we want something very expressive. We want to
  > define what is legal and what is illegal and these rules must be
  > expressive, legible, turning complete, and done through code. Version 1 used
  > json-schema, but it wasn't indulgent enough."
  > — [Omar Bohsali, Project Page](https://omarish.com/projects/metadata-enforcer)
  > "Why the rewrite: (1) This is the version I actually use... (2) One less
  > layer of abstraction... Python classes are already a language I (and my
  > tools) know. (3) Inheritance is free. Abstract bases, field overrides,
  > nested folder routes... (4) `class Meta` is a better home for collection
  > rules. Uniqueness, table-level constraints, indexes: easier to grow here
  > than by inventing more YAML next to JSON Schema. The goal is to get to
  > PostgreSQL-level collection integrity enforcement..."
  > — [Omar Bohsali, v2 Announcement](https://omarish.com/p/metadata-enforcer-v2)

### 4.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Ticket 20 already rejected JSON
Schema as Traces' primary schema format ("JSON Schema as the schema format:
rejected... Traces schemas also drive prompts, completion, file links and
label/value mapping"). Bohsali's real-world conclusion that JSON Schema is
ill-suited for markdown vault entity modeling directly reinforces Ticket 20.

---

## 5. Obsidian: Rewriting Frontmatter Quotes and Formatting (Bug #65851)

### 5.1 Verdict

**VERIFIED** [V]. When Obsidian edits note properties (via native Properties UI,
Bases, or the API `processFrontMatter`), it deserializes YAML into an in-memory
JavaScript object and reserializes it. This process strips string quotes,
deletes all comments, converts flow-style lists to block style, removes blank
lines, and alters explicit YAML tags. Obsidian's maintainers explicitly declared
this to be intended design.

### 5.2 Primary Source Evidence

- **Forum Bug Report #65851 [V]:**
  > "Calling `processFrontMatter` will destroy all previous formatting, and most
  > YAML features/syntax... A string's quotes can be removed, newlines are
  > removed, comments are removed, types are removed, timestamp types are
  > modified, and inline structures (arrays) are converted to block style."
  > — [Obsidian Forum #65851](https://forum.obsidian.md/t/yaml-properties-api-processfrontmatter-removes-alters-string-quotes-comments-types-formatting/65851)
- **Specific transformations reproduced and verified in #65851 [V]:**
  1. *Quote stripping:* Single quotes (`'course; note'`) and non-essential
     double quotes (`"deadbeef-123-$$"`) are stripped to bare unquoted text.
     Double quotes are preserved only when required to prevent type coercion
     (e.g. `b: "123"`).
  2. *Comment deletion:* All standalone and inline YAML comments (e.g.
     `# a comment`, `# an integer`) are permanently erased.
  3. *Structural reformatting:* Flow-style inline arrays
     (`nameArray: [six, seven]`) are forcibly reformatted into multiline
     block-style lists (`nameArray:\n  - six\n  - seven`).
  4. *Explicit tag coercion:* Tags like `!!timestamp 2022-01-25` become ISO
     UTC strings `2022-01-25T00:00:00.000Z`; `d: !!float 123` becomes `"123"`;
     `status: !!null Null` becomes `status: null`.
  5. *Whitespace:* Blank lines between fields are eliminated.
- **Obsidian core developer and moderator response [V]:**
  > "The decision we made is for plugins to be able to interact with frontmatter
  > easily. We do this by converting YAML into a JS object, which means it lose
  > all of its YAML specific formatting and comments. This makes it easy for
  > plugin developers to read and modify properties of a note, without requiring
  > them to interact with an excessively complex YAML syntax tree, which would
  > require them to understand how YAML is parsed internally."
  > — Licat (Obsidian co-founder / core developer), Post 6 in #65851
  > "As long as things are semantically the same, it's not a bug and you should
  > not expect for it to remain untouched."
  > — WhiteNoise (Obsidian moderator), Post 2 in #65851
- **User impact and related issues [V]:** Similar user data loss reports appear
  in [#101635](https://forum.obsidian.md/t/non-source-mode-property-edits-detrimentally-convert-flow-to-block-style-multi-level-yaml-list-of-lists-sequence-of-sequences/101635),
  confirming that native Properties edits alter frontmatter indiscriminately.

### 5.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Ticket 20 rejected relying on
quote conventions or frontmatter comments to disambiguate dates from strings
("A TOML-style single versus double quote convention is rejected [S]: Noyalib
exposes no per-scalar quote style outside the CST (+59% parse time), and
Obsidian's property editor rewrites quote style on save"). Proving that Obsidian
destroys quote styles and comments establishes that relying on them for vault
type safety would fail in real-world interoperability.

---

## 6. Logseq: Multi-Parent Extends and Hierarchy Integrity

### 6.1 Verdict

**REFUTED / UNSUPPORTED** [V]. The claim that users "broke hierarchy trees" with
multi-parent `extends` and that developers advised against deep inheritance is
an unsubstantiated scout exaggeration. Logseq DB explicitly supports multiple
parents in `extends` as a core architectural feature. The codebase contains
automated cycle detection, and the cited forum thread merely documents standard
user confusion over "is-a" vs "has-a" modeling.

### 6.2 Primary Source Evidence

- **Multi-parent extends is a documented core feature [V]:**
  > "Tags act as classes (`type: 'tag'`). Tags declare `tagProperties` (schemas)
  > ... In Logseq DB, tags can `extends` several parents; a root tag is the
  > default parent."
  > — [Logseq DB Documentation](https://github.com/logseq/docs/blob/master/db-version.md)
- **Built-in cycle detection protects hierarchy integrity [V]:** Logseq's
  database backend explicitly detects and prevents circular extends graphs:
  - Error key `:class.validation/extends-cycle` ("A cycle has occurred in the
    inheritance relationship").
  - Automated database routine: "Drop class extends updates that would
    introduce a cycle".
  - Offline sync handlers explicitly test and resolve "Offline conflicts with
    deleted nodes, parent cycle, and extends cycle".
  > — [Logseq Repository rebase.md & ja.edn](https://github.com/logseq/logseq/blob/master/rebase.md)
- **Analysis of cited forum thread #35010 [V]:**
  - Thread: ["Confusion due to different behavior of how tags are extended"](https://discuss.logseq.com/t/confusion-due-to-different-behavior-of-how-tags-are-extended/35010)
    (May 2026).
  - A single user (@Oesie) attempting to model schema.org types noticed that in
    Logseq DB's tree outline, a tag extending multiple parents appears under
    each parent branch (visual duplication).
  - When the user attempted to extend both an ancestor and a descendant
    concurrently (`#Person` extending `Thing` and `Thing/Intangible/PostalAddress`),
    the UI prevented redundant parent selection.
  - The reply was from another community user (@Matias_Gonzalez), not a Logseq
    developer, clarifying ontological modeling: LocalBusiness and Person *have*
    a PostalAddress (composition via tag properties), rather than *being* a
    PostalAddress (inheritance via extends).
  - No hierarchy broke, no graph view collapsed, and no developer advised
    abandoning multi-parent inheritance.

### 6.3 Impact on Ticket 20's Answer

**Does not alter the decision; validates it.** Ticket 20 Answer states:
"inheritance resolves within each class by the existing `extends` rule
(multiple parents are kept; first-listed parent wins, a child redefinition wins
over its parents). The resolved class set is reduced to its most-derived
members, so an ancestor never conflicts with its own descendant." Disproving
the claim that multi-parent inheritance is inherently unworkable validates
Ticket 20's choice to retain multi-parent `extends` with clean most-derived
reduction and cycle detection.
