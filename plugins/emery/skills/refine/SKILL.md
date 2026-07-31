---
name: emery-refine
description: Refine one plan entry's slice to `refined` by invoking the guest-routed `emery slice refine` orchestration and relaying its output. Use when driving one slice's extract-and-synthesis as a breakout; the `emery plan execute` loop runs the same orchestration itself.
argument-hint: "[slice-name]"
---

# Refine Skill

The engine guest owns the whole refine flow — slice create (re-entry safe), the serial per-binding extract fan-out, the synthesis judgment leg, the persist tail, validation, and the `refined` transition. This skill only resolves the slice name, invokes the verb, and relays its output.

## Invocation

```bash
RUST_LOG=info,opentelemetry=off,opentelemetry_sdk=off,omnia_wasi_otel=off \
  emery slice refine <slice-name>
```

Refine is a long-running orchestration — the `RUST_LOG` prefix (and the debug variant) follows the plugin rule's *Tracing and output* contract.

When `[slice-name]` is omitted, run `RUST_LOG=off emery plan status` and use the slice it names for the `refine` action; if the plan projects no refine action, surface the status output and stop.

## Relay

- Surface the CLI output verbatim — the persisted artifacts and any synthesis-tag counts (`[unknown]` / `[conflict]` / `[divergence]` are review signals, never a park).
- On non-zero exit, surface the structured error verbatim and stop. Never hand-edit slice artifacts to force progress — the synthesis kernel owns `model.yaml` and the rendered `ID:` / `Sources:` / `Status:` lines.
