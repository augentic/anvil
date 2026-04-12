# Synthesized Roadmap: Determinism and RWL Iteration

Unified roadmap for making Specify more deterministic and introducing Ralph Wiggum Loop (RWL) iteration for high-context skills. Synthesized from parallel analyses (Opus 4.6, Gemini 3.1 Pro, Composer 2) against the full codebase, superseding the separate `deterministic.md` and `rwl.md` documents.

## Design Principle

**The LLM generates content; code makes process decisions.** Every time a SKILL.md instruction says "check that X is true and halt if not," that decision should be in code. The LLM is excellent at understanding requirements and producing prose/code; it is unreliable at self-assessment, state management, structural validation, and loop control.

---

## Current State: Islands of Determinism


| Layer                     | Mechanism                                                                                                                                               | Determinism                                                                            | Files                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------- |
| **Schema structure**      | `schema.yaml` + `schema.schema.json` + Ajv in `checks.ts`                                                                                               | **Strong** — blueprint DAG, required fields, referential integrity are machine-checked | `schemas/schema.schema.json`, `scripts/checks.ts`  |
| **Spec merge**            | `merge-specs.ts` — parses headings, applies RENAMED→REMOVED→MODIFIED→ADDED in strict order                                                              | **Strong** — the single deterministic operation on artifact content                    | `plugins/spec/skills/merge/scripts/merge-specs.ts` |
| **Repo hygiene**          | `checks.ts` validates 13 categories: links, symlinks, frontmatter, skill references, variable consistency, directives, marketplace alignment, inventory | **Strong** — ~750 lines of TypeScript structural checks                                | `scripts/checks.ts`                                |
| **Spec format**           | `spec-format.md` — `### Requirement:`, `ID: REQ-XXX`, `#### Scenario:`                                                                                  | **Medium** — conventions documented, enforced only in merge                            | `plugins/spec/references/spec-format.md`           |
| **Workflow state**        | `.metadata.yaml` status enum, filesystem presence checks                                                                                                | **Medium** — state machine is implicit in skill prose                                  | `build/SKILL.md`, `define/SKILL.md`                |
| **Validation rules**      | `validate:` arrays in `schema.yaml`                                                                                                                     | **Weak** — prose sentences interpreted by the LLM                                      | `schemas/omnia/schema.yaml`                        |
| **Cross-artifact checks** | `validation:` boolean flags (`proposal-crates-have-specs`, etc.)                                                                                        | **Weak** — trigger LLM-interpreted checks                                              | `schemas/omnia/schema.yaml`                        |
| **Code generation**       | Instruction markdown + `defaults.rules`                                                                                                                 | **Weak** — fully LLM-generated output                                                  | `schemas/omnia/instructions/*.md`                  |
| **Loop control**          | Verify-repair and remediation loops in `build.md`                                                                                                       | **Weak** — the LLM reads prose and acts as loop controller                             | `schemas/omnia/instructions/build.md`              |


### The Core Problem

Every SKILL.md is a miniature program written in English, executed by an LLM. The `code-reviewer` SKILL.md alone defines an agent team with 3 specialists plus an antagonist, each with detailed system prompts, a synthesis protocol, and an auto-fix workflow. This is **process control disguised as documentation**. The LLM is currently the process controller rather than a content oracle.

### Existing RWL Structures (Prose-Embedded)

The architecture already has loop structures, but they are embedded as prose in `schemas/omnia/instructions/build.md`:


| Loop                    | Location                        | Gates                                                            | Limits                    |
| ----------------------- | ------------------------------- | ---------------------------------------------------------------- | ------------------------- |
| **Verify-repair**       | `build.md` §Verify-repair loop  | `cargo fmt --check`, `cargo check`, `cargo clippy`, `cargo test` | Max 3 iterations          |
| **Remediation**         | `build.md` §Remediation process | Parse `REVIEW.md`, route by severity                             | Max 2 iterations post-fix |
| **Analyze validation**  | `analyze/SKILL.md` Phase 5      | Compare artifacts to source                                      | Convergence or pass limit |
| **Define per-artifact** | `define/SKILL.md`               | `validate` rules from `schema.yaml`                              | One retry                 |


All share the same weaknesses: the loop controller is the LLM, feedback is unstructured, routing decisions require LLM judgment, and the whole crate is the unit of iteration.

---

