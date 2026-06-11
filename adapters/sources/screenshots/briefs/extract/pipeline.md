# `screenshots.extract` — vision pipeline

Loaded by [../extract.md](../extract.md) at the § Pipeline step. Carries the v1 spatial-inference pipeline — triage → chrome cropping → regions → containers → leaves → conservative component detection. The pipeline emits flat claims, not a hierarchical `layout.yaml`.

The pipeline runs top-down. Each stage produces evidence the next refines; uncertainty is recorded in `notes:` on the affected claim so the operator can act on it without re-running.

## 1. Triage

Group resolved images into screen / state / platform buckets using explicit hints first, visual similarity second.

1. Apply every `state <slug>:<name>=<path>` mapping bound to this lead. These bindings are authoritative.
2. Apply every `group <slug>:<paths>` mapping for un-bound images attached to this lead.
3. For remaining images, group by visual similarity (header / chrome match, dominant content match) and propose state names from visible cues like "no tasks yet" → `empty`.

Single-image leads are accepted; the component-detection ≥2-screens rule (stage 6) governs `component:` emission, not lead recognition.

## 2. Crop platform chrome

Skip when no `platform` hint is present and no chrome is detected. Otherwise remove:

- iOS: status bar, dynamic island / notch, software home indicator.
- Android: status bar, system navigation bar, gesture indicator.
- Web: browser chrome, devtools panes, surrounding OS chrome.
- Generic: emulator frames, screen recorder overlays, OS-level toasts that aren't part of the application.

Cropped pixels are staged in `$SCRATCH_DIR` only; they never leave the brief and never appear in Evidence. Record what was cropped on the lead's first emitted `region: { region: header }` claim under `notes.cropped_chrome:`.

## 3. Infer regions

Emit one `kind: region` claim per detected region. Closed region names:

- `header` (top app / navigation bar; record `title`, `leading[]`, `trailing[]` references via separate leaf claims).
- `body` (primary content area).
- `footer` (bottom app bar / tab bar / persistent action row).
- `fab` (floating action button — at most one per screen).
- `states.<name>` (replacement bodies for `loading`, `empty`, `error`, etc.; reuse the state names from explicit `state` hints when available, otherwise propose kebab-case names from visible cues).
- `overlays.<name>` (modals, sheets, dialogs, popovers, snackbars). Overlays MUST NOT include `trigger:` — that key is define-owned.
- `platforms.<ios|android|web>.<region>` (per-platform region overrides) — only when multiple platform-variant images supply distinct chrome shapes for the same screen.

A region MAY be omitted when there is no visible evidence for it (e.g. a screen with no FAB).

## 4. Infer containers

Emit one `kind: container` claim per `group`-style node organising content inside a region. Pick the closest schema container kind:

- `group` with `direction: row` for horizontal layouts; `direction: column` for vertical stacks.
- `list` with `each: <bind-name>` when content is clearly a repeating row set. The `each:` value is a placeholder kebab-case name (`tasks`, `messages`); synthesis rewires it to a real ViewModel binding later. Use `style: plain` / `inset` / `grouped` only when iOS-style grouping is visually obvious.
- `grid` with `each:` and `columns:` (or `rows:`) when content is clearly a 2-D matrix.
- `form` for grouped settings rows / field stacks.
- `card`, `surface`, `divider` for explicit decoration affordances.

Recover layout properties when they are visually unambiguous: `gap`, `padding`, `align`, `justify`, `size: { width: fill | hug | <px> }`, `background`, `corner_radius`, `elevation`. Prefer schema-permitted scalar values (`md`, `lg`, `16`) plus a `notes.todo: tokenise <prop> <value>` on the claim over inventing a token name. The token-reference rules in [../extract.md](../extract.md) forbid inventing token names entirely.

Every container claim carries a `parent:` reference to the enclosing region (or enclosing container) claim's `id`, so synthesis can rebuild the hierarchy.

## 5. Infer leaves

Emit one `kind: leaf` claim per leaf element. Closed leaf kinds:

