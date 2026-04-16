# RFC-1a: Deferred Validation

> Status: Draft · Parent: [RFC-1](rfc-1-cli.md)

## Abstract

Define the three-way Pass/Fail/Deferred classification for CLI validation results. This is the architectural mechanism that lets the CLI handle structural checks while the agent evaluates semantic ones — the core of the inversion-of-control model.

## Motivation

Deterministic frameworks (OpenSpec, SpecKit) can only do `Pass`/`Fail` because they have no agent to handle ambiguity. They either over-reject (blocking on rules they can't evaluate) or under-validate (skipping semantic rules entirely). Specify's model says: the CLI handles the structural checks, flags what it can't evaluate, and the agent applies judgment on the remainder. The agent's prompt surface for validation shrinks from "evaluate these 15 rules against this artifact" to "evaluate these 3 deferred rules that the CLI couldn't check."

This pattern generalises. Any time you're tempted to add a complex heuristic to the CLI, ask: "Is this better as a `Deferred` result that the agent evaluates?" The CLI should be conservative — it's better to defer a check to the agent than to implement a brittle heuristic that produces false positives.

## Detailed Design

### Classification Is Declared, Not Inferred

Rules are hardcoded per brief id in `specify_validate::rules_for` (see [RFC-1](rfc-1-cli.md) `validate.rs` in `crates/specify-validate`). Each registered `Rule` carries an explicit `Classification::Structural` or `Classification::Semantic` tag alongside its checker, so the CLI never has to pattern-match on human-readable rule strings to decide what it can handle. This is a deliberate departure from deterministic frameworks that infer classification from rule prose — declaring it at the definition site eliminates a class of "the CLI silently passed a rule it didn't actually evaluate" bugs.

Representative entries for the current brief set:

| Brief | Rule description | Classification | How the CLI decides |
|---|---|---|---|
| `proposal` | Has a `Why` section with at least one sentence | Structural | `has_content_after_heading` |
| `proposal` | Has a `Crates`/`Features` section listing at least one entry | Structural | `has_content_after_heading` |
| `proposal` | Uses imperative language for motivation | Semantic | always `Deferred` |
| `specs` | Every requirement has at least one scenario | Structural | `all_requirements_have_scenarios` |
| `specs` | IDs use the `REQ-[0-9]{3}` format | Structural | `ids_match_pattern` |
| `specs` | Uses SHALL/MUST language for normative requirements | Semantic | always `Deferred` |
| `design` | References only requirement ids present in specs | Structural | `design_references_exist` |
| `tasks` | All tasks use checkbox format | Structural | `all_tasks_use_checkbox` |
| `tasks` | Tasks grouped under headings | Structural | `tasks_grouped_under_headings` |
| cross | Proposal deliverables have matching spec files | Structural | `proposal_deliverables_have_specs` |

Semantic rules always emit `Deferred { reason: ... }`; their checker function is never called. A brief id with no registry entry yields no brief-scoped rules — only the generic checks (artifact exists, parses) run, so unknown brief types degrade gracefully rather than crashing the validator.

### What the Agent Receives

After running `specify validate`, the skill receives a structured report. Its responsibility is limited to:

1. Reporting `Fail` results to the user with suggested fixes.
2. Evaluating `Deferred` results using semantic understanding.
3. Deciding whether to proceed, fix, or ask for guidance.

The agent never has to count sections, verify ID patterns, or check dependency graphs. These are the operations most prone to LLM error and are now handled by the CLI.

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — parent RFC; `validate.rs` implements this classification via the hardcoded `rules_for` registry
