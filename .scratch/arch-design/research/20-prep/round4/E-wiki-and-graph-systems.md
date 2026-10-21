# Precedent Study: Wiki, CMS, and Knowledge-Graph Structured Data Systems

This study investigates how 14 established wiki, content management (CMS),
and knowledge-graph systems attach structured, typed, class-like schemas
to documents or graph entities. It evaluates how these systems handle
validation timing (advisory vs gating), schema evolution and migration,
property reuse across classes, large allowed-value lists, composition
(mixins/behaviors) versus inheritance, and architectural separation between
parsing, storage, and validation.

All findings are evaluated against the design choices of Traces Ticket 20
(schema note typing and validation). Claims verified by reading primary
documentation, issue trackers, source code, or published post-mortems are
tagged `[V]` with full URLs; historical recollections and inferences are
tagged `[H]`.

---

## 1. Scope

Fourteen systems and standards were examined:

1. Semantic MediaWiki (SMW): Semantic wiki extension attaching typed
   properties and allowed-value lists to wiki pages [V].
2. Cargo: MediaWiki extension providing template-declared relational tables
   as an alternative to SMW [V].
3. Wikibase / Wikidata Property Constraints: Knowledge-graph property
   constraint system and EntitySchema (ShEx) validation on Wikidata [V].
4. Foswiki / TWiki DataForms: Structured topic forms, mandatory fields,
   and dynamic lookups in flat-file wikis [V].
5. DokuWiki `struct` plugin: Centralized schema definitions, SQLite storage,
   and typed lookups for flat-file wikis [V].
6. BookStack: Document hierarchy and untyped key-value page tagging [V].
7. Confluence Page Properties: Free-form table metadata macro and aggregation
   reports [V].
8. XWiki Class/Object Model: XClass, XObject, property types, and document
   class migration [V].
9. Drupal Content Types & Field API: Decoupled field storage, bundle field
   instances, and schema update barriers [V].
10. Plone Dexterity & Behaviors: Content type composition via schema
    interfaces and mixin behaviors [V].
11. Notion API: Database property validation, strict select options, and
    partial page updates [V].
12. Datomic / DataScript: Universal attribute-based data model, immutable
    value types, and schema accretion [V].
13. TerminusDB: Document graph schema, weakening vs strengthening migrations,
    and Git-like schema versioning [V].
14. W3C SHACL / Solid Shapes: Shapes Constraint Language for RDF graphs,
    open vs closed world shapes, and severity levels [V].

All 14 projects were verified via current primary documentation, issue
trackers, or W3C specifications [V].

---

## 2. Per-Project Analysis

### 2.1 Semantic MediaWiki (SMW)

- **Feature & Architecture**: SMW allows users to annotate wiki wikitext with
  typed properties (`[[Property:Name::Value]]` or `{{#set:Property=Value}}`).
  Property definitions live in a dedicated `Property:` wiki namespace [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Properties_and_types>).
  Special property `Has type` assigns types (`Text`, `Number`, `Page`,
  `Date`, `Quantity`, etc.) [V].
- **Constraints & Value Sets**: Enumerations are declared on the property
  page using `[[Allows value::Val]]` [V]. To handle large lists, SMW 2.5+
  introduced `[[Allows value list::list_name]]` (`_PVALI`), referencing an
  external MediaWiki page or JSON document containing allowable values [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Special_property_Allows_value_list>).
  SMW 3.0+ added numeric intervals (`[[Allows value::1...100]]`) [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Special_property_Allows_value>).
- **Validation Timing**: Strictly advisory and post-parse [V]. Setting an
  invalid value displays an inline warning tooltip on the page; it never
  blocks saving the wiki page [V]. When set via `{{#set:...}}`, violations
  are silently ignored during rendering, and an alert is recorded under
  special property `Has improper value for` (`_IMPR`) [V].
- **Typing, Storage, and Migration**: Annotations are parsed into an internal
  semantic data container and written to a relational database (SQLStore) or
  triplestore. Changing a property's datatype is permitted, but triggers
  an asynchronous job queue (`smw.changePropagationDispatch`) [V]. All pages
  referencing that property must be re-parsed. While jobs are queued,
  queries return inconsistent results [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Changing_property_types>).
- **Complaints & Regrets**: Triplestore query performance degraded on large
  wikis; property pages proliferated (often comprising >90% of wiki pages);
  and silent failures in `{{#set:...}}` left invalid data undetected without
  explicit reporting queries [V].
- **Key Lesson**: Externalizing large value lists into separate JSON or wiki
  pages is essential for maintenance. Decoupling parsing from validation
  preserves wiki editability, but asynchronous re-parsing after schema type
  changes causes temporary data inconsistencies across the corpus.

### 2.2 Cargo (MediaWiki)