- `text` (with `style`, `role`, `content`).
- `button`, `icon-button`, `link` (with `label`, `style`, optional `icon`).
- `icon` (with `name`).
- `image` (with `name` referencing `assets.yaml`).
- `field`, `checkbox`, `switch`, `radio`, `slider`, `segmented-control` (form controls).
- `progress-indicator`, `badge`, `chip`.
- `divider`, `spacer`.

For each leaf, copy the visible text content into `content:` / `label:` (preserving casing). If the text is unreadable or visibly truncated, emit `content: "<unreadable>"` plus a `notes.todo: confirm text` and a top-level `gaps:` entry under the same claim.

Every leaf claim carries a `parent:` reference to its enclosing container or region claim's `id`.

## 5b. Resolve variant families

When a sibling `assets.yaml` exists, disambiguate `icon`, `icon-button`, and `image` leaves that belong to a variant family — a set of asset IDs sharing a common base with visual-state suffixes. This sub-stage runs after leaf inference (stage 5) and before candidate-component detection (stage 6).

1. **Discover variant families.** Scan `assets.yaml` IDs and group by longest shared kebab-case prefix where the remaining suffix is a recognised state token: `default`, `active`, `focussed`, `focused`, `selected`, `checked`, `disabled`, `empty`, `highlighted`, `pressed`, `hovered`, `high`, `medium`, `low`. Example: `{nav-lists-default, nav-lists-active, nav-lists-focussed}` → family `nav-lists` with 3 variants. When an entry carries an explicit `variant_of:` field, use that grouping instead of the suffix heuristic.
2. **Identify candidate leaves.** Walk the inferred leaf claims and collect every `icon`, `icon-button`, or `image` leaf whose `name:` matches any entry in a variant family.
3. **Multi-image comparison pass.** For each candidate leaf:
   - Load the source file (SVG / PNG) for every variant in the family.
   - Crop or zoom the relevant region from the input screenshot.
   - If any variant carries a `usage_hint:`, include those hints as labelled textual guidance alongside the source images (e.g. "Variant A (`nav-lists-active`): Outlined shapes with background halo.").
   - Present all variant source images + hints alongside the screenshot region to the vision model as a focused comparison: "Which of these N variants does the icon in this screenshot region most closely match?"
   - Replace the initially inferred `name:` with the best match.
4. **Confidence gate.** If the model is uncertain (e.g. two variants look too similar at the screenshot resolution to distinguish), emit the best-guess `name:` paired with `notes.todo: confirm variant — candidates: <a>, <b>, <c>` on the affected claim.

## 6. Detect candidate components conservatively

Walk every container claim produced in stage 4 and compare every `container: group` claim against every other for **structural identity**:

- Same ordered nested item kinds.
- Same nested-group shape.
- Same set of `*-when` keys *present* on nested groups (presence — not condition value — is part of the skeleton). `*-when` keys themselves are not emitted by this brief (define-owned); presence here means future-instance check.
- `platforms.*` overrides participate only against other `platforms.<same>` overrides; the **base** skeleton MUST still match across all instances.

Apply the conservative emission policy:

- Promote a container claim to `component: <slug>` only when **either** the operator confirms a candidate (a previous accepted Evidence carries the slug already) **or** the brief observes ≥2 structurally identical groups across screens of the *same run* (within `<lead>` plus any prior leads extracted for the same plan — synthesis aggregates across leads).
- Otherwise leave `component:` unset on the claim and add `notes.candidate_component: <slug>` so the operator can promote it explicitly later.
- Slugs MUST match `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` (kebab-case). Reserved region names (`header`, `body`, `footer`, `fab`) MUST NOT be used as slugs.
- Derive slugs from visible content (`task-row`, `setting-row`, `chip-tag`) — never from layout shape (`row-1`, `card-2`).

When in doubt, leave `component:` unset and emit the note. Promoting a note to a directive is cheap; demoting an emitted directive is operator work.

### Feed candidate skeletons forward into inference (candidate cache)

