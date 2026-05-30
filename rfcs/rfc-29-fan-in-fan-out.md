# RFC-29: Fan-In/Fan-Out Code Contract

> Status: Draft — Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../done/rfc-28-standards-contract.md) — Relates: [RFC-35](done/rfc-35-synthesis-determinism.md) (see §"Relationship to RFC-35") — Enables: provable multi-source fan-in and plan-level multi-slice fan-out (D5)

## Abstract

Specify's architectural promise is a fan-in / fan-out workflow:

- **Fan-in** happens twice per change. Multiple source adapters' `Lead`s fan in at plan time into the `slices[]` rows of `plan.yaml`. Multiple sources' `Evidence` fans in at slice time into one synthesized slice. Both are core's responsibility.
- **Fan-out** happens once per change, at the plan layer. One change decomposes into multiple slices — each slice binding exactly one target — joined by `depends-on` edges. The `refine -> build -> merge` loop runs per slice; baseline merge runs once per slice against one target's baseline.

This is the framework's "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project)). RFC-29 affirms it and does not extend the slice to multi-target.

The gap is that several load-bearing fan-in steps — survey, extract, and plan-time lead reconciliation — are still uncontracted agent discipline rather than agent judgment running under a CLI-owned envelope. Both lead reconciliation (plan time) and slice synthesis (slice time) stay agent-led, because both are cross-source judgment with no deterministic function; in each case the CLI owns the **envelope** and the **projection kernel** around that judgment (see §"Lead reconciliation engine (D2)", §"Slice synthesis engine (D3)", and §"Relationship to RFC-35").

This RFC turns the fan-in promise into an end-to-end contract by adding:

1. **Executable source operations** - first-class `specrun source survey` and `specrun source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Agent-led plan-time lead reconciliation** - an agent-led cross-source matching step that groups each source's `Lead[]` into unified slice candidates (including semantic matches that exact id / alias cannot catch) and binds each `(group-id, target)` row to a target, running under a stable input/output envelope wrapped by a CLI-owned projection kernel: a deterministic structural floor, schema validation, the global lead-partition invariant, slice-name derivation, journal events, and the existing plan writers.
3. **Slice synthesis engine** - an agent-led cross-modal synthesis step (which decides the requirement set, declares each requirement's `(source, claim-id)` claims and an `agreement` verdict, and authors its prose) running under a stable input/output envelope, wrapped by a CLI-owned projection kernel that projects over the agent's structure: RFC-27 authority resolution, REQ-id assignment, `sources` and winner-marker derivation, status derivation, provenance projection into `provenance.yaml`, and drift validators.
4. **Typed slice model** - a machine-readable, schema-pinned view of the slice emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
5. **Target build contract** - target adapters consume the slice model through a stable per-slice build envelope, with per-slice validation, review findings, and merge gates.
6. **Proof fixtures** - acceptance coverage that exercises `N sources -> one slice model -> 1 target per slice`, with cross-target fan-out proven across multiple slices joined by `depends-on`, and the kernel / envelope split proven by two **deterministic** gates: kernel-projection determinism over a fixed synthesis response, and an envelope-construction proof that the requirements-relevant inputs are byte-identical across target bindings (no LLM judge in any gate).

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant: source operations are briefs not executable commands, plan-time lead reconciliation is uncontracted agent work (no envelope, no validation, no journal trail), slice-time reconciliation has no production resolver, the machine-readable slice view is implicit, and target codegen is adapter-brief discipline with no stable envelope. The normative decisions below close each gap.

The goal is not to remove agents but to wrap the agent's judgment in a stable envelope and to move stable workflow, data-shape, and bookkeeping obligations into the CLI. The two cross-source judgment steps — lead reconciliation (D2) and slice synthesis (D3/D10) — both default to `execution: agent`; see §"Lead reconciliation engine (D2)" and §"Slice synthesis engine (D3)" for the matching agent/kernel split.

## Normative decisions


| ID                              | Decision                                                                                                                                                                                                                                 | Implementation consequence                                                                                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D1 Source operation runner**  | The CLI runs source adapter `survey` and `extract` operations.                                                                                                                                                                        | Add `specrun source survey` and `specrun source extract`; route through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events.                       |
| **D2 Lead reconciliation engine**  | The agent-led reconciliation step owns cross-source matching of `Lead[]` into lead groups — deciding which leads across sources describe the same unit of work (including semantic matches beyond exact id / alias / cross-reference), declaring each group's `(source-key, lead-id)` members and per-member `match-basis`, and binding each `(group-id, target)` row to a target. `group-id` is a **concept id** (the same concept may appear on multiple rows with different targets). The CLI owns the projection kernel around that judgment: a deterministic **structural floor** (exact id, exact alias, transitive cross-reference) the agent's grouping may extend but never split, schema validation of the proposed grouping, the **global lead-partition invariant** (every surveyed lead lands in exactly one group across the whole response), **slice-name derivation** (§"Slice-name derivation"), journal events, and the existing plan writers. The kernel never invents or drops a lead, never merges on its own heuristic, and never overrides a structural-floor match. Carries `execution: agent \| executable`, mirroring D10's agent-first stance; `agent` is the default and designed centre. | Add `specrun plan propose`. `propose --dry-run` returns the reconciliation request envelope; `propose --from` validates the response, derives slice names, enforces invariants, and writes `plan.yaml.slices[]`. |
| **D3 Slice synthesis engine** | The agent-led synthesis step owns cross-modal reconciliation of `Evidence[]` into the requirement set — which requirements exist, how claims merge or split, each requirement's `(source, claim-id)` claims, an `agreement` verdict (`agreed` / `disagreed`) over those claims, its prose (`title` / `statement` / `scenarios` / `notes`), and prose-only Markdown drafts (requirement bodies without `ID:` / `Sources:` / `Status:` lines). The CLI owns the projection kernel that *projects over* that structure (RFC-27 authority resolution, REQ-id assignment, `sources` and winner-marker derivation, status derivation, provenance into `provenance.yaml`, kernel rendering of provenance lines into `spec.md`, drift validation) and the synthesis envelope. The kernel never reconstructs the requirement set, never picks a winner the resolved authority did not, and never overrides the agent's agreement verdict. | Add `specrun slice synthesize <slice>`. Validate agent response against `synthesis-draft-model.schema.json`; project kernel; validate merged `model.yaml` against `model.schema.json`; render provenance into `spec.md`; persist. `/spec:refine` shells out to the engine. |
| **D3a Draft vs persisted model** | The synthesis envelope response validates against `synthesis-draft-model.schema.json` (kernel-owned fields absent). Persisted `model.yaml` validates against `model.schema.json` (kernel fields required after projection). | Add `schemas/slice/synthesis-draft-model.schema.json`; embed as `SYNTHESIS_DRAFT_MODEL_JSON_SCHEMA`; register with `model` + `synthesis-envelope` in `specify-schema`. |
| **D4 Typed slice model**           | Every synthesized slice carries `.specify/slices/<slice>/model.yaml`.                                                                                                                                                                       | Add `schemas/slice/model.schema.json` + `synthesis-draft-model.schema.json`; `specrun slice validate` checks drift (incl. `slice-spec-provenance-stale`); target build reads persisted model.                                                     |
| **D5 Per-slice fan-out**        | Each slice binds exactly one target adapter / project. Cross-target changes decompose at plan time into multiple slices joined by `depends-on`. RFC-29 introduces no per-output schema, lifecycle, or build envelope.                    | No `outputs[]` field on the slice model, build request, or build report. `plan.yaml.slices[].target` / `slices[].project` keep their existing shape and meaning. Cross-slice ordering uses the existing `slices[].depends-on`. |
| **D6 Target build envelope**    | Target adapters receive a stable per-slice build request and return a stable per-slice build report.                                                                                                                                     | Add `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json`, keyed on `(slice, target)`; reports may include RFC-28 findings.                                                        |
| **D7 Acceptance proof path**    | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together.                                                                                                                            | Add a cross-repo test in which two sources feed two slices (one targeting `contracts@v1`, one targeting `omnia@v1`), joined by `depends-on`; each slice independently synthesises, builds, and merges.                |
| **D8 Shape-brief scope** | Target `shape` briefs parameterise non-requirements model sections only; they MUST NOT influence `requirements[]`, claims, agreement, `sources[]`, or any provenance-bearing field. | Envelope walls off `target` and `shape-brief` from requirements reconciliation (`forbidden-inputs-for-requirements-reconciliation`). Target-neutrality is proven by the envelope-construction gate (§"Acceptance proof (D7)"); kernel projection is target-independent over a fixed response. |
| **D9 Adapter execution mode**   | Source adapters declare a closed `execution: executable | agent-fallback` field; first-party adapters MUST be `executable` before RFC-29 ships, third-party adapters MAY be `agent-fallback` indefinitely.                               | Extend `schemas/source.schema.json` and (symmetrically) `schemas/target.schema.json` with the closed enum. `agent-fallback` forces `cache: opt-out` and emits `source.execution.agent-fallback` per invocation.       |
| **D10 Synthesis execution mode** | The synthesis step inside `specrun slice synthesize` carries a closed `execution: agent | executable` enum. Unlike the adapter enum (D9), synthesis is **agent-first by design**: cross-modal Evidence reconciliation is judgment work, so `agent` is the default and the designed centre — not a fallback. An `execution: executable` path is optional, reserved for future declared synthesis tools that admit narrow deterministic cases (e.g. single-source statement-quality Evidence). | Add the closed enum to slice-synthesis configuration. `agent` forces `cache: opt-out` for the synthesis step (the kernel's projection over the returned structure remains deterministic) and emits `slice.synthesize.agent` per invocation. The engine validates the draft response against `synthesis-draft-model.schema.json`, the merged result against `model.schema.json`, and the drift checks regardless of execution mode. |
| **D11 Standalone provenance projection** | `specrun slice provenance <slice>` is the standalone entry point onto the same projection kernel as D3, reading persisted `model.yaml` claims + agreement verdicts. | Add `specrun slice provenance <slice> [--format json]`; shares the D3 kernel; no change to `schemas/slice/provenance.schema.json`. See §"Relationship to RFC-35". |
| **D12 Journal emitter** | `specrun journal emit` is the schema-validated writer for agent-orchestrated phases with no deterministic emit command (D2/D9/D10 agent paths, agent-driven build/merge). | Add `specrun journal emit <event-id> [--payload <json>] [--format json]` with `journal-emit-unknown-event` / `journal-emit-payload-schema` rejection. See §"Journal emitter (D12)" and §"Relationship to RFC-35". |
| **D13 Claim contract (`id` + `kind`)** | Every claim that contributes to a requirement carries a stable `claim-id` **and** its `kind`. RFC-29 (a) tightens `schemas/evidence.schema.json` so `claim-id` is required on **every** claim kind (closing the pre-RFC-29 gap where it was required only on `requirement` / `criterion` / `example`), and (b) carries `kind` on `model.yaml.requirements[].claims[]` so the projection kernel resolves per-kind authority (see §"Authority over mixed-kind claims") and populates the `kind`-bearing `provenance.yaml` `contributing-claims[]` **without re-reading Evidence**. | Add `kind` (required, mirrors `evidence.schema.json#/$defs/claimKind`) to `modelClaim` in `schemas/slice/model.schema.json`. Make `claim-id` unconditionally `required` in `evidence.schema.json#/$defs/claim`. `specrun slice validate` adds `slice-model-claim-kind-mismatch` when a claim's `kind` disagrees with the kind recorded for that `(source, claim-id)` in Evidence; `slice-model-source-orphan` still catches a `(source, claim-id)` absent from Evidence. |


