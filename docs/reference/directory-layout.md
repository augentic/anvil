# Directory Layout

Emery draws a clear boundary between **operator-facing platform artifacts**, generated repo context, and **framework-managed workflow state**. Operator artifacts (`registry.yaml`, the change-level `change.md` / `plan.yaml` / `discovery.md`, and `contracts/`) live at your project root so they are visible as ordinary repository artifacts and review well in PRs. `AGENTS.md` also lives at the root; Emery owns only its fenced generated block.

`.emery/` is **Emery's directory: committed configuration plus the system-of-record** — project config, every active slice, the baseline specs, the archive, and the journal. Its lone gitignored in-tree tenant is `.emery/scratch/`. Everything regenerable and machine-owned lives *outside* the working tree: the adapter/codex **cache** in a per-project directory under the Emery home (`$EMERY_HOME`, default `~/.emery`), and materialised **workspace slots** at the top-level `workspace/`.

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

.emery/
├── project.yaml                            # Project configuration (target, sources, workspace, emery-version)
├── context.lock                            # Sidecar for init-time AGENTS.md scaffold
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
    ├── YYYY-MM-DD-<slice-name>/            # Merged or dropped slices (prune via `emery archive prune`)
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

The regenerable **cache** lives outside the working tree, in a per-project directory under the Emery home (keyed by a digest of the project path):

```text
$EMERY_HOME/cache/<project-id>/                 # (default home: ~/.emery)
└── components/                                   # Project component cache
    ├── <name>.wasm                               # Seeded adapter component (adapter add / local init)
    ├── <name>.wasm.metadata.json                 # Digest-keyed describe answer sidecar
    └── <name>.meta.yaml                          # Per-component mirror provenance sidecar
```

## Key directories

### `slices/`

Each active slice gets its own directory under `slices/`. The directory name is kebab-case and validated by the `emery slice refine` orchestration, which mints the directory (re-entry safe) immediately before per-source `extract`. `emery plan add` does not create the slice directory — before execution the slice tree is empty regardless of slice count.

A slice directory contains the canonical artifacts (`proposal.md`, `design.md`, `tasks.md`, plus per-domain `specs/<domain>/spec.md`), the structured `model.yaml` synthesis artifact (carries provenance inline on each requirement; the synthesis persist tail inside `emery slice refine` / `emery plan execute` is its only writer and `spec.md` remains the authoritative artifact — `model.yaml` is audit-only), the per-source `evidence/<source>.yaml` files, and `metadata.yaml` for lifecycle state. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation alongside implementation code, not by core synthesis.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces — not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. Baseline specs represent the current known state of the system.

### Cache (out-of-tree)

The memoization root lives outside the working tree, in a per-project directory under the Emery home — `$EMERY_HOME/cache/<project-id>/`, defaulting to `~/.emery/cache/<project-id>/` (`<temp>/emery/cache/...` when no home is available) — keyed by a stable digest of the canonicalised project path. The same home carries the global adapter store at `$EMERY_HOME/store/`; `EMERY_HOME` is the single relocation override for both. Every subtree is keyed by content or version, so deleting it costs recomputation only, and because it is out-of-tree it survives `git clean` and never pollutes the working tree (each checkout, including each workspace slot, gets its own collision-free cache). `components/` is the project component cache — the single probe for bare (unpinned) adapter names, seeded by `emery adapter add <path.wasm>` or an operator-supplied local component at `emery init` (`<name>.wasm`, with per-component provenance stamped at `components/<name>.meta.yaml`); pinned identities resolve from the global store, so they are not mirrored here. There is no extraction-result cache: `survey` / `extract` are agent-run and re-execute the prompt every time, with the journal's completion events as the audit trail.

### `scratch/`

The transient working-state root and the lone gitignored tenant *inside* `.emery/` — per-run lanes recreated empty by their owning verb, so the tree can be wiped at any time at zero cost. Per-run lanes are recreated empty by their owning verb. Because the cache is out-of-tree, "a scratch write never pollutes a cache artifact" is structural rather than conventional.

### `workspace/` (top-level)

Workspace slots for multi-repo changes are materialised at the project root (not under `.emery/`) and gitignored. The operator or surrounding repository automation creates each `workspace/<project>/` path from the matching `registry.yaml` entry, using an ordinary checkout/worktree for a remote repository or a symlink for a local path.

Slots are read-only during planning and writable during execution. Branch creation, checkout, commits, publication, pull requests, and merges are operator-owned repository operations outside Emery. After publication is complete, `/emery:finalize` verifies the plan is drained and runs `emery plan archive`.

### `archive/`

A **prunable convenience cache** of merged slices, dropped slices, and archived plans — not the system of record. Nothing in `archive/` is read by the active workflow. The durable record of merged work is git history of the committed `.emery/specs/` baseline plus the append-only **outcome ledger** in `journal.jsonl` (one `slice.archive.created` entry per merge, carrying the slice name, touched baseline specs, a one-line outcome summary, and the git SHA the baseline sat at). Because the ledger and baseline already capture history, `archive/` folders can be reclaimed at will with `emery archive prune --keep <n>` / `--older-than <days>` (add `--dry-run` to preview); a folder is pruned when it falls outside any supplied retention bound.

## Files that do not live under `.emery/`

Operator-facing platform artifacts (`registry.yaml`, the change-level `change.md` / `plan.yaml` / `discovery.md`, `contracts/`), generated context (`AGENTS.md`), and source code generated by `/emery:build` (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Emery owns `.emery/` and the fenced generated block in `AGENTS.md`; everything else is yours.

In **workspace mode** (`project.yaml: workspace: true`), the registry sits at the workspace and per-project slots live at the top-level `workspace/<project>/`. Each slot carries its own `.emery/slices/<name>/` tree; the workspace's own `.emery/slices/` is unused. Plan artifacts (`change.md`, `plan.yaml`, `discovery.md`) live at the workspace root.
