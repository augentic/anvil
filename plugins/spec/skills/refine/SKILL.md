---
name: specify-refine
description: Refine a Specify slice — run extract per source, synthesise proposal/spec/design/tasks, validate, transition to refined. Use when an in-progress plan entry needs slice-time synthesis; not for first-time slice authoring without a plan (use /spec:plan).
argument-hint: "[slice-name]"
---

# Specify Refine

`/spec:refine` materialises one plan entry's slice directory, runs source `extract` serially, synthesises the four canonical artifacts from the resulting `Evidence[]` plus the target `shape` brief, validates the spec.md provenance, and transitions the slice to `refined`. The skill body owns the CLI choreography; the synthesis playbook at [`../../references/synthesis/`](../../references/synthesis/) owns what to write into the artifacts.

## Critical Path

1. **Resolve target and sources** — from `plan.yaml.slices[<slice>]`; each `sources[]` binding supplies `(source-key, candidate-id)`. Bare-string shorthand `<key>` normalises to `{ key: <key>, candidate: <slice-name> }`.
2. **Create the slice directory** — `specify slice create $SLICE_NAME --target <target>`. CLI stamps `.metadata.yaml` at `refining`; this is the first step that materialises any path under `.specify/slices/$SLICE_NAME/`.
3. **Extract serially per binding** — invoke each source adapter's `extract` brief once per binding in declaration order; persist `Evidence` at `.specify/slices/$SLICE_NAME/evidence/<source-key>.yaml`; emit `slice.extract.completed` per binding.
4. **Synthesise** — load the target `shape` brief and write `proposal.md → spec.md → design.md → tasks.md` per the synthesis playbook.
5. **Write `fusion.yaml`** — author the reconciliation index atomically at `.specify/slices/$SLICE_NAME/fusion.yaml` per [`../../references/synthesis/fusion.md`](../../references/synthesis/fusion.md); one entry per `REQ-*` id in `spec.md`, every contributing `(source, claim-id)` pair, inline truncated `value`, `winner` markers, closed `resolution` enum, and `resolution-trace` on override paths.
6. **Validate** — `specify slice validate $SLICE_NAME` runs the provenance parser, the Evidence schema validator, and the `spec.md ↔ fusion.yaml ↔ evidence` drift gate (exit 2 on `slice-fusion-drift`). Do not transition if validation fails.
7. **Transition to refined** — `specify slice transition $SLICE_NAME refined`. CLI emits `slice.transition.refined`. Print the closing hint.

## Step 1 — Resolve target and sources

Resolve `$SLICE_NAME` first:

- If the operator passed a slice name on the command line, use it verbatim.
- Otherwise, run `specify plan next --format json` and use the `name` of the returned in-progress entry. If `plan next` returns no in-progress entry, stop with `refine-no-active-slice` and tell the operator to either pass `[slice-name]` explicitly or claim a plan entry via `specify plan next` first.

Read `plan.yaml`; pick the `slices[]` entry whose `name == $SLICE_NAME`. Read its `target:` field and its `sources[]` list. Normalise every binding: a bare string `<key>` becomes `{ key: <key>, candidate: $SLICE_NAME }`; an object entry is taken as-is. Cross-resolve every `key` against the top-level `plan.yaml.sources.<key>` map; cross-resolve every `candidate` against the `## Candidate inventory` blocks in `discovery.md`. Halt with `refine-binding-unresolved` if either lookup fails.

## Step 2 — Create the slice directory

```bash
specify slice create "$SLICE_NAME" --target <target> --format json
```

The CLI handles kebab-case validation, directory creation under `.specify/slices/$SLICE_NAME/`, and the initial `.metadata.yaml` write at lifecycle `refining`. On non-zero exit, surface the structured error and stop — do not hand-roll any path under `.specify/slices/`. Before this step the on-disk shape is plan-only.

## Step 3 — Extract serially per binding

Walk `sources[]` in declaration order (serial, no parallelism). For each `{ key, candidate }` binding:

1. Resolve the source adapter via `specify source resolve <adapter> --format json`, where `<adapter>` is `plan.yaml.sources.<key>.adapter`.
2. Read the adapter's `extract` brief from the response. Invoke the brief with `<source-key>`, `<candidate-id>`, and the bound `path` / `value` from `plan.yaml.sources.<key>`. Source adapters run under the WASI preopen contract (read-only `SOURCE_DIR`, read-only `CAPABILITY_DIR`, write-only `SCRATCH_DIR`); the CLI host runner enforces the sandbox.
3. Persist the returned Evidence YAML at `.specify/slices/$SLICE_NAME/evidence/<source-key>.yaml`. The CLI validates against `evidence.schema.json` on read in the next step; emitting invalid YAML stays in `refining`.
4. Emit `slice.extract.completed` with payload `{ slice-name: $SLICE_NAME, source-key: <key> }` by appending one NDJSON line per event to `.specify/journal.jsonl` using the adjacency-tagged `{ timestamp, event, payload }` shape; field names are kebab-case per [`../../references/synthesis/tags.md`](../../references/synthesis/tags.md) §Journal-event hand-off and the worked line in [`../plan/fixtures/divergence-journal/journal.jsonl`](../plan/fixtures/divergence-journal/journal.jsonl).

If any `extract` returns a non-zero status or the host runner surfaces `source-extract-path-denied` (or `source-extract-failed`), stop the loop: no synthesis runs, the slice stays `refining`, and the closing hint names the failing source. Operator amends the plan (`specify plan amend $SLICE_NAME --remove-source <key>` to drop, or `--add-source <key>=<candidate-id>` to rebind) and re-runs `/spec:refine $SLICE_NAME`.

