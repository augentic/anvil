# Configuration Files

Specify uses several YAML and Markdown files for configuration. All are managed through the CLI or skills -- direct editing is supported for `project.yaml`, `registry.yaml`, and `change.md`, but `.metadata.yaml` and `plan.yaml` should only be written by the CLI.

## Contents

- [project.yaml](#projectyaml)
- [plan.yaml](#planyaml)
- [registry.yaml](#registryyaml)
- [change.md](#changemd)
- [.metadata.yaml](#metadatayaml)

## project.yaml

**Location:** `.specify/project.yaml`
**Created by:** `/spec:init` (via `specify init`)
**Edited by:** Operator (directly)

Project-level configuration that persists across changes. Two shapes -- the regular project shape (default) and the hub shape (`specify init --hub`).

### Regular project shape

```yaml
name: my-project
adapter: https://github.com/augentic/specify/adapters/omnia
specify_version: "0.1.0"
domain: |
  Brief description of the project's domain, purpose, and
  technical constraints. This context is available to all
  briefs during artifact generation.
rules:
  proposal: "Focus on user-facing changes"
  specs: "All APIs must require authentication"
  design: "Error responses must include a correlation ID"
  tasks: ""
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Project name (set by `specify init --name`) |
| `adapter` | Yes (regular projects) | Adapter identifier or URL (with optional `@ref` suffix). Accepts a bare name, an `https://…` URL, or a `file:///…` URI. Omitted on hubs. |
| `specify_version` | Yes | Minimum CLI version required (set by `specify init`, updated by `--upgrade`) |
| `domain` | No | Free-form domain description available to briefs |
| `rules` | No | Per-brief rule overrides keyed by brief ID (e.g. `proposal`, `specs`, `composition`, `design`, `tasks`). Empty values mean no rules apply. Scaffolded by `specify init` with one entry per `pipeline.define` brief. The Vectis adapter includes a `composition` entry for the screen layout brief. |
| `hub` | No | Absent or `false` for a regular project (the platform-as-project shape uses `url: .` in `registry.yaml` instead). |

### Hub shape

```yaml
name: shop-platform
hub: true
specify_version: "0.24.2"
```

A hub is a registry-only platform repo: it holds `registry.yaml`, `change.md`, `plan.yaml`, and `workspace/` but is never itself a code project. The single marker above identifies a hub:

| Field | Required | Description |
|-------|----------|-------------|
| `hub` | Yes | `true`. The presence of this flag (paired with the **absence** of `adapter:`) is the hub sentinel — it disables adapter resolution on the hub and triggers `Registry::validate_shape` to reject `url: .` entries with `hub-cannot-be-project`. |
| `adapter` | -- | **Omitted.** A hub has no adapter — its absence is what tells the CLI to skip adapter resolution and the per-project phase pipelines. |
| `rules` | -- | Omitted -- a hub has no phase pipelines to scaffold. |

**When to use the hub shape:** multi-repo platforms, greenfield changes where the topology is itself a design decision, and any setup where the operator wants the platform repo's identity to be unambiguous. See [Platform repo topologies](../explanation/platform-repo.md) for the full contract and the hub vs platform-as-project comparison.

## plan.yaml

**Location:** `plan.yaml`
**Created by:** `/change:draft` (via `specify change draft` + `specify plan add`)
**Modified by:** `specify plan amend`, `specify plan transition`

The change's table of contents -- an ordered, dependency-aware list of slices.

```yaml
name: platform-v2

sources:
  monolith: /path/to/legacy-codebase
  docs: /path/to/documentation

changes:
  - name: extract-auth
    description: "Extract authentication adapters from the monolith"
    depends-on: []
    sources: [monolith]
    status: done
    project: api

  - name: auth-api-contract
    adapter: contracts@v1
    description: "Define the auth API contract"
    depends-on: [extract-auth]
    status: pending

  - name: add-oauth
    description: "Add OAuth2 provider integration"
    depends-on: [extract-auth, auth-api-contract]
    context:
      - contracts/http/auth-api.yaml
      - contracts/schemas/oauth-token.yaml
    status: pending
    project: api

  - name: auth-ui
    description: "Login and registration screens"
    depends-on: [auth-api-contract]
    context:
      - contracts/http/auth-api.yaml
    status: pending
    project: mobile
```

| Field (top-level) | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Change name (kebab-case) |
| `sources` | No | Named source paths (key=path) for legacy code or documentation |

| Field (per entry) | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Slice name (kebab-case, unique within the plan) |
| `description` | Yes | What this slice does |
| `depends-on` | No | List of slice names that must be `done` first |
| `sources` | No | List of source keys from the top-level `sources` |
| `status` | Yes | Current state: `pending`, `in-progress`, `done`, `failed`, `blocked`, `skipped` |
| `adapter` | No | Plan-entry adapter identifier for project-less entries (e.g. `contracts@v1`). Required when `project` is absent. The `adapter:` key on a `plan.yaml` entry identifies the artefact-path identifier the entry targets, not the adapter that owns the work. |
| `context` | No | List of baseline paths (relative to `.specify/`) relevant to this slice. Used by briefs as a focus hint when scanning baseline directories. |
| `status-reason` | No | Explanation for non-happy-path status |
| `project` | No | Registry project name (multi-repo only). Each entry must have at least one of `project` or `adapter`. |

## registry.yaml

**Location:** `registry.yaml`
**Created by:** Operator (directly)
**Validated by:** `specify registry validate`

Platform catalogue for multi-repo changes. Optional -- not needed for single-repo projects.

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    adapter: omnia@v1
    description: >
      Real-time traffic ingestion and route optimisation.
      Owns Kafka consumers, the routing engine, and the
      traffic-state read model.

  - name: command-centre
    url: git@github.com:org/command-centre.git
    adapter: omnia@v1
    description: >
      Operator dashboard and alerting. Owns the web UI,
      notification dispatch, and escalation workflows.

  - name: mobile
    url: ../mobile
    adapter: vectis@v1
    description: >
      iOS and Android mobile application for field operators.
```

| Field | Required | Description |
|-------|----------|-------------|
| `version` | Yes | Schema version (currently `1`) |
| `projects[].name` | Yes | Project identifier (kebab-case) |
| `projects[].url` | Yes | Clone URL or relative path. For local paths, `workspace push` reads `git remote get-url origin` to discover the push target. |
| `projects[].adapter` | Yes | Adapter identifier or URL for this project. The YAML key is spelled `adapter:`; the value is a adapter identifier. |
| `projects[].description` | Conditional | Required when multiple projects exist. Describes the project's business domain. |

## change.md

**Location:** `change.md`
**Created by:** `specify slice create`
**Edited by:** Operator (directly)

Operator-authored brief for a slice. Contains YAML frontmatter for structured inputs and a prose body for intent.

```markdown
---
name: platform-modernisation
inputs:
  - path: /path/to/legacy
    kind: legacy-code
  - path: ./docs/prd.md
    kind: documentation
---

Modernise the traffic platform from the legacy Node.js monolith
to Omnia-based microservices. Priority is the auth and routing
subsystems. Notifications can follow in a later cycle.
```

| Frontmatter field | Required | Description |
|-------------------|----------|-------------|
| `name` | Yes | Change name (kebab-case) |
| `inputs[].path` | Yes | Filesystem path to the input |
| `inputs[].kind` | Yes | `legacy-code` or `documentation` |

The prose body describes the operator's intent. It is read by `/change:draft` during the propose phase.

## .metadata.yaml

**Location:** `.specify/slices/<name>/.metadata.yaml`
**Created by:** `specify slice create`
**Modified by:** `specify slice transition`, `specify slice outcome set`

Per-slice lifecycle metadata. **Never hand-edit this file.**

```yaml
status: building
created_at: "2026-04-24T10:30:00Z"
updated_at: "2026-04-24T11:15:00Z"
adapter: https://github.com/augentic/specify/adapters/omnia
touched_specs:
  - specs/greeting/spec.md
outcome: null
```

| Field | Description |
|-------|-------------|
| `status` | Current lifecycle state |
| `created_at` | ISO 8601 creation timestamp |
| `updated_at` | ISO 8601 last-transition timestamp |
| `adapter` | Adapter identifier or URL used for this slice. The YAML key is spelled `adapter:`; the value is a adapter identifier. |
| `touched_specs` | Spec files this slice affects |
| `outcome` | Phase outcome: `success`, `failure`, `deferred`, or `null` |
