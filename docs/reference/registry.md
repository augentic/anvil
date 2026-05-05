# Registry

> Status: Draft (Phase 2.10 of [RFC-13](../../rfcs/rfc-13-extensibility.md) landed). The registry is a first-party Specify component, not a capability — it owns project topology and the local materialised view that change orchestration runs against.

## What is the registry?

The registry is the first-party Specify component that owns *project topology* — the declared list of projects, their repository locations, human descriptions, and default capability — **and** the local *materialised view* of those projects under `.specify/workspace/`. It is not a capability: it has commands, libraries, and files, but it does not participate in the capability manifest protocol and is not activated through `capability.yaml`. See [RFC-13 §"Platform components are not capabilities"](../../rfcs/rfc-13-extensibility.md#platform-components-are-not-capabilities) and [RFC-13 §"Registry-materialised execution"](../../rfcs/rfc-13-extensibility.md#registry-materialised-execution).

Capabilities own outcome artefacts and their mechanics; the registry coordinates *where* — which project a slice runs against and how that project's working tree is materialised. The change component (see [`change-component.md`](change-component.md)) coordinates *when* — sequencing slices across one or more registry projects.

## Files and state

| Path                      | Owner    | Purpose                                                                                                                                                              |
| ------------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registry.yaml`           | operator | Topology ledger at the repo root. Optional: an absent or single-entry registry is equivalent to single-repo mode.                                                    |
| `.specify/workspace/<peer>/` | derived | Materialised view of each registry entry — a `git clone` for remote URLs or a symlink for `.` / repo-relative paths. Refreshed by `specify workspace sync`.          |
| `.specify/.cache/`        | derived  | Capability-manifest cache (owned by the capability resolver). The registry crate updates `.gitignore` to ignore both `.specify/workspace/` and `.specify/.cache/`. |

`.specify/workspace/` and `.specify/.cache/` are framework-managed scratch and must never be checked in. The registry crate appends the two `.gitignore` lines idempotently on every `specify init` and `specify workspace sync`.

## Topology shape

`registry.yaml` is a closed YAML document — unknown keys fail at parse time. A minimal entry:

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1
    description: Real-time traffic ingestion service.
  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1
    description: Operator dashboard and control plane.
```

| Field         | Required                          | Meaning                                                                                                                                                                              |
| ------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `version`     | yes                               | Schema version. `1` is the only accepted value for this release.                                                                                                                     |
| `projects`    | optional (defaults to empty)      | Ordered list of registered projects. Empty or single-entry registries behave like single-repo mode.                                                                                  |
| `projects[].name` | yes                           | Kebab-case identifier. Must be unique within the registry.                                                                                                                           |
| `projects[].url`  | yes                           | Clone target — `.`, a repo-relative path (`../peer`, `./foo`), `git@host:path`, or an `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://` remote.                            |
| `projects[].schema` | yes                         | Capability identifier — e.g. `omnia@v1`. Opaque at the registry layer; the `name@version` suffix is not parsed here.                                                                 |
| `projects[].description` | conditional               | Single-sentence domain characterisation. Required when more than one project is declared (the `description-missing-multi-repo` invariant); optional in single-project registries.   |
| `projects[].contracts`   | optional                  | Per-project contract role declarations (`produces`, `consumes`); see RFC-12 for the role surface.                                                                                    |

> **Note on the `schema:` field name.** RFC-13 Phase 1 renamed the *extension primitive* from "schema" to "capability" everywhere except this one field on registry entries. The on-disk key continues to be spelled `schema:` until a later phase ships the corresponding rename and migration. Treat the field name as opaque registry vocabulary for now.

The wire-level shape is enforced by the registry crate's `Registry::validate_shape` (kebab-case, non-empty required strings, version, URL classification, multi-project description, optional `contracts` consistency). For the full type definition, see `crates/registry/src/registry.rs` in `augentic/specify-cli`.

## Verbs

The registry surface is mutated **directly**, not through the slice loop:

| Verb                       | Purpose                                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify registry add`     | Append a new project entry. Creates `registry.yaml` with `version: 1` when absent. Validates the resulting shape, including the `description-missing-multi-repo` invariant.          |
| `specify registry remove`  | Delete a project entry. Warns (non-fatal) when the current `plan.yaml` references the removed project so the operator can rewire affected change entries.                            |
| `specify registry show`    | Render the parsed registry as text or JSON. JSON is the canonical surface that change-planning skills consume to detect multi-project mode.                                          |
| `specify registry validate`| Shape and referential integrity check; `Registry::validate_shape` plus hub-mode invariants. Absent registry is not an error (exit 0).                                                |

None of these verbs go through `define → build → merge`. The registry is substrate: it is what the slice loop runs *over*, not something the slice loop produces.

## Workspace materialisation

`.specify/workspace/` is **derived registry state**, not a separate component-owned topology. The registry crate owns the materialiser and the four workspace verbs:

| Verb                       | Purpose                                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify workspace sync`   | Materialise `.specify/workspace/<name>/` for every registry entry — symlink for `.`/relative URLs, shallow `git clone` for remotes. Idempotent. Updates `.gitignore`.                |
| `specify workspace status` | Per-project materialisation report — slot type (symlink, git-clone, missing), HEAD sha, dirty flag, `.specify/` summary.                                                             |
| `specify workspace push`   | Push each clone's `specify/<change-name>` branch to its remote and open a PR. Branch name resolves from `plan.yaml`.                                                                 |
| `specify workspace merge`  | Squash-merge open PRs once their CI is green ([RFC-9 §4A](../../rfcs/archive/rfc-9-platform.md)). Refuses on `branch-pattern-mismatch`; never `--admin`/`--auto`.                    |

The registry-materialisation resolver — the registry service that maps a registry-declared project to its materialised project root — is what change execution consumes when running the slice loop against a peer project (see [RFC-13 §"Registry-materialised execution"](../../rfcs/rfc-13-extensibility.md#registry-materialised-execution)). Capability skills run relative to *the clone's project root*; the core receives only the project root it should run against.

## Dependency direction

The registry sits between the change component and the lower-level core/capability crates. The post-Phase-3 dependency edge is one-way:

```text
specify-change → specify-registry → specify-capability
                                 → specify-core
```

The invariant: **`specify-core` does not depend on `specify-registry`**, and `specify-registry` does not depend on `specify-change`. The change component MAY depend on the registry because orchestration composes registry materialisation; the reverse is forbidden. RFC-13 invariant #4 spells this out and [RFC-5](../../rfcs/rfc-5-lint.md) is the home for the lint that enforces it. See [RFC-13 §Migration](../../rfcs/rfc-13-extensibility.md#migration).

> **Crate naming on the rfc-13 branch.** The umbrella crate is currently named `specify-initiative` on disk; Phase 3.4 of the RFC-13 plan renames it to `specify-change`. This page describes the post-Phase-3 surface so it stays accurate after the rename — read every reference to `specify-change` (the umbrella) as `specify-initiative` while you are working on the rfc-13 branch.

## What the registry must NOT own

The registry is topology plus local materialisation. It is **not** a place to park orchestration, validation findings, or PR metadata. Mirror of the [RFC-13 §"Platform components are not capabilities"](../../rfcs/rfc-13-extensibility.md#platform-components-are-not-capabilities) table:

- Change or plan status — owned by `specify change` (`change.md`, `plan.yaml`, `.specify/changes/<name>/.metadata.yaml`).
- Contract relationships beyond the per-project role declarations — owned by the `contracts@v1` capability.
- Validation findings — owned by capability skills and helper binaries.
- Change execution — owned by `specify change` (the orchestration umbrella).
- Capability-specific validation — owned by capability skills and the merge brief.
- PR metadata beyond the local project operation being requested — owned by the forge (GitHub via `gh`); the registry only round-trips per-project status from `gh pr view`.

## See also

- [Capabilities](capabilities/index.md) — capability manifest protocol and the dependency direction sister page.
- [Change Component](change-component.md) — operator brief, plan, execution state, finalization, and archive.
- [Platform Repo Topologies](../explanation/platform-repo.md) — the registry-only hub vs platform-as-project shapes.
- [`specify registry`](cli/registry.md) and [`specify workspace`](cli/workspace.md) — current CLI command reference.
- [RFC-13: Extensibility](../../rfcs/rfc-13-extensibility.md) — capability protocol, platform components, and migration plan.
