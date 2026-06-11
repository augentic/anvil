# Vectis build — composition

Loaded by [../build.md](../build.md) Step 1 before any per-platform phase. Regenerates `${SLICE_DIR}/composition.yaml` from the canonical `spec.md` + `design.md` and runs the validator gate. `composition.yaml` is a build output (not a Specify artifact); the merge brief lands it into the baseline alongside the code.

## Inputs

Priority order:

0. `${PROJECT_DIR}/.specify/specs/composition.yaml` — the merged baseline composition. When present and non-empty, regeneration **accumulates**: retain every baseline screen unchanged and add, modify, or remove only the screens this slice's `spec.md` positively references. When absent (or empty — `screens: {}`), this slice establishes the initial baseline. Read this before identifying screens (Step 1) so each screen can be classified relative to the baseline.
1. `${SLICE_DIR}/specs/<domain>/spec.md` — screen titles, platform-specific behaviour, observable token / asset references.
2. `${SLICE_DIR}/design.md` — ViewModel variants, per-page view struct fields, `Event` variants, `Route` variants, capability matrix.
3. Sibling UI inputs (operator-curated, read-only): `${SLICE_DIR}/tokens.yaml` and `${SLICE_DIR}/assets.yaml` when present; otherwise `${PROJECT_DIR}/design-system/tokens.yaml` and `${PROJECT_DIR}/design-system/assets.yaml`. Used to validate token / asset references; never to author requirements.
4. Component catalog (agent-inferred, read-only): `${PROJECT_DIR}/.specify/design-system/components.yaml` when present. Each `confirmed` entry names a shared component the build must factor; `rejected` entries are intentionally declined and ignored. When absent, skip all component-factoring logic.
5. Optional prior `${SLICE_DIR}/composition.yaml` from a prior `/spec:build` run on the same slice (refining iteration). When present, preserve any operator-applied `# GAP` comments and re-validate against the updated artifacts.

## Regeneration steps

1. **Identify screens.** Walk every `### Requirement:` block in `spec.md`. A requirement is a screen when its title or body describes a view (`Requirement: Todo List View`), or when a scenario describes navigation to a destination. Derive a kebab-case slug from the title (`Todo List View` → `todo-list`). Distinct ViewModel data shapes imply separate screens; transitions between loading / main / error are states within a screen. Then classify each screen relative to the priority-0 baseline (input 0): a slug absent from the baseline is a **new** screen; a slug already in the baseline whose spec requirements this slice materially changed is a **modified** screen; a baseline slug this slice's own `spec.md` / `design.md` positively retires (a requirement this slice removes or supersedes that owned the screen, or a design note deleting it) is a **removed** screen. Baseline screens this slice does not reference are **carried forward unchanged** — absence means "belongs to another slice," never "delete." Emit a slug under `delta.removed` only on a positive retirement signal in this slice's own artifacts; non-mention is never a removal.
2. **Adopt names from `design.md`.** ViewModel variant names, per-page view struct names, and field names come from `design.md`'s Domain Model section. Use them verbatim. If `design.md` does not document a screen the spec implies, surface as `# GAP: design.md missing variant for <screen>` and continue.
3. **Place items in regions.** For each screen, place screen title and navigation actions in `header`, primary content in `body` (`list`, `grid`, `form`, or group-based layout based on the data shape), secondary actions in `footer`, and a primary creation action as `fab` when one appears in the spec. Use `group` containers (`direction`, `gap`, `padding`, `align`, `justify`, `size`, `background`, `corner_radius`, `elevation`) to express layout intent the spec / design imply.
4. **Wire bindings.** For each screen entry, add:
   - `maps_to: "ViewModel::<ScreenName>(<ScreenName>View)"` (PascalCase from the slug).
   - `bind` on display and input items — the per-page view struct field name (from `design.md`).
   - `event` on interactive items — the `Event` variant the interaction triggers. Use `EventName` for no-arg, `EventName(arg)` for events that carry item-context fields or the `value` keyword.
   - `error` on `field` items when `design.md` describes validation for the input.
   - `*-when` conditional keys when the spec describes conditional visual states (`completed items show strikethrough` → `strikethrough-when: completed`).
