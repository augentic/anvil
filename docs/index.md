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
- **One workflow rhythm.** `/spec:plan` → Gate 1 (operator stamps `reviewed`) → `/spec:execute` → `/spec:finalize`. The same rhythm drives a one-slice fix and a twelve-slice migration.

## See it in action

```text
/spec:init https://github.com/augentic/specify/targets/omnia

/spec:plan add-greeting source intent="Add a greeting endpoint that accepts a name and returns a message"
  --> enumerates intent, proposes one slice, writes change.md + plan.yaml + discovery.md, exits at `pending`

specify plan transition add-greeting reviewed
  --> Gate 1: operator-stamped lifecycle transition

/spec:execute
  --> /spec:refine + /spec:build + /spec:merge per slice until drained

/spec:finalize add-greeting
  --> push branches, observe PRs, archive plan
```

The same rhythm runs at N=1 and N=12: `intent.enumerate` makes the trivial case degenerate rather than special.

## Two adapter roles

Specify 2.0 splits adapters by direction. **Source adapters** (`intent`, `documentation`, `code-typescript`, `screenshots`) read external material and emit `Evidence`. **Target adapters** (`omnia`, `vectis`, `contracts`) consume `spec.md` + `design.md` and produce code. See [Anatomy of an adapter](explanation/adapter-anatomy.md).

## Start here

{{#template templates/audience-grid-open.md}}

{{#template templates/audience-open.md who=New to Specify}}
Read [What is Specify?](orientation/index.md), install the [Prerequisites](orientation/prerequisites.md), then skim the [Quick reference card](reference/quick-reference.md).
{{#template templates/audience-close.md}}

{{#template templates/audience-open.md who=Returning from 1.x}}
[Release notes](explanation/release-notes.md) catalogues the 2.0 cut.
{{#template templates/audience-close.md}}

{{#template templates/audience-open.md who=Authoring skills or adapters}}
Jump to [Contributing](contributing/index.md) and [Skill authoring standards](standards/skill-authoring.md).
{{#template templates/audience-close.md}}

{{#template templates/audience-grid-close.md}}

## Guide structure

- **[Getting Started](orientation/index.md)** builds the mental model and gets you set up.
- **[Reference](reference/index.md)** provides the complete lookup table for every skill, CLI command, artifact format, and configuration file.
- **[Understanding Specify](explanation/layered-stack.md)** explains the architecture and design decisions.
- **[Contributing](contributing/index.md)** covers skill authoring, plugin development, and checks.
