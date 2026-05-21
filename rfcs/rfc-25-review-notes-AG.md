# RFC-25 Review Notes

> Reviewer: Andrew Goldie
> Status: Reconciled against RFC-25 working draft — one open item (hub-path mismatch in §On-disk and tooling); all other findings resolved in the RFC body.
> Subject: [RFC-25: Workflow](rfc-25-workflow.md)
> Resolution-marker convention: `> **Resolved:** …` blockquotes below each finding cite the section / lines in `rfc-25-workflow.md` where the change landed. `> **Open:** …` marks findings that still need an authoring pass.

## Open items

### Vectis splits into a source adapter and a target adapter

The source/target split implies pulling Vectis apart. The image-layout-inferer's spatial inference (triage, region/container/leaf recovery from screenshots) is input analysis — it determines what's in the source material. That's a source adapter's `extract`. The Crux code generation, shell writing, and `composition.yaml` wiring are target work.

Factored model: a `screenshots` source adapter produces spatial claims via `extract`; core synthesis folds them into `spec.md` / `design.md`; the Vectis target adapter structures those requirements into `composition.yaml` during its target-specific phase. This makes spatial inference target-agnostic — a hypothetical React target could consume the same claims differently.

```text
shape brief  →  core synthesis  →  spec.md, design.md
                                          ↓
                              Vectis target-specific step  →  composition.yaml
```

`shape` is input to synthesis; `composition.yaml` is output from a target-specific step that consumes synthesis results. The second arrow has no home in the current model.

Two tensions remain:

1. **Evidence shape.** Can flat claims (`kind`, `path`, `lines`) carry structured spatial data? Likely needs new claim kinds for layout regions/containers/leaves — an RFC-update path the document already anticipates.
2. **`composition.yaml` authoring.** Even after factoring out the inferer, the Vectis target still needs to author `composition.yaml` at refine-time. That's a target-specific specification artifact the three-capability model (`shape` as guidance text, `build`, `merge`) doesn't explicitly accommodate.

### Proposed change: optional `refine` capability on target adapters

Add a post-synthesis production step to the refine pipeline, driven by an optional target capability:

```text
1. Resolve target and sources.
2. Run serial extract.
3. Synthesize: proposal → specs → design → tasks.
4. Run target `refine` brief when declared.  ← new
5. Validate.
6. Transition to defined.
```

Target manifest gains an optional fourth capability:

```yaml
name: vectis
version: 1
axis: target
capabilities: [shape, refine, build, merge]
briefs:
  shape: briefs/shape.md
  refine: briefs/composition.md
  build: briefs/build.md
  merge: briefs/merge.md
```

This keeps `shape` clean (input guidance), gives Vectis a named slot to produce `composition.yaml` from freshly-synthesized specs, and targets that don't declare `refine` skip step 4 silently.

> **Resolved (differently):** The Vectis split landed; the optional fourth `refine` capability did not.
>
> - Input side: `sources/screenshots/` is now a first-party source adapter (§Default source adapters, lines 357–368) emitting the new `region` / `container` / `leaf` claim kinds added to `schemas/evidence.schema.json` (§`extract`, line 327).
> - Output side: §Target-specific structured outputs (lines 469–479) keeps the three-capability model intact. `composition.yaml` is authored by `targets/vectis/build` from `spec.md` + `design.md` (which already carry the spatial claims folded in by core synthesis), not by a new pre-build target capability. §Migration (line 820) calls out that `composition.yaml` regenerates on the first 2.0 `/spec:execute`.
> - Both tensions you flagged are closed: the Evidence schema gained the new claim kinds, and the `composition.yaml` authoring seam is now `build`-time consuming synthesised artifacts rather than refine-time consuming `shape`.

## Section notes

### Abstract

RFC-25 says "Specify 2.0" throughout (Abstract, §Normative decisions, §Migration, `migrate-to-2.0.sh`). The companion `commands.md` says "Specify 3.0 v1 target surface" and references `migrate-to-3.0.sh`. The archived RFC-26 (folded into RFC-25) also used "3.0". When the fold happened the version number wasn't reconciled — one document says 2.0, the other says 3.0.

> **Resolved:** `commands.md` line 3 now reads "Specify 2.0 v1 target surface"; migration is uniformly `migrate-to-2.0.sh` (rfc-25-workflow.md line 820, commands.md line 71). `rg '3\.0'` over both files returns no hits.

### Motivation

*No issues identified.*

### Normative decisions

*No issues identified.*

### Operator workflow

Three descriptions of Gate 1 disagree on who stamps `reviewed`. The §Execution model paragraph says `/spec:plan` itself "stamps **Gate 1** with `specify plan transition <scope> reviewed`" as its final step. But the operator rhythm line says `/spec:plan` -> **review** -> `/spec:execute`, and the execution model diagram annotates the `pending -> reviewed` transition with "(operator)". The headless trivial path shows `plan transition reviewed` as a discrete CLI invocation separate from plan creation. Is Gate 1 stamped by the skill automatically, by the operator after an in-skill review pause, or by the operator as a separate CLI step after `/spec:plan` exits?

