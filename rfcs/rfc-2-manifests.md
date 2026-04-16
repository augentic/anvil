# RFC-2: Manifests

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Drive complex, multi-change initiatives through Specify's define-build-merge loop using a **manifest** — an ordered, dependency-aware plan of changes with kind-aware orchestration, status tracking, and progressive baseline accumulation. Legacy migration, greenfield builds, and platform modernisations all use the same manifest format and the same loop; the only difference is where the input to `/spec:define` comes from. Each change in the manifest declares a `kind` — feature, fix, or refactor — which determines which steps in the loop apply.

## Motivation

Complex initiatives — greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

The define-build-merge loop already works for individual changes. What's missing is the layer above: a manifest that sequences changes, tracks dependencies between them, and lets progress accumulate in the baseline across iterations. Without it, every iteration starts from scratch — the agent doesn't know what came before, what's in flight, or what's blocked.

Real initiatives are not composed exclusively of features. Bugs surface during extraction, refactoring becomes necessary before the next feature can land cleanly. The manifest accommodates all three kinds of work — features, fixes, and refactors — in a single ordered plan with shared dependency tracking.

By expressing the initiative as an ordered list of changes with dependency constraints, the manifest turns a sprawling effort into a series of self-contained Specify changes, each building on the baseline left by the last.

## Dependency on RFC-1

The manifest orchestrator, manifest parsing, and change recommender are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (define → build → merge, optionally preceded by extract) already works today; what this RFC adds is the manifest-driven automation layer, implemented as `specify manifest` subcommands on top of the CLI foundation.

## Detailed Design

### The Manifest

A manifest is an ordered list of the changes to implement, along with their kinds, dependencies, and status. It is the initiative's table of contents: it tells the loop what to do next without requiring the agent to rediscover scope on every iteration.

```yaml
# .specify/manifest.yaml
name: platform-v2

# Optional — only for migration/extraction use cases.
# Named source repositories. Changes reference these by key in their
# `sources` list. File-level scoping within a source is deferred to
# the extract step.
sources:
  monolith: /path/to/legacy-codebase
  orders: git@github.com:org/orders-service.git
  payments: git@github.com:org/payments-service.git
  frontend: git@github.com:org/web-app.git

changes:
  - name: user-registration
    kind: feature                          # feature | fix | refactor
    sources: [monolith]                    # which sources to extract from
    status: done                           # pending | in-progress | done | blocked | skipped

  - name: email-verification
    kind: feature
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress

  - name: registration-duplicate-email-crash
    kind: fix
    affects: [user-registration]           # which changes/capabilities this touches
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending

  - name: notification-preferences
    kind: feature
    depends-on: [user-registration]        # no sources → greenfield
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending

  - name: extract-shared-validation
    kind: refactor
    affects: [user-registration, email-verification]
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending

  - name: product-catalog
    kind: feature
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending

  - name: shopping-cart
    kind: feature
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending

  - name: checkout-api
    kind: feature
    sources: [payments]
    depends-on: [shopping-cart]
    status: pending

  - name: checkout-ui
    kind: feature
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
```

#### Change Kinds

Every change declares a `kind` that determines which steps in the loop apply:

| Kind | When to use | Loop behaviour |
|---|---|---|
| `feature` | New capability or migrated capability | Full loop: extract → define → build → merge |
| `fix` | Defect in an existing capability | Define (delta spec) → build → merge |
| `refactor` | Structural improvement, no behaviour change | Define (may be design-only) → build → merge |

If `kind` is omitted, it defaults to `feature`.

#### Status State Machine

```
                    ┌──────────┐
         ┌────────►│  blocked  │◄──── dependency not met / manual flag
         │          └────┬─────┘
         │               │ unblock
         │               ▼
  ┌──────┴───┐    ┌─────────────┐    ┌──────┐
  │  pending  ├───►│ in-progress ├───►│ done │
  └──────┬───┘    └──────┬──────┘    └──────┘
         │               │
         │               │ fail / defer
         │               ▼
         │          ┌─────────┐
         └────────►│ skipped  │
                    └─────────┘
```

- **`pending`** — not started; all dependencies satisfied or not yet checked
- **`in-progress`** — a Specify change exists for this entry
- **`done`** — change merged; specs in baseline
- **`blocked`** — cannot proceed; dependency unmet or manually flagged
- **`skipped`** — deliberately excluded from this initiative (with a reason in `description`)

