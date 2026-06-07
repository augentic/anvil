---
id: CORE-008
title: Agent Teams Match Canonical
severity: important
trigger: A framework `agent-teams.md` symlink overlay resolves to content whose SHA-256 digest does not equal the canonical `docs/reference/review-team-protocol.md` review-team-protocol document, so a review brief points at drifted or stale review-team guidance.
rule_hints:
  - kind: content-digest-eq
    value: agent-teams-match-canonical
    config:
      canonical-path: docs/reference/review-team-protocol.md
    description: For each followed `agent-teams.md` symlink fact, assert that the resolved target's SHA-256 equals the `config.canonical-path` document's digest. One finding per symlink whose target digest diverges, with the `(resolved-target, expected-digest, actual-digest)` shape surfaced as structured evidence.
---

## Rule

Target adapter review briefs do not carry their own copy of the review-team protocol; each `adapters/targets/<name>/references/agent-teams.md` is a relative symlink to the shared `adapters/shared/references/runtime/review-team-protocol.md` overlay that resolves to the single canonical `docs/reference/review-team-protocol.md` document. The framework keeps one source of truth so review-team guidance never forks per adapter. This rule asserts the content invariant behind that arrangement: every `agent-teams.md` overlay must resolve to content whose SHA-256 digest equals the canonical document's digest.

The deterministic-hint interpreter consumes the `AgentTeam` facts the framework-profile indexer already produced by following each `agent-teams.md` symlink and recording the resolved endpoint plus a SHA-256 of the target's bytes (`crates/standards/src/lint/index/agent_teams.rs::record`). The expected canonical digest is derived from the fact set itself — the digest carried by any overlay that resolves to `docs/reference/review-team-protocol.md`. Every overlay whose recorded digest differs from that expected value (a symlink redirected at a different document, or a broken link with no readable target) is flagged. When no overlay resolves to the canonical document the expected digest cannot be established and the rule stays silent; the invariant is vacuous without an anchor.

This rule narrows on a fact family rather than a file glob, so it carries no `path-pattern` hint. The framework walker records an `agent-teams.md` symlink as an `AgentTeam` fact and emits no `file` fact for the symlink path, so the file-derived candidate set a `path-pattern` builds can never select these overlays; the interpreter evaluates the full `agent_teams` fact family directly.

CORE-008 is the landing path for the `content-digest-eq` deterministic hint kind: it checks that each `agent-teams.md` overlay's resolved content digest matches the canonical document. Every `agent-teams.md` overlay in the framework repo already resolves to the canonical document, so the rule fires zero findings against the current tree and surfaces only on drift.

## Look For

- A review brief whose `agent-teams.md` symlink was repointed at a forked or stale copy of the review-team protocol during a refactor, so its resolved content digest no longer matches the canonical document.
- A copy-pasted adapter `references/` tree whose `agent-teams.md` link target was rewritten to a sibling document rather than the shared canonical path.
- A broken `agent-teams.md` symlink whose target cannot be read, leaving no content digest to compare against the canonical value.

## Fix

Repoint the overlay's `agent-teams.md` symlink at the shared `adapters/shared/references/runtime/review-team-protocol.md` overlay (a relative symlink, matching the sibling adapters' overlays) so it resolves to the canonical `docs/reference/review-team-protocol.md` document and its resolved content digest matches the canonical digest. If the review-team protocol genuinely needs to change, edit the canonical document once; every overlay inherits the new content through its symlink and the digests stay aligned automatically.
