# Release notes

This page highlights the most recent user-visible changes to Specify. For the full version-by-version history, see [`CHANGELOG.md`](https://github.com/augentic/specify/blob/main/CHANGELOG.md) at the repo root. For the *reasoning* behind architectural decisions, see the [Decision log](decision-log.md).

## Change lifecycle restructured into three peer skills

The change layer reads `/change:draft → /change:execute → /change:finalize`, with a deliberate operator review pause between authoring and execution. `/change:draft` mints `change.md` and `plan.yaml`, runs registry validate, walks the brief pipeline, and stops. `/change:execute loop` drives the per-slice define → build → merge loop. `/change:finalize` pushes branches, observes PR state, and runs `specify change finalize` once every PR is `MERGED`. The `orchestrate` umbrella mode is removed; the CLI verb `specify change create` is renamed to `specify change draft`. See [RFC-23](../../rfcs/archive/rfc-23-change-lifecycle.md), the [Cross-Repo Changes tutorial](../tutorials/cross-repo-change.md), and [Change skills](../reference/change-skills/index.md).

## Contracts as first-party platform artifacts

API contracts live at `contracts/` alongside `registry.yaml` and `plan.yaml`, using JSON Schema for payloads with OpenAPI 3.1 and AsyncAPI 3.0 as protocol bindings. The contracts adapter owns the merge-time validator, and `/change:draft` automatically inserts a contract slice before implementation work whenever it detects an API boundary between projects. See the [Contracts adapter](../reference/adapters/contracts.md) reference and [Work with contracts across repos](../how-to/cross-repo-contracts.md).

## Hub vs platform-as-project topologies

A multi-repo change can start from a **registry-only platform hub** (`specify init --hub --name shop-platform`) that holds `registry.yaml`, `change.md`, `plan.yaml`, and the `workspace/` slots but is never itself a code project. The older platform-as-project shape — an initiating repo with `url: .` — is still supported for single-repo and small-team cases. See [Platform repo topologies](platform-repo.md) and [Bootstrap a platform hub](../how-to/bootstrap-a-platform-hub.md).

## Declared WASI adapter tools

Adapter authors and project authors declare WASI command components in `tools.yaml` (adapter scope) or `.specify/project.yaml` (project scope), and `specify tool {list, fetch, show, run}` resolves, caches, permissions, and executes them through a single CLI surface. Permissions are directory preopens — no globs, no symlink escapes, no writes to Specify lifecycle state — and project scope wins on collision so operators can redirect a adapter-shipped helper without editing the adapter. See [Tool declarations](tool-declarations.md) and [`specify tool`](../reference/cli/tool.md).

## Slice and change vocabulary

The two lifecycle nouns are stable. A **slice** is the single unit that flows through the fixed `define → build → merge` loop and lives at `.specify/slices/<name>/`. A **change** is the operator-defined unit of work — `change.md` plus `plan.yaml` — that coordinates one or more slices across one or more projects, driven through the three-skill lifecycle (`/change:draft → /change:execute → /change:finalize`). See [Core concepts](concepts.md).
