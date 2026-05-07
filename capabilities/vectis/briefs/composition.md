---
id: composition
description: Define the visual layout of each screen as a composition.yaml artifact
generates: composition.yaml
needs: [specs, proposal]
---

Generate a `composition.yaml` file describing the spatial composition of every screen in the application. The artifact uses the region-based format with `group` containers and item vocabulary defined in RFC-7, and follows the schema at `capabilities/vectis/composition.schema.json`. Groups carry flexbox-like layout properties (`direction`, `gap`, `padding`, `align`, `justify`) and optional sizing and surface decoration.

This brief discovers `layout.yaml` first from the active slice and then from the project design-system directory, falling back to existing `composition.yaml` inputs only when no layout is present (RFC-11 §H). It calls `specify tool run vectis-validate -- layout` (or `composition` when the resolved input is already wired) on the input before consuming it, and `specify tool run vectis-validate -- composition` on its output for cross-artifact token / asset checks.

## Input Resolution

Resolve the starting point for this run by checking inputs in this order. Stop at the first match.

1. **`layout.yaml` from the active slice** — `.specify/slices/<name>/layout.yaml`. Layout is unwired UI input produced by an inferer (e.g. [`vectis:image-layout-inferer`](../../../plugins/vectis/skills/image-layout-inferer/SKILL.md)) or hand-authored by the operator. The brief wires it into a `composition.yaml`.
2. **`layout.yaml` from the project** — `design-system/layout.yaml`. Same shape as slice-local layout; used when the slice is iterating against the project-wide baseline without introducing new layout intent.
3. **`composition.yaml` from the active slice** (`.specify/slices/<name>/composition.yaml`). A previously wired composition the brief is refining. Preserve every wiring key already present and re-validate against current specs.
4. **`composition.yaml` from the baseline** (`.specify/specs/composition.yaml`). The merged baseline. Use when the change has no local layout or composition yet and is proposing a delta.
5. **No input** — infer layout from the specs and proposal alone. Use `group` containers to express how items should be arranged (rows vs columns), spacing, alignment, and sizing behavior. Use the item vocabulary for leaf content.

The canonical locations are `.specify/slices/<name>/layout.yaml`, `design-system/layout.yaml`, `.specify/slices/<name>/composition.yaml`, and `.specify/specs/composition.yaml`. The CLI validators below honour the same cascade automatically when no explicit `[path]` is supplied.

When a `design-system/tokens.yaml` file exists, or an explicit slice-local `tokens.yaml` file is supplied at `.specify/slices/<name>/tokens.yaml`, reference token names for `style`, `color`, and `size` properties. The trigger keys off **file existence**, not Platforms membership: `design-system` is no longer a peer platform (RFC-11 §L), so its presence in the proposal's `Platforms` is neither sufficient nor necessary — the post-write `specify tool run vectis-validate -- composition` gate will auto-invoke `tokens` mode whenever a sibling `tokens.yaml` is present, and the absence of one short-circuits the check cleanly.

### Validate the resolved input

Before consuming the resolved input, run the deterministic CLI validator. The verb depends on which artifact resolved:

- For `layout.yaml` (cases 1–2 above) — validate the unwired subset (composition schema + `screens` only + no define-owned wiring keys + the §G structural-identity rule for any `component:` directives present):

  ```bash
  specify tool run vectis-validate -- layout
  ```

- For `composition.yaml` (cases 3–4 above) — validate the wired lifecycle artifact (composition schema + cross-artifact token / asset / wiring resolution + the §G structural-identity rule):

  ```bash
  specify tool run vectis-validate -- composition
  ```

Both verbs auto-discover the resolved path via the canonical Vectis cascade and exit non-zero on errors, zero with a printed warning report on warnings, and zero silently on a clean run. **Errors block this brief** — surface the report verbatim to the operator and stop; the brief MUST NOT fabricate a wired composition from invalid input. Warnings flow through into the operator-facing summary at the end of the brief but do not block consumption.

If case 5 applies (no input at all), there is nothing to validate up front; proceed directly to the steps below.

## Wiring Responsibilities

The brief's job is the wiring layer on top of layout-owned structure. The following rules come straight from RFC-11 §H "Wiring responsibilities" and MUST be honoured on every run:

