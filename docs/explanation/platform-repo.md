# Platform Repo Topologies

A **platform repo** is the repository an operator opens when they sit down to drive a Specify change across one or more projects. It owns `registry.yaml` (the catalogue of every project in scope), `change.md` (the operator-authored brief), `plan.yaml` (the dependency-aware slice list), and `.specify/workspace/` (the durable clones [`/change:execute`](../../plugins/change/skills/execute/SKILL.md) writes into). Platform-level state lives here; per-project code lives in registered project repos.

There are two valid shapes for a platform repo. Specify supports both, but the **registry-only hub** is canonical -- it is what `specify init --hub` scaffolds, what tutorials use as their reference shape, and what the platform-first vision composes against. The other shape -- **platform-as-project** -- remains valid for single-repo and small-team cases. This page explains the difference, the on-disk shape of each, and the validation invariant that keeps the two from being silently mixed.

```d2
direction: right

hub: "Registry-only hub" {
  shape: rectangle
  hubProj: "project.yaml\n{hub: true}" {shape: page}
  hubReg: "registry.yaml\n[peer-a, peer-b]" {shape: page}
  hubInit: "change.md" {shape: page}
  hubPlan: "plan.yaml" {shape: page}
  hubWs: "workspace/" {shape: cylinder}
  peerA: "workspace/peer-a/\n(clone of peer-a)" {shape: cylinder}
  peerB: "workspace/peer-b/\n(clone of peer-b)" {shape: cylinder}
  hubWs -> peerA
  hubWs -> peerB
}

pap: "Platform-as-project" {
  shape: rectangle
  papProj: "project.yaml\n{capability: omnia@v1}" {shape: page}
  papReg: "registry.yaml\n[my-app (url: .), peer-b]" {shape: page}
  papChanges: "changes/" {shape: cylinder}
  papSpecs: "specs/" {shape: cylinder}
  papWs: "workspace/" {shape: cylinder}
  papSelf: "workspace/my-app/\n(symlink to .)" {shape: cylinder}
  papPeer: "workspace/peer-b/\n(clone of peer-b)" {shape: cylinder}
  papWs -> papSelf
  papWs -> papPeer
}
```

## The registry-only hub (canonical)

In the hub topology, the platform repo is **never itself a code project**. It holds platform state and routes work to peer repos materialised under `.specify/workspace/<name>/`. Operator-facing platform artifacts (`registry.yaml`, `change.md`, `plan.yaml`) live at the repo root; framework-managed scratch (`project.yaml`, archive, workspace clones) lives under `.specify/`.

```text
platform-repo/
├── registry.yaml         # version: 1, projects: [<peer>, <peer>, …]
├── change.md         # operator brief (per-change)
├── plan.yaml             # dependency-aware change list (per-change)
└── .specify/
    ├── project.yaml      # { hub: true, … }   -- `capability:` is omitted on a hub
    ├── archive/
    │   └── plans/        # finalised changes
    └── workspace/
        └── <peer>/       # one durable clone per registry entry
```

A single marker identifies a hub:

- `project.yaml:hub: true` -- the hub sentinel. Its presence (paired with the **absence** of `capability:`) is what disables capability resolution and the per-project phase pipelines (define / build / merge), so the hub never runs `/spec:define` or `/spec:build` against its own working tree. The same flag flips `Registry::validate_shape` into hub-only mode, which rejects any registry entry whose `url` is `.`. See [RFC-13 §Migration "Hub project shape"](../../rfcs/archive/rfc-13-extensibility.md#migration) — the legacy `schema: hub` sentinel is removed in the same release that lands the capability rename, so post-cut-over hubs carry only `hub: true`.

The hub never appears in its own `registry.yaml`. Code projects always live in their own repos -- they are referenced by the registry's `projects[]` list and materialised under `.specify/workspace/<name>/` by `specify workspace sync`.

**When to choose the hub.** Multi-repo platforms, greenfield changes where the topology is itself a design decision, and any setup where the operator wants the platform repo's identity to be unambiguous. The hub is the recommended starting shape for new platform-first changes.

## The platform-as-project shape (still permitted)

In the platform-as-project topology, the initiating repo is **both** the platform repo and a code project. The repo's own registry entry uses `url: .` to mark itself. Operator-facing platform artifacts sit alongside the project's own source at the repo root; framework-managed scratch lives under `.specify/`.

```text
my-app/
├── src/                  # actual application code
├── registry.yaml         # projects: [{ name: my-app, url: . }, …]
├── change.md         # (optional)
├── plan.yaml             # (optional)
└── .specify/
    ├── project.yaml      # { capability: omnia@v1, … }   -- a real capability
    └── slices/           # active slices for this project
```

The `url: .` entry tells `specify workspace sync` to materialise the platform repo as its own workspace slot via a symlink. Phase pipelines run normally because `project.yaml:capability:` resolves to a real capability manifest. `project.yaml:hub` is absent (or `false`).

**When to choose platform-as-project.** Single-repo projects, small teams that have not factored their codebase into multiple repos, and migrations where peeling code out into a separate platform repo is itself unnecessary churn. The platform-first vision still works in this shape -- the operator just runs `/change:plan`, `/change:execute`, and `workspace push` against the same repo they edit code in.

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
specify init --hub --name <kebab-name>
```

`--hub` is the discriminator; **no positional argument** is passed in hub mode. Combining a capability positional with `--hub` is rejected with the diagnostic `init-requires-capability-or-hub` -- the same error you get if you pass neither. A hub does not have a capability.

The command writes:

- `.specify/project.yaml` with `hub: true`, the kebab-cased name, and a current `specify-version` floor. **`capability:` is omitted** -- the absence of the field is what tells the CLI to disable capability resolution. The `rules:` block is also omitted; a hub has no phase pipelines to scaffold.
- `registry.yaml` with `version: 1` and `projects: []`. Hub-mode validation runs against this seed; populating the registry happens via `specify registry add` (RFC-9 §2A) or by hand-editing.
- `.gitignore` upserts for `.specify/.cache/` and `.specify/workspace/`.

`change.md` and `plan.yaml` are not created by `specify init --hub`; `specify change create` and `specify change plan create` mint them when a specific change begins. The command **refuses** when `.specify/` already exists. This is deliberate: flipping an existing single-repo project into a hub would clobber `project.yaml`. Operators who genuinely want to convert remove `.specify/` first.

For the platform-as-project shape, use the regular `specify init <capability>` form (no `--hub` flag) where `<capability>` is a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI. The CLI rejects `specify init` with neither a capability positional nor `--hub` -- exactly one of the two is required, never both. See [`specify init`](../reference/cli/init.md) for the full flag surface and the [`/spec:init`](../../plugins/spec/skills/init/SKILL.md) skill for the agent-driven wrapper that prompts for project metadata.

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- end-to-end worked example that bootstraps a hub via `specify init --hub` and drives a change across two registered projects.
- [The Layered Stack](three-layer-stack.md) -- where the platform repo fits in the layered model.
- [Workspace Tiers](workspace-tiers.md) -- the legacy-source vs registered-project clone distinction the hub relies on.
- [`specify init`](../reference/cli/init.md) -- CLI reference for the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) -- CLI reference for `validate` (hub-mode dispatch) and `show`.
