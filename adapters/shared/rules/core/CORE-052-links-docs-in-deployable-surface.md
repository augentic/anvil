---
id: CORE-052
title: Links Docs In Deployable Surface
severity: important
trigger: A deployable plugin or adapter surface links into the repository's docs/ tree.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/*.md"
    description: Marketplace skills and their reference bodies (a deployed surface).
  - kind: path-pattern
    value: "adapters/**/briefs/*.md"
    description: Adapter operation briefs that ship with the adapter.
  - kind: path-pattern
    value: "adapters/**/references/*.md"
    description: Adapter reference bodies that ship with the adapter.
  - kind: regex
    value: "\\]\\((docs/|[^)]*\\.\\./docs/)[^)]*\\)"
    description: A markdown link whose target is root-relative `docs/…` or escapes into `../docs/…`.
---

## Rule

Deployable surfaces — marketplace skills and the briefs and references that ship inside an adapter — must not link into the repository's `docs/` tree. `docs/` is contributor documentation that does not travel with a published plugin or adapter, so a `docs/`-targeted link resolves in this repo but dangles wherever the surface is deployed.

The deterministic-hint interpreter narrows to the deployable file set with `path-pattern` hints, then flags any `[label](target)` link whose target is root-relative (`docs/…`) or escapes upward into `../docs/…`. URL targets are not matched because they begin with a scheme rather than `docs/` or `../docs/`.

## Look For

- A skill or brief that links to `docs/explanation/…` or `../../docs/reference/…`.
- A reference body that cites a contributor doc instead of co-located plugin/adapter material.

## Fix

Move the cited content into the deployable surface (`plugins/spec/references/`, `../references/spec-runtime/`, or the adapter's own `references/`), or link to the published site (`https://specify.augentic.io`) when external depth is intended.
