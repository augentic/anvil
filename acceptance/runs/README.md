# Acceptance run records

Filled run-summaries for manual `lifecycle` scenario runs. Run records are the audit trail; they are kept **separate from the scenario catalog** so the scenarios stay pristine fixtures and the catalog (`acceptance/suites/lifecycle/README.md`) tracks only status.

## How to record a run

1. Copy [`acceptance/suites/shared/run-summary-template.md`](../suites/shared/run-summary-template.md) to `acceptance/runs/<id>-<date>.md` (e.g. `pure-intent-2026-06-04.md`).
2. Fill every section that the scenario's declared `stages` covers; mark the rest `n/a`.
3. Set the **Verdict** to `pass` / `fail` / `deferred`.
4. Update the scenario's status in the [catalog](../suites/lifecycle/README.md).
5. On `fail` or `deferred`, file a follow-up issue in `augentic/specify` and link it from the run record.

These files are prose (no scenario frontmatter), so `specify lint framework` skips them. Commit them as the audit trail, or keep them with the run evidence for a fully local run.
