# `screenshots.extract`

For one `Candidate`, run the vision-assisted spatial pipeline against the image(s) bound to that screen under `$SOURCE_DIR` and return a single `Evidence` document carrying `region` / `container` / `leaf` claims. The CLI persists the result at `.specify/slices/<slice>/evidence/<source-key>.yaml`; this brief returns the YAML body only.

The v1 pipeline below preserves the inference algorithm of the retired `vectis-image-layout-inferer` skill verbatim — triage → chrome cropping → regions → containers → leaves → conservative component detection — only the *output* shape changes (from a hierarchical `layout.yaml` to a flat list of claims with explicit `screen` / `region` / `parent` references). Downstream synthesis (core) folds the claims back into the canonical artifacts; `targets/vectis/build` regenerates `composition.yaml` from the synthesised `spec.md` / `design.md`. This brief does NOT redesign that algorithm.

## Inputs

- `$SOURCE_DIR` — read-only preopen of the bound screenshots directory.
- `<source-key>` — the plan-level binding key under `plan.yaml.sources.<key>`.
- `<candidate-id>` — the candidate from `discovery.md` this run is extracting Evidence for (one screen, possibly with state and platform variants attached).
- `$SCRATCH_DIR` — per-slice write-only scratch space. Use only for unavoidable intermediate state (cropped image staging).

## Vision prerequisite

Identical to `screenshots.enumerate`: read at least one of the input image paths through the runtime's native attachment mechanism. On failure, exit `1` with the supported-runtimes message — never fall back to filename or metadata inference.

## Resolve the candidate's images

The candidate id was produced by `screenshots.enumerate` from one screen (potentially with state / platform variants triaged into it). Resolve it back:

1. Prefer images whose vision-inferred title slugs to `<candidate-id>`.
2. Fall back to images whose kebab-cased filename stem equals `<candidate-id>` (or starts with `<candidate-id>-` for state variants).
3. When operator `state` / `group` / `platform` hints accompany the source binding, apply them as authoritative: hint-bound images win over visual similarity.

If no image resolves, return Evidence with `claims: []` rather than fabricating content. The CLI treats empty `claims:` as valid; an unresolvable candidate becomes a `Status: unknown` requirement during synthesis.

## Pipeline

The pipeline runs top-down. Each stage produces evidence the next refines; uncertainty is recorded in `notes:` on the affected claim so the operator can act on it without re-running.

### 1. Triage

Group resolved images into screen / state / platform buckets using explicit hints first, visual similarity second.

1. Apply every `state <slug>:<name>=<path>` mapping bound to this candidate. These bindings are authoritative.
2. Apply every `group <slug>:<paths>` mapping for un-bound images attached to this candidate.
3. For remaining images, group by visual similarity (header / chrome match, dominant content match) and propose state names from visible cues like "no tasks yet" → `empty`.

Single-image candidates are accepted; the component-detection ≥2-screens rule (stage 6) governs `component:` emission, not candidate recognition.

### 2. Crop platform chrome

Skip when no `platform` hint is present and no chrome is detected. Otherwise remove:

- iOS: status bar, dynamic island / notch, software home indicator.
- Android: status bar, system navigation bar, gesture indicator.
- Web: browser chrome, devtools panes, surrounding OS chrome.
- Generic: emulator frames, screen recorder overlays, OS-level toasts that aren't part of the application.

Cropped pixels are staged in `$SCRATCH_DIR` only; they never leave the brief and never appear in Evidence. Record what was cropped on the candidate's first emitted `region: { region: header }` claim under `notes.cropped_chrome:`.

### 3. Infer regions

Emit one `kind: region` claim per detected region. Closed region names:

- `header` (top app / navigation bar; record `title`, `leading[]`, `trailing[]` references via separate leaf claims).
- `body` (primary content area).
- `footer` (bottom app bar / tab bar / persistent action row).
- `fab` (floating action button — at most one per screen).
- `states.<name>` (replacement bodies for `loading`, `empty`, `error`, etc.; reuse the state names from explicit `state` hints when available, otherwise propose kebab-case names from visible cues).
- `overlays.<name>` (modals, sheets, dialogs, popovers, snackbars). Overlays MUST NOT include `trigger:` — that key is define-owned.
- `platforms.<ios|android|web>.<region>` (per-platform region overrides) — only when multiple platform-variant images supply distinct chrome shapes for the same screen.

A region MAY be omitted when there is no visible evidence for it (e.g. a screen with no FAB).

### 4. Infer containers

Emit one `kind: container` claim per `group`-style node organising content inside a region. Pick the closest schema container kind:

