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
specify registry add <project> --url ... --schema ...
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
| `/contracts:writer` | -- | Validate spec alignment, produce contract delta |
| `/contracts:validator` | -- | Verify contract artifact consistency |
| `/contracts:importer` | -- | Import and normalise external contracts (Layer 2) |

## Artifacts

| Artifact | Question | Location |
|----------|----------|----------|
| `proposal.md` | Why? | `.specify/changes/<name>/proposal.md` |
| `spec.md` | What? | `.specify/changes/<name>/specs/<cap>/spec.md` |
| `contracts/**/*.yaml` | Shape? | `.specify/contracts/` (baseline) or `.specify/changes/<name>/contracts/` (delta) |
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
specify init <schema>                     # regular single-project scaffold
specify init hub --hub                    # registry-only platform hub (RFC-9 1D)

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
specify initiative create <name>          # scaffold .specify/initiative.md
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

## Schemas

| Schema | URL | Target |
|--------|-----|--------|
| Omnia | `https://github.com/augentic/specify/schemas/omnia` | Rust WASM |
| Vectis | `https://github.com/augentic/specify/schemas/vectis` | Crux cross-platform |
| Contracts | `https://github.com/augentic/specify/schemas/contracts` | API contracts |

## Directory structure

```
.specify/
├── project.yaml          # project config
├── plan.yaml             # initiative plan (optional)
├── registry.yaml         # multi-repo catalogue (optional)
├── initiative.md         # operator brief (optional)
├── plan.lock             # advisory lock for /spec:execute
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
├── .cache/               # cached schema + briefs
├── changes/              # active changes (contracts/, composition.yaml for Vectis)
├── specs/                # merged baseline (incl. composition.yaml for Vectis)
├── plans/                # initiative working dirs (discovery, proposal)
├── workspace/            # peer repo clones (multi-repo only)
└── archive/              # finalized changes and plans
```

## Install

```bash
brew install augentic/tap/specify
```
