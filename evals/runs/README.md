# Eval run records

Filled run-summaries for eval scenario runs. Run records are the audit trail; they are kept **separate from the scenario catalog** so the scenarios stay pristine fixtures and the catalog (`evals/scenarios/README.md`) tracks only status.

## How to read a run record

1. **Filename** — `<id>.<result>.md` (e.g. `documentation-one-slice.pass.md`).
2. **Title** — `# Run: <id> — **pass**` (or `fail` / `deferred`) is the headline verdict.
3. **Assertions** — every scenario assertion should be `pass` for an overall pass.
4. **Catalog** — [`evals/scenarios/README.md`](../scenarios/README.md) mirrors status (`passed` / `failed` / `deferred`).

## How to record a run

1. Copy [`evals/shared/run-template.md`](../shared/run-template.md) to `evals/runs/<id>.<result>.md` (e.g. `intent-only.pass.md`).
2. Set the title and fill **Context**, **Assertions** (grading through the [assertion taxonomy](../shared/assertions.md)), **Deviations**, **Notes**, and **Evidence**.
3. On `pass`, skip the fail-only sections at the bottom of the template.
4. On `fail` or `deferred`, add **Fault**, **Failure detail**, and (when useful) **Plan structure**; link a follow-up issue from **Notes**.
5. Update the scenario's status in the [catalog](../scenarios/README.md).
6. Retain the sandbox under `evals/.sandbox/<id>/`; use `scripts/snapshot.sh "$SANDBOX"` to inspect (see [`evals/shared/inspect.md`](../shared/inspect.md)).

## Record contract

Flipping a catalog status (`passed` / `failed` / `deferred`) **requires** the matching committed record at `evals/runs/<id>.<result>.md` — the record is the contract behind the status, and `specify lint framework` enforces the agreement both ways (a status-bearing row without its record, a record disagreeing with its row, more than one record per id, or a record against a `pending` row are all drift). Keep at most one record per scenario id: a re-run replaces the old record in the same change that flips the status.

The fully-local allowance survives for **triage and practice runs only**: keep the filled summary with the run evidence and leave the catalog row untouched. The status flip happens when the record is committed.

These files are prose (no scenario frontmatter), so the scenario schema checks skip them.
