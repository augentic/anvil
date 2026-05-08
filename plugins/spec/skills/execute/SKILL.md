---
name: specify-execute
description: "Deprecated alias for `/change:execute`. Use only when an operator invokes `/spec:execute`; it prints one deprecation warning and delegates to the canonical change execution driver. New work should call `/change:execute` directly."
---

# /spec:execute (DEPRECATED — use /change:execute)

`/spec:execute` is no longer the canonical entry point for plan execution. RFC-13 §3.9 moved the skill from the `spec` plugin to the new `change` plugin so that umbrella orchestration (the change surface) and per-loop phases (the spec surface) are owned separately.

**Use [`/change:execute`](../../../change/skills/execute/SKILL.md) instead.** This shim exists for one release cycle to ease the transition; it will be removed before the post-RFC-13 release per [RFC-13 §Migration](../../../../rfcs/archive/rfc-13-extensibility.md#migration).

## What to do when this skill is invoked

1. Print **exactly one** deprecation warning to stdout, on the first line of output:

   ```text
   Deprecated: /spec:execute moved to /change:execute in RFC-13 §3.9. This shim will be removed before release. See ../../../change/skills/execute/SKILL.md.
   ```

2. Then read `../../../change/skills/execute/SKILL.md` and follow it verbatim. Every step of the per-slice algorithm, every mode (`dry-run`, supervised, `loop`), every self-heal branch, and every `specify change plan *` shell-out is the canonical skill's responsibility — this shim adds no behaviour of its own beyond the warning above.

3. The positional mode shape is unchanged (`dry-run`, `loop`). Pass through whatever the operator supplied.

## Why this shim exists

Operators and CI scripts that pinned the old `/spec:execute` slash command keep working through one release cycle. Live documentation already teaches `/change:execute`; this shim is a compatibility aid only.

## When this shim disappears

The deprecation shim is removed **before the post-RFC-13 release** per the §Migration "Hard cut-over, no fallback path" rule. After removal, invoking `/spec:execute` returns "skill not found" and the operator is expected to use `/change:execute`.

## See also

- [`/change:execute`](../../../change/skills/execute/SKILL.md) — canonical driver skill on the change surface.
- [`/change:plan`](../../../change/skills/plan/SKILL.md) — Layer 3 authoring skill that produces the plan this driver consumes.
- [RFC-13 §Migration](../../../../rfcs/archive/rfc-13-extensibility.md#migration) — the cut-over plan and timeline.
