# Framework convergence rules (`CORE-*`)

First-party rules that enforce framework-repository invariants through the shared deterministic-hint interpreter. The pack root activates the second shared resolution root (`adapters/shared/rules/<pack>/`) with pack name `core`; resolved rules carry `origin: core`. `CORE-*` rules participate in `specify lint framework` runs by default and are excluded from consumer-side `specify rules export` / `specify lint` unless the operator passes `--include-core`.

This directory is the peer of [`adapters/shared/rules/universal/`](../universal/README.md): same file shape, same JSON Schema, different namespace ownership. `CORE-*` is the only namespace allowed under `adapters/shared/rules/core/`; the placement predicate in `check::rules` rejects any non-`CORE-*` rule placed here and any `CORE-*` rule placed elsewhere.

See [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) for how engineering standards relate to workflow, artifacts, and `docs/standards/` (authoring house style).

## File shape

Each rule is a small markdown file with YAML frontmatter and a required `## Rule` body — same shape as `UNI-*`, validated against the canonical `rule.schema.json` distributed by the CLI (editor-mirrored at [`.cursor/schemas/rule.schema.json`](../../../../.cursor/schemas/rule.schema.json)). The `id` follows the `CORE-NNN` pattern; the filename mirrors the id and the kebab-case title (for example `CORE-001-adapter-schema.md`).

```markdown
---
id: CORE-NNN
title: Short human title
severity: critical | important | suggestion | optional
trigger: One-sentence condition that tells a reviewer when this rule matters.
applicability:
  artifacts:
    - <one of the framework artifact tokens listed below>
deterministic_hints:
  - kind: schema | path-pattern | regex | tool
    value: <kind-specific payload>
    description: Optional human explanation.
---

## Rule

Canonical agent-readable explanation: what the rule enforces, why it matters, and what to fix when it fires.

## Look For

- Concrete patterns that hint the rule is being violated.

## Fix

What to change to clear the finding.
```

## Applicability tokens

The closed `applicability.artifacts` enum carries framework-side tokens alongside the consumer-side set. Prefer the narrowest fit:

| Token       | Targets                                           |
| ----------- | ------------------------------------------------- |
| `skill`     | `plugins/**/SKILL.md` (frontmatter + body)        |
| `adapter`   | `adapters/**/adapter.yaml` manifests              |
| `brief`     | `adapters/**/briefs/*.md`                         |
| `reference` | `adapters/**/references/*.md`                     |
| `codex`     | `adapters/**/rules/*.md` (rule files themselves)  |
| `doc`       | `docs/**/*.md`                                    |

Framework tokens compose with the existing consumer-side tokens (`code`, `tests`, `contracts`, `specs`, `design`, `tasks`); a single rule can list both sides.

**Chassis quirk — prefer `path-pattern` over `applicability.artifacts` until further notice.** The framework-profile resolver passes `include_unmatched: false` into `artifact_dimension_matches`, which drops any rule that declares a populated `applicability.artifacts` set from the resolved output before hints run. Until the chassis flips that behaviour for the framework profile (or wires artifact-kind facts off `WorkspaceModel`), leave `applicability.artifacts` unset and narrow the candidate file set with a `kind: path-pattern` deterministic hint instead (see [`CORE-001-adapter-schema.md`](CORE-001-adapter-schema.md) for the worked example). Revisit once a chassis follow-up enabling artifact-token filtering for the framework profile lands.

## Hint-kind preference

Every v1 hint kind is now executable: `path-pattern`, `schema`, `regex`, `tool`, `reference-resolves`, `unique`, `set-coverage`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, and `namespace-owner`. No kind carries an `"x-hint-status": "reserved"` annotation in the canonical `rule.schema.json` distributed by the CLI (editor-mirrored at [`.cursor/schemas/rule.schema.json`](../../../../.cursor/schemas/rule.schema.json)) anymore. The reserved-kind machinery survives in the interpreter as forward-compat scaffolding, so if a future kind ever lands shape-only ahead of its interpreter, pick it only when authoring a new rule alongside that interpreter in the same change; otherwise an authored hint will fail evaluation until its kind lands.

## Authoring conventions

1. Pick the next free `CORE-NNN`. Do not reuse retired ids; mark deprecated rules with a `deprecated:` block and leave the file in place so historical citations resolve.
2. Mirror an existing rule (start from [`CORE-001-adapter-schema.md`](CORE-001-adapter-schema.md)) for the frontmatter shape; the schema is the source of truth.
3. Add the rule, then run `make lint`. `specify lint framework` resolves the new file and exercises its hints across the framework tree; investigate any findings before opening the PR.
4. If retiring an imperative `Check` row alongside the rule, land the parity test at `crates/standards/tests/core_parity_<rule>.rs` in `augentic/specify-cli` and delete the predicate row in the same PR; the existing fingerprint algorithm collapses duplicate findings during the overlap.
5. Pair each new `CORE-*` rule with the existing imperative predicate it replaces by consulting the **predicate migration map** in the standards-enforcement decision record (filed in the design-history tree). The map names which `Check` row each reserved hint kind is intended to retire; rules without a mapped predecessor are still legal but should land with a smoke-test fixture rather than a parity test.

## References

- [Shared engineering standards (`UNI-*`)](../universal/README.md) — sibling pack; same file shape, different namespace ownership.
- [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) — how workflow, artifacts, and engineering standards compose.
- [docs/contributing/checks.md](../../../../docs/contributing/checks.md) — when to author a new imperative `Check` versus a declarative `CORE-*` rule.
