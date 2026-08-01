# Quick Reference

## The default rhythm

<div class="pipeline">


![Default workflow poster](../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">init → plan → Gate 1 → execute → finalize; breakouts available when execute parks.</p>
</div>


The same rhythm runs at N=1 and N=12. For multi-source slices, bind additional sources at plan time:

```text
/emery:plan <name> source legacy=typescript:./vendor/monolith source docs=documentation:./design-notes
```

## All skills

| Skill                    | Purpose                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| `/emery:init`             | One-time project setup; run `emery init --workspace` for a registry-only workspace               |
| `/emery:plan`             | Survey sources, propose `slices[]`, exit at Gate 1                                          |
| `/emery:execute`          | Confirm Gate 1, then drive the per-slice refine → build → merge loop (`emery plan execute`) |
| `/emery:finalize`         | Confirm operator-owned publication is complete, then archive the plan                         |
| `/emery:refine`           | Breakout: extract per source, synthesize artifacts, transition slice to `refined`              |
| `/emery:build`            | Breakout: validate artifacts, implement tasks                                                  |
| `/emery:merge`            | Breakout: apply deltas to baseline, archive slice, stamp per-entry `done`                      |
| `/emery:drop`             | Discard a slice without merging                                                                |

## Artifacts

| Artifact            | Question                            | Location                                                          |
| ------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| `change.md`         | Why is the change happening?        | `change.md` (project root; workspace mode: at workspace)          |
| `plan.yaml`         | Which slices, in what order?        | `plan.yaml` (project root)                                        |
| `discovery.md`      | What leads did sources surface? | `discovery.md` (project root)                                     |
| `proposal.md`       | Why does this slice exist?          | `.emery/slices/<name>/proposal.md`                              |
| `spec.md`           | What must the system do?            | `.emery/slices/<name>/specs/<domain>/spec.md`                     |
| `design.md`         | How will it be implemented?         | `.emery/slices/<name>/design.md`                                |
| `tasks.md`          | In what sequence?                   | `.emery/slices/<name>/tasks.md`                                 |
| `evidence/<key>.yaml` | What did this source say?         | `.emery/slices/<name>/evidence/<source>.yaml`               |

## Lifecycle states

Plan lifecycle (two stored states):

```text
pending --(first plan execute stamps Gate 1)--> approved
```

Per-entry status:

```text
pending --(plan next)--> in-progress --(slice merge)--> done
```

Slice lifecycle:

```text
refining --> refined --> built --> merged
                            \
                             `--> dropped (via slice drop --reason "...")
```

## Key CLI commands

```bash
# Project setup
emery init <target>                                    # single-project scaffold (positional target adapter)
emery init --workspace                                       # registry-only workspace
emery source resolve <name>                            # resolve a source adapter and report its settled identity
emery target resolve <value>                           # validate a target adapter (name, path, or URL)

# Plan management
emery plan author <plan-name> --source <key>=<adapter>:<path>    # scaffold + survey + reconcile + validate; exits at pending (--force replaces a pending plan)
emery plan add <entry> --sources <key>=<lead> --project <name>
emery plan amend <entry> --add-source <key>=<lead> --remove-source <key> --divergence accepted
emery plan remove <entry>                                  # Gate 1 deferral (replaceable plan only)
emery plan execute                                     # Gate 1 on first run (stamps approved) + the drained loop
emery plan status                                      # read-only next-action projection
emery plan next                                        # active in-progress, or pick next pending
emery plan archive

# Slice management
emery slice list                                       # read-only: every slice with status + target
emery slice refine <name>                              # guest-routed: create + extract + synthesis + refined
emery slice validate <name>
emery slice merge <name>                           # dry-runs: --preview / --conflict-check
emery slice drop <name> [--reason "..."]

# Maintenance & bootstrap
emery init --upgrade                                   # bump emery pin + re-scaffold preservation-safe files only
```

A pin newer than the binary is exit `3` (update the binary through its install channel first); an older pin loads normally — pre-1.0 majors are re-init, not migration. `emery init --upgrade` updates the project pin and preservation-safe scaffold, not the installed CLI.

## Adapters

First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). Target adapters live under `targets/<name>/`:

| Adapter   | URL                                                                | Target                |
| --------- | ------------------------------------------------------------------ | --------------------- |
| Omnia     | `https://github.com/augentic/emery-adapters/tree/main/targets/omnia`     | Rust WASM             |
| Vectis    | `https://github.com/augentic/emery-adapters/tree/main/targets/vectis`    | Crux cross-platform   |
| Contracts | `https://github.com/augentic/emery-adapters/tree/main/targets/contracts` | API contracts         |

First-party source adapters live under `sources/<name>/`: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.

## Directory structure

```text
<project-root>/
├── AGENTS.md             # generated agent guidance with operator-editable prose outside fences
├── registry.yaml         # workspace catalogue (workspace mode only)
├── change.md             # operator brief (per active change)
├── plan.yaml             # change plan
├── discovery.md          # plan-time lead inventory
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
├── workspace/            # workspace slots (workspace mode only; gitignored)
└── .emery/
    ├── project.yaml      # project config (target, sources, workspace, emery-version)
    ├── guest.lock        # create-exclusive marker held by guest orchestrations (second driver gets guest-marker-held)
    ├── scratch/          # transient per-run working state (gitignored)
    ├── slices/           # active slices (proposal/spec/design/tasks + evidence/)
    ├── specs/            # merged baseline
    ├── journal.jsonl     # append-only event log and outcome ledger
    └── archive/          # finalized plans and merged or dropped slices
```

The regenerable adapter cache lives outside the working tree under the Emery home (`$EMERY_HOME/cache/<project-id>/`, default `~/.emery`). See [Directory layout](directory-layout.md) for the full tree.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh

# or: brew tap augentic/tap && brew install emery
# or: cargo binstall --git https://github.com/augentic/emery emery@0.32.0
# or: cargo install --git https://github.com/augentic/emery --locked
```

Platform archives also ship on the GitHub Releases page (verify against each `.sha256` companion).
