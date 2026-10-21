# Specification Claims & Primary Source Verification

- **Target File:**
  `.scratch/arch-design/research/20-prep/round5/R2-spec-claims.md`
- **Scope:** Primary source verification of six external precedent and
  specification claims cited in ticket 20's Answer and round-4 research.
- **Status:** Complete.

---

## 1. LDAP Schema: RFC 4512 MUST/MAY Semantics

### 1.1 Provenance

- **Round-4 Citation:**
  `.scratch/arch-design/research/20-prep/round4/D-schema-languages-and-linters.md:13-14,26-29,131-186`
  [V: file read];
  `.scratch/arch-design/research/20-prep/round4/report.md:48,84-85`
  (Table 3: "LDAP MUST/MAY: all classes apply, no winner [H]";
  Section 5: "LDAP semantics (MUST/MAY union) ... are [H], standard but unread
  here") [V: file read].

### 1.2 Verdict

**VERIFIED** [V: primary RFC text read].

### 1.3 Evidence

RFC 4512 (June 2006), "Lightweight Directory Access Protocol (LDAP):
Directory Information Models"
(`https://www.rfc-editor.org/rfc/rfc4512.txt`):

1. **Object Class Inheritance & MUST/MAY Sets (§2.4, p. 9):**
   > "Each object class identifies the set of attributes required to be
   > present in entries belonging to the class and the set of attributes
   > allowed to be present in entries belonging to the class. As an entry of
   > a class must meet the requirements of each class it belongs to, it can
   > be said that an object class inherits the sets of allowed and required
   > attributes from its superclasses. A subclass can identify an attribute
   > allowed by its superclass as being required. If an attribute is a
   > member of both sets, it is required to be present." [V: RFC 4512 §2.4]

2. **Object Class Description Syntax (§4.1.1, p. 24):**

   ```text
   ObjectClassDescription = LPAREN WSP
       numericoid                 ; object identifier
       [ SP "NAME" SP qdescrs ]   ; short names (descriptors)
       [ SP "DESC" SP qdstring ]  ; description
       [ SP "OBSOLETE" ]          ; not active
       [ SP "SUP" SP oids ]       ; superior object classes
       [ SP kind ]                ; kind of class
       [ SP "MUST" SP oids ]      ; attribute types
       [ SP "MAY" SP oids ]       ; attribute types
       extensions WSP RPAREN
   ```

   > "MUST and MAY specify the sets of required and allowed attribute types,
   > respectively" [V: RFC 4512 §4.1.1]

3. **Multi-Class Entry Content Governance (§4.1.6, pp. 28-29):**
   *(Note: Section 4.1.4 is "Matching Rule Uses"; content governance is
   defined in §4.1.6 "DIT Content Rules" and §2.4).*
   > "An entry must contain all attributes required by the object classes
   > the entry belongs to as well as all attributes required by the
   > governing content rule. An entry may contain any non-precluded
   > attributes allowed by the object classes the entry belongs to as well
   > as all attributes allowed by the governing content rule."
   > [V: RFC 4512 §4.1.6]

**Semantic Rules Verified:**

- **Effective MUST:** The union of all `MUST` attribute sets across all
  assigned object classes (structural and auxiliary) plus their superclass
  chains (`SUP`).
- **Effective MAY:** The union of all `MAY` attribute sets across all
  assigned object classes, minus attributes precluded by DIT content rules or
  elevated to `MUST`.
- **MUST vs. MAY Collision:** If an attribute is `MUST` in one class and `MAY`
  in another, **MUST dominates**. It is required to be present. An optional
  declaration never overrides or relaxes a mandatory requirement.

### 1.4 Relevance to Ticket 20's Answer

Directly supports ticket 20's Answer (lines 105-106): unrelated classes
merge constraints additively where `required` and `multi` arity are checked if
either class declares them.

---

## 2. CUE: Unification Algebra & Bottom on Conflicts

### 2.1 Provenance

- **Round-4 Citation:**
  `.scratch/arch-design/research/20-prep/round4/D-schema-languages-and-linters.md:66-70,287-309`
  [V: file read];
  `.scratch/arch-design/research/20-prep/round4/report.md:48,84-85`
  (Table 3: "CUE: unification is commutative, conflicts fail [H]";
  Section 5: "CUE commutativity are [H], standard but unread here")
  [V: file read].

### 2.2 Verdict

**VERIFIED** [V: primary specification read].

### 2.3 Evidence

The CUE Language Specification (`https://cuelang.org/docs/reference/spec/`,
authored by Marcel van Lohuizen):

1. **Unification Properties (§Unification):**
   > "The unification of values `a` and `b` is defined as the greatest lower
   > bound of `a` and `b`... Since CUE values form a lattice, the unification
   > of two CUE values is always unique." [V: CUE Spec §Unification]
   >
   > "Unification in CUE is a binary expression, written `a & b`. It is
   > commutative, associative, and idempotent. As a consequence, order of
   > evaluation is irrelevant, a property that is key to many of the
   > constructs in the CUE language as well as the tooling layered on top of
   > it." [V: CUE Spec §Unification]

2. **Atoms and Bottom (§Values, §Bottom and errors):**
   > "An atom is any value whose only instances are itself and bottom.
   > Examples of atoms are `42.0`, `\"hello\"`, `true`, and `null`."
   > [V: CUE Spec §Values]
   >
   > "Any evaluation error in CUE results in a bottom value, represented by
   > the token `_|_`. Bottom is an instance of every other value. Any
   > evaluation error is represented as bottom."
   > [V: CUE Spec §Bottom and errors]

3. **Struct Unification & Concrete Conflicts (§Structs):**
   > "The successful unification of structs `a` and `b` is a new struct `c`
   > which has all fields of both `a` and `b`, where the value of a field
   > `f` in `c` is `a.f & b.f` if `f` is defined in both `a` and `b`, or
   > just `a.f` or `b.f` if `f` is in just `a` or `b`, respectively...
   > The result of a unification is bottom (`_|_`) if any of its defined
   > fields evaluates to bottom, recursively." [V: CUE Spec §Structs]

When two distinct concrete values (such as `port: 80` and `port: 8080`) are
unified, their greatest lower bound in the lattice is bottom (`_|_`). Neither
value overrides or masks the other; evaluation halts with an error.

### 2.4 Relevance to Ticket 20's Answer

Directly justifies ticket 20's rejection of "first-listed wins" for
unrelated classes (lines 102-108, 234-237), adopting CUE's order-independent
algebra where conflicting declarations yield `schema.class-conflict`.

---

## 3. Kubernetes Structural Schemas: Pruning & Issue #111074

### 3.1 Provenance

- **Round-4 Citation:**
  `.scratch/arch-design/research/20-prep/round4/D-schema-languages-and-linters.md:82-87,398-403,717-718,797-804`
  [V: file read];
  `.scratch/arch-design/research/20-prep/round4/report.md:53`
  (Table 3: "Kubernetes silent pruning incident [S]") [V: file read].

### 3.2 Verdict

**VERIFIED** [V: official docs and GitHub issue tracker read].

### 3.3 Evidence

1. **Official Kubernetes CRD Documentation:**
   Kubernetes Docs, "Extend the Kubernetes API with CustomResourceDefinitions",
   Section "Field pruning"
   (`https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#field-pruning`):
   > "if you specify a field that the API server does not recognize, the
   > unknown field is pruned (removed) before being persisted...
   > For compatibility with apiextensions.k8s.io/v1, update your custom
   > resource definitions to: 1. Use a structural OpenAPI schema. 2. Set
   > spec.preserveUnknownFields to false." [V: K8s Docs §Field pruning]
   >
   > "By default, all unspecified fields for a custom resource, across all
   > versions, are pruned. It is possible though to opt-out of that for
   > specific sub-trees of fields by adding
   > `x-kubernetes-preserve-unknown-fields: true` in the structural OpenAPI
   > v3 validation schema." [V: K8s Docs §Controlling pruning]

2. **The "Pruning Incident" / Backlash (`kubernetes/kubernetes#111074`):**
   GitHub Issue #111074, "Add ability to opt-out of pruning and perform
   server side validation", opened 2022-07-11 by `@dprotaso` (Knative lead):
   > "It was surprising to see that when a schema is fully defined pruning is
   > the default behaviour. I tried to read through the KEPs to determine why
   > unknown fields wouldn't cause the API server to return an error and
   > didn't really find an answer... If I mistype `replicas` for `replica`
   > there doesn't seem to be a way to have server side validation return an
   > error saying 'replica' is an unknown field."
   > [V: kubernetes/kubernetes#111074:12-53]
   >
   > Commenters noted severe production outages (e.g., database parameters
   > and replica specs silently erased by the API server when automated
   > deployment pipelines bypassed client-side `kubectl` validation). This
   > forced the development of KEP-2885 ("Server-Side Field Validation",
   > graduated in Kubernetes 1.25+) to support `--validate=strict` and
   > `--validate=warn`. [V: kubernetes/kubernetes#111074:104-128,249-282]

### 3.4 Relevance to Ticket 20's Answer

Confirms that Traces must never silently prune unknown frontmatter fields.
Ticket 20's Answer adheres to this by keeping notes open-world and returning
unknown fields as advisory diagnostics with closest-match hints (lines 87-88).

---

## 4. Plone: Dexterity Behaviors vs. Archetypes on Conflicts

### 4.1 Provenance

- **Round-4 Citation:**
  `.scratch/arch-design/research/20-prep/round4/E-wiki-and-graph-systems.md:40-41,304-320,488-492,597-600,683-689`
  [V: file read];
  `.scratch/arch-design/research/20-prep/round4/report.md:49,82`
  (Table 3: "Plone moved from class inheritance to behaviours (mixins) [H]")
  [V: file read].

### 4.2 Verdict

**VERIFIED** [V: official Plone docs and repository code read].

### 4.3 Evidence

1. **Framework Transition:**
   Plone replaced Archetypes (which relied on deep subclassing of
   `BaseContent` and schema mutation) with Dexterity
   (`plone.dexterity` and `plone.behavior`) due to fragile base classes and
   rigid inheritance hierarchies. [V: Plone 5.2 Dexterity Manual]

2. **Behavior Field Clashes and Storage Mechanisms:**
   Plone Documentation v5.2, "Schema-only behaviors using annotations or
   attributes"
   (`https://5.docs.plone.org/external/plone.app.dexterity/docs/behaviors/schema-only-behaviors.html`):
   - **Direct Attribute Storage (Marker behaviors without factory):**
     Stores fields directly on the content instance (`context.field_name`).
     The documentation explicitly warns of collisions:
     > "The attribute name may collide with another attribute on the object,
     > either from its class, its base schema, or another behavior. Whether
     > this is a problem in practice depends largely on whether the name is
     > likely to be unique." [V: Plone 5.2 Schema-only behaviors]
     When two behaviors share an attribute name, writes to one silently
     clobber the other.
   - **`AnnotationStorage` (`plone.behavior.AnnotationStorage`):**
     Specifically engineered to prevent field collisions. Field values are
     stored in the object's `__annotations__` BTree keyed by the behavior's
     unique dotted interface name (`schema.__identifier__`). This isolates
     fields into separate namespaces. [V: `plone/behavior/annotation.py`]
   - **Form Generation Clashes (`plone.autoform`):**
     When generating forms from mixed schemata, duplicate field names without
     namespace prefixes caused ambiguous schema collisions and broken field
     ordering (`plone.autoform` changelog 1.7.0, issue #18).

### 4.4 Relevance to Ticket 20's Answer

Because Markdown frontmatter is a flat key-value dictionary without
per-behavior namespace wrappers (unlike Plone's `AnnotationStorage`), ticket
20's Answer correctly flags duplicate differing declarations across classes
as a `schema.class-conflict` (lines 104-106).

---

## 5. Letta (MemGPT) & Graphiti: Claimed Delta/Diff Validation

### 5.1 Provenance

- **Round-4 Citation:**
  `.scratch/arch-design/research/20-prep/round4/F-agent-and-mcp-vault-tools.md:623-642`
  (Section 4.2: `"The delta rule blocks only newly introduced violations"`
  citing Graphiti backward compatibility and Obsidian Local REST API)
  [V: file read];
  `.scratch/arch-design/research/20-prep/round4/report.md:54`
  (Table 3: "Delta rule | Letta/Graphiti per scout F [S]; ruff/mypy
  baselines [H]") [V: file read].

### 5.2 Verdict

**REFUTED as precedent for delta/diff validation; CLAIM CONFLATED**
[V: upstream docs and repositories read].

### 5.3 Evidence

1. **Graphiti / Zep (`getzep/graphiti`):**
   Upstream documentation
   (`https://help.getzep.com/graphiti/core-concepts/custom-entity-and-edge-types.md`)
   demonstrates that Graphiti uses Pydantic models for custom entity and edge
   types. Validation occurs strictly on ingestion (`add_episode`) for newly
   extracted entities.
   Scout F cited Graphiti's schema evolution (where historical entities are not
   invalidated when new schema rules are added) as evidence for a "delta
   rule". However, this is simply standard open-world graph storage with
   optional Pydantic fields (`Optional[T] = Field(None)`), **not** a document
   delta validator comparing before/after states.

2. **Letta / MemGPT (`letta-ai/letta`):**
   Upstream documentation (`https://docs.letta.com`) shows that Letta manages
   agent memory via block tools (`core_memory_append`, `core_memory_replace`)
   or MemFS (markdown files committed to Git). Background audits ("Dreaming"
   and "Memory Doctor") run asynchronously to compact and organize memories.
   Letta has **no mechanism** that inspects document diffs or filters
   schema validation errors by edit deltas.

3. **Provenance Analysis:**
   Scout F conflated:
   (a) Graphiti's non-invalidation of historical graph nodes, and
   (b) Letta's asynchronous background auditing / Git commits,
   with
   (c) a document delta validation rule.
   The true precedents for delta-gating are code linters and static analyzers
   with baseline mechanisms (e.g., `ruff --diff`, `mypy --baseline`), not
   agent memory frameworks.

### 5.4 Relevance to Ticket 20's Answer

Does not alter ticket 20's Answer. Ticket 20 defines the delta rule
(`check_note(before, after)` keyed by `(field, code, hash(value))`, lines
186-188) based on PKM operational reality (preventing agents from being
blocked by legacy note syntax errors). Refuting Letta/Graphiti clarifies the
historical ledger.

---

## 6. ADR-0009 and Survey Analysis vs. Ticket 20's Answer

### 6.1 ADR-0009 Verification

- **Primary Source:** `docs/adr/0009-index-refresh.md` [V: file read].
- **ADR-0009 Invariant:** The persisted Redb index freshness signal is the
  union of file metadata (deltas), the config fingerprint of index-relevant
  settings, and store format version.
- **Excluded Settings:** Lines 76-77 state:
  > "Settings that do not affect indexing (root, templates, schema directory)
  > are deliberately excluded, so moving a project does not rebuild."
  Lines 21-25 explicitly identify that only the configured class-field name
  (`schemas.class_field`), frontmatter field mappings, and task status sets
  enter the config fingerprint. Schema definitions themselves are never
  indexed.
- **Verification vs. Ticket 20 Answer:**
  Ticket 20's Answer states:
  > "Decision: schemas validate note data as an advisory, on-demand check.
  > They never type values at parse time, never gate the read or index path,
  > and never enter the index or its fingerprint." (lines 51-53)
  >
  > "(e) schema definitions never affect parsing, the index or the
  > fingerprint (only the configured class-field name is shared)." (lines
  > 63-64)
  >
  > "Expansion happens at query time into the existing Class, Tag and Path
  > atoms plus a 'has a known explicit class' exclusion, so the index and its
  > fingerprint never learn about bindings." (lines 141-144)
  **Verdict:** **VERIFIED**. Ticket 20's Answer strictly adheres to ADR-0009.

### 6.2 Claims in `.scratch/arch-design/survey/schema.md` Contradicting the Answer

Reading `.scratch/arch-design/survey/schema.md` in full reveals five claims
that contradict ticket 20's Answer:

1. **`FileClassExpander` Seam:**
   - *Survey Claim (§8, lines 145-146):* Praised as a "model deep interface"
     that cleanly decouples `schema` and `query` and "withstands scrutiny".
   - *Answer Decision (lines 151-164, 302-303):* **Overturned**. Marked for
     complete deletion. Identified as a verified architectural smell (single
     implementation, consumer-defined trait, out-parameter mutation, orphan
     rule hazard). Replaced by passing class hierarchy as pure data.

2. **Role of the `global` Schema:**
   - *Survey Claim (§2, line 22; §8, line 147):* Asserts `global` is strictly
     an isolated, non-inheritable `$ref` pool that never checks note data.
   - *Answer Decision (lines 121-132, 322-323):* **Expanded**. `global`
     remains a `$ref` pool but additionally becomes the lowest-precedence
     advisory validation layer for *every* note (classed or not), producing the
     `schema.global.*` diagnostic family.

3. **Absence of Note Data Validation:**
   - *Survey Claim (§2, line 30; §4 [H1], lines 49-55):* Emphasizes that note
     value validation is 100% absent, presenting "keep schema as a pure passive
     metadata provider" as a viable alternative.
   - *Answer Decision (lines 51-58, 74-120, 217-219):* **Rejected**. Keeping
     schemas passive was explicitly rejected because it leaves constraints
     inert and blocks downstream LSP tooling. Advisory validation via
     `ResolvedFields::check(&FieldMap) -> Vec<Violation>` is formally adopted.

4. **Cross-Class Collision Resolution:**
   - *Survey Claim (§1, line 9):* Asserts field merging across multiple parents
     is strictly "first-listed wins".
   - *Answer Decision (lines 98-109):* **Refined**. First-listed wins is
     retained for parent inheritance within a class, but rejected between
     unrelated classes. Unrelated collisions produce `schema.class-conflict`
     and skip value checks.

5. **Query-Time Behavior on Schema Violations:**
   - *Survey Claim (§7, line 139):* Lists query execution on schema field type
     violations as open "fog" (whether to coerce, treat as null, or error).
   - *Answer Decision (lines 51-53, 228-229):* **Resolved**. Schemas never
     type values at parse or query time; query-bind typing is rejected per
     ticket 19 and ADR-0009. Query execution evaluates raw note values
     independently of schema validity.

### 6.3 Relevance to Ticket 20's Answer

Ticket 20's Answer explicitly acknowledged that it updates the documented
invariants in `src/schema/mod.rs` and supersedes the earlier survey findings
(lines 130-132, 319-333).

---

## 7. Summary Ledger

| # | Topic / Claim | Round-4 Status | Round-5 Verdict | Impact on Ticket 20 Answer |
|---|---|---|---|---|
| 1 | LDAP MUST/MAY union & MUST dominance | [H] unread | **VERIFIED** [V] | Validates additive requirement union |
| 2 | CUE commutative unification & bottom | [H] unread | **VERIFIED** [V] | Validates order-independent class conflict |
| 3 | K8s pruning & Issue #111074 backlash | [S] unread | **VERIFIED** [V] | Confirms open-world advisory diagnostics |
| 4 | Plone Dexterity collision handling | [H] unread | **VERIFIED** [V] | Confirms need for flat field conflict detection |
| 5 | Letta / Graphiti delta validation rule | [S] scout F | **REFUTED** [V] | Refutes precedent; delta rule rests on linters |
| 6a| ADR-0009 schema index exclusion | [V] ticket 20 | **VERIFIED** [V] | Confirms schemas never enter index/fingerprint |
| 6b| Survey schema.md contradictions | [V] survey | **VERIFIED** [V] | 5 claims superseded/overturned by Answer |
