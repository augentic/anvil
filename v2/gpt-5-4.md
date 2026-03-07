# MVP Multi-Repo OpenSpec Proposal

## Recommendation

Keep OpenSpec fully inside each target repo, and add one very thin coordination repo around it.

This matches how OpenSpec is actually documented:

- `openspec/specs/` is the source of truth for one project
- `openspec/changes/` are project-local change folders
- `/opsx:archive` merges delta specs back into that same repo's `openspec/specs/`

So the simplest multi-repo design is not to make OpenSpec itself global. It is to let OpenSpec remain repo-local, and add a small wrapper for cross-repo coordination.

## Goals

Use the simple `OPSX` workflow:

```text
/opsx:propose -> /opsx:apply -> /opsx:archive
```

while meeting these constraints:

- keep specs with the code they apply to
- stay simple enough to understand immediately
- avoid a heavyweight orchestrator or control plane
- allow gradual growth over time
- reuse OpenSpec and OPSX artifacts where practical

## Core Model

Use two layers:

1. A central control repo for coordination
2. Standard OpenSpec inside each target repo

### Control Repo

Suggested name:

```text
platform-openspec
```

Example layout:

```text
platform-openspec/
├── registry/
│   ├── repos.yaml
│   └── capabilities.yaml
├── profile/
│   ├── openspec-version.txt
│   └── config.base.yaml
├── changes/
│   └── <change-name>/
│       └── manifest.yaml
└── commands/
    ├── platform-propose.md
    ├── platform-apply.md
    └── platform-archive.md
```

### Target Repos

Each target repo remains a normal OpenSpec project:

```text
customer-api/
└── openspec/
    ├── config.yaml
    ├── specs/
    └── changes/

admin-web/
└── openspec/
    ├── config.yaml
    ├── specs/
    └── changes/
```

This is the key design choice. Each repo owns its own specs, changes, and archive lifecycle.

## Why This Is The Best MVP

This approach aligns with OpenSpec's documented model:

- specs are the source of truth for current behavior
- changes are isolated folders with local artifacts
- archive merges local delta specs into local main specs
- schemas and templates resolve at the project level

That means the lowest-friction multi-repo design is:

- do not centralize all specs in one place
- do not try to make archive merge into a global spec tree
- do not duplicate OpenSpec into a second orchestration layer

Instead:

- keep OpenSpec local
- make coordination global

## Problem 1: Central Registry Of Repos

Use a git-backed YAML registry in the control repo:

```text
registry/repos.yaml
```

Example:

```yaml
repos:
  - id: customer-api
    git: git@github.com:your-org/customer-api.git
    local_path: ../customer-api
    default_branch: main
    owners:
      - identity
    capabilities:
      - customer-profile
      - customer-lifecycle

  - id: admin-web
    git: git@github.com:your-org/admin-web.git
    local_path: ../admin-web
    default_branch: main
    owners:
      - admin
    capabilities:
      - admin-users
      - customer-search
```

### Registry Rules

- `id` is the stable identifier used everywhere
- `git` is the clone URL
- `local_path` is the expected workspace checkout location
- `default_branch` is the branch wrappers start from
- `owners` is optional metadata for routing and review
- `capabilities` gives simple first-pass impact analysis

No database is required. The registry is versioned, code-reviewed, and easy to evolve.

### Optional Capability Map

If you want better impact discovery, add:

```text
registry/capabilities.yaml
```

Example:

```yaml
capabilities:
  customer-deactivation:
    repos:
      - customer-api
      - admin-web
      - billing-worker
```

This stays intentionally simple: a hand-maintained lookup, not an inference engine.

## Problem 2: Single Source Of Truth For OPSX Artifacts

The MVP answer is:

- pin one OpenSpec CLI version across repos
- keep one shared base config in the control repo
- generate repo-local `openspec/config.yaml`
- use the stock `spec-driven` schema first

### Shared Control-Repo Assets

Keep these in the control repo:

- `profile/openspec-version.txt`
- `profile/config.base.yaml`

Example `profile/config.base.yaml`:

```yaml
schema: spec-driven

rules:
  proposal:
    - Keep scope tight and explicit
    - Identify impacted repos
  specs:
    - Prefer behavior changes over implementation detail
  design:
    - Call out cross-repo dependencies
  tasks:
    - Group tasks by repo-local delivery order
```

### Repo-Local Materialization

Each target repo gets its own generated `openspec/config.yaml`, combining shared defaults with repo-specific context:

