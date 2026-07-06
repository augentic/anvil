# specify plan execute

Drive an approved plan through refine → build → merge per entry under the guest lock.

`specify plan execute` is a guest-routed CLI verb, not a skill — the `/spec:execute` skill retired when the loop moved into the workflow guest. The loop claims the next eligible entry, runs the refine, build, and merge orchestrations, and repeats until `specify plan status` projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). It refuses unless the plan lifecycle is `approved`, and it holds the create-exclusive `.specify/guest.lock` marker for the run's lifetime — a second driver session exits with `guest-marker-held`.

Stops render the `specify plan status` projection verbatim: the closed reason (`plan-not-approved`, `refine-failed`, `build-failed`, `merge-conflict`, `slice-dropped`, `merge-incomplete`, `stuck`), the failure detail from the journal, a one-line hint, and the literal resume command. Re-running `specify plan execute` after a stop resumes from the same active entry.

## See also

- [specify plan](../cli/plan.md) — the full plan verb family, including `status` and `next`
- [Drive a slice manually](../../how-to/drive-slice-manually.md) — when execute parks
- [Drop down a layer](../../how-to/drop-down-a-layer.md) — manual CLI fallback
- [/spec:finalize](finalize.md) — post-drain closure
- [Slice skills](../slice-skills/index.md) — refine, build, merge breakouts
