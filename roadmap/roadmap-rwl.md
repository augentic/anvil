# RWL-Optimized Roadmap: Determinism and Iteration

Iterative roadmap for making Specify more deterministic, introducing Ralph Wiggum Loop (RWL) iteration for high-context skills, and adding compile-time type safety to skill definitions. Reorganized from the [phase-gated roadmap](roadmap.md) into iteration-based delivery, where each iteration produces a thin but complete vertical slice across all concerns and feedback from seeing the whole system work guides subsequent refinement. Supersedes the separate `deterministic.md`, `rwl.md`, and `dsl.md` documents.

## Design Principle

**The LLM generates content; code makes process decisions.** Every time a SKILL.md instruction says "check that X is true and halt if not," that decision should be in code. The LLM is excellent at understanding requirements and producing prose/code; it is unreliable at self-assessment, state management, structural validation, and loop control.

**Applied recursively**: This roadmap implements itself as an RWL. Each iteration's convergence is machine-checkable via `make checks` and the `specify` CLI — not judged by feel.

## Multi-Schema Scope

This roadmap targets **Omnia first**. The `schemas/vectis/schema.yaml` schema has the same prose `validate` arrays, implicit lifecycle, and prose-embedded loop control — but Vectis has a fundamentally different pipeline shape (`core-writer → ios-writer → android-writer → design-system-writer`) and platform-specific verification (`make build` for Xcode, `./gradlew :app:assembleDebug` for Android, not `cargo check`/`cargo test`).

Deliverables fall into two categories:

- **Schema-agnostic infrastructure** (shared): feedback file schemas, validator CLI, lifecycle state machine, diagnostic formatter, pipeline engine. Built once, consumed by both schemas.
- **Schema-specific definitions** (per-schema): structured validation rules, pipeline YAML, blueprint IR schemas. Defined separately for Omnia and Vectis.

Iterations 1–3 deliver Omnia-specific definitions alongside shared infrastructure. Iteration 4 extends to Vectis parity. Pipeline YAML step types must accommodate platform-specific build commands from the start — not just Cargo toolchain.

## Tracks

The roadmap maintains eight concurrent tracks. Each iteration advances all tracks, but at increasing depth. The tracks correspond to the concerns from the original phase-gated roadmap plus skill type safety from `dsl.md`:

| Track | Concern | Primary files |
| ----- | ------- | ------------- |
| **F** | Structured feedback files + diagnostic formatter | `.specify-feedback.yaml`, `.review-findings.yaml`, `specify diag` |
| **V** | Structured validation rules + validator CLI | `schema.yaml` validate arrays, `specify validate` |
| **L** | Lifecycle state machine | `schema.yaml` lifecycle section, `.metadata.yaml` |
| **IR** | Structured intermediate representation | `.blueprint-plan.yaml`, renderer |
| **RWL** | Deterministic verify-repair + skill chaining | `specify diag classify`, per-handler co-refinement |
| **P** | Pipeline YAML — declarative orchestration | `schema.yaml` pipelines section |
| **S** | Skill type safety — structured manifests + Rust DSL | `manifest.yaml` per skill, `skill-dsl/` crate, `checks.ts` |
| **E** | Rust engine (`specify-core`) | `specify-core` crate |

## Interface Contracts

In an iteration-based approach, tracks don't gate each other by completion — they gate each other by interface stability. Each iteration defines the minimum viable surface between tracks.

### Iteration 1 interfaces

- **F → RWL**: `.specify-feedback.yaml` has at minimum `schema_version` and `skill` fields. Skills can produce a feedback file; consumers can read it. The schema may grow but these fields are stable.
- **V → RWL**: `specify validate` accepts a `--schema-version` flag, validates at least one structured rule, and returns exit code 0/1.
- **V → IR**: The validator's check evaluation engine accepts a parsed document model, not raw Markdown. This ensures IR validation is possible without rewriting the engine.
- **F → P**: Pipeline YAML references skill IDs and step types. Feedback files use the same skill ID vocabulary.
- **S → V**: YAML manifest schema defines the `allowed-tools` enum and `arguments` structure. `checks.ts` cross-checks manifests against SKILL.md frontmatter. The manifest uses the same skill ID vocabulary as pipeline YAML and feedback files.
- **E → V**: `specify-core` exports a `parse_spec` function that returns `Vec<RequirementBlock>`. The Deno validator and Rust parser share the same notion of "requirement block."

### Iteration 2 interfaces

- **F**: Feedback files include `handlers_generated`, `known_gaps`, and `findings` arrays.
- **V**: `specify validate` handles `heading-present`, `pattern-match`, `keyword-present`, and cross-artifact reference checks.
- **L**: Lifecycle transitions `defining→defined→building` are enforced. `specify validate` checks `.metadata.yaml` status.
- **RWL**: `specify diag classify` distinguishes `test_issue` from `code_issue`. Skills accept structured feedback for repair via context injection.
- **P**: Pipeline YAML drives the verify step. The build skill reads and follows it.
- **S → P**: Manifests declare `skill-directives` (cross-skill references). `checks.ts` validates that pipeline YAML skill references match manifest-declared skills. Variable DAGs in manifests are schema-validated for completeness and acyclicity.
- **E**: `specify-core` validates all structured rules that the Deno validator handles for `specs`.

