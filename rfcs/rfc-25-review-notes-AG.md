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

### Motivation

### Normative decisions

### Operator workflow

### Concepts

### Implementation contract

### Source adapter contract

### Target adapter contract

`shape` is described as guidance text. The three-capability model needs a fourth optional capability (`refine`) for targets that produce specification artifacts after core synthesis. See proposed change above.

### Synthesis contract

The refine pipeline is a closed list: `proposal` → `specs` → `design` → `tasks`. The current Vectis adapter places `composition` between `specs` and `design` in `pipeline.define` — that position has no equivalent here. Gap by omission, not explicit prohibition. The proposed `refine` capability slots in after step 3 (synthesis) and before step 4 (validate).

### Worked examples

### On-disk and tooling

### Implementation plan

### Acceptance scenarios

### Migration

### Alternatives considered

### Non-goals

### Open questions
