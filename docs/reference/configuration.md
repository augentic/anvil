# Configuration Files

Emery uses several YAML and Markdown files for configuration. All are managed through the CLI or skills — direct editing is supported for `project.yaml` and `change.md`, but `metadata.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, and `targets.yaml` must only be written by the CLI.

## Contents

- [project.yaml](#projectyaml)
- [plan.yaml](#planyaml)
- [change.md](#changemd)
- [metadata.yaml](#metadatayaml)

## project.yaml

**Location:** `.emery/project.yaml`
**Created by:** `/emery:init` (via `emery init`)
**Edited by:** Operator (directly)

Project-level configuration that persists across changes.

```yaml
name: my-project
target: omnia                 # bare name; or a pin such as emery:omnia@1.0.0
sources: [intent, documentation, typescript]
emery-version: "2.0.0"
description: |
  Brief description of the project's domain, purpose, and
  technical constraints. This context is available to all
  briefs during artifact generation.
```

| Field             | Required               | Description |
| ----------------- | ---------------------- | ----------- |
| `name`            | Yes                    | Project name (set by `emery init --name`) |
| `target`          | Yes                    | Target adapter identifier or URL (with optional `@ref` suffix). Accepts a bare name, an `https://…` URL, or a `file:///…` URI. |
| `sources`         | No                     | List of source adapters available for `/emery:plan` to bind. Defaults to the first-party set when omitted. |
| `emery-version` | Yes                    | Minimum CLI version required (set by `emery init`). Kebab-case on disk; the Rust field stays snake_case via `#[serde(rename = "emery-version")]`. |
| `gap-policy`      | No                     | Standing gap policy for every `emery plan execute` epoch: `strict` (open gaps block build) or `defer` (open gaps are dispositioned into durable deferral facts at the build gate). Absent means `strict`. Written by `emery init --gap-policy`, preserved by `init --upgrade`, overridable per epoch with `emery plan execute --gap-policy`. |
| `description`     | No                     | Free-form project description (tech stack, architecture, testing) available to briefs. This is the only *authored* identity field; routing identity is otherwise *derived* — see below. |

A project's routing identity (the `surface[]` of owned domains and a `recent[]` merge tail surfaced in the reconciliation `projects[]`) is **derived**, not authored — projected from the project's baseline (`.emery/specs/` requirement titles + the `.emery/events/<writer>.jsonl` outcome ledger). The earlier hand-authored `capabilities` / `keywords` facets are removed; a stale `capabilities:` / `keywords:` key in an existing `project.yaml` is silently ignored.

## plan.yaml

**Location:** `plan.yaml` at the project root
**Created by:** `/emery:plan` (via `emery plan author`'s scaffold leg)
**Modified by:** `emery plan author`, `emery plan add`, `emery plan amend`, `emery plan remove`, `emery plan archive`

The change's table of contents — an ordered, dependency-aware list of slices.

```yaml
version: 1
name: identity-revamp
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
  - name: identity-password-reset
    project: identity-svc
    sources:
      - source: identity-design-notes
        lead: password-reset
      - source: legacy-monolith
        lead: account-pwd-reset
    divergence: likely
```

| Field (top-level)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `version`                | Yes      | Schema version (currently `1`). |
| `name`                   | Yes      | Change name (kebab-case). |
| `sources`                | No       | Map of source → `{ adapter, path or value, cid? }`. The keys are operator-chosen and referenced by `slices[].sources[].source`. After author, each binding carries a closed tree `cid`. |
| `slices`                 | Yes      | Ordered list of slice entries (see below). |

| Field (per slice)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `name`                   | Yes      | Slice name (kebab-case, unique within the plan). |
| `project`                | No       | Project this slice binds (an omitted value resolves to the sole topology project). The target adapter is resolved on demand from this project — it is not stored per slice. |
| `sources`                | Yes      | List of `{ source, lead }` bindings; cardinality ≥ 1. Bare `<source>` shorthand allowed when the lead id equals the slice's `name`. |
| `divergence`             | No       | Closed enum: `none` (default; absent), `likely` / `accepted` / `rejected` — all set by `emery plan amend <entry> --divergence`, staged after `propose --from` since slices do not exist until it runs. Advisory metadata in v1. |
| `depends-on`             | No       | List of slice names that must project `done` first. |
| `allow-composition-replace` | No    | `false` by default. Set via `emery plan amend <entry> --allow-composition-replace true`; authorizes a whole-document composition to overwrite a non-empty baseline when the execute loop merges this slice. |
| `context`                | No       | List of baseline paths relevant to the slice; used as a focus hint by briefs. |
| `description`            | No       | What this slice does (human-readable). |

## change.md

**Location:** `change.md` at the project root
**Created by:** `/emery:plan` (scaffolded; CLI helper)
**Edited by:** Operator (directly)

Operator-authored brief for a change. Scaffolded at plan time and editable during plan review. May carry an optional `## Tentative merges` block (call-outs from propose's uncertain lead reconciliation), an optional `## Cross-cutting leads` block (leads multi-homed across several slices, each listed with its member slices), and an optional `## Likely divergences` block (side-by-side summaries for materially-disagreeing lead pairs).

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

**Location:** `.emery/slices/<name>/metadata.yaml`
**Created by:** the execute loop's refine orchestration (slice create is re-entry safe)
**Modified by:** the refine / build / merge orchestrations inside `emery plan execute`, and `emery plan drop`

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
