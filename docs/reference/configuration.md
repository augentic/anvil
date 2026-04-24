# Configuration Files

Specify uses several YAML and Markdown files for configuration. All are managed through the CLI or skills -- direct editing is supported for `project.yaml`, `registry.yaml`, and `initiative.md`, but `.metadata.yaml` and `plan.yaml` should only be written by the CLI.

## project.yaml

**Location:** `.specify/project.yaml`
**Created by:** `/spec:init` (via `specify init`)
**Edited by:** Operator (directly)

Project-level configuration that persists across changes.

```yaml
schema: https://github.com/augentic/specify/schemas/omnia
domain: |
  Brief description of the project's domain, purpose, and
  technical constraints. This context is available to all
  briefs during artifact generation.
rules:
  - "All APIs must require authentication"
  - "Error responses must include a correlation ID"
```

| Field | Required | Description |
|-------|----------|-------------|
| `schema` | Yes | Schema URL (with optional `@ref` suffix) |
| `domain` | No | Free-form domain description available to briefs |
| `rules` | No | Project-level constraints the agent should respect |

## plan.yaml

**Location:** `.specify/plan.yaml`
**Created by:** `/spec:plan` (via `specify plan init` + `specify plan create`)
**Modified by:** `specify plan amend`, `specify plan transition`

The initiative's table of contents -- an ordered, dependency-aware list of changes.

```yaml
name: platform-v2

sources:
  monolith: /path/to/legacy-codebase
  docs: /path/to/documentation

changes:
  - name: extract-auth
    description: "Extract authentication capabilities from the monolith"
    depends-on: []
    sources: [monolith]
    affects: [auth]
    status: done
    project: api

  - name: add-oauth
    description: "Add OAuth2 provider integration"
    depends-on: [extract-auth]
    affects: [auth]
    status: pending
    project: api

  - name: auth-ui
    description: "Login and registration screens"
    depends-on: [add-oauth]
    affects: [auth-ui]
    status: pending
    project: mobile
```

| Field (top-level) | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Initiative name (kebab-case) |
| `sources` | No | Named source paths (key=path) for legacy code or documentation |

| Field (per entry) | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Change name (kebab-case, unique within the plan) |
| `description` | Yes | What this change does |
| `depends-on` | No | List of change names that must be `done` first |
| `sources` | No | List of source keys from the top-level `sources` |
| `affects` | No | List of spec/capability names this change touches |
| `status` | Yes | Current state: `pending`, `in-progress`, `done`, `failed`, `blocked`, `skipped` |
| `status-reason` | No | Explanation for non-happy-path status |
| `project` | No | Registry project name (multi-repo only, see RFC-3b) |

## registry.yaml

**Location:** `.specify/registry.yaml`
**Created by:** Operator (directly)
**Validated by:** `specify initiative registry validate`

Platform catalogue for multi-repo initiatives. Optional -- not needed for single-repo projects.

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1
    description: >
      Real-time traffic ingestion and route optimisation.
      Owns Kafka consumers, the routing engine, and the
      traffic-state read model.

  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1
    description: >
      Operator dashboard and alerting. Owns the web UI,
      notification dispatch, and escalation workflows.

  - name: mobile
    url: ../mobile
    schema: vectis@v1
    description: >
      iOS and Android mobile application for field operators.
```

| Field | Required | Description |
|-------|----------|-------------|
| `version` | Yes | Schema version (currently `1`) |
| `projects[].name` | Yes | Project identifier (kebab-case) |
| `projects[].url` | Yes | Clone URL or relative path. For local paths, `workspace push` reads `git remote get-url origin` to discover the push target. |
| `projects[].schema` | Yes | Schema URL for this project |
| `projects[].description` | Conditional | Required when multiple projects exist. Describes the project's business domain. |

## initiative.md

**Location:** `.specify/initiative.md`
**Created by:** `specify initiative brief init`
**Edited by:** Operator (directly)

Operator-authored brief for an initiative. Contains YAML frontmatter for structured inputs and a prose body for intent.

```markdown
---
name: platform-modernisation
inputs:
  - key: monolith
    path: /path/to/legacy
    kind: legacy-code
  - key: prd
    path: ./docs/prd.md
    kind: documentation
---

Modernise the traffic platform from the legacy Node.js monolith
to Omnia-based microservices. Priority is the auth and routing
subsystems. Notifications can follow in a later cycle.
```

| Frontmatter field | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Initiative name |
| `inputs[].key` | Yes | Source key (referenced in plan sources) |
| `inputs[].path` | Yes | Filesystem path to the input |
| `inputs[].kind` | Yes | `legacy-code` or `documentation` |

The prose body describes the operator's intent. It is read by `/spec:plan` during the propose phase.

## .metadata.yaml

**Location:** `.specify/changes/<name>/.metadata.yaml`
**Created by:** `specify change create`
**Modified by:** `specify change transition`, `specify change phase-outcome`

Per-change lifecycle metadata. **Never hand-edit this file.**

```yaml
status: building
created_at: "2026-04-24T10:30:00Z"
updated_at: "2026-04-24T11:15:00Z"
schema: https://github.com/augentic/specify/schemas/omnia
touched_specs:
  - specs/greeting/spec.md
outcome: null
```

| Field | Description |
|-------|-------------|
| `status` | Current lifecycle state |
| `created_at` | ISO 8601 creation timestamp |
| `updated_at` | ISO 8601 last-transition timestamp |
| `schema` | Schema URL used for this change |
| `touched_specs` | Spec files this change affects |
| `outcome` | Phase outcome: `success`, `failure`, `deferred`, or `null` |
