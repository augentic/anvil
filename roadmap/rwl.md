# Ralph Wiggum Loop (RWL) Iteration for High-Context Skills

Recommendations for introducing generate-evaluate-refine loops into
high-context skills (crate-writer, test-writer, guest-writer, spec:analyze,
code-reviewer) and the architectural changes needed to chain them.

## Current State: Where Iteration Already Lives

The architecture has two loop structures today, both embedded as prose in
`schemas/omnia/instructions/build.md`:

### 1. The verify-repair loop (max 3 iterations)

```
fmt → compile/clippy → test → classify failures → route to crate-writer OR test-writer → repeat
```

This runs *after* both crate-writer and test-writer have fully completed. The
feedback is "your output broke something" rather than "here's how to improve
your output."

### 2. The remediation loop (after code-reviewer)

```
code-reviewer findings → classify → route CRITICAL/HIGH to crate-writer or test-writer
→ verify-repair (max 2) → re-review
```

Both loops share the same weaknesses:

- Described in natural language; the loop controller is the LLM reading
  `build.md`
- Convergence criteria are "max N iterations or green" with no structured
  feedback
- Routing decisions (code issue vs test issue) require LLM judgment
- The whole crate is the unit of iteration, not individual handlers

## Where RWL Would Add Value

### Loop 1: Code-Analyzer Self-Critique

**Today**: `spec:analyze` is a single-pass skill. It reads source, produces
specs + design.md, and is done. There is no feedback mechanism to check whether
the artifacts are reconstruction-grade.

**RWL opportunity**: Generate → spot-check → refine.

```
Pass 1: Analyze source → produce specs + design.md
Pass 2: For each handler in design.md Business Logic:
         - Read the source function
         - Read the generated algorithm
         - Score: are all conditional branches captured?
         - Score: are all external calls documented?
         - Score: are config keys verbatim?
         → produce a delta of corrections
Pass 3: Apply corrections → re-validate
```

**Why this skill benefits most**: spec:analyze has the richest evaluation
signal available — the source code itself. You can mechanically check "did the
generated design.md reference every function in the source?" and "does the spec
have a requirement for every endpoint?" without LLM judgment.

**Convergence criterion**: Zero new findings in the spot-check pass, or max 2
refinement iterations.

### Loop 2: Crate-Writer + Test-Writer Co-Refinement

**Today**: Skills run sequentially (crate-writer finishes entirely, then
test-writer runs entirely), and the verify-repair loop patches up afterwards.
This means test-writer generates tests against code that may have structural
issues, and the repair loop has to untangle whether the bug is in code or tests.

**RWL opportunity**: Interleave generation with incremental verification.

```
Pass 1: crate-writer generates handler code
        → cargo check (structural smoke)
        → test-writer generates tests for that handler
        → cargo test (behavioral check)
        → if failures: classify and feed back to the appropriate skill
Pass 2: next handler (or refinement of previous)
...
Convergence: all handlers pass, or max 2 refinement passes per handler
```

**The per-handler granularity is the key change.** Today the loop operates on
the whole crate; this operates on individual handlers, catching issues before
they compound.

### Loop 3: Code-Reviewer Structured Feedback

**Today**: Code-reviewer runs after everything else, produces a REVIEW.md, and
remediation happens as a separate step. The reviewer's findings are prose that
the LLM interprets to decide what to fix.

**RWL opportunity**: Structured findings with typed feedback channels.

Instead of REVIEW.md as prose, code-reviewer produces structured output:

```yaml
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

This structured output feeds directly into a loop controller that routes
findings without LLM interpretation.

### Loop 4: Artifact Quality Loop (Define-side)

**Today**: `define` generates artifacts, runs LLM-interpreted `validate`
checks, and is done. If the artifacts have quality issues, they are discovered
during `build` when skills fail.

**RWL opportunity**: Generate → validate (deterministic) → refine → validate.

This connects directly to the deterministic validation work in
`roadmap/deterministic.md`. A standalone validator CLI (Tier 2 of that roadmap)
becomes the evaluation function in the RWL:

```
define generates artifacts → specify validate (deterministic)
→ if failures: refine → re-validate
```

## RWL Flow Diagrams

### Code-Analyzer Self-Critique

```
┌─────────────────────┐
│  Analyze source      │──→ specs + design.md
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Self-critique       │──→ For each handler: compare source vs generated
│  (spot-check pass)   │    algorithm. Score: branches, calls, config keys,
└─────────┬───────────┘    types.
          │ findings
          ▼
┌─────────────────────┐
│  Refine artifacts    │──→ Apply corrections to specs + design.md
└─────────┬───────────┘
          │
          ▼
       converged? ──no──→ back to self-critique (max 2)
          │yes
          ▼
        done
```

### Crate-Writer + Test-Writer Co-Refinement

```
┌─────────────────────┐
│  crate-writer        │──→ generated code
│  (per handler)       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  cargo check         │──→ compilation feedback
└─────────┬───────────┘
          │ pass
          ▼
┌─────────────────────┐
│  test-writer         │──→ tests for this handler
│  (per handler)       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  cargo test          │──→ test results
└─────────┬───────────┘
          │ failures?
          ▼
┌─────────────────────┐
│  classify + route    │──→ code issue → crate-writer (refine)
│                      │    test issue → test-writer (refine)
└─────────┬───────────┘
          │
          ▼
       next handler or re-iterate (max 2 per handler)
```

### Code-Reviewer Feedback Loop

```
┌─────────────────────┐
│  code-reviewer       │──→ structured findings (YAML)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  route by            │──→ skill_target: crate-writer → crate-writer refines
│  skill_target        │    skill_target: test-writer  → test-writer refines
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  re-review           │──→ verify fix quality (max 1 re-review)
└─────────┬───────────┘
          │
          ▼
        done
