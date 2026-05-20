# Directory Layout

Specify draws a clear boundary between **operator-facing platform artifacts**, generated repo context, and **framework-managed workflow state**. Operator artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at your project root so they are visible as ordinary repository artifacts and review well in PRs. `AGENTS.md` also lives at the root; Specify owns only its fenced generated block, leaving operator prose outside the fences untouched. Framework state — caches, working slices, baseline specs, archive, registry workspace slots, the advisory plan lock, and context lock — lives under `.specify/`.

## Tree overview

```text
registry.yaml                               # Platform catalogue (optional, multi-repo only)
plan.yaml                                   # Change plan (optional, created by /change:draft)
change.md                                  # Operator brief (optional)
AGENTS.md                                  # Generated agent context with operator prose outside fences

contracts/                                  # Baseline API contracts
├── schemas/                                # JSON Schema payload definitions
│   └── <type>.yaml
├── http/                                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── <domain>-api.yaml
└── messages/                               # AsyncAPI 3.0 bindings (when messaging is used)
    └── <domain>-events.yaml

.specify/
├── project.yaml                            # Project configuration (adapter, domain, rules) — `adapter:` is omitted on hubs (where `hub: true` is the sentinel)
├── context.lock                            # Fingerprint sidecar for `specify context check`
├── plan.lock                               # Advisory lock held by /change:execute
│
├── .cache/                                 # Cached adapter manifest and brief files
│   └── <adapter>/
│       ├── adapter.yaml
│       └── briefs/
│           ├── proposal.md
│           ├── specs.md
│           ├── composition.md             # Vectis only
│           ├── design.md
│           ├── tasks.md
│           ├── build.md
│           └── merge.md
├── slices/                                 # Active slices (one directory per slice)
│   └── <slice-name>/
│       ├── .metadata.yaml                 # Lifecycle state (managed by CLI)
│       ├── proposal.md                    # Why this slice exists
│       ├── composition.yaml               # Screen layout (Vectis only)
│       ├── design.md                      # Technical design
│       ├── tasks.md                       # Implementation checklist
│       ├── specs/                         # Behavioral specs (one per adapter)
│       │   └── <adapter>/
│       │       └── spec.md
│       └── contracts/                     # Per-slice contract delta (when API interactions exist)
│           ├── schemas/
│           ├── http/
│           └── messages/
│
├── specs/                                 # Merged baseline specs
│   ├── composition.yaml                   # Baseline screen layout (Vectis only)
│   ├── .composition-checksums.yaml        # Per-screen hashes for conflict detection
│   └── <adapter>/
│       └── spec.md                        # Accumulated behavioral requirements
│
├── plans/                                 # Change-draft working directories
│   └── <plan-name>/
│       ├── discovery.md                   # Adapter inventory from /change:analyze
│       ├── proposal.md                    # Slice accept/edit/reject audit trail
│       ├── workspace.md                   # Peer inventory (multi-repo only)
│       └── analyze/
│           └── <source-key>/
│               └── metadata.json          # Source-tree structural metadata
│
├── workspace/                             # Registry workspace slots (multi-repo only)
│   └── <project-name>/
│       └── ...                            # Peer repo clone or symlink, writable during execution
│
└── archive/                               # Finalized slices and plans
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

### `slices/`

Each active slice gets its own directory under `slices/`. The directory name is kebab-case and validated by the CLI when you run `specify slice create`.

A slice directory contains the core artifacts plus `.metadata.yaml` for lifecycle state. The `specs/` subdirectory holds one spec file per adapter. Vectis changes also include `composition.yaml` for screen layout.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

Contracts are a platform concern -- they describe interfaces *between* components, not internals of any one project. Both producer and consumer reference the same central definitions. When a slice is merged, its `contracts/` files are copied here using opaque file replacement.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces -- not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. For Vectis projects, composition deltas are also merged into a baseline `composition.yaml` in this directory, with per-screen checksums tracked in `.composition-checksums.yaml` for conflict detection. Baseline specs represent the current known state of the system -- what has been specified and implemented so far.

### `.cache/`

Adapter manifest and brief files fetched at `/spec:init` time. These are read by phase skills during define and build. The cache is populated once per adapter version and updated when you re-init with a different adapter identifier or ref. The active adapter is named by the project's `adapter` field in `project.yaml`; on a hub the field is omitted (and `.cache/` is not scaffolded).

### `plans/`

Working directories for change-draft authoring. Each plan gets a subdirectory containing the discovery output, proposal audit trail, and optional workspace inventory.

### `workspace/`

Registry workspace slots for multi-repo changes. Created or refreshed by `specify workspace sync`: remote URLs become Git clones and local paths (`.` or repo-relative URLs) become symlinks. With selectors, `workspace sync` materialises only the selected slots; with no selectors, it syncs every registry project.

Slots are read-only during drafting (`/change:draft`) and writable during execution (`/change:execute`). Before mutation, execution prepares the selected remote-backed slot on `specify/<change-name>` from `origin/HEAD`; humans normally inspect that state with `specify workspace status`. Committed changes are published explicitly via `specify workspace push`, which only transports an existing exact change branch and creates or updates PRs. PR merge is operator-owned through the forge; `specify change finalize` (invoked by `/change:finalize`) later verifies the merge state and may remove clean clones under `--clean`.

### `archive/`

Terminal storage for merged slices, dropped slices, and archived plans. Preserves the full directory for audit. Nothing in `archive/` is read by the active workflow -- it exists for traceability.

## Files that do not live under `.specify/`

Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`), generated context (`AGENTS.md`), and source code generated by `/spec:build` (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Specify owns `.specify/` and the fenced generated block in `AGENTS.md`; everything else is yours. The boundary makes the responsibilities explicit:

- **Operator artifacts** — durable, PR-reviewed, often hand-edited. Live at the root so reviewers see them at a glance and tooling can reference them with short, stable paths.
- **Generated context** — `AGENTS.md` is root-level guidance for agents. `specify context generate` refreshes the fenced block, and `.specify/context.lock` records the input fingerprint for `specify context check`.
- **Framework state** — CLI-managed, frequently mutated, sometimes ephemeral. Lives under `.specify/` so the dot-prefix signals "framework owns this".

See [Decision Log: Platform artifacts at the repo root](../explanation/decision-log.md) for the reasoning.