## Phase 1: Structured Feedback Files

**Goal**: Enable structured communication between skills without architectural overhaul.

**Effort**: Low. **Leverage**: High — unblocks all subsequent RWL work.

### 1.1 Define `.specify-feedback.yaml` schema

Add a JSON Schema for the feedback sidecar file. Skills produce this alongside their normal output.

```yaml
# $CRATE_PATH/.specify-feedback.yaml (produced by crate-writer)
skill: omnia:crate-writer
mode: create
handlers_generated:
  - name: CreateWorksite
    file: src/handlers/create_worksite.rs
    cargo_check: pass
    spec_coverage: [REQ-001, REQ-002]
  - name: ListWorksites
    file: src/handlers/list_worksites.rs
    cargo_check: pass
    spec_coverage: [REQ-003]
known_gaps:
  - type: todo-marker
    file: src/handlers/create_worksite.rs
    line: 67
    description: "Cache-aside pattern not implemented"
    spec_reference: REQ-002
```

### 1.2 Define `.review-findings.yaml` schema

Code-reviewer produces structured findings alongside `REVIEW.md`:

```yaml
# $CRATE_PATH/.review-findings.yaml (produced by code-reviewer)
findings:
  - id: SEC-1
    severity: critical
    file: src/handler.rs
    line: 45
    category: wasm-constraint
    auto_fixable: true
    skill_target: crate-writer
    fix_description: "Replace std::env::var with Config::get"
    spec_reference: null
  - id: COR-3
    severity: high
    file: tests/provider.rs
    line: 112
    category: mock-provider
    auto_fixable: false
    skill_target: test-writer
    fix_description: "MockProvider missing TableStore impl"
    spec_reference: REQ-002
```

### 1.3 Diagnostic formatter

Build a lightweight script that parses `cargo check --message-format=json` and `cargo test` output into a concise, LLM-friendly summary (file, line, error code, snippet). This prevents context window bloat when feeding compiler errors back into skills during RWL iterations.

```bash
# Usage:
cargo check --message-format=json 2>&1 | specify-diag format
cargo test -- --format json 2>&1 | specify-diag format
```

Output: structured YAML or Markdown with only the fields a skill needs for targeted repair. This is a first-class architectural component, not an afterthought.

### 1.4 Wire feedback into skills

- **crate-writer**: Produce `.specify-feedback.yaml` after generation.
Accept an optional `--feedback <file>` mode for repair passes (read
compiler errors or review findings, apply surgical fixes).
- **test-writer**: Read crate-writer's `.specify-feedback.yaml` for
handler signatures, trait bounds, and spec coverage. Produce its own
feedback sidecar with test-to-spec mapping.
- **code-reviewer**: Produce `.review-findings.yaml` alongside
`REVIEW.md`. Structured findings enable deterministic routing.

### 1.5 Feedback file lifecycle

Add cleanup rules: `.specify-feedback.yaml` and
`.review-findings.yaml` are removed at merge time by the merge skill,
archived with the change. Validate their schemas in `checks.ts`.

### Deliverables

- JSON Schema for `.specify-feedback.yaml`
- JSON Schema for `.review-findings.yaml`
- `specify-diag` formatter script (Deno/TypeScript)
- crate-writer produces feedback sidecar
- test-writer consumes crate-writer feedback
- code-reviewer produces structured findings
- `checks.ts` validates feedback file schemas
- Merge skill cleans up feedback files

---

## Phase 2: Structured Validation Rules + Validator CLI

**Goal**: Remove the LLM from the artifact validation loop entirely.

**Effort**: Medium. **Leverage**: Highest single determinism win.

### 2.1 Machine-readable validation rules

Extend `schema.yaml` `validate` arrays to support structured check
objects alongside prose strings (backward-compatible):

```yaml
validate:
  - check: heading-present
    heading: "#### Scenario:"
    scope: per-requirement
    description: Every requirement has at least one scenario
  - check: pattern-match
    pattern: "^ID: REQ-[0-9]{3}$"
    scope: per-requirement
    description: Requirement IDs use REQ-XXX format
  - check: keyword-present
    keywords: ["SHALL", "MUST"]
    scope: per-requirement
    description: Uses normative language
  # Backward-compatible: plain strings still accepted
  - Has a Why section with at least one sentence
```

The `description` stays for the LLM to use during generation; the structured fields drive the validator.

