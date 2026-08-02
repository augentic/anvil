<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Drive a slice by hand</h1>

Run one slice through refine, build, and merge one phase at a time — without `emery plan execute`. When you finish, you can recover any plan that stops mid-loop, because you have driven every phase yourself.

<div class="meta-row">

<span class="meta-chip"><strong>Time</strong> ~30 min</span>

<span class="meta-chip"><strong>Target</strong> Omnia</span>

<span class="meta-chip"><strong>Outcome</strong> One hand-driven merged slice</span>

</div>

</div>


<section id="outcome" markdown="1">

<h2><span class="num">1</span> What you will build</h2>

The same one-slice change as the [Quick start](quick-start.md), driven phase by phase. `emery plan execute` is a loop over three orchestrations — refine, build, merge — and each is invokable directly as a **[breakout](../appendices/glossary.md#b)**: `/emery:refine`, `/emery:build`, `/emery:merge`. You will use them here on a healthy plan so that when a real plan [parks](../appendices/glossary.md#p) on a failure, the recovery moves are already familiar. The same hand-driven rhythm is mandatory for workspace plans, which refuse the execute loop entirely.
</section>


<div class="prereq">
<strong>Prerequisites.</strong>

- Completed [Quick start](quick-start.md)
- An Omnia-initialised project (`/emery:init omnia` in a fresh or disposable repo is fine)
</div>


<section id="steps" markdown="1">

<h2><span class="num">2</span> Steps</h2>


<div class="tutorial-step" data-step="01">
<div class="step-label">01</div>
<h3 class="step-title">Plan a one-slice change</h3>

```text
/emery:plan health-check source intent=intent:value:"add a health-check endpoint"
```

The skill exits after authoring with one slice row. Note the slice name in `plan.yaml` — the steps below call it `<slice>`; substitute the name your plan shows.
</div>


<div class="tutorial-step" data-step="02">
<div class="step-label">02</div>
<h3 class="step-title">Advance the entry with plan advance</h3>

Instead of running execute, advance the entry directly:

```bash
emery plan advance
```

`plan advance` is the only writer of per-entry `in-progress`: it returns the active entry if one exists, otherwise it advances the next eligible `pending` entry. Checkpoint — `plan.yaml` now shows your entry at `status: in-progress`.

The review obligation is the same as on the execute path — read `change.md` and `plan.yaml` before this step.
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Refine</h3>

```text
/emery:refine <slice>
```

The breakout runs the same refine phase the execute loop runs: extract per bound source, synthesis, validation, and the `refined` transition. Checkpoint — `.emery/slices/<slice>/` now holds `proposal.md`, `specs/<domain>/spec.md`, `design.md`, `tasks.md`, `model.yaml`, and `evidence/intent.yaml`, and `metadata.yaml` reads `status: refined`.
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Build</h3>

```text
/emery:build <slice>
```

The build orchestration drives the target adapter's build operation, validates the report, and gates the `built` transition. Checkpoint — source code changes have landed in your project tree (not under `.emery/`), the checkboxes in `tasks.md` are complete, and `metadata.yaml` reads `status: built`.

The lifecycle gates are the safety fence for hand-driving: build refuses a slice that is not `refined`, and merge refuses a slice that is not `built` — you cannot run phases out of order.
</div>


<div class="tutorial-step" data-step="05">
<div class="step-label">05</div>
<h3 class="step-title">Merge</h3>

```text
/emery:merge <slice>
```

Merge applies the slice's spec deltas to the baseline at `.emery/specs/`, archives the slice directory, and stamps the plan entry `done` — merge is the only writer of per-entry `done`. Checkpoint:

```bash
emery plan status
```

```text
plan: health-check
entries: 1 done / 0 in-progress / 0 pending
drained — run /emery:finalize health-check
```

The plan is [drained](../appendices/glossary.md#d): no entry is `pending` or `in-progress`.
</div>


<div class="tutorial-step" data-step="06">
<div class="step-label">06</div>
<h3 class="step-title">Finalize</h3>

Publish the repository changes through your normal Git workflow, then:

```text
/emery:finalize health-check
```
</div>


</section>


> [!TIP]
> **Done.** You drove `plan advance` → refine → build → merge yourself. On a multi-slice plan the loop is the same — repeat `plan advance` and the three breakouts per entry — and you can hand the plan back to `emery plan execute` at any point: it reads on-disk lifecycle state and skips phases already complete.

## What you learned

- `emery plan execute` is a loop over three orchestrations you can invoke directly as breakouts.
- `emery plan advance` advances entries; the slice lifecycle gates (`refined` before build, `built` before merge) enforce phase order; merge alone writes per-entry `done`.
- Hand-driven and loop-driven slices are interchangeable — the orchestrations do not know which is driving them.

<div class="see-also">
<strong>See also</strong>

- [Drive a slice manually](../how-to/drive-slice-manually.md) — the recovery recipes when execute parks for real
- [Cross-repo changes](cross-repo-change.md) — where hand-driving is mandatory
- [Lifecycle](../reference/lifecycle.md) — the state machines behind the gates
- [The layered stack](../explanation/layered-stack.md) — why every layer is invokable on its own
</div>
