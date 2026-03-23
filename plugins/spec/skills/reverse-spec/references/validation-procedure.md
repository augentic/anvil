# Validation Procedure

Detailed procedure for the iterative validation loop (Phase 5 of reverse-spec).

Each validation pass systematically compares every generated artifact against the source code. The pass is structured into six dimensions, each with specific checks.

---

## V1: Type Fidelity

Compare every type definition in `design.md` against the source code.

### For Each Struct/Class

1. Open the source file containing the type definition
2. For each field in the source:
   - Verify the field exists in the design type table
   - Verify the field type matches exactly (e.g., `i32` vs `i64`, `int` vs `long`, `number` vs `string`, optional vs required)
   - Verify the wire name matches the serialization attribute/decorator
   - Verify optionality matches (nullable vs required, default-when-absent behavior)
   - Verify conditional serialization rules match (skip-if-null, skip-if-empty, etc.)
3. For each field in the design type table:
   - Verify it exists in the source (catch phantom fields)
4. Check type-level attributes:
   - Naming convention (e.g., camelCase, snake_case, PascalCase) — does the design's wire name convention match?
   - Type-level defaults — is default-when-absent behavior documented?

### For Each Enum

1. Verify variant count matches
2. For each variant: verify name, serialized representation, and any associated data
3. Verify the serialization strategy (string-based, integer-based, etc.)

### For Custom Deserializers

1. Verify the exact behavior is documented (input type → output type, edge cases)
2. Verify which fields use the custom deserializer

### Discrepancy Types (V1)

| Check | Severity |
|-------|----------|
| Wrong field type | CRITICAL |
| Missing field | CRITICAL |
| Phantom field (in spec but not source) | HIGH |
| Wrong wire name | HIGH |
| Missing serialization attribute | MEDIUM |
| Missing optionality marker | MEDIUM |
| Undocumented custom deserializer | MEDIUM |

---

## V2: Handler Logic

Compare every handler's documented business logic against the source code.

### For Each Handler

1. Open the source file for the handler
2. Compare `from_input` / input deserialization:
   - Input type matches (`Vec<u8>`, `String`, `Option<String>`, etc.)
   - Deserialization method matches (e.g., JSON body parsing, URL-encoded form parsing, XML parsing)
3. Compare validation logic:
   - Every validation check in source exists in spec
   - Validation order matches (when order matters for short-circuit)
   - Error messages/codes match
   - Guard conditions match exactly (e.g., `is_none()` vs `is_none_or(String::is_empty)`)
4. Compare URL construction:
   - Base URL config key matches
   - Path segments match exactly
   - Query parameters match (names and values)
   - URL encoding applied where source applies it
5. Compare headers:
   - Every header in source is documented
   - Header values (static and dynamic) match
   - Custom headers (e.g., `x-cub-audit`) captured
6. Compare authentication:
   - Auth method matches (API key, Bearer token, none)
   - Identity/token source matches
7. Compare response handling:
   - Response parsing matches (JSON, XML, text)
   - Error response handling matches
   - Status code checks match
8. Compare error paths:
   - Every error return in source has a corresponding scenario
   - Error types/codes match
9. Compare business logic:
   - Transformations match
   - Conditional branches match
   - Loop structures match (per-item error handling vs abort-on-first-error)

### Discrepancy Types (V2)

| Check | Severity |
|-------|----------|
| Wrong URL path | HIGH |
| Missing validation check | HIGH |
| Wrong error handling pattern (abort vs per-item) | HIGH |
| Missing header | MEDIUM |
| Missing query parameter | MEDIUM |
| Validation condition imprecise | MEDIUM |
| Missing error scenario | LOW |

---

## V3: API Contract

Compare the API contracts table in `design.md` against actual route definitions.

### For Each Endpoint

1. Verify HTTP method matches
2. Verify path matches (including parameter syntax)
3. Verify input type matches the handler's `from_input` input type
4. Verify output type matches the handler's return type
5. Verify auth tier matches (public, customer, admin)
6. Verify CORS/middleware configuration if documented

### Discrepancy Types (V3)

| Check | Severity |
|-------|----------|
| Wrong HTTP method | CRITICAL |
| Wrong path | HIGH |
| Wrong auth tier | HIGH |
| Wrong input/output type | MEDIUM |

---

## V4: Cross-Reference Integrity

Verify internal consistency across artifacts.

### Specs ↔ Design

1. Every requirement ID referenced in design.md exists in specs
2. Every handler in design.md has a corresponding requirement in specs
3. Every type used in specs scenarios is defined in design.md domain model

