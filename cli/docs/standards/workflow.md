# Workflow contract

The in-force contract this binary implements. Stable anchors that source code and skill briefs cite by `§`-name. This document is the live anchor surface for workflow behavior.

## Adapter vocabulary

Two adapter roles — `source` (operations: `survey`, `extract`) and `target` (operations: `shape`, `build`, `merge`). The shared on-disk shape is `adapter.yaml`; per-axis schemas refine it. See the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/specify/blob/main/AGENTS.md#vocabulary).

## Adapter implementation shape

Per-adapter `adapter.yaml` carries `name`, `version`, `axis`, the required closed `execution` mode, `description`, the `briefs.<operation>` map, and an optional singular `extension` declaration. The closed operation set is determined by the manifest's `axis`. Source adapters are agent-only (`source.schema.json` enumerates `execution: ["agent"]`); target manifests may declare `agent` or `tool`. The loader rejects a manifest that omits `execution` (`adapter-execution-mode-required`). Implementation: [`crates/workflow/src/adapter/`](../../crates/workflow/src/adapter); per-axis schemas at [`schemas/adapter.schema.json`](../../schemas/adapter.schema.json), [`source.schema.json`](../../schemas/source.schema.json), [`target.schema.json`](../../schemas/target.schema.json).

## Source adapter contract

`axis: source`; `briefs.keys() ⊆ {extract, survey}`. `survey` writes `## Lead inventory` blocks under `discovery.md` at plan time; `extract` writes one Evidence document per `(source, lead)` pair at slice time. See [`schemas/source.schema.json`](../../schemas/source.schema.json) and [`schemas/evidence.schema.json`](../../schemas/evidence.schema.json).

