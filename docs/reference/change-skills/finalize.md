# /spec:finalize

Close a drained change: push branches, observe PR state until every PR is `MERGED`, then archive the plan.

**Canonical reference.** The authoritative operator surface — synopsis, arguments, the step-by-step critical path, guardrails, the closing message, and error modes — is the [`/spec:finalize` skill body](../../../plugins/spec/skills/finalize/SKILL.md). This page is a navigation stub and carries no operator steps, so the two surfaces cannot drift.

## See also

- [/spec:execute](execute.md) — drives slices until drain
- [Cross-repo changes tutorial](../../tutorials/cross-repo-change.md) — workspace push and PR flow
- [specify plan](../cli/plan.md) — `plan archive`
- [Registry](../registry.md) — multi-repo platform setup
