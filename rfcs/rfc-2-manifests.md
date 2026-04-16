# RFC-2: Manifests

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Drive complex, multi-change initiatives through Specify's define-build-merge loop using a **manifest** — an ordered, dependency-aware plan of changes with status tracking and progressive baseline accumulation. The manifest format supports greenfield builds, legacy migrations, and platform modernisations; the only difference is where the input to `/spec:define` comes from.

This RFC is structured in two layers. **Layer 1** (the MVP) delivers the manifest format and CLI commands — enough for a human to drive a manifest-based initiative using the existing skill chain. **Layer 2** adds the orchestrator skill that automates the loop. Layer 1 is immediately useful without Layer 2.

## Motivation

Complex initiatives — greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

The define-build-merge loop already works for individual changes. What's missing is the layer above: a manifest that sequences changes, tracks dependencies between them, and lets progress accumulate in the baseline across iterations. Without it, every iteration starts from scratch — the agent doesn't know what came before, what's in flight, or what's blocked.

By expressing the initiative as an ordered list of changes with dependency constraints, the manifest turns a sprawling effort into a series of self-contained Specify changes, each building on the baseline left by the last.

## Dependency on RFC-1

Manifest parsing, validation, and status transitions are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (define → build → merge) already works today; what this RFC adds is the manifest-driven coordination layer, implemented as `specify manifest` subcommands on top of the CLI foundation.

---

## Layer 1: Manifest Format + CLI (MVP)

### The Manifest

A manifest is an ordered list of the changes to implement, along with their dependencies and status. It is the initiative's table of contents: it tells the loop what to do next without requiring the agent to rediscover scope on every iteration.

```yaml
# .specify/manifest.yaml
name: platform-v2

# Optional — only for migration/extraction use cases.
# Named source repositories. Changes reference these by key in their
# `sources` list. File-level scoping within a source is deferred to
# the define step (using extract skill).
# NOTE: source-aware execution is a Layer 2 / future capability.
# The MVP parses and validates source references but does not
# wire them into the define step automatically.
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

### Status State Machine

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

- **`pending`** — not started; eligible for selection by `manifest next` if all `depends-on` entries are `done`
- **`in-progress`** — a Specify change has been created; define/build/merge is underway
- **`done`** — change merged successfully; specs are in baseline
- **`blocked`** — manually flagged as unable to proceed (with a reason in `description`). Dependency ordering is *not* modelled as `blocked` — `manifest next` enforces `depends-on` at query time by only returning `pending` changes whose dependencies are all `done`
- **`failed`** — attempted but unsuccessful; the Specify change was dropped. Distinct from `skipped`, which is a deliberate exclusion
- **`skipped`** — deliberately excluded from this initiative (with a reason in `description`); never attempted or no longer needed

#### Transition Rules


| Transition | Trigger | Who |
| ---------- | ------- | --- |
| `pending → in-progress` | Specify change directory created for this entry | `specify manifest transition` (user or orchestrator) |
| `pending → blocked`     | Flagged with a reason — design uncertainty, external dependency, etc. | `specify manifest transition` (manual) |
| `pending → skipped` | Deliberately excluded before attempting | `specify manifest transition` (manual) |
| `blocked → pending` | Flag removed | `specify manifest transition` (manual) |
| `in-progress → done` | Specify change reaches `merged` (`/spec:merge` completes) | `specify manifest transition` (user or orchestrator) |
| `in-progress → failed` | Build or test failure; Specify change is dropped (`/spec:drop`) | `specify manifest transition` (user or orchestrator) |
| `in-progress → blocked` | Needs human decision mid-change (Layer 2: orchestrator defers) | `specify manifest transition` (orchestrator) |
| `failed → pending` | User decides to retry; a fresh Specify change will be created on next selection | `specify manifest transition` (manual) |
| `failed → skipped` | User decides not to retry | `specify manifest transition` (manual) |
| `skipped → pending` | Previously excluded change re-included | `specify manifest transition` (manual) |


Only **one** change may be `in-progress` at a time per manifest (single-threaded loop). `manifest next` refuses to return a new change while one is already `in-progress`.

On failure, the Specify change is **dropped** via `/spec:drop`, cleaning up partial artifacts. On retry (`failed → pending`), a fresh change is created when the entry is next selected.

#### Mapping to Specify LifecycleStatus

The manifest tracks coarse outcome; the Specify change tracks internal lifecycle. When a future orchestrator reads the change's `LifecycleStatus` to decide which loop step to run next, the manifest only records whether the change is finished.


| Manifest status                              | Specify change state                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------------- |
| `pending` / `blocked` / `skipped` / `failed` | No active Specify change                                                            |
| `in-progress`                                | Change exists — `LifecycleStatus` ∈ {`defining`, `defined`, `building`, `complete`} |
| `done`                                       | Change reached `merged`                                                             |


#### `affects` vs `depends-on`

- **`depends-on`** — ordering constraint. "Don't start this until those are `done`." Consumed by `specify manifest next`.
- **`affects`** — impact annotation. "This change modifies behaviour defined by those changes." In the MVP, `affects` is parsed, validated (targets must exist in the manifest), and reported by `specify manifest status` for impact visibility. Wiring `affects` into the define step as automatic delta-target resolution is a Layer 2 capability.

#### Fields


| Field | Required | Purpose |
| ----- | -------- | ------- |
| `name` | Yes | Kebab-case identifier; becomes the Specify change directory name. Must be unique across the entire manifest. |
| `status` | Yes | Current state in the status state machine |
| `depends-on` | No | List of change names that must be `done` before this change is eligible |
| `description` | No | Free-text context; guides the define step when scoping |
| `kind` | No | Label (`feature`, `fix`, `refactor`) for human readers; does not affect execution |
| `sources` | No | Which source repos to analyze; keys reference the top-level `sources` map. Absent → greenfield. Parsed and validated in Layer 1; source-aware execution in Layer 2. |
| `affects` | No | Which existing changes or capabilities are touched. Parsed and validated in Layer 1; automatic delta-target wiring in Layer 2. |
| `failure-reason` | No | Why the change failed; aids triage and retry decisions without overloading `description` |

### The Loop (Human-Driven)

In Layer 1, the human is the orchestrator. The CLI provides the coordination primitives; the human drives the skill chain:

```text
specify manifest status                          # where are we?
specify manifest next                            # what's eligible?
specify manifest transition <name> in-progress   # claim it

