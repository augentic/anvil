# RFC-86a: Gap Deferral

> **Status:** Draft — fix-up of [RFC-86](rfc-86-change-facts.md) (not a new series step; sits between RFC-86 and its consumers); open questions below are deliberately open for review.
>
> **Owns:** the typed gap **disposition** model (`open | deferred`), durable digest-bound **deferral facts**, the per-epoch **gap policy** (`strict | defer`), the build-scope exclusion contract for deferred requirements, debt conservation through merge and archive, and the unattended-execution posture the embedded eval composition depends on.
>
> **Builds on** RFC-86's fact substrate (per-writer logs, computed status, `plan.execute.started` typed coverage, one-member waves) and [RFC-88](rfc-88-detached-changes.md)'s three-verb surface and `refine-under-epoch` coverage. Consumed by [RFC-90](rfc-90-build-verification.md) (verification must respect build scope) and [RFC-91](rfc-91-concurrent-execution.md) (deferral facts union like every other fact).
>
> **Amends** RFC-86 D15 (an *open* `[unknown]` blocks build; a *deferred* one leaves build scope and build proceeds), D17 (per-epoch `--waive` is deleted in favor of durable deferral facts; conflict build-over remains forbidden), and D22 (Ready stays clean-gap; deferrals, like waivers before them, never contribute to Ready). D13 (gaps gate build, not refine), D16 (divergence informational), D18 (prose review human-owned), D19 (rollup presentation-only), and D24 (in-scope membership) are unchanged.

## Intent

Let an execute loop finish without knowing everything up front.

RFC-86 made typed gaps visible and made building over them a deliberate act. This RFC keeps both properties but changes the shape of the act: instead of *permission to build over* a gap (a waiver, re-supplied on every epoch), the operator or a declared policy assigns each gap a **disposition**. A deferred requirement leaves the build's scope entirely — generation is told, in a typed and machine-checkable way, that this behaviour is not to be implemented — and the gap is conserved as visible **debt** through build, merge, baseline, and archive, where the next change's evidence can resolve it.

This is the agile posture: ship the evidenced part now, carry the unevidenced part as an explicit backlog item in the baseline spec, iterate. The degenerate autonomous case — the embedded `examples/eval` workflow cases — runs `author → execute → archive` end-to-end with no operator gesture beyond starting the run.

## Why RFC-86's waiver does not scale

Desk experience with the landed gate shows four compounding problems:

| Problem | In plain terms |
| ------- | -------------- |
| Waivers are per-epoch | Waivers ride each `plan.execute.started`; a resume without `--waive` drops them. The gate even carries a dedicated "waived on an earlier epoch only" error path. Every re-run re-taxes the operator for decisions already made. |
| Waivers require pre-knowledge | Under `refine-under-epoch`, specs do not exist when execute starts — the operator cannot enumerate `--waive <slice>/<req>` for requirements that have not been minted yet. The only path is: run, park, read the inventory, re-run with waivers. That is an operator-in-the-loop protocol by construction. |
| Unattended execution is impossible | The eval composition (`cargo make eval <case> --restart`) drives real verbs with no waivers. Any `[unknown]` minted by synthesis parks the loop on `plan-gaps-unresolved`. The same applies to any future CI-shaped or agent-driven deployment. Thin-intent N=1 changes — the most common shape — are the most likely to mint unknowns. |
| Waivers never specified build behaviour | D15's fear was generation inventing missing behaviour. But a waiver only unlocked the gate — RFC-86 never defined what build should *do* with a waived `[unknown]`. The safety property lived in prompt conventions (the `[unknown]` body's non-invention rules), not in the contract. |

The last row is the opening. An `[unknown]` requirement body is non-operative by construction: a gap statement, `Sources: []`, and scenarios that must not invent behaviour. A `[conflict]` body is *only* `Note:` lines with no operative sentence. There is nothing for generation to implement — so the safe way to proceed is not to permit building *over* the gap but to take the gap *out of the build's obligations* and conserve it. That is a different act than the waiver RFC-86 debated, and it is the act this RFC types.

## The reframe: disposition, not permission

Every typed gap row `(slice, req, status)` in the inventory has a computed **disposition**:

