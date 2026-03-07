# MVP Multi-Repo OpenSpec: Revised Proposal

## Design Principles

1. **Centralised planning, distributed execution** — all thinking happens in one repo; all code happens where it belongs.
2. **Specs live with the code** — each target repo owns its `openspec/specs/` as the local source of truth.
3. **Wrap OPSX, don't replace it** — the platform workflow is a thin coordination layer around `/opsx:propose`, `/opsx:apply`, and `/opsx:archive`.
4. **Reviewable from one place** — `proposal.md`, `design.md`, and `tasks.md` contain enough per-repo detail that reviewers never need to visit target repos to approve.
5. **PR as final gate** — distributed execution produces PRs. Merging is the approval.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        HUB REPO (lifecycle)                         │
│                                                                     │
│  registry.toml           ← what repos/services exist                │
│  openspec/config.yaml    ← platform context + rules                 │
│  openspec/schemas/       ← custom schema + templates                │
│  openspec/changes/       ← all active cross-repo changes            │
│                                                                     │
│  The hub is the only repo with OPSX schema/templates/config.        │
│  It owns the planning artefacts. It delegates execution.            │
│                                                                     │
└────────────────┬──────────────────┬─────────────────────────────────┘
                 │                  │
        ┌────────▼──────┐   ┌──────▼────────┐
        │  TARGET REPO  │   │  TARGET REPO  │   ...
        │               │   │               │
        │  openspec/    │   │  openspec/    │
        │    specs/     │   │    specs/     │
        │               │   │               │
        │  src/...      │   │  src/...      │
        │               │   │               │
        │  No schema.   │   │  No schema.   │
        │  No templates.│   │  No templates.│
        │  No config.   │   │  No config.   │
        │  Just specs.  │   │  Just specs.  │
        └───────────────┘   └───────────────┘
```

Target repos carry minimal OPSX footprint: just `openspec/specs/` with one `spec.md` per capability. No schema, no templates, no config, no changes directory. The hub handles all of that.

---

## Hub Repo Structure

```
lifecycle/
├── registry.toml
├── domains/                          # optional domain docs
│   ├── train/README.md
│   ├── traffic/README.md
│   └── vessel/README.md
└── openspec/
    ├── config.yaml
    ├── schemas/
    │   └── platform/
    │       ├── schema.yaml
    │       └── templates/
    │           ├── proposal.md
    │           ├── specs.md
    │           ├── design.md
    │           └── tasks.md
    └── changes/
        ├── <change-name>/
        │   ├── proposal.md           # cross-repo why + what
        │   ├── specs/                # delta specs namespaced by service
        │   │   └── <service-id>/
        │   │       └── <capability>/spec.md
        │   ├── design.md             # cross-repo technical design
        │   ├── tasks.md              # implementation tasks grouped by repo
        │   └── pipeline.toml         # execution plan
        └── archive/
            └── YYYY-MM-DD-<name>/
```

## Target Repo Structure

```
<repo>/
├── openspec/
│   └── specs/
│       └── <capability>/spec.md      # source of truth for this service
└── src/...
```

That's it. Zero OPSX overhead beyond the specs themselves.

---

## Problem 1: Central Registry

The unit of registration is a **service** (crate), not a repo, because multiple services can share a repo. The unit of execution (branch, PR) is a **repo**.

```toml
# registry.toml

[[services]]
id = "r9k-connector"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-connector"
domain = "train"
capabilities = ["r9k-xml-ingest"]

[[services]]
id = "r9k-adapter"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-adapter"
domain = "train"
capabilities = ["r9k-xml-to-smartrak-gtfs"]

