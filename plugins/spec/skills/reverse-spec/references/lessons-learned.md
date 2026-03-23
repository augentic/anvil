# Lessons Learned

Anti-patterns and guardrails derived from real-world reverse-spec attempts. Each lesson describes a mistake that was made, why it happened, and how this skill prevents it.

---

## 1. Hand-Writing Types Instead of Copying

### What Happened
Types were written from memory or paraphrased after reading source code. This introduced subtle errors: wrong numeric types (e.g., `i32` vs `i64`, `int` vs `long`), wrong collection types (e.g., array written as optional scalar), missing serialization default attributes.

### Why It Happened
The analyst read the source types, then wrote the specification from memory rather than copying field-by-field. Over 100 field-level errors accumulated across 30+ structs.

### How We Prevent It
Phase 3 requires: "Copy type definitions verbatim from source. Do NOT hand-write types from memory." The V1 validation dimension then verifies every field, type, serialization attribute, and wire name against the source.

---

## 2. Breadth-First Analysis

### What Happened
All handlers were reviewed superficially in one pass, missing cross-cutting details like shared validation patterns, common header construction, and utility function behavior.

### Why It Happened
The natural tendency is to scan all endpoints quickly to "get the big picture." But this produces a surface-level understanding that misses the details needed for reconstruction-grade specs.

### How We Prevent It
Phase 3 mandates depth-first analysis by functional domain. Each domain is fully analyzed (types → handlers → utilities → cross-references) before moving to the next.

---

## 3. Invented Test Fixtures

### What Happened
Test fixture JSON files were invented based on the spec's type definitions rather than being derived from actual API response examples. When the spec had a type error (e.g., wrong field type), the fixture perpetuated it, making the test pass against wrong expectations.

### How We Prevent It
The guardrails section states: "Never generate test fixtures without verifying them against source response shapes." During validation, V1 checks that fixture field types match the source type definitions.

---

## 4. Assuming Config Key Names

### What Happened
Config keys were renamed for clarity (e.g., `CC_STATIC_URL` → `GTFS_STATIC_URL`). This caused runtime failures because the application tried to read keys that didn't exist.

### How We Prevent It
Phase 3 rule: "Config keys MUST be captured verbatim from source (never renamed)." V4 cross-reference checks verify every config key in the design against the source.

---

## 5. Inferring External Service Architecture

### What Happened
Two different external services (Cubic NextWave CRM API and Microsoft Dynamics 365 CRM) were conflated into one because both had "CRM" in their names. This led to incorrect base URL usage, wrong authentication methods, and incorrect header patterns.

### How We Prevent It
Phase 2 explicitly asks the user: "What are the external systems, their names, and their roles?" Phase 1 also captures every external API call with its exact URL pattern and auth method, making it harder to conflate distinct services.

---

## 6. Assuming Validation Logic

### What Happened
Validation was assumed to be `is_none()` checks on optional fields, but the source code actually used `is_none_or(String::is_empty)` to also reject empty strings. The spec missed these stronger checks.

### How We Prevent It
Phase 3 handler analysis requires: "Guard conditions match exactly (e.g., `is_none()` vs `is_none_or(String::is_empty)`)." V2 validation dimension explicitly checks every validation condition.

---

## 7. Wrong Error Handling Granularity

### What Happened
A refund handler that processed multiple trips was specified to abort on any error. The source code actually used per-trip error handling — recording failures for individual trips while continuing to process others.

### How We Prevent It
V2 handler logic checks include: "Loop structures match (per-item error handling vs abort-on-first-error)." This forces explicit comparison of error handling granularity.

---

## 8. No Automated Structural Diff

### What Happened
Comparison between spec and source was done by reading both manually and looking for differences. This is error-prone for files with 500+ lines and 50+ fields.

### How We Prevent It
Phase 1 builds a structural inventory that can be systematically compared. The V1-V5 validation dimensions provide a structured framework for comparison rather than relying on ad-hoc reading.

---

## 9. Skipping Serde Attribute Details

### What Happened
Serialization attributes were treated as secondary details and often omitted or simplified. For example:
- A deserialize-only rename was written as a bidirectional rename (which also affects serialization output)
- Default-when-absent attributes on fields were missed, causing deserialization failures
- Conditional serialization rules (e.g., "skip if null/empty") were assumed but not verified

### Why It Happened
Serialization attributes look like boilerplate and are easy to skim over. But in a serialization-heavy API layer, they ARE the behavior.

### How We Prevent It
Phase 3 type extraction explicitly requires capturing "every serialization attribute." V1 validation checks each attribute individually.

---

## 10. Not Asking When Uncertain

