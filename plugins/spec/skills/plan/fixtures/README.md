# `/spec:plan` scenario goldens

Reference outputs for the stable scenario IDs `/spec:plan` exercises end-to-end (see the [lifecycle scenario catalog](https://specify.augentic.io/contributing/evals.html)). Each subdirectory pins the artifacts a clean `/spec:plan` run produces against the documented inputs; the harness re-runs the skill and diffs against these files.

| Fixture | Scenario | What it pins |
|---|---|---|
| [`intent-fix-typo/`](intent-fix-typo/) | #1 | Pure-intent N=1: `change.md` minimal, `plan.yaml` with one structured `{ source, lead }` binding under the auto-bound project, three-section `discovery.md`. |
| [`documentation-account-revamp/`](documentation-account-revamp/) | #3 | Documentation source binding surfaces multiple leads; one slice per lead; clean propose without divergence. |
| [`cross-source-identity-revamp/`](cross-source-identity-revamp/) | #5e | Two source adapters surface the same lead id; propose merges automatically; the uncertain pair is called out in `change.md` under `## Tentative merges`. |
| [`divergence-journal/`](divergence-journal/) | propose-time divergence | One `plan.amend.divergence` JSON line emitted to `.specify/journal.jsonl` by `specify plan amend <entry> --divergence likely` (the CLI is the single writer of `slices[].divergence`). |
| [`reconcile-journal/`](reconcile-journal/) | propose-time reconcile | Single `plan.reconcile.completed` JSON line emitted to `.specify/journal.jsonl` by `specify plan propose --from`; carries `plan-name`, `slice-count`, and `slice-names[]`. |

Closing-hint wording is identical across all scenarios:

```text
Plan `<name>` is at `pending`. Run `specify plan transition <name> approved` to stamp Gate 1, then `/spec:execute` to drive the slices.
```
