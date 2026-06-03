---
id: CORE-009
title: Rule Namespace Owner
severity: important
trigger: A rule markdown file declares an id whose namespace prefix is not owned by the rules directory it lives under, so a `CORE-`, `UNI-`, `OMNIA-`, `VECTIS-`, `IFACE-`, or `SRC-` rule has been authored in the wrong tree.
rule_hints:
  - kind: path-pattern
    value: adapters/**/rules/**/*.md
    description: Narrow the candidate set to rule markdown files under any adapter or shared rules tree.
  - kind: namespace-owner
    value: rule-namespace-matches-owner
    description: For each candidate rule file, read its `id` frontmatter, derive the namespace prefix, and assert that prefix is owned by the containing rules directory. One finding per rule whose prefix is not owned by its directory.
---

## Rule

Rule ids are namespaced by a prefix (`CORE-009`, `UNI-014`, `OMNIA-001`, …), and each namespace prefix has exactly one owning rules directory. `CORE-*` rules live under `adapters/shared/rules/core/`; `UNI-*` rules live under `adapters/shared/rules/universal/`; each target adapter owns its own prefixes (`omnia` owns `OMNIA-*`, `RUST-*`, and `SEC-*`; `contracts` owns `IFACE-*`; `vectis` owns `VECTIS-*`) under `adapters/targets/<name>/rules/`; and every source adapter shares the `SRC-*` prefix under `adapters/sources/<name>/rules/`. This rule asserts the placement invariant behind that arrangement: a rule's id-namespace prefix must match the namespace its containing directory owns.

The deterministic-hint interpreter narrows the candidate set to rule markdown files with a `path-pattern` hint, then reads each candidate's `id` from the frontmatter fact the indexer already produced. It derives the `PREFIX` from a well-formed `PREFIX-NNN` id, resolves the prefix set owned by the rule's directory, and flags any rule whose prefix is not in that set. A file that is not under a recognised rules directory, or whose id is missing or malformed, is left to the hand-written namespace-ownership and schema predicates rather than flagged here.

This rule is the declarative companion to the hand-written namespace-ownership predicate, not a replacement for it. The imperative predicate additionally reserves the framework-only `FRAME-*` namespace, discovers source-adapter owners dynamically, reports unconfigured owners, validates rule frontmatter against the schema, and detects duplicate ids across files — branches a single declarative hint cannot express. Because every rule in the framework tree is already authored under its owning directory, this rule fires zero findings against the current tree and surfaces only on misplacement.

## Look For

- A `CORE-*` rule dropped into a target-adapter `rules/` tree (or any non-core directory) during a refactor, so its prefix no longer matches its directory's owner.
- A rule copied from one adapter into another without renaming its id, leaving an `OMNIA-*` or `VECTIS-*` prefix under the wrong adapter.
- A shared rule placed under `adapters/shared/rules/core/` with a `UNI-*` id (or under `universal/` with a `CORE-*` id), crossing the two shared packs.

## Fix

Move the rule file into the directory that owns its namespace prefix, or renumber the id to the prefix its current directory owns. Keep `CORE-*` under the core pack, `UNI-*` under the universal pack, each target adapter's prefixes under that adapter's `rules/` tree, and `SRC-*` under a source adapter's `rules/` tree.
