# Assertion taxonomy

Legacy operator detail for assertion ids now typed by `crates/scenario` and declared in `quality/scenarios/*.yaml`. Each id maps to **exactly one** grading mechanism:

- **Probe** — a deterministic post-run check: a CLI read verb (`specify plan status`, `specify journal show`, …), an artifact predicate, or a `jq` projection over a read verb's output. Probes never drive the workflow and never transition anything; probe output is evidence, never a transition.
- **Semantic rubric** — meaning or usefulness that no deterministic predicate can decide. Live profiles grade it with an evidence pointer and calibrated score.

Canonical profiles may automate setup, driving, and hard grading. The remaining negative expectations prohibit live-model calls in ordinary CI (`live-model-ci-required`) and byte-golden semantic grading (`semantic-byte-golden-required`).

Conventions:

- Run probes from the scenario sandbox root (`evals/.sandbox/<id>/`, or the workspace root for workspace scenarios) after the stage the assertion grades.
- Journal probes read through `specify journal show [--filter <event-id-prefix>] [--limit N]` — text mode emits the journal's canonical JSONL lines, so payload projection pipes to `jq -c .payload`. Wire shape per line: `{ timestamp, event, payload }` with kebab-case payload keys. Probes never read `.specify/journal.jsonl` directly.
- `<slice>`, `<key>`, `<plan>` placeholders come from the run's own plan; substitute before running.
- Record probe output (or its absence) as the **Evidence** entry in the [run-template](run-template.md) assertion table for any non-`pass` verdict; on `pass` the probe command itself is sufficient evidence.

Ids are deliberately shared across scenarios. `scenario::AssertionId` is the closed executable registry; this document supplies expanded legacy probe guidance while migration replaces registered probes with typed YAML probes or profile-specific evaluators.

## Shared assertions

### `composed-init-succeeds`

The hosted workflow guest completes the canonical `composed-init` command with exit code 0.

**Probe.** Typed `exit-code` probe in `quality/scenarios/composed-init.yaml`.

### `project-scaffold-written`

The composed init writes `.specify/project.yaml` through the writable project preopen.

**Probe.** Typed `path-exists` probe in `quality/scenarios/composed-init.yaml`.

### `plan-exists`

Used by every catalog scenario except `guest-execute-loop`, whose assertions start at the composed runtime's execute result. `plan.yaml` exists at the driving root after `/spec:plan` returns. Scenarios that execute (`workspace-two-projects`, `workspace-fail-resume`, `workspace-stale-recovery`) additionally expect `lifecycle: approved` before `specify plan execute` — the stamp the operator (or the agent at the operator's direction, with `--actor agent`) applied at Gate 1.

**Probe.**

```bash
test -f plan.yaml && echo present
grep '^lifecycle:' plan.yaml    # `pending` at Gate 1; `approved` before execute
```

### `plan-validates`

Used by `intent-only`, `documentation-one-slice`, `documentation-multi-slice`, `typescript-multi-slice`, `lead-reconciliation`, `single-project-plan`, `contract-lifecycle`. `specify plan validate` exits cleanly (no blocking findings) at the point the scenario names — after the draft, and again after any amendment step.

