<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Quick start</h1>

Run a complete Specify change in one sitting: one slice, intent-only source, Omnia target. When you finish, merged specs live in your baseline and the plan is archived.

<div class="meta-row">

<span class="meta-chip"><strong>Time</strong> ~30 min</span>

<span class="meta-chip"><strong>Target</strong> Omnia</span>

<span class="meta-chip"><strong>Outcome</strong> First merged slice</span>

</div>

</div>


<section id="outcome" markdown="1">

<h2><span class="num">1</span> What you will build</h2>

A minimal Omnia project where Specify plans, specifies, implements, and merges a single slice driven entirely by operator intent — fixing a typo in `user.rs`. A one-slice change uses the same steps as a twelve-slice migration; only the plan row count differs.
</section>


<div class="prereq">
<strong>Prerequisites.</strong>

Complete [Prerequisites](../orientation/prerequisites.md):

- Cursor with Augentic plugins installed
- `specify` CLI (`specify --version` succeeds)
- Rust toolchain with `wasm32-wasip2` target for Omnia

Open your project in Cursor Agent chat. This tutorial assumes a fresh or disposable repo.
</div>


<section id="steps" markdown="1">

<h2><span class="num">2</span> Steps</h2>


<div class="tutorial-step" data-step="01">
<div class="step-label">01</div>
<h3 class="step-title">Initialise the project</h3>

Run once per project:

```text
/spec:init omnia
```

The skill runs `specify init omnia`, which scaffolds:

```text
.specify/
├── project.yaml
├── slices/
├── specs/          ← baseline accumulates here after merge
└── archive/
AGENTS.md           ← generated when absent
```

See [Directory layout](../reference/directory-layout.md) for the full tree.
</div>


<div class="tutorial-step" data-step="02">
<div class="step-label">02</div>
<h3 class="step-title">Plan the change</h3>

Describe what you want in one line:

```text
/spec:plan fix-typo source intent="fix typo in user.rs"
```

`/spec:plan` writes three plan-time artifacts:

**`change.md`** — operator narrative:

```markdown
# Change — fix-typo

## Intent

fix typo in user.rs

## Scope

- One single-source slice driven by the operator's intent.
```

**`plan.yaml`** — slice table of contents:

```yaml
version: 1
name: fix-typo
sources:
  intent:
    adapter: intent
    value: "fix typo in user.rs"
slices:
  - name: fix-typo
    sources: [intent]
    status: pending
```

**`discovery.md`** — what sources surveyd:

```markdown
## Summary

Sources: 1. Leads: 1.

## Lead inventory

### intent:fix-typo

- lead: fix-typo
- source: intent
- synopsis: fix typo in user.rs
```

The skill exits at `plan.lifecycle: pending` and prints:

```text
Plan `fix-typo` is at `pending`. Run `specify plan transition fix-typo approved` to stamp Gate 1, then `specify plan execute` to drive the slices.
```

#### Operator review step (Gate 1)

Before any slice work runs, inspect `change.md` and `plan.yaml`. This pause is the **operator review step** — Specify calls it **Gate 1**. `/spec:plan` never stamps `approved` itself; you do:

```bash
specify plan transition fix-typo approved
```

Learn more: [Amend a plan at Gate 1](../how-to/amend-plan-at-gate-1.md).
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Execute the slice</h3>

Drive the per-slice loop:

```text
specify plan execute
```

Inside execute, each slice runs **refine → build → merge**:

<div class="pipeline">


![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

<p class="pipeline-caption">refine synthesizes artifacts; build implements tasks; merge folds specs into the baseline.</p>
</div>


##### After refine

Under `.specify/slices/fix-typo/` you will find:

| File | Purpose |
| ---- | ------- |
| `proposal.md` | Why the slice exists |
| `specs/<domain>/spec.md` | Behavioral requirements (`ID:`, `Sources:`, `Status:`) |
| `design.md` | Technical approach |
| `tasks.md` | Implementation sequence |
| `evidence/intent.yaml` | What the intent source contributed |

Requirement blocks carry provenance, for example:

```markdown
### Requirement: User display typo corrected

ID: REQ-001
Sources: [intent]
Status: agreed
```

Exact wording varies with your intent; see refine fixtures for shape reference.

##### After build

Source code changes land in your project tree (not under `.specify/`). Task checkboxes in `tasks.md` flip to complete via the CLI.

##### After merge

- Spec deltas apply to `.specify/specs/`
- The slice directory moves to `.specify/archive/`
- `plan.yaml` marks the entry `done`

If execute parks on a failure, see [Drive a slice manually](../how-to/drive-slice-manually.md).
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Finalize the change</h3>

When every plan entry is `done`, close the change:

```text
/spec:finalize fix-typo
```

Before finalizing, publish the completed repository changes through your normal Git and review workflow. Finalize confirms publication is complete and archives the drained plan; it performs no Git or forge operations.
</div>


</section>


> [!TIP]
> **Done.** You completed the full rhythm: `/spec:init` scaffolds once; `/spec:plan` exits at `pending`; Gate 1 is the operator review seam; `specify plan execute` loops refine → build → merge; `/spec:finalize` closes the change.

<div class="see-also">
<strong>See also</strong>

- [Core concepts](../explanation/concepts.md) — vocabulary tour
- [Your first multi-slice change](first-change.md) — three slices from documentation
- [Quick reference card](../reference/quick-reference.md) — command cheat sheet
- [Change skills](../reference/change-skills/index.md) — `/spec:plan`, `specify plan execute`, `/spec:finalize` reference
</div>

