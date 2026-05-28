# Directory Layout

Specify 2.0 draws a clear boundary between **operator-facing platform artifacts**, generated repo context, and **framework-managed workflow state**. Operator artifacts (`registry.yaml`, `contracts/`) live at your project root so they are visible as ordinary repository artifacts and review well in PRs. `AGENTS.md` also lives at the root; Specify owns only its fenced generated block. The change-level artifacts (`change.md`, `plan.yaml`, `discovery.md`), every active slice, the baseline specs, and the archive live under `.specify/`.

## Tree overview

```text
registry.yaml                               # Workspace catalogue (workspace mode only)
AGENTS.md                                   # Generated agent context with operator prose outside fences

contracts/                                  # Baseline API contracts
├── schemas/                                # JSON Schema payload definitions
│   └── <type>.yaml
├── http/                                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── <domain>-api.yaml
└── messages/                               # AsyncAPI 3.0 bindings (when messaging is used)
    └── <domain>-events.yaml

.specify/
├── project.yaml                            # Project configuration (target, sources, workspace, specify-version)
├── change.md                               # Operator brief for the active change
├── plan.yaml                               # Change plan (lifecycle + slices[])
├── discovery.md                            # Plan-time lead inventory
├── context.lock                            # Fingerprint sidecar for init-time AGENTS.md generation
├── plan.lock                               # Advisory lock held by /spec:execute and breakouts
│
├── .cache/                                 # Cached adapter manifests + briefs
│   ├── adapters/sources/<name>/                     # Source adapter cache
│   │   ├── adapter.yaml
│   │   └── briefs/{survey,extract}.md
│   └── adapters/targets/<name>/                     # Target adapter cache
│       ├── adapter.yaml
│       └── briefs/{shape,build,merge}.md
│
├── slices/                                 # Active slices (one directory per slice)
│   └── <slice-name>/
│       ├── .metadata.yaml                  # Slice lifecycle (managed by CLI)
│       ├── proposal.md                     # Why this slice exists
│       ├── design.md                       # Technical design
│       ├── tasks.md                        # Implementation checklist
│       ├── evidence/                       # Per-source extract output (managed by CLI)
│       │   └── <source-key>.yaml
│       ├── specs/                          # Behavioral specs (one per unit)
│       │   └── <unit>/spec.md
│       └── contracts/                      # Per-slice contract delta (when API interactions exist)
│           ├── schemas/
│           ├── http/
│           └── messages/
│
├── specs/                                  # Merged baseline specs
│   └── <unit>/spec.md                      # Accumulated behavioral requirements
│
├── workspace/                              # Workspace slots (workspace mode only)
│   └── <project-name>/
│       └── ...                             # Project clone or symlink, writable during execution
│
└── archive/                                # Finalized plans and merged/dropped slices
    ├── YYYY-MM-DD-<slice-name>/            # Merged or dropped slices
    │   ├── .metadata.yaml
    │   ├── proposal.md
    │   ├── design.md
    │   ├── tasks.md
    │   ├── specs/...
    │   └── evidence/...
    └── plans/
        └── YYYYMMDD-<plan-name>/           # Archived plans
            ├── plan.yaml
            ├── change.md
            ├── discovery.md
            └── ...
```

## Key directories

### `slices/`

Each active slice gets its own directory under `slices/`. The directory name is kebab-case and validated by the CLI when you run `specrun slice create` (which `/spec:refine` invokes immediately before per-source `extract`). `specrun plan add` does not create the slice directory — at Gate 1 the slice tree is empty regardless of slice count.

A slice directory contains the canonical artifacts (`proposal.md`, `design.md`, `tasks.md`, plus per-unit `specs/<unit>/spec.md`), the per-source `evidence/<source-key>.yaml` files, and `.metadata.yaml` for lifecycle state. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation alongside implementation code, not by core synthesis.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces — not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. Baseline specs represent the current known state of the system.

### `.cache/`

Adapter manifests and brief files. The adapter loader (`crates/domain/src/adapter/`) routes by axis: `adapters/sources/<name>/` for source adapters and `adapters/targets/<name>/` for target adapters. The cache is populated by `specrun source resolve` and `specrun target resolve` on first use.

### `workspace/`

Workspace slots for multi-repo changes. Created or refreshed by `specrun workspace sync`: remote URLs become Git clones and local paths (`.` or repo-relative URLs) become symlinks. With selectors, `workspace sync` materialises only the selected slots; with no selectors, it syncs every registered project.

Slots are read-only during planning and writable during execution. Before mutation, execution prepares the selected remote-backed slot on `specify/<change-name>` from `origin/HEAD`. Committed changes are published explicitly via `specrun workspace push`, which only transports an existing exact change branch and creates or updates PRs. PR merge is operator-owned through the forge; `/spec:finalize` later observes the merge state with `gh pr view` before running `specrun plan archive`.

### `archive/`

Terminal storage for merged slices, dropped slices, and archived plans. Preserves the full directory for audit. Nothing in `archive/` is read by the active workflow — it exists for traceability.

## Files that do not live under `.specify/`

Operator-facing platform artifacts (`registry.yaml`, `contracts/`), generated context (`AGENTS.md`), and source code generated by `/spec:build` (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Specify owns `.specify/` and the fenced generated block in `AGENTS.md`; everything else is yours.

In **workspace mode** (`project.yaml: workspace: true`), the registry sits at the workspace root and per-project slots live under `.specify/workspace/<project>/`. Each slot carries its own `.specify/slices/<name>/` tree; the workspace's own `.specify/slices/` is unused. Plan artifacts (`change.md`, `plan.yaml`, `discovery.md`) live at the workspace root.
