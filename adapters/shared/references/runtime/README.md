# Shared spec runtime bundle

This directory is the single **spec-runtime bundle**: a tree of relative symlinks pointing at the canonical references under `plugins/spec/references/`. Each source and target adapter exposes it as `references/spec-runtime/` via a single directory symlink (`adapters/{sources,targets}/<name>/references/spec-runtime -> ../../../shared/references/runtime`), so adapter briefs can link with `../references/spec-runtime/...` without escaping the adapter tree. `specify init` dereferences the symlinks when it vendors the bundle into each cached adapter, so consumer projects receive self-contained regular files.

There are no generated copies and no sync step: a symlink can never drift from its target. Edit the canonical file under `plugins/spec/references/` and every adapter sees the change immediately.

| Bundle path (symlink) | Canonical target |
| --- | --- |
| `guardrails.md` | `plugins/spec/references/guardrails.md` |
| `specialist-usage.md` | `plugins/spec/references/specialist-usage.md` |
| `reconciliation.md` | `plugins/spec/references/reconciliation.md` |
| `components.md` | `plugins/spec/references/components.md` |
| `standards-layer-snippet.md` | `plugins/spec/references/standards-layer-snippet.md` |
| `artifact-validation-checklist.md` | `plugins/spec/references/artifact-validation-checklist.md` |
| `phase-outcome-contract.md` | `plugins/spec/references/phase-outcome-contract.md` |
| `plan-lock.md` | `plugins/spec/references/plan-lock.md` |
| `stop-conditions.md` | `plugins/spec/references/stop-conditions.md` |
| `spec-to-test-mapping.md` | `plugins/spec/references/spec-to-test-mapping.md` |
| `cli/plan-propose.md` | `plugins/spec/references/cli/plan-propose.md` |
| `synthesis/authority.md` | `plugins/spec/references/synthesis/authority.md` |
| `synthesis/tags.md` | `plugins/spec/references/synthesis/tags.md` |
| `synthesis/provenance.md` | `plugins/spec/references/synthesis/provenance.md` |
| `synthesis/claim-reconciliation.md` | `plugins/spec/references/synthesis/claim-reconciliation.md` |

Top-level symlinks use four `../` segments; `cli/` and `synthesis/` entries use five (they sit one level deeper).

## Editing rules

- Edit the canonical file under `plugins/spec/references/`. Never replace a bundle entry with a regular-file copy, and never replace an adapter's `references/spec-runtime` symlink with a directory of copies — CI's "Verify spec-runtime symlinks resolve" step fails on either regression.
- Adding a new shared reference: drop the canonical file under `plugins/spec/references/`, then add one symlink here. Every adapter inherits it through its directory symlink automatically.
- Do not add agent-critical prose only under `docs/`; the spec-runtime bundle resolves into `plugins/spec/references/`, which is the surface the Cursor plugin cache and `specify init` ship.

## Review-team protocol

The review-team protocol is a separate surface and is **not** part of the spec-runtime bundle above (it resolves into `docs/`, not `plugins/spec/references/`). It is exposed here as a single overlay symlink, `review-team-protocol.md -> ../../../../docs/reference/review-team-protocol.md`, and each target adapter exposes it as `references/agent-teams.md -> ../../../shared/references/runtime/review-team-protocol.md`. Overlays MUST be symlinks (regular-file copies are forbidden); `CORE-011` guards the canonical document's presence and CI's symlink check verifies every overlay resolves to it.
