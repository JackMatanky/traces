# External Precedent: Schema Languages, Linters, and Multi-Class Models

- **Document:** `round4/D-schema-languages-and-linters.md`
- **Topic:** External precedent across schema definition languages, constraint
  engines, configuration systems, data modeling frameworks, frontmatter
  linters, and Language Server Protocol (LSP) diagnostics for Ticket 20
  (Schema Note Typing and Validation).
- **Scope & Focus:** Deep structural investigation of required vs optional
  semantics, open vs closed world models, inheritance and composition,
  conflict resolution in multi-class notes, validation timing (advisory vs
  gating), error reporting surfaces (source spans and codes), schema
  versioning, and maintainer regrets across production ecosystems.
- **Priority Precedents:** LDAP objectclasses (RFC 4512/4519) and W3C SHACL,
  analyzing how they resolve multi-class composition and open-world validation.

---

## 1. Scope & Investigated Projects

This research examines 18 distinct systems, standards, and libraries across
data modeling, directory services, semantic web, cloud infrastructure,
configuration languages, markdown tooling, and the Rust ecosystem.

### Directly Examined via Specification, Source Code, and Issues [V]

1. **LDAP Directory Information Models (RFC 4512, RFC 4519, X.501):**
   Read RFC 4512 specification text (§2.4, §2.5, §4.1-4.3) on structural vs
   auxiliary object classes, MUST/MAY attribute sets, superclass inheritance
   (`SUP`), global attribute type definitions, and `extensibleObject`.
   URL: `https://www.rfc-editor.org/rfc/rfc4512.html`
2. **W3C Shapes Constraint Language (SHACL):**
   Read W3C Recommendation 20 July 2017 text on `NodeShape`, `PropertyShape`,
   targets (`sh:targetClass`), open-world defaults, closed shapes
   (`sh:closed`), permitted escapes (`sh:ignoredProperties`), and validation
   severities (`sh:Violation`, `sh:Warning`, `sh:Info`).
   URL: `https://www.w3.org/TR/shacl/`
3. **JSON Schema (Draft-07, Draft 2019-09, Draft 2020-12):**
   Read specification releases on `allOf`, `$ref`, `additionalProperties`,
   the introduction of dynamic `unevaluatedProperties`, and Ajv performance
   implications of dynamic applicators.
   URLs: `https://json-schema.org/draft/2019-09/release-notes`,
   `https://github.com/ajv-validator/ajv/blob/master/docs/json-schema.md`
4. **Editor Schema Associations & CST Diagnostic Mappings:**
   Examined `yaml-language-server` (Red Hat) and `taplo` (TOML toolkit / LSP).
   Read source and documentation on how schema associations map JSON Pointer
   error locations (`instance_path`) to concrete syntax tree (CST) byte spans
   (`Rowan` in Taplo, `yaml-ast-parser` in YAML LSP).
   URLs: `https://github.com/redhat-developer/yaml-language-server`,
   `https://taplo.tamasfe.dev/`
5. **Markdown Frontmatter Linters & Formatters:**
   Examined `remark-lint-frontmatter-schema`, `markdownlint`, `Vale`, and
   `mdformat`. Read documentation on `remark-lint-frontmatter-schema` Ajv
   validation, Vale's dynamic frontmatter scoping (`text.frontmatter.<key>`),
   and `markdownlint` frontmatter stripping issues.
   URLs: `https://github.com/JulianCataldo/remark-lint-frontmatter-schema`,
   `https://docs.vale.sh/formats/front-matter.md`,
   `https://github.com/DavidAnson/markdownlint`
6. **Markdown PKM Language Servers & Agents:**
   Examined `iwe` (Interactive Writing Environment / PKM LSP & MCP memory),
   `markdown-oxide` (Rust Obsidian PKM LSP), `marksman` (Markdown LSP), and
   `rumdl` (Rust Markdown linter). Read IWE's `schematter` frontmatter
   validation, strict MCP agent gating, and LSP diagnostic reporting.
   URLs: `https://github.com/iwe-org/iwe`,
   `https://github.com/Feel-ix-343/markdown-oxide`,
   `https://github.com/artempyanykh/marksman`
7. **CUE (Configure, Unify, Execute):**
   Read language specification and design rationale by Marcel van Lohuizen on
   the value-type lattice, commutative/associative unification (`&`), rejection
   of inheritance and overrides, and closed definitions (`#Def: {...}`).
   URLs: `https://cuelang.org/docs/concept/configuration-use-case/`,
   `https://cuelang.org/docs/concept/the-logic-of-cue/`
8. **Apple Pkl Configuration Language:**
   Read Pkl language reference and object model on single-inheritance classes,
   dynamic object amending (`amends`), property constraints, and closed classes.
   URLs: `https://pkl-lang.org/main/current/language-reference/index.html`,
   `https://github.com/apple/pkl`
9. **Tweag Nickel Configuration Language:**
   Read Nickel user manual on symmetric record merging (`&`), contracts,
   merge priorities (`default`, numeral, `force`), and optional field promotion.
   URLs: `https://nickel-lang.org/user-manual/merging/`,
   `https://nickel-lang.org/user-manual/contracts/`
10. **Kubernetes CustomResourceDefinition (CRD) Structural Schemas & OpenAPI:**
    Read KEP-2335 ("Vanilla CRD OpenAPI Subset: Structural Schemas"), Kubernetes
    1.15/1.16 structural schema blog post, pruning mechanics, and CEL validation
    rules. URLs:
    `https://v1-36.docs.kubernetes.io/blog/2019/06/20/crd-structural-schema/`,
    `https://github.com/kubernetes/kubernetes/issues/111074`
11. **Spotify Backstage Catalog Model:** Read Backstage catalog entity envelope
    specification (`Entity.schema.json`) and `@backstage/catalog-model`
    validator (`entityKindSchemaValidator.ts`). URLs:
    `https://github.com/backstage/backstage/blob/master/packages/catalog-model/src/schema/Entity.schema.json`,
    `https://github.com/backstage/backstage/blob/master/packages/catalog-model/src/validation/entityKindSchemaValidator.ts`
12. **Home Assistant & ESPHome Configuration Validation:** Read Home Assistant
    core `config_validation.py` and ESPHome validation modules built on
    Voluptuous (`cv.has_at_least_one_key`, `cv.exclusive`). URLs:
    `https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/config_validation.py`,
    `https://github.com/esphome/esphome/blob/dev/esphome/config_validation.py`