### Iteration 3 interfaces

- All Omnia validation rules are structured; zero prose-only rules remain.
- `specify-core` replaces the Deno validator and `merge-specs.ts`.
- Pipeline YAML drives the full build chain. Prose pipeline removed.
- Per-handler co-refinement runs end-to-end.
- Rust DSL in `specify-core` generates SKILL.md from typed `SkillDef` structs. Generated output matches hand-authored SKILL.md. `checks.ts` validates rendered output as defense-in-depth.

### Iteration 4 interfaces

- Vectis-specific definitions use the same infrastructure.
- `specify-core` pipeline orchestrator replaces the LLM as loop controller.
- Structured IR round-trips through all four blueprints.
- Rust DSL covers all Vectis skills. Skill composition enables deriving platform-variant skills from shared definitions.

## Migration and Backward Compatibility

Each iteration ships a bundle of cross-cutting changes rather than a single-concern release. This requires more disciplined migration tooling from the start.

**Schema versioning**: The `version` field in `schema.yaml` already exists. Each iteration that changes the schema increments this version. The validator CLI accepts a `--schema-version` flag and defaults to the latest. Older projects validate cleanly against their declared version.

**Graceful degradation**: New files (feedback sidecars, frontmatter, lifecycle schema) are optional during transition. The validator reports their absence as warnings, not errors, until the project declares a schema version that requires them.

**Migration command**: `specify migrate` ships in Iteration 1 (not deferred to later) because cross-cutting changes demand it early. It upgrades an existing `.specify/` structure to the latest schema version — adding frontmatter to existing artifacts, creating missing `.metadata.yaml` fields, and updating `config.yaml` to the new version.

**In-flight changes**: A change currently in `building` status when the schema changes continues under the old schema version. The migration command only upgrades the project baseline, not active changes. Active changes complete under the version they started with.

**Iteration bundles**: Because each iteration touches multiple schema sections, the migration command must handle bundled changes atomically. Each iteration's migration is a single version bump, not per-track increments.

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
| **Skill type safety**     | `checks.ts` validates frontmatter, references, variables, directives, marketplace alignment, inventory                                                   | **Medium** — ~750 lines of CI-time checks, but SKILL.md bodies remain unvalidated     | `scripts/checks.ts`, SKILL.md frontmatter          |
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

## Iteration 1: Skeleton

**Goal**: A thin vertical slice across all tracks. Every track has one working example. The `specify` CLI exists with all subcommands stubbed. `specify-core` compiles. The system is end-to-end observable even if most checks are trivial.

**Convergence**: `make checks` passes with new schema sections; `specify validate` gates one structured rule; `specify-core` compiles and parses one heading type; `specify diag format` parses one Cargo diagnostic; pipeline YAML is valid against schema.

### Track F: Feedback files — minimal schema

Define JSON Schema for `.specify-feedback.yaml` and `.review-findings.yaml` with the minimum viable fields:

```yaml
# $CRATE_PATH/.specify-feedback.yaml (produced by crate-writer)
schema_version: 1
skill: omnia:crate-writer
mode: create
handlers_generated: []
known_gaps: []
```

```yaml
# $CRATE_PATH/.review-findings.yaml (produced by code-reviewer)
schema_version: 1
findings: []
```

Wire crate-writer to produce a `.specify-feedback.yaml` with `schema_version` and `skill` fields. The arrays can be empty stubs — the schema exists and is validated by `checks.ts`.

Add cleanup rules: feedback sidecars are removed from the crate path at merge time by the merge skill, archived to `.specify/changes/<change>/archive/`.

### Track F: Diagnostic formatter — `specify diag format`

Build the `specify` CLI entry point (Deno/TypeScript) with `specify diag format` as its first subcommand:

```bash
cargo check --message-format=json 2>&1 | specify diag format
```

The formatter defines an explicit input contract: `cargo check --message-format=json` produces Cargo diagnostic JSON. For stable toolchains where `--format json` is unavailable for test output, `specify diag` falls back to parsing human-readable test output. Output: structured YAML with file, line, error code, and snippet.

### Track V: One structured validation rule

Extend `schema.yaml` `validate` arrays to support structured check objects alongside prose strings (backward-compatible):

```yaml
validate:
  - check: heading-present
    heading: "#### Scenario:"
    scope: per-requirement
    description: Every requirement has at least one scenario
  # Backward-compatible: plain strings still accepted
  - Has a Why section with at least one sentence
```

Extend `schema.schema.json` to accept either a string or a structured check object:

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

**Scope vocabulary**: `scope` defines the unit of evaluation:
- `document` — check applies once to the entire artifact file
- `per-requirement` — check runs once per `### Requirement:` block
- `per-scenario` — check runs once per `#### Scenario:` block within each requirement

