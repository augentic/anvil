---
id: CORE-019
title: Links Broken Reference
severity: important
trigger: A SKILL.md links to a bundled `references/` or `examples/` path that does not exist on disk.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Marketplace skill bodies that cite their bundled support material.
  - kind: reference-resolves
    value: markdown-link
    description: Flag `[label](references/…)` and `[label](examples/…)` links whose relative target does not resolve against the skill's directory.
    config:
      target-prefixes:
        - "references/"
        - "examples/"
---

## Rule

Every `[label](references/…)` or `[label](examples/…)` link inside a `SKILL.md` must resolve to a file that exists on disk after the target is joined against the skill's own directory. These links point at a skill's bundled reference and example material; a broken one means the skill cites support content that was renamed, moved, or never committed.

This is a narrower surface than CORE-002 (which checks every relative markdown link under `adapters/`, `plugins/`, `docs/`, and `.cursor/`): CORE-019 is scoped to skill bodies and to their `references/`/`examples/` payload. The deterministic-hint interpreter consumes the `markdown_link` facts the indexer already produced, restricted to the `references/`/`examples/` target prefixes carried in `config`.

## Look For

- A `[label](references/foo.md)` link in a `SKILL.md` whose joined path does not exist.
- A `[label](examples/foo/bar.md)` link whose target was moved or deleted without updating the citation.
- Anchor fragments (`[label](references/foo.md#section)`) — only the path part is checked.

## Fix

Restore the missing reference or example file, or update the citation to its new path. Keep skill support material under the skill's own `references/` and `examples/` directories so the relative link stays stable.