5. **States and overlays.** For each screen, identify alternate states from the spec (loading, empty, error, saving) and add entries under `states` with `when:` predicates and replacement `body` content. Identify dialogs / sheets / snackbars and add entries under `overlays` with `kind`, `trigger` (the `Event` name that opens the overlay), optional `title`, and `content`.
6. **Apply component catalog.** When `${PROJECT_DIR}/.specify/design-system/components.yaml` is present, read every `confirmed` entry. For each confirmed slug, apply `component: <slug>` on every composition group whose structural skeleton matches the catalog entry's usage in prior slices (baseline `composition.yaml`) or in the current slice's Evidence `component:` directives. When multiple screens share a structurally identical group that matches a confirmed catalog entry, every instance receives the same `component: <slug>` directive — this is the signal downstream shell writers use to emit a shared component file instead of inlining. Ignore `rejected` entries. When the catalog is absent, skip this step entirely (no component factoring).
6a. **Retroactively factor prior-slice screens (B7).** Component inference is incremental: a component the build just promoted (Step 0.5 named and bound a `bound-slug: null` cluster) often has its other instances in **baseline screens this slice did not author**. For each baseline screen *outside the current slice's domains* whose group is structurally identical (same fingerprint) to a newly promoted confirmed component, emit a `delta.modified.<screen>` entry that **reproduces that prior screen as a faithful superset** (read the baseline screen, copy every region unchanged — Output format below) with `component: <slug>` attached to the matching group. This is what lets a shared structure detected at the Nth screen be factored across the prior screens that already landed, with no dedicated refactoring slice. The reconciliation is safe because `/spec:execute` runs slices sequentially under the exclusive plan lock, so every prior screen is already merged into the baseline when this slice builds — there is no concurrent-edit hazard. **Directive-only constraint:** the *sole* permitted change to a not-authored baseline screen is attaching (or detaching) a `component:` directive on a group whose skeleton **already matches** the factored component. Never restructure a prior screen's layout, reorder regions, or alter wiring here — the structural-identity invariant (`check_structural_identity`) guarantees a directive-only change alters *factoring*, never *rendering*. Layout restructuring of prior screens (e.g. introducing a region they lacked) is out of scope for inline factoring and stays on the dedicated-refactoring-slice path (full-document replacement authorised at merge time, per the A3 gate above). Idempotent: a re-run sees the component already `confirmed` and the directive already attached, so it emits nothing further for that screen. **Operator-pinned parts factor identically:** a component promoted from an operator `parts.yaml` pin (Step 0.5) carries a live fingerprint in the same clustering table as an inferred one, so this retroactive factoring reaches the prior-slice screens it matches with no separate path for pinned vs inferred components (RFC §C4 reuses B7 unchanged).
7. **Per-platform overrides.** When `spec.md` platform-specific sections describe materially different layouts (not just behavioural differences), add a `platforms` map with per-platform region overrides on the affected screens.
8. **Naming proposals.** The names this step proposes — screen slugs, ViewModel variants, field names, event names — must match what `design.md` already documents. When `design.md` is silent, prefer the `design.md` conventions (snake_case fields, PascalCase ViewModel / Event names, kebab-case screen slugs). Never invent names that contradict `design.md`.
9. **Surface gaps.** Emit YAML comments (`# GAP: ...`) for any of: a spec-described data element with no natural visual representation; a spec-described interaction with no interactive item to wire; structurally recurring groups that look like a missing `component: <slug>` directive; a `bind` value that has no matching field on the per-page view struct described in `design.md`; an `event` value that has no matching variant in `design.md`. When the priority-0 baseline contains screens this slice does not reference, do **not** surface them as gaps — they belong to prior slices and are carried forward unchanged.

## Output format: delta vs full document

The envelope you write depends on the priority-0 baseline (input 0):