Build `specify validate` as a subcommand of the unified `specify` CLI. It validates one rule end-to-end: `heading-present` for `#### Scenario:` within the `specs` blueprint. Returns exit code 0/1.

**Architecture constraint**: The validator's check evaluation engine accepts a parsed document model, not raw Markdown. This makes it reusable when the structured IR arrives.

### Track V: Artifact frontmatter

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

### Track L: Lifecycle skeleton in schema

Add the lifecycle section to `schema.yaml` with all states defined, but only the `defining→defined` transition enforced:

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

Add the `lifecycle` section to `schema.schema.json`. Build a lifecycle validator stub in `specify validate` that checks `defining→defined` and reports all other transitions as pass-through.

### Track IR: Schema definition only

Define the `.blueprint-plan.yaml` schema for the `specs` blueprint:

```yaml
# .specify/changes/my-change/.blueprint-plan.yaml
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

No renderer yet — that is Iteration 2. The schema exists so that the validator architecture can be designed format-agnostically from the start, and so the Rust parser can target it.

### Track RWL: Define-side artifact quality loop

Connect the single validator rule to a generate → validate → refine loop in the `define` skill:

```
define generates artifacts → specify validate (deterministic)
→ if failures: LLM refines → re-validate (max 2 iterations)
```

The validator is code; the correction is LLM. This is the simplest RWL and establishes the pattern for all subsequent loops.

Support `SPECIFY_MAX_VERIFY_ITERATIONS` environment variable for CI control. Default: 3.

### Track P: Skeletal pipeline YAML

Add a `pipelines` section to `schema.yaml` with the `create` pipeline's verify step only:

```yaml
pipelines:
  create:
    steps:
      - id: verify
        type: loop
        max_iterations: 3
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
```

Add the `pipelines` section to `schema.schema.json` with step types (`skill`, `loop`, `run`), `after` dependency DAG, convergence criteria, and routing tables. The full pipeline is filled in during Iteration 2.

**Variable substitution**: Pipeline YAML uses `${{ vars.CHANGE_ID }}` and `${{ outputs.step_id }}` syntax rather than raw shell interpolation.

### Track E: `specify-core` crate compiles

Create the `specify-core` crate with a minimal spec parser:

- Parse the `### Requirement:` heading from a spec file into a `RequirementBlock` struct
- Validate the `heading-present` rule (same rule as the Deno validator)
- Emit one diagnostic in JSON format
- Ship as a library crate; CLI binary is Iteration 2

The Rust parser and the Deno validator must produce identical results for the same input on this single rule. This is the integration test that proves the two implementations can coexist.

### Track S: YAML manifest schema + checks.ts hardening

Skills have two distinct layers: **structural metadata** (dependencies, tools, arguments, phase ordering) and **behavioral instructions** (how the agent should think and act). The structural layer benefits enormously from typing. The behavioral layer is inherently natural language. Today, `checks.ts` already validates frontmatter schemas, skill references, variable consistency, directives, marketplace alignment, and inventory — but SKILL.md bodies remain structurally unvalidated.

Define a YAML manifest schema for skill structural metadata and introduce it for one skill (`omnia:crate-writer`):

```yaml
# plugins/omnia/skills/crate-writer/manifest.yaml
name: crate-writer
plugin: omnia
description: "Write Rust WASM crates from Specify artifacts..."
license: MIT
argument-hint: "[crate-name]"

allowed-tools:
  - Read
  - Write
  - StrReplace
  - Shell
  - Grep
  - ReadLints

arguments:
  positional:
    - name: crate-name
      var: CRATE_NAME
      position: 0
      required: true
  derived:
    - name: CHANGE_DIR
      expr: ".specify/changes/$CRATE_NAME"
      depends_on: [CRATE_NAME]

references:
  - id: sdk-api
    path: references/sdk-api.md
    mode: both

skill-directives:
  - omnia:test-writer
  - omnia:guest-writer
```

Add a JSON Schema for `manifest.yaml` and validate it in `checks.ts`. Cross-check the manifest against the SKILL.md frontmatter — the manifest is the source of truth for structural metadata; the frontmatter must match. This extends the existing `checks.ts` infrastructure with structured skill metadata without changing the authoring format.

| What the manifest catches | Mechanism |
|---|---|
| Typo in `allowed-tools` (e.g., `Readlints` vs `ReadLints`) | Enum in JSON Schema |
| Variable `$CRATE_PATH` used but never defined | `depends_on` DAG validated for completeness |
| Skill directive references non-existent skill | Cross-check against skill registry |
| Manifest ↔ SKILL.md frontmatter divergence | Cross-check in `checks.ts` |

### Track V+L: `specify migrate`

Ship `specify migrate` in Iteration 1. It upgrades an existing `.specify/` structure to the latest schema version — adding frontmatter to existing artifacts, creating missing `.metadata.yaml` fields, updating `config.yaml` to the new version. Because each iteration bundles cross-cutting schema changes, migration tooling must exist from the start.

