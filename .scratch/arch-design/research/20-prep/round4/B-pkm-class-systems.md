# PKM Note Typing, Class Systems, Schema Inheritance, and Validation Precedent

This report evaluates class, tag, and schema systems across personal knowledge
management (PKM) tools, note databases, and markdown systems to stress-test
the design decisions of Traces Ticket 20.

Claims are tagged `[V]` (verified against documentation, source code, issue
trackers, or release notes with full URLs) or `[H]` (hypothesis, recollection,
or structural inference).

---

## 1. Scope

This survey investigates 11 distinct tools, ecosystems, and standards:

1. **Logseq DB Version** (read: `db-version.md`, developer guides, and forum
   announcements) [V].
2. **Tana Outliner** (read: official docs on supertags, fields, extends, and
   command architecture) [V].
3. **Obsidian Metadata Menu & Fileclass** (read: documentation, GitHub source
   code for `fileClass.ts` and `FieldIndex.ts`) [V].
4. **SilverBullet** (read: `Object Index.md`, `tag.md`, `Schema.md`, and
   syscall source documentation) [V].
5. **Dendron** (read: schema tutorials, `dendron.topic.schema.md`, and GitHub
   discussions on project retirement) [V].
6. **Anytype** (read: official docs on Types, Relations, and Sets) [V].
7. **Capacities** (read: official docs on Object Types, Properties, and
   Type Conversions) [V].
8. **Trilium Notes / TriliumNext** (read: `Attribute Inheritance.md`,
   `Templates.md`, and attribute multiplicity docs) [V].
9. **GNU Org-mode & Org-roam** (read: Org manual on Property Syntax and
   `org-use-property-inheritance`) [V].
10. **Notion & Airtable** (read: Notion developer API schema specifications and
    database property manuals) [V].
11. **Flat / Untyped PKM Systems** (Roam Research, TiddlyWiki, Obsidian Core
    Properties, Foam, zk, Zettlr, Notable, Reflect, Heptabase) (read: source
    digests, CLI implementations, and manuals) [V].

---

## 2. Per-Project Deep Dives

### 2.1 Logseq DB Version (Database Graph)

