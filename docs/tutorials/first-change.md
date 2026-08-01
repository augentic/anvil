<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Your first multi-slice change</h1>

Plan and execute a change with three slices bound to a documentation source. When you finish, three slices have moved through refine → build → merge and their specs live in your baseline.

<div class="meta-row">

<span class="meta-chip"><strong>Time</strong> ~45 min</span>

<span class="meta-chip"><strong>Target</strong> Omnia</span>

<span class="meta-chip"><strong>Outcome</strong> Three merged slices</span>

</div>

</div>


<section id="outcome" markdown="1">

<h2><span class="num">1</span> What you will build</h2>

An account-management revamp driven by written design notes: three Omnia slices (`account-registration`, `password-reset`, `account-audit-log`) that `emery plan execute` drives in plan order. The rhythm is the same one you learned in the [Quick start](quick-start.md) — only the source binding and the slice count change.
</section>


<div class="prereq">
<strong>Prerequisites.</strong>

- Completed [Quick start](quick-start.md)
- An Omnia-initialised project (`/emery:init omnia` in a fresh or disposable repo is fine)
</div>


<section id="steps" markdown="1">

<h2><span class="num">2</span> Steps</h2>


<div class="tutorial-step" data-step="01">
<div class="step-label">01</div>
<h3 class="step-title">Create the design notes</h3>

The `documentation` source adapter surveys a filesystem tree of written notes. Create a small one so you can check every later step against known input:

```text
design-notes/account/
├── registration.md
├── password-reset.md
└── audit-log.md
```

Each file describes one unit of work in a few sentences. For example, `design-notes/account/registration.md`:

```markdown
# Account registration

New users register with an email address and password.
Registration sends a confirmation email; unconfirmed accounts
expire after 48 hours.
```

Write `password-reset.md` (reset link, expiry window) and `audit-log.md` (append-only log of account events) in the same style. Short is fine — each document should read as one slice-sized piece of work.
</div>


<div class="tutorial-step" data-step="02">
<div class="step-label">02</div>
<h3 class="step-title">Plan with a documentation source</h3>

Bind the notes tree instead of inline intent. The binding grammar is `source <key>=<adapter>:<path>` — you choose the key (`docs` here), and `documentation` is the adapter:

```text
/emery:plan account-revamp source docs=documentation:./design-notes/account
```

The plan surveys the documentation adapter — one [lead](../appendices/glossary.md#l) per slice-sized unit it finds — and reconciles the leads into slices. Expected `plan.yaml` shape:

```yaml
version: 1
name: account-revamp
lifecycle: pending
sources:
  docs:
    adapter: documentation
    path: ./design-notes/account
slices:
  - name: account-registration
    sources:
      - source: docs
        lead: account-registration
    status: pending
  - name: password-reset
    sources:
      - source: docs
        lead: password-reset
    status: pending
  - name: account-audit-log
    sources:
      - source: docs
        lead: account-audit-log
    status: pending
```

Each slice row maps one lead from `discovery.md` to a unit of work. `discovery.md` (at the project root) lists the full lead inventory the survey found:

```markdown
## Summary

Sources: 1. Leads: 3.

## Lead inventory

### docs:account-registration

- lead: account-registration
- source: docs
- synopsis: Email/password registration with confirmation flow
```

Survey and reconciliation are model-driven, so your lead names and synopses will vary with your notes — the *shape* is what to check: one lead per note, one slice per lead, every slice `pending`.
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Review at Gate 1</h3>

`/emery:plan` exits at `plan.lifecycle: pending`. Before executing, read the three plan artifacts at the project root:

| File | Check |
| ---- | ----- |
| `change.md` | Intent and scope match what you asked for |
| `plan.yaml` | Three slices, sensible names, each bound to one `docs` lead |
| `discovery.md` | The lead inventory matches your three notes |

This pause is [Gate 1](../appendices/glossary.md#g) — the operator review step. When the plan looks right, approving it *is* running `emery plan execute` (its first run stamps `approved`). If the grouping is wrong — say survey merged two notes into one lead — fix the notes and re-run `/emery:plan` (it confirms before replacing), or curate entries with the CLI; see [Amend a plan at Gate 1](../how-to/amend-plan-at-gate-1.md).
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Execute and watch per-entry status</h3>

```text
emery plan execute
```

The first run stamps `approved`, then drives each slice through refine → build → merge in plan order. Only one entry is `in-progress` at a time; each slice gets its own directory under `.emery/slices/<name>/` while active, then moves to `.emery/archive/` when merged.

Check progress at any time from a second terminal:

```text
emery plan status
```

Expected shape mid-run — the entry counts walk from `3 pending` toward `3 done`, and `next-action` names the current phase:

```text
plan: account-revamp (approved)
entries: 1 done / 1 in-progress / 1 pending
next-action: build password-reset
resume: /emery:build password-reset
```

When every entry is `done`, status projects the literal `drained` line. If execute stops instead — a build failure, a merge conflict — the stop message names the reason and the resume command; see [Drive a slice manually](../how-to/drive-slice-manually.md).
</div>


<div class="tutorial-step" data-step="05">
<div class="step-label">05</div>
<h3 class="step-title">Finalize when drained</h3>

When all three entries are `done`, publish the repository changes through your normal Git workflow, then close the change:

```text
/emery:finalize account-revamp
```

Finalize confirms publication is complete and archives the [drained](../appendices/glossary.md#d) plan — `plan.yaml`, `change.md`, and `discovery.md` move to `.emery/archive/plans/`.
</div>


</section>


> [!TIP]
> **Done.** Three slices flowed through the same loop as the quick start's one: documentation bound at plan time, three leads reconciled into three slices, one Gate 1 review, one execute run to drain them all.

## What you learned

- Documentation sources bind at plan time with `source <key>=<adapter>:<path>` — here, `docs=documentation:./design-notes/account`.
- One lead per slice-sized unit: survey turns each note into a lead, and reconciliation turns each lead into a plan entry.
- Multi-slice plans share one `change.md` and one Gate 1 review; per-entry status (`pending → in-progress → done`) tracks progress through the execute loop.

<div class="see-also">
<strong>See also</strong>

- [Drive a slice by hand](drive-a-slice-by-hand.md) — the same loop, one phase at a time
- [Bind multiple sources](../how-to/bind-multiple-sources.md) — reconcile legacy code and docs at plan time
- [Cross-repo changes](cross-repo-change.md) — workspace mode
- [Lifecycle](../reference/lifecycle.md) — per-entry and slice state machines
</div>