## Operator surface

The default operator rhythm does not change:

```bash
/spec:plan identity-refresh source docs=documentation:./docs source legacy=code-typescript:./legacy
specrun plan transition identity-refresh approved
/spec:execute
/spec:finalize identity-refresh
```

The new CLI surfaces are lower-level breakouts:

```bash
specrun source survey docs --format json
specrun source extract docs password-reset --slice identity-password-reset --format json
specrun plan propose --dry-run --format json                 # D2 reconciliation request envelope (floor + inventory; never writes plan.yaml)
specrun plan propose --from grouping.json --format json      # D2 kernel: validate agent grouping, enforce invariants, write slices
specrun slice synthesize identity-password-reset --format json
specrun slice provenance identity-password-reset --format json   # D11 standalone projection onto the D3 kernel
specrun slice model show identity-password-reset --format json
specrun journal emit slice.synthesize.agent --payload '{"slice":"identity-password-reset"}'   # D12 agent-orchestrated emitter
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

Each entry keeps its existing one-target shape (see §"Per-slice fan-out (D5)"). In the default flow these rows are written by the D2 reconciliation kernel (`specrun plan propose --from`) projecting the agent's grouping through these same `plan add` writers; the explicit `plan add` form above stays available for manual authoring and illustrates the resulting plan shape.

A downstream slice that needs another slice's output (e.g. `omnia` consuming the `contracts` schema) declares the edge with `depends-on` at the plan layer; `specrun plan next` merges the upstream slice before the dependent starts, and the dependent target reads the upstream output from the merged working tree (see §"Target build envelope"). No multi-output, multi-target shape is added to a single slice — the plan layer is the only place fan-out happens.

## Source operation runner (D1)

### Commands

Add two commands under the existing `specify source` family:

```bash
specrun source survey <source-key> [--plan <name>] [--format json]
specrun source extract <source-key> <lead-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

### `survey`

`survey` runs the source adapter's `briefs.survey` operation under the source-adapter sandbox:


| Root              | Mode       | Contents                                                              |
| ----------------- | ---------- | --------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | Bound source path when the source uses `path:`.                       |
| `$CAPABILITY_DIR` | read-only  | Resolved source adapter manifest cache.                               |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch under `.specify/.cache/extractions/<adapter>/`. |
| `$PROJECT_DIR`    | none       | Not visible to the adapter operation.                                 |


For value-bound sources such as `intent`, `$SOURCE_DIR` is absent and the value is passed through the build request envelope.

Output is a lead set, validated against `schemas/discovery/lead.schema.json`, then merged into `discovery.md` by CLI-owned discovery helpers. Re-running `survey` for the same source replaces leads by canonical `id`, preserves operator aliases, and keeps deterministic ordering.

### `extract`

`extract` runs the source adapter's `briefs.extract` operation for one `(source-key, lead-id)` pair and writes:

```text
.specify/slices/<slice>/evidence/<source-key>.yaml
```

The CLI validates the Evidence document against `schemas/evidence.schema.json` before the write becomes visible to later synthesis. Failure leaves the slice in `refining`.

### Cache and journal

Both operations use the RFC-27 cache fingerprint model:

```text
source identity + adapter name@version + brief sha256 + sorted tool versions + lead id?
```

`lead id` is absent for `survey` and present for `extract`.

Survey cache events (`source.survey.cache-hit`, `source.survey.cache-miss`) are new; extract cache events already exist in RFC-27. Full event taxonomy: §"Journal events".

### Relationship to `specrun source preview`

The existing `specrun source preview` (`src/runtime/commands/source/preview.rs`) already resolves a source adapter, validates the `--source` path, scaffolds an `--out/evidence/` subtree, and surfaces brief paths — but it is **workflow-free**: no `.specify/` writes, no cache, no journal events, no `discovery.md` merge, and it does not dispatch the briefs (the agent runs them by hand against the prepared directory). The D1 runner is the **workflow-integrated** counterpart of that same operation. To keep one source-operation contract rather than two that drift:

- **Share the environment prep.** Factor a single internal helper — adapter resolution, brief-directory resolution (the landed RFC-35 D9 `briefs-dir`), the four-root sandbox preopen layout (`$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` / `$PROJECT_DIR`), and `evidence/` scaffolding — and have **both** `source preview` and `source survey` / `source extract` consume it. The runner is then a thin layer that adds the workflow-integrated behaviour on top of the shared prep.
- **Keep the surfaces distinct in role.** `source preview` stays the workflow-free dry run (adapter authoring / debugging, output under `--out`). `source survey` / `source extract` add the sandboxed `execution`-branched dispatch (D9), the RFC-27 cache fingerprint, the journal events, validate-before-visible, and the `discovery.md` merge (`survey`) / Evidence persist (`extract`). Equivalently, `preview` may be implemented as the `--dry-run --out <dir>` mode of the same runner so the dispatch code is literally shared.
- **Align the "which lead(s)" surface.** `source preview` already takes `--lead <id>…`; `source extract` takes a positional `<lead-id>` plus `--slice`. The shared helper should use one spelling for lead selection across the family so the `preview` → `extract` path reads consistently.

The genuinely-new machinery D1 introduces over today's `preview` — the sandbox preopens, the `executable` vs `agent-fallback` dispatch branch, the cache, the journal events, validate-before-visible, and the discovery/Evidence persistence — is what makes `survey` / `extract` workflow commands rather than a scaffolding helper; none of it should be re-implemented in `preview`.

## Lead reconciliation engine (D2)

### Two layers

There is no deterministic function from `Lead[]` across heterogeneous sources to a coherent set of slice candidates — deciding that the documentation lead `password-reset` and the legacy-code lead `reset-password` describe the same unit of work is exactly the cross-source judgment call the framework exists to make. So D2 mirrors D3: the judgment layer is agent-led and first, the projection layer is CLI-owned and deterministic.

1. **Lead-matching step (judgment, agent-led — the heart).** Cross-source reconciliation of `Lead[]` into lead groups: deciding which leads across sources describe the same unit of work, including **semantic** matches that exact id / alias / cross-reference cannot catch, declaring each group's `(source-key, lead-id)` members with a `match-basis` per member (`exact-id` | `exact-alias` | `cross-reference` | `semantic`), binding each group to exactly one target (one slice per `(group, target)` pair, per D5), authoring the per-group rationale and any `tentative` low-confidence flags the operator should eyeball, and rendering the "Lead inventory" / "Tentative merges" prose into `change.md`. This is the load-bearing judgment of plan-time fan-in and stays with the agent.
2. **Projection kernel (deterministic projection, CLI-owned).** A structural **floor** — exact id, exact alias, transitive cross-reference (rules 1–3 below) — computed deterministically and handed to the agent in the request envelope; schema validation of the returned grouping; the **global lead-partition invariant** (every surveyed lead lands in exactly one group across the whole response — no orphan or duplicate members, every cited `(source, lead-id)` exists in `discovery.md`); structural-floor preservation (the agent may *extend* a floor group with a semantic member but may not *split* a floor match); **slice-name derivation** (§"Slice-name derivation"); journal events; and the write of `plan.yaml.slices[]` through the existing `crates/workflow/src/change/plan/` writers. The kernel projects over the structure the agent returns; it never invents, drops, or re-groups leads on its own heuristic, never merges on textual similarity by itself, and never overrides a structural-floor match.

### Command

```bash
specrun plan propose --dry-run --format json          # returns the reconciliation request envelope (floor + inventory); writes nothing
specrun plan propose --from <response.json> [--format json]   # kernel: validate → partition/floor invariants → slice-name derivation → journal → plan writers
```

`propose --dry-run` reads `plan.yaml.sources`, the `discovery.md` lead inventory (via the in-place `crates/model/src/discovery/` model — `Discovery::parse` + `Discovery::resolve_lead` already cover the join surface), and optional operator-authored aliases. It writes **nothing** to disk and returns the request envelope. `propose --from` consumes the agent's grouping response and is the only writer; the agent never hand-edits `plan.yaml`.

### Structural floor (kernel)

The kernel's deterministic pre-pass is intentionally conservative — it is a *floor*, not the final grouping:

1. Exact canonical `id` match across source keys -> one floor group.
2. Exact alias match -> one floor group, recorded under the canonical id.
3. One lead's `sources` list transitively names another source's lead id (the existing `Lead.sources[]` cross-reference field) -> one floor group.
4. Otherwise each lead starts ungrouped.

The floor is a pure function of the parsed discovery document. The agent receives it pre-computed so it never has to re-derive the trivial matches and can spend its judgment on the semantic joins (rule 4 leftovers). The kernel later refuses any response that *splits* a floor group (`plan-reconcile-structural-floor-violated`); the agent may only add semantic members on top.

### Reconciliation envelope

The matching step receives a fixed-shape request and returns a fixed-shape response, dispatched to the operator's agent under `execution: agent` (the default and designed centre) or to a declared WASI tool under `execution: executable` (the D10-style mirror; see §"Synthesis execution mode (D10)" for the agent-first rationale, which applies identically here). The request:

```yaml
version: 1
kind: request
sources: [docs, legacy]
lead-inventory:
  docs:    [identity-api, password-reset]
  legacy:  [identity-api, reset-password]
structural-floor:
  - group-id: identity-api
    rule: exact-id
    members:
      - { source-key: docs,   lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
ungrouped:
  - { source-key: docs,   lead-id: password-reset }
  - { source-key: legacy, lead-id: reset-password }
bound-targets: [contracts@v1, omnia@v1]
```

The response declares the final grouping and target binding. Each row is one `(group-id, target)` pair (one plan slice per row, per D5):

```yaml
version: 1
kind: response
groups:
  - group-id: identity-api
    slice-name: identity-contracts
    members:
      - { source-key: docs,   lead-id: identity-api,   match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api,   match-basis: exact-id }
    target: contracts@v1
  - group-id: identity-api
    slice-name: identity-service
    members:
      - { source-key: docs,   lead-id: identity-api,     match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api,     match-basis: exact-id }
      - { source-key: docs,   lead-id: password-reset,   match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password,     match-basis: semantic, tentative: true }
    rationale: "identity-api floor plus semantic merge of docs 'password-reset' and legacy 'reset-password' into one omnia slice"
    target: omnia@v1
    depends-on: [identity-contracts]
```

`group-id` is a **concept id** for related work — it may repeat when the same concept fans out to multiple targets. It is not the plan slice name unless the derived name happens to equal it. The kernel derives the unique `plan.yaml.slices[]` name from optional `slice-name` or the rule in §"Slice-name derivation". `depends-on` lists **derived slice names**, not group-ids. Full request/response shape: [`rfc-29/schemas/discovery/proposal.schema.json`](rfc-29/schemas/discovery/proposal.schema.json) (`kind: request | response` discriminator), embedded as `PROPOSAL_JSON_SCHEMA`. `propose --dry-run` validates its own request output before returning; `propose --from` validates the response before projecting.

### Slice-name derivation

Each response row binds one `(group-id, target)` pair to exactly one `plan.yaml.slices[]` entry. The kernel assigns the slice name deterministically:

1. If the row carries optional `slice-name`, validate it against the slice-name grammar and use it.
2. Else if `group-id` is not already assigned as a slice name in this response, use `group-id`.
3. Else use `<group-id>-<adapter-slug>`, where `<adapter-slug>` is the adapter name segment before `@v` in `target` (e.g. `contracts@v1` → `contracts`, yielding `identity-api-contracts`).

The kernel validates every `depends-on` entry against the set of derived slice names from the same response before writing. Leads may not be legitimately dropped: every surveyed lead must appear in exactly one group **globally** across the response (`plan-reconcile-partition`).

### Match basis and operator review

`match-basis: semantic` (and any member flagged `tentative: true`) is the structured form of the "Tentative merges" Markdown block the agent renders into `change.md` for the operator. Semantic merges are the agent's judgment, surfaced for operator review at Gate 1 — the operator may accept them as-is, run `specrun plan amend --add-alias` to promote a recurring semantic match into a durable alias (so the next survey resolves it on the structural floor), or split the slice. The kernel does not adjudicate whether a semantic merge is *correct*; it only proves the grouping is a well-formed partition that respects the floor. The agent may also call `specrun plan amend --divergence likely` against any written slice whose bound leads carry materially disagreeing summaries; that writer path already exists.

### Agent role

`/spec:plan`'s `propose` sub-step:

1. Calls `specrun plan propose --dry-run --format json` to obtain the request envelope (floor + lead inventory + bound targets).
2. Matches the `ungrouped` leads across sources by judgment — extending floor groups with semantic members and forming new groups — without ever splitting a floor group.
3. Binds each group to one or more targets, expanding to one `(group, target)` slice per binding (cross-target work uses `depends-on`, per D5), and authors per-group `rationale` plus `tentative` flags.
4. Submits the grouping with `specrun plan propose --from <response.json>`, which validates, enforces the invariants, derives slice names, emits `plan.reconcile.agent`, and writes the slices through the existing plan writers.
5. Renders the semantic / `tentative` merges into `change.md` for operator review at Gate 1.

The agent never hand-edits `plan.yaml`, never writes `discovery.md` directly, and never decides authority — its scope is cross-source matching, target binding, and rationale.

## Slice synthesis engine (D3)

### Two layers

There is no deterministic function from `(design-prose, code-AST, vision-output)` to a coherent requirement set, so the engine splits cleanly into two layers — with the judgment layer first:

1. **Synthesis step (judgment, agent-led by default — the heart).** Cross-modal reconciliation of `Evidence[]` into the requirement set: deciding which requirements exist and how claims merge or split into them, declaring each requirement's `(source, claim-id)` claims and an `agreement` verdict (`agreed` when the contributors agree on value after semantic comparison, `disagreed` when they do not — the irreducibly-judgment call), authoring `requirements[].title` / `.statement` / `.scenarios[]` / `.notes`, recording which spec `unit` each requirement renders into, populating the prose fields of the rest of the model (`domain.types[].fields[].description`, `apis.surfaces[].operations[]` request/response/errors prose, `technical-logic.decisions[].statement` / `.rationale`, `observability[].description`, `tasks[].text`), and authoring **prose-only** Markdown drafts (`proposal.md`, `design.md`, `tasks.md`, and spec requirement bodies **without** `ID:` / `Sources:` / `Status:` lines). This is the load-bearing judgment of synthesis and stays with the agent.
2. **Projection kernel (deterministic projection, CLI-owned).** RFC-27 authority resolution, REQ-id assignment in the agent's declaration order, derivation of each requirement's `sources` (the unique source keys of its claims), winner-marker and `status` derivation from the agreement verdict plus authority over those claims, claim-level provenance projection into `provenance.yaml`, **kernel rendering of provenance lines into `spec.md`**, and drift validators in §"Drift validation". This is where RFC-27's authority resolver becomes production code; it projects over the structure the agent returns and never invents, drops, or re-groups requirements, never selects a winner the resolved authority did not, and never overrides the agent's agreement verdict.

The engine resolves authority, runs (1) under the envelope defined in §"Synthesis envelope", then runs (2) over the returned structure. Persist order:

1. Validate the agent response envelope and draft `model` against `synthesis-draft-model.schema.json`.
2. Reject usurped kernel fields (`generated-at`, `generator`, top-level `sources`, `requirements[].id`, `.status`, `.sources`, `claims[].winner`) with `slice-synthesize-kernel-field-usurped`; reject orphan claims with `slice-model-source-orphan`.
3. Project the kernel over the draft (ids, sources, status, winners, top-level `sources`, `generated-at`, `generator`, `provenance.yaml`).
4. Validate the merged `model.yaml` against `model.schema.json`.
5. Render kernel-owned provenance lines into `spec.md` from the projected model (§"Rendering").
6. Run drift validators and persist if clean.

The slice transitions to `refined` only after step 6 succeeds.

### Command

```bash
specrun slice synthesize <slice> [--format json]
```

The command reads:

- slice metadata and target binding;
- `plan.yaml.slices[].sources`;
- `evidence/*.yaml`;
- the bound target's `shape` brief;
- prior baseline specs when available;
- operator-authored override fields such as `authority-override`.

It produces, in order:

1. Resolved authority — per-Evidence, per-claim effective authority from `(Evidence[], authority-overrides, prior baseline)` via the RFC-27 resolver. No requirement skeleton is pre-computed; the requirement set is the synthesis step's to decide.
2. A synthesis request envelope (see below) handed to the synthesis step, carrying the Evidence map and resolved authority but **not** the bound target's `shape` brief for any requirements input.
3. After the synthesis step returns its draft model (claims, agreement verdicts, prose) and prose-only Markdown artifacts, the kernel runs the persist pipeline in §"Two layers" above.

```text
.specify/slices/<slice>/proposal.md
.specify/slices/<slice>/specs/<unit>/spec.md
.specify/slices/<slice>/design.md
.specify/slices/<slice>/tasks.md
.specify/slices/<slice>/provenance.yaml
.specify/slices/<slice>/model.yaml
```

The write is staged and validated before the slice transitions to `refined`. If any artifact fails schema validation or drift validation, the command exits non-zero and leaves the prior artifact set intact where possible.

### Production authority resolver

RFC-27's authority order becomes production code inside the projection kernel:

1. per-slice `authority-override`;
2. per-Evidence `authority-overrides`;
3. document-level `authority`;
4. tied effective authority -> `conflict`.

The micro-resolver currently pinned in tests becomes black-box coverage for the production resolver. The kernel resolves authority **before** dispatch and passes the resolution into the envelope, so the synthesis step knows which claims win when it decides what each requirement draws on; the step never re-decides authority and never marks winners itself. After the step returns its requirement set — each requirement carrying its `(source, claim-id)` claims and an `agreement` verdict — the kernel projects the resolution over those claims: it marks the winning and losing claims, derives `status` (an `agreed` verdict yields `agreed`; a `disagreed` verdict is resolved by authority into `divergence` with a unique winner, or `conflict` when the top authority class ties), derives the `sources` set, and writes `provenance.yaml`.

### Status and provenance derivation

`status` joins two responsibilities: **agreement classification** is judgment (does "30 minutes" agree with "1800 seconds"? semantic, agent-owned), while **winner selection among disagreements** is authority mechanics (deterministic, kernel-owned). The synthesis step supplies the agreement verdict per requirement; the kernel applies the resolved authority and projects the rest. The mapping is closed:

| `claims` | `agreement` | Kernel `status` | `provenance.yaml` `resolution` | Winner markers |
| --- | --- | --- | --- | --- |
| 0 | _(omitted)_ | `unknown` | `unknown-no-evidence` | none |
| 1 | _(omitted)_ | `agreed` | `single-source` | none |
| ≥2 | `agreed` | `agreed` | `single-value-agreement` | none |
| ≥2 | `disagreed`, unique top authority (or operator override) | `divergence` | `authority-resolved` / `per-slice-override` | kernel marks the authority-resolved winner `true`, every loser `false` |
| ≥2 | `disagreed`, top authority class ties | `conflict` | `tied-conflict` | none |

The agent never names the winning claim; the kernel selects it from the RFC-27 resolution it computed before dispatch, so authority resolution stays the single source of truth (RFC-27) and an agent cannot smuggle in a winner the authority order forbids. The losing claims survive in `provenance.yaml` (`winner: false`) for audit, exactly as [`provenance.md`](../../plugins/spec/references/synthesis/provenance.md) specifies.

