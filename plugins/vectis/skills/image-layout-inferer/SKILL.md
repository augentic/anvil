---
name: vectis-image-layout-inferer
description: Reconstructs `layout.yaml` from one or more screenshot images using a staged vision-assisted pipeline (triage, chrome cropping, region / container / leaf inference, conservative `component:` emission, gap reporting), validates the output via `specify vectis validate layout`, and folds cross-artifact reference checks against sibling `tokens.yaml` / `assets.yaml` when present. Use when an operator supplies PNG or JPEG screenshots and wants a schema-valid, unwired layout document `/spec:define` can later wire into `composition.yaml`, when refining an existing `layout.yaml` from new screenshot evidence, or when the user mentions image-layout-inferer.
argument-hint: "<image-paths>"
---

# Vectis Image Layout Inferer

Convert screenshots of an application into a schema-valid `layout.yaml` document using the unwired subset of [`schemas/vectis/composition.schema.json`](../../../../schemas/vectis/composition.schema.json). The output is the input `/spec:define` consumes when generating wired `composition.yaml`.

This skill is a layout recovery tool, not a visual design extraction tool. It does not invent token names from pixels, does not crop production assets out of screenshots, and does not produce define-owned wiring (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation events, `*-when` keys). Those keys are reserved for `/spec:define`.

The producer surface every layout inferer shares (arguments, output rules, idempotence rules, `component: <slug>` emission policy, verification verbs, terminal-summary contents) lives in the shared contract: [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md). Read that document first; this SKILL.md adds only image-specific guidance on top.

## Critical Path (Quick Reference)