### 2.2 Extend `schema.schema.json`

Update the blueprint `validate` items definition to accept either a string (current) or an object with structured check fields:

```json
"validate": {
  "type": "array",
  "items": {
    "oneOf": [
      { "type": "string" },
      { "$ref": "#/$defs/structured-check" }
    ]
  }
}
```

### 2.3 Standalone validator CLI

Build `specify validate` as a Deno/TypeScript CLI (extending the `checks.ts` and `merge-specs.ts` infrastructure):

- Parse spec files using the `spec-format.md` grammar
- Check structural rules from `schema.yaml` `validate` entries
- Verify cross-artifact references (`proposal-crates-have-specs`,
design references valid IDs, tasks reference existing specs)
- Validate `.metadata.yaml` status vs actual filesystem state
- Return exit code 0/1 with structured diagnostics

### 2.4 Wire the gate into skills

`/spec:build` and `/spec:define` invoke `specify validate` via the Shell tool and halt on non-zero exit. The LLM no longer decides whether validation passed; the CLI does. Each structured rule in the skill prose is **replaced** (not duplicated) by the corresponding validator check.

### 2.5 Artifact frontmatter

Add YAML frontmatter to generated artifacts:

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

The validator cross-references `req_ids` against the body, catching silent drops. Downstream tooling (merge, verify) uses frontmatter instead of parsing the entire markdown body.

### Deliverables

- Structured check types in `schema.schema.json` (backward-compatible)
- Convert `schemas/omnia/schema.yaml` validate rules to structured format
- `specify validate` CLI (Deno/TypeScript)
- `build/SKILL.md` calls `specify validate` as a gate
- `define/SKILL.md` calls `specify validate` as a gate
- Artifact frontmatter generation in `define` skill
- Remove redundant prose validation paragraphs from skills

---

## Phase 3: Lifecycle State Machine + Structured IR

**Goal**: Move the workflow contract from prose to data.

**Effort**: Medium. **Leverage**: High — eliminates an entire class of
state management bugs.

### 3.1 Codify the lifecycle in `schema.yaml`

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
    - from: complete
      to: merged
      action: merge-specs
    - from: [defining, defined, building]
      to: dropped
      action: archive
```

### 3.2 Lifecycle validator

Build a state-machine validator that reads `.metadata.yaml` and checks transitions against the schema. Add to `specify validate` and to `checks.ts`. Remove the "valid lifecycle status values" guardrail paragraphs from all skills — the code enforces this now.

### 3.3 Structured intermediate representation

Instead of the LLM generating free-form markdown, constrain artifact generation to a structured intermediate format:

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

The LLM fills the structured plan. A deterministic renderer converts it to markdown artifacts. Validation runs on the YAML before rendering. This inverts the current flow: generate structure, validate structure, render to markdown.

### Deliverables

- `lifecycle` section in `schema.schema.json`
- `lifecycle` section in `schemas/omnia/schema.yaml`
- Lifecycle validator in `specify validate`
- Remove lifecycle guardrail paragraphs from skills
- `.blueprint-plan.yaml` schema definition
- Renderer: structured plan → markdown artifacts
- LLM output constrained to JSON/YAML schema during generation

---

## Phase 4: RWL Inner Loop — Deterministic Verify-Repair

**Goal**: Replace the prose verify-repair loop with structured,
deterministic loop control.

**Effort**: Medium. **Leverage**: High — directly improves code
generation reliability.

### 4.1 Deterministic failure classification

The hardest part of the verify-repair loop is classifying failures. The current `build.md` classification table can be made partially deterministic:

1. Parse `cargo test` output (using `specify-diag` from Phase 1)
2. Extract failing test names and error locations
3. If the error location is in `tests/` → **test issue**
4. If the error location is in `src/` → **code issue**
5. For assertion mismatches: compare the expected value against the spec
  (this still requires LLM judgment, but the input is structured)

Steps 1–4 are code. Step 5 is the residual that stays with the LLM. The spec is the arbiter: a previously-passing test is only a regression if the behavior it validates is still specified.

### 4.2 Skill feedback ingestion mode

Update skills to accept structured feedback for repair:

- **crate-writer**: Accept `--feedback <file>` containing classified
errors from `specify-diag`. Operate in repair mode: fix the reported
errors, nothing else.
- **test-writer**: Same pattern. Receive test-classified errors.
- **Repair discipline** (preserved from `build.md`): minimum change
only, one failure class per re-entry, scope the diff.

### 4.3 Iteration tracking

Persist iteration counts in `.metadata.yaml` or a sidecar:

```yaml
verify_iterations: 2
remediation_iterations: 1
```

Support `SPECIFY_MAX_VERIFY_ITERATIONS` environment variable for CI control. Default: 3. After exhausting iterations: **STOP**, report failures, escalate.

### 4.4 Define-side artifact quality loop

Connect the validator CLI (Phase 2) to a generate → validate → refine loop in the `define` skill:

```
define generates artifacts → specify validate (deterministic)
→ if failures: LLM refines → re-validate (max 2 iterations)
```

The validator is code; the correction is LLM. This is the simplest RWL and establishes the pattern for all subsequent loops.

### Deliverables

- `specify-diag classify` command for failure classification
- crate-writer `--feedback` repair mode
- test-writer `--feedback` repair mode
- Iteration counter in `.metadata.yaml`
- `SPECIFY_MAX_VERIFY_ITERATIONS` env var support
- Define-side artifact quality RWL
- Replace prose verify-repair loop in `build.md` with structured control

---

## Phase 5: RWL Outer Loops — Skill Chaining

**Goal**: Compose skills with structured feedback loops at an orchestration layer above individual skills.

**Effort**: High. **Leverage**: High — replaces the current post-hoc verify-repair with proactive co-refinement.

### 5.1 Per-handler co-refinement (crate-writer + test-writer)

Replace the current sequential flow (crate-writer finishes entirely → test-writer finishes entirely → verify-repair patches up) with  
interleaved per-handler generation:

```
For each handler in the handler manifest:
  1. crate-writer generates handler code
  2. cargo check (structural smoke)
  3. test-writer generates tests for that handler
  4. cargo test (behavioral check)
  5. If failures: classify → route to appropriate skill
  6. Max 2 refinement passes per handler
