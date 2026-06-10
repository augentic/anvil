# specify slice

Create, validate, transition, merge, and archive individual slices. The `slice` noun group covers every per-slice operation; the `change` noun belongs to the umbrella surface.

Every per-slice verb takes the slice `<name>`. The CLI resolves the on-disk directory from the name internally (no `<slice-dir>` arg).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-slice-create) | Create a new slice directory with an initial `metadata.yaml`. |
| [`synthesize`](#specify-slice-synthesize) | Turn the slice's `Evidence[]` into the canonical artifacts and the typed `model.yaml`: `--dry-run` emits the agent inputs envelope; `--from <response.json>` runs the projection kernel and persists. |
| [`model`](#specify-slice-model) | `model show` — read-only view of the persisted `model.yaml`. |
| [`provenance`](#specify-slice-provenance) | Project the on-demand audit view of inline provenance from `model.yaml` + Evidence. |
| [`build`](#specify-slice-build) | Build the slice through its bound target adapter: `--phase prepare` assembles the build request, `--phase finalize` validates the report and gates the `built` transition. |
| [`transition`](#specify-slice-transition) | Move a slice through the lifecycle state machine (`refining` -> `refined` -> `built` -> `merged`/`dropped`). |
| [`validate`](#specify-slice-validate) | Run artifact validation. |
| [`merge`](#specify-slice-merge) | `merge {preview, conflict-check, run}` -- preview the delta merge, detect baseline conflicts, or execute the merge. |
| [`task`](#specify-slice-task) | `task {progress, mark}` -- inspect or update the task checkbox state in `tasks.md`. |
| [`touched-specs`](#specify-slice-touched-specs) | Scan or set the spec files this slice affects. |
| [`overlap`](#specify-slice-overlap) | Find slices whose touched specs overlap. |
| [`drop`](#specify-slice-drop) | Discard a slice without merging. Archive moves are owned by `slice merge run`, `slice drop`, and `change finalize`. |

## Subcommands

### specify slice create

Create a new slice directory.

```bash
specify slice create <name> [--if-exists fail|continue|restart] [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case slice name (validated) |
| `--if-exists` | Behavior when name exists: `fail` (default, refuse), `continue` (reuse existing -- requires valid `metadata.yaml`), or `restart` (delete and recreate -- destructive) |
| `--format` | Output format: `json` for structured output |

Creates `.specify/slices/<name>/` with an initial `metadata.yaml`.

### specify slice synthesize

Turn the slice's `Evidence[]` plus the bound target's `shape` brief into the canonical artifacts and the typed `model.yaml`. Two-phase, mirroring [`specify plan propose`](plan.md#specify-plan-propose): the CLI cannot run an agent, so `--dry-run` emits the inputs the agent reconciles and `--from` projects the agent's response.

```bash
specify slice synthesize <name> --dry-run [--format json]
specify slice synthesize <name> --from <response.json> [--format json]
```

- `--dry-run` assembles the **inputs** envelope — each bound source's inline `lead` + `claims` (read from `evidence/<source>.yaml`) plus the resolved target `shape` brief body. Authority is **not** included (the kernel resolves it after the response). Read-only: writes nothing and emits a `slice.synthesize.agent` journal event.
- `--from <response.json>` is the **only artifact writer**. It emits `slice.synthesize.started`, schema-gates the response (`synthesis.schema.json`, `kind: response`, with its `model` validated against `model.schema.json`), resolves authority from the on-disk Evidence and any per-slice `authority-override`, runs the CLI-owned **projection kernel** (assign `REQ` ids in declaration order, derive `status` and per-claim `winner` markers, render highest-authority-first `Sources:` lists, write inline provenance, stamp the `version` / `slice` / `project` header), renders the `ID:` / `Sources:` / `Status:` lines into each `specs/<unit>/spec.md`, runs the drift validators, then atomically persists `proposal.md` / `specs/<unit>/spec.md` / `design.md` / `tasks.md` / `model.yaml`. On success it emits `slice.synthesize.completed`; on any failure it emits `slice.synthesize.failed`, leaves the prior artifacts intact, and the slice stays `refining`.

The agent authors the response — per-requirement `(source, id, kind)` claims, an `agreement` verdict, prose (`title`, `statement`, `scenarios`, `notes`), and the prose-only `proposal.md` / `design.md` / `tasks.md` bodies plus spec bodies without provenance lines. It does **not** author `REQ` ids, `status`, `winner` markers, or rendered `Sources:` lists; the kernel ignores and re-derives any it supplies (normalize, never reject). The synthesis step is `cache: opt-out` — there is no tool path. There is no `provenance.yaml` write; provenance is carried inline in `model.yaml`.

This is the CLI verb invoked by [`/spec:refine`](../slice-skills/refine.md) at its synthesis step. See [CLI output shapes](../cli-output-shapes.md#specify-slice-synthesize---dry-run) for the JSON envelope shapes.

### specify slice model

Read-only view of the persisted typed model.

```bash
specify slice model show <name> [--format json]
```

Loads `.specify/slices/<name>/model.yaml` and renders it (text, or the schema-shaped object under `--format json`). The model carries the earned core — `requirements` (with inline provenance: `claims[]`, `winner` markers, rendered `sources`, `status`) and `tasks` — plus the `version` / `slice` / `project` header. `target` is not a `model.yaml` field; it is resolved on demand from the bound project.

### specify slice provenance

Project the audit view of a slice's inline provenance on demand.

```bash
specify slice provenance <name> [--format json]
```

Reshapes the inline `model.yaml` data plus on-disk Evidence into the per-requirement audit shape (`{ id, status, sources, contributing-claims, resolution, resolution-trace }`), recomputing `resolution` and reading each claim's `value` / `path` from `evidence/<source>.yaml`. Byte-stable given the same `model.yaml` and Evidence. Audit-only: no downstream verb reads a persisted provenance file. See [provenance projection](../../../plugins/spec/references/synthesis/provenance.md) for the block grammar.

### specify slice build

Build the slice through its bound target adapter's `build` operation and gate the `built` transition. Two-phase, mirroring `specify source survey` / `extract`: the CLI cannot run an agent, so `--phase prepare` assembles the build request the agent's `build` brief consumes and `--phase finalize` validates the report the brief writes. The CLI owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; the target brief owns only code generation.

```bash
specify slice build <name> [--phase prepare|finalize] [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name (under `.specify/slices/`) |
| `--phase` | `prepare` (default) or `finalize`. `execution: tool` adapters run the whole operation in one call regardless of the flag. |
| `--format` | Output format: `json` for the structured handoff / result envelope |

- `--phase prepare` resolves the target from the slice's bound project, assembles and schema-validates the build request (`schemas/target/build-request.schema.json`), writes `.specify/slices/<name>/build/request.yaml`, emits `target.execution.agent`, prints the handoff envelope (`slice`, `target`, `request`, `report`, `briefs-dir`, `build-brief`), and returns without blocking. The agent then runs the target `build` brief against the prepared request and writes `.specify/slices/<name>/build/report.yaml`.
- `--phase finalize` emits `slice.build.started`, validates the agent-produced report against `schemas/target/build-report.schema.json`, rejects a `status: success` report carrying any blocking finding (`target-build-success-with-blocking-finding`), gates the `refined -> built` transition, and journals `slice.build.succeeded` (or `slice.build.failed` with a short `reason`). A `required` adapter-declared input absent from the slice tree aborts prepare with `target-build-input-missing`.

This is the CLI verb invoked by [`/spec:build`](../slice-skills/build.md) — the skill no longer hand-transitions to `built`; `finalize` owns that gate. See [CLI output shapes](../cli-output-shapes.md#specify-slice-build) for the envelope shapes.

### specify slice transition

Move a slice through the lifecycle state machine.

```bash
specify slice transition <name> <target>
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name |
| `target` | Target state: `refining`, `refined`, `built`, `dropped`. Skills stamp `refined` and `built` after `/spec:refine` and `/spec:build`. The `merged` status is intentionally absent — `slice merge run` is the sole legal writer of `merged`, since landing a slice requires the spec merge, status transition, and archive move to happen atomically. |

Enforces legal transitions. Records timestamps in `metadata.yaml`.

### specify slice touched-specs

Scan or set the specs affected by a slice.

```bash
specify slice touched-specs <name> --scan
specify slice touched-specs <name> --set <spec-path>...
```

### specify slice overlap

Check for spec overlap between active slices.

```bash
specify slice overlap <name>
```

Reports which specs are touched by multiple active slices.

### specify slice drop

Drop a slice (transition to `dropped` and archive).

```bash
specify slice drop <name> [--reason "<rationale>"]
```

### specify slice validate

Run structural and semantic artifact validation against a slice.

```bash
specify slice validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Typed-model drift checks** (synthesized slices) -- load `model.yaml` and emit `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, and `slice-model-id-grammar`. Blocking findings gate the transition at exit 2. Every synthesized slice must carry `model.yaml`.
- **Adapter checks** -- artifacts conform to the active adapter's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Renders a `DiagnosticReport` on stdout — the neutral finding currency every check surface shares. Each finding carries a `kind`: `violation` (a structural defect; open `critical`/`important` violations block the gate and exit 2) or `review` (a deterministically-raised request for agent judgment — the semantic checks the CLI cannot score, surfaced but never blocking). This keeps the CLI handling structural invariants while the agent evaluates semantic ones, without flattening "needs judgment" into a silent skip. See [CLI output shapes](../cli-output-shapes.md#specify-slice-validate) for the wire shape.

### specify slice merge

Three subcommands cover the merge surface.

#### specify slice merge preview

Preview what a merge would do without writing anything.

```bash
specify slice merge preview <name> [--format json]
```

Shows which baseline specs would be created, modified, or removed. For Vectis slices, also previews composition delta operations (screen-level `added`/`modified`/`removed`). Used by `/spec:merge` before committing.

#### specify slice merge conflict-check

Detect whether the baseline has changed since the slice was defined.

```bash
specify slice merge conflict-check <name> [--format json]
```

Returns a pass/fail result. Checks for both spec conflicts and composition conflicts (Vectis only -- detects when a baseline screen has been modified by another merged slice since this slice was created, using per-screen checksums). If conflicts are detected, the slice's specs may need to be regenerated against the current baseline.

#### specify slice merge run

The terminal merge operation. Commits the delta merge and archives the slice.

```bash
specify slice merge run <name> [--format json]
```

Performs:

1. Applies spec deltas from the slice to the baseline at `.specify/specs/`.
2. Applies composition deltas (Vectis only) -- merges `composition.yaml` screen-level `added`/`modified`/`removed` operations into the baseline `composition.yaml`, using per-screen SHA-256 checksums (`.composition-checksums.yaml`) for conflict detection.
3. Validates coherence of the merged baseline.
4. Transitions the slice to `merged` and stamps the plan entry's per-entry status to `done`.
5. Moves the slice directory to `.specify/archive/YYYY-MM-DD-<name>/`.

This is the CLI command invoked by `/spec:merge` after preview and conflict-check pass. It is a single atomic operation -- if any step fails, no changes are committed.

**Journal events.** `slice merge run` brackets the merge with `slice.merge.started` then `slice.merge.succeeded` / `slice.merge.failed`, which fire on the merge **validator outcome** — there is no v1 merge envelope or merge report. The durable record stays the append-only `slice.archive.created` outcome ledger written by the archive step.

**Workspace clone auto-commit.** When `slice merge run` runs inside a workspace clone (CWD is under `.specify/workspace/*/` and contains `.specify/project.yaml`), it auto-commits the merged baseline and archived slice directory with message `"specify: merge <slice-name>"`. Only `.specify/` subtrees are staged. A commit failure is a warning, not an error -- the spec-merge still succeeds. Use `specify workspace push` to publish commits to remotes.

**Preconditions.** Slice must be in `built` state; `slice merge preview` and `slice merge conflict-check` should pass (the skill checks these before calling `merge run`).

### specify slice task

Two subcommands cover the task surface (renamed from the old top-level `specify task progress` / `specify task mark`).

#### specify slice task progress

Report task completion progress for a slice.

```bash
specify slice task progress <name> [--format json]
```

Returns the count of completed and total tasks, parsed from `tasks.md` checkbox syntax.

#### specify slice task mark

Mark a task as complete.

```bash
specify slice task mark <name> <task-id> [--format json]
```

Flips the checkbox from `- [ ]` to `- [x]` for the specified task. The task ID is the numbered identifier (e.g. `1.2`, `2.1`).

Used by `/spec:build` as it completes each task.

## See also

- [/spec:refine](../slice-skills/index.md) -- per-slice refine breakout
- [/spec:build](../slice-skills/build.md) -- skill that drives build, calls `slice task progress`/`mark`
- [/spec:merge](../slice-skills/merge.md) -- skill that orchestrates `slice merge {preview, conflict-check, run}`
- [/spec:drop](../slice-skills/drop.md) -- skill that drops slices
- [specify plan](plan.md) -- umbrella surface that coordinates one or more slices through `change.md` + `plan.yaml`.
- [Lifecycle](../lifecycle.md) -- slice state machine reference
- [Configuration Files](../configuration.md) -- project and slice metadata