/spec:define <name>                              # existing skill
/spec:build <name>                               # existing skill
/spec:merge <name>                               # existing skill

specify manifest transition <name> done          # record completion
```

On failure:

```text
/spec:drop <name>                                # existing skill
specify manifest transition <name> failed --reason "..."
```

Each iteration is a self-contained Specify change. The user runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain they would run for any single change — the only difference is that the manifest decides what to do next and progress is tracked across iterations.

When no manifest exists, the loop runs in **ad-hoc mode**: the user picks the next change interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

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

### CLI Commands

#### `specify manifest validate`

Structural validation of `manifest.yaml`:

- **Duplicate names** — every `name` must be unique
- **Cycle detection** — the `depends-on` graph must be a DAG (topological sort via `petgraph`)
- **Referential integrity** — every `depends-on` target, every `affects` target, and every `sources` key must reference an existing entry
- **Status values** — every `status` must be a valid state machine value
- **Single in-progress** — at most one change may have status `in-progress`
- **Manifest-to-change consistency** — any `in-progress` entry must have a corresponding `.specify/changes/<name>/` directory; report orphaned changes (directories without manifest entries) as warnings

Returns structured JSON (with `--format json`) or human-readable text.

#### `specify manifest next`

Return the next eligible change: a `pending` change whose `depends-on` entries are all `done`. Selection among multiple eligible changes follows list order (first eligible wins).

- If a change is `in-progress`, refuse and report which change is active.
- If no changes are eligible, report whether this means "all done" or "remaining changes are blocked/failed/pending-on-dependencies."

#### `specify manifest status`

Initiative progress report:

- Total changes, grouped by status (N done, M pending, etc.)
- Current `in-progress` change (if any), with its Specify `LifecycleStatus` from `.metadata.yaml`
- Blocked/failed entries with their reasons
- Next eligible changes (what `manifest next` would return)
- Impact report: which `done` changes are referenced by `affects` entries still pending
- Display in dependency order (topological sort), not list order

#### `specify manifest transition`

```
specify manifest transition <name> <target-status> [--reason "..."]
```

Validated status transitions. The command:

1. Reads `manifest.yaml`
2. Validates the transition is legal per the state machine
3. Updates the entry's `status`
4. If `--reason` is provided, sets `failure-reason` (for `failed`) or appends to `description` (for `blocked`/`skipped`)
5. Writes the manifest atomically
6. Outputs the new state

All manifest mutations go through this command, ensuring the state machine is always enforced. The future orchestrator will use the same command (or the underlying `specify-change` crate function) rather than editing YAML directly.

### Library Implementation

The manifest state machine is encoded in the `specify-change` crate (see [RFC-1](rfc-1-cli.md) `Workspace Layout`), alongside the existing `LifecycleStatus`:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManifestStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
    Failed,
    Skipped,
}

impl ManifestStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use ManifestStatus::*;
        matches!(
            (self, target),
            (Pending, InProgress)
                | (Pending, Blocked)
                | (Pending, Skipped)
                | (InProgress, Done)
                | (InProgress, Failed)
                | (InProgress, Blocked)
                | (Blocked, Pending)
                | (Failed, Pending)
                | (Failed, Skipped)
                | (Skipped, Pending)
        )
    }

    pub fn transition(
        &self,
        target: ManifestStatus,
    ) -> Result<ManifestStatus, Error> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(Error::ManifestTransition {
                from: self.clone(),
                to: target,
            })
        }
    }
}
```

