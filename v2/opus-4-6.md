# Multi-Repo OPSX: Design Proposal

## Problem

We have a multi-repo platform. Changes often span multiple repos. Today there's no central place to:

1. Know which repos exist and what they contain
2. Store shared OPSX artefacts (schema, templates, config) without duplicating them into every repo
3. Plan a cross-repo change, then distribute and apply specs to each affected repo

## Constraint: Simplest Possible

The design wraps the core OPSX workflow (`/opsx:propose` -> `/opsx:apply` -> `/opsx:archive`) and reuses OPSX artefacts directly. No new tooling, no new commands, no custom orchestrators.

---

## Design

### One Hub Repo, Many Spoke Repos

This repo is the hub. It's the control plane for spec-driven development across the platform.

```
├── registry.toml                 ← which repos exist
├── config.yaml                   ← platform context + rules
├── schemas/
│   └── schema.yaml               ← artifact DAG
├── templates/                    ← artifact templates
├── specs/                        ← source of truth for ALL repos
│   ├── <repo-a>/
│   │   └── <capability>/spec.md
│   └── <repo-b>/
│       └── <capability>/spec.md
└── changes/                      ← standard OPSX changes dir
    ├── <change-name>/
    │   ├── proposal.md
    │   ├── specs/                ← delta specs, namespaced by repo
    │   │   ├── <repo-a>/...
    │   │   └── <repo-b>/...
    │   ├── design.md
    │   ├── manifest.md
    │   └── pipeline.toml
    └── archive/
        └── YYYY-MM-DD-<name>/
```

Spoke repos contain only code. They don't have their own `openspec/` directories. All specs live in the hub.

### The Three Pieces

#### 1. Registry (`registry.toml`)

A flat file listing every repo in the platform:

```toml
[[repos]]
name = "repo-alpha"
url = "git@github.com:augentic/repo-alpha.git"
crates = ["crate-x", "crate-y"]
tags = ["domain-orders"]

[[repos]]
name = "repo-beta"
url = "git@github.com:augentic/repo-beta.git"
crates = ["crate-z"]
tags = ["domain-fulfillment"]
```

Fields:

| Field    | Required | Purpose                                      |
| -------- | -------- | -------------------------------------------- |
| `name`   | yes      | Human-readable identifier, used as namespace |
| `url`    | yes      | Git clone URL                                |
| `crates` | no       | Crate names within the repo (for Rust repos) |
| `tags`   | no       | Domain tags for filtering/grouping           |

The registry is read by the AI during `/opsx:propose` to understand the platform topology and during `/opsx:apply` to locate repos for implementation.

#### 2. Centralised Specs (`specs/`)

Specs are the source of truth for how each repo's capabilities currently behave. They're namespaced by repo name at the top level:

```
specs/
├── repo-alpha/
│   ├── order-ingestion/spec.md
│   └── order-validation/spec.md
└── repo-beta/
    └── fulfillment-dispatch/spec.md
```

Delta specs in changes follow the same namespace:

```
changes/add-priority-orders/specs/
├── repo-alpha/
│   └── order-ingestion/spec.md     ← ADDED/MODIFIED requirements
└── repo-beta/
    └── fulfillment-dispatch/spec.md ← MODIFIED requirements
```

On archive, deltas merge into `specs/` exactly as standard OPSX does -- the only difference is the extra repo-name directory level.

#### 3. Pipeline as Distribution Mechanism

The `pipeline.toml` artifact (already in the schema) maps changes to concrete (repo, crate) targets. This IS the distribution plan. No additional mechanism needed.

```toml
change = "changes/add-priority-orders"

[[targets]]
id = "alpha-ingestion"
repo = "repo-alpha"
repo_url = "git@github.com:augentic/repo-alpha.git"
crate = "crate-x"
project_dir = "crates/crate-x"
specs = ["order-ingestion"]

[[targets]]
id = "beta-dispatch"
repo = "repo-beta"
repo_url = "git@github.com:augentic/repo-beta.git"
crate = "crate-z"
project_dir = "crates/crate-z"
specs = ["fulfillment-dispatch"]
depends_on = ["alpha-ingestion"]

[[dependencies]]
from = "alpha-ingestion"
to = "beta-dispatch"
type = "event-schema"
```