```

This requires crate-writer to produce a **handler manifest** after its cross-cutting analysis step: a structured list of handlers to generate, with dependencies, trait bounds, and matrix entries.

**Fallback**: If per-handler iteration doesn't converge in 2 passes for a handler, fall back to whole-crate iteration. Some crates have deep handler interdependencies (shared state, cross-handler delegation, transaction boundaries) that don't decompose cleanly.

### 5.2 Code-reviewer structured feedback loop

Chain code-reviewer to the end of the build loop with structured routing:

```
code-reviewer produces .review-findings.yaml
→ filter by severity (CRITICAL/HIGH only)
→ group by skill_target
→ route each group to the target skill with --feedback
→ verify-repair (max 2 iterations — tighter cap for targeted repairs)
→ re-review (without --fix) to verify quality
→ if new CRITICAL: one more remediation cycle
```

The routing decision is data-driven from `.review-findings.yaml`: filter by `severity`, group by `skill_target`, invoke. No LLM judgment needed for the routing itself.

### 5.3 Code-analyzer self-critique loop

`spec:analyze` has the richest evaluation signal available — the source code itself. Introduce a partially deterministic self-critique:

```
Pass 1: Analyze source → produce specs + design.md
Pass 2: For each handler:
         - Parse generated design.md type tables
         - Parse source code type definitions
         - Compare: field names, types, counts
         - Check: every REQ-XXX in design.md exists in a spec file
         - Check: every exported handler has a Business Logic block
         → produce structured discrepancy report
Pass 3: LLM applies corrections → re-validate
Convergence: zero CRITICAL/HIGH discrepancies, or max 2 iterations
```

V1 (Type Fidelity), V3 (API Contract), and V4 (Cross-Reference) checks can be partially automated. V2 (Algorithm Fidelity) and V5 (Completeness) still need LLM judgment but with structured input.

### 5.4 The unified pipeline

The full Omnia create-mode chain becomes:

```
guest-writer
  → crate-writer (with handler manifest)
    → [per-handler: cargo check → test-writer → cargo test → classify]
      → verify-repair loop (max 3, deterministic classification)
        → code-reviewer (structured findings)
          → remediation loop (max 2, routed by skill_target)
            → final re-review
