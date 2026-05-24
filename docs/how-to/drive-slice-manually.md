{{#template ../templates/hero-open.md eyebrow=How-to title=Drive a slice manually}}
Resume or run one slice phase when `/spec:execute` parks or when you want operator control.

{{#template ../templates/meta-row-open.md}}
{{#template ../templates/meta-chip.md label=Assumes value=Reviewed plan}}
{{#template ../templates/meta-chip.md label=Skill value=/spec:execute breakout}}
{{#template ../templates/meta-row-close.md}}
{{#template ../templates/hero-close.md}}

{{#template ../templates/when-open.md}}
Use this guide when execute stops on build or merge failure, when you cancel a running execute session, or when you want to drive refine → build → merge phases yourself. Complete the [Quick start](../tutorials/quick-start.md) first.
{{#template ../templates/when-close.md}}

{{#template ../templates/section-open.md id=build-failure num=1 title=When execute parks on build failure}}
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
{{#template ../templates/section-close.md}}

{{#template ../templates/section-open.md id=merge-conflict num=2 title=When execute parks on merge conflict}}
1. Read conflicting baseline paths from the stop hint.
2. Re-run refine against the current baseline, or hand-edit slice specs.
3. Retry merge:

```text
/spec:merge <slice-name>
```
{{#template ../templates/section-close.md}}

{{#template ../templates/section-open.md id=breakout num=3 title=Breakout mid-execute}}
Cancel a running `/spec:execute` session and drive phases yourself:

```text
/spec:build <slice-name>
/spec:merge <slice-name>
/spec:execute
```

The execute loop reads on-disk lifecycle state — no resume flags required.
{{#template ../templates/section-close.md}}

{{#template ../templates/section-open.md id=plan-lock num=4 title=Plan lock}}
Standalone breakouts acquire `.specify/plan.lock` the same way execute does. If you see `plan-lock-busy`, another process holds the lock. When the holder is dead, remove the stale lock file manually.
{{#template ../templates/section-close.md}}

{{#template ../templates/callout-open.md variant=gate}}
**Gate reminder.** Per-entry `done` is only written by `/spec:merge` (or execute after a successful merge). Build success alone does not close the slice.
{{#template ../templates/callout-close.md}}

{{#template ../templates/see-also-open.md}}
- [/spec:execute](../reference/change-skills/execute.md) — stop conditions and re-entry
- [Drop down a layer](drop-down-a-layer.md) — full manual Layer 2 control
- [Slice skills](../reference/slice-skills/index.md) — refine, build, merge reference
{{#template ../templates/see-also-close.md}}
