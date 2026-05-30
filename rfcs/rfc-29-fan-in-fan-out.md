# RFC-29: Fan-In/Fan-Out Code Contract

> Status: Draft (umbrella) — Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../done/rfc-28-standards-contract.md) — Enables: provable multi-source fan-in and plan-level multi-slice fan-out (D5)

This document is the **umbrella** for the RFC-29 family. It owns the abstract, the decision catalogue, the operator surface, and — most importantly — the **shared wire contracts** (the schemas, the closed `EventKind` taxonomy, and the closed validation-finding / `Error::Validation` code vocabulary) that the four implementation milestones must keep stable across their boundaries. The detailed mechanics of each decision live in the sub-RFC that ships it:


| Sub-RFC                                                                  | Milestone | Decisions                          |
| ------------------------------------------------------------------------ | --------- | ---------------------------------- |
| [RFC-29a — Executable Source Operations](rfc-29a-source.md)              | **M1**    | D1, D9 (source side), D12          |
| [RFC-29b — Plan-Time Lead Reconciliation](rfc-29b-reconciliation.md)     | **M2a**   | D2                                 |
| [RFC-29c — Slice Synthesis Engine and Typed Model](rfc-29c-synthesis.md) | **M2b**   | D3, D3a, D4, D5, D8, D10, D11, D13 |
| [RFC-29d — Target Build Envelope and Fan-Out Proof](rfc-29d-target.md)   | **M3**    | D6, D9 (target side), D7           |


Ordering is **M1 → M2a → M2b → M3** (each consumes the prior), but each milestone is reviewable, testable, and releasable on its own. See §"Sub-RFCs and milestone ordering".

## Abstract

Specify's architectural promise is a fan-in / fan-out workflow:

- **Fan-in** happens twice per change. Multiple source adapters' `Lead`s fan in at plan time into the `slices[]` rows of `plan.yaml`. Multiple sources' `Evidence` fans in at slice time into one synthesized slice. Both are core's responsibility.
- **Fan-out** happens once per change, at the plan layer. One change decomposes into multiple slices — each slice binding exactly one target — joined by `depends-on` edges. The `refine -> build -> merge` loop runs per slice; baseline merge runs once per slice against one target's baseline.

This is the framework's "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project)). RFC-29 affirms it and does not extend the slice to multi-target.

The gap is that several load-bearing fan-in steps — survey, extract, and plan-time lead reconciliation — are still uncontracted agent discipline rather than agent judgment running under a CLI-owned envelope. Both lead reconciliation (plan time) and slice synthesis (slice time) stay agent-led, because both are cross-source judgment with no deterministic function; in each case the CLI owns the **envelope** and the **projection kernel** around that judgment (see [RFC-29b](rfc-29b-reconciliation.md) and [RFC-29c](rfc-29c-synthesis.md)).

This RFC turns the fan-in promise into an end-to-end contract by adding:

