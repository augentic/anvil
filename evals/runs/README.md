# Eval run records

Filled run-summaries for eval scenario runs. Run records are the audit trail; they are kept **separate from the scenario catalog** so the scenarios stay pristine fixtures and the catalog (`evals/scenarios/README.md`) tracks only status.

## How to read a run record

1. **Filename** — `<id>.<result>.md` (e.g. `documentation-one-slice.pass.md`).
2. **Title** — `# Run: <id> — **pass**` (or `fail` / `deferred`) is the headline verdict.
3. **Assertions** — every scenario assertion should be `pass` for an overall pass.
4. **Catalog** — [`evals/scenarios/README.md`](../scenarios/README.md) mirrors status (`passed` / `failed` / `deferred`).

## How to record a run

1. Copy [`evals/shared/run-template.md`](../shared/run-template.md) to `evals/runs/<id>.<result>.md` (e.g. `pure-intent.pass.md`).
2. Set the title and fill **Context**, **Assertions**, **Deviations**, **Notes**, and **Evidence**.
3. On `pass`, skip the fail-only sections at the bottom of the template.
4. On `fail` or `deferred`, add **Fault**, **Failure detail**, and (when useful) **Plan structure**; link a follow-up issue from **Notes**.
5. Update the scenario's status in the [catalog](../scenarios/README.md).
6. Retain the sandbox under `evals/.sandbox/<id>/`; use `scripts/snapshot.sh "$SANDBOX"` to inspect (see [`evals/shared/inspect.md`](../shared/inspect.md)).

These files are prose (no scenario frontmatter), so `specify lint framework` skips them. Commit them as the audit trail, or keep them with the run evidence for a fully local run.
