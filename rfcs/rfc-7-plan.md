# RFC-7 Implementation Plan

## Overview

RFC-7 introduces the `composition.yaml` artifact into the Specify/Vectis pipeline. The work spans two repositories: **specify** (prompt-engineering/docs) and **specify-cli** (Rust CLI). The RFC defines 5 phases, but Phases 3-5 are future work (Figma adapter, component library, web shell writer). This plan covers **Phases 1 and 2** — the core artifact, pipeline integration, and validation.

The plan is divided into **14 changes** across 4 tiers.

---

## Tier 0: Foundation (no dependencies)

These changes can all run **in parallel** — they touch independent files.

### Change 1: JSON Schema file

**Files:** `schemas/vectis/composition.schema.json` (new)

**Scope:** Create the JSON Schema from the Appendix A draft in the RFC. Validate it against the worked examples (skeleton and wired) from the RFC. This is a pure file creation — no dependencies on other changes.

**Dependencies:** None

### Change 2: Composition brief file

**Files:** `schemas/vectis/briefs/composition.md` (new)

**Scope:** Create the composition brief file with the exact content specified in the RFC (section "Brief Content"). The YAML frontmatter declares `id: composition`, `generates: composition.yaml`, `needs: [specs, proposal]`. The prose body contains the 7-step instructions (Identify Screens, Resolve Regions, Enrich with Bindings, Add Screen States, Add Overlays, Platform-Specific Regions, Surface Gaps) and the output structure template.

**Dependencies:** None

### Change 3: Update `schemas/vectis/schema.yaml` pipeline

**Files:** `schemas/vectis/schema.yaml`

**Scope:** Insert the `composition` stage between `specs` and `design` in the `define` pipeline. A single small edit: add `- id: composition` / `brief: briefs/composition.md` between the `specs` and `design` entries.