Dependency resolution uses `petgraph` for topological sort and cycle detection. The `manifest.rs` module (in `specify-change`, alongside the lifecycle state machine) provides:

```rust
pub struct Manifest {
    pub name: String,
    pub sources: BTreeMap<String, String>,
    pub changes: Vec<ManifestChange>,
}

pub struct ManifestChange {
    pub name: String,
    pub status: ManifestStatus,
    pub depends_on: Vec<String>,
    pub affects: Vec<String>,
    pub sources: Vec<String>,
    pub description: Option<String>,
    pub failure_reason: Option<String>,
    pub kind: Option<String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, Error>;
    pub fn save(&self, path: &Path) -> Result<(), Error>;
    pub fn validate(&self) -> Vec<ValidationResult>;
    pub fn next_eligible(&self) -> Option<&ManifestChange>;
    pub fn transition(
        &mut self,
        name: &str,
        target: ManifestStatus,
        reason: Option<&str>,
    ) -> Result<(), Error>;
    pub fn topological_order(&self) -> Result<Vec<&ManifestChange>, Error>;
    pub fn consistency_check(
        &self,
        changes_dir: &Path,
    ) -> Vec<ConsistencyWarning>;
}
```

### Conventions

- **Location.** One manifest per project at `.specify/manifest.yaml`. Multiple manifests are a future concern.
- **Name identity.** The manifest entry `name` becomes the Specify change name (the directory under `.specify/changes/`). Names must be unique across the entire manifest, including entries with terminal statuses (`done`, `skipped`).
- **Name format.** Same as Specify change names: kebab-case (lowercase letters, digits, hyphens).
- **List order.** The YAML list order is cosmetic. Execution order is determined solely by `depends-on` resolution. Reordering entries in the list has no effect on execution. When multiple changes are eligible, list order breaks ties (`manifest next` returns the first eligible).
- **Adding changes mid-initiative.** Use `specify manifest transition` to add or edit entries, or edit `manifest.yaml` directly and run `specify manifest validate` to check integrity.
- **Initiative completion.** The initiative is complete when no eligible changes remain. `specify manifest status` reports whether this means "all done" or "remaining changes are blocked/failed."
- **Manifest-to-change linkage.** `specify manifest validate` checks that `in-progress` entries have corresponding `.specify/changes/<name>/` directories and reports orphaned change directories (present on disk but absent from the manifest) as warnings.

---

## Layer 2: Orchestrator Skill (Future)

Layer 2 adds an orchestrator skill that automates the human-driven loop from Layer 1. It reads the manifest, selects the next eligible change, runs the define → build → merge skill chain, and updates the manifest — all without human intervention for each step.

The full orchestrator design is in the [orchestrator addendum](rfc-2-orchestrator.md). Key aspects:

### Skill-Invokes-Skill Execution Model

The orchestrator is the first skill that programmatically invokes other skills. This is a new execution model — no existing skill calls another skill. The mechanics of how one skill invokes another, waits for completion, and reads the result need to be designed as part of Layer 2. The non-interactive execution table in the addendum provides the specification for pre-resolving every interactive decision point.

### Automatic `sources` and `affects` Wiring

In Layer 2, the orchestrator resolves `sources` keys to paths and passes them to `/spec:define` so it can invoke `/spec:extract` for source analysis. It also passes `affects` entries as capability names so define loads the corresponding baseline specs as delta targets.

### Autonomous Loop Mode

```
/spec:orchestrate [--dry-run] [--loop]
```

Single-change processing (supervised mode) and full-initiative processing (`--loop`, autonomous mode) as described in the addendum.

