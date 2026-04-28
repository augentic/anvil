# Platform Repo Topologies

A **platform repo** is the repository an operator opens when they sit down to drive a Specify initiative. It owns `.specify/registry.yaml` (the catalogue of every project in scope), `.specify/initiative.md` (the operator-authored brief), `.specify/plan.yaml` (the dependency-aware change list), and `.specify/workspace/` (the durable clones [`/spec:execute`](../../plugins/spec/skills/execute/SKILL.md) writes into). Platform-level state lives here; per-project code lives in registered project repos.

There are two valid shapes for a platform repo. Specify supports both, but the **registry-only hub** is canonical -- it is what `specify init --hub` scaffolds, what tutorials use as their reference shape, and what the platform-first vision composes against. The other shape -- **platform-as-project** -- remains valid for single-repo and small-team cases. This page explains the difference, the on-disk shape of each, and the validation invariant that keeps the two from being silently mixed.

## The registry-only hub (canonical)

In the hub topology, the platform repo is **never itself a code project**. It holds platform state and routes work to peer repos materialised under `.specify/workspace/<name>/`.

```text
platform-repo/
└── .specify/
    ├── project.yaml      # { schema: hub, hub: true, … }
    ├── registry.yaml     # version: 1, projects: [<peer>, <peer>, …]
    ├── initiative.md     # operator brief (per-initiative)
    ├── plan.yaml         # dependency-aware change list (per-initiative)
    ├── archive/
    │   └── plans/        # finalised initiatives
    └── workspace/
        └── <peer>/       # one durable clone per registry entry
```

Two markers identify a hub:

- `project.yaml:schema: hub` -- the schema-resolution sentinel. Phase pipelines (define / build / merge) are disabled on the hub itself; the hub never runs `/spec:define` or `/spec:build` against its own working tree.
- `project.yaml:hub: true` -- the validation flag. When set, `Registry::validate_shape` runs in hub-only mode and rejects any registry entry whose `url` is `.`.

The hub never appears in its own `registry.yaml`. Code projects always live in their own repos -- they are referenced by the registry's `projects[]` list and materialised under `.specify/workspace/<name>/` by `specify workspace sync`.

**When to choose the hub.** Multi-repo platforms, greenfield initiatives where the topology is itself a design decision, and any setup where the operator wants the platform repo's identity to be unambiguous. The hub is the recommended starting shape for new platform-first initiatives.

## The platform-as-project shape (still permitted)

In the platform-as-project topology, the initiating repo is **both** the platform repo and a code project. The repo's own registry entry uses `url: .` to mark itself.

```text
my-app/
├── src/                  # actual application code
└── .specify/
    ├── project.yaml      # { schema: omnia@v1, … }   -- a real schema
    ├── registry.yaml     # projects: [{ name: my-app, url: . }, …]
    ├── initiative.md     # (optional)
    ├── plan.yaml         # (optional)
    └── changes/          # active changes for this project
```

The `url: .` entry tells `specify workspace sync` to materialise the platform repo as its own workspace slot via a symlink. Phase pipelines run normally because `project.yaml:schema:` resolves to a real schema. `project.yaml:hub` is absent (or `false`).

**When to choose platform-as-project.** Single-repo projects, small teams that have not factored their codebase into multiple repos, and migrations where peeling code out into a separate platform repo is itself unnecessary churn. The platform-first vision still works in this shape -- the operator just runs `/spec:plan`, `/spec:execute`, and `workspace push` against the same repo they edit code in.

## Validation rules

The two topologies are not interchangeable, and the framework enforces the boundary at registry-validation time:

| Mode | Trigger | Accepts `url: .` entry? | Diagnostic when violated |
|------|---------|-------------------------|--------------------------|
| **Plain (default)** | `project.yaml:hub` absent or `false` | Yes (the platform-as-project marker) | -- |
| **Hub-only** | `project.yaml:hub: true` | No (rejected with `hub-cannot-be-project`) | `registry.yaml: projects[<idx>] (<name>).url is `.`; a registry-only platform hub must not appear in its own registry — code projects always live in their own repos (hub-cannot-be-project)` |

Hub-only mode is opt-in. Non-hub callers continue to use the base `validate_shape` unchanged (additive API). The CLI verbs that wire up the hub-mode check today are:

- `specify init --hub` -- runs hub-mode validation against the seed `version: 1, projects: []` registry it writes (trivially passes; the wiring exists so future writes inherit the same invariant).
- `specify registry validate` -- reads `project.yaml:hub` and dispatches to hub-mode validation when set, so a hub repo that gets a hand-edited `url: .` entry fails loud on the next `validate`.

The diagnostic always carries the stable code `hub-cannot-be-project`, alongside the offending `projects[idx]` and project name, so tooling can match on the code without parsing prose.

## Scaffolding

Use `specify init --hub` to scaffold the canonical hub shape:

```bash
specify init hub --schema-dir . --name <kebab-name> --hub
```

(The first positional argument is the `schema` value -- ignored in hub mode but still required by the parser. `hub` is a convenient placeholder; any value works.)

The command writes:

- `.specify/project.yaml` with `schema: hub`, `hub: true`, the kebab-cased name, and a current `specify-version` floor. The `rules:` block is omitted -- a hub has no phase pipelines to scaffold.
- `.specify/registry.yaml` with `version: 1` and `projects: []`. Hub-mode validation runs against this seed; populating the registry happens via `specify registry add` (RFC-9 §2A) or by hand-editing.
- `.specify/initiative.md` from the canonical template, named after the project. The brief is per-initiative -- subsequent initiatives overwrite it via `specify initiative create`.
- `.gitignore` upserts for `.specify/.cache/` and `.specify/workspace/`.

The command **refuses** when `.specify/` already exists. This is deliberate: flipping an existing single-repo project into a hub would clobber `project.yaml`. Operators who genuinely want to convert remove `.specify/` first.

For the platform-as-project shape, use the regular `specify init <schema>` (no `--hub` flag). See [`specify init`](../reference/cli/init.md) for the full flag surface and the [`/spec:init`](../../plugins/spec/skills/init/SKILL.md) skill for the agent-driven wrapper that populates the schema cache and prompts for project metadata.

## See also

- [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) -- end-to-end worked example that bootstraps a hub via `specify init --hub` and drives an initiative across two registered projects.
- [The Three-Layer Stack](three-layer-stack.md) -- where the platform repo fits in the layered model.
- [Workspace Tiers](workspace-tiers.md) -- the legacy-source vs registered-project clone distinction the hub relies on.
- [`specify init`](../reference/cli/init.md) -- CLI reference for the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) -- CLI reference for `validate` (hub-mode dispatch) and `show`.
