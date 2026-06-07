---
id: CORE-036
title: Skill Description Grammar
severity: important
trigger: SKILL.md description violates authoring grammar.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-036-skill-description-grammar.md
    description: Sentinel path so the whole-tree skill tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill
    config:
      allowed-verbs:
        - add
        - annotate
        - apply
        - audit
        - author
        - build
        - categorise
        - categorize
        - check
        - compare
        - compile
        - complete
        - compose
        - compute
        - configure
        - convert
        - create
        - decompose
        - define
        - describe
        - design
        - diff
        - discover
        - drive
        - drop
        - enforce
        - execute
        - expose
        - export
        - extract
        - fetch
        - fix
        - format
        - generate
        - guard
        - implement
        - import
        - infer
        - ingest
        - init
        - initialize
        - list
        - load
        - merge
        - monitor
        - orchestrate
        - plan
        - preview
        - process
        - produce
        - propose
        - publish
        - reconstruct
        - refine
        - render
        - resolve
        - review
        - run
        - scaffold
        - select
        - show
        - shorten
        - split
        - stage
        - store
        - summarize
        - test
        - translate
        - transform
        - trim
        - validate
        - verify
        - wire
        - wrap
        - write
    description: Run the `skill` framework checker, which flags any SKILL.md whose `description` does not start with a verb in the allow-list. The allow-list is policy carried here, not in the tool.
---

## Rule

A skill's `description` frontmatter field must begin with an approved imperative verb so the skill catalog reads consistently. The first alphabetic word of the description (lowercased) must be a member of the `allowed-verbs` allow-list, which is supplied in `config:` so the policy lives in this rule file, not the tool.

This check is whole-tree: the `skill` framework tool discovers every `SKILL.md` under `plugins/`, then checks each one's `description`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A `description` with no leading alphabetic word.
- A `description` whose first word is not in the `allowed-verbs` allow-list.

## Fix

Begin the `description` with an imperative verb from the approved allow-list; if a genuinely imperative verb is missing, add it to the rule's `allowed-verbs` list.