- **Preserve layout-owned structure.** Regions, group hierarchy, `direction`, `gap`, `padding`, `align`, `justify`, `size`, `background`, `corner_radius`, `elevation`, token references, asset references, the `component: <slug>` directive on groups, comments, and `platforms.*` overrides all originate with the layout author and stay as-is.
- **Add only define-owned wiring.** `maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation targets encoded in event values, and conditional visual keys such as `strikethrough-when` are this brief's responsibility — and only this brief's.
- **Add screens only when specs require them.** When a spec describes a screen the layout has no entry for, add it with a `# inferred-from-requirements` comment so the operator can spot define-derived layout next to externally supplied layout.
- **Do not silently insert or remove a `component:` slug.** When this brief observes structurally identical groups across screens that suggest a missing slug, propose it as a `# GAP` comment adjacent to each occurrence (e.g. `# GAP: candidate component task-row`). Promotion to a directive is operator work; demotion of an existing directive is also operator work.
- **Do not rewrite token names or asset IDs** unless the existing reference is invalid AND a single confirmed replacement exists in `tokens.yaml` / `assets.yaml`. When neither holds, emit a `# GAP` comment naming the unresolved reference and stop wiring that property — `specify tool run vectis-validate -- composition` treats unresolved token / asset references as **errors** (not warnings), so the post-write gate will block the brief and the operator will see the validator report verbatim. The brief MUST NOT invent a replacement to silence the gate; an explicit `# GAP` plus a hard exit is the contract.
- **Single-artifact handoff.** v1 has no separate pre-define merge ceremony. The brief consumes one resolved input from the cascade above and reports any conflicts with prior structure as `# GAP` comments — it does not attempt to reconcile multiple layout sources itself. Future RFCs may define a richer multi-source workflow.

## Steps

### 1. Identify Screens

Read all spec files and extract every distinct screen or page the user interacts with. Each distinct view becomes a screen entry keyed by a kebab-case slug derived from the spec's screen description.

Screen identification heuristics (when no layout or composition entry exists for the screen):

1. **Explicit view requirements.** Requirements whose title or body describes "a screen," "a page," or "a view" each map to a screen entry. Example: "Requirement: Todo List View" → screen `todo-list`.
2. **Navigation references.** Scenarios that describe navigating to a destination imply a screen for that destination. Example: "WHEN user taps add THEN the app navigates to the add todo form" → screen `add-todo`.
3. **Distinct ViewModel states.** When the spec describes materially different data shapes for different contexts (list vs detail vs form), each shape implies a separate screen.
4. **Page transition requirements.** Requirements describing transitions between states (loading → main, error → retry) describe states within a screen, not separate screens. These become `states` entries, not screen entries.

When the spec is ambiguous about whether two behaviors belong to the same screen or separate screens, prefer fewer screens with states over more screens — the composition can always be split later.

### 2. Resolve or Infer Regions

For each screen:

- **If a layout entry exists for this screen** (resolved input from cases 1–4 above): Read its region and group structure. The layout provides the container tree — which items appear in each region, how they are grouped (rows, columns, cards), their spacing and alignment, and their token references. Do not restructure groups or modify layout properties (per Wiring Responsibilities above).
- **If no layout entry exists:** Infer regions and group structure from the spec's behavioral requirements. Place the screen title and navigation actions in `header`, primary content in `body` (choosing `list`, `grid`, `form`, or group-based layout based on the data shape), secondary actions in `footer`, and a primary creation action as `fab` when appropriate. Use `group` containers to express layout intent: `direction: row` for items that should sit side-by-side, `direction: column` for stacked content, `size: { width: fill }` for elements that should expand, and surface decoration (`background`, `corner_radius`) for card-like containers.

### 3. Enrich with Bindings

For each screen, add wired-mode keys:

- **`maps_to`** on the screen entry: `"ViewModel::ScreenName(ScreenNameView)"` using PascalCase derived from the screen slug.
- **`bind`** on display and input items: the field name from the per-page view struct that this item renders or edits. Derive field names from the spec's data descriptions (e.g., "remaining items count" → `count`, "todo title" → `title`).
- **`event`** on interactive items: the Event variant this interaction triggers. Derive event names from the spec's interaction descriptions (e.g., "user taps add" → `AddTodo`, "user toggles completion" → `ToggleTodo(id)`). Use the event syntax: `EventName` for no-arg events, `EventName(arg)` with item-context fields or the `value` keyword.
- **`error`** on `field` items when the spec describes validation for that input.
- **`*-when`** conditional properties when the spec describes conditional visual states (e.g., "completed items show strikethrough" → `strikethrough-when: completed`).

