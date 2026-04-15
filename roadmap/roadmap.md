# Specify Roadmap — Findings and Recommendations

*Exported on 14/04/2026 from Cursor*

---

## Context

Most organisations building AI-assisted development frameworks use deterministic programs to orchestrate the workflow (e.g. OpenSpec, SpecKit's spec-driven development, Geoff Huntley's Loop). Specify inverts control: an agent orchestrates skills to achieve the same outcome — automated code generation — with deterministic tools used only where precision is required.

This document captures a critique of the current framework and a roadmap structured as three horizons.

---

## What's Working Well

The inversion-of-control thesis is sound and architecturally distinct from the deterministic-program crowd. The key insight — that an LLM agent is better at orchestrating flexible, judgment-heavy workflows while deterministic tools should own precision-critical operations — maps well to the actual failure modes you'd see in practice.

### The skill-as-contract model is strong

Skills like `define`, `build`, and `merge` are behavioural contracts that the agent interprets, not rigid scripts. This gives resilience to ambiguity that a deterministic pipeline can't handle (e.g. the `build` skill's "pause if task is unclear" or "suggest artifact updates if design issues emerge").

### The delta-merge spec format is clever

Using stable `REQ-XXX` IDs as merge keys with ADDED/MODIFIED/REMOVED/RENAMED operations gives a specification system that can evolve without losing traceability.

---

## Where the Tension Shows

### The "deterministic islands" are ad-hoc and fragile

There is exactly one deterministic tool today: `merge-specs.py`. Everything else that needs precision — validation, task parsing, artifact structure checking — is done by the LLM interpreting prose rules. This creates a reliability gradient: the merge step is solid (deterministic Python with exit codes), but validation is probabilistic (the agent reading `validate` strings from `schema.yaml` and making judgment calls).

The build instructions in `schemas/omnia/instructions/build.md` illustrate this well — they contain shell commands like `cargo test 2>&1 | tee /tmp/...` embedded in prose. The agent must parse the prose, decide when to run the command, interpret the output, and classify failures. That's a lot of cognitive load on the LLM for what is fundamentally a structured decision tree.


The `validate` arrays in `schema.yaml` are human-readable strings like "Has a Why section with at least one sentence" and "Uses SHALL/MUST language for normative requirements." These are great for communicating intent, but the agent's interpretation of "at least one sentence" or "WHEN/THEN format" will vary across invocations. The `checks.ts` script validates the *framework itself* rigorously, but the *artifacts it produces* get weaker validation.

### Multi-repo is structurally hard

The `.specify/` directory is project-local. The `touched_specs` conflict detection in `.metadata.yaml` only works within a single workspace. There's no concept of a "spec reference" that spans repositories, and the schema resolution assumes a single project root.

---

## Roadmap — Three Horizons

### Horizon 1: `specify` CLI (Rust) — Deterministic Foundation

Extract every precision-critical operation from prose skills into a single Rust CLI binary (`specify`) that lives in this repo and ships alongside the plugins. Skills invoke it via shell commands. The agent handles judgment; the CLI handles correctness.

See [cli.md](cli.md) for the full crate structure sketch.

**Subcommands:**


| Command                              | Replaces                                        |
| ------------------------------------ | ----------------------------------------------- |
| `specify init <schema>`              | Agent's mkdir/copy/write logic in init skill    |
| `specify validate <change-dir>`      | 40 lines of prose validation in build skill     |
| `specify merge <change-dir>`         | `merge-specs.py`                                |
| `specify status [change-name]`       | Agent parsing .metadata.yaml + task checkboxes  |
| `specify schema resolve <value>`     | Agent interpreting schema-resolution.md         |
| `specify schema check <schema-dir>`  | Parts of checks.ts                              |
| `specify task next <change-dir>`     | Agent parsing tasks.md for next incomplete task |
| `specify task mark <change-dir> <n>` | Agent editing checkbox in tasks.md              |
| `specify lint <change-dir>`          | Cross-artifact consistency checks               |


**The key benefit:** `specify validate` replaces the entire "Per-blueprint validation" and "Cross-blueprint consistency checks" sections in `build/SKILL.md` (currently ~40 lines of prose the agent must interpret) with a single shell command that returns structured JSON or a pass/fail exit code. The skill prose shrinks to "run the validation and act on the result."

