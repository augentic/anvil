# Workflow contract

The in-force contract this binary implements. Stable anchors that source code and adapter prompts cite by `§`-name. This document is the live anchor surface for workflow behavior.

## Adapter vocabulary

Two adapter roles — `source` (operations: `survey`, `extract`) and `target` (operations: `guidance`, `build`, `merge`). An adapter ships as a single WebAssembly component exporting its axis interface from the WIT contract (one component, no manifest). See the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/specify/blob/main/AGENTS.md#vocabulary).

## Adapter implementation shape

An adapter is one `.wasm` component. Identity (`name`, `version`) is carried by the artifact's location — the store entry filename for a pinned identity, the guest crate's `Cargo.toml` version at publish time. Non-identity metadata — an optional `specify` compatibility floor and (targets) an optional `platforms` capability and `inputs[]` list — is the WIT `metadata` record returned by the component's deterministic `metadata` export, dispatched host-side at resolve time and cached against the component's digest. The closed operation set derives from the binding axis per the WIT contract; operation behaviour (prompts included) is compiled into the component. Implementation: [`crates/workflow/src/adapter/`](../../crates/workflow/src/adapter); the WIT contract at [`wit/specify.wit`](../../wit/specify.wit).

## Source adapter contract

The WIT `source` interface; the closed operation set `{extract, survey}` derives from the WIT contract. `survey` writes `## Lead inventory` blocks under `discovery.md` at plan time; `extract` writes one Evidence document per `(source, lead)` pair at slice time. See [`schemas/evidence.schema.json`](../../schemas/evidence.schema.json).

`specify source survey <source> [--plan <name>]` and `specify source extract <source> <lead> --slice <slice>` are the guest-routed runners (`orchestrate::survey` / `orchestrate::extract`). `<source>` resolves against `plan.yaml.sources.<key>`, then the adapter from `SourceBinding.adapter`. Both validate before the write becomes visible (lead set against `schemas/discovery/lead.schema.json` then `discovery.md` merge; Evidence against `schemas/evidence.schema.json` then persist to `.specify/slices/<slice>/evidence/<source>.yaml`). Source operations are agent-only, collapsed into one guest orchestration each — the judgment leg runs against the adapter guest's compiled-in prompt. Value-bound sources (`intent`) carry `value-inline`; path bindings carry `source-path`.

## Target adapter contract

The WIT `target` interface; the closed operation set `{guidance, build, merge}` derives from the WIT contract. `guidance` is read by core synthesis; `build` and `merge` are agent-driven. The optional `inputs[]` in the target's `metadata` answer (a flat `{ path, required }` list, paths relative to the build request's `inputs.root`) declares the target-specific build inputs the CLI assembles into `inputs.artifacts.additional[]`.

`specify slice build <slice> [--format json]` is the guest-routed target build runner (`orchestrate::build`). It is the symmetric target-side twin of `specify source survey` / `extract`: the orchestrator owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate, while the bound target's `build` prompt (compiled into the adapter guest) owns only code generation. It resolves the target from the slice's bound project — `plan.yaml` stores the slice's `project`, not a resolved `target`. The orchestration assembles + schema-validates the request, writes `.specify/slices/<slice>/build/request.yaml`, emits `target.execution.agent`, drives the adapter guest's `build` judgment leg (any build prelude, e.g. vectis asset materialization, is in-guest adapter code), validates the resulting `build/report.yaml`, rejects a `success` report carrying a blocking finding, gates the `Refined → Built` transition, and journals `slice.build.succeeded` / `slice.build.failed`.

Both build envelopes are closed-shape YAML, keyed on `(slice, target)`, schema-validated by the CLI: the request (`schemas/target/build-request.schema.json`, `BUILD_REQUEST_JSON_SCHEMA`) carries `{ version, slice, project-dir, inputs: { root, artifacts: { proposal, design, tasks, specs[], additional[] } } }` and omits `target` / `execution` / prompt paths / `model.yaml` (audit input, not a build input); `inputs.root` (slice tree) and `project-dir` (working tree) are distinct. A missing `required` adapter-declared input raises `target-build-input-missing`. The report (`schemas/target/build-report.schema.json`, `BUILD_REPORT_JSON_SCHEMA`) carries `{ version, slice, target, status: success|failure, findings[] }` — `findings[]` `$ref` the canonical diagnostic schema and default `[]`; a `success` report with any blocking finding is rejected (`target-build-success-with-blocking-finding`). The four pinned `target-build-*` codes are `Error::Validation` outcomes (exit 2), not new enum arms.