**Probe.**

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0
```

### `execute-loop-all-done`

Used by `documentation-one-slice`, `contract-lifecycle`, `execute-pause-resume`, `workspace-two-projects`, `workspace-fail-resume`, `workspace-stale-recovery`. `specify plan execute` exits because the plan is complete — every per-entry status is `done` and the scheduler reports drained — not because it parked, failed, or was interrupted.

**Probe.**

```bash
specify plan next --format json     # expect {"reason":"drained", ...}
grep -c 'status: done' plan.yaml    # equals the slice count
specify journal show --filter plan.entry.advanced    # one advance per slice, none after the last merge
```

## `intent-only`

### `intent-single-lead`

The degenerate `intent` survey produces exactly one lead, and propose writes exactly one slice from it.

**Probe.**

```bash
grep -c '^- lead: ' discovery.md    # expect 1
specify journal show --filter plan.reconcile.completed | jq -c .payload    # expect "slice-count":1
```

### `gate-1-not-auto-stamped`

`/spec:plan` exits at `pending` and prints the literal `specify plan transition <plan> approved` command; the skill never stamps `approved` itself. The stamp that later appears in the journal is the operator's.

**Probe.** Immediately after the skill run, before the operator stamps:

```bash
grep -q 'lifecycle: pending' plan.yaml && echo pending
specify journal show --filter plan.transition.approved    # expect no output yet
```

After the operator stamp, the single approval event carries the actor (`--actor` on `specify plan transition`, default `operator`; an agent stamping on the operator's literal instruction passes `--actor agent`):

```bash
specify journal show --filter plan.transition.approved | jq -c .payload    # exactly one; records who stamped
```

### `sources-intent-only`

The slice's provenance is `Sources: [intent]` and nothing else, on every requirement.

**Probe.**

```bash
grep -h '^Sources:' .specify/slices/<slice>/specs/*/spec.md | sort -u    # the only key is `intent` (rendered `Sources: intent` or `Sources: [intent]`)
specify slice provenance <slice> --format json    # every requirement's sources == ["intent"]
```

### `refine-reaches-refined`

After Gate 1, `specify plan execute` drives the entry through the refine orchestration (the `/spec:refine` breakout runs the same verb); the slice validates cleanly and transitions to `refined`.

**Probe.**

```bash
specify slice validate <slice>; echo "exit=$?"    # expect exit=0
specify journal show --filter slice.transition.refined    # expect one event naming <slice>
```

## `documentation-one-slice`

### `single-slice-from-doc`

The single bound docs path maps to exactly one proposed slice.

**Probe.**

```bash
specify journal show --filter plan.reconcile.completed | jq -c .payload    # expect "slice-count":1
grep -c '^  - name: ' plan.yaml    # expect 1
```

### `sources-documentation-only`

The slice's provenance is `Sources: [<doc-key>]` (the documentation binding key) and nothing else.

**Probe.**

```bash
grep -h '^Sources:' .specify/slices/<slice>/specs/*/spec.md | sort -u    # expect exactly `Sources: [<doc-key>]`
```

## `documentation-multi-slice`

### `multiple-slices-proposed`

The multi-feature docs path maps to more than one proposed slice.

**Probe.**

```bash
specify journal show --filter plan.reconcile.completed | jq -c .payload    # expect "slice-count" > 1
```

### `cross-cutting-lead-multi-homed`

The cross-cutting conventions lead appears in the `sources:` of more than one proposed slice (multi-homing implies no `depends-on` edge between the hosts), and `change.md` lists it under `## Cross-cutting leads`.

**Probe.**

```bash
grep -c 'source: conventions' plan.yaml    # expect >= 2
grep -A4 '^## Cross-cutting leads' change.md    # expect the conventions lead listed
```

### `propose-edit-reject-loop`

An operator edit or reject applied through `specify plan amend` (never a hand-edit) is reflected in `plan.yaml`.

**Probe.** Capture the amend command from the run, then:

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0 after the amendment
grep -A6 '  - name: <amended-slice>' plan.yaml    # shows the amended/removed shape
```

### `gate-1-amendment`

The plan remains at `pending` after amendment and the closing hint still prints the literal transition command — amendment does not consume or skip Gate 1.

**Probe.**

```bash
grep -q 'lifecycle: pending' plan.yaml && echo pending
specify journal show --filter plan.transition.approved    # expect no output (scenario never stamps)
```

## `typescript-multi-slice`

### `multiple-slices-from-code`

The bound legacy TypeScript repo maps to more than one slice.

**Probe.**

```bash
specify journal show --filter plan.reconcile.completed | jq -c .payload    # expect "slice-count" > 1
```

### `sources-legacy-only`

Each slice's provenance is `Sources: [<legacy-key>]` — the survey attributed every lead to the legacy binding and nothing else leaked in.

**Probe.**

```bash
grep -h 'source: ' plan.yaml | sort -u    # expect only the legacy key
```

### `no-under-slicing`

Distinct legacy behaviors are not collapsed into one slice. Whether two surveyed surfaces are "distinct behaviors" is a reading of the legacy code, not a mechanical property.

**Judgment flag.** Evidence pointer: the `## Lead inventory` blocks in `discovery.md` set against the proposed `plan.yaml` slices — name the legacy surfaces (routes/handlers) and show each landed in a sensible slice rather than one catch-all.

## `lead-reconciliation`

### `merged-slice-combines-sources`

The propose step merges the same candidate surfaced by two adapters: the merged slice's `sources:` lists both contributing keys.

**Probe.**

```bash
grep -B2 -A6 '  - name: <merged-slice>' plan.yaml    # sources: lists both keys
```

### `tentative-merge-surfaced`

Any merge the propose step was uncertain about is surfaced under `## Tentative merges` in `change.md` rather than silently committed. Whether a given merge *should* have been flagged tentative is a judgment on the agent's own confidence, not a mechanical property.

