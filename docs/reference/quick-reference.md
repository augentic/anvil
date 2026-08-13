# Quick Reference

## The default rhythm

<div class="pipeline">


![Default workflow poster](../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">init → plan → review → refine → review → execute → finalize; re-run the parked stage when it stops.</p>
</div>


The same rhythm runs for a one-slice change and a twelve-slice change alike. For multi-source slices, bind additional sources at plan time:

```text
/emery:plan <name> source legacy=typescript:./vendor/monolith source docs=documentation:./design-notes
```

## All skills

| Skill                    | Purpose                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| `/emery:init`             | One-time project setup                                                                         |
| `/emery:plan`             | Survey sources, propose `slices[]`, exit for operator review                                 |
| `/emery:refine`           | Drain specification refinement over the closed plan, stop before code work (`emery plan refine`) |
| `/emery:execute`          | Drive the per-slice build → merge loop (`emery plan execute` — opens the authorization epoch over the refinement digests) |
| `/emery:status`           | Report where the plan stands and the literal next command (read-only)                          |
| `/emery:finalize`         | Confirm operator-owned publication is complete, then archive the plan                         |

## Artifacts

| Artifact            | Question                            | Location                                                          |
| ------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| `change.md`         | Why is the change happening?        | `.emery/change/change.md`                                         |
| `plan.yaml`         | Which slices, in what order?        | `.emery/change/plan.yaml`                                         |
| `discovery.md`      | What leads did sources surface? | `.emery/change/discovery.md`                                      |
| `proposal.md`       | Why does this slice exist?          | `.emery/change/slices/<name>/proposal.md`                              |
| `spec.md`           | What must the system do?            | `.emery/change/slices/<name>/specs/<domain>/spec.md`                     |
| `design.md`         | How will it be implemented?         | `.emery/change/slices/<name>/design.md`                                |
| `tasks.md`          | In what sequence?                   | `.emery/change/slices/<name>/tasks.md`                                 |
| `evidence/<key>.yaml` | What did this source say?         | `.emery/change/slices/<name>/evidence/<source>.yaml`               |

## Lifecycle states (projected)

Per-entry ladder (from claims + merge/archive facts — not stored on `plan.yaml`):

```text
pending --(execute claims)--> in-progress --(merge phase)--> done
```

Slice ladder (from phase timestamps + artifacts/facts — not a `metadata.yaml` status field):

```text
refining --> refined --> built --> merged
                            \
                             `--> dropped (via emery plan drop --reason "...")
```

## Key CLI commands

```bash
# Project setup
emery init <target>                                    # single-project scaffold (positional target adapter)
emery source resolve <name>                            # resolve a source adapter and report its settled identity
emery target resolve <value>                           # validate a target adapter (name, path, or URL)

# Plan management
emery plan author <plan-name> --source <key>=<adapter>:<path>    # scaffold + survey + reconcile + validate; exits for review (--force replaces a replaceable plan)
emery plan add <entry> --sources <key>=<lead>
emery plan amend <entry> --add-source <key>=<lead> --remove-source <key> --divergence accepted
emery plan remove <entry>                                  # pre-execution deferral (replaceable plan only)
emery plan drop <entry> [--reason "..."]               # abandon a refined slice without merging
emery plan refine [--slice <slice>]...                 # specification drain: extract + synthesize + refinement.yaml per leaf
emery plan execute                                     # authorization epoch over refinement digests + drained build → merge loop
emery plan status                                      # read-only next-action + Ready/Authorized + debt counts
emery plan gaps                                        # typed gap inventory with open|deferred dispositions
emery plan archive                                     # archive the drained plan (prints the carried-debt summary)
emery debt                                             # baseline debt projection (carried unknown/conflict backlog)

# Slice projections (read-only)
emery slice list                                       # every slice with status + target
emery slice validate <name>                            # artifact validation + staleness advisories
emery slice provenance <name>                          # on-demand provenance audit view
emery slice model show <name>                          # render model.yaml

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
├── contracts/            # baseline API contracts (schemas/, http/, messages/)
└── .emery/
    ├── project.yaml      # project config (target, sources, emery-version)
    ├── guest.lock        # lock held by a running plan refine / plan execute (a second driver gets guest-marker-held)
    ├── scratch/          # transient per-run working state (gitignored)
    ├── specs/            # merged baseline
    └── change/           # in-place change home
        ├── plan.yaml     # change plan
        ├── change.md     # operator brief
        ├── discovery.md  # plan-time lead inventory
        ├── slices/       # active slices (proposal/spec/design/tasks + evidence/ + refinement.yaml + builds/)
        ├── targets/      # one-member wave manifests
        ├── events/       # per-writer fact logs (<writer>.jsonl)
        └── archive/      # finalized plans and merged or dropped slices
```

The regenerable adapter cache lives outside the working tree under the Emery home (`$EMERY_HOME/cache/<project-id>/`, default `~/.emery`). See [Directory layout](directory-layout.md) for the full tree.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh

# or: brew tap augentic/tap && brew install emery
# or: cargo binstall --git https://github.com/augentic/emery emery@<version>
# or: cargo install --git https://github.com/augentic/emery --locked
```

Platform archives also ship on the GitHub Releases page (verify against each `.sha256` companion).