`merge` requires lifecycle `built` and re-runs target-specific validators per the merge prompt. v1 adds **no** merge envelope: `specify slice merge` is the writer, `slice.merge.started` / `.succeeded` / `.failed` fire on its validator outcome (not on a merge report), and the durable record stays `slice.archive.created`. A future merge-findings need reuses the build-report shape as `build/merge-report.yaml`. Build outputs are not cached.

## Resolver and cache

`adapter::Resolver` is the provider capability used by operations and kernels. The shipped WASI provider delegates it to `adapter::resolver::Component`, whose per-axis methods resolve the identity to exactly one `.wasm` component. A pinned `(name, version)` resolves only the global single-file store entry `<store-root>/<name>@<version>.wasm` (D4 verify-on-read against the recorded byte digest). A bare name resolves the development probes, in order:

1. `<project-cache>/components/<name>.wasm` — the project component cache (an operator-supplied local component mirrored at init).
2. `target/wasm32-wasip2/release/<name>.wasm` under the project, then under the sibling `specify-adapters` checkout — live development release builds (`cargo make release` in the adapters repo).

Resolution is project-local plus the global store; there is no environment-variable fallback to an out-of-tree framework checkout. When no probe matches, resolution fails with `adapter-not-found`, naming every probed path.

In workspace mode, slot setup is operator-owned. Each materialized slot must independently satisfy adapter resolution; Specify does not mirror the workspace's component cache into slots.

`specify init <adapter>` accepts a package reference (`specify:<name>@<semver>` — installed into the store on fetch), the first-party **shorthand** (`omnia@1.0.0` is package-reference sugar; bare `omnia` resolves the development release build), or a local `.wasm` path. GitHub URLs are refused (`adapter-github-uri-unsupported`).

The `source resolve` / `target resolve` JSON envelope carries `axis`, `name`, `version`, `resolved-path`, `location`, and `operations`. `location` and `resolved-path` project the resolver's opaque `Origin`; the component provider emits `store` / `dev`, while other providers may use another mechanism label. It is diagnostic: operation prompts are compiled into each adapter's deployment, so no engine code resolves prompt files at run time.

## Adapter name uniqueness

Adapter names remain unique across axes — one component exports exactly one axis interface, and a name identifies one published package. A component bound on the wrong axis fails resolve with the typed `adapter-axis-mismatch` (the metadata dispatch verifies the expected axis export before the call).

## Discovery handshake

`survey` writes `## Lead inventory` blocks — one **raw, unmerged** lead per source, each identified by its `(source, lead)` pair (`survey` stamps `source` from the surveyed source). A re-survey of one source replaces only that source's blocks by `(source, lead)`; the same `lead` may appear under different source keys. The operator stamps `approved`; `extract` resolves `slices[].sources[].lead` against the canonical `lead` id within the binding's `source`. Cross-source unification is deferred to plan-time reconciliation. Schema at [`schemas/discovery/lead.schema.json`](../../schemas/discovery/lead.schema.json); parser at [`crates/artifacts/src/discovery/document.rs`](../../crates/artifacts/src/discovery/document.rs).

## The Plan

`plan.yaml` shape is fixed by [`schemas/plan/plan.schema.json`](../../schemas/plan/plan.schema.json). Two stored lifecycle states (`pending | approved`); per-entry status is `pending | in-progress | done`. Writer ownership is split — see §"Writer ownership" below.

## Workflow vocabulary

`Slice`, `Lead`, `Evidence`, `Source`, `Target`, `Plan`, `Discovery`. Definitions live in the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/specify/blob/main/AGENTS.md#vocabulary).

## Plan-time reconciliation

The guest `plan author` orchestration reconciles surveyed leads across sources at plan time and writes the `plan.yaml.slices[]` rows through the `Plan::propose_from` projection kernel. The kernel schema-gates the judgment response (`PROPOSAL_JSON_SCHEMA` at [`schemas/discovery/proposal.schema.json`](../../schemas/discovery/proposal.schema.json), kebab wire fields, closed `kind: request | response`), re-reads `discovery.md`, validates total lead coverage (at most one lead per source, fan-out `sources[]` consistency), binds each slice's explicit `name` and `project` (the target adapter is resolved on demand from that project, never written to `plan.yaml`), and replaces `slices[]` only on a replaceable plan (`lifecycle: pending` and every entry `pending`). Cross-source matching is agent judgment; the operator curates at Gate 1. The closed `plan-reconcile-*` codes are `Error::Validation` outcomes (exit 2). See [`crates/workflow/src/change/plan/core/propose.rs`](../../crates/workflow/src/change/plan/core/propose.rs).

