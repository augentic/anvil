---
id: CORE-011
title: Agent Teams Missing Canonical
severity: important
trigger: The canonical review-team-protocol document is missing so overlays cannot be validated.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-011-agent-teams-missing-canonical.md
    description: Sentinel path so the whole-tree agent-teams tool runs exactly once; the tool walks PROJECT_DIR/adapters/targets itself rather than the passed candidate.
  - kind: tool
    value: agent-teams
    config:
      canonical-path: docs/reference/review-team-protocol.md
    description: Run the `agent-teams` framework checker, which flags an absent canonical review-team-protocol document. The canonical path is policy carried here, not in the tool.
---

## Rule

Per-target `agent-teams.md` overlays are validated against a single canonical review-team-protocol document. When that canonical document is absent the overlays cannot be checked against any baseline, so its absence is itself a violation. The canonical path is supplied in `config:` so the policy lives in this rule file, not the tool.

This check is whole-tree: the `agent-teams` framework tool reads the canonical document named in `config:` and, when it is missing, reports exactly once. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- The canonical review-team-protocol document named in `canonical-path` is missing.

## Fix

Restore the canonical review-team-protocol document at the path named in `canonical-path`.
