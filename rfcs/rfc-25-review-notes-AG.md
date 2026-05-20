# RFC-25 Review Notes

> Reviewer: Andrew Goldie
> Status: In progress
> Subject: [RFC-25: Workflow](rfc-25-workflow.md)

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

## Section notes

### Abstract

RFC-25 says "Specify 2.0" throughout (Abstract, §Normative decisions, §Migration, `migrate-to-2.0.sh`). The companion `commands.md` says "Specify 3.0 v1 target surface" and references `migrate-to-3.0.sh`. The archived RFC-26 (folded into RFC-25) also used "3.0". When the fold happened the version number wasn't reconciled — one document says 2.0, the other says 3.0.

### Motivation

### Normative decisions

### Operator workflow

Three descriptions of Gate 1 disagree on who stamps `reviewed`. The §Execution model paragraph says `/spec:plan` itself "stamps **Gate 1** with `specify plan transition <scope> reviewed`" as its final step. But the operator rhythm line says `/spec:plan` -> **review** -> `/spec:execute`, and the execution model diagram annotates the `pending -> reviewed` transition with "(operator)". The headless trivial path shows `plan transition reviewed` as a discrete CLI invocation separate from plan creation. Is Gate 1 stamped by the skill automatically, by the operator after an in-skill review pause, or by the operator as a separate CLI step after `/spec:plan` exits?

The plan lifecycle is `pending -> reviewed -> in-progress -> drained` but nothing says who or what transitions the plan from `reviewed` to `in-progress`. Is it `/spec:execute` on first loop entry? `specify plan next`? The diagram shows the transition but doesn't annotate the actor. `commands.md` is silent too — `plan transition` only accepts `reviewed` for plans and `done` for entries.

Minor: the `specify plan transition` positional is called `<scope>` in §Execution model and §The plan gate, but `<name>` in `commands.md`. The headless trivial path drops it entirely. Should be consistent.

### Concepts

The §Adapter vocabulary table defines `plugin` as "Shared implementation shape for either role." This is too vague to be useful — it doesn't say whether a plugin is a manifest, a package, a code-level interface, or just a label for "thing that can be a source or a target." The term appears nowhere else in the RFC. Either define it concretely or drop it from the vocabulary table.

### Implementation contract

`planSlice` has a `candidate` field that appears in the example but not in the type definition:

- §Types defines `planSlice` as carrying `target`, `sources[]`, `project`, `status` — no `candidate`.
- The `plan.yaml` example in §Planning at every scale includes `candidate: add-search-filter`.
- `commands.md`'s `plan add` flags don't include `--candidate`.

Either the type table needs to list `candidate`, or the example should drop it. If `candidate` exists to preserve the discovery candidate `id` separately from the slice `name`, two follow-on questions: when can `name` and `candidate` diverge, and how does the `correlates-with` merge flow (combining candidates into one slice) surface in the plan?

The only full `plan.yaml` example is the degenerate N=1 intent case. The `planSlice.sources` section shows a multi-source slice entry but not the corresponding top-level `sources:` bindings that define `legacy-monolith` and `identity-design-notes`. For a feature this central the reader has to mentally assemble the full shape from fragments. A complete multi-source `plan.yaml` example would anchor the relationship between top-level bindings and per-slice source lists.

### Source adapter contract

§Extraction reliability says "`optional: true` on binding allows fail-soft" but the RFC never shows where this flag lives — on the top-level `plan.yaml.sources` binding? On the per-slice `planSlice.sources` entry? In the source manifest? No example includes it, and neither `plan add` nor `plan amend` in `commands.md` list an `--optional` flag.

### Target adapter contract

`shape` is described as guidance text. The three-capability model needs a fourth optional capability (`refine`) for targets that produce specification artifacts after core synthesis. See proposed change above.

### Synthesis contract

The refine pipeline is a closed list: `proposal` → `specs` → `design` → `tasks`. The current Vectis adapter places `composition` between `specs` and `design` in `pipeline.define` — that position has no equivalent here. Gap by omission, not explicit prohibition. The proposed `refine` capability slots in after step 3 (synthesis) and before step 4 (validate).

### Worked examples

### On-disk and tooling

Two different path notations for hub slice storage. §Hub routing says "slice artifacts live in `.specify/workspace/<project>/`" (under the hub's `.specify/`). §`.specify/` layout says "slices under `workspace/<project>/.specify/slices/`" (under the repo root). These can't both be right — one has the wrong nesting.

`discovery.md` and `discovery-summary.md` appear to be the same file under two names. §Source adapter contract and §Discovery handshake reference `discovery.md` as the candidate-block container and "plan-time source of truth". §`surfaces.json` and `discovery-summary.md` says "`survey.md` becomes generic `discovery-summary.md` with `## Summary`, `## Source inventory`, and `## Candidate inventory`." If `discovery-summary.md` contains `## Candidate inventory`, it sounds like the same file as `discovery.md` — but the names differ and the relationship is never stated.

### Implementation plan

### Acceptance scenarios

### Migration

### Alternatives considered

### Non-goals

### Open questions
