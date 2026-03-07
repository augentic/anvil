# MVP Multi-Repo OpenSpec Implementation (Clean Slate)

## Goals

- Keep OpenSpec specs with the code they apply to (inside each target repo).
- Start with a simple model that can be iterated and extended over time.
- Wrap the simple OPSX workflow and reuse OPSX artifacts where practical:
  - `/opsx:propose`
  - `/opsx:apply`
  - `/opsx:archive`

## OpenSpec Foundations This Design Preserves

- Each repo keeps its own `openspec/specs/` as its local source of truth.
- Change work is done in `openspec/changes/<change-id>/` with standard artifacts:
  - `proposal.md`
  - `specs/**/spec.md` (delta specs)
  - `design.md`
  - `tasks.md`
- Archive merges delta specs into that repo's `openspec/specs/` and moves change folders to archive.
- No attempt is made to centralize runtime spec ownership across repos; centralization is only for orchestration and template distribution.

## MVP Architecture

Use three pieces only:

1. **Platform control repo** (new): `platform-opsx`
2. **Target code repos** (existing product/service repos)
3. **CI/bot jobs** (optional, can start as local scripts)

This keeps all behavior transparent and Git-native.

## Problem 1: Central Registry of Repos

Create a single registry file in `platform-opsx`:

`registry/repos.yaml`

```yaml
version: 1
repos:
  - id: service-a
    url: git@github.com:org/service-a.git
    default_branch: main
    owner_team: team-a
    active: true
  - id: service-b
    url: git@github.com:org/service-b.git
    default_branch: main
    owner_team: team-b
    active: true
```

MVP behavior:

- Registry is the only place that defines which repos participate.
- Scripts consume this file for fan-out operations.
- Git history on this file becomes the audit trail of platform scope.

## Problem 2: Single Source of Truth for OPSX Artifacts

Use `platform-opsx` as the canonical artifact pack source:

```text
artifacts/openspec/
├── config.base.yaml
├── schemas/
│   └── spec-driven/
│       ├── schema.yaml
│       └── templates/
│           ├── proposal.md
│           ├── spec.md
│           ├── design.md
│           └── tasks.md
```

Rules:

- Canonical files are edited only here.
- Target repos receive managed copies via PRs.
- Each target repo may have a small local overlay for repo-specific context (for example, stack details in `openspec/config.yaml`), but schema/templates stay centrally managed in MVP.

## Problem 3: Distributing and Applying Specs Across Impacted Repos

Introduce a central change manifest per multi-repo initiative:

`changes/<change-id>/manifest.yaml`

```yaml
change_id: r9k-http
title: XML ingest and GTFS transformation
status: proposed
impacted_repos:
  - service-a
  - service-b
artifacts_version: 2026-03-08
rollout:
  order: [service-a, service-b]
  strategy: parallel
```

MVP behavior:

- `dispatch` script reads manifest + registry.
- It creates/scaffolds `openspec/changes/<change-id>/` in each impacted repo.
- It seeds standard OPSX artifacts and capability delta specs as needed.
- Implementation is done in each repo with normal `/opsx:apply`.
- Completion is tracked per repo; central archive only happens when all required repos are archived.

## Proposed Repository Layouts

### Platform repo (`platform-opsx`)

```text
platform-opsx/
├── registry/
│   └── repos.yaml
├── artifacts/
│   └── openspec/
│       ├── config.base.yaml
│       └── schemas/spec-driven/...
├── changes/
│   └── <change-id>/
│       ├── manifest.yaml
│       ├── proposal.md        # optional central narrative
│       ├── design.md          # optional cross-repo design
│       └── tasks.md           # optional orchestration tasks
└── scripts/
    ├── sync-artifacts
    ├── dispatch-change
    ├── status-change
    └── archive-change
```

### Target repo (`service-a`, `service-b`, ...)

```text
<repo>/
└── openspec/
    ├── config.yaml
    ├── schemas/spec-driven/...          # managed copy from platform
    ├── specs/...                        # local source of truth for this repo
    └── changes/
        └── <change-id>/
            ├── proposal.md
            ├── design.md
            ├── tasks.md
            └── specs/**/spec.md         # deltas for this repo only
```

## Wrapper Workflow (Simple and Familiar)

Keep your preferred lifecycle but wrap it at platform level:

### 1) `opsx-platform propose <change-id>`

- Creates central `changes/<change-id>/manifest.yaml`.
- Selects impacted repos.
- Scaffolds `openspec/changes/<change-id>/` in each impacted repo.
- Opens PRs to add per-repo artifacts.

### 2) `opsx-platform apply <change-id>`

- Aggregates status across repos.
- Surfaces which repos are blocked/in progress/done.
- Actual implementation remains native: each repo uses `/opsx:apply`.

### 3) `opsx-platform archive <change-id>`

- Verifies each impacted repo has completed and archived its local change.
- Marks central manifest archived.
- Optionally moves central change folder to `changes/archive/`.

## Why This Is a Good MVP

- **Simple:** YAML files + scripts + PR workflow.
- **Low risk:** no custom server, no database, no custom merge engine.
- **OpenSpec-aligned:** local `specs/` truth is preserved by design.
- **Incremental:** can add automation later without breaking data model.

## Operating Rules for MVP

1. Specs never move out of target repos.
2. Central repo governs only registry, shared artifacts, and orchestration metadata.
3. All distribution happens through normal Git commits/PRs.
4. Use built-in `spec-driven` schema first; customize only after one successful pilot.
5. Use one shared `change-id` across all repos for traceability.

## Immediate Implementation Plan (Week 1)

1. Create `platform-opsx` with `registry/`, `artifacts/`, `changes/`, `scripts/`.
2. Add 2-3 pilot repos to `registry/repos.yaml`.
3. Copy baseline OpenSpec schema/templates into `artifacts/openspec/`.
4. Implement `sync-artifacts` script to open artifact-sync PRs in pilot repos.
5. Implement `dispatch-change` script for one cross-repo change.
6. Run one real pilot end-to-end:
   - `opsx-platform propose`
   - per-repo `/opsx:apply`
   - per-repo `/opsx:archive`
   - `opsx-platform archive`

## Practical Next Iterations (Post-MVP)

- Add repo dependency metadata in registry (upstream/downstream impact hints).
- Add automatic impacted-repo suggestions from changed capabilities.
- Add required `verify` checks before central archive.
- Version the artifact pack and pin versions per target repo.
- Add dashboards/metrics once process is stable.

## Final Recommendation

Start with a **Git-native control plane** plus **repo-local OpenSpec truth**.  
This gives you immediate multi-repo coordination while staying faithful to OpenSpec's model and keeping complexity low enough to learn quickly.
