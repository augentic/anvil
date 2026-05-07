---
name: specify-plan
description: "Deprecated alias. /spec:plan moved to /change:plan in RFC-13 §3.9. This shim delegates to the canonical change-plan skill, prints a one-line deprecation warning, and is removed before the post-RFC release. Use only if invoking the historical command path; new work should call /change:plan directly."
argument-hint: "<change-name>"
---

# /spec:plan (DEPRECATED — use /change:plan)

`/spec:plan` is no longer the canonical entry point for plan authoring. RFC-13 §3.9 moved the skill from the `spec` plugin to the new `change` plugin so that umbrella orchestration (the change surface) and per-loop phases (the spec surface) are owned separately.

**Use [`/change:plan`](../../../change/skills/plan/SKILL.md) instead.** This shim exists for one release cycle to ease the transition; it will be removed before the post-RFC-13 release per [RFC-13 §Migration](../../../../rfcs/archive/rfc-13-extensibility.md#migration).

## What to do when this skill is invoked

1. Print **exactly one** deprecation warning to stdout, on the first line of output:

   ```text
   Deprecated: /spec:plan moved to /change:plan in RFC-13 §3.9. This shim will be removed before release. See ../../../change/skills/plan/SKILL.md.
   ```

2. Then read `../../../change/skills/plan/SKILL.md` and follow it verbatim. Every flag, every step of the Critical Path, every brief invocation, and every `specify change plan *` shell-out is the canonical skill's responsibility — this shim adds no behaviour of its own beyond the warning above.

3. The argument shape is unchanged (`<change-name>` plus the same `--from`, `--against`, `--source`, `--focus`, `--extend`, `--dry-run`, `--orchestrate`, and `--shape` flag set; the retired `--auto-merge` flag is passed through only so the canonical skill can reject it with the RFC-14 diagnostic). Pass through whatever the operator supplied.

## Why this shim exists

Operators and CI scripts that pinned the old `/spec:plan` slash command keep working through one release cycle. Live documentation already teaches `/change:plan`; this shim is a compatibility aid only.

## When this shim disappears

The deprecation shim is removed **before the post-RFC-13 release** per the §Migration "Hard cut-over, no fallback path" rule. After removal, invoking `/spec:plan` returns "skill not found" and the operator is expected to use `/change:plan`.

## See also

- [`/change:plan`](../../../change/skills/plan/SKILL.md) — canonical authoring skill on the change surface.
- [`/change:execute`](../../../change/skills/execute/SKILL.md) — Layer 2 driver that consumes the authored plan.
- [RFC-13 §Migration](../../../../rfcs/archive/rfc-13-extensibility.md#migration) — the cut-over plan and timeline.
