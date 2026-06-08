# Shared engineering standards (UNI-\*)

Shared **engineering standards** catalog — target-agnostic rules under `adapters/shared/`. Codex is the on-disk rule format; these files are durable policy, not workflow state or slice artifacts. Read by every target adapter's build review brief during `/spec:build` and (when implemented) by `specify lint` for deterministic CI enforcement. Findings cite a rule here as a stable `rule_id` (for example `UNI-014`) alongside a report-local occurrence id (for example `UNI-3`) in `REVIEW.md`.

See [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) for how engineering standards relate to workflow, artifacts, and `docs/standards/` (authoring house style).

This directory owns the `UNI-*` namespace. Target-specific rules live in per-adapter overlays under `adapters/targets/<name>/rules/` (omnia: `OMNIA-*` / `RUST-*` / `SEC-*`; contracts: `IFACE-*`; vectis: `VECTIS-*`). Source-adapter overlays under `adapters/sources/<name>/rules/` share a single namespace, `SRC-*`: every source-adapter owner maps to `{"SRC"}` in `check::rules`'s namespace map by the `check::rules` namespace map, so any new source adapter that grows an overlay opts into `SRC-*` without coordinating a per-adapter namespace. `FRAME-*` is reserved for declarative framework rules and MUST NOT appear under `adapters/*/rules/`. Namespace ownership is enforced by `specify lint framework`.

Sibling shared hook directory: [`../../target-hooks/replay/`](../../target-hooks/replay/) — shared build-time replay hook contract for targets that opt in.

## Rule inventory

Rules are grouped by severity (highest first). `UNI-*` ids are stable citation keys — they are not renumbered when severity or grouping changes.

**Enforcement mode (review-only vs deterministically exported).** Every `UNI-*` rule is applied as a **model-assisted review finding** by the target build review briefs during `/spec:build`; none gate deterministically by default. The single exception today is **UNI-014** ([`hardcoded-configuration.md`](hardcoded-configuration.md)), which additionally carries deterministic `rule_hints` (a `path-pattern` + `regex` pair) and is therefore the only id exported to `specify lint` as a deterministic check. Treat every other id as review-only until it grows its own `rule_hints` block.

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
| UNI-022 | [`ignore-directive-missing-rationale.md`](ignore-directive-missing-rationale.md) |
| UNI-023 | [`ignore-directive-orphan.md`](ignore-directive-orphan.md)             |

### Suggestion

| ID      | File                                                   |
| ------- | ------------------------------------------------------ |
| UNI-013 | [`dead-code.md`](dead-code.md)                         |
| UNI-016 | [`error-message-quality.md`](error-message-quality.md) |

## File shape

Each rule is a small markdown file with YAML frontmatter followed by a required `## Rule` heading. The canonical schema lives in the `augentic/specify-cli` workspace at `schemas/rules/rule.schema.json` and is embedded in the CLI binary; see [`docs/contributing/checks.md`](../../../../docs/contributing/checks.md) for how `specify lint framework` consumes it. The minimum form:

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

Optional frontmatter fields (`applicability`, `lint_mode`, `rule_hints`, `references`, `deprecated`) are documented in the schema. `id` must be globally unique across every codex tree the checker discovers.

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
3. Wire the new id into any target review references that should apply it (Omnia [`review-categories.md`](../../../targets/omnia/references/review-categories.md), Vectis [`universal-checks.md`](../../../targets/vectis/references/review/universal-checks.md), etc.) — `make lint` does **not** verify that every consumer cites every rule, so coverage is a manual concern.
4. Run `make lint` (which forwards to `specify lint framework`). The relevant predicate is `framework::check::rules` in `specify-standards`, which enforces frontmatter validity, the `## Rule` body heading, namespace ownership, and id uniqueness across the shared tree and every per-adapter overlay.

`README.md` files (case-insensitive) under any codex directory are skipped by the discovery walk and are reserved for index pages like this one — they are never validated as rules.