> **Resolved:** Operator stamps explicitly; `/spec:plan` never writes `reviewed`. §Execution model (line 122) now says `/spec:plan` "exits at `pending` and prints the literal `specify plan transition <name> reviewed` command in its closing message; the operator stamps Gate 1 explicitly. `/spec:plan` never writes `reviewed` itself." §The plan gate table (line 145) and the implementing-agent note (line 754) repeat the rule. Acceptance scenario #1 (line 794) tests it.

The plan lifecycle is `pending -> reviewed -> in-progress -> drained` but nothing says who or what transitions the plan from `reviewed` to `in-progress`. Is it `/spec:execute` on first loop entry? `specify plan next`? The diagram shows the transition but doesn't annotate the actor. `commands.md` is silent too — `plan transition` only accepts `reviewed` for plans and `done` for entries.

> **Resolved:** The lifecycle collapsed to two stored states. §Execution model (line 120): "The plan lifecycle has two stored states: `pending` (default after `plan create`) and `reviewed` (operator-stamped at Gate 1). It does not move further during execution. 'Currently executing' and 'drained' are computed from per-entry `status`." The actor question dissolves — there is no plan-level `in-progress` to transition into. Per-entry `in-progress` is written only by `specify plan next` (commands.md line 28); per-entry `done` only by `specify slice merge`. The implementing-agent note (line 754) and §Workflow vocabulary (lines 221–223) restate the split.

Minor: the `specify plan transition` positional is called `<scope>` in §Execution model and §The plan gate, but `<name>` in `commands.md`. The headless trivial path drops it entirely. Should be consistent.

> **Resolved:** `<scope>` no longer appears in `rfc-25-workflow.md` or `commands.md`; every reference reads `specify plan transition <name> <target>`.

### Concepts

The §Adapter vocabulary table defines `plugin` as "Shared implementation shape for either role." This is too vague to be useful — it doesn't say whether a plugin is a manifest, a package, a code-level interface, or just a label for "thing that can be a source or a target." The term appears nowhere else in the RFC. Either define it concretely or drop it from the vocabulary table.

> **Resolved:** §Adapter vocabulary (line 198) now defines `plugin` concretely: "Shared shape for either adapter role; schema `plugin.schema.json`, loader `crates/domain/src/plugin/`, audience tag for source + target adapter authors." Step 1 of the implementation plan (line 762) ships `schemas/plugin.schema.json`; step 3 (line 764) lands the `crates/domain/src/plugin/` loader that replaces `adapter/`. The term now resolves to a manifest schema plus a loader module rather than a label.

### Implementation contract

`planSlice` has a `candidate` field that appears in the example but not in the type definition:

- §Types defines `planSlice` as carrying `target`, `sources[]`, `project`, `status` — no `candidate`.
- The `plan.yaml` example in §Planning at every scale includes `candidate: add-search-filter`.
- `commands.md`'s `plan add` flags don't include `--candidate`.

Either the type table needs to list `candidate`, or the example should drop it. If `candidate` exists to preserve the discovery candidate `id` separately from the slice `name`, two follow-on questions: when can `name` and `candidate` diverge, and how does the `correlates-with` merge flow (combining candidates into one slice) surface in the plan?

> **Resolved:** The standalone `slices[].candidate` field is gone; the candidate id moved into each `sources[]` binding. §Types (line 238) defines `Slice.sources[]` as `{ key, candidate }[]`; §`Slice.sources` (lines 396–428) gives the full grammar plus the bare-string shorthand for the N=1 intent case (`<key>` normalises to `{ key, candidate: <slice.name> }`). `commands.md` line 28 documents that `plan add` / `plan amend` `--sources` and `--add-source` take `<key>=<candidate-id>` arguments. The implementing-agent note (line 753) and the migration script (line 820) both call out the reshape from `string[]` to `{ key, candidate }[]` and the removal of the standalone `slices[].candidate` field. The "when can name and candidate diverge" question is answered by the worked example below: divergence is the normal case for combined-evidence slices (the `identity-password-reset` slice in §Worked multi-source `plan.yaml` binds two differently-named candidates, `password-reset` and `account-pwd-reset`); the `correlates-with` merge flow surfaces as the per-slice `sources[]` list itself, with the `divergence:` enum carrying the operator's Gate-1 acknowledgement.

The only full `plan.yaml` example is the degenerate N=1 intent case. The `planSlice.sources` section shows a multi-source slice entry but not the corresponding top-level `sources:` bindings that define `legacy-monolith` and `identity-design-notes`. For a feature this central the reader has to mentally assemble the full shape from fragments. A complete multi-source `plan.yaml` example would anchor the relationship between top-level bindings and per-slice source lists.

