# RFC-86a: Gap Deferral

> **Status:** Draft — fix-up of [RFC-86](rfc-86-change-facts.md) (not a new series step; sits between RFC-86 and its consumers); all product / contract questions are **closed** (decision trail below).
>
> **Amended after landing:** the `strict | defer` gap-policy knob (D3/D6 — the `project.yaml` declaration, `emery init --gap-policy`, `emery plan execute --gap-policy`), the `emery plan defer` verb with its `--retract` reversal (`gap.deferral-retracted`), and the `origin` field on `gap.deferred` are **deleted**. Gate-time auto-deferral is now unconditional: the build gate mints one `gap.deferred` fact per open row and build always proceeds. Everything downstream (D1/D2/D4/D5/D9 — build-scope exclusion, debt conservation, `plan gaps`, `emery debt`, the archive summary) stands. The operator pause on gaps returns with [RFC-91](rfc-91-refinement-stage.md)'s refinement stage. The body below is the historical record of the original design.
>
> **Owns:** the typed gap **disposition** model (`open | deferred`), durable digest-bound **deferral facts**, the **gap policy** (`strict | defer`) with its project-level declaration and per-epoch override, the build-scope exclusion contract for deferred requirements, debt conservation through merge and archive, the **baseline debt projection** surfaced at the change boundaries, and the unattended-execution posture the embedded eval composition depends on.
>
> **Builds on** RFC-86's fact substrate (per-writer logs, computed status, `plan.execute.started` typed coverage, one-member waves) and [RFC-88](rfc-88-detached-changes.md)'s three-verb surface and `refine-under-epoch` coverage. Consumed by [RFC-90](rfc-90-build-verification.md) (verification must respect build scope) and [RFC-96](rfc-96-concurrent-execution.md) (deferral facts union like every other fact).
>
> **Amends** RFC-86 D15 (an *open* `[unknown]` blocks build; a *deferred* one leaves build scope and build proceeds), D17 (per-epoch `--waive` is deleted in favor of durable deferral facts; conflict build-over remains forbidden), and D22 (Ready stays clean-gap; deferrals, like waivers before them, never contribute to Ready). D13 (gaps gate build, not refine), D16 (divergence informational), D18 (prose review human-owned), D19 (rollup presentation-only), and D24 (in-scope membership) are unchanged.

## Intent

Let an execute loop finish without knowing everything up front.

RFC-86 made typed gaps visible and made building over them a deliberate act. This RFC keeps both properties but changes the shape of the act: instead of *permission to build over* a gap (a waiver, re-supplied on every epoch), the operator or a declared policy assigns each gap a **disposition**. A deferred requirement leaves the build's scope entirely — generation is told, in a typed and machine-checkable way, that this behaviour is not to be implemented — and the gap is conserved as visible **debt** through build, merge, baseline, and archive, where the next change's evidence can resolve it.

This is the agile posture: ship the evidenced part now, carry the unevidenced part as an explicit backlog item in the baseline spec, iterate. The degenerate autonomous case — the embedded `examples/eval` workflow cases — runs `author → execute → archive` end-to-end with no operator gesture beyond starting the run.

The operating posture is **keep going unless told not to**: under a declared `defer` policy the loop never parks on a gap, and the operator's review moment moves from mid-execute to the change boundaries — the archive summary looking back, the baseline debt projection looking ahead (D9).

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

- **`requirement-digest`** is the canonical digest of the requirement's body (title, statement, scenarios, notes — the same content `model.yaml` carries). The digest, not the `REQ-NNN` id, is the match key: slice-local ids are kernel-minted in declaration order and a re-refine may renumber them, but a verbatim-unchanged unknown keeps its digest. The full match key is `(slice, digest)` — the fact's `slice` scopes the join, and the `req` id it carries is advisory presentation only. Two requirements in one slice with identical bodies (degenerate but legal) share one digest and therefore one disposition: same content, same decision.
- **Lifetime:** a deferral is **live** while some requirement in that slice's current model carries the digest. Re-refine that resolves or reshapes the requirement (new evidence arrived, the gap statement changed) makes the digest disappear — the deferral **lapses** automatically and the new row is `open` again, forcing re-disposition. This is RFC-86's staleness philosophy applied to dispositions: a decision binds exactly the content it was made about. A lapsed deferral whose exact body returns in a later refine **revives** — liveness is recomputed from the fact union against the live model, never re-asserted; re-forcing a decision already on the log would be a park-shaped tax.
- **Retraction:** `gap.deferral-retracted` reopens a live deferral explicitly (surface in D3).
- **No stored state:** disposition is computed by joining the fact union to the live gap inventory. `spec.md`, `model.yaml`, and `plan.yaml` are not touched — no `deferred:` field anywhere, per RFC-86 D2.

