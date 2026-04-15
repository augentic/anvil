# Iterative Legacy Migration — The Migration Loop

> **Dependency:** This horizon builds on the [CLI (Horizon 1)](cli.md). The migration orchestrator, manifest parsing, and slice recommender are deterministic operations that belong in the CLI. The skill-level loop (extract → define → build → merge) already works today; what this horizon adds is the manifest-driven automation layer, implemented as `specify migrate` subcommands on top of the CLI foundation.

Migrate existing systems into Specify-managed codebases by running the same define-build-merge loop that powers greenfield development — repeatedly, feature by feature, until the legacy system is fully reconstituted. Rather than a big-bang rewrite, each feature of the legacy system moves through an iteration of the standard Specify workflow, with the extracted code as the proposal source instead of a blank canvas.

This is the "Ralph Wiggum Loop": extract a feature, define it, build it, merge it, pick the next one. The baseline grows with every merge. The legacy system shrinks with every iteration.

## The Migration Manifest

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

## The Loop

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

## Progressive Baseline Accumulation

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

## Fixture-Backed Verification

For slices where the `wiretapper` has captured runtime request/response fixtures from the legacy system, each iteration gains an additional verification step:

1. Before the build phase, the `replay-writer` generates tests from the captured fixtures
2. During the build phase, the implementation is verified against these replay tests
3. The tests assert that the new implementation produces the same outputs as the legacy system for the same inputs

This creates a behavioural regression safety net that catches semantic drift — the most common failure mode in legacy migrations. The loop doesn't just check that the new code satisfies the specs; it checks that it behaves identically to the old code.

## Why This Works

Legacy migration fails most often because the new system diverges from the old system's actual behaviour — the behaviour nobody wrote down. By extracting specs *from the running code* (not from stale documentation), the define-build-merge loop preserves behavioural fidelity while allowing the target architecture to differ completely.

The migration loop also solves the motivation problem. Big-bang rewrites take months before delivering any value. The migration loop delivers a working, spec-governed feature after every iteration. Progress is visible, reversible, and incremental. If a slice stalls, the baseline still reflects everything that has been successfully migrated — nothing is lost.

## What Exists Today

- The `code-analyzer` skill reads source code and produces Specify artifacts (specs + design.md), giving the extraction step
- The `wiretapper` skill instruments a legacy codebase to capture real request/response pairs as fixture JSON
- The `replay-writer` skill generates tests from those fixtures, providing a behavioural regression safety net
- The `extract` skill in `/spec` wraps extraction with iterative validation
- The `define`, `build`, and `merge` skills form the core loop

## What's Needed

The existing skill chain covers the core loop. New capabilities fall into two categories: those that extend the [CLI (Horizon 1)](cli.md) with `specify migrate` subcommands, and those that are agent-level skill work.

| Capability                            | Status      | Notes                                                                                      |
| ------------------------------------- | ----------- | ------------------------------------------------------------------------------------------ |
| Extract specs from source code        | Exists      | `code-analyzer`, `/spec:extract`                                                           |
| Capture runtime fixtures              | Exists      | `wiretapper`                                                                               |
| Generate replay tests                 | Exists      | `replay-writer`                                                                            |
| Define → Build → Merge chain          | Exists      | `/spec:define`, `/spec:build`, `/spec:merge`                                               |
| Migration manifest (`migration.yaml`) | New (CLI)   | Ordered feature list with source paths, dependencies, and per-slice status                 |
| `specify migrate init`                | New (CLI)   | Scaffold `migration.yaml` from a legacy codebase scan                                      |
| `specify migrate next`                | New (CLI)   | Return the next pending slice from the manifest (respecting `depends_on`)                  |
| `specify migrate status`              | New (CLI)   | Show migration progress: N/M slices migrated, current slice, blockers                      |
| Migration orchestrator                | New (skill) | Reads the manifest, selects the next pending slice, wires extract → define → build → merge |
| Slice recommender                     | New (skill) | Analyse legacy dependency graph and suggest migration ordering                             |
| Behavioural diff                      | New (CLI)   | Compare legacy fixture output against new implementation output                            |
| Cross-stack define                    | New (skill) | Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust)      |


## Slice Strategy

Not every slice of a legacy system is equally suitable for early migration. Good early slices are:

- **Leaf services** with few upstream dependents — migrating them doesn't break anything
- **Clear API boundaries** — the input/output contract is well-defined and testable
- **Existing test coverage** or easy-to-capture request/response patterns (good `wiretapper` candidates)
- **Low cross-boundary coupling** — the feature doesn't reach deep into shared mutable state

The `depends_on` field in the manifest encodes inter-slice dependencies. The migration orchestrator (or `specify migrate next`) respects these: a slice won't be selected until all its dependencies have status `migrated`. This prevents the loop from attempting to build a feature that references specs that haven't been merged into the baseline yet.

For teams without a clear migration order, the slice recommender analyses the legacy codebase's dependency graph and suggests an ordering that minimises cross-boundary risk — leaf-first, core-last.

## The Key Benefit

Migration becomes incremental and reversible. Each loop iteration produces a self-contained change with specs, tests, and verified code. The same skills, validation, and merge semantics that govern greenfield development govern migration — no separate tooling, no separate process. The migration manifest provides a plan; the loop provides the discipline; the baseline provides the proof.
