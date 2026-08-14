<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Your first multi-slice change</h1>

Plan, refine, and execute a change with three slices bound to a documentation source. When you finish, three slices have moved through refine → build → merge and their specs live in your baseline.

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

Sources arrive through the reviewed handoff (`--from` / `--wave`). Intent is the reserved key with an inline `value`; a documentation tree is a locator plus CID. There is no `--intent` or `--source` authoring flag:

```text
/emery:plan account-revamp --from .emery/system/ --wave deliver
```

The plan surveys each bound source — one [lead](../appendices/glossary.md#l) per slice-sized unit — and decomposes the catalog into slices. Expected `plan.yaml` shape:

```yaml
name: account-revamp
targets:
  default:
    adapter: emery:omnia@0.12.0
    locator: "."
    cid: sha256:…
sources:
  docs:
    adapter: emery:documentation@0.12.0
    locator: ./design-notes/account
    cid: sha256:…
slices:
  - name: account-registration
    target: default
    sources:
      - source: docs
        lead: account-registration
  - name: password-reset
    target: default
    sources:
      - source: docs
        lead: password-reset
  - name: account-audit-log
    target: default
    sources:
      - source: docs
        lead: account-audit-log
```

Each slice row maps one lead from `leads.md` to a unit of work. `leads.md` (under `.emery/change/`) is catalog-only:

```markdown
## Lead inventory

### docs:account-registration

- lead: account-registration
- source: docs
- synopsis: Email/password registration with confirmation flow
```

Survey and decomposition are model-driven, so your lead names and synopses will vary with your notes — the *shape* is what to check: one lead per note, one slice per lead (entries project `pending` until claimed).
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Review the plan</h3>

`/emery:plan` exits after authoring. Before executing, read the plan artifacts under `.emery/change/`:

| File | Check |
| ---- | ----- |
| `change.md` | Intent and scope match what you asked for |
| `plan.yaml` | Three slices, required `target`, each bound to one `docs` lead |
| `discovery.yaml` | Reviewed handoff and pinned CIDs |
| `leads.md` | The lead inventory matches your three notes |
| `decomposition.yaml` | Conflict-domain hierarchy the projector reads |

This pause is the topology review step. If the grouping is wrong — say survey merged two notes into one lead — fix the notes and re-run `/emery:plan` (it confirms before replacing), or curate entries with the CLI; see [Amend a plan before executing](../how-to/amend-a-plan.md).
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Refine the slices</h3>

```text
emery plan refine
```

The drain refines all three slices serially in plan order: each extraction reads the bound `docs` source, synthesis writes the slice artifacts under `.emery/change/slices/<name>/`, and the refinement manifest (`refinement.yaml`) records the exact inputs and output bundle. When the drain completes, read the three `specs/<domain>/spec.md` files — this pause is the specification review step. If a spec is wrong, fix the note (or amend the plan) and re-run `emery plan refine`; only the staled slices re-refine.
</div>


<div class="tutorial-step" data-step="05">
<div class="step-label">05</div>
<h3 class="step-title">Execute and watch per-entry status</h3>

```text
emery plan execute
```

Execute opens the authorization epoch over the plan and refinement digests, then drives each slice through build → merge in plan order (a single execute process walks entries one-by-one; other journal writers may still claim different slices). Execute never refines — a missing or stale manifest stops it with `plan-refinement-required`. Each slice keeps its directory under `.emery/change/slices/<name>/` while active, then moves to `.emery/change/archive/` when merged.

Check progress at any time from a second terminal:

```text
emery plan status
```

Expected shape mid-run — the projected entry counts walk from `3 pending` toward `3 done`, and `next-action` names the current phase:

```text
plan: account-revamp
entries: 1 done / 1 in-progress / 1 pending
ready: false  authorized: true
next-action: build password-reset
resume: emery plan execute
```

When every entry is `done`, status projects the literal `drained` line. If execute stops instead — a build failure, a merge conflict — the stop message names the reason and the resume command; fix the cause and re-run `emery plan execute`. See [Drop down a layer](../how-to/drop-down-a-layer.md).
</div>


<div class="tutorial-step" data-step="06">
<div class="step-label">06</div>
<h3 class="step-title">Finalize when drained</h3>

When all three entries are `done`, publish the repository changes through your normal Git workflow, then close the change:

```text
/emery:finalize account-revamp
```

Finalize confirms publication is complete and archives the [drained](../appendices/glossary.md#d) plan — `plan.yaml`, `change.md`, `discovery.yaml`, `leads.md`, and `decomposition.yaml` move to `.emery/change/archive/plans/`.
</div>


</section>


> [!TIP]
> **Done.** Three slices flowed through the same rhythm as the quick start's one: documentation bound at plan time, three leads reconciled into three slices, one plan review, one refine drain plus spec review, one execute run to drain them all.

## What you learned

- Documentation sources bind through the reviewed handoff (`--from` / `--wave`); a locator plus CID identifies the notes tree — here, `docs` → `emery:documentation@…` + `./design-notes/account`.
- One lead per slice-sized unit: survey turns each note into a lead, and decomposition turns each lead into a plan entry with a required `target`.
- Multi-slice plans share one `change.md`, one plan review, and one refinement drain; per-entry status (`pending → in-progress → done`) tracks progress through the execute loop.

<div class="see-also">
<strong>See also</strong>

- [Bind multiple sources](../how-to/bind-multiple-sources.md) — reconcile legacy code and docs at plan time
- [Lifecycle](../reference/lifecycle.md) — per-entry and slice state machines
</div>
