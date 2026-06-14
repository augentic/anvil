# Configuration Files

Specify uses several YAML and Markdown files for configuration. All are managed through the CLI or skills — direct editing is supported for `project.yaml`, `registry.yaml`, and `change.md`, but `metadata.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, and `targets.yaml` must only be written by the CLI.

## Contents

- [project.yaml](#projectyaml)
- [plan.yaml](#planyaml)
- [registry.yaml](#registryyaml)
- [change.md](#changemd)
- [metadata.yaml](#metadatayaml)

## project.yaml

**Location:** `.specify/project.yaml`
**Created by:** `/spec:init` (via `specify init`)
**Edited by:** Operator (directly)

Project-level configuration that persists across changes. Two shapes — the regular project shape (default) and the workspace shape (`specify init --workspace`).

### Regular project shape

```yaml
name: my-project
target: https://github.com/augentic/specify/targets/omnia
sources: [intent, documentation, typescript]
specify-version: "2.0.0"
workspace: false
description: |
  Brief description of the project's domain, purpose, and
  technical constraints. This context is available to all
  briefs during artifact generation.
```

| Field             | Required               | Description |
| ----------------- | ---------------------- | ----------- |
| `name`            | Yes                    | Project name (set by `specify init --name`) |
| `target`          | Yes (regular projects) | Target adapter identifier or URL (with optional `@ref` suffix). Accepts a bare name, an `https://…` URL, or a `file:///…` URI. Omitted on workspaces. |
| `sources`         | No                     | List of source adapters available for `/spec:plan` to bind. Defaults to the first-party set when omitted. |
| `specify-version` | Yes                    | Minimum CLI version required (set by `specify init`). Kebab-case on disk; the Rust field stays snake_case via `#[serde(rename = "specify-version")]`. |
| `workspace`       | No                     | Absent or `false` for a regular project; `true` for a workspace. |
| `description`     | No                     | Free-form project description (tech stack, architecture, testing) available to briefs. This is the only *authored* identity field; routing identity is otherwise *derived* — see below. |

A project's routing identity (the `surface[]` of owned domains and a `recent[]` merge tail surfaced in the reconciliation `projects[]`) is **derived**, not authored: `specify workspace sync` projects it deterministically from the project's own baseline (`.specify/specs/` requirement titles + the `.specify/journal.jsonl` outcome ledger) into `.specify/topology.lock`. The earlier hand-authored `capabilities` / `keywords` facets are removed; a stale `capabilities:` / `keywords:` key in an existing `project.yaml` is silently ignored.

### Workspace shape

```yaml
name: platform
workspace: true
specify-version: "2.0.0"
```

A workspace is a registry-only platform repo: it holds `registry.yaml`, `change.md`, `plan.yaml`, and workspace slots under top-level `workspace/` but is never itself a code project.

| Field             | Required | Description |
| ----------------- | -------- | ----------- |
| `workspace`       | Yes      | `true`. The presence of this flag (paired with the absence of `target:`) is the workspace sentinel. |
| `target`          | --       | **Omitted.** A workspace has no target — its absence tells the CLI to skip target resolution and the per-project phase pipelines. |

**When to use the registry-only workspace:** multi-repo platforms, greenfield changes where the topology is itself a design decision, and any setup where the operator wants the platform repo's identity to be unambiguous.

## plan.yaml

**Location:** `plan.yaml` at the project root (single-project) or `<workspace>/plan.yaml` (workspace mode)
**Created by:** `/spec:plan` (via `specify plan create`)
**Modified by:** `specify plan propose --from`, `specify plan add`, `specify plan amend`, `specify plan remove`, `specify plan transition`, `specify plan next`, `specify plan archive`

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
    adapter: typescript
    path: ./vendor/legacy-monolith
slices:
  - name: identity-user-registration
    project: identity-svc
    sources:
      - source: identity-design-notes
        lead: user-registration
      - source: legacy-monolith
        lead: user-registration
    status: pending
  - name: identity-password-reset
    project: identity-svc
    sources:
      - source: identity-design-notes
        lead: password-reset
      - source: legacy-monolith
        lead: account-pwd-reset
    divergence: likely
    status: pending
