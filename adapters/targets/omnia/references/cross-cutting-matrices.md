# Cross-Cutting Matrices and Traceability Verification

The crate-writer skill builds three working matrices (Side-Effect, Outbound Message, Transaction Boundary) before generating handler code. The matrices are not persisted — they live in working memory — but every cell must land in the generated code. After the code is written, the traceability rules below verify that every spec requirement, design.md Business Logic step, and matrix cell has a corresponding code path.

These rules apply in both **Create** mode (across every handler) and **Update** mode (across every handler classified Additive or Modifying, plus any unchanged handler whose cross-cutting behavior depends on a modified entity).

## Matrix a — Side-Effect Matrix

For every handler that performs write operations (HTTP POST/PUT/PATCH/DELETE endpoints, message-triggered handlers that insert or update data), read the design.md Business Logic section and list every entity the handler must read or mutate *beyond its primary entity*. Include cross-handler delegations where one handler invokes or depends on another handler's write path.

| Handler | Primary Entity | Cross-Entity Read | Cross-Entity Mutation | Spec Reference |
|---------|---------------|-------------------|----------------------|----------------|

Every cell in the **Cross-Entity Mutation** column becomes a mandatory code path in the generated handler. If a handler's Business Logic references another entity's data, that reference MUST appear in the generated code — even if the handler could function without it on the "happy path."

## Matrix b — Outbound Message Matrix

For every event or notification published as a side effect in design.md, compare the outbound payload shape against the primary entity's API response shape. If they differ, document the transformation (field additions, removals, renames). Each transformation becomes a dedicated serialization function — never serialize the entity struct directly for outbound messages unless the shapes are confirmed identical.

| Topic | Source Entity | Stripped Fields | Added Fields | Transform Function | Spec Reference |
|-------|-------------|----------------|--------------|-------------------|----------------|

## Matrix c — Transaction Boundary Matrix

For every handler whose Business Logic contains multiple sequential write operations (inserts/updates, or delegated calls to other handlers that write), identify whether the spec requires atomicity (look for REQ references to transactions, "all-or-nothing" language, multi-entity consistency requirements, or post-commit-only side effects).

| Handler | Write Operations | Atomic? | Post-Commit Side Effects | Spec Reference |
|---------|-----------------|---------|--------------------------|----------------|

Every row with **Atomic=Yes** MUST generate transaction-scoped wrapping for its write operations, with event/notification publishes occurring only after successful commit.

## Traceability Verification

Run after the matrices are populated and the code is generated (Create mode step 16; Update mode Step 7).

For each `### Requirement:` block in `specs/<domain>/spec.md`:

- Verify a traceability comment referencing the requirement ID exists in the generated code.
- For each `#### Scenario:` under that requirement, verify that the described behavior has a corresponding branch or code path in a handler.

For each row in the Side-Effect Matrix:

- Verify that every Cross-Entity Mutation has a corresponding function call in the handler.

For each row in the Outbound Message Matrix:

- Verify that the transform function exists and is called before publishing.

For each row in the Transaction Boundary Matrix where **Atomic=Yes**:

- Verify that transaction-scoped wrapping encloses the handler's write operations and that post-commit side effects are outside the transaction.

If any verification fails: implement the missing code path before proceeding. Do not rely on the verify-repair loop or test-writer to catch these — the code must satisfy the spec before handoff. After implementing any missing code paths, re-run `cargo check` to verify the new code compiles.

## Update mode — example plan

The Update mode Step 3 plan is a structured list logged for traceability before any file is modified:

```text
STRUCTURAL (apply first):
  1. Rename OrderEvent → PurchaseEvent
     - src/types.rs: lines 15-30 (struct definition)
     - src/handler.rs: lines 45, 67 (references)

SUBTRACTIVE (apply second):
  2. Remove GET /legacy-status endpoint
     - src/handlers/legacy_status.rs: delete file
     - src/handlers.rs: remove mod + pub use
     - guest src/lib.rs: remove route + import

MODIFYING (apply third):
  3. Add `priority` field to WorksiteRequest
     - src/handlers/worksite.rs: lines 20-28 (struct definition)
     - src/handlers/worksite.rs: lines 45-60 (filter builder)

ADDITIVE (apply last):
  4. Add POST /worksite handler
     - src/handlers/create_worksite.rs: new file
     - src/handlers.rs: add mod + pub use
     - guest src/lib.rs: add route + import
```