**Dependencies:** None (the brief file from Change 2 doesn't need to exist for the YAML to be valid, but logically they should land together)

---

## Tier 1: Specify-repo artifact & skill updates (depends on Tier 0)

These changes update existing briefs and skills to be composition-aware. They can all run **in parallel** with each other since they touch separate files.

### Change 4: Update specs brief

**Files:** `schemas/vectis/briefs/specs.md`

**Scope:** Strengthen view-naming guidance as described in RFC section "Impact on Existing Artifacts > `schemas/vectis/briefs/specs.md`". Add a note directing authors to name each distinct view explicitly in requirement titles so the composition brief can derive screen slugs.

**Dependencies:** Tier 0

### Change 5: Update design brief

**Files:** `schemas/vectis/briefs/design.md`

**Scope:** Per the RFC: update `needs` to `[proposal, specs]` (making the implicit specs dependency explicit). Add prose-based composition awareness: instruct the design brief to check for `composition.yaml` and adopt screen/ViewModel/field names proposed by the composition artifact. Add ViewModel adoption instructions and gap-surfacing guidance. Preserve backward compatibility (when composition is absent, infer as before).

**Dependencies:** Tier 0

### Change 6: Update tasks brief

**Files:** `schemas/vectis/briefs/tasks.md`

**Scope:** Per the RFC: add prose-based composition awareness. Express shell task dependency on `composition.yaml` when present. Add guidance that composition validation failure blocks shell tasks. No change to `needs`.

**Dependencies:** Tier 0

### Change 7: Update merge brief

**Files:** `schemas/vectis/briefs/merge.md`

**Scope:** Per the RFC: add instruction that `specify merge` handles both spec deltas (markdown) and composition deltas (YAML). Mention reviewing composition delta alongside spec changes in `specify spec preview` output.

**Dependencies:** Tier 0

### Change 8: Update define skill

**Files:** `plugins/spec/skills/define/SKILL.md`

**Scope:** Per the RFC: update for YAML output handling (dispatch on `generates` extension — `.yaml` files get YAML formatting validation), skeleton passthrough (the agent reads filesystem when the brief instructs), and change directory placement (`composition.yaml` alongside other artifacts).

**Dependencies:** Tier 0

### Change 9: Update ios-writer skill

**Files:** `plugins/vectis/skills/ios-writer/SKILL.md`

**Scope:** Per the RFC: add `composition.yaml` to the input list. Add Input Analysis table rows for screen regions, container structure, sizing, surface decoration, field bindings, event wiring, token references, conditional rendering, and iteration. Add mapping priority rule (composition present → deterministic layout; absent → inference fallback). Add platform-specific override handling. Add group-to-SwiftUI mapping details (HStack/VStack/ZStack, `.frame()` modifiers, styled containers).

**Dependencies:** Tier 0

### Change 10: Update android-writer skill

**Files:** `plugins/vectis/skills/android-writer/SKILL.md`

**Scope:** Mirrors Change 9 for Android: add `composition.yaml` input, Input Analysis table, mapping priority, platform overrides, group-to-Compose mapping (Row/Column/Box, Modifier, Card/Surface).

**Dependencies:** Tier 0

### Change 11: Update core-writer skill

**Files:** `plugins/vectis/skills/core-writer/SKILL.md`

**Scope:** Per the RFC: add a note to the Artifact-to-Code Mapping table that per-page view struct fields align with `composition.yaml` field bindings via `design.md`. Clarify that core-writer reads `design.md`, not `composition.yaml`.

**Dependencies:** Tier 0

---

## Tier 2: Build brief & validation (Phase 2 — depends on Tiers 0-1)

### Change 12: Update build brief (Phase 2)

**Files:** `schemas/vectis/briefs/build.md`

**Scope:** Per the RFC: add pre-shell validation gate (run composition validation checks before invoking shell writers — field coverage, event coverage, ViewModel mapping). Add `composition.yaml` to the shell writer handoff contract. Add severity-level handling (errors halt, warnings log).

**Dependencies:** Changes 1-3 (schema and brief must exist), Changes 9-10 (shell writer skills should be updated to know about composition)

---

## Tier 3: CLI changes (specify-cli repo)

These are the Rust implementation changes in the `specify-cli` repo. They are listed in dependency order. **Changes 13c and 13d can run in parallel** as they touch different CLI subcommands/crates. **Change 13e depends on 13c and 13d.** **Change 14 depends on all of Tier 2.**

### Change 13a: CLI — `specify change create` composition manifest

**Status: NOT NEEDED.** The CLI already tracks artifact completion dynamically via `PipelineView::completion_for` — it reads the pipeline briefs from `schema.yaml` and checks for each `generates` target in the change directory. Since Change 3 adds `composition` to the Vectis schema's define pipeline with `generates: composition.yaml`, the CLI automatically recognizes `composition.yaml` as an expected artifact. No code changes needed.

### Change 13b: CLI — `specify status` composition reporting

**Status: NOT NEEDED for basic tracking.** `collect_status` in `src/main.rs` already calls `pipeline.completion_for(Phase::Define, change_dir)` which dynamically reads the pipeline briefs. The new `composition` brief appears automatically in the artifacts map. Enhanced reporting (skeleton vs wired mode, screen count) is deferred — it would require parsing YAML content, which is a nice-to-have beyond the core Phase 1 deliverable.

### Change 13c: CLI — `specify validate` schema validation (Phase 1)

**Files:** `specify-cli` — `crates/validate/`

**Scope:** Add structural validation: parse `composition.yaml` against the JSON Schema. Report schema violations as errors. This is Phase 1 schema-only validation (no cross-artifact checks yet).

**Dependencies:** Change 1 (JSON Schema must exist)

### Change 13d: CLI — `specify spec preview` and `specify spec conflict-check` composition awareness

**Files:** `specify-cli` — `crates/spec/`

**Scope:** Include composition delta in dry-run merge preview. Check for composition conflicts (added screen already exists, modified screen changed in baseline).

**Dependencies:** Tier 0

### Change 13e: CLI — `specify merge` YAML delta merge with per-screen checksums

**Files:** `specify-cli` — `crates/merge/`

**Scope:** Add the YAML delta merge codepath: parse per-change `composition.yaml` delta, apply `added`/`modified`/`removed` operations to baseline, detect conflicts at screen-entry level using `.composition-checksums.yaml` with SHA-256 hashes. Implement the 7-step merge algorithm from the RFC (Parse, Validate delta structure, Process removed, Process added, Process modified, Write, Archive).

**Dependencies:** Changes 13a, 13c, 13d (the manifest, schema validation, and conflict-check infrastructure should exist)

### Change 14: CLI — `specify validate` cross-artifact checks (Phase 2)

**Files:** `specify-cli` — `crates/validate/`

**Scope:** Add cross-artifact validation: field coverage (every view struct field has a `bind`), event coverage (every shell-facing Event has an `event` wiring), ViewModel mapping (`maps_to` references valid variants from `design.md`), token resolution (references resolve to `tokens.yaml`), overlay trigger consistency, navigation graph consistency (`Navigate(X)` targets exist). Implement severity levels (error vs warning per the RFC table).

**Dependencies:** Change 13c (Phase 1 schema validation must exist), Tier 2 (build brief must reference these checks)

---

## Tier 4: Documentation & checks (can run after Tier 1)

### Change 15: Update `scripts/checks.ts` and project documentation

**Files:** `scripts/checks.ts`, `.cursor/rules/project.mdc`, `AGENTS.md`, `docs/` (if architecture docs exist)

**Scope:** Update `checks.ts` to recognize `composition.yaml` as a valid artifact in consistency checks. Update project-level documentation (`.cursor/rules/project.mdc`, `AGENTS.md`) to reference the composition artifact where relevant. Ensure `make checks` passes with the new artifact type.

**Dependencies:** Tiers 0-1 (all specify-repo changes should be landed)

---

## Dependency Graph

```
Tier 0 (parallel):
  [1] JSON Schema
  [2] Composition Brief
  [3] schema.yaml pipeline update

Tier 1 (parallel, after Tier 0):
  [4]  specs brief
  [5]  design brief
  [6]  tasks brief
  [7]  merge brief
  [8]  define skill
  [9]  ios-writer skill
  [10] android-writer skill
  [11] core-writer skill

Tier 2 (after Tiers 0-1):
  [12] build brief ←── [1,2,3,9,10]

Tier 3 (CLI, mostly parallel):
  [13a] change create ──────┐
  [13b] status reporting ───┤  (parallel, after Tier 0)
  [13c] validate schema ────┤
  [13d] preview/conflict ───┘
  [13e] merge YAML delta ←── [13a, 13c, 13d]
  [14]  validate cross-artifact ←── [13c, 12]

Tier 4 (after Tier 1):
  [15] docs & checks ←── [all specify-repo changes]
```

## Parallelism Summary

| Parallel batch | Changes |
|---|---|
| Batch A | 1, 2, 3 |
| Batch B (after A) | 4, 5, 6, 7, 8, 9, 10, 11 |
| Batch C (after B) | 12, 13a, 13b, 13c, 13d |
| Batch D (after C) | 13e, 15 |
| Batch E (after D) | 14 |

## Scope Notes

- **Phases 3-5** (Figma adapter, component library, web shell writer) are explicitly out of scope per the RFC's incremental adoption path.
- **Extract integration** is deferred beyond Phase 1 per the RFC — no changes to `plugins/spec/skills/extract/SKILL.md`.
- The CLI changes (Tier 3) are in the `specify-cli` repo and will need their own git branch/PR workflow separate from the `specify` repo changes.
- Each change is scoped to 1-3 files, keeping context requirements manageable for a single subagent.

## Implementation Learnings

1. **Changes 13a and 13b were not needed.** The CLI already tracks artifact completion dynamically via `PipelineView::completion_for` — it reads pipeline briefs from `schema.yaml` and checks for each `generates` target in the change directory. Adding `composition` to the Vectis schema's define pipeline (Change 3) was sufficient for the CLI to automatically recognize `composition.yaml` in `specify change create`, `specify status`, and `specify schema pipeline` output.

2. **The `checks.ts` integrity checks worked out of the box.** The `checkSchemaIntegrity` function dynamically collects pipeline entries from the `define` phase and verifies brief files, frontmatter IDs, and `needs` references. The new `composition` stage passed all checks without any `checks.ts` modifications.

3. **The merge crate needed a new `composition` module.** The existing markdown-based merge in `merge.rs` could not be reused for YAML composition merges. A new `composition.rs` module was added alongside the existing `merge.rs` with screen-level delta operations.

4. **Changes 13d and 13e were merged.** The preview/conflict-check and YAML delta merge both modify the same `change.rs` file in the merge crate, so they were implemented together.
