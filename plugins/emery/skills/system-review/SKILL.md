---
name: emery-system-review
description: Record architectural authority over one exact wave handoff by invoking `emery system review` and relaying its output. Use after the operator has read a wave's current `handoffs/<digest>.yaml`; the fact is `system.wave.reviewed` and grants no product mutation authority.
argument-hint: <wave> --handoff <digest>
---

# System Review Skill

The CLI owns current-handoff selection (digest-exact against the live definition files, never time-based), the stale-review refusal, and the `system.wave.reviewed` fact append to `<system>/events/`. This skill elicits the wave and the reviewed digest, invokes the verb, and relays its output.

## Invocation

```bash
emery system review <wave> --handoff <digest> --quiet
```

Review is a short deterministic verb — it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug). The digest is the `handoffs/<digest>.yaml` filename stem (or the full `sha256:…` form) the operator actually read. Forward a non-CWD definition home as `--dir <home>`.

## Relay

- Surface the CLI output verbatim. `recorded: true` means the fact was appended; a re-review of the same handoff reports the read-only no-op — relay it without re-running anything.
- On `system-review-stale`, the supplied digest is not the wave's current handoff — the definition moved under the reviewer. Relay the error; the operator re-reads the named current handoff before reviewing again.
- On `system-review-handoff-stale`, no handoff matches the live files — relay the hint pointing at `emery system plan`, which reprojects the current handoff.
- Never elicit a digest by picking the newest file for the operator — the whole point of the gesture is that a human read that exact projection.
