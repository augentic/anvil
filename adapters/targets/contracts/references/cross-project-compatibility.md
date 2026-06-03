# Cross-Project Compatibility

Cross-project producer-to-consumer compatibility reporting is deferred until a real consumer workflow needs it. The contracts target now relies on the declared contract WASI verifier for deterministic single-slice and merged-baseline validation:

```bash
specify tool run contract -- "$PROJECT_ROOT/contracts" --format json
```

Keep any future consumer-impact report adapter-owned. It should read `registry.yaml`, root `contracts/`, and consumer workspace snapshots directly, then classify findings only when a concrete workflow needs that product surface. Historical vocabulary reserved for that future report: `additive`, `breaking`, `ambiguous`, and `unverifiable`.

These classifiers are contract-domain evidence fields — **not** the closed `LintFinding` severity enum (`critical` / `important` / `suggestion` / `optional`). When a future consumer-impact report surfaces findings as `LintFinding` records (see `schemas/diagnostics/diagnostic.schema.json` and §"Relationship to contracts and compatibility"), the closed severity enum sits on the envelope while the classifier and the rest of the consumer-impact payload travel inside `evidence.kind: structured` under `evidence.data`. The two dimensions are independent: a `breaking` change may carry any `LintFinding` severity, and an `additive` change may still warrant `suggestion` or `optional`.