1. **Executable source operations** ([RFC-29a](rfc-29a-source.md)) - first-class `specrun source survey` and `specrun source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Agent-led plan-time lead reconciliation** ([RFC-29b](rfc-29b-reconciliation.md)) - an agent-led cross-source matching step that groups each source's `Lead[]` into unified slice candidates (including semantic matches that exact id / alias cannot catch) and binds each `(group-id, target)` row to a target, running under a stable input/output envelope wrapped by a CLI-owned projection kernel: a deterministic structural floor, schema validation, the global lead-partition invariant, slice-name derivation, journal events, and the existing plan writers.
3. **Slice synthesis engine** ([RFC-29c](rfc-29c-synthesis.md)) - an agent-led cross-modal synthesis step (which decides the requirement set, declares each requirement's `(source, claim-id)` claims and an `agreement` verdict, and authors its prose) running under a stable input/output envelope, wrapped by a CLI-owned projection kernel that projects over the agent's structure: RFC-27 authority resolution, REQ-id assignment, `sources` and winner-marker derivation, status derivation, provenance projection into `provenance.yaml`, and drift validators.
4. **Typed slice model** ([RFC-29c](rfc-29c-synthesis.md)) - a machine-readable, schema-pinned view of the slice emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
5. **Target build contract** ([RFC-29d](rfc-29d-target.md)) - target adapters consume the slice model through a stable per-slice build envelope, with per-slice validation, review findings, and merge gates.
6. **Proof fixtures** ([RFC-29d](rfc-29d-target.md)) - acceptance coverage that exercises `N sources -> one slice model -> 1 target per slice`, with cross-target fan-out proven across multiple slices joined by `depends-on`, and the kernel / envelope split proven by two **deterministic** gates: kernel-projection determinism over a fixed synthesis response, and an envelope-construction proof that the requirements-relevant inputs are byte-identical across target bindings (no LLM judge in any gate).

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant: source operations are briefs not executable commands, plan-time lead reconciliation is uncontracted agent work (no envelope, no validation, no journal trail), slice-time reconciliation has no production resolver, the machine-readable slice view is implicit, and target codegen is adapter-brief discipline with no stable envelope. The normative decisions below close each gap.

The goal is not to remove agents but to wrap the agent's judgment in a stable envelope and to move stable workflow, data-shape, and bookkeeping obligations into the CLI. The two cross-source judgment steps — lead reconciliation (D2) and slice synthesis (D3/D10) — both default to `execution: agent`; see [RFC-29b](rfc-29b-reconciliation.md) and [RFC-29c](rfc-29c-synthesis.md) for the matching agent/kernel split.

## Normative decisions

The catalogue below is the canonical decision list. Each decision's full mechanics and implementation consequence live in the **Home** sub-RFC.


| ID                                     | Decision                                                                                                                                                                                                                                                                                               | Home                                                                |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| **D1 Source operation**                | The CLI runs source adapter `survey` and `extract` operations.                                                                                                                                                                                                                                         | [29a](rfc-29a-source.md)                                            |
| **D2 Lead reconciliation**             | Agent-led cross-source matching of `Lead[]` into lead groups, binding each `(group-id, target)` row to a target and (workspace mode) a registry project, under a CLI-owned projection kernel (structural floor, partition invariant, project-binding validation, slice-name derivation, plan writers). | [29b](rfc-29b-reconciliation.md)                                    |
| **D3 Slice synthesis**                 | Agent-led cross-modal reconciliation of `Evidence[]` into the requirement set; CLI owns the synthesis envelope and the projection kernel (authority resolution, REQ-ids, status/sources/winner derivation, provenance projection, rendering).                                                          | [29c](rfc-29c-synthesis.md)                                         |
| **D3a Draft vs persisted**             | Synthesis response validates against `draft-model.schema.json`; persisted `model.yaml` validates against `model.schema.json`.                                                                                                                                                                          | [29c](rfc-29c-synthesis.md)                                         |
| **D4 Typed slice**                     | Every synthesized slice carries `.specify/slices/<slice>/model.yaml`.                                                                                                                                                                                                                                  | [29c](rfc-29c-synthesis.md)                                         |
| **D5 Per-slice fan-out**               | Each slice binds exactly one target adapter / project; cross-target changes decompose at plan time into multiple slices joined by `depends-on`. No `outputs[]`.                                                                                                                                        | [29c](rfc-29c-synthesis.md)                                         |
| **D6 Target build**                    | Target adapters receive a stable per-slice build request and return a stable per-slice build report.                                                                                                                                                                                                   | [29d](rfc-29d-target.md)                                            |
| **D7 Acceptance proof**                | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together.                                                                                                                                                                                          | [29d](rfc-29d-target.md)                                            |
| **D8 Shape-brief scope**               | Target `shape` briefs parameterise non-requirements model sections only; never `requirements[]`, claims, agreement, `sources[]`, or any provenance-bearing field.                                                                                                                                      | [29c](rfc-29c-synthesis.md)                                         |
| **D9 Adapter execution**               | Source and target adapters declare a closed `execution: tool                                                                                                                                                                                                                                           | agent`field selecting deterministic dispatch vs an agent-run brief. |
| **D10 Synthesis execution**            | The synthesis step carries a closed `execution: agent                                                                                                                                                                                                                                                  | tool` enum; agent-first by design.                                  |
| **D11 Standalone provenance**          | `specrun slice provenance <slice>` is the standalone entry point onto the same projection kernel as D3.                                                                                                                                                                                                | [29c](rfc-29c-synthesis.md)                                         |
| **D12 Journal emitter**                | `specrun journal emit` is the schema-validated writer for agent-orchestrated phases with no deterministic emit command.                                                                                                                                                                                | [29a](rfc-29a-source.md)                                            |
| **D13 Claim contract (`id` + `kind`)** | Every contributing claim carries a stable `claim-id` and its `kind`; `evidence.schema.json` requires `claim-id` on every claim kind.                                                                                                                                                                   | [29c](rfc-29c-synthesis.md)                                         |


## Operator surface

The default operator rhythm does not change:

```bash
/spec:plan identity-refresh source docs=documentation:./docs source legacy=code-typescript:./legacy
specrun plan transition identity-refresh approved
/spec:execute
/spec:finalize identity-refresh
```

The new CLI surfaces are lower-level breakouts (each owned by the sub-RFC noted):

```bash
specrun source survey docs --format json                                                       # 29a
specrun source extract docs password-reset --slice identity-password-reset --format json       # 29a
specrun plan propose --dry-run --format json                 # D2 reconciliation request envelope (floor + inventory)   # 29b
specrun plan propose --from grouping.json --format json      # D2 kernel: validate agent grouping, write slices         # 29b
specrun slice synthesize identity-password-reset --format json                                 # 29c
specrun slice provenance identity-password-reset --format json   # D11 standalone projection onto the D3 kernel         # 29c
specrun slice model show identity-password-reset --format json                                 # 29c
specrun journal emit slice.synthesize.agent --payload '{"slice":"identity-password-reset"}'   # 29a (D12 emitter)
```

Cross-source lead matching and target binding are the `/spec:plan` agent step's judgment (D2); `specrun plan propose --dry-run` seeds it with the deterministic structural floor, and `specrun plan propose --from` is the kernel that validates the grouping and writes the slices through the existing plan writers.

Cross-target changes are planned as multiple slices, each bound to one target, joined by `depends-on`:

```bash
specrun plan add identity-contracts \
  --sources docs=identity-api \
  --target contracts@v1 --project identity-contracts

