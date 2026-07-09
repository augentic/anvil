# Directory Layout

Specify draws a clear boundary between **operator-facing platform artifacts**, generated repo context, and **framework-managed workflow state**. Operator artifacts (`registry.yaml`, the change-level `change.md` / `plan.yaml` / `discovery.md`, and `contracts/`) live at your project root so they are visible as ordinary repository artifacts and review well in PRs. `AGENTS.md` also lives at the root; Specify owns only its fenced generated block.

`.specify/` is **Specify's directory: committed configuration plus the system-of-record** — project config, every active slice, the baseline specs, the archive, and the journal. Its lone gitignored in-tree tenant is `.specify/scratch/`. Everything regenerable and machine-owned lives *outside* the working tree: the adapter/codex **cache** in a per-project OS cache directory, and materialised **workspace slots** at the top-level `workspace/`.

## Tree overview

```text
registry.yaml                               # Workspace catalogue (workspace mode only)
change.md                                   # Operator brief for the active change
plan.yaml                                   # Change plan (lifecycle + slices[])
discovery.md                                # Plan-time lead inventory
AGENTS.md                                   # Generated agent context with operator prose outside fences

contracts/                                  # Baseline API contracts
├── schemas/                                # JSON Schema payload definitions
│   └── <type>.yaml
├── http/                                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── <domain>-api.yaml
└── messages/                               # AsyncAPI 3.0 bindings (when messaging is used)
    └── <domain>-events.yaml

workspace/                                  # Workspace slots (workspace mode only; gitignored)
└── <project-name>/                         # Git worktree (remote peer) or symlink (local peer)
    └── ...                                 # writable during execution

.specify/
├── project.yaml                            # Project configuration (target, sources, workspace, specify-version)
├── context.lock                            # Fingerprint sidecar for init-time AGENTS.md generation
├── topology.lock                           # Committed projection of member project.yaml topology (workspace mode)
├── guest.lock                              # Create-exclusive marker held by guest orchestrations
│
├── scratch/                                # Transient working state (per-run lanes; wiped freely; gitignored)
│   ├── <adapter>/{survey,<slice>}/         # Per-operation agent scratch lanes ($SCRATCH_DIR)
│
├── slices/                                 # Active slices (one directory per slice)
│   └── <slice-name>/
│       ├── metadata.yaml                  # Slice lifecycle (managed by CLI)
│       ├── proposal.md                     # Why this slice exists
│       ├── model.yaml                      # Structured synthesis model (inline provenance; spec.md is authoritative)
│       ├── design.md                       # Technical design
│       ├── tasks.md                        # Implementation checklist
│       ├── evidence/                       # Per-source extract output (managed by CLI)
│       │   └── <source>.yaml
│       ├── specs/                          # Behavioral specs (one per domain)
│       │   └── <domain>/spec.md
│       └── contracts/                      # Per-slice contract delta (when API interactions exist)
│           ├── schemas/
│           ├── http/
│           └── messages/
│
├── specs/                                  # Merged baseline specs (committable; system of record)
│   └── <domain>/spec.md                    # Accumulated behavioral requirements
│
├── journal.jsonl                           # Append-only event log; also the outcome ledger
│                                           #   (slice.archive.created: slice, touched-specs, summary, merge SHA)
│
└── archive/                                # Prunable cache of merged/dropped slices + finalized plans
    ├── YYYY-MM-DD-<slice-name>/            # Merged or dropped slices (prune via `specify archive prune`)
    │   ├── metadata.yaml
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

The regenerable **cache** lives outside the working tree, in a per-project directory inside your OS cache (keyed by a digest of the project path):

```text
$XDG_CACHE_HOME/specify/projects/<project-id>/   # (or $SPECIFY_PROJECT_CACHE)
├── components/                                   # Project component cache
│   ├── <name>.wasm                               # Local adapter component mirrored at init
│   ├── <name>.wasm.describe.json                 # Digest-keyed describe answer sidecar
│   └── component-meta.yaml                       # Component mirror provenance stamp
└── deployment/                                   # Generated deployment manifest (omnia.toml)