The agent's agreement verdict is authoritative for `agreed`-vs-not — value agreement is semantic and the CLI does not adjudicate it. As a non-blocking guard, the kernel runs a cheap normalised-string inequality check over the claims of any requirement the agent marked `agreed`; a mismatch emits a `review`-kind finding (`slice-synthesize-agreement-suspect`, advisory, never a transition blocker) so an operator can eyeball a possible mislabel without the CLI re-litigating semantics.

### Authority over mixed-kind claims

RFC-27 authority is resolved **per claim kind**: the per-Evidence `authority-overrides` map is keyed by `ClaimKind`, and the per-slice `authority-override <kind>=<key>` map likewise pins a class for one kind. A single requirement, however, may draw on claims of **different kinds** (e.g. a `criterion` from `docs` disagreeing with an `example` from `legacy`). The kernel therefore resolves authority at the **claim** granularity, not by assuming one kind per requirement. Because each `model.yaml` claim now carries its own `kind` (D13), the reduction is well-defined and needs no re-read of Evidence to recover kinds:

1. **Per-claim effective class.** For each claim `(source, claim-id, kind)` the kernel computes the effective authority class by the RFC-27 order: per-slice `authority-override[kind]` (if it names this claim's source) → the claim's Evidence `authority-overrides[kind]` → the Evidence document-level `authority` → the workflow default `intent > documentation > behaviour`.
2. **Requirement winner.** Among a `disagreed` requirement's claims, the winner is the claim with the strictly-greatest effective class. The kernel marks it `winner: true` and every other claim `winner: false`, yielding `status: divergence` and `resolution: authority-resolved` (or `per-slice-override` when step 1 fired a per-slice override).
3. **Tie at the top class.** When two or more claims share the strictly-greatest effective class (regardless of kind), the requirement is a `conflict` with `resolution: tied-conflict` and no winner markers — identical to the same-class tie the single-kind table already specifies.

This makes the §"Status and provenance derivation" table's "unique top authority" / "top authority class ties" rows precise for the multi-kind case: "top authority" means the maximum **per-claim** effective class across the requirement's claims, and a per-kind override changes only the effective class of claims of that kind. The micro-resolver pinned in `crates/model/src/evidence/authority.rs` tests already resolves one kind at a time; the production kernel applies it per claim and then takes the strict maximum, so the four pinned scenarios remain black-box coverage of step 1.

### Synthesis envelope

The synthesis step receives a fixed-shape request and returns a fixed-shape response. The engine dispatches the request to the operator's agent under `execution: agent` (the default and designed centre), or to a declared WASI tool when `execution: executable` is configured (D10). Either way, the envelope is stable:

```yaml
version: 1
kind: request
slice: identity-service
target: omnia@v1
shape-brief: /.../adapters/targets/omnia/briefs/shape.md
evidence:
  docs:
    path: .specify/slices/identity-service/evidence/docs.yaml
    authority: documentation
  legacy:
    path: .specify/slices/identity-service/evidence/legacy.yaml
    authority: behaviour
authority:
  resolved-path: .specify/.cache/synthesize/<slice>/authority.resolved.yaml
prior-baseline:
  specs-dir: .specify/specs/
constraints:
  forbidden-inputs-for-requirements-reconciliation: [target, shape-brief]
```

`authority.resolved-path` carries the kernel's pre-dispatch RFC-27 resolution so the synthesis step knows which claims win without re-deciding authority. The `constraints.forbidden-inputs-for-requirements-reconciliation` field is part of the contract: a conforming synthesis step reconciles the **entire requirements section** — entries, `sources`, `statement` / `title` / `scenarios[]` / `notes` — from the Evidence map and resolved authority alone, never from `target` or `shape-brief`. (`target` and `shape-brief` remain present in the envelope because they are legitimate inputs to the non-requirements sections of the model.) This is the agent-with-envelope expression of D8's target-neutrality requirement; it is checked at the boundary by the cross-target invariant in §"Acceptance proof (D7)" and re-asserted by the synthesis prompt body shipped with first-party Specify.

The response carries a **draft model** (requirement set, per-requirement `(source, claim-id)` claims, `agreement` verdict, `unit`, prose — validated against [`synthesis-draft-model.schema.json`](rfc-29/schemas/slice/synthesis-draft-model.schema.json)) plus **prose-only** Markdown artifacts. The synthesis step does not assign REQ-ids, does not derive `sources`, does not mark winners, does not derive `status`, does not set `generated-at` / `generator`, and does not write `provenance.yaml` — those are the kernel's. The engine validates the draft, projects the kernel (id assignment in declaration order, `sources`/winner/status derivation, provenance projection), rejects any response that usurps a kernel-owned field or cites a `(source, claim-id)` absent from the Evidence map, validates the merged `model.yaml` against [`model.schema.json`](rfc-29/schemas/slice/model.schema.json), renders provenance lines into `spec.md`, then persists. Full request/response shape: [`rfc-29/schemas/slice/synthesis-envelope.schema.json`](rfc-29/schemas/slice/synthesis-envelope.schema.json) (`kind: request | response` discriminator); the response `model` `$ref`s the draft schema, not the persisted one (D3a).

### Shape-brief scope (D8)

The bound target's `shape` brief is an input to the **non-requirements sections of the synthesis step only** — the slice model's `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks` sections (e.g. surface-by-surface vs type-by-type grouping; which optional sub-fields are populated; how much narrative each design decision carries). It is never an input to the requirements section, which the synthesis step reconciles from Evidence and resolved authority alone.

Shape briefs MUST NOT influence:

- `requirements[]` — entries, ids, ordering, statements, status, scenarios, or any other field;
- `requirements[].claims`, `requirements[].agreement`, `requirements[].sources`, or any `sources` field elsewhere in the slice model;
- `domain.types[].sources`, `apis.surfaces[].operations[].sources`, `technical-logic.decisions[].sources`, or any other provenance-bearing field.

The engine enforces D8 with two deterministic gates (see §"Acceptance proof (D7)"):

1. **Envelope-construction proof** — requirements-relevant synthesis request inputs (`evidence`, resolved authority, `forbidden-inputs-for-requirements-reconciliation`) are byte-identical across target bindings.
2. **Kernel-projection determinism** — given a fixed synthesis response, kernel output (`provenance.yaml`, ids, sources, status, winners) is byte-identical and target-independent.

The `slice-synthesize-forbidden-input-leak` probe (mechanical set-difference) complements gate 1. An optional LLM-judge equivalence check over live agent runs MAY run as a non-gating `review` diagnostic but is never a release gate.

### Rendering

Synthesis persist is a three-phase pipeline:

| Phase | Author | Output |
| --- | --- | --- |
| **Synthesis step** | Agent | Draft model (`claims`, `agreement`, prose fields) + prose-only Markdown (`proposal.md`, `design.md`, `tasks.md`, spec requirement bodies without provenance lines) |
| **Projection kernel** | CLI | Full `model.yaml`, `provenance.yaml` |
| **Render step** | CLI | Injects kernel-owned `ID:` / `Sources:` / `Status:` lines (and status tags in requirement headlines) into `specs/<unit>/spec.md` from projected `model.yaml`, merging agent-authored requirement prose |

The kernel stamps `generated-at`, `generator`, top-level `sources`, `requirements[].id`, `.sources`, `.status`, `requirements[].tags`, and each claim's `winner` marker deterministically. The requirement set, claims, agreement verdicts, and behavioral prose are the synthesis step's, persisted once they pass draft validation.

`spec.md` stays the behavioral review artifact and baseline merge input. Operators may hand-edit behavioral prose after synthesis; hand-edits to kernel-rendered provenance lines without re-synthesis emit `slice-spec-provenance-stale` (§"Drift validation"). `model.yaml` is the machine view target builders consume. `provenance.yaml` remains audit-only and is always kernel-projected.

Re-running `specrun slice synthesize` overwrites `model.yaml`, `provenance.yaml`, and kernel-rendered provenance lines in `spec.md`; operator prose edits outside those lines survive only until the next full re-synthesis if the agent returns different bodies.

### Trade-offs (three provenance-bearing surfaces)

RFC-29 keeps three on-disk surfaces that encode requirement/provenance state:

- **`model.yaml`** — machine source of truth for structure, claims, agreement, and kernel-projected status/sources.
- **`spec.md`** — human review and merge input; provenance lines are **rendered from** `model.yaml`, not independently authored by the agent.
- **`provenance.yaml`** — audit index **projected from** `model.yaml` claims + authority (D11).

Bidirectional drift between three independently-authored copies was the RFC-35 failure mode. Rendering provenance into `spec.md` and projecting `provenance.yaml` from the same kernel output collapses most parity checks to internal consistency (`model` ↔ `provenance.yaml`, by construction) plus one re-sync gate when operators hand-edit provenance lines in `spec.md`.

### Standalone provenance projection (D11)

The projection kernel above is not only reachable through `specrun slice synthesize`. RFC-29 also exposes it as a standalone verb:

```bash
specrun slice provenance <slice> [--format json]
```

`specrun slice provenance` reads the slice's already-persisted `model.yaml` (the agent-authored `requirements[].claims` and `agreement` verdicts), the slice's `Evidence[]`, and the per-slice / per-Evidence authority-overrides, then runs the **identical** projection D3 wraps — RFC-27 authority resolution, winner-marker derivation, `status` derivation from the agreement verdict, and the claim-level projection into `provenance.yaml`. It writes `.specify/slices/<slice>/provenance.yaml` and nothing else. Re-running it over an unchanged `model.yaml` is byte-stable (it is the same pure function of `(model structure, Evidence[], authority-overrides)` described in §"Shape-brief scope (D8)").

Uses:

1. Regenerate `provenance.yaml` without re-synthesis after hand-editing `model.yaml` claims.
2. Single kernel module shared with `specrun slice synthesize`; natural seam for kernel-projection determinism tests (D7).

RFC-35 deferral rationale and how claim-level input resolves it: §"Relationship to RFC-35".

`specrun slice provenance` never reads the bound `target` or its `shape` brief, never re-decides the requirement set, never selects a winner the resolved authority did not, and never overrides the agreement verdict — the same kernel constraints D3 lists.

## Typed slice model (D4)

### File

```text
.specify/slices/<slice>/model.yaml
```

The slice model is generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators should edit `spec.md` or `design.md`, not `model.yaml`; re-running synthesize will overwrite `model.yaml`.

### Shape

Normative schema: [`rfc-29/schemas/slice/model.schema.json`](rfc-29/schemas/slice/model.schema.json) (lands in `specify-cli` as `schemas/slice/model.schema.json`). The slice model is closed at the top level (`additionalProperties: false`) and uses kebab-case field names on disk; required top-level fields are `version`, `slice`, `generated-at`, `generator`, `sources`, `target`, `requirements`, `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. The `project` field is optional (mirroring `plan.yaml.slices[].project`).

Sketch of the on-disk shape (illustrative; the schema is normative):

```yaml
version: 1
slice: identity-service
generated-at: 2026-05-28T05:45:00Z
generator: specrun@2.1.0
sources:
  - key: docs
    adapter: documentation
    lead: password-reset
    authority: documentation
    evidence-path: .specify/slices/identity-service/evidence/docs.yaml
target: omnia@v1
project: identity-service
requirements:
    # agent authors: title, statement, scenarios, unit, claims, agreement.
    # kernel derives: id, sources, status, and each claim's winner marker.
  - id: REQ-001
    title: Request password reset
    status: agreed
    unit: password-reset
    sources: [docs, legacy]
    agreement: agreed
    claims:
      - { source: docs,   claim-id: password-reset.request,         kind: requirement }
      - { source: legacy, claim-id: users.password-reset.request,   kind: example }
    statement: The system lets a registered user request a password reset link by email.
    scenarios:
      - Given REQ-001 and a registered email, when the user requests a reset, then the system accepts the request.
  - id: REQ-007
    title: Reset link expiry
    status: divergence
    unit: password-reset
    sources: [docs, legacy]
    agreement: disagreed
    claims:
      - { source: docs,   claim-id: password-reset.expiry, kind: criterion, winner: true }
      - { source: legacy, claim-id: password-reset.expiry, kind: example,   winner: false }
    statement: The system expires password reset links 30 minutes after issue.
    tags: [divergence]
domain:
  types: []
apis:
  surfaces: []
configuration: []
technical-logic:
  decisions: []
observability: []
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
```

### ID grammar

`model.yaml` introduces six closed three-digit id grammars in addition to the existing `REQ-NNN` from `crates/model/src/spec/provenance.rs`:


| Id         | Grammar           | Used by                                                                                 |
| ---------- | ----------------- | --------------------------------------------------------------------------------------- |
| `REQ-NNN`  | `^REQ-[0-9]{3}$`  | `requirements[].id`, plus `satisfies[]` references from operations / decisions / tasks. |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`, plus `tasks[].depends-on[]`.                                              |
| `DEC-NNN`  | `^DEC-[0-9]{3}$`  | `technical-logic.decisions[].id`.                                                       |
| `TYP-NNN`  | `^TYP-[0-9]{3}$`  | `domain.types[].id`.                                                              |
| `OP-NNN`   | `^OP-[0-9]{3}$`   | `apis.surfaces[].operations[].id`.                                                      |
| `CFG-NNN`  | `^CFG-[0-9]{3}$`  | `configuration[].id`.                                                                   |
| `OBS-NNN`  | `^OBS-[0-9]{3}$`  | `observability[].id`.                                                                   |


All seven grammars are enforced by `schemas/slice/model.schema.json`. The synthesis engine assigns ids in declaration order per section, with no cross-section reuse and no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds seven checks for RFC-29 slices:


| Finding                      | Meaning                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `slice-model-schema`            | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                     |
| `slice-spec-provenance-stale` | Kernel-rendered provenance lines in `spec.md` (`ID:`, `Sources:`, `Status:`, status tags) disagree with projected `model.yaml` — typically an operator hand-edit without re-synthesis. |
| `slice-model-provenance-drift`      | `model.yaml.requirements[].claims` disagrees with `provenance.yaml`, compared at `(source, claim-id)` granularity, for any matching `REQ-*`.                      |
| `slice-model-target-drift`      | `model.yaml.target` (or `model.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`.      |
| `slice-model-source-orphan`     | A `claims[]` entry references a `(source, claim-id)` whose source key is absent from `model.yaml.sources[].key` or whose claim id is absent from that source's Evidence.                        |
| `slice-model-cross-ref-orphan`  | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                     |
| `slice-model-claim-kind-mismatch` | A `claims[]` entry's `kind` (D13) disagrees with the kind recorded for that `(source, claim-id)` in the source's Evidence.                                     |


Absence of `model.yaml` is allowed for pre-RFC-29 slices and rejected for slices synthesized by an RFC-29-aware CLI.

### Build input

Target builders consume `model.yaml` as their machine input and may also read rendered Markdown for behavioral context. **`model.yaml` is authoritative for structure and provenance** (ids, status, sources, claims). **`spec.md` is authoritative for behavioral prose** after operator review. Kernel-rendered provenance lines in `spec.md` must match `model.yaml` or `slice-spec-provenance-stale` fires; operators who need to change provenance outcomes edit claims/agreement in a re-synthesis path or amend authority overrides — not provenance lines directly.

## Per-slice fan-out (D5)

Cross-target fan-out is **plan-level**, not slice-level (D5; [decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project)).

### Plan schema

Unchanged. Each `plan.yaml.slices[]` entry continues to carry exactly one `target` and (optionally) one `project`:

```yaml
slices:
  - name: identity-contracts
    status: pending
    target: contracts@v1
    project: identity-contracts
    sources:
      - key: docs
        lead: identity-api
  - name: identity-service
    status: pending
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - key: docs
        lead: identity-api
      - key: legacy
        lead: identity-api
```

The same `Lead` may appear in more than one slice's `sources[]` when both slices need the same Evidence — this is the fan-in side, not fan-out. Lead reconciliation (D2) proposes one slice per `(target, lead-group)` pair; the operator may split or merge proposed slices at Gate 1.

### Lifecycle

Unchanged. Each slice walks the existing single-state lifecycle:

```text
refining -> refined -> built -> merged
```

Per-slice build detail in `.metadata.yaml`:

```yaml
target: omnia@v1
status: built
generated-paths:
  - crates/identity_service
build-report-path: build/report.yaml
```

`/spec:build` transitions the slice to `built` when the target's build report validates with `status: success`. `/spec:merge` performs one delta merge against one baseline (the slice's bound target / project).

### Workspace routing

Unchanged from RFC-25. `/spec:build` for a workspace-routed slice resolves the slice's `project` against the registry, prepares that project slot, writes target-specific files, records generated paths in the build report, and restores CWD to the workspace root. The plan lock stays at the workspace root. Cross-slice ordering — e.g. building `identity-contracts` before `identity-service` because the latter `depends-on` the former — is enforced by `specrun plan next`, not by anything inside a slice.

## Target build envelope (D6)

The build request and build report are both closed-shape YAML envelopes, keyed on `(slice, target)`. Normative schemas — `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` — are authored during implementation (M3, Wave E). Examples below are illustrative.

### Build request

`/spec:build` constructs one build request per slice and either pipes it on stdin to a declared WASI tool (when the target's `execution: executable`) or writes it to `.specify/slices/<slice>/build/request.yaml` (when `execution: agent-fallback`):

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
  mode: executable
  tool:
    name: omnia
    version: v1.4.2
cache-fingerprint: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

A slice builds against the **merged working tree**, not against a cross-slice data channel. When a slice depends on another slice's output — e.g. `identity-service` (omnia) consuming `identity-contracts`' generated schema — the dependency is declared at the plan layer via `plan.yaml.slices[].depends-on`, and `specrun plan next` orders execution so the depended-on slice reaches `done` (merged into the baseline) before the dependent slice starts. The dependent target then reads the upstream output from the working tree as ordinary files, the way its build tooling already resolves dependencies. No per-request cross-slice channel is introduced; see §"Open questions" for the deferred cross-project (workspace-mode) artifact-handoff case.

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

### First-party target migration

The first migration path should be:

1. `contracts` first, because API contracts are already structured outputs.
2. `omnia` second, because Rust crate generation benefits most from typed requirements, APIs, configuration, and replay examples.
3. `vectis` third, because UI layout, assets, tokens, and `composition.yaml` need the widest slice-model shape.

## Adapter execution mode (D9)

Source and target adapters declare a closed `execution` field on their respective `adapter.yaml`:

```yaml
# adapters/sources/<name>/adapter.yaml
execution: executable     # or `agent-fallback`
```

The two values are:

- `**executable**` — `survey` and `extract` (sources) or `build` and `merge` (targets) are dispatched through a declared WASI tool or a deterministic Rust adapter path. Inputs and outputs validate against the schemas committed in this RFC. Required for first-party adapters before RFC-29 ships.
- `**agent-fallback**` — the adapter's brief is executed by an agent against the same sandbox preopens. The CLI orchestrates inputs and validates outputs against the same schemas, but does not cache the result. Permitted for third-party adapters indefinitely.

When `execution: agent-fallback`, the CLI:

1. emits a `source.execution.agent-fallback` (sources) or `target.execution.agent-fallback` (targets) journal event on every operation invocation;
2. forces `cache: opt-out` regardless of the adapter's declared cache mode (rejected at parse time as `adapter-execution-agent-fallback-cache-conflict` if the manifest declares any other cache mode);
3. surfaces a `suggestion`-severity `adapter-execution-agent-fallback` finding on the framework standards layer for first-party adapters, and not at all for third-party adapters.

The schema additions are mechanical extensions of `schemas/source.schema.json` and `schemas/target.schema.json`:

```json
{
  "execution": {
    "type": "string",
    "enum": ["executable", "agent-fallback"],
    "description": "Closed adapter execution mode per RFC-29 D9."
  }
}
```

with `execution` added to the `required` list on both schemas. Manifests authored before RFC-29 must add the field at first read; the loader rejects missing values rather than defaulting silently.

## Synthesis execution mode (D10)

The synthesis step inside `specrun slice synthesize` carries a closed `execution: agent | executable` enum. It deliberately does **not** mirror the adapter enum (D9): adapters aspire to `executable` and treat the agent path as a fallback, whereas synthesis is **agent-first by design**. Cross-modal Evidence reconciliation into a requirement set is the load-bearing judgment of the framework, so `agent` is the default and the designed centre — the two values are named as first-class peers, with no "fallback" connotation on the agent path. An `execution: executable` path is optional, reserved for future declared synthesis tools that admit narrow deterministic cases (e.g. single-source slices where Evidence already carries statement-quality prose).

The configuration lives on the workspace, not on individual adapter manifests, because the synthesis step is core-owned and per-slice:

```yaml
# project.yaml (one entry per project; defaults to agent)
synthesize:
  execution: agent     # or `executable`
```

The two values are:

- `**agent**` — the engine resolves authority, hands the synthesis envelope to the operator's agent, validates the draft response, projects the kernel, validates the merged `model.yaml`, renders provenance into `spec.md`, runs drift validators, and persists. This is the first-party default and the designed centre of synthesis.
- `**executable**` — the engine additionally requires a declared synthesis WASI tool to be configured (`synthesize.tool: { name, version }`), pipes the envelope on stdin, projects and validates the returned response identically, and caches the result under a synthesis-specific fingerprint (Evidence sha256 set + authority-overrides + shape-brief sha256 + tool `name@version`). Optional and reserved for narrow deterministic cases.

When `execution: agent`, the engine:

1. emits a `slice.synthesize.agent` journal event on every invocation;
2. forces `cache: opt-out` for the synthesis step (the kernel's projection over the returned structure remains deterministic, and `provenance.yaml` is reproducible from a fixed response under a kernel-only fingerprint of structure + Evidence + authority-overrides);
3. surfaces no finding by default — `agent` is the expected and recommended mode for cross-modal slices. A `suggestion`-severity `slice-synthesize-agent-mode` finding is raised only when an operator has explicitly opted in to tool-only enforcement (`synthesize.enforce-executable: true`), which is itself an unusual choice the framework does not encourage for cross-modal synthesis.

Regardless of execution mode, the engine validates the draft response against `synthesis-draft-model.schema.json`, the merged result against `model.schema.json`, and the drift checks before the slice transitions to `refined`. The execution mode does not relax any validation; it only changes who authors the requirement set and prose.

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path — fan-in twice (Leads and Evidence), fan-out once (across slices):

```text
documentation + code-typescript
        -> source survey                 (fan-in #1: Lead sets)
        -> plan propose --dry-run           (kernel returns the structural floor + lead inventory)
        -> plan propose --from              (envelope: agent-led cross-source matching — structural
                                             + semantic — and per-group target binding;
                                             kernel: validate, partition/floor invariants, group-id
                                             assignment, journal, plan writers)
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

- `specrun source survey` produces schema-valid leads for both sources, including a deliberate semantic-only pair (`docs` lead `password-reset` and `legacy` lead `reset-password`) that shares no id, alias, or cross-reference.
- `specrun plan propose --dry-run --format json` returns a `kind: request` envelope whose `structural-floor` contains one group for the shared `identity-api` lead (`rule: exact-id`) and whose `ungrouped` list contains the `password-reset` / `reset-password` pair; it validates against `proposal.schema.json` and writes nothing.
- The fixture's `/spec:plan` agent step (or the test harness simulating it) returns a `kind: response` that preserves the `identity-api` floor group, **semantically merges** `password-reset` with `reset-password` (`match-basis: semantic`) into the `identity-service` row, and binds rows to targets with explicit `slice-name` values (`identity-contracts`, `identity-service`); `specrun plan propose --from` writes two single-target slices with `identity-service.depends-on: [identity-contracts]`, emits `plan.reconcile.agent` + `plan.reconcile.completed`, and the semantic merge is rendered into `change.md` for Gate-1 review.
- `specrun plan propose --from` rejects a response that splits the `identity-api` floor group (`plan-reconcile-structural-floor-violated`), one that leaves a surveyed lead unaccounted for or double-counts one (`plan-reconcile-partition`), and one that cites a `(source-key, lead-id)` absent from `discovery.md` (`plan-reconcile-lead-orphan`); `specrun plan propose` with neither `--dry-run` nor `--from` exits non-zero with `plan-propose-missing-grouping`.
- `specrun source extract` writes schema-valid Evidence for every `(slice, source)` pair.
- `specrun slice synthesize` writes valid artifacts, `provenance.yaml`, and `model.yaml` for each slice.
- `specrun slice validate` catches no provenance or slice-model drift on either slice.
- Each slice builds independently against its single bound target; `identity-service` reads `identity-contracts`' merged output from the working tree (the dependency is ordered by `depends-on` + `plan next`, not carried on the build request).
- `specrun plan next` orders execution so `identity-contracts` reaches `merged` before `identity-service` starts.
- **Kernel-projection determinism.** Re-run kernel projection twice over a golden synthesis response; `provenance.yaml` and kernel-owned `model.yaml` fields are byte-identical and target-independent (D11). Live agent runs are not byte-stable on requirement set or prose.
- **D8 envelope-construction proof.** Synthesis request requirements-relevant inputs are byte-identical across `contracts@v1` and `omnia@v1` bindings; `target` / `shape-brief` differ only in non-requirements fields.
- **Forbidden-input-leak probe (deterministic).** A fixture-local test confirms the envelope walls `target` and `shape-brief` off from the requirements section: a probe response whose requirements section contains a token present in `target` or the `shape-brief` file but in **no** cited Evidence claim is flagged by `slice-synthesize-forbidden-input-leak` via a mechanical set-difference test (not a semantic judgement), proving the target-neutrality-by-construction layer of D8.
- **Synthesis envelope contract.** A fixture-local test re-runs `specrun slice synthesize` with a deliberately-malformed synthesis-step response that usurps a kernel-owned field — pre-assigns a `REQ-NNN` id, sets `status`, `sources`, or a per-claim `winner`, sets `generated-at` / `generator`, or cites a `(source, claim-id)` absent from the Evidence map. The engine rejects the draft with `slice-synthesize-kernel-field-usurped` (kernel fields) or `slice-model-source-orphan` (orphan claim) **before** projection, proving the kernel is the sole authority on id assignment, `sources` derivation, winner selection, status derivation, and provenance projection while the agent remains the sole author of the requirement set, its claims, and its agreement verdict.

## Schemas added by this RFC

Four new JSON Schemas ship as draft files alongside this RFC under [`rfc-29/schemas/`](rfc-29/schemas/); the two build-envelope schemas (D6) are authored during implementation (M3, Wave E) rather than shipped as drafts. Implementation copies the draft files into `specify-cli/schemas/` and embeds all six in `specify-schema` as `SLICE_MODEL_JSON_SCHEMA`, `SYNTHESIS_DRAFT_MODEL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, and `SYNTHESIS_ENVELOPE_JSON_SCHEMA`. **`model.schema.json`, `synthesis-draft-model.schema.json`, and `synthesis-envelope.schema.json` MUST be registered together** so relative `$ref`s compile without a registry lookup (same discipline as the adapter loader's inlined `$defs`). Field names are kebab-case on disk; top-level shapes are closed (`additionalProperties: false`).

| Schema | RFC draft path | `specify-cli` path | Embed constant | Used by |
| --- | --- | --- | --- | --- |
| Slice model (persisted) | [`slice/model.schema.json`](rfc-29/schemas/slice/model.schema.json) | `schemas/slice/model.schema.json` | `SLICE_MODEL_JSON_SCHEMA` | Post-projection `model.yaml`; D4; D6 build input |
| Synthesis draft model | [`slice/synthesis-draft-model.schema.json`](rfc-29/schemas/slice/synthesis-draft-model.schema.json) | `schemas/slice/synthesis-draft-model.schema.json` | `SYNTHESIS_DRAFT_MODEL_JSON_SCHEMA` | D3a agent response `model` |
| Build request | _authored in M3 (Wave E)_ | `schemas/target/build-request.schema.json` | `BUILD_REQUEST_JSON_SCHEMA` | D6 |
| Build report | _authored in M3 (Wave E)_ | `schemas/target/build-report.schema.json` | `BUILD_REPORT_JSON_SCHEMA` | D6 |
| Reconciliation envelope | [`discovery/proposal.schema.json`](rfc-29/schemas/discovery/proposal.schema.json) | `schemas/discovery/proposal.schema.json` | `PROPOSAL_JSON_SCHEMA` | D2 (request + response) |
| Synthesis envelope | [`slice/synthesis-envelope.schema.json`](rfc-29/schemas/slice/synthesis-envelope.schema.json) | `schemas/slice/synthesis-envelope.schema.json` | `SYNTHESIS_ENVELOPE_JSON_SCHEMA` | D3, D10 |

All slice-model, build-request, and build-report schemas key on `(slice, target)` per D5 — none carries `outputs[]` or `output-id`. `proposal.schema.json` discriminates request vs response via closed `kind: request | response`: the request carries the lead inventory and the deterministic structural floor, the response carries `(group-id, target)` rows (members with `match-basis`, optional `slice-name`, optional `tentative` flags and `depends-on` in derived slice names). `synthesis-envelope.schema.json` discriminates request vs response via closed `kind: request | response`; the response `model` `$ref`s **`synthesis-draft-model.schema.json`**, not the persisted model (D3a).

## Journal emitter (D12)

Deterministic commands emit their own events; agent-orchestrated steps (D2/D9/D10 agent paths, agent-driven build/merge) use the guarded emitter below. Why RFC-35 rejected this verb and why RFC-29 adds it: §"Relationship to RFC-35".

RFC-29 introduces:

```bash
specrun journal emit <event-id> [--payload <json>] [--format json]
```

The emitter is deliberately thin and closed:

- `<event-id>` must be a member of the closed `EventKind` taxonomy in `crates/workflow/src/journal.rs`; an unknown id is rejected with `journal-emit-unknown-event` (exit 2).
- `--payload` is validated against the per-kind payload shape before the line is appended; a payload that fails its kind's required fields is rejected with `journal-emit-payload-schema` (exit 2).
- The CLI stamps the `timestamp` (second-precision UTC) and appends one well-formed line to `.specify/journal.jsonl`. The agent never composes the envelope, the timestamp, or the wire id by hand.

This keeps a **single emission path and a single closed taxonomy**: deterministic commands and the agent-facing verb both write the same `Event` shape through the same writer, so there is no second NDJSON format to drift. The emitter adds no new event kinds of its own — it is purely a guarded front door onto the kinds defined below.

## Journal events

The closed `Event` / `EventKind` taxonomy in `crates/workflow/src/journal.rs` gains the following kebab-case event kinds. Wire ids are normative; Rust variants follow the existing `#[serde(rename = …)]` pattern. Both deterministic commands and the D12 `specrun journal emit` verb write them through the one closed taxonomy.


| Event                             | When                                                                                                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `source.survey.cache-hit`      | Lead set was read from cache.                                                                                                                           |
| `source.survey.cache-miss`     | Source-adapter `survey` ran.                                                                                                                              |
| `source.execution.agent-fallback` | A source-adapter operation ran in `agent-fallback` mode (`survey` or `extract`).                                                                          |
| `plan.reconcile.agent`         | The lead-matching step (D2) ran in `agent` mode (the default and designed centre). One event per `specrun plan propose --from` invocation.              |
| `plan.reconcile.completed`     | `specrun plan propose --from` validated the agent grouping, enforced the partition / structural-floor invariants, derived slice names, and wrote `plan.yaml.slices[]`. |
| `slice.extract.cache-hit`         | (Existing) Evidence was read from cache.                                                                                                                     |
| `slice.extract.cache-miss`        | (Existing) Source-adapter `extract` ran.                                                                                                                     |
| `slice.extract.completed`         | (Existing) Evidence file was successfully persisted.                                                                                                         |
| `slice.synthesize.started`        | `specrun slice synthesize` began for a slice.                                                                                                                |
| `slice.synthesize.authority-resolved` | The projection kernel resolved RFC-27 authority over `Evidence[]`. The synthesis envelope is about to be dispatched. No requirement skeleton is pre-computed. |
| `slice.synthesize.agent`          | The synthesis step ran in `agent` mode (the first-party default and designed centre). One event per invocation.                                                |
| `slice.synthesize.completed`      | `specrun slice synthesize` finished and all artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, `provenance.yaml`, `model.yaml`) validated and persisted. |
| `slice.synthesize.failed`         | `specrun slice synthesize` aborted; prior artifacts left intact where possible.                                                                              |
| `slice.build.started`             | `/spec:build` (or `specrun slice build`) began work on a slice.                                                                                              |
| `slice.build.succeeded`           | A slice's build report validated with `status: success`.                                                                                                     |
| `slice.build.failed`              | A slice's build report carried `status: failure` or failed schema validation.                                                                                |
| `slice.merge.started`             | `/spec:merge` began work on a slice.                                                                                                                         |
| `slice.merge.succeeded`           | A slice's merge report validated with `status: success`.                                                                                                     |
| `slice.merge.failed`              | A slice's merge report carried `status: failure` or failed schema validation.                                                                                |
| `target.execution.agent-fallback` | A target-adapter operation ran in `agent-fallback` mode.                                                                                                     |
| `slice.model.show.requested`         | Operator invoked `specrun slice model show` (audit-only; useful for measuring model-consumer adoption).                                                         |


## Error discriminants and exit codes

`Exit::from(&Error)` in `src/runtime/output.rs` is the single source of truth for the wire contract; this RFC adds the closed `Error` variants below. The CLI dispatch table maps each one to a fixed exit code.


| Error variant (kebab-case discriminant)           | Exit | Cause                                                                                                   |
| ------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------- |
| `slice-model-schema`                                 | 2    | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                |
| `slice-spec-provenance-stale`                      | 2    | Kernel-rendered provenance lines in `spec.md` disagree with projected `model.yaml` (operator hand-edit or stale render).                                     |
| `slice-model-provenance-drift`                           | 2    | `model.yaml.requirements[].claims` disagrees with `provenance.yaml` at `(source, claim-id)` granularity.                                          |
| `slice-model-target-drift`                           | 2    | `model.yaml.target` (or `model.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`. |
| `slice-model-source-orphan`                          | 2    | A `claims[]` entry references a `(source, claim-id)` whose source key is absent from `model.yaml.sources[].key` or whose claim id is absent from that source's Evidence.                   |
| `slice-model-cross-ref-orphan`                       | 2    | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                |
| `slice-model-claim-kind-mismatch`                    | 2    | A `claims[]` entry's `kind` (D13) disagrees with the kind recorded for that `(source, claim-id)` in Evidence. |
| `slice-model-id-grammar`                             | 2    | A REQ / TASK / DEC / TYP / OP / CFG / OBS id does not match its closed three-digit grammar.             |
| `target-build-request-schema`                     | 2    | A build request fails `schemas/target/build-request.schema.json`.                                       |
| `target-build-report-schema`                      | 2    | A build report fails `schemas/target/build-report.schema.json`.                                         |
| `target-build-success-with-critical-finding`      | 2    | A build report sets `status: success` while carrying a finding at severity `critical`.                  |
| `adapter-execution-mode-required`                 | 2    | An adapter manifest does not declare `execution`.                                                       |
| `adapter-execution-agent-fallback-cache-conflict` | 2    | An adapter manifest sets `execution: agent-fallback` together with any cache mode other than `opt-out`. |
| `plan-reconcile-lead-orphan`                      | 2    | A `specrun plan propose --from` response cites a `(source-key, lead-id)` absent from the surveyed `discovery.md` lead inventory. |
| `plan-reconcile-partition`                        | 2    | A `specrun plan propose --from` response is not a well-formed **global** partition: a surveyed lead is unaccounted for, or a `(source-key, lead-id)` appears in more than one group. |
| `plan-reconcile-structural-floor-violated`        | 2    | A `specrun plan propose --from` response splits a deterministic structural-floor group (exact id / alias / cross-reference). The agent may extend a floor group with semantic members but may not split it. |
| `plan-propose-missing-grouping`                   | 2    | `specrun plan propose` was invoked without `--dry-run` and without `--from`; one of the two modes is required. |
| `slice-synthesize-execution-mode-required`        | 2    | A workspace declares `synthesize.execution: executable` without configuring `synthesize.tool: { name, version }`. |
| `slice-synthesize-kernel-field-usurped`           | 2    | A synthesis-step draft set a kernel-owned field it does not author — top-level `generated-at` or `generator`, a `requirements[].id` (`REQ-NNN`), `requirements[].status`, `requirements[].sources`, or a `claims[].winner` value. The engine derives ids, sources, winners, status, and metadata; it rejects the draft before projection. (Orphan `(source, claim-id)` claims are caught separately by `slice-model-source-orphan`.) |
| `slice-synthesize-forbidden-input-leak`           | 2    | A synthesis-step response's requirements section (entries, `claims`, `agreement`, `statement`, `title`, `scenarios`, `notes`) demonstrably referenced `target` or `shape-brief` content (detected by fixture-local target-neutrality probes). |
| `journal-emit-unknown-event`                      | 2    | `specrun journal emit` (D12) was given an `<event-id>` that is not a member of the closed `EventKind` taxonomy. |
| `journal-emit-payload-schema`                     | 2    | `specrun journal emit` (D12) was given a `--payload` that fails the named event kind's required-field shape. |


`EXIT_VALIDATION_FAILED = 2` is the only new code RFC-29 needs. Adapter resolution failures, sandbox preopen failures, WASI tool runtime failures, and I/O errors keep the existing `EXIT_GENERIC_FAILURE = 1` mapping.

## Implementation plan

A PR-sized breakdown of these waves lands in companion milestone plans [`rfc-29-m1-plan.md`](rfc-29-m1-plan.md) and [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md) (mirroring the [rfc-34-core-rules.md](./rfc-34-core-rules.md) / [rfc-34-plan.md](./rfc-34-plan.md) split). Each wave owns a defined set of new schemas, error variants, and journal events from the tables above.

### Landing as independently shippable milestones

RFC-29 is large — it spans an executable source runner, an agent-led lead-reconciliation engine, a synthesis kernel, a typed slice model, a build envelope, and several new verbs. It is **not** meant to land as one PR or even one branch. The waves below group into three **independently shippable milestones**, each of which is a defensible release on its own and each of which gets its **own** `rfc-29-<milestone>-plan.md` companion (same `rfc-34` precedent). A milestone may graduate into its own numbered RFC if its scope or open questions grow; this document stays the source of truth for the contracts they share.

| Milestone | Waves / decisions | Lands independently because… | Unblocks |
| --- | --- | --- | --- |
| **M1 — Executable source operations** | Wave A (D1, D9 source side, D12 emitter) | `specrun source survey` / `extract` are useful the day they ship — they make `/spec:refine` extraction CLI-owned and give acceptance (RM-05) a durable seam — without depending on synthesis or build changes. | RM-05 durable proof; M2 inputs. |
| **M2a — Lead reconciliation** | Wave B (D2) | `specrun plan propose` closes plan-time fan-in without synthesis or `model.yaml`. Preconditions: D2 slice-name derivation pinned (§"Slice-name derivation"); `proposal.schema.json` + worked example aligned. | Plan-time fan-in contract; M2b plan rows. |
| **M2b — Synthesis kernel + typed model** | Wave C (D3, D3a, D4, D8, D10, D11, D13), Wave D (D5 confirmation) | Slice synthesis, draft/persisted model split, kernel rendering into `spec.md`, and drift validators form one contract over Evidence the agent already produces; consumes M1's surveys/Evidence but not the build envelope. Preconditions: `synthesis-draft-model.schema.json` + rendering pipeline pinned (§"Rendering"). | RM-11 machine-readable producer/consumer impact; M3 build input. |
| **M3 — Target build envelope** | Wave E (D6, D9 target side), Wave F (D7, docs) | The build request/report and the first-party target migrations consume `model.yaml` from M2b; the end-to-end D7 fixture is the final release gate that proves fan-in twice and fan-out once. | RM-18 hosted execute; the RFC-29 acceptance proof. |

Ordering is **M1 → M2a → M2b → M3** (each consumes the prior), but each milestone is reviewable, testable, and releasable on its own; M1 does not wait for M2 design to settle. The shared contracts that must stay stable across milestone boundaries are the six draft schemas in [`rfc-29/schemas/`](rfc-29/schemas/), the closed `EventKind` additions, and the `Error` discriminants — all pinned in this document so a later milestone cannot silently redefine an earlier one's wire shape.

### Readiness (pre-implementation)

| Milestone | Readiness |
| --- | --- |
| **M1** | **Ready.** D1/D9-source/D12 are well-specified; `source preview` reuse plan is sound. See [`rfc-29-m1-plan.md`](rfc-29-m1-plan.md). |
| **M2a** | **Ready after this revision.** D2 identity model, `slice-name` derivation, and `targetRef` unification are pinned above. See [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md) Wave B. |
| **M2b** | **Ready after this revision.** Draft vs persisted model split (D3a), rendering pipeline, and D13 read-path guarantee are pinned above. See [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md) Waves C–D. |
| **M3** | **Deferred by design.** Build envelopes authored during M3 implementation. |

### Wave A — Source runner (D1, D9 source, D12)

Ship `specrun source survey` / `extract`, adapter `execution` enum, survey cache events, and `specrun journal emit`. Factor the shared source-operation environment prep (adapter resolve, `briefs-dir`, sandbox preopens, `evidence/` scaffolding) so `source preview` and the new runners consume one helper rather than forking (§"Relationship to `specrun source preview`"). Details in companion `rfc-29-m1-plan.md`.

### Wave B — Lead reconciliation engine (D2)

Deterministic structural floor + reconciliation envelope (`proposal.schema.json`, request + response), `specrun plan propose --dry-run` (floor + inventory) and `specrun plan propose --from` (kernel: validate, global partition / floor invariants, slice-name derivation, `plan.reconcile.*` events, plan writers). Agent-led matching + binding by design; optional lead target-axis hints deferred (§"Open questions" Q1). **M2a milestone.** Details in [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md) Wave B.

### Wave C — Synthesis kernel + typed model (D3, D3a, D4, D8, D10, D11, D13)

Copy schemas from [`rfc-29/schemas/`](rfc-29/schemas/) (register `model`, `synthesis-draft-model`, and `synthesis-envelope` together); ship projection kernel, draft validation, kernel render into `spec.md`, `specrun slice synthesize` / `provenance`, drift validators. Enforce D8 gates per §"Acceptance proof (D7)". **M2b milestone.** Details in [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md) Wave C.

### Wave D — Plan loader confirmation (D5)

Parser regression: reject stray `outputs[]` on `plan.yaml.slices[]`; confirm singular `--target` binding.

### Wave E — Target build envelope (D6, D9 target)

Author the `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` envelopes; migrate first-party targets (`contracts`, `omnia`, `vectis`). Cross-slice dependencies build against the merged working tree in `depends-on` order (`plan next`); no per-request cross-slice channel ships in v1. Details in `rfc-29-m3-plan.md`.

### Wave F — Proof fixtures and docs

1. Add the RFC-29 end-to-end fixture (D7): two slices over two sources, joined by `depends-on`, each binding one target. Assert D8 envelope-construction proof and kernel-projection determinism per §"Acceptance proof (D7)".
2. Update `docs/explanation/concepts.md` and `docs/explanation/adapter-anatomy.md` to distinguish source fan-in (Leads + Evidence) from slice fan-out (plan-level decomposition with `depends-on`). Reaffirm "one slice, one target" alongside the existing `docs/explanation/decision-log.md` entry.
3. Update CLI reference pages for source, plan, slice, and target build reports — none of them gain an `outputs[]` field.
4. Update acceptance docs with the new proof command sequence (two `specrun plan add` calls, one per target, second with `--depends-on`).

## Migration

Existing projects continue to work without any change to `plan.yaml`:

- `plan.yaml.slices[]` keeps one `target`, optional `project` (D5).
- Slices without `model.yaml` validate under the pre-RFC-29 compatibility path unless re-synthesised.
- Target build briefs may initially read Markdown and ignore `model.yaml`, but first-party targets must migrate before RFC-29 is marked implemented.
- Source adapters may initially keep agent-run briefs, but first-party adapters must declare `execution: executable` before RFC-29 is marked implemented. Third-party adapters MAY remain `execution: agent-fallback` indefinitely.
- Existing first-party adapter manifests must add the new `execution` field at first read; the loader rejects missing values with `adapter-execution-mode-required` rather than defaulting silently. The companion `rfc-29-plan.md` PR list pins which adapters land each migration.
- Synthesis defaults to `execution: agent` (D10); `/spec:refine` shells out to `specrun slice synthesize`.
- `provenance.yaml` schema unchanged; D3/D11 projection replaces hand-authoring. See §"Relationship to RFC-35".
- **Claim contract (D13).** `schemas/evidence.schema.json` is tightened so `claim-id` is required on **every** claim kind (it was previously required only on `requirement` / `criterion` / `example`). This is the one breaking schema change RFC-29 makes to an already-landed artifact: any source adapter that emits an id-less claim of another kind must add a `claim-id` before re-extracting. First-party adapters already emit ids for the kinds that feed requirements; the change formalises it so every `(source, claim-id)` cited by a requirement resolves. **Existing Evidence that predates the tightening keeps validating until re-extracted** (the stricter rule applies on the next `specrun source extract` write path). **`specrun slice synthesize` and `specrun slice provenance` read Evidence without re-validating persisted files against the tightened schema** — only the extract write path and explicit validate commands apply D13 on ingest. `model.yaml` only ever cites claims; a slice re-synthesised by an RFC-29-aware CLI carries `kind` on every claim and is checked by `slice-model-claim-kind-mismatch`.

Once a slice has been synthesized by an RFC-29-aware CLI, `model.yaml` becomes required for that slice and drift validation applies.

## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No graph database or global knowledge store for synthesis.
- No *kernel-side* heuristic auto-merge of semantically similar leads. Semantic cross-source matching is the agent's judgment under the D2 envelope (`match-basis: semantic`), surfaced for operator review at Gate 1; the CLI kernel never merges leads on textual similarity by itself, only validates the agent's grouping against the partition and structural-floor invariants.
- **No multi-target slices (D5).** Fan-out is plan-level via `depends-on`; reopening requires amending the decision log.
- No target-specific behavior in the projection kernel (D8).
- No deterministic requirement reconciliation (D3/D10). Kernel projection and envelope-construction proof only; see §"Acceptance proof (D7)".
- No CLI adjudication of semantic value agreement. The `agreement` verdict is the agent's; the kernel applies authority to it but never re-decides whether two claim values mean the same thing. The advisory `slice-synthesize-agreement-suspect` finding is a non-blocking nudge, not a semantic judge.
- No commitment to per-target determinism on day one. RFC-29 commits only to a stable build envelope and validation contract; per-target determinism milestones are tracked in each target adapter's manifest and changelog.

## Relationship to RFC-35

[RFC-35](done/rfc-35-synthesis-determinism.md) has **landed**. It corrected synthesis references, sharpened `specrun slice validate` diagnostics, and added `briefs-dir` to resolve output. It **deferred** `specrun slice provenance` and **rejected** `specrun journal emit`. RFC-29 owns both (D11, D12) and reuses `briefs-dir`.

**Provenance (D11).** RFC-35 kept `provenance.yaml` agent-authored because claim→requirement mapping is not recoverable from `Sources:` lines alone. RFC-29's synthesis response carries per-requirement `(source, claim-id)` claims and an `agreement` verdict on `model.yaml`, so the kernel projects `provenance.yaml`, `sources`, winners, and `status` faithfully. Agreement is judgment (agent); winner selection among disagreements is authority mechanics (kernel) — see §"Status and provenance derivation". `specrun slice provenance` is the standalone entry point onto the same kernel as D3; the on-disk provenance schema is unchanged.

**Journal emitter (D12).** RFC-35 rejected a generic emitter because deterministic commands can emit directly. RFC-29 introduces agent-orchestrated phases (D9/D10) without such a command at emit time, so `specrun journal emit` is the guarded front door onto the closed `EventKind` taxonomy (§"Journal emitter (D12)").

## Open questions

Two questions are deliberately left open. RFC-29 does **not** answer either in v1 and is fully implementable without them — each v1 decision is already pinned, and the open questions are prerequisites for deferred follow-on work.

**Q1. Optional lead target-axis *hints* (agent assist, not replacement).** D2 makes cross-source matching and target binding the agent's judgment by design — the same stance D10 takes for synthesis — so there is no deterministic "full writer" to gate on, and no arbitrary CLI heuristic to design. The only open question is whether leads should *optionally* carry target-axis hints to **assist** the agent (narrowing the candidate targets it considers), never to replace its binding. Options considered:

   - **(a) Optional target hints on Leads.** Source adapters tag each lead with a closed `axes: [api, service, ui, …]` enum at `survey` time; the request envelope surfaces them so the agent's binding starts from a narrowed candidate set. Cleanest assist; requires extending `schemas/discovery/lead.schema.json` and per-source-adapter authoring discipline. Probably needs its own RFC.
   - **(b) No hints — pure agent binding (the v1 decision).** The agent binds each group to a target from `bound-targets` using the lead summaries and rationale alone, exactly as it semantically matches leads. Honest about the judgment involved and consistent with the D2/D3 agent-first stance; the journal records every binding via `plan.reconcile.agent`, so binding is *not* outside the audit trail.

   v1 ships option (b): agent binding is the designed centre, not a stopgap. Option (a) is a **purely additive optimization** — an optional hint that prunes the agent's candidate set — and is deferred to a future RFC once a lead target-axis vocabulary is designed. It blocks no RFC-29 wave; D1, D2, D3, D4, D5, D6, and D7 all land against the agent-binding form.

**Q2. Cross-project artifact handoff (workspace mode).** When two slices in a fan-out are bound to *different projects* (the worked example binds `identity-contracts` and `identity-service` to separate `--project` slots), the dependent slice's target cannot read the upstream output from a shared working tree — workspace mode materialises each project into an independent `.specify/workspace/<project>/` slot synced and pushed on its own branch, with no build-time cross-slot artifact path. The idiomatic channel is a published package / schema artifact (a versioned `contracts` crate, npm package, or schema-registry entry) that the dependent project consumes through its own dependency manifest, exactly as it would consume any third-party dependency. v1 deliberately ships **no** Specify-specific cross-slice channel: an earlier draft's `prior-slices[]` build-request field was cut because it carried only the upstream *build report* (metadata), not the artifacts the dependent actually needs, and because plan-level `depends-on` + `plan next` ordering already covers the same-tree case. Designing the cross-project handoff — artifact resolution, versioning, and cache-fingerprint participation — is deferred to a future RFC and is to be undertaken only when a first-party target has a concrete cross-project consuming dependency. It blocks no RFC-29 wave.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- [RFC-28: Engineering Standards — Codex Contract and Findings](../done/rfc-28-standards-contract.md)
- [RFC-35: Synthesis Determinism](done/rfc-35-synthesis-determinism.md) — see §"Relationship to RFC-35"
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim reconciliation](../../plugins/spec/references/synthesis/claim-reconciliation.md)
- [Provenance index](../../plugins/spec/references/synthesis/provenance.md)