```

### Deliverables

- Handler manifest schema and crate-writer integration
- Per-handler co-refinement loop in build orchestration
- Whole-crate fallback when per-handler doesn't converge
- Code-reviewer structured routing from `.review-findings.yaml`
- Code-analyzer self-critique with partial automation
- Unified pipeline documentation replacing current `build.md` prose

---

## Phase 6: Pipeline YAML — Declarative Orchestration

**Goal**: Replace prose-embedded pipelines with structured, inspectable, configurable pipeline definitions.

**Effort**: High. **Leverage**: Medium-high — makes pipelines a first-class configurable contract.

### 6.1 Pipeline section in `schema.yaml`

```yaml
pipelines:
  create:
    steps:
      - id: guest
        skill: omnia:guest-writer

      - id: generate-crate
        skill: omnia:crate-writer
        after: [guest]

      - id: generate-tests
        skill: omnia:test-writer
        after: [generate-crate]

      - id: verify
        type: loop
        max_iterations: 3
        after: [generate-tests]
        checks:
          - run: cargo fmt --check
            fix: cargo fmt
          - run: cargo check && cargo clippy -- -D warnings
            on_fail: classify-and-route
          - run: cargo test
            on_fail: classify-and-route
        routing:
          test_issue: omnia:test-writer
          code_issue: omnia:crate-writer
        convergence: all-green

      - id: review
        skill: omnia:code-reviewer
        after: [verify]
        output: structured

      - id: remediate
        type: loop
        max_iterations: 2
        after: [review]
        input: review.findings
        route_by: skill_target
        after_each: verify
        convergence: no-critical-findings

  update:
    steps:
      - id: capture-baseline
        run: cargo test 2>&1 | tee /tmp/${CHANGE_ID}-${CRATE_NAME}-baseline.txt

      - id: generate-crate
        skill: omnia:crate-writer
        after: [capture-baseline]

      - id: generate-tests
        skill: omnia:test-writer
        after: [generate-crate]

      # ... same verify/review/remediate structure
