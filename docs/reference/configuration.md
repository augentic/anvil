# Configuration Files

Specify 2.0 uses several YAML and Markdown files for configuration. All are managed through the CLI or skills — direct editing is supported for `project.yaml`, `registry.yaml`, and `change.md`, but `.metadata.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, and `targets.yaml` must only be written by the CLI.

## Contents

- [project.yaml](#projectyaml)
- [plan.yaml](#planyaml)
- [registry.yaml](#registryyaml)
- [change.md](#changemd)
- [.metadata.yaml](#metadatayaml)

## project.yaml

**Location:** `.specify/project.yaml`
**Created by:** `/spec:init` (via `specrun init`)
**Edited by:** Operator (directly)

Project-level configuration that persists across changes. Two shapes — the regular project shape (default) and the hub shape (`specrun init --hub`).

### Regular project shape

```yaml
name: my-project
target: https://github.com/augentic/specify/targets/omnia
sources: [intent, documentation, code-typescript]
specify-version: "2.0.0"
workspace: false
domain: |
  Brief description of the project's domain, purpose, and
  technical constraints. This context is available to all
  briefs during artifact generation.
```

| Field             | Required               | Description |
| ----------------- | ---------------------- | ----------- |
| `name`            | Yes                    | Project name (set by `specrun init --name`) |
| `target`          | Yes (regular projects) | Target adapter identifier or URL (with optional `@ref` suffix). Accepts a bare name, an `https://…` URL, or a `file:///…` URI. Omitted on workspaces. |
| `sources`         | No                     | List of source adapters available for `/spec:plan` to bind. Defaults to the first-party set when omitted. |
| `specify-version` | Yes                    | Minimum CLI version required (set by `specrun init`). Kebab-case on disk; the Rust field stays snake_case via `#[serde(rename = "specify-version")]`. |
| `workspace`       | No                     | Absent or `false` for a regular project; `true` for a workspace. |
| `domain`          | No                     | Free-form domain description available to briefs. |

### Hub shape

```yaml
name: shop-platform
workspace: true
specify-version: "2.0.0"
```

A hub is a registry-only platform repo: it holds `registry.yaml`, `change.md`, `plan.yaml`, and `workspace/` slots but is never itself a code project.

| Field             | Required | Description |
| ----------------- | -------- | ----------- |
| `workspace`       | Yes      | `true`. The presence of this flag (paired with the absence of `target:`) is the hub sentinel. |
| `target`          | --       | **Omitted.** A hub has no target — its absence tells the CLI to skip target resolution and the per-project phase pipelines. |

**When to use the registry-only platform hub:** multi-repo platforms, greenfield changes where the topology is itself a design decision, and any setup where the operator wants the platform repo's identity to be unambiguous.

## plan.yaml

**Location:** `.specify/plan.yaml` (single-project) or `<workspace-root>/.specify/plan.yaml` (workspace mode)
**Created by:** `/spec:plan` (via `specrun plan create`)
**Modified by:** `specrun plan add`, `specrun plan amend`, `specrun plan remove`, `specrun plan transition`, `specrun plan next`, `specrun plan archive`

The change's table of contents — an ordered, dependency-aware list of slices, plus the plan lifecycle.

```yaml
version: 1
name: identity-revamp
lifecycle: approved
sources:
  identity-design-notes:
    adapter: documentation
    path: ./design-notes/identity
  legacy-monolith:
    adapter: code-typescript
    path: ./vendor/legacy-monolith
slices:
  - name: identity-user-registration
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        lead: user-registration
      - key: legacy-monolith
        lead: user-registration
    status: pending
  - name: identity-password-reset
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        lead: password-reset
      - key: legacy-monolith
        lead: account-pwd-reset
    divergence: likely
    status: pending
```

| Field (top-level)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `version`                | Yes      | Schema version (currently `1`). |
| `name`                   | Yes      | Change name (kebab-case). |
| `lifecycle`              | Yes      | `pending` or `approved`. Written by `specrun plan transition`; `/spec:plan` exits at `pending`. |
| `sources`                | No       | Map of source-key → `{ adapter, path or value }`. The keys are operator-chosen and referenced by `slices[].sources[].key`. |
| `slices`                 | Yes      | Ordered list of slice entries (see below). |

