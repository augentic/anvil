# Eval scorecards — the release-gate record

One dated document per graded eval run (remediation Phase 4 item 3), written by the operator-invoked runner in [`emery-adapters/examples/eval`](https://github.com/augentic/emery-adapters/tree/main/examples/eval) and committed here verbatim as `<date>.md`. Both the Create Release and Publish Release workflows refuse to cut unless one scorecard is `status: green`, covers the complete case catalog, and its `emery-sha:` names the workflow's tip sha (`scripts/scorecard-gate.sh`; CONSTITUTION invariant 6) — CI verifies the record, it never runs the live eval. A filtered (single-case) run is an iteration aid; the runner never marks it green.

The machine-readable lines the gate greps, exactly as the runner renders them:

```markdown
- status: green
- emery-sha: <full sha of the augentic/emery commit the binary was built from>
- adapters-sha: <full sha of the augentic/emery-adapters commit>
- catalog: complete
```

The rest of the document carries the product.md numbers (time to first reviewable specification ≤30 min; per-operation success ≥95%) and per-case outcomes; anything unmeasured stays `unconfirmed`, never silently green.
