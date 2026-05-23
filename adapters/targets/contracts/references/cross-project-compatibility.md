# Cross-Project Compatibility

Cross-project producer-to-consumer compatibility reporting is deferred until a real consumer workflow needs it. The contracts target now relies on the declared contract WASI verifier for deterministic single-slice and merged-baseline validation:

```bash
specify tool run contract -- "$PROJECT_ROOT/contracts" --format json
```

Keep any future consumer-impact report adapter-owned. It should read `registry.yaml`, root `contracts/`, and consumer workspace snapshots directly, then classify findings only when a concrete workflow needs that product surface. Historical vocabulary reserved for that future report: `additive`, `breaking`, `ambiguous`, and `unverifiable`.
