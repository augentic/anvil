
# "Deferred" Validation

The most important architectural decision in the CLI is the three-way classification of validation results: `Pass`, `Fail`, `Deferred`. This is what makes the inversion-of-control model work in practice.

Deterministic frameworks (OpenSpec, SpecKit) can only do `Pass`/`Fail` because they have no agent to handle ambiguity. They either over-reject (blocking on rules they can't evaluate) or under-validate (skipping semantic rules entirely). Specify's model says: the CLI handles the structural checks, flags what it can't evaluate, and the agent applies judgment on the remainder. The agent's prompt surface for validation shrinks from "evaluate these 15 rules against this artifact" to "evaluate these 3 deferred rules that the CLI couldn't check."

This pattern generalises. Any time you're tempted to add a complex heuristic to the CLI, ask: "Is this better as a `Deferred` result that the agent evaluates?" The CLI should be conservative — it's better to defer a check to the agent than to implement a brittle heuristic that produces false positives.

## Classification Heuristic

For each validation rule string in `schema.yaml`, the CLI applies a pattern-matching heuristic to decide whether it can handle the rule deterministically:

| Rule pattern | Classification | Example |
|---|---|---|
| "Has a X section" | Structural — check heading exists | `Pass`/`Fail` |
| "Has a X section with at least one Y" | Structural — check heading + content | `Pass`/`Fail` |
| "Every requirement has at least one scenario" | Structural — parsed spec check | `Pass`/`Fail` |
| "Uses X format" (WHEN/THEN, checkbox, etc.) | Structural — regex check | `Pass`/`Fail` |
| "IDs use the REQ-XXX format" | Structural — regex check | `Pass`/`Fail` |
| "Uses SHALL/MUST language" | Semantic — requires NLP | `Deferred` |
| "Crate names are kebab-case" | Structural — regex check | `Pass`/`Fail` |

Rules that don't match any known pattern default to `Deferred`. This ensures the CLI never silently passes a rule it doesn't understand.

## What the Agent Receives

After running `specify validate`, the skill receives a structured report. Its responsibility is limited to:

1. Reporting `Fail` results to the user with suggested fixes.
2. Evaluating `Deferred` results using semantic understanding.
3. Deciding whether to proceed, fix, or ask for guidance.

The agent never has to count sections, verify ID patterns, or check dependency graphs. These are the operations most prone to LLM error and are now handled by the CLI.