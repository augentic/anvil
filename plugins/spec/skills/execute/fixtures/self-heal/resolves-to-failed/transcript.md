# resolves-to-failed — self-heal reconciles a build failure

The prior `/spec:execute` run saw `/spec:build checkout-api` exhaust its verify-repair budget and stamp `outcome: build failure` with a type-mismatch summary, but crashed before reaching step 11 of the supervised run (`/spec:drop checkout-api …` then `specify plan transition checkout-api failed …`).

On the next startup, self-heal:

1. Scans `plan.yaml`, finds `checkout-api` with `status: in-progress`.
2. Reads `.specify/changes/checkout-api/.metadata.yaml`. Classifies `outcome.outcome == failure` → drop-and-fail path.
3. Runs `/spec:drop checkout-api --reason "<outcome.summary>"` to archive the partial artifacts (idempotent against an already- dropped change; in this fixture the change dir is still active so the drop actually runs).
4. Runs `specify plan transition checkout-api failed --reason "<outcome.summary>"`. The `--reason` string is byte-identical to `outcome.summary` in `metadata.yaml`; `plan.yaml.after`'s `status-reason` pins that equality.
5. Appends one `type: recovery` entry to `journal.yaml` (see `journal.yaml.after`); the three pre-existing `type: failure` entries the phase wrote during its verify-repair loop are preserved unchanged.
6. Emits exactly one diagnostic line and falls through to step 4 of the supervised run.

```text
Self-heal: checkout-api → failed (build failure: "cargo test failed: 1/7 tests red (checkout_gateway::line_item_round_trip).")
```

The `outcome.context` block stays on disk in the archived `.metadata.yaml` for human triage; it is deliberately NOT copied into the plan entry. Only `summary` travels to `status-reason`.
