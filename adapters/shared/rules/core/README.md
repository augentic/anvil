# Framework convergence rules (`CORE-*`)

First-party rules that enforce framework-repository invariants through the shared deterministic-hint interpreter. The pack root activates the second shared resolution root (`adapters/shared/rules/<pack>/`) with pack name `core`; resolved rules carry `origin: core`. `CORE-*` rules participate in `specify lint framework` runs by default and are excluded from consumer-side `specify rules export` / `specify lint` unless the operator passes `--include-core`.

This directory is the peer of [`adapters/shared/rules/universal/`](../universal/README.md): same file shape, same JSON Schema, different namespace ownership. `CORE-*` is the only namespace allowed under `adapters/shared/rules/core/`; the `rules` WASI tool (CORE-009) rejects any non-`CORE-*` rule placed here and any `CORE-*` rule placed elsewhere.

See [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) for how engineering standards relate to workflow, artifacts, and `docs/standards/` (authoring house style).

## File shape

Each rule is a small markdown file with YAML frontmatter and a required `## Rule` body — same shape as `UNI-*`, validated against the canonical `rule.schema.json` embedded in the CLI binary. The `id` follows the `CORE-NNN` pattern; the filename mirrors the id and the kebab-case title (for example `CORE-001-adapter-schema.md`).

