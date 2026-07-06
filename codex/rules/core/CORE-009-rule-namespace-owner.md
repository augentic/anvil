---
id: CORE-009
title: Rule Namespace Owner
severity: important
trigger: A rule markdown file declares an id whose namespace prefix is not owned by the rules directory it lives under, so a `CORE-`, `UNI-`, `OMNIA-`, `VECTIS-`, `IFACE-`, or `SRC-` rule has been authored in the wrong tree.
rule_hints:
  - kind: path-pattern
    value: adapters/codex/rules/core/CORE-009-rule-namespace-owner.md
    description: Sentinel path so the whole-tree rules tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: rules
    config:
      owner-prefixes:
        universal: [UNI]
        core: [CORE]
        omnia: [OMNIA, RUST, SEC]
        contracts: [IFACE]
        vectis: [VECTIS]
      source-axis-prefixes: [SRC]
      reserved-namespaces:
        FRAME: universal
    description: Run the `rules` framework checker scoped to CORE-009. It reads each rule file's `id`, derives the namespace prefix and the rules-directory owner, and flags any rule whose prefix is not owned by its directory. The owner→prefix map, the source-axis prefixes, and the reserved-namespace owners are policy carried here, not in the tool.
---

## Rule

Rule ids are namespaced by a prefix (`CORE-009`, `UNI-014`, `OMNIA-001`, …), and each namespace prefix has exactly one owning rules directory. `CORE-*` rules live under `adapters/codex/rules/core/`; `UNI-*` rules live under `adapters/codex/rules/universal/`; each target adapter owns its own prefixes (`omnia` owns `OMNIA-*`, `RUST-*`, and `SEC-*`; `contracts` owns `IFACE-*`; `vectis` owns `VECTIS-*`) under `adapters/targets/<name>/prose/rules/`; and every source adapter shares the `SRC-*` prefix under `adapters/sources/<name>/prose/rules/`. This rule asserts the placement invariant behind that arrangement: a rule's id-namespace prefix must match the namespace its containing directory owns.

The check is whole-tree: the `rules` framework tool walks `PROJECT_DIR` itself, reads each rule file's `id`, derives the rules-directory owner from the path, and resolves the allowed prefix set. The owner→prefix map, the `SRC-*` source-axis prefixes, and the reserved-namespace owners (`FRAME-*` is reserved for the framework `universal` pack) all live in this rule's `config:` so they are framework-owned policy, not baked into the checker; the engine relays the config to the tool. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint.

The tool preserves the four branches of the historical namespace check: the reserved-namespace reservation (`FRAME-*` may only be authored under the `universal` owner), dynamic source-owner discovery (every `adapters/sources/<name>/prose/rules/` directory contributes a `SRC-*` owner found at runtime), the unknown-owner diagnostic (a rules directory whose owner is not in `owner-prefixes`), and the placement check (a well-formed `PREFIX-NNN` id whose prefix is not in its owner's allowed set). A file that is not under a recognised rules directory, or whose id is missing or malformed, is left to the schema rule rather than flagged here.

## Look For

- A `CORE-*` rule dropped into a target-adapter `rules/` tree (or any non-core directory) during a refactor, so its prefix no longer matches its directory's owner.
- A rule copied from one adapter into another without renaming its id, leaving an `OMNIA-*` or `VECTIS-*` prefix under the wrong adapter.
- A shared rule placed under `adapters/codex/rules/core/` with a `UNI-*` id (or under `universal/` with a `CORE-*` id), crossing the two shared packs.

## Fix

Move the rule file into the directory that owns its namespace prefix, or renumber the id to the prefix its current directory owns. Keep `CORE-*` under the core pack, `UNI-*` under the universal pack, each target adapter's prefixes under that adapter's `rules/` tree, and `SRC-*` under a source adapter's `rules/` tree.