13. **Data Modeling & Serialization Libraries:**
    Examined Pydantic v1/v2 (Python), Zod (TypeScript), and Serde (Rust). Read
    GitHub issues regarding Serde's `deny_unknown_fields` interactions with
    `#[serde(flatten)]` and untagged enums, and Pydantic MRO inheritance bugs.
    URLs: `https://github.com/serde-rs/serde/issues/1547`,
    `https://github.com/serde-rs/serde/issues/2283`,
    `https://github.com/pydantic/pydantic/issues/4480`
14. **Rust Validation & Diagnostic Crates:**
    Examined `garde` (compile-time path-based validation), `validator` (derive
    macro), `schemars` (JSON Schema generation), `serde_valid`, and `miette`
    (diagnostic reporting).
    URLs: `https://crates.io/crates/garde`,
    `https://crates.io/crates/validator`,
    `https://crates.io/crates/miette`

### Examined from Architectural Knowledge & Verified Context [H]

15. **Protocol Buffers (Google):** Proto2 `required` vs `optional`; proto3
    removal of `required` (Google post-mortem on compatibility traps);
    proto3.15+
    optional field presence tracking; `reserved` field tags and names.
16. **GraphQL:** Schema Definition Language (SDL) `interface` and `union`
    constructs; strict covariant field implementation; lack of input unions.
17. **Terraform (HashiCorp HCL):** Variable `validation` blocks with condition
    expressions and custom error messages; optional object attributes with
    defaults (`optional(type, default)`).
18. **Dhall Configuration Language:** Total functional programming language;
    typed records and union types; absence of inheritance; record completion.

---

## 2. Per-Project Deep Dives

### 2.1. LDAP Directory Information Models (RFC 4512 / RFC 4519)

- **What it is:** The global directory standard (derived from X.500 X.501)
  governing hierarchical entries composed of typed attributes and object
  classes [V: RFC 4512 §1].
- **How fields, types, inheritance, and multi-class work:**
  - *Decoupled attribute definitions:* In LDAP, attributes are **not** defined
    inside object classes [V: §2.5]. Attribute types (`attributeTypes`) are
    globally defined schema entities with their own OID, syntax (e.g. Directory
    String, Integer), single vs multi-valued constraint (`SINGLE-VALUE`), and
    matching rules (`EQUALITY`, `ORDERING`, `SUBSTR`) [V: §4.1.2].
  - *Object class kinds:* Classes are strictly categorized into three kinds:
    `ABSTRACT`, `STRUCTURAL`, and `AUXILIARY` [V: §2.4].
    - `ABSTRACT`: Base classes (such as `top`) that cannot appear alone on an
      entry [V: §2.4.1].
    - `STRUCTURAL`: Defines the primary real-world entity and its position in
      the Directory Information Tree (DIT) [V: §2.4.2]. An entry **MUST** belong
      to **precisely one** structural object class chain [V: §2.4.2]. You
      cannot combine two independent structural classes (e.g. `person` and
      `organization`) on one entry!
    - `AUXILIARY`: Mixin classes used to augment an entry with additional
      characteristics (e.g. `posixAccount`, `mailRecipient`) [V: §2.4.3]. An
      entry may belong to zero or more auxiliary classes.
  - *Field lists:* Object classes declare only two lists: `MUST` (required
    attributes) and `MAY` (optional attributes) [V: §4.1.1].
  - *Inheritance (`SUP`):* An object class may inherit from superclasses via
    `SUP`. Structural classes can only subclass structural or abstract classes;
    auxiliary classes cannot subclass structural classes [V: §2.4.2, §2.4.3].
  - *Multi-class conflict resolution:* When an entry lists multiple classes (one
    structural class plus multiple auxiliary mixins):
    1. The entry's required attributes are the **set union** of all `MUST`
       attributes across all assigned classes and their superclass chains [V:
       §2.4].
    2. The entry's allowed attributes are the **set union** of all `MAY`
       attributes across all assigned classes and their superclass chains [V:
       §2.4].
    3. If an attribute is listed in `MUST` by one class and in `MAY` by another,
       **`MUST` wins**: the attribute becomes required [V: §2.4: "If an
       attribute is a member of both sets, it is required to be present."].
    4. **Type conflicts are impossible by design:** Because classes only name
       attributes and do not define their types, two classes can never disagree
       on whether an attribute is an integer or a string.
- **Open vs Closed World:**
  - By default, LDAP is **closed-world**: an entry cannot contain any attribute
    that is not explicitly listed in the `MUST` or `MAY` sets of its assigned
    object classes [V: §2.4].
  - To support an open-world model, LDAP defines a standardized escape hatch:
    the
    `extensibleObject` auxiliary class (OID 1.3.6.1.4.1.1466.101.120.111)
    [V: §4.3]. An entry containing `extensibleObject` may hold any user
    attribute defined in the directory schema, while still enforcing the `MUST`
    requirements of its other classes.
- **Validation timing:** Hard gating at write time (LDAP `ADD` and `MODIFY`
  operations). If an unknown attribute is supplied or a `MUST` attribute is
  omitted, the server rejects the operation immediately with
  `LDAP_OBJECT_CLASS_VIOLATION` (result code 65) [V: RFC 4511 §4.1.9].
- **Architectural placement:** The directory schema resides in subschema
  subentries [V: §4.2], discovered via the operational attribute
  `subschemaSubentry`. Storage engines store raw attribute-value sets indexed by
  numeric attribute OIDs.
- **User complaints & maintainer regrets:**
  - Inability to change structural class: RFC 4512 §2.4.2 mandates that "The
    structural object class of an entry shall not be changed" [V]. In practice,
    users frequently needed an entity to evolve (e.g. from a `person` to an
    `inetOrgPerson` or `accountant`), requiring deleting and re-creating the
    entry or non-standard server extensions.
  - The single-structural-class restriction forced directory administrators to
    create artificial single-inheritance hierarchies or rely heavily on
    unconstrained auxiliary classes.
- **Key lesson for Traces:**
  - **Decouple attribute/field type definitions from class membership.** If
    field
    types and value constraints are defined globally or referenced cleanly,
    multi-class notes do not suffer from conflicting field definitions.
  - **Conflict rule:** When multiple classes overlap on optionality, `MUST`
    unions with `MAY` to make the field **required** (conjunction of
    constraints). "First wins" is unnecessary when constraints compose
    logically.
  - Single structural identity + multiple mixins (auxiliary classes) is far more
    disciplined than unconstrained multiple inheritance.

---

### 2.2. W3C Shapes Constraint Language (SHACL)

- **What it is:** The W3C Recommendation standard for describing and validating
  Resource Description Framework (RDF) graph data [V: SHACL §1].
- **How fields, types, inheritance, and validation work:**
  - *Shapes over graphs:* SHACL separates data graphs from shapes graphs. A
    shape
    specifies constraints on focus nodes using `sh:NodeShape` (constraints on
    the node itself) and `sh:PropertyShape` (constraints on property values)
    [V: §2].
  - *Targets:* Shapes bind to data via target declarations: `sh:targetClass`
    (binds to instances of an RDF class), `sh:targetNode`,
    `sh:targetSubjectsOf`,
    and `sh:targetObjectsOf` [V: §2.1.3].
  - *Constraints:* Rich declarative constraint components: `sh:datatype`,
    `sh:class`, `sh:minCount` (required when >= 1), `sh:maxCount`,
    `sh:minInclusive`,
    `sh:maxInclusive`, `sh:pattern` (regex), `sh:in` (enums), and `sh:hasValue`
    [V: §4].
  - *Severity levels:* Every constraint can declare an explicit severity:
    `sh:severity sh:Violation` (default), `sh:Warning`, or `sh:Info` [V:
    §3.4.4].
    Validation produces a structured validation report containing
    `sh:ValidationResult` nodes tagged with the corresponding severity [V:
    §3.4].
  - *Multi-shape evaluation on a node:* A focus node may match multiple shapes
    (e.g. targeted by multiple `sh:targetClass` shapes or boolean combinators).
    In SHACL, each shape validates **independently**. The results are
    accumulated into the validation report. There is no concept of one shape
    "overriding" another; constraints are conjunctive [V: §3.4].
- **Open vs Closed World:**
  - By default, SHACL adheres to RDF's **open-world assumption**: properties on
    a
    focus node that are not mentioned in a shape are silently ignored [V: §5.6].
  - *Closed shapes:* A shape can opt into closed-world validation by declaring
    `sh:closed true` [V: §5.6.2]. When closed, any property on the focus node
    that does not have an explicit `sh:property` shape causes a validation
    violation.
  - *Escapes via `sh:ignoredProperties`:* Because closed shapes would otherwise
    reject metadata like `rdf:type`, SHACL requires `sh:ignoredProperties` to
    specify an RDF list of predicates (such as `rdf:type`) permitted on the
    node without triggering a violation [V: §5.6.2].
- **Validation timing:** **Purely advisory by specification** [V: §3]. The SHACL
  specification defines the shape evaluation and the resulting
  `sh:ValidationReport` graph (`sh:conforms` boolean, list of `sh:result`). It
  deliberately leaves enforcement policy (abort write, prompt user, log, or
  filter) to the host environment.
- **Architectural placement:** The SHACL engine runs as a detached evaluation
  pass over an RDF graph. It does not parse or store the graph.
- **User complaints & maintainer regrets:**
  - *The closed shape composition trap:* If Shape A targets class `Person` with
    `sh:closed true`, and Shape B targets class `Employee` (which subclasses
    `Person`) adding property `ex:employeeId`, validating an `Employee` node
    against Shape A **fails**! Shape A flags `ex:employeeId` as an unexpected
    property because Shape A does not know about Shape B's properties [V: W3C
    Shapes WG ISSUE-95, ISSUE-196]. Closed shapes severely impede modular
    composition and mixin design in RDF.
  - *Lack of standard shape inheritance:* SHACL core does not define object-
    oriented shape inheritance with field overriding. Users must rely on
    `sh:and` (which can create unsatisfiable conjunctions if types conflict) or
    non-standard extensions like DASH `dash:subShapeOf`.