Because deferrals are facts rather than epoch payload, they survive resumes, survive fresh epochs, union across writers (RFC-96), and travel with the change home (RFC-88). The entire "re-supply `--waive` on every run" protocol — and its earlier-epoch footgun handler — is deleted.

### D3 — Two deferral surfaces: an explicit act and a declared policy

**Explicit act** — `emery plan defer <slice>/<req> --reason <text>` (repeatable selector; `--retract` reverses). This is the attended path: read the parked inventory, defer what should wait, resume execute. It replaces `--waive` one-for-one but the decision is made once, not once per epoch. Like `plan drop`, it is CLI-only with no skill wrapper; exact clap names are illustrative.

**Declared policy** — the **effective** gap policy for an epoch resolves in three layers: a per-epoch `emery plan execute --gap-policy <strict|defer>` flag, else the project's `project.yaml` `gap-policy:` declaration, else the built-in `strict`. The declaration is written by `emery init --gap-policy <strict|defer>` and preserved by `init --upgrade` — like every `project.yaml` field it is CLI-written, never hand-edited. This is the "keep going unless I tell you not to" knob: an iterate-fast or CI-shaped project declares `defer` once and never re-supplies it; a fresh project keeps the strict gate until its operator says otherwise; the flag overrides either way for one run. The effective policy rides the epoch: it is recorded on the `plan.execute.started` coverage payload (replacing the `unknown-waivers` field). Under `defer`, when the loop reaches a build gate and finds gap rows with disposition `open` — `[unknown]` and `[conflict]` alike (D6) — it dispositions them **at the gate**, writing one `gap.deferred` fact per requirement with `origin: policy` and a synthesized reason (`deferred by gap-policy under epoch <ts>`; no `--reason` is demanded of an unattended run), and proceeds. Gate-time (not epoch-start) application is load-bearing: under `refine-under-epoch`, the unknowns do not exist until the refine phase has run inside the same epoch.

This deliberately revisits RFC-86's rejection of `--force` / `--allow-gaps` / bulk waive. The objection was "invisible skip as a one-flag off-switch". The policy differs in every property that made the skip invisible:

1. Nothing is skipped — each finding is individually dispositioned into a typed, journaled, digest-bound fact, visible in `plan gaps`, `plan status`, the build record, the wave-commit fact, and the archive summary.
2. Nothing is invented — the deferred requirement leaves build scope (D4); the failure mode D15 guarded against is addressed structurally rather than waved through.
3. Nothing evaporates — a waiver died with its epoch and left no trace in the product; a deferral is conserved as debt into the baseline (D5).

The policy is an explicit operator gesture on the same command that opens the epoch — the invocation says what it authorizes, and the fact log says exactly what that came to mean.

### D4 — Deferred requirements are excluded from build obligations, machine-checkably

The build phase's request assembly gains a typed exclusion set: `BuildRequest` carries `deferred[]` — the slice-local requirement id, title, and requirement digest of every deferred row on the slice. The engine's side of the contract:

- Target build prompts receive the deferred set and must treat those requirements as **out of scope**: no implementation, no scaffolding, no invented placeholders, no TODO markers in product code. The baseline spec carries the debt; product code carries nothing.
- The report gate rejects a coverage claim on a deferred requirement (`target-build-deferred-covered`, sibling to the existing `target-build-*` aborts). As shipped under [RFC-90](rfc-90-build-verification.md)'s phase machine, the gate runs **fail-fast** on the `build` dispatch's phase report — a claim halts the attempt before `verify` dispatches — and again on the engine-assembled terminal `BuildReport` at commit, beside `enforce_no_blocking`.
- The `BuildRecord` records the deferred set the build consumed, extending RFC-86's input fence: a build is stale if the disposition set it was built under no longer matches, exactly as it is stale under pin drift.
- RFC-90 verification must not fail a build for unimplemented deferred scenarios; deferred requirements are outside the verification surface by the same exclusion set. Whether verification additionally asserts positive *absence* — that no product code implements a deferred requirement's scenarios — is [RFC-90](rfc-90-build-verification.md)'s decision, taken there with `deferred[]` in hand (resolved question 7).

