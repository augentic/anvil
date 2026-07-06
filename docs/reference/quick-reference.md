# Quick Reference

## The default rhythm

<div class="pipeline">


![Default workflow poster](../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">init → plan → Gate 1 → execute → finalize; breakouts available when execute parks.</p>
</div>


The same rhythm runs at N=1 and N=12. For multi-source slices, bind additional sources at plan time:

```text
/spec:plan <name> source legacy=./vendor/monolith source docs=./design-notes
```

## All skills

| Skill                    | Purpose                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| `/spec:init`             | One-time project setup; run `specify init --workspace` for a registry-only workspace               |
| `/spec:plan`             | Survey sources, propose `slices[]`, exit at Gate 1                                          |
| `/spec:execute`          | Drive the per-slice refine → build → merge loop                                                |
| `/spec:finalize`         | Push branches, then archive the plan (PRs are opened and merged by the operator outside Specify) |
| `/spec:refine`           | Breakout: extract per source, synthesize artifacts, transition slice to `refined`              |
| `/spec:build`            | Breakout: validate artifacts, implement tasks                                                  |
| `/spec:merge`            | Breakout: apply deltas to baseline, archive slice, stamp per-entry `done`                      |
| `/spec:drop`             | Discard a slice without merging                                                                |

## Artifacts

| Artifact            | Question                            | Location                                                          |
| ------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| `change.md`         | Why is the change happening?        | `change.md` (project root; workspace mode: at workspace)          |
| `plan.yaml`         | Which slices, in what order?        | `plan.yaml` (project root)                                        |
| `discovery.md`      | What leads did sources surface? | `discovery.md` (project root)                                     |
| `proposal.md`       | Why does this slice exist?          | `.specify/slices/<name>/proposal.md`                              |
| `spec.md`           | What must the system do?            | `.specify/slices/<name>/specs/<domain>/spec.md`                     |
| `design.md`         | How will it be implemented?         | `.specify/slices/<name>/design.md`                                |
| `tasks.md`          | In what sequence?                   | `.specify/slices/<name>/tasks.md`                                 |
| `evidence/<key>.yaml` | What did this source say?         | `.specify/slices/<name>/evidence/<source>.yaml`               |

## Lifecycle states

Plan lifecycle (two stored states):

```text
pending --(operator stamps Gate 1)--> approved
```

Per-entry status:

```text
pending --(plan next)--> in-progress --(slice merge)--> done
```

Slice lifecycle:

```text
refining --> refined --> built --> merged
                            \
                             `--> dropped (via slice transition --reason "...")
```

## Key CLI commands

```bash
# Project setup
specify init <target>                                    # single-project scaffold (positional target adapter)
specify init --workspace                                       # registry-only workspace
specify source resolve <name>                            # validate a source adapter manifest
specify target resolve <value>                           # validate a target adapter (name, path, or URL)

# Plan management
specify plan create <plan-name> --source <key>=<adapter>:<path>     # or <adapter>:value:<literal>
specify plan author                                            # survey + reconcile + validate; exits at pending
specify plan add <entry> --sources <key>=<lead> --project <name>
specify plan amend <entry> --add-source <key>=<lead> --remove-source <key> --divergence accepted
specify plan remove <entry>                                  # Gate 1 deferral (replaceable plan only)
specify plan transition <plan-name> approved                 # Gate 1; operator-only (lock-exempt)
specify plan status                                      # read-only next-action projection
specify plan next                                        # active in-progress, or pick next pending (requires the plan lock)
specify plan archive

# Slice management
specify slice create <name> --target <target>
specify slice transition <name> <refining|refined|built|dropped> [--reason "..."]
specify slice validate <name>
specify slice merge <name> [--dry-run|--check-only]

# Workspace (multi-repo)
specify workspace sync [<project>...]                    # materialise slots from registry.yaml
specify workspace prepare <project> --change <name>
specify workspace push [<project>...]                    # publish specify/<name> branch as PR

# Tools
# adapter helper tools are in-guest library code (no host dispatch verb)

# Maintenance & bootstrap
specify upgrade [--channel cargo|brew|binary] [--dry-run|--yes]  # channel-aware CLI self-update
specify plugins doctor [--marketplace <path>] [--format json]    # read-only Cursor plugin-cache drift report
specify plugins refresh --yes                            # clear the plugin cache; restart Cursor to repopulate
specify init --upgrade                                   # bump specify pin + re-scaffold preservation-safe files only
```

A pin newer than the binary is exit `3` (upgrade the binary first); an older pin loads normally — pre-1.0 majors are re-init, not migration. For `specify upgrade`, the `cargo` and `brew` channels are fully wired; the `binary`-channel self-replace is deferred, so that channel prints planned-action plus manual-upgrade guidance. `specify plugins doctor` never exits non-zero on drift — drift is a finding (`ok | drifted | present | missing | extra`).

## Adapters

Target adapters live under `adapters/targets/<name>/`:

| Adapter   | URL                                                       | Target                |
| --------- | --------------------------------------------------------- | --------------------- |
| Omnia     | `https://github.com/augentic/specify/adapters/targets/omnia`       | Rust WASM             |
| Vectis    | `https://github.com/augentic/specify/adapters/targets/vectis`      | Crux cross-platform   |
| Contracts | `https://github.com/augentic/specify/adapters/targets/contracts`   | API contracts         |

First-party source adapters live under `adapters/sources/<name>/`: `intent`, `documentation`, `typescript`, `screenshots`.

## Directory structure

```text
<project-root>/
├── AGENTS.md             # generated agent guidance with operator-editable prose outside fences
├── registry.yaml         # workspace catalogue (workspace mode only)
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
└── .specify/
    ├── project.yaml      # project config (target, sources, workspace, specify-version)
    ├── change.md         # operator brief (per active change)
    ├── plan.yaml         # change plan
    ├── discovery.md      # plan-time lead inventory
    ├── plan.lock         # advisory file lock for /spec:execute and breakouts (CLI-probed: unlocked drivers get plan-lock-not-held)
    ├── cache/           # cached adapter manifests + briefs ({sources,targets}/)
    ├── slices/           # active slices (proposal/spec/design/tasks + evidence/)
    ├── specs/            # merged baseline
    ├── workspace/        # workspace slots (workspace mode only)
    └── archive/          # finalized plans and merged or dropped slices
```

## Install

```bash
brew install augentic/tap/specify
```
