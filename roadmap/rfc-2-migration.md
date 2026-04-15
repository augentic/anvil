# RFC-2: Iterative Legacy Migration

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Migrate existing systems into Specify-managed codebases by running the same define-build-merge loop that powers greenfield development — repeatedly, feature by feature, driven by a migration manifest. Each slice of the legacy system is extracted, defined, built, and merged in a self-contained iteration. Progress is incremental and reversible.

## Motivation

Legacy migration typically fails because the new system diverges from the old system's actual behaviour — the behaviour nobody wrote down. Big-bang rewrites take months before delivering value, and when they stall, everything is lost.

By extracting specs *from the running code* (not from stale documentation), the define-build-merge loop preserves behavioural fidelity while allowing the target architecture to differ completely. Each iteration delivers a working, spec-governed feature. Progress is visible, reversible, and incremental. If a slice stalls, the baseline still reflects everything that has been successfully migrated — nothing is lost.

## Dependency on RFC-1

The migration orchestrator, manifest parsing, and slice recommender are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (extract → define → build → merge) already works today; what this RFC adds is the manifest-driven automation layer, implemented as `specify migrate` subcommands on top of the CLI foundation.

## Detailed Design

### The Migration Manifest

A migration can be driven by an optional **migration manifest** — a predetermined, ordered list of the features (slices) to migrate. The manifest is the migration's table of contents: it tells the loop what to do next without requiring the agent to rediscover the legacy system's structure on every iteration.

```yaml
# .specify/migration.yaml
source: /path/to/legacy-codebase   
# or source: https://github.com/org/legacy
# or source: git@github.com:org/legacy.git
target_schema: omnia@v1

features:
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

### The Loop

Each iteration of the migration loop is a single Specify change that moves one slice from the legacy system into the target codebase under spec governance. The loop reuses the existing skill chain with one additional step at the front:

```text
for each slice in migration.yaml (or user's choice):

  ┌─────────────────────────────────────────────────────────┐
  │                                                         │
  │  1. EXTRACT  /spec:extract                              │
  │     Analyse the slice's source files and produce        │
  │     Specify artifacts (specs + design.md) capturing     │
  │     the legacy behaviour in a single pass.              │
  │                                                         │
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

### Progressive Baseline Accumulation

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

### Fixture-Backed Verification

For slices where the `wiretapper` has captured runtime request/response fixtures from the legacy system, each iteration gains an additional verification step:

1. Before the build phase, the `replay-writer` generates tests from the captured fixtures
2. During the build phase, the implementation is verified against these replay tests
3. The tests assert that the new implementation produces the same outputs as the legacy system for the same inputs

This creates a behavioural regression safety net that catches semantic drift — the most common failure mode in legacy migrations. The loop doesn't just check that the new code satisfies the specs; it checks that it behaves identically to the old code.

### Slice Strategy

Not every slice of a legacy system is equally suitable for early migration. Good early slices are:

- **Leaf services** with few upstream dependents — migrating them doesn't break anything
- **Clear API boundaries** — the input/output contract is well-defined and testable
- **Existing test coverage** or easy-to-capture request/response patterns (good `wiretapper` candidates)
- **Low cross-boundary coupling** — the feature doesn't reach deep into shared mutable state

The `depends_on` field in the manifest encodes inter-slice dependencies. The migration orchestrator (or `specify migrate next`) respects these: a slice won't be selected until all its dependencies have status `migrated`. This prevents the loop from attempting to build a feature that references specs that haven't been merged into the baseline yet.

For teams without a clear migration order, the slice recommender analyses the legacy codebase's dependency graph and suggests an ordering that minimises cross-boundary risk — leaf-first, core-last.

## Existing Infrastructure

| Capability                           | Status | Notes                                    |
| ------------------------------------ | ------ | ---------------------------------------- |
| Extract specs from source code       | Exists | `/spec:extract`                          |
| Capture runtime fixtures             | Exists | `wiretapper`                             |
| Generate replay tests                | Exists | `replay-writer`                          |
| Define → Build → Merge chain        | Exists | `/spec:define`, `/spec:build`, `/spec:merge` |

## New Capabilities Required

| Capability                            | Type  | Notes                                                                                      |
| ------------------------------------- | ----- | ------------------------------------------------------------------------------------------ |
| Migration manifest (`migration.yaml`) | CLI   | Ordered feature list with source paths, dependencies, and per-slice status                 |
| `specify migrate init`                | CLI   | Scaffold `migration.yaml` from a legacy codebase scan                                      |
| `specify migrate next`                | CLI   | Return the next pending slice from the manifest (respecting `depends_on`)                  |
| `specify migrate status`              | CLI   | Show migration progress: N/M slices migrated, current slice, blockers                      |
| Migration orchestrator                | Skill | Reads the manifest, selects the next pending slice, wires extract → define → build → merge |
| Slice recommender                     | Skill | Analyse legacy dependency graph and suggest migration ordering                             |
| Behavioural diff                      | CLI   | Compare legacy fixture output against new implementation output                            |
| Cross-stack define                    | Skill | Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust)      |

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; migration subcommands extend the CLI
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — complements migration for cross-repo features
