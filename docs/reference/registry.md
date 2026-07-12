# Registry

The registry is a first-party Specify component — it owns project membership and location and the local materialised view that workspace-mode planning and execution run against.

## What is the registry?

The registry owns *project membership and location* — the declared list of projects and their repository locations — **and** the local *materialised view* of those projects under top-level `workspace/`. It is not a plugin: it has commands, libraries, and files, but it does not participate in the source/target adapter manifest protocol.

The registry does **not** author a project's target adapter or description for plan-time topology — `adapter` and `description` are authored in each project's `.specify/project.yaml`, and the project's *derived* routing identity (`surface[]` / `recent[]`) is projected from that project's own baseline into the committed `.specify/topology.lock`. Regenerating that lock and materializing slots are operator-owned. The registry's `adapter` field survives only as an optional greenfield scaffold seed for surrounding repository tooling.

Target adapters own outcome artefacts and their mechanics; the registry coordinates *where* — which project a slice runs against and how that project's working tree is materialised. The plan (`/spec:plan`, `specify plan *`) coordinates *when* — sequencing slices across one or more registered projects.

## Files and state

| Path                          | Owner    | Purpose |
| ----------------------------- | -------- | ------- |
| `registry.yaml`               | operator | Membership + location ledger at the repo root. Optional: absent or single-entry registries behave like single-repo mode. |
| `.specify/topology.lock`      | derived  | Committed projection of each member project's identity: authored `target` / `description` plus the deterministic baseline projection `surface[]` (owned domains + requirement titles) and `recent[]` (merge-outcome tail). Machine-written by operator-owned topology tooling; never hand-edited. |
| `workspace/<peer>/`  | derived  | Materialised view of each registry entry at the project root — typically a checkout/worktree for remote URLs or a symlink for `.` / repo-relative paths. |
| `<project-cache>/` (out-of-tree) | derived  | Memoization root in the out-of-tree per-project cache, including operator-supplied components mirrored under `components/`. |
| `.specify/scratch/`          | derived  | Transient working state: per-operation agent scratch lanes under `<adapter>/{survey,<slice>}/` and the plan handoff lane under `plan/`. Lanes are recreated empty by their owning verb. |

`.specify/scratch/` and top-level `workspace/` are regenerable and must never be checked in; the per-project cache lives out-of-tree. `specify init` appends the matching `.gitignore` lines idempotently.

## Topology shape

`registry.yaml` is a closed YAML document — unknown keys fail at parse time. A minimal entry:

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    adapter: omnia@1.0.0        # optional greenfield scaffold seed only
    greenfield_seed:         # optional plan-time routing seed only
      domains: [ingest, alerting]
  - name: command-centre
    url: git@github.com:org/command-centre.git
```

| Field                       | Required                     | Meaning |
| --------------------------- | ---------------------------- | ------- |
| `version`                   | yes                          | Schema version. `1` is the only accepted value for this release. |
| `projects`                  | optional (defaults to empty) | Ordered list of registered projects. Empty or single-entry registries behave like single-repo mode. |
| `projects[].name`           | yes                          | Kebab-case identifier. Must be unique within the registry. The slot name and the binding key written to `plan.yaml.slices[].project`. |
| `projects[].url`            | yes                          | Clone target — `.`, a repo-relative path (`../peer`, `./foo`), `git@host:path`, or an `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://` remote. |
| `projects[].adapter`        | optional                     | Greenfield scaffold seed for operator tooling. Not read for plan-time topology; the project's own `project.yaml` is authoritative once it exists. |
| `projects[].contracts`      | optional                     | Per-project contract role declarations (`produces`, `consumes`). |
| `projects[].greenfield_seed.domains` | optional            | Greenfield routing seed only — kebab-case domain slugs that project into the project's plan-time `surface[]` as domains with empty requirements *while the project has no baseline*. The greenfield analog of the derived `surface[]` domain list, it lets a fresh project route leads before `.specify/specs/` exists. Ignored once a real baseline exists (the derived surface supersedes it); a still-declared seed then surfaces the advisory `greenfield-seed-shadowed` finding at plan-authoring time. Carries domain slugs only — never adapter or description material. |

A project's `description` is **not** a registry field — it is authored in the project's own `.specify/project.yaml`. Routing identity (`surface[]` / `recent[]`) is derived from that project's baseline and projected into `.specify/topology.lock`; the hand-authored `capabilities` / `keywords` facets are removed.

## Verbs

The registry is hand-curated YAML — operators edit `registry.yaml` directly; first-use validators reject malformed shapes:

| Caller                          | Validation timing |
| ------------------------------- | ----------------- |
| `/spec:plan`                    | Validates the registry before propose; refuses to write a plan against a malformed registry. |

None of these go through the per-slice loop. The registry is substrate: it is what the slice loop runs *over*, not something the slice loop produces.

## Workspace materialisation

Top-level `workspace/` is **derived registry state**, not a separate component-owned topology. The workspace owns `registry.yaml`; each child path under `workspace/<project>/` is a workspace slot for one registry entry. Slots may be refreshed, inspected, published from, or removed during final cleanup without changing the registry ledger.

Specify exposes no workspace materialization or publication verbs. The operator or surrounding repository automation owns:

- Creating or refreshing each slot from `projects[].url`.
- Regenerating `.specify/topology.lock` from member project metadata and baselines.
- Preparing branches and clean working trees before execution.
- Committing, publishing, reviewing, and merging repository changes after execution.

`specify plan execute` expects required slots to exist and routes project-bound work into them. `/spec:finalize` archives only after the operator confirms publication is complete.

## Dependency direction

The dependency edge is one-way; `specify-core` never depends on the registry.

```text
specify (binary) → workflow → specify-tool → error
                                  ↑
                                  plugin loader (routes by axis)
```

The invariant: the slice and plan layers MAY depend on the registry because workspace routing consumes registry topology; the reverse is forbidden.

## What the registry must NOT own

The registry is topology plus the declaration behind operator-owned local materialisation. It is **not** a place to park orchestration, validation findings, or PR metadata:

- Plan or slice status — owned by `specify plan *` and `specify slice *`.
- Contract relationships beyond the per-project role declarations — owned by the `contracts` target adapter.
- Validation findings — owned by adapter skills and helper binaries.
- Synthesis output — owned by core (`/spec:refine`).
- PR metadata — owned entirely by the forge and the operator; Specify does not publish branches or track pull-request state.

## See also

- [Target Adapters](targets/index.md) — target adapter manifest protocol.
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — the source/target split.
- [`specify registry`](cli/registry.md) and [Workspace topology](cli/workspace.md) — registry commands and operator-owned workspace setup.
