# RFC-29d: Target Build Envelope and Fan-Out Proof

> Status: Draft — Milestone **M3** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29c](rfc-29c-synthesis.md) (consumes its `model.yaml`), [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (the `execution` enum, [durable spec in `specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-execution-mode-d9)) — Unblocks: RM-18 hosted execute; the RFC-29 acceptance proof

This is the final independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). The build request/report envelopes and the first-party targets consume `model.yaml` from M2b; the end-to-end D7 fixture is the final release gate that proves fan-in twice and fan-out once. The build-envelope schemas are **authored during this milestone's implementation** (not shipped as drafts), and the cross-project artifact-handoff case is a deferred open question.

The cross-milestone wire contracts this milestone appends to are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D6, the target side of D9, and D7.

## Decisions owned by this milestone

| ID | Decision |
| -- | -------- |
| **D6 Target build envelope** | Target adapters receive a stable per-slice build request and return a stable per-slice build report, keyed on `(slice, target)`; reports may include RFC-28 findings. |
| **D9 Adapter execution mode** (target side) | Target adapters adopt the closed `execution` enum (shipped in M1; durable spec in [`specify-cli` `DECISIONS.md` §"Adapter execution mode (D9)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-execution-mode-d9)) for `build` / `merge` dispatch. |
| **D7 Acceptance proof path** | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together. |

## Target build envelope (D6)

The build request and build report are both closed-shape YAML envelopes, keyed on `(slice, target)`. Normative schemas — `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` — are authored during this milestone's implementation. Examples below are illustrative.

### Build request

`/spec:build` constructs one build request per slice and either pipes it on stdin to a declared WASI tool (when the target's `execution: tool`) or writes it to `.specify/slices/<slice>/build/request.yaml` (when `execution: agent`). The request's `target` is **resolved at build time** from the slice's bound project (via `specrun plan next`, which derives it from the project topology) — `plan.yaml` no longer stores a per-slice `target`:

```yaml
version: 1
slice: identity-service
target: omnia@v1
phase: build
project-root: /workspace/.specify/workspace/identity-service
workspace-root: /workspace
slice-dir: /workspace/.specify/slices/identity-service
model-path: /workspace/.specify/slices/identity-service/model.yaml
artifacts:
  proposal: proposal.md
  design: design.md
  tasks: tasks.md
  specs:
    - specs/identity/spec.md
  provenance: provenance.yaml
briefs:
  shape: /.../adapters/targets/omnia/briefs/shape.md
  build: /.../adapters/targets/omnia/briefs/build.md
execution:
  mode: tool
  tool:
    name: omnia
    version: v1.4.2
cache-fingerprint: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

A slice builds against the **merged working tree**, not against a cross-slice data channel. When a slice depends on another slice's output — e.g. `identity-service` (omnia) consuming `identity-contracts`' generated schema — the dependency is declared at the plan layer via `plan.yaml.slices[].depends-on`, and `specrun plan next` orders execution so the depended-on slice reaches `done` (merged into the baseline) before the dependent slice starts. The dependent target then reads the upstream output from the working tree as ordinary files, the way its build tooling already resolves dependencies. No per-request cross-slice channel is introduced; see [RFC-29 §"Open questions"](rfc-29-fan-in-fan-out.md#open-questions) for the deferred cross-project (workspace-mode) artifact-handoff case.

### Build report

Each target returns a build report. `status` is two-state (`success | failure`); partial outcomes land as `success` plus non-blocking findings at severity `optional` or `suggestion`:

```yaml
version: 1
slice: identity-service
target: omnia@v1
phase: build
status: success
started-at: 2026-05-28T05:45:00Z
finished-at: 2026-05-28T05:46:12Z
generator: omnia@v1.4.2
generated-paths:
  - crates/identity_service
  - crates/identity_service/Cargo.toml
validation:
  commands:
    - command: cargo check
      exit-code: 0
      duration-ms: 4123
    - command: cargo test
      exit-code: 0
      duration-ms: 18802
findings: []
evidence-cited: [docs, legacy]
cache:
  fingerprint: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  outcome: miss
```

`findings[]` items validate against `schemas/diagnostics/diagnostic.schema.json` (RFC-28). The CLI rejects `status: success` reports carrying any `critical`-severity finding (`target-build-success-with-critical-finding`).

The report is persisted at `.specify/slices/<slice>/build/report.yaml` as the slice-local record of its own build; it is not consumed by other slices' build requests.

### Target adapter responsibilities

Target `build` briefs change from "read Markdown and decide what to do" to "consume the build request and produce a build report":

- `shape` remains synthesis guidance.
- `build` consumes `model.yaml` and rendered artifacts.
- `merge` consumes build reports and target-specific validation state.
- Any agent-generated code must still pass target-local validation before `status: success`.

### First-party target build order

The recommended implementation order is:

1. `contracts` first, because API contracts are already structured outputs.
2. `omnia` second, because Rust crate generation benefits most from typed requirements, APIs, configuration, and replay examples.
3. `vectis` third, because UI layout, assets, tokens, and `composition.yaml` need the widest slice-model shape.

## Adapter execution mode (D9, target side)

Target adapters declare the same closed `execution: tool | agent` field shipped in M1 ([`specify-cli` `DECISIONS.md` §"Adapter execution mode (D9)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-execution-mode-d9)). On the target side it governs `build` / `merge` dispatch:

- `tool` — `build` and `merge` run through a declared WASI tool or deterministic Rust path; inputs/outputs validate against the build envelopes above.
- `agent` — the target brief is agent-executed against the same sandbox; the CLI orchestrates inputs and validates outputs but does not cache, emits `target.execution.agent` per invocation, and forces `cache: opt-out`.

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path — fan-in twice (Leads and Evidence), fan-out once (across slices):

```text
documentation + code-typescript
        -> source survey                 (fan-in #1: Lead sets)
        -> plan propose --dry-run           (kernel returns a flat lead catalog)
        -> plan propose --from              (envelope: agent-led cross-source matching
                                             and per-scope project binding;
                                             kernel: validate, partition invariant, slice-name
                                             derivation, journal, plan writers)
        -> per slice:
             source extract                 (fan-in #2: Evidence per source)
             slice synthesize               (envelope: agent-led cross-modal reconciliation
                                              into the requirement set + prose;
                                              kernel: deterministic projection — ids, status,
                                              provenance — over the returned structure;
                                              one Evidence map -> one slice model)
             model.yaml + artifacts + provenance.yaml
             target build (one target)
             slice merge (one baseline)
        -> validate cross-slice ordering via depends-on
```

Minimum fixture:

```text
tests/fixtures/rfc-29/fan-in-fan-out/
  sources/
    docs/
    legacy/
  expected/
    discovery.md
    plan.yaml                               # two slices, identity-service depends-on identity-contracts
    slices/identity-contracts/
      evidence/docs.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      provenance.yaml
      model.yaml                                # target: contracts@v1
      build/report.yaml
    slices/identity-service/
      evidence/docs.yaml
      evidence/legacy.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      provenance.yaml
      model.yaml                                # target: omnia@v1; sources include docs + legacy
      build/report.yaml                      # slice-local build record
```

Required assertions:

- `specrun source survey` produces schema-valid leads for both sources, including a deliberate id-mismatch pair (`docs` lead `password-reset` and `legacy` lead `reset-password`) that exercises the agent's synopsis-based grouping.
- `specrun plan propose --dry-run --format json` returns a `kind: request` envelope whose `leads[]` carries one row per `(source, lead)` (including two rows that share slug `identity-api` and two rows with different slugs for password reset), and whose `projects[]` surfaces the two registry projects (`identity-contracts` → `contracts@v1`, `identity-service` → `omnia@v1`); it validates against `proposal.schema.json` and writes nothing.
- The fixture's `/spec:plan` agent step (or the test harness simulating it) returns a `kind: response` whose `slices[]` matches `docs:identity-api` with `legacy:identity-api` by shared slug under `scope: identity-api` (fanned out to two slices with explicit `name` values `identity-contracts` and `identity-service` and **bound registry projects** `identity-contracts`, `identity-service`, both carrying identical `sources[]`) and matches `password-reset` with `reset-password` by synopsis judgment under a separate `scope: password-reset`; `specrun plan propose --from` writes the single-target slices each carrying its bound `project` (the target is resolved on demand from that project, not written to `plan.yaml`), with `identity-service.depends-on: [identity-contracts]`, emits `plan.reconcile.agent` + `plan.reconcile.completed`, and the cross-source matches are rendered into `change.md` for Gate-1 review.
- `specrun plan propose --from` rejects a response that leaves a surveyed lead unaccounted for or double-counts one across scopes (`plan-reconcile-partition`), one that cites a `(source, lead)` pair absent from the **recomputed** catalog (`plan-reconcile-lead-orphan`), one whose scope names two leads from the same source (`plan-reconcile-slice-source-collision`), one whose slices share a `scope` id but carry differing `sources[]` (`plan-reconcile-fanout-source-mismatch`), one with duplicate `(scope, project)` rows (`plan-reconcile-slice-duplicate`), one whose agent-supplied explicit `name` values collide (`plan-reconcile-slice-name-collision`), one whose `depends-on` graph cycles (`plan-reconcile-depends-on-cycle`), one that omits `project` on a slice when more than one project is offered (`plan-reconcile-project-binding-required`), one that binds a slice to a `project` absent from the request's `projects[]` (`plan-reconcile-project-orphan`), and one that would replace an approved or partially executed plan (`plan-reconcile-plan-not-replaceable`); `specrun plan propose` with neither `--dry-run` nor `--from` exits non-zero with `plan-propose-mode-required`.
- `specrun source extract` writes schema-valid Evidence for every `(slice, source)` pair.
- `specrun slice synthesize` writes valid artifacts, `provenance.yaml`, and `model.yaml` for each slice.
- `specrun slice validate` catches no provenance or slice-model drift on either slice.
- Each slice builds independently against its single bound target; `identity-service` reads `identity-contracts`' merged output from the working tree (the dependency is ordered by `depends-on` + `plan next`, not carried on the build request).
- `specrun plan next` orders execution so `identity-contracts` reaches `merged` before `identity-service` starts.
- **Kernel-projection determinism.** Re-run kernel projection twice over a golden synthesis response; `provenance.yaml` and kernel-owned `model.yaml` fields are byte-identical and target-independent (D11). Live agent runs are not byte-stable on requirement set or prose.
- **D8 envelope-construction proof.** Synthesis request requirements-relevant inputs are byte-identical across `contracts@v1` and `omnia@v1` bindings; `target` / `shape-brief` differ only in non-requirements fields.
- **Forbidden-input-leak probe (deterministic).** A fixture-local test confirms the envelope walls `target` and `shape-brief` off from the requirements section: a probe response whose requirements section contains a token present in `target` or the `shape-brief` file but in **no** cited Evidence claim is flagged by `slice-synthesize-forbidden-input-leak` via a mechanical set-difference test (not a semantic judgement), proving the target-neutrality-by-construction layer of D8.
- **Synthesis envelope contract.** A fixture-local test re-runs `specrun slice synthesize` with a deliberately-malformed synthesis-step response that usurps a kernel-owned field — pre-assigns a `REQ-NNN` id, sets `status`, marks a per-claim `winner`, or cites a `(source, id)` absent from the Evidence map. The engine rejects the draft with `slice-synthesize-kernel-field-usurped` (kernel fields) or `slice-model-source-orphan` (orphan claim) **before** projection, proving the kernel is the sole authority on id assignment, rendered source list derivation, winner selection, status derivation, and provenance projection while the agent remains the sole author of the requirement set, its claims, and its agreement verdict.

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `slice.build.started`, `slice.build.succeeded`, `slice.build.failed`, `slice.merge.started`, `slice.merge.succeeded`, `slice.merge.failed`, `target.execution.agent`.
- **Operational validation codes (`Error::Validation`, not new enum variants):** `target-build-request-schema`, `target-build-report-schema`, `target-build-success-with-critical-finding` — single-signal build-envelope aborts at exit 2. See [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts) for the error-tiering model.
- **Schemas (authored in this milestone):** `schemas/target/build-request.schema.json` (`BUILD_REQUEST_JSON_SCHEMA`), `schemas/target/build-report.schema.json` (`BUILD_REPORT_JSON_SCHEMA`).

## Open question

Cross-project artifact handoff in workspace mode (when two fan-out slices bind different projects) is deferred to a future RFC; v1 ships no Specify-specific cross-slice channel. See [RFC-29 §"Open questions"](rfc-29-fan-in-fan-out.md#open-questions) Q2.
