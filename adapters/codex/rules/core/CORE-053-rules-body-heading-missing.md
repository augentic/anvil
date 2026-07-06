---
id: CORE-053
title: Rules Body Heading Missing
severity: important
trigger: A rules markdown file's body does not carry the verbatim `## Rule` heading.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-053-rules-body-heading-missing.md
    description: Sentinel path so the whole-tree rules tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: rules
    description: Run the `rules` framework checker scoped to CORE-053. It walks the whole rules tree, reads each rule file's post-frontmatter body, and flags any file whose body lacks a verbatim `## Rule` heading. No policy is required — the heading convention is structural.
---

## Rule

Every first-party and adapter rule ships as a markdown file whose body carries a verbatim `## Rule` heading on its own line. `rule.schema.json` (CORE-027) validates only the frontmatter between the `---` delimiters and deliberately does not cover body conventions, so the heading is enforced here. Reviewing agents read the `## Rule` section as the canonical policy text — a body without that heading leaves the policy statement unanchored for the consumers of the resolved codex.

The check is whole-tree: the `rules` framework tool walks `PROJECT_DIR` itself, reads each rule file's body across the target / source axes and the shared `universal` / `core` packs (skipping `README.md` catalogs), and flags any file missing the heading. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the body scan needs no policy from `config:`.

## Look For

- A new rule file that states its policy as bare prose without a leading `## Rule` heading.
- A rule whose heading was renamed (e.g. `## Policy`) or demoted to a different heading level.

## Fix

Add a verbatim `## Rule` heading on its own line above the rule's policy statement.
