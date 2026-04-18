# resolves-to-done — self-heal reconciles a completed merge

The prior `/spec:execute` run finished `/spec:merge email-verification`
(so `.metadata.yaml.outcome == { phase: merge, outcome: success }` and
`LifecycleStatus == merged`) but crashed before reaching step 10 of
the supervised run (`specify plan transition email-verification done`).

On the next startup, self-heal:

1. Scans `plan.yaml`, finds `email-verification` with `status:
   in-progress`.
2. Reads `.specify/changes/email-verification/.metadata.yaml`
   (`metadata.yaml` in this fixture). Classifies `outcome.outcome ==
   success` with `outcome.phase == merge` → terminal success path.
3. Runs `specify plan transition email-verification done`. No
   `/spec:drop` — nothing to clean up, the merge already archived the
   change directory.
4. Appends one `type: recovery` entry to `journal.yaml` (see
   `journal.yaml.after`) naming the reconciliation. No further
   phases invoked.
5. Emits exactly one diagnostic line and falls through to step 4 of
   the supervised run (`specify plan next`).

```text
Self-heal: email-verification → done (merge success from prior run)
```

`specify plan next` (step 4 of the outer supervised run) then reports
`notification-preferences` as the next eligible entry, and the driver
continues from there as if the crash had never happened.
