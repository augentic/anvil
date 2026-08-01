<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Drive a slice manually</h1>

Resume or run one slice phase when `emery plan execute` parks or when you want operator control.

<div class="meta-row">

<span class="meta-chip"><strong>Assumes</strong> Reviewed plan</span>

<span class="meta-chip"><strong>Skill</strong> emery plan execute breakout</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when execute stops on build or merge failure, when you cancel a running execute session, or when you want to drive refine → build → merge phases yourself. Complete the [Quick start](../tutorials/quick-start.md) first.
</div>


<section id="build-failure" markdown="1">

<h2><span class="num">1</span> When execute parks on build failure</h2>

1. Read the stop hint — note `failing-task` and `log-path`.
2. Fix the code or configuration issue.
3. Re-run build for the active slice:

```text
/emery:build <slice-name>
```

4. If build succeeds, either merge by hand or resume execute:

```text
/emery:merge <slice-name>
```

—or—

```text
emery plan execute
```

Execute re-enters at the active `in-progress` entry and skips phases already complete.
</section>


<section id="merge-conflict" markdown="1">

<h2><span class="num">2</span> When execute parks on merge conflict</h2>

1. Read conflicting baseline paths from the stop hint.
2. Re-run refine against the current baseline, or hand-edit the slice's spec body prose (never the kernel-rendered `ID:` / `Sources:` / `Status:` lines).
3. Retry merge:

```text
/emery:merge <slice-name>
```
</section>


<section id="merge-postflight-failed" markdown="1">

<h2><span class="num">3</span> When execute stops on merge postflight failure</h2>

A postflight failure is non-rollback: the merge already committed, the slice is archived, and the plan entry is `done`. Do **not** retry `/emery:merge` for that archived slice and do not treat the stop as a baseline conflict.

1. Inspect the archived gate report at `.emery/archive/<date>-<slice>/merge/postflight.yaml`.
2. Repair the unclean baseline (hand-fix, or author a follow-up slice via `/emery:plan`).
3. Re-run execute to acknowledge the sticky stop and continue (or finalize when the plan is otherwise drained):

```text
emery plan execute
```

`emery plan status` keeps projecting `stop merge-postflight-failed` (`resume: emery plan execute`) until that re-run emits `plan.merge-postflight.acknowledged`.
</section>


<section id="breakout" markdown="1">

<h2><span class="num">4</span> Breakout mid-execute</h2>

Cancel a running `emery plan execute` session and drive phases yourself:

```text
/emery:build <slice-name>
/emery:merge <slice-name>
emery plan execute
```

The execute loop reads on-disk lifecycle state — no resume flags required.
</section>


<section id="plan-lock" markdown="1">

<h2><span class="num">5</span> Guest lock</h2>

The `emery plan execute` loop holds the create-exclusive `.emery/guest.lock` marker for the run's lifetime; a second driver session exits with `guest-marker-held` (exit 2). Standalone breakouts (`slice refine`, `slice build`, `slice merge`) do not take the marker — the lifecycle gates (only `refined` builds, only `built` merges) are the correctness fence. If a dead holder left a stale marker, confirm the holder is gone and remove `.emery/guest.lock` by hand.
</section>


> [!IMPORTANT]
> **Gate reminder.** Per-entry `done` is only written by `/emery:merge` (or execute after a successful merge). Build success alone does not close the slice.

<div class="see-also">
<strong>See also</strong>

- [emery plan execute](../reference/cli/plan.md#emery-plan-execute) — stop conditions and re-entry
- [Drop down a layer](drop-down-a-layer.md) — full manual Layer 2 control
- [Slice skills](../reference/slice-skills/index.md) — refine, build, merge reference
</div>