### Deliverables

| Track | Deliverable | Gate |
| ----- | ----------- | ---- |
| F | JSON Schema for `.specify-feedback.yaml` and `.review-findings.yaml` | `checks.ts` validates schemas |
| F | `specify` CLI entry point with `specify diag format` | Parses one `cargo check` diagnostic |
| F | crate-writer produces minimal feedback sidecar | File exists and validates |
| F | Merge skill archives feedback files | Archive directory created |
| V | Structured check type in `schema.schema.json` | Ajv validates the new format |
| V | One structured rule in `schemas/omnia/schema.yaml` | `specify validate` enforces it |
| V | `specify validate` subcommand | Exit code 0/1 on one rule |
| V | Artifact frontmatter in `define` skill | Frontmatter present in generated artifacts |
| L | `lifecycle` section in `schema.yaml` + `schema.schema.json` | Schema validates |
| L | `defining→defined` transition enforced | `specify validate` rejects invalid transition |
| IR | `.blueprint-plan.yaml` schema for `specs` | Schema validates |
| RWL | Define-side artifact quality loop | `define` retries on validation failure |
| RWL | `SPECIFY_MAX_VERIFY_ITERATIONS` env var | Respected by define loop |
| P | Skeletal `pipelines` section in `schema.yaml` | Schema validates; `checks.ts` passes |
| P | Pipeline schema in `schema.schema.json` | Ajv validates pipeline definitions |
| E | `specify-core` crate: parse one heading type | `cargo test` passes; matches Deno output |
| S | JSON Schema for `manifest.yaml` | Ajv validates the schema |
| S | `manifest.yaml` for `omnia:crate-writer` | `checks.ts` cross-checks against frontmatter |
| V+L | `specify migrate` subcommand | Upgrades a test `.specify/` directory |

---

## Iteration 2: Single-Crate End-to-End

**Goal**: One Omnia crate's full build cycle runs with structured feedback, deterministic validation, pipeline-driven verify-repair, and Rust-validated specs. The system is useful, not just observable.

**Convergence**: A single crate build succeeds with `specify validate` gating all artifacts, `specify diag classify` routing all verify-repair failures, the build skill following pipeline YAML for the verify step, and `specify-core` validating all structured rules for `specs`.

### Track F: Full feedback schemas

Extend `.specify-feedback.yaml` to include `handlers_generated` with per-handler `cargo_check` status and `spec_coverage`, plus `known_gaps`:

```yaml
schema_version: 1
skill: omnia:crate-writer
mode: create
handlers_generated:
  - name: CreateWorksite
    file: src/handlers/create_worksite.rs
    cargo_check: pass
    spec_coverage: [REQ-001, REQ-002]
known_gaps:
  - type: todo-marker
    file: src/handlers/create_worksite.rs
    line: 67
    description: "Cache-aside pattern not implemented"
    spec_reference: REQ-002
```

Extend `.review-findings.yaml` with typed findings:

```yaml
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
```

Wire all three skills:
- **crate-writer**: Produce full `.specify-feedback.yaml`. Accept feedback-injection mode for repair passes via context injection.
- **test-writer**: Read crate-writer's feedback for handler signatures, trait bounds, and spec coverage. Produce its own sidecar with test-to-spec mapping.
- **code-reviewer**: Produce `.review-findings.yaml` alongside `REVIEW.md`.

### Track F: Diagnostic classifier

Add `specify diag classify` subcommand for failure classification:

1. Parse `cargo test` output (using `specify diag format`)
2. Extract failing test names and error locations
3. If the error location is in `tests/` → **test issue**
4. If the error location is in `src/` → **code issue**
5. For assertion mismatches: produce structured input for LLM judgment

Steps 1–4 are code. Step 5 is the residual that stays with the LLM. Add `cargo test -- --format json` support with stable-toolchain fallback.

### Track V: Full Omnia validation rules

Convert all `schemas/omnia/schema.yaml` validate rules to structured format. Add check types `pattern-match`, `keyword-present`, and cross-artifact references (`proposal-crates-have-specs`, design references valid IDs, tasks reference existing specs).

Wire the gate: `/spec:build` and `/spec:define` invoke `specify validate` and halt on non-zero exit. **Replace** (not duplicate) each prose validation paragraph in skills with the corresponding structured rule.

### Track L: Full lifecycle enforcement

Enforce all transitions: `defining→defined→building→complete→merged` and `*→dropped`. Build the state-machine validator in `specify validate`. Remove "valid lifecycle status values" guardrail paragraphs from all skills.

### Track IR: Renderer for `specs` blueprint

Build the renderer: `.blueprint-plan.yaml` → markdown `spec.md`. Constrain LLM output to the IR schema during generation in the `define` skill.

Add auto-detection to `specify validate`: if `.blueprint-plan.yaml` exists and is newer than rendered artifacts, validate the IR. Otherwise, validate the Markdown.

Proof-of-concept round-trip: IR → Markdown → merge → verify.

