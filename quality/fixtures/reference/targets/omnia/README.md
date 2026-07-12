# `evals/fixtures/targets/omnia/`

Fixture for the Omnia target adapter (eval scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief").

## What this fixture demonstrates

The Omnia target's `shape` brief carries idiom guidance — provider-based DI, WASM-Preview-2 guardrails, error-variant conventions, validation placement — that **core synthesis** (`/spec:refine`) folds into a slice's `spec.md` and `design.md` regardless of source. Two fixtures (one pure-intent, one documentation-sourced) should produce artifacts that both honour the same idioms.

This fixture pins the *output* shape: it shows what synthesised `spec.md`, `design.md`, and `tasks.md` look like once the `shape` brief has been consumed, and the crate skeleton that `targets/omnia/briefs/build.md` produces from those artifacts.

## Layout

```text
evals/fixtures/targets/omnia/
├── README.md
├── input/                       # synthesised slice artifacts (post-shape-injection)
│   ├── spec.md                  # requirement blocks with ID / Sources / Status
│   ├── design.md                # provider trait deps, operation delegation, validation table
│   └── tasks.md                 # build sequence the build brief expects
└── expected/
    ├── shape-evidence.md        # checklist of shape-derived sections present in spec/design
    └── crate/                   # crate skeleton build.md would emit
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            ├── error.rs
            └── handlers.rs
```

## Status

This fixture is a documentation pin for the target adapter. No automated harness walks these target fixtures; executable replay of `/spec:refine` and `/spec:build` remains deferred to a future agent/CLI harness.

## See also

- `targets/omnia/briefs/shape.md` — the idiom guidance this fixture's `input/` reflects.
- `targets/omnia/briefs/build.md` — the orchestration this fixture's `expected/crate/` reflects.
- [`target-shape` scenario](../../../scenarios/target-shape.md) and the [evals entry point](../../../../docs/contributing/evals.md).