### 4. Add Screen States

For each screen, identify alternate states from the spec (loading, empty, error, saving). Add entries under `states` with:

- `when`: a predicate in the form `"<field> is <true|false|empty|not empty>"`.
- `body`: the content (items and groups) for that state (replaces the screen's body region by default).

### 5. Add Overlays

For each screen, identify dialogs, sheets, or snackbars from the spec (confirmation prompts, detail panels, feedback messages). Add entries under `overlays` with `kind`, `trigger` (the Event name that opens the overlay), optional `title`, and `content` (items and groups). The `trigger` value connects the overlay to the event that presents it — e.g., `trigger: RequestDelete` on a dialog means the dialog appears when `RequestDelete` fires from an `event` key elsewhere on the screen.

### 6. Platform-Specific Regions

If the proposal targets multiple platforms and the spec's platform-specific requirements sections describe materially different layouts for a screen (not just behavioral differences), add a `platforms` map with per-platform region overrides.

### 7. Surface Gaps

Report any of:
- A layout screen with no matching spec (the screen may be decorative or the spec may be incomplete).
- A spec screen with no layout entry (the layout may need updating or the screen was added in this change — see the `# inferred-from-requirements` rule under Wiring Responsibilities).
- A spec-described data element that has no natural visual representation in any region.
- A spec-described interaction that has no interactive item to wire to.
- Structurally recurring groups that look like a missing `component:` slug (per Wiring Responsibilities).

Include gap reports as YAML comments in the output (e.g., `# GAP: spec describes "export" action but no item wired to Export event`).

### 8. Validate the Output

After writing `composition.yaml`, run the cross-artifact validator:

```bash
specify tool run vectis-validate -- composition
```

This re-validates the wired composition against the patched composition schema and **automatically invokes** `tokens` and `assets` validation whenever sibling `tokens.yaml` / `assets.yaml` files exist (whether slice-local or project-level). It enforces the §G structural-identity rule on every `component: <slug>` instance in the document.

- **Errors** — block the brief. Fix `composition.yaml` (or the input artifacts) and re-run; the brief MUST NOT report success while errors remain.
- **Warnings only** — proceed, and forward the warning report into the operator-facing summary so the operator can decide whether to act now or in a follow-up change.
- **Clean** — proceed silently.

The build phase repeats `specify tool run vectis-validate -- composition` on the resolved artifact set (RFC-11 §I), so any warning the brief leaves behind will reappear there too.

## Output Structure

```yaml
version: 1

provenance:
  sources:
    - kind: manual  # or figma, legacy, screenshots, code — based on layout origin

screens:
  <screen-slug>:
    name: "<Screen Name>"
    maps_to: "ViewModel::<ScreenName>(<ScreenName>View)"

    header:
      title: "<Screen Title>"
      trailing:
        - badge: { bind: <field>, color: <token> }

    body:
      list:  # or grid, form, or group-based layout
        each: <collection_field>
        item:
          - group:
              direction: row
              gap: <token>
              align: center
              # component: <slug>          # only if already present in layout
              items:
                # leaf items with bind, event, token references
                # nested groups for sub-layout (e.g., column stack)

    footer:
      - segments: { bind: <field>, event: <Event>, options: [...] }

    fab: { icon: <name>, event: <Event> }

    states:
      <state-slug>:
        when: "<field> is <predicate>"
        body:
          - group:
              direction: column
              align: center
              items:
                # replacement items

    overlays:
      <overlay-slug>:
        kind: dialog | sheet | snackbar
        trigger: <EventName>
        title: "..."
        content:
          # items and groups

    platforms:          # only when platform regions differ
      ios:
        body: { ... }
      android:
        body: { ... }
```

## Naming Conventions

The names proposed in this artifact (screen slugs, ViewModel variants, field names, Event names) are **proposals** for the design brief that follows. The design brief adopts these names when formalizing the Rust type system, adjusting only when Rust conventions or domain model considerations require it.

- Screen slugs: kebab-case (`todo-list`, `add-todo`)
- ViewModel variants: PascalCase from screen slug (`TodoList`, `AddTodo`)
- Per-page view struct: variant name + `View` suffix (`TodoListView`, `AddTodoView`)
- Field names: snake_case (`due_date`, `title_error`)
- Event names: PascalCase (`ToggleTodo`, `SaveTodo`, `Navigate`)
