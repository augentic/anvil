# emery slice

Refine, build, validate, merge, and archive individual slices. The `slice` noun group covers every per-slice operation; the `change` noun belongs to the umbrella surface.

Every per-slice verb takes the slice `<name>`. The CLI resolves the on-disk directory from the name internally (no `<slice-dir>` arg). Slice directories are minted by the refine orchestration; lifecycle transitions are owned by the orchestrations (`refined`, `built`) and the merge/drop verbs — there is no standalone create or transition verb.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`list`](#emery-slice-list) | Read-only listing of every slice under `.emery/slices/` with its lifecycle status and target. |
| [`refine`](#emery-slice-refine) | Refine one plan entry's slice to `refined`: slice create, extract per bound source, synthesis, validation, and the `refined` transition in one guest orchestration. |
| [`model`](#emery-slice-model) | `model show` — read-only view of the persisted `model.yaml`. |
| [`provenance`](#emery-slice-provenance) | Project the on-demand audit view of inline provenance from `model.yaml` + Evidence. |
| [`build`](#emery-slice-build) | Build the slice through its bound target adapter: the guest orchestration assembles the build request, drives the target's build operation, validates the report, and gates the `built` transition. |
| [`validate`](#emery-slice-validate) | Run artifact validation. |
| [`merge`](#emery-slice-merge) | `merge {preview, conflict-check, run}` -- preview the delta merge, detect baseline conflicts, or execute the merge. |
| [`drop`](#emery-slice-drop) | Discard a slice without merging. Archive moves are owned by `slice merge`, `slice drop`, and `emery plan archive`. |

## Subcommands

### emery slice list

List every slice under `.emery/slices/` with its lifecycle status and recorded target.

```bash
emery slice list [--format json]
```

Read-only: one line per slice (`<name>  <status>  <target>`), sorted by name; directories without a `metadata.yaml` are skipped. JSON returns `slices[]` with `name`, `status`, and `target` per entry.

### emery slice refine

Refine one named plan entry's slice to `refined` — the verb behind the [`/emery:refine`](../slice-skills/index.md#emeryrefine) breakout and the phase the `plan execute` loop runs per entry.

```bash
emery slice refine <name> [--format json]
```

Guest-routed: one orchestration owns slice create (re-entry safe), the per-binding `extract` fan-out, the synthesis judgment leg, the persist tail, validation, and the `refined` transition. It acts on the named slice directly against a `pending` or `in-progress` plan entry, never advances per-entry status, and refuses a `done` entry.

Exit codes: `0` success (slice at `refined`); `2` for blocking validation findings; non-zero on an extract or synthesis failure — the persist tail leaves prior artifacts intact and the slice stays `refining`.

JSON output: see the [synthesis envelopes](../cli-output-shapes.md#synthesis-envelopes) and the synthesis persist summary.

**Synthesis internals (summary).** The synthesis leg turns the slice's `Evidence[]` plus the bound target's `guidance` prompt into the canonical artifacts and the typed `model.yaml`. The agent authors the prose and per-requirement claims; the CLI-owned projection kernel alone derives `REQ` ids, `status`, `winner` markers, and the rendered `Sources:` lists (anything the agent supplies is ignored and re-derived), then atomically persists `proposal.md` / `specs/<domain>/spec.md` / `design.md` / `tasks.md` / `model.yaml`. Provenance is carried inline in `model.yaml` — there is no `provenance.yaml`. For the full authority-resolution and reconciliation story, see [From sources to slices](../../explanation/reconciliation.md).

### emery slice model

Read-only view of the persisted typed model.

```bash
emery slice model show <name> [--format json]
```

Loads `.emery/slices/<name>/model.yaml` and renders it (text, or the schema-shaped object under `--format json`). The model carries the earned core — `requirements` (with inline provenance: `claims[]`, `winner` markers, rendered `sources`, `status`) and `tasks` — plus the `version` / `slice` / `project` header. `target` is not a `model.yaml` field; it is resolved on demand from the bound project.

### emery slice provenance

Project the audit view of a slice's inline provenance on demand.

```bash
emery slice provenance <name> [--format json]
```

Reshapes the inline `model.yaml` data plus on-disk Evidence into the per-requirement audit shape (`{ id, status, sources, contributing-claims, resolution, resolution-trace }`), recomputing `resolution` and reading each claim's `value` / `path` from `evidence/<source>.yaml`. Byte-stable given the same `model.yaml` and Evidence. Audit-only: no downstream verb reads a persisted provenance file. See [provenance projection](../provenance.md) for the block grammar.

### emery slice build

Build the slice through its bound target adapter's `build` operation and gate the `built` transition. Guest-routed: one orchestration owns request assembly, the adapter guest's judgment leg, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; the target's compiled-in brief owns only code generation.

```bash
emery slice build <name> [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name (under `.emery/slices/`) |
| `--format` | Output format: `json` for the structured result envelope |

The orchestration resolves the target from the slice's bound project, assembles the typed build request, writes `.emery/slices/<name>/build/request.yaml`, emits `target.execution.agent`, drives the adapter guest's `build` brief (including any in-guest build prelude, e.g. vectis asset materialization and host-prereq gates), then in its finalize tail emits `slice.build.started`, gates the typed report (slice-name match, blocking findings), rejects a `status: success` report carrying any blocking finding (`target-build-success-with-blocking-finding`), gates the `refined -> built` transition, and journals `slice.build.succeeded` (or `slice.build.failed` with a short `reason`). A `required` adapter-declared input absent from the slice tree aborts with `target-build-input-missing`.

Exit codes: `0` success (slice at `built`); `2` for the `target-build-*` aborts and report-gate refusals.

JSON output: the [`emery slice build` envelope](../cli-output-shapes.md#emery-slice-build).

This is the verb invoked by [`/emery:build`](../slice-skills/index.md#emerybuild) — the finalize tail owns the `built` transition gate.

### emery slice drop

Drop a slice (transition to `dropped` and archive).

```bash
emery slice drop <name> [--reason "<rationale>"]
```

### emery slice validate

Run structural artifact validation against a slice.

```bash
emery slice validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Typed-model drift checks** (synthesized slices) -- load `model.yaml` and emit `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, and `slice-model-id-grammar`. Blocking findings gate the transition at exit 2. Every synthesized slice must carry `model.yaml`.
- **Adapter checks** -- artifacts conform to the active adapter's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Renders a `DiagnosticReport` on stdout — the neutral finding currency every check surface shares. Each finding carries a `kind`: `violation` (a structural defect; open `critical`/`important` violations block the gate and exit 2) or `review` (a deterministically-raised request for agent judgment — e.g. thin discovery-lead synopses — surfaced but never blocking). See [CLI output shapes](../cli-output-shapes.md#emery-slice-validate) for the wire shape.

### emery slice merge

One subcommand covers the merge surface, with two read-only dry-run flags.

#### emery slice merge

The merge operation. By default it commits the delta merge and archives the slice; either dry-run flag projects instead of committing.

```bash
emery slice merge <name> [--preview | --conflict-check] [--format json]
```

- `--preview` — read-only projection of what the merge would do: which baseline specs would be created, modified, or removed, plus composition delta operations for Vectis slices (screen-level `added`/`modified`/`removed`). Rejects flat requirement-block deltas against a non-empty baseline with `merge-delta-headers-required` (prose-only no-op deltas with zero requirement headings remain valid). No lifecycle writes.
- `--conflict-check` — read-only pass/fail probe for baseline drift since the slice was defined. Checks for both spec conflicts and composition conflicts (Vectis only — detects when a baseline screen has been modified by another merged slice since this slice was created, using per-screen checksums). If conflicts are detected, the slice's specs may need to be regenerated against the current baseline. No lifecycle writes.
- The two dry-run flags are mutually exclusive (argument error, exit 2).

Without a flag, the full merge performs:

1. Applies spec deltas from the slice to the baseline at `.emery/specs/`.
2. Applies composition deltas (Vectis only) -- merges `composition.yaml` screen-level `added`/`modified`/`removed` operations into the baseline `composition.yaml`, using per-screen SHA-256 checksums (`.composition-checksums.yaml`) for conflict detection.
3. Validates coherence of the merged baseline.
4. Transitions the slice to `merged` and stamps the plan entry's per-entry status to `done`.
5. Moves the slice directory to `.emery/archive/YYYY-MM-DD-<name>/`.

This is the CLI command invoked by `/emery:merge` (which offers the dry-run flags when the operator asks to look first). It is a single atomic operation -- if any step fails, no changes are committed. Re-running it against a torn merge (slice already `merged` and archived, plan entry still `in-progress`) heals the entry's `done` stamp and exits success without re-merging.

**Journal events.** `slice merge` brackets the merge with `slice.merge.started` then `slice.merge.succeeded` / `slice.merge.failed`, which fire on the merge **validator outcome** — there is no v1 merge envelope or merge report. The durable record stays the append-only `slice.archive.created` outcome ledger written by the archive step.

**No git surface.** `slice merge` owns no git side effects: the workspace-clone commit leg is skipped explicitly with a `slice.merge.commit-skipped` journal event, and the `slice.archive.created` ledger entry carries no `merge-sha`. Committing and publishing merged baselines is operator-owned.

**Preconditions.** Slice must be in `built` state; `merge --preview` and `merge --conflict-check` should pass. When a `plan.yaml` exists at the plan root, `merge` writes plan state (the per-entry `done` stamp), so it preflights the completion gate **before** touching the baseline: a missing entry refuses with `plan-entry-not-found`, and an entry that is not `in-progress` refuses with `slice-merge-entry-not-in-progress` (advance it with `emery plan advance` first). Standalone breakouts do not take the guest marker — the lifecycle gates are the correctness fence.

## See also

- [/emery:refine](../slice-skills/index.md) -- per-slice refine breakout
- [/emery:build](../slice-skills/index.md#emerybuild) -- skill that drives the guest-routed `slice build`
- [/emery:merge](../slice-skills/index.md#emerymerge) -- skill that invokes `slice merge` (with its dry-run flags)
- [/emery:drop](../slice-skills/index.md#emerydrop) -- skill that drops slices
- [emery plan](plan.md) -- umbrella surface that coordinates one or more slices through `change.md` + `plan.yaml`.
- [Lifecycle](../lifecycle.md) -- slice state machine reference
- [Configuration Files](../configuration.md) -- project and slice metadata
