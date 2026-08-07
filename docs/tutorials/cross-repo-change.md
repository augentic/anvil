<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Cross-repo changes</h1>

Coordinate one Emery change across two repositories from a registry-only workspace. When you finish, plan artifacts live at the workspace while each slice's work lands in its own project checkout.

<div class="meta-row">

<span class="meta-chip"><strong>Time</strong> ~60 min</span>

<span class="meta-chip"><strong>Mode</strong> Workspace</span>

<span class="meta-chip"><strong>Outcome</strong> Two project-routed slices</span>

</div>

</div>


<section id="outcome" markdown="1">

<h2><span class="num">1</span> What you will build</h2>

A workspace-scoped plan where `change.md`, `plan.yaml`, and `discovery.md` live at the workspace root, while slice work runs in materialised [workspace slots](../appendices/glossary.md#w) under top-level `workspace/<project>/`. This tutorial follows one golden path — a fresh registry-only workspace routing slices to two local peer projects. Registering a workspace inside an existing platform project is a variant covered in the [Registry](../reference/registry.md) and [Configuration files](../reference/configuration.md) references.
</section>


<div class="prereq">
<strong>Prerequisites.</strong>

- Completed [Quick start](quick-start.md) and [Your first multi-slice change](first-change.md)
- Two peer Emery projects checked out as siblings of your workspace directory (each initialised with `/emery:init omnia`) — the tutorial calls them `identity-svc` and `billing-svc`
</div>


<section id="steps" markdown="1">

<h2><span class="num">2</span> Steps</h2>


<div class="tutorial-step" data-step="01">
<div class="step-label">01</div>
<h3 class="step-title">Initialise a registry-only workspace</h3>

Create a fresh directory for the workspace, open it in Cursor, and scaffold it:

```text
/emery:init workspace
```

This writes the workspace shape of `project.yaml` (`workspace: true`, no `target:`) — the workspace holds plan artifacts and the registry, but is never itself a code project.
</div>


<div class="tutorial-step" data-step="02">
<div class="step-label">02</div>
<h3 class="step-title">Declare the registry and materialise slots</h3>

Author `registry.yaml` at the workspace root (this file is operator-written — one of the few Emery reads but does not manage):

```yaml
version: 1
projects:
  - name: identity-svc
    url: ../identity-svc
  - name: billing-svc
    url: ../billing-svc
```

Then materialise each `workspace/<project>/` slot. Emery does not clone, refresh, or prepare slots — for local peers a symlink is enough; for a remote `url`, clone the repository into the slot path instead:

```bash
mkdir -p workspace
ln -s ../../identity-svc workspace/identity-svc
ln -s ../../billing-svc workspace/billing-svc
```

Checkpoint: `ls workspace/` shows both slots, and each resolves to a project with its own `.emery/` directory.
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Plan from the workspace</h3>

Run `/emery:plan` at the workspace root (not inside a slot), binding sources exactly as in a single-repo change:

```text
/emery:plan platform-auth source docs=documentation:./design-notes/auth
```

Because the registry declares multiple projects, reconciliation must bind every slice to a project. Expected `plan.yaml` shape — note the `project:` field on each slice row:

```yaml
version: 1
name: platform-auth
sources:
  docs:
    adapter: documentation
    path: ./design-notes/auth
slices:
  - name: auth-token-issue
    project: identity-svc
    sources:
      - source: docs
        lead: auth-token-issue
  - name: billing-auth-check
    project: billing-svc
    depends-on: [auth-token-issue]
    sources:
      - source: docs
        lead: billing-auth-check
```

During plan review, check the `project:` routing along with the usual scope review. If a slice is routed to the wrong project, fix it with `emery plan amend <entry> --project <name>`; a missing `project` on a multi-project registry is caught by `emery plan validate` (`project-missing-multi-repo`).
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Drive slices hand-driven</h3>

Workspace plans refuse the automated loop. If you run `emery plan execute` here, it exits before touching any state:

```text
error: plan-execute-workspace-unsupported   (exit 2)
```

That refusal is expected — drive the plan one entry at a time instead. Advance the next eligible entry:

```bash
emery plan advance
```

then run the [breakouts](../appendices/glossary.md#b) for that slice — `/emery:refine`, `/emery:build`, `/emery:merge` — and repeat `emery plan advance` until `emery plan status` projects `drained`. When a slice is project-bound, refine, build, and merge run inside that slot's checkout: `auth-token-issue`'s artifacts appear under `workspace/identity-svc/.emery/slices/`, and its code lands in the `identity-svc` tree. Commits and branch management remain operator-owned.
</div>


<div class="tutorial-step" data-step="05">
<div class="step-label">05</div>
<h3 class="step-title">Publish and finalize</h3>

After the plan drains, commit and publish every affected repository through its normal Git and forge workflow — open and merge pull requests, or satisfy the equivalent publication gate. Then close the change from the workspace:

```text
/emery:finalize platform-auth
```

Finalize confirms publication is complete, then runs `emery plan archive`. It performs no Git or forge operations.
</div>


</section>


> [!TIP]
> **Done.** The workspace centralised the plan; the registry routed each slice to its project slot; the hand-driven loop replaced `emery plan execute`; publication stayed in your hands.

## What you learned

- A registry-only workspace holds `registry.yaml` and the plan artifacts; it has no target of its own.
- Per-slice `project:` routes phase work into a materialised workspace slot — operators materialise slots and publish repository changes outside Emery.
- Workspace plans are hand-driven: `emery plan advance` plus the per-slice breakouts, because `emery plan execute` refuses workspace routing.
- Finalize archives only after publication is complete.

<div class="see-also">
<strong>See also</strong>

- [Registry](../reference/registry.md) — `registry.yaml` format and slot semantics
- [Workspace topology](../reference/cli/workspace.md) — slots, topology lock, and operator-owned publication
- [Drive a slice by hand](drive-a-slice-by-hand.md) — the hand-driven loop on a single project
- [Drop down a layer](../how-to/drop-down-a-layer.md) — manual CLI when automation fails
</div>
