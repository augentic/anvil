# Prompt evaluation

A manual native harness for evaluating reconciliation and synthesis prompt changes against a live model. It uses the same fixture adapter core as the deterministic suites, avoiding WASM build and hosting costs during prompt iteration.

## Run

Install and authenticate `cursor-agent`, then run from the repository root:

```bash
cargo make prompt-eval
```

The harness drives an adversarial lead set through plan authoring and execution. Hard assertions grade cross-source reconciliation, authority divergence, evidence gaps, provenance, lifecycle completion, and build output. It also reports request and repair counts as an early prompt-drift signal.

The temporary project path is printed at startup. Successful runs remove it; failed runs retain it for inspection.
