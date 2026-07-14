# specify slice

Refine, build, validate, merge, and archive individual slices. The `slice` noun group covers every per-slice operation; the `change` noun belongs to the umbrella surface.

Every per-slice verb takes the slice `<name>`. The CLI resolves the on-disk directory from the name internally (no `<slice-dir>` arg). Slice directories are minted by the refine orchestration; lifecycle transitions are owned by the orchestrations (`refined`, `built`) and the merge/drop verbs — there is no standalone create or transition verb.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`list`](#specify-slice-list) | Read-only listing of every slice under `.specify/slices/` with its lifecycle status and target. |
| [synthesis](#synthesis-inside-specify-slice-refine) | The synthesis leg inside `specify slice refine`: turns the slice's `Evidence[]` into the canonical artifacts and the typed `model.yaml` via the projection kernel. |
| [`model`](#specify-slice-model) | `model show` — read-only view of the persisted `model.yaml`. |
| [`provenance`](#specify-slice-provenance) | Project the on-demand audit view of inline provenance from `model.yaml` + Evidence. |
| [`build`](#specify-slice-build) | Build the slice through its bound target adapter: the guest orchestration assembles the build request, drives the target's build operation, validates the report, and gates the `built` transition. |
| [`validate`](#specify-slice-validate) | Run artifact validation. |
| [`merge`](#specify-slice-merge) | `merge {preview, conflict-check, run}` -- preview the delta merge, detect baseline conflicts, or execute the merge. |
| [`drop`](#specify-slice-drop) | Discard a slice without merging. Archive moves are owned by `slice merge run`, `slice drop`, and `specify plan archive`. |

## Subcommands

### specify slice list

List every slice under `.specify/slices/` with its lifecycle status and recorded target.

```bash
specify slice list [--format json]
```

Read-only: one line per slice (`<name>  <status>  <target>`), sorted by name; directories without a `metadata.yaml` are skipped. JSON returns `slices[]` with `name`, `status`, and `target` per entry.

### Synthesis (inside `specify slice refine`)

The synthesis engine turns the slice's `Evidence[]` plus the bound target's `guidance` prompt into the canonical artifacts and the typed `model.yaml`. It runs as the judgment leg inside the guest-routed `specify slice refine` (and the `plan execute` loop).

- The **inputs** leg assembles the **inputs** envelope — each bound source's inline `lead` + `claims` (read from `evidence/<source>.yaml`) plus the resolved target guidance body (wire field `guidance-brief`). Authority is **not** included (the kernel resolves it after the response). Read-only: writes nothing and emits a `slice.synthesize.agent` journal event.
- The **persist** tail is the **only artifact writer**. It emits `slice.synthesize.started`, schema-gates the response (`synthesis.schema.json`, `kind: response`, with its `model` validated against `model.schema.json`), resolves authority from the on-disk Evidence and any per-slice `authority-override`, runs the CLI-owned **projection kernel** (baseline-aware `REQ` id assignment — slice-global for new domains, continuing from baseline max for additive requirements in modified domains; honour `baseline-id` for modifications; derive `status` and per-claim `winner` markers; render highest-authority-first `Sources:` lists; write inline provenance; stamp the `version` / `slice` / `project` header), renders `## ADDED` / `## MODIFIED` delta sections (modified domains) or flat blocks (new domains) with `ID:` / `Sources:` / `Status:` lines into each `specs/<domain>/spec.md`, auto-scans `metadata.touched_specs`, runs the drift validators, then atomically persists `proposal.md` / `specs/<domain>/spec.md` / `design.md` / `tasks.md` / `model.yaml`. On success it emits `slice.synthesize.completed`; on any failure it emits `slice.synthesize.failed`, leaves the prior artifacts intact, and the slice stays `refining`.

The agent authors the response — per-requirement `(source, id, kind)` claims, an `agreement` verdict, prose (`title`, `statement`, `scenarios`, `notes`), and the prose-only `proposal.md` / `design.md` / `tasks.md` bodies plus spec bodies without provenance lines. It does **not** author `REQ` ids, `status`, `winner` markers, or rendered `Sources:` lists; the kernel ignores and re-derives any it supplies (normalize, never reject). The synthesis step is always agent-dispatched — there is no tool path. There is no `provenance.yaml` write; provenance is carried inline in `model.yaml`.

This is the synthesis step inside [`/spec:refine`](../slice-skills/index.md#specrefine). See [CLI output shapes](../cli-output-shapes.md#synthesis-envelopes) for the envelope shapes.

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

Reshapes the inline `model.yaml` data plus on-disk Evidence into the per-requirement audit shape (`{ id, status, sources, contributing-claims, resolution, resolution-trace }`), recomputing `resolution` and reading each claim's `value` / `path` from `evidence/<source>.yaml`. Byte-stable given the same `model.yaml` and Evidence. Audit-only: no downstream verb reads a persisted provenance file. See [provenance projection](../provenance.md) for the block grammar.

### specify slice build

Build the slice through its bound target adapter's `build` operation and gate the `built` transition. Guest-routed: one orchestration owns request assembly, the adapter guest's judgment leg, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; the target's compiled-in brief owns only code generation.

```bash
specify slice build <name> [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name (under `.specify/slices/`) |
| `--format` | Output format: `json` for the structured result envelope |

The orchestration resolves the target from the slice's bound project, assembles the typed build request, writes `.specify/slices/<name>/build/request.yaml`, emits `target.execution.agent`, drives the adapter guest's `build` brief (including any in-guest build prelude, e.g. vectis asset materialization and host-prereq gates), then in its finalize tail emits `slice.build.started`, gates the typed report (slice-name match, blocking findings), rejects a `status: success` report carrying any blocking finding (`target-build-success-with-blocking-finding`), gates the `refined -> built` transition, and journals `slice.build.succeeded` (or `slice.build.failed` with a short `reason`). A `required` adapter-declared input absent from the slice tree aborts with `target-build-input-missing`.

This is the verb invoked by [`/spec:build`](../slice-skills/index.md#specbuild) — the finalize tail owns the `built` transition gate. See [CLI output shapes](../cli-output-shapes.md#specify-slice-build) for the envelope shapes.

### specify slice drop

Drop a slice (transition to `dropped` and archive).

```bash
specify slice drop <name> [--reason "<rationale>"]
```

### specify slice validate

Run structural artifact validation against a slice.

```bash
specify slice validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Typed-model drift checks** (synthesized slices) -- load `model.yaml` and emit `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, and `slice-model-id-grammar`. Blocking findings gate the transition at exit 2. Every synthesized slice must carry `model.yaml`.
- **Adapter checks** -- artifacts conform to the active adapter's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Renders a `DiagnosticReport` on stdout — the neutral finding currency every check surface shares. Each finding carries a `kind`: `violation` (a structural defect; open `critical`/`important` violations block the gate and exit 2) or `review` (a deterministically-raised request for agent judgment — e.g. thin discovery-lead synopses — surfaced but never blocking). See [CLI output shapes](../cli-output-shapes.md#specify-slice-validate) for the wire shape.

### specify slice merge

Three subcommands cover the merge surface.

#### specify slice merge preview

Preview what a merge would do without writing anything.

```bash
specify slice merge preview <name> [--format json]
```

Shows which baseline specs would be created, modified, or removed. For Vectis slices, also previews composition delta operations (screen-level `added`/`modified`/`removed`). Rejects flat requirement-block deltas against a non-empty baseline with `merge-delta-headers-required` (prose-only no-op deltas with zero requirement headings remain valid). Used by `/spec:merge` before committing.

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

**No git surface.** `slice merge run` owns no git side effects: the workspace-clone commit leg is skipped explicitly with a `slice.merge.commit-skipped` journal event, and the `slice.archive.created` ledger entry carries no `merge-sha`. Committing and publishing merged baselines is operator-owned.

**Preconditions.** Slice must be in `built` state; `slice merge preview` and `slice merge conflict-check` should pass. When a `plan.yaml` exists at the plan root, `merge run` writes plan state (the per-entry `done` stamp), so it preflights the completion gate **before** touching the baseline: a missing entry refuses with `plan-entry-not-found`, and an entry that is not `in-progress` refuses with `slice-merge-entry-not-in-progress` (claim it with `specify plan next` first). Standalone breakouts do not take the guest marker — the lifecycle gates are the correctness fence.

## See also

- [/spec:refine](../slice-skills/index.md) -- per-slice refine breakout
- [/spec:build](../slice-skills/index.md#specbuild) -- skill that drives the guest-routed `slice build`
- [/spec:merge](../slice-skills/index.md#specmerge) -- skill that orchestrates `slice merge {preview, conflict-check, run}`
- [/spec:drop](../slice-skills/index.md#specdrop) -- skill that drops slices
- [specify plan](plan.md) -- umbrella surface that coordinates one or more slices through `change.md` + `plan.yaml`.
- [Lifecycle](../lifecycle.md) -- slice state machine reference
- [Configuration Files](../configuration.md) -- project and slice metadata
