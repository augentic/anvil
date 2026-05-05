# Quick Reference

## The two loops

### Single change

```text
/spec:init                     # one-time setup
/spec:define "description"     # generate artifacts
/spec:build                    # implement tasks
/spec:merge                    # merge specs (+ composition) into baseline
```

### Multi-change initiative

```text
/spec:plan <name> --source legacy=./path    # author plan
/spec:execute --loop                        # run until done
```

### Cross-repo umbrella

```text
/spec:init --hub                                     # bootstrap a platform hub
specify registry add <project> --url ... --schema ...      # `--schema` flag is the legacy spelling; renamed in a later phase
/spec:plan --orchestrate <name> [--shape ...] [--auto-merge]
```

## All skills

| Skill | Layer | Purpose |
|-------|-------|---------|
| `/spec:init` | 2 | One-time project setup (`--hub` for a platform hub) |
| `/spec:define` | 2 | Generate artifacts for a new change |
| `/spec:build` | 2 | Implement tasks |
| `/spec:merge` | 2 | Merge completed change into baseline |
| `/spec:drop` | 2 | Discard a change |
| `/spec:extract` | 2 | Extract specs from existing code |
| `/spec:analyze` | 3 | Plan-time capability inference (invoked by `/spec:plan`) |
| `/spec:plan` | 3 | Author a multi-change plan |
| `/spec:execute` | 3 | Automate the plan loop |
| `/spec:plan --orchestrate` | 4 | Cross-repo umbrella mode: brief -> registry -> plan -> execute -> push -> merge -> finalize (was `/spec:initiative`) |
| `/contract:openapi` | -- | Author / import / verify OpenAPI 3.1 contracts (HTTP / resource APIs); intent dispatched internally |
| `/contract:asyncapi` | -- | Author / import / verify AsyncAPI 3.0 contracts (evented / pub-sub / streaming); intent dispatched internally |
| `/contract:json-schema` | -- | Author / import / verify reusable JSON Schema payloads; intent dispatched internally |

## Artifacts

| Artifact | Question | Location |
|----------|----------|----------|
| `proposal.md` | Why? | `.specify/changes/<name>/proposal.md` |
| `spec.md` | What? | `.specify/changes/<name>/specs/<cap>/spec.md` |
| `contracts/**/*.yaml` | Shape? | `contracts/` (baseline) or `.specify/changes/<name>/contracts/` (delta) |
| `composition.yaml` | Where? (Vectis) | `.specify/changes/<name>/composition.yaml` |
| `design.md` | How? | `.specify/changes/<name>/design.md` |
| `tasks.md` | Sequence? | `.specify/changes/<name>/tasks.md` |

## Lifecycle states

```
created --> defining --> defined --> building --> complete --> merged
    \          \           \           \            \
     `-------->  `--------->  `--------->  `--------->  `-----> dropped
```

`defining` and `building` are transient states indicating a phase is in-flight.

## Key CLI commands

```bash
# Status
specify status                            # project dashboard
specify change status <name>              # single-change view

# Project setup
specify init <capability>                 # regular single-project scaffold (positional capability identifier or URL)
specify init --hub                        # registry-only platform hub (RFC-9 1D)

# Change management
specify change list
specify change transition <name> <target>

# Plan management
specify plan status
specify plan next
specify plan doctor                       # validate + cycle / orphan / stale-clone / unreachable
specify plan transition <name> <target>
specify plan lock status

# Workspace (multi-repo)
specify workspace sync
specify workspace status
specify workspace push
specify workspace merge                   # squash-merge PRs once CI is green (RFC-9 4A)

# Platform registry (multi-repo)
specify registry show
specify registry validate
specify registry add <name> --url <url> --schema <schema> --description "..."
specify registry remove <name>

# Initiative brief and closure
specify initiative create <name>          # scaffold initiative.md
specify initiative show
specify initiative finalize               # confirm PRs merged, archive plan (RFC-9 4C)
specify initiative finalize --clean       # also prune .specify/workspace/<peer>/

# Plan authoring (multi-repo)
specify plan add <name> --project <project>
specify plan amend <name> --project <project>

# Per-change inspection
specify change validate <name>
specify change task progress <name>
specify change merge preview <name>
specify change merge conflict-check <name>
```

## Capabilities

| Capability | URL | Target |
|------------|-----|--------|
| Omnia | `https://github.com/augentic/specify/capabilities/omnia` | Rust WASM |
| Vectis | `https://github.com/augentic/specify/capabilities/vectis` | Crux cross-platform |
| Contracts | `https://github.com/augentic/specify/capabilities/contracts` | API contracts |

## Directory structure

```
<project-root>/
├── registry.yaml         # platform catalogue (optional, multi-repo)
├── plan.yaml             # initiative plan (optional)
├── initiative.md         # operator brief (optional)
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
└── .specify/
    ├── project.yaml      # project config
    ├── plan.lock         # advisory lock for /spec:execute
    ├── .cache/           # cached capability manifest + briefs
    ├── changes/          # active changes (contracts/, composition.yaml for Vectis)
    ├── specs/            # merged baseline (incl. composition.yaml for Vectis)
    ├── plans/            # initiative working dirs (discovery, proposal)
    ├── workspace/        # peer repo clones (multi-repo only)
    └── archive/          # finalized changes and plans
```

The `0.2.0` v2 layout split operator-facing platform artifacts (root) from framework-managed state (`.specify/`); v1-layout projects upgrade with [`specify migrate v2-layout`](cli/migrate.md).

## Install

```bash
brew install augentic/tap/specify
```
