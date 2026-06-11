<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Drive a slice manually</h1>

Resume or run one slice phase when `/spec:execute` parks or when you want operator control.

<div class="meta-row">

<span class="meta-chip"><strong>Assumes</strong> Reviewed plan</span>

<span class="meta-chip"><strong>Skill</strong> /spec:execute breakout</span>

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
/spec:build <slice-name>
```

4. If build succeeds, either merge by hand or resume execute:

```text
/spec:merge <slice-name>
```

—or—

```text
/spec:execute
```

Execute re-enters at the active `in-progress` entry and skips phases already complete.
</section>


<section id="merge-conflict" markdown="1">

<h2><span class="num">2</span> When execute parks on merge conflict</h2>

1. Read conflicting baseline paths from the stop hint.
2. Re-run refine against the current baseline, or hand-edit the slice's spec body prose (never the kernel-rendered `ID:` / `Sources:` / `Status:` lines).
3. Retry merge:

```text
/spec:merge <slice-name>
```
</section>


<section id="breakout" markdown="1">

<h2><span class="num">3</span> Breakout mid-execute</h2>

Cancel a running `/spec:execute` session and drive phases yourself:

```text
/spec:build <slice-name>
/spec:merge <slice-name>
/spec:execute
```

The execute loop reads on-disk lifecycle state — no resume flags required.
</section>


<section id="plan-lock" markdown="1">

<h2><span class="num">4</span> Plan lock</h2>

Standalone breakouts acquire `.specify/plan.lock` the same way execute does. If you see `plan-lock-busy`, another process holds the lock. When the holder is dead, remove the stale lock file manually. The CLI also probes the lock itself: `specify plan next`, per-entry `specify plan transition`, and `specify slice merge run` refuse with `plan-lock-not-held` (exit 2) when no session holds it — so a breakout that skipped the lock cannot advance plan state.
</section>


> [!IMPORTANT]
> **Gate reminder.** Per-entry `done` is only written by `/spec:merge` (or execute after a successful merge). Build success alone does not close the slice.

<div class="see-also">
<strong>See also</strong>

- [/spec:execute](../reference/change-skills/execute.md) — stop conditions and re-entry
- [Drop down a layer](drop-down-a-layer.md) — full manual Layer 2 control
- [Slice skills](../reference/slice-skills/index.md) — refine, build, merge reference
</div>