## Step 4 — Synthesise the four artifacts

Load the target `shape` brief via `specify target resolve <target> --format json` and read the `shape` brief from `adapters/targets/<target>/briefs/shape.md`. Load every persisted Evidence YAML. Then write the four artifacts in fixed substep order, following the synthesis playbook:

1. `proposal.md` — motivation, scope, non-goals. See [`../../references/synthesis/substeps.md`](../../references/synthesis/substeps.md).
2. `spec.md` — fused requirements with `ID:` / `Sources:` / `Status:` per [`../../references/synthesis/requirement-block.md`](../../references/synthesis/requirement-block.md). Apply [`../../references/synthesis/authority.md`](../../references/synthesis/authority.md) per fused `claim-id` group and [`../../references/synthesis/claim-fusion.md`](../../references/synthesis/claim-fusion.md) per claim kind.
3. `design.md` — fold the target `shape` brief plus design-side claims (`decision` / `section` / `excerpt` / `type` / `call`); include a `## UI / layout` H2 when spatial claims contribute.
4. `tasks.md` — flat `- [ ] …` checkbox list following the target's task skeleton.

Tags never park the slice — proceed to step 5 regardless of tag count.

## Step 5 — Write `fusion.yaml`

Author the reconciliation index at `.specify/slices/$SLICE_NAME/fusion.yaml` atomically (write to a sibling temp file, then rename); a partial write must never land on disk. There is no `specify slice fusion write` verb — the skill body is the writer and the validator in step 6 catches structural drift.

Follow the block grammar in [`fusion.md`](../../references/synthesis/fusion.md). After the atomic rename succeeds, emit `slice.fusion.written` with payload `{ slice-name: $SLICE_NAME, generator: specify@<version>, requirement-count: <N> }` where `<N>` is the number of `requirements[]` entries, by appending to `.specify/journal.jsonl`.

## Step 6 — Validate

```bash
specify slice validate "$SLICE_NAME" --format json
```

The CLI runs the spec.md provenance parser (`ID:` / `Sources:` / `Status:` shape; closed `Status` enum; tag/Status coherence; `Sources:` keys cross-resolved against plan-level bindings), the Evidence schema validator, and the `fusion.yaml` drift gate. On success it appends synthesis-tag journal events per [`../../references/synthesis/tags.md`](../../references/synthesis/tags.md) §Journal-event hand-off (one line per tagged requirement; CLI-owned — not skill-appended). On non-zero exit, surface the structured error verbatim; do **not** transition; the slice stays `refining`. Common causes: malformed `REQ-NNN` id, `Sources:` key not in the slice's bindings, headline tag without matching `Status:`, Evidence missing `claim-id` on a `requirement` / `criterion` claim, or `slice-fusion-drift` when `fusion.yaml` is stale w.r.t. `spec.md` or `evidence/*.yaml` (re-run step 5 to regenerate).

## Step 7 — Transition to refined

```bash
specify slice transition "$SLICE_NAME" refined --format json
```

The CLI emits `slice.transition.refined`. Print the closing hint below.

## Closing hint

On success:

```text
Slice <slice-name> refined. spec.md tags: <U> unknown, <C> conflict, <D> divergence. Review .specify/slices/<slice-name>/spec.md, then run /spec:build <slice-name> or resume /spec:execute.
```

On extract failure (any binding):

```text
Refine halted: source <source-key> extract failed (<error-code>). Slice <slice-name> stays refining. Amend the plan (specify plan amend <slice-name> --remove-source <source-key>, or --add-source <source-key>=<candidate-id>) and re-run /spec:refine <slice-name>.
```

On validation failure:

```text
Refine halted: synthesis output failed validation (<error-code>). Slice <slice-name> stays refining. Fix .specify/slices/<slice-name>/spec.md, re-run specify slice validate <slice-name>, then specify slice transition <slice-name> refined.
```

These three shapes are the contract `/spec:execute` matches when invoking refine as a loop step or as a breakout.

## References

- [`../../references/synthesis/`](../../references/synthesis/) — synthesis playbook (substeps, authority, requirement-block, claim-fusion, fusion, tags).
- [`../../references/synthesis/fusion.md`](../../references/synthesis/fusion.md) — reconciliation-index block grammar, truncation rule, `resolution` enum, and `resolution-trace` step names step 5 cites.
- [`adapters/targets/<target>/briefs/shape.md`](../../../../adapters/targets/) — per-target idiom guidance synthesis folds into `design.md`.
- [`adapters/sources/<adapter>/briefs/extract.md`](../../../../adapters/sources/) — per-source-adapter extract brief invoked in step 3.

## Guardrails

- **Lifecycle single-writer:** [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- **Never materialise `.specify/slices/$SLICE_NAME/` outside step 2.** `specify slice create` is the sole writer; before step 2 the on-disk shape is plan-only.
- **Never run `extract` in parallel.** Bindings are processed in `slices[].sources[]` declaration order; deterministic goldens depend on it.
- **Never park the slice on tags.** `[unknown]` / `[conflict]` / `[divergence]` are review signals; the slice still transitions to `refined`. Validation failure (provenance / Evidence schema) is the only condition that keeps the slice in `refining` after extract succeeds.
- **Never invent provenance.** A `Sources:` key on a `spec.md` requirement that did not contribute a claim is a parser failure; let the tag surface instead.
- **Never call a `specify slice synthesize` verb.** It does not exist; the four substeps are hand-coded per the playbook.