- **Baseline exists and is non-empty** (its `screens` map has ≥1 entry) → you MUST write the `delta: { added, modified, removed }` envelope. `added` carries screens whose slug is absent from the baseline; `modified` carries baseline screens this slice materially changed **and** prior-slice baseline screens this slice retroactively factors (Step 6a), where **each `modified` entry is a whole-screen faithful superset** (read the baseline screen, apply this slice's change — or, for a retroactive factor, attach only the `component:` directive — and reproduce every unchanged region — the merge engine replaces the entire screen entry, it does not merge sub-screen regions); `removed` carries only slugs this slice positively retires (Step 1). This drives the merge engine's accumulation path rather than its replacement path, so prior-slice baseline screens are preserved.
- **No baseline exists, or the baseline is empty** (`screens: {}`) → write the `screens:` (full-document) format to establish the initial baseline.

Delta envelope:

```yaml
version: 1
delta:
  added:
    new-screen:
      name: New Screen
      maps_to: "ViewModel::NewScreen(NewScreenView)"
      # ... full screen entry
  modified:
    existing-screen:
      name: Existing Screen
      maps_to: "ViewModel::ExistingScreen(ExistingScreenView)"
      # ... full replacement for this screen
  removed: {}
```

A whole-document `screens:` composition against a non-empty baseline is blocked at merge time (`composition-baseline-overwrite-blocked`); routine per-screen add / modify / remove flows through `delta:` and never reaches that gate.

Write via the stage → validate → rename sequence used by every Vectis producer:

```bash
COMP="${SLICE_DIR}/composition.yaml"
write_yaml "${COMP}.tmp"
specify tool run vectis -- validate composition "${COMP}.tmp"
mv "${COMP}.tmp" "${COMP}"
```

The validator auto-invokes `tokens` and `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Errors are blocking: surface the report verbatim, delete the staging file, and exit non-zero — any prior `${SLICE_DIR}/composition.yaml` is preserved untouched. Warnings forward into the operator-facing summary. Clean runs proceed silently. On a validation error, fix `spec.md` / `design.md` (or the operator-curated `tokens.yaml` / `assets.yaml`) and re-run `/spec:build`; regeneration is idempotent against unchanged inputs.

When the slice has no UI surface at all, this step writes no `composition.yaml`. Detect this from the slice's own `spec.md`: skip composition regeneration when `spec.md` describes **no** screen-bearing requirements (Step 1 identifies zero screens), regardless of which platforms `## Platforms` lists. `## Platforms` is an app-level constant stamped verbatim to every slice and never narrows per slice, so it cannot signal whether *this slice* contributes any UI — never key the skip off it.

## Validation gate (pre-shell)

After regenerating, re-run the deterministic validator against the merged input set:

```bash
specify tool run vectis -- validate composition
```

That single call covers:

1. **Composition schema validity** — `composition.yaml` conforms to the Vectis composition schema (regions, group hierarchy, allowed wiring keys, slug grammar, reserved-slug prohibitions).
2. **Wiring coverage** — every field in each per-page view struct (from `design.md`) appears as a `bind`; every shell-facing `Event` variant relevant to a screen has an `event` wiring; every `maps_to` resolves to a declared ViewModel variant; every overlay `trigger` matches an `event` name in the same screen; every `Navigate(X)` argument has a corresponding screen slug and `Route` variant.
3. **Structural identity** — every `component: <slug>` reused across screens has a structurally identical skeleton (with allowed `*-when`-gated sub-groups, state-replaced bodies, and per-instance `platforms.*` overrides).
4. **Auto-invoked `tokens` mode** — when a sibling `tokens.yaml` is present, every token reference in `composition.yaml` (and in `assets.yaml` when present) resolves against it.
5. **Auto-invoked `assets` mode** — when a sibling `assets.yaml` is present, every `image:` / `icon:` / `icon-button:` / `fab:` reference resolves to a declared asset id, every declared asset file exists on disk, and per-platform raster densities / vector exports cover the targeted shell platforms.
6. **Catalog cross-reference** — when `components.yaml` is discoverable, every `component: <slug>` in `composition.yaml` must resolve to a `confirmed` catalog entry (a `rejected` or missing entry is an error), and every `confirmed` catalog entry should have at least one `component: <slug>` reference in `composition.yaml` (warning, not error).

Validation errors halt shell generation for the affected screens. Warnings are logged and reported but do not block generation. A tool invocation failure (missing sidecar, bad arguments, unreadable preopen) is a WASI tool failure; report separately from host prerequisite failures.

When `composition.yaml` is absent (core-only slice), the validator exits cleanly without performing wired-mode checks.
