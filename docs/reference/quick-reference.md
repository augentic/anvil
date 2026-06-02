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
| `/spec:init`             | One-time project setup; run `specrun init --workspace` for a registry-only workspace root               |
| `/spec:plan`             | Survey sources, propose `slices[]`, exit at Gate 1                                          |
| `/spec:execute`          | Drive the per-slice refine → build → merge loop                                                |
| `/spec:finalize`         | Push branches, observe PR state, archive once every PR is `MERGED`                             |
| `/spec:refine`           | Breakout: extract per source, synthesize artifacts, transition slice to `refined`              |
| `/spec:build`            | Breakout: validate artifacts, implement tasks                                                  |
| `/spec:merge`            | Breakout: apply deltas to baseline, archive slice, stamp per-entry `done`                      |
| `/spec:drop`             | Discard a slice without merging                                                                |

## Artifacts

| Artifact            | Question                            | Location                                                          |
| ------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| `change.md`         | Why is the change happening?        | `.specify/change.md` (workspace mode: at workspace root)          |
| `plan.yaml`         | Which slices, in what order?        | `.specify/plan.yaml`                                              |
| `discovery.md`      | What leads did sources surface? | `.specify/discovery.md`                                           |
| `proposal.md`       | Why does this slice exist?          | `.specify/slices/<name>/proposal.md`                              |
| `spec.md`           | What must the system do?            | `.specify/slices/<name>/specs/<unit>/spec.md`                     |
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
specrun init <target>                                    # single-project scaffold (positional target adapter)
specrun init --workspace                                       # registry-only workspace root
specrun source resolve <name>                            # validate a source adapter manifest
specrun target resolve <value>                           # validate a target adapter (name, path, or URL)

# Plan management
specrun plan create <plan-name> --source <key>=<adapter>:<path>     # or <adapter>:value:<literal>
specrun plan propose --dry-run --format json                   # request the flat lead catalog + project topology
specrun plan propose --from <response.json>                    # default slice writer after survey
specrun plan add <entry> --sources <key>=<lead> --project <name>
specrun plan amend <entry> --add-source <key>=<lead> --remove-source <key> --divergence accepted
specrun plan remove <entry>                                  # Gate 1 deferral (replaceable plan only)
specrun plan transition <plan-name> approved                 # Gate 1; operator-only
specrun plan next                                        # active in-progress, or pick next pending
specrun plan archive

# Slice management
specrun slice create <name> --target <target>
specrun slice transition <name> <refining|refined|built|dropped> [--reason "..."]
specrun slice validate <name>
specrun slice merge <name> [--dry-run|--check-only]

# Workspace (multi-repo)
specrun workspace sync [<project>...]                    # materialise slots from registry.yaml
specrun workspace prepare <project> --change <name>
specrun workspace push [<project>...]                    # publish specify/<name> branch as PR

# Tools
specrun tool run <name> [args...]                        # run a declared WASI tool

# Maintenance & bootstrap
specrun upgrade [--channel cargo|brew|binary] [--dry-run|--yes]  # channel-aware CLI self-update
specrun plugins doctor [--marketplace <path>] [--format json]    # read-only Cursor plugin-cache drift report
specrun plugins refresh --yes                            # clear the plugin cache; restart Cursor to repopulate
specrun migrate [--from <X.Y.Z>] [--to <X.Y.Z>] [--dry-run|--yes]  # run registered migrators
specrun init --upgrade                                   # bump specify_version + re-scaffold preservation-safe files only
specrun init --check-migration --format json             # read-only major-version probe (needs-migration bool + plan)
```

`specrun migrate --to` pins `specify_version` **verbatim** to the requested target (not necessarily the running binary). A pin on a major older than the binary is exit `4` (migrate up); a pin newer than the binary is exit `3` (upgrade the binary first). For `specrun upgrade`, the `cargo` and `brew` channels are fully wired; the `binary`-channel self-replace is deferred, so that channel prints planned-action plus manual-upgrade guidance. `specrun plugins doctor` never exits non-zero on drift — drift is a finding (`ok | drifted | present | missing | extra`).

## Adapters

Target adapters live under `adapters/targets/<name>/`:

| Adapter   | URL                                                       | Target                |
| --------- | --------------------------------------------------------- | --------------------- |
| Omnia     | `https://github.com/augentic/specify/adapters/targets/omnia`       | Rust WASM             |
| Vectis    | `https://github.com/augentic/specify/adapters/targets/vectis`      | Crux cross-platform   |
| Contracts | `https://github.com/augentic/specify/adapters/targets/contracts`   | API contracts         |

First-party source adapters live under `adapters/sources/<name>/`: `intent`, `documentation`, `code-typescript`, `screenshots`.

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
    ├── plan.lock         # advisory file lock for /spec:execute and breakouts
    ├── .cache/           # cached adapter manifests + briefs ({sources,targets}/)
    ├── slices/           # active slices (proposal/spec/design/tasks + evidence/)
    ├── specs/            # merged baseline
    ├── workspace/        # workspace slots (workspace mode only)
    └── archive/          # finalized plans and merged or dropped slices
```

## Install

```bash
brew install augentic/tap/specify
```
