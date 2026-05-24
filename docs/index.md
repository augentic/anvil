{{#template templates/hero-open.md eyebrow=Specify Developer Guide title=From prompts to durable specs}}
Specify 2.0 turns ad-hoc AI prompting into a repeatable, auditable workflow. Every change you make through Specify produces durable artifacts -- a plan, a per-slice proposal, behavioral specs, a technical design, and a task list -- that accumulate as your project's living specification.

{{#template templates/meta-row-open.md}}
{{#template templates/meta-chip.md label=Version value=2.0}}
{{#template templates/meta-chip.md label=Status value=Released}}
{{#template templates/meta-chip.md label=Workflow value=plan → reviewed → execute → finalize}}
{{#template templates/meta-row-close.md}}
{{#template templates/hero-close.md}}

## What you get

- **Durable reasoning.** The thinking that connects intent to implementation is captured in version-controlled artifacts, not lost when the chat session ends.
- **Baseline accumulation.** Each merged slice adds to a growing specification. Future slices build on what came before rather than starting from scratch.
- **One workflow rhythm.** `/spec:plan` → operator review step (Gate 1) → `/spec:execute` → `/spec:finalize`. A one-slice change uses the same steps as a twelve-slice migration.

## Choose your path

| I want to… | Start here |
| ---------- | ---------- |
| Run Specify for the first time | [Quick start](tutorials/quick-start.md) |
| Look up a command | [Quick reference](reference/quick-reference.md) |
| Understand the architecture | [Layered stack](explanation/layered-stack.md) |
| Recover when execute stops | [Drive a slice manually](how-to/drive-slice-manually.md) |

## See it in action

```text
/spec:init omnia

/spec:plan fix-typo source intent="fix typo in user.rs"
  --> writes change.md + plan.yaml + discovery.md, exits at pending

specify plan transition fix-typo reviewed
  --> operator review step (Gate 1)

/spec:execute
  --> /spec:refine + /spec:build + /spec:merge per slice until drained

/spec:finalize fix-typo
  --> push branches, observe PRs, archive plan
```

## Two adapter roles

Specify 2.0 splits adapters by direction. **Source adapters** read external material and emit `Evidence`. **Target adapters** consume `spec.md` + `design.md` and produce code. See [Anatomy of an adapter](explanation/adapter-anatomy.md).

## Start here

{{#template templates/audience-grid-open.md}}

{{#template templates/audience-open.md who=New to Specify}}
Read [What is Specify?](orientation/index.md), install [Prerequisites](orientation/prerequisites.md), then follow the [Quick start](tutorials/quick-start.md) tutorial.
{{#template templates/audience-close.md}}

{{#template templates/audience-open.md who=Returning from 1.x}}
[Release notes](explanation/release-notes.md) catalogues the 2.0 cut.
{{#template templates/audience-close.md}}

{{#template templates/audience-open.md who=Authoring skills or adapters}}
Jump to [Contributing](contributing/index.md) and [Skill authoring standards](standards/skill-authoring.md).
{{#template templates/audience-close.md}}

{{#template templates/audience-grid-close.md}}

## Guide structure

- **[Getting Started](orientation/index.md)** — what Specify is and how to install it.
- **[Tutorials](tutorials/index.md)** — hands-on walkthroughs from first slice to cross-repo changes.
- **[How-to Guides](how-to/index.md)** — task-oriented recipes for common operator situations.
- **[Understanding Specify](explanation/concepts.md)** — concepts, architecture, and design decisions.
- **[Reference](reference/index.md)** — skills, CLI commands, artifact formats, and configuration.
- **[Contributing](contributing/index.md)** — skill authoring, plugin development, and checks.