**What the agent keeps:** The agent still decides *when* to validate, *how to respond* to failures, *whether to pause* for user input, and *what to suggest* for fixes. The CLI is a precision instrument; the agent is the practitioner.

### Horizon 2: Multi-Repo Coordination

Extend the existing `config.yaml` to declare peer repositories and use the CLI to coordinate.

See [horizons.md](horizons.md) for the full design.

### Horizon 3: Iterative Legacy Migration — The Migration Loop

Migrate existing systems into Specify-managed codebases by running the same define-build-merge loop that powers greenfield development — repeatedly, feature by feature, until the legacy system is fully reconstituted. Rather than a big-bang rewrite, each feature of the legacy system moves through an iteration of the standard Specify workflow, with the extracted code as the proposal source instead of a blank canvas.

This is the "Ralph Wiggum Loop": extract a feature, define it, build it, merge it, pick the next one. The baseline grows with every merge. The legacy system shrinks with every iteration.

#### The Migration Manifest

A migration can be driven by an optional **migration manifest** — a predetermined, ordered list of the features (slices) to migrate. The manifest is the migration's table of contents: it tells the loop what to do next without requiring the agent to rediscover the legacy system's structure on every iteration.

```yaml
# .specify/migration.yaml
source: /path/to/legacy-codebase   # or git@github.com:org/legacy.git
target_schema: omnia@v1

slices:
  - name: user-registration
    source_paths:
      - src/auth/register.ts
      - src/auth/models/user.ts
    status: migrated                # migrated | in-progress | pending | skipped

  - name: email-verification
    source_paths:
      - src/auth/verify-email.ts
      - src/auth/tokens.ts
    depends_on: [user-registration]
    status: in-progress

  - name: password-reset
    source_paths:
      - src/auth/reset.ts
      - src/auth/models/reset-token.ts
    depends_on: [user-registration]
    status: pending

  - name: product-catalog
    source_paths:
      - src/catalog/products.ts
      - src/catalog/categories.ts
      - src/catalog/models/
    status: pending

  - name: shopping-cart
    source_paths:
      - src/cart/
    depends_on: [product-catalog, user-registration]
    status: pending

  - name: checkout-flow
    source_paths:
      - src/checkout/
      - src/payments/
    depends_on: [shopping-cart]
    status: pending
```

The manifest can be:

- **Human-authored** — a tech lead lists the features they want migrated and the order they want them in, encoding institutional knowledge about risk, priority, and dependencies
- **Auto-generated** — the slice recommender (see below) analyses the legacy codebase's dependency graph and proposes an ordering that minimises cross-boundary risk
- **Hybrid** — the recommender proposes, the human reorders and prunes

When no manifest exists, the loop runs in **ad-hoc mode**: the user picks the next feature interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

#### The Loop

Each iteration of the migration loop is a single Specify change that moves one slice from the legacy system into the target codebase under spec governance. The loop reuses the existing skill chain with one additional step at the front:

```text
for each slice in migration.yaml (or user's choice):

  ┌─────────────────────────────────────────────────────────┐
  │                                                         │
  │  1. EXTRACT  /spec:extract                              │
  │     Analyse the slice's source files and produce        │
  │     Specify artifacts (specs + design.md) capturing     │
  │     the legacy behaviour. The code-analyzer reads the   │
  │     source; the extract skill validates iteratively     │
  │     until convergence.                                  │
  │                                                         │
  │  2. DEFINE   /spec:define                               │
  │     Refine the extracted artifacts for the target       │
  │     stack. Adapt types to target idioms, generate       │
  │     tasks, wire up skill directives. The proposal       │
  │     source is the extraction output, not prose.         │
  │                                                         │
  │  3. BUILD    /spec:build                                │
  │     Implement every task against the target stack       │
  │     using the specs as source of truth. Run tests,      │
  │     verify against replay fixtures if available.        │
  │                                                         │
  │  4. MERGE    /spec:merge                                │
  │     Merge the change. Delta specs fold into baseline.   │
  │     The migrated feature is now under spec governance.  │
  │     Update migration.yaml: status → migrated.           │
  │                                                         │
  │  5. NEXT     Loop to the next pending slice             │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

Each iteration is a self-contained Specify change. The agent runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain it would run for any greenfield feature — the only difference is that the input to `/spec:define` comes from `/spec:extract` (existing code) rather than a human description.

#### Progressive Baseline Accumulation

The key mechanism is **baseline growth through merge**. After each iteration:

- The migrated feature's specs join `.specify/specs/` as baseline
- Subsequent iterations can reference these specs (e.g., the cart feature can reference the product-catalog specs that were merged in a prior iteration)
- The `touched_specs` conflict detection in `.metadata.yaml` prevents two in-flight migrations from stomping on each other
- The archived changes in `.specify/changes/archive/` provide a complete audit trail of the migration

```text
Iteration 1:  baseline = {}
              extract(user-registration) → define → build → merge
              baseline = { user-registration }

