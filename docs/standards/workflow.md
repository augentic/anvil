# Workflow contract

The in-force contract this binary implements. Stable anchors that source code and adapter prompts cite by `§`-name. This document is the live anchor surface for workflow behavior.

## Adapter vocabulary

Two adapter roles — `source` (operations: `survey`, `extract`) and `target` (operations: `guidance`, `build`, `merge`). An adapter ships as a single WebAssembly component exporting its axis interface from the WIT contract (one component, no manifest). See the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/emery/blob/main/AGENTS.md#vocabulary).

## Adapter implementation shape

An adapter is one `.wasm` component. Identity (`name`, `version`) is carried by the artifact's location — the store entry filename for a pinned identity, the guest crate's `Cargo.toml` version at publish time. Non-identity metadata — an optional `emery` compatibility floor and (targets) an optional `platforms` capability and `inputs[]` list — is the WIT `metadata` record returned by the component's deterministic `metadata` export, dispatched host-side at resolve time and cached against the component's digest. The closed operation set derives from the binding axis per the WIT contract; operation behaviour (prompts included) is compiled into the component. Implementation: [`crates/project/src/adapter/`](../../crates/project/src/adapter); the WIT contract at [`wit/emery.wit`](../../wit/emery.wit).

## Source adapter contract

The WIT `source` interface; the closed operation set `{extract, survey}` derives from the WIT contract. `survey` writes `## Lead inventory` blocks under `discovery.md` at plan time; `extract` writes one Evidence document per `(source, lead)` pair at slice time. The typed shape lives at [`crates/artifacts/src/evidence.rs`](../../crates/artifacts/src/evidence.rs).

`emery source survey <source> [--plan <name>]` and `emery source extract <source> <lead> --slice <slice>` are the guest-routed runners (`orchestrate::survey` / `orchestrate::extract`). `<source>` resolves against `plan.yaml.sources.<key>`, then the adapter from `SourceBinding.adapter`. Both validate before the write becomes visible (the lead set through `artifacts::discovery::validate_leads` then the `discovery.md` merge; Evidence through the typed `artifacts::evidence::Document` parse then persist to `.emery/slices/<slice>/evidence/<source>.yaml`). Source operations are agent-only, collapsed into one guest orchestration each — the judgment leg runs against the adapter guest's compiled-in prompt. Value-bound sources (`intent`) carry `value-inline`; path bindings carry `source-path`.

## Target adapter contract

The WIT `target` interface; the closed operation set `{guidance, build, merge}` derives from the WIT contract. `guidance` is read by core synthesis; `build` and `merge` are agent-driven. The optional `inputs[]` in the target's `metadata` answer (a flat `{ path, required }` list, paths relative to the build request's `inputs.root`) declares the target-specific build inputs the CLI assembles into `inputs.artifacts.additional[]`.

`emery slice build <slice> [--format json]` is the guest-routed target build runner (`orchestrate::build`). It is the symmetric target-side twin of `emery source survey` / `extract`: the orchestrator owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate, while the bound target's `build` prompt (compiled into the adapter guest) owns only code generation. It resolves the target from the slice's bound project — `plan.yaml` stores the slice's `project`, not a resolved `target`. The orchestration assembles + schema-validates the request, writes `.emery/slices/<slice>/build/request.yaml`, emits `target.execution.agent`, drives the adapter guest's `build` judgment leg (any build prelude, e.g. vectis asset materialization, is in-guest adapter code), validates the resulting `build/report.yaml`, rejects a `success` report carrying a blocking finding, gates the `Refined → Built` transition, and journals `slice.build.succeeded` / `slice.build.failed`.

Both build envelopes are closed-shape YAML, keyed on `(slice, target)`, owned by the typed `BuildRequest` / `BuildReport` DTOs in `project::seam::wire`: the request carries `{ version, slice, project-dir, inputs: { root, artifacts: { proposal, design, tasks, specs[], additional[] } } }` and omits `target` / `execution` / prompt paths / `model.yaml` (audit input, not a build input); `inputs.root` (slice tree) and `project-dir` (working tree) are distinct. A missing `required` adapter-declared input raises `target-build-input-missing`. The report carries `{ version, slice, target, status: success|failure, findings[] }` — `findings[]` are typed `Diagnostic`s and default `[]`; a `success` report with any blocking finding is rejected (`target-build-success-with-blocking-finding`). The four pinned `target-build-*` codes are `Error::Validation` outcomes (exit 2), not new enum arms.

