# Synthesized Roadmap: Determinism and RWL Iteration

Unified roadmap for making Specify more deterministic and introducing Ralph Wiggum Loop (RWL) iteration for high-context skills. Synthesized from parallel analyses (Opus 4.6, Gemini 3.1 Pro, Composer 2) against the full codebase, superseding the separate `deterministic.md` and `rwl.md` documents.

## Design Principle

**The LLM generates content; code makes process decisions.** Every time a SKILL.md instruction says "check that X is true and halt if not," that decision should be in code. The LLM is excellent at understanding requirements and producing prose/code; it is unreliable at self-assessment, state management, structural validation, and loop control.

## Multi-Schema Scope

This roadmap targets **Omnia first**. The `schemas/vectis/schema.yaml` schema has the same prose `validate` arrays, implicit lifecycle, and prose-embedded loop control — but Vectis has a fundamentally different pipeline shape (`core-writer → ios-writer → android-writer → design-system-writer`) and platform-specific verification (`make build` for Xcode, `./gradlew :app:assembleDebug` for Android, not `cargo check`/`cargo test`).

For each phase, deliverables fall into two categories:

- **Schema-agnostic infrastructure** (shared): feedback file schemas, validator CLI, lifecycle state machine, diagnostic formatter, pipeline engine. These are built once and consumed by both schemas.
- **Schema-specific definitions** (per-schema): structured validation rules, pipeline YAML, blueprint IR schemas. These must be defined separately for Omnia and Vectis.

Phases 1–6 deliver Omnia-specific definitions alongside the shared infrastructure. Vectis parity follows as a fast-follow using the same infrastructure, with schema-specific definitions adapted for multi-platform build verification. Phase 6 pipeline YAML must be designed generically enough to support Vectis's multi-platform steps — the `step` type vocabulary should accommodate platform-specific build commands, not just Cargo toolchain.

## Migration and Backward Compatibility

Phases 1–3 introduce new files (`.specify-feedback.yaml`, `.review-findings.yaml`, artifact frontmatter, `.blueprint-plan.yaml`) and extend existing schemas. Downstream projects with existing `.specify/` directories need a migration path.

**Schema versioning**: The `version` field in `schema.yaml` already exists. Each phase that changes the schema increments this version. The validator CLI accepts a `--schema-version` flag and defaults to the latest. Older projects validate cleanly against their declared version.

**Graceful degradation**: New files (feedback sidecars, frontmatter) are optional during transition. The validator reports their absence as warnings, not errors, until the project declares a schema version that requires them.

**Migration command**: A `specify migrate` subcommand upgrades an existing `.specify/` structure to the latest schema version — adding frontmatter to existing artifacts, creating missing `.metadata.yaml` fields, and updating `config.yaml` to the new version. This ships with Phase 2.

**In-flight changes**: A change currently in `building` status when the lifecycle schema changes continues under the old schema version. The migration command only upgrades the project baseline, not active changes. Active changes complete under the version they started with.

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
schema_version: 1
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
schema_version: 1
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

### 1.3 Diagnostic formatter (`specify diag`)

Build a lightweight script that parses compiler and test output into a concise, LLM-friendly summary (file, line, error code, snippet). This prevents context window bloat when feeding compiler errors back into skills during RWL iterations. The formatter is a subcommand of a unified `specify` CLI (alongside `specify validate` in Phase 2, `specify migrate` in Phase 2, and `specify diag classify` in Phase 4).

```bash
# Usage:
cargo check --message-format=json 2>&1 | specify diag format
cargo test -- --format json 2>&1 | specify diag format
```

The formatter defines an explicit input contract per subcommand: `cargo check --message-format=json` produces Cargo diagnostic JSON; `cargo test -- --format json` produces libtest JSON (a different shape, requiring nightly or `--format` support). For stable toolchains where `--format json` is unavailable, `specify diag` falls back to parsing human-readable test output. Document the supported toolchain and exact subcommand invocations.

Output: structured YAML or Markdown with only the fields a skill needs for targeted repair. This is a first-class architectural component, not an afterthought.

### 1.4 Wire feedback into skills

- **crate-writer**: Produce `.specify-feedback.yaml` after generation.
Accept a feedback-injection mode for repair passes: the orchestrating
skill (or build pipeline) includes a feedback file in the skill's
context, and crate-writer reads classified compiler errors or review
findings to apply surgical fixes. (These are LLM skills invoked via
prompt context, not CLI binaries — "feedback mode" means the
orchestrator injects the feedback file content into the skill's
prompt, not a literal `--feedback` flag.)
- **test-writer**: Read crate-writer's `.specify-feedback.yaml` for
handler signatures, trait bounds, and spec coverage. Produce its own
feedback sidecar with test-to-spec mapping.
- **code-reviewer**: Produce `.review-findings.yaml` alongside
`REVIEW.md`. Structured findings enable deterministic routing.