Iteration 2:  baseline = { user-registration }
              extract(email-verification) → define → build → merge
              baseline = { user-registration, email-verification }

Iteration 3:  baseline = { user-registration, email-verification }
              extract(product-catalog) → define → build → merge
              baseline = { user-registration, email-verification, product-catalog }

...

Iteration N:  baseline = { all features }
              Migration complete. Legacy system fully reconstituted under spec governance.
```

#### Fixture-Backed Verification

For slices where the `wiretapper` has captured runtime request/response fixtures from the legacy system, each iteration gains an additional verification step:

1. Before the build phase, the `replay-writer` generates tests from the captured fixtures
2. During the build phase, the implementation is verified against these replay tests
3. The tests assert that the new implementation produces the same outputs as the legacy system for the same inputs

This creates a behavioural regression safety net that catches semantic drift — the most common failure mode in legacy migrations. The loop doesn't just check that the new code satisfies the specs; it checks that it behaves identically to the old code.

#### Why This Works

Legacy migration fails most often because the new system diverges from the old system's actual behaviour — the behaviour nobody wrote down. By extracting specs *from the running code* (not from stale documentation), the define-build-merge loop preserves behavioural fidelity while allowing the target architecture to differ completely.

The migration loop also solves the motivation problem. Big-bang rewrites take months before delivering any value. The migration loop delivers a working, spec-governed feature after every iteration. Progress is visible, reversible, and incremental. If a slice stalls, the baseline still reflects everything that has been successfully migrated — nothing is lost.

#### What Exists Today

- The `code-analyzer` skill reads source code and produces Specify artifacts (specs + design.md), giving the extraction step
- The `wiretapper` skill instruments a legacy codebase to capture real request/response pairs as fixture JSON
- The `replay-writer` skill generates tests from those fixtures, providing a behavioural regression safety net
- The `extract` skill in `/spec` wraps extraction with iterative validation
- The `define`, `build`, and `merge` skills form the core loop

#### What's Needed


| Capability                      | Status | Notes                                                                                 |
| ------------------------------- | ------ | ------------------------------------------------------------------------------------- |
| Extract specs from source code  | Exists | `code-analyzer`, `/spec:extract`                                                      |
| Capture runtime fixtures        | Exists | `wiretapper`                                                                          |
| Generate replay tests           | Exists | `replay-writer`                                                                       |
| Define → Build → Merge chain    | Exists | `/spec:define`, `/spec:build`, `/spec:merge`                                          |
| Migration manifest (`migration.yaml`) | New    | Ordered feature list with source paths, dependencies, and per-slice status      |
| Migration orchestrator          | New    | Reads the manifest, selects the next pending slice, wires extract → define → build → merge |
| Slice recommender               | New    | Analyse legacy dependency graph and suggest migration ordering                        |
| Behavioural diff                | New    | Compare legacy fixture output against new implementation output                       |
| Migration dashboard             | New    | Track which slices are migrated, in-progress, and remaining across iterations         |
| Cross-stack define              | New    | Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust) |
| `specify migrate init`          | New    | Scaffold `migration.yaml` from a legacy codebase scan                                 |
| `specify migrate status`        | New    | Show migration progress: N/M slices migrated, current slice, blockers                 |
| `specify migrate next`          | New    | Return the next pending slice from the manifest (respecting `depends_on`)             |


#### Slice Strategy

Not every slice of a legacy system is equally suitable for early migration. Good early slices are:

- **Leaf services** with few upstream dependents — migrating them doesn't break anything
- **Clear API boundaries** — the input/output contract is well-defined and testable
- **Existing test coverage** or easy-to-capture request/response patterns (good `wiretapper` candidates)
- **Low cross-boundary coupling** — the feature doesn't reach deep into shared mutable state

The `depends_on` field in the manifest encodes inter-slice dependencies. The migration orchestrator (or `specify migrate next`) respects these: a slice won't be selected until all its dependencies have status `migrated`. This prevents the loop from attempting to build a feature that references specs that haven't been merged into the baseline yet.

For teams without a clear migration order, the slice recommender analyses the legacy codebase's dependency graph and suggests an ordering that minimises cross-boundary risk — leaf-first, core-last.

#### The Key Benefit

Migration becomes incremental and reversible. Each loop iteration produces a self-contained change with specs, tests, and verified code. The same skills, validation, and merge semantics that govern greenfield development govern migration — no separate tooling, no separate process. The migration manifest provides a plan; the loop provides the discipline; the baseline provides the proof.

---

## Shell Commands in Skills: Design Principles


| Use CLI (`specify ...`) when:                 | Use agent judgment when:                    |
| --------------------------------------------- | ------------------------------------------- |
| The operation must be idempotent              | The response depends on context             |
| The output is structured (JSON, exit codes)   | The output is natural language              |
| Correctness is verifiable (schema validation) | Correctness requires semantic understanding |
| The operation is repeated across many skills  | The operation is unique to one skill        |
| Failure modes are enumerable                  | Failure modes are open-ended                |


The `specify` CLI gives a clean abstraction boundary. Instead of skills containing scattered shell commands, they can use `specify` subcommands that return structured output. The principle: **the CLI owns Specify operations; external tool invocation stays with the agent.**

A good litmus test: "Would this command need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays as a direct shell command in the skill.

---

## Suggested Priority Order

1. **Specify CLI scaffolding** — `specify init`, `specify validate`, `specify merge` (replaces `merge-specs.py`)
2. **Migrate `init` and `merge` skills** to use CLI commands
3. **Migrate `build` validation** to use `specify validate`
4. **`specify task`** subcommands for deterministic task tracking
5. **Federation config** and `specify federation sync` for multi-repo
6. **Cross-repo spec references** and `specify federation validate`
7. **Migration manifest** — `specify migrate init` to scaffold `migration.yaml` from a legacy codebase scan
8. **Migration orchestrator** — `specify migrate next` to select the next pending slice and wire the extract → define → build → merge chain
9. **Slice recommender** — analyse legacy dependency graph and suggest migration ordering for the manifest
10. **Behavioural diff** — compare legacy fixtures against new implementation output
11. **Migration dashboard** — `specify migrate status` to track slice-level migration progress across iterations

The first three items would take a single `/spec:define` + `/spec:build` cycle to implement and would immediately simplify the three most complex skills. Items 7–11 build on the existing `code-analyzer`, `wiretapper`, `replay-writer`, and core `/spec:*` skills — the migration loop reuses the entire greenfield workflow, so most of the infrastructure is already in place.

---

## Impact on Existing Skills


| Skill    | Current agent-interpreted logic                           | Moves to CLI                                 |
| -------- | --------------------------------------------------------- | -------------------------------------------- |
| `init`   | mkdir, file creation, schema resolution, cache population | `specify init`                               |
| `define` | Schema resolution, metadata writes, overlap detection     | `specify schema resolve`, `specify status`   |
| `build`  | Artifact validation, task progress tracking               | `specify validate`, `specify task next/mark` |
| `merge`  | merge-specs.py invocation, coherence check, archive move  | `specify merge`                              |
| `verify` | Spec parsing, requirement extraction                      | `specify diff`                               |
| `status` | Metadata + task parsing                                   | `specify status`                             |


---

## The `checks.ts` to `specify check` Migration

The `roadmap/dsl.md` conversation explored extending `checks.ts` vs building a Rust DSL. With the `specify` CLI as Horizon 1, this becomes natural:

- **Phase 1 checks (already done):** `checks.ts` validates the framework repo (symlinks, schema integrity, skill frontmatter, etc.)
- **Phase 2:** `specify check` validates consumer project artifacts at runtime. `checks.ts` remains for framework CI; `specify check` is for runtime project validation.
- **Phase 3 (optional):** If the skill count grows significantly, the Rust DSL from `roadmap/dsl.md` can generate SKILL.md files from typed definitions, with the `specify` crate providing the type system. But this is premature for 18 skills.