```yaml
schema: spec-driven

context: |
  Repo: customer-api
  Domain: customer lifecycle and profile APIs
  Key constraints:
  - Backwards compatibility for public endpoints
  - PostgreSQL is source of record

rules:
  proposal:
    - Keep scope tight and explicit
    - Identify impacted repos
  specs:
    - Prefer behavior changes over implementation detail
  design:
    - Call out cross-repo dependencies
  tasks:
    - Group tasks by repo-local delivery order
```

### Important Constraint

OpenSpec does not support config includes. So for the MVP, the practical way to achieve a single source of truth is not runtime inheritance. It is generated local config from one shared base.

### Schema Recommendation

Do not fork or customize the schema on day one.

Start with the upstream `spec-driven` schema:

```text
proposal -> specs -> design -> tasks
```

Only fork `schema.yaml` and templates once you have real repeated pain points.

That keeps the MVP understandable and close to upstream OpenSpec.

## Problem 3: Distributing And Applying Specs To Impacted Repos

Use one central coordination manifest per cross-repo change, and the same change slug in each impacted repo.

### Central Manifest

```text
platform-openspec/changes/<change-name>/manifest.yaml
```

Example:

```yaml
change: add-customer-deactivation
summary: Add customer deactivation flow across API, admin UI, and billing worker

repos:
  - repo: customer-api
    status: proposed
    capabilities:
      - customer-lifecycle

  - repo: admin-web
    status: proposed
    capabilities:
      - admin-users

  - repo: billing-worker
    status: proposed
    capabilities:
      - billing-events
```

### Repo Distribution Model

Distribute by creating the same change slug in each impacted repo:

```text
customer-api/openspec/changes/add-customer-deactivation/
admin-web/openspec/changes/add-customer-deactivation/
billing-worker/openspec/changes/add-customer-deactivation/
```

Each repo then owns its own:

- `proposal.md`
- delta specs under `specs/`
- `design.md`
- `tasks.md`

This preserves OpenSpec's native model while giving the platform change one shared coordination identity.

## Wrapped OPSX Workflow

The wrapper should be thin. It should coordinate standard local OPSX usage, not replace it.

## `platform-propose`

Responsibilities:

1. Read the repo registry
2. Suggest impacted repos based on repo metadata and capabilities
3. Confirm the repo list with a human
4. Create the central `manifest.yaml`
5. Scaffold the same change slug in each impacted repo
6. Hand off to repo-local OpenSpec planning

Recommended local flow per impacted repo:

```text
/opsx:propose <change-name>
```

### Output

- one central manifest in the control repo
- one same-slug OpenSpec change folder in each impacted repo
- normal repo-local OpenSpec planning artifacts

## `platform-apply`

Responsibilities:

1. Read `manifest.yaml`
2. Work repo-by-repo
3. In each repo, run normal local apply
4. Update central status after each repo completes

Recommended local flow per impacted repo:

```text
/opsx:apply <change-name>
```

### Recommended V1 Behavior

Apply sequentially, not in parallel.

Sequential execution is easier to understand, easier to recover, and a better fit for dependencies between repos.

## `platform-archive`

Responsibilities:

1. Check the manifest to confirm all impacted repos are complete
2. Archive each repo-local change using standard OpenSpec
3. Mark each repo archived in the manifest
4. Mark the platform change complete when all repos are archived

Recommended local flow per impacted repo:

```text
/opsx:archive <change-name>
```

This preserves OpenSpec's normal archive semantics:

- each repo merges its delta specs into its own `openspec/specs/`
- each repo keeps its own archived history

## Recommended MVP

Build only these pieces:

1. One control repo: `platform-openspec`
2. One repo registry: `registry/repos.yaml`
3. Optional capability map: `registry/capabilities.yaml`
4. One shared base config: `profile/config.base.yaml`
5. Generated repo-local `openspec/config.yaml`
6. One central manifest per platform change
7. Same-slug local OpenSpec changes in each impacted repo
8. Sequential repo-by-repo `propose -> apply -> archive`

## Explicit Non-Goals For V1

- no global `openspec/specs/` source of truth for the whole platform
- no centralized archive merge into one repo
- no custom OpenSpec schema unless and until needed
- no shared template distribution mechanism beyond generated local config
- no service-based registry or orchestration plane
- no parallel cross-repo apply engine
- no full automatic impact analysis without human confirmation

## Summary

The simplest viable multi-repo OpenSpec design is:

- one thin control repo for coordination
- normal OpenSpec inside each target repo
- one git-backed registry of repositories
- one shared base config
- one central manifest per cross-repo change
- the same change slug created in every impacted repo
- standard local `OPSX` lifecycle inside each repo

This keeps specs with the code they govern, stays close to upstream OpenSpec, and gives you a clean path to iterate later without redesigning the whole system.