```

### 6.2 Pipeline schema in `schema.schema.json`

Extend the schema to validate pipeline definitions: step types (`skill`, `loop`, `run`), `after` dependency DAG, convergence criteria, routing tables.

### 6.3 Pipeline reader in build skill

Initially, the build skill reads the pipeline YAML and executes it step by step. The LLM is still the controller but with **structured guidance** — it follows a data definition rather than interpreting prose. This is the bridge to a compiled engine.

### Deliverables

- `pipelines` section in `schema.schema.json`
- `pipelines` section in `schemas/omnia/schema.yaml`
- `build/SKILL.md` reads and executes pipeline YAML
- Remove prose pipeline from `schemas/omnia/instructions/build.md`
- Pipeline DAG validation in `checks.ts`

---

## Phase 7: Rust Engine

**Goal**: The LLM stops being the driver and becomes a called capability.

**Effort**: Very high. **Leverage**: Maximum determinism ceiling.

### 7.1 `specify-core` crate — parser and validator

- Parse spec files into typed AST (`RequirementBlock`, `Scenario`,
`DeltaOperation`)
- Validate structural rules exhaustively
- Perform merge deterministically (replacing `merge-specs.ts`)
- Detect drift structurally
- Emit diagnostics in JSON/SARIF format
- Ship as single CLI binary, potentially WASM for editor integration

### 7.2 `specify-core` — lifecycle and pipeline engine

- Read `schema.yaml` pipelines, resolve DAG
- Manage `.metadata.yaml` transitions
- Execute pipeline steps: invoke LLM for content generation via defined
interface, run validation deterministically, control loops, route
failures
- The LLM becomes a "content oracle" called by the engine

### 7.3 Integration with DSL roadmap

The Rust DSL from `roadmap/dsl.md` is complementary: once skills have structured manifests (from Phase 1 feedback schemas and Phase 6 pipeline YAML), the Rust DSL can consume those manifests and generate type-safe skill definitions. The DSL catches structural errors at compile time; the engine catches process errors at runtime.

### Deliverables

- `specify-core` crate: spec parser + validator
- `specify-core` crate: merge (replacing `merge-specs.ts`)
- `specify-core` crate: drift detection
- `specify-core` crate: pipeline orchestrator
- CLI binary distribution
- WASM build for editor integration (optional)

---

## Phase Summary


| Phase | What                                             | Key Principle                                        | Effort    | Connects to                              |
| ----- | ------------------------------------------------ | ---------------------------------------------------- | --------- | ---------------------------------------- |
| **1** | Structured feedback files + diagnostic formatter | Skills communicate via data, not prose               | Low       | Enables all RWL work                     |
| **2** | Structured validation rules + validator CLI      | The LLM generates; code validates                    | Medium    | Removes LLM from validation              |
| **3** | Lifecycle state machine + structured IR          | Workflow contract is data, not narrative             | Medium    | Eliminates state management bugs         |
| **4** | Deterministic verify-repair (inner RWL)          | Code classifies; LLM repairs                         | Medium    | Directly improves generation reliability |
| **5** | Skill chaining with outer RWL loops              | Per-handler co-refinement, structured review routing | High      | Replaces post-hoc patching               |
| **6** | Pipeline YAML in `schema.yaml`                   | Orchestration is declarative and configurable        | High      | Bridge to compiled engine                |
| **7** | Rust engine (`specify-core`)                     | LLM is capability; engine is controller              | Very high | Maximum determinism                      |


### Parallelism

Phases 1–3 are mostly independent and can be worked in parallel. Phase 4 depends on Phase 1 (feedback files) and Phase 2 (validator CLI). Phase 5 depends on Phase 4. Phase 6 can start alongside Phase 5 once the loop patterns stabilize. Phase 7 depends on all prior phases for its input contracts.

---

## Trade-offs and Risks

### Over-constraining the creative parts

Skills like `spec:analyze` Phase 3 (domain-by-domain extraction) and `code-reviewer` category checklists require genuine language understanding. The THINK prompts ("Before extracting each type, reason through...") leverage the LLM's flexible reasoning. **Constrain the process, liberate the content generation.**

### YAML complexity ceiling

As pipeline definitions grow, YAML becomes its own DSL. If it needs conditional logic (`if Cargo.toml exists, use update pipeline`), inheritance, or parameterization, YAML gets unwieldy. The YAML layer declares *what happens*; the Rust engine handles complex control flow. Don't over-invest in YAML expressiveness — that's Phase 7's job.

### Context window dilution in RWL loops

Feeding a massive `cargo test` failure dump back into crate-writer alongside the original specs, design, and existing code will overwhelm the context window. The diagnostic formatter (Phase 1.3) is critical infrastructure, not an optimization. It must truncate stack traces and provide only file, line, and localized error message.

### Cost and latency from nested loops

Code-reviewer spawns 4 agents. Running this inside a loop, combined with multiple crate-writer and test-writer invocations, significantly increases token costs. **Layer the RWL strictly**: never run the expensive code-reviewer until the cheap `cargo check` and `cargo test` loops have fully passed.

### Test vs. code authority conflict

When a test fails, the system doesn't inherently know if the code is wrong or the test was hallucinated. **The spec is the arbiter.** Compare the failing test against the `spec.md` ground truth. If the test matches the spec, fix the code. If the test deviates, fix the test. This principle is already in `build.md`; it must be preserved through all RWL work.

### Dual maintenance during transition

During migration from prose to structured definitions, skills contain both prose instructions (for the LLM) and structured YAML (for the validator). If these diverge, the LLM follows one set of rules while the validator enforces another. **Each structured rule must replace, not duplicate, the corresponding prose instruction.**

### Feedback file proliferation

If every skill produces feedback sidecars, the `.specify/changes/` directory accumulates machine-generated files. These need lifecycle management (cleaned at merge, archived), schema validation, and clear ownership. Define this in Phase 1.5 before it becomes organic.

### Per-handler granularity may not fit all crates

The per-handler co-refinement loop assumes handlers are independent enough to generate and test individually. Some crates have deep handler interdependencies (shared state, cross-handler delegation). The Transaction Boundary Matrix in crate-writer exists precisely because handlers are not always independent. **Always provide a whole-crate fallback.**

### Infinite loop / hallucination spirals

The LLM fails to understand a compiler error, applies a wrong fix, breaks something else, loops indefinitely. **Strict iteration caps** are non-negotiable. After cap: stop, output the diagnostic state, escalate for human guidance. Never weaken this to "try harder."

### Scope creep on the Rust engine

A Rust orchestrator is easy to overbuild. **Validate IR and CLI gates first** — the Rust engine only adds value after the validation rules, feedback schemas, and pipeline definitions exist as stable contracts.