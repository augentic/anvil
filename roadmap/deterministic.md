> **Superseded** — This document has been incorporated into [`roadmap/roadmap.md`](roadmap.md), the unified roadmap for determinism and RWL iteration. Refer to that document for current plans.

# Making Specify More Deterministic

Recommendations for striking the right balance between natural language skills,
YAML-based configuration, and a Rust engine.

## Where Determinism Lives Today

| Layer | Mechanism | Determinism |
|---|---|---|
| **Schema structure** | `schema.yaml` + `schema.schema.json` + Ajv validation in `checks.ts` | Strong — blueprint DAG, required fields, referential integrity are machine-checked |
| **Spec merge** | `merge-specs.ts` — parses headings, applies RENAMED→REMOVED→MODIFIED→ADDED in strict order | Strong — the one truly deterministic operation on artifact content |
| **Spec format** | `spec-format.md` — `### Requirement:`, `ID: REQ-XXX`, `#### Scenario:` | Medium — conventions are documented, but enforcement is LLM-interpreted except in merge |
| **Workflow state** | `.metadata.yaml` status enum, filesystem presence checks | Medium — the state machine is implicit in skill prose |
| **Validation rules** | `validate:` arrays in `schema.yaml` | Weak — human-readable sentences interpreted by the LLM at generation time |
| **Artifact generation** | Instruction markdown + `defaults.rules` | Weak — templates with HTML comment placeholders, but output is fully LLM-generated |
| **Cross-artifact checks** | `validation:` flags like `proposal-crates-have-specs` | Weak — boolean flags that trigger LLM-interpreted checks, no parser |

## The Non-Determinism Problem

The validate rules in `schema.yaml` are natural language:

```yaml
validate:
  - Every requirement has at least one scenario
  - Uses SHALL/MUST language for normative requirements
  - Every scenario uses WHEN/THEN format
```

These are prose instructions that the LLM "checks" by reading the artifact and
making a judgment call. The same applies to cross-blueprint validation flags and
build instructions. The only place where artifact *content* is parsed by real
code is `merge-specs.ts` and the heading conventions in `spec-format.md`.

## Tier 1: Tighten the YAML Layer

Low effort, high leverage.

### 1. Machine-readable validation rules

Extend the schema to support structured validators alongside the human
description:

```yaml
validate:
  - check: has-section
    heading: "## Why"
    min_content: 1 sentence
    description: Has a Why section with at least one sentence
  - check: id-format
    pattern: "^REQ-[0-9]{3}$"
    description: Requirement IDs use the REQ-XXX format
  - check: heading-present
    heading: "#### Scenario:"
    scope: per-requirement
    description: Every requirement has at least one scenario
```

A small validator (like `merge-specs.ts`) runs these checks
deterministically after generation, turning the LLM's self-assessment into a
verifiable gate. The prose `description` stays for the LLM to use during
generation; the structured fields drive automated post-checks.

### 2. Codify the state machine

Today the status transitions (`defining` → `defined` → `building` → `complete`
→ `merged`) are embedded in skill prose. Pull them into the schema:

```yaml
lifecycle:
  states: [defining, defined, building, complete, merged, dropped]
  transitions:
    - from: defining
      to: defined
      requires: [proposal, specs, design, tasks]
    - from: defined
      to: building
      requires: [tasks]
    - from: building
      to: complete
      when: all-tasks-checked
```

This makes the workflow contract inspectable and enforceable by tooling rather
than by hoping the LLM follows the SKILL.md narrative correctly.

### 3. Structured artifact frontmatter

Add a YAML frontmatter block to each generated artifact:

```yaml
---
schema: omnia@1
blueprint: specs
capability: my-capability
change: my-change
req_ids: [REQ-001, REQ-002, REQ-003]
generated_at: 2026-04-12T10:00:00Z
---
```

This gives downstream tooling (merge, verify, validate) a machine-readable
handle without parsing the entire markdown body. The merge script could
cross-reference `req_ids` against the body, catching silent drops.

## Tier 2: A Validation Engine

Medium effort, high leverage.

### 4. Standalone validator CLI

Unify artifact validation into a single tool that can:

- Parse spec files using the `spec-format.md` grammar
- Check structural rules from `schema.yaml` `validate` entries (the
  machine-readable subset from Tier 1)
- Verify cross-artifact references (`proposal-crates-have-specs`, design
  references valid IDs, tasks reference existing specs)
- Validate the state machine (`.metadata.yaml` status vs actual filesystem
  state)