### Specs ↔ Source

1. Every requirement maps to identifiable source code
2. No source behavior exists without a corresponding requirement (completeness)

### Design ↔ Source

1. Every config key in design.md exists in source
2. Every external service in design.md is called in source
3. Every type in design.md domain model exists in source

### ID Stability

1. Requirement IDs are sequential with no gaps
2. No duplicate IDs

### Discrepancy Types (V4)

| Check | Severity |
|-------|----------|
| Orphaned requirement ID | MEDIUM |
| Unspecified source behavior | MEDIUM |
| Missing cross-reference | LOW |
| ID gap or duplicate | LOW |

---

## V5: Completeness

Verify nothing is missing from the specification.

### Source → Spec Coverage

For each source file:

1. Read the file
2. Identify all behaviors (handlers, utilities, type definitions)
3. Verify each behavior is covered by a requirement or design section
4. Flag any uncovered behavior as UNSPECIFIED

### Spec → Source Coverage

For each requirement:

1. Verify the documented behavior exists in source
2. Flag any requirement without source backing as PHANTOM

### Config Coverage

1. Search source for all config/env var access patterns
2. Verify each is documented in design.md Constants section

### Discrepancy Types (V5)

| Check | Severity |
|-------|----------|
| UNSPECIFIED behavior in source | MEDIUM |
| PHANTOM requirement (no source) | HIGH |
| Undocumented config variable | MEDIUM |

---

## V6: Dependency Version Compatibility

Verify that documented dependency versions are captured and that API usage patterns are compatible.

### Version Capture

1. Verify the source project's lock file was read (not just the manifest)
2. For each dependency in design.md:
   - Verify it has an exact pinned version (e.g., `1.4.0`, not `^1.4` or `*`)
   - Verify the version matches the lock file entry
3. If no lock file exists, verify this is flagged in Risks / Open Questions

### API Surface Compatibility

For each dependency that the source code imports types, traits, functions, or modules from:

1. Verify the import paths documented in the design are valid for the pinned version
2. Flag any import that relies on re-exports (re-exports are the first thing libraries remove in new versions)
3. Flag any usage of deprecated APIs that may be removed in newer versions
4. Verify feature flags / optional features match between source manifest and design

### Discrepancy Types (V6)

| Check | Severity |
|-------|----------|
| Missing dependency version (no pin) | HIGH |
| Version mismatch between design and lock file | HIGH |
| Import path relies on unstable re-export | MEDIUM |
| Missing feature flag | MEDIUM |
| No lock file (version constraints only) | LOW |

---

## Running the Loop

### Pass Execution

1. Run all six dimensions (V1-V6) in sequence
2. Collect all discrepancies with severity, location, and description
3. Sort by severity: CRITICAL → HIGH → MEDIUM → LOW

### Reporting

After each pass, present a summary:

```
## Validation Pass N

| # | Severity | Dimension | Location | Description |
|---|----------|-----------|----------|-------------|
| 1 | CRITICAL | V1-Type | Token.is_active | Field type is `bool` in source, `String` in spec |
| 2 | HIGH | V2-Handler | account_lookup | Missing empty-string validation for encrypted_token |
| 3 | MEDIUM | V1-Type | Ride.fare | Missing default-when-absent attribute |

### Summary
- CRITICAL: 1
- HIGH: 1
- MEDIUM: 1
- LOW: 0
- Total: 3
```

### Resolution

For each discrepancy:

- **Can be resolved from source alone**: Fix the artifact immediately
- **Needs user clarification**: Batch the question and ask
- **Ambiguous in source**: Mark as `[unknown]` and ask the user

### Convergence Criteria

The loop converges (exits) when a pass returns zero CRITICAL and HIGH discrepancies. MEDIUM and LOW discrepancies are reported but do not block convergence.

### Pass Limit

If after 5 passes there are still CRITICAL or HIGH discrepancies, HALT and present the remaining issues to the user with a recommendation:

> "After 5 validation passes, N discrepancies remain. These may indicate areas where the source code is ambiguous or where I need additional context. Please review the remaining issues."

---

## Anti-Pattern: Shallow Validation

A validation pass that only checks whether sections exist is insufficient. The pass must compare actual values:

**Wrong** (shallow):
> "design.md has a Domain Model section ✓"

**Right** (deep):
> "Domain Model → Token struct: field `is_active` is `bool` in source, `String` in spec → CRITICAL"

Every validation check must compare a **specific value** in the artifact against a **specific value** in the source code.