### Track RWL: Inner loop — deterministic verify-repair

Replace the prose verify-repair loop in `build.md` with structured control:

- crate-writer feedback-injection repair mode (receive classified errors, fix reported errors only)
- test-writer feedback-injection repair mode
- Iteration counter in `.metadata.yaml`
- **Repair discipline**: minimum change only, one failure class per re-entry, scope the diff

### Track RWL: Per-handler co-refinement

Introduce per-handler generation for a single crate:

```
Step 0: crate-writer generates shared types, domain models, and module tree
        → cargo check (structural smoke)

For each handler in the handler manifest:
  1. crate-writer generates handler code
  2. cargo check (structural smoke)
  3. test-writer generates tests for that handler
  4. cargo test (behavioral check)
  5. If failures: classify → route to appropriate skill
  6. Max 2 refinement passes per handler
```

The handler manifest is a structured list of handlers to generate, with dependencies, trait bounds, shared type references, and matrix entries.

**Fallback**: If per-handler iteration doesn't converge in 2 passes, fall back to whole-crate iteration.

### Track S: All Omnia skill manifests

Extend `manifest.yaml` to all Omnia skills (`crate-writer`, `test-writer`, `guest-writer`, `code-reviewer`). Each manifest declares:

- `allowed-tools` (validated against the Tool enum in the JSON Schema)
- `arguments` with positional args, derived variables, and `depends_on` DAGs (schema-validated for completeness and acyclicity)
- `references` with paths (cross-checked for existence)
- `skill-directives` (cross-checked against the skill registry)

`checks.ts` validates all manifests, cross-checks against SKILL.md frontmatter, and verifies that pipeline YAML skill references match manifest-declared skill IDs. This closes the gap between pipeline orchestration (Track P) and skill metadata — the same skill ID vocabulary is enforced everywhere.

### Track P: Full create pipeline

Fill in the complete pipeline definition:

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

The build skill reads and follows the pipeline YAML for the verify step. Prose verify-repair loop remains for non-verify steps until Iteration 3.

Add pipeline DAG validation and conformance check to `checks.ts`.

### Track E: Validator parity

Extend `specify-core` to:
- Parse spec files into typed AST (`RequirementBlock`, `Scenario`, `DeltaOperation`)
- Validate all structured rules that the Deno validator handles for `specs`
- Emit diagnostics in JSON/SARIF format
- Ship as CLI binary alongside the Deno implementation (both runnable, Deno is still primary)

### Deliverables

| Track | Deliverable | Gate |
| ----- | ----------- | ---- |
| F | Full `.specify-feedback.yaml` and `.review-findings.yaml` schemas | All three skills produce/consume structured feedback |
| F | `specify diag classify` | Routes test vs code issues deterministically |
| V | All Omnia `specs` validate rules structured | `specify validate` enforces all; prose paragraphs removed |
| V | Cross-artifact reference validation | `proposal-crates-have-specs` etc. checked by code |
| L | All lifecycle transitions enforced | Guardrail paragraphs removed from skills |
| IR | `.blueprint-plan.yaml` → Markdown renderer | Round-trip passes: IR → Markdown → merge → verify |
| IR | `specify validate` auto-detection | IR vs Markdown input handled transparently |
| RWL | Deterministic verify-repair loop | Prose loop replaced in `build.md` |
| RWL | Per-handler co-refinement | One crate builds with per-handler iteration |
| RWL | Handler manifest schema | crate-writer produces manifest after cross-cutting analysis |
| P | Full `create` and `update` pipelines | Build skill follows pipeline YAML for verify step |
| P | Pipeline conformance check | `checks.ts` validates skill ↔ pipeline alignment |
| S | All Omnia skill manifests | `checks.ts` validates all manifests; cross-check passes |
| S | Pipeline ↔ manifest skill ID alignment | Same vocabulary enforced everywhere |
| E | `specify-core` validates all `specs` rules | Matches Deno output on full rule set |
| E | `specify-core` CLI binary | `specify-core validate` runs standalone |

---

## Iteration 3: Full Omnia Determinism

**Goal**: The entire Omnia pipeline runs deterministically. `specify-core` replaces the Deno validator and merge script. The build skill executes entirely from pipeline YAML. All prose-based process control is removed.

**Convergence**: Zero prose-only validate rules in `schemas/omnia/schema.yaml`; `specify-core` is the sole validator and merge engine; pipeline YAML drives the full build chain including review and remediation; all Omnia skills produce and consume structured feedback.

### Track F: Feedback lifecycle complete

All Omnia skills produce feedback sidecars. `checks.ts` validates all feedback file schemas. Merge skill archives feedback files to `.specify/changes/<change>/archive/`. Maximum feedback file size defined; only latest iteration's findings retained.

### Track V: All blueprints validated

Extend structured validation to all four Omnia blueprints (`proposal`, `specs`, `design`, `tasks`). The validator cross-references `req_ids` in frontmatter against the body.

### Track L: Complete

