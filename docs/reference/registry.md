# Registry

The registry is a first-party Specify component, not a adapter — it owns project topology and the local materialised view that change orchestration runs against.

## What is the registry?

The registry is the first-party Specify component that owns *project topology* — the declared list of projects, their repository locations, human descriptions, and default adapter — **and** the local *materialised view* of those projects under `.specify/workspace/`. It is not a adapter: it has commands, libraries, and files, but it does not participate in the adapter manifest protocol and is not activated through `adapter.yaml`. See [Platform components are not adapters](../explanation/decision-log.md#platform-components-are-not-adapters) for the rationale.

Adapters own outcome artefacts and their mechanics; the registry coordinates *where* — which project a slice runs against and how that project's working tree is materialised. The slice component (see [`change-component.md`](change-component.md)) coordinates *when* — sequencing slices across one or more registry projects.

## Files and state

| Path                      | Owner    | Purpose                                                                                                                                                              |
| ------------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registry.yaml`           | operator | Topology ledger at the repo root. Optional: an absent or single-entry registry is equivalent to single-repo mode.                                                    |
| `.specify/workspace/<peer>/` | derived | Materialised view of each registry entry — a `git clone` for remote URLs or a symlink for `.` / repo-relative paths. Refreshed by `specify workspace sync`.          |
| `.specify/.cache/`        | derived  | Adapter-manifest cache (owned by the adapter resolver). The registry crate updates `.gitignore` to ignore both `.specify/workspace/` and `.specify/.cache/`. |

`.specify/workspace/` and `.specify/.cache/` are framework-managed scratch and must never be checked in. The registry crate appends the two `.gitignore` lines idempotently on every `specify init` and `specify workspace sync`.

## Topology shape

`registry.yaml` is a closed YAML document — unknown keys fail at parse time. A minimal entry:

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    adapter: omnia@v1
    description: Real-time traffic ingestion service.
  - name: command-centre
    url: git@github.com:org/command-centre.git
    adapter: omnia@v1
    description: Operator dashboard and control plane.
```

| Field         | Required                          | Meaning                                                                                                                                                                              |
| ------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `version`     | yes                               | Schema version. `1` is the only accepted value for this release.                                                                                                                     |
| `projects`    | optional (defaults to empty)      | Ordered list of registered projects. Empty or single-entry registries behave like single-repo mode.                                                                                  |
| `projects[].name` | yes                           | Kebab-case identifier. Must be unique within the registry.                                                                                                                           |
| `projects[].url`  | yes                           | Clone target — `.`, a repo-relative path (`../peer`, `./foo`), `git@host:path`, or an `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://` remote.                            |
| `projects[].adapter` | yes                         | Adapter identifier — e.g. `omnia@v1`. Opaque at the registry layer; the `name@version` suffix is not parsed here.                                                                 |
| `projects[].description` | conditional               | Single-sentence domain characterisation. Required when more than one project is declared (the `description-missing-multi-repo` invariant); optional in single-project registries.   |
| `projects[].contracts`   | optional                  | Per-project contract role declarations (`produces`, `consumes`).                                                                                                                     |

> **Note on the `adapter:` field name.** The on-disk key on registry entries is spelled `adapter:`. Treat the field name as opaque registry vocabulary.

The wire-level shape is enforced by the registry crate's `Registry::validate_shape` (kebab-case, non-empty required strings, version, URL classification, multi-project description, optional `contracts` consistency). For the full type definition, see `crates/registry/src/registry.rs` in `augentic/specify-cli`.

## Verbs

The registry surface is mutated **directly**, not through the slice loop:

| Verb                       | Purpose                                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify registry add`     | Append a new project entry. Creates `registry.yaml` with `version: 1` when absent. Validates the resulting shape, including the `description-missing-multi-repo` invariant.          |
| `specify registry remove`  | Delete a project entry. Warns (non-fatal) when the current `plan.yaml` references the removed project so the operator can rewire affected change entries.                            |
| `specify registry show`    | Render the parsed registry as text or JSON. JSON is the canonical surface that change-draft skills consume to detect multi-project mode.                                          |
| `specify registry validate`| Shape and referential integrity check; `Registry::validate_shape` plus hub-mode invariants. Absent registry is not an error (exit 0).                                                |

None of these verbs go through `define → build → merge`. The registry is substrate: it is what the slice loop runs *over*, not something the slice loop produces.

## Workspace materialisation

`.specify/workspace/` is **derived registry state**, not a separate component-owned topology. The coordinator root owns `registry.yaml`; each child path under `.specify/workspace/<project>/` is a workspace slot for one registry entry. Slots may be refreshed, inspected, published from, or removed during final cleanup without changing the registry ledger.

The registry crate owns the materialiser and workspace verbs:

| Verb                       | Purpose                                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify workspace sync [<project>...]`   | Materialise selected workspace slots. With no selectors, materialises every registry project; with selectors, materialises only those slots. Unknown selectors fail before filesystem or Git side effects. |
| `specify workspace status [<project>...]` | Read-only per-project materialisation report: slot path/type, configured target, actual origin or symlink target, current branch, HEAD, dirty flag, exact `specify/<change-name>` branch match, `.specify/project.yaml` presence, and active slices. |
| `specify workspace push [<project>...]`   | Transport-only publication for selected slots already on exact `specify/<change-name>`. Pushes with lease protection and creates or updates PRs; never creates local branches, commits files, pushes default branches, or merges PRs. |

Selection is resolved once, before side effects. This means `specify workspace sync api typo` fails without materialising `api`, and `specify workspace push web unknown` fails before any push or PR creation. A human-invoked `workspace sync` with no selectors still refreshes every registry project. `/change:execute` uses selected sync behavior to materialise only the next plan entry's target slot before execution.

Before `/change:execute` mutates a remote-backed slot, it prepares the slot on the change branch (`specify/<change-name>`) from the remote default branch (`origin/HEAD`). If `origin/HEAD` cannot be resolved, the executor surfaces `origin-head-unresolved` and does not run define/build/merge. Humans generally do not invoke the hidden branch-preparation helper directly; they use `/change:execute`, inspect with `workspace status`, publish with `workspace push`, merge PRs through their forge, and close with `change finalize`. `specify workspace merge` has been removed.

The registry-materialisation resolver — the registry service that maps a registry-declared project to its materialised project root — is what change execution consumes when running the slice loop against a peer project. Adapter skills run relative to *the clone's project root*; the core receives only the project root it should run against.

After `workspace push` opens or updates PRs, landing is an explicit operator action outside Specify. Use the forge UI, `gh pr merge`, or the repository's normal merge queue. `specify change finalize` later verifies that every required per-project PR is merged, checks workspace cleanliness, archives the coordinator state, and optionally removes clean workspace clones with `--clean`; it never merges PRs.

## Dependency direction

The registry sits between the slice component and the lower-level core/adapter crates. The post-Phase-3 dependency edge is one-way:

```text
specify-change → specify-registry → specify-adapter
                                 → specify-core
```

The invariant: **`specify-core` does not depend on `specify-registry`**, and `specify-registry` does not depend on `specify-change`. The slice component MAY depend on the registry because orchestration composes registry materialisation; the reverse is forbidden. A workspace lint enforces it.

## What the registry must NOT own

The registry is topology plus local materialisation. It is **not** a place to park orchestration, validation findings, or PR metadata:

- Change or plan status — owned by `specify change` (`change.md`, `plan.yaml`, `.specify/slices/<name>/.metadata.yaml`).
- Contract relationships beyond the per-project role declarations — owned by the `contracts@v1` adapter.
- Validation findings — owned by adapter skills and helper binaries.
- Change execution — owned by `specify change` (the orchestration umbrella).
- Adapter-specific validation — owned by adapter skills and the merge brief.
- PR metadata beyond the local project operation being requested — owned by the forge (GitHub via `gh`); the registry only round-trips per-project status from `gh pr view`.

## See also

- [Adapters](adapters/index.md) — adapter manifest protocol and the dependency direction sister page.
- [Change Component](change-component.md) — operator brief, plan, execution state, finalization, and archive.
- [Platform Repo Topologies](../explanation/platform-repo.md) — the registry-only hub vs platform-as-project shapes.
- [`specify registry`](cli/registry.md) and [`specify workspace`](cli/workspace.md) — current CLI command reference.