The adapter-facing half (build and review prompt changes in `augentic/emery-adapters`) is a cross-repo implementation requirement. On the request side there is no new WIT surface — the deferred set rides the existing build-request envelope (and `attempt::copy_request` carries it into every RFC-90 attempt). As shipped, the report side does gain one WIT field: the **`phase-report`** record — the per-dispatch return RFC-90's build phase machine consumes — carries `covered: list<string>` (slice-local requirement ids the dispatch claims to have implemented; empty when omitted; meaningful only on `build` dispatches, like `outputs` / `ui-surface`). That is the coverage declaration the `target-build-deferred-covered` gate checks against the request's `deferred[]`; the engine projects the build round's `covered` onto the terminal `BuildReport`, and the `merge` operation's `report` record carries no coverage claim.

### D5 — Debt is conserved: merge, baseline, archive

Deferral defers; it never deletes.

- **Merge:** deferred requirements fold into the target baseline with their `Status:` preserved — `unknown` rows stay `[unknown]`, and conflict rows stay `[conflict]` with both arms' `Note:` lines intact (baseline merge validation learns to carry conflict rows; no downgrade to `unknown`, which would lose the typed distinction the corrective change needs). The fold also appends one `Note:` line carrying the deferral's reason, origin (`operator | policy`), originating change, and deferral date, so every baseline debt row is **self-describing** — the D9 projection reads the baseline alone and never joins archived fact logs. Wave commit assigns their final baseline `REQ-NNN` like any other row, and the `target.merge.wave-committed` fact snapshots the deferred set it carried — the committed audit trail names exactly which debt this wave accepted.
- **Baseline:** the merged baseline spec is the backlog. The next change's synthesis reads the baseline (as it already does), and new evidence — richer intent, docs, runtime `captures` — resolves the carried `[unknown]`s through the ordinary refine path. This is the iteration loop: debt raised in change *N* is first-class input to change *N+1*.
- **Archive:** `plan archive` succeeds with debt and prints a carried-debt summary (slice, requirement, reason, origin, age). Archiving never launders debt — the rows are in the baseline, and the archived change retains its deferral facts.

### D6 — Conflicts defer under the same exclusion semantics — build-over stays forbidden

RFC-86 D17 held that `[conflict]` is never *waiveable*, and this RFC preserves exactly that: there is no disposition under which generation builds over an unresolved contradiction. But deferral is not waiver. A conflict requirement's body is non-operative by construction (only `Note:` lines, one per source value, no operative sentence) — generation cannot implement either arm, so *excluding it from build scope* is structurally as safe as excluding an unknown, and an autonomous run needs that exit: a tied-authority disagreement between two sources must not permanently park an unattended loop when the rest of the slice is evidenced and buildable.

Therefore: `[conflict]` rows take the same `open | deferred` dispositions, deferrable by the explicit act and by the `defer` policy, with both arms conserved as notes in the debt row. The resolution path (authority override or source correction, then re-refine) remains the preferred exit and is unchanged.

This is the sharpest revision of RFC-86 in this RFC (resolved question 1). The counterargument — a contradiction is qualitatively riskier debt than an absence, because adjacent agreed requirements may silently assume one arm — is met with **visibility, not blocking**: everywhere debt is counted or listed (`plan gaps`, `plan status`, the archive summary, the D9 projection), deferred conflicts are rendered separately from deferred unknowns, so a shipped-around contradiction is always louder news than a shipped-around absence.

### D7 — Projections: disposition column, debt counts, Ready stays clean

