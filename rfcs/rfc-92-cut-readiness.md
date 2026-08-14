# RFC-92 cut readiness

> Status: Editing brief for [RFC-92](rfc-92-model-policy.md). Not a new RFC. Fold the decided items into RFC-92, then delete this file.
>
> Verdict: **policy-ready, not cut-ready.** D1–D5 and the eight acceptance criteria are a complete policy. A faithful first PR would stall on where the route table lives before RFC-88, and on how usage is observed across two judgment seams.

## How to use this

Each finding is one RFC-92 hole plus a recommended edit. Accept, rewrite, or reject the recommendation in RFC-92 itself. Do not implement against this file.

Suggested fold order: F1 (interim home) and F2 (observation seam) first — they change the cut split. Then F3–F6 (wire shape). Then F7–F11 (closed sets and later cuts). Then implementation requirements and acceptance criteria.

## Recommended cut split

Staff two internal cuts, not one RFC and not a second RFC.

**Cut A — record.** On the implemented RFC-86/90/104 substrate, with no RFC-88 profile and no epoch invalidation.

- Closed `OperationKey` over *live* model legs (see F7).
- Capture `Reply.usage` at both judgment kernels; emit `model.usage.recorded`.
- Read-only `emery cost` over the fact union (exact attributions only; see F6).
- Compiled launcher binding of `frontier | balanced | economy` to the current Cursor backend (`CURSOR_MODEL` plus effort/context). One default route for every key. No in-flight escalation.

**Cut B — pin.** When RFC-88’s model-capability profile exists.

- Fold the `routes` table into that profile digest.
- AC1 epoch invalidation.
- Closed escalation ladder and availability fallback.
- Requirement attribution and `plan archive` spend summary.

RFC-103 AC7 stays a consumer patch: emit facts now; populate `outcome.yaml` when RFC-103 lands.

The preamble’s “does not wait for RFC-88” then means Cut A, not “implement D1 against a profile that does not exist.”

---

## F1 — No legal home for the route table before RFC-88

**Where:** D1, AC1, rejected alternative “a separate model-policy object,” preamble “does not wait for it.”

**Finding.** D1 and AC1 put `routes` on RFC-88’s model-capability profile and invalidate the epoch on digest change. That profile does not exist. Today’s `ClosedPlanCoverage` is `plan-digest` plus per-leaf refinement digests. The RFC says not to wait for RFC-88 *and* rejects a second policy object. Cut A has no legal place to put the table, so AC1 cannot hold.

**Recommend.** Split D1 into current vs RFC-88:

- Cut A compiles a default route table in the deployment-provider layer (launcher). It is not a change artifact, has no digest in coverage, and does not invalidate an epoch. Changing it is a deployment upgrade, like changing `CURSOR_MODEL` today.
- Cut B, on RFC-88 D3, moves that table onto the pinned profile. From then on a route or ladder change is a new profile digest and invalidates the epoch, same rule as a threshold change.
- Keep the rejected alternative. The interim table is deployment data, not a second pinned policy object.

Amend AC1: “Once the route table lives on the RFC-88 profile, changing one route or escalation entry produces a new capability-profile digest and invalidates the prior closed-plan epoch.” Cut A has no equivalent criterion.

---

## F2 — Two judgment seams, not one

**Where:** Implementation requirement “thread the resolved route policy through existing judgment dispatch … without a new plumbing seam.”

**Finding.** Engine legs (`engine.propose`, `engine.synthesize`, RFC-104 `system.correlate` / `system.propose`) go through `project::judgment`. Survey, extract, and the build-loop legs call `Model::create` *inside* the adapter guest (`adapter::call`). WIT results do not return usage. Both kernels drop `Reply.usage` today.

“One fact per completed model call” therefore cannot be satisfied by engine orchestration alone. Intercepting adapter-internal `create` needs either:

- a host `Model` wrapper that records usage and needs ambient operation context on the request, or
- usage returned on every WIT result (a new WIT field).

That is a seam. The current note is false.

**Recommend.** Decide the observation path in D3 (or a short D6):

- **Preferred:** the host `Model` implementation records one usage fact per `create` return. The engine (and adapter guests) pass operation context on the existing `Request` — operation key, optional slice, optional phase ordinal, route, route-step, trigger — without a new WIT result field. Adapters never write the journal.
- **Rejected:** adapters emit journal events; WIT grows a usage result; the engine estimates usage from elapsed time.

Replace “without a new plumbing seam” with: “operation context rides the existing `Model::create` request; the host records the fact; engine crates still carry no provider constant.”

Name both kernels: `project::judgment` and `adapter::call`. Cut A is “stop dropping `Reply.usage` and emit the fact.” Cut B is “select the route before `create` and pass it on the request.”

---

## F3 — Usage fact identity is slice-and-phase shaped

**Where:** D3 YAML; AC2 (“carrying its operation key, slice, phase ordinal, …”).