`merge` requires lifecycle `built` and is dispatched **twice** per slice merge — the closed WIT `merge-phase` enum (`preflight | postflight`) brackets the engine's deterministic core merge. `emery slice merge` orders the transaction as: target **preflight** gate → deterministic commit (validators, 3-way spec fold, Decision Record promotion, lifecycle, archive, per-entry `done`) → target **postflight** gate. Each gate's report reuses the typed build-report shape, is gated by the orchestrator, and persists to `merge/preflight.yaml` in the slice tree (archived with the slice) and `merge/postflight.yaml` beside the archive. A preflight failure aborts with the slice still `built` (`target-merge-preflight-failed`); a postflight failure is a non-rollback terminal diagnostic (`target-merge-postflight-failed`) because the merge has already committed. Neither gate performs lifecycle transitions, baseline spec merging, or archive moves — those stay engine-owned. Build outputs are not cached.

## Resolver and cache

`adapter::Resolver` is the provider capability used by operations and kernels. The shipped WASI provider delegates it to `adapter::resolver::Component`, whose per-axis methods resolve the identity to exactly one `.wasm` component. A pinned `(name, version)` resolves only the global single-file store entry `<store-root>/<name>@<version>.wasm` (D4 verify-on-read against the recorded byte digest). A bare name or persisted component selector resolves only the seeded project component cache: `<project-cache>/components/<name>.wasm`, populated by `emery adapter add <path.wasm>` (pre-init, axis-neutral, per-component `<name>.meta.yaml` provenance sidecar) or an operator-supplied local component at init.

`Resolver::expand` is the defaulted deployment-policy hook ahead of ensure: the component deployment widens a bare name with no seeded cache entry to the embedded first-party **adapter train** pin (`emery:<name>@FIRST_PARTY_ADAPTER_TRAIN`), while every other implementor (the native catalog host included) keeps the identity default. Expansion runs at the two places a bare selector enters the system — init's target ensure and `plan author`'s source-binding ensure loop — and the effective selector is persisted (`project.yaml.adapter` / `plan.yaml.sources.<key>.version`) before first use, so resolution stays reproducible across hosts with different embedded trains. Bare at resolve/dispatch time remains cache-only: the resolver seam's contract is unchanged.

Both roots derive from one carried `Locations` value — `EMERY_HOME`, else `~/.emery`, with `store/` and `cache/` beneath it — captured once at each composition root; kernels never read the environment. There is no build-tree probe, no sibling-checkout probe, and no environment-variable fallback to an out-of-tree framework checkout. An adapter built elsewhere reaches the project through `emery adapter add`, an operator-supplied local `.wasm` at init (mirrored into the project component cache), or a pinned store install. When the cache misses, component resolution fails with `adapter-not-found`, suggesting `adapter add` or an exact pin; the native host's static catalog match fails with `adapter-not-linked`, naming the linked identities on that axis.

An unpinned cache resolve carries no package identity: resolved versions are optional (`version` is omitted from envelopes and topology targets project the bare `name`; pinned targets stay `name@version`). A pre-cut topology `name@0.0.0` reads as an exact package pin — derived topology locks must be regenerated after the cut.

In workspace mode, slot setup is operator-owned. Each materialized slot must independently satisfy adapter resolution; Emery does not mirror the workspace's component cache into slots.

`emery init <adapter>` accepts a package reference (`emery:<name>@<semver>` — installed into the store on fetch), the first-party **shorthand** (`omnia@1.0.0` is package-reference sugar; bare `omnia` resolves the seeded project component cache, else auto-pins to the embedded adapter train at ensure time), or a local `.wasm` path. GitHub URLs are refused (`adapter-github-uri-unsupported`).

