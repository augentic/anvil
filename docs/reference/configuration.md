# Configuration Files

Emery uses several YAML and Markdown files for configuration. Direct editing is supported for `project.yaml` and `change.md`. `metadata.yaml`, `plan.yaml`, `discovery.yaml`, `leads.md`, `decomposition.yaml`, `sources.yaml`, and `targets.yaml` must only be written by the CLI.

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
| `description`     | No                     | Free-form project description (tech stack, architecture, testing) available to briefs. This is the only *authored* identity field; routing identity is otherwise *derived* — see below. |

A project's routing identity (the `surface[]` of owned domains and a `recent[]` merge tail surfaced in the reconciliation `projects[]`) is **derived**, not authored — projected from the project's baseline (`.emery/specs/` requirement titles + the `.emery/change/events/<writer>.jsonl` outcome ledger). The earlier hand-authored `capabilities` / `keywords` facets are removed; a stale `capabilities:` / `keywords:` key in an existing `project.yaml` is silently ignored.

## plan.yaml

**Location:** `.emery/change/plan.yaml`
**Created by:** `/emery:plan` (via `emery plan author`'s scaffold leg)
**Modified by:** `emery plan author`, `emery plan add`, `emery plan amend`, `emery plan remove`, `emery plan archive`

The change's table of contents — an ordered, dependency-aware list of slices.

```yaml
name: identity-revamp
discovery-digest: sha256:…
leads-digest: sha256:…
decomposition-digest: sha256:…
targets:
  identity-svc:
    adapter: emery:omnia@0.12.0
    locator: "."
    cid: sha256:…
sources:
  intent:
    adapter: emery:intent@0.12.0
    value: "Bring legacy authentication into Omnia."
  identity-design-notes:
    adapter: emery:documentation@0.12.0
    locator: ./design-notes/identity
    cid: sha256:…
  legacy-monolith:
    adapter: emery:typescript@0.12.0
    locator: ./vendor/legacy-monolith
    cid: sha256:…
slices:
  - name: identity-user-registration
    target: identity-svc
    sources:
      - source: identity-design-notes
        lead: user-registration
      - source: legacy-monolith
        lead: user-registration
  - name: identity-password-reset
    target: identity-svc
    sources:
      - source: identity-design-notes
        lead: password-reset
      - source: legacy-monolith
        lead: account-pwd-reset
    divergence: likely
```

| Field (top-level)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `name`                   | Yes      | Change name (kebab-case). |
| `targets`                | Yes      | Map of target key → `{ adapter, locator, cid, model-capability-profile? }`. Every slice names one of these keys. |
| `sources`                | Yes      | Map of source key → `{ adapter, locator xor value, cid? }`. The reserved `intent` row is value-only (no locator, no CID). Location rows carry a closed tree `cid`. |
| `slices`                 | Yes      | Ordered list of slice entries (see below). |
| `discovery-digest`       | Yes after author | Canonical digest of `discovery.yaml`. |
| `leads-digest`           | Yes after survey | Canonical digest of `leads.md`. |
| `decomposition-digest`   | Yes after decompose | Canonical digest of `decomposition.yaml`. |

| Field (per slice)        | Required | Description |
| ------------------------ | -------- | ----------- |
| `name`                   | Yes      | Slice name (kebab-case, unique within the plan). |
| `target`                 | Yes      | Key in `plan.yaml.targets`. Required; there is no omit-and-auto-bind. |
| `sources`                | Yes      | List of `{ source, lead }` bindings; cardinality ≥ 1. Bare `<source>` shorthand allowed when the lead id equals the slice's `name`. |
| `divergence`             | No       | Closed enum: `none` (default; absent), `likely` / `accepted` / `rejected` — all set by `emery plan amend <entry> --divergence`. Advisory metadata in v1. |
| `depends-on`             | No       | List of slice names that must project `done` first. |
| `allow-composition-replace` | No    | `false` by default. Set via `emery plan amend <entry> --allow-composition-replace true`; authorizes a whole-document composition to overwrite a non-empty baseline when the execute loop merges this slice. |
| `context`                | No       | List of baseline paths relevant to the slice; used as a focus hint by briefs. |
| `description`            | No       | What this slice does (human-readable). |

## change.md

**Location:** `.emery/change/change.md`
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

**Location:** `.emery/change/slices/<name>/metadata.yaml`
**Created by:** the `emery plan refine` drain (slice create is re-entry safe)
**Modified by:** the `plan refine` drain, the build / merge orchestrations inside `emery plan execute`, and `emery plan drop`

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
