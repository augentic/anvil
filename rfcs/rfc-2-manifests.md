# RFC-2: Manifests

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Drive complex, multi-change initiatives through Specify's define-build-merge loop using a **manifest** — an ordered, dependency-aware plan of changes with status tracking and progressive baseline accumulation. Legacy migration, greenfield builds, and platform modernisations all use the same manifest format and the same loop; the only difference is where the input to `/spec:define` comes from.

## Motivation

Complex initiatives — greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

The define-build-merge loop already works for individual changes. What's missing is the layer above: a manifest that sequences changes, tracks dependencies between them, and lets progress accumulate in the baseline across iterations. Without it, every iteration starts from scratch — the agent doesn't know what came before, what's in flight, or what's blocked.

By expressing the initiative as an ordered list of changes with dependency constraints, the manifest turns a sprawling effort into a series of self-contained Specify changes, each building on the baseline left by the last.

## Dependency on RFC-1

The manifest orchestrator and manifest parsing are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (define → build → merge) already works today; what this RFC adds is the manifest-driven automation layer, implemented as `specify manifest` subcommands on top of the CLI foundation.

## Detailed Design

### The Manifest

A manifest is an ordered list of the changes to implement, along with their dependencies and status. It is the initiative's table of contents: it tells the loop what to do next without requiring the agent to rediscover scope on every iteration.

```yaml
# .specify/manifest.yaml
name: platform-v2

# Optional — only for migration/extraction use cases.
# Named source repositories. Changes reference these by key in their
# `sources` list. File-level scoping within a source is deferred to
# the define step (using extract skill).
sources:
  monolith: /path/to/legacy-codebase
  orders: git@github.com:org/orders-service.git
  payments: git@github.com:org/payments-service.git
  frontend: git@github.com:org/web-app.git

changes:
  - name: user-registration
    sources: [monolith]              # which sources to analyze
    status: done                     # pending | in-progress | done | blocked | failed | skipped

  - name: email-verification
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress

  - name: registration-duplicate-email-crash
    affects: [user-registration]           # which changes/capabilities this touches
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending

  - name: notification-preferences
    depends-on: [user-registration]        # no sources → greenfield
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending

  - name: extract-shared-validation
    affects: [user-registration, email-verification]
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending

  - name: product-catalog
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending

  - name: shopping-cart
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending

  - name: checkout-api
    sources: [payments]
    depends-on: [shopping-cart]
    status: failed
    failure-reason: >
      Type mismatch between cart line-item schema and payment gateway contract.
      Needs design revision after shopping-cart specs are updated.

  - name: checkout-ui
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
```

#### Status State Machine

```
                      ┌─────────┐
             flag ───►│ blocked │──── unflag ────┐
                      └─────────┘                │
                                                 ▼
    ┌───────────┐   select    ┌─────────────┐  merge   ┌──────┐
───►│  pending  ├────────────►│ in-progress ├─────────►│ done │
    └─────┬─────┘             └──────┬──────┘          └──────┘
          │  ▲                       │
          │  │ retry                 │ drop
          │  │                       ▼
          │  │                ┌──────────┐
          │  └────────────────┤  failed  │
          │                   └─────┬────┘
          │  exclude     abandon    │
          │  ┌──────────────────────┘
          ▼  ▼
    ┌───────────┐
    │  skipped  │──── re-include ────► pending
    └───────────┘
```

- `**pending**` — not started; eligible for selection by `manifest next` if all `depends-on` entries are `done`
- `**in-progress**` — a Specify change has been created; define/build/merge is underway
- `**done**` — change merged successfully; specs are in baseline
- `**blocked**` — manually flagged as unable to proceed (with a reason in `description`). Dependency ordering is *not* modelled as `blocked` — `manifest next` enforces `depends-on` at query time by only returning `pending` changes whose dependencies are all `done`
- `**failed*`* — attempted but unsuccessful; the Specify change was dropped. Distinct from `skipped`, which is a deliberate exclusion
- `**skipped**` — deliberately excluded from this initiative (with a reason in `description`); never attempted or no longer needed

#### Transition Rules


| Transition | Trigger | Who |
| ---------- | ------- | --- |
| `pending → in-progress` | Specify change directory created for this entry | Orchestrator or user |
| `pending → blocked`     | Flagged with a reason — design uncertainty, external dependency, etc. | Manual |
| `pending → skipped` | Deliberately excluded before attempting | Manual |
| `blocked → pending` | Flag removed | Manual |
| `in-progress → done` | Specify change reaches `merged` (`/spec:merge` completes) | Orchestrator updates manifest after merge |
| `in-progress → failed` | Build or test failure; Specify change is dropped (`/spec:drop`) | Orchestrator or user |
| `failed → pending` | User decides to retry; a fresh Specify change will be created on next selection | Manual |
| `failed → skipped` | User decides not to retry | Manual |
| `skipped → pending` | Previously excluded change re-included | Manual |