- `group` with `direction: row` for horizontal layouts; `direction: column` for vertical stacks.
- `list` with `each: <bind-name>` when content is clearly a repeating row set. The `each:` value is a placeholder kebab-case name (`tasks`, `messages`); synthesis rewires it to a real ViewModel binding later. Use `style: plain` / `inset` / `grouped` only when iOS-style grouping is visually obvious.
- `grid` with `each:` and `columns:` (or `rows:`) when content is clearly a 2-D matrix.
- `form` for grouped settings rows / field stacks.
- `card`, `surface`, `divider` for explicit decoration affordances.

Recover layout properties when they are visually unambiguous: `gap`, `padding`, `align`, `justify`, `size: { width: fill | hug | <px> }`, `background`, `corner_radius`, `elevation`. Prefer schema-permitted scalar values (`md`, `lg`, `16`) plus a `notes.todo: tokenise <prop> <value>` on the claim over inventing a token name. The token-reference rules below forbid inventing token names entirely.

Every container claim carries a `parent:` reference to the enclosing region (or enclosing container) claim's `claim-id`, so synthesis can rebuild the hierarchy.

### 5. Infer leaves

Emit one `kind: leaf` claim per leaf element. Closed leaf kinds:

- `text` (with `style`, `role`, `content`).
- `button`, `icon-button`, `link` (with `label`, `style`, optional `icon`).
- `icon` (with `name`).
- `image` (with `name` referencing `assets.yaml`).
- `field`, `checkbox`, `switch`, `radio`, `slider`, `segmented-control` (form controls).
- `progress-indicator`, `badge`, `chip`.
- `divider`, `spacer`.

For each leaf, copy the visible text content into `content:` / `label:` (preserving casing). If the text is unreadable or visibly truncated, emit `content: "<unreadable>"` plus a `notes.todo: confirm text` and a top-level `gaps:` entry under the same claim.

Every leaf claim carries a `parent:` reference to its enclosing container or region claim's `claim-id`.

### 6. Detect candidate components conservatively

Walk every container claim produced in stage 4 and compare every `container: group` claim against every other for **structural identity**:

- Same ordered nested item kinds.
- Same nested-group shape.
- Same set of `*-when` keys *present* on nested groups (presence — not condition value — is part of the skeleton). `*-when` keys themselves are not emitted by this brief (define-owned); presence here means future-instance check.
- `platforms.*` overrides participate only against other `platforms.<same>` overrides; the **base** skeleton MUST still match across all instances.

Apply the conservative emission policy:

- Promote a container claim to `component: <slug>` only when **either** the operator confirms a candidate (a previous accepted Evidence carries the slug already) **or** the brief observes ≥2 structurally identical groups across screens of the *same run* (within `<candidate-id>` plus any prior candidates extracted for the same plan — synthesis aggregates across candidates).
- Otherwise leave `component:` unset on the claim and add `notes.candidate_component: <slug>` so the operator can promote it explicitly later.
- Slugs MUST match `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` (kebab-case). Reserved region names (`header`, `body`, `footer`, `fab`) MUST NOT be used as slugs.
- Derive slugs from visible content (`task-row`, `setting-row`, `chip-tag`) — never from layout shape (`row-1`, `card-2`).

When in doubt, leave `component:` unset and emit the note. Promoting a note to a directive is cheap; demoting an emitted directive is operator work.

### 7. Emit gaps

Record uncertainty on the affected claim under a `notes:` map when:

- Grouping is ambiguous (e.g. two visually plausible group boundaries).
- Text is unreadable or truncated.
- Icon identity is uncertain (record `notes.todo: confirm icon`; do **not** guess between e.g. `chevron-right` and `arrow-right`).
- A token reference is expected but no name resolves (`notes.todo: tokenise gap 16`).
- An asset reference is expected but `assets.yaml` does not list the ID (`notes.todo: add image '<id>' to assets.yaml`).
- A candidate component skeleton is borderline (`notes.candidate_component: <slug>` — see stage 6).

Each `notes.todo` and `notes.candidate_component` surfaces in the slice's synthesis output as a `[unknown]` tag against the affected requirement during fusion.

## Token references

- Never reverse-engineer `tokens.yaml` from pixels.
- MAY reference token names from a sibling `design-system/tokens.yaml` when an entry resolves cleanly to a visible value (a known `colors.primary.light` matches the visible button colour). Auto-discovery looks at `design-system/tokens.yaml`.
- Otherwise prefer raw layout values that the composition schema permits (`gap: 16`, `corner_radius: 8`) and emit `notes.todo: tokenise <prop>` on the affected claim.
- Never invent token names that do not appear in `tokens.yaml`.