### What Happened
When the purpose of a config key, the meaning of an enum variant, or the relationship between two types was unclear, the analyst guessed rather than asking. These guesses compounded into architectural misunderstandings.

### How We Prevent It
The cardinal rule: "NEVER assume, infer, or hallucinate. If anything is unclear, ASK the user." Phase 1 and Phase 2 include explicit clarification checkpoints. The guardrails state: "The cost of asking is low. The cost of a wrong assumption is an incorrect specification."

---

## 11. Single-Pass Review

### What Happened
The specification was reviewed once and declared complete. Subsequent user-initiated reviews found new issues each time, eroding confidence.

### How We Prevent It
Phase 5 mandates iterative validation passes with convergence criteria. The loop continues until zero CRITICAL/HIGH discrepancies, with a hard limit at 5 passes to prevent infinite loops.

---

## 12. Missing Path Traversal and Security Checks

### What Happened
Security validation present in the source code (path traversal rejection, input sanitization) was not captured in the specification because it was considered "infrastructure" rather than "business logic."

### How We Prevent It
Phase 3 handler analysis includes validation logic as a first-class concern. V2 checks specifically verify that every validation check in the source exists in the spec, regardless of whether it's business or security logic.

---

## 13. Dependency Version Drift

### What Happened
Dependencies were captured by name only (e.g., `rsa`, `axios`, `Newtonsoft.Json`) without recording the exact version the source was built against. When the build phase resolved "latest" versions, breaking API changes caused compilation failures — types were moved, methods were renamed, re-exports were removed.

In one case, a cryptographic library's v0.10 release moved a core type (`BigUint`) out of the root module, breaking every import. The specification was correct, the handler logic was correct, but the generated code failed to compile because it used API patterns from v0.9 against a v0.10 dependency.

### Why It Happened
Dependency versions feel like infrastructure details, not business logic. The natural instinct is to record package names and let the build tool resolve "latest." But external package APIs ARE part of the codebase's contract — they determine which import paths, type names, method signatures, and trait implementations are valid.

### How We Prevent It
Phase 1.1b mandates reading the project's **lock file** (not just the manifest) and recording exact pinned versions for every dependency. The design.md Dependencies section must include version pins. The validation procedure's V6 dimension verifies that documented API usage patterns are compatible with the documented versions.

This applies to every ecosystem:
- A Rust crate renaming a re-export between minor versions
- An npm package changing its default export between majors
- A NuGet package deprecating a namespace
- A Python package moving a class between submodules

---

## 14. Missing Generated Trait/Interface Implementations

