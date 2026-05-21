# `tests/fixtures/targets/omnia/`

Fixture for the Omnia target adapter (RFC-25 W2.5, acceptance scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief").

## What this fixture demonstrates

The Omnia target's `shape` brief carries idiom guidance — provider-based DI, WASM-Preview-2 guardrails, error-variant conventions, validation placement — that **core synthesis** (W3.1, `/spec:refine`) folds into a slice's `spec.md` and `design.md` regardless of source. Two fixtures (one pure-intent, one documentation-sourced) should produce artifacts that both honour the same idioms.

This fixture pins the *output* shape: it shows what synthesised `spec.md`, `design.md`, and `tasks.md` look like once the `shape` brief has been consumed, and the crate skeleton that `targets/omnia/briefs/build.md` produces from those artifacts.

## Layout

```text
tests/fixtures/targets/omnia/
├── README.md
├── input/                       # synthesised slice artifacts (post-shape-injection)
│   ├── spec.md                  # requirement blocks with ID / Sources / Status
│   ├── design.md                # provider trait deps, handler delegation, validation table
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

## How W3.1 / W3.4 consume this

- **W3.1 (`/spec:refine`)** — point a synthesis golden test at `input/`. The harness must verify that the synthesised artifacts include each section listed in `expected/shape-evidence.md` (provider trait dependencies, error mapping table, validation placement table, etc.). Both the pure-intent variant and a documentation variant of the same slice must produce these sections — the `shape` brief is what makes them appear.
- **W3.4 (`/spec:build`)** — point a build golden test at `input/` and assert that the produced crate matches `expected/crate/` modulo formatter passes. `cargo check` on the expected crate skeleton must pass (after dropping it into a workspace with `omnia-sdk` available).

## Status

This fixture documents the contract; it is **not yet executable end-to-end** because W3.1 (the synthesis library) and W3.4 (`/spec:build` skill body) have not landed. Once they ship, the harness referenced above can run the fixture through both verbs and capture goldens.

## See also

- `targets/omnia/briefs/shape.md` — the idiom guidance this fixture's `input/` reflects.
- `targets/omnia/briefs/build.md` — the orchestration this fixture's `expected/crate/` reflects.
- `rfcs/rfc-25-workflow.md` §Acceptance scenarios #5h.
