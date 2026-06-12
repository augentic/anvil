# Assertion taxonomy

The executable grading contract for every assertion id used in scenario frontmatter (`assertions[]` in `evals/scenarios/<id>.md`). Each id maps to a definition plus **exactly one** grading mechanism:

- **Probe** — a deterministic post-run check: a CLI read verb (`specify plan status`, `specify journal show`, …), an artifact predicate, or a `jq` projection over a read verb's output. Probes never drive the workflow and never transition anything; probe output is evidence, never a transition.
- **Judgment flag** — the irreducibly human residue (prose quality, decomposition sensibility, ergonomics). Graded by operator or agent **with an evidence pointer** — a verdict without one is not a grade.

Driving stays agent-/operator-led per the scenario's Invocation; grading is what happens after — the two are deliberately separate concerns, so deterministic post-run probes never count as "automating the run". The `negative-expectations` (`automated-runner-added`, `fake-forge-added`, `transcript-replay-added`, `ci-target-added`, `golden-output-required`) constrain *driving*; running these probes after a sweep violates none of them.

Conventions:

- Run probes from the scenario sandbox root (`evals/.sandbox/<id>/`, or the workspace root for workspace scenarios) after the stage the assertion grades.
- Journal probes read through `specify journal show [--filter <event-id-prefix>] [--limit N]` — text mode emits the journal's canonical JSONL lines, so payload projection pipes to `jq -c .payload`. Wire shape per line: `{ timestamp, event, payload }` with kebab-case payload keys. Probes never read `.specify/journal.jsonl` directly.
- `<slice>`, `<key>`, `<plan>` placeholders come from the run's own plan; substitute before running.
- Record probe output (or its absence) as the **Evidence** entry in the [run-template](run-template.md) assertion table for any non-`pass` verdict; on `pass` the probe command itself is sufficient evidence.

Ids are deliberately shared across scenario files; this document is their single definition. If machine enforcement is ever needed, this file becomes a structured carrier plus a lint check that every scenario id resolves here — never a per-scenario `probe` field (the probe definitions live here, in one place, so scenario frontmatter stays declarative).

## Shared assertions

### `plan-exists`