### What Happened
Enums were documented with their variants and serialization behavior but not their full set of generated implementations. All enums implemented equality comparison (Rust: `PartialEq`/`Eq`, equivalent in other stacks: C# `IEquatable`, Python `__eq__`) which handlers used for `==` comparisons. The generated code failed to compile because the equality implementation was missing.

### Why It Happened
The focus was on serialization attributes, not equality or other non-serialization behaviors. Generated implementations like equality, cloning, and debug formatting were treated as boilerplate.

### How We Prevent It
Phase 3 type extraction must capture ALL generated implementations on every type, not just serialization-related ones. V1 validation explicitly checks these.

---

## 15. Cross-Struct Column Merging

### What Happened
A cross-struct type comparison table used merged column headers like `Trip / CustomerTrip` implying both structs shared the listed fields. In reality, `CustomerTrip` was a stripped-down type that did NOT contain 4 of the 5 listed fields. A code generator following this table would emit non-existent fields.

### Why It Happened
The two types were treated as variations of the same concept. But `CustomerTrip` is intentionally minimal — it exists specifically to EXCLUDE fields from the customer-facing response.

### How We Prevent It
Cross-struct tables must use SEPARATE columns for each struct, with `—` for absent fields. Never merge columns for types that differ.

---

## 16. Wire Name Divergence Within Same Codebase

### What Happened
14 request types used `transitaccount_id` (wire name: `transitaccountId`), but one type used `transit_account_id` (wire name: `transitAccountId` — capital A). The spec did not flag this difference. When the generated code used the "standard" field name for all types, one endpoint had the wrong JSON key.

### Why It Happened
The naming inconsistency was in the original source and was likely unintentional. But serialization naming conventions (e.g., camelCase conversion) produce different wire names for different field spellings, and consumers depend on the exact wire name.

### How We Prevent It
Phase 3 must check the ACTUAL serialization wire name for every field by applying the project's naming convention rules. When field names diverge from the convention, flag them in the design's notable exceptions.

---

## 17. Assuming Collection Field Defaults

### What Happened
The design stated "collection/array fields default to empty when absent" as a general rule. But 5 response collection fields had NO default-when-absent behavior, meaning they would fail deserialization if the upstream API omitted the field. A code generator following the "all collections get defaults" rule changed the deserialization semantics.

### Why It Happened
The pattern was identified from the majority of collection fields and assumed universal. But response types that always expect the field to be present intentionally require it.

### How We Prevent It
Never state patterns as universal rules. Document which specific fields follow the pattern and which are exceptions. V1 validation must check EVERY collection field's serialization attributes individually.

---

## 18. Undocumented Guest/Entry-Point Behaviors

### What Happened
The specification focused on the domain crate but missed load-bearing behaviors in the guest/entry-point layer:
- CORS middleware (`CorsLayer::permissive()`)
- Error code → HTTP status mapping (`nextwave_unauthorized` → 401, `crm_account_not_found` → 404)
- Body injection patterns (injecting `grantToken`/`azureAdToken` from Authorization header)
- Dual-stage JWT validation (guest fast-fail + domain full verification)
- `owner` parameter sourcing (static `"at"` vs `extract_source_ip`)

These are all functional behaviors visible to consumers that were completely absent from the spec.

### Why It Happened
The skill's Phase 1 scan focused on the domain crate. The guest was treated as "just wiring" and not analyzed for behavioral details.

### How We Prevent It
Phase 1 scan must include the guest/entry-point layer alongside the domain crate. Specifically check for: middleware, error mapping, body transformation/injection, parameter sourcing, and any validation performed before the domain handler.

---

## 19. Response Check Function Behavioral Differences

### What Happened
Two related functions (`check_ok` and `check_ok_crm_account`) were documented by name but their behavioral differences were not fully captured. `check_ok` had special 401 handling with a distinctive error code; `check_ok_crm_account` did NOT. The spec said `check_ok_crm_account` "falls through to check_ok behaviour" which was wrong — it had different error message prefixes ("CRM" vs "NextWave") and missing 401 handling.

### Why It Happened
The functions were assumed to be wrappers of each other rather than independent implementations with distinct behavior.

### How We Prevent It
When multiple related utility functions exist, document each one's behavior independently. Never say "behaves like X" without verifying every code path. V2 validation should compare each function's error paths side-by-side.

---

## 20. Dependency Version Source Confusion

### What Happened
The design listed dependency versions from the lock file (resolved versions like `1.0.102`) instead of the manifest (declared specifiers like `"1.0.100"`). This overstated the minimum version requirement. A code generator using `1.0.102` as the minimum would unnecessarily exclude compatible older versions.

### Why It Happened
The skill said "capture from lock file" which is correct for understanding what version was tested, but the manifest specifier (Cargo.toml, package.json, pyproject.toml, .csproj) is what should appear in the generated project's dependency declaration.

### How We Prevent It
The design should list BOTH: the manifest version specifier (for the generated project's dependency declaration) and the lock file version (for understanding API compatibility). Use the manifest specifier as the primary.

---

## 21. Keyword-Collision Field Renames

### What Happened
Five fields in the source used the word `type` as their wire name, but since `type` is a reserved keyword in the implementation language, the source used renamed identifiers (`balance_type`, `type_`, `phone_type`) with explicit field-level rename attributes mapping them back to `"type"`. The specification missed these field-level renames because it only documented struct-level naming conventions (like `camelCase`). Generated code serialized `balanceType` or `type_` instead of `"type"`, breaking wire compatibility.

### Why It Happened
The scan focused on struct-level serialization attributes (`rename_all = "camelCase"`) and assumed individual fields would follow the same convention. Fields that override the convention at the field level were not checked.

### How We Prevent It
Phase 3 type extraction must capture EVERY field-level rename/alias attribute, not just struct-level ones. Common patterns to watch for: keyword collisions (`type`, `class`, `import`, `return`), legacy naming compatibility, and multi-format deserialization aliases.

---

## 22. Missing Deserialization Aliases

### What Happened
A field accepted two different wire names for the same value (e.g., both `maskedPan` from naming convention and `maskedPAN` from upstream API). The specification documented only the convention-derived name. Generated code could not deserialize responses using the alternative name.

### Why It Happened
Aliases are a secondary deserialization path — the primary name works in tests. The issue only surfaces with real upstream data that uses the alternative casing.

### How We Prevent It
Phase 3 must capture alias/alternative-name attributes on every field. V1 validation should check for alias attributes in addition to rename attributes.

---

## 23. Unconditional Serialization Skip

### What Happened
A response type deserialized an internal envelope field from the upstream API but used an unconditional skip-serialization attribute to strip it from the outbound response. The specification did not document this, so generated code leaked internal envelope data to API consumers.

### Why It Happened
Skip-serialization attributes were conflated with conditional ones (`skip_serializing_if`). An unconditional skip means the field is never serialized — a load-bearing behavioral difference.

### How We Prevent It
Phase 3 must distinguish between conditional skip (`skip_serializing_if`) and unconditional skip (`skip_serializing` / `JsonIgnore`). Both must be documented explicitly.

---

## 24. Undocumented Wire Format Structural Differences

### What Happened
Two correction endpoints sent line items to the same upstream API path, but one used a flat array structure and the other wrapped each item in a container object. The specification listed both body types but did not explicitly call out the structural difference. A code generator used the wrong body structure for one endpoint, causing the upstream API to reject requests.

### Why It Happened
Both correction body types had similar field names and were used for the same API. The wrapper type was seen as "just a Rust struct" rather than a wire-format-changing container.

### How We Prevent It
When multiple request body types target the same upstream API, the design must explicitly document the wire format JSON shape for each, including any wrapper/container differences. V2 validation should compare the actual JSON structure, not just the type name.

---

## 25. Undocumented Construction Details in Orchestration Handlers

### What Happened
A multi-step orchestration handler (journey refund) had several construction details that differed from a simpler handler of the same category:
- A unique `client_ref_id` format string: `"journey-{id}-trip-{id}"`
- Full voids set `adjustment_amount = None`, not `Some(total_fare)`
- Body fields (`is_approved`, `manual_review_reason`) were documented for the simple handler but not for the orchestration handler

These details were all present in the source but absent from the specification. Generated code produced incorrect request bodies.

### Why It Happened
The orchestration handler was treated as "calling the same correction API" and assumed to share all construction details with the simple handler. Each handler was not verified independently.

### How We Prevent It
Every handler must be documented independently, even if it calls the same upstream API as another handler. Phase 3's depth-first analysis must capture the exact request body construction for each handler — field values, format strings, conditional logic.

---

## 26. External Service Audit Field Names

### What Happened
An audit write to an external CRM used vendor-specific field names (e.g., `cr884_clientrefid`) that were not standard or guessable. The specification documented the audit flow but not the exact field names. Generated code either omitted the audit or used incorrect field names.

### Why It Happened
The audit write was treated as a secondary concern ("best-effort, non-critical") and its implementation details were not captured at the same level as primary API calls.

### How We Prevent It
All external API calls — including best-effort, secondary, and audit calls — must document exact request bodies with field names. "Best-effort" does not mean "under-specified."

---

## 27. IntoBody / Response Serialization Deduplication

### What Happened
Five handlers returned the same response type as another handler but relied on the other handler's file to provide the serialization implementation. A code generator that emitted serialization impls in every handler file produced duplicate implementations, causing compilation errors.

### Why It Happened
The design listed response types per handler but did not flag which file "owns" the serialization impl. Without this information, a generator cannot know which handler file should contain the impl.

### How We Prevent It
The design must include a table showing which handlers share response types and which file contains the canonical serialization implementation. Code generators should only emit the impl once.

---

## Summary: The Root Causes

Most errors fall into six categories:

1. **Imprecision** — paraphrasing instead of copying (types, config keys, wire names, generated implementations)
2. **Incompleteness** — skipping details that seem minor (serialization attrs, validation conditions, error granularity, guest-level behaviors, field-level renames, aliases, unconditional skips)
3. **Assumption** — inferring rather than verifying (service architecture, field types, validation logic, function equivalence, shared construction details)
4. **Version drift** — capturing dependency names without versions, then building against incompatible API surfaces
5. **Scope blindness** — analyzing domain code but missing load-bearing behavior in the guest/entry-point layer (CORS, error mapping, body injection, dual-stage validation)
6. **Deduplication blindness** — not tracking which module "owns" shared implementations (serialization, response types), causing duplicates or missing impls in generated code

The skill's structure directly addresses each:

- **Imprecision** → Copy verbatim, validate field-by-field (Phase 3, V1)
- **Incompleteness** → Depth-first analysis, structured validation dimensions (Phase 3, V1-V5); capture ALL field-level attributes (renames, aliases, unconditional skips)
- **Assumption** → Clarification checkpoints, ask-don't-guess guardrails (Phase 1-2, guardrails); verify each handler independently
- **Version drift** → Lock file version capture, dependency version validation (Phase 1.1b, V6)
- **Scope blindness** → Phase 1 scan includes guest layer; V3 validates entry-point behaviors
- **Deduplication blindness** → Track shared response types and serialization impl ownership (Phase 3, design)
