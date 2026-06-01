---
name: specify-refine
description: Refine a Specify slice — run extract per source, synthesise proposal/spec/design/tasks, validate, transition to refined. Use when an in-progress plan entry needs slice-time synthesis; not for first-time slice authoring without a plan (use /spec:plan).
argument-hint: "[slice-name]"
---

# Specify Refine

`/spec:refine` materialises one plan entry's slice directory, runs source `extract` serially, synthesises the four canonical artifacts from the resulting `Evidence[]` plus the target `shape` brief, validates the spec.md provenance, and transitions the slice to `refined`. The skill body owns the CLI choreography; the synthesis playbook at [`../../references/synthesis/`](../../references/synthesis/) owns what to write into the artifacts.

## Critical Path

1. **Resolve target and sources** — pick `$SLICE_NAME` from the operator argument; if absent, fall back to `specrun plan next --format json` and halt with `refine-no-active-slice` when nothing is in-progress. Take the resolved `target` from the `specrun plan next` response (the plan no longer stores a per-slice `target` — it is resolved on demand from the slice's bound `project`) and read `sources[]` from `plan.yaml.slices[<slice>]`. Each binding supplies `(source, lead)`; bare-string shorthand `<key>` normalises to `{ key: <key>, lead: $SLICE_NAME }`. Cross-resolve every `key` against `plan.yaml.sources.<key>` and every `lead` against the `## Lead inventory` blocks in `discovery.md`; halt with `refine-binding-unresolved` on lookup failure.
2. **Create the slice directory** — `specrun slice create "$SLICE_NAME" --target <target> --format json`. CLI stamps `.metadata.yaml` at `refining`; this is the first step that materialises any path under `.specify/slices/$SLICE_NAME/`. On non-zero exit, surface the structured error and stop.
3. **Extract serially per binding** — walk `sources[]` in declaration order (no parallelism). For each `{ key, lead }`, run the two-phase `specrun source extract <source> <lead> --slice "$SLICE_NAME"` handoff: `--phase prepare --format json` resolves the bound source adapter, builds the four-root sandbox (read-only `SOURCE_DIR` / `CAPABILITY_DIR`, write-only `SCRATCH_DIR`), scaffolds the `evidence/` target, emits `source.execution.agent`, and prints the handoff envelope. Run the adapter's `extract` brief against that prepared sandbox, writing outputs to the declared paths. Then `--phase finalize --format json` validates the Evidence against `schemas/evidence.schema.json`, persists it to `.specify/slices/$SLICE_NAME/evidence/<source>.yaml`, and writes the cache and journal events — CLI-owned, so the skill never persists Evidence or appends the journal line by hand. `tool`-execution adapters run the whole operation in a single call with no `--phase`. On `source-extract-path-denied` / `source-extract-failed`, stop the loop and print the extract-failure closing hint; the slice stays `refining` and the operator amends the plan (`--remove-source` / `--add-source`) before re-running.
4. **Synthesise** — `specrun target resolve <target> --format json` to locate `adapters/targets/<target>/briefs/shape.md`; load every persisted Evidence YAML; write `proposal.md → specs/<unit>/spec.md → design.md → tasks.md` in fixed order per [`../../references/synthesis/substeps.md`](../../references/synthesis/substeps.md) (which in turn cites [`requirement-block.md`](../../references/synthesis/requirement-block.md), [`authority.md`](../../references/synthesis/authority.md), [`claim-reconciliation.md`](../../references/synthesis/claim-reconciliation.md), and [`spec-format.md`](../../references/spec-format.md)). Write one spec file per `proposal.md` `## Units` entry at `specs/<unit>/spec.md`; the target shape brief explains how to choose units for that target, but the file layout is workflow-owned and identical for every target. Tags never park the slice — proceed to step 5 regardless of tag count.
5. **Validate** — `specrun slice validate "$SLICE_NAME" --format json` runs the provenance parser (`ID:` / `Sources:` / `Status:` shape, closed `Status` enum, tag/Status coherence, `Sources:` keys cross-resolved against the slice's bindings), the Evidence schema validator, and the spec-vs-model staleness + orphan-claim checks. The CLI renders a `DiagnosticReport` on stdout — `kind: violation` findings are blocking defects; `kind: review` findings are the semantic checks raised for your judgment and never block (read your worklist as `findings.filter(kind == "review")`). On success the CLI appends synthesis-tag journal events per [`../../references/synthesis/tags.md`](../../references/synthesis/tags.md) §Journal-event hand-off (one line per tagged requirement; CLI-owned). On non-zero exit, surface the stdout report plus the payload-free error envelope (its `error` is the gate discriminant) verbatim, do **not** transition, and print the validation-failure closing hint; common causes are malformed `REQ-NNN` id, `Sources:` key outside the slice's bindings, headline tag without matching `Status:`, or Evidence missing `id` on a `requirement` / `criterion` claim.
6. **Transition to refined** — `specrun slice transition "$SLICE_NAME" refined --format json`. CLI emits `slice.transition.refined`. Print the closing hint.

> **Provenance is not a hand-written file.** There is one structured slice artifact — `model.yaml` — carrying provenance **inline** on each requirement; the M2b synthesis kernel (`specrun slice synthesize`) is its sole writer, and `specrun slice provenance` projects the audit view on demand. The refine skill does not author a `provenance.yaml`. See [`../../references/synthesis/provenance.md`](../../references/synthesis/provenance.md).

## Closing hint

On success:

```text
Slice <slice-name> refined. spec tags: <U> unknown, <C> conflict, <D> divergence. Review .specify/slices/<slice-name>/specs/, then run /spec:build <slice-name> or resume /spec:execute.
```

On extract failure (any binding):

```text
Refine halted: source <source> extract failed (<error-code>). Slice <slice-name> stays refining. Amend the plan (specrun plan amend <slice-name> --remove-source <source>, or --add-source <source>=<lead>) and re-run /spec:refine <slice-name>.
```

On validation failure:

```text
Refine halted: synthesis output failed validation (<error-code>). Slice <slice-name> stays refining. Fix .specify/slices/<slice-name>/specs/<unit>/spec.md, re-run specrun slice validate <slice-name>, then specrun slice transition <slice-name> refined.
```

These three shapes are the contract `/spec:execute` matches when invoking refine as a loop step or as a breakout.

## References

- [`../../references/synthesis/`](../../references/synthesis/) — synthesis playbook (substeps, authority, requirement-block, claim-reconciliation, reconciliation, tags).
- [`../../references/synthesis/provenance.md`](../../references/synthesis/provenance.md) — the on-demand provenance projection (`specrun slice provenance`): inline-in-`model.yaml` shape, truncation rule, `resolution` enum, and `resolution-trace` step names.
- [`../../references/spec-format.md`](../../references/spec-format.md) — canonical heading conventions for requirement blocks and scenario headings in spec files.
- [`adapters/targets/<target>/briefs/shape.md`](../../../../adapters/targets/) — per-target idiom guidance synthesis folds into `design.md`.
- [`adapters/sources/<adapter>/briefs/extract.md`](../../../../adapters/sources/) — per-source-adapter extract brief invoked in step 3.

## Guardrails

- **Lifecycle single-writer:** [shared guardrails](../../../../docs/standards/skill-guardrails.md#single-writer-for-lifecycle-state).
- **Never materialise `.specify/slices/$SLICE_NAME/` outside step 2.** `specrun slice create` is the sole writer; before step 2 the on-disk shape is plan-only.
- **Never run `extract` in parallel.** Bindings are processed in `slices[].sources[]` declaration order; deterministic goldens depend on it.
- **Never park the slice on tags.** `[unknown]` / `[conflict]` / `[divergence]` are review signals; the slice still transitions to `refined`. Validation failure (provenance / Evidence schema) is the only condition that keeps the slice in `refining` after extract succeeds.
- **Never invent provenance.** A `Sources:` key on a `spec.md` requirement that did not contribute a claim is a parser failure; let the tag surface instead.
- **Never call a `specrun slice synthesize` verb.** It does not exist; the four substeps are hand-coded per the playbook.