All lifecycle transitions enforced by code. No lifecycle guardrail paragraphs remain in any skill.

### Track IR: All Omnia blueprints

Extend the structured IR and renderer to `proposal`, `design`, and `tasks` blueprints. Handle escaping issues: long-form prose, Mermaid diagrams, complex scenarios in YAML string literals. Migration guide for instruction files targeting the IR format.

### Track RWL: Code-reviewer structured routing + analyzer self-critique

Chain code-reviewer with structured routing:

```
code-reviewer produces .review-findings.yaml
→ filter by severity (CRITICAL/HIGH only)
→ group by skill_target
→ route each group to the target skill via feedback injection
→ verify-repair (max 2 iterations)
→ re-review to verify quality
→ if new CRITICAL: one more remediation cycle
```

Add code-analyzer self-critique loop:

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

### Track P: Pipeline drives full build

The build skill executes entirely from pipeline YAML. Remove all prose pipeline descriptions from `schemas/omnia/instructions/build.md`. The full Omnia create-mode chain:

```
guest-writer
  → crate-writer (with handler manifest)
    → [per-handler: cargo check → test-writer → cargo test → classify]
      → verify-repair loop (max 3, deterministic classification)
        → code-reviewer (structured findings)
          → remediation loop (max 2, routed by skill_target)
            → final re-review
```

### Track S: Rust DSL generates SKILL.md from typed definitions

With YAML manifests stable (Iteration 2) and `specify-core` available, move skill structural metadata into Rust. Define skills as Rust structs in a `skill-dsl` crate, embed prose blocks via `include_str!`, and generate SKILL.md at build time. The Rust compiler and a build script enforce correctness.

What the compiler catches that YAML manifests cannot:

1. **Broken references**: `ref!(omnia::nonexistent)` fails to compile because the target does not exist as a const/type.
2. **Blueprint alignment**: `artifact!(design)` checks against an enum generated from `schema.yaml`.
3. **Phase DAG validation**: dependency cycles or missing phases caught at compile time.
4. **Tool allow-lists**: enum-based, so typos are impossible.
5. **Cross-skill directives**: typed consts validated against a registry.
6. **Variable DAGs**: `depends_on` fields are checked for completeness and acyclicity at compile time, not CI time.

Core types:

```rust
pub struct SkillDef {
    pub name: SkillId,
    pub plugin: PluginId,
    pub description: &'static str,
    pub license: License,
    pub arguments: Arguments,
    pub allowed_tools: &'static [Tool],
    pub references: &'static [Reference],
    pub authority: &'static [AuthorityLevel],
    pub rules: &'static [HardRule],
    pub phases: Vec<Phase>,
    pub modes: Vec<Mode>,
    pub body_sections: Vec<Section>,
}

pub enum Tool {
    Read, Write, StrReplace, Shell, Grep, Glob,
    ReadLints, WebFetch, WebSearch, AskQuestion,
    Task, TodoWrite, SemanticSearch,
    Mcp(&'static str),
}

pub struct Arguments {
    pub positional: &'static [Arg],
    pub derived: &'static [DerivedVar],
}
```

Build integration — the Makefile chains generation before validation:

```makefile
.PHONY: generate
generate:
	cargo run --manifest-path skill-dsl/Cargo.toml --bin generate

.PHONY: checks
checks: generate
	@$(DENO) run --allow-read scripts/checks.ts
```

`checks.ts` continues validating rendered SKILL.md output as defense-in-depth. The Rust DSL catches structural errors at compile time; `checks.ts` catches rendering regressions at CI time.

The DSL consumes the same structured manifests (feedback schemas, pipeline YAML, skill manifests) that `specify-core` uses at runtime. The DSL catches structural errors at compile time; the engine catches process errors at runtime.

### Track E: Replace Deno implementations

- `specify-core` replaces `merge-specs.ts` for merge
- `specify-core` replaces Deno validator for all structured rules
- `specify-core` handles drift detection
- `specify-core` lifecycle engine reads schema, manages `.metadata.yaml` transitions
- Ship as single CLI binary
- WASM build for editor integration (optional)

### Deliverables

| Track | Deliverable | Gate |
| ----- | ----------- | ---- |
| F | All Omnia skills produce/consume feedback | `checks.ts` validates all schemas |
| V | All four blueprints use structured validation | Zero prose-only validate rules remain |
| L | Complete lifecycle enforcement | No guardrail paragraphs in any skill |
| IR | All Omnia blueprints use structured IR | Round-trip for all four blueprints |
| RWL | Code-reviewer structured routing | Routing is data-driven from `.review-findings.yaml` |
| RWL | Code-analyzer self-critique | Partial automation of V1, V3, V4 checks |
| P | Full pipeline-driven build | Prose pipeline removed; conformance check passes |
| S | Rust DSL generates all Omnia SKILL.md files | Generated output matches hand-authored; `checks.ts` validates rendered |
| S | `skill-dsl` crate with typed `SkillDef` structs | `cargo build` catches broken refs, tool typos, DAG cycles |
| E | `specify-core` replaces Deno validator + merge | Single binary, JSON/SARIF diagnostics |
| E | `specify-core` lifecycle engine | State machine invariants at engine level |

