# Quick Reference

## The two loops

### Single change

```text
/spec:init                     # one-time setup
/spec:define "description"     # generate artifacts
/spec:build                    # implement tasks
/spec:merge                    # merge specs into baseline
```

### Multi-change initiative

```text
/spec:plan <name> --source legacy=./path    # author plan
/spec:execute --loop                        # run until done
```

## All skills

| Skill | Purpose |
|-------|---------|
| `/spec:init` | One-time project setup |
| `/spec:define` | Generate artifacts for a new change |
| `/spec:build` | Implement tasks |
| `/spec:merge` | Merge completed specs into baseline |
| `/spec:drop` | Discard a change |
| `/spec:status` | Check progress |
| `/spec:verify` | Detect drift between code and specs |
| `/spec:explore` | Think through a problem |
| `/spec:extract` | Extract specs from existing code |
| `/spec:plan` | Author a multi-change plan |
| `/spec:execute` | Automate the plan loop |

## Artifacts

| Artifact | Question | Location |
|----------|----------|----------|
| `proposal.md` | Why? | `.specify/changes/<name>/proposal.md` |
| `spec.md` | What? | `.specify/changes/<name>/specs/<cap>/spec.md` |
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
specify status
specify status <name>

# Change management
specify change list
specify change status <name>
specify change transition <name> <target>

# Plan management
specify plan status
specify plan next
specify plan transition <name> <target>
specify plan lock status

# Workspace (multi-repo)
specify workspace sync
specify workspace status
specify workspace push

# Plan authoring (multi-repo)
specify plan create <name> --project <project>
specify plan amend <name> --project <project>

# Inspection
specify spec preview <change-dir>
specify spec conflict-check <change-dir>
specify task progress <change-dir>
specify validate <change-dir>
```

## Schemas

| Schema | URL | Target |
|--------|-----|--------|
| Omnia | `https://github.com/augentic/specify/schemas/omnia` | Rust WASM |
| Vectis | `https://github.com/augentic/specify/schemas/vectis` | Crux cross-platform |

## Directory structure

```
.specify/
├── project.yaml          # project config
├── plan.yaml             # initiative plan (optional)
├── registry.yaml         # multi-repo catalogue (optional)
├── initiative.md         # operator brief (optional)
├── plan.lock             # advisory lock for /spec:execute
├── .cache/               # cached schema + briefs
├── changes/              # active changes
├── specs/                # merged baseline
├── plans/                # initiative working dirs (discovery, proposal)
├── workspace/            # peer repo clones (multi-repo only)
└── archive/              # finalized changes and plans
```

## Install

```bash
brew install augentic/tap/specify
```
