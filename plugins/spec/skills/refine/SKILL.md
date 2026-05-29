---
name: specify-refine
description: Refine a Specify slice — run extract per source, synthesise proposal/spec/design/tasks, validate, transition to refined. Use when an in-progress plan entry needs slice-time synthesis; not for first-time slice authoring without a plan (use /spec:plan).
argument-hint: "[slice-name]"
---

# Specify Refine

`/spec:refine` materialises one plan entry's slice directory, runs source `extract` serially, synthesises the four canonical artifacts from the resulting `Evidence[]` plus the target `shape` brief, validates the spec.md provenance, and transitions the slice to `refined`. The skill body owns the CLI choreography; the synthesis playbook at [`../../references/synthesis/`](../../references/synthesis/) owns what to write into the artifacts.

## Critical Path

1. **Resolve target and sources** — pick `$SLICE_NAME` from the operator argument; if absent, fall back to `specrun plan next --format json` and halt with `refine-no-active-slice` when nothing is in-progress. Read `plan.yaml.slices[<slice>]` for `target:` and `sources[]`. Each binding supplies `(source-key, lead-id)`; bare-string shorthand `<key>` normalises to `{ key: <key>, lead: $SLICE_NAME }`. Cross-resolve every `key` against `plan.yaml.sources.<key>` and every `lead` against the `## Lead inventory` blocks in `discovery.md`; halt with `refine-binding-unresolved` on lookup failure.
2. **Create the slice directory** — `specrun slice create "$SLICE_NAME" --target <target> --format json`. CLI stamps `.metadata.yaml` at `refining`; this is the first step that materialises any path under `.specify/slices/$SLICE_NAME/`. On non-zero exit, surface the structured error and stop.
3. **Extract serially per binding** — walk `sources[]` in declaration order (no parallelism). For each `{ key, lead }`: `specrun source resolve <adapter> --format json` (where `<adapter>` is `plan.yaml.sources.<key>.adapter`), then invoke the adapter's `extract` brief with `<source-key>`, `<lead-id>`, and the bound `path` / `value`. The CLI host enforces the source adapter's WASI preopen contract (read-only `SOURCE_DIR` / `CAPABILITY_DIR`, write-only `SCRATCH_DIR`). Persist Evidence at `.specify/slices/$SLICE_NAME/evidence/<source-key>.yaml`; append `slice.extract.completed` with payload `{ slice-name, source-key }` as one NDJSON line on `.specify/journal.jsonl` (adjacency-tagged `{ timestamp, event, payload }`, kebab-case keys, per [`../../references/synthesis/tags.md`](../../references/synthesis/tags.md) §Journal-event hand-off and the worked line in [`../plan/fixtures/divergence-journal/journal.jsonl`](../plan/fixtures/divergence-journal/journal.jsonl)). On `source-extract-path-denied` / `source-extract-failed`, stop the loop and print the extract-failure closing hint; the slice stays `refining` and the operator amends the plan (`--remove-source` / `--add-source`) before re-running.
4. **Synthesise** — `specrun target resolve <target> --format json` to locate `adapters/targets/<target>/briefs/shape.md`; load every persisted Evidence YAML; write `proposal.md → spec.md → design.md → tasks.md` in fixed order per [`../../references/synthesis/substeps.md`](../../references/synthesis/substeps.md) (which in turn cites [`requirement-block.md`](../../references/synthesis/requirement-block.md), [`authority.md`](../../references/synthesis/authority.md), and [`claim-reconciliation.md`](../../references/synthesis/claim-reconciliation.md)). Tags never park the slice — proceed to step 5 regardless of tag count.
5. **Write `provenance.yaml`** — atomically (sibling temp file, then rename; partial writes must never land on disk) at `.specify/slices/$SLICE_NAME/provenance.yaml` per the block grammar in [`../../references/synthesis/provenance.md`](../../references/synthesis/provenance.md); one entry per `REQ-*` id in `spec.md`, every contributing `(source, claim-id)` pair, inline truncated `value`, `winner` markers, closed `resolution` enum, and `resolution-trace` on override paths. No `specrun slice reconcile` verb exists — the skill body is the writer and step 6 catches structural drift. After the rename succeeds, emit `slice.provenance.written` with payload `{ slice-name, generator: specify@<version>, requirement-count: <N> }` on `.specify/journal.jsonl`.
6. **Validate** — `specrun slice validate "$SLICE_NAME" --format json` runs the provenance parser (`ID:` / `Sources:` / `Status:` shape, closed `Status` enum, tag/Status coherence, `Sources:` keys cross-resolved against the slice's bindings), the Evidence schema validator, and the `spec.md ↔ provenance.yaml ↔ evidence` drift gate (exit 2 on `slice-provenance-drift`). The CLI renders a `DiagnosticReport` on stdout — `kind: violation` findings are blocking defects; `kind: review` findings are the semantic checks raised for your judgment and never block (read your worklist as `findings.filter(kind == "review")`). On success the CLI appends synthesis-tag journal events per [`../../references/synthesis/tags.md`](../../references/synthesis/tags.md) §Journal-event hand-off (one line per tagged requirement; CLI-owned). On non-zero exit, surface the stdout report plus the payload-free error envelope (its `error` is the gate discriminant) verbatim, do **not** transition, and print the validation-failure closing hint; common causes are malformed `REQ-NNN` id, `Sources:` key outside the slice's bindings, headline tag without matching `Status:`, Evidence missing `claim-id` on a `requirement` / `criterion` claim, or stale `provenance.yaml` (re-run step 5).
7. **Transition to refined** — `specrun slice transition "$SLICE_NAME" refined --format json`. CLI emits `slice.transition.refined`. Print the closing hint.

## Closing hint

On success:

```text
Slice <slice-name> refined. spec.md tags: <U> unknown, <C> conflict, <D> divergence. Review .specify/slices/<slice-name>/spec.md, then run /spec:build <slice-name> or resume /spec:execute.
```

On extract failure (any binding):

```text
Refine halted: source <source-key> extract failed (<error-code>). Slice <slice-name> stays refining. Amend the plan (specrun plan amend <slice-name> --remove-source <source-key>, or --add-source <source-key>=<lead-id>) and re-run /spec:refine <slice-name>.
```

On validation failure:

```text
Refine halted: synthesis output failed validation (<error-code>). Slice <slice-name> stays refining. Fix .specify/slices/<slice-name>/spec.md, re-run specrun slice validate <slice-name>, then specrun slice transition <slice-name> refined.
```

These three shapes are the contract `/spec:execute` matches when invoking refine as a loop step or as a breakout.

## References

- [`../../references/synthesis/`](../../references/synthesis/) — synthesis playbook (substeps, authority, requirement-block, claim-reconciliation, reconciliation, tags).
- [`../../references/synthesis/provenance.md`](../../references/synthesis/provenance.md) — provenance-index block grammar, truncation rule, `resolution` enum, and `resolution-trace` step names step 5 cites.
- [`adapters/targets/<target>/briefs/shape.md`](../../../../adapters/targets/) — per-target idiom guidance synthesis folds into `design.md`.
- [`adapters/sources/<adapter>/briefs/extract.md`](../../../../adapters/sources/) — per-source-adapter extract brief invoked in step 3.

## Guardrails

- **Lifecycle single-writer:** [shared guardrails](../../../../docs/standards/skill-guardrails.md#single-writer-for-lifecycle-state).
- **Never materialise `.specify/slices/$SLICE_NAME/` outside step 2.** `specrun slice create` is the sole writer; before step 2 the on-disk shape is plan-only.
- **Never run `extract` in parallel.** Bindings are processed in `slices[].sources[]` declaration order; deterministic goldens depend on it.
- **Never park the slice on tags.** `[unknown]` / `[conflict]` / `[divergence]` are review signals; the slice still transitions to `refined`. Validation failure (provenance / Evidence schema) is the only condition that keeps the slice in `refining` after extract succeeds.
- **Never invent provenance.** A `Sources:` key on a `spec.md` requirement that did not contribute a claim is a parser failure; let the tag surface instead.
- **Never call a `specrun slice synthesize` verb.** It does not exist; the four substeps are hand-coded per the playbook.
