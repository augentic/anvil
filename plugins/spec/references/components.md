# Component catalog (runtime)

Vectis-only, opt-in: `.specify/design-system/components.yaml` declares shared UI components the Vectis target factors at build time (alongside `tokens.yaml` and `assets.yaml`). Projects without the file behave as before.

## Problem: cross-slice component drift

Each `screenshots.extract` invocation only sees one lead. Stage-6 detection promotes `component: <slug>` only when two or more identical groups appear in the same run. Across slices, the adapter has no memory — repeated structures can be inlined twice and drift. The catalog lets the operator declare shared components once.

## File location

```text
.specify/design-system/components.yaml
```

Workspace mode: `<coordinator-root>/.specify/workspace/<project>/.specify/design-system/components.yaml`

## Schema (minimal)

```yaml
version: 1
components:
  tab-bar:
    status: confirmed
    description: "Bottom navigation across primary sections."
```

- **`status`** — `confirmed` (build factors shared code) or `rejected` (suppresses catalog-drift warnings for that slug).
- **`description`** — optional operator note.
- Slugs: kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).

## Operator workflow

1. **Observe hints** — `notes.candidate_component: <slug>` on Evidence from screenshots stage-6.
2. **Curate** — add `status: confirmed` when reuse is intentional.
3. **Build** — Vectis reads the catalog and factors `shared/src/components/<slug>.rs`, iOS `Components/<Slug>View.swift`, Android `components/<Slug>Component.kt` per confirmed slug referenced in `composition.yaml`.
4. **Reject false positives** — `status: rejected` suppresses drift findings without building shared code.

## Validation

| Surface | Finding | Meaning |
| --- | --- | --- |
| `specify slice validate` | `slice-catalog-drift` | Evidence has `component: <slug>` not in catalog or `rejected`. Absent catalog = no-op. |
| `specify tool run vectis -- validate composition` | Catalog cross-reference | Every `component:` in `composition.yaml` must be `confirmed`. |

## What the catalog does not do

- No auto-population — operator-curated only.
- No retroactive baseline rewrite without a refactor slice.
- No CLI verbs for catalog edits — edit YAML directly like tokens/assets.
- No sharing across projects.

Full guide: [Component catalog](https://specify.augentic.io/explanation/components.html).
