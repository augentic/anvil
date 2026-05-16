# Reviewing the plan

`/change:draft` ends at "plan validated, hand back to operator." The skill prints a hand-off summary and stops. Nothing happens automatically after that — the operator decides when (and whether) to continue with `/change:execute loop`.

This page covers the seam: what to look at, when to edit `plan.yaml` before execute, and how to abort if the draft was wrong. The three-skill lifecycle (`/change:draft → /change:execute → /change:finalize`) is built around this pause; treating it as a real review step is the recommended posture.

**Prerequisites:**

- `/change:draft <name>` has completed against your current change. `plan.yaml` exists at the repo root with one or more entries.
- The change brief at `change.md` is up to date.

## Contents

- [Where you are in the loop](#where-you-are-in-the-loop)
- [What to look at](#what-to-look-at)
- [When to edit with `specify plan amend`](#when-to-edit-with-specify-plan-amend)
- [When to abort](#when-to-abort)
- [When you are ready](#when-you-are-ready)

## Where you are in the loop

```text
/change:draft <name>      ← authoring stops here, plan validated
        │
        ▼
  operator review         ← you are here
        │
        ▼
/change:execute loop      ← per-slice define / build / merge
        │
        ▼
/change:finalize <name>   ← push, observe PRs, archive
```

Authoring (`/change:draft`) and execution (`/change:execute`) are deliberately separated. There is no automatic transition between them and the framework does not ship a one-command wrapper that would skip this step.

## What to look at

Run `specify plan status` to see the slice list with statuses, dependencies, and routed projects:

```bash
specify plan status
```

<details>
<summary>Expected output (oauth-login example)</summary>

```text
oauth-login
  pending  oauth-login-contract                                   (depends-on: [])
  pending  add-oauth-tokens     project: shop-backend             (depends-on: [oauth-login-contract])
  pending  add-oauth-screens    project: shop-mobile              (depends-on: [oauth-login-contract])

Summary: 3 pending, 0 in-progress, 0 done
```

</details>

Cross-check that against the brief and the authoring trail under `.specify/plans/<change>/`:

```bash
cat plan.yaml
cat change.md
cat .specify/plans/<change>/discovery.md
cat .specify/plans/<change>/proposal.md
```

If `/change:draft` ran the survey + synthesise sub-steps (multi-source decomposition), `.specify/plans/<change>/survey.md` and the `## Reconciliation` section in `discovery.md` are the ground truth for how the slices fall out of the inputs. Read them together with `plan.yaml` to confirm every capability you expected made it into the plan, and that nothing extra was invented.

The questions to answer at this stage:

- **Slice count and shape.** Is every slice you expected present? Are there slices you did not expect? Are any obviously redundant or trivially mergeable?
- **Dependencies.** Are the `depends-on` edges consistent with the contract / implementation ordering you want? Cross-project contract slices should precede the implementations that consume them.
- **Project routing.** For multi-repo plans, does every implementation slice have the right `project:`? The contract slice (if any) should carry no `project:` and run against the hub.
- **Descriptions and sources.** Will `/spec:define` have enough scoping context to do its job, or is a description too thin?

`specify plan validate` already ran inside `/change:draft`; you do not need to re-run it unless you edit `plan.yaml`.

## When to edit with `specify plan amend`

`specify plan amend <name>` edits a single entry's non-status fields. Reach for it when:

- **A description reads thin or contains a typo.** `specify plan amend <name> --description "…"`.
- **A project assignment is wrong.** `specify plan amend <name> --project <other-project>`. Clear with `--project ""`.
- **A dependency edge is missing or stale.** `specify plan amend <name> --depends-on <pred>` (repeat or comma-separate to set multiple; pass with no value to clear).
- **A source key needs adding or removing.** `specify plan amend <name> --sources <key>`.
- **The context paths the slice should see at define time are off.** `specify plan amend <name> --context <path>`.

Run `specify plan validate` after any amend to confirm the plan still passes the four health diagnostics (`cycle-in-depends-on`, `orphan-source-key`, `stale-workspace-clone`, `unreachable-entry`).

For larger reshapes — adding a slice that propose missed, splitting one slice into two — re-run `/change:draft <name> extend` to append-only re-enter the brief pipeline rather than hand-editing `plan.yaml`.

## When to abort

If the draft was directionally wrong — the proposed decomposition does not reflect the change you actually want, or the inputs were the wrong ones — abort and start over by deleting the change brief and authoring trail:

```bash
rm change.md plan.yaml
rm -rf .specify/plans/<change>/
```

After that, re-run `specify change draft <change> [--source …]` (or `/change:draft <change> …`) with the corrected inputs.

This is destructive: it deletes the brief, the validated plan, and every artefact under `.specify/plans/<change>/` (discovery, workspace, proposal, optional survey + reconciliation). The change has not yet touched any workspace clone or remote, so there is nothing else to undo — but if you are at all unsure, commit `change.md` and `plan.yaml` to a scratch branch first.

If a single slice is wrong and the rest of the plan is good, prefer `specify plan amend` over an abort. Aborting throws away the whole draft.

## When you are ready

Once `plan.yaml` reads the way you want and `specify plan validate` is clean, hand off to execute:

```text
/change:execute loop
```

Re-entering the seam later (after a halt, an amend, or a project re-assignment) is the same loop: edit, validate, then `/change:execute loop` again. The driver re-reads `plan.yaml` on every invocation.

## Next

- [Working across repos: executing](cross-repo-execute.md) — the next stage in the cross-repo flow.
- [A multi-slice change](single-repo-change.md) — the same seam in a single-repo plan.
