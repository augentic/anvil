# Component catalog (runtime)

Vectis-only, agent-inferred and operator-reviewable: `.specify/design-system/components.yaml` declares shared UI components the Vectis target factors at build time (alongside `tokens.yaml` and `assets.yaml`). The catalog is written by `specify catalog infer` during each build when shared structures appear across the accumulated composition baseline — it is not hand-curated — and the operator reviews, rejects, or renames entries. An operator who already knows a shared part of the design may pre-define it in a hand-authored `parts.yaml` ([Operator-defined parts](#operator-defined-parts-partsyaml)) that seeds inference with naming and promotion authority; everything else is discovered. Projects with no shared structures have no catalog and behave as before.

## Problem: cross-slice component drift

Each `screenshots.extract` invocation only sees one lead. Stage-6 detection promotes `component: <slug>` only when two or more identical groups appear in the same run. Across slices, the adapter has no memory — repeated structures can be inlined twice and drift. `specify catalog infer` closes this gap: at build time it clusters structurally identical groups across the accumulated composition baseline (plus the screenshots candidate cache), and the build skill identifies, names, and binds each shared structure into the catalog — so components are discovered automatically rather than declared by hand.

## File location

```text
.specify/design-system/components.yaml
```

Workspace mode: `<coordinator-root>/workspace/<project>/.specify/design-system/components.yaml`

## Schema (minimal)

```yaml
version: 1
components:
  tab-bar:
    status: confirmed
    description: "Bottom navigation across primary sections."
    fingerprint: "<64-char lowercase hex>"   # optional; the structural identity bound to this slug
```

- **`status`** — `confirmed` (build factors shared code) or `rejected` (suppresses catalog-drift warnings for that slug).
- **`description`** — optional note.
- **`fingerprint`** — optional lowercase SHA-256 hex (`^[0-9a-f]{64}$`) of the component's normalized structural skeleton. `specify catalog infer --phase bind` writes it so a later `--phase report` echoes the bound slug for an already-named cluster (run-to-run binding stability). Hand-authored / pre-inference entries omit it.
- Slugs: kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).

## Operator workflow

Inference is the default author; the operator reviews rather than curating from nothing.

0. **Pre-define (optional)** — declare a known shared part up front in `parts.yaml` ([Operator-defined parts](#operator-defined-parts-partsyaml)). A part seeds inference with an authoritative name and is factored even below the occurrence threshold; inference discovers the rest. Skip this when no parts are known in advance.
1. **Infer** — each Vectis build runs `specify catalog infer --phase report` over the accumulated baseline (plus the screenshots candidate cache), the build skill identifies and names each new shared structure by judgement, and `--phase bind` writes the named entries as `status: confirmed`. This is the only writer of the catalog.
2. **Factor** — composition regeneration attaches `component: <slug>` to every matching group, and the shell writers factor `shared/src/components/<slug>.rs`, iOS `Components/<Slug>View.swift`, Android `components/<Slug>Component.kt` per confirmed slug referenced in `composition.yaml`. Retroactive factoring reaches backward into prior-slice screens that share the structure.
3. **Review** — inspect what was clustered and named with `specify catalog infer --phase report` (read-only) or the `--phase bind --dry-run` diff.
4. **Reject or rename** — set `status: rejected` to permanently suppress a slug, or rename an inferred entry; `bind`'s no-overwrite rule keeps both stable on later runs.

## Operator-defined parts (`parts.yaml`)

Operators who know a shared part of a design up front can declare it instead of waiting for inference to discover it. `.specify/design-system/parts.yaml` is a hand-authored **input** that sits beside `tokens.yaml` and `assets.yaml`; the agent-written `components.yaml` stays the **resolved** catalog. This is an inputs-vs-resolved split, not a second writer over one file — `specify catalog infer --phase bind` re-derives the part-backed catalog entries from `parts.yaml` on every run, so there is nothing to clobber and no collision with the catalog's no-overwrite rules.

```yaml
version: 1
parts:
  tab-bar:
    description: "Bottom navigation across primary sections."
    group:                  # schema-compliant composition `group` fragment
      active-when: "$route"
      items:
        - icon-button: { bind: "home",     event: "Navigate(Home)" }
        - icon-button: { bind: "search",   event: "Navigate(Search)" }
        - icon-button: { bind: "settings", event: "Navigate(Settings)" }
```

- **`group`** — required; a schema-compliant composition `group` fragment. Identity is its normalized structural skeleton (the same `build_group_skeleton` output that drives inference), so the `bind` / `event` / `*-when` *values* are illustrative and stripped before fingerprinting — paste a representative real group.
- **`description`** — optional; carried into the resolved catalog entry.
- Slugs: kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`), same grammar as the catalog.

A part carries two authorities over inference: **naming** (the operator's slug wins for that structure) and **promotion** (a matched part is factored as shared even below `--min-occurrences`). A part is never mandatory — it is a best-effort matching hint exactly like a `tokens.yaml` / `assets.yaml` entry. Parts whose skeleton matches at least one baseline group are projected into `components.yaml` as `status: confirmed` and factored — including retroactively across prior-slice screens, like any inferred component; parts that match nothing are listed in the non-blocking `part-unmatched` report and inference proceeds regardless. `parts.yaml` is schema-validated on read (`parts.schema.json`); beyond schema conformance there are no coherence gates, so a mis-authored part resolves deterministically (operator slug wins, `rejected` suppression holds) and the operator repairs the file if the outcome is wrong.

## Validation

| Surface | Finding | Meaning |
| --- | --- | --- |
| `specify slice validate` | `slice-catalog-drift` | Evidence has `component: <slug>` not in catalog or `rejected`. Absent catalog = no-op. |
| `specify extension run vectis -- validate composition` | Catalog cross-reference | Every `component:` in `composition.yaml` must be `confirmed`. |

## What the catalog does not do

- No CLI verbs for hand-editing entries — `specify catalog infer --phase bind` writes the catalog (binding the names the build skill or operator parts supply), but to reject or rename an entry the operator edits the YAML directly, like tokens / assets.
- No sharing across projects.

Full guide: [Component catalog](https://specify.augentic.io/explanation/components.html).