- **Feature & Architecture**: Cargo was created by Yaron Koren as an explicit
  alternative to SMW's RDF model [V]
  (<https://www.mediawiki.org/wiki/Extension:Cargo/Cargo_and_Semantic_MediaWiki>).
  Instead of independent property pages, schemas are declared inside wiki
  templates using `#cargo_declare` (defining tables and typed fields), and
  populated via `#cargo_store` calls [V].
- **Validation Timing**: Gating on template execution / store [V]. If stored
  values fail SQL type constraints, database insertion fails or warns, but
  wikitext saving succeeds.
- **Typing, Storage, and Migration**: Cargo maps template calls directly to
  standard relational SQL tables. Modifying a table schema requires dropping
  or recreating the table via `Special:CargoTables`, triggering a full
  re-population from template calls across all wiki pages [V].
- **Complaints & Regrets**: Recreating tables on large wikis locks tables or
  imposes heavy server load. Unlike SMW, Cargo cannot query category
  hierarchies transitively without manual hierarchy definitions [V].
- **Key Lesson**: Relational table schemas tied to templates are easier to
  understand and query (via SQL) than RDF property graphs, but schema
  migrations on pre-existing data require whole-table repopulation.

### 2.3 Wikibase / Wikidata Property Constraints & EntitySchemas

- **Feature & Architecture**: Wikidata entities (items `Q...` and properties
  `P...`) have statements. Constraints are specified as statements directly
  on property entities using `property constraint` (`P2302`) and a constraint
  type item (e.g. `subject type constraint` `Q21503250`, `range constraint`
  `Q21510860`, `single-value constraint` `Q19474404`) [V]
  (<https://www.wikidata.org/wiki/Help:Property_constraints_portal>).
- **Validation Timing**: 100% advisory and non-gating [V]. The
  `WikibaseQualityConstraints` extension runs checks asynchronously or on
  demand, displaying warnings on item pages and on `Special:ConstraintReport`
  [V]. Violations never block edits. Non-gating is an intentional design
  mandate: real-world knowledge is messy and has edge cases (e.g., Stubbs the
  cat served as mayor of Talkeetna, violating human-only mayor constraints)
  [V] (<https://www.wikidata.org/wiki/Help:Property_constraints_portal>).
- **Severity Levels**: Three levels exist: `mandatory constraint` (`Q21502408`),
  unnamed normal (default), and `suggestion constraint` (`Q62026391`) [V].
  Even "mandatory" constraints are non-gating; the term indicates high audit
  priority, not an edit blocker [V]
  (<https://m.wikidata.org/wiki/Wikidata:2020_report_on_Property_constraints>).
- **EntitySchemas (ShEx)**: Shape Expressions (ShExC) validate entire entity
  shapes via `Extension:EntitySchema` (`E...` IDs) [V]
  (<https://www.mediawiki.org/wiki/Extension:EntitySchema>). ShEx validation is
  similarly decoupled from entity editing and executed on demand.
- **Complaints & Regrets**: The 2020 Property Constraints Report (Phabricator
  T244043) found widespread confusion over severity naming [V]. Furthermore,
  4.6% of constraints had manual exceptions added (`P2303`); editors
  routinely appeased constraint warnings by adding exceptions rather than
  correcting flawed constraint definitions [V].
- **Key Lesson**: In open knowledge systems, validation must be advisory.
  Gating writes on constraint violations halts valid contributions because
  reality always contains legitimate exceptions. However, advisory warnings
  require clear severity semantics to avoid editor fatigue and false-positive
  appeasement.

### 2.4 Foswiki / TWiki DataForms

- **Feature & Architecture**: Structured topic data is declared in a TML
  table inside a form definition topic (e.g., `QuestionForm`). The table
  defines field rows: `Name`, `Type`, `Size`, `Values`, `Description`,
  `Attributes`, `Default` [V] (<https://foswiki.org/System/DataForms>).
  A topic attaches exactly one form via a `formtemplate` setting [V].
- **Validation Timing**: Gating at save time [V]. Form fields marked with the
  `M` (Mandatory) attribute in the form definition reject topic saves if empty,
  displaying an error page [V].
- **External Values & Lookups**: If the `Values` column is empty for a
  `select` field, Foswiki dynamically fetches allowed values from another
  topic named after the field, or dynamically executes a `%SEARCH{...}%`
  macro to populate dropdown options [V].
- **Typing, Storage, and Migration**: Field values are stored in topic meta-data
  blocks (`%META:FIELD{name="..." title="..." value="..."}%`) within the raw
  text file [V]. If a field is deleted from the form definition, existing data
  remains in the topic file until the next edit/save cycle, at which point
  unrecognized fields are permanently purged from the file [V].
- **Complaints & Regrets**: Silently dropping removed form fields upon saving
  caused irreversible data loss. Binding a topic to only a single form
  prevented compositional typing [V].
- **Key Lesson**: Single-class binding is overly restrictive. Purging
  unrecognized fields on save without explicit confirmation destroys data
  when schemas change. Dynamic queries for allowed values provide high
  flexibility in flat-file wikis.

### 2.5 DokuWiki `struct` Plugin

- **Feature & Architecture**: Built by Cosmocode to replace the older,
  untyped `data` plugin [V] (<https://www.dokuwiki.org/plugin:struct>).
  Schemas define typed fields (Text, Integer, Decimal, Date, Dropdown, User,
  Lookup, etc.). Schemas are assigned to pages dynamically via namespace
  patterns or regexes (e.g. `projects:*` gets schema `project`) [V].
- **Validation Timing**: Gating during GUI form editing; advisory when
  aggregating [V]. Form submission enforces types, regex patterns, and
  required flags.
- **Typing, Storage, and Migration**: Structured values are decoupled from
  raw wikitext and stored in a central SQLite database
  (`data/meta/struct.sqlite3`) via the `sqlite` helper plugin [V]. The wiki
  page retains its wikitext while SQLite powers relational queries,
  filtering, and aggregations [V].
- **Lookup Fields**: A `Lookup` field type references fields in other schemas
  or external database tables, enabling foreign-key-like relationships [V].
- **Complaints & Regrets**: Decoupling structured data from the markdown/page
  file means Git-based wiki backups or direct filesystem edits to page files
  do not touch or update the structured metadata in SQLite [V].
- **Key Lesson**: Folder/namespace pattern assignment eliminates the need for
  pages to explicitly self-declare classes, but storing structured data in
  an external database breaks the self-contained simplicity of text files.

### 2.6 BookStack

- **Feature & Architecture**: BookStack organizes content into a rigid
  hierarchy: Shelves -> Books -> Chapters -> Pages [V]
  (<https://www.bookstackapp.com/docs/>).
- **Metadata Model**: BookStack deliberately rejects custom page schemas,
  content types, and field definitions [V]. Instead, it provides a simple
  untyped key-value tagging system (`tag_name: tag_value`) attached to
  pages or books [V].
- **Validation**: None. Tags are arbitrary strings with auto-completion based
  on existing tags, but no required keys, types, or enums [V].
- **Complaints & Regrets**: Community requests for custom metadata fields
  (e.g., Issue #419, #1241, #5530) have been repeatedly closed as out of
  scope to maintain simplicity and prevent UI/database bloat [V]
  (<https://github.com/BookStackApp/BookStack/issues/1241>).
- **Key Lesson**: Systems without schema validation rely on conventions,
  which inevitably breakdown across large teams, leading users to demand
  structured field types.

### 2.7 Confluence Page Properties

- **Feature & Architecture**: Confluence provides a "Page Properties" macro
  containing a standard two-column rich-text table (Property Name | Property
  Value), and a "Page Properties Report" macro that aggregates rows across
  pages sharing a given Confluence label [V]
  (<https://support.atlassian.com/confluence/kb/confluence-page-properties-report-macro-not-working/>).
- **Validation Timing**: None. There is no schema definition, no field
  typing, and no validation engine [V].
- **Complaints & Regrets**: Massive user frustration on Atlassian forums [V]:
  a single whitespace typo in a property header (e.g., `"Status "` vs
  `"Status"`) splits report columns; text wrapping and table layout changes
  break parsing; pagination limits (capped at 30 rows) break sorting across
  large datasets; and the absence of select lists leads to chaotic value
  drift [V] (<https://jira.atlassian.com/browse/CONFCLOUD-78762>).
- **Key Lesson**: Schema-free, untyped page properties produce brittle
  aggregations that degrade rapidly under user typos.

### 2.8 XWiki Class / Object Model (XClass & XObject)

- **Feature & Architecture**: XWiki implements a complete presentation-layer
  object-oriented data model [V]
  (<http://www.xwiki.org/xwiki/bin/view/documentation/xs/dev/data-model/>).
  An `XClass` defines property definitions (String, TextArea, Boolean,
  Date, StaticList, DatabaseList, etc.) and is attached to a Page (one XClass
  per page) [V]. A document can hold multiple `XObject` instances, including
  multiple objects of different classes (composition / mixins) [V].
- **Validation Timing**: Configurable per property (regular expressions,
  mandatory flags). Validation runs on document save in the UI [V].
- **Typing, Storage, and Migration**: XObjects are stored in relational
  database tables (via Hibernate EAV or custom mapping) [V].
  When an XClass property is deleted or renamed, existing XObjects in the
  database retain their old data as orphaned properties [V]. Adding support
  for cleanly removing class properties took five years to resolve (Issue
  XWIKI-99, 2005-2010) because of database persistence complexity [V]
  (<https://jira.xwiki.org/browse/XWIKI-99>).
  Furthermore, moving or renaming a class page requires an explicit
  refactoring wizard to update references across all dependent document
  objects [V]
  (<http://www.xwiki.org/xwiki/bin/view/documentation/xs/user/base/page/operations-refactor/move-rename/class-page-move-rename>).
  A long-standing issue (XWIKI-23815) noted that XWiki diffs historically
  evaluated old document revisions against the *current* XClass, producing
  incorrect diffs when classes evolved [V]
  (<https://jira.xwiki.org/browse/XWIKI-23815>).
- **Key Lesson**: Multi-class composition (multiple XObjects on one page)
  avoids the fragility of deep inheritance hierarchies. However, schema
  evolution (renaming/deleting properties) creates severe migration and
  diff hazards when old entity snapshots are evaluated against current
  class definitions.

### 2.9 Drupal Content Types & Field API

- **Feature & Architecture**: Drupal decouples field storage from bundle
  instances [V]
  (<https://api.drupal.org/api/drupal/core!modules!field!field.api.php/11.x>).
  `FieldStorageConfig` defines the global property (name, type, cardinality,
  custom settings) and is shared across multiple content types (bundles) [V].
  `FieldConfig` attaches the field storage to a specific bundle (content type),
  specifying bundle-specific labels, descriptions, default values, and required
  flags [V]. This is direct property reuse across classes, identical to Traces'
  `$ref`.
- **Validation Timing**: Gating on entity save [V]. Typed Data validators
  execute constraint plugins (Symfony Validator-based) before database
  commit.
- **Migration & Evolution Barriers**: Drupal enforces an absolute barrier on
  field storage changes: once an entity exists with data (`hasData() == TRUE`),
  Drupal throws `FieldStorageDefinitionUpdateForbiddenException` if a module
  or administrator attempts to alter storage settings or field types [V]
  (<https://api.drupal.org/api/drupal/core!lib!Drupal!Core!Entity!Exception!FieldStorageDefinitionUpdateForbiddenException.php/class/FieldStorageDefinitionUpdateForbiddenException/main>).
  Developers must execute complex multi-step batch migrations
  (`hook_post_update_N`) to spin up a temporary field, migrate data, drop
  the original field, and recreate it.
- **Complaints & Regrets**: Sharing field storage across bundles creates tight
  coupling: a requirement to alter cardinality or settings for one bundle
  is blocked because another bundle holds data (Issue #3345006) [V]
  (<https://www.drupal.org/project/drupal/issues/3345006>).
- **Key Lesson**: Decoupling global property definitions from class-specific
  attachments (`$ref`) enables universal queries and consistent naming, but
  enforcing strict storage invariants on shared definitions locks all
  referencing classes once data is written.

### 2.10 Plone Dexterity & Behaviors

- **Feature & Architecture**: Plone completely re-architected its content
  type model when transitioning from Archetypes to Dexterity [V]
  (<https://5.docs.plone.org/external/plone.app.dexterity/docs/intro.html>).
  Archetypes relied on deep class inheritance (`ATContentType`), which created
  rigid, monolithic classes that resisted reuse. Dexterity replaced inheritance
  with composition via **Behaviors** [V].
- **Behaviors as Mixins**: A Dexterity content type's schema is composed by
  combining a base `zope.schema` interface with a list of reusable behaviors
  (e.g., `IDublinCore`, `IRelatedItems`, `ITagging`, `ILocation`) [V]
  (<https://5.docs.plone.org/external/plone.app.dexterity/docs/behaviors/intro.html>).
- **Storage & Collision Rules**: Plone identified a critical hazard in mixin
  composition: attribute collision [V]
  (<https://5.docs.plone.org/external/plone.app.dexterity/docs/behaviors/schema-only-behaviors.html>).
  If two behaviors declare a field with the same name, direct attribute
  storage causes silent data overwrites. Dexterity resolved this by offering
  two storage strategies:
  1. *Direct attribute storage*: Fast, but assumes unique field names across
     all enabled behaviors [V].
  2. *Annotation storage (`plone.behavior.AnnotationStorage`)*: Stores behavior
     fields inside a namespaced BTree (`__annotations__`) keyed by the
     behavior's full interface name, guaranteeing zero collisions [V].
- **Key Lesson**: Composition via mixins/behaviors is vastly superior to class
  inheritance for content types. However, when composing multiple behaviors,
  systems must either explicitly namespace field storage or detect and reject
  field name collisions.

### 2.11 Notion API

- **Feature & Architecture**: Notion databases define properties (Title, Text,
  Number, Select, Multi-select, Date, Relation, Rollup, etc.). Pages within a
  database inherit these properties [V]
  (<https://developers.notion.com/reference/property-object>).
- **Validation Timing**: Gating on write [V]. The Notion REST API strictly
  validates payloads against database property definitions, returning HTTP 400
  `validation_error` on type mismatches or unknown properties [V].
- **Select Option Gating**: Unlike the Notion UI (which allows typing new
  options on the fly), the Notion API enforces strict enum validation: passing
  a `select` value that does not already exist in the database schema throws
  a `validation_error` [V]
  (<https://developers.notion.com/reference/update-property-schema-object>).
  Integrations must make an explicit schema-update API call to add the option
  before assigning it to a page [V].
- **Key Lesson**: Gating write APIs on strict enum checks forces external
  clients (and automated agents) into multi-turn roundtrips (inspect schema ->
  update schema -> update page).

### 2.12 Datomic / DataScript

- **Feature & Architecture**: Datomic rejects classes, tables, and document
  schemas entirely [V] (<https://docs.datomic.com/data-model.html>).
  It models all data as atomic facts (datoms: EAVT).
  Attributes are first-class entities with universal identity (`:db/ident`),
  `:db/valueType` (e.g. `:db.type/string`, `:db.type/ref`), and
  `:db/cardinality` (`one` or `many`) [V].
- **Property Reuse**: Any entity can possess any attribute from the universal
  attribute pool. There are no class boundaries restricting attributes to
  specific entities [V].
- **Schema Evolution & Immutability**: Datomic enforces strict schema
  accretion. Attributes can be renamed (`:db/ident`), but their fundamental
  value type (`:db/valueType`) **can never be altered** once defined [V]
  (<https://docs.datomic.com/schema/schema-change.html>). Changing cardinality
  from `many` to `one` is permitted only if no entity currently asserts
  multiple values; otherwise the transaction aborts [V].
- **Enumerations**: Enumerations are represented as independent entities with
  unique `:db/ident` names [V]
  (<https://docs.datomic.com/schema/schema-modeling.html>).
- **Key Lesson**: Universal attributes avoid class-hierarchy rigidity, but
  immutable attribute types demand permanent deprecation and forward aliasing
  when types evolve.

### 2.13 TerminusDB

- **Feature & Architecture**: TerminusDB is a schema-enforced revision-control
  document graph database [V] (<https://terminusdb.com/docs/at-a-glance>).
  Schemas are declared in JSON-LD / OWL, defining classes, properties,
  inheritance (`@subClassOf`), and document keys [V].
- **Validation Timing**: Strictly gating on document commit [V].
- **Schema Migrations**: TerminusDB categorizes all schema changes into two
  distinct classes [V]
  (<https://terminusdb.com/docs/schema-migration-reference-guide/>):
  1. *Weakening* (backward-compatible): Adding optional fields, adding classes,
     widening types. No existing instance data requires modification [V].
  2. *Strengthening* (breaking): Adding required fields, deleting classes,
     narrowing types, renaming fields. These require explicit instance
     transformation operations with default values (`CreateClassProperty`,
     `CastClassProperty`, `MoveClassProperty`) executed in the migration
     transaction [V].
- **Dry-run Mode**: Migration API supports `dry_run=true` to preview the
  effects of schema migrations across all database documents before committing
  [V].
- **Key Lesson**: Classifying schema changes into "weakening" (safe) and
  "strengthening" (breaking) provides a formal, predictable framework for
  schema evolution across persisted documents.

### 2.14 W3C SHACL / Solid Shapes

- **Feature & Architecture**: SHACL (Shapes Constraint Language) is the W3C
  recommendation for validating RDF graphs [V] (<https://www.w3.org/TR/shacl/>).
  Shapes are defined as `sh:NodeShape` and `sh:PropertyShape`, targeting
  nodes by class (`sh:targetClass`) or explicit node IRIs [V].
- **Validation Timing**: Decoupled and advisory [V]. RDF graphs are stored
  independently of shapes. A SHACL processor evaluates a data graph against
  a shapes graph on demand, outputting a `sh:ValidationReport` graph [V].
- **Severity Levels**: Built-in standard severities: `sh:Info`, `sh:Warning`,
  and `sh:Violation` (default) [V]. Clients programmatically inspect the
  report to decide whether to block pipelines or notify users.
- **Open vs Closed World**: By default, SHACL assumes an open world: undeclared
  properties on a node are ignored [V]. If `sh:closed true` is set, unexpected
  properties trigger violations, but this routinely breaks valid metadata
  unless `sh:ignoredProperties` is configured to bypass system properties
  (e.g., `rdf:type`) [V] (<https://rdf-studio.com/shacl/closed-shapes>).
- **Key Lesson**: Decoupling the shapes graph from the data graph allows
  independent evolution and multi-profile validation. However, closed-world
  validation on semi-structured data requires explicit escape hatches
  (`sh:ignoredProperties`) for extraneous metadata.

---

## 3. Cross-Project Comparison Matrix

| Project / Standard | Field Type Set | Required / Optional | Constraints (Min/Max, Regex, Enum) | Composition vs Inheritance & Conflict Rule | Class / Type Declaration | Validation Timing | Open vs Closed World | External Value Sources | Schema Format & Versioning | Diagnostics Surface |
|---|---|---|---|---|---|---|---|---|---|---|
| **Semantic MediaWiki** | Rich (Text, Number, Date, Page, Quantity, Coords) | Optional (unless via Page Forms) | Bounded intervals (`...`), regex (`Allows pattern`), enum (`Allows value`) [V] | Category hierarchy (subcategories); Property pages independent | Category tag on page (`[[Category:X]]`) [V] | Advisory (inline badges, special property `_IMPR`) [V] | Open (any property can be added) | External JSON / wikitext page (`_PVALI`) [V] | Wikitext in `Property:` and `Category:` namespaces [V] | Tooltips, `Special:Browse`, `_IMPR` queries [V] |
| **Cargo** | Relational SQL (String, Integer, Float, Date, Coordinates) | Optional (NULL allowed) | SQL-like constraints in declaration | Relational tables; manual hierarchy queries | Template call (`#cargo_declare`, `#cargo_store`) [V] | Gating on store, advisory on wiki page | Closed per table (undeclared fields dropped) | Relational table queries / lookups [V] | Wikitext template calls; table drop/recreate [V] | Table view, special pages, parser warnings [V] |
| **Wikidata / Wikibase** | Item, Property, String, Time, Quantity, Commons, etc. [V] | Optional (mandatory qualifier constraint available) [V] | Range, regex format, single/multi-value, one-of, conflicts-with [V] | Multi-class (`instance of` P31 / `subclass of` P279); additive constraints [V] | Statements on entity (`P31`) [V] | 100% Advisory (never gates edits) [V] | Open (any statement allowed) | Wikidata items (`one-of` Q-item list) [V] | Wikibase statements (`P2302`) or ShEx (`E...`) [V] | Special:ConstraintReport, UI warning icons [V] |
| **Foswiki / TWiki** | Text, Textarea, Select, Radio, Checkbox, Date, Label [V] | Mandatory attribute `M` [V] | Regex (plugins), select value lists | Single form per topic (`formtemplate`); no multi-inheritance [V] | Topic setting `formtemplate` [V] | Gating at save time for `M` fields [V] | Closed on edit save (unlisted fields purged!) [V] | Topic tables, `%SEARCH%` macro queries [V] | Wiki table in `*Form` topic; no formal versioning [V] | Save-time error page, UI asterisks [V] |
| **DokuWiki `struct`** | Text, Int, Decimal, Date, Dropdown, User, Lookup, Media [V] | Required flag per field [V] | Min/max, regex, dropdown lists, foreign lookups [V] | Schemas assigned by namespace pattern; multiple schemas per page [V] | Assigned via namespace regex config [V] | Gating on form submit, advisory on aggregate [V] | Closed within schema, open to raw wikitext [V] | Schema lookups, external DB tables [V] | GUI editor / SQLite table schema [V] | Form field validation errors, aggregate filters [V] |
| **BookStack** | None (untyped strings) [V] | None (all tags optional) [V] | None (free-form key-value strings) [V] | Fixed hierarchy (Shelf -> Book -> Chapter -> Page); no classes [V] | Tag list in page metadata [V] | Never (no validation) [V] | Open (any tag allowed) [V] | None (typeahead suggestions only) [V] | None (relational DB schema fixed in code) [V] | None [V] |
| **Confluence** | None (rich text cells) [V] | None [V] | None [V] | Page label grouping; no classes [V] | Page label + Page Properties macro [V] | Never (parsing only) [V] | Open (arbitrary table rows) [V] | None [V] | None [V] | Broken report tables, silent omissions [V] |
| **XWiki** | String, TextArea, Boolean, Date, StaticList, DBList, etc. [V] | Mandatory property flag [V] | Validation regex, static list enums [V] | Multi-class composition (multiple XObjects per page); independent namespaces [V] | XObject attached to document [V] | Gating on UI form save [V] | Open (documents hold arbitrary XObjects) [V] | DatabaseList queries, external documents [V] | XClass document; refactoring wizards [V] | Form save error tooltips, validation popups [V] |
| **Drupal** | Typed Data (Text, Number, Date, EntityRef, File, etc.) [V] | Required setting on field config [V] | Min/max, regex, allowed values, plugins [V] | Content types (bundles) compose shared `FieldStorageConfig` ($ref) [V] | Bundle property in entity entity-type [V] | Gating on entity save [V] | Closed per bundle (strict entity field schema) [V] | Allowed values callbacks, taxonomy terms [V] | YAML config sync (`field.storage.*`, `field.field.*`) [V] | Form validation errors, `ConstraintViolationList` [V] |
| **Plone Dexterity** | `zope.schema` (Text, Int, Datetime, Choice, Relation) [V] | `required=True/False` [V] | Min, max, regex constraint functions, Vocabularies [V] | Composition via Behaviors (mixins); annotation storage prevents collision [V] | FTI `behaviors` list [V] | Gating on form save [V] | Closed per schema/behavior set [V] | Named Vocabularies / Source Binders [V] | XML / Python interfaces; GenericSetup [V] | `zope.interface.Invalid` exceptions, form errors [V] |
| **Notion API** | Title, RichText, Number, Select, MultiSelect, Date, Relation [V] | None natively (API requires valid types) [V] | Select enum list, number format [V] | Single database parent; relation properties link databases [V] | Database membership (`parent.database_id`) [V] | Gating on API write (HTTP 400 `validation_error`) [V] | Closed (unknown properties rejected) [V] | Pre-declared database select options [V] | Notion database schema API [V] | JSON validation error payload [V] |
| **Datomic** | Scalar types (String, Long, Double, Instant, Ref, etc.) [V] | None (schema is sparse/attribute-only) [V] | Unique identity, cardinality (one/many) [V] | Universal attribute pool (no classes); entities compose any attributes [V] | Entity ID has attribute datoms [V] | Gating on transaction commit [V] | Open (any entity accepts any attribute) [V] | Enumeration entities (`:db/ident`) [V] | Transactional schema facts; immutable value types [V] | Transaction abort exceptions [V] |
| **TerminusDB** | XSD types, Subdocuments, Document references [V] | Required fields (unless wrapped in `Optional`) [V] | Regex pattern, min/max lexical, enums [V] | Inheritance (`@subClassOf`); weakening vs strengthening rules [V] | `@type` property in JSON document [V] | Gating on document commit [V] | Closed per class schema [V] | Enum documents, referenced documents [V] | JSON-LD schema; commit graph migration operations [V] | API error response, migration report [V] |
| **W3C SHACL / Solid** | RDF literals, datatypes, node kinds [V] | `sh:minCount 1` [V] | `sh:minInclusive`, `sh:maxInclusive`, `sh:pattern`, `sh:in` [V] | Shapes target classes (`sh:targetClass`); additive property shapes [V] | `rdf:type` on data node [V] | Decoupled / Advisory on demand [V] | Open by default; closed via `sh:closed true` [V] | RDF URIs, `sh:in` literal lists [V] | Turtle / JSON-LD shapes graph [V] | `sh:ValidationReport` (FocusNode, Path, Severity) [V] |

---

## 4. Challenges to Traces' Design

This section examines the architectural decisions in Traces Ticket 20
against the empirical findings from the 14 precedent systems.

### 4.1 Advisory-Only Validation in Core

- **Traces Decision**: Schemas validate note frontmatter as an advisory,
  on-demand check producing structured violations with stable codes. The
  core index and parser never gate on schemas. Gating is purely a host
  policy (CLI flag, MCP guardrail) [V].
- **Supporting Precedent**:
  - *Wikidata / Wikibase*: 100% advisory constraints. Real-world data
    contains legitimate edge cases (Stubbs the cat as mayor). Gating would
    break collaborative knowledge creation [V].
  - *Semantic MediaWiki*: Annotations parse into the triplestore regardless
    of validity; invalid values raise advisory tooltips and `_IMPR` flags,
    preventing user save lockouts [V].
  - *W3C SHACL*: Completely decoupled from the RDF data graph; outputs a
    `sh:ValidationReport` without preventing graph updates [V].
- **Contradicting Precedent**:
  - *Notion API & Drupal*: Enforce strict gating at write time. If a value
    violates a constraint, the commit fails [V].
  - *Foswiki DataForms*: Mandatory (`M`) fields block saving with an error
    screen [V].
  - *TerminusDB*: Rejects documents failing schema constraints on write [V].
- **Recommendation**: **KEEP**. For a local-first markdown PKM system where
  files are edited by arbitrary text editors, Git syncs, and scripts,
  advisory validation in core is strictly correct. Gating note parsing or
  indexing would cause markdown files with slight frontmatter typos to vanish
  from queries and links. Gating belongs exclusively in write-capable hosts
  (MCP agent tools, CLI lint commands).

### 4.2 Multi-Class Resolution: "First Wins" on Conflicting Declarations

- **Traces Decision**: When a note declares multiple classes, field
  inheritance resolves within each class; then for the note as a whole,
  the first-listed class wins conflicting field declarations. An advisory
  `schema.class-conflict` violation is emitted [V].
- **Supporting Precedent**:
  - *Python MRO / Multiple Inheritance*: In classical inheritance with
    multiple parents, an ordered linearization (C3 MRO) dictates that the
    earlier class in the declaration order wins conflicting attribute
    definitions [H].
- **Contradicting Precedent**:
  - *Plone Dexterity*: Explicitly rejected "first wins" attribute masking
    for behaviors. If two behaviors define the same field name, direct
    attribute storage causes silent data corruption. Plone introduced
    `AnnotationStorage` to namespace fields by behavior interface, or treated
    un-namespaced collisions as severe configuration errors [V].
  - *XWiki*: Multi-classing is achieved via composition (attaching multiple
    `XObjects` to a single document). Each XObject maintains its own property
    namespace derived from its specific XClass; properties never mask or
    overwrite each other [V].
  - *Wikidata*: An entity can be an instance of multiple classes (`human`,
    `politician`, `screenwriter`). Constraints across classes are
    *additive/unioned*, not masked. A screenwriter constraint requiring
    `screenwriter (Q28389)` is not cancelled or masked because `human` was
    listed first [V].
- **Recommendation**: **CHANGE / REVISE**. The "first wins" conflict rule is
  a dangerous shortcut. If `Project` defines `status` as an enum (`active`,
  `done`) and `Meeting` defines `status` as a date (`scheduled_for`), declaring
  `class = ["Project", "Meeting"]` arbitrarily forces `status` to be an enum,
  silently invalidating the meeting date. Emitting `schema.class-conflict` is
  good, but masking the declaration under "first wins" creates fragile
  ordering dependencies (reordering frontmatter classes breaks validation).
  *Concrete recommendation*: For conflicting declarations, report a blocking
  or high-severity `schema.class-conflict` diagnostic, and do not validate
  the field against either class until the collision is resolved, or adopt
  Plone/XWiki's pattern where fields can be qualified (`Project.status` vs
  `Meeting.status`).

### 4.3 Constraint Set: Required, Multi, Min, Max, Step, Date Format

- **Traces Decision**: Keeps `required`, `multi`, `min`, `max`, `step`, and
  `date.format` as verified production options checked by `schema` [V].
- **Supporting Precedent**:
  - *W3C SHACL*: Implements `sh:minCount` (required), `sh:maxCount` (multi),
    `sh:minInclusive`/`sh:maxInclusive` (min/max), and `sh:pattern` [V].
  - *TerminusDB*: Implements lexical ranges and required/optional splits [V].
  - *Foswiki*: Mandatory flag `M` is heavily utilized across wikis [V].
- **Contradicting Precedent**:
  - *Confluence*: Omitted all constraint types; resulted in severe user
    dissatisfaction and unmaintainable aggregation tables [V].
  - *BookStack*: Rejected constraints to minimize maintenance overhead [V].
- **Recommendation**: **KEEP**. This bounded set represents the exact baseline
  required to prevent frontmatter chaos without requiring a Turing-complete
  expression engine.

### 4.4 Schemas Carrying Value Sources (External Lists)

- **Traces Decision**: Select fields support external value sources (JSON
  lists of value/label/link) [V].
- **Supporting Precedent**:
  - *Semantic MediaWiki*: Hard-coded `[[Allows value::...]]` lists in
    property definitions became unwieldy for large vocabularies (e.g. ISO
    country codes, licenses). SMW 2.5 introduced `[[Allows value list::...]]`
    (`_PVALI`) specifically to reference external MediaWiki or JSON pages
    [V].
  - *Foswiki DataForms*: `select` fields with empty value columns dynamically
    load value tables from separate topics or `%SEARCH%` queries [V].
  - *Plone*: Decouples schema fields from allowable values via named
    `Vocabularies` and `SourceBinders` [V].
- **Contradicting Precedent**:
  - None. Every mature structured data system that started with inlined enum
    values eventually added externalized vocabulary lists.
- **Recommendation**: **KEEP**. Inlined enums in schema TOML files break down
  when lists exceed 20-30 entries. External JSON list referencing is
  essential for real-world PKM vaults (taxonomies, journal names, tags).

### 4.5 Class-in-Frontmatter vs Folder, Tag, or Registry

- **Traces Decision**: A note explicitly names its class(es) in frontmatter
  (default field `class`, configurable) [V].
- **Supporting Precedent**:
  - *XWiki*: Documents explicitly declare attached XObjects [V].
  - *Drupal*: Entities explicitly store their bundle name (`node.type`) [V].
  - *Wikidata*: Entities declare `instance of (P31)` in their statements [V].
- **Contradicting Precedent**:
  - *DokuWiki `struct`*: Schemas are assigned by folder/namespace patterns
    in global configuration [V]. Notes do not need to self-declare their
    schema; placing a page in `recipes/` automatically binds the recipe schema.
  - *Cargo*: Schemas are bound by template invocation (`{{Recipe ...}}`),
    not a separate metadata field [V].
- **Recommendation**: **KEEP class-in-frontmatter, but DEFER folder-based
  defaults**. Explicit frontmatter declaration ensures notes remain
  self-describing when moved across folders. However, DokuWiki's precedent
  shows that users strongly appreciate folder-based fallback (e.g., all
  notes in `daily/` implicitly inherit class `DailyNote` without requiring
  boilerplate frontmatter).

### 4.6 Strict TOML Schema Format

- **Traces Decision**: Schemas are strict TOML files where unknown keys are
  errors [V].
- **Supporting Precedent**:
  - *Drupal Config Schema*: YAML schema definitions strictly validate config
    files; undeclared keys fail schema validation [V].
  - *TerminusDB*: Schema documents are strictly typed JSON-LD [V].
- **Contradicting Precedent**:
  - *Semantic MediaWiki*: Wikitext-based property definitions tolerate
    arbitrary wikitext and formatting [V].
- **Recommendation**: **KEEP**. Strict parsing of schema files prevents silent
  misconfigurations (e.g. misspelled options like `requred = true` or
  `values_list = ...`). Schema files are maintained by vault owners and
  must be held to strict syntax invariants.

### 4.7 Inheritance vs Composition / Mixins

- **Traces Decision**: Schemas use `extends` (multiple inheritance, first
  wins), `$ref` to other fields, and `excludes` [V].
- **Supporting Precedent**:
  - *TerminusDB*: Supports `@subClassOf` class inheritance [V].
- **Contradicting Precedent**:
  - *Plone*: Completely abandoned class inheritance (`Archetypes`) because
    deep inheritance caused fragile base classes and prevented selective
    feature reuse. Plone switched entirely to behavior composition
    (mixins) [V].
  - *Drupal*: Uses bundle composition over shared `FieldStorageConfig`
    definitions (`$ref`), avoiding class inheritance entirely [V].
  - *XWiki*: Replaced inheritance with multi-object composition [V].
- **Recommendation**: **CHANGE / SIMPLIFY**. Deep multiple inheritance with
  `extends` and `excludes` re-introduces the notorious fragile base class
  problem that Plone spent years escaping. Traces' `$ref` mechanism (reusing
  independent field declarations across classes) already provides clean
  composition.
  *Concrete recommendation*: Deprecate multiple inheritance (`extends =
  ["A", "B"]`). Instead, adopt Plone's behavior/mixin model: allow classes
  to compose named field groups (mixins) via `$ref` or a `mixins` list,
  without full class inheritance hierarchies.

### 4.8 What Automated Agents and LLM Tooling Need

- **Traces Design**: Pure advisory checks over resolved fields, stable
  violation codes, bounded expected sets (`strsim` suggestions), delta checks
  `(field, code, hash(value))`, and class-change confirmation channels [V].
- **Supporting Precedent**:
  - *Notion API*: The pain point of the Notion API is that unexpected option
    values throw opaque errors without returning allowable options, forcing
    agents to make extra roundtrips [V]. Bounded expected sets solve this.
  - *TerminusDB Dry-Run*: TerminusDB's `dry_run=true` API allows programmatic
    callers to test changes and receive structured migration previews before
    committing [V]. Traces' planned `check_note(before, after)` directly
    matches this requirement.
- **Recommendation**: **KEEP and PRIORITISE**. Automated agents cannot
  effectively fix frontmatter errors without stable machine-readable codes
  and bounded candidate sets. The delta rule (blocking only newly introduced
  violations) is critical to prevent an agent from getting trapped by
  pre-existing legacy frontmatter errors in old notes.

---

## 5. Failure Stories and Regrets

- **Semantic MediaWiki Property Type Re-indexing Queue Collapse**:
  Changing a property's datatype in SMW requires dispatching asynchronous
  jobs (`smw.changePropagationDispatch`) to re-parse every wiki page using
  that property. On large wikis, this flooded the MediaWiki job queue,
  causing queries to return corrupted or mixed data types for days until
  jobs cleared [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Changing_property_types>).
- **Semantic MediaWiki Silent Failure in `{{#set:...}}`**:
  SMW's parser function `{{#set: Property=Value }}` silently ignores values
  that violate `Allows value` constraints, storing nothing without raising an
  error during page edit. Admins had to write custom maintenance queries
  against special property `_IMPR` to discover that users had been entering
  invalid data for years [V]
  (<https://www.semantic-mediawiki.org/wiki/Help:Special_property_Allows_value>).
- **Wikidata Constraint False-Positive Appeasement**:
  On Wikidata, 4.6% of constraints acquired manual exceptions (`P2303`).
  Because property constraints were often configured too broadly, editors
  faced constant violation warnings on valid items. Instead of reporting or
  loosening the constraint, editors routinely added the item as an explicit
  exception (`P2303`) to silence the UI badge, turning the constraint database
  into an unmaintainable catalogue of ad-hoc overrides [V]
  (<https://m.wikidata.org/wiki/Wikidata:2020_report_on_Property_constraints>).
- **XWiki Five-Year Block on Property Deletion (XWIKI-99)**:
  Deleting or renaming a property in an XClass left orphaned database columns
  and raw data records across all existing documents in the wiki. Resolving
  how to safely delete class properties without corrupting document history
  remained an unresolved major blocker for five years (2005 to 2010) [V]
  (<https://jira.xwiki.org/browse/XWIKI-99>).
- **XWiki Historical Document Diff Corruption (XWIKI-23815)**:
  XWiki historical revision diffs evaluated old document objects against the
  *current* XClass definition rather than the historical XClass snapshot.
  When properties were renamed or reused with different types, viewing
  older history rendered wrong values or threw NullPointerExceptions
  [V] (<https://jira.xwiki.org/browse/XWIKI-23815>).
- **Drupal Field Storage Lock-in**:
  Drupal's `FieldStorageDefinitionUpdateForbiddenException` permanently blocks
  developers from altering field types or decreasing cardinality once an entity
  has data. When requirements change for one content type, the entire site
  is locked because another content type shares the storage [V]
  (<https://api.drupal.org/api/drupal/core!lib!Drupal!Core!Entity!Exception!FieldStorageDefinitionUpdateForbiddenException.php/class/FieldStorageDefinitionUpdateForbiddenException/main>).
- **Plone Archetypes Deep Inheritance Collapse**:
  Plone's initial framework (Archetypes) relied on deep subclassing of
  content classes. Adding or modifying fields in base classes cascaded
  unpredictable breakage across third-party types, leading to massive technical
  debt and forcing a complete framework rewrite to Dexterity behaviors [V]
  (<https://5.docs.plone.org/external/plone.app.dexterity/docs/intro.html>).
- **Plone Attribute Namespace Collision**:
  Composing Dexterity types by attaching multiple behaviors that store fields
  as plain object attributes resulted in silent collisions when two behaviors
  used common names (e.g., `description` or `date`). Plone had to engineer
  `AnnotationStorage` to namespace fields inside ZODB BTrees to prevent
  attribute clobbering [V]
  (<https://5.docs.plone.org/external/plone.app.dexterity/docs/behaviors/schema-only-behaviors.html>).
- **Datomic Immutable Attribute Type Regret**:
  In Datomic, `:db/valueType` cannot be altered after an attribute is
  installed. Developers who inadvertently installed `:type/string` for an
  identifier that later required `:type/uuid` or `:type/ref` could never
  migrate the attribute; they were forced to create `:item/uuid-v2` and
  maintain dead attribute metadata indefinitely [V]
  (<https://docs.datomic.com/schema/schema-change.html>).
- **Foswiki Silent Metadata Purging**:
  When a form definition is altered in Foswiki (deleting a field), editing
  and saving an existing topic silently strips the deleted field's data
  from the topic file without warning the user [V]
  (<https://foswiki.org/System/DataForms>).
- **Confluence Page Properties Report Whitespace Hazard**:
  Because Confluence lacks schema definitions, property keys are parsed from
  table header strings. A trailing space in a table header (`"Owner "`)
  creates a separate, blank column in the aggregation report, causing team
  members to miss deadlines because tasks vanished from reports [V]
  (<https://community.atlassian.com/forums/discussion/2347928/page-properties-report-macro-filter-forgetting-the-not>).
- **Notion API Select Option Gating Barrier**:
  The Notion API's hard gating on select options (throwing HTTP 400 if an
  option does not already exist in the database) breaks automated import
  scripts and agents whenever new tags appear, requiring two-phase schema
  mutation roundtrips [V]
  (<https://developers.notion.com/reference/property-object>).

---

## 6. Transferable Lessons

1. **Decouple Note Parsing from Schema Validation**:
   Every system that tied basic file parsing or indexing to schema conformance
   (Drupal, Notion, Datomic) suffered from severe rigidity and complex
   migration hurdles. Systems that kept storage and parsing open while treating
   schemas as independent lens/validation overlays (W3C SHACL, Wikidata,
   Semantic MediaWiki) remained resilient to schema drift. Traces' decision to
   keep note parsing schema-free is strongly validated.
2. **Beware the "First Wins" Fallacy in Multi-Classing**:
   Masking conflicting field declarations behind an arbitrary declaration
   order ("first wins") is an anti-pattern. Real-world systems either union
   constraints additively (Wikidata, SHACL) or isolate field namespaces
   (Plone `AnnotationStorage`, XWiki XObjects). Traces should flag field
   type collisions as fatal schema conflicts rather than silently picking
   the first class.
3. **Prefer Flat Mixins / `$ref` over Deep Multiple Inheritance**:
   Plone, Drupal, and XWiki all demonstrated that class inheritance
   hierarchies (`extends`) degrade rapidly under schema evolution. Reusable
   field definitions (`$ref`) and flat mixins provide all the necessary
   modularity without the fragile base class problems of multi-parent
   inheritance.
4. **Externalize Large Value Vocabularies Early**:
   Embedding large allowed-value lists inside class definitions clutters
   schemas and slows down tooling. Following SMW (`_PVALI`) and Plone
   (Vocabularies) by referencing external JSON lists is the proven standard.
5. **Agent Tooling Requires Bounded Suggestions and Delta Guardrails**:
   Programmatic agents cannot handle boolean "valid/invalid" verdicts. They
   require stable error codes, bounded candidate correction sets (Levenshtein/
   `strsim` matches), and delta-based gating that prevents pre-existing
   legacy violations from blocking incremental edits.
6. **Class-Change Confirmation Prevents Data Loss**:
   Foswiki's silent purging of orphaned fields upon form change demonstrates
   why changing a note's class must be a gated, high-friction operation.
   Traces' requirement for user confirmation on class changes is directly
   justified by the failure modes of Foswiki and XWiki.