#### `affects` vs `depends-on`

- **`depends-on`** — ordering constraint. "Don't start this until those are `done`." Consumed by `specify manifest next`.
- **`affects`** — impact annotation. "This change modifies behaviour defined by those changes." Consumed by the define step to know which baseline specs to load as delta targets, and by `specify manifest status` for impact reporting. Relevant for `fix` and `refactor` kinds.

#### Optional Fields

| Field | Relevant kinds | Purpose |
|---|---|---|
| `sources` | feature | Which source repos to extract from; keys reference the top-level `sources` map. Absent → greenfield |
| `affects` | fix, refactor | Which existing changes or capabilities are touched |
| `description` | all | Free-text context; guides the extract step when scoping within a source |

Extraction changes (those with `sources`) and greenfield changes (those without) coexist in the same manifest. A change's `sources` is a list of keys referencing entries in the top-level `sources` map — it declares *which* repositories to extract from, not *which files*. File-level scoping within a source is the extract step's responsibility; the change's `description` can provide hints when needed. A platform modernisation might extract core services from several legacy codebases while adding new capabilities and fixing defects discovered along the way — the manifest handles all three kinds in a single, ordered plan.

The manifest can be:

- **Human-authored** — a tech lead lists the changes they want and the order they want them in, encoding institutional knowledge about risk, priority, and dependencies
- **Auto-generated** — `specify manifest init` analyses the source codebases' directory structure and import graph to propose change boundaries and a leaf-first ordering (see [Manifest Generation](#manifest-generation-specify-manifest-init) below)
- **Hybrid** — the recommender proposes, the human reorders and prunes

When no manifest exists, the loop runs in **ad-hoc mode**: the user picks the next change interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

### The Loop

Each iteration of the loop is a single Specify change that implements one change from the manifest. The loop reuses the existing skill chain; the `kind` determines which steps apply:

```text
for each change in manifest.yaml (or user's choice):

  ┌─────────────────────────────────────────────────────────┐
  │                                                         │
  │  match change.kind:                                     │
  │    feature  → extract? → define → build → merge         │
  │    fix      → define (delta) → build → merge            │
  │    refactor → define (design-focused) → build → merge   │
  │                                                         │
  │  1. EXTRACT  /spec:extract   [feature + sources]         │
  │     Analyse the change's source repositories and        │
  │     produce Specify artifacts (specs + design.md)       │
  │     capturing the legacy behaviour in a single pass.    │
  │                                                         │
  │  2. DEFINE   /spec:define                               │
  │     Create or refine the artifacts for the target       │
  │     stack. For features: new specs or adapted extracts. │
  │     For fixes: delta specs against affected baseline.   │
  │     For refactors: design-focused, may skip new specs.  │
  │                                                         │
  │  3. BUILD    /spec:build                                │
  │     Implement every task against the target stack       │
  │     using the specs as source of truth. Run tests,      │
  │     verify against replay fixtures if available.        │
  │                                                         │
  │  4. MERGE    /spec:merge                                │
  │     Merge the change. Delta specs fold into baseline.   │
  │     The change is now under spec governance.            │
  │     Update manifest.yaml: status → done.                │
  │                                                         │
  │  5. NEXT     Loop to the next pending change            │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

Each iteration is a self-contained Specify change. The agent runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain it would run for any single change — the only difference is that the manifest decides what to do next, the `kind` determines which steps apply, and progress is tracked across iterations.

### Progressive Baseline Accumulation

The key mechanism is **baseline growth through merge**. After each iteration:

- The completed change's specs join `.specify/specs/` as baseline
- Subsequent iterations can reference these specs (e.g., the cart feature can reference the product-catalog specs that were merged in a prior iteration)
- The `touched-specs` conflict detection in `.metadata.yaml` prevents two in-flight changes from stomping on each other
- The archived changes in `.specify/changes/archive/` provide a complete audit trail

```text
Iteration 1:  baseline = {}
              define(user-registration) → build → merge          [feature]
              baseline = { user-registration }

Iteration 2:  baseline = { user-registration }
              define(registration-duplicate-email-crash) → build → merge  [fix]
              baseline = { user-registration (patched) }

