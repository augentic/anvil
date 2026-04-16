# RFC-2: Feature Manifests

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Drive complex, multi-feature initiatives through Specify's define-build-merge loop using a **feature manifest** — an ordered, dependency-aware plan of features with status tracking and progressive baseline accumulation. Legacy migration, greenfield multi-feature builds, and platform modernisations all use the same manifest format and the same loop; the only difference is where the input to `/spec:define` comes from.

## Motivation

Complex initiatives — multi-feature greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

The define-build-merge loop already works for individual features. What's missing is the layer above: a manifest that sequences features, tracks dependencies between them, and lets progress accumulate in the baseline across iterations. Without it, every iteration starts from scratch — the agent doesn't know what came before, what's in flight, or what's blocked.

By expressing the initiative as an ordered list of features with dependency constraints, the manifest turns a sprawling multi-feature effort into a series of self-contained Specify changes, each building on the baseline left by the last.

## Dependency on RFC-1

The manifest orchestrator, manifest parsing, and feature recommender are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (define → build → merge, optionally preceded by extract) already works today; what this RFC adds is the manifest-driven automation layer, implemented as `specify manifest` subcommands on top of the CLI foundation.

## Detailed Design

### The Feature Manifest

A feature manifest is an ordered list of the features to implement, along with their dependencies and status. It is the initiative's table of contents: it tells the loop what to do next without requiring the agent to rediscover scope on every iteration.

```yaml
# .specify/features.yaml
name: platform-v2
target-schema: omnia@v1

# Optional — only for migration/extraction use cases.
# Named source repositories. Features reference these by key in their
# `source` field. source-paths are resolved relative to the named source.
sources:
  monolith: /path/to/legacy-codebase
  orders: git@github.com:org/orders-service.git
  payments: git@github.com:org/payments-service.git

features:
  - name: user-registration
    source: monolith
    source-paths:                     # present → extract from legacy
      - src/auth/register.ts
      - src/auth/models/user.ts
    status: migrated                  # migrated | in-progress | pending | skipped

  - name: email-verification
    source: monolith
    source-paths:
      - src/auth/verify-email.ts
      - src/auth/tokens.ts
    depends-on: [user-registration]
    status: in-progress

  - name: notification-preferences
    depends-on: [user-registration]   # no source → greenfield feature
    status: pending

  - name: product-catalog
    source: monolith
    source-paths:
      - src/catalog/products.ts
      - src/catalog/categories.ts
      - src/catalog/models/
    status: pending

  - name: shopping-cart
    source: orders
    source-paths:
      - src/cart/
    depends-on: [product-catalog, user-registration]
    status: pending

  - name: checkout-flow
    source: payments
    source-paths:
      - src/checkout/
      - src/payments/stripe.ts
    depends-on: [shopping-cart]
    status: pending
```

Extracted features (those with `source-paths`) and greenfield features (those without) coexist in the same manifest. The `sources` map supports migration from multiple codebases — a common scenario in microservices architectures where the legacy system is already split across repos. Each feature declares which source it extracts from; features without a `source` are greenfield. A platform modernisation might extract core services from several legacy codebases while adding new capabilities that never existed before — the manifest handles both in a single, ordered plan.

The manifest can be:

- **Human-authored** — a tech lead lists the features they want and the order they want them in, encoding institutional knowledge about risk, priority, and dependencies
- **Auto-generated** — `specify manifest init` analyses the source codebases' directory structure and import graph to propose feature boundaries and a leaf-first ordering (see [Manifest Generation](#manifest-generation-specify-manifest-init) below)
- **Hybrid** — the recommender proposes, the human reorders and prunes

When no manifest exists, the loop runs in **ad-hoc mode**: the user picks the next feature interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

### The Loop

Each iteration of the loop is a single Specify change that implements one feature from the manifest. The loop reuses the existing skill chain, with an optional extraction step at the front for features that have `source-paths`:

```text
for each feature in manifest.yaml (or user's choice):

  ┌─────────────────────────────────────────────────────────┐
  │                                                         │
  │  1. EXTRACT  /spec:extract        [if source-paths]     │
  │     Analyse the feature's source files and produce      │
  │     Specify artifacts (specs + design.md) capturing     │
  │     the legacy behaviour in a single pass.              │
  │                                                         │
  │  2. DEFINE   /spec:define                               │
  │     Create or refine the artifacts for the target       │
  │     stack. For extracted features, adapt types to        │
  │     target idioms and generate tasks. For greenfield     │
  │     features, author from the proposal description.     │
  │                                                         │
  │  3. BUILD    /spec:build                                │
  │     Implement every task against the target stack       │
  │     using the specs as source of truth. Run tests,      │
  │     verify against replay fixtures if available.        │
  │                                                         │
  │  4. MERGE    /spec:merge                                │
  │     Merge the change. Delta specs fold into baseline.   │
  │     The feature is now under spec governance.           │
  │     Update manifest.yaml: status → migrated.            │
  │                                                         │
  │  5. NEXT     Loop to the next pending feature           │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

Each iteration is a self-contained Specify change. The agent runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain it would run for any single feature — the only difference is that the manifest decides what to build next and tracks what's already done.

### Progressive Baseline Accumulation

The key mechanism is **baseline growth through merge**. After each iteration:

- The completed feature's specs join `.specify/specs/` as baseline
- Subsequent iterations can reference these specs (e.g., the cart feature can reference the product-catalog specs that were merged in a prior iteration)
- The `touched-specs` conflict detection in `.metadata.yaml` prevents two in-flight features from stomping on each other
- The archived changes in `.specify/changes/archive/` provide a complete audit trail

```text
Iteration 1:  baseline = {}
              define(user-registration) → build → merge
              baseline = { user-registration }

Iteration 2:  baseline = { user-registration }
              define(notification-preferences) → build → merge
              baseline = { user-registration, notification-preferences }

Iteration 3:  baseline = { user-registration, notification-preferences }
              extract(product-catalog) → define → build → merge
              baseline = { user-registration, notification-preferences, product-catalog }

...

Iteration N:  baseline = { all features }
              Initiative complete. Every feature under spec governance.