[[services]]
id = "tomtom"
repo = "git@github.com:wasm-replatform/traffic.git"
project_dir = "."
crate = "tomtom"
domain = "traffic"
capabilities = ["tomtom-flow-ingest", "tomtom-incidents-ingest", "tomtom-route-monitoring"]
```

| Field | Purpose |
|-------|---------|
| `id` | Stable service identifier, used as namespace in specs and pipeline |
| `repo` | Git clone URL — multiple services can share one repo |
| `project_dir` | Where the crate lives within the repo |
| `crate` | Rust crate name |
| `domain` | Domain grouping for discovery |
| `capabilities` | What the service does — used during impact analysis |

The registry is read during `/opsx:propose` for platform topology and during execution for repo URLs.

---

## Problem 2: Single Source of Truth for OPSX Artefacts

The hub is the only place that stores config, schema, and templates. Target repos don't need them.

### Custom Schema

`openspec/schemas/platform/schema.yaml`:

```yaml
name: platform
version: 1
description: Multi-repo platform planning schema

artifacts:
  - id: proposal
    generates: proposal.md
    requires: []

  - id: specs
    generates: specs/**/*.md
    requires: [proposal]

  - id: design
    generates: design.md
    requires: [proposal]

  - id: tasks
    generates: tasks.md
    requires: [specs, design]

apply:
  requires: [tasks]
  tracks: tasks.md
```

This is the stock `spec-driven` DAG. The differentiation is in the templates, not the graph. The templates instruct the AI to:

- Read `registry.toml` for platform topology
- Namespace delta specs under `specs/<service-id>/<capability>/`
- Write `design.md` with per-service sections containing enough implementation detail for centralised review
- Write `tasks.md` with tasks grouped by repo, each task referencing the relevant delta specs
- Produce `pipeline.toml` as a companion artefact (convention, not schema-tracked)

### Config

`openspec/config.yaml`:

```yaml
schema: platform

context: |
  Platform: Rust WASM (wasm32-wasip2) on Omnia SDK
  Architecture: Handler<P> pattern with provider trait bounds
  Registry: registry.toml contains all services, repos, and capabilities
  Spec location: each target repo has openspec/specs/<capability>/spec.md
  Multi-repo: changes may span services across multiple repos

rules:
  proposal:
    - Read registry.toml to discover platform topology
    - Identify all affected services and repos
    - Write the proposal with enough detail that reviewers approve from this repo
  specs:
    - Namespace delta specs under specs/<service-id>/<capability>/
    - Use ADDED/MODIFIED/REMOVED sections
    - One spec file per capability
  design:
    - Include a per-service section for each impacted service
    - Document domain model changes with field-level detail
    - Document API/event contract changes with request/response shapes
    - Flag cross-service dependencies (event schemas, shared types)
  tasks:
    - Group tasks under "## Repo: <repo-name>" headers
    - Within each repo section, group by service/crate
    - Each task references the relevant delta spec requirement
    - Tasks must be concrete enough that an AI agent can implement them
```

One set of artefacts, version-controlled in the hub, never duplicated. If you later want OpenSpec installed in target repos for standalone changes, you can sync config/schemas/templates via PRs from the hub. But for MVP, the hub is the only OPSX installation.

---

## Problem 3: Distributing and Applying Specs

The **change artefacts** in the hub are the distribution mechanism. The **pipeline.toml** is the execution plan. The **branch + PR** is the delivery vehicle.

### Pipeline

`pipeline.toml` (convention, lives in the change folder):

```toml
change = "changes/r9k-http"

[[targets]]
id = "r9k-connector"
repo = "git@github.com:wasm-replatform/train.git"
crate = "r9k-connector"
project_dir = "."
specs = ["r9k-xml-ingest"]
branch = "opsx/r9k-http"

[[targets]]
id = "r9k-adapter"
repo = "git@github.com:wasm-replatform/train.git"
crate = "r9k-adapter"
project_dir = "."
specs = ["r9k-xml-to-smartrak-gtfs"]
branch = "opsx/r9k-http"
depends_on = ["r9k-connector"]

