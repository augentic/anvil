# Your first multi-slice change

Plan and execute a change with three slices bound to a documentation source. This tutorial assumes you completed the [Quick start](quick-start.md).

## What you will build

An account-management revamp driven by written design notes: three Omnia slices (`account-registration`, `password-reset`, `account-audit-log`) that `/spec:execute` drives in plan order.

## Prerequisites

- Completed [Quick start](quick-start.md)
- A `documentation` source path with design notes (or use your own `./design-notes/account` tree)

## Step 1 — Plan with a documentation source

Bind a filesystem path instead of inline intent:

```text
/spec:plan account-revamp source docs=./design-notes/account
```

The plan enumerates the documentation adapter and proposes multiple slices. Expected `plan.yaml` shape:

```yaml
version: 1
name: account-revamp
sources:
  docs:
    adapter: documentation
    path: ./design-notes/account
slices:
  - name: account-registration
    target: omnia
    sources:
      - key: docs
        candidate: account-registration
    status: pending
  - name: password-reset
    target: omnia
    sources:
      - key: docs
        candidate: password-reset
    status: pending
  - name: account-audit-log
    target: omnia
    sources:
      - key: docs
        candidate: account-audit-log
    status: pending
```

Each slice row maps one candidate from `discovery.md` to a unit of work.

## Step 2 — Inspect at Gate 1

Before stamping `reviewed`, read:

- **`change.md`** — scope and any tentative merge notes
- **`plan.yaml`** — slice names, source bindings, dependency order
- **`discovery.md`** — full candidate inventory

Amend if needed:

```bash
specrun plan amend account-revamp --add-source ...
specrun plan transition account-revamp reviewed
```

See [Amend a plan at Gate 1](../how-to/amend-plan-at-gate-1.md).

## Step 3 — Execute and watch per-entry status

```text
/spec:execute
```

Watch `plan.yaml.slices[].status` move from `pending` to `in-progress` to `done`.

Only one entry is `in-progress` at a time. `specrun plan next` picks the next eligible slice. Each slice gets its own directory under `.specify/slices/<name>/`.

If you need to run one phase by hand (a **breakout**), cancel execute and invoke `/spec:refine`, `/spec:build`, or `/spec:merge` directly. See [Drive a slice manually](../how-to/drive-slice-manually.md).

## Step 4 — Finalize when drained

When all three entries are `done`:

```text
/spec:finalize account-revamp
```

## What you learned

- Documentation sources bind at plan time with `source docs=<adapter>:<path>`.
- Multi-slice plans share one `change.md` and one operator review step.
- Per-entry status tracks progress through the execute loop.

## Next steps

- [Bind multiple sources](../how-to/bind-multiple-sources.md) — fuse legacy code and docs at plan time
- [Cross-repo changes](cross-repo-change.md) — workspace mode
- [Lifecycle](../reference/lifecycle.md) — per-entry and slice state machines