```markdown
---
id: CORE-NNN
title: Short human title
severity: critical | important | suggestion | optional
trigger: One-sentence condition that tells a reviewer when this rule matters.
applicability:
  artifacts:
    - <one of the framework artifact tokens listed below>
rule_hints:
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

**Authoring checklist (avoid the `applicability.artifacts` footgun):**

- [ ] **Do not** rely on `applicability.artifacts` to scope a framework rule — on the framework profile it silently drops the rule from the resolved set before any hint runs.
- [ ] **Do** add a `kind: path-pattern` hint whose `value` glob matches the target files (e.g. `plugins/**/SKILL.md`, `adapters/**/adapter.yaml`).
- [ ] Run `make lint` and confirm the new rule actually fires on a known-bad fixture; a rule that resolves but matches nothing is the usual symptom of the quirk.
- [ ] Cross-check against [`CORE-001-adapter-schema.md`](CORE-001-adapter-schema.md), which scopes with `path-pattern` rather than `applicability.artifacts`.

[`CORE-054-rule-applicability-artifacts.md`](CORE-054-rule-applicability-artifacts.md) enforces this checklist: it fails `make lint` when any `CORE-*` rule declares a populated `applicability.artifacts` set (the degenerate empty `artifacts: []` form is admitted). Until the chassis flips the framework-profile behaviour, that guard is the backstop against silently shipping a dead rule.

## Hint-kind preference

Every v1 hint kind is executable: `path-pattern`, `schema`, `regex`, `tool`, `unique`, `reference-resolves`, `set-coverage`, `cardinality`, `constant-eq`, `fenced-block`, `presence`, `field-grammar`, `cross-reference`, and `cli-contract`. Prefer native declarative kinds for new rules; reach for `kind: tool` (a referenced WASI tool) only when a check is branchy, whole-tree, cross-fact, or registry-backed. No kind carries `"x-hint-status": "reserved"` in the canonical `rule.schema.json`.

The three relational / presence kinds dispatch on a `value:` mechanism selector with policy in `config:`: `presence` (`frontmatter`, `file`, `markdown-section`, `directory-index`) for a missing required artifact, `field-grammar` (`field-tokens`, `field-first-word`) for a frontmatter field grammar, and `cross-reference` (`adapter-dir` / `expected-set` source against an `adapter-manifest` target) for a relational set-difference / value-equality join. The `schema` and `unique` kinds additionally accept a whole-tree `value: scenario` selector over the scenario fact family. These serve `presence` → CORE-042 / CORE-011 / CORE-041 / CORE-059, `field-grammar` → CORE-035 / CORE-036, `cross-reference` → CORE-010, `schema` scenario → CORE-032, and `unique` scenario → CORE-030. The `cli-contract` kind (`invocations` / `event-ids` / `error-codes` / `test-citations` selectors over the binary-injected CLI contract) serves CORE-057 and CORE-060.

**Lint posture.** `specify lint framework` is a generic dispatcher running entirely through declarative hints (Road A) and name-resolved framework checkers (Road B) — there is no imperative `Check` rule producer and no imperative-predicate bridge. Whole-tree and branchy checks (CORE-009 namespace ownership, CORE-026 duplicate id, and the `scenarios` / `skill-body` / `links-registry` / `marketplace` / `prose` families) run through their `kind: tool` family checker; all policy rides the rule's `config:`. Benchmark locally with `/usr/bin/time make lint`.

### Hint config cookbook (native rules)

`config:`-driven evaluators carry rule policy out of the engine. Examples: `regex` accepts optional `config` (capture-group threshold, negative-match, suffix guard), `path-pattern` `value`s accept `!` exclusion globs, and the fact-consuming kinds (`cardinality`, `set-coverage`, `constant-eq`, `unique`, `fenced-block`) read their cap / set / map / constant from `config:`. The canonical `config:` shape for each kind is pinned by the `$def`s in `schemas/rules/rule.schema.json` (embedded in the `specify` binary); see existing `CORE-*` rule files for worked examples.

## Authoring conventions

1. Pick the next free `CORE-NNN`. Do not reuse retired ids; mark deprecated rules with a `deprecated:` block and leave the file in place so historical citations resolve.
2. Mirror an existing rule (start from [`CORE-001-adapter-schema.md`](CORE-001-adapter-schema.md)) for the frontmatter shape; the schema is the source of truth.
3. Add the rule, then run `make lint`. `specify lint framework` resolves the new file and exercises its hints across the framework tree; investigate any findings before opening the PR. Confirm the rule actually fires against a known-bad fixture — a rule that resolves but matches nothing is the usual failure mode.
4. Keep all policy in the rule's `config:`. The engine's `lint_no_embedded_policy` Layer-3 guard rejects any rule-specific literal that creeps into the dispatcher. For a Road B (`kind: tool`) rule, add the check to the in-process family checker under `crates/standards/src/lint/framework_tools/<name>.rs` in `specify-cli` (see [docs/contributing/checks.md](../../../../docs/contributing/checks.md)).
5. New engine behaviour is covered by mechanism-named, rule-agnostic tests (`crates/standards/tests/lint_hint_<kind>.rs`) and each tool's in-crate tests — not by per-rule parity tests.

## Rule families and overlapping concerns

Two concerns are split across several cooperating `CORE-*` rules. They are intentionally separate (each is a distinct, line-scoped failure mode), but knowing the family up front lowers the learning cost when one fires.

**Agent-teams overlay integrity** — keeps every target adapter's `agent-teams.md` symlink overlay faithful to the canonical `docs/reference/review-team-protocol.md`:

| Rule | Title | Role |
| --- | --- | --- |
| CORE-011 | Agent Teams Missing Canonical | The canonical document itself must exist, otherwise no overlay can resolve (presence guard). |

Overlays MUST be symlinks — regular-file `agent-teams.md` overlays are forbidden, so there is no copied content to drift. CI's symlink check verifies every overlay resolves to the canonical document; the retired CORE-008 (digest pin) and CORE-012 (`agent-teams` Road B tool) duplicated that guarantee and were removed.

**Link and reference resolution** — keeps cross-document references from rotting, each scoped to a different surface:

| Rule | Title | Role |
| --- | --- | --- |
| CORE-002 | Markdown Links Resolve | Generic `[label](target)` relative links resolve on disk. |
| CORE-018 | Links Brief Schema Link Resolve | Adapter briefs cite only known `schemas.specify.dev` tool-schema URLs. |
| CORE-019 | Links Broken Reference | `SKILL.md` references to bundled `references/` / `examples/` paths exist. |
| CORE-020 | Links Unresolved Directive | Skill directive paths resolve. |

When editing one member of a family, check whether the sibling rules need a matching update — for example, moving the review-team-protocol document touches CORE-011 and the CI symlink check at once.

## References

- [Shared engineering standards (`UNI-*`)](../universal/README.md) — sibling pack; same file shape, different namespace ownership.
- [docs/explanation/standards-layer.md](../../../../docs/explanation/standards-layer.md) — how workflow, artifacts, and engineering standards compose.
- [docs/contributing/checks.md](../../../../docs/contributing/checks.md) — how to extend framework checks: Road A declarative hints versus Road B referenced WASI tools.
