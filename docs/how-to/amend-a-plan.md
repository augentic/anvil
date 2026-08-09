# Amend a plan before executing

Inspect and edit a plan after `/emery:plan` and before executing it.

**Prerequisites:** An authored plan whose entries are all `pending`; completed [Quick start](../tutorials/quick-start.md).

The pause between plan authoring and execution is the operator review step. `/emery:plan` exits after authoring; you run `emery plan execute` — opening the authorization epoch — only after the plan looks right.

## Which verb when

| Goal | Prefer |
| --- | --- |
| Rethink cross-source grouping | Re-run `emery plan author --force` (or `/emery:plan`, which confirms) — replaces a pending plan wholesale |
| Defer a [lead](../appendices/glossary.md#l) out of this change | `emery plan remove <entry>` |
| Split or merge entries | `emery plan add` + `emery plan amend` + `emery plan remove` — see the [plan command reference](../reference/cli/plan.md) |
| Divergence stamp, authority override, single-source fix | `emery plan amend <entry>` (the scalpel) |

There is one active `plan.yaml` per project. `emery plan amend` takes **one positional — the entry (slice) name** — not a plan name plus entry name.

## Step 1 — Read the plan artifacts

Open these files at the project root:

| File | Check |
| ---- | ----- |
| `change.md` | Intent, scope, tentative merge notes |
| `plan.yaml` | Slice names, targets, source bindings, order |
| `discovery.md` | Lead inventory matches your expectations |

## Step 2 — Amend entries if needed

Use CLI verbs — never hand-edit `plan.yaml`:

```bash
# Add a source binding to an existing entry (one lead per source key —
# a key the entry already binds is refused as duplicate-source-key)
emery plan amend <entry> --add-source <key>=<lead>

# Remove a source binding
emery plan amend <entry> --remove-source <key>

# Mark likely divergence for operator acknowledgement
emery plan amend <entry> --divergence likely

# Accept or reject a predicted divergence during review
emery plan amend <entry> --divergence accepted

# Override authority for a claim kind on this entry
emery plan amend <entry> --authority-override <kind>=<source>

# Defer an entry's lead(s) without re-surveying discovery.md
emery plan remove <entry>
```

See [emery plan](../reference/cli/plan.md) for the full amend surface.

## Step 3 — Validate (optional)

```bash
emery plan validate --format json
```

Surface Error-level findings before executing.

## Step 4 — Execute

```bash
emery plan execute
```

Invoking execute on the reviewed plan journals `plan.execute.started` and drives the slices under gap gates.

## Splitting one slice into two

When review shows a slice should split, add a second row with `emery plan add`, narrow the original with `emery plan amend <original> --sources …`, and `emery plan remove <original>` when the original entry is empty.

## See also

- [`/emery:plan` skill body](../../plugins/emery/skills/plan/SKILL.md) — plan skill reference
- [Lifecycle](../reference/lifecycle.md) — per-entry and slice lifecycle states
- [Your first multi-slice change](../tutorials/first-change.md) — multi-slice plan inspection
