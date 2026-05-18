---
name: vectis-image-layout-inferer
description: Reconstruct an unwired `layout.yaml` from screenshot images via a staged vision pipeline (triage, cropping, region / container / leaf inference, `component:` emission, gap reporting); validate via `specify tool run vectis` and cross-check sibling `tokens.yaml` / `assets.yaml`. Use when screenshots are the only source for a layout `/spec:define` will wire into `composition.yaml`, or when an existing layout needs new visual evidence; not when `layout.yaml` is already the source of truth.
argument-hint: <image-path>...
---

# Vectis Image Layout Inferer

> **Layout recovery, not visual design extraction.** Convert screenshots into a schema-valid `layout.yaml` document using the unwired subset of [`composition.schema.json`](../../../../adapters/vectis/composition.schema.json). Never invent token names from pixels, never crop production assets, never emit define-owned wiring (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation events, `*-when` keys). The producer surface every layout inferer shares lives in [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md); read it first.

## Critical Path

1. Run the **vision prerequisite check** by attempting to read at least one input image through the agent runtime's native attachment / file-read mechanism. If the runtime cannot inspect images, exit 1 with the supported-runtimes message — never fall back to filename-based inference.
2. **Triage and crop.** Group inputs into screens / states, then crop platform chrome (status bars, navigation bars, browser chrome, emulator frames) when `platform` is supplied or detected.
3. **Stage the recovery.** Walk top-down: regions (header / body / footer / fab / overlays / state replacements) → containers (rows, columns, lists, grids, cards, padding, gap, alignment, sizing, surface decoration) → leaves (text, controls, images, icons, fields).
4. **Detect candidate components conservatively.** Compare groups across screens for structural identity (§G); emit `component: <slug>` only when the operator confirms it or the same skeleton appears in **≥2 screens of the same run**, otherwise leave a `# candidate component: <slug>` comment. See [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#component-directive-emission).
5. **Reference siblings, never invent.** Use `tokens.yaml` / `assets.yaml` names only when they already resolve; otherwise emit raw values plus `# TODO` comments and a gap entry.
6. **Stage, validate, then rename.** Write the inferred YAML to `<output-path>.tmp`, run `specify tool run vectis -- validate layout <output-path>.tmp` (and `specify tool run vectis -- validate composition <output-path>.tmp` when sibling token / asset manifests exist), and only on a clean / warnings-only result atomically rename onto `<output-path>`. Errors delete the staging file and exit non-zero; the previous `<output-path>` is preserved untouched.
7. **Print the terminal summary** named in the contract: screens added, screens refined, warnings, unresolved gaps, source provenance entries appended, candidate components, exact output path.

## Orientation

The image inferer is one producer behind a shared layout-inferer contract. Common arguments (`output`, `baseline`, `screen`) and their defaults come from that contract; image-specific arguments add `image-paths`, `platform`, `group`, and `state`. PNG and JPEG are the only accepted inputs — every other format must be converted by the operator before invocation.

The pipeline is strictly top-down. Triage groups inputs into screens / states (explicit `state` mappings beat `group` mappings beat visual similarity), platform chrome is cropped per-platform, then regions / containers / leaves are inferred in turn against the schema vocabulary. Candidate-component detection compares groups for structural identity (ordered nested kinds, nested-group shape, presence of `*-when` keys) and only promotes to `component: <slug>` after operator confirmation or ≥2 structurally identical occurrences in the same run; everything else stays flat with a `# candidate component:` comment. Gaps surface as `# TODO` comments adjacent to the affected node and are repeated in the terminal summary.

Token and asset references resolve only against existing siblings (`tokens.yaml` / `assets.yaml`, auto-discovered at `design-system/`); raw values plus `# TODO` comments are preferred over invention. Idempotent re-runs preserve operator edits verbatim, append new evidence, and surface conflicts as comments and warnings — never delete YAML, even when source evidence has gone stale (a `# stale-source: …` comment fires instead).

Mode is detected automatically: greenfield when no `layout.yaml` exists at the resolved output or `baseline`, refine otherwise. Both modes stage to `<output-path>.tmp`, run `specify tool run vectis -- validate layout` (plus `validate composition` when sibling token / asset manifests are present), and only atomically rename onto `<output-path>` on a clean or warnings-only result. The terminal summary is mandatory and must contain the seven items the contract names; image-specific "Cropped chrome" and "Triage" lines may be added on top.

See [`references/runbook.md`](references/runbook.md) for the operational detail (authority hierarchy, full argument table, vision prerequisite text, every pipeline stage 1–7, token / asset rules, idempotence, mode detection, verification mechanics, terminal summary, fixtures, operator ergonomics).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Authority hierarchy, full argument table, vision prerequisite, pipeline stages 1–7, token / asset rules, idempotence, mode detection, verification mechanics, terminal summary, fixtures, operator ergonomics |
| [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md) | Producer-side contract every layout inferer follows (arguments, output rules, idempotence, component directive emission, verification, terminal summary) |
| [`fixtures/`](fixtures/) | Paired regression fixtures: `<name>/input.png` + `<name>/expected.layout.yaml` |
| [`../../../../adapters/vectis/composition.schema.json`](../../../../adapters/vectis/composition.schema.json) | Schema both `layout.yaml` (unwired) and `composition.yaml` (wired) validate against |
| [`../../../../adapters/vectis/tokens.schema.json`](../../../../adapters/vectis/tokens.schema.json) | Sibling input schema cross-artifact reference checks consume when present |
| [`../../../../adapters/vectis/assets.schema.json`](../../../../adapters/vectis/assets.schema.json) | Sibling input schema cross-artifact reference checks consume when present |
| [`../../../../rfcs/archive/rfc-11-ui-spec.md`](../../../../rfcs/archive/rfc-11-ui-spec.md) | RFC-11: normative source for §A (shared contract), §C (image inferer specifics), §G (component primitives), §J (skill naming + plugin layout) |

## Guardrails

- **NEVER invent token names from pixels, crop production assets out of screenshots, or emit define-owned wiring** (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation events, `*-when` keys). Reference siblings only when they already resolve; otherwise emit raw values plus `# TODO` comments and a gap entry.
- **NEVER write `<output-path>` directly or roll your own validation.** Stage to `<output-path>.tmp`, run `specify tool run vectis -- validate layout` (and `validate composition` when sibling manifests exist), and atomically rename only on a clean or warnings-only result; on errors, delete the staging file and exit non-zero with the validator output verbatim in the terminal summary.
- **NEVER delete YAML on re-runs.** Operator edits, accepted `component:` slugs, and prior `# TODO` comments are preserved verbatim; stale evidence fires `# stale-source: …` comments and a warning, not a deletion. Promote candidate components only on operator confirmation or ≥2 structurally identical groups in the same run.
