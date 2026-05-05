# Change Component

> Status: Draft (Phase 2.10 of [RFC-13](../../rfcs/rfc-13-extensibility.md) landed). The change component is a first-party Specify component, not a capability — it coordinates an operator outcome from brief through executable plan, execution state, and close-out.

## What is the change component?

The change component (today implemented in the `specify-initiative` crate; renamed to `specify-change` in Phase 3.4 of the RFC-13 plan) coordinates an operator outcome from **brief** (`change.md`) through **executable plan** (`plan.yaml`), **execution state** (`.specify/changes/<name>/.metadata.yaml`), and **close-out** (`specify change finalize` and `specify change archive`). It consumes registry project ids, materialised project paths, and core slice phase outcomes — but it does not own any of those.

The change component is **not** a capability: it has commands, libraries, and files, but it does not appear in any `capability.yaml`, it is not activated through the manifest protocol, and the core never switches on a capability name to invoke it. See [RFC-13 §"Platform components are not capabilities"](../../rfcs/rfc-13-extensibility.md#platform-components-are-not-capabilities) and [RFC-13 §"Cross-capability coordination"](../../rfcs/rfc-13-extensibility.md#cross-capability-coordination).

## Files and state

| Path                                   | Owner             | Purpose                                                                                                                                                                                  |
| -------------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `change.md`                            | operator          | Operator-authored brief at the repo root: desired outcome, scope, impacted projects, close-out criteria. (Currently named `initiative.md` until Phase 3 ships `specify migrate change-noun`.) |
| `plan.yaml`                            | change skills + operator | Executable dependency-aware plan at the repo root: sequenced slice entries, per-entry scope (project, capability, sources), and lifecycle status.                                  |
| `.specify/changes/<name>/.metadata.yaml` | core (slice loop) | Per-slice phase outcomes (`define`, `build`, `merge`) consumed by the plan. (`.specify/changes/` is renamed to `.specify/slices/` by Phase 3's `specify migrate slice-layout`.)             |
| `.specify/changes/<name>/journal.yaml` | core (slice loop) | Append-only audit log per slice. Capability merge briefs append `failure` and `recovery` entries here for the change component to surface.                                               |
| `.specify/plan.lock`                   | change component  | Advisory PID stamp serialising concurrent `/spec:execute` drivers (RFC-2 §"Driver Concurrency").                                                                                         |
| `.specify/archive/plans/<YYYYMMDD>-<name>/` | change component | Archive destination written atomically by `specify change finalize` once every per-project PR has merged.                                                                                |

## Verbs

The change component publishes the **post-Phase-3** umbrella surface below. The `Plan` family is folded under `Change` so planning is a subresource — `plan.yaml` is the file name, but it is not a peer top-level CLI noun.

> **Today's branch ships these under `specify initiative *` and `specify plan *`; Phase 3.5 of the RFC-13 plan folds them under `specify change *`.** Read every `specify change` verb on this page as the post-rename equivalent of today's `specify initiative` / `specify plan` surface. The verb set (`create`, `plan`, `execute`, `finalize`, `archive`) is preserved across the rename.

| Verb                                              | Purpose                                                                                                                                                                           |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify change create <name>`                    | Scaffold `change.md` from the canonical template. Refuses to overwrite an existing file.                                                                                          |
| `specify change plan add <name>`                  | Append a new plan entry (`pending` status). Validates the entry and the resulting plan shape.                                                                                     |
| `specify change plan amend <name>`                | Edit non-status fields on an existing plan entry (depends-on, sources, description, project, schema, context).                                                                    |
| `specify change plan next`                        | Return the next eligible plan entry, respecting `depends-on` and the `multiple-in-progress` invariant. The selection that `/spec:execute` drives.                                 |
| `specify change plan status`                      | Render the per-entry status table (text or JSON). The reporting surface a coordinator polls between waves.                                                                        |
| `specify change plan doctor`                      | Strict superset of `plan validate`: cycle, orphan-source, stale-clone, and unreachable-entry diagnostics on top of the base shape rules.                                          |
| `specify change plan transition <name> <target>`  | Apply a validated lifecycle transition (`pending` → `in-progress` → `done`/`failed`/`blocked`/`skipped`). The single-writer for `Entry::status`.                                  |
| `specify change plan archive`                     | Sweep `plan.yaml` and the `.specify/plans/<name>/` authoring trail into the archive on its own.                                                                                   |
| `specify change plan lock {acquire,release,status}` | Manage the `.specify/plan.lock` PID stamp that serialises concurrent `/spec:execute` drivers.                                                                                   |
| `specify change execute`                          | Drive eligible plan entries through the slice loop end-to-end. The long-running orchestrator that calls `/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`.           |
| `specify change finalize`                         | Closure verb. Confirms every plan entry is terminal, every per-project PR has merged on its remote, and every workspace clone is clean — then atomically archives the change.    |
| `specify change archive`                          | Operator-driven archive of a completed change without re-running the finalize guards. Used when finalize ran in a previous session.                                               |

The change component owns only the umbrella + plan surface. The per-slice verbs (`specify slice {create, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task}`) are core and live on the slice loop crate.

## Dependency direction

The change component is downstream of the registry and the slice loop:

```text
specify-change → specify-registry → specify-capability
              → specify-slice    (per-loop primitives)
              → specify-core
```

The invariant: **the change component depends on `specify-registry` and the core slice loop, never the reverse.** Orchestration composes registry materialisation and the fixed slice loop; neither lower-level service knows about the umbrella. RFC-13 invariant #4 spells this out and [RFC-5](../../rfcs/rfc-5-lint.md) is the home for the lint that enforces it. See [RFC-13 §Migration](../../rfcs/rfc-13-extensibility.md#migration).

> **Crate naming on the rfc-13 branch.** The umbrella crate is currently named `specify-initiative` on disk; the per-loop crate is currently `specify-change`. Phase 3.1 renames `specify-change` → `specify-slice`, which frees the `specify-change` package name; Phase 3.4 then renames `specify-initiative` → `specify-change`. This page describes the post-Phase-3 surface so it stays accurate after the rename.

## Plan / change relationship

A *change* is the umbrella concept: an operator-defined outcome that coordinates one or more *slices*. A *slice* is the single unit that flows through the fixed `define → build → merge` loop — a per-project transaction with its own proposal, specs, design, tasks, and merge step. See [RFC-13 §Glossary](../../rfcs/rfc-13-extensibility.md#glossary).

Concretely:

- The change component owns `change.md` (intent) and `plan.yaml` (executable graph).
- Each plan entry names a slice that the slice loop will materialise as `.specify/slices/<name>/` (currently `.specify/changes/<name>/`) and run `define → build → merge` against.
- Cross-capability outcomes are coordinated by additional plan entries, not by fusing capabilities into a larger hidden slice (see [RFC-13 §"Cross-capability coexistence"](../../rfcs/rfc-13-extensibility.md#cross-capability-coexistence) and [§"Cross-capability coordination"](../../rfcs/rfc-13-extensibility.md#cross-capability-coordination)).

The post-Phase-3 surface uses these nouns; the current rfc-13 branch is mid-rename, with the umbrella still spelled `initiative` and the per-loop unit still spelled `change`.

## What the change component must NOT own

The change component is operator intent, an executable plan, and the close-out protocol. Mirror of the [RFC-13 §"Platform components are not capabilities"](../../rfcs/rfc-13-extensibility.md#platform-components-are-not-capabilities) table:

- **Domain artefact ownership.** Specs, contracts, code, fixtures — every mutable artefact has exactly one capability owner. The change component never reaches into a capability's baseline directories.
- **Topology materialisation.** Project clones and symlinks are derived registry state owned by `specify registry`. The change component reads materialised project roots; it does not create them.
- **Hidden multi-capability transactions.** A single slice runs against exactly one capability/scope. Cross-capability outcomes are explicit plan entries — never one slice that writes multiple capabilities' baselines.

## Merge and adoption contract

The slice loop and the change component share a thin go/no-go protocol with capability merge skills. The full surface is in [RFC-13 §"Merge and adoption contract"](../../rfcs/rfc-13-extensibility.md#merge-and-adoption-contract); the loop is:

1. The capability merge skill validates the staged artefacts, decides whether each is promoted / replaced / generated / cleaned up, and runs any capability-specific drift or format checks.
2. The skill records the decision via `specify slice outcome set --phase merge --outcome {success,failed,blocked}` and appends opaque diagnostics via `specify slice journal append --kind {failure,recovery}`. (Today's surface: `specify change outcome set` / `specify change journal append`.)
3. The core reads the merge phase outcome. On `success` it proceeds with archival; on `failed` or `blocked` it halts archival and surfaces the journal entries to the operator.
4. The change component reads the slice's terminal outcome to drive the matching plan-entry transition (`done` / `failed` / `blocked` / `skipped`) and to gate finalization.

The core does not parse capability diagnostics — they round-trip as opaque journal entries, and the change component surfaces them verbatim.

## See also

- [Capabilities](capabilities/index.md) — capability manifest protocol and the dependency direction sister page.
- [Registry](registry.md) — topology ledger and workspace materialisation.
- [`/spec:plan`](initiative-skills/plan.md) — change plan authoring skill (moves to `/change:plan` in Phase 3.9).
- [`/spec:execute`](initiative-skills/execute.md) — change execution driver (moves to `/change:execute` in Phase 3.9).
- [Lifecycle](lifecycle.md) — slice-loop state machine the change component drives entries through.
- [RFC-13: Extensibility](../../rfcs/rfc-13-extensibility.md) — capability protocol, platform components, and migration plan.