This could start as Deno/TypeScript (existing infrastructure) or Python (to
extend `merge-specs.ts`). The key decision: this should be the **gate** that
build/define must pass. The skill says "run `specify validate`" and halts on
non-zero exit, removing the LLM from the judgment loop.

### 5. Template-driven generation with structural constraints

Instead of giving the LLM a markdown example and saying "fill this in," provide
a structured intermediate representation:

```yaml
# .specify/changes/my-change/.blueprint-plan.yaml
proposal:
  why: "..."
  source: manual
  crates:
    - name: my-crate
      action: new
specs:
  capabilities:
    - name: my-crate
      requirements:
        - id: REQ-001
          name: Handle Widget Creation
          scenarios:
            - name: Valid widget
              when: "..."
              then: "..."
```

The LLM fills in this structured plan. A renderer converts it to the markdown
artifacts. Validation runs on the YAML before rendering. This inverts the current
flow: instead of generating markdown then checking it, you generate structure
then render it.

## Tier 3: A Rust Engine

High effort, highest determinism ceiling.

### 6. Spec parser and validator in Rust

The `spec-format.md` heading grammar is simple enough for a hand-written parser.
A Rust crate could:

- Parse baseline and delta specs into a typed AST (`RequirementBlock`,
  `Scenario`, `DeltaOperation`)
- Validate structural rules exhaustively
- Perform the merge deterministically (replacing `merge-specs.py`)
- Detect drift structurally (comparing spec AST against code AST, at least at
  the signature/export level)
- Emit diagnostics in a structured format (JSON/SARIF) consumable by CI and
  editors

This would be a `specify-core` crate, compiled to a CLI and potentially to WASM
for editor integration. Advantages over Python/Deno: ships as a single binary,
fast, and can grow into more ambitious analysis.

### 7. Schema-driven workflow orchestrator

The most ambitious step: a Rust engine that reads `schema.yaml`, resolves the
blueprint DAG, manages `.metadata.yaml` transitions, and orchestrates the LLM as
a *capability* rather than the *driver*. The engine would:

- Own the state machine (no more trusting the LLM to update `.metadata.yaml`
  correctly)
- Call the LLM for content generation via a defined interface (prompt template +
  structured output schema)
- Run validation deterministically after each generation step
- Retry or reject on validation failure without LLM judgment

The LLM becomes a "content oracle" called by the engine, not the process
controller. This is the biggest shift — from "LLM reads SKILL.md and follows
instructions" to "engine executes workflow, calls LLM for the parts that need
language understanding."

## Tier 4: Structured LLM Output

Orthogonal — applies at any tier.

### 8. Use structured output / JSON mode for generation steps

When the LLM generates artifacts, constrain its output to a JSON schema rather
than free-form markdown. Most model providers support this. Combined with Tier 2's
intermediate representation, the flow becomes:

```
schema.yaml → prompt + JSON schema → LLM → structured output → validate → render to markdown
```

This eliminates an entire class of non-determinism: formatting drift, missing
sections, wrong heading levels, inconsistent ID patterns.

## Recommended Sequencing

| Phase | What | Why first |
|---|---|---|
| **Now** | Structured validate rules in `schema.yaml` (Tier 1, item 1) + artifact frontmatter (item 3) | Zero breaking changes, immediate value, makes everything else easier |
| **Next** | Standalone validator CLI (Tier 2, item 4) that skills call as a gate | Removes LLM from the validation loop; biggest single determinism win |
| **Then** | Codified lifecycle (Tier 1, item 2) + structured intermediate representation (Tier 2, item 5) | Moves the workflow contract from prose to data |
| **Later** | Rust spec parser (Tier 3, item 6) replacing `merge-specs.ts` and the validator | Performance, single binary, foundation for the orchestrator |
| **Eventually** | Rust workflow orchestrator (Tier 3, item 7) | The LLM stops being the driver and becomes a called capability |

## Design Principle

**The LLM should generate content, not make process decisions.** Every time a
SKILL.md instruction says "check that X is true and halt if not," that's a signal
the decision should be in code. The LLM is excellent at understanding
requirements and producing prose/code; it's unreliable at self-assessment, state
management, and structural validation.

The current architecture is already well-positioned for this evolution —
`schema.yaml` is a real contract, `spec-format.md` is a real grammar,
`merge-specs.ts` is a real parser. The gap is that these islands of determinism
don't yet cover the full lifecycle. Closing that gap incrementally, starting from
YAML and working toward Rust, gets there without a rewrite.