Used by every scenario (13 of 13). `plan.yaml` exists at the driving root after `/spec:plan` returns. Scenarios that execute (`workspace-execute-two-projects`, `workspace-breakout`, `stale-workspace-recovery`) additionally expect `lifecycle: approved` before `/spec:execute` — the stamp the operator (or the agent at the operator's direction, with `--actor agent`) applied at Gate 1.

**Probe.**

```bash
test -f plan.yaml && echo present
grep '^lifecycle:' plan.yaml    # `pending` at Gate 1; `approved` before execute
```

### `plan-validates`

Used by `pure-intent`, `documentation-one-slice`, `documentation-multi-slice`, `code-multi-slice`, `cross-source-merge`, `plan-single-project`, `cross-repo-contract-flow`. `specify plan validate` exits cleanly (no blocking findings) at the point the scenario names — after the draft, and again after any amendment step.

**Probe.**

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0
```

### `execute-loop-all-done`

Used by `documentation-one-slice`, `cross-repo-contract-flow`, `stepthrough-breakout`, `workspace-execute-two-projects`, `workspace-breakout`, `stale-workspace-recovery`. `/spec:execute` exits because the plan is complete — every per-entry status is `done` and the scheduler reports drained — not because it parked, failed, or was interrupted.

**Probe.**

```bash
specify plan next --format json     # expect {"reason":"drained", ...}
grep -c 'status: done' plan.yaml    # equals the slice count
specify journal show --filter plan.entry.advanced    # one advance per slice, none after the last merge
```

## `pure-intent`

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

After Gate 1, `/spec:execute` drives the entry through `/spec:refine`; the slice validates cleanly and transitions to `refined`.

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

## `code-multi-slice`

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

## `cross-source-merge`

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

## `plan-single-project`

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

## `cross-repo-contract-flow`

### `contract-slice-first`

The dependency graph makes the contract slice the first executable slice.

**Probe.** On a fresh approved copy of the plan (or from the recorded first advance):

```bash
specify journal show --filter plan.entry.advanced | head -1 | jq -c .payload    # first advance names the contract slice
```

### `implementation-slices-routed`

Exactly two implementation slices route to `shop-backend` and `shop-mobile`.

**Probe.**

```bash
grep -c 'project: shop-backend' plan.yaml    # expect 1
grep -c 'project: shop-mobile' plan.yaml     # expect 1
```

### `dependencies-contract-before-implementations`

Each implementation slice's `depends-on` includes the contract slice.

**Probe.**

```bash
grep -A4 'project: shop-backend' plan.yaml | grep 'depends-on'    # names the contract slice
grep -A4 'project: shop-mobile' plan.yaml | grep 'depends-on'     # names the contract slice
```

### `draft-stops-at-handoff`

`/spec:plan` exits at the hand-off (`pending`) without executing, pushing, or finalizing.

**Probe.** Immediately after the draft:

```bash
grep -q 'lifecycle: pending' plan.yaml && echo pending
specify journal show --filter plan.entry.advanced    # expect no output
specify journal show --filter workspace.push.completed    # expect no output
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
git -C .specify/workspace/shop-backend branch --show-current    # expect specify/<plan>
git -C .specify/workspace/shop-mobile branch --show-current     # expect specify/<plan>
specify journal show --filter workspace.push.completed | jq -c .payload    # "branch":"specify/<plan>", both projects listed
```

### `finalize-halts-on-unmerged-prs`

The first `/spec:finalize` runs push, observes the fresh PRs unmerged, and halts with `pr-not-merged` naming each PR and URL — it never merges anything itself.

**Probe.**

```bash
test -f plan.yaml && echo still-active    # plan not archived by the first finalize
gh pr list --state open    # the per-project PRs exist and are open
specify journal show --filter workspace.push.completed    # the push completed before the halt
```

### `finalize-archives-plan`

After the external merges, the second `/spec:finalize` archives the plan via `specify plan archive`.

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

### `merged-pr-list-recorded`

The wrap-up lists exactly two merged PRs (one per routed project) with numbers and URLs — reporting ergonomics over the forge state (`gh pr view` cross-checks the underlying facts).

**Judgment flag.** Evidence pointer: the captured second-`/spec:finalize` wrap-up against `gh pr view <n>` for each listed PR (state `MERGED`, one per routed project).

### `rerun-finalize-plan-not-found`

A third `/spec:finalize` reports no active plan remains and exits 0 — clean re-entry, not an error.

**Probe.**

```bash
test ! -f plan.yaml && echo no-active-plan    # precondition holds
# the captured third invocation exits 0 with the no-active-plan report
```

## `target-shape-injection`

### `spec-reflects-shape-idioms`

`spec.md` reflects the target `shape` brief's idiom guidance — whether synthesized prose honours idioms is a quality reading.

**Judgment flag.** Evidence pointer: the shape-derived sections of `.specify/slices/<slice>/specs/*/spec.md` quoted against the target's `shape` brief.

### `design-reflects-shape-idioms`

`design.md` reflects the target `shape` idiom guidance (provider DI, error conventions, validation placement).

**Judgment flag.** Evidence pointer: the matching `design.md` sections quoted against the `shape` brief's named idioms.

### `intent-and-doc-fixtures-agree`

The intent-driven and documentation-driven fixtures honour the same `shape`-derived sections — agreement on structure and idioms, never a byte compare.

**Judgment flag.** Evidence pointer: the two fixtures' shape-derived sections side by side, noting structural agreement and any divergence.

## `stepthrough-breakout`

### `breakout-state-consistent`

After cancelling `/spec:execute` and running `/spec:build` directly, on-disk slice and plan state is consistent — nothing half-written, exactly one active entry.

**Probe.**

```bash
specify plan validate --format json; echo "exit=$?"    # expect exit=0
grep -c 'status: in-progress' plan.yaml    # expect 1
specify slice validate <active-slice>; echo "exit=$?"    # expect exit=0
```

### `execute-resumes-without-flags`

Re-invoking `/spec:execute` resumes from the in-progress entry with no extra flags — the scheduler returns the active entry rather than advancing a new one.

**Probe.**

```bash
specify journal show --filter plan.entry.advanced | jq -c .payload    # no duplicate advance for the in-progress slice across the cancel/resume window
```

## `execute-build-failure`

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

The resumed loop continues through merge to `all-done`.

**Probe.**

```bash
specify journal show --filter slice.merge.succeeded | jq -c .payload    # fires for the slice
specify journal show --filter slice.archive.created | jq -c .payload    # fires for the slice
specify plan next --format json    # expect {"reason":"drained", ...}
```

## `workspace-execute-two-projects`

### `per-slice-project-routing`

Each slice runs against its routed project slot — the work lands in the slot the plan routes it to.

**Probe.**

```bash
grep 'project:' plan.yaml    # each slice names its routed project
git -C .specify/workspace/shop-backend log --oneline specify/<plan>    # backend slice commits in the backend slot
git -C .specify/workspace/shop-mobile log --oneline specify/<plan>     # mobile slice commits in the mobile slot
```

### `slots-materialised`

`.specify/workspace/shop-backend/` and `.specify/workspace/shop-mobile/` are materialised by workspace sync.

**Probe.**

```bash
test -d .specify/workspace/shop-backend && test -d .specify/workspace/shop-mobile && echo materialised
specify journal show --filter workspace.sync.completed | jq -c .payload    # "projects" lists both slots
```

### `plan-lock-at-workspace`

The plan-lock is held at the workspace while phase work runs in the slots — the workspace root owns `plan.yaml` and every per-entry status write; no slot grows its own plan. The CLI enforces the lock at runtime: with no session holding it, the plan-state-writing verbs refuse `plan-lock-not-held` (exit 2).

**Probe.**

```bash
test -f plan.yaml && echo workspace-owns-plan
ls .specify/workspace/*/plan.yaml 2>/dev/null    # expect no output
specify journal show --filter plan.entry.advanced    # advances recorded in the workspace journal, not a slot's
specify plan next --format json    # run after the driver released the lock: expect exit 2, "error":"plan-lock-not-held"
```

## `workspace-breakout`

### `breakout-routes-to-slot`

`/spec:build` invoked from the workspace routes into the parked slice's project slot.

**Probe.**

```bash
git -C .specify/workspace/shop-backend status --short    # the breakout build's changes are in the routed slot
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

## `stale-workspace-recovery`

### `dirty-slot-detected-at-sync`

`specify workspace sync` after the interruption detects the dirty/uncommitted slot rather than clobbering or ignoring it.

**Probe.** Grade the captured resync step:

```bash
git -C .specify/workspace/<project> status --short    # the slot really was dirty at resync time
# the captured `specify workspace sync` output surfaces the dirty-slot diagnostic for that slot
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
