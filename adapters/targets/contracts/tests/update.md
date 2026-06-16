---
id: contracts-update-boundary
owner: contracts
kind: adapter-boundary
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - implementation-schema-emits-no-contract-yaml
  - regression-path-files-exist
  - regression-path-contract-validator-clean
expected-artifacts:
  - contracts/http/loyalty-api.yaml
  - contracts/schemas/loyalty-enrollment-request.yaml
  - contracts/schemas/loyalty-enrollment.yaml
  - contracts/schemas/error-response.yaml
negative-expectations:
  - implementation-design-emits-contract-yaml
  - implementation-slice-merges-contract-deltas-to-baseline
  - implementation-slice-pre-authors-interface-shapes
---

# Generate From An Updated `design.md` Created By `/spec:refine`

Scenario ID: `contracts-update-boundary`

This is a negative or boundary test for the current pipeline. Omnia/Vectis
implementation changes do not include a `contracts` define stage. They consume
baseline contracts as context, while new or changed interface shapes are
introduced through a separate `contracts@1.0.0` change.

## Intent

Prove that the pipeline upholds the contract/implementation boundary: an
implementation slice (Omnia or Vectis) cannot become the source of contract
generation by augmenting its `design.md`. The scenario has two parts:

1. The negative path — an implementation-schema `/spec:refine` regeneration
   over an updated `design.md` must **not** emit contract YAML. This is the
   primary assertion and the **Negative Expectations** section is the
   load-bearing oracle.
2. The regression path — a separate `contracts@1.0.0` slice authored from the
   same endpoint description **does** produce contract artifacts; the
   **Expected Artifacts** section above describes those.

A pass means: (a) the implementation slice produces no contract YAML, and
(b) the regression `contracts@1.0.0` slice produces the expected artifacts and
verifies cleanly.

## Workspace

- **Adapter under test:** `contracts@1.0.0` (boundary).
- **Project shape:** the negative path runs in an implementation-schema
  project (Omnia or Vectis) where `/spec:refine` produces a slice with
  `design.md`. The regression path runs in a project initialised with the
  `contracts@1.0.0` schema. Operators may use one project that supports both
  schemas, or two distinct projects.
- **Registry shape:** not applicable.
- **Isolation:** `fresh-project` for both paths. Start each path from an
  empty `contracts/` baseline so the absence/presence of contract artifacts
  is unambiguous.
- **Backend:** `manual` — a human or agent runs the prompts in **Invocation**
  and records results in the [run summary](run-summary-template.md).

## Inputs

### Updated implementation `design.md`

After running the **Initial Prompt** below in an implementation-schema
project, update `.specify/slices/loyalty-enrollment/design.md` with this
contract-detail block:

```markdown
## API Contracts

POST /loyalty/enrollments

Request LoyaltyEnrollmentRequest:
- customer_id: string, required
- email: string, required, format email
- referral_code: string, optional

Responses:
- 201 LoyaltyEnrollment with id, customer_id, tier, created_at
- 400 ErrorResponse for invalid email
- 409 ErrorResponse when customer_id is already enrolled
```

This update is the bait: a naive pipeline might be tempted to derive contract
YAML from it. The scenario asserts the pipeline does not.

## Invocation

### Initial Prompt (negative path)

Run inside an implementation-schema project. Start with a high-level change:

```text
/spec:refine loyalty-enrollment

Create a loyalty enrollment adapter. It should expose an HTTP API, but leave
endpoint details initially high level:
- customers can enroll in loyalty
- duplicate enrollment is rejected
- enrollment returns a loyalty account identifier
```

Then apply the **Updated implementation `design.md`** block from **Inputs** to
the slice's `design.md` and re-run `/spec:refine loyalty-enrollment` (or the
implementation pipeline's regenerate equivalent). Confirm — per
**Negative Expectations** — that this run produces no `contracts/**/*.yaml`
output.

### Recommended Regression Path

Now run the regression path inside a `contracts@1.0.0` project to produce the
correct contract artifacts:

```text
/spec:refine loyalty-enrollment-interface
/spec:build loyalty-enrollment-interface
/spec:merge loyalty-enrollment-interface
```

The regression slice is what `Expected Artifacts` describes; it is the
*correct* path an operator would take after observing the boundary in part 1.

## Expected Artifacts

These belong to the **regression path** (`/spec:refine loyalty-enrollment-interface`),
not the negative path. During `/spec:build`, the dedicated contract change
should produce these change-local contract deltas. After merge, the same paths
become root `contracts/` baseline files.

- `contracts/http/loyalty-api.yaml`
- `contracts/schemas/loyalty-enrollment-request.yaml`
- `contracts/schemas/loyalty-enrollment.yaml`
- `contracts/schemas/error-response.yaml`

The negative path has no expected artifacts under `contracts/**/*.yaml`; the
absence of contract output is itself the oracle (see **Negative Expectations**).

## Assertions

- `implementation-schema-emits-no-contract-yaml`: after the implementation
  slice's `/spec:refine` regeneration over the updated `design.md`, no
  `contracts/**/*.yaml` files exist in the slice working tree, and no contract
  deltas are queued for merge into the baseline.
- `regression-path-files-exist`: every path in **Expected Artifacts** exists in
  the regression slice's working tree after `/spec:build loyalty-enrollment-interface`,
  and in the baseline `contracts/` tree after `/spec:merge loyalty-enrollment-interface`.
- `regression-path-contract-validator-clean`: the regression slice's contract
  verifier exits `0` with no findings and no manual-review warnings on the
  produced artifacts.

## Negative Expectations

These are the load-bearing oracle for this scenario.

- `implementation-design-emits-contract-yaml`: a plain implementation-schema
  `/spec:refine` regeneration over the updated `loyalty-enrollment/design.md`
  must not derive contract YAML. The `## API Contracts` block in `design.md`
  is design-time context for the implementation slice; it is not a contract
  authoring source.
- `implementation-slice-merges-contract-deltas-to-baseline`: the
  `loyalty-enrollment` implementation slice's merge step must not promote any
  `contracts/**/*.yaml` files into the root `contracts/` baseline. The only
  legitimate way new contract artifacts reach the baseline is through a
  separate `contracts@1.0.0` slice (the regression path above).
- `implementation-slice-pre-authors-interface-shapes`: the implementation
  slice must not author Omnia or Vectis interface shapes inline as a
  workaround. Implementation work that depends on the new API must depend on
  the regression `contracts@1.0.0` slice and read the merged baseline
  `contracts/` files.

## Cleanup

Drop or archive both slices (the implementation `loyalty-enrollment` slice and
the regression `loyalty-enrollment-interface` slice) before moving to the
next scenario. If you ran the negative path and regression path in the same
project, drop the implementation slice first so its `design.md` update does
not leak into subsequent runs.