The closed `Divergence` enum (`none | likely | accepted | rejected`) records a reconciliation outcome's confidence. See [`crates/workflow/src/change/plan/core/model.rs`](../../crates/workflow/src/change/plan/core/model.rs).

## Source

`plan.yaml.sources.<key>` is the structured `{ adapter, path?, value? }` object with exactly one of `path` / `value`.

`Slice.sources` (a slice's per-source binding list) accepts the bare-string shorthand on parse and serialises as the structured `{ source, lead }`.

## Authority hierarchy

Closed enum `intent > documentation > behaviour`. v1 resolution order per `(source, kind)`: per-slice `authority-override` → Evidence document-level `authority:` → tie at the top class is a `conflict` (the per-Evidence per-kind override is deferred — see §"D2 — Per-kind authority on Evidence (deferred)"). The kernel resolves authority **after** the synthesis response returns and projects winners/`status` from it (§"Slice synthesis"). Closed enums at [`crates/artifacts/src/evidence/authority.rs`](../../crates/artifacts/src/evidence/authority.rs); the production resolver at [`crates/workflow/src/slice/synthesis/authority.rs`](../../crates/workflow/src/slice/synthesis/authority.rs).

## Execution model

`pending → approved` plan-level (Gate 1; operator-only). Per-entry: `pending → in-progress → done`. `done` is absorbing in v1; the operator-reversed flow lives behind `specify plan transition --undo`.

## Refinement

The guest `slice refine` orchestration runs `extract` per bound source and drives the synthesis kernel (§"Slice synthesis") to produce `proposal.md` / `spec.md` / `design.md` / `tasks.md` / `model.yaml` (provenance is carried inline in the single `model.yaml` artifact, projected on demand by `specify slice provenance`), and transitions the slice to `refined`. Validators live in [`crates/artifacts/src/validate/`](../../crates/artifacts/src/validate/) and [`crates/workflow/src/slice/handlers/validate.rs`](../../crates/workflow/src/slice/handlers/validate.rs).

## Slice synthesis

The synthesis engine turns a slice's `Evidence[]` into its requirement set, the single `model.yaml`, and the rendered Markdown artifacts. It runs as the judgment leg inside the guest `slice refine` / `plan execute` orchestrations:

- The **inputs** leg is read-only: it reads each bound source's inline `lead` + `claims` from `evidence/<source>.yaml` and the resolved target guidance body (wire field `guidance-brief`), then hands the agent the **inputs** envelope (`kind: inputs`). Authority is **not** included. It writes nothing and emits `slice.synthesize.agent` (synthesis is always agent-dispatched — no tool path, no closed *request* wire shape).
- The **persist** tail is the only writer: it schema-gates the judgment response against `synthesis.schema.json` (`kind: response`, code `synthesis-schema`), resolves authority from on-disk Evidence + per-slice `authority-override`, runs the projection kernel, renders provenance lines into `specs/<domain>/spec.md`, drift-validates, then atomically/staged-persists `proposal.md` / `specs/<domain>/spec.md` / `design.md` / `tasks.md` / `model.yaml` (prior artifacts intact on failure). It emits `slice.synthesize.started` then `slice.synthesize.completed` (or `slice.synthesize.failed`). No `provenance.yaml` is ever written.

**Kernel ownership (normalize, never reject).** The agent authors per-requirement `claims[]` `(source, id, kind)`, an `agreement` verdict, prose (`title` / `statement` / `scenarios` / `notes`), the owning `domain`, the agent-authored `tasks[]` with `TASK` ids, and prose-only spec bodies (no `ID:` / `Sources:` / `Status:` lines). The kernel owns and re-derives the `version` / `slice` / `project` header, `REQ-NNN` ids (declaration order, no holes), `status`, per-claim `winner` markers, the rendered `sources` lists (highest authority first), and the inline provenance; any agent-supplied `id` / `status` / `winner` / `sources` is ignored and recomputed. Modules at [`crates/workflow/src/slice/synthesis/`](../../crates/workflow/src/slice/synthesis). Schema gate at [`crates/workflow/src/schema_gate.rs`](../../crates/workflow/src/schema_gate.rs) (`validate_synthesis_json`); `model.schema.json` and `synthesis.schema.json` are registered together through a `jsonschema::Registry` so the relative `model` `$ref` resolves. `specify slice model show <slice> [--format json]` is the read-only model viewer.

**Drift validators.** `specify slice validate` adds seven blocking typed-model findings (exit 2), emitted as `Diagnostic` findings on the `DiagnosticReport` surface:

| Finding                           | Meaning                                                                                                                           |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `slice-model-schema`              | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                                    |
| `slice-spec-provenance-stale`     | Kernel-rendered provenance lines in `spec.md` disagree with `model.yaml`.                                                         |
| `slice-model-target-drift`        | `model.yaml.project` disagrees with `plan.yaml.slices[<slice>].project`. (`target` is not persisted, so there is no target half.) |
| `slice-model-source-orphan`       | A claim references an absent source key or Evidence claim id.                                                                     |
| `slice-model-cross-ref-orphan`    | A `satisfies[]` `REQ-*` reference is missing from `requirements[].id`.                                                            |
| `slice-model-claim-kind-mismatch` | A claim `kind` (D13) disagrees with the Evidence kind for that `(source, id)`.                                                    |
| `slice-model-id-grammar`          | A `REQ` or `TASK` id does not match its closed three-digit grammar.                                                               |

## Extraction

Per-source `extract` is agent-executed and never memoized: agent outputs are non-deterministic, so every run re-extracts. The validated Evidence at `.specify/slices/<slice>/evidence/<source>.yaml` is the only persisted result.

## Requirement block contract

`spec.md` requirements carry `ID:` / `Sources:` / `Status:` metadata; the closed `RequirementStatus` enum is `agreed | unknown | conflict | divergence`. Parser at [`crates/artifacts/src/spec/provenance.rs`](../../crates/artifacts/src/spec/provenance.rs).

## Wire format

Kebab-case discriminants on the JSON envelope; `snake_case` Rust variants bridge to the wire via `#[serde(rename = "…")]`. Lifecycle values, claim kinds, divergence enum, authority enum — all kebab on the wire.

## Sandboxing

Source-operation sandboxing is guest-owned: the composed runtime's adapter guests see only the preopens the host wiring grants them. No host environment leaks. There is no project-scope declared-tool surface.

## CLI surface

Headline verbs: `init`, `source {resolve, survey, extract}`, `target resolve`, `slice {create, refine, model show, build, transition, validate, provenance, merge}`, `plan {create, author, execute, add, amend, transition, next, status, archive}`, and `journal {emit, show}`. Workspace slots and topology remain plan inputs, but repository materialization and publication have no Specify command surface. `plan status` is the read-only next-action projection (plan entries + slice metadata + journal tail → `refine|build|merge <slice>` / `stop <reason>` / `drained`); the same body carries the RM-15 re-entry fields — `current-step` / `last-completed` (loop position) and `resume` (the literal command that makes progress, `null` when no single command does); it writes nothing. `journal show [--filter <event-id-prefix>] [--limit N]` is the read verb over the journal — text mode emits the canonical JSONL lines; it emits no event of its own. See [`specify --help`](../init.md) and the parent repo's [`AGENTS.md` §"Skill / CLI responsibility split"](https://github.com/augentic/specify/blob/main/AGENTS.md#skill--cli-responsibility-split).

Plan artifacts (`plan.yaml` / `change.md` / `discovery.md`) resolve at the invoked project directory — the same root every `.specify/` path anchors on. There is no plan-root override flag; slot-side workspace routing (letting a slot-run verb read the initiating workspace's plan) awaits a real implementation. Relative `sources.<key>.path` bindings join onto the project root.

## Writer ownership

Per-entry status writes route to exactly one CLI verb each — `plan add` / `plan amend` write `pending`, `plan next` writes `in-progress`, `slice merge` (via `plan transition <entry> done`) writes `done`. Plan-level `approved` is operator-only.

Driver mutual exclusion is guest-owned: the `plan execute` orchestrator holds the `.specify/guest.lock` marker for the loop's lifetime and refuses a concurrent driver with `guest-marker-held`. It is the only concurrency fence — there is no native lock wrapper.

## Observability

Newline-delimited JSON journal at `.specify/journal.jsonl`. The closed `EventKind` taxonomy — the authoritative per-event id set — lives in [`crates/workflow/src/journal/event.rs`](../../crates/workflow/src/journal/event.rs). Source operations add `source.survey.completed`, `slice.extract.completed`, and `source.execution.agent` — emitted by the guest `survey` / `extract` orchestrations. The synthesis engine adds `slice.synthesize.{started,agent,completed,failed}` (§"Slice synthesis"), distinct from the per-requirement `slice.synthesis.{conflict,divergence,unknown}` tag events. The guest `plan author` orchestration emits a single `plan.reconcile.completed` event on a successful write. `specify plan transition <plan> approved` records the closed `actor` enum (`operator | agent`, default `operator`, self-reported via `--actor`) on `plan.transition.approved`; `specify plan next` emits `plan.entry.advanced` only when an entry actually moves `pending → in-progress`. The guest `slice build` orchestration adds `target.execution.agent` and brackets its finalize tail with `slice.build.started` then `slice.build.succeeded` / `slice.build.failed`; `specify slice merge` fires `slice.merge.started` / `.succeeded` / `.failed` on its validator outcome (not on a merge report) alongside the durable `slice.archive.created` (§"Target adapter contract"). Agent-orchestrated phases that lack a deterministic emit command write through `specify journal emit <event-id> [--payload <json>]` — a guarded front door onto the same closed taxonomy, errors `journal-emit-unknown-event` / `journal-emit-payload-schema` (exit 2). The read side is `specify journal show [--filter <event-id-prefix>] [--limit N]` — append-order, lenient to unparseable lines like every other journal reader, emitting no event of its own; text mode prints the canonical JSONL lines so consumers project payloads without re-parsing the file by hand.

## Operations typed at parse boundary

The closed `SourceOperation` / `TargetOperation` enums in [`crates/workflow/src/adapter/operation.rs`](../../crates/workflow/src/adapter/operation.rs) are the typed per-axis operation sets, derived from the binding axis.

## Deliberate absences

The `specify adapter` namespace carries adapter *authoring* verbs only — `specify adapter build` and `specify adapter publish` (adapter packaging and transport) — never adapter *resolution* (resolution stays on `specify source`/`specify target`). There is no `change` verb namespace on the CLI, and no bare-string `sources` shorthand on `plan.yaml.sources.<key>`.

## Note to the implementing agent

Touching `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/artifacts/src/spec/provenance.rs`, `crates/workflow/src/adapter/`, `crates/workflow/src/journal.rs`, or `crates/workflow/src/schema_gate.rs` requires a repo-wide `rg` sweep across both the in-tree Rust workspace and the surrounding `augentic/specify` prose in the same PR — the contract spans both trees.

## D1 — Runtime source adapter (`captures`)

`captures` emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`. Schema entry in [`schemas/evidence.schema.json`](../../schemas/evidence.schema.json); claim type at [`crates/artifacts/src/evidence/claim/example.rs`](../../crates/artifacts/src/evidence/claim/example.rs).

## D2 — Per-kind authority on Evidence (deferred)

A per-Evidence `authority-overrides` map keyed by claim kind is **deferred to a future RFC**. v1 resolves authority at document level via the Evidence `authority:` field, with the per-slice `authority-override` on `plan.yaml` as the sole override surface (D3). See [`crates/artifacts/src/evidence/authority.rs`](../../crates/artifacts/src/evidence/authority.rs).

## D3 — Per-slice authority on `plan.yaml`

`plan.yaml.slices[].authority-override` maps claim kind to a source key bound on the slice. Orphan keys surface as `slice-authority-override-orphan-source`.

## D4 — Provenance is an on-demand projection

Provenance is carried inline in the single `model.yaml` artifact; `spec.md` is the authoritative artifact. There is no persisted `provenance.yaml` — `specify slice provenance <slice> [--format]` projects the audit view on demand. Projection schema at [`schemas/slice/provenance.schema.json`](../../schemas/slice/provenance.schema.json); projector at [`crates/workflow/src/slice/provenance.rs`](../../crates/workflow/src/slice/provenance.rs).

## D5 — Operator-driven `divergence`

The CLI is the single writer of every `Divergence` variant, all through `specify plan amend --divergence`. Operators flip `accepted | rejected`; guest-side plan authoring stages `likely` after a successful reconcile write. `plan create` scaffolds an empty plan and never stamps divergence. See [`crates/workflow/src/change/plan/core/model.rs`](../../crates/workflow/src/change/plan/core/model.rs).

## D7 — `--auto-approve`

`specify plan create --auto-approve` stamps Gate 1 in the same invocation when validation passes. Failure under `--auto-approve` MUST NOT stamp; the operator re-runs after fixing.

## Provenance projection

Closed top-level shape on the projected view: `version`, `slice`, `generated-at`, `generator`, `requirements[]`. The view is computed from `model.yaml` on demand by `specify slice provenance` and is never persisted. See [`crates/workflow/src/slice/provenance.rs`](../../crates/workflow/src/slice/provenance.rs) and `kind: tool` evaluator contract above.
