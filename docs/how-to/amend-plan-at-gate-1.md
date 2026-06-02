# Amend a plan at Gate 1

Inspect and edit a plan after `/spec:plan` and before stamping `approved`.

**Prerequisites:** A plan at `plan.lifecycle: pending`; completed [Quick start](../tutorials/quick-start.md).

Gate 1 is the operator review step between plan authoring and execution. `/spec:plan` exits at `pending`; you stamp `approved` only after the plan looks right.

## Which verb when

| Goal | Prefer |
| --- | --- |
| Rethink cross-source grouping | Re-run `specrun plan propose --from` (replaces all slices) |
| Defer a lead out of this change | `specrun plan remove <entry>` |
| Split or merge entries | `specrun plan add` + `specrun plan amend` + `specrun plan remove` — see the [plan command reference](../reference/cli/plan.md) |
| Divergence stamp, authority override, single-source fix | `specrun plan amend <entry>` (the scalpel) |

There is one active `plan.yaml` per project. `specrun plan amend` takes **one positional — the entry (slice) name** — not a plan name plus entry name.

## Step 1 — Read the plan artifacts

Open these files at `.specify/` (workspace mode: workspace):

| File | Check |
| ---- | ----- |
| `change.md` | Intent, scope, tentative merge notes |
| `plan.yaml` | Slice names, targets, source bindings, order |
| `discovery.md` | Lead inventory matches your expectations |

## Step 2 — Amend entries if needed

Use CLI verbs — never hand-edit `plan.yaml`:

```bash
# Add a source binding to an existing entry
specrun plan amend <entry> --add-source <key>=<lead>

# Remove a source binding
specrun plan amend <entry> --remove-source <key>

# Mark likely divergence for Gate 1 acknowledgement
specrun plan amend <entry> --divergence likely

# Accept or reject a predicted divergence at Gate 1
specrun plan amend <entry> --divergence accepted

# Override authority for a claim kind on this entry
specrun plan amend <entry> --authority-override <entry> <kind>=<source>

# Defer an entry's lead(s) without re-surveying discovery.md
specrun plan remove <entry>
```

See [specrun plan](../reference/cli/plan.md) for the full amend surface.

## Step 3 — Validate (optional)

```bash
specrun plan validate --format json
```

Surface Error-level findings before stamping approved.

## Step 4 — Stamp approved

```bash
specrun plan transition <plan-name> approved
```

Only after this transition will `/spec:execute` start.

## Splitting one slice into two

When Gate 1 review shows a slice should split, add a second row with `specrun plan add`, narrow the original with `specrun plan amend <original> --sources …`, and `specrun plan remove <original>` when the original entry is empty. Scenario coverage lives in acceptance scenario #7.

## See also

- [/spec:plan](../reference/change-skills/plan.md) — plan skill reference
- [Lifecycle](../reference/lifecycle.md) — plan lifecycle states
- [Your first multi-slice change](../tutorials/first-change.md) — multi-slice Gate 1 inspection