---

## Workflow

The workflow is the standard OPSX core flow, operating from the hub repo:

### `/opsx:propose <change-name>`

The AI:
1. Reads `registry.toml` to know the platform
2. Reads `specs/` for affected repos to understand current state
3. Creates `changes/<change-name>/` with:
   - `proposal.md` -- cross-repo change proposal
   - `specs/<repo>/<capability>/spec.md` -- delta specs per affected repo
   - `design.md` -- technical design spanning repos
   - `manifest.md` -- classified changes per target, with cross-target deps
   - `pipeline.toml` -- execution config mapping targets to repos/crates

All artefacts are standard OPSX artefacts. The schema DAG enforces ordering.

### `/opsx:apply <change-name>`

The AI reads `pipeline.toml` and works through targets in dependency order:

1. For each target: check out the repo, read the relevant delta specs, implement the changes in that repo's codebase
2. Cross-target dependencies determine ordering (e.g., event schema changes in repo-alpha before the consumer in repo-beta)
3. Progress is tracked in the hub's `manifest.md` or `tasks.md`

The apply step is the only part that touches spoke repos. It uses the repo URLs from `pipeline.toml` (sourced from `registry.toml`).

### `/opsx:archive <change-name>`

Standard OPSX archive:
1. Merge delta specs from `changes/<name>/specs/<repo>/...` into `specs/<repo>/...`
2. Move the change folder to `changes/archive/YYYY-MM-DD-<name>/`

After archive, `specs/` reflects the new platform-wide state.

---

## What This Doesn't Do (and Shouldn't)

- **No automated git operations.** The AI checks out repos and pushes branches, but there's no CI/CD orchestrator built into this. That's a separate concern.
- **No per-repo openspec directories.** Spoke repos are just code. Specs live centrally. This avoids sync problems entirely.
- **No schema duplication.** The schema, templates, and config exist once in the hub. Spoke repos don't need OpenSpec installed.
- **No multi-repo locking or coordination.** Changes are sequential per the pipeline dependency graph. If two changes touch the same repo, they're serialised by the human running them (same as single-repo OPSX).

---

## Example: End-to-End

```
1. You: /opsx:propose add-priority-orders

   AI reads registry.toml → sees repo-alpha, repo-beta
   AI reads specs/repo-alpha/order-ingestion/spec.md
   AI reads specs/repo-beta/fulfillment-dispatch/spec.md
   AI creates changes/add-priority-orders/
     ├── proposal.md          (cross-repo: new priority field flows through)
     ├── specs/
     │   ├── repo-alpha/order-ingestion/spec.md   (MODIFIED: priority field)
     │   └── repo-beta/fulfillment-dispatch/spec.md (MODIFIED: priority routing)
     ├── design.md            (event schema change + consumer update)
     ├── manifest.md          (2 targets, medium complexity, event-schema dep)
     └── pipeline.toml        (alpha first, then beta)

2. You: /opsx:apply add-priority-orders

   AI reads pipeline.toml
   Target 1: repo-alpha/crate-x → adds priority field, updates handler
   Target 2: repo-beta/crate-z  → reads new event schema, updates dispatch logic

3. You: /opsx:archive add-priority-orders

   specs/repo-alpha/order-ingestion/spec.md      ← merged
   specs/repo-beta/fulfillment-dispatch/spec.md  ← merged
   changes/archive/2026-03-07-add-priority-orders/ ← preserved
```

---

## Why This Is the Simplest Option

1. **No new abstractions.** Registry is a flat TOML file. Specs use the existing OPSX delta format with one extra directory level. Pipeline is already in the schema.
2. **No duplication.** One schema, one config, one set of templates, one specs directory. Spoke repos have zero OPSX footprint.
3. **No new workflow.** The 3-step OPSX core flow (`propose` / `apply` / `archive`) works unchanged. The AI just reads more context (registry + multi-repo specs) during propose.
4. **Incremental.** Start with the registry and a few specs. Add repos and specs as they're touched by real changes. No big-bang migration.
