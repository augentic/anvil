# Component catalog

Vectis-only, agent-inferred and operator-reviewable: `.emery/design-system/components.yaml` declares shared UI components the Vectis target factors at build time (alongside `tokens.yaml` and `assets.yaml`). The catalog is written by the vectis target's in-guest inference during each build when shared structures appear across the accumulated composition baseline — it is not hand-curated — and the operator reviews, rejects, or renames entries. An operator who already knows a shared part of the design may pre-define it in a hand-authored `parts.yaml` that seeds inference with naming and promotion authority; everything else is discovered. Projects with no shared structures have no catalog and behave as before.

The file formats, slug grammar, and validation findings live in the [Component catalog and parts](../reference/artifact-format.md#component-catalog-and-parts-vectis-only) reference. This page explains why the catalog exists and how the operator interacts with it.

## Problem: cross-slice component drift

Each `screenshots.extract` invocation only sees one lead. Stage-6 detection promotes `component: <slug>` only when two or more identical groups appear in the same run. Across slices, the adapter has no memory — repeated structures can be inlined twice and drift. The vectis build's in-guest inference closes this gap: at build time it clusters structurally identical groups across the accumulated composition baseline (plus the screenshots candidate cache), and the build's judgment leg identifies, names, and binds each shared structure into the catalog — so components are discovered automatically rather than declared by hand.

## Inputs vs resolved

`parts.yaml` is a hand-authored **input** that sits beside `tokens.yaml` and `assets.yaml`; the agent-written `components.yaml` stays the **resolved** catalog. This is an inputs-vs-resolved split, not a second writer over one file — the bind step re-derives the part-backed catalog entries from `parts.yaml` on every run, so there is nothing to clobber and no collision with the catalog's no-overwrite rules. A part is never mandatory — it is a best-effort matching hint exactly like a `tokens.yaml` / `assets.yaml` entry, carrying two authorities over inference: the operator's slug wins for that structure (**naming**), and a matched part is factored as shared even below the occurrence threshold (**promotion**).

## Operator workflow

Inference is the default author; the operator reviews rather than curating from nothing.

0. **Pre-define (optional)** — declare a known shared part up front in `parts.yaml` ([format](../reference/artifact-format.md#parts-format-partsyaml)). Skip this when no parts are known in advance.
1. **Infer** — each Vectis build runs its in-guest inference report over the accumulated baseline (plus the screenshots candidate cache), the build's judgment leg identifies and names each new shared structure, and the bind step writes the named entries as `status: confirmed`. This is the only writer of the catalog.
2. **Factor** — composition regeneration attaches `component: <slug>` to every matching group, and the shell writers factor `shared/src/components/<slug>.rs`, iOS `Components/<Slug>View.swift`, Android `components/<Slug>Component.kt` per confirmed slug referenced in `composition.yaml`. Retroactive factoring reaches backward into prior-slice screens that share the structure.
3. **Review** — inspect what was clustered and named in the build's inference report.
4. **Reject or rename** — set `status: rejected` to permanently suppress a slug, or rename an inferred entry; the bind step's no-overwrite rule keeps both stable on later runs.

## What the catalog does not do

- No CLI verbs for the catalog — the vectis build's in-guest bind step writes it (binding the names the build's judgment leg or operator parts supply), and to reject or rename an entry the operator edits the YAML directly, like tokens / assets.
- No sharing across projects.

## See also

- [Component catalog and parts (Vectis only)](../reference/artifact-format.md#component-catalog-and-parts-vectis-only) — file formats, slug grammar, and validation findings
- [Composition document (Vectis only)](../reference/artifact-format.md#composition-document-vectis-only) — the `composition.yaml` the factored components are referenced from