`specify source survey <source> [--plan <name>] [--phase prepare|finalize]` and `specify source extract <source> <lead> --slice <slice> [--phase prepare|finalize]` are the CLI-owned runners. `<source>` resolves against `plan.yaml.sources.<key>`, then the adapter from `SourceBinding.adapter`. Both validate before the write becomes visible (lead set against `schemas/discovery/lead.schema.json` then `discovery.md` merge; Evidence against `schemas/evidence.schema.json` then persist to `.specify/slices/<slice>/evidence/<source>.yaml`). Source operations are agent-only and two-phase: `prepare` builds the sandbox + prints the handoff envelope; `finalize` validates / persists / journals. Value-bound sources (`intent`) carry `value-inline`; path bindings carry `source-path`. See [`DECISIONS.md` §"Source operations"](../../DECISIONS.md#source-operations) and [`DECISIONS.md` §"Adapter execution mode"](../../DECISIONS.md#adapter-execution-mode).

## Target adapter contract

`axis: target`; `briefs.keys() ⊆ {shape, build, merge}`. `shape` is read by core synthesis; `build` and `merge` are agent-driven. The optional manifest `inputs[]` (a flat `{ path, required }` list, paths relative to the build request's `inputs.root`) declares the target-specific build inputs the CLI assembles into `inputs.artifacts.additional[]`. See [`schemas/target.schema.json`](../../schemas/target.schema.json).

`specify slice build <slice> [--phase prepare|finalize] [--format json]` is the CLI-owned target build runner. It is the symmetric target-side twin of `specify source survey` / `extract`: the CLI owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate, while the bound target's `build` brief owns only code generation. It resolves the target from the slice's bound project — `plan.yaml` stores the slice's `project`, not a resolved `target`. Under `execution: agent` (every first-party target today) the verb is two-phase: `--phase prepare` (default) assembles + schema-validates the request, writes `.specify/slices/<slice>/build/request.yaml`, runs the Vectis-only RFC-46 prepare side-effects when the bound target is `vectis` (resolve effective `assets.yaml`, auto-dispatch `vectis materialize assets` for in-scope missing exports via `target-build-materialize-failed` on guest failure, then re-run the bootstrap `app-icon` gate with `plan-bootstrap-app-icon-missing`), emits `target.execution.agent`, prints a kebab-case handoff envelope, and returns without blocking; the agent then runs the `build` brief and writes `build/report.yaml`; `--phase finalize` validates that report, rejects a `success` report carrying a blocking finding, gates the `Refined → Built` transition, and journals `slice.build.succeeded` / `slice.build.failed`. Under `execution: tool` a single call runs the whole operation (no first-party build tool ships today — the dispatch is a wired seam). See [`DECISIONS.md` §"Target build envelope (D6, D9 target side, D7 proof)"](../../DECISIONS.md#target-build-envelope-d6-d9-target-side-d7-proof) and [`DECISIONS.md` §"Adapter execution mode (D9)"](../../DECISIONS.md#adapter-execution-mode-d9).

Both build envelopes are closed-shape YAML, keyed on `(slice, target)`, schema-validated by the CLI: the request (`schemas/target/build-request.schema.json`, `BUILD_REQUEST_JSON_SCHEMA`) carries `{ version, slice, project-dir, inputs: { root, artifacts: { proposal, design, tasks, specs[], additional[] } } }` and omits `target` / `execution` / brief paths / `model.yaml` (audit input, not a build input); `inputs.root` (slice tree) and `project-dir` (working tree) are distinct. A missing `required` adapter-declared input raises `target-build-input-missing`. The report (`schemas/target/build-report.schema.json`, `BUILD_REPORT_JSON_SCHEMA`) carries `{ version, slice, target, status: success|failure, findings[] }` — `findings[]` `$ref` the RFC-28 diagnostic schema and default `[]`; a `success` report with any blocking finding is rejected (`target-build-success-with-blocking-finding`). The four pinned `target-build-*` codes are `Error::Validation` outcomes (exit 2), not new enum arms.

`merge` requires lifecycle `built` and re-runs target-specific validators per the merge brief. v1 adds **no** merge envelope: `specify slice merge` is the writer, `slice.merge.started` / `.succeeded` / `.failed` fire on its validator outcome (not on a merge report), and the durable record stays `slice.archive.created`. A future merge-findings need reuses the build-report shape as `build/merge-report.yaml`. Build outputs are not cached.

## Resolver and cache

`SourceAdapter::resolve(name, project_dir)` and `TargetAdapter::resolve(name, project_dir)` are the per-axis entry points. Probe order:

1. `<project-cache>/manifests/{sources,targets}/<name>/` — agent-populated out-of-tree mirror.
2. `<project_dir>/adapters/{sources,targets}/<name>/` — in-repo manifest.

Resolution is project-local only; there is no environment-variable fallback to an out-of-tree framework checkout. When neither location matches, resolution fails with `adapter-not-found`.

In workspace mode, `specify workspace sync` provisions probe location 1 for each synced slot: it mirrors the workspace's adapter set (both axes, each adapter's `extension` included) into the slot's manifest cache — per-name delete-then-copy, skipping any name the slot vendors under its own `adapters/` tree. Resolution semantics are unchanged. See [`DECISIONS.md` §"Slot adapter provisioning via workspace sync"](../../DECISIONS.md#slot-adapter-provisioning-via-workspace-sync).

The `{sources,targets}` segment is keyed by `Axis`. See [`DECISIONS.md` §"Adapter loader axis routing"](../../DECISIONS.md#adapter-loader-axis-routing) and [`DECISIONS.md` §"Cache layout"](../../DECISIONS.md#cache-layout).

`specify init <adapter>` additionally accepts a first-party **shorthand** (`omnia`, `omnia@1.0.0`; a bare name resolves the single installed identity, a semver pin records the full `name@<semver>` per RFC-47) that resolves to the published adapter on GitHub. See [`DECISIONS.md` §"First-party `<adapter>` shorthand at init"](../../DECISIONS.md#first-party-adapter-shorthand-at-init).

The `source resolve` / `target resolve` JSON envelope carries `briefs-dir` — the absolute path to the resolved adapter's `briefs/` directory — alongside `resolved-path`, `operations`, and `description`. The source-operation prep seam consumes it for brief-directory resolution.

## Adapter name uniqueness

A name appears under `adapters/sources/<name>/` xor `adapters/targets/<name>/`. Collisions surface as `adapter-name-axis-collision`. See [`DECISIONS.md` §"Adapter name uniqueness"](../../DECISIONS.md#adapter-name-uniqueness).

## Discovery handshake

`survey` writes `## Lead inventory` blocks — one **raw, unmerged** lead per source, each identified by its `(source, lead)` pair (`survey` stamps `source` from the surveyed source). A re-survey of one source replaces only that source's blocks by `(source, lead)`; the same `lead` may appear under different source keys. The operator stamps `approved`; `extract` resolves `slices[].sources[].lead` against the canonical `lead` id within the binding's `source`. Cross-source unification is deferred to plan-time reconciliation. Schema at [`schemas/discovery/lead.schema.json`](../../schemas/discovery/lead.schema.json); parser at [`crates/model/src/discovery/document.rs`](../../crates/model/src/discovery/document.rs).

## The Plan

`plan.yaml` shape is fixed by [`schemas/plan/plan.schema.json`](../../schemas/plan/plan.schema.json). Two stored lifecycle states (`pending | approved`); per-entry status is `pending | in-progress | done`. Writer ownership is split — see §"Writer ownership" below.

## Workflow vocabulary

`Slice`, `Lead`, `Evidence`, `Source`, `Target`, `Plan`, `Discovery`. Definitions live in the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/specify/blob/main/AGENTS.md#vocabulary).

## Plan-time reconciliation

`specify plan propose` reconciles surveyed leads across sources at plan time and writes the `plan.yaml.slices[]` rows. `--dry-run [--format json]` reads `plan.yaml.sources`, the `discovery.md` lead inventory, and the project topology, then emits the `kind: request` envelope (flat `(source, lead)` lead catalog + `projects[]`) for the agent; it writes nothing. `--from <response.json> [--format json]` is the only slice writer: it schema-gates the response (`PROPOSAL_JSON_SCHEMA` at [`schemas/discovery/proposal.schema.json`](../../schemas/discovery/proposal.schema.json), kebab wire fields, closed `kind: request | response`), re-reads `discovery.md`, validates total lead coverage (at most one lead per source, fan-out `sources[]` consistency), binds each slice's explicit `name` and `project` (the target adapter is resolved on demand from that project, never written to `plan.yaml`), and replaces `slices[]` only on a replaceable plan (`lifecycle: pending` and every entry `pending`). Cross-source matching is agent judgment; the operator curates at Gate 1. The closed `plan-reconcile-*` / `plan-propose-mode-required` codes are `Error::Validation` outcomes (exit 2). See [`DECISIONS.md` §"Lead reconciliation (D2)"](../../DECISIONS.md#lead-reconciliation-d2) and [`crates/workflow/src/change/plan/core/propose.rs`](../../crates/workflow/src/change/plan/core/propose.rs).

The closed `Divergence` enum (`none | likely | accepted | rejected`) records a reconciliation outcome's confidence. See [`DECISIONS.md` §"`Divergence` enum"](../../DECISIONS.md#divergence-enum) and [`crates/workflow/src/change/plan/core/model.rs`](../../crates/workflow/src/change/plan/core/model.rs).

## Source

`plan.yaml.sources.<key>` is the structured `{ adapter, path?, value? }` object with exactly one of `path` / `value`. See [`DECISIONS.md` §"Plan source bindings"](../../DECISIONS.md#plan-source-bindings).

`Slice.sources` (a slice's per-source binding list) accepts the bare-string shorthand on parse and serialises as the structured `{ source, lead }`. See [`DECISIONS.md` §"`SliceSourceBinding`: bare shorthand plus structured form"](../../DECISIONS.md#slicesourcebinding-bare-shorthand-plus-structured-form).

## Authority hierarchy

Closed enum `intent > documentation > behaviour`. v1 resolution order per `(source, kind)`: per-slice `authority-override` → Evidence document-level `authority:` → tie at the top class is a `conflict` (the per-Evidence per-kind override is deferred — see §"D2 — Per-kind authority on Evidence (deferred)"). The kernel resolves authority **after** the synthesis response returns and projects winners/`status` from it (§"Slice synthesis"). Closed enums at [`crates/model/src/evidence/authority.rs`](../../crates/model/src/evidence/authority.rs); the production resolver at [`crates/workflow/src/slice/synthesis/authority.rs`](../../crates/workflow/src/slice/synthesis/authority.rs).

## Execution model

`pending → approved` plan-level (Gate 1; operator-only). Per-entry: `pending → in-progress → done`. `done` is absorbing in v1; the operator-reversed flow lives behind `specify plan transition --undo` (see [`DECISIONS.md` §"Plan lifecycle: two stored states"](../../DECISIONS.md#plan-lifecycle-two-stored-states)).

## Refinement

`/spec:refine` runs `extract` per bound source, drives `specify slice synthesize` (§"Slice synthesis") to produce `proposal.md` / `spec.md` / `design.md` / `tasks.md` / `model.yaml` (provenance is carried inline in the single `model.yaml` artifact, projected on demand by `specify slice provenance`), and transitions the slice to `refined`. Validators live in [`crates/model/src/validate/`](../../crates/model/src/validate/) and [`src/runtime/commands/slice/validate.rs`](../../src/runtime/commands/slice/validate.rs).

## Slice synthesis

`specify slice synthesize <slice>` turns a slice's `Evidence[]` into its requirement set, the single `model.yaml`, and the rendered Markdown artifacts. It mirrors `plan propose`'s two mutually-exclusive modes, exactly one of which is required (neither fails `slice-synthesize-mode-required`; the parser rejects both):

- `--dry-run [--format json]` is read-only: it reads each bound source's inline `lead` + `claims` from `evidence/<source>.yaml` and the resolved target `shape` brief body, then emits the agent **inputs** envelope (`kind: inputs`). Authority is **not** included. It writes nothing and emits `slice.synthesize.agent` (synthesis is always agent-dispatched — no tool path, no closed *request* wire shape).
- `--from <response.json> [--format json]` is the only writer: it schema-gates the response against `synthesis.schema.json` (`kind: response`, code `synthesis-schema`), resolves authority from on-disk Evidence + per-slice `authority-override`, runs the projection kernel, renders provenance lines into `specs/<domain>/spec.md`, drift-validates, then atomically/staged-persists `proposal.md` / `specs/<domain>/spec.md` / `design.md` / `tasks.md` / `model.yaml` (prior artifacts intact on failure). It emits `slice.synthesize.started` then `slice.synthesize.completed` (or `slice.synthesize.failed`). No `provenance.yaml` is ever written.

**Kernel ownership (normalize, never reject).** The agent authors per-requirement `claims[]` `(source, id, kind)`, an `agreement` verdict, prose (`title` / `statement` / `scenarios` / `notes`), the owning `domain`, the agent-authored `tasks[]` with `TASK` ids, and prose-only spec bodies (no `ID:` / `Sources:` / `Status:` lines). The kernel owns and re-derives the `version` / `slice` / `project` header, `REQ-NNN` ids (declaration order, no holes), `status`, per-claim `winner` markers, the rendered `sources` lists (highest authority first), and the inline provenance; any agent-supplied `id` / `status` / `winner` / `sources` is ignored and recomputed. Modules at [`crates/workflow/src/slice/synthesis/`](../../crates/workflow/src/slice/synthesis). Schema gate at [`crates/workflow/src/schema.rs`](../../crates/workflow/src/schema.rs) (`validate_synthesis_json`); `model.schema.json` and `synthesis.schema.json` are registered together through a `jsonschema::Registry` so the relative `model` `$ref` resolves. `specify slice model show <slice> [--format json]` is the read-only model viewer. See [`DECISIONS.md` §"Slice synthesis engine (RFC-29 M2b)"](../../DECISIONS.md#slice-synthesis-engine-rfc-29-m2b).

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

`spec.md` requirements carry `ID:` / `Sources:` / `Status:` metadata; the closed `RequirementStatus` enum is `agreed | unknown | conflict | divergence`. Parser at [`crates/model/src/spec/provenance.rs`](../../crates/model/src/spec/provenance.rs).

## Wire format

Kebab-case discriminants on the JSON envelope; `snake_case` Rust variants bridge to the wire via `#[serde(rename = "…")]`. Lifecycle values, claim kinds, divergence enum, authority enum — all kebab on the wire. See [`DECISIONS.md` §"Wire compatibility"](../../DECISIONS.md#wire-compatibility).

## Sandboxing

WASI extension runner pre-opens `$PROJECT_DIR` always, `$CAPABILITY_DIR` only for adapter-scope extensions. No host environment leaks. See [`DECISIONS.md` §"`$CAPABILITY_DIR` replaces `$ADAPTER_DIR`"](../../DECISIONS.md#capability_dir-replaces-adapter_dir).

Source-operation runners (`survey` / `extract`) preopen a four-root sandbox: `$SOURCE_DIR` read-only (absent for value-bound sources), `$CAPABILITY_DIR` read-only (manifest cache), `$SCRATCH_DIR` write-only, and `$PROJECT_DIR` **not visible**. Scratch lives under the transient working-state root, structurally outside the cache tree — `extract` under `.specify/scratch/<adapter>/<slice>/`, `survey` under `.specify/scratch/<adapter>/survey/`. See [`DECISIONS.md` §"Source operations"](../../DECISIONS.md#source-operations) and [§"Cache layout"](../../DECISIONS.md#cache-layout).

## CLI surface

Headline verbs: `init`, `source {resolve, survey, extract, preview}`, `target resolve`, `slice {create, synthesize, model show, build, transition, validate, provenance, merge}`, `plan {create, propose, add, amend, transition, next, status, archive, lock}`, `workspace {sync, push, prepare}`, `extension {run, fetch, gc}`, `journal {emit, show}`. `plan lock -- <cmd>` is the `flock(1)`-style command-wrapper that takes the driver lock for the spawned child's lifetime (releasing on exit), passes the child exit code through, refuses a busy lock with `plan-lock-busy` (exit 2), and skips re-acquisition under `SPECIFY_PLAN_LOCK_HELD=1`. `plan status` is the read-only next-action projection (plan entries + slice metadata + journal tail → `refine|build|merge <slice>` / `stop <reason>` / `drained`); the same body carries the RM-15 re-entry fields — `current-step` / `last-completed` (loop position) and `resume` (the literal command that makes progress, `null` when no single command does); it writes nothing. `journal show [--filter <event-id-prefix>] [--limit N]` is the read verb over the journal — text mode emits the canonical JSONL lines; it emits no event of its own. See [`specify --help`](../init.md) and the parent repo's [`AGENTS.md` §"Skill / CLI responsibility split"](https://github.com/augentic/specify/blob/main/AGENTS.md#skill--cli-responsibility-split).

The global `--plan-dir <PATH>` flag (env `SPECIFY_PLAN_DIR`) overrides where `plan.yaml` / `change.md` / `discovery.md` resolve — the workspace-routing bridge that lets slot-side phase verbs (`source extract`, `slice synthesize`, `slice validate`, `slice provenance`, `slice merge`'s `done` stamp) read the initiating workspace's plan while every `.specify/` path stays slot-local. Relative `sources.<key>.path` bindings join onto the plan root. See [`DECISIONS.md` §"Plan-root override: global `--plan-dir`"](../../DECISIONS.md#plan-root-override-global---plan-dir-env-specify_plan_dir).

## Writer ownership

Per-entry status writes route to exactly one CLI verb each — `plan add` / `plan amend` write `pending`, `plan next` writes `in-progress`, `slice merge` (via `plan transition <entry> done`) writes `done`. Plan-level `approved` is operator-only; in workspace mode the slot-side merge stamps the workspace plan through `--plan-dir`. See [`DECISIONS.md` §"Lifecycle write-ownership"](../../DECISIONS.md#lifecycle-write-ownership).

The loop-phase plan-state writers (`plan next`, per-entry `plan transition` including `--undo`, and a plan-backed `slice merge run`) additionally probe the OS advisory lock on `<plan-root>/.specify/plan.lock` and refuse an unlocked driver with `plan-lock-not-held` (exit 2) — dual-driving refusal as a runtime property ([`crates/workflow/src/plan_lock.rs`](../../crates/workflow/src/plan_lock.rs), proven by [`tests/workflow/plan_lock.rs`](../../tests/workflow/plan_lock.rs)). Lock *acquisition* is CLI-owned: `specify plan lock -- <cmd>` takes the lock and runs the loop driver under it (the framework's `/spec:execute` and breakout skills wrap their loop in this verb), so the same binary owns acquisition cross-platform with no `flock(1)` / `zsystem` / Python snippet. The plan-level Gate 1 stamp and standalone plan-less merges are exempt. Both `acquire` and the probe resolve through `Layout::plan_dir()`, so slot-side merge work under `--plan-dir` locks the *workspace*.

## Observability

Newline-delimited JSON journal at `.specify/journal.jsonl`. The closed `EventKind` taxonomy lives in [`crates/workflow/src/journal.rs`](../../crates/workflow/src/journal.rs); the per-event table is in [`DECISIONS.md` §"Journal event names"](../../DECISIONS.md#journal-event-names). Source operations add `source.survey.completed`, `slice.extract.completed`, and `source.execution.agent` — all CLI-owned by the `survey` / `extract` runners. `specify slice synthesize` adds `slice.synthesize.{started,agent,completed,failed}` (§"Slice synthesis"), distinct from the per-requirement `slice.synthesis.{conflict,divergence,unknown}` tag events. `specify plan propose --from` emits a single `plan.reconcile.completed` event on a successful write. `specify plan transition <plan> approved` records the closed `actor` enum (`operator | agent`, default `operator`, self-reported via `--actor`) on `plan.transition.approved`; `specify plan next` emits `plan.entry.advanced` only when an entry actually moves `pending → in-progress`. `specify slice build` adds `target.execution.agent` (on the agent `prepare` phase) and brackets finalize with `slice.build.started` then `slice.build.succeeded` / `slice.build.failed`; `specify slice merge` fires `slice.merge.started` / `.succeeded` / `.failed` on its validator outcome (not on a merge report) alongside the durable `slice.archive.created` (§"Target adapter contract"). `specify workspace sync` / `push` emit `workspace.sync.completed` / `workspace.push.completed` on success (dry runs, failed pushes, and the registry-less sync no-op emit nothing). Agent-orchestrated phases that lack a deterministic emit command write through `specify journal emit <event-id> [--payload <json>]` — a guarded front door onto the same closed taxonomy, errors `journal-emit-unknown-event` / `journal-emit-payload-schema` (exit 2). See [`DECISIONS.md` §"`specify journal emit` — guarded front door (D12)"](../../DECISIONS.md#specify-journal-emit--guarded-front-door-d12). The read side is `specify journal show [--filter <event-id-prefix>] [--limit N]` — append-order, lenient to unparseable lines like every other journal reader, emitting no event of its own; text mode prints the canonical JSONL lines so consumers project payloads without re-parsing the file by hand (see [`DECISIONS.md` §"Projection over persistence"](../../DECISIONS.md#projection-over-persistence)).

## Operations typed at parse boundary

`briefs.keys()` is the canonical operation iterator; the closed `SourceOperation` / `TargetOperation` enums in [`crates/workflow/src/adapter/operation.rs`](../../crates/workflow/src/adapter/operation.rs) are the typed key set. See [`DECISIONS.md` §"Operations typed at parse boundary"](../../DECISIONS.md#operations-typed-at-parse-boundary).

## What was cut and why

The `specify adapter` namespace carries adapter *authoring* verbs only — `specify adapter build` and `specify adapter publish` (RFC-48 packaging and transport) — never adapter *resolution* (resolution stays on `specify source`/`specify target`). There is no `specify change *` namespace, and no bare-string `sources` shorthand on `plan.yaml.sources.<key>`. See [`DECISIONS.md` §"Plan source bindings"](../../DECISIONS.md#plan-source-bindings).

## Note to the implementing agent

Touching `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/model/src/spec/provenance.rs`, `crates/workflow/src/adapter/`, `crates/workflow/src/journal.rs`, `crates/workflow/src/schema.rs`, the `$CAPABILITY_DIR` env var, or the `adapter--<axis>--<slug>` tool cache scope requires a cross-repo `rg` sweep against both [`augentic/specify-cli`](https://github.com/augentic/specify-cli) and [`augentic/specify`](https://github.com/augentic/specify) in the same PR — the contract spans both repos.

## D1 — Runtime source adapter (`captures`)

`captures` emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`. Schema entry in [`schemas/evidence.schema.json`](../../schemas/evidence.schema.json); claim type at [`crates/model/src/evidence/claim/example.rs`](../../crates/model/src/evidence/claim/example.rs).

## D2 — Per-kind authority on Evidence (deferred)

A per-Evidence `authority-overrides` map keyed by claim kind is **deferred to a future RFC**. v1 resolves authority at document level via the Evidence `authority:` field, with the per-slice `authority-override` on `plan.yaml` as the sole override surface (D3). See [`DECISIONS.md` §"Authority: document-level plus one override (v1)"](../../DECISIONS.md#authority-document-level-plus-one-override-v1) and [`crates/model/src/evidence/authority.rs`](../../crates/model/src/evidence/authority.rs).

## D3 — Per-slice authority on `plan.yaml`

`plan.yaml.slices[].authority-override` maps claim kind to a source key bound on the slice. Orphan keys surface as `slice-authority-override-orphan-source`. See [`DECISIONS.md` §"Plan per-slice authority overrides"](../../DECISIONS.md#plan-per-slice-authority-overrides).

## D4 — Provenance is an on-demand projection

Provenance is carried inline in the single `model.yaml` artifact; `spec.md` is the authoritative artifact. There is no persisted `provenance.yaml` — `specify slice provenance <slice> [--format]` projects the audit view on demand. Projection schema at [`schemas/slice/provenance.schema.json`](../../schemas/slice/provenance.schema.json); projector at [`crates/workflow/src/slice/provenance.rs`](../../crates/workflow/src/slice/provenance.rs). See [`DECISIONS.md` §"Single slice-model artifact"](../../DECISIONS.md#single-slice-model-artifact).

## D5 — Operator-driven `divergence`

The CLI is the single writer of every `Divergence` variant, all through `specify plan amend --divergence`. Operators flip `accepted | rejected`; the `/spec:plan` agent stages `likely` after `specify plan propose --from`. `plan create` scaffolds an empty plan and never stamps divergence. See [`crates/workflow/src/change/plan/core/model.rs`](../../crates/workflow/src/change/plan/core/model.rs).

## D7 — `--auto-approve`

`specify plan create --auto-approve` stamps Gate 1 in the same invocation when validation passes. Failure under `--auto-approve` MUST NOT stamp; the operator re-runs after fixing.

## Provenance projection

Closed top-level shape on the projected view: `version`, `slice`, `generated-at`, `generator`, `requirements[]`. The view is computed from `model.yaml` on demand by `specify slice provenance` and is never persisted. See [`crates/workflow/src/slice/provenance.rs`](../../crates/workflow/src/slice/provenance.rs) and `kind: tool` evaluator contract above.