### 1.5 Feedback file lifecycle

Add cleanup rules: `.specify-feedback.yaml` and
`.review-findings.yaml` are removed from the crate path at merge time
by the merge skill, archived to `.specify/changes/<change>/archive/`.
The archive directory preserves the final state of all machine-generated
sidecars for post-mortem analysis. Validate feedback file schemas in
`checks.ts`, including the required `schema_version` field for
forward-compatibility (see Migration and Backward Compatibility).

### Deliverables

- JSON Schema for `.specify-feedback.yaml` (with `schema_version` field)
- JSON Schema for `.review-findings.yaml` (with `schema_version` field)
- `specify` CLI entry point with `specify diag format` subcommand (Deno/TypeScript)
- Input contracts for Cargo diagnostic JSON and libtest JSON (with stable fallback)
- crate-writer produces feedback sidecar
- test-writer consumes crate-writer feedback
- code-reviewer produces structured findings
- `checks.ts` validates feedback file schemas
- Merge skill archives feedback files to `.specify/changes/<change>/archive/`

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

**Scope vocabulary**: `scope` defines the unit of evaluation:
- `document` — check applies once to the entire artifact file
- `per-requirement` — check runs once per `### Requirement:` block
- `per-scenario` — check runs once per `#### Scenario:` block within each requirement

Evaluating `per-requirement` and `per-scenario` scopes requires parsing spec files into requirement and scenario blocks. The parsing logic already exists in `merge-specs.ts` (`parseRequirementBlocks`); the Phase 2 validator reuses it. Phase 7's Rust parser replaces this implementation, not introduces the concept.

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

Build `specify validate` as a subcommand of the unified `specify` CLI (extending the `checks.ts` and `merge-specs.ts` infrastructure):

- Parse spec files using the `spec-format.md` grammar
- Check structural rules from `schema.yaml` `validate` entries
- Verify cross-artifact references (`proposal-crates-have-specs`,
design references valid IDs, tasks reference existing specs)
- Validate `.metadata.yaml` status vs actual filesystem state
- Return exit code 0/1 with structured diagnostics

**Architecture constraint**: The validator must be decoupled from any single input format. It validates **Markdown artifacts** (for drift detection, merge, manual edits, and `spec:verify`) and, once Phase 3B introduces the structured IR, validates **IR YAML** (during generation). The check evaluation engine accepts a parsed document model, not raw Markdown — this makes it reusable when the input source changes. This also ensures `spec:verify` and `spec:build` share the same diagnostics format and notion of "green."

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
- `specify validate` subcommand (Deno/TypeScript), format-agnostic check engine
- `specify migrate` subcommand for upgrading existing `.specify/` structures
- `build/SKILL.md` calls `specify validate` as a gate
- `define/SKILL.md` calls `specify validate` as a gate
- Artifact frontmatter generation in `define` skill
- Remove redundant prose validation paragraphs from skills

---

## Phase 3A: Lifecycle State Machine

**Goal**: Move the workflow state contract from prose to data.

**Effort**: Medium. **Leverage**: High — eliminates an entire class of
state management bugs.

### 3A.1 Codify the lifecycle in `schema.yaml`

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

### 3A.2 Lifecycle validator

Build a state-machine validator that reads `.metadata.yaml` and checks transitions against the schema. Add to `specify validate` and to `checks.ts`. Remove the "valid lifecycle status values" guardrail paragraphs from all skills — the code enforces this now.

### Deliverables

- `lifecycle` section in `schema.schema.json`
- `lifecycle` section in `schemas/omnia/schema.yaml`
- Lifecycle validator in `specify validate`
- Remove lifecycle guardrail paragraphs from skills

---

## Phase 3B: Structured Intermediate Representation

**Goal**: Invert artifact generation from free-form markdown to structured data rendered to markdown.

**Effort**: High. **Leverage**: High — enables deterministic validation before rendering and makes Phase 7 dramatically more effective.

This is architecturally the most disruptive change in the roadmap. It inverts the LLM's contract from "produce markdown" to "fill a YAML schema, code renders it." This requires structured output / JSON-mode prompt engineering for every blueprint, a complete YAML-to-markdown renderer that faithfully reproduces the current format, migration of all instruction files to target the new intermediate format, and testing that rendered output passes all downstream consumption (merge, verify, build).

