# Registry

The registry is a first-party Specify component — it owns project topology and the local materialised view that workspace-mode planning and execution run against.

## What is the registry?

The registry owns *project topology* — the declared list of projects, their repository locations, human descriptions, and default target adapter — **and** the local *materialised view* of those projects under `.specify/workspace/`. It is not a plugin: it has commands, libraries, and files, but it does not participate in the source/target adapter manifest protocol.

Target adapters own outcome artefacts and their mechanics; the registry coordinates *where* — which project a slice runs against and how that project's working tree is materialised. The plan (`/spec:plan`, `specrun plan *`) coordinates *when* — sequencing slices across one or more registered projects.

## Files and state

| Path                          | Owner    | Purpose |
| ----------------------------- | -------- | ------- |
| `registry.yaml`               | operator | Topology ledger at the repo root. Optional: absent or single-entry registries behave like single-repo mode. |
| `.specify/workspace/<peer>/`  | derived  | Materialised view of each registry entry — a `git clone` for remote URLs or a symlink for `.` / repo-relative paths. Refreshed by `specrun workspace sync`. |
| `.specify/.cache/`            | derived  | Adapter-manifest cache (owned by the plugin resolver, split into `adapters/sources/` and `adapters/targets/` subdirectories). |

`.specify/workspace/` and `.specify/.cache/` are framework-managed scratch and must never be checked in. `specrun init` and `specrun workspace sync` append the matching `.gitignore` lines idempotently.

## Topology shape

`registry.yaml` is a closed YAML document — unknown keys fail at parse time. A minimal entry:

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    target: omnia@v1
    description: Real-time traffic ingestion service.
  - name: command-centre
    url: git@github.com:org/command-centre.git
    target: omnia@v1
    description: Operator dashboard and control plane.
```

| Field                       | Required                     | Meaning |
| --------------------------- | ---------------------------- | ------- |
| `version`                   | yes                          | Schema version. `1` is the only accepted value for this release. |
| `projects`                  | optional (defaults to empty) | Ordered list of registered projects. Empty or single-entry registries behave like single-repo mode. |
| `projects[].name`           | yes                          | Kebab-case identifier. Must be unique within the registry. |
| `projects[].url`            | yes                          | Clone target — `.`, a repo-relative path (`../peer`, `./foo`), `git@host:path`, or an `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://` remote. |
| `projects[].target`         | yes                          | Target adapter identifier — e.g. `omnia@v1`. Opaque at the registry layer. |
| `projects[].description`    | conditional                  | Single-sentence domain characterisation. Required when more than one project is declared. |
| `projects[].contracts`      | optional                     | Per-project contract role declarations (`produces`, `consumes`). |

## Verbs

The registry is hand-curated YAML — operators edit `registry.yaml` directly; first-use validators reject malformed shapes:

| Caller                          | Validation timing |
| ------------------------------- | ----------------- |
| `specrun workspace sync`        | Validates the registry before materialising any slot; refuses to operate on a malformed registry. |
| `/spec:plan`                    | Validates the registry before propose; refuses to write a plan against a malformed registry. |

None of these go through the per-slice loop. The registry is substrate: it is what the slice loop runs *over*, not something the slice loop produces.

## Workspace materialisation

`.specify/workspace/` is **derived registry state**, not a separate component-owned topology. The workspace root owns `registry.yaml`; each child path under `.specify/workspace/<project>/` is a workspace slot for one registry entry. Slots may be refreshed, inspected, published from, or removed during final cleanup without changing the registry ledger.

The registry crate owns the materialiser and workspace verbs:

| Verb                                                | Purpose |
| --------------------------------------------------- | ------- |
| `specrun workspace sync [<project>...]`              | Materialise selected workspace slots. With no selectors, materialises every registry project; with selectors, materialises only those slots. Unknown selectors fail before filesystem or Git side effects. |
| `specrun workspace prepare <project>`         | Prepare the selected slot on `specify/<change-name>` from `origin/HEAD` before mutation. |
| `specrun workspace push [<project>...]`              | Transport-only publication for selected slots already on `specify/<change-name>`. Creates or updates PRs; never creates local branches, commits files, pushes default branches, or merges PRs. |

Selection is resolved once, before side effects. A human-invoked `workspace sync` with no selectors refreshes every registered project. `/spec:execute` uses selected sync behavior to materialise only the next plan entry's target slot before execution.

Before `/spec:execute` mutates a remote-backed slot, it prepares the slot on the change branch (`specify/<change-name>`) from the remote default branch (`origin/HEAD`). If `origin/HEAD` cannot be resolved, the executor surfaces `origin-head-unresolved` and does not run refine/build/merge. The `workspace merge` verb was removed pre-2.0 — landing is an explicit operator action outside Specify.

After `workspace push` opens or updates PRs, landing is an explicit operator action outside Specify. Use the forge UI, `gh pr merge`, or the repository's normal merge queue. `/spec:finalize` invokes `specrun plan finalize` to verify that every required per-project PR is merged, check workspace cleanliness, archive the plan, and (with `specrun plan finalize --clean`) optionally remove clean workspace clones; it never merges PRs.

## Dependency direction

The dependency edge is one-way; `specify-core` never depends on the registry.

```text
specify (binary) → specify-domain → specify-tool → specify-error
                                  ↑
                                  plugin loader (routes by axis)
```

The invariant: the slice and plan layers MAY depend on the registry because workspace routing composes registry materialisation; the reverse is forbidden.

## What the registry must NOT own

The registry is topology plus local materialisation. It is **not** a place to park orchestration, validation findings, or PR metadata:

- Plan or slice status — owned by `specrun plan *` and `specrun slice *`.
- Contract relationships beyond the per-project role declarations — owned by the `contracts` target adapter.
- Validation findings — owned by adapter skills and helper binaries.
- Synthesis output — owned by core (`/spec:refine`).
- PR metadata beyond the local project operation being requested — owned by the forge (GitHub via `gh`); the registry only round-trips per-project status from `gh pr view`.

## See also

- [Target Adapters](targets/index.md) — target adapter manifest protocol.
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — the source/target split.
- [`specrun registry`](cli/registry.md) and [`specrun workspace`](cli/workspace.md) — current CLI command reference.
