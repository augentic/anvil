---
id: CORE-011
title: Agent Teams Missing Canonical
severity: important
trigger: The canonical review-team-protocol document is missing so overlays cannot be validated.
rule_hints:
  - kind: path-pattern
    value: codex/rules/core/CORE-011-agent-teams-missing-canonical.md
    description: Sentinel include so the rule carries a candidate set; the `presence` file selector evaluates the whole file fact family and ignores the candidate set.
  - kind: presence
    value: file
    config:
      path: docs/reference/review-team-protocol.md
    description: Flag the canonical review-team-protocol document when no file fact carries its path. The required path is policy carried here, not in the engine.
---

## Rule

Per-target `agent-teams.md` overlays resolve to a single canonical review-team-protocol document. When that canonical document is absent every overlay symlink dangles, so its absence is itself a violation. The required path is supplied in `config:` so the policy lives in this rule file, not the engine.

Overlays MUST be symlinks; regular-file `agent-teams.md` overlays are forbidden. A symlink chain cannot drift in content — only its endpoint can vanish or be repointed — so this presence guard plus CI's symlink check (which verifies each overlay resolves to the canonical document) is the whole enforcement surface. The retired CORE-008 digest pin and CORE-012 `agent-teams` tool duplicated that guarantee for a regular-file overlay form that is no longer admitted.

This check runs natively: the `kind: presence` hint with `value: file` and `config: { path }` flags the required document whenever the lint indexer recorded no file fact at that path. The rule's `path-pattern` is a sentinel include; the file presence selector evaluates the whole file fact family regardless of the candidate set.

## Look For

- The canonical review-team-protocol document named in `config: { path }` is missing.

## Fix

Restore the canonical review-team-protocol document at the path named in `config: { path }`.