```

This works identically whether the features are extracted from legacy code, authored from scratch, or a mix of both. The baseline doesn't care where the specs originated — it only cares that they passed through the define-build-merge loop.

### Migration Mode

When the manifest includes a `sources` section and features have `source-paths`, the loop operates in **migration mode** — the same loop with additional extraction and verification capabilities.

#### Extraction

For each feature with `source-paths`, the EXTRACT step analyses the legacy source files and produces Specify artifacts capturing the existing behaviour. The input to `/spec:define` comes from `/spec:extract` rather than a human description.

#### Fixture-Backed Verification

For features where the `wiretapper` has captured runtime request/response fixtures from the legacy system, each iteration gains an additional verification step:

1. Before the build phase, the `replay-writer` generates tests from the captured fixtures
2. During the build phase, the implementation is verified against these replay tests
3. The tests assert that the new implementation produces the same outputs as the legacy system for the same inputs

This creates a behavioural regression safety net that catches semantic drift — the most common failure mode in legacy migrations.

#### Slice Strategy

Not every feature of a legacy system is equally suitable for early migration. Good early candidates are:

- **Leaf services** with few upstream dependents — migrating them doesn't break anything
- **Clear API boundaries** — the input/output contract is well-defined and testable
- **Existing test coverage** or easy-to-capture request/response patterns (good `wiretapper` candidates)
- **Low cross-boundary coupling** — the feature doesn't reach deep into shared mutable state

The `depends-on` field in the manifest encodes inter-feature dependencies. The orchestrator (or `specify manifest next`) respects these: a feature won't be selected until all its dependencies have status `migrated`. This prevents the loop from attempting to build a feature that references specs that haven't been merged into the baseline yet.

For teams without a clear ordering, `specify manifest init` analyses the legacy codebase's directory structure and import graph to suggest feature boundaries and a leaf-first ordering (see [Manifest Generation](#manifest-generation-specify-manifest-init)).

### Multi-Repo Initiatives

The `sources` map handles the multi-repo *source* side — extracting features from several legacy codebases into a single initiative. When an initiative also spans multiple *target* repositories, features in the manifest can declare which repo they target. Cross-repo spec references are resolved through the federation model defined in [RFC-3](rfc-3-multi-repo.md). The manifest provides the coordination layer (what features, in what order, with what dependencies); federation provides the resolution layer (where specs live, how to validate cross-repo contracts).

### Manifest Generation (`specify manifest init`)

`specify manifest init` bootstraps a draft `features.yaml` from one or more source codebases. The analysis is deliberately shallow — it identifies feature boundaries, dependency edges, and a safe ordering. It does not analyse behaviour, generate specs, or make target design decisions; that work belongs to `/spec:extract` and `/spec:define` when each feature reaches the head of the loop.

The generation is split into two layers: deterministic structural discovery (CLI) and optional LLM-assisted refinement (feature recommender skill).

#### Layer 1: Structural Discovery (CLI)

Pure filesystem and import-graph analysis — no LLM involved.

**Step 1 — Discover module boundaries.** Walk the source codebase(s) and identify natural groupings:

- **Directory structure** — the strongest signal. Most codebases organise by domain at some level (`src/auth/`, `src/cart/`, `src/catalog/`). Directories that contain models, handlers, routes, or services are candidate feature roots.
- **Build system boundaries** — workspace members in `Cargo.toml`, `package.json` workspaces, Go module paths, Python packages with `__init__.py`. These are explicit module declarations by the original authors.
- **Entry points** — route registrations, handler files, exported modules. These hint at what the codebase considers its public surface.

**Step 2 — Build a coarse import graph.** Parse import/require/use statements across files — just the paths, not the symbols. This doesn't require a full AST; regex patterns per detected language handle the common cases:

- TypeScript/JS: `import ... from '...'` / `require('...')`
- Rust: `use crate::...` / `mod ...`
- Go: `import "..."`
- Python: `from ... import ...` / `import ...`

Aggregate file-level imports to the cluster level. If any file in cluster A imports any file in cluster B, that's a dependency edge A → B.

**Step 3 — Classify clusters.** Not every directory is a feature. Utility clusters — directories imported by most other clusters but importing none of them (e.g., `shared/`, `utils/`, `lib/`) — are classified as infrastructure rather than features. Infrastructure clusters are either absorbed into the features that reference them or flagged as a prerequisite to migrate first.

**Step 4 — Topological sort.** With clusters and dependency edges, topological sort produces a leaf-first ordering — the safest migration sequence since leaf features have no downstream dependents. Cycles are detected and flagged for human review; they usually indicate the feature boundaries need splitting.

**Step 5 — Emit the manifest.** Write `features.yaml` with the discovered features, their `source-paths`, `depends-on` edges, and all statuses set to `pending`.

For multi-source initiatives the CLI runs structural discovery against each entry in the `sources` map independently, then merges the results into a single feature list. Cross-source dependencies are unlikely (the sources are separate repos) but are flagged if detected.

#### Layer 2: Feature Recommender (Skill, Optional)

The CLI output from Layer 1 uses directory-derived names (`auth`, `cart`, `catalog`). The feature recommender skill can optionally refine the draft:

- Propose human-readable feature names from file contents (top-level exports, function signatures, route paths — not deep analysis)
- Suggest splitting oversized clusters or merging trivially small ones
- Flag ambiguous boundaries for human review

Layer 2 is explicitly optional. The CLI produces a valid manifest from Layer 1 alone; the skill makes it more readable.

#### Scope Boundary

The manifest is a table of contents, not a design document. `specify manifest init` deliberately does not:

- Analyse function bodies or business logic — that's `/spec:extract`
- Generate specs or design documents — that's `/spec:define`
- Make target architecture decisions — that's the human + `/spec:define`
- Resolve types or follow indirection — unnecessary for boundary detection

Getting the boundaries roughly right and the dependency ordering defensible is enough. The human refines the draft, and the per-feature loop does the real work.

## Existing Infrastructure


| Capability                     | Status | Notes                                        |
| ------------------------------ | ------ | -------------------------------------------- |
| Extract specs from source code | Exists | `/spec:extract`                              |
| Capture runtime fixtures       | Exists | `wiretapper`                                 |
| Generate replay tests          | Exists | `replay-writer`                              |
| Define → Build → Merge chain   | Exists | `/spec:define`, `/spec:build`, `/spec:merge` |


## New Capabilities Required


| Capability                         | Type  | Notes                                                                                        |
| ---------------------------------- | ----- | -------------------------------------------------------------------------------------------- |
| Feature manifest (`manifest.yaml`) | CLI   | Ordered feature list with dependencies and per-feature status                                |
| `specify manifest init`            | CLI   | Structural discovery: directory walking, import-graph analysis, cluster classification, topological sort → draft `features.yaml` |
| `specify manifest next`            | CLI   | Return the next pending feature from the manifest (respecting `depends-on`)                  |
| `specify manifest status`          | CLI   | Show initiative progress: N/M features complete, current feature, blockers                   |
| Manifest orchestrator              | Skill | Reads the manifest, selects the next pending feature, wires the loop                         |
| Feature recommender                | Skill | Optional Layer 2 refinement: improve feature names, suggest cluster splits/merges, flag ambiguous boundaries |
| Behavioural diff                   | CLI   | Compare legacy fixture output against new implementation output (migration mode)             |
| Cross-stack define                 | Skill | Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust)        |


## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; manifest subcommands extend the CLI
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — provides federation resolution for manifests that span repositories

