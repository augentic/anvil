# RFC-29d: Target Build Envelope and Fan-Out Proof

> Status: Draft — Milestone **M3** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29c](rfc-29c-synthesis.md) (rendered artifacts), [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (`execution`, [durable spec](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-execution-mode-d9)) — Unblocks: RM-18 hosted execute; the RFC-29 acceptance proof

This is the final independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). It defines the target build request/report envelopes and the acceptance fixture that proves fan-in twice (Leads and Evidence) and fan-out once (multiple slices from shared source claims).

This document is the source of truth for D6, the target side of D9, and D7. The cross-milestone wire contracts it appends to are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Decisions owned by this milestone

| ID | Decision |
| --- | --- |
| **D6 Target build envelope** | Target adapters receive a stable per-slice build request and return a stable per-slice build report, keyed on `(slice, target)`; reports may include RFC-28 findings. |
| **D9 Adapter execution mode** (target side) | Target adapters use the shipped `execution: tool | agent` enum for `build` / `merge` dispatch. |
| **D7 Acceptance proof path** | The release is not complete until one end-to-end fixture demonstrates cross-source fan-in and cross-slice fan-out together. |

## Design invariants

- `model.yaml` remains the audit/provenance source produced by [RFC-29c](rfc-29c-synthesis.md). It is not a target build input.
- Targets build from rendered artifacts: `proposal.md`, one or more `spec.md` files, `design.md`, `tasks.md`, and target-specific inputs.
- `plan.yaml` stores a slice's bound `project`, not a resolved per-slice `target`; the CLI resolves the target from project topology at build time.
- A dependent slice reads upstream output from the merged working tree. RFC-29d adds no per-request cross-slice data channel.
- Build envelopes are closed-shape YAML and schema-validated; target-specific input growth goes through an explicit `additional` list.

## Target build envelope (D6)

The build request and build report validate against `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json`.

### Build request