**Judgment flag.** Evidence pointer: the `## Tentative merges` section of `change.md` (or its absence) plus the lead synopses that motivated the merge.

### `amend-overrides-merge`

`specify plan amend` can split or rebind a wrong merge at Gate 1, and the override lands in `plan.yaml`.

**Probe.** Capture the amend command from the run, then:

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0 after the override
grep -A6 '  - name: <amended-slice>' plan.yaml    # shows the post-override binding
```

### `extract-runs-per-contributing-source`

Downstream `extract` runs once per contributing source — one Evidence document per `(source, slice)` pair.

**Probe.**

```bash
ls .specify/slices/<slice>/evidence/    # one <key>.yaml per contributing source
specify journal show --filter slice.extract.completed | jq -c .payload    # one event per (source, slice)
```

## `single-project-plan`

### `slices-match-expected-shape`

Plan entries are named, scoped, and ordered consistently with the brief — slice naming quality and scope fidelity are a reading of the brief.

**Judgment flag.** Evidence pointer: `plan.yaml` slice names/descriptions set against the brief's Goals/Scope sections, noting any goal with no home or slice with no grounding.

### `no-project-routing-required`

Single-project planning invents no project routing: no `registry.yaml`, and no registry-derived assignments on the entries.

**Probe.**

```bash
test ! -f registry.yaml && echo no-registry
grep 'project:' plan.yaml | sort -u    # absent, or only the sole project synthesised from project.yaml
```

## `contract-lifecycle`

### `contract-slice-first`

The dependency graph makes the contract slice the first feature slice and places it before both implementation slices. A target-required bootstrap slice may execute before it without violating the contract-first feature ordering.

**Probe.**

```bash
grep -n 'name: \|project: ' plan.yaml    # contract entry precedes backend/mobile implementation entries
```

### `implementation-slices-routed`

Exactly two implementation slices route to `backend` and `mobile`.

**Probe.**

```bash
grep -c 'project: backend' plan.yaml    # expect 1
grep -c 'project: mobile' plan.yaml     # expect 1
```

### `dependencies-contract-before-implementations`

Each implementation slice's `depends-on` includes the contract slice.

**Probe.**

```bash
grep -A4 'project: backend' plan.yaml | grep 'depends-on'    # names the contract slice
grep -A4 'project: mobile' plan.yaml | grep 'depends-on'     # names the contract slice
```

### `draft-stops-at-handoff`

`/spec:plan` exits at the hand-off (`pending`) without executing, pushing, or finalizing.

**Probe.** Immediately after the draft:

```bash
grep -q 'lifecycle: pending' plan.yaml && echo pending
specify journal show --filter plan.entry.advanced    # expect no output
```

### `review-step-no-op`

Read-only review between draft and execute reports the plan as authored — inspection does not mutate it.

**Probe.**

```bash
shasum plan.yaml    # capture before review; identical after review
specify plan validate --format json; echo "exit=$?"    # expect exit=0
```

### `workspace-branches-prepared`

Routed project work happens on `specify/<plan>` branches in the project slots.

**Probe.**

```bash
git -C workspace/backend branch --show-current    # expect specify/<plan>
git -C workspace/mobile branch --show-current     # expect specify/<plan>
```

### `publication-complete-before-finalize`

The operator publishes every routed project's completed branch before invoking `/spec:finalize`; finalize performs no Git or forge operations.

**Probe.**

```bash
for proj in backend mobile contracts; do    # every routed project; trim to the set the plan routed to
  git -C "$proj" ls-remote --heads origin "refs/heads/specify/<plan>"    # branch present on the bare remote