---

## Iteration 4: Vectis Parity + Pipeline Orchestrator

**Goal**: Vectis schema uses the same infrastructure with schema-specific definitions. The Rust pipeline orchestrator replaces the LLM as loop controller. The LLM becomes a content oracle called by the engine.

**Convergence**: Both Omnia and Vectis pipelines run end-to-end through `specify-core`; the LLM is invoked for content generation only, never for process decisions.

### Track F–V–L: Vectis-specific definitions

Define Vectis-specific:
- Structured validation rules for multi-platform build verification
- Feedback schemas adapted for `core-writer → ios-writer → android-writer → design-system-writer` pipeline shape
- Lifecycle transitions for the Vectis workflow

### Track IR: Full convergence

Structured IR for Vectis blueprints. `specify validate` auto-detects IR vs Markdown for both schemas.

### Track P: Vectis pipeline YAML

Pipeline step types accommodate platform-specific build commands: `make build` for Xcode, `./gradlew :app:assembleDebug` for Android. The `step` type vocabulary is generic; schema-specific definitions provide the commands.

### Track S: Vectis skills + full composability

Extend the Rust DSL to all Vectis skills (`core-writer`, `ios-writer`, `android-writer`, `design-system-writer`, `core-reviewer`, `ios-reviewer`, `android-reviewer`, `test-writer`). Skill composition becomes valuable at this scale — platform-variant skills (e.g., `ios-reviewer` and `android-reviewer` share structural patterns but differ in platform-specific checks) can be derived from shared base definitions rather than maintained independently.

The full comparison of approaches at this point:

| Dimension | checks.ts (Iteration 1) | YAML Manifests (Iteration 2) | Rust DSL (Iteration 3–4) |
|---|---|---|---|
| **Authoring format** | Markdown directly | YAML + markdown | Rust structs + `include_str!` |
| **Feedback loop** | CI-time (`make checks`) | CI-time (`make checks`) | Compile-time (`cargo build`) |
| **Broken references** | Runtime file-exists check | Runtime file-exists check | Build script panic |
| **Tool name typo** | String comparison | String against schema enum | Enum variant — won't compile |
| **Variable consistency** | Regex-based heuristic | Schema-validated DAG | Typed `depends_on` DAG |
| **Composability** | Limited | Medium | High — skills are data |

`checks.ts` remains as defense-in-depth at every iteration. The YAML manifest is the pragmatic intermediate step that the Rust DSL subsumes but does not eliminate — manifests continue to serve as the human-readable declaration that `checks.ts` validates against rendered output.

### Track E: Pipeline orchestrator

`specify-core` pipeline orchestrator:
- Read `schema.yaml` pipelines, resolve DAG
- Execute pipeline steps: invoke LLM for content generation via defined interface, run validation deterministically, control loops, route failures
- The LLM becomes a "content oracle" called by the engine

### Deliverables

| Track | Deliverable | Gate |
| ----- | ----------- | ---- |
| F–V–L | Vectis-specific feedback, validation, lifecycle | `make checks` passes for Vectis schema |
| IR | Vectis structured IR | Round-trip for Vectis blueprints |
| P | Vectis pipeline YAML | Platform-specific build commands work |
| S | Rust DSL covers all Vectis skills | `cargo build` catches all structural errors; composition works |
| E | `specify-core` pipeline orchestrator | Both schemas run end-to-end through engine |

---

## Iteration Summary


| Iteration | Scope | Done when |
| --------- | ----- | --------- |
| **1** | Skeleton: one rule, one parse, one loop, one manifest, all tracks present | `make checks` passes; `specify validate` gates one rule; `specify-core` compiles; pipeline YAML valid; one skill manifest validates |
| **2** | Single-crate end-to-end: full Omnia feedback, all `specs` rules, pipeline-driven verify, per-handler co-refinement, all Omnia manifests | One crate build succeeds with structured feedback, deterministic validation, pipeline-driven verify-repair; all Omnia manifests cross-checked |
| **3** | Full Omnia determinism: `specify-core` replaces Deno; pipeline drives full build; Rust DSL generates SKILL.md; all prose control removed | Zero prose-only rules; `specify-core` is sole validator/merge; pipeline YAML drives full chain; `cargo build` on `skill-dsl` catches structural errors |
| **4** | Vectis parity + Rust orchestrator + full DSL composability: LLM becomes content oracle | Both schemas run end-to-end through `specify-core` pipeline orchestrator; Rust DSL covers all skills with composition |

---

## Trade-offs and Risks

### Over-constraining the creative parts

Skills like `spec:analyze` domain-by-domain extraction and `code-reviewer` category checklists require genuine language understanding. The THINK prompts ("Before extracting each type, reason through...") leverage the LLM's flexible reasoning. **Constrain the process, liberate the content generation.**

