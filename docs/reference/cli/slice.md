# emery slice

Read-only projections over individual slices. The `slice` noun group is inspection-only: refinement is the [`emery plan refine`](plan.md#emery-plan-refine) drain, build and merge are phases of the [`emery plan execute`](plan.md#emery-plan-execute) loop, and drop is [`emery plan drop`](plan.md#emery-plan-drop) — none are standalone per-slice verbs.

Every verb takes the slice `<name>`. The CLI resolves the on-disk directory from the name internally (no `<slice-dir>` arg). Slice directories are minted by the `plan refine` drain; lifecycle transitions are owned by the plan orchestrations.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`list`](#emery-slice-list) | Read-only listing of every slice under `.emery/slices/` with its lifecycle status and target. |
| [`validate`](#emery-slice-validate) | Run artifact validation, including refinement-freshness and baseline-conflict review advisories. |
| [`model`](#emery-slice-model) | `model show` — read-only view of the persisted `model.yaml`. |
| [`provenance`](#emery-slice-provenance) | Project the on-demand audit view of inline provenance from `model.yaml` + Evidence. |

## Subcommands

### emery slice list

List every slice under `.emery/slices/` with its lifecycle status and recorded target.

```bash
emery slice list [--format json]
```

Read-only: one line per slice (`<name>  <status>  <target>`), sorted by name; directories without a `metadata.yaml` are skipped. JSON returns `slices[]` with `name`, `status`, and `target` per entry.

### emery slice validate

Run structural artifact validation against a slice.

```bash
emery slice validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Typed-model drift checks** (synthesized slices) -- load `model.yaml` and emit `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, and `slice-model-id-grammar`. Blocking findings gate the transition at exit 2. Every synthesized slice must carry `model.yaml`.
- **Refinement-freshness review advisories** -- `slice-refinement-missing` when the slice has no `refinement.yaml` manifest, `slice-refinement-stale` (one finding per drifted input or bundle artifact) when the recorded manifest no longer matches its live inputs and bundle (run `emery plan refine` to re-refine — execute never refines), and `slice-baseline-conflict` when the baseline drifted under a built slice since it was defined. These are `review` findings: surfaced, never blocking.
- **Adapter checks** -- artifacts conform to the active adapter's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Renders a `DiagnosticReport` on stdout — the neutral finding currency every check surface shares. Each finding carries a `kind`: `violation` (a structural defect; open `critical`/`important` violations block the gate and exit 2) or `review` (a deterministically-raised request for agent judgment — e.g. thin discovery-lead synopses or staleness advisories — surfaced but never blocking). See [CLI output shapes](../cli-output-shapes.md#emery-slice-validate) for the wire shape.

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

## See also

- [emery plan](plan.md) -- the umbrella surface; `plan refine` drains refinement, `plan execute` drives the build → merge phases per entry, `plan drop` abandons one entry's slice.
- [Lifecycle](../lifecycle.md) -- slice state machine reference
- [Configuration Files](../configuration.md) -- project and slice metadata