- `emery plan gaps` gains a **disposition** column (`open | deferred`), the deferral's reason and origin on deferred rows, deferred conflicts rendered separately from deferred unknowns (D6), and keeps D19's shared-lead presentation rollup over both dispositions.
- `emery plan status` counts debt with conflicts broken out (for example `3 deferred gaps (2 unknown, 1 conflict)` beside the milestone line) and computes next-actions over **open** findings only: a plan whose every gap is deferred projects `plan execute` as the resume, not `review-gaps`.
- **Ready is unchanged** (D22 preserved with deferrals substituted for waivers): Ready means every in-scope slice refined and the *clean* gap policy passes — zero open **and zero deferred** findings. A plan carrying debt reaches build through Authorized, never through Ready. No new milestone rung is minted.

### D8 — Unattended composition: the eval loop runs end-to-end

The probe workflow-case runner passes `--gap-policy defer` on `plan execute` by default (a case may pin `strict` to exercise the gate, or declare the policy at init through D3's `project.yaml` declaration — the flag wins for one epoch either way). Acceptance: `cargo make eval auth --restart` completes `init → plan author → plan execute (refine → build → merge) → drained` with no operator gesture, even when synthesis honestly mints `[unknown]` rows — and the retained sandbox's fact log, gaps projection, and archive summary show exactly what was deferred. The same posture serves the wasm examples and any future CI-shaped deployment.

Skills stay ultrathin: `/emery:execute` continues to elicit arguments and relay — `--gap-policy` is just another argument. No new skill.

### D9 — Baseline debt is a projection, surfaced at the change boundaries

Under a `defer` policy nothing parks mid-loop, so the strict gate's forcing function needs a replacement: a review surface at the boundary between changes. Both boundary surfaces read the same debt, and both are read-only projections — nothing new is stored (RFC-86 D2; the baseline spec *is* the backlog, per D5).

- **Looking back — archive.** `plan archive` prints the carried-debt summary (D5) and stays advisory: it never blocks on debt, and there is no threshold gate — a gate at archive would be a park by another name (resolved question 6).
- **Looking ahead — the baseline debt projection.** A read-only verb (drafted `emery debt`; exact surface illustrative, like D3's clap names) walks the baseline specs and lists every requirement whose status is `unknown` or `conflict`, with the reason, origin, originating change, and age read from the self-describing `Note:` line D5 folds in — conflicts rendered separately from unknowns. `plan author` renders the same inventory in the plan review prose it authors, so a corrective change is scoped with the backlog in front of the operator. Resolution then flows through the ordinary path the RFC already defines: new evidence in the corrective change's sources resolves carried rows at refine, and they disappear from the baseline at the next merge.

Debt aging beyond the projection — nagging, SLAs, thresholds, cross-change dashboards — stays out of scope (resolved question 9).

## Commands that change

| Command | Change |
| ------- | ------ |
| `emery init` | Gains `--gap-policy <strict\|defer>`, writing the optional `project.yaml` `gap-policy:` declaration (absent means `strict`); `init --upgrade` preserves it. CLI-written only, never hand-edited |
| `emery plan defer <slice>/<req> --reason …` | **New** plan act (CLI-only, like `plan drop`): appends a durable digest-bound `gap.deferred` fact. `--retract` appends `gap.deferral-retracted`. Refuses unknown selectors, missing reasons, and rows without gap status (`plan-deferral-invalid`) |
| `emery plan author` | Review prose gains the baseline debt inventory (D9), so a corrective change is scoped with the backlog in view |
| `emery plan execute` | `--waive <slice>/<req>` / `--reason` **removed** (hard cut). Gains `--gap-policy <strict\|defer>` as a one-epoch override of the `project.yaml` declaration; the **effective** policy (flag → declaration → `strict`) is recorded on `plan.execute.started` coverage. Under `defer`, open gap rows — `[unknown]` and `[conflict]` alike — are dispositioned at each build gate with `origin: policy` facts. `plan-gaps-unresolved` remains the strict-mode refusal, its hints now naming `plan defer` / `--gap-policy defer` |
| `emery plan gaps` | Disposition column + reason/origin on deferred rows; deferred conflicts rendered separately from deferred unknowns; rollup unchanged |
| `emery plan status` | Debt counts with conflicts broken out; next-actions computed over open findings only; Ready stays clean-only |
| `emery plan archive` | Prints the carried-debt summary; never blocks on debt (advisory — resolved question 6) |
| `emery debt` | **New** read-only baseline debt projection (name illustrative): walks the baseline specs and lists every `unknown` / `conflict` requirement with reason, origin, originating change, and age (D9) |
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

- New journal events `gap.deferred` / `gap.deferral-retracted` in the closed `EventKind` taxonomy (kebab-case wire ids), payload per D2 with closed `origin: operator | policy`; policy-minted facts carry the synthesized reason (D3). Duplicate deferrals for one `(slice, digest)` are idempotent under projection.
- Canonical requirement-body digest exported from the `model.yaml` requirement representation (kernel-owned, format-independent — the same digest posture as RFC-88's planning digests); recorded on each deferral and joined at projection time under the `(slice, digest)` match key (D2).
- `project.yaml` gains the optional `gap-policy: strict | defer` declaration, written by `emery init --gap-policy` and preserved by `init --upgrade`; the file stays CLI-written only.
- `ClosedPlanCoverage::ClosedPlan` replaces `unknown-waivers` with `gap-policy` (closed enum) carrying the **effective** policy — per-epoch flag, else the `project.yaml` declaration, else `strict` (D3). Epoch freshness (`project::plan::epoch`) does not cover deferral facts — facts are append-only and cannot drift; input fencing is the build record's job.
- Gap gate (`change::orchestrate::gap_gate`): join dispositions before classifying blockers; under `defer` policy, mint gate-time deferrals for open rows — `[unknown]` and `[conflict]` alike, each with the synthesized policy reason — and proceed; delete `waived_on_earlier_epoch` and its error arm.
- Build assembly: `BuildRequest.deferred[]` (id, title, requirement digest); the `covered[]` coverage declaration on the WIT `phase-report` (build dispatches only), gated fail-fast under RFC-90's phase machine and re-checked on the terminal report (`target-build-deferred-covered`); `BuildRecord` records the consumed deferred set and staleness includes disposition drift.
- Merge: deferred rows fold to baseline with status preserved (`conflict` stays `[conflict]` with both arms' notes intact; baseline merge validation accepts conflict rows) and final ids assigned, plus the appended self-describing deferral `Note:` (reason, origin, originating change, date — D5); `target.merge.wave-committed` snapshots the deferred member set; archive renders the carried-debt summary.
- Projections: disposition column in `plan gaps` with deferred conflicts rendered separately; debt counts (conflicts broken out) and open-only next-actions in `plan status`; Ready over open + deferred.
- Baseline debt projection (D9): a read-only verb (drafted `emery debt`; naming illustrative) over the baseline specs listing `unknown` / `conflict` rows with the self-describing note fields; `plan author` renders the same inventory in the plan review prose.
- Probe: workflow cases pass `--gap-policy defer` by default with per-case override; the auth case (and adapter-repo workflow cases) run drained unattended.
- Cross-repo (`augentic/emery-adapters`): build and review prompts consume `deferred[]` as out-of-scope; RFC-90 verification excludes deferred requirements from its surface.
- Diagnostics (exit 2): `plan-deferral-invalid` (bad selector / missing reason / non-gap row); `plan-gaps-unresolved` retained for strict mode with defer-facing hints. `plan-waiver-invalid` is deleted with the waiver surface.
- Hard cut (pre-1.0): no compatibility parsing for `unknown-waivers` coverage payloads or `--waive` argv; goldens regenerate.
- Documentation: `AGENTS.md`, `docs/standards/workflow.md`, CLI help, and RFC-86's gap-policy prose gain the disposition vocabulary in the same change (RFC-86 D20 posture — shipped-doc drift is delivery work, not a freeze blocker).

## Acceptance criteria

1. **Strict default preserved:** with no deferrals, no `project.yaml` declaration, and no `--gap-policy` flag (or an explicit `strict`), the gate behaves as RFC-86 shipped it — open `[unknown]` / `[conflict]` refuse build with the rendered inventory; `[divergence]` is listed and allowed.
2. **Durable disposition:** `plan defer` writes a digest-bound fact that covers the requirement across resumes and fresh epochs with no re-supply; `--retract` reopens it; a re-refine that changes the requirement body lapses it and the new row blocks again under strict; a re-refine that restores the exact body revives the deferral without re-assertion.
3. **Policy mode:** under an effective `defer` policy, an execute run that mints unknowns or tied conflicts via `refine-under-epoch` dispositions them at the build gate (one `origin: policy` fact each, with the synthesized reason) and proceeds through build and merge; every deferral is visible in `plan gaps` and the fact log.
4. **Project-level policy:** a `project.yaml` `gap-policy: defer` declaration makes every execute epoch defer with no flags supplied; a per-epoch `--gap-policy strict` overrides it for that run; the effective policy lands on the `plan.execute.started` coverage payload.
5. **Build-scope exclusion:** the build request enumerates the deferred set; a build phase report claiming coverage of a deferred requirement fails `target-build-deferred-covered` under the phase machine before verification dispatches; the build record binds the disposition set and disposition drift is staleness.
6. **Debt conservation:** deferred rows appear in the merged baseline with status preserved (`conflict` rows stay `[conflict]` with both arms' notes intact), the appended self-describing deferral note, and final `REQ-NNN` assigned; the wave-commit fact snapshots the deferred set; `plan archive` prints the carried-debt summary and succeeds.
7. **Boundary visibility:** after a debt-carrying merge, the baseline debt projection lists each carried row with reason, origin, originating change, and age, conflicts rendered separately from unknowns; `plan author` renders the same inventory in the plan review prose.
8. **Projection coherence:** `plan status` next-actions compute over open findings only; a fully-dispositioned plan resumes at `plan execute`; Ready projects only on zero open and zero deferred; debt counts render with conflicts broken out.
9. **Waiver surface deleted:** `--waive` is unknown argv; no coverage payload carries `unknown-waivers`; no path demands re-supplying a decision already on the log.
10. **Unattended eval:** `cargo make eval auth --restart` runs `init → author → execute → drained` with no operator gesture, including when synthesis mints `[unknown]` rows; the retained sandbox shows the deferral trail.
11. **Multi-writer:** deferral facts from two writers union losslessly; duplicate deferrals of one `(slice, digest)` project one disposition; retraction from either writer reopens.
12. `cargo make ci` passes with crate-level integration coverage for dispositions, lapse-on-digest-change, revival, policy layering, gate-time policy minting, build-scope exclusion, debt-through-merge, the baseline debt projection, and the strict-mode regression suite.

## Open questions

All product / contract questions are **closed**. Items below are the decision trail, in the original numbering. The governing posture across every closure: **keep going unless told not to** — the loop never parks under a declared `defer` policy, and the review moment moves to the change boundaries (D9).

### Closed — decision trail

1. ~~**Conflict deferral (D6).**~~ **Closed — D6.** `[conflict]` defers under **both** surfaces — the explicit `plan defer` act and the `defer` policy — with the same exclusion semantics as `[unknown]`; build-over stays forbidden. Every conflict reaching the gate has already been through the authority walk (per-slice override → document ordering) and survived only as a top-class tie, so there is no automatic resolution left to protect, and an unattended loop must not park permanently on a disagreement orthogonal to the rest of its slice. The residual risk — adjacent `agreed` requirements silently assuming one arm — is met with **visibility, not blocking**: deferred conflicts are counted and rendered separately from deferred unknowns in `plan gaps`, `plan status`, the archive summary, and the D9 projection. Rejected: hard-blocking as RFC-86 shipped (parks autonomy on exactly the residue authority could not settle); explicit-act-only conflict deferral (same park under unattended runs; the human review moment moves to the boundaries instead).
2. ~~**Default policy trajectory.**~~ **Closed — D3.** The effective policy resolves in three layers per epoch: `--gap-policy` flag → `project.yaml` `gap-policy:` declaration → built-in `strict`. The declaration is written by `emery init --gap-policy` (never hand-edited), so iterate-fast and eval/CI-shaped projects say "keep going" once and never re-supply it; fresh projects keep the strict gate. `defer` never becomes the built-in default — opting in is an explicit, durable operator gesture, consistent with "the invocation says what it authorizes." Rejected: flipping the global built-in to `defer`; keeping the policy per-epoch-flag-only (re-taxes every run for a standing posture).
3. ~~**Deferral match key.**~~ **Closed — D2.** The match key is `(slice, digest)`; the fact's `req` id is advisory presentation. Two identical bodies in one slice share one digest and therefore one disposition — same content, same decision. A lapsed deferral whose exact body returns in a later refine **revives**: liveness is recomputed from the fact union against the live model, never re-asserted — re-forcing a decision already on the log is a park-shaped tax against the keep-going posture. Rejected: digest-only matching across slices; requiring re-assertion after revival.
4. ~~**Baseline shape for deferred conflicts.**~~ **Closed — D5.** Fold with `Status: conflict` preserved and both arms' `Note:` lines intact; baseline merge validation learns to carry conflict rows. The fold appends a self-describing `Note:` (reason, origin, originating change, date) so the baseline alone answers "what are we carrying and why" — the D9 projection never joins archived fact logs. Rejected: downgrading to `unknown` with the arms as notes (loses the typed distinction the corrective change needs to re-adjudicate).
5. ~~**Reason on policy deferrals.**~~ **Closed — D3.** Synthesized reason (`deferred by gap-policy under epoch <ts>`); the fact already carries `origin: policy` and the epoch. Rejected: mandatory `--reason` on the policy invocation — friction with no reader on unattended runs.
6. ~~**Archive posture.**~~ **Closed — D9.** Always advisory. A threshold gate at archive is a park by another name and contradicts the keep-going posture; the backstop for accumulated debt is the boundary review D9 provides — the archive summary behind, the baseline debt projection (rendered by `plan author`) ahead. Rejected: opt-in refusal above N carried debts / conflicts.
7. ~~**Verification depth (RFC-90 seam).**~~ **Closed — hand-off.** This RFC ships the typed coverage gate (`target-build-deferred-covered`, checking the build phase report's `covered[]` under RFC-90's machine) and the exclusion set on the request envelope every phase dispatch reads; whether verification additionally asserts positive *absence* — that no product code implements a deferred requirement's scenarios — is [RFC-90](rfc-90-build-verification.md)'s decision, taken there with `deferred[]` in hand (D4).
8. ~~**Retraction surface.**~~ **Closed — D3.** Dedicated `plan defer --retract` appending `gap.deferral-retracted`. Retraction is a domain act with domain validation (must name a live deferral) and a typed projection join; routing one use case through a generic fact-retraction surface trades that for genericity with no second consumer. Rejected: a generic `fact.retracted` surface as the operator-facing retraction verb.
9. ~~**Debt aging.**~~ **Closed — split.** The baseline debt projection — with age, origin, and reason read from D5's self-describing note — is **promoted into this RFC** (D9): under a defer-default posture it is the replacement for the strict gate's forcing function, not optional polish. Everything beyond the projection — nagging, SLAs, thresholds, cross-change dashboards — stays out of scope as a roadmap candidate. Rejected: leaving all visibility to next-plan-time `plan gaps` (it projects the live change's inventory, not the carried baseline).

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
- **Explicit-act-only conflict deferral** — parks unattended runs on the residue the authority walk already failed to settle; the adjacent-arm risk is met with separated conflict visibility, not a block (resolved question 1).
- **Flipping the built-in default to `defer`** — the bias is a per-project declaration, not a global flip; fresh projects keep the strict gate until the operator says otherwise (resolved question 2).
- **An archive threshold gate on carried debt** — a park by another name; the boundary projections are the backstop (resolved question 6).
- **Mandatory `--reason` on the `defer` policy invocation** — friction with no reader on unattended runs; every policy fact carries origin, epoch, and a synthesized reason (resolved question 5).
- **Downgrading deferred conflicts to `unknown` at baseline merge** — loses the typed distinction the corrective change needs (resolved question 4).

## Non-goals

- Changing Evidence schemas, the authority ranking, synthesis status derivation, or when `[unknown]` / `[conflict]` / `[divergence]` are minted.
- Permitting generation to build over (implement, guess at, or scaffold around) any gap-status requirement under any disposition or policy.
- New plan milestones or a projected rung between Ready and Authorized.
- A cross-change debt-tracking product, dashboards, SLAs, or nagging on carried debt — resolved question 9 promotes only the baseline debt projection (D9) into scope; aging machinery beyond it is a roadmap candidate.
- Multi-operator countersign on deferrals — one writer's fact suffices, matching RFC-86's waiver posture.
- Reintroducing any `approve` vocabulary — execute remains the sole privileged-start surface; deferral is a disposition, not an authorization.
