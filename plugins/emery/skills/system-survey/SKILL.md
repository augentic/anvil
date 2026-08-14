---
name: emery-system-survey
description: Survey the declared coverage of a definition home by invoking `emery system survey` and relaying its output. Use when the operator wants to recover an existing estate's `as-is` architecture from a hand-authored `scope.yaml` + `coverage.yaml`; re-running is the resume path.
argument-hint: [dir]
---

# System Survey Skill

The CLI orchestration owns the whole run — materializing each included source, the survey and extract legs, the lead and claim gates, coverage accounting, Evidence persistence, and the `as-is` correlation into `system.yaml`. This skill invokes the verb and relays its output.

## Invocation

```bash
emery system survey
```

The survey is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract. When the operator names a definition home other than the current directory, forward it:

```bash
emery system survey --dir <home>
```

## Relay

- Surface the CLI output verbatim: the per-source survey accounting (leads, observed tree, failures) and the persisted `as-is` sizes.
- A failed source is accounting, not a run failure — its coverage row records `survey-error` and the run continues; relay the failed lines without treating exit 0 as an error.
- On `system-scope-missing` / `system-coverage-missing`, relay the error and its hint verbatim — the hint prints the two-file template the operator authors by hand (there is no `system init`).
- On a gate stop (`system-survey-lead-limit`, `system-correlation-claim-limit`), relay the typed stop; recovery is narrowing coverage or authoring another definition home, never raising the engine constants.
- Never hand-edit `system.yaml`, `coverage.yaml`'s survey-owned fields, or anything under `evidence/` — re-running the survey is the only refresh path.