Iteration 3:  baseline = { user-registration }
              define(notification-preferences) → build → merge   [feature]
              baseline = { user-registration, notification-preferences }

Iteration 4:  baseline = { user-registration, notification-preferences }
              extract(product-catalog) → define → build → merge  [feature]
              baseline = { user-registration, notification-preferences, product-catalog }

...

Iteration N:  baseline = { all changes }
              Initiative complete. Every change under spec governance.
```

This works identically regardless of change kind. Features add new specs to the baseline, fixes produce delta specs against existing baseline entries, and refactors may restructure specs without changing behaviour. The baseline doesn't care where the specs originated or what kind of change produced them — it only cares that they passed through the define-build-merge loop.

### Migration Mode

When the manifest includes a `sources` section and changes reference them, the loop operates in **migration mode** — the same loop with additional extraction and verification capabilities.

#### Extraction

For each change with `sources`, the EXTRACT step analyses the referenced source repositories and produces Specify artifacts capturing the existing behaviour. The extract step determines which files within the source are relevant to the change — using the change name, description, and dependency context as scoping hints. The input to `/spec:define` comes from `/spec:extract` rather than a human description.

#### Fixture-Backed Verification

For changes where the `wiretapper` has captured runtime request/response fixtures from the legacy system, each iteration gains an additional verification step:

1. Before the build phase, the `replay-writer` generates tests from the captured fixtures
2. During the build phase, the implementation is verified against these replay tests
3. The tests assert that the new implementation produces the same outputs as the legacy system for the same inputs

This creates a behavioural regression safety net that catches semantic drift — the most common failure mode in legacy migrations.

#### Slice Strategy

Not every part of a legacy system is equally suitable for early migration. Good early candidates are:

- **Leaf services** with few upstream dependents — migrating them doesn't break anything
- **Clear API boundaries** — the input/output contract is well-defined and testable
- **Existing test coverage** or easy-to-capture request/response patterns (good `wiretapper` candidates)
- **Low cross-boundary coupling** — the change doesn't reach deep into shared mutable state

The `depends-on` field in the manifest encodes inter-change dependencies. The orchestrator (or `specify manifest next`) respects these: a change won't be selected until all its dependencies have status `done`. This prevents the loop from attempting to build a change that references specs that haven't been merged into the baseline yet.

For teams without a clear ordering, `specify manifest init` analyses the legacy codebase's directory structure and import graph to suggest change boundaries and a leaf-first ordering (see [Manifest Generation](#manifest-generation-specify-manifest-init)).

### Multi-Repo Initiatives

The manifest supports multi-repo initiatives on both the source and target sides:

- **Multi-source extraction.** The top-level `sources` map names the repositories available to the initiative. A change's `sources` list declares which of these repos to extract from; a change may reference multiple sources when understanding the feature requires both (e.g., backend handlers and frontend components). File-level scoping is deferred to the extract step.

- **Multi-target implementation.** When a logical feature spans multiple build targets — for example, a backend API and a frontend UI — it is decomposed into separate changes with explicit `depends-on` edges between them.

- **Cross-repo resolution.** Cross-repo spec references are resolved through the federation model defined in [RFC-3](rfc-3-multi-repo.md). The manifest provides the coordination layer (what changes, in what order, with what dependencies); federation provides the resolution layer (where specs live, how to validate cross-repo contracts).

### Manifest Generation (`specify manifest init`)

`specify manifest init` bootstraps a draft `manifest.yaml` from one or more source codebases. The analysis is deliberately shallow — it identifies change boundaries, dependency edges, and a safe ordering. It does not analyse behaviour, generate specs, or make target design decisions; that work belongs to `/spec:extract` and `/spec:define` when each change reaches the head of the loop.

The generation is split into two layers: deterministic structural discovery (CLI) and optional LLM-assisted refinement (change recommender skill).

#### Layer 1: Structural Discovery (CLI)

Pure filesystem and import-graph analysis — no LLM involved.

**Step 1 — Discover module boundaries.** Walk the source codebase(s) and identify natural groupings:

- **Directory structure** — the strongest signal. Most codebases organise by domain at some level (`src/auth/`, `src/cart/`, `src/catalog/`). Directories that contain models, handlers, routes, or services are candidate change roots.
- **Build system boundaries** — workspace members in `Cargo.toml`, `package.json` workspaces, Go module paths, Python packages with `__init__.py`. These are explicit module declarations by the original authors.
- **Entry points** — route registrations, handler files, exported modules. These hint at what the codebase considers its public surface.

**Step 2 — Build a coarse import graph.** Parse import/require/use statements across files — just the paths, not the symbols. This doesn't require a full AST; regex patterns per detected language handle the common cases:

- TypeScript/JS: `import ... from '...'` / `require('...')`
- Rust: `use crate::...` / `mod ...`
- Go: `import "..."`
- Python: `from ... import ...` / `import ...`

Aggregate file-level imports to the cluster level. If any file in cluster A imports any file in cluster B, that's a dependency edge A → B.

**Step 3 — Classify clusters.** Not every directory is a change. Utility clusters — directories imported by most other clusters but importing none of them (e.g., `shared/`, `utils/`, `lib/`) — are classified as infrastructure rather than changes. Infrastructure clusters are either absorbed into the changes that reference them or flagged as a prerequisite to migrate first.

**Step 4 — Topological sort.** With clusters and dependency edges, topological sort produces a leaf-first ordering — the safest migration sequence since leaf changes have no downstream dependents. Cycles are detected and flagged for human review; they usually indicate the change boundaries need splitting.

**Step 5 — Emit the manifest.** Write `manifest.yaml` with the discovered changes (all `kind: feature`), their `sources` references, `depends-on` edges, and all statuses set to `pending`. Fixes and refactors are typically added by hand as the initiative progresses.

For multi-source initiatives the CLI runs structural discovery against each entry in the `sources` map independently, then merges the results into a single change list. Cross-source dependencies are unlikely (the sources are separate repos) but are flagged if detected.

#### Layer 2: Change Recommender (Skill, Optional)

The CLI output from Layer 1 uses directory-derived names (`auth`, `cart`, `catalog`). The change recommender skill can optionally refine the draft:

- Propose human-readable change names from file contents (top-level exports, function signatures, route paths — not deep analysis)
- Suggest splitting oversized clusters or merging trivially small ones
- Flag ambiguous boundaries for human review

Layer 2 is explicitly optional. The CLI produces a valid manifest from Layer 1 alone; the skill makes it more readable.

#### Scope Boundary

The manifest is a table of contents, not a design document. `specify manifest init` deliberately does not:

- Analyse function bodies or business logic — that's `/spec:extract`
- Generate specs or design documents — that's `/spec:define`
- Make target architecture decisions — that's the human + `/spec:define`
- Resolve types or follow indirection — unnecessary for boundary detection

Getting the boundaries roughly right and the dependency ordering defensible is enough. The human refines the draft, and the per-change loop does the real work.

## Existing Infrastructure


| Capability                     | Status | Notes                                        |
| ------------------------------ | ------ | -------------------------------------------- |
| Extract specs from source code | Exists | `/spec:extract`                              |
| Capture runtime fixtures       | Exists | `wiretapper`                                 |
| Generate replay tests          | Exists | `replay-writer`                              |
| Define → Build → Merge chain   | Exists | `/spec:define`, `/spec:build`, `/spec:merge` |


## New Capabilities Required


| Capability                         | Type  | Notes                                                                                                                             |
| ---------------------------------- | ----- | --------------------------------------------------------------------------------------------------------------------------------- |
| Manifest (`manifest.yaml`)         | CLI   | Ordered change list with kinds, dependencies, and per-change status                                                               |
| `specify manifest init`            | CLI   | Structural discovery: directory walking, import-graph analysis, cluster classification, topological sort → draft `manifest.yaml`  |
| `specify manifest next`            | CLI   | Return the next pending change from the manifest (respecting `depends-on`)                                                        |
| `specify manifest status`          | CLI   | Show initiative progress: N/M changes complete, current change, blockers. Filterable by `--kind`                                  |
| Manifest orchestrator              | Skill | Reads the manifest, selects the next pending change, wires the kind-appropriate loop                                              |
| Change recommender                 | Skill | Optional Layer 2 refinement: improve change names, suggest cluster splits/merges, flag ambiguous boundaries                       |
| Behavioural diff                   | CLI   | Compare legacy fixture output against new implementation output (migration mode)                                                  |
| Cross-stack define                 | Skill | Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust)                                             |


## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; manifest subcommands extend the CLI
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — provides federation resolution for manifests that span repositories