The `source resolve` / `target resolve` JSON envelope carries `axis`, `name`, `version` (omitted for an unpinned cache resolve), `resolved-path`, `location`, and `operations`. `location` and `resolved-path` project the resolver's opaque `Origin`; the component provider emits `store` / `cache`, while other providers may use another mechanism label. It is diagnostic: operation prompts are compiled into each adapter's deployment, so no engine code resolves prompt files at run time.

## Adapter name uniqueness

Adapter names remain unique across axes — one component exports exactly one axis interface, and a name identifies one published package. A component bound on the wrong axis fails at the dispatch seam: no deployed guest exports the requested `<axis>:<name>` id, so the metadata dispatch (and any operation call) cannot reach it.

## Discovery handshake

`survey` writes `## Lead inventory` blocks — one **raw, unmerged** lead per source, each identified by its `(source, lead)` pair (`survey` stamps `source` from the surveyed source). A re-survey of one source replaces only that source's blocks by `(source, lead)`; the same `lead` may appear under different source keys. The operator stamps `approved`; `extract` resolves `slices[].sources[].lead` against the canonical `lead` id within the binding's `source`. Cross-source unification is deferred to plan-time reconciliation. Typed shape and validator at [`crates/artifacts/src/discovery/lead.rs`](../../crates/artifacts/src/discovery/lead.rs); parser at [`crates/artifacts/src/discovery/document.rs`](../../crates/artifacts/src/discovery/document.rs).

## The Plan

`plan.yaml` shape is fixed by the typed `Plan` model at [`crates/project/src/plan/model.rs`](../../crates/project/src/plan/model.rs). Two stored lifecycle states (`pending | approved`); per-entry status is `pending | in-progress | done`. Writer ownership is split — see §"Writer ownership" below.

## Workflow vocabulary

`Slice`, `Lead`, `Evidence`, `Source`, `Target`, `Plan`, `Discovery`. Definitions live in the parent repo's [`AGENTS.md` §"Vocabulary"](https://github.com/augentic/emery/blob/main/AGENTS.md#vocabulary).

## Plan-time reconciliation

The guest `plan author` orchestration reconciles surveyed leads across sources at plan time and writes the `plan.yaml.slices[]` rows through the `Plan::propose_from` projection kernel. The kernel parses the judgment response through the typed proposal DTOs (kebab wire fields, closed `kind: request | response`; the answer schema handed to the model host is generated from them by `project::answers::proposal`), re-reads `discovery.md`, validates total lead coverage (at most one lead per source, fan-out `sources[]` consistency), binds each slice's explicit `name` and `project` (the target adapter is resolved on demand from that project, never written to `plan.yaml`), and replaces `slices[]` only on a replaceable plan (`lifecycle: pending` and every entry `pending`). Cross-source matching is agent judgment; the operator curates at Gate 1. The closed `plan-reconcile-*` codes are `Error::Validation` outcomes (exit 2). See [`crates/project/src/plan/propose.rs`](../../crates/project/src/plan/propose.rs).

The closed `Divergence` enum (`none | likely | accepted | rejected`) records a reconciliation outcome's confidence. See [`crates/project/src/plan/model.rs`](../../crates/project/src/plan/model.rs).

## Source

`plan.yaml.sources.<key>` is the structured `{ adapter, path?, value? }` object with exactly one of `path` / `value`.

