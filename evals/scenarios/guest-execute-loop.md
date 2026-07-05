---
id: guest-execute-loop
owner: scenarios
kind: suite
entrypoint: /spec:execute
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - guest-loop-drained
  - guest-journal-cadence
  - guest-generated-crate-verifies
  - guest-marker-released
  - guest-spec-sensible
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Composed guest execute loop (inverted)

Scenario ID: `guest-execute-loop`

> **RFC-61 Step 4 gate.** This scenario proves the inverted loop: the workflow guest — not the skill — drives `plan execute` inside the composed Omnia deployment, with judgment legs served by the live cursor backend and adapter dispatch served by the committed adapter guests.

## Intent

Prove the full inverted loop on a real model: a one-slice intent plan drains through the composed runtime's `plan execute` — survey and extract through the `source:intent` guest, synthesis through the workflow guest's judgment leg, build through the `target:omnia` guest (real codegen into the shared mount), merge folding the slice into the baseline — with the whole cadence journalled over the `"."` preopen and the generated crate passing its own verification.

The entrypoint names `/spec:execute` because the guest's `plan execute` is that skill's inverted counterpart: same plan contract, same Gate 1, same per-entry `done` writer — but the loop runs inside the wasm32 workflow guest against `omnia:model/completion` instead of inside skill markdown against a Cursor session.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md), except `init` uses a local adapter path (the guest resolves the target adapter project-locally; the out-of-tree manifest cache is not preopened): vendor `targets/omnia` from the sibling `augentic/specify-adapters` checkout into the sandbox at `adapters/targets/omnia/`, then `specify init ./adapters/targets/omnia`. `cursor-agent` must be on PATH and logged in. The [`guest-execute-loop` driver](../drivers/README.md) automates the clerical setup below and the invocation's runtime calls; driving through it is equivalent to typing the steps.

Create the plan with the native CLI (plan authoring stays native; the guest owns the loop):

```bash
specify plan create guest-demo --intent "Provide a greeting service with one operation that returns a fixed greeting string."
specify plan add greeting-service --sources intent
specify plan transition guest-demo approved     # Gate 1 — the operator's stamp
```

Write the deployment manifest: workflow guest (built by `cargo build -p specify-workflow-guest --target wasm32-wasip2` under `engine/`) plus the eight committed adapter `guest.wasm` files from the sibling checkout, each adapter's MCP shelf routed at `/mcp/<name>`, one writable `"."` mount at the sandbox — the checked-in [`engine/omnia.toml`](../../engine/omnia.toml) shape with the mount re-pointed. Export `SPECIFY_INTENT_MCP_URL` / `SPECIFY_OMNIA_MCP_URL` at the served `HTTP_ADDR` so the spawned cursor-agent can read the adapter shelves.

## Invocation

1. **Survey** — `cargo run -p specify-runtime -- run --config <sandbox>/omnia.toml -- source survey intent` (from `engine/`). The workflow guest dispatches to the `source:intent` guest, whose judgment leg runs on the live cursor backend; confirm one lead named `greeting-service` lands in `discovery.md`.
2. **Execute** — `… -- plan execute`. The guest loop claims the entry, refines (extract through the intent guest, synthesis through the workflow guest's own judgment leg), builds through the `target:omnia` guest (generated crate lands under `crates/` in the sandbox), merges, and exits drained.
3. **Verify the generated output** — `cargo check` (and `cargo test` where tests were generated) in the generated crate, per the [generated-output-correctness gate](../../docs/contributing/evals.md#fan-in--fan-out-proof).

## Assertions

- `guest-loop-drained`: the guest `plan execute` exits 0 reporting drained; the entry is `done`.
- `guest-journal-cadence`: the journal carries the full per-slice cadence (claim → extract → synthesize → build → merge → archive) written by the guest over the `"."` preopen.
- `guest-generated-crate-verifies`: the omnia build's generated crate passes `cargo check` — envelope validity alone does not count the slice done.
- `guest-marker-released`: the guest execute marker (`.specify/guest.lock`) is released on the clean exit.
- `guest-spec-sensible`: the synthesized baseline spec reads as a faithful, well-formed rendering of the intent.

## Scope

This is the **runtime-composition and live-judgment gate** for the inverted loop, not a planning-ergonomics scenario (that is `intent-only`). The deterministic substrate — argv/exit passthrough, link dispatch, the model-free merge leg, journal-append over the preopen, MCP shelves — is proven by the composed tests under `engine/crates/runtime/tests/` on every commit; this scenario admits only what those cannot reach: judgment legs against the live cursor backend end to end (source survey/extract, synthesis, target codegen) and the quality of what they emit.

## Negative expectations

Manual by design — see [`docs/contributing/evals.md`](../../docs/contributing/evals.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison. The checked-in driver is an operator replay script per the [drivers posture](../drivers/README.md), not CI automation.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
