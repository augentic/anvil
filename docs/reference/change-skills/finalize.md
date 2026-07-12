# /spec:finalize

Close a drained change: push the prepared branches, then archive the plan. Opening and merging pull requests is operator-owned and happens outside Specify.

**Canonical reference.** The authoritative operator surface — synopsis, arguments, the step-by-step critical path, guardrails, the closing message, and error modes — is the [`/spec:finalize` skill body](../../../plugins/spec/skills/finalize/SKILL.md). This page is a navigation stub and carries no operator steps, so the two surfaces cannot drift.

## See also

- [specify plan](../cli/plan.md#specify-plan-execute) — `plan execute` drives slices until drain
- [Cross-repo changes tutorial](../../tutorials/cross-repo-change.md) — operator-owned publication and archive flow
- [specify plan](../cli/plan.md) — `plan archive`
- [Registry](../registry.md) — multi-repo platform setup