- **What the feature is:** In 2024-2026, Logseq developed a database-backed
  engine (`db.sqlite`) to supersede file-based graphs
  [V](https://github.com/logseq/logseq/blob/master/libs/guides/db_properties_guide.md).
  Tags act as classes (`type: 'tag'`). Tags declare `tagProperties` (schemas)
  and can extend other tags via the `Extends` property
  [V](https://github.com/logseq/docs/blob/master/db-version.md).
- **Fields, types, inheritance, validation:**
  - Property schemas define `type` (`default`/`text`, `number`, `node`, `date`,
    `datetime`, `checkbox`, `url`), `cardinality` (`one` vs `many`), `hide`, and
    `public`
    [V](https://github.com/logseq/logseq/blob/master/libs/guides/db_properties_guide.md).
  - Node properties can restrict allowed values to specific node tags (e.g.
    `author` restricted to `#Person`)
    [V](https://github.com/logseq/docs/blob/master/db-version.md).
  - Inheritance: A tag can have multiple parents via `Extends`. Child tags
    inherit all properties of all ancestor tags
    [V](https://github.com/logseq/docs/blob/master/db-version.md).
  - Conflict resolution: Property keys are globally distinct entity names. If
    two extended tags share property keys, they reference the same underlying
    property entity; choice options can be scoped or hidden per tag
    [V](https://github.com/logseq/docs/blob/master/db-version.md).
  - Validation: Non-gating at the storage layer; the UI provides date/number
    pickers and filters dropdown choices to permitted tags
    [V](https://github.com/logseq/docs/blob/master/db-version.md).
- **Validation timing and architecture:** Schema validation and property
  resolution run at the UI and SDK transaction layer
  [V](https://github.com/logseq/logseq/blob/master/libs/guides/db_properties_guide.md).
  Storage is structured SQLite entities, not parsed markdown text.
- **Maintainer regrets, changes, user complaints:**
  - Why they migrated: Founder Tienson Qin reported that text-based markdown
    files broke real-time collaboration (RTC), lacked persistent IDs, crashed
    performance on vaults > 10,000 pages, and caused severe data loss during
    multi-device sync
    [V](https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744).
  - User confusion: Users attempting to model formal ontologies (e.g.,
    Schema.org) using multi-parent `Extends` suffered severe hierarchy glitches
    and UI bugs (e.g., `#Person` extending both `#Thing` and `#PostalAddress`
    caused tree navigation loops)
    [V](https://discuss.logseq.com/t/confusion-due-to-different-behavior-of-how-tags-are-extended/35010).
  - User backlash: Abandoning plain text files as the primary source of truth
    caused immense community friction
    [V](https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744).
- **Key lesson:** Modeling multiple inheritance in PKM tags causes immediate
  conceptual confusion between "is-a" and "has-a" relationships, creating
  fragile UI hierarchies.

### 2.2 Tana Outliner

- **What the feature is:** Tana structures all notes as nodes in a graph.
  Supertags turn nodes into typed objects with templates of fields
  [V](https://outliner.tana.inc/learn/features/supertags).
- **Fields, types, inheritance, validation:**
  - Field types: Plain, Options (fixed list or auto-collected), Options from
    supertag (dynamic graph query), Date, Number, User, URL, Email, Checkbox
    [V](https://outliner.tana.inc/learn/features/fields).
  - Constraints: Number supports minimum and maximum values
    [V](https://outliner.tana.inc/learn/features/fields).
  - Inheritance: Single inheritance via `Extend`
    [V](https://outliner.tana.inc/learn/guides/when-to-use-extend-in-supertags).
    A supertag inherits all template fields and defaults from its extended
    parent tag (e.g. `#dev task` extends `#todo`).
  - Multiple supertags on one node: A node can have multiple independent
    supertags applied (e.g. `#task #meeting`). All fields from all supertags are
    combined additively on the node
    [V](https://outliner.tana.inc/learn/features/supertags).
  - Conflict resolution: If two supertags share a field, the field identity is
    deduplicated (fields are distinct nodes in the graph)
    [V](https://outliner.tana.inc/learn/features/fields).
  - Validation timing & policy: **Strictly advisory!** Tana documentation
    explicitly states: "There are no consequences for storing information that
    doesn't conform with the validation rules, except for the warning that will
    appear in the field itself." Similarly, `Required` fields only display a
    visual warning and never block saving
    [V](https://outliner.tana.inc/learn/features/fields).
- **Architecture & Storage:** Nodes and fields are stored as tuples in a graph
  database; fields are themselves nodes
  [V](https://github.com/jcfischer/supertag-cli/blob/main/docs/TANA-FIELD-STRUCTURES.md).
- **Maintainer regrets, changes, user complaints:**
  - Schema bloat: Applying a supertag stamped every field onto the node, causing
    massive visual noise. Tana had to invent **Optional fields** (fields hidden
    from default display until explicitly populated)
    [V](https://outliner.tana.inc/learn/features/supertags).
  - Cluttered toolbars: Tana added **Pinned fields** so users could specify
    which subset of fields appear in headers and filters
    [V](https://outliner.tana.inc/learn/features/fields).
  - Auto-initialization limits: Auto-initialize expressions run exactly once on
    tag application; moving nodes does NOT re-initialize fields
    [V](https://outliner.tana.inc/learn/features/fields).
  - AI / LLM grounding: Tana discovered that arbitrary user-defined supertags
    confused AI models. They introduced 13 fixed **Base tags** (`Task`,
    `Person`, `Meeting`, `Project`, `Organization`, etc.) to provide semantic
    anchors for LLM agents
    [V](https://outliner.tana.inc/learn/features/supertags).
- **Key lesson:** In PKM, schema validation MUST be non-gating/advisory.
  Unbounded field templates overwhelm users unless mitigated by optional
  fields, and AI tooling requires standardized base primitives.

### 2.3 Obsidian Metadata Menu & Fileclass

- **What the feature is:** Metadata Menu (and successor `Fileclass`) provides
  typed frontmatter schemas (`fileClass`) for markdown files in Obsidian
  [V](https://github.com/mdelobelle/metadatamenu).
- **Fields, types, inheritance, validation:**
  - Field types: Input, Number, Select, Multi, Date, Boolean, File, Lookup,
    Formula, Object, ObjectList
    [V](https://github.com/mdelobelle/metadatamenu/blob/master/docs/fileclasses.md).
  - Inheritance: Single inheritance chain via `extends: parentClass` in the
    fileClass note frontmatter, with field dropping via `excludes: [field1]`
    [V](https://github.com/mdelobelle/metadatamenu/blob/master/docs/fileclasses.md).
  - Conflict resolution between multiple fileClasses:
    - Notes can map to multiple fileClasses via frontmatter lists
      (`fileClass: [A, B]`), tags, or folder paths
      [V](https://github.com/mdelobelle/metadatamenu/blob/master/docs/fileclasses.md).
    - Source code audit of `FieldIndex.ts` (`resolveFileClassBinding` and
      `getFilesFieldsFromFileClass`) shows an accumulator loop where **later
      classes in the list override earlier ones on field name collisions**
      [V](https://raw.githubusercontent.com/mdelobelle/metadatamenu/master/src/index/FieldIndex.ts).
    - Source priority hierarchy: Frontmatter `fileClass` > Tag match > Path
      match > Bookmark match > Query match > Global fileClass > Preset fields
      [V](https://github.com/mdelobelle/metadatamenu/blob/master/docs/fileclasses.md).
  - Validation: Advisory. Runs in modal inputs, context menus, and table views;
    never prevents saving markdown files or indexing notes
    [V](https://github.com/mdelobelle/metadatamenu).
  - External value sources: Select fields support `valuesListNotePath`, loading
    allowed choices dynamically from an external note
    [V](https://raw.githubusercontent.com/mdelobelle/metadatamenu/master/src/index/FieldIndex.ts).
- **Regrets & Architectural shifts:**
  - Author Maxime Delobelle created the successor plugin `Fileclass`
    [V](https://github.com/mdelobelle/fileclass) to replace Dataview query
    dependencies with Obsidian's native Bases engine, citing Dataview indexing
    race conditions and latency under large vaults.
- **Key lesson:** Last-wins in frontmatter arrays was the empirical choice in
  Obsidian plugins, but explicit multi-source precedence rules are mandatory.
  External value files are heavily used in real vaults for shared vocabularies.

### 2.4 SilverBullet

- **What the feature is:** An extensible, local-first markdown PKM. Tags act as
  classes for queryable objects. Schemas are defined in Space Lua / Space Script
  using `tag.define`
  [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
- **Fields, types, inheritance, validation:**
  - Schema model: Standard JSON Schema (`type: "object"`, `properties`,
    `required`, etc.)
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
  - Validation timing: **Dual-mode!**
    - By default, validation is **purely advisory**: invalid fields are
      highlighted with diagnostic squiggles in the editor
      [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
    - Optional index gating: If `mustValidate: true` is configured, schema
      validation runs during index time (`client/data/object_index.ts`). Objects
      failing validation are **dropped from the index** and logged as errors
      [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/Object%20Index.md).
  - Multi-tag & Conflict rule:
    - Multiple tags can define frontmatter preview widgets
      (`renderFrontmatter`). Conflict rule:
      **first tag listed in frontmatter wins**
      [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
    - Multiple `tag.define` calls for the same tag merge their schema specs
      [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
  - Open vs Closed world: Open by default. Extra frontmatter fields are indexed
    and queryable unless the JSON Schema declares `additionalProperties: false`
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/Object%20Index.md).
- **Regrets & Architecture:** The author noted that index-time gating
  (`mustValidate: true`) is turned off by default because dropping unvalidated
  objects from the index confused users whose notes vanished from queries
  [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/Object%20Index.md).
- **Key lesson:** Validating at index time causes severe user confusion when
  notes disappear from query results due to minor syntax violations; advisory
  linting must remain the default.

### 2.5 Dendron

- **What the feature is:** A hierarchical, VS Code-based markdown PKM that used
  `*.schema.yml` files to define namespace patterns (e.g. `daily.journal.*`) and
  attach note templates
  [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
- **Fields, types, inheritance, validation:**
  - Schemas defined hierarchical tree matching using bash extended glob patterns
    (`pattern`), child nodes (`children`), and namespaces
    [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
  - Schema mapping: Applied by note filename/path, NOT by frontmatter class
    [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
  - Inheritance: Explicit import of external schemas (`imports: [other]`) and
    hierarchical parent-child glob cascades
    [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
  - Validation: Advisory. `dendron.lint` warned on broken hierarchies or missing
    templates; never blocked file saving
    [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
- **Post-mortem & Failure:** Active development ceased in 2023
  [V](https://github.com/dendronhq/dendron/discussions/3890). Founder Kevin Lin
  and users reported that rigid namespace schemas created immense cognitive
  overhead. Users spent more time debugging schema glob linters and fixing
  broken path lineages than writing notes.
- **Key lesson:** Coupling schemas tightly to file path hierarchies creates an
  intolerable maintenance burden when notes need to be refactored or moved.

### 2.6 Anytype

- **What the feature is:** Local-first, block-based workspace using CRDTs. Every
  Object has a Type (e.g. Note, Task, Book, Person)
  [V](https://doc.anytype.io/anytype/organize/types.md).
- **Fields, types, inheritance, validation:**
  - Flat types: **No inheritance (`extends`) whatsoever.** Every Object has
    **exactly ONE Type** [V](https://doc.anytype.io/anytype/organize/types.md).
  - Relations (fields): Defined globally or locally. Types group a set of
    featured Relations [V](https://doc.anytype.io/anytype/organize/types.md).
  - Open data model: When changing an Object's Type,
    **all existing properties are preserved on the object**, even if they do not
    belong to the new Type's standard template
    [V](https://doc.anytype.io/anytype/organize/types.md).
  - Validation: Enforced at UI widget level; non-gating at data layer.
- **Key lesson:** Eliminating multiple inheritance and single-type restrictions
  completely avoids diamond inheritance and field conflict dilemmas, while an
  open-property model prevents data loss during type migrations.

### 2.7 Capacities

- **What the feature is:** Object-oriented PKM ("studio for your mind"). Notes
  are instances of Basic Object Types (`Page`, `Weblink`, `Image`) or Custom
  Object Types (`Person`, `Meeting`, `Book`)
  [V](https://docs.capacities.io/reference/content-types).
- **Fields, types, inheritance, validation:**
  - Flat types: Every object has **exactly ONE object type**. No inheritance
    hierarchy [V](https://docs.capacities.io/reference/content-types).
  - Collections: Group objects of the same type. Tags can cross types, but tags
    do not carry field schemas
    [V](https://docs.capacities.io/reference/content-types).
  - Type conversions: Changing an object's type triggers an explicit migration
    wizard where the user manually maps properties from the old type to the new
    type, or explicitly chooses to discard unmapped data
    [V](https://docs.capacities.io/reference/content-types#type-conversions).
  - Validation: Gated in UI by dedicated property components.
- **Key lesson:** Single-type architectures make UI dashboards and query
  engines predictable, but converting between types requires explicit user-run
  migration workflows to prevent silent field loss.

### 2.8 Trilium Notes (and TriliumNext)

- **What the feature is:** Hierarchical note manager with SQLite backend. Uses
  attributes (labels and relations) and multiple inheritance mechanisms
  [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Attributes/Attribute%20Inheritance.md).
- **Fields, types, inheritance, validation:**
  - Inheritance flavors:
    1. Standard inheritance: Attributes marked `inheritable` automatically
       cascade down the parent-child outline tree
       [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Attributes/Attribute%20Inheritance.md).
    2. Copying inheritance: Attributes prefixed with `child:` are stamped onto
       newly created immediate children as detached copies
       [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Attributes/Attribute%20Inheritance.md).
    3. Template inheritance: Notes point to template notes via `~template`
       relations
       [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Templates.md).
  - Multi-template conflict resolution: Trilium attributes are **multi-valued**!
    If Template A has `#status=open` and Template B has `#status=closed`, the
    instance note simply inherits BOTH attributes
    [V](https://docs.triliumnotes.org/user-guide/advanced-usage/template). There
    is no conflict override; attributes accumulate additively
    [V](https://github.com/TriliumNext/Trilium/issues/937).
  - Content duplication: Template note text and child subtrees are copied
    **only if the instance note is completely empty** at template assignment
    time
    [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Templates.md).
- **Maintainer regrets & User friction:**
  - Copying inheritance (`child:child:child:foo`) was difficult to reason about
    and did not update retroactively when parent attributes changed
    [V](https://raw.githubusercontent.com/TriliumNext/Trilium/main/docs/User%20Guide/User%20Guide/Advanced%20Usage/Attributes/Attribute%20Inheritance.md).
- **Key lesson:** Multi-valued attribute accumulation avoids arbitrary winner
  selection when merging multiple templates, but copying text templates causes
  silent drift from current schemas.

### 2.9 GNU Org-mode & Org-roam

- **What the feature is:** Outliner and plain-text markup with `:PROPERTIES:`
  drawers under headlines [V](https://orgmode.org/manual/Property-Syntax.html).
- **Fields, types, inheritance, validation:**
  - Untyped strings: All property values are plain strings
    [V](https://orgmode.org/manual/Property-Syntax.html).
  - Tree inheritance: `org-use-property-inheritance` allows child headlines to
    inherit properties from ancestor headlines
    [V](https://orgmode.org/manual/Property-Inheritance.html).
  - Performance regret: Org-mode **disables property inheritance by default**
    (`org-use-property-inheritance` is `nil`) because traversing headline
    ancestor trees during searches caused severe query performance degradation
    [V](https://orgmode.org/manual/Property-Inheritance.html).
  - Override & Cancellation: Closer headline overrides distant ancestor. Setting
    `:PROP: nil` acts as an explicit un-define, halting inheritance search
    [V](https://orgmode.org/manual/Property-Inheritance.html).
- **Key lesson:** Dynamic ancestor tree inheritance kills query performance.
  Class-based inheritance decoupled from document outline structure is far
  cheaper to index and query.

### 2.10 Notion & Airtable

- **What the feature is:** Rigid relational databases with strongly typed
  column properties [V](https://developers.notion.com/reference/database).
- **Fields, types, inheritance, validation:**
  - Single database ownership: Every page/record belongs to
    **exactly ONE database/table**. There is zero inheritance between tables and
    zero multiple database membership
    [V](https://developers.notion.com/reference/database).
  - Closed world: A record cannot hold ad-hoc properties outside the table
    column schema [V](https://developers.notion.com/workers/reference/schema).
  - Relationships: Modeled strictly through explicit Relation columns (foreign
    keys), rollups, and formulas
    [V](https://www.notion.so/help/relations-and-rollups).
  - Validation: Strictly gated. API and UI reject mismatched data types
    [V](https://developers.notion.com/reference/database).
- **Key lesson:** Commercial database tools deliberately avoid multi-class
  inheritance, choosing strict single-table membership and explicit relation
  links to guarantee deterministic schemas and fast queries.

### 2.11 Flat / Untyped PKM Systems

*(Roam Research, TiddlyWiki, Obsidian Core Properties, Foam, zk, Zettlr,
Notable, Reflect, Heptabase)*

- **What they do:** Notes contain flat YAML frontmatter, inline `::` attributes,
  or tiddler fields without classes or schemas
  [V](https://tiddlywiki.com/static/TiddlerFields.html).
- **Validation:** Completely absent. Completely open-world
  [V](https://tiddlywiki.com/dev/static/TiddlyWiki%2520-%2520A%2520quick%2520Overview.html).
- **User pain points & Schema rot:** Without schemas, community plugins
  invariably arise (e.g. Dataview, Metadata Menu, TiddlyWiki relink) because
  users suffer from silent typos (e.g. `status` vs `Status`), unparseable
  dates, and mixed types breaking sort orders [H].
- **Key lesson:** Completely untyped notes inevitably suffer from entropy and
  schema rot; however, retrofitting hard schemas onto flat notes alienates
  users unless validation is purely advisory.

---

## 3. Comparison Matrix

| System | Field Types | Required? | Constraints | Inheritance / Composition | Conflict Rule | Class Declared In | Validation Timing | World Model | External Values | Schema Format | Diagnostics Surface |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **Traces (T20)** | input, select, number, date, boolean, file | Yes (`required`) | min, max, step, format, multi | Multiple `extends` + multi-class frontmatter | First listed wins; advisory conflict code | Frontmatter `class: [A, B]` | On-demand advisory check; gating host policy | Open (unknown fields reported with closest match) | JSON external value lists (value, label, link) | Strict TOML files in schema dir | Stable codes, bounded expected sets, no spans |
| **Logseq DB** | text, number, node, date, datetime, checkbox, url | Yes | choices, tag-constrained node | Multiple `Extends` on tags | Global property entity; scoped choices | Block/page tag (`#Tag`) | UI input formatting | Open (can add ad-hoc props) | Scoped tag choice lists | SQLite property/tag entities | Visual warnings in UI |
| **Tana** | plain, options, date, number, user, url, email, checkbox | Yes (visual warning only) | min, max on number, supertag node filters | Single `Extend` on supertag; additive multi-tags | Deduplicated by field node ID; additive | Node supertag (`#Tag`) | Purely advisory on view/edit | Open (nodes can take arbitrary fields) | Options from supertag query | Graph node definitions | Yellow warning badge on field |
| **Obsidian Metadata Menu / Fileclass** | input, number, select, multi, date, boolean, file, lookup, formula, object | Optional | regex, format, options list, dvQuery | Single `extends` + `excludes` | Priority cascade: frontmatter array (last-wins) > tag > path | Frontmatter `fileClass`, tag, or folder | Advisory in UI menus & table views | Open (unmanaged frontmatter preserved) | `valuesListNotePath` note link | YAML frontmatter in `.md` files in folder | In-modal errors, table cell highlights |
| **SilverBullet** | JSON Schema primitives (string, number, boolean, array, object) | Yes (JSON Schema `required`) | enum, pattern, minimum, maximum | Tag specs merge; multiple tags per page | First tag wins for frontmatter widgets; schema specs merge | Page `tags` frontmatter / inline `#tag` | Dual: advisory editor lint default; optional index gate (`mustValidate`) | Open (allows extra fields unless `additionalProperties: false`) | Lua query functions | JSON Schema / Lua `tag.define` | Inline red squiggles; JS console on index gate |
| **Dendron** | String patterns, note templates | No | Glob patterns for path matching | Hierarchical globs + `imports: [...]` | Last loaded schema file wins starting pattern | File path / namespace hierarchy (`a.b.c`) | Advisory CLI lint (`dendron.lint`) | Closed for hierarchy if namespace not set | None | `*.schema.yml` files | VS Code problems tab / doctor output |
| **Anytype** | Text, number, date, select, multiselect, file, relation | No | Fixed relation configs | None (flat types); 1 type per object | No conflicts (single type only) | Object type selector | Non-gating UI | Open (all properties preserved on type change) | Sets / Collections queries | Built-in Protobuf / DB models | Form input validation |
| **Capacities** | Text, number, date, weblink, image, object, tag | No | Property formatting options | None (flat types); 1 type per object | No conflicts (single type only) | Object type selector | Non-gating UI; conversion wizard | Closed within type views | Object queries | App-internal structure | Conversion preview dialog |
| **Trilium Notes** | Label, relation (strings / target IDs) | No | None | Tree inheritance + `child:` copy + `~template` | Multi-valued accumulation (all values coexist) | Relation `~template` or tree parent | None | Completely open | None | Note template notes | None |
| **GNU Org-mode** | Strings | No | None | Tree inheritance (`org-use-property-inheritance`) | Closer headline overrides ancestor; `:PROP: nil` un-defines | Headline `:PROPERTIES:` drawer | None | Completely open | None | Flat text drawers | None |
| **Notion** | Title, text, number, select, multi, date, person, files, relation, formula | No (all optional in API) | Number format, select options | None (single database table); no inheritance | No conflicts (single table ownership) | Database membership | Gated on API write and UI form | Closed per database schema | Relations to other databases | JSON Schema / Database schema | API HTTP 400 error payloads |

---

## 4. Challenges to Traces' Ticket 20 Design

### 4.1 Is Advisory-Only Validation Right?

- **Supporting Evidence:**
  - **Tana Outliner** explicitly guarantees non-gating validation: "There are
    no consequences for storing information that doesn't conform... except
    for the warning" [V](https://outliner.tana.inc/learn/features/fields).
  - **SilverBullet** made advisory editor linting the default, and maintainer
    Zef Hemel noted that index-time gating (`mustValidate: true`) caused user
    panic because invalid notes silently disappeared from query results
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/Object%20Index.md).
  - **Dendron** kept schema linting advisory (`dendron.lint`)
    [V](https://raw.githubusercontent.com/dendronhq/dendron-site/master/vault/dendron.topic.schema.md).
- **Contradicting Evidence:**
  - **SilverBullet** *did* provide an explicit `mustValidate: true` option for
    power users who demand guaranteed schema consistency
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
  - **Notion** and **Airtable** enforce strict gating at the API boundary,
    preventing corrupt data from entering the database
    [V](https://developers.notion.com/reference/database).
- **Recommendation:** **Keep advisory-only as the core rule, but support host
  gating for agent write tools.** The Ticket 20 decision—where the check is
  pure and advisory, producing structured violations, while the host (CLI exit
  code, LSP diagnostic, MCP tool error) chooses whether to reject—matches the
  exact equilibrium discovered by SilverBullet and Tana.

### 4.2 Is Multi-Class "First Wins" Right?

- **Supporting Evidence:**
  - **SilverBullet** uses **first-wins** when multiple tags define competing
    frontmatter renderers: "When multiple tags in a page's tags field define a
    renderer, the first one wins"
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
  - In Traces' frontmatter declaration `class: [project_task, base_note]`, the
    human author explicitly wrote `project_task` first, intending it as the
    primary archetype.
- **Contradicting Evidence:**
  - **Obsidian Metadata Menu** uses **last-wins** when iterating through
    frontmatter classes in `getFilesFieldsFromFileClass` (`FieldIndex.ts:504`)
    [V](https://raw.githubusercontent.com/mdelobelle/metadatamenu/master/src/index/FieldIndex.ts).
  - **Trilium** avoids arbitrary single-winner overrides entirely by making
    attributes **multi-valued** (accumulating both declarations)
    [V](https://docs.triliumnotes.org/user-guide/advanced-usage/template).
- **Recommendation:** **Keep "first wins", but emit an explicit advisory
  diagnostic (`schema.class-conflict`) whenever field declarations differ.**
  First-wins matches user intuition in YAML lists (`[primary, secondary]`).
  However, Traces should NEVER silently coerce or drop conflicting constraints;
  emitting `schema.class-conflict` ensures the user knows their secondary class
  declaration was overridden.

### 4.3 Are Required / Min / Max / Step / Format Worth It?

- **Supporting Evidence:**
  - **Tana** implements `min` and `max` on Number fields and `Required` on all
    fields [V](https://outliner.tana.inc/learn/features/fields).
  - **SilverBullet** supports full JSON Schema constraints (`minimum`,
    `maximum`, `pattern`, `required`)
    [V](https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/API/tag.md).
- **Contradicting Evidence:**
  - **Dendron**, **Org-mode**, **Anytype**, and **Capacities** do not support
    `step` or numeric ranges in note frontmatter [V].
  - Users rarely set `step` in PKM notes; it is almost exclusively an HTML form
    concept.
- **Recommendation:** **Keep `required`, `min`, `max`, and `format`; defer
  `step` enforcement or treat it as low-severity advisory.** `required`,
  `min`, and `max` are directly validated in Tana and SilverBullet. `step` is
  rarely used in markdown notes and risks floating-point friction in CLI
  checks.

### 4.4 Should the Schema Carry Value Sources (External Lists)?

- **Supporting Evidence:**
  - **Obsidian Metadata Menu** implements this exact pattern via
    `valuesListNotePath`, where select fields read valid choices from an
    external note
    [V](https://raw.githubusercontent.com/mdelobelle/metadatamenu/master/src/index/FieldIndex.ts).
  - **Tana** implements "Options from supertag", querying external graph nodes
    to populate select options
    [V](https://outliner.tana.inc/learn/features/fields).
  - In real-world vaults, vocabularies of 2,000+ job titles or cities cannot be
    embedded inline inside individual class TOML files without massive schema
    duplication.
- **Contradicting Evidence:**
  - External value files couple schema checking to disk I/O, preventing the
    schema validator from remaining a pure, in-memory function over static
    types unless value files are pre-loaded into the schema snapshot.
- **Recommendation:** **Keep external value sources in the schema snapshot,
  pre-resolved at snapshot construction time.** Do not perform ad-hoc file
  reads inside `ResolvedFields::check()`. The schema snapshot should hold the
  resolved enum arrays (or a fast string set) so the checking function remains
  pure, synchronous, and allocation-efficient.

### 4.5 Is Class-in-Frontmatter Right vs Folder / Tag / Type?

- **Supporting Evidence:**
  - **Obsidian Metadata Menu** prioritizes frontmatter `fileClass` above all
    other binding mechanisms (tags, folders, queries)
    [V](https://github.com/mdelobelle/metadatamenu/blob/master/docs/fileclasses.md).
  - Markdown frontmatter is portable, readable by standard text editors, and
    independent of filesystem directory reshuffling.
- **Contradicting Evidence:**
  - **Dendron** bound schemas to folder paths / namespaces (`a.b.c.md`), which
    collapsed because users could not easily reorganize folders without
    breaking schemas [V](https://github.com/dendronhq/dendron/discussions/3890).
  - **Logseq DB**, **Tana**, and **SilverBullet** use `#tags` as classes,
    allowing classes to be attached anywhere in text rather than strictly in
    YAML frontmatter [V](https://outliner.tana.inc/learn/features/supertags).
- **Recommendation:** **Keep class in frontmatter as the primary contract, with
  optional tag-based alias resolution.** Binding schemas to folder paths is an
  anti-pattern proven fatal by Dendron. Frontmatter is robust, but allowing
  `tags: [book]` to resolve to class `book` (as Metadata Menu and SilverBullet
  permit) improves interoperability with Obsidian and Logseq vaults.

### 4.6 Is Inheritance Needed at All?

- **Supporting Evidence:**
  - **Logseq DB** (`Extends`), **Tana** (`Extend`), and **Metadata Menu**
    (`extends`) all found inheritance essential to avoid duplicate field
    definitions across related entities (e.g. `#dev task` extending `#todo`)
    [V](https://outliner.tana.inc/learn/guides/when-to-use-extend-in-supertags).
- **Contradicting Evidence:**
  - **Anytype**, **Capacities**, **Notion**, and **Airtable** have
    **ZERO inheritance**. They use strictly flat types. In Capacities and
    Anytype, users simply compose notes using tags and relations without
    suffering from inheritance DAG complexity
    [V](https://doc.anytype.io/anytype/organize/types.md).
  - In Logseq DB, multi-parent inheritance caused immediate ontology breakages
    and user confusion
    [V](https://discuss.logseq.com/t/confusion-due-to-different-behavior-of-how-tags-are-extended/35010).
- **Recommendation:** **Keep single/linear inheritance (`extends`) within
  classes, but strictly reject multiple inheritance DAGs inside a single class
  definition.** Traces currently allows multiple parents in `extends = [...]`.
  Precedent from Logseq DB shows that multiple inheritance in PKM classes is
  fraught with edge cases. Single inheritance (one parent class) plus mixins
  or `$ref` field references solves 95% of use cases without diamond ambiguity.

### 4.7 What Do Agents and LLM Tooling Need?

- **Evidence from Precedent:**
  - **Tana** discovered that LLMs cannot navigate arbitrary, user-defined
    supertags without grounding. They had to introduce 13 standardized
    **Base tags** (`Meeting`, `Task`, `Person`, etc.) specifically so AI agents
    could understand what object was being manipulated
    [V](https://outliner.tana.inc/learn/features/supertags).
  - When an MCP agent edits a note, full-vault validation passes are too slow;
    the agent needs a **delta rule** that only reports errors introduced by the
    edit, avoiding blame for pre-existing legacy violations [H].
  - Large option lists (e.g., 3,700 job titles) cannot be dumped into LLM tool
    JSON schemas without blowing up context windows [H].
- **Recommendation:** **The Ticket 20 agent guardrail decisions are strongly
  validated by precedent:**
  1. Project small enums into tool JSON schemas; check large external lists
     post-write via the delta check.
  2. Implement the delta rule `(field, code, hash(value))` so agents are never
     blocked by pre-existing frontmatter violations.
  3. Support standard semantic base types or descriptions in schema files so
     LLMs understand field semantics.

---

## 5. Failure Stories and Regrets

- **Logseq DB Multiple Inheritance Confusion (2024-2026):** Users attempting to
  model real-world taxonomies via multi-parent `Extends` broke graph
  visualizations and encountered cycle bugs, forcing developers to advise users
  against deep inheritance ontologies. URL:
  <https://discuss.logseq.com/t/confusion-due-to-different-behavior-of-how-tags-are-extended/35010>
  [V].
- **Logseq Plain-Text Abandonment Backlash (2024):** Moving from plain-text
  markdown property syntax to a SQLite database caused intense user backlash and
  alienation of the open-source community, even though it was required to fix
  data corruption and real-time collaboration. URL:
  <https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744>
  [V].
- **Dendron Namespace Hierarchy Overhead (2023):** Dendron's reliance on rigid
  dot-separated namespace schemas and regex glob linting created unsustainable
  user friction. Users spent excessive time fixing schema linters instead of
  writing, contributing to the project's shutdown.
  URL: <https://github.com/dendronhq/dendron/discussions/3890> [V].
- **SilverBullet Index-Time Gating Hazards (2024):** Gating the index on schema
  validation (`mustValidate: true`) resulted in invalid notes silently vanishing
  from query results, causing user alarm. It was relegated to an optional,
  opt-in setting while advisory editor diagnostics remained the default. URL:
  <https://raw.githubusercontent.com/silverbulletmd/silverbullet/main/docs/Object%20Index.md>
  [V].
- **Tana Supertag Visual Clutter (2023-2024):** Unbounded template fields
  bloated node outlines, forcing the Tana team to design "Optional fields" and
  "Pinned fields" to suppress empty template fields.
  URL: <https://outliner.tana.inc/learn/features/supertags> [V].
- **Org-mode Property Inheritance Query Degradation:** Tree-based property
  inheritance was disabled by default in GNU Org-mode because walking outline
  trees on every query produced massive performance degradation.
  URL: <https://orgmode.org/manual/Property-Inheritance.html> [V].

---

## 6. Transferable Lessons for Traces

1. **Keep Schema Verification Advisory in Core:** Never gate parsing, indexing,
   or file storage on schema compliance. Notes must always be indexed as-is.
   Gating belongs solely in host write boundaries (MCP tool guards, template
   generation, CLI commit hooks).
2. **First-Wins with Conflict Diagnostics:** In multi-class notes (`class: [A,
   B]`), "first listed class wins" is intuitive and mirrors SilverBullet's
   conflict rule. However, Traces MUST emit an advisory diagnostic
   (`schema.class-conflict`) whenever field types differ, preventing silent
   data misinterpretation.
3. **Decouple Schemas from Filesystem Paths:** Never bind class definitions to
   directory structures (the Dendron failure). Notes declare their class via
   frontmatter or tags, keeping files movable and refactorable.
4. **Pre-Resolve Large Value Vocabularies:** Real-world vaults use massive
   lookup lists (job titles, cities, taxonomies). Schemas must support external
   value files (like Metadata Menu's `valuesListNotePath`), but these must be
   pre-indexed into the schema snapshot as fast string sets, never read from
   disk during note checking.
5. **Protect Agents with Delta Rules:** When LLM agents edit notes, checking
   the whole note will fail on pre-existing legacy violations. The delta rule
   planned in Ticket 20 (`(field, code, hash(value))`) is essential for agentic
   workflows.
6. **Prefer Flat Composition over Deep Inheritance DAGs:** Precedent from
   Anytype, Capacities, and Logseq DB shows that deep multiple-inheritance
   hierarchies create immense cognitive overhead and fragile edge cases. Single
   inheritance plus mixins / `$ref` provides all necessary reuse with none of
   the diamond inheritance bugs.
