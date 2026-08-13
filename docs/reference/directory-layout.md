# Directory Layout

Emery draws a clear boundary between **durable product state**, **change-scoped workflow state**, generated repo context, and operator-owned source. Durable product state (`project.yaml`, `specs/`, `decisions/`) lives under `.emery/` and ships with the repository. Change-scoped artifacts (`plan.yaml`, `change.md`, `discovery.md`, `slices/`, `events/`, `targets/`, `archive/`) live under `.emery/change/`. `contracts/` and `AGENTS.md` stay at the project root; Emery owns only the fenced generated block in `AGENTS.md`.

`.emery/` is **Emery's directory: committed configuration plus the system-of-record**. Its lone gitignored in-tree tenant is `.emery/scratch/`. Everything regenerable and machine-owned lives *outside* the working tree, in a per-project directory under the Emery home (`$EMERY_HOME`, default `~/.emery`).

## Tree overview

```text
AGENTS.md                                   # Generated agent context with operator prose outside fences

contracts/                                  # Baseline API contracts
├── schemas/                                # JSON Schema payload definitions
│   └── <type>.yaml
├── http/                                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── <domain>-api.yaml
└── messages/                               # AsyncAPI 3.0 bindings (when messaging is used)
    └── <domain>-events.yaml

.emery/
├── project.yaml                            # Project configuration (target, sources, emery-version)
├── context.lock                            # Sidecar for init-time AGENTS.md scaffold
├── guest.lock                              # Create-exclusive marker held by guest orchestrations
│
├── scratch/                                # Transient working state (per-run lanes; wiped freely; gitignored)
│   ├── <adapter>/{survey,<slice>}/         # Per-operation agent scratch lanes ($SCRATCH_DIR)
│
├── specs/                                  # Merged baseline specs (committable; system of record)
│   └── <domain>/spec.md                    # Accumulated behavioral requirements
│
├── decisions/                              # Append-only Decision Record catalogue
│   └── DEC-NNNN-<slug>.md
│
├── design-system/                          # Shared UI catalog (opt-in; Vectis)
│
└── change/                                 # In-place change home (temporary; archived or deleted)
    ├── plan.yaml                           # Change plan (topology + slices[]; progress is projected)
    ├── change.md                           # Operator brief for the active change
    ├── discovery.md                        # Plan-time lead inventory
    ├── slices/                             # Active slices (one directory per slice)
    │   └── <slice-name>/
    │       ├── metadata.yaml               # Phase timestamps + target (managed by CLI; no stored status)
    │       ├── refinement.yaml             # Refinement manifest: exact inputs + covered output bundle (RFC-91)
    │       ├── proposal.md                 # Why this slice exists
    │       ├── model.yaml                  # Structured synthesis model (inline provenance; spec.md is authoritative)
    │       ├── design.md                   # Technical design
    │       ├── tasks.md                    # Implementation checklist
    │       ├── evidence/                   # Per-source extract output (managed by CLI)
    │       │   └── <source>.yaml
    │       ├── specs/                      # Behavioral specs (one per domain)
    │       │   └── <domain>/spec.md
    │       ├── builds/                     # Content-addressed fact-substrate build records
    │       │   └── <digest>.yaml
    │       └── contracts/                  # Per-slice contract delta (when API interactions exist)
    │           ├── schemas/
    │           ├── http/
    │           └── messages/
    ├── targets/                            # Target-wave manifests (RFC-86 D9)
    │   └── <target>/waves/<digest>.yaml
    ├── events/                             # Per-writer append-only fact logs (union via `emery journal show`)
    │   └── <writer>.jsonl
    └── archive/                            # Prunable cache of merged/dropped slices + finalized plans
        ├── YYYY-MM-DD-<slice-name>/        # Merged or dropped slices (prune via `emery archive prune`)
        └── plans/
            └── YYYYMMDD-<plan-name>/       # Archived plans
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

Each active slice gets its own directory under `slices/`. The directory name is kebab-case and validated by the `emery plan refine` drain, which mints the directory (re-entry safe) immediately before per-source `extract`. `emery plan add` does not create the slice directory — before refinement the slice tree is empty regardless of slice count.

A slice directory contains the canonical artifacts (`proposal.md`, `design.md`, `tasks.md`, plus per-domain `specs/<domain>/spec.md`), the structured `model.yaml` synthesis artifact (carries provenance inline on each requirement; the synthesis persist tail inside the `emery plan refine` drain is its only writer and `spec.md` remains the authoritative artifact — `model.yaml` is audit-only), the per-source `evidence/<source>.yaml` files, the `refinement.yaml` manifest (the canonical record of the refinement's exact inputs and complete output bundle — its content digest is the refinement digest `plan execute` covers), content-addressed `builds/<digest>.yaml` records, and `metadata.yaml` for phase timestamps (lifecycle labels project from those plus artifacts/facts). Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation alongside implementation code, not by core synthesis.

### `contracts/`

Platform-level API contracts at the repository root. Contains JSON Schema payload definitions (`schemas/`), OpenAPI 3.1 HTTP bindings (`http/`), and AsyncAPI 3.0 messaging bindings (`messages/`). Subdirectories are present only when the platform uses the corresponding transport type.

A slice directory may also contain a `contracts/` subdirectory holding the proposed additions or replacements for that change only. The slice-level directory contains only the files the slice adds or replaces — not a full copy of the baseline.

### `specs/`

The baseline. When a slice is merged, its spec deltas are applied here. Baseline specs represent the current known state of the system.

### Cache (out-of-tree)

The memoization root lives outside the working tree, in a per-project directory under the Emery home — `$EMERY_HOME/cache/<project-id>/`, defaulting to `~/.emery/cache/<project-id>/` (`<temp>/emery/cache/...` when no home is available) — keyed by a stable digest of the canonicalised project path. The same home carries the global adapter store at `$EMERY_HOME/store/`, the content-addressed code-snapshot store at `$EMERY_HOME/snapshots/` (in the shipped deployment the engine guest reaches it through `wasi:blobstore`, so its on-disk layout is owned by the omnia-backends filesystem backend; native test/lab deployments use the kernel's `FsObjects` layout under their own isolated homes — the two formats never share a home), and the disposable private build workspaces at `$EMERY_HOME/workspaces/` (RFC-87 — prepared per build, captured as a code patch, discarded after); `EMERY_HOME` is the single relocation override for all of them. Every subtree is keyed by content or version, so deleting it costs recomputation only, and because it is out-of-tree it survives `git clean` and never pollutes the working tree (each checkout gets its own collision-free cache). `components/` is the project component cache — the single probe for bare (unpinned) adapter names, seeded by `emery adapter add <path.wasm>` or an operator-supplied local component at `emery init` (`<name>.wasm`, with per-component provenance stamped at `components/<name>.meta.yaml`); pinned identities resolve from the global store, so they are not mirrored here. There is no extraction-result cache: `survey` / `extract` are agent-run and re-execute the prompt every time, with the journal's completion events as the audit trail.

### `scratch/`

The transient working-state root and the lone gitignored tenant *inside* `.emery/` — per-run lanes recreated empty by their owning verb, so the tree can be wiped at any time at zero cost. Per-run lanes are recreated empty by their owning verb. Because the cache is out-of-tree, "a scratch write never pollutes a cache artifact" is structural rather than conventional.

### `archive/`

A **prunable convenience cache** of merged slices, dropped slices, and archived plans — not the system of record. Nothing in `archive/` is read by the active workflow. The durable record of merged work is git history of the committed `.emery/specs/` baseline plus the append-only **outcome ledger** in `.emery/change/events/<writer>.jsonl` (one `slice.archive.created` entry per merge, carrying the slice name, touched baseline specs, a one-line outcome summary, and the git SHA the baseline sat at). Because the ledger and baseline already capture history, `archive/` folders can be reclaimed at will with `emery archive prune --keep <n>` / `--older-than <days>` (add `--dry-run` to preview); a folder is pruned when it falls outside any supplied retention bound.

## Files that do not live under `.emery/`

`contracts/`, generated context (`AGENTS.md`), and source code generated by the build phase (e.g. `crates/<name>/` for Omnia, `shared/src/` for Vectis) all live at the repo root, alongside the project's normal directory structure. Change-scoped artifacts live under `.emery/change/`, not at the repo root. Emery owns `.emery/` and the fenced generated block in `AGENTS.md`; everything else is yours.
