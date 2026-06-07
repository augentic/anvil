---
id: CORE-012
title: Agent Teams Non Canonical Overlay
severity: important
trigger: A target adapter agent-teams.md overlay does not match the canonical review-team protocol.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-012-agent-teams-non-canonical-overlay.md
    description: Sentinel path so the whole-tree agent-teams tool runs exactly once; the tool walks PROJECT_DIR/adapters/targets itself rather than the passed candidate.
  - kind: tool
    value: agent-teams
    config:
      canonical-path: docs/reference/review-team-protocol.md
    description: Run the `agent-teams` framework checker, which flags any per-target agent-teams.md overlay that drifts from the canonical document. The canonical path is policy carried here, not in the tool.
---

## Rule

Each target adapter's `references/agent-teams.md` must mirror the canonical review-team-protocol document — ideally as a symlink to it, or as a byte-identical regular file. This rule is deliberately stricter than CORE-008 (`content-digest-eq`), whose symlink-only fact cannot express the path-equality, regular-file content-drift, and unsupported-entry-type branches enforced here. The canonical path is supplied in `config:` so the policy lives in this rule file, not the tool.

An overlay drifts when it is a symlink that resolves to a path other than the canonical document (or does not resolve), a regular file whose contents differ from the canonical document, or an entry that is neither a regular file nor a symlink.

This check is whole-tree: the `agent-teams` framework tool walks every `adapters/targets/*/references/agent-teams.md` overlay. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A `references/agent-teams.md` symlink resolving to anything other than the canonical document, or one that does not resolve.
- A `references/agent-teams.md` regular file whose contents differ from the canonical document.
- A `references/agent-teams.md` that is neither a regular file nor a symlink.

## Fix

Replace the overlay with a symlink to the canonical review-team-protocol document, or re-sync its contents so the digests match.
