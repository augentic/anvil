---
id: CORE-002
title: Markdown Links Resolve
severity: important
trigger: A markdown file contains an `[label](target)` link whose relative target does not exist on disk after joining against the file's parent directory.
rule_hints:
  - kind: path-pattern
    value: "adapters/**/*.md"
    description: Adapter manifests, briefs, references, and rules.
  - kind: path-pattern
    value: "plugins/**/*.md"
    description: Plugin marketplace skills and their reference bodies.
  - kind: path-pattern
    value: "docs/**/*.md"
    description: Long-form contributor and explanation documentation.
  - kind: path-pattern
    value: ".cursor/**/*.md"
    description: Editor-mirrored rules and schemas under `.cursor/`.
  - kind: reference-resolves
    value: markdown-link
    description: Walk every fence-aware `[label](target)` link extracted by the indexer and flag those whose `resolves` flag came back `false` (the relative target did not exist on disk).
---

## Rule

Every relative `[label](target)` markdown link under `adapters/`, `plugins/`, `docs/`, or `.cursor/` must resolve to an existing path on disk after the link target is joined against the markdown file's parent directory. URL-style targets (`http://…`, `https://…`, `mailto://…`), anchor-only references (`#section`), and targets the indexer cannot reason about (empty after fragment stripping) are skipped — the rule fires only on references the resolver attempted and rejected.

The path scope excludes archival trees and the proposals directory by design, because they intentionally cite future or deferred work whose targets do not yet exist on disk.

Broken links rot documentation: skill briefs, target adapter references, codex bodies, and AGENTS map files all rely on the fence-aware `[label](target)` shape that this rule covers. The deterministic-hint interpreter consumes the `markdown_link` facts the indexer already produced (`crates/standards/src/lint/index/markdown.rs::extract_links` + `index.rs::resolve_link`), so the rule cost is one BTreeSet lookup per candidate file at lint time.

## Look For

- A relative link target (`[label](./relative/path.md)` or `[label](../sibling.md)`) whose joined path does not exist on disk.
- A link target with an anchor fragment (`[label](./page.md#section)`) whose path part resolves but whose anchor does not — only the path part is checked.
- Stale references introduced after a file rename, move, or deletion where the citing markdown was not updated.

## Fix

Either restore the missing target (recreate the file, restore the rename) or update the citation to the new path. Cross-check with `git log --diff-filter=D` to confirm whether the target was renamed or removed before patching the link.