> **Resolved:** §Worked multi-source `plan.yaml` (lines 430–466) ships a complete two-slice example showing the top-level `sources:` map (`identity-design-notes`, `legacy-monolith`) wired to per-slice `slices[].sources[]` bindings, including a `divergence: likely` slice where the two sources surface differently-named candidates that `propose` fused.

### Source adapter contract

§Extraction reliability says "`optional: true` on binding allows fail-soft" but the RFC never shows where this flag lives — on the top-level `plan.yaml.sources` binding? On the per-slice `planSlice.sources` entry? In the source manifest? No example includes it, and neither `plan add` nor `plan amend` in `commands.md` list an `--optional` flag.

> **Resolved (by removal):** The fail-soft `optional: true` binding flag was dropped — `rg 'optional:|--optional'` over `rfc-25-workflow.md` returns no hits. §Extraction reliability (lines 553–559) now has a deterministic `Failure` row: "Any `extract` fails -> stay `refining`, no synthesis. Operator amends the plan to drop the source if they want to proceed without it." The `--remove-source` flag on `plan amend` (commands.md line 28) is the escape hatch. Acceptance scenario #5f (line 803) covers extract failure; #5j (line 806) covers the path-denied variant.

### Target adapter contract

`shape` is described as guidance text. The three-capability model needs a fourth optional capability (`refine`) for targets that produce specification artifacts after core synthesis. See proposed change above.

> **Resolved (differently):** Three-capability model preserved. §Target-specific structured outputs (lines 469–479) lands the alternative: target-specific manifests like `composition.yaml` are authored by `build`, not by a new pre-build target capability. The build brief reads `spec.md` + `design.md` (which already carry every spatial / structural claim that core synthesis folded in from source adapters) and writes the manifest in the same pass as the implementation code. See also the Open-items resolution above.

### Synthesis contract

The refine pipeline is a closed list: `proposal` → `specs` → `design` → `tasks`. The current Vectis adapter places `composition` between `specs` and `design` in `pipeline.define` — that position has no equivalent here. Gap by omission, not explicit prohibition. The proposed `refine` capability slots in after step 3 (synthesis) and before step 4 (validate).

> **Resolved (differently):** `composition` does not sit inside the refine pipeline. Same answer as §Target adapter contract above — `targets/vectis/build` regenerates `composition.yaml` from the synthesised `spec.md` + `design.md` on the first 2.0 `/spec:execute` (§Migration, line 820). The refine pipeline (§`/spec:refine` pipeline, lines 506–514) stays the closed four-substep list.

### Worked examples

*No issues identified.*

### On-disk and tooling

Two different path notations for hub slice storage. §Hub routing says "slice artifacts live in `.specify/workspace/<project>/`" (under the hub's `.specify/`). §`.specify/` layout says "slices under `workspace/<project>/.specify/slices/`" (under the repo root). These can't both be right — one has the wrong nesting.

> **Open:** `hub` → `workspace` and `workspace/<project>/` → `slots/<project>/` renames happened (implementing-agent note, line 748), but the path-shape mismatch survives the rename:
>
> - §Workspace routing (line 183): "slice artifacts live in `.specify/slots/<project>/`" — slots nested *inside* the workspace's `.specify/`.
> - §`.specify/` layout (line 691): "slices under `slots/<project>/.specify/slices/`" — slots at the workspace root, each carrying its own `.specify/`.
>
> Per the implementing-agent note ("tier-2 executor checkouts only"), the second form (per-slot `.specify/`) is consistent with how a workspace slot is a separate project checkout; line 183 reads as a leftover from the pre-rename text. Needs one editing pass to pick a form and update the other reference.

`discovery.md` and `discovery-summary.md` appear to be the same file under two names. §Source adapter contract and §Discovery handshake reference `discovery.md` as the candidate-block container and "plan-time source of truth". §`surfaces.json` and `discovery-summary.md` says "`survey.md` becomes generic `discovery-summary.md` with `## Summary`, `## Source inventory`, and `## Candidate inventory`." If `discovery-summary.md` contains `## Candidate inventory`, it sounds like the same file as `discovery.md` — but the names differ and the relationship is never stated.

> **Resolved:** Single name `discovery.md`. `rg 'discovery-summary'` over `rfc-25-workflow.md` returns no hits; §Discovery handshake (lines 372–394) and §`discovery.md` consolidation (lines 695–697) both refer to `discovery.md` carrying `## Summary`, `## Source inventory`, and `## Candidate inventory`. The `surfaces.json` intermediate is now framed as an adapter-internal staging concern, not a sibling Specify artifact.

### Implementation plan

*No issues identified.*

### Acceptance scenarios

*No issues identified.*

### Migration

*No issues identified.*

### Alternatives considered

*No issues identified.*

### Non-goals

*No issues identified.*

### Open questions

*No issues identified.*