[[dependencies]]
from = "r9k-connector"
to = "r9k-adapter"
type = "event-schema"
```

When multiple services share a repo (as above), they share a branch and a PR. The pipeline tracks dependencies at the service level; execution groups by repo.

---

## Workflow

### Step 1: `/opsx:propose <change-name>` — Centralised Planning

Run in the hub repo. The AI:

1. Reads `registry.toml` to discover services, repos, domains, capabilities
2. Reads `openspec/specs/` from affected target repos to understand current behaviour (via local filesystem if repos are co-located, or by cloning)
3. Reads `domains/` for domain context if present
4. Creates `changes/<change-name>/` with all artefacts:

   - **`proposal.md`** — problem, scope, approach. Per-affected-service summary. Rich enough that a reviewer understands the full picture without opening any target repo.
   - **`specs/<service-id>/<capability>/spec.md`** — delta specs per capability per service. ADDED/MODIFIED/REMOVED sections.
   - **`design.md`** — technical design per service: domain model changes, API/event contract changes, handler changes, provider capability changes. Mermaid diagrams for cross-service flows.
   - **`tasks.md`** — implementation tasks grouped by repo, then by service/crate within each repo. Each task is a checkbox with a concrete description and a reference to the relevant delta spec requirement.
   - **`pipeline.toml`** — execution config.

5. Commit to a branch in the hub. Open a PR.

### Step 2: Review — Centralised Approval

Reviewers read the hub PR. They see:

- **Why** (proposal.md)
- **What's changing** (delta specs)
- **How** (design.md)
- **What work** (tasks.md)
- **What order** (pipeline.toml)

They approve or request changes. All review happens in this one PR in the hub repo. No need to visit target repos.

When the hub PR merges, the change is approved for execution.

### Step 3: Distributed Execution

For each repo in `pipeline.toml`, in dependency order:

1. **Branch** — create `opsx/<change-name>` from the default branch
2. **Implement** — the AI reads the hub's `design.md` (the section for this repo) and `tasks.md` (the tasks for this repo) and the relevant delta specs. It implements the code changes in the target repo.
3. **Update specs** — merge the delta spec content into the target repo's `openspec/specs/<capability>/spec.md`. The spec files in the target repo now reflect the new behaviour. This happens as part of the same commit/branch.
4. **PR** — open a PR titled `opsx: <change-name>`. The PR description links back to the hub's change folder for context and includes a summary of what changed.

The PR includes both code changes AND updated specs. Reviewers see the final state. When the PR merges, the target repo's specs and code are consistent.

#### What "executing /opsx: commands" looks like in practice

You don't need the OpenSpec CLI installed in target repos. The platform wrapper in the hub reads the artefacts and dispatches work. But the conceptual model is the same OPSX flow:

| Platform Wrapper | Equivalent OPSX Command | Where |
|---|---|---|
| `platform:propose` | `/opsx:propose` | Hub |
| `platform:apply` | `/opsx:apply` (per repo) | Target repos |
| `platform:archive` | `/opsx:archive` | Hub + target repos |

If you later install OpenSpec in target repos, you could scaffold `openspec/changes/<change-name>/` in each target repo with the relevant delta specs and tasks, then use the actual `/opsx:apply` command. But for MVP, the wrapper achieves the same outcome.

### Step 4: `/opsx:archive <change-name>` — Centralised Archival

After all target-repo PRs are merged:

1. Verify all targets are complete (all PRs merged)
2. In the hub, move `changes/<change-name>/` to `changes/archive/YYYY-MM-DD-<change-name>/`
3. Commit to hub

The hub's archive preserves the full planning context. The target repos' `openspec/specs/` reflect the new state (updated during the PR).

---

## Example: End-to-End

```
1. You: /opsx:propose r9k-http
   ═══════════════════════════════════════════════════════
   Hub reads registry.toml → finds r9k-connector, r9k-adapter (train repo)
   Hub reads train repo's openspec/specs/r9k-xml-ingest/spec.md
   Hub reads train repo's openspec/specs/r9k-xml-to-smartrak-gtfs/spec.md

   Hub creates changes/r9k-http/
     ├── proposal.md
     │     "Migrate R9K from SOAP/XML to HTTP/JSON ingest.
     │      Affects r9k-connector (ingest handler) and
     │      r9k-adapter (transformation pipeline)."
     ├── specs/
     │   ├── r9k-connector/r9k-xml-ingest/spec.md
     │   │     MODIFIED: new HTTP/JSON endpoint alongside SOAP
     │   │     ADDED: JSON schema validation
     │   └── r9k-adapter/r9k-xml-to-smartrak-gtfs/spec.md
     │         MODIFIED: accept both XML and JSON input formats
     ├── design.md
     │     Per-service technical design with handler changes,
     │     type changes, provider changes, test changes
     ├── tasks.md
     │     ## Repo: train
     │     ### Service: r9k-connector
     │     - [ ] 1.1 Add JSON request handler alongside XML
     │     - [ ] 1.2 Add JSON schema validation
     │     ### Service: r9k-adapter
     │     - [ ] 2.1 Accept JSON input format
     │     - [ ] 2.2 Update transformation pipeline
     └── pipeline.toml
           Target 1: r9k-connector (specs: r9k-xml-ingest)
           Target 2: r9k-adapter (specs: r9k-xml-to-smartrak-gtfs)
           Dependency: r9k-connector → r9k-adapter (event-schema)

