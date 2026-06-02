# Component Catalog

> [!NOTE]
> **Who needs this page.** The component catalog is a **Vectis-only**, opt-in feature for projects generating cross-platform UI. If your target is Omnia or Contracts, or you are not factoring shared UI components, you can skip it — projects that never create the file behave exactly as before.

The component catalog at `.specify/design-system/components.yaml` lets operators declare shared UI components that the Vectis target factors into reusable code at build time. It is the third operator-curated design-system file, joining `tokens.yaml` (colours, typography, spacing) and `assets.yaml` (raster, vector, and symbol entries).

## The problem: cross-slice component drift

Each `screenshots.extract` invocation only sees the lead it is asked to extract. The screenshots adapter's stage-6 component detection fires within one run: a `component: <slug>` directive lands when two or more structurally identical groups appear in the same extraction. Across slices and across runs, the adapter has no memory.

Consider two successive plans importing different screens. Both carry a tab bar, but each extract sees only one instance in its run. Stage 6 emits `notes.candidate_component: tab-bar` on both — a hint, not a promotion. The Vectis build inlines the tab bar twice, and the two implementations drift visually over time. The operator never knew the reuse opportunity existed.

The catalog closes this gap by giving the operator a place to declare "this structure is a shared component" once, after which every future build factors it automatically.

## Relationship to tokens and assets

All three design-system files follow the same pattern:

| File | What it declares | Who authors it | Who consumes it |
| --- | --- | --- | --- |
| `tokens.yaml` | Colours, typography, spacing, radii, elevation | Operator | Vectis build (theme code per shell) |
| `assets.yaml` | Raster, vector, and symbol entries with per-platform sources | Operator | Vectis build (asset catalogs per shell) |
| `components.yaml` | Shared UI components with `confirmed` / `rejected` status | Operator | Vectis build (shared component files per shell) |

All three are opt-in. Projects that never create the file work exactly as before — no behavior change.

## File location

```text
.specify/design-system/components.yaml
```

In workspace mode the path is per-project, following the same routing as slices and sibling design-system files:

```text
<coordinator-root>/.specify/workspace/<project>/.specify/design-system/components.yaml
```

## Schema

The schema is CLI-owned (alongside `evidence.schema.json`, `plan.schema.json`, and other framework-level schemas) and lives in the CLI repo at `schemas/design-system/components.schema.json`.

A minimal catalog:

```yaml
version: 1
components:
  tab-bar:
    status: confirmed
    description: "Bottom navigation across the primary app sections."
  card-row:
    status: confirmed
    description: "Horizontal card layout used in browse and search screens."
  social-banner:
    status: rejected
    description: "Initially looked shared but each instance diverges too much."
```

Each entry has:

- **`status`** — `confirmed` (the build factors this as a shared component) or `rejected` (intentionally declined; suppresses `slice-catalog-drift` warnings for Evidence that carries the slug in `notes.candidate_component`).
- **`description`** — human-readable note for operators and agents. Optional.

Component slugs must be kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).

## Operator workflow

1. **Observe hints.** During `/spec:refine`, the screenshots adapter's stage-6 detection emits `notes.candidate_component: <slug>` on container claims that look like shared components but do not yet meet the promotion threshold. These hints surface in Evidence and in the slice's synthesized `spec.md` as `[unknown]` tags.

2. **Curate the catalog.** When the operator notices repeated `notes.candidate_component` annotations across slices — or spots repeated structures by visual inspection — they open `.specify/design-system/components.yaml` and add an entry with `status: confirmed`.

3. **Build factors shared code.** On the next `/spec:build`, the Vectis target reads the catalog. For every `confirmed` entry referenced by `component:` directives in `composition.yaml`, the build generates one shared component file per in-scope shell tree:

   - **Core:** `shared/src/components/<slug>.rs` — a shared view-model helper module.
   - **iOS:** `iOS/<AppName>/Components/<Slug>View.swift` — a named SwiftUI view.
   - **Android:** `Android/.../components/<Slug>Component.kt` — a named Composable.

   Per-screen rendering invokes the shared component instead of inlining.

4. **Reject false positives.** When a candidate component turns out not to be shared (e.g. instances diverge too much), the operator sets `status: rejected`. This suppresses `slice-catalog-drift` findings for Evidence that carries the slug in `notes.candidate_component` without cluttering future builds.

## Validation

Two validation surfaces check catalog consistency:

| Surface | Finding | Meaning |
| --- | --- | --- |
| `specrun slice validate` | `slice-catalog-drift` | Evidence persists a claim with `component: <slug>` where the slug is not in the catalog or has `status: rejected`. Absent catalog silently skips (opt-in). |
| `specrun tool run vectis -- validate composition` | Catalog cross-reference (check 5) | Every `component: <slug>` in `composition.yaml` must resolve to a `confirmed` entry. Every `confirmed` entry should have at least one reference (warning if unreferenced). |

Both validations treat an absent catalog file as a no-op — the catalog is opt-in.

## Worked example

**Plan 1** — operator imports five onboarding screens via `screenshots`:

```bash
/spec:plan seed-app source ui=screenshots:./screens/onboarding
```

- `survey` produces five leads (`splash`, `signin`, `task-list`, `archive`, `settings`).
- `/spec:execute` runs refine; extract sees a 3-tab footer on `task-list` + `archive` + `settings` — stage 6 emits `component: tab-bar` on three claims (≥2 instances in the same run).
- Vectis build factors `tab-bar` as a shared component on all three screens.
- Plan merges; baseline screens reference `component: tab-bar`.
- The operator adds `tab-bar: status: confirmed` to `.specify/design-system/components.yaml` so future slices benefit.

**Plan 2** — operator imports two new profile screens:

```bash
/spec:plan profile-screens source ui=screenshots:./screens/profile
```

- `survey` produces `profile` + `profile-edit`.
- Refine runs extract. Both new screens carry a structurally matching footer — the operator (or the agent reading the catalog) applies `component: tab-bar` to both claims.
- Vectis build emits `composition.yaml` for both new screens with `component: tab-bar` already wired. No shared-component file regeneration needed (it already exists from plan 1).

The tab bar is now factored across all seven screens with no visual drift.

## What the catalog does not do

- **No auto-population.** The catalog is operator-curated, not auto-populated by the CLI. Stage-6 `notes.candidate_component` annotations serve as hints; the operator decides what to add.
- **No retroactive rewrite.** Adding a catalog entry does not retroactively rewrite baseline `composition.yaml`. The operator schedules a refactor slice when ready.
- **No CLI verbs for catalog management.** The file is small and human-readable; operators edit it directly, the same way they edit `tokens.yaml` and `assets.yaml`.
- **No sharing across projects.** Each project's catalog is local to that project root (including its workspace slot).

## See also

- [Tool declarations](tool-declarations.md) — WASI tool resolution and the `specrun tool schema` verb for retrieving tool-owned schemas
- [Artifacts](artifacts.md) — the four slice artifacts and `composition.yaml`
- [Decision log](decision-log.md) — design rationale for tool-owned schemas, standalone preview, and the component catalog