`specrun slice build <slice> [--phase prepare|finalize] [--format json]` owns the build envelopes; `/spec:build` orchestrates the target brief between the two phases. Dispatch mirrors the shipped source two-phase agent contract ([`specify-cli` `DECISIONS.md` §"Source operations (D1)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1)), so build is the symmetric target-side twin of `specrun source survey` / `extract`:

- `execution: tool` ignores the phase flag: one call pipes the request on stdin to the declared WASI tool or deterministic Rust path and captures the report on stdout.
- `execution: agent` splits. `--phase prepare` (the default) resolves the target from the bound project, assembles and schema-validates the request, writes `.specify/slices/<slice>/build/request.yaml`, emits `target.execution.agent`, prints a kebab-case handoff envelope on stdout, and returns without blocking. The agent then runs the target `build` brief against the prepared request and writes `.specify/slices/<slice>/build/report.yaml`. `--phase finalize` validates that report, gates the `built` transition, and journals `slice.build.succeeded` / `slice.build.failed`.

The CLI owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; the brief owns only code generation. v1 keeps the verb deliberately thin — a future milestone can add flags (e.g. a partial-rebuild filter) without changing the envelope shapes.

| Field | Purpose |
| --- | --- |
| `version` | Envelope version. |
| `slice` | Slice being built. |
| `project-dir` | Working tree the target builds into and validates against. |
| `inputs.root` | Slice tree that all artifact paths resolve against. |
| `inputs.artifacts.proposal` / `design` / `tasks` | Singular rendered artifacts. |
| `inputs.artifacts.specs[]` | One or more per-unit `spec.md` files. |
| `inputs.artifacts.additional[]` | Target-specific inputs, such as vectis `tokens.yaml` / `assets.yaml` / `components.yaml` or the contracts `contracts/` subtree. |

`inputs.artifacts.additional[]` is assembled from the bound target adapter's manifest, not from convention: each target declares the extra inputs its `build` consumes (paths relative to `inputs.root`, each flagged `required`). The CLI resolves those declarations against the slice tree and raises `target-build-input-missing` when a `required` path is absent. v1 keeps the declaration a flat path list; richer selection (globs, conditional inputs) can extend the manifest field later without changing the envelope.

The request omits data the recipient already owns or must not consume:

| Omitted field | Reason |
| --- | --- |
| `target` | The recipient adapter is the target; the CLI derives `(slice, target)` from the bound project. |
| `execution` | The CLI uses the target's declared execution mode to choose delivery, then leaves it out of the payload. |
| brief paths | Tool targets have compiled logic; agent targets resolve their `build` brief from the bound adapter. |
| `model.yaml` | It is audit/provenance input to rendered artifacts, not a build input. |

```yaml
version: 1
slice: identity-service
project-dir: /workspace/.specify/workspace/identity-service
inputs:
  root: /workspace/.specify/slices/identity-service
  artifacts:
    proposal: proposal.md
    specs:
      - specs/identity/spec.md
    design: design.md
    tasks: tasks.md
    additional:
      - tokens.yaml
```

`inputs.root` and `project-dir` are distinct by design. In workspace mode, for example, `inputs.root` is `<workspace>/.specify/slices/<slice>` while `project-dir` is `<workspace>/.specify/workspace/<project>`.

Cross-slice dependency is plan-level ordering, not envelope plumbing. If `identity-service` depends on `identity-contracts`, `plan.yaml.slices[].depends-on` and `specrun plan next` ensure `identity-contracts` merges first; `identity-service` then reads the generated files as ordinary in-tree files.

### Build report

Each target returns a build report. `status` is `success` or `failure`. Partial success is `success` carrying non-blocking findings only (per the RFC-28 `blocking` predicate).

| Field | Purpose |
| --- | --- |
| `version` | Envelope version. |
| `slice` | Slice that was built; must match the request. |
| `target` | Adapter that produced the report (e.g. `omnia@v1`). |
| `status` | `success` or `failure`. |
| `findings` | RFC-28 diagnostics; default `[]`. On `success`, only non-blocking findings (or empty). On `failure`, populate with blocking (`critical` / `important`) violations when the target can map them; otherwise leave empty. |

Success:

```yaml
version: 1
slice: identity-service
target: omnia@v1
status: success
findings: []
```

Failure:

```yaml
version: 1
slice: identity-service
target: omnia@v1
status: failure
findings: []
```

Failure with findings:

```yaml
version: 1
slice: identity-contracts
target: contracts@v1
status: failure
findings:
  - id: DIAG-0001
    rule-id: contract.id-unique
    title: Duplicate info.x-specify-id across baseline
    severity: critical
    source: tool
    kind: violation
    target-adapter: contracts
    slice: identity-contracts
    artifact: contracts
    location:
      path: contracts/http/user-api.yaml
    evidence:
      kind: structured
      summary: x-specify-id user-api collides with legacy-api.yaml
      data:
        detail: info.x-specify-id user-api also present on contracts/http/legacy-api.yaml
    impact: Downstream consumers cannot resolve a unique contract id.
    remediation: Rename or remove the duplicate id before merge.
    fingerprint: sha256:a2e95674f838eb042eba78e16239f32199def3ca976e29499f8275beb30225e4
```

`findings[]` items validate against `schemas/diagnostics/diagnostic.schema.json` (RFC-28). The CLI rejects `status: success` reports carrying any blocking-severity finding — `critical` or `important` per the RFC-28 `blocking` predicate (`target-build-success-with-blocking-finding`).

The report is persisted at `.specify/slices/<slice>/build/report.yaml`, validated before `specrun slice transition <slice> built`, and is not consumed by other slice builds.

### Target adapter responsibilities

- `shape` remains synthesis guidance.
- `build` consumes only the request's `inputs` manifest and produces a build report.
- `merge` requires lifecycle `built` and re-runs target-specific validators per the merge brief. v1 adds no merge envelope: `specrun slice merge` is the writer, `slice.merge.succeeded` / `slice.merge.failed` fire on its validator outcome, and the durable record is the existing `slice.archive.created` outcome ledger. If a target later needs merge findings persisted, the build-report shape is reused as `build/merge-report.yaml` rather than authoring a second schema.
- Agent-generated code must pass target-local validation before `status: success`.

Recommended first-party implementation order:

1. `contracts`, because API contracts are already structured outputs.
2. `omnia`, because Rust crate generation benefits from typed requirements, APIs, configuration, and replay examples.
3. `vectis`, because UI layout, assets, tokens, and `composition.yaml` need the widest slice-model shape.

## Adapter execution mode (D9, target side)

Target adapters declare the shipped `execution: tool | agent` field:

- `tool` — `build` and `merge` run through a declared WASI tool or deterministic Rust path; inputs and outputs validate against the build envelopes above.
- `agent` — the target brief is agent-executed in the same sandbox; the CLI orchestrates inputs, validates outputs, and emits `target.execution.agent` per invocation.

Build outputs are not cached in either mode.

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path:

```text
documentation + code-typescript
  -> source survey              # fan-in #1: Lead sets
  -> plan propose --dry-run     # kernel returns a flat lead catalog
  -> plan propose --from        # agent groups leads; kernel validates and writes plan
  -> per slice:
       source extract           # fan-in #2: Evidence per source
       slice synthesize         # agent authors requirements; kernel projects ids/status/provenance
       target build             # one target resolved from the bound project
       slice merge              # one shared baseline
  -> validate depends-on ordering
```

The fixture uses the **same-tree** topology: both registry projects resolve into one working tree through `registry.yaml` URLs of `.` / repo-relative paths, materialised as symlinks per [Registry §"Files and state"](../docs/reference/registry.md#files-and-state). They therefore share one `.specify/slices/` tree and one baseline; upstream generated output becomes visible when the upstream slice merges.

Minimum fixture:

```text
tests/fixtures/rfc-29/fan-in-fan-out/
  sources/{docs,legacy}/
  expected/
    discovery.md
    plan.yaml
    slices/identity-contracts/
      evidence/docs.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      model.yaml
      build/report.yaml
    slices/identity-service/
      evidence/{docs,legacy}.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      model.yaml
      build/report.yaml
```

### Plan-time assertions

- `specrun source survey` produces schema-valid leads for both sources, including the deliberate id mismatch `docs:password-reset` / `legacy:reset-password` for synopsis-based grouping.
- `specrun plan propose --dry-run --format json` returns a schema-valid `kind: request` envelope, writes nothing, carries one `leads[]` row per `(source, lead)`, and exposes the two registry projects: `identity-contracts` → `contracts@v1` and `identity-service` → `omnia@v1`.
- The `/spec:plan` agent step, or a harness simulating it, returns a `kind: response` that matches `docs:identity-api` with `legacy:identity-api`, fans that shared source set into `identity-contracts` and `identity-service`, binds each slice to its project, links them with `identity-service.depends-on: [identity-contracts]`, and matches `password-reset` with `reset-password` into a third slice by synopsis judgment.
- `specrun plan propose --from` writes single-target slices carrying `project` bindings only, emits `plan.reconcile.completed`, and renders cross-source matches into `change.md` for Gate-1 review.
- `specrun plan propose` with neither `--dry-run` nor `--from` exits non-zero with `plan-propose-mode-required`.

Malformed `--from` responses are rejected with these validation codes:

- `plan-reconcile-partition` — a surveyed lead is left unaccounted for; fan-out may cite one lead from multiple slices.
- `plan-reconcile-lead-orphan` — a cited `(source, lead)` pair is absent from the recomputed catalog.
- `plan-reconcile-slice-source-collision` — one slice names two leads from the same source.
- `plan-reconcile-slice-name-collision` — two slices share an agent-supplied `name`.
- `plan-reconcile-depends-on-cycle` — the `depends-on` graph cycles.
- `plan-reconcile-project-binding-required` — a slice omits `project` when more than one project is offered.
- `plan-reconcile-project-orphan` — a slice binds a `project` absent from the request's `projects[]`.
- `plan-reconcile-plan-not-replaceable` — the response would replace an approved or partially executed plan.

### Slice-time assertions

- `specrun source extract` writes schema-valid Evidence for every `(slice, source)` pair.
- `specrun slice synthesize` writes valid artifacts and `model.yaml` with inline provenance for each slice.
- `specrun slice provenance` projects the audit view from `model.yaml` on demand.
- `specrun slice validate` catches no slice-model staleness on either slice.
- A fixture-local synthesis response that pre-assigns `REQ-NNN`, sets `status`, or marks a per-claim `winner` is normalized: the kernel ignores and re-derives every kernel-owned field.
- A synthesis response citing a `(source, id)` absent from the Evidence map is rejected with `slice-model-source-orphan` before projection.

### Build and merge assertions

- Each slice builds independently against its single bound target.
- `specrun plan next` orders execution so `identity-contracts` reaches `merged` before `identity-service` starts.
- Because both projects share one working tree, `identity-service` resolves `identity-contracts` output as an ordinary in-tree dependency; no cross-slice build-request channel exists.
- The release gate is generated output correctness: each target build must pass the target's replay/golden suite and `cargo check` / `cargo test` where applicable. A slice whose generated output fails is not done, regardless of envelope validity.

### Non-blocking determinism property

Re-running kernel projection twice over a golden synthesis response yields byte-identical, target-independent kernel-owned `model.yaml` fields and projected provenance.

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **CLI surface:** `specrun slice build <slice> [--phase prepare|finalize] [--format json]` — the build-envelope owner, mirroring the shipped `specrun source survey` / `extract` two-phase agent contract. Merge stays on the existing `specrun slice merge`; v1 adds no `specrun slice merge` envelope.
- **Journal events:** `slice.build.started`, `slice.build.succeeded`, `slice.build.failed`, `slice.merge.started`, `slice.merge.succeeded`, `slice.merge.failed`, `target.execution.agent`. The `slice.merge.*` pair fires on the `specrun slice merge` validator outcome, not on a merge report.
- **Operational validation codes (`Error::Validation`, not new enum variants):** `target-build-request-schema`, `target-build-report-schema`, `target-build-success-with-blocking-finding`, `target-build-input-missing` (a `required` adapter-declared `inputs` path is absent from the slice tree) — single-signal build-envelope aborts at exit 2.
- **Schemas:** `schemas/target/build-request.schema.json` (`BUILD_REQUEST_JSON_SCHEMA`), `schemas/target/build-report.schema.json` (`BUILD_REPORT_JSON_SCHEMA`). No merge schema in v1; if one is needed later it reuses the build-report shape.