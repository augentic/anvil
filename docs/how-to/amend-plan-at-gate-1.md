# Amend a plan at Gate 1

Inspect and edit a plan after `/spec:plan` and before stamping `reviewed`.

**Prerequisites:** A plan at `plan.lifecycle: pending`; completed [Quick start](../tutorials/quick-start.md).

Gate 1 is the operator review step between plan authoring and execution. `/spec:plan` exits at `pending`; you stamp `reviewed` only after the plan looks right.

## Step 1 — Read the plan artifacts

Open these files at `.specify/` (workspace mode: workspace root):

| File | Check |
| ---- | ----- |
| `change.md` | Intent, scope, tentative merge notes |
| `plan.yaml` | Slice names, targets, source bindings, order |
| `discovery.md` | Candidate inventory matches your expectations |

## Step 2 — Amend entries if needed

Use CLI amend verbs — never hand-edit `plan.yaml`:

```bash
# Add a source binding to an existing slice
specrun plan amend <name> <slice> --add-source <key>=<candidate-id>

# Remove a source
specrun plan amend <name> <slice> --remove-source <key>

# Mark likely divergence for Gate 1 acknowledgement
specrun plan amend <name> <slice> --divergence likely

# Override authority for a claim kind (per-slice)
specrun plan amend <name> --authority-override <slice> <kind>=<source-key>
```

See [specrun plan](../reference/cli/plan.md) for the full amend surface.

## Step 3 — Validate (optional)

```bash
specrun plan validate --format json
```

Surface Error-level findings before stamping reviewed.

## Step 4 — Stamp reviewed

```bash
specrun plan transition <name> reviewed
```

Only after this transition will `/spec:execute` start.

## Splitting one slice into two

When Gate 1 review shows a slice should split, amend the plan to add a second slice row (scenario covered in acceptance scenario #7). Each new row gets its own `specrun plan add` invocation.

## See also

- [/spec:plan](../reference/change-skills/plan.md) — plan skill reference
- [Lifecycle](../reference/lifecycle.md) — plan lifecycle states
- [Your first multi-slice change](../tutorials/first-change.md) — multi-slice Gate 1 inspection