done
```

### `finalize-archives-plan`

`/spec:finalize` archives the plan via `specify plan archive` only after operator-owned publication is confirmed.

**Probe.**

```bash
ls .specify/archive/plans/    # expect <plan>-<YYYYMMDD>/ (or .yaml) entry
test ! -f plan.yaml && echo archived
```

### `archived-plan-path-recorded`

The finalize wrap-up names the archived plan path under `.specify/archive/plans/` — a reporting-ergonomics claim about the skill's closing output, not about the archive itself (which `finalize-archives-plan` probes).

**Judgment flag.** Evidence pointer: the captured second-`/spec:finalize` wrap-up output showing the archived path.

### `archived-change-md-present`

The archived directory contains the archived `change.md` next to the archived `plan.yaml`.

**Probe.**

```bash
ls .specify/archive/plans/<plan>-*/    # expect change.md and plan.yaml together
```

### `publication-confirmation-recorded`

The captured finalize interaction records the operator's publication confirmation before the archive command runs.

**Judgment flag.** Evidence pointer: the captured `/spec:finalize` interaction showing publication confirmation before the archived path.

### `rerun-finalize-plan-not-found`

A second `/spec:finalize` reports no active plan remains and exits 0 — clean re-entry, not an error.

**Probe.**

```bash
test ! -f plan.yaml && echo no-active-plan    # precondition holds
# the captured third invocation exits 0 with the no-active-plan report
```

## `target-shape`

### `spec-reflects-shape-idioms`

`spec.md` reflects the target `shape` brief's idiom guidance — whether synthesized prose honours idioms is a quality reading.

**Judgment flag.** Evidence pointer: the shape-derived sections of `.specify/slices/<slice>/specs/*/spec.md` quoted against the target's `shape` brief.

### `design-reflects-shape-idioms`

`design.md` reflects the target `shape` idiom guidance (provider DI, error conventions, validation placement).

**Judgment flag.** Evidence pointer: the matching `design.md` sections quoted against the `shape` brief's named idioms.

### `intent-and-doc-fixtures-agree`

The intent-driven and documentation-driven fixtures honour the same `shape`-derived sections — agreement on structure and idioms, never a byte compare.

**Judgment flag.** Evidence pointer: the two fixtures' shape-derived sections side by side, noting structural agreement and any divergence.

## `execute-pause-resume`

### `breakout-state-consistent`

After cancelling `specify plan execute` and running `/spec:build` directly, on-disk slice and plan state is consistent — nothing half-written, exactly one active entry.

**Probe.**

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0
grep -c 'status: in-progress' plan.yaml    # expect 1
specify slice validate <active-slice>; echo "exit=$?"    # expect exit=0
```

### `execute-resumes-without-flags`

Re-invoking `specify plan execute` resumes from the in-progress entry with no extra flags — the scheduler returns the active entry rather than advancing a new one.

**Probe.**

```bash
specify journal show --filter plan.entry.advanced | jq -c .payload    # no duplicate advance for the in-progress slice across the cancel/resume window
```

## `execute-fail-resume`

### `build-failure-stop-hint`

The loop parks on the build failure with a structured stop hint naming the failed task/slice, leaving the entry `in-progress`. The stop classification is CLI-rendered: `specify plan status` projects the journal's `slice.build.failed` into `stop build-failed`.

**Probe.** At the parked state:

```bash
specify journal show --filter slice.build.failed | jq -c .payload    # names the slice with a non-empty reason
grep -c 'status: in-progress' plan.yaml    # expect 1 — parked, not failed-out
specify journal show --filter plan.entry.advanced    # no new advance after the failure — parked and did not advance
specify plan status --format json    # expect "action":"stop" with "stop".reason == "build-failed"
```

### `build-resumes-from-failed-task`

After the operator's fix, the build resumes from the failed task rather than restarting the slice.

**Probe.** Capture `tasks.md` task states at the park, then after resume:

```bash
specify journal show | jq -c 'select(.event | startswith("slice.build.") or startswith("slice.synthesize."))'    # failed -> started again, with no re-synthesis between
diff <(parked tasks.md states) .specify/slices/<slice>/tasks.md    # previously-completed tasks stay completed
```

### `loop-continues-to-merge`

The resumed loop continues through merge to drained.

**Probe.**

```bash
specify journal show --filter slice.merge.succeeded | jq -c .payload    # fires for the slice
specify journal show --filter slice.archive.created | jq -c .payload    # fires for the slice
specify plan next --format json    # expect {"reason":"drained", ...}
```

## `workspace-two-projects`

### `per-slice-project-routing`

Each slice runs against its routed project slot — the work lands in the slot the plan routes it to.

**Probe.**

```bash
grep 'project:' plan.yaml    # each slice names its routed project
git -C workspace/backend log --oneline specify/<plan>    # backend slice commits in the backend slot
git -C workspace/mobile log --oneline specify/<plan>     # mobile slice commits in the mobile slot
```

### `slots-materialised`

`workspace/backend/` and `workspace/mobile/` are materialised by operator-owned setup before planning or execution.

**Probe.**

```bash
test -d workspace/backend && test -d workspace/mobile && echo materialised
```

### `plan-lock-at-workspace`

The plan lock is held at the workspace while phase work runs in the slots — the workspace root owns `plan.yaml` and every per-entry status write; no slot grows its own plan. An unlocked mutating plan command is refused with `plan-lock-not-held`.

**Probe.**

```bash
test -f plan.yaml && echo workspace-owns-plan
ls workspace/*/plan.yaml 2>/dev/null    # expect no output
specify journal show --filter plan.entry.advanced    # advances recorded in the workspace journal, not a slot's
specify plan next --format json    # after lock release, expect plan-lock-not-held rather than mutation
```

## `workspace-fail-resume`

### `breakout-routes-to-slot`

`/spec:build` invoked from the workspace routes into the parked slice's project slot.

**Probe.**

```bash
git -C workspace/backend status --short    # the breakout build's changes are in the routed slot
specify journal show --filter slice.build. | jq -c .payload    # the breakout's started/succeeded pair
```

### `active-slice-resolved-across-boundary`

The breakout verb resolves the active slice across the workspace/slot boundary — the operator names no slice; the build runs against the parked one.

**Probe.**

```bash
specify journal show --filter slice.build.started --limit 1 | jq -c .payload    # names the parked slice
grep -B2 'status: in-progress' plan.yaml    # the same slice is the active entry
```

### `chdir-without-operator-intervention`

The correct `chdir` into the slot happens without the operator changing directories — an operator-experience claim; the durable routing residue is `breakout-routes-to-slot`'s probe.

**Judgment flag.** Evidence pointer: the captured breakout stage output showing the operator stayed at the workspace root while the build reported work in the slot.

## `guest-execute-loop`

### `guest-loop-drained`

The composed runtime's `plan execute` (the workflow guest's inverted loop) exits 0 reporting drained, and the plan entry is stamped `done` by the guest merge.

**Probe.**

```bash
# the captured `plan execute` invocation exits 0 with the drained envelope
specify plan status --format json    # expect "action":"drained"
grep -c 'status: done' plan.yaml     # equals the slice count
```

### `guest-journal-cadence`

The guest loop journalled the full per-slice cadence over the `"."` preopen — claim, extract, synthesize, build, merge, archive — in claim order.

**Probe.**

```bash
specify journal show --filter plan.entry.advanced | jq -c .payload      # one advance per slice
specify journal show --filter slice.synthesize.completed | jq -c .payload
specify journal show --filter slice.build.succeeded | jq -c .payload
specify journal show --filter slice.merge.succeeded | jq -c .payload
specify journal show --filter slice.archive.created | jq -c .payload    # no merge-sha key (guest skips git)
```

### `guest-generated-crate-verifies`

The `target:omnia` guest build's generated crate passes its own verification — the generated-output-correctness gate; a schema-valid `build/report.yaml` alone does not count the slice done.

**Probe.**

```bash
ls crates/                                   # the generated crate directory exists
cargo check --manifest-path crates/*/Cargo.toml; echo "exit=$?"    # expect exit=0
```

### `guest-marker-released`

The guest execute marker is released on the clean exit — no stale `.specify/guest.lock` survives a drained run.

**Probe.**

```bash
test ! -f .specify/guest.lock && echo released
```

### `guest-spec-sensible`

The synthesized baseline spec is a faithful, well-formed rendering of the operator intent — prose quality is a reading, not a mechanical property.

**Judgment flag.** Evidence pointer: the merged `.specify/specs/<domain>/spec.md` quoted against the plan's intent string, noting requirement fidelity and `Sources: intent` provenance.

## `workspace-stale-recovery`

### `dirty-slot-preserved`

Operator inspection after the interruption confirms the dirty/uncommitted slot remains intact before it is made safe for resume.

**Probe.** Grade the captured inspection step:

```bash
git -C workspace/<project> status --short    # interrupted work is present before operator reconciliation
```

### `slice-state-preserved`

Slice state survives the interruption — no lost or duplicated work.

**Probe.**

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0
specify slice validate <interrupted-slice>; echo "exit=$?"    # expect exit=0
specify journal show --filter plan.entry.advanced | jq -c .payload    # exactly one advance per slice — no duplicates from the interruption
```

### `resume-continues-from-in-progress`

Resume continues from the in-progress entry, not a restart — the scheduler returns the active entry; nothing is re-advanced or re-synthesized.

**Probe.**

```bash
specify journal show --filter plan.entry.advanced | jq -c .payload    # no new advance for the interrupted slice at resume
specify journal show --filter slice.synthesize.started | jq -c .payload    # no second synthesis for the interrupted slice
```