- **Key lesson for Traces:**
  - SHACL proves the power of **decoupling validation from parsing and
    storage**.
    Validation evaluates over a pure graph/map and outputs a structured
    diagnostic
    report with stable severity levels (`Violation`, `Warning`, `Info`).
  - **Beware closed-world composition:** Closed classes immediately break when
    combined with mixins or multi-class notes unless the engine merges allowed
    properties across all active classes before checking for unknown fields.

---

### 2.3. CUE (Configure, Unify, Execute)

- **What it is:** A declarative configuration and validation language developed
  at Google by Marcel van Lohuizen (co-designer of Borg/Kubernetes config
  systems) [V: CUE concept docs].
- **How fields, types, inheritance, and validation work:**
  - *Lattice theory & types as values:* In CUE, there is no distinction between
    types and values. Both occupy a single value lattice ordered by subsumption
    ($\sqsubseteq$) [V: "The Logic of CUE"]. A type (like `string` or `int &
    >0`)
    is simply a value that subsumes many concrete instances.
  - *Unification (`&`):* The sole composition mechanism in CUE is unification.
    Merging two declarations `A & B` computes their greatest lower bound.
  - *Order-independence:* Unification is strictly **commutative**,
    **associative**, and **idempotent** ($A \land B = B \land A$) [V:
    "Configuration use case"].
  - *Explicit rejection of inheritance & overrides:* CUE explicitly rejects
    inheritance with overrides [V: "Configuration use case" § Comparisons].
    In override-based inheritance, order matters, making it impossible to
    reason algebraically about where a value came from. In CUE, if schema A
    declares `port: 80` and schema B declares `port: 8080`, `A & B` fails with
    a conflict error (`_|_`, bottom) [V]. Neither declaration "wins".
  - *Open vs closed structs:* Concrete structs `{...}` are open by default.
    Definitions prefixed with `#` (e.g. `#Note: {...}`) define **closed
    structs** [V: CUE definitions tour]. Closed structs disallow undeclared
    fields unless explicitly marked with an ellipsis `...`.
- **Validation timing:** Validation is unified with evaluation. Parsing a CUE
  graph performs full type checking; partial configurations remain valid until
  concretization.
- **Maintainer regrets & design critique of override models:**
  - Marcel van Lohuizen documented the failure of GCL (Google Configuration
    Language) and Jsonnet at scale: deep inheritance hierarchies with overrides
    create "spaghetti configurations" where humans and automated tools cannot
    predict the final resolved value without executing the entire inheritance
    stack in exact order [V: CUE concept docs].
- **Key lesson for Traces:**
  - **"First wins" inheritance in multi-class notes is an anti-pattern.** If a
    note specifies `class: [Book, Media]`, and `Book` and `Media` define the
    same
    field with conflicting constraints, silently picking the first class's
    declaration destroys commutativity ($[Book, Media] \neq [Media, Book]$) and
    hides schema design errors. Conflicts should either be validated as a
    hard/advisory `class-conflict` error or unified as mutual constraints.

---

### 2.4. Tweag Nickel Configuration Language

- **What it is:** Tweag's open-source functional configuration language
  featuring
  lightweight modular schemas and gradual typing with contracts [V: Nickel
  docs].