```

## The Architectural Change: A Pipeline Controller

The fundamental issue is that **loop control is currently embedded in
`build.md` as prose**. For RWL to work well, a layer above individual skills
must own:

1. **Pipeline definition** — which skills chain, in what order, with what
   feedback loops
2. **Feedback routing** — structured output from one skill becomes structured
   input to the next
3. **Convergence tracking** — iteration count, delta between passes, halt
   conditions
4. **State management** — what has been generated, what has been verified, what
   needs refinement

### Option A: Schema-Driven Pipeline (YAML layer)

Extend `schema.yaml` with a `pipeline` section that declares skill chains and
loops:

```yaml
pipelines:
  create:
    steps:
      - id: guest
        skill: omnia:guest-writer

      - id: generate-crate
        skill: omnia:crate-writer

      - id: generate-tests
        skill: omnia:test-writer
        after: [generate-crate]

      - id: verify
        type: loop
        max_iterations: 3
        steps:
          - run: cargo fmt --check
            fix: cargo fmt
          - run: cargo check && cargo clippy -- -D warnings
            on_fail: classify-and-route
          - run: cargo test
            on_fail: classify-and-route
        classify-and-route:
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
        input: review.findings
        route:
          - match: { skill_target: crate-writer }
            skill: omnia:crate-writer
          - match: { skill_target: test-writer }
            skill: omnia:test-writer
        after_each: verify
        convergence: no-critical-findings
```

This declarative pipeline replaces the prose in `build.md` with structured
data. A controller (initially the LLM reading this structured config, eventually
a Rust engine) executes it.

### Option B: Rust Pipeline Engine

The pipeline YAML from Option A becomes the input to a Rust engine:

```rust
struct Pipeline {
    steps: Vec<PipelineStep>,
}

enum PipelineStep {
    Skill {
        id: String,
        skill: SkillRef,
        after: Vec<String>,
    },
    Loop {
        id: String,
        max_iterations: u32,
        steps: Vec<VerifyStep>,
        routing: FailureRouter,
        convergence: ConvergenceCriterion,
    },
}

enum ConvergenceCriterion {
    AllGreen,
    NoCriticalFindings,
    DeltaBelowThreshold(f64),
    MaxIterations,
}
```

The engine calls the LLM for each skill invocation but owns loop control,
routing, and convergence decisions. This is the same "engine calls LLM as
capability" vision from `roadmap/deterministic.md` (Tier 3), extended to
multi-skill pipelines.

### Option C: Hybrid — Structured Feedback Files

The lightest-weight option: keep `build.md` as the instruction format but
introduce two conventions:

**1. Structured feedback files.** Skills produce a machine-readable sidecar
alongside their normal output. For example, crate-writer produces
`$CRATE_PATH/.specify-feedback.yaml` with compilation status, generated files,
and known gaps. Test-writer reads this as input.

```yaml
# .specify-feedback.yaml (produced by crate-writer)
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

**2. Loop blocks in `build.md`.** Formalize the verify-repair loop with a
structured syntax the build skill can parse:

```markdown
## Loop: verify-repair
- max_iterations: 3
- convergence: all-green
- steps:
  1. `cargo fmt --check` → fix: `cargo fmt`
  2. `cargo check && cargo clippy -- -D warnings` → route
  3. `cargo test` → route
- routing:
  - test_issue → /omnia:test-writer
  - code_issue → /omnia:crate-writer
```

This is more deterministic than today's prose but does not require a Rust
engine.

## Recommended Sequencing

| Phase | What | Connects to |
|---|---|---|
| **Now** | Structured feedback files between skills (`.specify-feedback.yaml`) | Enables all RWLs without architectural overhaul |
| **Next** | spec:analyze self-critique loop (intra-skill RWL) | Highest leverage — improves all downstream quality |
| **Then** | Per-handler interleaving of crate-writer + test-writer | Replaces the current post-hoc verify-repair loop |
| **Then** | Structured code-reviewer output with typed routing | Replaces the prose-based remediation loop |
| **Later** | Pipeline YAML in `schema.yaml` declaring loops declaratively | Makes pipelines configurable per-schema |
| **Eventually** | Rust pipeline engine executing the YAML | Full determinism — LLM is capability, engine is controller |

## Design Principle

**The LLM should generate content, not control iteration.** Every time the
system relies on the LLM to decide "should I iterate again?" or "which skill
should handle this failure?", it introduces non-determinism. The RWL controller
— whether structured YAML read by the build skill or a Rust engine — should own
the iteration decision. The LLM's job is to produce the best output it can on
each pass, given structured feedback from the previous pass.

The feedback format is the linchpin. If skills produce structured feedback
(compilation results, test results, coverage gaps, review findings), the routing
and convergence decisions become mechanical. If they produce prose, the system
is back to LLM interpretation.

## Relationship to Deterministic Roadmap

This roadmap complements `roadmap/deterministic.md`:

| Deterministic roadmap item | RWL connection |
|---|---|
| Machine-readable validation rules (Tier 1) | Becomes the evaluation function in the artifact quality loop (Loop 4) |
| Standalone validator CLI (Tier 2) | The gate that RWL loops call for convergence checks |
| Codified lifecycle (Tier 1) | Pipeline controller needs typed state transitions |
| Structured LLM output (Tier 4) | Skills producing `.specify-feedback.yaml` is a form of this |
| Rust workflow orchestrator (Tier 3) | The pipeline engine (Option B) is the same component |

Both roadmaps converge on the same architectural end state: a Rust engine that
owns workflow, state, and iteration, calling the LLM as a content-generation
capability.
