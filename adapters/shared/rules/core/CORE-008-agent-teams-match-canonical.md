---
id: CORE-008
title: Agent Teams Match Canonical
severity: important
trigger: A framework `agent-teams.md` symlink overlay resolves to content whose SHA-256 digest does not equal the canonical `docs/reference/review-team-protocol.md` review-team-protocol document, so a review brief points at drifted or stale review-team guidance.
deterministic_hints:
  - kind: content-digest-eq
    value: agent-teams-match-canonical
    description: For each followed `agent-teams.md` symlink fact, assert that the resolved target's SHA-256 equals the canonical `docs/reference/review-team-protocol.md` digest. One finding per symlink whose target digest diverges, with the `(resolved-target, expected-digest, actual-digest)` shape surfaced as structured evidence.
---

## Rule

Target adapter review briefs do not carry their own copy of the review-team protocol; each `adapters/targets/<name>/references/agent-teams.md` is a relative symlink into the single canonical `docs/reference/review-team-protocol.md` document. The framework keeps one source of truth so review-team guidance never forks per adapter. This rule asserts the content invariant behind that arrangement: every `agent-teams.md` overlay must resolve to content whose SHA-256 digest equals the canonical document's digest.

The deterministic-hint interpreter consumes the `AgentTeam` facts the framework-profile indexer already produced by following each `agent-teams.md` symlink and recording the resolved endpoint plus a SHA-256 of the target's bytes (`crates/lints/src/lint/index/agent_teams.rs::record`). The expected canonical digest is derived from the fact set itself — the digest carried by any overlay that resolves to `docs/reference/review-team-protocol.md`. Every overlay whose recorded digest differs from that expected value (a symlink redirected at a different document, or a broken link with no readable target) is flagged. When no overlay resolves to the canonical document the expected digest cannot be established and the rule stays silent; the invariant is vacuous without an anchor.

This rule narrows on a fact family rather than a file glob, so it carries no `path-pattern` hint. The framework walker records an `agent-teams.md` symlink as an `AgentTeam` fact and emits no `file` fact for the symlink path, so the file-derived candidate set a `path-pattern` builds can never select these overlays; the interpreter evaluates the full `agent_teams` fact family directly.

No imperative `Check` row is retired by this rule. The hand-written `check::agent_teams` predicate enforces a *path*-equality invariant (the symlink must canonicalize to the canonical document's path) and additionally owns the regular-file, missing-canonical, and unsupported-entry branches that produce no `AgentTeam` fact. Those branches are structurally invisible to a fact-iterating digest evaluator, and path equality is a stricter check than content-digest equality, so this rule is the smoke-test landing path for the `content-digest-eq` deterministic hint kind, not a replacement for the predicate. Every `agent-teams.md` overlay in the framework repo already resolves to the canonical document, so the rule fires zero findings against the current tree and surfaces only on drift.

## Look For

- A review brief whose `agent-teams.md` symlink was repointed at a forked or stale copy of the review-team protocol during a refactor, so its resolved content digest no longer matches the canonical document.
- A copy-pasted adapter `references/` tree whose `agent-teams.md` link target was rewritten to a sibling document rather than the shared canonical path.
- A broken `agent-teams.md` symlink whose target cannot be read, leaving no content digest to compare against the canonical value.

## Fix

Repoint the overlay's `agent-teams.md` symlink at the canonical `docs/reference/review-team-protocol.md` document (a relative symlink, matching the sibling adapters' overlays) so its resolved content digest matches the canonical digest. If the review-team protocol genuinely needs to change, edit the canonical document once; every overlay inherits the new content through its symlink and the digests stay aligned automatically.
