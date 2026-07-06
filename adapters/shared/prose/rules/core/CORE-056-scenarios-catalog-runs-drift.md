---
id: CORE-056
title: Scenarios Catalog Runs Drift
severity: important
trigger: The scenario catalog's group tables disagree with the scenario files on disk or the committed run records.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-056-scenarios-catalog-runs-drift.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    config:
      catalog: evals/scenarios/README.md
      scenarios-dir: evals/scenarios
      runs-dir: evals/runs
      statuses: [pending, parked, passed, failed, deferred]
      gates: [release-blocker, full]
      status-result-map:
        passed: pass
        failed: fail
        deferred: deferred
    description: Run the `scenarios` framework checker's catalog↔runs check. The catalog path, the scenario and run directories, the legal Status / Gate value sets, and the status↔result agreement map are policy carried here, not in the tool.
---

## Rule

The scenario catalog (`evals/scenarios/README.md`), the scenario files (`evals/scenarios/<id>.md`), and the committed run records (`evals/runs/<id>.<result>.md`) must agree. The catalog is the single status surface and the run record is the contract behind every status flip (see the [record contract](../../../../evals/runs/README.md)): a status-bearing row (`passed` / `failed` / `deferred`) requires exactly one committed record whose `<result>` token agrees per this rule's `status-result-map`, and a `pending` or `parked` row must have no record at all.

This check is whole-tree: the `scenarios` framework tool parses every group-table row in the catalog (id from the File link), joins it against the scenario tree and the run-record filenames, and validates the Status and Gate cells against the closed value sets carried in this rule's `config:`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A catalog row whose linked scenario file does not exist, or a scenario file with no catalog row (including duplicate rows for one id).
- A `Status` or `Gate` cell outside the closed sets (`pending | parked | passed | failed | deferred`; `release-blocker | full`).
- A `passed` / `failed` / `deferred` row without its committed `evals/runs/<id>.<result>.md` record, or a record whose `<result>` disagrees with the row's status.
- A record filed against a `pending` or `parked` row, more than one record per id, a record naming an unknown scenario id, or a record filename that does not parse as `<id>.<result>.md`.

## Fix

Reconcile the three surfaces in one change: fix the catalog row (or scenario file) so they pair one-to-one, set Status / Gate to legal values, and commit exactly one run record per status-bearing row whose `<result>` matches the status (`passed` ↔ `pass`, `failed` ↔ `fail`, `deferred` ↔ `deferred`). For a triage or practice run, keep the summary with the run evidence and leave the catalog row `pending`.
