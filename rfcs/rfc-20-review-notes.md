# RFC-20 Review Notes — Plan Loop & `/change:analyze` After Survey

> Scratch notes from a review pass over [`rfc-20-survey.md`](rfc-20-survey.md). Two questions about the Abstract:
>
> 1. What is the plan skill's five-step loop, and does the proposed survey change preserve it?
> 2. What roles will `/change:analyze` play after the RFC has landed?

## 1. The plan skill's five-step loop

The "five-step loop" refers to the Critical Path that `/change:plan` runs on every invocation, defined in [`plugins/change/skills/plan/SKILL.md`](../plugins/change/skills/plan/SKILL.md):

1. **Parse and validate inputs** — validate `<change-name>` as kebab-case. Require at least one of `from`, `against`, `source`, or a populated `change.md:inputs`. Refuse if `plan.yaml` already exists (unless `extend`).
2. **Scaffold the brief and plan** — `specify change create <change-name> [--source <key>=<path-or-url> ...]`. Writes `change.md` and `plan.yaml` together (atomic refusal if either already exists). Skipped under `extend`.
3. **Run the plan brief pipeline** from `capability.yaml`:
   - **(a) Discovery** — invoke the discovery brief via `/change:analyze`; writes `discovery.md`. May surface a `## Proposed registry topology` block that triggers the **greenfield registry bootstrap** before step 3(b) when no `registry.yaml` exists yet.
   - **(b) Sync workspace** (multi-repo only) — discovery-time `specify workspace sync` (may sync all projects) + author `workspace.md`.
   - **(c) Propose** — run the propose brief; iterate accept/edit/reject/abort per slice; `specify change plan add` for each accepted slice.
   - **(d) Assignment** (multi-repo only) — infer `project` per entry; `specify change plan amend --project <project>`. When an unresolved row names a project that does not exist in `registry.yaml`, run the **registry-proposal sub-step** before continuing.
4. **Validate** — `specify change plan validate`. Non-zero exit on any `Error`-level finding. Never skip this step.
5. **Exit with hand-off summary** — point the operator at `specify change plan status` and `/change:execute loop`.

So: parse → scaffold → brief-pipeline → validate → hand-off, with the brief-pipeline step itself being a 2-step (single-repo) or 4-step (multi-repo) sub-sequence.

**Yes, RFC-20 preserves it explicitly.** From the Abstract:

> The plan skill's five-step loop, the single-writer invariant for `plan.yaml`, and the closed-kind enum's strict validation posture are preserved.

What changes is *only* the inner brief pipeline at step 3. RFC-20 inserts two new sub-steps:

- **3(b.5) Survey** — top-down DAG decomposition → `survey.md`
- **3(b.6) Synthesise** — appends `## Reconciliation` to `discovery.md`

See the "Pipeline ordering" table in `rfc-20-survey.md` (lines 339–349). Steps 1, 2, 4, and 5 are untouched, and step 3's outer wrapper is untouched — the brief sequence inside step 3 simply grows from 4 sub-briefs to 6. Survey and synthesise are also read-only with respect to `plan.yaml`, so the single-writer invariant the loop guards is unaffected.

## 2. `/change:analyze`'s role after RFC-20

`/change:analyze` keeps its current shape but gains one branch and one new sidecar consumer downstream.

### Still its job (unchanged contract)

- It remains the **sole** plan-time discovery skill, invoked once per input at step 3(a) — a "one-shot fan-out per source" (Abstract).
- Same positional arity: `<input-path> <output-dir> <kind> [source-key]` (Design §"CLI surface").
- For `legacy-code`: still produces capability blocks in `discovery.md` plus the existing structural sidecar at `<plan-dir>/analyze/<source-key>/metadata.json`.
- For `documentation`: still produces capability blocks in `discovery.md` only.
- Idempotency, sort order, and "unknown kinds are a hard exit" all preserved (Design Principle 5).

### New responsibilities added by RFC-20

1. **A third closed-enum kind: `domain-model`** (`{legacy-code, documentation, domain-model}`). The branch:
   - Validates a YAML/JSON document against the new schema at `specify-cli/schemas/domain-model/schema.json`.
   - Appends a `## Domain model` block (one per bounded context) to `discovery.md` under a stable wrapper, alphabetically sorted.
   - Writes a *second* structural sidecar at `<plan-dir>/analyze/<source-key>/domain-model.json` — the parsed, byte-canonicalised model. (See Design §"`domain-model` as a third closed-enum kind".)

2. **Becomes the upstream feeder for the new survey brief.** Survey consumes analyze's outputs but never re-invokes it:

   > `/change:analyze` remains a single fan-out at 3(a). The survey brief does **not** re-invoke analyze at deeper levels. […] This preserves analyze's per-source idempotency contract.
   > — `rfc-20-survey.md`, lines 351–352

   Specifically, survey reads `discovery.md`, every `metadata.json`, and every `domain-model.json` — analyze's three artefact types — but treats them as plan-time facts, not as a source to re-cluster. If a cut needs finer granularity than analyze produced, the surveyor records `unresolved: true` and defers to the operator rather than recursing into analyze.

3. **Per-capability `analyze.md` briefs grow a third branch.** The framework SKILL pins the third kind; the per-kind clustering/extraction prompt for `domain-model` lives in `plugins/change/skills/plan/briefs/<capability>/analyze.md` alongside the existing two (Design §"`domain-model` as a third closed-enum kind").

### What it explicitly is *not* asked to do

- Decomposition — that's the survey brief.
- Reconciliation — that's the synthesise brief.
- Spec extraction — that remains `/spec:extract` at define time (Non-Goals).

## Summary

Post-RFC-20, `/change:analyze`'s identity is unchanged — *plan-time, shallow, per-source capability inventory* — it just acquires one more input shape (`domain-model`) and becomes the structured-architectural-input feeder that anchors the new top-down survey. The plan skill's five-step loop is unchanged at the outer level; only the inner brief pipeline at step 3 grows two new read-only sub-steps.
