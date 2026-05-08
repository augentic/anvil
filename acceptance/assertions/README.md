# Acceptance Assertions

> Status: Starter helpers landed by C05 (contracts smoke runner). Helpers under this directory are the runner's pass/fail oracle; the runner stages their dispatch between `backend.invoke()` and `backend.teardown()` (see [`../runner/backends/README.md`](../runner/backends/README.md#assertions-stage)).

Assertion helpers live here so every suite — narrow capability scenarios under `capabilities/<capability>/tests/` and shared outside-in suites under [`../suites/`](../suites/README.md) — uses the same vocabulary. The [runner](../runner/README.md) collects evidence; this layer turns evidence into pass/fail.

The guiding rule is: **assert durable structures, not generated prose.** A live agent will phrase the same plan or proposal differently from one run to the next; the framework's job is to prove that the structure underneath is correct.

## What To Assert

Assertions should target stable structures:

- **Files exist or do not exist** at expected change-local or baseline paths.
- **YAML/JSON fields match expected roles** — for example, a plan entry has `schema: contracts@v1`, or an implementation entry has `project: shop-backend`.
- **CLI validation passes** — `specify change plan validate`, `specify change plan doctor`, `specify slice validate`, the `contract` WASI tool exit cleanly.
- **Lifecycle statuses transition legally** — slice and plan-entry transitions follow the legal set the CLI enforces.
- **Capability-owned paths are touched** — generated artifacts land inside the directories that capability briefs declare, and only those.
- **Forbidden paths remain untouched** — implementation slices do not write into the contract baseline; specialist skills do not write outside their capability's project paths.
- **Generated code builds or tests when the capability requires it** — the narrowest reliable check that proves the artifact is real (`cargo check` for an Omnia crate, the contracts WASI validator for contract artifacts).
- **Branch names and commit boundaries match the workflow contract** — branches are exactly `specify/<change-name>`; baseline merge commits contain only `.specify/specs/` and `.specify/archive/`; residue commits are tagged `specify: residue <slice-name>`.
- **Fake forge state reaches the expected PR status** — push opens or updates a PR, an externally-merged PR is observed by `specify change finalize`, a re-run of finalize returns `plan-not-found`.

## What Not To Assert

Assertions should not target unstable structures:

- **Exact generated proposal, spec, or design prose.** Live models legitimately rephrase. Asserting prose locks the framework to a specific model and a specific run.
- **Exact generated implementation code beyond capability-owned structural contracts.** Assert that an Omnia crate has the expected file layout and passes its capability check; do not assert exact function bodies.
- **Exact live-agent wording in transcripts or tool-call argument prose.** Recorded-transcript replay may compare *intent* (which tool was called with which structural argument) but never byte-for-byte prose.
- **Ordering of independent explanatory bullets.** Order matters when the workflow says it does (plan dependencies, transition sequences); it does not matter for descriptive prose.

For live agent runs, prefer **role-based matching** over exact names. An RM-01 plan should contain one contract slice and two routed implementation slices; the exact slice names can vary as long as roles, dependencies, and project routing are correct.

## Output Shape

Every assertion module writes a structured entry into `assertions.json` in the run directory. The exact schema is owned by the runner skeleton change in the plan; this README fixes the contract that suites and CI can rely on:

- Each entry carries an assertion id, a short human-readable description, a pass/fail verdict, and — on failure — the evidence path that disproves it (a missing file, a JSON field whose value did not match, a verifier finding line in `stdout.log`).
- A failing entry should also carry a fault-domain hint when the assertion can credibly attribute the failure (see the runner's [Failure Reporting](../runner/README.md#failure-reporting) taxonomy).

This shape lets `summary.md` render a compact verdict for humans while CI consumes the same `assertions.json` for annotations.

## Reusable Helpers (Available Now)

C05 landed the starter set as TypeScript modules importable from `acceptance/assertions/index.ts`:

- [`files.ts`](files.ts) — `assertFileExists(id, workspace, declaredPath)`, `assertFileAbsent(id, workspace, declaredPath)`, `assertNoMatchingPath(id, workspace, globs[])` for `*` / `**` patterns.
- [`forbidden.ts`](forbidden.ts) — `assertForbiddenPathsUntouched(id, workspace, globs[])` rolls multiple matches into one record per declared boundary id.
- [`verifier.ts`](verifier.ts) — `assertVerifierStatus({ id, contractsDir, stdout, expected })` placeholder for the contract WASI validator. Returns `skip` when no stdout was captured (the real `specify tool run contract` wiring lands with C13).
- [`yaml.ts`](yaml.ts) — `assertYamlField({ id, path, jsonPointer, expected, faultDomain? })` reads a YAML file, navigates to a node addressed by an RFC 6901 JSON Pointer, and compares the leaf against an expected scalar (deep equality for arrays/objects). Reusable beyond RM-01 — registry shape rules, plan-file fields, project.yaml invariants. Failures attribute through the same fault-domain taxonomy as the rest of the suite.
- [`setup.ts`](setup.ts) — `setupHandlers(inputs)` and `runSetupAssertions(inputs, ctx)` for the four `setup-*` invariants (C07): `setup-hub-project-yaml-has-hub-true-and-no-capability`, `setup-registry-has-two-entries`, `setup-registry-entries-have-non-empty-descriptions`, `setup-registry-validate-clean`.
- [`plan-roles.ts`](plan-roles.ts) — `planRoleHandlers(inputs)` for the nine RM-01 cross-repo `plan-*` rules (C09). Reads `plan.yaml` once per run, resolves the contract-role entry from structure (schema match + no project + empty depends-on), and demotes downstream rules to `skip` when an upstream `setup-*` rule failed.
- [`types.ts`](types.ts) — `AssertionContext`, `AssertionEvidence`, `AssertionHandler`, plus `pass`/`fail`/`skip` builders that produce the runner's `AssertionRecord` shape.

Helpers must:

- accept a workspace-relative path and reject anything that escapes the workspace (`..` or absolute),
- attribute failures to one of the runner's [Failure Reporting](../runner/README.md#failure-reporting) fault domains — never invent new strings,
- produce one record per `assertions.json` row; if a single id maps to several checks, return multiple records or roll matches into one record's `evidence` list.

### Assertion Dispatch

The runner translates `assertions:` ids declared in scenario frontmatter to handler calls during the `assertions` stage. The current dispatch table lives in [`../runner/assertions.ts`](../runner/assertions.ts) and registers, at a minimum:

| Assertion id | Handler |
| --- | --- |
| `files-exist` | `assertFileExists` over `expected-artifacts:` |
| `regression-path-files-exist` | `assertFileExists` over `expected-artifacts:` |
| `files-absent` | `assertFileAbsent` over scenario-supplied paths |
| `implementation-schema-emits-no-contract-yaml` | `assertForbiddenPathsUntouched` against `contracts/**/*.yaml` |
| `implementation-slice-merges-contract-deltas-to-baseline` | `assertForbiddenPathsUntouched` against `contracts/**/*.yaml` |
| `contract-validator-clean` | `assertVerifierStatus` (currently `skip` until the verifier is wired) |
| `regression-path-contract-validator-clean` | `assertVerifierStatus` (currently `skip`) |
| `setup-*` (×4) | [`setup.ts`](setup.ts) handlers — wired only when the run carries cross-repo setup state (`RunContext.setup`). |
| `plan-*` (×9) | [`plan-roles.ts`](plan-roles.ts) handlers — wired only when the run carries cross-repo setup state. The runner sorts `setup-*` ids ahead of `plan-*` ids so a setup failure cleanly demotes plan rules to `skip`. |

Unknown ids produce a `skip` record that points at the dispatcher so a missing handler is visible without failing the run. New handlers register here; capability- or suite-specific helpers can stay close to the suite and call the shared modules above.

Capability- or suite-specific assertions stay close to the suite that needs them and call the shared helpers; only assertions that more than one suite uses should land in this directory.

## Out Of Scope For This Directory

- This directory does not own scenario discovery or static metadata validation. Scenario discovery is documented in [`../README.md`](../README.md#scenario-discovery); static metadata validation is added to [`scripts/checks.ts`](../../scripts/checks.ts) by a follow-up change.
- This directory does not own backend selection or evidence collection. Those live in the [runner](../runner/README.md).
- This directory does not own prose goldens. The framework deliberately avoids prose goldens for live-agent output; selected JSON or tool-call goldens are allowed only when the output is intentionally stable and cheap to review.