specrun plan add identity-service \
  --sources docs=identity-api,legacy=identity-api \
  --target omnia@v1 --project identity-service \
  --depends-on identity-contracts
```

Each entry keeps its existing one-target shape (see [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md)). In the default flow these rows are written by the D2 reconciliation kernel (`specrun plan propose --from`) projecting the agent's grouping through these same `plan add` writers; the explicit `plan add` form above stays available for manual authoring and illustrates the resulting plan shape.

A downstream slice that needs another slice's output (e.g. `omnia` consuming the `contracts` schema) declares the edge with `depends-on` at the plan layer; `specrun plan next` merges the upstream slice before the dependent starts, and the dependent target reads the upstream output from the merged working tree (see [RFC-29d §"Target build envelope (D6)"](rfc-29d-target.md)). No multi-output, multi-target shape is added to a single slice — the plan layer is the only place fan-out happens.

## Shared wire contracts

These contracts span milestone boundaries and are **pinned here** so a later milestone cannot silently redefine an earlier one's wire shape. Each sub-RFC names the subset it introduces and links back to these canonical tables. The shared contracts are: the six schemas in `[rfc-29/schemas/](rfc-29/schemas/)`, the closed `EventKind` additions, the closed validation-finding / `Error::Validation` code vocabulary, and the D13 `evidence.schema.json` `claim-id` requirement.

### Schemas

Four JSON Schemas ship as draft files alongside this RFC under `[rfc-29/schemas/](rfc-29/schemas/)`; the two build-envelope schemas (D6) are authored during M3 implementation rather than shipped as drafts. Implementation copies the draft files into `specify-cli/schemas/` and embeds all six in `specify-schema` as `SLICE_MODEL_JSON_SCHEMA`, `DRAFT_MODEL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, and `SYNTHESIS_JSON_SCHEMA`. `**model.schema.json`, `draft-model.schema.json`, and `synthesis.schema.json` MUST be registered together** so relative `$ref`s compile without a registry lookup (same discipline as the adapter loader's inlined `$defs`). Field names are kebab-case on disk; top-level shapes are closed (`additionalProperties: false`).


| Schema                  | RFC draft path                                                                    | `specify-cli` path                         | Embed constant              | Used by                                          |
| ----------------------- | --------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------- | ------------------------------------------------ |
| Slice model (persisted) | `[slice/model.schema.json](rfc-29/schemas/slice/model.schema.json)`               | `schemas/slice/model.schema.json`          | `SLICE_MODEL_JSON_SCHEMA`   | Post-projection `model.yaml`; D4; D6 build input |
| Draft model             | `[slice/draft-model.schema.json](rfc-29/schemas/slice/draft-model.schema.json)`   | `schemas/slice/draft-model.schema.json`    | `DRAFT_MODEL_JSON_SCHEMA`   | D3a agent response `model`                       |
| Build request           | *authored in M3 ([RFC-29d](rfc-29d-target.md))*                                   | `schemas/target/build-request.schema.json` | `BUILD_REQUEST_JSON_SCHEMA` | D6                                               |
| Build report            | *authored in M3 ([RFC-29d](rfc-29d-target.md))*                                   | `schemas/target/build-report.schema.json`  | `BUILD_REPORT_JSON_SCHEMA`  | D6                                               |
| Reconciliation envelope | `[discovery/proposal.schema.json](rfc-29/schemas/discovery/proposal.schema.json)` | `schemas/discovery/proposal.schema.json`   | `PROPOSAL_JSON_SCHEMA`      | D2 (request + response)                          |
| Synthesis               | `[slice/synthesis.schema.json](rfc-29/schemas/slice/synthesis.schema.json)`       | `schemas/slice/synthesis.schema.json`      | `SYNTHESIS_JSON_SCHEMA`     | D3, D10                                          |


All slice-model, build-request, and build-report schemas key on `(slice, target)` per D5 — none carries `outputs[]` or `output-id`. `proposal.schema.json` discriminates request vs response via closed `kind: request | response`: the request carries the lead inventory, the deterministic structural floor, and the registry `projects[]` topology; the response carries `(group-id, target)` rows (members with `match-basis`, optional `slice-name`, optional `tentative` flags, `depends-on` in derived slice names, and the bound `project`). `synthesis.schema.json` discriminates request vs response via closed `kind: request | response`; the response `model` `$ref`s `**draft-model.schema.json`**, not the persisted model (D3a).

### Journal events

The closed `Event` / `EventKind` taxonomy in `crates/workflow/src/journal.rs` gains the following kebab-case event kinds. Wire ids are normative; Rust variants follow the existing `#[serde(rename = …)]` pattern. Both deterministic commands and the D12 `specrun journal emit` verb write them through the one closed taxonomy.


| Event                                 | Milestone  | When                                                                                                                                                                   |
| ------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source.survey.cache-hit`             | M1         | Lead set was read from cache.                                                                                                                                          |
| `source.survey.cache-miss`            | M1         | Source-adapter `survey` ran.                                                                                                                                           |
| `source.execution.agent`              | M1         | A source-adapter operation ran in `agent` mode (`survey` or `extract`).                                                                                                |
| `plan.reconcile.agent`                | M2a        | The lead-matching step (D2) ran in `agent` mode (the default and designed centre). One event per `specrun plan propose --from` invocation.                             |
| `plan.reconcile.completed`            | M2a        | `specrun plan propose --from` validated the agent grouping, enforced the partition / structural-floor invariants, derived slice names, and wrote `plan.yaml.slices[]`. |
| `slice.extract.cache-hit`             | (existing) | Evidence was read from cache.                                                                                                                                          |
| `slice.extract.cache-miss`            | (existing) | Source-adapter `extract` ran.                                                                                                                                          |
| `slice.extract.completed`             | (existing) | Evidence file was successfully persisted.                                                                                                                              |
| `slice.synthesize.started`            | M2b        | `specrun slice synthesize` began for a slice.                                                                                                                          |
| `slice.synthesize.authority-resolved` | M2b        | The projection kernel resolved RFC-27 authority over `Evidence[]`. The synthesis envelope is about to be dispatched. No requirement skeleton is pre-computed.          |
| `slice.synthesize.agent`              | M2b        | The synthesis step ran in `agent` mode (the first-party default and designed centre). One event per invocation.                                                        |
| `slice.synthesize.completed`          | M2b        | `specrun slice synthesize` finished and all artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, `provenance.yaml`, `model.yaml`) validated and persisted.    |
| `slice.synthesize.failed`             | M2b        | `specrun slice synthesize` aborted; prior artifacts left intact where possible.                                                                                        |
| `slice.model.show.requested`          | M2b        | Operator invoked `specrun slice model show` (audit-only; useful for measuring model-consumer adoption).                                                                |
| `slice.build.started`                 | M3         | `/spec:build` (or `specrun slice build`) began work on a slice.                                                                                                        |
| `slice.build.succeeded`               | M3         | A slice's build report validated with `status: success`.                                                                                                               |
| `slice.build.failed`                  | M3         | A slice's build report carried `status: failure` or failed schema validation.                                                                                          |
| `slice.merge.started`                 | M3         | `/spec:merge` began work on a slice.                                                                                                                                   |
| `slice.merge.succeeded`               | M3         | A slice's merge report validated with `status: success`.                                                                                                               |
| `slice.merge.failed`                  | M3         | A slice's merge report carried `status: failure` or failed schema validation.                                                                                          |
| `target.execution.agent`              | M3         | A target-adapter operation ran in `agent` mode.                                                                                                                        |


The `specrun journal emit` verb (D12, [RFC-29a](rfc-29a-source.md)) is the guarded front door onto this taxonomy for agent-orchestrated phases (D2/D9/D10 agent paths, agent-driven build/merge). It adds no event kinds of its own.

### Validation outcomes and exit codes

RFC-29 adds **no new exit code**. `Exit::from(&Error)` in `src/runtime/output.rs` stays the single source of truth, and every failure below lands on the existing `EXIT_VALIDATION_FAILED = 2`. Adapter-resolution, sandbox-preopen, WASI-tool-runtime, and I/O failures keep the existing `EXIT_GENERIC_FAILURE = 1` mapping.

The kebab `code` strings below are a **closed, documented vocabulary**, not new `Error` enum variants. The `Error` enum stays small (`[crates/error/src/error.rs](https://github.com/augentic/specify-cli/blob/main/crates/error/src/error.rs)`); a condition is promoted to its own typed variant only if it needs a distinct exit code or structured payload (none here do). RFC-29 conditions reach exit 2 through two existing surfaces:

- `**Diagnostic` findings** (the [RFC-28](../done/rfc-28-standards-contract.md) substrate) — a stable `code` plus `severity`, `kind`, message, and location, emitted as a `DiagnosticReport` by the validate surface (`specrun slice validate`). A report carrying any blocking finding gates the transition at exit 2.
- `**Error::Validation { code, detail }`** — a single operational abort raised by a command that fails one specific check; `code` is the JSON `error` discriminant skills branch on.

The split is "a *set of findings over an artifact* (`Diagnostic`) vs a *single command abort* (`Error::Validation`)", not a new enum arm per condition. Both are exit 2; both keep a stable `code` that skills branch on and acceptance tests assert.

#### Validation findings (`Diagnostic` codes, validate surface)

Emitted by `specrun slice validate` as a `DiagnosticReport`; a report carrying any of these at blocking severity gates the slice transition at exit 2.


| Finding code                            | Milestone | Meaning                                                                                                                                                                                                                                       |
| --------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `slice-model-schema`                    | M2b       | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                                                                                                                                                |
| `slice-spec-provenance-stale`           | M2b       | Kernel-rendered provenance lines in `spec.md` disagree with projected `model.yaml` (operator hand-edit or stale render).                                                                                                                      |
| `slice-model-provenance-drift`          | M2b       | `model.yaml.requirements[].claims` disagrees with `provenance.yaml` at `(source, claim-id)` granularity.                                                                                                                                      |
| `slice-model-target-drift`              | M2b       | `model.yaml.target` (or `.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`.                                                                                                                                           |
| `slice-model-source-orphan`             | M2b       | A `claims[]` entry references a `(source, claim-id)` whose source key is absent from `model.yaml.sources[].key` or whose claim id is absent from that source's Evidence. Also raised as a `specrun slice synthesize` abort before projection. |
| `slice-model-cross-ref-orphan`          | M2b       | A `satisfies[]` `REQ-`* reference does not exist in `requirements[].id`.                                                                                                                                                                      |
| `slice-model-claim-kind-mismatch`       | M2b       | A `claims[]` entry's `kind` (D13) disagrees with the kind recorded for that `(source, claim-id)` in Evidence.                                                                                                                                 |
| `slice-model-id-grammar`                | M2b       | A REQ / TASK / DEC / TYP / OP / CFG / OBS id does not match its closed three-digit grammar.                                                                                                                                                   |
| `slice-synthesize-forbidden-input-leak` | M2b       | A synthesis response's requirements section demonstrably referenced `target` or `shape-brief` content (detected by fixture-local target-neutrality probes).                                                                                   |


#### Operational validation codes (`Error::Validation`, command aborts)

Raised as a single `Error::Validation { code }` by the named command; exit 2.


| Code                                         | Milestone | Command               | Cause                                                                                                                                                                                                     |
| -------------------------------------------- | --------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `slice-synthesize-kernel-field-usurped`      | M2b       | `slice synthesize`    | A synthesis draft set a kernel-owned field it does not author — top-level `generated-at` / `generator`, a `requirements[].id`, `requirements[].status`, `requirements[].sources`, or a `claims[].winner`. |
| `slice-synthesize-execution-mode-required`   | M2b       | `slice synthesize`    | A workspace declares `synthesize.execution: tool` without configuring `synthesize.tool: { name, version }`.                                                                                               |
| `plan-reconcile-lead-orphan`                 | M2a       | `plan propose --from` | The response cites a `(source-key, lead-id)` absent from the surveyed `discovery.md` lead inventory.                                                                                                      |
| `plan-reconcile-partition`                   | M2a       | `plan propose --from` | The response is not a well-formed **global** partition: a surveyed lead is unaccounted for, or a `(source-key, lead-id)` appears in more than one group.                                                  |
| `plan-reconcile-structural-floor-violated`   | M2a       | `plan propose --from` | The response splits a deterministic structural-floor group (exact id / alias / cross-reference).                                                                                                          |
| `plan-reconcile-project-binding-required`    | M2a       | `plan propose --from` | The response omits `project` on a row in workspace mode (request carried a non-empty `projects[]`), or sets `project` on a row in single-repo mode.                                                       |
| `plan-reconcile-project-orphan`              | M2a       | `plan propose --from` | The response binds a row to a `project` absent from `registry.yaml`.                                                                                                                                      |
| `plan-reconcile-project-target-mismatch`     | M2a       | `plan propose --from` | The response binds a row to a project whose `registry.yaml` target does not equal the row's `target`.                                                                                                     |
| `plan-propose-missing-grouping`              | M2a       | `plan propose`        | Invoked without `--dry-run` and without `--from`; one of the two modes is required.                                                                                                                       |
| `adapter-execution-mode-required`            | M1        | adapter load          | An adapter manifest does not declare `execution`.                                                                                                                                                         |
| `adapter-execution-agent-cache-conflict`     | M1        | adapter load          | An adapter manifest sets `execution: agent` together with any cache mode other than `opt-out`.                                                                                                            |
| `journal-emit-unknown-event`                 | M1        | `journal emit`        | An `<event-id>` that is not a member of the closed `EventKind` taxonomy.                                                                                                                                  |
| `journal-emit-payload-schema`                | M1        | `journal emit`        | A `--payload` that fails the named event kind's required-field shape.                                                                                                                                     |
| `target-build-request-schema`                | M3        | `slice build`         | A build request fails `schemas/target/build-request.schema.json`.                                                                                                                                         |
| `target-build-report-schema`                 | M3        | `slice build`         | A build report fails `schemas/target/build-report.schema.json`.                                                                                                                                           |
| `target-build-success-with-critical-finding` | M3        | `slice build`         | A build report sets `status: success` while carrying a finding at severity `critical`.                                                                                                                    |


### D13 evidence-schema claim-id requirement (cross-cutting)

`schemas/evidence.schema.json` requires `claim-id` on **every** claim kind, so every `(source, claim-id)` cited by a requirement resolves. The detail and read-path guarantees live in [RFC-29c §"Claim contract (D13)"](rfc-29c-synthesis.md). It is called out here because M1 source adapters emit `claim-id` on every claim from the start, keeping the milestones coherent with the M2b synthesis kernel.

## Sub-RFCs and milestone ordering

RFC-29 is large — it spans an executable source runner, an agent-led reconciliation engine, a synthesis kernel, a typed slice model, a build envelope, and several new verbs. It is **not** meant to land as one PR or even one branch. It is split into four **independently shippable milestones**, each a defensible release on its own and each its own numbered sub-RFC in the `rfc-29` family; this umbrella stays the source of truth for the contracts they share (§"Shared wire contracts").


| Milestone                     | Sub-RFC                              | Lands independently because…                                                                                                                                                                                               | Unblocks                                                         |
| ----------------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **M1 — Source operations**    | [RFC-29a](rfc-29a-source.md)         | `specrun source survey` / `extract` are useful the day they ship — they make `/spec:refine` extraction CLI-owned and give acceptance (RM-05) a durable seam — without depending on synthesis or build changes.             | RM-05 durable proof; M2 inputs.                                  |
| **M2a — Lead reconciliation** | [RFC-29b](rfc-29b-reconciliation.md) | `specrun plan propose` closes plan-time fan-in without synthesis or `model.yaml`.                                                                                                                                          | Plan-time fan-in contract; M2b plan rows.                        |
| **M2b — Slice synthesis**     | [RFC-29c](rfc-29c-synthesis.md)      | Slice synthesis, draft/persisted model split, kernel rendering into `spec.md`, and drift validators form one contract over Evidence the agent already produces; consumes M1's surveys/Evidence but not the build envelope. | RM-11 machine-readable producer/consumer impact; M3 build input. |
| **M3 — Target build**         | [RFC-29d](rfc-29d-target.md)         | The build request/report and the first-party targets consume `model.yaml` from M2b; the end-to-end D7 fixture is the final release gate that proves fan-in twice and fan-out once.                                         | RM-18 hosted execute; the RFC-29 acceptance proof.               |


Ordering is **M1 → M2a → M2b → M3** (each consumes the prior), but each milestone is reviewable, testable, and releasable on its own; M1 does not wait for M2 design to settle. The shared contracts that must stay stable across milestone boundaries are pinned in §"Shared wire contracts".

### Readiness (pre-implementation)


| Milestone                                      | Readiness                                                                                                                  |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **M1** ([RFC-29a](rfc-29a-source.md))          | **Ready.** D1/D9-source/D12 are well-specified; `source preview` reuse plan is sound.                                      |
| **M2a** ([RFC-29b](rfc-29b-reconciliation.md)) | **Ready.** D2 identity model, `slice-name` derivation, project-binding validation, and `targetRef` unification are pinned. |
| **M2b** ([RFC-29c](rfc-29c-synthesis.md))      | **Ready.** Draft vs persisted model split (D3a), rendering pipeline, and D13 read-path guarantee are pinned.               |
| **M3** ([RFC-29d](rfc-29d-target.md))          | **Deferred by design.** Build envelopes authored during M3 implementation; cross-project handoff is an open question (Q2). |


## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No graph database or global knowledge store for synthesis.
- No *kernel-side* heuristic auto-merge of semantically similar leads. Semantic cross-source matching is the agent's judgment under the D2 envelope (`match-basis: semantic`), surfaced for operator review at Gate 1; the CLI kernel never merges leads on textual similarity by itself, only validates the agent's grouping against the partition and structural-floor invariants.
- **No multi-target slices (D5).** Fan-out is plan-level via `depends-on`; reopening requires amending the decision log.
- No target-specific behavior in the projection kernel (D8).
- No deterministic requirement reconciliation (D3/D10). Kernel projection and envelope-construction proof only; see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target.md).
- No CLI adjudication of semantic value agreement. The `agreement` verdict is the agent's; the kernel applies authority to it but never re-decides whether two claim values mean the same thing. The advisory `slice-synthesize-agreement-suspect` finding is a non-blocking nudge, not a semantic judge.
- No commitment to per-target determinism on day one. RFC-29 commits only to a stable build envelope and validation contract; per-target determinism milestones are tracked in each target adapter's manifest and changelog.

## Open questions

**Q1. Cross-project artifact handoff (workspace mode).** When two fan-out slices bind *different projects*, the dependent slice cannot read the upstream output from a shared working tree — each project lives in its own `.specify/workspace/<project>/` slot on its own branch. 

By design there is **no** Specify-native cross-project reference: the dependent project consumes the upstream output as an ordinary published dependency (a versioned crate, npm package, or schema-registry entry) through its own manifest, exactly as it would any third-party dependency; plan-level `depends-on` + `plan next` ordering covers the same-tree case.

The open question is narrow — whether a future RFC ever needs more than this — and is undertaken only if a first-party target gains a concrete cross-project consuming dependency. 

RFC-29 is fully implementable without this question being resolved.

## References

- [RFC-29a: Executable Source Operations](rfc-29a-source.md) — M1
- [RFC-29b: Plan-Time Lead Reconciliation](rfc-29b-reconciliation.md) — M2a
- [RFC-29c: Slice Synthesis Engine and Typed Model](rfc-29c-synthesis.md) — M2b
- [RFC-29d: Target Build Envelope and Fan-Out Proof](rfc-29d-target.md) — M3
- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- [RFC-28: Engineering Standards — Codex Contract and Findings](../done/rfc-28-standards-contract.md)
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim reconciliation](../../plugins/spec/references/synthesis/claim-reconciliation.md)
- [Provenance index](../../plugins/spec/references/synthesis/provenance.md)