| Field (per slice)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `name`                   | Yes      | Slice name (kebab-case, unique within the plan). |
| `target`                 | Yes      | Target adapter identifier for the slice (or the plan-level default). |
| `project`                | No       | Workspace project name (workspace mode only). |
| `sources`                | Yes      | List of `{ key, lead }` bindings; cardinality ≥ 1. Bare `<key>` shorthand allowed when the lead id equals the slice's `name`. |
| `status`                 | Yes      | Per-entry status: `pending`, `in-progress`, or `done`. Written exclusively by CLI verbs. |
| `divergence`             | No       | Closed enum: `none` (default; absent), `likely` (set by propose), `accepted` / `rejected` (set by `plan amend --divergence`). Advisory metadata in v1. |
| `depends-on`             | No       | List of slice names that must be `done` first. |
| `context`                | No       | List of baseline paths relevant to the slice; used as a focus hint by briefs. |
| `description`            | No       | What this slice does (human-readable). |

## registry.yaml

**Location:** `registry.yaml`
**Created by:** Operator (directly)
**Validated by:** First-use validators (`specrun workspace sync`, `/spec:plan`)

Workspace catalogue for multi-repo changes. Optional — not needed for single-repo projects.

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    target: omnia@v1
    description: >
      Real-time traffic ingestion and route optimisation.

  - name: command-centre
    url: git@github.com:org/command-centre.git
    target: omnia@v1
    description: >
      Operator dashboard and alerting.

  - name: mobile
    url: ../mobile
    target: vectis@v1
    description: >
      iOS and Android mobile application for field operators.
```

| Field                       | Required    | Description |
| --------------------------- | ----------- | ----------- |
| `version`                   | Yes         | Schema version (currently `1`). |
| `projects[].name`           | Yes         | Project identifier (kebab-case). |
| `projects[].url`            | Yes         | Clone URL or relative path. For local paths, `workspace push` reads `git remote get-url origin` to discover the push target. |
| `projects[].target`         | Yes         | Target adapter identifier or URL for this project. |
| `projects[].description`    | Conditional | Required when multiple projects exist. Describes the project's business domain. |

## change.md

**Location:** `.specify/change.md` (workspace mode: at workspace root)
**Created by:** `/spec:plan` (scaffolded; CLI helper)
**Edited by:** Operator (directly)

Operator-authored brief for a change. Scaffolded at plan time and editable at Gate 1. May carry an optional `## Tentative merges` block (call-outs from propose's uncertain lead reconciliation) and an optional `## Likely divergences` block (side-by-side summaries for materially-disagreeing lead pairs).

```markdown
# Identity revamp

Bring legacy authentication and account-management flows
into Omnia. Priority: user registration, password reset.

## Tentative merges

- `identity-design-notes#user-registration` + `legacy-monolith#account-create`:
  same unit of work? operator to confirm.

## Likely divergences

- `password-reset.expiry`:
  - `identity-design-notes`: 30 minutes
  - `legacy-monolith`: 24 hours
```

## .metadata.yaml

**Location:** `.specify/slices/<name>/.metadata.yaml`
**Created by:** `specrun slice create`
**Modified by:** `specrun slice transition`, `specrun slice merge`

Per-slice lifecycle metadata. **Never hand-edit this file.**

```yaml
status: built
created_at: "2026-05-21T10:30:00Z"
updated_at: "2026-05-21T11:15:00Z"
target: omnia
touched_specs:
  - specs/identity/spec.md
```

| Field             | Description |
| ----------------- | ----------- |
| `status`          | Current slice lifecycle state (`refining`, `refined`, `built`, `merged`, `dropped`). |
| `created_at`      | ISO 8601 creation timestamp. |
| `updated_at`      | ISO 8601 last-transition timestamp. |
| `target`          | Target adapter identifier or URL used for this slice. |
| `touched_specs`   | Spec files this slice affects. |