### Question Recording and Journal

When the orchestrator encounters a situation requiring human input, it records the question in `.specify/changes/<name>/journal.yaml`, transitions the manifest entry to `blocked`, and moves on. The journal provides structured context for the human to resolve the question.

---

## Future Capabilities

These are supported by the manifest format but not part of the initial implementation:

### Migration Mode

When the manifest includes a `sources` section and changes reference them, the loop can operate in **migration mode** — the same define-build-merge loop with source-aware define and additional verification capabilities.

**Source-aware define.** For changes with `sources`, the define step invokes `/spec:extract` to analyze the referenced source repositories and produce Specify artifacts (specs + design.md) capturing the existing behaviour. The define step determines which files within the source are relevant to the change — using the change name, description, and dependency context as scoping hints. This is the only difference between migration and greenfield: where define gets its input.

**Fixture-backed verification.** For changes where the `wiretapper` has captured runtime request/response fixtures from the legacy system, the `replay-writer` generates tests from the captured fixtures and the build phase verifies the new implementation against them. This creates a behavioural regression safety net.

**Slice strategy.** Good early migration candidates are leaf services with few dependents, clear API boundaries, existing test coverage, and low cross-boundary coupling. The `depends-on` field encodes these ordering decisions.

### Multi-Repo Initiatives

The manifest supports multi-repo initiatives on both the source and target sides:

- **Multi-source extraction.** A change's `sources` list declares which repos to extract from; a change may reference multiple sources.
- **Multi-target implementation.** Features spanning multiple build targets are decomposed into separate changes with `depends-on` edges.
- **Cross-repo resolution.** Cross-repo spec references are resolved through the federation model defined in [RFC-3](rfc-3-multi-repo.md).

### Other Deferred Capabilities


| Capability | Rationale for deferral |
| ---------- | ---------------------- |
| `specify manifest init` | Humans write better initial manifests than automated structural discovery. Add when the volume of initiatives justifies the tooling. |
| Change recommender | LLM-assisted refinement of auto-generated manifests. Depends on `manifest init`. |
| Behavioural diff | Undesigned. The existing `replay-writer` already provides fixture-backed verification for migration use cases. |
| Cross-stack define | A mode of `/spec:define`, not a manifest concern. Can be added to define independently. |


## Existing Infrastructure


| Capability                     | Status | Notes                                        |
| ------------------------------ | ------ | -------------------------------------------- |
| Source code analysis for define | Exists | `/spec:extract` (invoked by `/spec:define` when change has `sources`) |
| Capture runtime fixtures       | Exists | `wiretapper`                                 |
| Generate replay tests          | Exists | `replay-writer`                              |
| Define → Build → Merge chain   | Exists | `/spec:define`, `/spec:build`, `/spec:merge` |


## New Capabilities Required

### Layer 1 (MVP)


| Capability                       | Type  | Notes                                                                          |
| -------------------------------- | ----- | ------------------------------------------------------------------------------ |
| Manifest format (`manifest.yaml`)| Schema| Ordered change list with dependencies and per-change status                    |
| `manifest.rs` in `specify-change`| Lib   | Parsing, validation, state machine, dependency graph, consistency checks       |
| `specify manifest validate`      | CLI   | Cycle detection, referential integrity, duplicate names, consistency check     |
| `specify manifest next`          | CLI   | Return the next pending change (respecting `depends-on`, single in-progress)   |
| `specify manifest status`        | CLI   | Initiative progress in dependency order: counts, blockers, next eligible       |
| `specify manifest transition`    | CLI   | Validated status transitions with state machine enforcement                    |


### Layer 2 (Orchestrator)


| Capability                       | Type  | Notes                                                                          |
| -------------------------------- | ----- | ------------------------------------------------------------------------------ |
| Manifest orchestrator            | Skill | Automated define → build → merge loop. See [orchestrator addendum](rfc-2-orchestrator.md) |
| Skill invocation model           | Design| How one skill programmatically invokes another and interprets the result        |
| `sources` execution wiring       | Skill | Orchestrator resolves source paths and passes them to define/extract           |
| `affects` execution wiring       | Skill | Orchestrator passes affected capability names to define for delta targeting    |
| `journal.yaml`                   | Schema| Structured question/failure recording per change for autonomous operation      |


## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; manifest subcommands extend the CLI
- [RFC-2 Addendum: Orchestrator Design](rfc-2-orchestrator.md) — Layer 2 orchestrator specification
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — provides federation resolution for manifests that span repositories
