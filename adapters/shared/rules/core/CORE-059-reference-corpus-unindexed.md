---
id: CORE-059
title: Reference Corpus Unindexed
severity: important
trigger: A reference-corpus subdirectory under an adapter's `references/` tree holds two or more files but no `README.md` index, so an agent applying the corpus must open every file to discover what it contains instead of spending context on the one file it needs.
rule_hints:
  - kind: presence
    value: directory-index
    config:
      roots:
        - adapters/sources/*/prose/references/*
        - adapters/targets/*/prose/references/*
        - adapters/shared/references/*
      index: README.md
      min-files: 2
    description: Over the directory prefixes of the file facts, each directory matching a `roots` glob (one directory depth; `*` does not cross `/`) with at least `min-files` files beneath it must carry a `README.md` directly inside it; an unindexed corpus directory is a finding located at the directory.
---

## Rule

Reference corpora are context-budget surfaces: prompts steer agents into `references/` trees with instructions like "read the matching capability example", and the agent must be able to pick that file from an index instead of opening the whole directory. Every subdirectory of an adapter's `references/` tree that has grown to two or more files owes a `README.md` index — a short file listing each member with a one-line description of when to read it.

The deterministic hint walks the directory prefixes of the indexed file facts, keeps those matching the configured corpus roots (direct children of each adapter's `references/` directory, plus the shared corpus root), counts the files beneath each (recursively, so a corpus organized into nested subfolders still owes its top-level index), and flags any at-threshold directory whose `README.md` is absent.

## Look For

- A new `references/<topic>/` subdirectory that accumulated sibling files without an index — the usual smell is a prompt that says "see `references/<topic>/`" with no guidance on which file inside it answers what.
- An existing corpus whose `README.md` was deleted or renamed during a restructure while the member files survived.

## Fix

Add a `README.md` directly inside the flagged directory: a title, one sentence on what the corpus covers, and a table or list naming each member file with a one-line description of when an agent should read it. Mirror an existing corpus index (for example `adapters/targets/omnia/prose/references/providers/README.md`). Keep it an index — deep content belongs in the member files, not the README.
