# /spec:refine

Refine a plan entry's slice — run extract per bound source, synthesize proposal, spec, design, and tasks, validate, transition to `refined`.

**Canonical reference.** The authoritative operator surface — synopsis, arguments, the step-by-step critical path, guardrails, closing hints, and error modes — is the [`/spec:refine` skill body](../../../plugins/spec/skills/refine/SKILL.md). What the agent writes into the synthesis response is owned by the [synthesis playbook](../../../plugins/spec/references/synthesis/). This page is a navigation stub and carries no operator steps, so the two surfaces cannot drift.

## See also

- [Resolve spec conflicts](../../how-to/resolve-spec-conflicts.md) — `[conflict]` and `[divergence]` tags
- [/spec:build](build.md) — next phase after refine
- [Artifact format](../artifact-format.md) — requirement block shape
- [Lifecycle](../lifecycle.md) — slice state machine
