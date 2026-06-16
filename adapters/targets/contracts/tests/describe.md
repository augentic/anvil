---
id: contracts-describe
owner: contracts
kind: adapter
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - files-exist
  - contract-validator-clean
expected-artifacts:
  - contracts/schemas/create-adapter-request.yaml
  - contracts/schemas/adapter.yaml
  - contracts/schemas/update-adapter-request.yaml
  - contracts/schemas/error-response.yaml
  - contracts/http/adapter-api.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
  - implementation-shapes-authored-inline
---

# Generate From Prose Passed To `/spec:refine`

Scenario ID: `contracts-describe`

Use this prompt to test authoring JSON Schema and OpenAPI artifacts from prose
requirements.

Pipeline note:

- In the `contracts` schema, `/spec:refine` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts as
  context. New or changed interface shapes should be introduced through a
  separate `contracts@1.0.0` change before implementation depends on them.

## Intent

Prove that the `contracts@1.0.0` slice loop can author HTTP and JSON Schema
artifacts from a prose requirements block embedded directly in a `/spec:refine`
prompt. The scenario covers the full define → build → merge path for a
greenfield contract change with no pre-existing source document.

## Workspace

- **Adapter:** `contracts@1.0.0`.
- **Project shape:** a single project initialised with the `contracts@1.0.0`
  schema (run `/spec:init` first if the workspace is fresh).
- **Registry shape:** not applicable; this scenario does not exercise
  multi-project registry behavior.
- **Isolation:** `fresh-project`. Start from an empty `contracts/` baseline so
  the expected artifact list is unambiguous.
- **Backend:** `manual` — a human or agent runs the prompts in **Invocation**
  and records results in the [run summary](run-summary-template.md).

## Inputs

This scenario has no source files to seed. The prose requirements live inside
the `/spec:refine` prompt itself; see **Invocation** below.

## Invocation

```text
/spec:refine user-adapter-api

Generate API contracts from prose.

Authorship Mode: Generate from prose
Participants:
- adapter-service: producer
- mobile-app: consumer
- admin-console: consumer

Define a User Adapter HTTP API.

Endpoints:
1. POST /adapters
   - Request body CreateProfileRequest:
     - user_id: string, required
     - display_name: string, required, 1-80 chars
     - timezone: string, optional, IANA timezone
   - 201 response Adapter:
     - id: string
     - user_id: string
     - display_name: string
     - timezone: string|null
     - created_at: string date-time
   - 400 ErrorResponse for invalid fields
   - 409 ErrorResponse when a adapter already exists for user_id

2. GET /adapters/{adapter_id}
   - path parameter adapter_id: string, required
   - 200 response Adapter
   - 404 ErrorResponse when not found

3. PATCH /adapters/{adapter_id}
   - Request body UpdateProfileRequest:
     - display_name: string, optional, 1-80 chars
     - timezone: string|null, optional
   - 200 response Adapter
   - 400 ErrorResponse for invalid fields
   - 404 ErrorResponse when not found

All endpoints use application/json. ErrorResponse has code: string, message:
string, and optional details: object.
```

After `/spec:refine` succeeds, drive `/spec:build user-adapter-api` to produce
the contract YAML, then optionally `/spec:merge user-adapter-api` to promote
the deltas into the baseline.

## Expected Artifacts

During `/spec:build`, the slice should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/schemas/create-adapter-request.yaml`
- `contracts/schemas/adapter.yaml`
- `contracts/schemas/update-adapter-request.yaml`
- `contracts/schemas/error-response.yaml`
- `contracts/http/adapter-api.yaml`

## Assertions

- `files-exist`: every path in **Expected Artifacts** exists in the slice
  working tree after `/spec:build`, and (when merge is run) in the baseline
  `contracts/` tree after `/spec:merge`.
- `contract-validator-clean`: the build's contract verifier (the `contract`
  WASI tool, run as `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`)
  exits `0` with no findings and no manual-review warnings on the produced
  artifacts.

## Negative Expectations

- `artifacts-outside-contracts-directory`: no contract YAML is written outside
  `contracts/http/` or `contracts/schemas/`. The slice must not author
  implementation files (Omnia crates, Vectis Crux modules, etc.).
- `implementation-shapes-authored-inline`: the slice must not pre-author
  Omnia/Vectis interface shapes; only the `contracts@1.0.0` artifacts above are
  produced.

## Cleanup

Drop or archive the slice before moving to the next scenario unless you
explicitly want the new baseline contracts to persist:

- `specify slice drop user-adapter-api` to discard without merging, or
- `/spec:merge user-adapter-api` to merge the baseline contracts; the merge
  skill calls `slice merge run`, which atomically merges, transitions the
  slice to `merged`, and archives it.

Default for a clean run-all sequence: drop. Record the choice in the run
summary's **Cleanup** section.