2. Review: Hub PR reviewed and merged. Change approved.

3. Distributed execution:
   ═══════════════════════════════════════════════════════
   In train repo:
     Branch: opsx/r9k-http
     Implement: code changes for r9k-connector and r9k-adapter
     Update specs: merge deltas into openspec/specs/
     PR: "opsx: r9k-http" → reviewed → merged

4. You: /opsx:archive r9k-http
   ═══════════════════════════════════════════════════════
   Hub: changes/r9k-http/ → changes/archive/2026-03-08-r9k-http/
```

---

## Comparison with Previous Proposal (v2/opus-4-6.md)

| Aspect | Previous Proposal | This Proposal |
|---|---|---|
| **Spec location** | Centralised in hub `specs/` | Distributed in target repos `openspec/specs/` |
| **Target repo footprint** | Zero (no openspec dir) | Minimal (`openspec/specs/` only) |
| **Spec sync** | No sync needed (all central) | Hub reads during propose, target repos update during PRs |
| **Review model** | Review in hub (same) | Review in hub (same) |
| **Archive** | Merge deltas into hub's `specs/` | Merge deltas into target repo's `specs/`; archive change folder in hub |
| **Spec drift risk** | None (single location) | Low — specs update as part of the same PR as code changes |
| **Standalone changes** | Impossible (no specs in target repos) | Possible — target repos can evolve specs independently |

The key tradeoff: the previous proposal was simpler (no spec sync) but kept specs away from the code. This proposal honours the requirement that specs live with the code at the cost of reading target repos during propose and updating them during execution.

---

## What This Doesn't Do (and Shouldn't — MVP)

- **No orchestration engine.** No daemon, no CI bot, no automated branch creation. The wrapper is human-driven or AI-assisted in a coding session.
- **No cross-repo spec validation.** The hub doesn't verify that target repo specs are up to date before proposing. Trust the process; validate later.
- **No per-repo OpenSpec CLI.** Target repos don't run `/opsx:*` commands directly. The hub dispatches.
- **No parallel execution.** Work through the pipeline sequentially. Parallel execution is a growth-path optimisation.
- **No config/template sync.** Target repos don't have schemas or templates. If they need standalone OPSX later, add it then.
- **No automated PR creation.** The AI (or human) creates PRs manually. Automation is a growth-path feature.

---

## Growth Path

| When | Add |
|---|---|
| After first pilot | `domains/` docs with event contracts and shared types |
| After second pilot | Simple shell script to automate branch + PR creation from `pipeline.toml` |
| When standalone changes are needed | Install OpenSpec in target repos with synced config/templates |
| When team grows | Status dashboard reading `pipeline.toml` + GitHub PR status |
| When velocity is high | Parallel execution across independent targets |
| When contracts matter | Cross-repo spec validation (hub reads all target specs and checks consistency) |

---

## Where to Start

1. **Create the hub repo** with `registry.toml`, `openspec/config.yaml`, and the `platform` schema with templates.
2. **Add `openspec/specs/` to 2-3 target repos** — start with repos that have existing v1 specs or that are being actively changed.
3. **Run one pilot end-to-end.** Pick a real change. Propose in the hub. Review. Execute in target repos. Archive. Learn what's awkward.
4. **Iterate.** The design is deliberately minimal so that every piece can be changed independently.