Only **one** change may be `in-progress` at a time per manifest (single-threaded loop). `manifest next` refuses to return a new change while one is already `in-progress`.

On failure, the Specify change is **dropped** via `/spec:drop`, cleaning up partial artifacts. On retry (`failed → pending`), a fresh change is created when the entry is next selected.

#### Mapping to Specify LifecycleStatus

The manifest tracks coarse outcome; the Specify change tracks internal lifecycle. The orchestrator reads the change's `LifecycleStatus` to decide which loop step to run next; the manifest only records whether the change is finished.


| Manifest status                              | Specify change state                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------------- |
| `pending` / `blocked` / `skipped` / `failed` | No active Specify change                                                            |
| `in-progress`                                | Change exists — `LifecycleStatus` ∈ {`defining`, `defined`, `building`, `complete`} |
| `done`                                       | Change reached `merged`                                                             |


#### `affects` vs `depends-on`

- `**depends-on`** — ordering constraint. "Don't start this until those are `done`." Consumed by `specify manifest next`.
- `**affects**` — impact annotation. "This change modifies behaviour defined by those changes." Consumed by the define step to know which baseline specs to load as delta targets, and by `specify manifest status` for impact reporting. The orchestrator passes `affects` entries to define as a list of capability names; define resolves them to `.specify/specs/<name>/` paths. The orchestrator does not need to understand the spec filesystem.

#### Optional Fields


| Field | Purpose |
| ----- | ------- |
| `kind` | Optional label (`feature`, `fix`, `refactor`) for human readers; does not affect execution |
| `sources` | Which source repos to analyze; keys reference the top-level `sources` map. Absent → greenfield |
| `affects` | Which existing changes or capabilities are touched |
| `description`    | Free-text context; guides the define step when scoping within a source |
| `failure-reason` | Why the change failed; aids triage and retry decisions without overloading `description` |


Extraction changes (those with `sources`) and greenfield changes (those without) coexist in the same manifest. A change's `sources` is a list of keys referencing entries in the top-level `sources` map — it declares *which* repositories to analyze, not *which files*. File-level scoping within a source is the define step's responsibility; the change's `description` can provide hints when needed. A platform modernisation might extract core services from several legacy codebases while adding new capabilities and fixing defects discovered along the way — the manifest handles all of these in a single, ordered plan.

Manifests are human-authored — a tech lead lists the changes they want and the order they want them in, encoding institutional knowledge about risk, priority, and dependencies. Automated generation (`specify manifest init`) from source codebase analysis is a planned future capability.

When no manifest exists, the loop runs in **ad-hoc mode**: the user picks the next change interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

### The Loop

Each iteration of the loop is a single Specify change that implements one change from the manifest. The loop reuses the existing skill chain:

```text
for each change in manifest.yaml (or user's choice):

  ┌─────────────────────────────────────────────────────────┐
  │                                                         │
  │  1. DEFINE   /spec:define                               │
  │     Create the change artifacts. When the change has    │
  │     sources, define invokes /spec:extract to analyze    │
  │     the source repositories and produce specs +         │
  │     design from existing code. When no sources are      │
  │     present, define works from the change description.  │
  │     Changes with `affects` produce delta specs against  │
  │     the affected baseline entries.                      │
  │                                                         │
  │  2. BUILD    /spec:build                                │
  │     Implement every task against the target stack       │
  │     using the specs as source of truth. Run tests,      │
  │     verify against replay fixtures if available.        │
  │                                                         │
  │  3. MERGE    /spec:merge                                │
  │     Merge the change. Delta specs fold into baseline.   │
  │     The change is now under spec governance.            │
  │     Update manifest.yaml: status → done.                │
  │                                                         │
  │  4. NEXT     Loop to the next pending change            │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

Each iteration is a self-contained Specify change. The agent runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain it would run for any single change — the only difference is that the manifest decides what to do next and progress is tracked across iterations.

### Progressive Baseline Accumulation

The key mechanism is **baseline growth through merge**. After each iteration:

- The completed change's specs join `.specify/specs/` as baseline
- Subsequent iterations can reference these specs (e.g., the cart feature can reference the product-catalog specs that were merged in a prior iteration)
- The `touched-specs` conflict detection in `.metadata.yaml` prevents two in-flight changes from stomping on each other
- The archived changes in `.specify/changes/archive/` provide a complete audit trail

```text
Iteration 1:  baseline = {}
              define(user-registration) → build → merge
              baseline = { user-registration }