- **How fields, types, inheritance, and validation work:**
  - *Symmetric record merging (`&`):* Nickel composes records via symmetric
    merging (`left & right == right & left`) [V: "Merging records"].
  - *Metadata annotations:* Metadata is attached to fields using the `|`
    operator: contracts (e.g. `| String`), documentation (`| doc "..."`),
    optionality (`| optional`), and merge priorities [V: § Merging records with
    metadata].
  - *Merge priorities:* To resolve conflicting field values without relying on
    lexical or list order, Nickel introduces explicit priorities [V: § Merge
    priorities]:
    - `default`: lowest priority (overridden by any standard value).
    - Standard value: priority `0`.
    - `priority N`: explicit numeric priority.
    - `force`: highest priority (cannot be overridden).
    - If two merging fields have the same priority and distinct values, the
      merge fails with `error: non mergeable terms` [V].
  - *Optional field promotion:* In Nickel, when `{bar | optional}` is merged
    with `{bar}` (which has no optional annotation), the result is **required**
    (`bar` is not optional anymore) [V: § Optional fields].
- **Validation timing:** Contracts evaluate at runtime when records are
  evaluated or exported to JSON/YAML. Contract failures report detailed blame
  traces and source spans.
- **Key lesson for Traces:**
  - **Conflict resolution must be explicit, not positional.** If fields collide,
    resolving them via explicit priorities or failing with an unmergeable error
    is far safer than positional "first-listed class wins".
  - **Optional + Required = Required:** Nickel confirms LDAP's rule: when an
    optional field declaration meets a required declaration in a multi-class
    composition, the resulting effective field must be required.

---

### 2.5. Kubernetes CustomResourceDefinition (CRD) Structural Schemas (KEP-2335)

- **What it is:** The schema standard for extending the Kubernetes API with
  user-defined custom resources [V: Kubernetes docs].
- **How fields, types, and validation work:**
  - *The 1.15 shift to Structural Schemas (KEP-2335):* In Kubernetes 1.8-1.14,
    CRDs allowed arbitrary OpenAPI v3 validation schemas. This failed
    disastrously
    at scale: non-structural schemas with unconstrained `oneOf`, `anyOf`, and
    `allOf` made it mathematically impossible for the API server to know which
    fields were valid, how to apply default values, and how to safely merge
    patches (Server-Side Apply) [V: KEP-2335, Kubernetes 1.15 blog post].
  - *Structural schema restrictions:* In `apiextensions.k8s.io/v1`, all CRDs
    **must** use a structural schema:
    1. Every object field and array item must declare a non-empty `type`.
    2. Sub-schemas inside `allOf`, `anyOf`, `oneOf`, or `not` cannot introduce
       new properties; they can only restrict existing ones.
  - *Pruning of unknown fields:* By default, the Kubernetes API server
    **prunes (silently deletes)** any field in a submitted resource that is not
    defined in the structural schema [V: CRD documentation].
  - *Escape hatch (`x-kubernetes-preserve-unknown-fields`):* An explicit vendor
    extension `x-kubernetes-preserve-unknown-fields: true` is required to allow
    unstructured or arbitrary maps.
  - *CEL validation rules (K8s 1.25+):* In-line declarative validation using
    Common Expression Language (`x-kubernetes-validations`), replacing slow,
    unreliable admission webhooks with fast, in-process evaluation.
