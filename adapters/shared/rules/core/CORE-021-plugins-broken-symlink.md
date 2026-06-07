---
id: CORE-021
title: Plugins Broken Symlink
severity: important
trigger: A symlink under plugins/ points at a target that does not exist on disk.
rule_hints:
  - kind: reference-resolves
    value: symlink
    description: Flag every symlink recorded under `plugins/` whose target does not resolve on disk.
    config:
      path-prefix: "plugins/"
---

## Rule

Every symlink under `plugins/` must resolve to an existing target on disk. Marketplace plugins share reference and example material through symlinks; a dangling link breaks the published plugin tree for every consumer that materialises it.

The deterministic-hint interpreter consumes the `symlink` facts the indexer records and flags those whose `broken` flag is set, scoped to the `plugins/` path prefix carried in `config`.

## Look For

- A symlink under `plugins/` whose target was renamed, moved, or never committed.
- A relative symlink target that escapes the plugin tree and no longer resolves.

## Fix

Repoint the symlink at its current target, recreate the missing target, or remove the stale link. Cross-check with `git log --diff-filter=D` to confirm whether the target was renamed or deleted.