Iteration 2:  baseline = { user-registration }
              define(registration-duplicate-email-crash) → build → merge
              baseline = { user-registration (patched) }

Iteration 3:  baseline = { user-registration }
              define(notification-preferences) → build → merge
              baseline = { user-registration, notification-preferences }

Iteration 4:  baseline = { user-registration, notification-preferences }
              define(product-catalog) → build → merge
              baseline = { user-registration, notification-preferences, product-catalog }

...

Iteration N:  baseline = { all changes }
              Initiative complete. Every change under spec governance.
```

This works identically for all changes. New capabilities add specs to the baseline, while changes with `affects` produce delta specs against existing baseline entries. The baseline doesn't care where the specs originated — it only cares that they passed through the define-build-merge loop.

### Migration Mode

When the manifest includes a `sources` section and changes reference them, the loop operates in **migration mode** — the same define-build-merge loop with source-aware define and additional verification capabilities.

#### Source-Aware Define

For changes with `sources`, the define step invokes `/spec:extract` to analyze the referenced source repositories and produce Specify artifacts (specs + design.md) capturing the existing behaviour. The define step determines which files within the source are relevant to the change — using the change name, description, and dependency context as scoping hints. This is the only difference between migration and greenfield: where define gets its input. The loop steps are identical.

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

### Multi-Repo Initiatives

The manifest supports multi-repo initiatives on both the source and target sides:

- **Multi-source extraction.** The top-level `sources` map names the repositories available to the initiative. A change's `sources` list declares which of these repos to extract from; a change may reference multiple sources when understanding the feature requires both (e.g., backend handlers and frontend components). File-level scoping is deferred to the define step.
- **Multi-target implementation.** When a logical feature spans multiple build targets — for example, a backend API and a frontend UI — it is decomposed into separate changes with explicit `depends-on` edges between them.
- **Cross-repo resolution.** Cross-repo spec references are resolved through the federation model defined in [RFC-3](rfc-3-multi-repo.md). The manifest provides the coordination layer (what changes, in what order, with what dependencies); federation provides the resolution layer (where specs live, how to validate cross-repo contracts).

### Conventions

- **Location.** One manifest per project at `.specify/manifest.yaml`. Multiple manifests are a future concern.
- **Name identity.** The manifest entry `name` becomes the Specify change name (the directory under `.specify/changes/`). This is the key that links the manifest to the Specify change lifecycle.
- **List order.** The YAML list order is cosmetic. Execution order is determined solely by `depends-on` resolution. Reordering entries in the list has no effect on execution.
- **Adding changes mid-initiative.** Edit `manifest.yaml` and add an entry with `status: pending`. No command required.
- **Initiative completion.** The loop stops when no eligible changes remain. `specify manifest status` reports whether this means "all done" or "remaining changes are blocked/failed".

## Existing Infrastructure


| Capability                     | Status | Notes                                        |
| ------------------------------ | ------ | -------------------------------------------- |
| Source code analysis for define | Exists | `/spec:extract` (invoked by `/spec:define` when change has `sources`) |
| Capture runtime fixtures       | Exists | `wiretapper`                                 |
| Generate replay tests          | Exists | `replay-writer`                              |
| Define → Build → Merge chain   | Exists | `/spec:define`, `/spec:build`, `/spec:merge` |


## New Capabilities Required


| Capability                    | Type  | Notes                                                                          |
| ----------------------------- | ----- | ------------------------------------------------------------------------------ |
| Manifest (`manifest.yaml`)    | CLI   | Ordered change list with dependencies and per-change status                    |
| `specify manifest validate`   | CLI   | Cycle detection, referential integrity (`depends-on`, `affects`, `sources` keys), duplicate name check |
| `specify manifest next`       | CLI   | Return the next pending change from the manifest (respecting `depends-on`)     |
| `specify manifest status`     | CLI   | Show initiative progress: N/M changes complete, current change, blockers       |
| Manifest orchestrator         | Skill | Reads the manifest, selects the next pending change, wires the define → build → merge loop. See [orchestrator addendum](rfc-2-orchestrator.md) |


## Deferred Capabilities

These are planned but not part of the initial implementation:

| Capability | Rationale for deferral |
| ---------- | ---------------------- |
| `specify manifest init` | Humans write better initial manifests than automated structural discovery. Add when the volume of initiatives justifies the tooling. |
| Change recommender | LLM-assisted refinement of auto-generated manifests. Depends on `manifest init`. |
| Behavioural diff | Undesigned. The existing `replay-writer` already provides fixture-backed verification for migration use cases. |
| Cross-stack define | A mode of `/spec:define`, not a manifest concern. Can be added to define independently. |


## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; manifest subcommands extend the CLI
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — provides federation resolution for manifests that span repositories