```

| Field (top-level)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `version`                | Yes      | Schema version (currently `1`). |
| `name`                   | Yes      | Change name (kebab-case). |
| `lifecycle`              | Yes      | `pending` or `approved`. Written by `specify plan transition`; `/spec:plan` exits at `pending`. |
| `sources`                | No       | Map of source → `{ adapter, path or value }`. The keys are operator-chosen and referenced by `slices[].sources[].source`. |
| `slices`                 | Yes      | Ordered list of slice entries (see below). |

| Field (per slice)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `name`                   | Yes      | Slice name (kebab-case, unique within the plan). |
| `project`                | No       | Project this slice binds. Required when the registry declares multiple projects; optional for single-project setups (an omitted value resolves to the sole topology project). The target adapter is resolved on demand from this project — it is not stored per slice. |
| `sources`                | Yes      | List of `{ source, lead }` bindings; cardinality ≥ 1. Bare `<source>` shorthand allowed when the lead id equals the slice's `name`. |
| `status`                 | Yes      | Per-entry status: `pending`, `in-progress`, or `done`. Written exclusively by CLI verbs. |
| `divergence`             | No       | Closed enum: `none` (default; absent), `likely` / `accepted` / `rejected` — all set by `specify plan amend <entry> --divergence`, staged after `propose --from` since slices do not exist until it runs. Advisory metadata in v1. |
| `depends-on`             | No       | List of slice names that must be `done` first. |
| `context`                | No       | List of baseline paths relevant to the slice; used as a focus hint by briefs. |
| `description`            | No       | What this slice does (human-readable). |

## registry.yaml

**Location:** `registry.yaml`
**Created by:** Operator (directly)
**Validated by:** First-use validators (`specify workspace sync`, `/spec:plan`)

Workspace membership + location ledger for multi-repo changes. Optional — not needed for single-repo projects. It carries only `name` + `url` (plus optional `contracts` wiring and an optional greenfield `adapter` seed); a project's `description` is authored in its own `.specify/project.yaml`, and its derived identity (`surface[]` / `recent[]`) is projected into `.specify/topology.lock` from that project's baseline.

```yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    adapter: omnia@v1        # optional greenfield scaffold seed only
  - name: command-centre
    url: git@github.com:org/command-centre.git
  - name: mobile
    url: ../mobile
```

| Field                       | Required    | Description |
| --------------------------- | ----------- | ----------- |
| `version`                   | Yes         | Schema version (currently `1`). |
| `projects[].name`           | Yes         | Project identifier (kebab-case). The slot name and the `plan.yaml.slices[].project` binding key. |
| `projects[].url`            | Yes         | Clone URL or relative path. For local paths, `workspace push` reads `git remote get-url origin` to discover the push target. |
| `projects[].adapter`        | No          | Greenfield scaffold seed only — written into a new project's `project.yaml` when `workspace sync` clones an empty repo. Not read for plan-time topology. |
| `projects[].contracts`      | No          | Per-project contract role declarations (`produces`, `consumes`). |

## change.md

**Location:** `change.md` at the project root (workspace mode: at workspace root)
**Created by:** `/spec:plan` (scaffolded; CLI helper)
**Edited by:** Operator (directly)

Operator-authored brief for a change. Scaffolded at plan time and editable at Gate 1. May carry an optional `## Tentative merges` block (call-outs from propose's uncertain lead reconciliation), an optional `## Cross-cutting leads` block (leads multi-homed across several slices, each listed with its member slices), and an optional `## Likely divergences` block (side-by-side summaries for materially-disagreeing lead pairs).

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

## metadata.yaml

**Location:** `.specify/slices/<name>/metadata.yaml`
**Created by:** `specify slice create`
**Modified by:** `specify slice transition`, `specify slice merge`

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