| Disposition | Meaning | Build gate effect |
| ----------- | ------- | ----------------- |
| `open` | No live deferral covers the requirement | Blocks build (unchanged from RFC-86) |
| `deferred` | A live deferral fact covers the requirement | Build proceeds; the requirement is excluded from build scope and carried as debt |

Resolution is not a disposition — a resolved gap's row simply disappears from the inventory after re-refine, exactly as today. `[divergence]` rows stay informational and take no disposition.

The disposition vocabulary already has a precedent at slice granularity: `plan drop` durably excludes a slice from in-scope (D24) without deleting its row. Deferral is the same shape one level down — *drop : slice :: defer : requirement* — a durable, retractable exclusion fact, not an authorization.

## Decisions

### D1 — Open gaps block; deferred gaps leave build scope

The build gate keeps its seam and its default. For each in-scope leaf, before build:

- `[conflict]`, disposition `open` — **block** (unchanged).
- `[unknown]`, disposition `open` — **block** (unchanged; D15's default is preserved).
- `[unknown]` or `[conflict]`, disposition `deferred` — **proceed**; the requirement is excluded from the build's obligations (D4) and conserved as debt (D5).
- `[divergence]` — **allow, listed** (unchanged).

"Deferred" is never "build over". There is no disposition, flag, or policy under which generation is asked to implement — or invent around — a gap-status requirement. What changed from RFC-86 is only that the excluded requirement no longer holds the rest of its slice hostage.

### D2 — Deferrals are durable, digest-bound facts

A deferral is a journal fact (`gap.deferred`, per-writer like every fact) carrying:

```json
{"event":"gap.deferred","payload":{"slice":"auth-login","req":"REQ-003","requirement-digest":"sha256:…","reason":"reset path deferred to next change","origin":"operator"}}
```

- **`requirement-digest`** is the canonical digest of the requirement's body (title, statement, scenarios, notes — the same content `model.yaml` carries). The digest, not the `REQ-NNN` id, is the match key: slice-local ids are kernel-minted in declaration order and a re-refine may renumber them, but a verbatim-unchanged unknown keeps its digest.
- **Lifetime:** a deferral is **live** while some requirement in that slice's current model carries the digest. Re-refine that resolves or reshapes the requirement (new evidence arrived, the gap statement changed) makes the digest disappear — the deferral **lapses** automatically and the new row is `open` again, forcing re-disposition. This is RFC-86's staleness philosophy applied to dispositions: a decision binds exactly the content it was made about.
- **Retraction:** `gap.deferral-retracted` reopens a live deferral explicitly (surface in D3).
- **No stored state:** disposition is computed by joining the fact union to the live gap inventory. `spec.md`, `model.yaml`, and `plan.yaml` are not touched — no `deferred:` field anywhere, per RFC-86 D2.

Because deferrals are facts rather than epoch payload, they survive resumes, survive fresh epochs, union across writers (RFC-91), and travel with the change home (RFC-88). The entire "re-supply `--waive` on every run" protocol — and its earlier-epoch footgun handler — is deleted.

### D3 — Two deferral surfaces: an explicit act and a declared policy

**Explicit act** — `emery plan defer <slice>/<req> --reason <text>` (repeatable selector; `--retract` reverses). This is the attended path: read the parked inventory, defer what should wait, resume execute. It replaces `--waive` one-for-one but the decision is made once, not once per epoch. Like `plan drop`, it is CLI-only with no skill wrapper; exact clap names are illustrative.

**Declared policy** — `emery plan execute --gap-policy <strict|defer>`, default `strict`. The policy rides the epoch: it is recorded on the `plan.execute.started` coverage payload (replacing the `unknown-waivers` field). Under `defer`, when the loop reaches a build gate and finds gap rows with disposition `open`, it dispositions them **at the gate** — writing one `gap.deferred` fact per requirement with `origin: policy` — and proceeds. Gate-time (not epoch-start) application is load-bearing: under `refine-under-epoch`, the unknowns do not exist until the refine phase has run inside the same epoch.

This deliberately revisits RFC-86's rejection of `--force` / `--allow-gaps` / bulk waive. The objection was "invisible skip as a one-flag off-switch". The policy differs in every property that made the skip invisible:

1. Nothing is skipped — each finding is individually dispositioned into a typed, journaled, digest-bound fact, visible in `plan gaps`, `plan status`, the build record, the wave-commit fact, and the archive summary.
2. Nothing is invented — the deferred requirement leaves build scope (D4); the failure mode D15 guarded against is addressed structurally rather than waved through.
3. Nothing evaporates — a waiver died with its epoch and left no trace in the product; a deferral is conserved as debt into the baseline (D5).

The policy is an explicit operator gesture on the same command that opens the epoch — the invocation says what it authorizes, and the fact log says exactly what that came to mean.

### D4 — Deferred requirements are excluded from build obligations, machine-checkably

The build phase's request assembly gains a typed exclusion set: `BuildRequest` carries `deferred[]` — the slice-local requirement id, title, and requirement digest of every deferred row on the slice. The engine's side of the contract:

- Target build prompts receive the deferred set and must treat those requirements as **out of scope**: no implementation, no scaffolding, no invented placeholders, no TODO markers in product code. The baseline spec carries the debt; product code carries nothing.
- The report gate rejects a `BuildReport` that claims coverage of a deferred requirement (`target-build-deferred-covered`, sibling to the existing `target-build-*` aborts).
- The `BuildRecord` records the deferred set the build consumed, extending RFC-86's input fence: a build is stale if the disposition set it was built under no longer matches, exactly as it is stale under pin drift.
- RFC-90 verification must not fail a build for unimplemented deferred scenarios; deferred requirements are outside the verification surface by the same exclusion set.

The adapter-facing half (build and review prompt changes in `augentic/emery-adapters`) is a cross-repo implementation requirement, not new WIT surface — the deferred set rides the existing build-request envelope.

### D5 — Debt is conserved: merge, baseline, archive

Deferral defers; it never deletes.

- **Merge:** deferred requirements fold into the target baseline with their `Status:` preserved (`unknown` rows stay `[unknown]`; conflict handling per D6 / open question 4). Wave commit assigns their final baseline `REQ-NNN` like any other row, and the `target.merge.wave-committed` fact snapshots the deferred set it carried — the committed audit trail names exactly which debt this wave accepted.
- **Baseline:** the merged baseline spec is the backlog. The next change's synthesis reads the baseline (as it already does), and new evidence — richer intent, docs, runtime `captures` — resolves the carried `[unknown]`s through the ordinary refine path. This is the iteration loop: debt raised in change *N* is first-class input to change *N+1*.
- **Archive:** `plan archive` succeeds with debt and prints a carried-debt summary (slice, requirement, reason, origin, age). Archiving never launders debt — the rows are in the baseline, and the archived change retains its deferral facts.

### D6 — Conflicts defer under the same exclusion semantics — build-over stays forbidden

RFC-86 D17 held that `[conflict]` is never *waiveable*, and this RFC preserves exactly that: there is no disposition under which generation builds over an unresolved contradiction. But deferral is not waiver. A conflict requirement's body is non-operative by construction (only `Note:` lines, one per source value, no operative sentence) — generation cannot implement either arm, so *excluding it from build scope* is structurally as safe as excluding an unknown, and an autonomous run needs that exit: a tied-authority disagreement between two sources must not permanently park an unattended loop when the rest of the slice is evidenced and buildable.

Therefore: `[conflict]` rows take the same `open | deferred` dispositions, deferrable by the explicit act and by the `defer` policy, with both arms conserved as notes in the debt row. The resolution path (authority override or source correction, then re-refine) remains the preferred exit and is unchanged.

This is the sharpest revision of RFC-86 in this RFC and is flagged as open question 1.

### D7 — Projections: disposition column, debt counts, Ready stays clean

- `emery plan gaps` gains a **disposition** column (`open | deferred`), the deferral's reason and origin on deferred rows, and keeps D19's shared-lead presentation rollup over both dispositions.
- `emery plan status` counts debt (for example `2 deferred gaps` beside the milestone line) and computes next-actions over **open** findings only: a plan whose every gap is deferred projects `plan execute` as the resume, not `review-gaps`.
- **Ready is unchanged** (D22 preserved with deferrals substituted for waivers): Ready means every in-scope slice refined and the *clean* gap policy passes — zero open **and zero deferred** findings. A plan carrying debt reaches build through Authorized, never through Ready. No new milestone rung is minted.

### D8 — Unattended composition: the eval loop runs end-to-end

The probe workflow-case runner passes `--gap-policy defer` on `plan execute` by default (a case may pin `strict` to exercise the gate). Acceptance: `cargo make eval auth --restart` completes `init → plan author → plan execute (refine → build → merge) → drained` with no operator gesture, even when synthesis honestly mints `[unknown]` rows — and the retained sandbox's fact log, gaps projection, and archive summary show exactly what was deferred. The same posture serves the wasm examples and any future CI-shaped deployment.

Skills stay ultrathin: `/emery:execute` continues to elicit arguments and relay — `--gap-policy` is just another argument. No new skill.

## Commands that change

| Command | Change |
| ------- | ------ |
| `emery plan defer <slice>/<req> --reason …` | **New** plan act (CLI-only, like `plan drop`): appends a durable digest-bound `gap.deferred` fact. `--retract` appends `gap.deferral-retracted`. Refuses unknown selectors, missing reasons, and rows without gap status (`plan-deferral-invalid`) |
| `emery plan execute` | `--waive <slice>/<req>` / `--reason` **removed** (hard cut). Gains `--gap-policy <strict\|defer>` (default `strict`), recorded on `plan.execute.started` coverage. Under `defer`, open gap rows are dispositioned at each build gate with `origin: policy` facts. `plan-gaps-unresolved` remains the strict-mode refusal, its hints now naming `plan defer` / `--gap-policy defer` |
| `emery plan gaps` | Disposition column + reason/origin on deferred rows; rollup unchanged |
| `emery plan status` | Debt counts; next-actions computed over open findings only; Ready stays clean-only |
| `emery plan archive` | Prints the carried-debt summary; never blocks on debt (advisory — open question 6) |
| eval / probe runner | Workflow cases execute under `--gap-policy defer` by default; per-case override |

## Amendments to RFC-86 (explicit)

| RFC-86 decision | Status here |
| --------------- | ----------- |
| D15 — `[unknown]` always blocks build | **Amended:** an **open** `[unknown]` blocks build; a **deferred** one leaves build scope and build proceeds. The default gate is unchanged; the non-invention property is now a typed contract (D4) instead of a prompt convention |
| D17 — per-epoch `--waive`, conflict never waiveable, no separate verb | **Amended:** per-epoch waivers and the coverage `unknown-waivers` field are deleted; the disposition surface is `plan defer` + `--gap-policy` (durable facts). The "separate verb creates waived-but-not-executed limbo" objection is void — a deferral is a disposition like `plan drop`, not an authorization; execute alone still opens epochs. Conflict **build-over** remains forbidden; conflict **deferral** (exclusion) is D6 |
| D22 — Ready is clean-gap only | **Preserved**, with deferrals in the waiver seat: Ready counts open and deferred findings; debt-carrying plans reach build via Authorized only |
| D13, D16, D18, D19, D21, D23, D24, D26 | **Unchanged** — gaps still gate build not refine; divergence informational; prose review human-owned; rollup presentation-only; no topology approve; per-slice claims; in-scope membership; post-author resume |
| Rejected: warn-only `[unknown]` | **Still rejected** — defer is not warn-and-build-over; the requirement leaves scope |
| Rejected: silent auto-waive | **Still rejected** — every policy deferral is an enumerated, journaled, digest-bound, debt-conserved fact under an explicitly declared policy |

## Implementation requirements

- New journal events `gap.deferred` / `gap.deferral-retracted` in the closed `EventKind` taxonomy (kebab-case wire ids), payload per D2 with closed `origin: operator | policy`. Duplicate deferrals for one digest are idempotent under projection.
- Canonical requirement-body digest exported from the `model.yaml` requirement representation (kernel-owned, format-independent — the same digest posture as RFC-88's planning digests); recorded on each deferral and joined at projection time.
- `ClosedPlanCoverage::ClosedPlan` replaces `unknown-waivers` with `gap-policy` (closed enum, default `strict`). Epoch freshness (`project::plan::epoch`) does not cover deferral facts — facts are append-only and cannot drift; input fencing is the build record's job.
- Gap gate (`change::orchestrate::gap_gate`): join dispositions before classifying blockers; under `defer` policy, mint gate-time deferrals for open rows and proceed; delete `waived_on_earlier_epoch` and its error arm.
- Build assembly: `BuildRequest.deferred[]` (id, title, requirement digest); report gate `target-build-deferred-covered`; `BuildRecord` records the consumed deferred set and staleness includes disposition drift.
- Merge: deferred rows fold to baseline with status preserved and final ids assigned; `target.merge.wave-committed` snapshots the deferred member set; archive renders the carried-debt summary.
- Projections: disposition column in `plan gaps`; debt counts and open-only next-actions in `plan status`; Ready over open + deferred.
- Probe: workflow cases pass `--gap-policy defer` by default with per-case override; the auth case (and adapter-repo workflow cases) run drained unattended.
- Cross-repo (`augentic/emery-adapters`): build and review prompts consume `deferred[]` as out-of-scope; RFC-90 verification excludes deferred requirements from its surface.
- Diagnostics (exit 2): `plan-deferral-invalid` (bad selector / missing reason / non-gap row); `plan-gaps-unresolved` retained for strict mode with defer-facing hints. `plan-waiver-invalid` is deleted with the waiver surface.
- Hard cut (pre-1.0): no compatibility parsing for `unknown-waivers` coverage payloads or `--waive` argv; goldens regenerate.
- Documentation: `AGENTS.md`, `docs/standards/workflow.md`, CLI help, and RFC-86's gap-policy prose gain the disposition vocabulary in the same change (RFC-86 D20 posture — shipped-doc drift is delivery work, not a freeze blocker).

## Acceptance criteria

1. **Strict default preserved:** with no deferrals and `--gap-policy strict` (or the flag absent), the gate behaves as RFC-86 shipped it — open `[unknown]` / `[conflict]` refuse build with the rendered inventory; `[divergence]` is listed and allowed.
2. **Durable disposition:** `plan defer` writes a digest-bound fact that covers the requirement across resumes and fresh epochs with no re-supply; `--retract` reopens it; a re-refine that changes the requirement body lapses it and the new row blocks again under strict.
3. **Policy mode:** under `--gap-policy defer`, an execute run that mints unknowns via `refine-under-epoch` dispositions them at the build gate (one `origin: policy` fact each) and proceeds through build and merge; every deferral is visible in `plan gaps` and the fact log.
4. **Build-scope exclusion:** the build request enumerates the deferred set; a report claiming coverage of a deferred requirement fails `target-build-deferred-covered`; the build record binds the disposition set and disposition drift is staleness.
5. **Debt conservation:** deferred rows appear in the merged baseline with status and notes preserved and final `REQ-NNN` assigned; the wave-commit fact snapshots the deferred set; `plan archive` prints the carried-debt summary and succeeds.
6. **Projection coherence:** `plan status` next-actions compute over open findings only; a fully-dispositioned plan resumes at `plan execute`; Ready projects only on zero open and zero deferred; debt counts render.
7. **Waiver surface deleted:** `--waive` is unknown argv; no coverage payload carries `unknown-waivers`; no path demands re-supplying a decision already on the log.
8. **Unattended eval:** `cargo make eval auth --restart` runs `init → author → execute → drained` with no operator gesture, including when synthesis mints `[unknown]` rows; the retained sandbox shows the deferral trail.
9. **Multi-writer:** deferral facts from two writers union losslessly; duplicate deferrals of one digest project one disposition; retraction from either writer reopens.
10. `cargo make ci` passes with crate-level integration coverage for dispositions, lapse-on-digest-change, gate-time policy minting, build-scope exclusion, debt-through-merge, and the strict-mode regression suite.

## Open questions

Deliberately open — this RFC expects iteration on each before freeze.

1. **Conflict deferral (D6).** Should `[conflict]` defer under the policy like `[unknown]`, defer only via the explicit `plan defer` act (autonomy then parks on conflicts), or stay hard-blocking as RFC-86 shipped? The draft proposes full deferability on the exclusion-not-build-over argument; the counterargument is that a contradiction is qualitatively riskier debt than an absence, because adjacent agreed requirements may silently assume one arm.
2. **Default policy trajectory.** `strict` stays the default here. Should a project-level default live in `project.yaml` (so eval/CI-shaped projects declare it once), and should `defer` ever become the global default once debt tooling matures?
3. **Deferral match key.** Body digest alone (draft), or `(slice, digest)` with the `REQ` id advisory? What happens if two requirements in one slice carry identical bodies (degenerate but possible), or if a lapsed deferral's exact body returns in a later refine — does the old fact revive (draft: yes, liveness is recomputed) or must deferral be re-asserted?
4. **Baseline shape for deferred conflicts.** Fold to baseline with `Status: conflict` preserved (honest, but baseline merge validation has never carried conflict rows), or downgrade to `unknown` with both arms as notes (loses the typed distinction)?
5. **Reason on policy deferrals.** Synthesized reason (`deferred by gap-policy under epoch <ts>`, draft) or require a `--reason` on the `defer`-policy invocation that stamps every minted fact?
6. **Archive posture.** Always advisory (draft), or an opt-in threshold gate (`plan archive` refuses above N carried debts / conflicts) for operators who want a hard backstop?
7. **Verification depth (RFC-90 seam).** Is the typed report gate (`target-build-deferred-covered`) enough, or should verification positively assert *absence* — that no product code implements a deferred requirement's scenarios?
8. **Retraction surface.** Dedicated `plan defer --retract` (draft) vs routing through a generic `fact.retracted` surface, if one ships.
9. **Debt aging.** Baseline `[unknown]`s can now accumulate across changes by design. Does anything nag (a baseline-debt projection, an `emery slice validate` advisory with age, a roadmap RM item), or is the gaps projection at next-plan time sufficient? Draft: out of scope here; candidate roadmap entry.

## Rejected alternatives

- **Warn-only unknowns / flipping D15** — re-litigates the settled failure mode; generation over an in-scope gap invents behaviour. Deferral removes the gap from scope instead.
- **Sticky waivers (make `--waive` survive epochs) without the scope contract** — fixes only the re-supply tax; still per-requirement pre-knowledge the operator cannot have under `refine-under-epoch`, and still no defined build behaviour for the waived row.
- **A blind skip flag (`--force` / `--allow-gaps`) with no facts** — RFC-86's rejection stands; the `defer` policy is accepted only because every disposition is enumerated, journaled, digest-bound, and conserved.
- **Model-owned disposition** — letting synthesis or the build agent decide what to defer moves an authority decision into a judgment leg. The model reports statuses; the engine and operator disposition them.
- **A backlog artifact (`debt.yaml` / tracker file)** — a second store to drift; disposition is computed from facts plus the live model, and the baseline spec *is* the backlog (RFC-86 D2).
- **Rendering a build-scoped spec view that omits deferred rows** — two spec surfaces; `spec.md` stays the single authoritative artifact and the exclusion set rides the request envelope.
- **Deleting deferred requirements at merge** — silent loss; debt must land in the baseline to be resolvable by the next change.
- **Lead-level or slice-level deferral** — the slice-level act exists (`plan drop`); lead-level dispositions repeat D19's rejected lead-wide waiver. Granularity stays per requirement.
- **Auto-resolving conflicts by authority re-ranking at the gate** — authority resolution is refine-time synthesis policy; the gate must not re-adjudicate evidence.

## Non-goals

- Changing Evidence schemas, the authority ranking, synthesis status derivation, or when `[unknown]` / `[conflict]` / `[divergence]` are minted.
- Permitting generation to build over (implement, guess at, or scaffold around) any gap-status requirement under any disposition or policy.
- New plan milestones or a projected rung between Ready and Authorized.
- A cross-change debt-tracking product, dashboards, or SLAs on carried unknowns (open question 9 names the follow-on seam).
- Multi-operator countersign on deferrals — one writer's fact suffices, matching RFC-86's waiver posture.
- Reintroducing any `approve` vocabulary — execute remains the sole privileged-start surface; deferral is a disposition, not an authorization.