**Approach**: Start with a single blueprint (`specs`) as a proof of concept before committing to all four. This validates the round-trip (YAML → Markdown → merge → verify) and surfaces escaping issues (long-form prose, Mermaid diagrams, complex scenarios in YAML string literals) early.

### 3B.1 Structured intermediate representation

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

### 3B.2 Transition: Markdown vs IR as primary input

During transition, two sources of truth coexist: hand-edited Markdown artifacts and IR-generated Markdown. The validator must handle both:

- **Generation path** (`spec:define`): LLM produces IR YAML → validator checks IR → renderer produces Markdown. The IR is the source of truth; the Markdown is a render target.
- **Editing path** (human edits, `spec:verify`, `spec:merge`): Markdown is the source of truth. The validator parses Markdown using the Phase 2 parser.
- **Convergence**: `specify validate` auto-detects the input source. If `.blueprint-plan.yaml` exists and is newer than the rendered artifacts, validate the IR. Otherwise, validate the Markdown. This avoids forcing a choice between the two modes during transition.

### Deliverables

- `.blueprint-plan.yaml` schema definition (starting with `specs` blueprint)
- Renderer: structured plan → markdown artifacts
- LLM output constrained to JSON/YAML schema during generation
- `specify validate` auto-detection of IR vs Markdown input
- Proof-of-concept round-trip validation (IR → Markdown → merge → verify)
- Migration guide for instruction files targeting the IR format

---

## Phase 4: RWL Inner Loop — Deterministic Verify-Repair

**Goal**: Replace the prose verify-repair loop with structured,
deterministic loop control.

**Effort**: Medium. **Leverage**: High — directly improves code
generation reliability.

### 4.1 Deterministic failure classification

The hardest part of the verify-repair loop is classifying failures. The current `build.md` classification table can be made partially deterministic:

1. Parse `cargo test` output (using `specify diag` from Phase 1)
2. Extract failing test names and error locations
3. If the error location is in `tests/` → **test issue**
4. If the error location is in `src/` → **code issue**
5. For assertion mismatches: compare the expected value against the spec
  (this still requires LLM judgment, but the input is structured)

Steps 1–4 are code. Step 5 is the residual that stays with the LLM. The spec is the arbiter: a previously-passing test is only a regression if the behavior it validates is still specified.

### 4.2 Skill feedback ingestion mode

Update skills to accept structured feedback for repair (via context
injection — the orchestrator includes the feedback file in the skill's
prompt, as established in Phase 1.4):

- **crate-writer**: Receive classified errors from `specify diag`.
Operate in repair mode: fix the reported errors, nothing else.
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

- `specify diag classify` subcommand for failure classification
- crate-writer feedback-injection repair mode
- test-writer feedback-injection repair mode
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
Step 0: crate-writer generates shared types, domain models, and module tree
        (mod.rs registrations, common error types, shared newtypes)
        → cargo check (structural smoke on shared foundation)

For each handler in the handler manifest:
  1. crate-writer generates handler code
  2. cargo check (structural smoke)
  3. test-writer generates tests for that handler
  4. cargo test (behavioral check)
  5. If failures: classify → route to appropriate skill
  6. Max 2 refinement passes per handler
```

Step 0 is critical: a new handler file won't pass `cargo check` unless it's registered in `mod.rs` and its dependencies on shared types are satisfied. The handler manifest must explicitly track these shared dependencies so the module tree is complete before per-handler iteration begins.

This requires crate-writer to produce a **handler manifest** after its cross-cutting analysis step: a structured list of handlers to generate, with dependencies, trait bounds, shared type references, and matrix entries.

**Fallback**: If per-handler iteration doesn't converge in 2 passes for a handler, fall back to whole-crate iteration. Some crates have deep handler interdependencies (shared state, cross-handler delegation, transaction boundaries) that don't decompose cleanly.

### 5.2 Code-reviewer structured feedback loop

Chain code-reviewer to the end of the build loop with structured routing:

```
code-reviewer produces .review-findings.yaml
→ filter by severity (CRITICAL/HIGH only)
→ group by skill_target
→ route each group to the target skill via feedback injection
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
        run: cargo test
        output: ${{ outputs.baseline_results }}

      - id: generate-crate
        skill: omnia:crate-writer
        after: [capture-baseline]

      - id: generate-tests
        skill: omnia:test-writer
        after: [generate-crate]

      # ... same verify/review/remediate structure