## Asset references

- MAY reference asset IDs from a sibling `design-system/assets.yaml` when an `assets.<id>` entry already exists.
- Otherwise emit a `name:` placeholder paired with `notes.todo: add <id> to assets.yaml`.
- Never crop or extract production assets out of screenshots.

## `path` grammar

Every claim carries a `path:` rooted relative to `$SOURCE_DIR`. The schema's path grammar accepts `<path>` for whole-image claims and `<path>#L<n>` / `<path>#L<start>-L<end>` for line ranges; screenshots have no line vocabulary, so every claim uses the whole-image form `<path>`.

Spatial coordinates (when known) ride on a per-claim `bbox: { x, y, w, h }` body field in **image-relative integer pixels**, not on the `path:`. The bbox is optional — emit it when the source image lets you measure it confidently, omit it otherwise.

## Claim-id grammar

`claim-id` is **optional** on the spatial kinds (the Evidence schema only requires it on `requirement` and `criterion`). Include it anyway — downstream synthesis fuses claims across multiple Evidence documents using the `claim-id` as the deterministic key, and components in particular need stable ids across re-runs.

Recommended pattern:

- Region: `<screen-slug>.<region-name>` (e.g. `task-list.body`, `task-list.states.empty`).
- Container: `<screen-slug>.<region-name>.<role-or-position>` (e.g. `task-list.body.tasks-list`, `task-list.body.tasks-list.task-row`).
- Leaf: `<screen-slug>.<region-name>.<parent>.<role>` (e.g. `task-list.body.tasks-list.task-row.title`).

Keep the dotted segments kebab-case. Re-running `extract` against the same source MUST produce byte-identical `claim-id`s.

## Output

Return one Evidence document matching `schemas/evidence.schema.json`. Field order is fixed (`source`, `adapter`, `authority`, `candidate`, `claims`).

```yaml
source: <source-key>
adapter: screenshots
authority: documentation
candidate: <candidate-id>
claims:
  - kind: region
    claim-id: <screen-slug>.<region-name>
    path: <screen-relative-image-path>
    screen: <screen-slug>
    region: header | body | footer | fab | states.<name> | overlays.<name> | platforms.<platform>.<region>
    # optional body fields per region:
    # bbox: { x, y, w, h }
    # title: <verbatim header title>
    # overlay_kind: dialog | sheet | popover | snackbar
    # state_when: <expression>            # state regions only; condition lifted from visible cues
    # state_replaces: body                # state regions only; defaults to body
    # notes:
    #   cropped_chrome: [status-bar, home-indicator]
  - kind: container
    claim-id: <screen-slug>.<region-name>.<role>
    path: <screen-relative-image-path>
    screen: <screen-slug>
    region: <region-name>
    parent: <enclosing-claim-id>
    container: group | list | grid | form | card | surface | divider
    # optional body fields per container kind:
    # direction: row | column
    # gap: <token-or-px>
    # padding: <token-or-px>
    # align: start | center | end | baseline | stretch
    # justify: start | center | end | space-between | space-around | space-evenly
    # size: { width: fill | hug | <px>, height: fill | hug | <px> }
    # background: <token>
    # corner_radius: <token-or-px>
    # elevation: <token-or-px>
    # each: <kebab>              # list / grid only
    # columns: <n>               # grid only
    # rows: <n>                  # grid only
    # style: plain | inset | grouped
    # component: <kebab>         # set only under the stage-6 promotion rule
    # notes:
    #   candidate_component: <kebab>
    #   todo: tokenise gap 16
  - kind: leaf
    claim-id: <screen-slug>.<region-name>.<parent>.<role>
    path: <screen-relative-image-path>
    screen: <screen-slug>
    region: <region-name>
    parent: <enclosing-container-claim-id>
    leaf: text | button | icon-button | link | icon | image | field | checkbox | switch | radio | slider | segmented-control | progress-indicator | badge | chip | divider | spacer
    # optional body fields per leaf kind:
    # content: <verbatim text> | "<unreadable>"
    # label: <verbatim text>
    # style: <enum>
    # role: heading | body | caption | label | error | ...
    # name: <icon-or-asset-id>
    # color: <token>
    # notes:
    #   todo: confirm text
```

`adapter` is always the literal `screenshots`. `authority` is always the literal `documentation` (operator-provided written product / technical intent — see the authority hierarchy `intent > documentation > behaviour`). `source` is the supplied `<source-key>`. `candidate` is the supplied `<candidate-id>`.

## Worked example

Input — a single candidate `task-list` with two images in `$SOURCE_DIR`:

