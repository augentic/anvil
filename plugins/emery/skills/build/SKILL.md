---
name: specify-build
description: Build a refined slice by invoking the guest-routed `specify slice build` orchestration and relaying its output. Use when building one slice as a breakout — after `/spec:refine`, or to retry after fixing a build failure; the `specify plan execute` loop runs the same orchestration itself.
argument-hint: "[slice-name]"
---

# Build Skill

The engine guest owns the whole build flow — request assembly and schema validation, the target adapter's build operation, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `refined → built` transition gate. This skill only resolves the slice name, invokes the verb, and relays its output.

## Invocation

```bash
specify slice build <slice-name>
```

When `[slice-name]` is omitted, run `specify plan status` and use the slice it names for the `build` action; if the plan projects no build action, surface the status output and stop.

## Relay

- Surface the CLI output verbatim. On success the orchestration has already stamped `built`; never write the lifecycle yourself.
- On non-zero exit, surface the structured error verbatim and stop; the slice stays `refined`, so re-running after a fix re-enters cleanly. Never patch adapters, templates, or cache in-band — during a build the agent is a consumer of Specify and adapters, not a maintainer.