1. Run the **vision prerequisite check** by attempting to read at least one input image through the agent runtime's native attachment / file-read mechanism (RFC-11 §C). If the runtime cannot inspect images, exit 1 with the supported-runtimes message — never fall back to filename-based inference.
2. **Triage and crop.** Group inputs into screens / states, then crop platform chrome (status bars, navigation bars, browser chrome, emulator frames) when `--platform` is supplied or detected.
3. **Stage the recovery.** Walk top-down: regions (header / body / footer / fab / overlays / state replacements) → containers (rows, columns, lists, grids, cards, padding, gap, alignment, sizing, surface decoration) → leaves (text, controls, images, icons, fields).
4. **Detect candidate components conservatively.** Compare groups across screens for structural identity (§G); emit `component: <slug>` only when the operator confirms it or the same skeleton appears in **≥2 screens of the same run**, otherwise leave a `# candidate component: <slug>` comment. See [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#component-directive-emission).
5. **Reference siblings, never invent.** Use `tokens.yaml` / `assets.yaml` names only when they already resolve; otherwise emit raw values plus `# TODO` comments and a gap entry.
6. **Validate before writing.** Call `specify vectis validate layout [<output-path>]`; non-zero exit blocks the write. When sibling token / asset manifests exist, also call `specify vectis validate composition [<output-path>]` so cross-artifact resolution surfaces in the same run.
7. **Print the terminal summary** named in the contract: screens added, screens refined, warnings, unresolved gaps, source provenance entries appended, candidate components, exact output path.

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **This SKILL.md** — image-specific pipeline rules and prerequisite checks.
2. **[`references/layout-inferer-contract.md`](references/layout-inferer-contract.md)** — the producer-side contract every inferer shares (arguments, output, idempotence, component directive, verification, terminal summary).
3. **[`schemas/vectis/composition.schema.json`](../../../../schemas/vectis/composition.schema.json)** — the YAML the skill emits MUST validate against the unwired subset.
4. **Existing `layout.yaml` content** — preserve operator edits and previously emitted comments; refine, never overwrite.
5. **Source images** — reference for visible content only.
6. **Inferred suggestions** — emit as `# TODO` / `# candidate component` comments rather than committed YAML when in doubt.

## Arguments

The contract pins three common arguments (`--output`, `--baseline`, `--screen`). They MUST behave identically across every inferer; consult [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#common-arguments) for the canonical surface. The image-specific arguments below sit on top of that surface.

| Argument | Required | Description |
|---|---|---|
| `image-paths` | **Yes** | One or more PNG or JPEG file paths. Repeat or comma-separate to pass several images in a single run. |
| `--platform <ios\|android\|web>` | No | When supplied, helps the skill ignore system chrome and recognise platform conventions during triage and chrome cropping. |
| `--group <screen-slug>:<path>,<path>...` | No | Repeatable. Identifies images that represent the same screen (e.g. populated, empty, error states) so triage groups them deterministically rather than relying on visual similarity. |
| `--state <screen-slug>:<state-name>=<path>` | No | Repeatable. Explicit state mapping (`loading`, `empty`, `populated`, `error`) for a named screen. Wins over `--group` triage when both target the same image. |
| `--output <path>` | No | Inherited from the contract. Defaults to the active change directory's `layout.yaml`, then `design-system/layout.yaml`. |
| `--baseline <path>` | No | Inherited from the contract. Defaults to the existing output, then `design-system/layout.yaml`, then `.specify/specs/composition.yaml`. |
| `--screen <slug>=<hint>` | No | Inherited from the contract. The image inferer treats `<hint>` as a free-form note for screen-boundary disambiguation. |

Accepted image formats: PNG and JPEG only. HEIC, TIFF, PDF, SVG, WebP, and GIF MUST be converted before invocation; the skill MUST NOT invent a conversion step or call out to a hosted service. A `screen-slug` is kebab-case (`login`, `task-list`, `settings-detail`); a `state-name` is kebab-case (`loading`, `empty`, `populated`, `error`).

## Vision Prerequisite

The skill assumes the agent runtime can inspect attached images. The check is **positive**: at least one of the input image paths MUST be successfully read through the runtime's native attachment / file-read mechanism. The skill MUST NOT consult a host-provided "vision capability" flag (those are announced inconsistently across runtimes), and it MUST NOT fall back to filename-based or metadata-only inference.

When the check fails, exit `1` with a single-line message naming the supported runtimes:

```text
image-layout-inferer requires a runtime that can read attached images
(Cursor IDE, Claude Code, cursor-agent CLI, or any host that exposes image
attachments to the agent). Verify the runtime can open the image at <path>
and re-run.
```

## Pipeline

The pipeline mirrors RFC-11 §C and runs top-down. Each stage produces evidence the next stage refines; comments record uncertainty so the operator can act on it without re-running.

### 1. Triage

Group images into screens and states using explicit hints first, visual similarity second:

1. Apply every `--state <slug>:<name>=<path>` mapping. These bindings are authoritative.
2. Apply every `--group <slug>:<paths>` mapping for un-bound images.
3. For remaining images, group by visual similarity (header / chrome match, dominant content match) and propose screen slugs from visible titles. Emit `# screen <slug>: triaged from <path>, <path>` comments above each screen entry.
4. Single-image screens are accepted; the contract's "≥2 screens" rule governs `component:` emission, not screen recognition itself.

### 2. Crop platform chrome

Skip when `--platform` is absent and no chrome is detected. Otherwise remove:

- iOS: status bar, dynamic island / notch, software home indicator.
- Android: status bar, system navigation bar, gesture indicator.
- Web: browser chrome, devtools panes, surrounding OS chrome.
- Generic: emulator frames, screen recorder overlays, OS-level toasts that aren't part of the application.

Cropped regions never become `layout.yaml` content; record what was cropped in the terminal summary so reviewers can spot accidental application chrome that was treated as system chrome.

### 3. Infer regions

For each triaged screen, identify the schema's region keys:

- `header` (top app / navigation bar; `title`, `leading`, `trailing`).
- `body` (primary content area).
- `footer` (bottom app bar / tab bar / persistent action row).
- `fab` (floating action button — at most one per screen).
- `states.<name>` (replacement bodies for `loading`, `empty`, `error`, etc.; reuse the state names from `--state` when available, otherwise propose kebab-case names from visible cues like "no tasks yet" → `empty`).
- `overlays.<name>` (modals, sheets, dialogs, popovers, snackbars). Overlays MUST NOT include `trigger:` — that key is define-owned.
- `platforms.<ios|android>` (per-platform region overrides) — only when multiple `--platform` runs supply distinct chrome shapes for the same screen.

A region MAY be omitted when there is no visible evidence for it (e.g. a screen with no FAB).

### 4. Infer containers

Recover `group` nodes that organise content inside a region. For each visible container, pick the closest schema container kind:

- `group` with `direction: row` for horizontal layouts; `direction: column` for vertical stacks.
- `list` with `each: <bind-name>` when content is clearly a repeating row set. The `each:` value is a placeholder kebab-case name (`tasks`, `messages`); `/spec:define` rewires it to a real ViewModel binding. Use `style: plain` / `inset` / `grouped` only when iOS-style grouping is visually obvious.
- `grid` with `each:` and `columns:` (or `rows:`) when content is clearly a 2-D matrix.
- `form` for grouped settings rows / field stacks.
- `card`, `surface`, `divider` for explicit decoration affordances.

Recover layout properties when they are visually unambiguous: `gap`, `padding`, `align`, `justify`, `size: { width: fill | hug | <px> }`, `background`, `corner_radius`, `elevation`. Prefer schema-permitted scalar values (`md`, `lg`, `16`) plus `# TODO` comments over inventing a token name. The contract forbids inventing token names entirely (see [Token references](#token-references)).

### 5. Infer leaves

Leaves are the visible items the schema vocabulary supports:

- `text` (with `style`, `role`, `content`).
- `button`, `icon-button`, `link` (with `label`, `style`, optional `icon`).
- `icon` (with `name`).
- `image` (with `name` referencing `assets.yaml`).
- `field`, `checkbox`, `switch`, `radio`, `slider`, `segmented-control` (form controls).
- `progress-indicator`, `badge`, `chip`.
- `divider`, `spacer`.

For each leaf, copy the visible text content into `content:` / `label:` (preserving casing). If the text is unreadable or visibly truncated, emit `content: "<unreadable>"` plus a `# TODO: confirm text` comment and add the leaf to the unresolved-gaps summary.

### 6. Detect candidate components

Walk every screen produced by stages 3–5 and compare every `group` against every other `group` in the same run for **structural identity** (§G):

- Same ordered nested item kinds.
- Same nested-group shape.
- Same set of `*-when` keys *present* on nested groups (presence — not condition value — is part of the skeleton).
- `platforms.*` overrides participate only against other `platforms.<same>` overrides; the **base** skeleton (the keys outside `platforms.*`) MUST still match across all instances.

Apply the conservative emission policy from the contract:

- Promote to `component: <slug>` only when **either** the operator confirms a candidate (via a `--screen` hint, an existing `component:` slug already on the group, or a previous accepted `layout.yaml`) **or** the inferer observes ≥2 structurally identical groups across screens of the same run.
- Otherwise leave the groups flattened and emit a `# candidate component: <slug>` comment adjacent to each occurrence.
- Slugs MUST match `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` (kebab-case). Reserved region names (`header`, `body`, `footer`, `fab`) MUST NOT be used as slugs.
- Derive slugs from visible content (`task-row`, `setting-row`, `chip-tag`) — never from layout shape (`row-1`, `card-2`).

When in doubt, flatten and comment. Promoting a comment to a directive is cheap; demoting an emitted directive is operator work.

### 7. Emit gaps

Add comments adjacent to each affected node when:

- Grouping is ambiguous (e.g. two visually plausible group boundaries).
- Text is unreadable or truncated.
- Icon identity is uncertain (record `# TODO: confirm icon`; do **not** guess between e.g. `chevron-right` and `arrow-right`).
- A token reference is expected but no name resolves (`# TODO: tokenise gap 16`).
- An asset reference is expected but `assets.yaml` does not list the ID (`# TODO: add image '<id>' to assets.yaml`).
- A candidate component skeleton is borderline (`# candidate component: <slug>` — see Stage 6).

Each comment surfaced this way MUST also appear in the terminal summary's "Unresolved gaps" section so reviewers can scan a single block.

## Token references

- The skill MUST NOT reverse-engineer `tokens.yaml` from pixels.
- The skill MAY reference token names from a sibling `tokens.yaml` when an entry resolves cleanly to a visible value (a known `colors.primary.light` matches the visible button colour). Auto-discovery looks at `design-system/tokens.yaml` per the contract.
- Otherwise prefer raw layout values that the schema permits (`gap: 16`, `corner_radius: 8`) and emit `# TODO: replace measured gap 16 with spacing token` comments.
- The skill MUST NOT invent token names that do not appear in `tokens.yaml`.

## Asset references

- The skill MAY reference asset IDs from a sibling `assets.yaml` when an `assets.<id>` entry already exists. Auto-discovery looks at `design-system/assets.yaml` per the contract.
- Otherwise emit a `name:` placeholder paired with `# TODO: add <id> to assets.yaml` and an entry in the unresolved-gaps summary.
- The skill MUST NOT crop or extract production assets out of screenshots.

## Idempotence

Re-runs are additive and conservative; details live in [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#idempotence-rules). The image-specific applications are:

- Re-running against the same `layout.yaml` MAY add new screens, fill empty regions, or refine previously emitted leaves when new evidence supports the refinement.
- Operator-edited content (token names committed by hand, `component:` slugs the operator already accepted, `# TODO` comments retained from a previous run) MUST be preserved verbatim. The merge rule is: preserve existing structure, append new evidence, surface conflicts as comments and terminal warnings.
- When the new screenshots no longer contain a previously inferred element, emit a `# stale-source: …` comment next to it and a stale-source warning in the terminal summary. Do **not** delete the YAML.

## Mode detection

Before writing, classify the run:

- **Greenfield mode.** No existing `layout.yaml` at the resolved output path or `--baseline`. Stage outputs become the entire document; provenance starts at `provenance.sources[]: [{ kind: screenshots, captured_at: <ISO 8601> }]`.
- **Refine mode.** A `layout.yaml` (or wired `composition.yaml` consumed via `--baseline`) already exists. Diff the inferred output against the baseline screen-by-screen, group-by-group; apply the idempotence rules above; **append** to `provenance.sources[]` instead of replacing it.

Detection rule: if `--baseline` is supplied OR a file exists at the resolved `--output`, run in refine mode.

## Verification

Verification is the contract's deterministic gate; full surface lives in [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#verification). The image inferer MUST:

1. Run `specify vectis validate layout [<output-path>]` **before writing** (or before reporting success on an in-place refinement). Errors MUST block the write; warnings MUST be forwarded into the terminal summary but do not block.
2. Run `specify vectis validate composition [<output-path>]` after writing whenever a sibling `tokens.yaml` or `assets.yaml` exists at the canonical paths from `schemas/vectis/schema.yaml`'s `artifacts:` block. The CLI auto-invokes the `tokens` and `assets` modes when those siblings exist; reports surface in the same envelope.
3. Surface the validator output verbatim into the terminal summary (the operator should never have to re-run validation by hand to see what failed).

The skill MUST NOT roll its own schema, structural-identity, or cross-artifact reference validation. Every check the contract requires has an authoritative `specify vectis validate <mode>` verb; reimplementing them in skill prose causes drift.

Exit semantics:

- **Errors** — non-zero exit; nothing is written. Surface the report and stop.
- **Warnings only** — zero exit; the write proceeds and warnings appear in the terminal summary.
- **Clean** — zero exit silently except for the terminal summary itself.

## Terminal summary

Every run MUST conclude with the seven-item summary named in the contract ([`references/layout-inferer-contract.md`](references/layout-inferer-contract.md#terminal-summary)):

1. Screens added.
2. Screens refined.
3. Warnings (including stale-source and stale-directive warnings, plus warnings forwarded from `specify vectis validate layout` / `composition`).
4. Unresolved gaps (every `# TODO` and `# candidate component` comment emitted in this run, plus unresolved token / asset references).
5. Source provenance entries appended (one line per `provenance.sources[]` entry written — `kind: screenshots` for an image run, with the input image count).
6. Candidate components — both directives emitted (`component: <slug>` written into the YAML) and `# candidate component: <slug>` comments left for operator review.
7. Exact output path.

Image-specific additions the contract permits:

- "Cropped chrome" line per image: which platform chrome regions were removed during stage 2. Helps reviewers spot accidental application chrome.
- "Triage" line per screen: which input images contributed evidence (e.g. `triaged from screenshots/task-list-populated.png, screenshots/task-list-empty.png`).

These additions MUST NOT replace any of the seven required items.

## Fixtures

The skill ships paired regression fixtures under `fixtures/<name>/`:

- `fixtures/<name>/input.png` — the screenshot bundle that exercises a recovery path.
- `fixtures/<name>/expected.layout.yaml` — the layout the pipeline should produce.

Fixtures are operator-runnable references: they exist so a reviewer can replay the pipeline against a known input and diff against an accepted output. v1 does **not** enforce these in `make checks` (RFC-11 §C); a future RFC may promote them into a CI gate once the runner shape is established.

When adding a fixture, follow the existing convention:

- Use a synthetic graphic (no real product screenshots, no third-party imagery).
- Cover at least one new pipeline branch (e.g. a new region kind, a new state replacement, a new candidate-component skeleton).
- The expected YAML MUST validate cleanly under `specify vectis validate layout fixtures/<name>/expected.layout.yaml`.

## Operator ergonomics

- Optimise for **reviewable, bounded** runs. Operators SHOULD invoke the skill for one screen or one small coherent flow at a time, especially when refining an existing `layout.yaml`. Bulk-processing a 30-screen app in a single run is contract-legal but produces an un-reviewable diff.
- Multiple inputs in a single run are appropriate when they describe the same screen set (e.g. several state variants of one screen). Use `--state` and `--group` to keep triage explicit.
- Mixed-source reconciliation (image inputs combined with future Figma or code-source inputs) is **not** a v1 mode. Operators run each inferer separately against the same `layout.yaml`; the idempotence rules keep that workflow reviewable.

## See also

- [`references/layout-inferer-contract.md`](references/layout-inferer-contract.md) — the producer-side contract every layout inferer follows (arguments, output rules, idempotence rules, component-directive emission policy, verification, terminal summary).
- [RFC-11: UI Specification Workflow](../../../../rfcs/rfc-11-ui-spec.md) — normative source for §A (shared contract), §C (image inferer specifics), §G (component primitives), §J (skill naming + plugin layout).
- [`schemas/vectis/composition.schema.json`](../../../../schemas/vectis/composition.schema.json) — the schema both `layout.yaml` (unwired) and `composition.yaml` (wired) validate against.
- [`schemas/vectis/tokens.schema.json`](../../../../schemas/vectis/tokens.schema.json) and [`schemas/vectis/assets.schema.json`](../../../../schemas/vectis/assets.schema.json) — the sibling input schemas the cross-artifact reference checks consume when their files exist.
- [`schemas/vectis/schema.yaml`](../../../../schemas/vectis/schema.yaml) — the `artifacts:` block the CLI consults for default-path resolution.
