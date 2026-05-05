---
id: composition
description: Define the visual layout of each screen as a composition.yaml artifact
generates: composition.yaml
needs: [specs, proposal]
---

Generate a `composition.yaml` file describing the spatial composition of every screen in the application. The artifact uses the region-based format with `group` containers and item vocabulary defined in RFC-7, and follows the schema at `capabilities/vectis/composition.schema.json`. Groups carry flexbox-like layout properties (`direction`, `gap`, `padding`, `align`, `justify`) and optional sizing and surface decoration.

## Input Resolution

Before generating, check whether a `composition.yaml` already exists — first in the active change directory (`.specify/changes/<name>/`), then in the baseline (`.specify/specs/`).

1. **Existing `composition.yaml` found** (skeleton or baseline from a prior change) — use it as the starting point. Preserve the region structure and all `group` layout properties (`direction`, `gap`, `padding`, `align`, `justify`, `size`, `background`, `corner_radius`, `elevation`). Add `maps_to`, `bind`, `event`, and `*-when` keys to leaf items based on the specs. Do not rearrange groups or modify layout properties.
2. **No existing `composition.yaml`** — infer layout from the specs and proposal. Use `group` containers to express how items should be arranged (rows vs columns), their spacing and alignment, and their sizing behavior. Use the item vocabulary for leaf content.

When a `design-system/tokens.yaml` file exists or `design-system` is listed in the proposal's Platforms, reference token names for `style`, `color`, and `size` properties.

## Steps

### 1. Identify Screens

Read all spec files and extract every distinct screen or page the user interacts with. Each distinct view becomes a screen entry keyed by a kebab-case slug derived from the spec's screen description.

Screen identification heuristics (when no skeleton exists):

1. **Explicit view requirements.** Requirements whose title or body describes "a screen," "a page," or "a view" each map to a screen entry. Example: "Requirement: Todo List View" → screen `todo-list`.
2. **Navigation references.** Scenarios that describe navigating to a destination imply a screen for that destination. Example: "WHEN user taps add THEN the app navigates to the add todo form" → screen `add-todo`.
3. **Distinct ViewModel states.** When the spec describes materially different data shapes for different contexts (list vs detail vs form), each shape implies a separate screen.
4. **Page transition requirements.** Requirements describing transitions between states (loading → main, error → retry) describe states within a screen, not separate screens. These become `states` entries, not screen entries.

When the spec is ambiguous about whether two behaviors belong to the same screen or separate screens, prefer fewer screens with states over more screens — the composition can always be split later.

### 2. Resolve or Infer Regions

For each screen:

- **If a skeleton exists for this screen:** Read its region and group structure. The skeleton provides the container tree — which items appear in each region, how they are grouped (rows, columns, cards), their spacing and alignment, and their token references. Do not restructure groups or modify layout properties.
- **If no skeleton exists:** Infer regions and group structure from the spec's behavioral requirements. Place the screen title and navigation actions in `header`, primary content in `body` (choosing `list`, `grid`, `form`, or group-based layout based on the data shape), secondary actions in `footer`, and a primary creation action as `fab` when appropriate. Use `group` containers to express layout intent: `direction: row` for items that should sit side-by-side, `direction: column` for stacked content, `size: { width: fill }` for elements that should expand, and surface decoration (`background`, `corner_radius`) for card-like containers.

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
- A skeleton screen with no matching spec (the screen may be decorative or the spec may be incomplete).
- A spec screen with no skeleton entry (the skeleton may need updating or the screen was added in this change).
- A spec-described data element that has no natural visual representation in any region.
- A spec-described interaction that has no interactive item to wire to.

Include gap reports as YAML comments in the output (e.g., `# GAP: spec describes "export" action but no item wired to Export event`).

## Output Structure

```yaml
version: 1

provenance:
  sources:
    - kind: manual  # or figma, legacy — based on skeleton origin

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
