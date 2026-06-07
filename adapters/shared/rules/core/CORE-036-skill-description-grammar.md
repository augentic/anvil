---
id: CORE-036
title: Skill Description Grammar
severity: important
trigger: SKILL.md description violates authoring grammar.
rule_hints:
  - kind: path-pattern
    value: plugins/**/SKILL.md
    description: Candidate set of every SKILL.md the `field-grammar` field-first-word mode then narrows on.
  - kind: field-grammar
    value: field-first-word
    config:
      field: description
      allowed:
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
    description: Flag any SKILL.md whose `description` does not start with a verb in the allow-list. The field name and allow-list are policy carried here, not in the engine.
---

## Rule

A skill's `description` frontmatter field must begin with an approved imperative verb so the skill catalog reads consistently. The first alphabetic word of the description (lowercased) must be a member of the `allowed` allow-list, which is supplied in `config:` so the policy lives in this rule file, not the engine.

This check runs natively: the `path-pattern` hint selects every `SKILL.md` under `plugins/`, and the `kind: field-grammar` hint with `value: field-first-word` takes the first alphabetic word of each candidate's `description` field and flags it when it is not a member of the `allowed` list.

## Look For

- A `description` with no leading alphabetic word.
- A `description` whose first word is not in the `allowed` allow-list.

## Fix

Begin the `description` with an imperative verb from the approved allow-list; if a genuinely imperative verb is missing, add it to the rule's `allowed` list.
