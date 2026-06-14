# Layout Inferer Contract

The producer-side contract every Vectis layout inferer follows. It pins the argument surface, the output shape, the idempotence rules, the component-directive emission policy, and the deterministic verification step that every inferer MUST run before reporting success.

The contract is source-agnostic and self-normative for current Vectis layout inferers. The first-pass implementer is the `vectis-image-layout-inferer` skill; planned future producers (`vectis-figma-layout-inferer`, `vectis-code-layout-inferer`) reuse the same contract until their own maintained reference amends it. Source-specific arguments (image input lists, Figma file IDs, source-tree paths) live in each skill's `SKILL.md`; this document covers only the surface every inferer shares. Tool-owned Vectis schemas and validators are the deterministic authority for output shape.

## Common arguments

Every layout inferer accepts the following arguments. Source-specific arguments (e.g. `--figma-json`, `--source`, image positionals) MAY be added by individual skills but MUST NOT replace any of these.

| Argument | How it is used | Default | Precedence |
|---|---|---|---|
| `--output <path>` | Names the exact file the inferer should write. | Active slice directory's `layout.yaml` (`.specify/slices/<name>/layout.yaml`); falls back to `design-system/layout.yaml` when no slice is active. | Explicit `--output` wins over every project-side default. |
| `--baseline <path>` | Existing `layout.yaml` (or wired `composition.yaml`) the inferer should refine rather than overwrite. | Existing output-path content; then `design-system/layout.yaml`; then `.specify/specs/composition.yaml`. | Explicit `--baseline` wins over discovered local or baseline files. |
| `--screen <slug>=<hint>` | Repeatable screen-boundary hint. The hint is source-specific (frame ID, screenshot group name, source-code view entrypoint). | None — inferers derive screen candidates from their source material. | Hints constrain or name inferred candidates; they MUST NOT force schema-invalid output. |

Argument placeholders:

- `<path>` — local file or directory path, relative or absolute (e.g. `screenshots/login.png`, `.specify/slices/onboarding/layout.yaml`).
- `<slug>` — stable kebab-case identifier for a logical screen (e.g. `login`, `task-list`, `settings-detail`).
- `<hint>` — source-specific evidence that helps name or bound a screen.

Arguments deliberately excluded from the common surface:

- `--slice-dir <path>` — redundant with default active-slice discovery plus `--output` for explicit routing. When active-slice detection is ambiguous, operators pass `--output .specify/slices/<name>/layout.yaml`.
- `--tokens <path>` / `--assets <path>` — inferers SHOULD auto-discover `design-system/tokens.yaml` and `design-system/assets.yaml` for reference checks. Non-standard locations wait for demonstrated demand or live in source-specific arguments on individual skills.

## Operator ergonomics

- Optimise for **reviewable, bounded** runs. Operators SHOULD invoke an inferer for one screen or one small coherent flow at a time, especially when refining an existing `layout.yaml`.
- Inferers MAY accept multiple inputs in a single run when those inputs clearly describe the same screen set (e.g. several screenshot states of one screen). They SHOULD NOT bulk-process unrelated screens silently.
- To accumulate layout information in a single change, run the inferer repeatedly against the same `layout.yaml` (the idempotence rules below keep the output reviewable).
- **Mixed-source reconciliation is not a v1 mode.** When more than one source kind contributes to a layout, runs are reviewed one at a time against the same `layout.yaml`; future RFCs MAY define a richer multi-source workflow.

## Output rules