**Finding.** The example requires `slice` and `phase-ordinal`. Plan-time `source.survey`, `engine.propose`, and RFC-104 correlation have neither. One `target.build` dispatch is many adapter `create` calls (phase scaffolding). D3 says one event per model call; D5 escalation is per operation.

**Recommend.** Make identity fields optional and typed:

```yaml
kind: model.usage.recorded
operation: target.build   # required
slice: orders-checkout    # absent when the leg is not slice-scoped
phase-ordinal: 2          # absent when the leg is not an RFC-90 phase
route: frontier
route-step: 0
trigger: initial          # closed; see F5
model: cursor:…
input-tokens: 48210
cached-tokens: unknown    # see F4
output-tokens: 6122
reasoning-tokens: 800     # Omnia Usage already has this
cost: unknown
elapsed-ms: 41307
```

Rules:

- One fact per `Model::create` that returns, including adapter-internal phase legs under the same operation key.
- Escalation (Cut B) keys on the *operation dispatch*, not on each inner `create`. Inner creates of one dispatch share `route` / `route-step` / `trigger`.
- AC2: every completed model call emits exactly one usage fact carrying its operation key, resolved route, route-step, trigger, and provider-reported model identity. Slice and phase ordinal are present iff the dispatch has them.

---

## F4 — `Usage` wire vs the fact envelope

**Where:** D3 fields `cached-tokens`, `cost`.

**Finding.** Omnia `Usage` today is `input_tokens`, `output_tokens`, `reasoning_tokens`. There is no `cached-tokens` and no `cost`. Cursor may not report either. D3 already allows `cost: unknown`; the example still presents `cached-tokens` as a number.

**Recommend.** Every consumption field is provider-reported or `unknown`. Never zero, never estimated.

- Always: `input-tokens`, `output-tokens` when the provider supplies them, else `unknown`.
- `reasoning-tokens`: same; Omnia already has the field.
- `cached-tokens` and `cost`: `unknown` until a provider field exists. Do not imply they are already populated.
- AC3 stays. Add: no code path invents `cached-tokens` from input/output arithmetic.

---

## F5 — Trigger set and ladder schema are incomplete

**Where:** D5 closed triggers; D3 `trigger: initial`; D5 availability fallback; implementation “ladders above the compiled bound.”

**Finding.**

- `trigger: initial` is on the fact but not in the closed trigger list.
- Availability fallback is a recorded event with no YAML and no trigger name.
- Compiled ladder bound is unnamed.
- `context-limit-refusal` has no typed producer: `project::judgment` maps every model failure to `judgment-model-failed`.
- `unchanged-failure-set` is correctly deferred to RFC-97 Phase A.

**Recommend.** Close the trigger enum:

| Trigger | Producer | Cut |
| --- | --- | --- |
| `initial` | first dispatch of the operation | A (always) |
| `answer-repair-exhausted` | `MAX_REPAIRS` exhausted on that operation | B |
| `context-limit-refusal` | typed provider outcome, not the generic `judgment-model-failed` | B, once the host distinguishes it |
| `unchanged-failure-set` | RFC-97 Phase A | B, declared, never fires before then |
| `provider-unavailable` | host binding failover; does **not** consume an escalation step | B |

Show fallback on the route policy:

```yaml
target.repair:
  start: economy
  escalate:
    - { on: unchanged-failure-set, to: balanced }
  unavailable: balanced
```

Name the compiled bound (suggest **one** step: start plus at most one escalation). Prefer offline RFC-103 promotion over growing it.

Cut A never escalates and never failovers: every fact has `trigger: initial`, `route-step: 0`.

---

## F6 — Requirement attribution and archive summary are unspecified

**Where:** D4 `--by requirement`; `plan archive` spend summary; AC4.

**Recommend.** Cut A: `emery cost --by slice | phase | route | target` only. Those four are exact fields on the fact (slice/phase absent rows fall into an explicit `unscoped` bucket, labelled as such).

Cut B: `--by requirement` is derived. Specify the algorithm: equal split of that slice’s usage-fact total across the slice’s current `model.yaml` requirements. Label it `derived`. Do not weight by claim count, token estimates, or status.

`plan archive` spend summary is Cut B (or RFC-103): a projection over the same union, not a new fact.

AC4 splits: Cut A sums the four exact attributions to the usage-fact union; Cut B adds labelled `--by requirement`.

---

## F7 — Closed key set does not match live (or future) legs

**Where:** D1 table.

**Finding.** Live model legs missing from the table: RFC-104 `system.correlate`, `system.propose`. Keys with no producer until later RFCs: `engine.topology`, `engine.decompose`, `engine.boundary` (RFC-88), `engine.readiness` (RFC-94), `target.decompose` (RFC-96). `target.guidance` / `target.merge` / `target.verify` may or may not invoke a model depending on the adapter and on RFC-97.