### Interface instability between tracks

The defining risk of the RWL approach. When all tracks advance together, interfaces between them are being defined and used simultaneously. A feedback schema change in Iteration 2 might break the pipeline YAML work also happening in Iteration 2. **Mitigation**: The Interface Contracts section above defines minimum viable surfaces per iteration. Changes to a track's interface within an iteration require explicit coordination.

### Dual maintenance across all tracks

In the phase-gated roadmap, dual maintenance (prose + structured) is a transition concern. In the RWL approach, it is the **defining tension** of the entire implementation. During Iterations 1–2, skills contain both prose instructions (for the LLM) and structured YAML (for the validator). If these diverge, the LLM follows one set of rules while the validator enforces another. **Each structured rule must replace, not duplicate, the corresponding prose instruction** — from the very first iteration.

### Spreading too thin

Each iteration produces something across all eight tracks. If the team is small, each track gets very little attention per pass. **Mitigation**: Iterations are not time-boxed — they are convergence-gated. An iteration is done when its convergence criterion passes, not when a sprint ends. Focus investment where convergence is hardest.

### YAML complexity ceiling

Pipeline definitions exist from Iteration 1 and accumulate complexity over more iterations than in the phased approach. As pipeline definitions grow, YAML becomes its own DSL. **The YAML layer declares *what happens*; the Rust engine handles complex control flow.** Don't over-invest in YAML expressiveness — that's Iteration 3–4's job.

### Context budget strategy

Context pressure in RWL loops comes from multiple compounding sources:

1. **Compiler/test output**: The diagnostic formatter truncates stack traces and provides only file, line, and localized error message.
2. **Feedback file growth**: Define a maximum feedback file size. Keep only the latest iteration's findings.
3. **Spec/design payload**: The per-handler approach inherently mitigates this by scoping each LLM call to a single handler's spec subset.
4. **Accumulated repair history**: Each repair pass receives only the current classified error and the relevant source context. No full history.

### Cost and latency from nested loops

Code-reviewer spawns 4 agents. Running this inside a loop, combined with multiple crate-writer and test-writer invocations, significantly increases token costs. **Layer the RWL strictly**: never run the expensive code-reviewer until the cheap `cargo check` and `cargo test` loops have fully passed.

### Test vs. code authority conflict

When a test fails, the system doesn't inherently know if the code is wrong or the test was hallucinated. **The spec is the arbiter.** Compare the failing test against the `spec.md` ground truth. If the test matches the spec, fix the code. If the test deviates, fix the test.

### Markdown vs IR input source conflict

The validator architecture must be format-agnostic from the start (Iteration 1's architecture constraint). `specify validate` auto-detects the input source: if `.blueprint-plan.yaml` exists and is newer than rendered artifacts, validate the IR; otherwise validate the Markdown. This co-evolution is less risky than the phase-gated approach because both paths exist from early on — they grow together rather than the IR arriving as a disruptive mid-stream change.

### Feedback file proliferation

If every skill produces feedback sidecars, the `.specify/changes/` directory accumulates machine-generated files. These need lifecycle management (cleaned at merge, archived), schema validation, and clear ownership. Defined in Iteration 1 before it becomes organic.

### Per-handler granularity may not fit all crates

Some crates have deep handler interdependencies (shared state, cross-handler delegation, transaction boundaries). The Transaction Boundary Matrix in crate-writer exists precisely because handlers are not always independent. **Always provide a whole-crate fallback.**

### Infinite loop / hallucination spirals

The LLM fails to understand a compiler error, applies a wrong fix, breaks something else, loops indefinitely. **Strict iteration caps are non-negotiable.** After cap: stop, output the diagnostic state, escalate for human guidance. Never weaken this to "try harder."

### DSL authoring friction vs type safety payoff

The Rust DSL (Iteration 3) requires authors to define skill skeletons in Rust and embed prose via `include_str!`. For ~25 skills this is moderate ceremony. The YAML manifest intermediate step (Iterations 1–2) gets most of the structural validation without the authoring cost. **The Rust DSL is only justified once skill count and complexity make the compile-time feedback loop clearly cheaper than CI-time cross-checking.** The YAML manifests are not throwaway — they remain as the human-readable declaration that `checks.ts` validates against rendered output, even after the DSL generates from Rust.

### Scope creep on the Rust engine

In the phase-gated roadmap, the Rust engine is naturally constrained by arriving last. In the RWL approach, `specify-core` exists from Iteration 1 and could accumulate scope before contracts are stable. **Constraint**: `specify-core` only implements what the Deno tools already validate. It must produce identical results for identical inputs. The Deno implementation is the reference; Rust is the replacement, not the pioneer.

### Migration complexity from bundled changes

Each iteration ships cross-cutting schema changes rather than single-concern releases. `specify migrate` must handle bundled changes atomically. **Mitigation**: Each iteration is a single schema version bump. The migration command applies all changes for that version in one pass. Test migration against a real `.specify/` directory as part of each iteration's convergence gate.
