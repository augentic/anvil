# Quick Reference

## The two loops

### Single Slice

```text
/spec:init                     # one-time setup
/spec:define "description"     # generate artifacts
/spec:build                    # implement tasks
/spec:merge                    # merge specs (+ composition) into baseline
```

### Multi-slice change

```text
/change:draft <name> source legacy=./path   # author plan, stop at hand-off
# operator reviews plan.yaml (specify plan amend / status as needed)
/change:execute loop                        # run until done
/change:finalize <name>                     # push, observe PRs, archive
```

### Cross-repo bootstrap

```text
/spec:init hub                                     # bootstrap a platform hub
specify registry add <project> --url ... --capability ...
/change:draft <name> source legacy=./path          # author the cross-repo plan
# then proceed through /change:execute loop and /change:finalize <name> as above
```

## All skills

| Skill | Purpose |
|-------|---------|
| `/spec:init` | One-time project setup (`hub` for a platform hub) |
| `/spec:define` | Generate artifacts for a new slice |
| `/spec:build` | Implement tasks |
| `/spec:merge` | Merge completed slice into baseline |
| `/spec:drop` | Discard a slice |
| `/spec:extract` | Extract specs from existing code |
| `/change:analyze` | Plan-time capability inference (invoked by `/change:draft`) |
| `/change:draft` | Author a multi-slice plan; stop at the operator review seam |
| `/change:execute` | Automate the plan loop |
| `/change:finalize` | Push branches, observe PR state, run `specify change finalize` once every PR is `MERGED` |
| `/contract:openapi` | Author / import / verify OpenAPI 3.1 contracts (HTTP / resource APIs); intent dispatched internally |
| `/contract:asyncapi` | Author / import / verify AsyncAPI 3.0 contracts (evented / pub-sub / streaming); intent dispatched internally |
| `/contract:json-schema` | Author / import / verify reusable JSON Schema payloads; intent dispatched internally |

## Artifacts

| Artifact | Question | Location |
|----------|----------|----------|
| `proposal.md` | Why? | `.specify/slices/<name>/proposal.md` |
| `spec.md` | What? | `.specify/slices/<name>/specs/<cap>/spec.md` |
| `contracts/**/*.yaml` | Shape? | `contracts/` (baseline) or `.specify/slices/<name>/contracts/` (delta) |
| `composition.yaml` | Where? (Vectis) | `.specify/slices/<name>/composition.yaml` |
| `design.md` | How? | `.specify/slices/<name>/design.md` |
| `tasks.md` | Sequence? | `.specify/slices/<name>/tasks.md` |

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
specify slice status <name>              # single-slice view

# Project setup
specify init <capability>                 # regular single-project scaffold (positional capability identifier or URL)
specify init --hub                        # registry-only platform hub
specify context generate                  # write or refresh generated AGENTS.md guidance
specify context generate --check          # CI dry-run; exit 1 when context would change
specify context check                     # report stale AGENTS.md/context.lock state

# Slice management
specify status                                   # render every active slice
specify slice transition <name> <target>

# Plan management
specify plan status
specify plan next
specify plan validate                     # base shape + cycle / orphan / stale-clone / unreachable
specify plan transition <name> <target>
specify plan lock status

# Workspace (multi-repo)
specify workspace sync [<project>...]      # omit selectors to sync all registry projects
specify workspace status [<project>...]    # slot path/type, target, origin, branch, HEAD, dirty, slices
specify workspace push [<project>...]      # transport existing specify/<change-name> branch, create/update PR

# Platform registry (multi-repo)
specify registry show
specify registry validate
specify registry add <name> --url <url> --capability <schema> --description "..."
specify registry remove <name>

# Change brief and closure
specify change draft <name>          # scaffold change.md + plan.yaml
specify change show
specify change finalize              # verify merged PRs, archive plan
specify change finalize --clean      # also remove clean .specify/workspace/<peer>/ clones

# Plan authoring (multi-repo)
specify plan add <name> --project <project>
specify plan amend <name> --project <project>

# Per-slice inspection
specify slice validate <name>
specify slice task progress <name>
specify slice merge preview <name>
specify slice merge conflict-check <name>
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
├── AGENTS.md             # generated agent guidance with operator-editable prose outside fences
├── registry.yaml         # platform catalogue (optional, multi-repo)
├── plan.yaml             # change plan (optional)
├── change.md             # operator brief (optional)
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
└── .specify/
    ├── project.yaml      # project config
    ├── context.lock      # fingerprint for specify context check
    ├── plan.lock         # advisory lock for /change:execute
    ├── .cache/           # cached capability manifest + briefs
    ├── slices/           # active slices (contracts/, composition.yaml for Vectis)
    ├── specs/            # merged baseline (incl. composition.yaml for Vectis)
    ├── plans/            # change-draft working dirs (discovery, proposal)
    ├── workspace/        # registry workspace slots (multi-repo only)
    └── archive/          # finalized slices and plans
```

The `0.2.0` v2 layout split operator-facing platform artifacts (root) from framework-managed state (`.specify/`).

## Install

```bash
brew install augentic/tap/specify
```
