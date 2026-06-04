---
id: VECTIS-005
title: Declared Platform Shells Must Be Present
severity: important
trigger: A Vectis project declares platforms in project.yaml and a proposal.md carries the platform set.
rule_hints:
  - kind: path-pattern
    value: '**/proposal.md'
    description: Any proposal that carries a ## Platforms section should be checked against the on-disk shell trees.
  - kind: tool
    value: vectis
    description: The vectis verify tool detects declared-but-absent shell trees.
---

## Rule

Every platform declared in `project.yaml.platforms` and carried through `proposal.md ## Platforms` must have a corresponding on-disk shell tree by the time the slice reaches `built`. The vectis verify tool (`specify tool run vectis -- verify --mode verify "${PROJECT_DIR}"`) is the deterministic authority: `core` requires `shared/src/app.rs`, `ios` requires the `iOS/` tree with at least one `.swift` file and the Crux bridge, `android` requires the `Android/` tree with at least one `.kt` file and the Gradle/Cargo bridge.

`web` and `desktop` are valid platform tokens but have no on-disk interpretation yet — the tool emits a `platform-not-yet-supported` info finding and treats them as present.

## Look For

- A `proposal.md ## Platforms` section that lists platforms not present on disk (no shell tree or an empty scaffold).
- A build report with `status: success` but missing `outputs[]` entries for declared supported platforms.
- A slice that reaches `built` without the verify gate having run.

## Spec Guidance

Platforms are an app-level fact carried to every slice. When a declared platform's shell tree is absent, plan-time reconciliation should have inserted a bootstrap slice to stand it up. If the shell is still missing at build time, the verify gate prevents the slice from reaching `built`.
