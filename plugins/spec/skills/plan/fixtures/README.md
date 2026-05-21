# `/spec:plan` scenario goldens

Reference outputs for the acceptance scenarios `/spec:plan` exercises end-to-end (per [rfc-25-workflow.md §Acceptance scenarios](../../../../../rfcs/rfc-25-workflow.md)). Each subdirectory pins the artifacts a clean `/spec:plan` run produces against the documented inputs; the harness re-runs the skill and diffs against these files.

| Fixture | Scenario | What it pins |
|---|---|---|
| [`intent-fix-typo/`](intent-fix-typo/) | #1 | Pure-intent N=1: `change.md` minimal, `plan.yaml` with `sources: [intent]` shorthand, three-section `discovery.md`. |
| [`documentation-account-revamp/`](documentation-account-revamp/) | #3 | Documentation source binding surfaces multiple candidates; one slice per candidate; clean propose without divergence. |
| [`cross-source-identity-revamp/`](cross-source-identity-revamp/) | #5e | Two source adapters surface the same candidate id; propose merges automatically; the uncertain pair is annotated `tentative: true` and called out in `change.md`. |
| [`divergence-journal/`](divergence-journal/) | propose-time divergence | One `plan.propose.divergence` JSON line emitted to `.specify/journal.jsonl` when propose sets `slices[].divergence: likely`. |

Closing-hint wording is identical across all scenarios:

```text
Plan `<name>` is at `pending`. Run `specify plan transition <name> reviewed` to stamp Gate 1, then `/spec:execute` to drive the slices.
```
