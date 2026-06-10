# Registry

The registry is a first-party Specify component — it owns project membership and location and the local materialised view that workspace-mode planning and execution run against.

## What is the registry?

The registry owns *project membership and location* — the declared list of projects and their repository locations — **and** the local *materialised view* of those projects under `.specify/workspace/`. It is not a plugin: it has commands, libraries, and files, but it does not participate in the source/target adapter manifest protocol.

The registry does **not** author a project's target adapter or description for plan-time topology — `adapter` and `description` are authored in each project's `.specify/project.yaml`, and the project's *derived* routing identity (`surface[]` / `recent[]`) is projected from that project's own baseline into the committed `.specify/topology.lock` by `specify workspace sync`. The registry's `adapter` field survives only as an optional *greenfield scaffold seed* used when `workspace sync` clones a brand-new, empty project.

Target adapters own outcome artefacts and their mechanics; the registry coordinates *where* — which project a slice runs against and how that project's working tree is materialised. The plan (`/spec:plan`, `specify plan *`) coordinates *when* — sequencing slices across one or more registered projects.

## Files and state

| Path                          | Owner    | Purpose |
| ----------------------------- | -------- | ------- |
| `registry.yaml`               | operator | Membership + location ledger at the repo root. Optional: absent or single-entry registries behave like single-repo mode. |
| `.specify/topology.lock`      | derived  | Committed projection of each member project's identity: authored `target` / `description` plus the deterministic baseline projection `surface[]` (owned units + requirement titles) and `recent[]` (merge-outcome tail). Machine-written by `specify workspace sync`; never hand-edited. |
| `.specify/workspace/<peer>/`  | derived  | Materialised view of each registry entry — a `git clone` for remote URLs or a symlink for `.` / repo-relative paths. Refreshed by `specify workspace sync`. |
| `.specify/cache/`            | derived  | Adapter-manifest cache (owned by the plugin resolver, split into `manifests/sources/` and `manifests/targets/` subdirectories, with extraction results under `extractions/`). |

`.specify/workspace/` and `.specify/cache/` are framework-managed scratch and must never be checked in. `specify init` and `specify workspace sync` append the matching `.gitignore` lines idempotently.

## Topology shape

`registry.yaml` is a closed YAML document — unknown keys fail at parse time. A minimal entry:

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    adapter: omnia@v1        # optional greenfield scaffold seed only
  - name: command-centre
    url: git@github.com:org/command-centre.git
```

| Field                       | Required                     | Meaning |
| --------------------------- | ---------------------------- | ------- |
| `version`                   | yes                          | Schema version. `1` is the only accepted value for this release. |
| `projects`                  | optional (defaults to empty) | Ordered list of registered projects. Empty or single-entry registries behave like single-repo mode. |
| `projects[].name`           | yes                          | Kebab-case identifier. Must be unique within the registry. The slot name and the binding key written to `plan.yaml.slices[].project`. |
| `projects[].url`            | yes                          | Clone target — `.`, a repo-relative path (`../peer`, `./foo`), `git@host:path`, or an `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://` remote. |
| `projects[].adapter`        | optional                     | Greenfield scaffold seed only — the adapter written into a brand-new project's `project.yaml` when `workspace sync` clones an empty repo. Not read for plan-time topology; the project's own `project.yaml` is authoritative once it exists. |
| `projects[].contracts`      | optional                     | Per-project contract role declarations (`produces`, `consumes`). |

A project's `description` is **not** a registry field — it is authored in the project's own `.specify/project.yaml`. Routing identity (`surface[]` / `recent[]`) is derived from that project's baseline and projected into `.specify/topology.lock`; the hand-authored `capabilities` / `keywords` facets are removed.

## Verbs

The registry is hand-curated YAML — operators edit `registry.yaml` directly; first-use validators reject malformed shapes:

| Caller                          | Validation timing |
| ------------------------------- | ----------------- |
| `specify workspace sync`        | Validates the registry before materialising any slot; refuses to operate on a malformed registry. |
| `/spec:plan`                    | Validates the registry before propose; refuses to write a plan against a malformed registry. |

None of these go through the per-slice loop. The registry is substrate: it is what the slice loop runs *over*, not something the slice loop produces.

## Workspace materialisation

`.specify/workspace/` is **derived registry state**, not a separate component-owned topology. The workspace owns `registry.yaml`; each child path under `.specify/workspace/<project>/` is a workspace slot for one registry entry. Slots may be refreshed, inspected, published from, or removed during final cleanup without changing the registry ledger.

The registry crate owns the materialiser and workspace verbs:

| Verb                                                | Purpose |
| --------------------------------------------------- | ------- |
| `specify workspace sync [<project>...]`              | Materialise selected workspace slots. With no selectors, materialises every registry project; with selectors, materialises only those slots. Unknown selectors fail before filesystem or Git side effects. |
| `specify workspace prepare <project>`         | Prepare the selected slot on `specify/<change-name>` from `origin/HEAD` before mutation. |
| `specify workspace push [<project>...]`              | Transport-only publication for selected slots already on `specify/<change-name>`. Creates or updates PRs; never creates local branches, commits files, pushes default branches, or merges PRs. |

Selection is resolved once, before side effects. A human-invoked `workspace sync` with no selectors refreshes every registered project. `/spec:execute` uses selected sync behavior to materialise only the next plan entry's target slot before execution.

Before `/spec:execute` mutates a remote-backed slot, it prepares the slot on the change branch (`specify/<change-name>`) from the remote default branch (`origin/HEAD`). If `origin/HEAD` cannot be resolved, the executor surfaces `origin-head-unresolved` and does not run refine/build/merge. There is no `workspace merge` verb — landing is an explicit operator action outside Specify.

After `workspace push` opens or updates PRs, landing is an explicit operator action outside Specify. Use the forge UI, `gh pr merge`, or the repository's normal merge queue. `/spec:finalize` verifies each required per-project PR with `gh pr view`, then invokes `specify plan archive` to archive the plan; it never merges PRs.

## Dependency direction

The dependency edge is one-way; `specify-core` never depends on the registry.

```text
specify (binary) → specify-workflow → specify-tool → specify-error
                                  ↑
                                  plugin loader (routes by axis)
```

The invariant: the slice and plan layers MAY depend on the registry because workspace routing composes registry materialisation; the reverse is forbidden.

## What the registry must NOT own

The registry is topology plus local materialisation. It is **not** a place to park orchestration, validation findings, or PR metadata:

- Plan or slice status — owned by `specify plan *` and `specify slice *`.
- Contract relationships beyond the per-project role declarations — owned by the `contracts` target adapter.
- Validation findings — owned by adapter skills and helper binaries.
- Synthesis output — owned by core (`/spec:refine`).
- PR metadata beyond the local project operation being requested — owned by the forge (GitHub via `gh`); the registry only round-trips per-project status from `gh pr view`.

## See also

- [Target Adapters](targets/index.md) — target adapter manifest protocol.
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — the source/target split.
- [`specify registry`](cli/registry.md) and [`specify workspace`](cli/workspace.md) — current CLI command reference.