Whenever you emit a `notes.candidate_component: <slug>` hint, also write a candidate-cache sidecar so build-time component inference (`specify catalog infer`) has cross-slice memory before the composition baseline accumulates the screens. Write one YAML file per hinted group at `${PROJECT_DIR}/.specify/.cache/component-candidates/<slice>/<screen>/<group-path>.yaml`, keyed **strictly by provenance** — the slice slug, the screen slug, and the dotted group path within the screen. Never key the file by the derived slug (two distinct skeletons sharing a name would silently clobber each other) and never by a fingerprint: this is a vision brief, so it cannot compute the inference tool's canonical structural hash, and it must not try — provenance keys are collision-free by construction and the tool recomputes the canonical fingerprint at read time from the skeleton you store.

The sidecar body carries the **normalized `group` skeleton** as a composition-`group`-shaped fragment — the exact shape the inference tool's normalizer consumes — plus the derived slug stored as an inner `candidate_component:` label hint (a suggestion the build skill may adopt or override at naming time; it is never an identity) and the enclosing `region:`. Because stage 4 emits Evidence `container: group` claims (and child `container` / `leaf` claims) rather than composition nodes, perform the Evidence→composition shape translation at write time: assemble the group's `items:` array in claim order, mapping each child leaf to its `{ <kind>: { … } }` item and each nested container `group` to a nested `group:`, so the cached fragment is already in composition shape. Wiring values (`bind`, `event`, `*-when` conditions, asset / token references, free text) are illustrative and stripped before fingerprinting, so copy them through verbatim or omit them — they never affect identity.

```yaml
candidate_component: task-row   # derived slug — a non-authoritative label hint
region: body                    # enclosing region for the inferred group
group:                          # composition-`group`-shaped fragment
  items:
    - icon: {}
    - text: {}
```

Re-runs are byte-stable: the same source images produce the same provenance path and the same normalized fragment, so the cache file is overwritten in place with identical content.

## 7. Emit gaps

Record uncertainty on the affected claim under a `notes:` map when:

- Grouping is ambiguous (e.g. two visually plausible group boundaries).
- Text is unreadable or truncated.
- Icon identity is uncertain (record `notes.todo: confirm icon`; do **not** guess between e.g. `chevron-right` and `arrow-right`).
- A token reference is expected but no name resolves (`notes.todo: tokenise gap 16`).
- An asset reference is expected but `assets.yaml` does not list the ID (`notes.todo: add image '<id>' to assets.yaml`).
- A candidate component skeleton is borderline (`notes.candidate_component: <slug>` — see stage 6).

Each `notes.todo` and `notes.candidate_component` surfaces in the slice's synthesis output as a `[unknown]` tag against the affected requirement during reconciliation.

## Determinism

- Emit claims in pipeline order: regions first (in the closed-region order above), then containers (in pre-order tree walk under each region), then leaves (in pre-order tree walk under each container).
- `id`s follow the dotted-kebab grammar defined in [../extract.md](../extract.md). Re-running against unchanged inputs produces byte-identical Evidence.
- Quote `content` / `label` / `title` verbatim from the screen where legible. Light grammatical normalisation (terminal punctuation) is allowed; rephrasing is not.
- Do not invent layout properties. Omit `gap` / `padding` / `align` / `size` when measurement is unconfident; emit `notes.todo` instead.
- Do not include timestamps, host paths, or other run-state in the output.

## Idempotence

Re-runs are additive and conservative; the CLI replaces Evidence by `(<source>, <lead>)` tuple, but within a run:

- A re-run against the same source images MAY refine previously emitted body fields when the same images still support the refinement.
- Operator overrides committed at synthesis time (post-reconciliation edits in `spec.md` / `design.md`) are NOT visible to `extract`; the brief only sees the source images. Use stable `id`s so the reconciliation layer can detect and preserve operator edits.
- When the new screenshots no longer contain a previously inferred element, simply do not emit its claim. The synthesis layer detects the drop via the missing `id` and tags affected requirements with `[unknown]` / `[divergence]` — there is no `# stale-source:` annotation at the Evidence layer.