**Recommend.** Keep “closed per programme state.” Split the table:

| Family | Live now (Cut A) | Joins when that RFC lands |
| --- | --- | --- |
| Source | `source.survey`, `source.extract` | |
| Target | `target.guidance`, `target.build`, `target.verify`, `target.repair`, `target.review`, `target.merge` | `target.decompose` (RFC-96) |
| Engine | `engine.propose`, `engine.synthesize` | `engine.topology`, `engine.decompose`, `engine.boundary` (RFC-88); `engine.readiness` (RFC-94) |
| System | `system.correlate`, `system.propose` | |

Absent key → `default`. A key whose operation has no model leg in this deployment is never consulted. Adapter-internal phase `create`s inherit the WIT operation’s key (`target.build`, not a new `target.build.write` key).

---

## F8 — Terms vs D2: where effort and context live

**Where:** Terms “route”; D2.

**Finding.** Terms: a route is “a model identity plus its reasoning-effort and context settings.” D2: the profile names `frontier | balanced | economy`; deployment maps those names to provider, endpoint, model identity, and credential. Effort and context have no schema. Route names look closed (implementation rejects unknown route names) but D2 never says the set is closed.

**Recommend.** Close it:

- A **route name** is one of `frontier`, `balanced`, `economy`. The profile/table speaks only these names.
- A **route binding** (deployment) maps a name to provider, endpoint, model identity, credential, reasoning-effort, and context settings. That is where effort/context live. Engine crates carry none of it.
- The usage fact records the *name* (`route: frontier`) and the provider-reported model identity (`model: cursor:…`), not effort or context.

Reconcile Terms with that split. D2 “same shape as RFC-97 profile policies” is analogical only until RFC-97’s deployment-provider layer exists; Cut A’s shape is launcher policy beside `CURSOR_MODEL`.

---

## F9 — Default first-party table is unspecified

**Where:** D1 YAML is an example, not the shipped default.

**Finding.** Today every leg draws `CURSOR_MODEL`. Cut A needs a compiled default or it is not a policy.

**Recommend.** Cut A default: `default: { start: balanced }` and no per-key overrides. `balanced` binds to the deployment’s current Cursor model. Per-key starts and ladders are Cut B profile content. The D1 YAML stays illustrative of the *shape*, labelled as such.

---

## F10 — RFC-103 and RFC-97 are consumers, not Cut A work

**Where:** Implementation “Populate RFC-103’s `cost` block”; AC7; D2 “beside RFC-97 profile policies.”

**Recommend.**

- AC7: “When RFC-103 outcome records exist, they populate `cost` from the usage-fact union and aggregate by route and operation key.” Not a Cut A gate.
- D2: “Route binding lives in the deployment-provider layer. RFC-97’s profile-policy registry is the same *kind* of layer when it lands; Cut A binds through the launcher next to `CURSOR_MODEL`.”
- D5 RFC-103 promotion paragraph is future consumption, not an implementation requirement of this RFC’s first cut.

---

## F11 — “Does not wait for RFC-88” overclaims Cut A

**Where:** Preamble; [platform.md](platform.md) “routes and usage facts land on the implemented substrate and fold into the profile when that cut lands.”

**Recommend.** Align preamble with the cut split:

> Startable on implemented RFC-90 (Cut A: usage facts, default binding, `emery cost`). Pinned per-operation routes, escalation, and epoch invalidation fold into RFC-88’s model-capability profile when that cut lands (Cut B). Supplies RFC-103’s `cost` producer and the routes `model-route-change` patches.

Keep platform.md in sync in the same RFC-92 edit.

---

## RFC-92 section checklist

Fold into the RFC, then delete this file.

- [ ] Preamble: Cut A vs Cut B; drop the implication that D1 is startable now.
- [ ] Terms: route name vs route binding; effort/context on the binding; usage-fact optional slice/phase.
- [ ] D1: live vs future keys; system keys; illustrative YAML labelled; Cut A default table; Cut B profile fold-in.
- [ ] D2: closed name set; binding fields; Cut A launcher vs RFC-97 analogy.
- [ ] D3: optional identity fields; one fact per `create`; host records; `unknown` consumption fields; `reasoning-tokens`.
- [ ] D4: Cut A exact four; `unscoped` bucket; Cut B equal-split requirement algorithm.
- [ ] D5: full trigger enum; ladder bound = 1; fallback YAML; Cut A never escalates.
- [ ] New short decision or D3 paragraph: observation seam (F2).
- [ ] Implementation requirements: split by cut; name both judgment kernels; drop “no new plumbing seam”; drop RFC-103 populate as a first-cut item.
- [ ] Acceptance criteria: AC1/AC6/AC7 marked Cut B (or “once the profile exists”); AC2 optional slice/phase; AC4 split.
- [ ] platform.md one-line Cut A / Cut B alignment.
