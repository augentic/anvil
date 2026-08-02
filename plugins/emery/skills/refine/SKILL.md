---
name: emery-refine
description: Refine one plan entry's slice to `refined` by invoking the `emery slice refine` orchestration and relaying its output. Use when driving one slice's extract-and-synthesis as a breakout; the `emery plan execute` loop runs the same orchestration itself.
argument-hint: "[slice-name]"
---

# Refine Skill

The CLI orchestration owns the whole refine flow — creating the slice (re-entry safe), extracting evidence from each bound source in turn, synthesizing the artifacts, persisting and validating them, and the `refined` transition. This skill only resolves the slice name, invokes the verb, and relays its output.

## Invocation

```bash
emery slice refine <slice-name>
```

Refine is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

When `[slice-name]` is omitted, run `emery plan status --quiet` and use the slice it names for the `refine` action; if the status names no refine action, surface the status output and stop.

## Relay

- Surface the CLI output verbatim — the persisted artifacts and any synthesis-tag counts (`[unknown]` / `[conflict]` / `[divergence]` are review signals for the operator, not failures — they never halt the loop).
- On non-zero exit, surface the structured error verbatim and stop. Never hand-edit slice artifacts to force progress — the synthesis kernel owns `model.yaml` and the rendered `ID:` / `Sources:` / `Status:` lines.
