# Change Component

The change component is a first-party Specify component, not a capability — it coordinates an outcome from brief through executable plan, execution state, and close-out.

## What is the change component?

The change component (implemented in the `specify-change` crate) coordinates an outcome from **brief** (`change.md`) through **executable plan** (`plan.yaml`), **execution state** (`.specify/slices/<name>/.metadata.yaml`), and **close-out** (`specify change finalize` and `specify change archive`). It consumes registry project ids, materialised project paths, and core slice phase outcomes — but it does not own any of those.

The change component is **not** a capability: it has commands, libraries, and files, but it does not appear in any `capability.yaml`, it is not activated through the manifest protocol, and the core never switches on a capability name to invoke it. See [Platform components are not capabilities](../explanation/decision-log.md#platform-components-are-not-capabilities).

## Files and state

| Path                                   | Owner             | Purpose                                                                                                                                                                                  |
| -------------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `change.md`                            | operator          | Operator-authored brief at the repo root: desired outcome, scope, impacted projects, close-out criteria.                                                                            |
| `plan.yaml`                            | change skills + operator | Executable dependency-aware plan at the repo root: sequenced slice entries, per-entry scope (project, capability, sources), and lifecycle status.                                  |
| `.specify/slices/<name>/.metadata.yaml` | core (slice loop) | Per-slice phase outcomes (`define`, `build`, `merge`) consumed by the plan.                                                                                                          |
| `.specify/slices/<name>/journal.yaml` | core (slice loop) | Append-only audit log per slice. Capability merge briefs append `failure` and `recovery` entries here for the change component to surface.                                               |
| `.specify/plan.lock`                   | change component  | Advisory PID stamp serialising concurrent `/change:execute` drivers.                                                                                                                       |
| `.specify/archive/plans/<YYYYMMDD>-<name>/` | change component | Archive destination written atomically by `specify change finalize` once every per-project PR has merged.                                                                                |

## The three-skill lifecycle

The change component is coordinated agent-side by three peer skills with an explicit operator review seam between authoring and execution:

| Skill | Owns |
|-------|------|
| [`/change:draft <name>`](change-skills/draft.md) | Brief scaffold, registry validate, plan brief pipeline (discovery → [sync-workspace] → propose → [assignment]), `specify plan validate`. Ends at hand-off — never starts execution. |
| *(operator reviews `plan.yaml`)* | Operator runs `specify plan amend`, `specify plan status`, etc. as needed. |
| [`/change:execute loop`](change-skills/execute.md) | Per-slice `/spec:define → /spec:build → /spec:merge`, status transitions, driver lock. |
| *(operator reviews implementation)* | |
| [`/change:finalize <name>`](change-skills/finalize.md) | `specify workspace push`, `gh pr list` observation, `specify change finalize`. |

The lifecycle reads `draft → execute → finalize` and deliberately mirrors `/spec`'s `define → build → merge` rhythm at the change layer. There is no umbrella mode and no automatic transition between the three skills; the human seam between authoring and execution is the design.

## Verbs

The change component publishes the CLI surface below. The `Plan` family is folded under `Change` so planning is a subresource — `plan.yaml` is the file name, but it is not a peer top-level CLI noun.

| Verb                                              | Purpose                                                                                                                                                                           |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify change draft <name>`                     | Scaffold `change.md` and `plan.yaml` together from canonical templates. Refuses to overwrite an existing file.                              |
| `specify plan add <name>`                  | Append a new plan entry (`pending` status). Validates the entry and the resulting plan shape.                                                                                     |
| `specify plan amend <name>`                | Edit non-status fields on an existing plan entry (depends-on, sources, description, project, schema, context).                                                                    |
| `specify plan next`                        | Return the next eligible plan entry, respecting `depends-on` and the `multiple-in-progress` invariant. The selection that `/change:execute` drives.                                 |
| `specify plan status`                      | Render the per-entry status table (text or JSON). The reporting surface a coordinator polls between waves.                                                                        |
| `specify plan validate`                    | Base shape rules plus the four health diagnostics (cycle, orphan-source, stale-clone, unreachable-entry). First triage step when `/change:execute loop` reports `stuck`.            |
| `specify plan transition <name> <target>`  | Apply a validated lifecycle transition (`pending` → `in-progress` → `done`/`failed`/`blocked`/`skipped`). The single-writer for `Entry::status`.                                  |
| `specify plan archive`                     | Sweep `plan.yaml` and the `.specify/plans/<name>/` authoring trail into the archive on its own.                                                                                   |
| `specify plan lock {acquire,release,status}` | Manage the `.specify/plan.lock` PID stamp that serialises concurrent `/change:execute` drivers.                                                                                   |
| `specify change execute`                          | Drive eligible plan entries through the slice loop end-to-end. The long-running orchestrator that calls `/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`.           |
| `specify change finalize`                         | Closure verb. Confirms every plan entry is terminal, every per-project PR has merged on its remote, and every workspace clone is clean — then atomically archives the change.    |
| `specify change archive`                          | Operator-driven archive of a completed change without re-running the finalize guards. Used when finalize ran in a previous session.                                               |

The change component owns only the change + plan surface. The per-slice verbs (`specify slice {create, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task}`) are core and live on the slice loop crate.

## Dependency direction

The change component is downstream of the registry and the slice loop:

```text
specify-change → specify-registry → specify-capability
              → specify-slice    (per-loop primitives)
              → specify-core
```

The invariant: **the change component depends on `specify-registry` and the core slice loop, never the reverse.** Orchestration composes registry materialisation and the fixed slice loop; neither lower-level service knows about the change layer. A workspace lint enforces the rule.

## Plan / change relationship

A *change* is the operator-defined outcome that coordinates one or more *slices*. A *slice* is the single unit that flows through the fixed `define → build → merge` loop — a per-project transaction with its own proposal, specs, design, tasks, and merge step.

Concretely:

- The change component owns `change.md` (intent) and `plan.yaml` (executable graph).
- Each plan entry names a slice that the slice loop will materialise as `.specify/slices/<name>/` and run `define → build → merge` against.
- Cross-capability outcomes are coordinated by additional plan entries, not by fusing capabilities into a larger hidden slice.

## What the change component must NOT own

The change component is user intent, an executable plan, and the close-out protocol:

- **Domain artefact ownership.** Specs, contracts, code, fixtures — every mutable artefact has exactly one capability owner. The change component never reaches into a capability's baseline directories.
- **Topology materialisation.** Project clones and symlinks are derived registry state owned by `specify registry`. The change component reads materialised project roots; it does not create them.
- **Hidden multi-capability transactions.** A single slice runs against exactly one capability/scope. Cross-capability outcomes are explicit plan entries — never one slice that writes multiple capabilities' baselines.

## Merge and adoption contract

The slice loop and the change component share a thin go/no-go protocol with capability merge skills:

1. The capability merge skill validates the staged artefacts, decides whether each is promoted / replaced / generated / cleaned up, and runs any capability-specific drift or format checks.
2. The skill records the decision via `specify slice outcome set --phase merge --outcome {success,failed,blocked}` and appends opaque diagnostics via `specify slice journal append --kind {failure,recovery}`.
3. The core reads what the merge phase reports back. On `success` it proceeds with archival; on `failed` or `blocked` it halts archival and surfaces the journal entries to the user.
4. The change component reads the slice's terminal outcome to drive the matching plan-entry transition (`done` / `failed` / `blocked` / `skipped`) and to gate finalization.

The core does not parse capability diagnostics — they round-trip as opaque journal entries, and the change component surfaces them verbatim.

## See also

- [Capabilities](capabilities/index.md) — capability manifest protocol and the dependency direction sister page.
- [Registry](registry.md) — topology ledger and workspace materialisation.
- [`/change:draft`](change-skills/draft.md) — change plan authoring skill.
- [`/change:execute`](change-skills/execute.md) — change execution driver.
- [`/change:finalize`](change-skills/finalize.md) — change close-out skill (push, observe PRs, archive).
- [Lifecycle](lifecycle.md) — slice-loop state machine the change component drives entries through.
