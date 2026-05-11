# Release notes

This page highlights the most recent user-visible changes to Specify. For the full version-by-version history, see [`CHANGELOG.md`](https://github.com/augentic/specify/blob/main/CHANGELOG.md) at the repo root. For the *reasoning* behind architectural decisions, see the [Decision log](decision-log.md).

## Contracts as first-party platform artifacts

API contracts live at `contracts/` alongside `registry.yaml` and `plan.yaml`, using JSON Schema for payloads with OpenAPI 3.1 and AsyncAPI 3.0 as protocol bindings. The contracts capability owns the merge-time validator, and `/change:plan` automatically inserts a contract slice before implementation work whenever it detects an API boundary between projects. See the [Contracts capability](../reference/capabilities/contracts.md) reference and [Work with contracts across repos](../how-to/cross-repo-contracts.md).

## Cross-repo platform-first workflow

A single operator action — `/change:plan <name> orchestrate` — drives the cross-repo loop end-to-end: brief, registry validate, plan, execute loop, workspace push, operator PR merge, and `specify change finalize`. The umbrella mode composes existing CLI verbs and skills without adding new logic, so halts surface verbatim and re-running an in-progress change resumes at the first incomplete step. See the [Cross-Repo Changes tutorial](../tutorials/cross-repo-change.md) and the orchestrate umbrella under [Change skills](../reference/change-skills/index.md).

## Hub vs platform-as-project topologies

A multi-repo change can start from a **registry-only platform hub** (`specify init --hub --name shop-platform`) that holds `registry.yaml`, `change.md`, `plan.yaml`, and the `workspace/` slots but is never itself a code project. The older platform-as-project shape — an initiating repo with `url: .` — is still supported for single-repo and small-team cases. See [Platform repo topologies](platform-repo.md) and [Bootstrap a platform hub](../how-to/bootstrap-a-platform-hub.md).

## Declared WASI capability tools

Capability authors and project authors declare WASI command components in `tools.yaml` (capability scope) or `.specify/project.yaml` (project scope), and `specify tool {list, fetch, show, run}` resolves, caches, permissions, and executes them through a single CLI surface. Permissions are directory preopens — no globs, no symlink escapes, no writes to Specify lifecycle state — and project scope wins on collision so operators can redirect a capability-shipped helper without editing the capability. See [Tool declarations](tool-declarations.md) and [`specify tool`](../reference/cli/tool.md).

## Slice and change vocabulary

The two lifecycle nouns are stable. A **slice** is the single unit that flows through the fixed `define → build → merge` loop and lives at `.specify/slices/<name>/`. A **change** is the operator-defined umbrella — `change.md` plus `plan.yaml` — that coordinates one or more slices across one or more projects. See [Core concepts](concepts.md).