$XDG_CACHE_HOME/specify/mirrors/<url-id>.git      # Persistent bare mirror per remote peer URL
```

## Key directories

### `slices/`

Each active slice gets its own directory under `slices/`. The directory name is kebab-case and validated by the CLI when you run `specify slice create` (which `/spec:refine` invokes immediately before per-source `extract`). `specify plan add` does not create the slice directory — at Gate 1 the slice tree is empty regardless of slice count.

A slice directory contains the canonical artifacts (`proposal.md`, `design.md`, `tasks.md`, plus per-domain `specs/<domain>/spec.md`), the structured `model.yaml` synthesis artifact (carries provenance inline on each requirement; the synthesis persist tail inside `specify slice refine` / `specify plan execute` is its only writer and `spec.md` remains the authoritative artifact — `model.yaml` is audit-only), the per-source `evidence/<source>.yaml` files, and `metadata.yaml` for lifecycle state. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation alongside implementation code, not by core synthesis.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces — not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. Baseline specs represent the current known state of the system.

### Cache (out-of-tree)

The memoization root lives outside the working tree, in a per-project directory inside your OS cache — `$SPECIFY_PROJECT_CACHE`, else `$XDG_CACHE_HOME/specify/projects/<project-id>/`, else `~/.cache/...` — keyed by a stable digest of the canonicalised project path. Every subtree is keyed by content or version, so deleting it costs recomputation only, and because it is out-of-tree it survives `git clean` and never pollutes the working tree (each checkout, including each workspace slot, gets its own collision-free cache). `components/` is the project component cache — an operator-supplied local adapter component mirrored at `specify init` (`<name>.wasm`, with provenance stamped at `components/component-meta.yaml`); pinned identities resolve from the global store and development builds resolve live, so neither is mirrored here. `deployment/` holds the generated deployment manifest (`omnia.toml`). There is no extraction-result cache: `survey` / `extract` are agent-run and re-execute the prompt every time, with the journal's completion events as the audit trail.

### `scratch/`

The transient working-state root and the lone gitignored tenant *inside* `.specify/` — per-run lanes recreated empty by their owning verb, so the tree can be wiped at any time at zero cost. Per-run lanes are recreated empty by their owning verb. Because the cache is out-of-tree, "a scratch write never pollutes a cache artifact" is structural rather than conventional.

### `workspace/` (top-level)

Workspace slots for multi-repo changes, materialised at the project root (not under `.specify/`) and gitignored. Created or refreshed by `specify workspace sync`: remote URLs become `git worktree`s of a persistent out-of-tree bare mirror (so a peer's object store is shared across changes and fresh checkouts), and local paths (`.` or repo-relative URLs) become symlinks. With selectors, `workspace sync` materialises only the selected slots; with no selectors, it syncs every registered project.

Slots are read-only during planning and writable during execution. Before mutation, execution prepares the selected remote-backed slot on `specify/<change-name>` from `origin/HEAD`. Committed changes are published explicitly via `specify workspace push`, which only transports an existing exact change branch to `origin`. Opening and merging pull requests is operator-owned through the forge, entirely outside Specify; `/spec:finalize` runs `specify workspace push` and then `specify plan archive`.

### `archive/`

A **prunable convenience cache** of merged slices, dropped slices, and archived plans — not the system of record. Nothing in `archive/` is read by the active workflow. The durable record of merged work is git history of the committed `.specify/specs/` baseline plus the append-only **outcome ledger** in `journal.jsonl` (one `slice.archive.created` entry per merge, carrying the slice name, touched baseline specs, a one-line outcome summary, and the git SHA the baseline sat at). Because the ledger and baseline already capture history, `archive/` folders can be reclaimed at will with `specify archive prune --keep <n>` / `--older-than <days>` (add `--dry-run` to preview); a folder is pruned when it falls outside any supplied retention bound.

## Files that do not live under `.specify/`

Operator-facing platform artifacts (`registry.yaml`, the change-level `change.md` / `plan.yaml` / `discovery.md`, `contracts/`), generated context (`AGENTS.md`), and source code generated by `/spec:build` (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Specify owns `.specify/` and the fenced generated block in `AGENTS.md`; everything else is yours.

In **workspace mode** (`project.yaml: workspace: true`), the registry sits at the workspace and per-project slots live at the top-level `workspace/<project>/`. Each slot carries its own `.specify/slices/<name>/` tree; the workspace's own `.specify/slices/` is unused. Plan artifacts (`change.md`, `plan.yaml`, `discovery.md`) live at the workspace root.
