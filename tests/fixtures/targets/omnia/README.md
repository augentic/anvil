# `tests/fixtures/targets/omnia/`

Fixture for the Omnia target adapter (Wave 2.5, acceptance scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief").

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

## How the harness consumes this

- **`/spec:refine` synthesis** — `tests/cross_repo/targets_test.ts` parses `input/spec.md` with the W1.3 provenance parser (closed `Status:` enum + `Sources:` line + `ID:` block) and structurally validates that `expected/shape-evidence.md` is non-empty and bullet-headed.
- **`/spec:build` regeneration** — the harness asserts `expected/crate/Cargo.toml` declares `[package]` and that `expected/crate/src/lib.rs` is present alongside it. End-to-end byte-replay of the synthesised crate would require an LLM in the loop and is deferred to a follow-up RFC (see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md)).

## Status

The deterministic boundary the harness covers — provenance shape on `spec.md`, expected-crate structural shape, and the shape-evidence checklist — runs green on every `make test`. Byte-exact synthesis-replay against the LLM-driven skill bodies is intentionally out of scope and tracked separately.

## See also

- `targets/omnia/briefs/shape.md` — the idiom guidance this fixture's `input/` reflects.
- `targets/omnia/briefs/build.md` — the orchestration this fixture's `expected/crate/` reflects.
- `rfcs/done/rfc-25-workflow.md` §Acceptance scenarios #5h.
