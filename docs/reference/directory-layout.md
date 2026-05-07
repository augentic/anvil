# Directory Layout

Specify draws a clear boundary between **operator-facing platform artifacts** and **framework-managed workflow state**. Operator artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at your project root so they are visible as ordinary repository artifacts and review well in PRs. Framework state — caches, working changes, baseline specs, archive, workspace clones, the advisory plan lock — lives under `.specify/`.

## Tree overview

```text
registry.yaml                               # Platform catalogue (optional, multi-repo only)
plan.yaml                                   # Initiative plan (optional, created by /change:plan)
change.md                               # Operator brief (optional)

contracts/                                  # Baseline API contracts
├── schemas/                                # JSON Schema payload definitions
│   └── <type>.yaml
├── http/                                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── <domain>-api.yaml
└── messages/                               # AsyncAPI 3.0 bindings (when messaging is used)
    └── <domain>-events.yaml

.specify/
├── project.yaml                            # Project configuration (capability, domain, rules) — `capability:` is omitted on hubs (where `hub: true` is the sentinel)
├── plan.lock                               # Advisory lock held by /change:execute
│
├── .cache/                                 # Cached capability manifest and brief files
│   └── <capability>/
│       ├── capability.yaml
│       └── briefs/
│           ├── proposal.md
│           ├── specs.md
│           ├── composition.md             # Vectis only
│           ├── design.md
│           ├── tasks.md
│           ├── build.md
│           └── merge.md
├── changes/                                # Active slices (one directory per change)
│   └── <slice-name>/
│       ├── .metadata.yaml                 # Lifecycle state (managed by CLI)
│       ├── proposal.md                    # Why this slice exists
│       ├── composition.yaml               # Screen layout (Vectis only)
│       ├── design.md                      # Technical design
│       ├── tasks.md                       # Implementation checklist
│       ├── specs/                         # Behavioral specs (one per capability)
│       │   └── <capability>/
│       │       └── spec.md
│       └── contracts/                     # Per-change contract delta (when API interactions exist)
│           ├── schemas/
│           ├── http/
│           └── messages/
│
├── specs/                                 # Merged baseline specs
│   ├── composition.yaml                   # Baseline screen layout (Vectis only)
│   ├── .composition-checksums.yaml        # Per-screen hashes for conflict detection
│   └── <capability>/
│       └── spec.md                        # Accumulated behavioral requirements
│
├── plans/                                 # Initiative working directories
│   └── <plan-name>/
│       ├── discovery.md                   # Capability inventory from /spec:analyze
│       ├── proposal.md                    # Slice accept/edit/reject audit trail
│       ├── workspace.md                   # Peer inventory (multi-repo only)
│       └── analyze/
│           └── <source-key>/
│               └── metadata.json          # Source-tree structural metadata
│
├── workspace/                             # Cloned peer repos (multi-repo only)
│   └── <project-name>/
│       └── ...                            # Peer repo clone (writable during execution)
│
└── archive/                               # Finalized changes and plans
    ├── YYYY-MM-DD-<slice-name>/          # Merged or dropped slices
    │   ├── .metadata.yaml
    │   ├── proposal.md
    │   ├── composition.yaml               # Vectis only
    │   ├── design.md
    │   ├── tasks.md
    │   └── specs/...
    └── plans/
        └── YYYYMMDD-<plan-name>/          # Archived plans
            ├── plan.yaml
            └── ...
```

## Key directories

### `changes/`

Each active slice gets its own directory under `changes/`. The directory name is kebab-case and validated by the CLI when you run `specify slice create`.

A slice directory contains the core artifacts plus `.metadata.yaml` for lifecycle state. The `specs/` subdirectory holds one spec file per capability. Vectis changes also include `composition.yaml` for screen layout.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

Contracts are a platform concern -- they describe interfaces *between* components, not internals of any one project. Both producer and consumer reference the same central definitions. When a slice is merged, its `contracts/` files are copied here using opaque file replacement.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces -- not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. For Vectis projects, composition deltas are also merged into a baseline `composition.yaml` in this directory, with per-screen checksums tracked in `.composition-checksums.yaml` for conflict detection. Baseline specs represent the current known state of the system -- what has been specified and implemented so far.

### `.cache/`

Capability manifest and brief files fetched at `/spec:init` time. These are read by phase skills during define and build. The cache is populated once per capability version and updated when you re-init with a different capability identifier or ref. The active capability is named by the project's `capability` field in `project.yaml`; on a hub the field is omitted (and `.cache/` is not scaffolded).

### `plans/`

Working directories for initiative authoring. Each plan gets a subdirectory containing the discovery output, proposal audit trail, and optional workspace inventory.

### `workspace/`

Cloned peer repositories for multi-repo initiatives. Created by `specify workspace sync`. Read-only during planning (`/change:plan`); writable during execution (`/change:execute`) -- define, build, and merge write into the clone's `.specify/` tree. Committed changes are pushed explicitly via `specify workspace push`.

### `archive/`

Terminal storage for merged slices, dropped slices, and archived plans. Preserves the full directory for audit. Nothing in `archive/` is read by the active workflow -- it exists for traceability.

## Files that do not live under `.specify/`

Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) and source code generated by `/spec:build` (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Specify owns `.specify/`; everything else is yours. The boundary makes the responsibilities explicit:

- **Operator artifacts** — durable, PR-reviewed, often hand-edited. Live at the root so reviewers see them at a glance and tooling can reference them with short, stable paths.
- **Framework state** — CLI-managed, frequently mutated, sometimes ephemeral. Lives under `.specify/` so the dot-prefix signals "framework owns this".

See [Decision Log: Platform artifacts at the repo root](../explanation/decision-log.md) for the reasoning, and [`specify migrate v2-layout`](cli/migrate.md) for the one-shot mover that upgrades a v1-layout project in place.

## v1 layout (deprecated)

Pre-`0.2.0` projects nested every operator artifact under `.specify/` (`.specify/registry.yaml`, `.specify/plan.yaml`, etc.). The CLI no longer reads that shape: `specify` errors out with the stable `legacy-layout` code (exit 1) and points the operator at `specify migrate v2-layout`, the one-shot mover that renames each present artifact in place. See [Migrating to the v2 layout](../how-to/migrate-to-v2-layout.md) for the operator-facing walkthrough.
