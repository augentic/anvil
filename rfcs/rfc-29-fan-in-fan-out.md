# RFC-29: Fan-In/Fan-Out Code Contract

> Status: Draft (umbrella) — Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../done/rfc-28-standards-contract.md) — Enables: provable multi-source fan-in and plan-level multi-slice fan-out (D5)

This document is the **umbrella** for the RFC-29 family. It owns the abstract, the decision catalogue, the operator surface, and — most importantly — the **shared wire contracts** (the schemas, the closed `EventKind` taxonomy, and the closed validation-finding / `Error::Validation` code vocabulary) that the four implementation milestones must keep stable across their boundaries. The detailed mechanics of each decision live in the sub-RFC that ships it:


| Sub-RFC                                                                  | Milestone | Decisions                          |
| ------------------------------------------------------------------------ | --------- | ---------------------------------- |
| RFC-29a — Executable Source Operations — **shipped** ([durable spec in `specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1)) | **M1**    | D1, D9 (source side), D12          |
| RFC-29b — Plan-Time Lead Reconciliation — **shipped** ([durable spec in `specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2))     | **M2a**   | D2                                 |
| RFC-29c — Slice Synthesis Engine and Typed Model — **shipped** ([durable spec in `specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)) | **M2b**   | D3, D4, D5, D8, D10, D13 |
| [RFC-29d — Target Build Envelope and Fan-Out Proof](rfc-29d-target.md)   | **M3**    | D6, D9 (target side), D7           |


Ordering is **M1 → M2a → M2b → M3** (each consumes the prior), but each milestone is reviewable, testable, and releasable on its own. See §"Sub-RFCs and milestone ordering".

## Abstract

Specify's architectural promise is a fan-in / fan-out workflow:

- **Fan-in** happens twice per change. Multiple source adapters' `Lead`s fan in at plan time into the `slices[]` rows of `plan.yaml`. Multiple sources' `Evidence` fans in at slice time into one synthesized slice. Both are core's responsibility.
- **Fan-out** happens once per change, at the plan layer. One change decomposes into multiple slices — each slice binding exactly one target — joined by `depends-on` edges. The `refine -> build -> merge` loop runs per slice; baseline merge runs once per slice against one target's baseline.

This is the framework's "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project)). RFC-29 affirms it and does not extend the slice to multi-target.

The gap is that several load-bearing fan-in steps — survey, extract, and plan-time lead reconciliation — are still uncontracted agent discipline rather than agent judgment running under a CLI-owned envelope. Both lead reconciliation (plan time) and slice synthesis (slice time) stay agent-led, because both are cross-source judgment with no deterministic function; in each case the CLI owns the **envelope** and the **projection kernel** around that judgment (lead reconciliation shipped — durable spec in [`specify-cli` `DECISIONS.md` §"Lead reconciliation (D2)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2); slice synthesis in [RFC-29c](rfc-29c-synthesis.md)).

This RFC turns the fan-in promise into an end-to-end contract by adding:

1. **Executable source operations** (M1, **shipped** — durable spec in [`specify-cli` `DECISIONS.md` §"Source operations (D1)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1)) - first-class `specrun source survey` and `specrun source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Agent-led plan-time lead reconciliation** (M2a, **shipped** — durable spec in [`specify-cli` `DECISIONS.md` §"Lead reconciliation (D2)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)) - an agent-led cross-source matching step that emits `slices[]`, each carrying an explicit kebab-case `name`, its matched `sources[]` (at most one lead per source), and a bound `project` from the request (the kernel derives `target` from the bound project), running under a stable input/output envelope wrapped by a CLI-owned projection kernel: schema validation, the total lead-coverage invariant, slice-name validation, one journal event, and the existing plan writers. Cross-target fan-out is multiple slices joined by `depends-on` — there is no `scope` grouping noun (RFC-29 review F3). Cross-source matching is agent judgment; the operator curates at Gate 1 after propose.
3. **Slice synthesis engine** ([RFC-29c](rfc-29c-synthesis.md)) - an agent-led cross-modal synthesis step (which decides the requirement set, declares each requirement's `(source, id)` claims and an `agreement` verdict, and authors its prose) running under a stable input/output envelope, wrapped by a CLI-owned projection kernel that projects over the agent's structure: RFC-27 authority resolution, REQ-id assignment, rendered source lists, winner-marker derivation, status derivation, inline provenance written into the single `model.yaml` (and surfaced on demand as a `provenance` projection), and drift validators.
4. **Typed slice model** ([RFC-29c](rfc-29c-synthesis.md)) - a machine-readable, schema-pinned view of the slice emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
5. **Target build contract** ([RFC-29d](rfc-29d-target.md)) - target adapters consume the slice model through a stable per-slice build envelope, with per-slice validation, review findings, and merge gates.
6. **Proof fixtures** ([RFC-29d](rfc-29d-target.md)) - acceptance coverage that exercises `N sources -> one slice model -> 1 target per slice`, with cross-target fan-out proven across multiple slices joined by `depends-on`, and the kernel / envelope split corroborated by one **deterministic** non-blocking gate: kernel-projection determinism over a fixed synthesis response (no LLM judge in any gate). Target-neutrality is a by-construction property of the kernel rather than a separate cross-target fixture, since D5 binds one slice to one target.

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant: source operations are briefs not executable commands, plan-time lead reconciliation is uncontracted agent work (no envelope, no validation, no journal trail), slice-time reconciliation has no production resolver, the machine-readable slice view is implicit, and target codegen is adapter-brief discipline with no stable envelope. The normative decisions below close each gap.

The goal is not to remove agents but to wrap the agent's judgment in a stable envelope and to move stable workflow, data-shape, and bookkeeping obligations into the CLI. The two cross-source judgment steps — lead reconciliation (D2) and slice synthesis (D3/D10) — are both agent-led; see [`specify-cli` `DECISIONS.md` §"Lead reconciliation (D2)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2) and [RFC-29c](rfc-29c-synthesis.md) for the matching agent/kernel split.

## Normative decisions

The catalogue below is the canonical decision list. Each decision's full mechanics and implementation consequence live in the **Home** sub-RFC.


| ID                                     | Decision                                                                                                                                                                                                                                                                                               | Home                                                                |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| **D1 Source operation**                | The CLI runs source adapter `survey` and `extract` operations. `survey` writes one raw, unmerged lead per source to `discovery.md` (each identified by its `(source, lead)` pair) and does **not** merge across sources; cross-source unification is deferred to D2.                                                                                                                                                                                                                                         | [shipped — `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1)                                            |
| **D2 Lead reconciliation**             | Agent-led cross-source matching of `Lead[]` into `slices[]`, each carrying an explicit `name`, its matched `sources[]` (at most one lead per source), and a `project` from the request (auto-bound when only one project exists; the kernel derives `target` from the bound project), under a CLI-owned projection kernel (total lead-coverage invariant, same-source-fusion rejection, project-binding validation, slice-name validation, plan writers). No `scope` grouping noun — cross-target fan-out is multiple slices joined by `depends-on`. Operator curates at Gate 1 after propose. | [shipped — `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)                                    |
| **D3 Slice synthesis**                 | Agent-led cross-modal reconciliation of `Evidence[]` into the requirement set; the CLI assembles the agent step's inputs and owns the projection kernel (authority resolution, REQ-ids, status, rendered source lists, winner derivation, inline provenance, rendering). The agent-authored and persisted shapes are one schema (`model.schema.json`); the kernel re-derives its owned fields and ignores any the agent supplied (normalize, never reject).                                                         | [29c](rfc-29c-synthesis.md)                                         |
| **D4 Typed slice**                     | Every synthesized slice carries one structured artifact `.specify/slices/<slice>/model.yaml` carrying provenance inline; `provenance.yaml` is not a file but an on-demand projection (`specrun slice provenance`).                                                                                                                                                                                                                                  | [29c](rfc-29c-synthesis.md)                                         |
| **D5 Per-slice fan-out**               | Each slice binds exactly one target adapter / project; cross-target changes decompose at plan time into multiple slices joined by `depends-on`. No `outputs[]`.                                                                                                                                        | [29c](rfc-29c-synthesis.md)                                         |
| **D6 Target build**                    | Target adapters receive a stable per-slice build request and return a stable per-slice build report.                                                                                                                                                                                                   | [29d](rfc-29d-target.md)                                            |
| **D7 Acceptance proof**                | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together.                                                                                                                                                                                          | [29d](rfc-29d-target.md)                                            |
| **D8 Shape-brief scope**               | Target `shape` briefs parameterise non-requirements model sections only; never `requirements[]`, claims, agreement, `sources[]`, or any provenance-bearing field. Enforced by the kernel-determinism property (kernel output is byte-identical and target-independent given a fixed response), not by a runtime input-leak finding.                                                                                                                                      | [29c](rfc-29c-synthesis.md)                                         |
| **D9 Adapter execution**               | Source and target adapters declare a closed `execution: tool \| agent` field selecting deterministic dispatch vs an agent-run brief.                                                                                                                                                                                                                                           | [29a (source) shipped](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-execution-mode-d9) / [29d (target)](rfc-29d-target.md) |
| **D10 Synthesis dispatch**             | The synthesis step is always agent-dispatched (`cache: opt-out`); no deterministic tool path.                                                                                                                                                                                                                                                  | [29c](rfc-29c-synthesis.md)                                         |
| **D12 Journal emitter**                | `specrun journal emit` is the schema-validated writer for agent-orchestrated phases with no deterministic emit command.                                                                                                                                                                                | [shipped — `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#specrun-journal-emit--guarded-front-door-d12)                                            |
| **D13 Claim contract (`id` + `kind`)** | Every contributing claim carries a stable `id` and its `kind`; `evidence.schema.json` requires `id` on every claim kind.                                                                                                                                                                   | [29c](rfc-29c-synthesis.md)                                         |


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
specrun plan propose --dry-run --format json                 # D2 request envelope (flat lead catalog) # 29b
specrun plan propose --from grouping.json --format json      # D2 kernel: validate agent grouping, write slices         # 29b
specrun plan remove <entry>                                  # D2 Gate 1: drop a pending entry (replaceable plan only) # 29b
specrun slice synthesize identity-password-reset --format json                                 # 29c
specrun slice model show identity-password-reset --format json                                 # 29c
specrun slice provenance identity-password-reset --format json                                 # 29c (provenance projected from model.yaml)
specrun journal emit slice.synthesize.agent --payload '{"slice":"identity-password-reset"}'   # 29a (D12 emitter)
```

Cross-source lead matching and project binding are the `/spec:plan` agent step's judgment (D2); `specrun plan propose --dry-run` seeds it with a flat lead catalog, and `specrun plan propose --from` is the kernel that validates the grouping, binds each slice to a project (its target resolves on demand from that project), and writes the slices through the existing plan writers. The operator curates at Gate 1 before `approved`.

Cross-target changes are planned as multiple slices, each bound to one target, joined by `depends-on`:

```bash
specrun plan add identity-contracts \
  --sources docs=identity-api \
  --project identity-contracts

specrun plan add identity-service \
  --sources docs=identity-api,legacy=identity-api \
  --project identity-service \
  --depends-on identity-contracts
```

Each entry binds one project, which resolves to one target (see [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md)). In the default flow these rows are written by the D2 reconciliation kernel (`specrun plan propose --from`) projecting the agent's grouping through these same `plan add` writers; the explicit `plan add` form above stays available for manual authoring and illustrates the resulting plan shape.

A downstream slice that needs another slice's output (e.g. `omnia` consuming the `contracts` schema) declares the edge with `depends-on` at the plan layer; `specrun plan next` merges the upstream slice before the dependent starts, and the dependent target reads the upstream output from the merged working tree (see [RFC-29d §"Target build envelope (D6)"](rfc-29d-target.md)). No multi-output, multi-target shape is added to a single slice — the plan layer is the only place fan-out happens.

## Shared wire contracts

These contracts span milestone boundaries and are **pinned here** so a later milestone cannot silently redefine an earlier one's wire shape. Each sub-RFC names the subset it introduces and links back to these canonical tables. The shared contracts are: the five schemas in `[rfc-29/schemas/](rfc-29/schemas/)`, the closed `EventKind` additions, the closed validation-finding / `Error::Validation` code vocabulary, and the D13 `evidence.schema.json` `id` requirement.

### Schemas

Three JSON Schemas ship as draft files alongside this RFC under `[rfc-29/schemas/](rfc-29/schemas/)`; the two build-envelope schemas (D6) are authored during M3 implementation rather than shipped as drafts. Implementation copies the draft files into `specify-cli/schemas/` and embeds all five in `specify-schema` as `SLICE_MODEL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, and `SYNTHESIS_JSON_SCHEMA`. `**model.schema.json` and `synthesis.schema.json` MUST be registered together** so relative `$ref`s compile without a registry lookup (same discipline as the adapter loader's inlined `$defs`). A single `model.schema.json` validates both the agent's synthesis response `model` and the persisted `model.yaml`: kernel-owned fields (`requirements[].id`, `.status`, `claims[].winner`) are optional, so the agent omits them and the kernel re-derives them on projection (normalize, never reject). Field names are kebab-case on disk; top-level shapes are closed (`additionalProperties: false`).


| Schema                  | RFC draft path                                                                    | `specify-cli` path                         | Embed constant              | Used by                                          |
| ----------------------- | --------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------- | ------------------------------------------------ |
| Slice model             | `[slice/model.schema.json](rfc-29/schemas/slice/model.schema.json)`               | `schemas/slice/model.schema.json`          | `SLICE_MODEL_JSON_SCHEMA`   | Agent response `model` and persisted `model.yaml` (provenance inline); D3; D4; D6 build input |
| Build request           | *authored in M3 ([RFC-29d](rfc-29d-target.md))*                                   | `schemas/target/build-request.schema.json` | `BUILD_REQUEST_JSON_SCHEMA` | D6                                               |
| Build report            | *authored in M3 ([RFC-29d](rfc-29d-target.md))*                                   | `schemas/target/build-report.schema.json`  | `BUILD_REPORT_JSON_SCHEMA`  | D6                                               |
| Reconciliation envelope | `[discovery/proposal.schema.json](rfc-29/schemas/discovery/proposal.schema.json)` | `schemas/discovery/proposal.schema.json`   | `PROPOSAL_JSON_SCHEMA`      | D2 (request + response)                          |
| Synthesis               | `[slice/synthesis.schema.json](rfc-29/schemas/slice/synthesis.schema.json)`       | `schemas/slice/synthesis.schema.json`      | `SYNTHESIS_JSON_SCHEMA`     | D3, D10                                          |


All slice-model, build-request, and build-report schemas key on `(slice, target)` per D5 — none carries `outputs[]` or `output-id`. `proposal.schema.json` discriminates request vs response via closed `kind: request | response`: the request carries `leads[]` (one row per raw `(source, lead)` lead, with synopsis and optional `aliases[]` hints from `discovery.md`) and the `projects[]` topology (always at least one project; for a hub the entries are projected from the committed `.specify/topology.lock` derived from each member project's `project.yaml` per [RFC-36](rfc-36-registry-projection.md); a single regular project is synthesized from `project.yaml`; each project entry carries its normalized `target` adapter plus optional `capabilities[]` / `keywords[]` routing tags); the response carries a single `slices[]` list, each row carrying an explicit kebab-case `name`, its matched `sources[]` (members as `{ source, lead }`, at most one per source), an optional `rationale`, `depends-on` in slice names, and the bound `project` — optional when only one project exists, since the kernel auto-binds it; the kernel resolves each slice's target on demand from the bound project's `projects[].target` and does not write it to `plan.yaml`. There is no `scope` grouping noun (RFC-29 review F3): cross-target fan-out is multiple slices that may reference the same lead, joined by `depends-on`, and the kernel enforces total lead coverage (every surveyed lead referenced by at least one slice) rather than an exactly-once partition. `propose --from` re-reads current `discovery.md`, rebuilds the lead catalog, and may replace `plan.yaml.slices[]` only while the plan is still replaceable (`lifecycle: pending` and all existing entries `pending`). `synthesis.schema.json` validates the agent synthesis response (`kind: response`); synthesis is always agent-dispatched (D10), so there is no closed *request* wire shape — the CLI assembles the step's inputs directly. The response `model` `$ref`s `**model.schema.json`** (the single slice-model schema), relying on the kernel-owned fields being optional rather than a separate draft schema.

### Journal events

The closed `Event` / `EventKind` taxonomy in `crates/workflow/src/journal.rs` gains the following kebab-case event kinds. Wire ids are normative; Rust variants follow the existing `#[serde(rename = …)]` pattern. Both deterministic commands and the D12 `specrun journal emit` verb write them through the one closed taxonomy.


| Event                                 | Milestone  | When                                                                                                                                                                   |
| ------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source.survey.cache-hit`             | M1         | Lead set was read from cache.                                                                                                                                          |
| `source.survey.cache-miss`            | M1         | Source-adapter `survey` ran.                                                                                                                                           |
| `source.execution.agent`              | M1         | A source-adapter operation ran in `agent` mode (`survey` or `extract`).                                                                                                |
| `plan.reconcile.completed`            | M2a        | `specrun plan propose --from` validated the agent response and wrote `plan.yaml.slices[]`. One event per successful invocation; payload carries `plan-name`, `slice-count`, and `slice-names[]`. The skill does not call `specrun journal emit` for D2. (RFC-29 review F8 folded the former `plan.reconcile.agent` + `plan.reconcile.completed` pair into this single event.) |
| `slice.extract.cache-hit`             | (existing) | Evidence was read from cache.                                                                                                                                          |
| `slice.extract.cache-miss`            | (existing) | Source-adapter `extract` ran.                                                                                                                                          |
| `slice.extract.completed`             | (existing) | Evidence file was successfully persisted.                                                                                                                              |
| `slice.synthesize.started`            | M2b        | `specrun slice synthesize` began for a slice.                                                                                                                          |
| `slice.synthesize.agent`              | M2b        | The synthesis step was dispatched to the operator's agent. One event per invocation. (Authority is resolved by the kernel after the response returns, as part of projection — there is no separate pre-dispatch authority event.)                                                        |
| `slice.synthesize.completed`          | M2b        | `specrun slice synthesize` finished and all artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, `model.yaml`) validated and persisted. Provenance is carried inline in `model.yaml`, not a separate file. |
| `slice.synthesize.failed`             | M2b        | `specrun slice synthesize` aborted; prior artifacts left intact where possible.                                                                                        |
| `slice.build.started`                 | M3         | `/spec:build` (or `specrun slice build`) began work on a slice.                                                                                                        |
| `slice.build.succeeded`               | M3         | A slice's build report validated with `status: success`.                                                                                                               |
| `slice.build.failed`                  | M3         | A slice's build report carried `status: failure` or failed schema validation.                                                                                          |
| `slice.merge.started`                 | M3         | `/spec:merge` began work on a slice.                                                                                                                                   |
| `slice.merge.succeeded`               | M3         | A slice's merge report validated with `status: success`.                                                                                                               |
| `slice.merge.failed`                  | M3         | A slice's merge report carried `status: failure` or failed schema validation.                                                                                          |
| `slice.archive.created`               | M3         | A merged or dropped slice directory was archived. Payload carries `slice`, `touched-specs`, the merge `outcome` summary, and the merge commit SHA — a durable append-only outcome-ledger entry that records the slice's fate even after its archived folder is gone. |
| `target.execution.agent`              | M3         | A target-adapter operation ran in `agent` mode.                                                                                                                        |


The `specrun journal emit` verb (D12, shipped — [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#specrun-journal-emit--guarded-front-door-d12)) is the guarded front door onto this taxonomy for agent-orchestrated phases (D2/D9/D10 agent paths, agent-driven build/merge). It adds no event kinds of its own.

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
| `slice-model-target-drift`              | M2b       | `model.yaml.project` disagrees with `plan.yaml.slices[<slice>].project`. (`target` is not persisted in `model.yaml`, so there is no target-vs-resolved-target half.)                                                                                                                                           |
| `slice-model-source-orphan`             | M2b       | A `claims[]` entry references a `(source, id)` whose source key is absent from the slice binding / Evidence map, or whose claim id is absent from that source's Evidence. Also raised as a `specrun slice synthesize` abort before projection. |
| `slice-model-cross-ref-orphan`          | M2b       | A `satisfies[]` `REQ-`* reference does not exist in `requirements[].id`.                                                                                                                                                                      |
| `slice-model-claim-kind-mismatch`       | M2b       | A `claims[]` entry's `kind` (D13) disagrees with the kind recorded for that `(source, id)` in Evidence.                                                                                                                                 |
| `slice-model-id-grammar`                | M2b       | A REQ or TASK id does not match its closed three-digit grammar. (The DEC / TYP / OP / CFG / OBS grammars are deferred with their sub-trees per RFC-29c §"ID grammar".)                                                                                                                                                   |


#### Operational validation codes (`Error::Validation`, command aborts)

Raised as a single `Error::Validation { code }` by the named command; exit 2.


| Code                                         | Milestone | Command               | Cause                                                                                                                                                                                                     |
| -------------------------------------------- | --------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `plan-reconcile-lead-orphan`                 | M2a       | `plan propose --from` | The response cites a `(source, lead)` pair absent from the catalog recomputed from current `discovery.md`.                                                                                              |
| `plan-reconcile-empty-catalog`               | M2a       | `plan propose --dry-run` / `--from` | `discovery.md` has no leads — the catalog is empty.                                                                                              |
| `plan-reconcile-partition`                   | M2a       | `plan propose --from` | The response does not achieve **total lead coverage**: a surveyed `(source, lead)` pair is referenced by no slice. (A lead referenced by more than one slice is legal fan-out, not a failure.)                                                               |
| `plan-reconcile-slice-source-collision`       | M2a       | `plan propose --from` | A slice names two leads from the same source (its `sources[]` carries the same `source` twice). Cross-source matching never fuses one source's candidate slices.                                                               |
| `plan-reconcile-slice-name-invalid`           | M2a       | `plan propose --from` | A slice `name` is not kebab-case (normally caught as `proposal-schema` at the wire gate first).                                                                                                   |
| `plan-reconcile-slice-name-collision`         | M2a       | `plan propose --from` | Two response slices resolve to the same `name`. With `scope` removed (RFC-29 review F3) name uniqueness is the sole duplicate gate.                                        |
| `plan-reconcile-depends-on-cycle`             | M2a       | `plan propose --from` | The response's `depends-on` graph contains a cycle (same detection as `cycle-in-depends-on`).                                                                                               |
| `plan-reconcile-project-binding-required`    | M2a       | `plan propose --from` | A slice omits `project` while the request's `projects[]` offers more than one project, so the kernel cannot unambiguously auto-bind a single project.                               |
| `plan-reconcile-project-orphan`              | M2a       | `plan propose --from` | The response binds a slice to a `project` absent from the request's `projects[]`.                                                                                                                          |
| `plan-reconcile-plan-not-replaceable`         | M2a       | `plan propose --from` | The command would replace slices on a plan that is already approved or has any non-pending entry.                                                                                              |
| `plan-propose-mode-required`                 | M2a       | `plan propose`        | Invoked without `--dry-run` and without `--from`; exactly one of the two modes is required (passing both is rejected by the argument parser).                                                              |
| `plan-remove-plan-not-replaceable`           | M2a       | `plan remove`         | The plan is already approved or has any non-pending entry.                                                                                              |
| `plan-remove-entry-referenced`               | M2a       | `plan remove`         | Another entry lists the target in `depends-on`.                                                                                              |
| `adapter-execution-mode-required`            | M1        | adapter load          | An adapter manifest does not declare `execution`.                                                                                                                                                         |
| `adapter-execution-agent-cache-conflict`     | M1        | adapter load          | An adapter manifest sets `execution: agent` together with any cache mode other than `opt-out`.                                                                                                            |
| `journal-emit-unknown-event`                 | M1        | `journal emit`        | An `<event-id>` that is not a member of the closed `EventKind` taxonomy.                                                                                                                                  |
| `journal-emit-payload-schema`                | M1        | `journal emit`        | A `--payload` that fails the named event kind's required-field shape.                                                                                                                                     |
| `target-build-request-schema`                | M3        | `slice build`         | A build request fails `schemas/target/build-request.schema.json`.                                                                                                                                         |
| `target-build-report-schema`                 | M3        | `slice build`         | A build report fails `schemas/target/build-report.schema.json`.                                                                                                                                           |
| `target-build-success-with-critical-finding` | M3        | `slice build`         | A build report sets `status: success` while carrying a finding at severity `critical`.                                                                                                                    |


### D13 evidence-schema id requirement (cross-cutting)

`schemas/evidence.schema.json` requires `id` on **every** claim kind, so every `(source, id)` cited by a requirement resolves. The detail and read-path guarantees live in [RFC-29c §"Claim contract (D13)"](rfc-29c-synthesis.md). It is called out here because M1 source adapters emit `id` on every claim from the start, keeping the milestones coherent with the M2b synthesis kernel.

## Sub-RFCs and milestone ordering

RFC-29 is large — it spans an executable source runner, an agent-led reconciliation engine, a synthesis kernel, a typed slice model, a build envelope, and several new verbs. It is **not** meant to land as one PR or even one branch. It is split into four **independently shippable milestones**, each a defensible release on its own and each its own numbered sub-RFC in the `rfc-29` family; this umbrella stays the source of truth for the contracts they share (§"Shared wire contracts").


| Milestone                     | Sub-RFC                              | Lands independently because…                                                                                                                                                                                               | Unblocks                                                         |
| ----------------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **M1 — Source operations**    | **shipped** ([`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1), [workflow §"Source adapter contract"](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md))         | `specrun source survey` / `extract` are useful the day they ship — they make `/spec:refine` extraction CLI-owned and give acceptance (RM-05) a durable seam — without depending on synthesis or build changes.             | RM-05 durable proof; M2 inputs.                                  |
| **M2a — Lead reconciliation** | **shipped** ([`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)) | `specrun plan propose` closes plan-time fan-in without synthesis or `model.yaml`.                                                                                                                                          | Plan-time fan-in contract; M2b plan rows.                        |
| **M2b — Slice synthesis**     | **shipped** ([`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)) | Slice synthesis, the single-schema normalize-not-reject kernel, kernel rendering into `spec.md`, and drift validators form one contract over Evidence the agent already produces; consumes M1's surveys/Evidence but not the build envelope. | RM-11 machine-readable producer/consumer impact; M3 build input. |
| **M3 — Target build**         | [RFC-29d](rfc-29d-target.md)         | The build request/report and the first-party targets consume `model.yaml` from M2b; the end-to-end D7 fixture is the final release gate that proves fan-in twice and fan-out once.                                         | RM-18 hosted execute; the RFC-29 acceptance proof.               |


Ordering is **M1 → M2a → M2b → M3** (each consumes the prior), but each milestone is reviewable, testable, and releasable on its own; M1 does not wait for M2 design to settle. The shared contracts that must stay stable across milestone boundaries are pinned in §"Shared wire contracts".

### Readiness (pre-implementation)


| Milestone                                      | Readiness                                                                                                                  |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **M1** (shipped — [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1))          | **Shipped.** D1/D9-source/D12 landed across both repos; durable spec lives in `DECISIONS.md` (§"Source operations (D1)", §"Adapter execution mode (D9)", §"`specrun journal emit` — guarded front door (D12)") and `docs/standards/workflow.md`.                                      |
| **M2a** (shipped — [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)) | **Shipped.** D2 lead-partition kernel, slice-name derivation, project-binding validation, and the `propose --dry-run`/`--from` envelope landed across both repos; durable spec lives in `DECISIONS.md` §"Lead reconciliation (D2)" and `docs/standards/workflow.md`. |
| **M2b** (shipped — [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)) | **Shipped.** The `specrun slice synthesize` two-phase command, the single-schema normalize-not-reject projection kernel, the rendering pipeline, the seven drift validators, `slice model show`, the `slice.synthesize.*` journal quartet, and the D13 read-path guarantee landed across both repos; durable spec lives in `DECISIONS.md` §"Slice synthesis engine (RFC-29 M2b)", §"Single slice-model artifact (RFC-29 M2b simplification)", and §"Authority: document-level plus one override (v1)", plus `docs/standards/workflow.md` §"Slice synthesis (RFC-29 M2b)". |
| **M3** ([RFC-29d](rfc-29d-target.md))          | **Deferred by design.** Build envelopes authored during M3 implementation; cross-project handoff is an open question (Q2). |


## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No *read-write authoritative* knowledge store for synthesis — no graph database or global store that the synthesis step writes back to and later trusts as a source of requirements. Such a store would make a slice's output depend on slice order and prior runs (breaking the re-derive-in-isolation review model and kernel determinism), muddy provenance (a requirement could be justified by "the store said so" rather than by Evidence), and propagate one slice's reconciliation error into every later slice. It also re-introduces the rejected kernel-side auto-merge of leads below. The *advisory, read-only* retrieval variant is a deferred open question (Q2), not a non-goal.
- No *kernel-side* auto-merge of leads — cross-source matching is the agent's judgment under the D2 envelope, surfaced for operator review at Gate 1; the CLI kernel never merges leads on slug overlap, alias intersection, or textual similarity. It validates the agent's grouping against the partition invariant only.
- **No multi-target slices (D5).** Fan-out is plan-level via `depends-on`; reopening requires amending the decision log.
- No target-specific behavior in the projection kernel (D8).
- No deterministic requirement reconciliation (D3/D10). Kernel-projection determinism only; see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target.md).
- No CLI adjudication of semantic value agreement. The `agreement` verdict is the agent's; the kernel applies authority to it but never re-decides whether two claim values mean the same thing.
- No commitment to per-target determinism on day one. RFC-29 commits only to a stable build envelope and validation contract; per-target determinism milestones are tracked in each target adapter's manifest and changelog.

## Open questions

**Q1. Cross-project artifact handoff (workspace mode).** When two fan-out slices bind *different projects*, the dependent slice cannot read the upstream output from a shared working tree — each project lives in its own `.specify/workspace/<project>/` slot on its own branch. 

By design there is **no** Specify-native cross-project reference: the dependent project consumes the upstream output as an ordinary published dependency (a versioned crate, npm package, or schema-registry entry) through its own manifest, exactly as it would any third-party dependency; plan-level `depends-on` + `plan next` ordering covers the same-tree case.

The open question is narrow — whether a future RFC ever needs more than this — and is undertaken only if a first-party target gains a concrete cross-project consuming dependency. 

RFC-29 is fully implementable without this question being resolved.

**Q2. Advisory retrieval-augmented synthesis (read-only).** Synthesis today is hermetic and amnesiac: each slice reconciles from only its own `Evidence[]`, so the agent has no cross-slice memory of prior requirement wording, settled conflicts, or house terminology. A future RFC may surface a **read-only, advisory** cross-slice index to the synthesis agent as context/hints to improve consistency — explicitly *not* the read-write authoritative store ruled out in Non-goals. The hard constraints any such design must keep: Evidence stays the sole producer of requirements; the index never originates a requirement and never appears in provenance; and kernel projection stays pure (the index feeds only the already-nondeterministic agent step, so no determinism guarantee is lost).

The corpus is buildable from artifacts the framework already keeps — baseline `spec.md`, archived per-slice provenance, and the append-only outcome ledger (the `slice.archive.created` journal entries) — so the ledger is the natural first brick, with no graph database or writeback protocol required. A cheap intermediate that needs no index at all: when a slice touches a unit already present in the baseline, feed that existing `spec.md` into the synthesis step as read-only context (one optional, non-provenance-bearing input). That step is the litmus test for whether the fuller index earns its keep.

RFC-29 is fully implementable without this question being resolved.

## Relationship to RFC-35

[RFC-35 (synthesis determinism)](rfc-35-synthesis-determinism.md) is the stepping stone that lands first: a set of small, additive, deterministic CLI surfaces — `briefs-dir` on `specrun source resolve` / `target resolve` output, and the determinism scaffolding the agent-driven loop already needs — that RFC-29 then reuses rather than re-invents. RFC-29a's `survey` / `extract` runners locate brief bodies through RFC-35's `briefs-dir` field (D1), and RFC-29c's synthesis kernel reuses the same brief-resolution surface.

The one place the two RFCs diverge is `specrun journal emit`. RFC-35 considered a guarded journal-emit verb and **deferred** it: at that stage every workflow event had a deterministic command that owned its own emission, so a general-purpose agent-facing emitter had no caller that a deterministic command could not already serve, and adding one risked a second emission path drifting from the closed taxonomy. RFC-29 changes that calculus — D2/D9/D10's agent-dispatched phases (and agent-driven build/merge) are workflow steps with **no** deterministic command to emit on their behalf — so RFC-29 adds `specrun journal emit` (D12) as a single guarded front door onto the *same* closed `EventKind` taxonomy, adding no event kinds of its own and keeping "one taxonomy, one writer." See [RFC-29a §"Journal emitter (D12)"](rfc-29a-source.md#journal-emitter-d12) for the emitter mechanics.

## References

- RFC-29a: Executable Source Operations — M1, **shipped**; durable spec in [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#source-operations-d1) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md)
- RFC-29b: Plan-Time Lead Reconciliation — M2a, **shipped**; durable spec in [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md)
- [RFC-29c: Slice Synthesis Engine and Typed Model](rfc-29c-synthesis.md) — M2b
- [RFC-29d: Target Build Envelope and Fan-Out Proof](rfc-29d-target.md) — M3
- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- [RFC-28: Engineering Standards — Codex Contract and Findings](../done/rfc-28-standards-contract.md)
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim reconciliation](../../plugins/spec/references/synthesis/claim-reconciliation.md)
- [Provenance index](../../plugins/spec/references/synthesis/provenance.md)