```

**Variable substitution**: Pipeline YAML uses `${{ vars.CHANGE_ID }}` and `${{ outputs.step_id }}` syntax rather than raw shell interpolation. This keeps the pipeline schema portable and engine-agnostic — the pipeline engine resolves variables before invoking commands, rather than relying on platform-specific shell expansion. Outputs are abstract references, not hardcoded filesystem paths.

### 6.2 Pipeline schema in `schema.schema.json`

Extend the schema to validate pipeline definitions: step types (`skill`, `loop`, `run`), `after` dependency DAG, convergence criteria, routing tables.

### 6.3 Pipeline reader in build skill

Initially, the build skill reads the pipeline YAML and executes it step by step. The LLM is still the controller but with **structured guidance** — it follows a data definition rather than interpreting prose. This is the bridge to a compiled engine.

### 6.4 Pipeline conformance check

Add a conformance check to `checks.ts` that ensures `build/SKILL.md` (or the relevant instruction file) references the pipeline ID from `schema.yaml`. This prevents drift where pipeline YAML exists but agents ignore it in favor of stale prose. The check can be as simple as verifying a hash or version marker of the pipeline section matches what's embedded in the skill reference.

### Deliverables

- `pipelines` section in `schema.schema.json` (with `${{ }}` variable substitution)
- `pipelines` section in `schemas/omnia/schema.yaml`
- `build/SKILL.md` reads and executes pipeline YAML
- Remove prose pipeline from `schemas/omnia/instructions/build.md`
- Pipeline DAG validation in `checks.ts`
- Pipeline conformance check in `checks.ts` (skill ↔ pipeline alignment)

---

## Phase 7: Rust Engine

**Goal**: The LLM stops being the driver and becomes a called capability.

**Effort**: Very high. **Leverage**: Maximum determinism ceiling.

Unlike the claim in earlier drafts, Phase 7 does **not** depend on all prior phases. Each sub-phase has specific, narrower entry criteria that enable earlier starts.

### 7.1 `specify-core` crate — parser and validator

**Entry criteria**: Phase 2 (structured validation rules and `spec-format.md` grammar are stable).

- Parse spec files into typed AST (`RequirementBlock`, `Scenario`,
`DeltaOperation`)
- Validate structural rules exhaustively
- Emit diagnostics in JSON/SARIF format
- Ship as single CLI binary, potentially WASM for editor integration

### 7.2 `specify-core` — merge replacement

**Entry criteria**: Phase 7.1 (parser produces typed AST).

- Perform merge deterministically (replacing `merge-specs.ts`)
- Detect drift structurally
- Reuse the same parsed document model for both merge and validation

### 7.3 `specify-core` — lifecycle engine

**Entry criteria**: Phase 3A (lifecycle state machine schema is stable).

- Read lifecycle schema from `schema.yaml`
- Manage `.metadata.yaml` transitions
- Enforce state machine invariants at the engine level

### 7.4 `specify-core` — pipeline orchestrator

**Entry criteria**: Phase 6 (pipeline YAML schema is stable).

- Read `schema.yaml` pipelines, resolve DAG
- Execute pipeline steps: invoke LLM for content generation via defined
interface, run validation deterministically, control loops, route
failures
- The LLM becomes a "content oracle" called by the engine

### 7.5 Integration with DSL roadmap

The Rust DSL from `roadmap/dsl.md` is complementary: once skills have structured manifests (from Phase 1 feedback schemas and Phase 6 pipeline YAML), the Rust DSL can consume those manifests and generate type-safe skill definitions. The DSL catches structural errors at compile time; the engine catches process errors at runtime. See `dsl.md` Phase 2 for SKILL compile-time structure; `specify-core` consumes the same manifests at runtime.

### Deliverables

- `specify-core` crate: spec parser + validator (can start after Phase 2)
- `specify-core` crate: merge (after 7.1, replacing `merge-specs.ts`)
- `specify-core` crate: drift detection (after 7.1)
- `specify-core` crate: lifecycle engine (can start after Phase 3A)
- `specify-core` crate: pipeline orchestrator (after Phase 6)
- CLI binary distribution
- WASM build for editor integration (optional)

---

## Phase Summary


| Phase  | What                                             | Effort    | Done when                                                                                     |
| ------ | ------------------------------------------------ | --------- | --------------------------------------------------------------------------------------------- |
| **1**  | Structured feedback files + diagnostic formatter | Low       | All skills produce feedback sidecars; `checks.ts` validates their schemas                     |
| **2**  | Structured validation rules + validator CLI      | Medium    | Zero prose-only validate rules remain in `schemas/omnia/schema.yaml`; `specify validate` gates CI |
| **3A** | Lifecycle state machine                          | Medium    | All lifecycle transitions enforced by code; guardrail paragraphs removed from skills          |
| **3B** | Structured intermediate representation           | High      | `specs` blueprint round-trips through IR → Markdown → merge → verify                         |
| **4**  | Deterministic verify-repair (inner RWL)          | Medium    | `specify diag classify` handles all failure routing; prose verify-repair loop removed          |
| **5**  | Skill chaining with outer RWL loops              | High      | Per-handler co-refinement runs end-to-end with shared types Step 0                            |
| **6**  | Pipeline YAML in `schema.yaml`                   | High      | Build skill executes from pipeline YAML; prose pipeline removed; conformance check passes     |
| **7**  | Rust engine (`specify-core`)                     | Very high | `specify-core` replaces `merge-specs.ts` and `specify validate` Deno implementation           |


### Parallelism and Dependencies

The foundational deliverables — Phase 1 (feedback schemas), Phase 2 (structured validation rules), and Phase 3A (lifecycle schema) — can be **defined** in parallel since they target different sections of the schema and different runtime concerns. However, they share `schema.schema.json` as an edit target, so parallel work requires clear ownership of schema file sections and merge-order coordination.

Phase 3B (structured IR) is a substantial standalone effort that can proceed independently of Phases 1 and 3A, but benefits from Phase 2's validator architecture (the format-agnostic check engine). It should not block other work.

The integration deliverables have sequential dependencies:
- Phase 2.4 (wire gates into skills) depends on Phase 2.3 (validator CLI)
- Phase 4 depends on Phase 1 (feedback files) and Phase 2 (validator CLI)
- Phase 5 depends on Phase 4
- Phase 6 can start alongside Phase 5 once loop patterns stabilize

Phase 7 sub-phases have independent entry criteria:
- 7.1 (parser + validator) can start after Phase 2 stabilizes
- 7.2 (merge) depends on 7.1
- 7.3 (lifecycle engine) can start after Phase 3A
- 7.4 (pipeline orchestrator) depends on Phase 6

---

## Trade-offs and Risks

### Over-constraining the creative parts

Skills like `spec:analyze` Phase 3 (domain-by-domain extraction) and `code-reviewer` category checklists require genuine language understanding. The THINK prompts ("Before extracting each type, reason through...") leverage the LLM's flexible reasoning. **Constrain the process, liberate the content generation.**

### YAML complexity ceiling

As pipeline definitions grow, YAML becomes its own DSL. If it needs conditional logic (`if Cargo.toml exists, use update pipeline`), inheritance, or parameterization, YAML gets unwieldy. The YAML layer declares *what happens*; the Rust engine handles complex control flow. Don't over-invest in YAML expressiveness — that's Phase 7's job.

### Context budget strategy

Context pressure in RWL loops comes from multiple compounding sources, not just compiler output:

1. **Compiler/test output**: The diagnostic formatter (Phase 1.3) truncates stack traces and provides only file, line, and localized error message. This is critical infrastructure, not an optimization.
2. **Feedback file growth**: As iterations accumulate, feedback sidecars grow. Define a maximum feedback file size or summarize across iterations (keeping only the latest iteration's findings).
3. **Spec/design payload**: Repair passes may not need the full spec + design. The per-handler approach (Phase 5) inherently mitigates this by scoping each LLM call to a single handler's spec subset.
4. **Accumulated repair history**: Avoid feeding the full history of prior repair attempts. Each repair pass receives only the current classified error and the relevant source context.

The per-handler decomposition in Phase 5 is the primary structural mitigation — it reduces scope per LLM call. The diagnostic formatter addresses the remaining single largest source of bloat.

### Cost and latency from nested loops

Code-reviewer spawns 4 agents. Running this inside a loop, combined with multiple crate-writer and test-writer invocations, significantly increases token costs. **Layer the RWL strictly**: never run the expensive code-reviewer until the cheap `cargo check` and `cargo test` loops have fully passed.

### Test vs. code authority conflict

When a test fails, the system doesn't inherently know if the code is wrong or the test was hallucinated. **The spec is the arbiter.** Compare the failing test against the `spec.md` ground truth. If the test matches the spec, fix the code. If the test deviates, fix the test. This principle is already in `build.md`; it must be preserved through all RWL work.

### Phase 2 / Phase 3B input source conflict

Phase 2 builds a validator that parses Markdown. Phase 3B introduces a structured IR where the LLM produces YAML that is rendered to Markdown. During transition, two sources of truth coexist. If the validator only checks Markdown, IR-only errors are invisible. If it only checks IR, manually edited Markdown drifts undetected. The validator architecture must be format-agnostic from Phase 2 (see 2.3) and Phase 3B.2 defines the auto-detection and convergence strategy.

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