```text
task-list-populated.png   # visible header: "Today"; rows of task items + FAB
task-list-empty.png       # same header / chrome; empty-state illustration replaces body
```

Output (Evidence for `candidate: task-list`, bound under `<source-key>` `screens`; only one task row's claims shown for brevity):

```yaml
source: screens
adapter: screenshots
authority: documentation
candidate: task-list
claims:
  - kind: region
    claim-id: task-list.header
    path: task-list-populated.png
    screen: task-list
    region: header
    title: Today
  - kind: region
    claim-id: task-list.body
    path: task-list-populated.png
    screen: task-list
    region: body
  - kind: region
    claim-id: task-list.fab
    path: task-list-populated.png
    screen: task-list
    region: fab
  - kind: region
    claim-id: task-list.states.empty
    path: task-list-empty.png
    screen: task-list
    region: states.empty
    state_when: tasks.is_empty
    state_replaces: body
  - kind: container
    claim-id: task-list.body.tasks
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body
    container: list
    each: tasks
    style: plain
  - kind: container
    claim-id: task-list.body.tasks.task-row
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body.tasks
    container: group
    direction: row
    gap: md
    padding: md
    align: center
    notes:
      candidate_component: task-row
  - kind: leaf
    claim-id: task-list.body.tasks.task-row.checkbox
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body.tasks.task-row
    leaf: checkbox
    label: Mark task complete
  - kind: leaf
    claim-id: task-list.fab.action
    path: task-list-populated.png
    screen: task-list
    region: fab
    parent: task-list.fab
    leaf: icon
    name: plus
```

A full input / output fixture for this example lives at [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../../tests/fixtures/sources/screenshots/task-list-two-screen/) in the repo. When `screenshots.extract` runs against a *second* candidate later in the same plan (e.g. an `archive` screen sharing the same row skeleton), the brief promotes the candidate-component note to `component: task-row` per stage 6's ≥2-screens rule.

## Determinism

- Emit claims in pipeline order: regions first (in the closed-region order above), then containers (in pre-order tree walk under each region), then leaves (in pre-order tree walk under each container).
- `claim-id`s follow the dotted-kebab grammar above. Re-running against unchanged inputs produces byte-identical Evidence.
- Quote `content` / `label` / `title` verbatim from the screen where legible. Light grammatical normalisation (terminal punctuation) is allowed; rephrasing is not.
- Do not invent layout properties. Omit `gap` / `padding` / `align` / `size` when measurement is unconfident; emit `notes.todo` instead.
- Do not include timestamps, host paths, or other run-state in the output.

## Idempotence

Re-runs are additive and conservative; the CLI replaces Evidence by `(<source-key>, <candidate-id>)` tuple, but within a run:

- A re-run against the same source images MAY refine previously emitted body fields when the same images still support the refinement.
- Operator overrides committed at synthesis time (post-fusion edits in `spec.md` / `design.md`) are NOT visible to `extract`; the brief only sees the source images. Use stable `claim-id`s so the fusion layer can detect and preserve operator edits.
- When the new screenshots no longer contain a previously inferred element, simply do not emit its claim. The synthesis layer detects the drop via the missing `claim-id` and tags affected requirements with `[unknown]` / `[divergence]` — there is no `# stale-source:` annotation at the Evidence layer.

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-extract-path-denied`; never attempt to widen the preopen.
- Never write Evidence to disk yourself — return the YAML body to the CLI, which persists it under `.specify/slices/<slice>/evidence/<source-key>.yaml`.
- Never emit define-owned wiring on a claim: no `maps_to`, no `bind`, no `event`, no `error`, no overlay `trigger`, no navigation events, no `*-when` body keys. The `state_when:` body field on a `states.<name>` region claim is the *condition expression* lifted from visible cues, not a `*-when` wiring key.
- Never emit closed-enum claim kinds outside `{region, container, leaf}` from this adapter. Behavioural kinds (`excerpt` / `type` / `call`) belong to code source adapters; intent kinds belong to `intent`.
- Never crop or extract production assets out of screenshots. `$SCRATCH_DIR` is for transient chrome-cropping staging only.
- Never invent token names that do not appear in `design-system/tokens.yaml`. Raw values plus `notes.todo` are the v1 escape hatch.
- Never invent asset IDs that do not appear in `design-system/assets.yaml`. Emit `name:` placeholder plus `notes.todo`.
- Never promote a candidate component to `component: <slug>` unless stage 6's policy is satisfied. When in doubt, emit the note.
- Empty `claims: []` is valid output when the candidate cannot be resolved to any image content. Do not pad with speculative claims.