- **Maintainer regrets & user outages:**
  - *The silent pruning disaster (Issue #111074):* Users strongly condemned
    silent pruning [V: kubernetes/kubernetes#111074]. When a user made a typo in
    a manifest (e.g. `replicas` spelled `replcias`), `kubectl apply` succeeded
    without warning, but the API server silently erased the field! Users
    demanded strict validation warnings over silent pruning.
- **Key lesson for Traces:**
  - **Never silently prune or drop unknown frontmatter fields.** In a PKM vault,
    silent pruning would destroy user data.
  - **Open-world storage with advisory diagnostics** is the correct architecture
    for human-written markdown. Preserve all unknown fields verbatim, but report
    advisory `unknown-field` warnings with closest-match suggestions.

---

### 2.6. Serde (`deny_unknown_fields`) & Rust Deserialization

- **What it is:** Rust's standard serialization/deserialization framework [V:
  serde.rs].
- **Mechanics of `deny_unknown_fields` and `flatten`:**
  - Serde structs default to open-world parsing: unrecognized JSON/YAML keys are
    silently discarded.
  - Adding `#[serde(deny_unknown_fields)]` turns the struct into a closed-world
    parser, returning a `custom` deserialization error when an extra key is
    seen.
- **Maintainer regrets & design failures:**
  - *Flattening breaks `deny_unknown_fields` (Serde Issue #1547, #2283):* When
    composing structs via `#[serde(flatten)]`, `deny_unknown_fields` either
    fails to compile, fails at runtime on valid nested data, or silently
    bypasses
    the check [V: serde-rs/serde#1547, #2283]. To implement `flatten`, Serde has
    to buffer unrecognized fields into an intermediate `serde::de::Content` map
    and re-deserialize it, breaking zero-copy parsing and confusing
    unknown-field
    tracking.
  - *Untagged enums error disaster (Serde Issue #2035):* Deserializing
    `#[serde(untagged)]` tries each variant in sequence. When validation fails,
    Serde discards earlier variant errors and returns only the error of the
    **last** variant tried, generating utterly misleading diagnostics for users.
- **Key lesson for Traces:**
  - Do **not** use Serde's `deny_unknown_fields` or untagged enum
    deserialization
    for note frontmatter typing.
  - Parse frontmatter into an untyped, span-preserving map/CST first, and run
    schema checking as an independent semantic pass.

---

### 2.7. JSON Schema Dynamic Applicators (`unevaluatedProperties`)

- **What it is:** The IETF draft standard for declarative JSON validation [V:
  json-schema.org].
- **Mechanics & Evolution:**
  - In Draft-07, `additionalProperties: false` was purely static: it evaluated
    only the `properties` declared within the immediate subschema. If a schema
    used `allOf` to inherit fields from a `$ref` base, `additionalProperties:
    false` on the outer schema flagged all base fields as unexpected! [V:
    Tour of JSON Schema].
  - In Draft 2019-09 and 2020-12, JSON Schema introduced
    `unevaluatedProperties` [V: 2019-09 release notes]. Unlike
    `additionalProperties`, `unevaluatedProperties` is dynamic: it tracks which
    properties were validated by any subschema inside `allOf`, `anyOf`, `oneOf`,
    or `$ref`, and only rejects properties that remained uninspected.
- **Maintainer regrets & performance cost:**
  - In `ajv` (the premiere JS JSON Schema validator), `unevaluatedProperties`
    requires generating dynamic state-tracking code across all branch
    evaluations, incurring severe performance penalties compared to static
    Draft-07 schemas [V: ajv/docs/json-schema.md].
- **Key lesson for Traces:**
  - In multi-class or inherited schemas, unknown-field detection must evaluate
    against the **resolved effective field set** of the entire class union,
    not against individual parent schemas in isolation.

---

### 2.8. Editor LSPs & CST Mappings (`taplo` & `yaml-language-server`)

- **What they are:**
  - `taplo`: Tamas Feher's TOML toolkit and Language Server in Rust [V:
    taplo.tamasfe.dev].
  - `yaml-language-server`: Red Hat's Language Server for YAML [V: github.com].
- **How they map schema errors to source spans:**
  - Both LSPs decouple the schema validator from the file parser.
  - `taplo` validates TOML documents by converting the TOML DOM into a
    `serde_json::Value` and evaluating it with the `jsonschema` Rust crate [V:
    taplo-common/src/schema].
  - Because `jsonschema` returns span-free JSON Pointer paths (`instance_path`,
    e.g. `/server/ports/0`), `taplo` traverses its `Rowan` concrete syntax tree
    down the path segments, locates the exact syntax token, and extracts its
    `rowan::TextRange` byte span to construct an LSP `Diagnostic` [V].
  - `yaml-language-server` does the same: it validates parsed ASTs via
    `vscode-json-languageservice` and maps JSON Pointers to AST node ranges.
- **Key lesson for Traces:**
  - **Completely validates Ticket 20's architectural split.** `schema::check`
    should remain a pure, span-free function that evaluates a field map and
    returns field paths with stable error codes. The host layer (LSP, CLI) maps
    those field paths back to CST byte spans in the note document.

---

### 2.9. Markdown PKM Tooling: IWE (`schematter`), Markdown-Oxide, & Vale

- **IWE (`iwe-org/iwe`):**
  - An open-source Markdown knowledge graph and LSP for PKM and AI agents [V:
    github.com/iwe-org/iwe].
  - Frontmatter schema validation: Every note has YAML frontmatter with a `type`
    field discriminating its schema. Frontmatter shapes, required sections, and
    heading depths are enforced via `schematter` schemas in `.iwe/schemas/` [V].
  - *Strict write gating for AI agents:* Under strict mode, IWE automatically
    **rejects any write operation** executed by an AI agent (via MCP) or CLI
    that would leave frontmatter invalid [V: discussions/370].
  - Real-time LSP diagnostics: Emits squiggly underlines in editors for broken
    links, missing required metadata, and schema violations.
- **Markdown-Oxide (`Feel-ix-343/markdown-oxide`):**
  - A Rust-based PKM Language Server for Obsidian vaults [V: github.com].
  - Parses frontmatter for tags and aliases; evaluates vault-wide unresolved
    wikilink diagnostics across markdown files.
- **Vale (`docs.vale.sh`):**
  - Prose style linter. Dynamically assigns frontmatter fields to scopes
    (`text.frontmatter.title`, `text.frontmatter.description`) [V:
    formats/front-matter.md]. Lints only string-valued fields for prose style,
    ignoring nested data or structured metadata.
- **Key lesson for Traces:**
  - IWE directly validates Traces' agent guardrail strategy: **advisory feedback
    for human interactive editing in LSP, but hard write-gating for AI agents
    operating via MCP tools.**

---

### 2.10. Spotify Backstage Catalog Entity Model

- **What it is:** The open developer portal platform standardizing service
  catalogs via YAML metadata files [V: backstage.io].
- **How entity typing and validation work:**
  - Entities define a standard envelope: `apiVersion`, `kind` (e.g. `Component`,
    `API`, `System`), `metadata`, and `spec` [V: `Entity.schema.json`].
  - Validation: Uses `entityKindSchemaValidator` in `@backstage/catalog-model`
    to validate entities against JSON Schemas based on `kind` [V].
  - Ingestion behavior: Registration APIs strictly reject malformed entities.
    However, when catalog processors harvest YAML files from git repositories,
    parsing is permissive: invalid entities do not abort processing; their
    failure is recorded in the catalog entity status.
- **Key lesson for Traces:**
  - Keep note ingestion permissive so background indexing never crashes on a
    malformed note. Surface schema failures in diagnostic queries and UI status.

---

### 2.11. Home Assistant & ESPHome Configuration Validation

- **What it is:** Python-based home automation platforms with extensive YAML
  configuration schemas [V: home-assistant.io, esphome.io].
- **Mechanics of Voluptuous:**
  - Uses the `voluptuous` library for declarative schema checking.
  - Supports structural constraints: `cv.has_at_least_one_key`, `cv.exclusive`
    (mutually exclusive keys), `cv.deprecated`.
- **Maintainer regrets & architectural transition:**
  - Voluptuous schemas are procedural Python functions. Because they could not
    be cleanly serialized to JSON (`voluptuous-serialize` limitations), Home
    Assistant could not generate dynamic UI forms from YAML schemas.
  - Home Assistant was forced to deprecate YAML configuration for major
    integrations in favor of "Config Entries" and "DataEntryFlow" (structured
    schema UI forms).
- **Key lesson for Traces:**
  - **Schemas must remain declarative data (TOML/JSON), never code or procedural
    macros.** Declarative schemas are essential for driving LSP completion,
    template prompts, and MCP agent input schemas.

---

### 2.12. Rust Validation Crates (`garde`, `validator`, `schemars`, `miette`)

- **`garde` vs `validator`:**
  - `validator` (`Keats/validator`): Popular derive macro. Returns a
    `ValidationErrors` map of `&'static str` to `Vec<ValidationError>`. Does not
    support passing dynamic context (e.g. database lookups) into validation.
  - `garde` (`murex-group/garde`): Modern compile-time validation crate [V:
    crates.io/crates/garde]. Zero allocation on success. Returns a `Report`
    iterator of `(&Path, &Error)`. Accepts an explicit `&Context` argument
    (`validate(&self, &Context)`), allowing validation rules to query external
    state or indexes.
- **`miette`:**
  - Rust's leading diagnostic reporting library [V: crates.io/crates/miette].
  - Uses the `Diagnostic` trait with `code()`, `severity()`, `help()`, and
    `labels()` (source spans). Decouples error generation from terminal/LSP
    rendering.
- **Key lesson for Traces:**
  - Validation error reporting in Rust should follow `garde` and `miette`:
    span-free evaluation returning paths and codes, enriched at the presentation
    boundary with CST spans and severity styling.

---

## 3. Comprehensive Comparison Matrix

| System / Standard | Field Types Supported | Required / Optional Mechanics | Value Constraints Supported | Inheritance / Composition Model | Conflict Resolution Rule | Type/Class Declaration Location | Validation Timing | Open vs Closed World | Diagnostics Surface |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **LDAP (RFC 4512)** | Global OID syntaxes (string, int, DN, time, binary) | Explicit `MUST` (required) and `MAY` (optional) lists | Single/multi-value, syntax matching rules | Multiple inheritance (`SUP`); 1 structural + N auxiliary | `MUST` unions with `MAY` -> required; types cannot conflict | `objectClass` attribute on entry | Gating at write (rejects with result code 65) | Closed by default; open via `extensibleObject` | Result code, matched DN, diagnostic message |
| **W3C SHACL** | Full RDF datatypes & classes (XSD, IRI) | `sh:minCount >= 1` (required), default 0 | Min/max, regex, enums, range, node shape | Boolean combinators (`sh:and`, `sh:or`); no overrides | Conjunction: all shapes must satisfy; conflicts fail | `sh:targetClass`, target node/predicate | Advisory by spec (produces validation report) | Open by default; closed via `sh:closed true` | `ValidationReport`: severity, path, message |
| **CUE** | Lattice types: primitive types are values | Optional marked with `?`; required by default | Numeric bounds, regex, non-empty, disjunctions | Unification (`&`); lattice meet; no inheritance | Non-commutative conflicts fail (`_|_`, bottom) | Scoped struct definitions (`#Def`) | Unified with evaluation / export | Open for structs `{}`; closed for `#Def` | Bottom error: path, conflicting values |
| **Tweag Nickel** | Primitives, records, arrays, custom contracts | `optional` annotation; default is required | Arbitrary user-defined contract predicates | Symmetric record merge (`&`); metadata annotations | Priority system: `default` < `0` < `N` < `force` | In-line contract annotations (`| Contract`) | Runtime evaluation / export | Open records; sealed contracts | Contract blame traces, source spans |
| **JSON Schema (2020-12)** | string, number, integer, boolean, object, array, null | `required: ["field"]` array of field names | min/max, pattern, enum, const, format | Applicators: `allOf`, `anyOf`, `oneOf`, `$ref` | Conjunction in `allOf`: overlapping fields must satisfy both | `$schema` keyword or external association | Advisory (linters) or gating (APIs) | Open by default; closed via `unevaluatedProperties` | JSON Pointer `instance_path`, error keyword |
| **Kubernetes CRD** | OpenAPI v3 subset: primitive, object, array | `required: ["field"]` in structural schema | min/max, pattern, enum, CEL validation expressions | Strict structural schemas; no property intro in allOf | Fields cannot conflict; structural schemas disallow | `apiVersion` and `kind` in YAML resource | Gating at admission (API server rejects) | Pruned by default; open via `x-kubernetes-...` | HTTP 422 status, field path, cause message |
| **Apple Pkl** | Static types, UInt, Regex, Duration, DataSize | Nullable `?` for optional; non-null required | `(isBetween(x,y))`, regex matches, custom | Single-class inheritance; object `amends` | Single inheritance: subclass overrides superclass | Class declaration or module `amends` | Ahead-of-time evaluation | Closed classes by default; `open class` | Compiler error with source code snippet |
| **IWE (`schematter`)** | String, list, date, wikilink, text | Required keys, heading depths, token budgets | Pattern regex, list bounds, date formats | Class schemas in `.iwe/schemas/` | Path-glob binding precedence in `config.toml` | `type` in YAML frontmatter | Advisory in LSP; gating for AI agents in MCP | Strict closed frontmatter validation | LSP Diagnostics (squiggly lines, problem list) |
| **Backstage Catalog** | JSON Schema primitives in `spec` | Required fields declared in kind schemas | Standard JSON Schema validation constraints | Fixed envelope (`Entity`); per-kind `spec` | No inheritance; kinds are distinct schemas | `kind` and `apiVersion` in YAML envelope | Advisory on ingest (status); gating on API | Closed `spec` based on kind schema | Error status on entity, catalog log |
| **Home Assistant** | Python primitives, entities, templates, time | `vol.Required()` and `vol.Optional()` | `cv.has_at_least_one_key`, `cv.exclusive` | Procedural schema composition in Python | Procedural: first check or dict update | `platform` or domain key in YAML | Gating at startup; advisory in UI config | Closed by default (`extra=vol.PREVENT_EXTRA`) | Exception with dict path and error string |
| **Serde (Rust)** | Rust native types (struct, enum, Vec, Option) | `Option<T>` is optional; bare `T` required | Field attributes, custom deserialize fns | Struct composition via `#[serde(flatten)]` | Flat deserialization buffers unknown fields | Externally determined by Rust target type | Gating at deserialization | Open by default; closed via `deny_unknown_fields` | `serde::de::Error` with line/col (no AST) |
| **Traces (Ticket 20)** | input, select, number, date, boolean, file | `required = true` on field declaration | min, max, step, format, select values | Multiple inheritance (`extends = [...]`) | First-listed class wins whole field declaration | `class` in note frontmatter (default) | Advisory for humans; gating for MCP agents | Open storage/index; advisory unknown-field check | Span-free violation codes; LSP maps spans |

---

## 4. Challenges to Traces' Ticket 20 Design

This section evaluates the specific decisions in Ticket 20's Answer against
the external precedent surveyed above.

### Decision A: Advisory-Only Validation and Untyped Storage

- **Supporting Evidence:**
  - *W3C SHACL:* SHACL explicitly decouples graph parsing and storage from shape
    validation. The triplestore parses and queries RDF without shapes; SHACL
    runs as an advisory diagnostic layer [V].
  - *Markdown LSPs (`marksman`, `markdown-oxide`):* Human note-taking in
    markdown
    requires high tolerance for half-written frontmatter, draft states, and
    exploratory tags. Hard gating at parse time breaks the user's editor [V].
  - *Backstage Catalog:* Ingests entities permissively from git repos, recording
    validation errors in entity status rather than halting the catalog [V].
- **Contradicting Evidence:**
  - *LDAP:* Hard gating on write ensures the directory never contains corrupt or
    non-conforming entries [V].
  - *IWE Strict Mode:* Automatically blocks AI agent writes via MCP if the
    resulting frontmatter violates the schema [V].
- **Recommendation:** **KEEP.** Ticket 20's decision is completely sound for a
  human-centric markdown vault. However, Traces must pair advisory parsing with
  IWE's model: **hard gating on the write path for AI agents (MCP tools)** to
  prevent automated vault degradation.

---

### Decision B: "First-Listed Class Wins" on Conflicting Declarations

- **Supporting Evidence:**
  - Simple CSS-style cascades and prototype chains where users understand
    left-to-right priority.
- **Contradicting Evidence:**
  - *CUE (Marcel van Lohuizen):* Order-dependent "first wins" destroys algebraic
    commutativity ($[A, B] \neq [B, A]$). It makes automated tooling and
    multi-class refactoring fragile and unpredictable [V].
  - *LDAP (RFC 4512 §2.4):* Avoids "first wins" entirely by defining attribute
    types globally. When an entry has multiple classes, `MUST` and `MAY` sets
    are unioned. If one class specifies `MUST` and another `MAY`, **`MUST`
    wins** (the field is required) [V]. Type contradictions cannot occur.
  - *Tweag Nickel:* Symmetrical merging (`&`). Conflicting declarations with the
    same priority fail immediately with an unmergeable error rather than
    silently
    choosing one side [V].
  - *SHACL:* Shapes are conjunctive. Contradictory constraints cause validation
    failure; there is no positional winner [V].
  - *Python/Pydantic:* Decades of MRO (Method Resolution Order) bugs where
    changing inheritance order silently altered field types and defaults [V].
- **Recommendation:** **CHANGE.** Do not let "first class silently wins" define
  field types or options when classes collide!
  - If Class A defines `due: date` and Class B defines `due: input (text)`,
    silently parsing `due` as a date when `class = ["A", "B"]` but as text
    when `class = ["B", "A"]` is a severe trap for users and agents.
  - **Adopt LDAP/Nickel semantics:**
    1. If options conflict (e.g. Class A says `required = true`, Class B says
       `required = false`), **conjunction wins: `required = true`**.
    2. If field types contradict (e.g. `number` vs `input`), emit a hard
       advisory `schema.class-conflict` error and refuse to silently pick one!

---

### Decision C: Multiple Inheritance (`extends`) and Priority

- **Supporting Evidence:**
  - Familiarity from object-oriented programming (Python, C++).
- **Contradicting Evidence:**
  - *Apple Pkl:* Explicitly rejected multiple class inheritance in favor of
    strict single inheritance (`class B extends A`) to eliminate the diamond
    problem and priority ambiguity [V].
  - *LDAP:* Restricts structural classes to a single inheritance chain. Multiple
    inheritance is permitted only for auxiliary mixins, which cannot define
    conflicting types [V].
  - *GraphQL:* Interfaces cannot override field types; implementing types must
    faithfully match the interface definition.
- **Recommendation:** **CHANGE or STRICTLY RESTRICT.** If `extends` supports
  multiple parents, forbid conflicting field definitions between parents. If two
  parents declare the same field with different types or incompatible
  constraints, the schema itself must fail to load at compile/start time.

---

### Decision D: Pure, Span-Free Check and Host Span Resolution

- **Supporting Evidence:**
  - *`taplo` (TOML LSP):* Evaluates `jsonschema` (completely span-free) to get
    JSON Pointer paths (`instance_path`), then walks the `Rowan` concrete
    syntax tree to resolve exact token spans for LSP diagnostics [V].
  - *`yaml-language-server`:* Validates JSON AST without byte ranges, then
    resolves JSON Pointer paths against YAML parser CST nodes [V].
  - *Rust `garde` & `validator`:* Both crates operate on structured data models
    returning field paths (`&Path`) and static codes, completely decoupled from
    text offsets [V].
- **Contradicting Evidence:**
  - Linters that operate directly on CST nodes (like `Vale` or ESLint) can
    attach
    spans immediately during the check.
- **Recommendation:** **KEEP.** This is the cleanest, most maintainable
  architectural seam in the entire ticket. It allows the core validation logic
  to remain pure, fast, and reusable across headless CLI, background indexing,
  and LSP servers.

---

### Decision E: Advisory Unknown Fields with Closest Match Suggestions

- **Supporting Evidence:**
  - *Kubernetes CRD Pruning Disaster:* Proves that silent pruning of unknown
    fields causes catastrophic data loss and user fury [V].
  - *SHACL:* Open-world default prevents fragile schema breakage [V].
  - *LDAP `extensibleObject`:* Proves that real-world directory systems always
    require an open-world escape hatch [V].
- **Contradicting Evidence:**
  - Strict config systems (CUE closed structs `#Def`, Serde
    `deny_unknown_fields`)
    treat unknown fields as fatal errors to catch typos immediately [V].
- **Recommendation:** **KEEP.** In a markdown vault, open storage with advisory
  `unknown-field` diagnostics (and Levenshtein `closest` matches via `strsim`)
  gives users the safety of typo detection without the danger of data loss.

---

### Decision F: Tiered Gating (Advisory for Humans, Gating for Agents)

- **Supporting Evidence:**
  - *IWE (`iwe-org/iwe`):* Implements this exact dual model: real-time advisory
    diagnostics in the editor LSP, but strict write rejection for AI agents
    interacting via MCP [V].
  - *Git & CI hooks:* Human local commits can be permissive or warned, while CI
    and release gates enforce invariants.
- **Contradicting Evidence:**
  - Pure advisory systems (Vale, Marksman) do not gate any writes.
- **Recommendation:** **KEEP.** AI agents lack human contextual judgment; if
  they are not gated at write time via MCP, they will silently hallucinate
  invalid frontmatter fields across hundreds of notes. The delta rule (only
  block newly introduced violations) is essential to prevent agents from getting
  stuck on pre-existing legacy note flaws.

---

### Decision G: Field constraints (`required`, `min`, `max`, `step`, `format`)

- **Supporting Evidence:**
  - *Frictionless Table Schema & JSON Schema:* Formats, ranges, and steps are
    standard baseline validation primitives [V].
- **Contradicting Evidence:**
  - *Google Protocol Buffers (Proto3):* Removed `required` fields because they
    permanently broke backward/forward compatibility as schemas evolved [H].
  - In a PKM vault, marking a field `required` can turn 500 existing notes
    invalid overnight if the schema is updated.
- **Recommendation:** **KEEP, BUT TREAT `required` AS ADVISORY.**
  - `required` must never block reading or indexing historical notes.
  - Scope hard `required` enforcement strictly to template generation (`traces
    new`) and agent write gates for new notes.

---

### Unconsidered Aspects That External Projects Found Essential

1. **Reserved Fields & Tombstoning (Protobuf `reserved`):**
   When a field is removed or renamed in a schema, Traces has no way to prevent
   users or agents from accidentally re-introducing the old field name with a
   different meaning. Protobuf's `reserved "old_field";` is a proven antidote to
   schema drift [H].
2. **Dynamic Value Source Decoupling (Home Assistant lessons):**
   Home Assistant proved that embedding dynamic lookups (like entity IDs or
   external file lists) inside static schema validators breaks serialization and
   caching [V]. Traces must ensure external JSON/link value sources are resolved
   by an independent provider, not baked into the schema validator core.
3. **Diagnostic Severities (SHACL `sh:severity`):**
   Ticket 20 deferred severities to Ticket 31. SHACL proves that distinguishing
   `Violation` (invalid syntax/type), `Warning` (unknown field, missing
   recommended metadata), and `Info` (deprecated field) is vital for actionable
   editor diagnostics [V].

---

## 5. Failure Stories and Regrets Across External Projects

- **Google Protocol Buffers: The `required` Field Compatibility Trap:**
  In Proto2, fields could be declared `required`. Google's engineering post-
  mortems revealed that almost every `required` field eventually needed to be
  made optional as systems evolved. Changing a `required` field to `optional` or
  vice-versa broke binary compatibility between old and new binaries. Kenton
  Varda and the Protobuf team completely eliminated `required` in Proto3.
  *Lesson:* Required fields are a permanent forward-compatibility trap in
  evolving distributed or long-lived data stores [H].
- **Kubernetes CRDs: The Silent Pruning Disaster (Issue #111074):**
  When Kubernetes 1.15 introduced structural schemas and defaulted to pruning
  unknown fields, thousands of users experienced production outages. A typo in a
  manifest (e.g. `seletor` instead of `selector`) was silently deleted by the
  API
  server during `kubectl apply`, creating broken workloads without any error
  message.
  URL: `https://github.com/kubernetes/kubernetes/issues/111074` [V].
- **JSON Schema: `additionalProperties: false` Breaking `allOf` Composition:**
  In Draft-07, `additionalProperties: false` evaluated only the immediate schema
  properties, completely ignoring properties validated in referenced schemas
  inside `allOf`. Developers trying to model inheritance found that extending a
  base schema immediately made all base fields invalid additional properties.
  The JSON Schema committee spent years resolving this, culminating in the
  complex dynamic `unevaluatedProperties` keyword in 2019-09.
  URL: `https://github.com/json-schema-org/json-schema-spec/issues/556` [V].
- **Serde: `deny_unknown_fields` Incompatibility with `#[serde(flatten)]`:**
  Combining `#[serde(flatten)]` and `#[serde(deny_unknown_fields)]` in Rust
  causes subtle runtime deserialization errors (Issue #1547) or silently ignores
  the check (Issue #2283). Because flattening requires buffering unrecognized
  fields to pass to child parsers, Serde cannot determine whether an unknown key
  is genuinely invalid or belongs to another flattened struct.
  URL: `https://github.com/serde-rs/serde/issues/1547` [V].
- **Home Assistant: Procedural Schema Inability to Drive UI Forms:** Home
  Assistant originally relied entirely on procedural Voluptuous Python schemas
  for YAML configuration validation. When the project wanted to introduce
  web-based configuration forms, Voluptuous schemas could not be inspected or
  converted to JSON Schema (`voluptuous-serialize` failed on custom functions).
  Home Assistant was forced to deprecate YAML configuration for hundreds of
  integrations and re-architect the system around declarative "Config Entries".
  URL:
  `https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/config_validation.py`
  [V].
- **Pydantic: Multiple Inheritance Diamond Bugs & MRO Field Shadowing:**
  Users modeling complex domain entities with multiple inheritance in Pydantic
  frequently ran into silent field shadowing and validator bypass bugs (Issue
  #4480, #5055). Python's C3 MRO resolved field definitions in unintuitive
  ways, where adding an innocent mixin class silently replaced a child model's
  validated field with an unvalidated base field.
  URL: `https://github.com/pydantic/pydantic/issues/4480` [V].
- **W3C SHACL: Closed Shape Composition Failure (ISSUE-95 / ISSUE-196):**
  In SHACL, defining a closed shape (`sh:closed true`) on a base class prevents
  any derived shape or mixin from adding new properties to instances of that
  class, unless all child properties are manually re-declared in
  `sh:ignoredProperties`. This rendered modular shape libraries largely unusable
  with closed shapes.
  URL: `https://www.w3.org/2014/shapes-resolutions` [V].

---

## 6. Transferable Lessons for Traces

1. **Decouple Attribute Types from Class Membership (The LDAP Rule):**
   The cleanest solution to multi-class conflicts is to define field types
   consistently across the vault. If `due` is declared as a `date` in one
   schema,
   it should not be declared as a `number` in another. Classes should primarily
   group fields and specify optionality (`required` vs `optional`).
2. **Conjunction Over Positional Priority (The Nickel/LDAP Rule):**
   Never use "first-listed class wins" to silently pick field options. If Class
   A
   says `required = true` and Class B says `required = false`, the effective
   field is **required**. If types contradict, emit an explicit
   `schema.class-conflict` error rather than guessing based on list order.
3. **Preserve Untyped Storage; Validate Later (The SHACL/Taplo Rule):**
   Note frontmatter must be parsed into an untyped, quote-insensitive AST/map
   without requiring schemas. Validation is an independent, pure function
   returning stable violation codes.
4. **Decouple Diagnostic Codes from Source Spans (The Taplo/YAML LSP Rule):**
   The core schema engine should return span-free violation codes and field
   paths (`due`, `author.name`). Let the host application (LSP, CLI) traverse
   the note's concrete syntax tree (CST) to attach byte spans. This keeps the
   schema engine blazing fast and decoupled from parsing text coordinates.
5. **Differentiate Human Advisory from Agent Gating (The IWE Rule):**
   Human writers in Obsidian/Neovim require advisory LSP diagnostics (squiggly
   lines) that never block typing or saving. Automated AI agents operating via
   MCP require **hard gating** (rejection of invalid edits via the delta rule)
   to prevent vault corruption.
6. **Open Storage with Advisory Typos (Kubernetes Anti-Pruning Rule):**
   Never drop or prune unknown fields. Accept all frontmatter keys in the
   storage and index layers, but report advisory `unknown-field` warnings with
   Levenshtein closest-match suggestions in CLI and LSP.
7. **Keep Schemas as Pure Declarative Data (The Home Assistant Regret):**
   Avoid procedural validation hooks. Schemas must remain pure TOML data so they
   can easily be projected into JSON Schema for MCP agent tool parameters,
   template prompts, and editor auto-completion.