- Inferers MUST emit `layout.yaml` documents using the **unwired subset** of [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json). Allowed structure is a full `screens` document with screen names, regions, groups, the item vocabulary, token references, asset references, the optional `component: <slug>` directive on groups (see [Component directive emission](#component-directive-emission)), states, overlays without `trigger`, and `platforms.*` overrides.
- A layout document MUST NOT use the change-local `delta` shape. `delta` is reserved for the wired `composition.yaml` lifecycle artifact.
- The unwired subset forbids define-owned wiring. Inferers MUST NOT emit any of:
  - `maps_to`
  - `bind`
  - `event`
  - `error`
  - overlay `trigger`
  - navigation targets encoded inside event values
  - conditional visual keys that end with `-when` (e.g. `strikethrough-when`, `disabled-when`)
- Inferers MAY use **token references** when the source supplies a named token, variable, or style that confidently maps to an entry in `tokens.yaml`. Otherwise they SHOULD prefer raw layout values that the composition schema permits and add `# TODO` comments where tokenisation is expected later. Inferers MUST NOT invent token names.
- Inferers MAY reference **asset IDs** only when those IDs resolve through `assets.yaml`, or when the reference is paired with a `# TODO` comment asking the operator to add the missing inventory entry. Inferers MUST NOT crop or extract production assets from source material.
- Inferers MAY emit the `component: <slug>` directive only under the rules in [Component directive emission](#component-directive-emission). The directive belongs to the unwired subset and `/spec:refine` MUST preserve it; inferers stay conservative so refine-time review remains the operator's call.
- Inferers MUST append to `provenance.sources[]` rather than replacing it. The composition schema's provenance vocabulary is `figma`, `legacy`, `manual`, `screenshots`, and `code` (Appendix F.1). `screenshots` and `code` are the new entries reserved for the image and future code paths; `legacy` remains valid for broad source-code migration runs.
- Multi-source output is a single `layout.yaml`. Per-screen provenance is represented through comments adjacent to screen entries in v1, not a schema change. A future schema bump MAY promote per-screen provenance into structured metadata.

## Symbol promotion policy

When an inferer encounters an `icon` / `icon-button` item with no matching
`assets.<id>` entry, apply the inference-time symbol exception:

1. **Known asset** — reference the existing id.
2. **Generic platform glyph** — MAY add a `kind: symbol` entry with
   `inferred: true` and `symbols.ios` / `symbols.android` mappings, **or** pair
   the layout reference with `# TODO: promote <slug> as kind: symbol` for
   operator approval before merge.
3. **Branded / custom shape** — emit `# TODO: add <id> to assets.yaml` only;
   never auto-symbol.

Symbol promotion is inventory authoring. Layout inferers MUST NOT treat symbol
entries as permission for shell writers to substitute platform glyphs for
`vector` / `raster` composition references at build time.

## Idempotence rules

- Re-runs are **additive and conservative**. An inferer MAY add new screens, add missing regions, fill empty hints, or refine content it previously emitted when the same source still supports the refinement.
- Inferers MUST NOT silently delete screens, groups, layout properties, token references, asset references, or comments that may have been operator-edited. When source material no longer supports a previously inferred element, the inferer reports a **stale-source warning** in the terminal summary instead of removing the YAML.
- The contract does NOT use "owned by inferer" markers in the YAML. The merge rule is intentionally simple to review: preserve existing structure, append new evidence, surface conflicts as comments and terminal warnings.
- When an inferer encounters a previously emitted `component: <slug>` directive whose evidence no longer holds (e.g. the structurally identical groups it was based on were edited away), the inferer MUST leave the directive in place and report a stale-directive warning. Demoting a slug is operator work.

## Component directive emission

The `component: <slug>` directive is the v1 cross-shell factoring contract; see [Component Catalog](./spec-runtime/components.md) for the operator-curated catalog and validation surfaces. Inferers emit it conservatively because every shell writer (iOS, Android, future React+TypeScript) factors a single named element per slug and the operator owns the eventual prop-shape contract.

Emission policy:

- An inferer MAY emit `component: <slug>` only when **either** of these holds:
  - the operator confirms a candidate via the existing layout itself — a previous accepted `layout.yaml` containing the slug, or a pre-existing `component: <slug>` already on the group in `--baseline`. There is no CLI flag for component approval in v1; promotion of a single-occurrence candidate is operator-edit work.
  - the inferer observes structurally identical groups in **≥2 screens of the same run**.
- Otherwise the inferer MUST flatten the group and emit a `# candidate component: <slug>` comment adjacent to each occurrence so the operator can promote it explicitly in a later edit. Single-occurrence candidates always remain comments, never directives.
- Slugs MUST match `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` (kebab-case). The reserved region names — `header`, `body`, `footer`, `fab` — MUST NOT be used as slugs (the schema enforces this; inferers MUST avoid producing them in the first place).

Structural identity (cross-instance rule, validated by `specify tool run vectis -- validate layout`):

- Two groups carrying the same `component:` slug MUST share the same skeleton: same ordered nested item kinds and the same nested-group shape across the document.
- Instances MAY differ in `bind`, `event`, `error`, `asset`, token references, the *condition expressions* on `*-when` keys, and free text content. Skeleton divergence is an error; wiring divergence is the expected use of the directive.
- **`*-when`-gated sub-groups.** A `*-when` gated sub-group is part of the skeleton: every instance MUST present the same set of `*-when` keys on the same nested groups. The condition expression MAY differ; the presence MUST NOT.
- **State-replaced bodies.** Slug instances inside `states.<name>.body` participate in identity checks against slug instances in the screen's main `body` and across screens. A state body that replaces another for a given state is a new instance of the slug, not an exemption.
- **Per-instance `platforms.*` overrides.** When an instance carries `platforms.ios.*` or `platforms.android.*` overrides on the slug-bearing group, the override skeleton MAY differ from the base skeleton — overrides exist precisely to express per-platform divergence — but the **base** skeleton (the keys outside `platforms.*`) MUST still match across all instances.

When the inferer is uncertain whether observed similarity meets the structural-identity bar, the safe action is to flatten and emit a `# candidate component: <slug>` comment. Promoting a comment to a directive is cheap; demoting an emitted directive is not.

## Verification

Every inferer MUST invoke the deterministic validators **before reporting success**, then translate any reported errors into terminal output the operator can act on. The validators live in the declared `vectis` (`validate`) WASI tool, run through `specify tool run`, read their input from disk, and are the only authoritative source of pass/fail.

Because the validator reads a file path, "errors block writes" is enforced through a **stage-then-validate-then-rename** sequence rather than a literal pre-write check. Validating before any write would either error on a missing file (greenfield) or re-check the previous run's content (refine):

1. Write the inferred output to a sibling staging path (`<output-path>.tmp`). Refine runs MUST stage even when an existing `<output-path>` already validates clean — the validator never sees the new content otherwise.
2. Run the validator against the staging path explicitly:

    ```bash
    specify tool run vectis -- validate layout <output-path>.tmp
    ```

3. On a clean or warnings-only result, atomically rename the staging file onto `<output-path>` (`rename(2)` / `mv <output-path>.tmp <output-path>`).
4. On errors, delete the staging file, surface the validator report verbatim, and exit non-zero. Any prior `<output-path>` is preserved untouched.

This validates YAML syntax, the composition schema, the unwired-subset rules above, and the §G structural-identity rule for any `component:` directives present. Pass the staging path explicitly so a failed run cannot validate stale or default-resolved content; the optional default-path resolution (slice-local `layout.yaml` then `design-system/layout.yaml`) exists for ad-hoc operator invocations, not for the inferer's own gate. Errors block the rename; warnings surface in the terminal summary but do not block.

Cross-artifact reference checks (when the sibling input artifacts exist):

```bash
specify tool run vectis -- validate composition <output-path>.tmp
```

Inferers SHOULD run `composition` mode against the **same staging path** before the atomic rename — never against a default-resolved path or the prior `<output-path>` — so token / asset references in the new content are checked, not last run's. `composition` mode auto-invokes `tokens` and `assets` modes when sibling `tokens.yaml` / `assets.yaml` files exist (whether slice-local or project-level); their reports surface in the same envelope. Errors fold into the same rename-blocking gate as `validate layout`; warnings forward into the terminal summary.

The full per-mode surface every inferer can call:

| Verb | Validates |
|---|---|
| `specify tool run vectis -- validate layout [path]` | `layout.yaml` against the unwired subset (composition schema + structural identity + no define-owned wiring keys + no `delta`). |
| `specify tool run vectis -- validate composition [path]` | Wired or unwired composition; auto-invokes `tokens` and `assets` when siblings exist. |
| `specify tool run vectis -- validate tokens [path]` | `tokens.yaml` against the published [`tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json). |
| `specify tool run vectis -- validate assets [path]` | `assets.yaml` against the published [`assets.schema.json`](https://schemas.specify.dev/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**`. |
| `specify tool run vectis -- validate all` | Runs all four against the active slice and baseline. Convenience mode. |

Exit semantics for every mode:

- **Errors** — exit non-zero. Inferers MUST treat this as a write block and surface the report verbatim.
- **Warnings only** — exit zero with a printed warning report. Inferers MUST forward warnings into the terminal summary; the write proceeds.
- **Clean** — exit zero silently.

Inferers MUST NOT roll their own schema or reference validation. Every check the contract requires has an authoritative CLI verb above; reimplementing them in skill prose causes drift.

## Terminal summary

Every inferer's terminal output MUST conclude with a structured summary so reviewers can scan a single block:

- Screens added.
- Screens refined.
- Warnings (including stale-source and stale-directive warnings).
- Unresolved gaps (`# TODO` comments emitted, unresolved token references, missing asset IDs).
- Source provenance entries appended (one line per `provenance.sources[]` entry added).
- Candidate components — both directives emitted and `# candidate component: <slug>` comments left for operator review.
- Exact output path (the file the inferer wrote).

Source-specific skills MAY add additional sections (e.g. the image inferer reports cropped chrome regions; a future Figma inferer reports unmapped variables). They MUST NOT drop any item above.

## See also

- [Component Catalog](./spec-runtime/components.md) — operator workflow and validation surfaces for `components.yaml`.
- [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json) — the schema both `layout.yaml` (unwired) and `composition.yaml` (wired) validate against. Retrieve with `specify tool schema vectis composition`.
- [`tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json) and [`assets.schema.json`](https://schemas.specify.dev/vectis/assets.schema.json) — the sibling input schemas the cross-artifact reference checks consume.
