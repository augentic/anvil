# Shared engineering standards (UNI-\*)

Shared **engineering standards** catalog — target-agnostic codex rules under `adapters/shared/`. Codex is the on-disk rule format; these files are durable policy, not workflow state or slice artifacts. Read by every target adapter's build review brief during `/spec:build` and (when implemented) by `specrun lint` for deterministic CI enforcement. Findings cite a rule here as a stable `rule_id` (for example `UNI-014`) alongside a report-local occurrence id (for example `UNI-3`) in `REVIEW.md`.

See [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) for how engineering standards relate to workflow, artifacts, and `docs/standards/` (authoring house style).

This directory owns the `UNI-*` namespace. Target-specific rules live in per-adapter overlays under `adapters/targets/<name>/codex/` (omnia: `OMNIA-*` / `RUST-*` / `SEC-*`; contracts: `IFACE-*`; vectis: `VECTIS-*`). Source-adapter overlays under `adapters/sources/<name>/codex/` share a single namespace, `SRC-*`: every source-adapter owner maps to `{"SRC"}` in `check::codex`'s namespace map per [RFC-28 §Namespaces](../../../../rfcs/done/rfc-28-standards-contract.md#namespaces), so any new source adapter that grows an overlay opts into `SRC-*` without coordinating a per-adapter namespace. `FRAME-*` is reserved for [RFC-32](../../../../rfcs/done/rfc-32-standards-enforcement.md) Phase 3 declarative framework rules and MUST NOT appear under `adapters/*/codex/`. Namespace ownership is enforced by `specdev check`.

Sibling shared hook directory: [`../../target-hooks/replay/`](../../target-hooks/replay/) — shared build-time replay hook contract for targets that opt in.

## Rule inventory

Rules are grouped by severity (highest first). `UNI-*` ids are stable citation keys — they are not renumbered when severity or grouping changes.

### Critical

| ID      | File                                                           |
| ------- | -------------------------------------------------------------- |
| UNI-002 | [`unvalidated-input.md`](unvalidated-input.md)                 |
| UNI-006 | [`concurrency-issues.md`](concurrency-issues.md)               |
| UNI-010 | [`unhandled-exceptions.md`](unhandled-exceptions.md)           |
| UNI-018 | [`hardcoded-secrets.md`](hardcoded-secrets.md)                 |
| UNI-019 | [`injection-vulnerabilities.md`](injection-vulnerabilities.md) |
| UNI-020 | [`unsafe-deserialization.md`](unsafe-deserialization.md)       |
| UNI-021 | [`missing-auth.md`](missing-auth.md)                           |

### Important

| ID      | File                                                                   |
| ------- | ---------------------------------------------------------------------- |
| UNI-001 | [`uninitialised-defaults.md`](uninitialised-defaults.md)               |
| UNI-003 | [`serialization-failures.md`](serialization-failures.md)               |
| UNI-004 | [`logic-bugs.md`](logic-bugs.md)                                       |
| UNI-005 | [`resource-leaks.md`](resource-leaks.md)                               |
| UNI-007 | [`chatty-external-calls.md`](chatty-external-calls.md)                 |
| UNI-008 | [`instrumentation-issues.md`](instrumentation-issues.md)               |
| UNI-009 | [`handle-then-throw.md`](handle-then-throw.md)                         |
| UNI-011 | [`missing-timeout-retry.md`](missing-timeout-retry.md)                 |
| UNI-012 | [`persisted-state-compatibility.md`](persisted-state-compatibility.md) |
| UNI-014 | [`hardcoded-configuration.md`](hardcoded-configuration.md)             |
| UNI-015 | [`stale-closure-captures.md`](stale-closure-captures.md)               |
| UNI-017 | [`type-safety-erosion.md`](type-safety-erosion.md)                     |

### Suggestion

| ID      | File                                                   |
| ------- | ------------------------------------------------------ |
| UNI-013 | [`dead-code.md`](dead-code.md)                         |
| UNI-016 | [`error-message-quality.md`](error-message-quality.md) |

## File shape

Each rule is a small markdown file with YAML frontmatter followed by a required `## Rule` heading. The canonical schema lives in the `augentic/specify-cli` workspace at `crates/authoring/schemas/codex-rule.schema.json`; see [`docs/contributing/checks.md`](../../../../docs/contributing/checks.md) for how `specdev check` consumes it. An editor-side mirror at [`.cursor/schemas/codex-rule.schema.json`](../../../../.cursor/schemas/codex-rule.schema.json) keeps Cursor's JSON language server aligned with the same shape; the two are to be kept byte-identical by `specdev check`'s `codex.schema-drift` predicate (lands with RFC-28 Phase 2). The minimum form:

```markdown
---
id: UNI-NNN
title: Short human title
severity: critical | important | suggestion | optional
trigger: One-sentence condition that tells a reviewer when this rule matters.
---

## Rule

What the rule actually requires, in prose.

## Look For

- Concrete code patterns or smells that hint the rule is being violated.
```

Optional frontmatter fields (`applicability`, `lint_mode`, `deterministic_hints`, `references`, `deprecated`) are documented in the schema. `id` must be globally unique across every codex tree the checker discovers.

## How rules are consumed

Target review briefs read this directory directly and apply each rule with target-specific heuristics:

- **Omnia** — [`adapters/targets/omnia/briefs/build/review.md`](../../../targets/omnia/briefs/build/review.md) phase 3 ("Universal checks (lead)") applies every `UNI-*` rule in the inventory above, skipping rules already covered by the SEC / COR / QUA specialists per the table in [`review-categories.md`](../../../targets/omnia/references/review-categories.md).
- **Vectis** — [`adapters/targets/vectis/references/review/universal-checks.md`](../../../targets/vectis/references/review/universal-checks.md) lists the Crux/Rust heuristics for each `UNI-*` and the overlaps to skip.
- **Contracts** — [`docs/reference/targets/contracts.md`](../../../../docs/reference/targets/contracts.md) cites its overlay alongside this shared set.

A review finding always carries:

- a report-local occurrence id (`UNI-1`, `UNI-2`, …) that restarts in each `REVIEW.md`, and
- a stable `rule_id` (`UNI-014`, `OMNIA-002`, …) that cites the codex file.

Adapter overlays are preferred over the shared rule when both match — e.g. a hardcoded secret in Omnia handler code maps to `SEC-001`, not `UNI-018`.

## Adding or evolving rules

1. Pick the next free `UNI-NNN`. Do not reuse retired ids; mark old rules with a `deprecated:` block in the frontmatter and keep the file so historical citations still resolve.
2. Create the file with the frontmatter and `## Rule` heading shown above.
3. Wire the new id into any target review references that should apply it (Omnia [`review-categories.md`](../../../targets/omnia/references/review-categories.md), Vectis [`universal-checks.md`](../../../targets/vectis/references/review/universal-checks.md), etc.) — `make check` does **not** verify that every consumer cites every rule, so coverage is a manual concern.
4. Run `make check` (which forwards to `specdev check --framework-root .`). The relevant predicate is `check::codex` in the `specify-authoring` crate, which enforces frontmatter validity, the `## Rule` body heading, namespace ownership, and id uniqueness across the shared tree and every per-adapter overlay.

`README.md` files (case-insensitive) under any codex directory are skipped by the discovery walk and are reserved for index pages like this one — they are never validated as rules.