`Slice.sources` (a slice's per-source binding list) accepts the bare-string shorthand on parse and serialises as the structured `{ source, lead }`.

## Authority hierarchy

Closed enum `intent > documentation > behaviour`. v1 resolution order per `(source, kind)`: per-slice `authority-override` → Evidence document-level `authority:` → tie at the top class is a `conflict` (the per-Evidence per-kind override is deferred — see §"D2 — Per-kind authority on Evidence (deferred)"). The kernel resolves authority **after** the synthesis response returns and projects winners/`status` from it (§"Slice synthesis"). Closed enums at [`crates/artifacts/src/evidence/authority.rs`](../../crates/artifacts/src/evidence/authority.rs); the production resolver at [`crates/slice/src/synthesis/authority.rs`](../../crates/slice/src/synthesis/authority.rs).

## Execution model

`pending → approved` plan-level (Gate 1; operator-only). Per-entry: `pending → in-progress → done`. `done` is absorbing in v1; the operator-reversed flow lives behind `emery plan transition --undo`.

## Refinement

The guest `slice refine` orchestration runs `extract` per bound source and drives the synthesis kernel (§"Slice synthesis") to produce `proposal.md` / `spec.md` / `design.md` / `tasks.md` / `model.yaml` (provenance is carried inline in the single `model.yaml` artifact, projected on demand by `emery slice provenance`), and transitions the slice to `refined`. Validators live in [`crates/artifacts/src/validate/`](../../crates/artifacts/src/validate/) and [`crates/slice/src/handlers/validate.rs`](../../crates/slice/src/handlers/validate.rs).

## Slice synthesis

The synthesis engine turns a slice's `Evidence[]` into its requirement set, the single `model.yaml`, and the rendered Markdown artifacts. It runs as the judgment leg inside the guest `slice refine` / `plan execute` orchestrations:

- The **inputs** leg is read-only: it reads each bound source's `lead` from `evidence/<source>.yaml` and carries the project-relative `evidence-path` to that document (wire field `evidence-path`; the agent reads the claims from the lent tree, not from the prompt), plus the resolved target guidance body (wire field `guidance-brief`), then hands the agent the **inputs** envelope (`kind: inputs`). Authority is **not** included. It writes nothing and emits `slice.synthesize.agent` (synthesis is always agent-dispatched — no tool path, no closed *request* wire shape).
- The **persist** tail is the only writer: it parses the judgment response through the typed `SynthesisResponse` (`kind: response`, code `synthesis-schema`; the model host enforces the generated `slice::answers::synthesis` schema), resolves authority from on-disk Evidence + per-slice `authority-override`, runs the projection kernel, renders provenance lines into `specs/<domain>/spec.md`, drift-validates, then atomically/staged-persists `proposal.md` / `specs/<domain>/spec.md` / `design.md` / `tasks.md` / `model.yaml` (prior artifacts intact on failure). It emits `slice.synthesize.started` then `slice.synthesize.completed` (or `slice.synthesize.failed`). No `provenance.yaml` is ever written.

**Kernel ownership (normalize, never reject).** The agent authors per-requirement `claims[]` `(source, id, kind)`, an `agreement` verdict, prose (`title` / `statement` / `scenarios` / `notes`), the owning `domain`, the agent-authored `tasks[]` with `TASK` ids, and prose-only spec bodies (no `ID:` / `Sources:` / `Status:` lines). The kernel owns and re-derives the `version` / `slice` / `project` header, `REQ-NNN` ids (declaration order, no holes), `status`, per-claim `winner` markers, the rendered `sources` lists (highest authority first), and the inline provenance; any agent-supplied `id` / `status` / `winner` / `sources` is ignored and recomputed. Modules at [`crates/slice/src/synthesis/`](../../crates/slice/src/synthesis). The typed `SynthesisResponse` parse is the shape gate; the answer schema handed to the model host is generated from the same DTOs by `slice::answers::synthesis`. `emery slice model show <slice> [--format json]` is the read-only model viewer.

**Drift validators.** `emery slice validate` adds seven blocking typed-model findings (exit 2), emitted as `Diagnostic` findings on the `DiagnosticReport` surface:

| Finding                           | Meaning                                                                                                                           |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `slice-model-schema`              | `model.yaml` does not deserialise into the typed slice model.                                                                     |
| `slice-spec-provenance-stale`     | Kernel-rendered provenance lines in `spec.md` disagree with `model.yaml`.                                                         |
| `slice-model-target-drift`        | `model.yaml.project` disagrees with `plan.yaml.slices[<slice>].project`. (`target` is not persisted, so there is no target half.) |
| `slice-model-source-orphan`       | A claim references an absent source key or Evidence claim id.                                                                     |
| `slice-model-cross-ref-orphan`    | A `satisfies[]` `REQ-*` reference is missing from `requirements[].id`.                                                            |
| `slice-model-claim-kind-mismatch` | A claim `kind` (D13) disagrees with the Evidence kind for that `(source, id)`.                                                    |
| `slice-model-id-grammar`          | A `REQ` or `TASK` id does not match its closed three-digit grammar.                                                               |

## Extraction

Per-source `extract` is agent-executed and never memoized: agent outputs are non-deterministic, so every run re-extracts. The validated Evidence at `.emery/slices/<slice>/evidence/<source>.yaml` is the only persisted result.

## Requirement block contract

`spec.md` requirements carry `ID:` / `Sources:` / `Status:` metadata; the closed `RequirementStatus` enum is `agreed | unknown | conflict | divergence`. Parser at [`crates/artifacts/src/spec/provenance.rs`](../../crates/artifacts/src/spec/provenance.rs).

## Wire format

Kebab-case discriminants on the JSON envelope; `snake_case` Rust variants bridge to the wire via `#[serde(rename = "…")]`. Lifecycle values, claim kinds, divergence enum, authority enum — all kebab on the wire.

## Sandboxing

Source-operation sandboxing is guest-owned: the hosted WASM runtime's adapter guests see only the preopens the host wiring grants them. No host environment leaks. There is no project-scope declared-tool surface.

## CLI surface

Headline verbs: `init`, `source {resolve, survey, extract}` and `target resolve` (the adapter debug/breakout surface), `slice {list, refine, model show, build, validate, provenance, merge, drop}`, `plan {author, execute, add, amend, remove, transition, next, status, archive}`, and `journal {emit, show}`. Workspace slots and topology remain plan inputs, but repository materialization and publication have no Emery command surface. `plan status` is the read-only next-action projection (plan entries + slice metadata + journal tail → `refine|build|merge <slice>` / `stop <reason>` / `drained`); the same body carries the RM-15 re-entry fields — `current-step` / `last-completed` (loop position) and `resume` (the literal command that makes progress, `null` when no single command does); it writes nothing. `journal show [--filter <event-id-prefix>] [--limit N]` is the read verb over the journal — text mode emits the canonical JSONL lines; it emits no event of its own. See [`emery --help`](../init.md) and the parent repo's [`AGENTS.md` §"Skill / CLI responsibility split"](https://github.com/augentic/emery/blob/main/AGENTS.md#skill--cli-responsibility-split).

Plan artifacts (`plan.yaml` / `change.md` / `discovery.md`) resolve at the invoked project directory — the same root every `.emery/` path anchors on. There is no plan-root override flag; slot-side workspace routing (letting a slot-run verb read the initiating workspace's plan) awaits a real implementation. Relative `sources.<key>.path` bindings join onto the project root.

## Writer ownership

Per-entry status writes route to exactly one CLI verb each — `plan add` / `plan amend` write `pending`, `plan next` writes `in-progress`, `slice merge` (via `plan transition <entry> done`) writes `done`. Plan-level `approved` is operator-only and written solely by the nameless `plan approve`: the operator runs it directly, or `/emery:execute` runs it on the operator's explicit confirmation (`--actor` stays `operator`).

Driver mutual exclusion is guest-owned: the `plan execute` orchestrator holds the `.emery/guest.lock` marker for the loop's lifetime and refuses a concurrent driver with `guest-marker-held`. It is the only concurrency fence — there is no native lock wrapper.

## Observability

Newline-delimited JSON journal at `.emery/journal.jsonl`. The closed `EventKind` taxonomy — the authoritative per-event id set — lives in [`crates/project/src/journal/event.rs`](../../crates/project/src/journal/event.rs). Source operations add `source.survey.completed`, `slice.extract.completed`, and `source.execution.agent` — emitted by the guest `survey` / `extract` orchestrations. The synthesis engine adds `slice.synthesize.{started,agent,completed,failed}` (§"Slice synthesis"), distinct from the per-requirement `slice.synthesis.{conflict,divergence,unknown}` tag events. The guest `plan author` orchestration emits a single `plan.reconcile.completed` event on a successful write. `emery plan approve` records the closed `actor` enum (`operator | agent`, default `operator`, self-reported via `--actor`) on `plan.transition.approved`; `emery plan next` emits `plan.entry.advanced` only when an entry actually moves `pending → in-progress`. The guest `slice build` orchestration adds `target.execution.agent` and brackets its finalize tail with `slice.build.started` then `slice.build.succeeded` / `slice.build.failed`; `emery slice merge` fires `slice.merge.started`, then `slice.merge.succeeded` when both target merge gates and the deterministic commit pass, `slice.merge.failed` on a pre-commit failure (target preflight or the commit itself), or `slice.merge.postflight-failed` when the target postflight gate fails after the commit (non-rollback — the merge stands), alongside the durable `slice.archive.created` (§"Target adapter contract"). Agent-orchestrated phases that lack a deterministic emit command write through `emery journal emit <event-id> [--payload <json>]` — a guarded front door onto the same closed taxonomy, errors `journal-emit-unknown-event` / `journal-emit-payload-schema` (exit 2). The read side is `emery journal show [--filter <event-id-prefix>] [--limit N]` — append-order, lenient to unparseable lines like every other journal reader, emitting no event of its own; text mode prints the canonical JSONL lines so consumers project payloads without re-parsing the file by hand.

## Operations typed at parse boundary

The closed `SourceOperation` / `TargetOperation` enums in [`crates/project/src/adapter/operation.rs`](../../crates/project/src/adapter/operation.rs) are the typed per-axis operation sets, derived from the binding axis.

## Deliberate absences

The `emery adapter` namespace carries adapter *authoring* verbs only — `emery adapter build` and `emery adapter publish` (adapter packaging and transport) — never adapter *resolution* (resolution stays on `emery source`/`emery target`). There is no `change` verb namespace on the CLI, and no bare-string `sources` shorthand on `plan.yaml.sources.<key>`.

## Note to the implementing agent

Touching `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/artifacts/src/spec/provenance.rs`, `crates/artifacts/src/evidence/`, `crates/project/src/adapter/`, or `crates/project/src/journal.rs` requires a repo-wide `rg` sweep across both the in-tree Rust workspace and the surrounding `augentic/emery` prose in the same PR — the contract spans both trees.

## D1 — Runtime source adapter (`captures`)

`captures` emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`. Claim kind in [`crates/artifacts/src/evidence/authority.rs`](../../crates/artifacts/src/evidence/authority.rs); claim type at [`crates/artifacts/src/evidence/claim/example.rs`](../../crates/artifacts/src/evidence/claim/example.rs).

## D2 — Per-kind authority on Evidence (deferred)

A per-Evidence `authority-overrides` map keyed by claim kind is **deferred to a future RFC**. v1 resolves authority at document level via the Evidence `authority:` field, with the per-slice `authority-override` on `plan.yaml` as the sole override surface (D3). See [`crates/artifacts/src/evidence/authority.rs`](../../crates/artifacts/src/evidence/authority.rs).

## D3 — Per-slice authority on `plan.yaml`

`plan.yaml.slices[].authority-override` maps claim kind to a source key bound on the slice. Orphan keys surface as `slice-authority-override-orphan-source`.

## D4 — Provenance is an on-demand projection

Provenance is carried inline in the single `model.yaml` artifact; `spec.md` is the authoritative artifact. There is no persisted `provenance.yaml` — `emery slice provenance <slice> [--format]` projects the audit view on demand. Typed projection shape and projector at [`crates/slice/src/provenance.rs`](../../crates/slice/src/provenance.rs).

## D5 — Operator-driven `divergence`

The CLI is the single writer of every `Divergence` variant, all through `emery plan amend --divergence`. Operators flip `accepted | rejected`; guest-side plan authoring stages `likely` after a successful reconcile write. The `plan author` scaffold never carries divergence. See [`crates/project/src/plan/model.rs`](../../crates/project/src/plan/model.rs).

## D7 — Gate 1 is operator-stamped only (auto-approve removed)

The `--auto-approve` surface left with the standalone `plan create` verb. Gate 1 is stamped exclusively by `emery plan approve`; a repeated stamp on an already-approved plan is an idempotent no-op (no disk write, no journal event).

## Provenance projection

Closed top-level shape on the projected view: `version`, `slice`, `generated-at`, `generator`, `requirements[]`. The view is computed from `model.yaml` on demand by `emery slice provenance` and is never persisted. See [`crates/slice/src/provenance.rs`](../../crates/slice/src/provenance.rs) and `kind: tool` evaluator contract above.
