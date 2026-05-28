# Release notes

This page highlights the most recent user-visible changes to Specify. For the full version-by-version history, see [`CHANGELOG.md`](https://github.com/augentic/specify/blob/main/CHANGELOG.md) at the repo root. For the *reasoning* behind architectural decisions, see the [Decision log](decision-log.md).

## 2.0 — Source / target split + plan-led workflow

The 2.0 release is a hard cut from 1.x. Two structural changes ship together.

**Adapters split by direction.** The unqualified 1.x "adapter" becomes two qualified roles: **source adapters** at `adapters/sources/<name>/adapter.yaml` (operations `survey` + `extract`) emit `Evidence`; **target adapters** at `adapters/targets/<name>/adapter.yaml` (operations `shape` + `build` + `merge`) consume `spec.md` + `design.md` and produce code. The adapter loader (`crates/domain/src/adapter/`) routes by axis; the manifest cache splits as `.specify/.cache/manifests/{sources,targets}/<name>/`, and the workflow §D8 per-source extraction cache lives in a sibling tree at `.specify/.cache/extractions/<adapter>/`. Core owns synthesis at both layers — `propose` fuses `Lead[]` into `slices[]`, `/spec:refine` fuses `Evidence[]` into `proposal.md` / `spec.md` / `design.md` / `tasks.md`. See [Anatomy of an adapter](adapter-anatomy.md).

**One operator workflow.** `/change:*` retires; every change runs through `/spec:plan` → Gate 1 (operator stamps `approved`) → `/spec:execute` → `/spec:finalize`. N=1 is degenerate, not special: `intent.survey` produces one lead. `/spec:define` and `/spec:extract` retire — `/spec:refine` covers both, breaking out of the loop only when execute parks or the operator wants to drive one slice by hand. See [Core concepts](concepts.md) and [The layered stack](layered-stack.md).

**New CLI verbs.** `specrun source resolve <name>`, `specrun target resolve <value>`, `specrun plan transition <name> approved`, `specrun plan amend --add-source / --remove-source / --divergence`. Retired: `specify adapter *`, `specify change *`, `specify change survey`, `specrun plan doctor`.

**Migration.** 2.0 is a hard cut with no in-tree upgrade script — bump the binary and reload plugins.

## Declared WASI adapter tools

Adapter authors and project authors declare WASI command components in `tools[]` on `adapter.yaml` (adapter scope) or `.specify/project.yaml` (project scope), and `specrun tool run <name>` resolves, caches, permissions, and executes them through a single CLI surface. Permissions are directory preopens — no globs, no symlink escapes, no writes to Specify lifecycle state — and project scope wins on collision so operators can redirect an adapter-shipped helper without editing the adapter. See [Tool declarations](tool-declarations.md) and [`specrun tool`](../reference/cli/tool.md).

## Slice and change vocabulary

The two lifecycle nouns are stable. A **slice** is the single unit that flows through the fixed `refine → build → merge` loop and lives at `.specify/slices/<name>/`. A **change** is the operator-defined umbrella — `change.md` plus `plan.yaml` — that coordinates one or more slices across one or more projects, driven through `/spec:plan` → `/spec:execute` → `/spec:finalize`. See [Core concepts](concepts.md).
