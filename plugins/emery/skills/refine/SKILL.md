---
name: emery-refine
description: Refine a closed plan by invoking the `emery plan refine` drain and relaying its output. Use after `/emery:plan` exits to generate every slice's specifications before any code work, or to re-run refinement after inputs change — re-running is the resume path; fresh slices are skipped.
argument-hint: [slice]...
---

# Refine Skill

The CLI orchestration owns the whole drain — the guest marker, topological order over `depends-on`, per-slice extract + synthesize + validate, the atomic `refinement.yaml` write, and every stop it reports. No code work happens here: no target build, workspace, wave, or authorization epoch. This skill invokes the drain and relays its output.

## Invocation

```bash
emery plan refine [--change-dir <dir>]
```

The drain inherits the Cursor workspace cwd as the change root. When that is not the change home, elicit `--change-dir` and pass it through.

The drain is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

When the operator names specific slices, forward each as one repeated `--slice` flag (selectors also pull in the stale or missing predecessors those slices need):

```bash
emery plan refine --slice <slice>
```

## Relay

- Surface the drain's output verbatim. On completion it prints the per-slice `refined` / `fresh (skipped)` lines and the canonical closing line pointing at `emery plan execute` — relay it as-is without adding another pointer.
- Gaps (`[unknown]` / `[conflict]` / `[divergence]`) are review outputs, not failures — when the output points at `emery plan gaps`, relay that line; do not treat it as an error.
- On `plan-refine-stopped` (exit 2), the drain already prints the canonical stop card on stdout beside the error envelope — relay both verbatim; fix the reported problem, then re-run `emery plan refine` (the drain skips fresh manifests and resumes the missing or stale work).
- On any other non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly.
- Never hand-edit slice artifacts or `refinement.yaml` — bundle artifacts are engine-owned between refine and execute; a direct edit is detected as staleness and re-refinement replaces it. Durable corrections travel through inputs (source material, `emery plan amend`, authority overrides).
