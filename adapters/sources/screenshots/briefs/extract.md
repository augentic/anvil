# `screenshots.extract`

For one `Lead`, run the vision-assisted spatial pipeline against the image(s) bound to that screen under `$SOURCE_DIR` and return a single `Evidence` document carrying `region` / `container` / `leaf` claims. The CLI persists the result at `.specify/slices/<slice>/evidence/<source>.yaml`; this brief returns the YAML body only.

The pipeline body lives in [`extract/pipeline.md`](extract/pipeline.md) and runs the spatial-inference pipeline — triage → chrome cropping → regions → containers → leaves → conservative component detection. The pipeline emits flat claims, not a hierarchical `layout.yaml`. Downstream synthesis (core) folds the claims back into the canonical artifacts (see [From sources to slices](../references/spec-runtime/reconciliation.md#slice-time-evidence-becomes-a-spec)); `adapters/targets/vectis/build` regenerates `composition.yaml` from the synthesised `spec.md` / `design.md`.

## Inputs

- `$SOURCE_DIR` — read-only preopen of the bound screenshots directory.
- `<source>` — the plan-level binding key under `plan.yaml.sources.<key>`.
- `<lead>` — the lead from `discovery.md` this run is extracting Evidence for (one screen, possibly with state and platform variants attached).
- `$SCRATCH_DIR` — per-slice write-only scratch space. Use only for unavoidable intermediate state (cropped image staging).

## Vision prerequisite

Identical to `screenshots.survey`: read at least one of the input image paths through the runtime's native attachment mechanism. On failure, exit `1` with the supported-runtimes message — never fall back to filename or metadata inference.

## Resolve the lead's images

The lead id was produced by `screenshots.survey` from one screen (potentially with state / platform variants triaged into it). Resolve it back:

1. Prefer images whose vision-inferred title slugs to `<lead>`.
2. Fall back to images whose kebab-cased filename stem equals `<lead>` (or starts with `<lead>-` for state variants).
3. When operator `state` / `group` / `platform` hints accompany the source binding, apply them as authoritative: hint-bound images win over visual similarity.

If no image resolves, return Evidence with `claims: []` rather than fabricating content. The CLI treats empty `claims:` as valid; an unresolvable lead becomes a `Status: unknown` requirement during synthesis.

## Pipeline

Run the seven-stage pipeline defined in [`extract/pipeline.md`](extract/pipeline.md): triage → crop platform chrome → infer regions → infer containers → infer leaves → detect candidate components conservatively → emit gaps. Each stage records uncertainty in `notes:` on the affected claim. The pipeline brief also carries the determinism and idempotence rules.

## Token references

- Never reverse-engineer `tokens.yaml` from pixels.
- MAY reference token names from a sibling `design-system/tokens.yaml` when an entry resolves cleanly to a visible value (a known `colors.primary.light` matches the visible button colour). Auto-discovery looks at `design-system/tokens.yaml`.
- Otherwise prefer raw layout values that the composition schema permits (`gap: 16`, `corner_radius: 8`) and emit `notes.todo: tokenise <prop>` on the affected claim.
- Never invent token names that do not appear in `tokens.yaml`.

## Asset references

- MAY reference asset IDs from a sibling `design-system/assets.yaml` when an `assets.<id>` entry already exists.
- Otherwise emit a `name:` placeholder paired with `notes.todo: add <id> to assets.yaml`.
- Never crop or extract production assets out of screenshots.

### Symbol inference policy

When a leaf claim carries `leaf: icon` or `leaf: icon-button` and no matching
`assets.<id>` entry exists, apply the symbol inference policy below (inventory
authoring only — not a build shortcut):

1. **Known asset** — the screenshot shape matches an existing `assets.<id>`
   entry → reference that id in the claim / downstream layout.
2. **Generic platform glyph** — the shape is a close match to a standard SF
   Symbol / Material Icon and no designer asset exists → MAY add a `kind: symbol`
   entry to `assets.yaml` with `inferred: true` and the platform glyph mapping
   under `symbols.ios` / `symbols.android`, **or** emit `notes.todo: promote
   <slug> as kind: symbol` for operator approval before merge.
3. **Branded / custom shape** — logo, illustration, or non-glyph artwork → emit
   `notes.todo: add <id> to assets.yaml` only; **never** auto-promote to
   `kind: symbol`.

Symbol promotion is explicit inventory work. Shell writers render
`kind: symbol` at the call site; they MUST NOT substitute platform glyphs for
`vector` / `raster` ids at build time.

## `path` grammar

Every claim carries a `path:` rooted relative to `$SOURCE_DIR`. The schema's path grammar accepts `<path>` for whole-image claims and `<path>#L<n>` / `<path>#L<start>-L<end>` for line ranges; screenshots have no line vocabulary, so every claim uses the whole-image form `<path>`.

Spatial coordinates (when known) ride on a per-claim `bbox: { x, y, w, h }` body field in **image-relative integer pixels**, not on the `path:`. The bbox is optional — emit it when the source image lets you measure it confidently, omit it otherwise.

## Claim-id grammar

`id` is **optional** on the spatial kinds (the Evidence schema only requires it on `requirement` and `criterion`). Include it anyway — downstream synthesis reconciles claims across multiple Evidence documents using the `id` as the deterministic key, and components in particular need stable ids across re-runs.

Recommended pattern:

- Region: `<screen-slug>.<region-name>` (e.g. `task-list.body`, `task-list.states.empty`).
- Container: `<screen-slug>.<region-name>.<role-or-position>` (e.g. `task-list.body.tasks-list`, `task-list.body.tasks-list.task-row`).
- Leaf: `<screen-slug>.<region-name>.<parent>.<role>` (e.g. `task-list.body.tasks-list.task-row.title`).

Keep the dotted segments kebab-case. Re-running `extract` against the same source MUST produce byte-identical `id`s.

## Output

Return one Evidence document matching `schemas/evidence.schema.json`. Field order is fixed (`authority`, `lead`, `claims`). Each claim's body fields depend on its `kind`:

- **region** — `screen`, `region` (closed enum: `header | body | footer | fab | states.<name> | overlays.<name> | platforms.<platform>.<region>`); optional `bbox`, `title`, `overlay_kind`, `state_when`, `state_replaces`, `notes.cropped_chrome`.
- **container** — `screen`, `region`, `parent`, `container` (closed enum: `group | list | grid | form | card | surface | divider`); optional `direction`, `gap`, `padding`, `align`, `justify`, `size`, `background`, `corner_radius`, `elevation`, `each`, `columns`, `rows`, `style`, `component`, `notes.candidate_component`, `notes.todo`.
- **leaf** — `screen`, `region`, `parent`, `leaf` (closed enum: `text | button | icon-button | link | icon | image | field | checkbox | switch | radio | slider | segmented-control | progress-indicator | badge | chip | divider | spacer`); optional `content`, `label`, `style`, `role`, `name`, `color`, `notes.todo`.

`authority` is always the literal `documentation` (operator-provided written product / technical intent; the authority precedence `intent > documentation > behaviour` is defined in [`authority.md`](../references/spec-runtime/synthesis/authority.md)). The document's `(slice, source)` identity is path-borne (the CLI persists it at `.specify/slices/<slice>/evidence/<source>.yaml`) and the adapter resolves from `plan.yaml.sources.<source>.adapter`, so neither is written in-document.

Worked example: [`references/examples/task-list.md`](../references/examples/task-list.md) — a `task-list` lead with populated and empty-state images, ending in a candidate-component note that promotes to `component: task-row` on the next pass.

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-extract-path-denied`; never attempt to widen the preopen.
- Never write Evidence to disk yourself — return the YAML body to the CLI, which persists it under `.specify/slices/<slice>/evidence/<source>.yaml`.
- Never emit define-owned wiring on a claim: no `maps_to`, no `bind`, no `event`, no `error`, no overlay `trigger`, no navigation events, no `*-when` body keys. The `state_when:` body field on a `states.<name>` region claim is the *condition expression* lifted from visible cues, not a `*-when` wiring key.
- Never emit closed-enum claim kinds outside `{region, container, leaf}` from this adapter. Behavioural kinds (`excerpt` / `type` / `call`) belong to code source adapters; intent kinds belong to `intent`.
- Never crop or extract production assets out of screenshots. `$SCRATCH_DIR` is for transient chrome-cropping staging only.
- Never invent token names that do not appear in `design-system/tokens.yaml`. Raw values plus `notes.todo` are the v1 escape hatch.
- Never invent asset IDs that do not appear in `design-system/assets.yaml`. Emit `name:` placeholder plus `notes.todo`.
- Never promote a candidate component to `component: <slug>` unless the pipeline's stage-6 policy is satisfied. When in doubt, emit the note.
- Empty `claims: []` is valid output when the lead cannot be resolved to any image content. Do not pad with speculative claims.
