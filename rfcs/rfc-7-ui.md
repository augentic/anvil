# RFC-7: View Layout Artifact for UI Generation

> Status: Draft · Depends: — · Enables: web shell writer, improved iOS/Android shell fidelity

## Abstract

Introduce a structured **composition artifact** (`composition.yaml`) into the Specify define pipeline that describes the spatial composition of each screen as a schema-validated YAML document. Screen content is organized into named regions (`header`, `body`, `footer`, `fab`) whose contents form a lightweight **container tree** — items and `group` containers carrying a small flexbox-like property set (`direction`, `gap`, `padding`, `align`, `justify`, sizing modes, and optional surface decoration). This model maps directly to Figma Auto Layout, CSS Flexbox, SwiftUI stacks, and Compose Row/Column/Box, giving shell writers deterministic layout instructions rather than requiring them to infer container structure. The composition artifact can be authored from multiple sources — the define agent (inferred from specs), external design tools (Figma), reverse-engineering of legacy applications, or direct manual editing. This bridges the gap between behavioral specs (which define *what* the app does) and shell writers (which must decide *how* to arrange it on screen), without polluting the spec format with visual concerns.

## Motivation

### The Inference Gap

Today the pipeline from spec to screen works like this:

1. **Spec** defines behaviors (`WHEN user taps add THEN item appears in list`)
2. **Design** defines the type system (`ViewModel::TodoList(TodoListView { items: Vec<ItemView>, count: String })`)
3. **Shell writer** infers layout from the ViewModel struct fields + design tokens

Step 3 is where fidelity breaks down. The shell writer sees `TodoListView { items: Vec<ItemView>, count: String }` and produces a reasonable default — a scrollable list with a count label somewhere — but has no guidance on whether the count should be in a header bar, a floating badge, or inline at the bottom. It also has no guidance on container structure: should the item title and due date be in a horizontal row or a vertical stack? Should they fill the remaining space or hug their content? Should a group of items be wrapped in a card with rounded corners? The design system gives colors, fonts, and spacing scales; it does not give composition or layout structure.

The result is that generated UIs are *functionally correct* (every field is rendered, every event is wired) but *visually arbitrary* (layout choices are made by the LLM based on convention, not intent). For a todo app this is acceptable. For anything with a deliberate design — onboarding flows, dashboards, e-commerce product pages — it produces output that must be substantially reworked by hand.

Recent research confirms this gap. The DOne framework (Alibaba/HKUST, 2025) shows that fixing the structural hierarchy of a UI — which elements group together and how they nest — provides the single largest quality gain in design-to-code generation, more than element detection or schema guidance individually. The Figma2Code benchmark (HUST, 2025) demonstrates that preserving Figma's relative layout model (direction, gap, padding, alignment, sizing modes) dramatically outperforms both image-only inference and absolute-coordinate approaches. Every successful design-to-code tool — Facebook's Yoga engine (React Native), Google's Relay, FigmaToCode, Plasmic — converges on the same layout model: a flexbox-like container tree with a small property set that maps cleanly across platforms.

### Why Specs Should Not Change

The BDD spec format (`GIVEN... WHEN... THEN...`) defines observable behavior. This is the right abstraction for driving the Crux shared core — every `Event` variant, every `update()` match arm, every state transition traces back to a behavioral requirement. Adding layout concerns to specs would:

- **Blur the what/how boundary.** The spec says "the user sees their todo items"; it should not say "the todo items appear in a scrollable list with swipe-to-delete." The first is a requirement; the second is a layout decision.
- **Make specs brittle.** Every visual tweak (move the count badge, change from a list to a grid) would become a spec change, triggering the full define → build → merge cycle for what is fundamentally a presentation adjustment.
- **Couple the core to the shell.** The Crux architecture deliberately separates business logic (core) from presentation (shell). Specs drive the core. Layout drives the shell. Mixing them in one artifact erodes this separation.

### What Is Missing

A layer that communicates **spatial composition** — the arrangement of components on each screen, the container structure that groups them (rows, columns, cards), the mapping from ViewModel fields to visual elements, and the interaction points that wire to Event variants. This layer should be:

- **Platform-neutral.** Described in abstract primitives that map to every target — CSS Flexbox, SwiftUI stacks, Compose Row/Column/Box — not tied to any single platform's types.
- **High-fidelity.** Captures the container tree, layout direction, spacing, alignment, sizing modes, and surface decoration that distinguish a deliberate design from a generic layout.
- **Multi-source.** Authorable from design tools (Figma), legacy app analysis, manual editing, or agent inference — not limited to a single authoring path. Figma Auto Layout maps directly to the layout model.
- **Verifiable.** Every ViewModel field must appear in the layout; every Event must be wired to an interaction point.
- **Diffable.** Layout changes show up in version control as text diffs and support the existing ADDED/MODIFIED/REMOVED delta operations.
- **Generatable.** The define agent can produce it from the spec and proposal, the same way it produces `design.md` today. When external input is available (e.g., a Figma import), the agent enriches rather than invents.

## Design Principles

| Use the composition artifact when: | Keep in the spec when: | Keep in the design when: |
| --- | --- | --- |
| Deciding *where* a field appears on screen | Deciding *what* the field's value means | Deciding *what type* the field is |
| Choosing between a list, grid, or card layout | Specifying that items must be scrollable | Defining the ViewModel struct and its fields |
| Grouping items in rows, columns, or cards | Specifying that tapping "add" creates an item | Mapping the Event variant to `update()` logic |
| Specifying spacing, alignment, and sizing | Specifying page transitions and navigation | Defining the Route and Page enums |
| Placing a floating action button | Specifying error states and recovery | Defining capability requirements |
| Referencing design tokens for spacing/color | | |

The boundary follows the existing Specify principle: specs define behavior, design defines the technical contract, and the new artifact defines visual arrangement. Shell writers consume all three.

## Detailed Design

### New Artifact: `composition.yaml`

A schema-validated YAML document that describes the layout of each screen in the application using a region-based format with a lightweight container tree. Each screen declares its content through named regions (`header`, `body`, `footer`, `fab`). Within each region, content is organized as a tree of **items** (leaf elements like `text`, `button`, `field`) and **groups** (container nodes carrying flexbox-like layout properties). Groups specify `direction` (row/column/stack), `gap`, `padding`, `align`, `justify`, optional sizing, and optional surface decoration (`background`, `corner_radius`, `elevation`). This property set maps directly to Figma Auto Layout, CSS Flexbox, SwiftUI HStack/VStack/ZStack, and Compose Row/Column/Box — giving shell writers deterministic, platform-neutral layout instructions.

One entry per screen keyed by slug. The artifact lives alongside `spec.md` in the Specify lifecycle — per-change deltas in `.specify/changes/<name>/`, merged baseline in `.specify/specs/`.

`composition.yaml` supports two modes:

1. **Skeleton mode.** Regions with items, token references, and content hints — but no `bind`, `event`, or `maps_to` keys. This is the form produced by external tools (Figma adapters, legacy extractors) and by manual authoring before the define pipeline runs.

2. **Wired mode.** The same regions enriched with data bindings (`bind`), event wiring (`event`), and `maps_to` traceability. This is the form produced by the define pipeline and consumed by shell writers.

The define pipeline reads an existing skeleton (when present), preserves its region structure, and adds bindings and wiring based on the specs and design. When no skeleton exists, the pipeline infers layout from the specs and proposal — the same inference that would otherwise fall to shell writers, but captured as an explicit, reviewable artifact.

#### Skeleton Example

A skeleton authored before the define pipeline runs — no `bind`, `event`, or `maps_to` keys, just the region structure with layout properties, token references, and content hints. The `group` containers carry flexbox-like properties (`direction`, `gap`, `align`, etc.) imported directly from Figma's Auto Layout structure:

```yaml
# composition.yaml — skeleton (pre-define)
version: 1

provenance:
  sources:
    - kind: figma
      uri: "https://www.figma.com/design/abc123/MyApp"
      captured_at: "2026-04-25T08:00:00Z"

screens:
  todo-list:
    name: "Todo List"
    description: "Main screen showing all todo items"

    header:
      title: "My Todos"
      trailing:
        - badge: { color: primaryContainer }

    body:
      list:
        each: items
        item:
          - group:
              direction: row
              gap: md
              align: center
              items:
                - checkbox
                - group:
                    direction: column
                    gap: xs
                    size: { width: fill }
                    items:
                      - text: { style: body }
                      - text: { style: caption, color: onSurfaceVariant }
                - action: { icon: trash }

    footer:
      - segments: { options: ["All", "Active", "Completed"] }

    fab: { icon: plus }

    states:
      empty:
        when: "no items to display"
        body:
          - group:
              direction: column
              gap: md
              align: center
              padding: xl
              items:
                - icon: { name: clipboard, style: xl, color: onSurfaceVariant }
                - text: { content: "No todos yet", style: title }
                - text: { content: "Tap + to add your first todo", style: body, color: onSurfaceVariant }
```

#### Wired Example

After the define pipeline enriches the skeleton. This example shows two screens — a list and a form — with navigation between them, loading/error states, and a confirmation dialog. The `group` containers preserve the layout structure from the skeleton while `bind`, `event`, and `maps_to` keys are added.

Given:
- `ViewModel::TodoList(TodoListView { items: Vec<ItemView>, count: String, filter: String, loading: bool, error_message: String })`
- `ViewModel::AddTodo(AddTodoView { title: String, due_date: String, saving: bool, title_error: String })`

```yaml
# composition.yaml — wired (post-define)
version: 1

screens:
  todo-list:
    name: "Todo List"
    maps_to: "ViewModel::TodoList(TodoListView)"

    header:
      title: "My Todos"
      trailing:
        - badge: { bind: count, color: primaryContainer }

    body:
      list:
        each: items
        item:
          - group:
              direction: row
              gap: md
              align: center
              items:
                - checkbox: { bind: completed, event: ToggleTodo(id) }
                - group:
                    direction: column
                    gap: xs
                    size: { width: fill }
                    items:
                      - text: { bind: title, style: body, strikethrough-when: completed }
                      - text: { bind: due_date, style: caption, color: onSurfaceVariant }
                - action: { icon: trash, event: RequestDelete(id) }

    footer:
      - segments: { bind: filter, event: SetFilter(value), options: ["All", "Active", "Completed"] }

    fab: { icon: plus, event: Navigate(AddTodo) }

    states:
      loading:
        when: "loading is true"
        body:
          - group:
              direction: column
              gap: md
              align: center
              padding: xl
              items:
                - progress: { color: primary }
                - text: { content: "Loading todos…", style: body, color: onSurfaceVariant }

      error:
        when: "error_message is not empty"
        body:
          - group:
              direction: column
              gap: md
              align: center
              padding: xl
              items:
                - icon: { name: alert-circle, style: xl, color: error }
                - text: { bind: error_message, style: body, color: onSurfaceVariant }
                - button: { content: "Retry", style: filled, event: RetryLoad }

      empty:
        when: "items is empty"
        body:
          - group:
              direction: column
              gap: md
              align: center
              padding: xl
              items:
                - icon: { name: clipboard, style: xl, color: onSurfaceVariant }
                - text: { content: "No todos yet", style: title }
                - text: { content: "Tap + to add your first todo", style: body, color: onSurfaceVariant }

    overlays:
      delete-confirmation:
        kind: dialog
        trigger: RequestDelete
        title: "Delete Todo?"
        content:
          - text: { content: "This action cannot be undone.", style: body }
          - group:
              direction: row
              gap: sm
              justify: end
              items:
                - button: { content: "Cancel", style: text, event: DismissDialog }
                - button: { content: "Delete", style: filled, color: error, event: ConfirmDelete(id) }

  add-todo:
    name: "Add Todo"
    maps_to: "ViewModel::AddTodo(AddTodoView)"

    header:
      leading:
        - action: { icon: back, event: NavigateBack }
      title: "New Todo"

    body:
      form:
        - field: { bind: title, event: UpdateTitle(value), placeholder: "What needs to be done?", error: title_error }
        - field: { bind: due_date, event: UpdateDueDate(value), placeholder: "Due date (optional)" }
        - button: { content: "Save", style: filled, event: SaveTodo, disabled-when: saving }

    states:
      saving:
        when: "saving is true"
        replaces: screen
        body:
          - group:
              direction: column
              align: center
              justify: center
              items:
                - progress: { color: primary }
```

Enrichment adds: `maps_to` traceability on screens, `bind` keys connecting items to ViewModel fields, `event` keys wiring interactions to Event variants, and conditional `*-when` properties. The group structure and layout properties are preserved unchanged — the pipeline adds data bindings, it does not rearrange the layout.

The example demonstrates several patterns:
- **Container structure:** List items use `group: { direction: row }` to arrange checkbox, text stack, and delete action horizontally. The inner `group: { direction: column, size: { width: fill } }` stacks title and due date vertically while filling the remaining horizontal space.
- **Cross-screen navigation:** `event: Navigate(AddTodo)` on the FAB, `event: NavigateBack` on the back button.
- **Loading, error, and saving states:** `when: "loading is true"`, `when: "error_message is not empty"`, and `when: "saving is true"` with progress indicators, error display with retry button, and full-screen overlay respectively. Each state body wraps content in a centered column group.
- **Dialogs:** The overlay's `trigger: RequestDelete` declares which event opens it. Dialog buttons are grouped in a `direction: row` with `justify: end`. Confirmation via `event: ConfirmDelete(id)`, dismissal via `event: DismissDialog`.
- **Form validation:** `error: title_error` on a `field` item.
- **Disabled state:** `disabled-when: saving` on the save button.
- **Content patterns:** The todo-list screen uses `body.list` for iterable content; the add-todo screen uses `body.form` for vertical input layout; states use groups with `align: center` for centered content.

#### Provenance

The composition artifact optionally tracks where its content came from via the top-level `provenance` key:

```yaml
provenance:
  sources:
    - kind: figma
      uri: "https://www.figma.com/design/abc123/MyApp"
      captured_at: "2026-04-25T08:00:00Z"
    - kind: manual
```

Supported `kind` values:

| Kind | Description |
| --- | --- |
| `figma` | Imported from a Figma file via adapter tooling |
| `legacy` | Reverse-engineered from a legacy application |
| `manual` | Authored directly by a human or agent |

Multiple sources can contribute to the same artifact. This is the expected case — import from Figma as a starting point, then refine manually. The provenance block is optional; its absence implies agent-generated or manual authoring.

#### Authoring Modes

##### Agent Inference (the Default)

When no skeleton exists, the define pipeline's composition brief infers layout from the specs and proposal. This is the zero-configuration path and produces the same quality of layout decisions that would otherwise fall to shell writers, but captured as an explicit, reviewable artifact.

##### Figma Import

A Figma adapter reads a Figma file's frame hierarchy and produces a `composition.yaml` skeleton directly. The mapping leverages Figma Auto Layout, which is a flexbox model — the same model used by the composition artifact's `group` containers:

- Figma Frames → screen entries
- Figma Auto Layout frames → `group` containers with `direction` (from layoutMode), `gap` (from itemSpacing), `padding` (from paddingTop/Right/Bottom/Left), `align` (from counterAxisAlignItems), `justify` (from primaryAxisAlignItems)
- Figma "Hug contents" → `size: { width: hug }` or default (absent)
- Figma "Fill container" → `size: { width: fill }`
- Figma "Fixed size" → `size: { width: N }`
- Figma navigation bars → `header` region with title and trailing/leading items
- Figma main content area → `body` region (list, grid, form, or groups based on content pattern)
- Figma bottom bars → `footer` region
- Figma FABs → `fab` item
- Figma Text layers → `text` items with style tokens
- Figma Icons → `icon` or `action` items
- Figma input fields → `field`, `checkbox`, `switch`, etc.
- Figma frames with fills, corner radius, or effects → `group` with `background`, `corner_radius`, `elevation`
- Unrecognized patterns → best-match item with a `# TODO: review` comment

Because Figma Auto Layout maps directly to the composition's layout model, the adapter preserves the designer's intended container structure — which elements are grouped in rows vs columns, how they're spaced and aligned, and which containers have card-like decoration. This structural fidelity is what enables high-fidelity code generation downstream.

##### Legacy App Reverse-Engineering

When `/spec:extract` runs against a legacy application, it could optionally produce a `composition.yaml` skeleton alongside the extracted specs:

- Screen components in the legacy code → screen entries with region structure
- Navigation bars → `header` region
- List/table views → `body.list` or `body.grid` patterns
- Form views → `body.form` patterns
- Bottom bars / tab bars → `footer` region
- Flex/grid containers → `group` nodes with `direction`, `gap`, `align` derived from CSS flexbox or framework layout props
- Card/panel components → `group` with `background`, `corner_radius`, `elevation`
- UI framework widgets → item vocabulary mapping (e.g., React `<input>` → `field`, React `<button>` → `button`)

This fits naturally with the existing RT plugin's analysis capabilities. However, the current `/spec:extract` skill does not produce composition artifacts. **Extract integration is deferred beyond Phase 1** — it is not a prerequisite for any phase in the adoption path. When extract gains composition support, it will produce a skeleton `composition.yaml` that the define pipeline enriches during the next `/spec:define` run, using the same skeleton-to-wired enrichment strategy described above.

##### Manual Authoring

Direct editing of `composition.yaml`. The region-based format with `group` containers maps intuitively to the visual structure a designer has in mind: header at the top, body in the middle with items arranged in rows and columns, footer at the bottom, fab floating. Skeletons are valid without any `bind`, `event`, or `maps_to` keys, so manual authoring does not require knowledge of the type system. The `group` properties (`direction`, `gap`, `padding`, `align`) are the same concepts designers work with in Figma Auto Layout.

##### Hybrid (the Common Case)

Import from Figma or a legacy app as a starting point, then manually refine: add missing screens, adjust item choices, align token references with `tokens.yaml`. The `provenance` block tracks which sources contributed, enabling auditing.

##### Skeleton-to-Wired Enrichment

When the composition brief receives an existing skeleton, it must match skeleton items to spec behaviors without restructuring the layout. This matching follows explicit heuristics, not unconstrained inference:

1. **Screen matching.** Match skeleton screen slugs to spec-described screens by name similarity. If a skeleton has `todo-list` and the spec describes "a screen showing the user's todo items," the match is direct. Unmatched skeleton screens are preserved with a `# GAP` comment. Unmatched spec screens get new inferred entries.

2. **Collection binding.** A `list` or `grid` pattern in the body with an `each` key binds to the `Vec<T>` field whose item type contains fields matching the leaf items inside the item template (including items nested inside `group` containers). Heuristic: if the skeleton's item template has two `text` items and a `checkbox` (possibly grouped in rows and columns), and the spec describes items with a title, due date, and completion state, the agent binds `each: items` and the inner items to the corresponding fields by positional and semantic matching (first text → `title`, second text → `due_date`, checkbox → `completed`). The `group` structure is preserved unchanged.

3. **Display item binding.** `text`, `badge`, `image`, and other display items bind to the view struct field whose semantic role matches the item's position and context. A `text` in the header with `content: "My Todos"` is static (no `bind`). A `badge` in `header.trailing` with no `content` but a token reference binds to the count-like field. Heuristic: items with a `content` property containing a literal string are static; items without `content` in a position that implies dynamic data get a `bind`.

4. **Input item wiring.** `field`, `checkbox`, `switch`, `slider`, `segments`, and `dropdown` items are inherently interactive. Each gets both a `bind` (to the field it displays/edits) and an `event` (the Event variant triggered on change). The agent derives the event name from the spec's interaction descriptions for that field.

5. **Action item wiring.** `button`, `action`, and `fab` items get an `event` key derived from the spec's interaction descriptions. If the skeleton has an `action` with `icon: trash` and the spec describes "user can delete items," the agent wires `event: DeleteTodo(id)` (or `event: RequestDelete(id)` if the spec describes a confirmation flow).

6. **Decorative items and groups.** Items that serve a purely visual purpose (`divider`, static `icon` with no interaction, static `text` with literal content) are left untouched — no `bind` or `event` added. `group` containers and their layout properties (`direction`, `gap`, `padding`, `align`, `justify`, `background`, `corner_radius`, `elevation`, `size`) are never modified by enrichment. The enrichment pipeline adds data bindings to leaf items; it does not rearrange, restructure, or modify container layout.

7. **Ambiguous matches.** When the agent cannot confidently match a skeleton item to a spec behavior, it adds a `# TODO: review binding` comment rather than guessing. This preserves the skeleton while flagging areas for human review.

#### Format Rules

1. **Top-level structure.** Every `composition.yaml` has a `version` key (currently `1`) and a `screens` map keyed by screen slug. Optional top-level keys: `provenance`, `delta` (for per-change artifacts). There is one `composition.yaml` per change, not one per feature. When a change involves multiple features (multiple spec files), the composition brief reads all specs for the change and produces a single `composition.yaml` containing all screens across all features. Screen slugs must be unique across the entire file — if two features introduce screens with the same name, the composition brief must disambiguate (e.g., `settings-account` vs `settings-notifications` instead of two `settings` entries).

2. **Screen entries.** Each screen entry has a `name`, optional `description`, and one or more region keys (`header`, `body`, `footer`, `fab`). Optional keys: `states`, `overlays`, `platforms`. In wired mode, the entry gains `maps_to` establishing traceability to a ViewModel variant. In skeleton mode, `maps_to` is absent.

3. **Regions.** Each screen is divided into named regions that map to standard screen areas:

   - **`header`** — Top navigation bar. Contains `title` (string), optional `leading` (content node array for left-side actions like back buttons), and optional `trailing` (content node array for right-side actions like badges, search, settings).
   - **`body`** — Main content area. Can be one of:
     - **`list`** pattern — `{ each: field, item: [...items] }` for scrollable list content.
     - **`grid`** pattern — `{ each: field, columns: N, item: [...items] }` for grid layouts.
     - **`form`** pattern — a content node array rendered as a vertical input layout.
     - **Content node array** — items and groups rendered as centered/stacked content (used for states and simple screens).
   - **`footer`** — Bottom bar area. A content node array (typically segments, buttons, or tab items).
   - **`fab`** — Floating action button. A single item (typically `{ icon: name, event: Event }`).

   All regions are optional. Shell writers map each region to platform-native containers: `header` → `NavigationTitle` + toolbar (iOS) / `TopAppBar` (Android), `body` → main content view, `footer` → bottom toolbar / `BottomAppBar`, `fab` → overlay button / `FloatingActionButton`.

4. **Items and groups.** Content within regions is expressed as a tree of **items** and **groups**.

   - **Items** are leaf elements — YAML mappings with a single key (the item type) whose value is either `null` (bare item like `- divider`) or a properties object (e.g., `- text: { bind: title, style: body }`). Items describe content: what data to display or what interaction to offer.
   - **Groups** are container nodes that organize items (and other groups) with flexbox-like layout properties. A group is written as `- group:` followed by layout properties and an `items` array containing children. Groups describe structure: how content is arranged spatially.

   Together, items and groups form a shallow tree within each region. The tree depth is typically 2–3 levels — enough to express rows within columns (or vice versa), cards containing content stacks, and similar real-world patterns. Deeply nested trees (5+ levels) should be flattened where possible.

5. **Group layout properties.** Groups carry a small set of flexbox-like properties that map directly to every target platform:

   | Property | Values | Default | Maps to |
   | --- | --- | --- | --- |
   | `direction` | `row`, `column`, `stack` | `column` | flex-direction / HStack-VStack-ZStack / Row-Column-Box |
   | `gap` | token ref or number | none | gap / spacing / Arrangement.spacedBy |
   | `padding` | token ref, number, or `{ top, right, bottom, left }` | none | padding |
   | `align` | `start`, `center`, `end`, `stretch`, `baseline` | `stretch` | align-items / alignment / verticalAlignment-horizontalAlignment |
   | `justify` | `start`, `center`, `end`, `space-between`, `space-around` | `start` | justify-content / implicit in stack / horizontalArrangement-verticalArrangement |
   | `wrap` | boolean | `false` | flex-wrap / LazyVGrid alternative / FlowRow |

   These are the only layout properties on groups. They match the universal flexbox subset: Figma Auto Layout, CSS Flexbox, SwiftUI stacks, Compose Row/Column/Box, and React Native's Yoga engine.

6. **Sizing.** Items and groups accept an optional `size` property that specifies responsive sizing behavior:

   ```yaml
   size: { width: fill }              # expand to fill available space
   size: { width: 48, height: 48 }    # fixed dimensions
   size: { width: fill, height: 200 } # fill width, fixed height
   ```

   Each dimension (`width`, `height`) is one of:
   - A **number** — fixed size in logical pixels/points (maps to explicit width/height).
   - **`fill`** — expand to fill available space (maps to `flex: 1` / `.frame(maxWidth: .infinity)` / `Modifier.fillMaxWidth()`).
   - **`hug`** — size to content (the default when `size` is absent; maps to intrinsic sizing).

   Sizing captures the three fundamental responsive modes that Figma, CSS, SwiftUI, and Compose all support. When `size` is absent, the element uses its intrinsic size (hug).

7. **Surface decoration.** Groups accept optional surface decoration properties for card-like containers:

   | Property | Values | Maps to |
   | --- | --- | --- |
   | `background` | token ref (e.g., `surfaceContainer`) | background color / fill |
   | `corner_radius` | token ref or number | border-radius / cornerRadius / clip(RoundedCornerShape) |
   | `elevation` | token ref (e.g., `sm`) | box-shadow / shadow / Modifier.shadow |
   | `border` | `{ color: token, width: number }` | border / overlay(RoundedRectangle) / Modifier.border |

   Decoration properties are optional. When absent, groups are transparent containers with no visual treatment. Decoration is valid in both skeleton and wired modes — it is a layout concern, not a data-binding concern.

8. **Field bindings (wired mode).** The `bind` key on an item connects it to a per-page view struct field. In wired mode, every field in the view struct must appear as a `bind` value at least once. In skeleton mode, `bind` keys are absent.

9. **Event wiring (wired mode).** The `event` key on interactive items wires them to shell-facing Event variants. The value follows the syntax `EventName` or `EventName(arg1, arg2)`. Arguments are one of three kinds:

   - **Item-context fields.** Inside an `each` iteration, bare names like `id` or `completed` refer to fields on the current item struct. Example: `event: ToggleTodo(id)` inside a list with `each: items` means "send `Event::ToggleTodo` with the current item's `id` field."
   - **The `value` keyword.** On input items (`field`, `segments`, `slider`, `dropdown`), `value` is a reserved keyword meaning "the item's current input value." Example: `event: UpdateTitle(value)` on a `field` means "send `Event::UpdateTitle` with whatever the user typed."
   - **Screen-slug references.** In `Navigate(ScreenName)`, the argument is a PascalCase screen name that maps to a Route variant. Example: `event: Navigate(AddTodo)` maps to `Event::Navigate(Route::AddTodo)`.

   Events with no arguments omit parentheses: `event: NavigateBack`, `event: DismissDialog`. Multiple arguments are comma-separated: `event: MoveItem(id, position)`. In wired mode, every shell-facing Event that belongs to this screen must be wired. In skeleton mode, `event` keys are absent.

10. **Navigation mapping.** `event: Navigate(ScreenName)` uses a PascalCase argument that maps to both a screen slug and a Route variant via deterministic conversion:
   - PascalCase argument → kebab-case screen slug: `AddTodo` → `add-todo` (insert hyphens at case boundaries, lowercase).
   - PascalCase argument → Route variant: `AddTodo` → `Route::AddTodo` (identity).
   - The reverse applies for validation: screen slug `add-todo` → PascalCase `AddTodo`.
   
   This three-way mapping (event argument ↔ screen slug ↔ Route variant) is deterministic and validation checks all three references for consistency: every `Navigate(X)` must have a corresponding screen slug in composition and a corresponding Route variant in design.

11. **Design token references.** Properties like `style` (typography and display size), `color`, `gap`, and `padding` reference design system tokens by name. Shell writers resolve these to `VectisTypography.title`, `VectisColors.primary`, `VectisSpacing.md` on each platform. On items, `style` controls the visual variant (e.g., `body` for text, `xl` for icon display size, `filled` for button style). On groups, `gap` and `padding` reference spacing tokens. Valid in both skeleton and wired modes.

12. **Conditional rendering.** Two syntax forms serve different contexts:

   - **Screen-level conditions** (`states[].when`): A predicate expression with the syntax `"<field> is <predicate>"`. Supported predicates: `is true`, `is false`, `is empty`, `is not empty`. The field name references a boolean or collection field on the screen's per-page view struct. Examples: `when: "loading is true"`, `when: "items is empty"`. In skeleton mode, the `when` value is a plain descriptive string (e.g., `when: "no items to display"`) that the enrichment pipeline replaces with a formal predicate.

     **Predicate language limitations and extensibility.** The initial predicate set (`is true`, `is false`, `is empty`, `is not empty`) is deliberately minimal — it covers the common cases (loading states, empty states, error states) with predicates that map cleanly to `if field` / `if field.is_empty()` in Rust. Real applications will encounter conditions that don't fit this vocabulary: comparisons (`count > 0`), enum matching (`status is error`), or compound predicates (`loading is false AND items is not empty`). The recommended approach for Phase 1 is to push complex conditions into the core: the `view()` function computes a boolean field (e.g., `show_empty_state: bool`) and the composition predicate tests that boolean. This keeps the predicate language simple and the composition artifact thin. If demand for richer predicates emerges, extending the grammar is a backward-compatible change — new predicate forms (e.g., `is "value"`, `> N`, `AND`/`OR` combinators) can be added in a future RFC without invalidating existing predicates.

     Each state entry has a `replaces` key that declares what the state's body replaces when the condition is true:
     - `replaces: body` (the default when `replaces` is omitted) — the state's `body` replaces only the body region of the screen. The header, footer, and fab remain visible. This is the common case for loading, empty, and error states where the screen chrome should persist.
     - `replaces: screen` — the state replaces the entire screen, including all regions. Use this for full-screen takeovers like splash screens or blocking error pages.
   - **Item-level conditions** (`*-when` properties): The value is a bare field name referencing a boolean field on the current view struct (or item struct within an `each` context). The property name prefix determines the effect. Examples: `disabled-when: saving` (disable when `saving` is true), `strikethrough-when: completed` (apply strikethrough when `completed` is true), `visible-when: has_avatar` (show only when `has_avatar` is true). The `*-when` pattern is open — any visual property can be made conditional by appending `-when` to its name.

13. **Iteration.** The `each` key on a body content pattern (`list` or `grid`) describes repeated content bound to a `Vec<T>` field. The `item` key holds the items for each element. In skeleton mode, `each` names the collection conceptually; in wired mode, it binds to a specific field. Iteration contexts are nested — within an `each` block, `bind` and `event` arguments reference fields on the item struct, not the screen's per-page view struct. In nested iteration (e.g., `each: sections` containing a nested `list` with `each: items`), the innermost `each` context takes precedence: `bind` values resolve to fields on the innermost item struct. To reference a field on an outer iteration context from a nested context, use dot notation: `bind: section.heading`. Dot notation follows the pattern `<each-name>.<field>`, where `<each-name>` matches the `each` key of the target iteration level. The common pattern is to place bindings at the appropriate nesting level rather than using dot notation (as in the settings example), but dot notation provides an escape hatch when an item inside an inner loop must reference data from an outer loop.

14. **Overlays.** Dialogs, sheets, and snackbars appear under the screen's `overlays` map, keyed by slug. Each overlay has a `kind`, a `trigger` (the Event name that causes the overlay to present), optional `title`, and a `content` array (items and groups). They are not part of the main regions — they are presented modally when the trigger event fires. The `trigger` value is an Event name without arguments (e.g., `trigger: RequestDelete`); shell writers match it against `event` keys elsewhere on the screen to wire presentation logic. In skeleton mode, `trigger` is absent.

15. **Platform-specific regions.** When a screen's layout differs between platforms, the screen gains a `platforms` map with per-platform region overrides that replace the shared regions for that platform. Shell writers use the platform-specific regions when present, falling back to the shared regions when absent. Only overridden regions are specified — unspecified regions fall through to the shared definition.

16. **Accessibility annotations.** Optional `label`, `role`, and `hint` properties on items provide screen reader semantics. Valid in both skeleton and wired modes. See [Accessibility Annotations](#accessibility-annotations).

### Schema

The JSON Schema enforces the structure described in the Format Rules above. This section defines the type shape that the schema validates, expressed as TypeScript-style interfaces for readability. The actual JSON Schema file lives at `schemas/vectis/composition.schema.json`. A draft of the full JSON Schema is provided in [Appendix A](#appendix-a-composition-json-schema) — translating these TypeScript interfaces into the shipping schema is a Phase 1 deliverable.

#### Top-Level Structure

```typescript
interface CompositionDocument {
  version: 1;
  provenance?: Provenance;
  custom_items?: CustomItem[];
  // Baseline documents use `screens`; per-change deltas use `delta`
  screens?: Record<ScreenSlug, ScreenEntry>;
  delta?: DeltaDocument;
}

interface CustomItem {
  name: string;              // lowercase item type name, validation allowlist entry
  description?: string;
}

type ScreenSlug = string; // kebab-case, e.g. "todo-list", "add-todo"
```

#### Provenance

```typescript
interface Provenance {
  sources: ProvenanceSource[];
}

interface ProvenanceSource {
  kind: "figma" | "legacy" | "manual";
  uri?: string;
  captured_at?: string; // ISO 8601 datetime
}
```

#### Screen Entry

```typescript
interface ScreenEntry {
  name: string;
  description?: string;
  maps_to?: string;          // wired mode only, e.g. "ViewModel::TodoList(TodoListView)"
  header?: HeaderRegion;
  body?: BodyRegion;
  footer?: ContentNode[];
  fab?: ItemProps;            // single item (e.g. { icon: "plus", event: "Navigate(AddTodo)" })
  states?: Record<string, StateEntry>;
  overlays?: Record<string, OverlayEntry>;
  platforms?: Record<PlatformId, PlatformOverride>;
}

type PlatformId = "ios" | "android" | "web";

// A ContentNode is either a leaf item or a group container
type ContentNode = Item | GroupItem;

// A group is written as `- group: { direction: row, items: [...] }`
type GroupItem = { group: GroupProps };

interface HeaderRegion {
  title?: string;
  leading?: ContentNode[];   // left-side items (back button, menu icon)
  trailing?: ContentNode[];  // right-side items (badges, action icons)
}

// Body is polymorphic: one of these four shapes
type BodyRegion =
  | { list: ListPattern }
  | { grid: GridPattern }
  | { form: ContentNode[] }
  | ContentNode[];           // item/group array for centered/stacked content

interface ListPattern {
  each: string;              // field name of Vec<T> collection
  item: ContentNode[];       // item template for each element (items and groups)
  style?: string;            // e.g. "grouped" for iOS grouped list style
}

interface GridPattern {
  each: string;
  columns: number;
  item: ContentNode[];
}

interface StateEntry {
  when: string;              // predicate: "<field> is <true|false|empty|not empty>" (wired)
                             // or plain descriptive string (skeleton)
  replaces?: "body" | "screen";  // default: "body" (replaces body region only)
  body?: BodyRegion | ContentNode[];  // replacement body content
  header?: HeaderRegion;         // only when replaces: "screen"
  footer?: ContentNode[];        // only when replaces: "screen"
  fab?: ItemProps;               // only when replaces: "screen"
}

interface OverlayEntry {
  kind: "dialog" | "sheet" | "snackbar";
  trigger?: string;          // wired mode: Event name that opens this overlay (e.g. "RequestDelete")
  title?: string;
  content: ContentNode[];    // overlay content (items and groups)
}

interface PlatformOverride {
  header?: HeaderRegion;
  body?: BodyRegion;
  footer?: ContentNode[];
  fab?: ItemProps;
}
```

#### Group

Groups are container nodes with flexbox-like layout properties and an `items` array of children:

```typescript
interface GroupProps {
  // --- Layout (flexbox subset) ---
  direction?: "row" | "column" | "stack";  // default: "column"
  gap?: TokenRef | number;
  padding?: TokenRef | number | PaddingSpec;
  align?: "start" | "center" | "end" | "stretch" | "baseline";  // default: "stretch"
  justify?: "start" | "center" | "end" | "space-between" | "space-around";  // default: "start"
  wrap?: boolean;            // default: false

  // --- Sizing ---
  size?: SizingSpec;

  // --- Surface decoration (optional, for card-like containers) ---
  background?: TokenRef;     // e.g. "surfaceContainer", "primaryContainer"
  corner_radius?: TokenRef | number;
  elevation?: TokenRef;      // e.g. "sm", "md"
  border?: { color: TokenRef; width: number };

  // --- Children ---
  items: ContentNode[];      // the group's children (items and nested groups)

  // --- Conditional (wired mode) ---
  [key: `${string}-when`]?: string;  // e.g. visible-when: has_items

  // --- Accessibility ---
  label?: string;
  role?: "heading" | "button" | "image" | "link";
  hint?: string;
}

interface PaddingSpec {
  top?: TokenRef | number;
  right?: TokenRef | number;
  bottom?: TokenRef | number;
  left?: TokenRef | number;
}
```

#### Sizing

Items and groups share the same sizing model:

```typescript
interface SizingSpec {
  width?: SizingValue;
  height?: SizingValue;
}

type SizingValue =
  | number         // fixed size in logical pixels/points
  | "fill"         // expand to fill available space (flex: 1)
  | "hug";         // size to content (the default when size is absent)
```

#### Item

Each leaf item in a region is a YAML mapping with a single key (the item type) whose value is either `null` (bare item) or a properties object:

```typescript
// In YAML: `- divider` or `- text: { bind: title, style: body }`
type Item = Record<ItemType, ItemProps | null>;

type ItemType = string; // lowercase, from vocabulary or custom_items

interface ItemProps {
  // --- Wiring (wired mode only) ---
  bind?: string;             // field name on the per-page view struct (or item struct in `each` context)
                             // Bare name (e.g. "title") resolves to the innermost iteration context.
                             // Dot notation (e.g. "section.heading") references an outer iteration
                             // context by its `each` name — only valid inside nested `each` blocks.
                             // Pattern: ^[a-z_][a-z0-9_]*(\.[a-z_][a-z0-9_]*)?$
  event?: string;            // "EventName" or "EventName(arg1, arg2)"
  error?: string;            // field name for validation error display (field items)

  // --- Content ---
  content?: string;          // static text content
  name?: string;             // icon name (for icon items)
  icon?: string;             // icon name (for action, fab items)
  placeholder?: string;      // input placeholder (for field items)
  options?: string[];         // options for segments/dropdown items

  // --- Styling ---
  style?: string;            // typography token or component variant, e.g. "title", "body", "filled"
  color?: TokenRef;          // e.g. "primary", "onSurfaceVariant", "error"

  // --- Sizing ---
  size?: SizingSpec;         // responsive sizing (fixed, fill, or hug)
  corner_radius?: TokenRef | number;  // for image items

  // --- Conditional (wired mode) ---
  // Any property suffixed with `-when` takes a field name (boolean)
  [key: `${string}-when`]?: string;

  // --- Nested iteration (for list/grid items within body) ---
  each?: string;             // field name of Vec<T> collection (nested list)
  columns?: number;          // grid columns (nested grid)
  item?: ContentNode[];      // item template for nested iteration

  // --- Accessibility ---
  label?: string;            // accessible label (string literal or field name)
  role?: "heading" | "button" | "image" | "link";
  hint?: string;             // screen reader hint
}

type TokenRef = string; // references a key in tokens.yaml
```

#### Delta Document

```typescript
interface DeltaDocument {
  added?: Record<ScreenSlug, ScreenEntry>;
  modified?: Record<ScreenSlug, ScreenEntry>;  // full screen replacement
  removed?: Record<ScreenSlug, { reason: string }>;
}
```

#### Validation Rules (schema-enforced)

The JSON Schema enforces structural validity. Cross-artifact checks (field coverage, event coverage, ViewModel mapping) run as separate validation passes against `design.md` and `spec.md` — they cannot be expressed in JSON Schema alone.

Schema-enforced rules:
- `version` must be `1`.
- A document must have exactly one of `screens` or `delta` (not both).
- Screen slugs must be kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).
- Item types must be lowercase (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).
- `when` values must be non-empty strings.
- `event` values must match `^[A-Z][a-zA-Z0-9]*(\([\w, ]*\))?$`.
- `kind` on overlays must be one of `dialog`, `sheet`, `snackbar`.
- `trigger` on overlays, when present, must match `^[A-Z][a-zA-Z0-9]*$` (Event name without arguments).
- `replaces` on state entries, when present, must be one of `body`, `screen`.
- `platforms` keys must be one of `ios`, `android`, `web`.

### Item Vocabulary

The vocabulary is deliberately small — a content-placement set of primitives plus a single container type (`group`), not a UI framework. Shell writers map items to platform-native components and map groups to platform-native containers (HStack/VStack, Row/Column, flex containers) using the group's layout properties.

> **Note:** The tables below show SwiftUI and Compose mappings for Phases 1–2. Web mappings (HTML/CSS/JS) are deferred to Phase 5. See [Phase 5: Web shell writer](#phase-5-web-shell-writer) for the planned web item mapping table.

#### Layout Container

| Node | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `group` (direction: row) | Horizontal container | `HStack(spacing:)` | `Row(horizontalArrangement:)` |
| `group` (direction: column) | Vertical container | `VStack(spacing:)` | `Column(verticalArrangement:)` |
| `group` (direction: stack) | Overlapping container | `ZStack` | `Box` |

Groups with surface decoration (`background`, `corner_radius`, `elevation`) map to card-like wrappers on each platform — e.g., a `Group` inside a styled container view on iOS, or a `Card`/`Surface` on Android.

#### Regions

Regions are implicit — shell writers map them to platform-native screen structure:

| Region | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `header` | Top navigation bar (title, leading/trailing items) | `NavigationTitle` + toolbar | `TopAppBar` |
| `body` | Main content area (list, grid, form, or items) | Body content view | Scaffold `content` |
| `footer` | Bottom bar (tabs, segments, actions) | `TabView` or toolbar | `BottomAppBar` / `NavigationBar` |
| `fab` | Floating action button | `.overlay` / `ZStack` | `FloatingActionButton` |

#### Content Patterns

Content patterns describe how the body region organizes its content:

| Pattern | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `list` | Scrollable list with iteration | `List` / `LazyVStack` | `LazyColumn` |
| `grid` | Grid layout with iteration | `LazyVGrid` | `LazyVerticalGrid` |
| `form` | Vertical input layout | `Form` / `VStack` | `Column` with form styling |

When body is a content node array (no content pattern), shell writers render it using the group layout properties if present, or as centered/stacked content by default — the standard pattern for loading, empty, and error states.

#### Display Items

| Item | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `text` | Text label (static or bound) | `Text` | `Text` |
| `icon` | Icon display | `Image(systemName:)` | `Icon` |
| `image` | Image display | `AsyncImage` / `Image` | `AsyncImage` / `Image` |
| `badge` | Count or status indicator | `.badge()` modifier | `Badge` |
| `progress` | Loading spinner or progress bar | `ProgressView` | `CircularProgressIndicator` |
| `divider` | Visual separator | `Divider()` | `HorizontalDivider()` |

#### Input Items

| Item | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `field` | Text input | `TextField` | `OutlinedTextField` |
| `checkbox` | Multi-select toggle | `Toggle` (checkbox style) | `Checkbox` |
| `switch` | On/off toggle | `Toggle` | `Switch` |
| `slider` | Range input | `Slider` | `Slider` |
| `segments` | Segment picker | `Picker(.segmented)` | `SingleChoiceSegmentedButtonRow` |
| `dropdown` | Selection from a list | `Menu` / `Picker(.menu)` | `ExposedDropdownMenuBox` |

#### Action Items

| Item | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `button` | Text button | `Button` | `Button` / `TextButton` |
| `action` | Icon button (tappable icon) | `Button` with `Image` | `IconButton` |

New item types can be added as needed. The vocabulary is intentionally open — if a screen requires an item type not in the table, introduce it with a descriptive lowercase name and document the platform mapping.

#### Custom Items (Phase 1 Escape Hatch)

Real applications will quickly need item types beyond the initial vocabulary (`avatar`, `date-picker`, `chip`, `tab-bar`, etc.). Before the component library (Phase 4) provides formal `components.yaml` definitions, the composition artifact supports a top-level `custom_items` declaration that registers additional item type names for validation:

```yaml
version: 1

custom_items:
  - name: avatar
    description: "Circular user profile image"
  - name: date-picker
    description: "Date selection input"

screens:
  user-profile:
    # ... can now use avatar without triggering a validation warning ...
```

The `custom_items` list serves only as a validation allowlist — it tells the "Item validity" check that these names are intentional, not typos. It does not carry platform mappings (that is the role of `components.yaml` in Phase 4). Shell writers encountering a custom item name not in the built-in vocabulary render it as a best-effort match based on the name and any properties present (e.g., an `avatar` with `bind: avatar_url` renders as a circular image).

In Phase 1, item validity is always a **warning**, never an error, regardless of whether the item appears in the vocabulary or `custom_items`. This ensures early adopters are not blocked by vocabulary gaps. In Phase 4, when `components.yaml` provides formal definitions with platform mappings, items not in the vocabulary, `custom_items`, or `components.yaml` become errors.

#### Accessibility Annotations

Items in the composition artifact support optional accessibility annotation properties:

- `label: "..."` — accessible label for screen readers (maps to `accessibilityLabel` on iOS, `contentDescription` on Android).
- `role: heading | button | image | link` — semantic role when the default item role is insufficient.
- `hint: "..."` — additional context for screen reader users (maps to `accessibilityHint` on iOS, `stateDescription` on Android).

Example:

```yaml
- action: { icon: trash, label: "Delete todo", event: DeleteTodo(id) }
- image: { bind: avatar_url, label: user_name, role: image }
```

Accessibility annotations are optional. When absent, shell writers apply platform defaults — interactive items get labels derived from their content, and semantic roles follow the item type. When present, they override the defaults. The annotations are valid in both skeleton and wired modes.

### Why YAML

Layout is fundamentally structural data — a tree of components with properties. Unlike proposal, spec, and design (which are prose-heavy documents describing intent, behavior, and rationale), the composition artifact is consumed primarily by machines (shell writers, the validation CLI) and produced primarily by machines (the define agent, Figma adapters, legacy analyzers). YAML is the right format for this content:

- **Machine-parseable.** Shell writers deserialize `composition.yaml` against a JSON Schema rather than pattern-matching on indented markdown lists. This eliminates an entire class of parsing fragility.
- **Schema-validatable.** A single JSON Schema enforces item validity, token resolution, structural rules, and binding completeness. Validation runs once, against one format, with one codepath.
- **Consistent with `tokens.yaml`.** The design system layer already persists structured data as YAML. Layout composition is the natural companion — both feed shell writers, both are structural, both benefit from schema validation.
- **Round-trippable.** External tools (Figma adapters, legacy analyzers) produce `composition.yaml` directly. Re-imports from updated designs produce a new YAML file that can be diffed against the existing `composition.yaml` — same-format diffing, not cross-format.
- **Diff-friendly.** YAML region structures produce clear, readable diffs in version control. The shallow container tree (typically 2–3 levels deep) keeps diffs concise and easy to review.

### Pipeline Integration

#### Schema Change

Add the `composition` brief to `schemas/vectis/schema.yaml`:

```yaml
pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: composition
      brief: briefs/composition.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
```

The `composition` brief declares:

```yaml
---
id: composition
description: Define the visual layout of each screen
generates: composition.yaml
needs: [specs, proposal]
---
```

It reads the spec to know which screens exist (ViewModel variants from spec requirements about views/pages) and what interactions they support (Event variants from spec requirements about features). It reads the proposal to know which platforms are targeted (determines whether to include platform-specific region overrides).

The brief's prose instructions direct the agent to check for an existing `composition.yaml` — first in the active change directory (`.specify/changes/<name>/`), then in the baseline (`.specify/specs/`). If one exists, the agent uses it as a skeleton and enriches it with `bind`, `event`, and `maps_to` keys derived from the specs, preserving the existing region structure. If no `composition.yaml` exists, the agent infers layout from the specs and proposal. This optional-input behavior is expressed entirely in the brief's prose instructions; it does not require new pipeline machinery.

```
existing composition.yaml skeleton (optional)
         │
         ▼
    ┌─────────────┐
    │ composition  │◄── specs (screens, behaviors)
    │    brief     │◄── proposal (platforms)
    └──────┬──────┘
           │
           ▼
    composition.yaml (wired regions with bind/event keys)
           │
           ▼
    shell writers (iOS, Android, web)
```

The composition brief resolves inputs in priority order:

1. **Existing composition.yaml skeleton present** — preserve region structure, enrich with `bind`, `event`, and `maps_to` keys from specs.
2. **No skeleton present** — infer layout from specs and proposal (agent-generated).

#### Why `composition` precedes `design`

In the current pipeline, the design brief infers per-page view struct fields from the spec. With the composition artifact, this inference becomes explicit: the layout declares which fields appear on screen and how, and the design brief reads the layout to confirm the view struct has the right shape. This ordering means:

1. **Spec** defines behavior and identifies screens.
2. **Composition** defines how each screen is composed and which data it needs.
3. **Design** defines the type system, now with composition as an additional input to validate view struct completeness.

If the composition artifact shows `bind: due_date` on the todo-list screen but the spec never mentions a due date, the design brief can surface this as a gap.

#### Type Name Proposal (Agent-Inference Path)

When the agent infers layout without a skeleton (input priority 2), the composition artifact is the **first** artifact in the pipeline to name screens, ViewModel variants, and field bindings. It reads behavioral spec text like "the user sees their todo items with a count of remaining items" and proposes:

- **Screen slugs:** `todo-list`, `add-todo` (derived from spec screen/page references)
- **ViewModel variant names:** `ViewModel::TodoList(TodoListView)` (PascalCase from screen name, via `maps_to`)
- **Field names:** `items`, `count`, `filter` (derived from spec data references, via `bind` keys)
- **Event names:** `ToggleTodo(id)`, `DeleteTodo(id)` (derived from spec interaction descriptions, via `event` keys)

These are **proposed names**, not references to existing types. The design brief, which runs after composition, reads `composition.yaml` and adopts the proposed names when formalizing the Rust type system — or adjusts them if naming conventions or domain model considerations require changes. When design adjusts a name, the build phase's cross-artifact validation (see [Validation](#validation)) catches any resulting mismatch between `composition.yaml` and `design.md`, prompting reconciliation before shell writers run.

For the skeleton path (input priority 1), this is not a concern — skeletons do not contain `maps_to`, `bind`, or `event` keys. The define pipeline adds these only after it has access to both the region structure and the spec's behavioral content. The names it proposes follow the same convention: derived from spec language, adopted or adjusted by design.

This approach is consistent with how the existing pipeline works — the spec brief proposes screen concepts and event descriptions in natural language, and the design brief formalizes them into typed Rust constructs. The composition artifact sits between these two, proposing names at a specificity level between prose and Rust types.

#### Brief Content (`schemas/vectis/briefs/composition.md`)

The full brief file:

````markdown
---
id: composition
description: Define the visual layout of each screen as a composition.yaml artifact
generates: composition.yaml
needs: [specs, proposal]
---

Generate a `composition.yaml` file describing the spatial composition of every screen in the application. The artifact uses the region-based format with `group` containers and item vocabulary defined in RFC-7, and follows the schema at `schemas/vectis/composition.schema.json`. Groups carry flexbox-like layout properties (`direction`, `gap`, `padding`, `align`, `justify`) and optional sizing and surface decoration.

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
````

### Shell Writer Consumption

#### Input Analysis Changes

Both `ios-writer` and `android-writer` currently have an Input Analysis step that reads `app.rs` types. The updated input analysis additionally reads `composition.yaml`. Because the artifact is schema-validated YAML, shell writers deserialize it directly rather than pattern-matching on text:

| Extract | Source | Maps to |
| --- | --- | --- |
| Screen regions | `composition.yaml` `header`, `body`, `footer`, `fab` | View structure (nav bar, content, bottom bar, FAB) |
| Container structure | `composition.yaml` `group` nodes with `direction`, `gap`, `align`, `justify` | HStack/VStack/ZStack (iOS), Row/Column/Box (Android), flex containers (web) |
| Sizing | `composition.yaml` `size` on groups and items (`fill`, `hug`, fixed) | `.frame(maxWidth: .infinity)` (iOS), `Modifier.fillMaxWidth()` (Android), `flex: 1` (web) |
| Surface decoration | `composition.yaml` `background`, `corner_radius`, `elevation` on groups | Styled container views (iOS), Card/Surface (Android), styled divs (web) |
| Field bindings | `composition.yaml` `bind` keys on items | Property bindings in views |
| Event wiring | `composition.yaml` `event` keys on items | `onEvent()` / interaction handlers |
| Token references | `composition.yaml` `style`, `color`, `gap`, `padding` | `VectisTypography.*` / `VectisColors.*` / `VectisSpacing.*` |
| Conditional rendering | `composition.yaml` `states` and `*-when` keys | `if`/`switch` in view code |
| Iteration | `composition.yaml` `list.each` / `grid.each` + `item` keys | `ForEach` / `LazyColumn items` |

#### Mapping Priority

When the composition artifact is present, shell writers use it as the primary layout guide — the group structure, layout properties, and sizing modes are followed deterministically rather than inferred. When absent (for backward compatibility with existing changes that predate RFC-7), shell writers fall back to the current inference behavior. The fallback ensures existing projects and in-flight changes are not disrupted.

### Delta Operations

The composition artifact supports the same delta operations as specs, expressed via a top-level `delta` key in the per-change artifact:

```yaml
# composition.yaml — per-change delta
version: 1

delta:
  added:
    todo-list:
      name: "Todo List"
      maps_to: "ViewModel::TodoList(TodoListView)"
      header:
        title: "My Todos"
      body:
        list:
          each: items
          item:
            # ... items ...

  modified:
    home:
      name: "Home"
      maps_to: "ViewModel::Home(HomeView)"
      header:
        title: "Home"
      body:
        # ... updated regions ...

  removed:
    onboarding:
      reason: "Onboarding flow replaced by in-app tooltips"
```

The baseline `composition.yaml` uses a flat `screens` map. Per-change artifacts use `delta` with `added`, `modified`, and `removed` keys. This integrates with the existing spec-merge infrastructure. When `/spec:merge` runs, the composition delta merges into `composition.yaml` in the baseline alongside the spec files.

#### Merge Strategy

Delta operations are **screen-level**, not region-level or item-level:

- **`added`**: Insert the screen entry into the baseline `screens` map. If the slug already exists in the baseline, the merge fails with a conflict (the screen should be under `modified` instead).
- **`modified`**: Replace the entire screen entry in the baseline with the version from the delta. This is a full-screen replacement, not a region-level merge. The rationale: merging independently edited region structures at the item level would require positional diff logic with ambiguous conflict resolution. Full-screen replacement is simple, predictable, and sufficient because the define pipeline always produces complete screen entries.
- **`removed`**: Delete the screen entry from the baseline. The `reason` field is preserved in the archive for audit purposes.

This is consistent with how spec deltas work — `MODIFIED Requirements` replaces the entire requirement block, not individual sentences within it.

#### Conflict Detection

When `/spec:merge` runs, the CLI checks for conflicts between the delta and the current baseline:

- **`added` conflict**: A screen slug in `added` already exists in the baseline → merge fails, prompt user to use `modified`.
- **`modified` conflict**: The baseline screen has been modified by another merged change since this change was created → merge fails with a diff showing both versions. The user resolves by re-running the define pipeline against the updated baseline.
- **`removed` conflict**: The screen slug in `removed` does not exist in the baseline (already removed by another change) → warning, not a failure.

##### Per-Screen Change Tracking

The spec-merge infrastructure tracks changes at the file level (one file per feature spec). The composition artifact is a single file containing all screens, so conflict detection operates at the **screen-entry level** within that file. The CLI tracks per-screen modifications using a content hash stored in the baseline metadata.

When `specify merge` writes the baseline `composition.yaml`, it also writes (or updates) a sibling `.composition-checksums.yaml` file in `.specify/specs/`:

```yaml
# .specify/specs/.composition-checksums.yaml
# Auto-generated by `specify merge` — do not edit manually.
screens:
  todo-list: "sha256:a1b2c3..."
  add-todo: "sha256:d4e5f6..."
  settings: "sha256:g7h8i9..."
```

Each value is the SHA-256 hash of the YAML-serialized screen entry (normalized: sorted keys, consistent whitespace). When a change's `modified` delta targets a screen, the CLI computes the hash of the current baseline screen entry and compares it to the stored checksum. A mismatch means another merged change has modified that screen since the checksum was recorded, triggering a conflict.

##### Merge Algorithm

The YAML merge codepath in `specify merge` executes these steps:

1. **Parse.** Deserialize the per-change `composition.yaml` (must have a `delta` key) and the baseline `composition.yaml` (must have a `screens` key). If either is missing or malformed, fail with a descriptive error.

2. **Validate delta structure.** Confirm the delta contains only `added`, `modified`, and/or `removed` keys with valid screen entries. Schema validation should have already caught structural issues, but belt-and-suspenders checking here prevents corrupt merges.

3. **Process `removed`.** For each slug in `removed`:
   - If the slug exists in baseline `screens`: delete it, record the removal in the archive metadata.
   - If the slug does not exist: emit a warning ("screen `{slug}` already absent from baseline") and continue.

4. **Process `added`.** For each slug in `added`:
   - If the slug does not exist in baseline `screens`: insert the screen entry.
   - If the slug already exists: fail with conflict ("screen `{slug}` already exists in baseline; use `modified` to update it").

5. **Process `modified`.** For each slug in `modified`:
   - If the slug does not exist in baseline `screens`: fail with conflict ("screen `{slug}` not found in baseline; use `added` for new screens").
   - If the slug exists: compare the baseline screen's current SHA-256 hash against `.composition-checksums.yaml`. If the hashes match (no intervening change), replace the screen entry. If they differ, fail with conflict and output a diff of the two versions.

6. **Write.** Serialize the updated baseline `composition.yaml`, recompute all screen checksums, and write `.composition-checksums.yaml`.

7. **Archive.** Copy the per-change `composition.yaml` (with its `delta` key) into the archive alongside the spec deltas.

This algorithm is the YAML counterpart of the markdown section-based merge that specs use. The operations are semantically identical; only the serialization format and conflict-detection granularity differ.

#### Format Note

Spec deltas use markdown headers (`## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`). Composition deltas use YAML keys (`added`, `modified`, `removed`). This difference is intentional — specs are prose documents where section headers are the natural delimiter, while composition is structured data where map keys are the natural delimiter. The `specify merge` CLI handles both formats: markdown section parsing for specs, YAML key-based merging for composition. The delta operations are semantically identical; only the serialization differs.

### Validation

The `specify validate` command gains checks for the composition artifact, all enforced against the JSON Schema and cross-artifact references:

| Check | Description |
| --- | --- |
| **Schema validity** | `composition.yaml` conforms to the JSON Schema (version, screen structure, region/item shape) |
| **Screen uniqueness** | No duplicate screen slugs |
| **Item validity** | Every item type name resolves to the vocabulary or `custom_items` |
| **Token resolution** | Every token reference (`style`, `color`, `size`) resolves to an entry in `tokens.yaml` (when the design system exists) |
| **Field coverage** | Every field in each per-page view struct (from design) appears as a `bind` value in the corresponding screen (wired mode only) |
| **Event coverage** | Every shell-facing Event variant relevant to a screen has an `event` wiring in that screen's regions (wired mode only) |
| **ViewModel mapping** | Every `maps_to` value references a declared ViewModel variant from the design (wired mode only) |
| **Overlay trigger consistency** | Every overlay `trigger` value matches an `event` name (without args) used somewhere in the same screen's regions (wired mode only) |
| **Navigation consistency** | Every `Navigate(X)` argument has a corresponding screen slug in composition and a corresponding Route variant in design (wired mode only) |

These checks run during the build phase before shell writers are invoked, catching mismatches between the composition artifact and the spec/design early.

#### Validation Severity Levels

Each validation check produces diagnostics at one of two severity levels:

| Severity | Meaning | Build phase behavior |
| --- | --- | --- |
| **error** | The artifact is structurally invalid or has a cross-artifact mismatch that will produce incorrect shell code. | Halts shell generation for the affected screen(s). The agent reports the errors and does not proceed until they are resolved. |
| **warning** | A non-blocking issue that may indicate incomplete authoring but will not produce incorrect code. | Logged and reported to the user. Shell generation proceeds. |

Severity assignments by check:

| Check | Severity |
| --- | --- |
| Schema validity | error |
| Screen uniqueness | error |
| Item validity (known vocabulary) | warning (Phase 1), error (Phase 4 with `components.yaml`) |
| Token resolution | warning (tokens may be added later) |
| Field coverage (unbound view struct field) | warning |
| Event coverage (unwired Event variant) | warning |
| ViewModel mapping (`maps_to` mismatch) | error |
| Overlay trigger consistency (trigger doesn't match any event) | error |
| Navigation consistency (`Navigate(X)` target missing) | error |

The distinction matters for incremental adoption: warnings allow the pipeline to proceed with partial composition artifacts (e.g., a skeleton with incomplete bindings), while errors catch structural problems that would break shell generation. The `specify validate` CLI outputs diagnostics in the format: `[error|warning] composition:<screen-slug>: <message>`.

### Impact on Existing Artifacts

The composition artifact is a new addition, but it changes the inputs and responsibilities of several existing briefs and skills. This section summarizes the required changes to each.

#### `schemas/vectis/briefs/specs.md`

The spec brief currently instructs: "Views (screens the user sees) become requirements describing what the user sees and when. View structure details belong in design." This guidance is sufficient for the composition brief to identify screens — spec requirements that describe views or pages map to composition screen entries. No structural change to the spec brief is required. However, the existing guidance could be strengthened with a note: "Name each distinct view explicitly in its requirement title (e.g., 'Requirement: Todo List View', 'Requirement: Add Todo Form'). The composition brief uses these titles to derive screen slugs."

#### `schemas/vectis/briefs/design.md`

The design brief currently declares `needs: [proposal]` (it also implicitly reads specs, though this is not declared in `needs`). With the composition artifact, it gains additional inputs:

- **`needs`** changes to `[proposal, specs]` — making the existing implicit dependency on specs explicit.
- **Composition awareness (prose-based).** The design brief gains prose instructions to check for a `composition.yaml` in the change directory or baseline. When present, the design brief uses it as an additional input to validate view struct completeness. When absent, the design brief infers ViewModel shape from specs alone (the current behavior). This preserves backward compatibility: projects that predate RFC-7 (or Omnia-schema projects that don't use Vectis composition) continue to work because the design brief proceeds without composition.
- **Domain Model § ViewModel:** The brief currently instructs the agent to derive ViewModel variants and per-page view struct fields from the spec. With composition as input, the brief additionally instructs: "When `composition.yaml` is present, read it and adopt the screen names, ViewModel variant names, and field names proposed by the composition artifact. Adjust naming only when Rust conventions or domain model considerations require it. Every `bind` value in `composition.yaml` must appear as a field in the corresponding per-page view struct. When `composition.yaml` is absent, infer the ViewModel shape from specs as before."
- **Gap surfacing:** The design brief gains an instruction to flag mismatches — a `bind` in composition with no spec backing, or a spec-described data element with no composition binding.

The design brief does **not** gain layout responsibilities. It continues to define the type system; composition provides an additional input that makes the ViewModel shape more explicit.

#### `schemas/vectis/briefs/build.md`

The build brief currently orchestrates core-writer → shell-writers. Changes:

- **Pre-shell validation:** Before invoking shell writers, the build brief instructs the agent to run composition validation checks (field coverage, event coverage, ViewModel mapping). If validation fails, the agent reports mismatches and halts shell generation for the affected screens.
- **Shell writer invocation:** The build brief's shell-writer handoff contract gains `composition.yaml` as a required input alongside `app.rs`, `design.md`, and `tokens.yaml`. The handoff instruction reads: "Pass the `composition.yaml` artifact to the shell writer. When present, the shell writer uses it as the primary layout guide (mapping regions and items to platform-native views). When absent, the shell writer falls back to inference from `app.rs` types."

#### `plugins/vectis/skills/ios-writer/SKILL.md`

The ios-writer's Input Analysis step currently extracts types from `app.rs` and reads optional `## iOS Shell Requirements` from the spec. Changes:

- **New input:** Add `composition.yaml` to the input list alongside `app.rs`, `tokens.yaml`, and spec shell sections.
- **Input Analysis table:** Add rows for screen regions, field bindings, event wiring, token references, conditional rendering, and iteration (the extraction table from [Shell Writer Consumption](#input-analysis-changes)).
- **Mapping priority:** When `composition.yaml` is present, the region structure and group container tree take precedence over the ios-writer's current convention-based inference for view body composition. Groups map to SwiftUI stacks (`HStack`/`VStack`/`ZStack`) with their layout properties; sizing maps to `.frame()` modifiers; surface decoration maps to styled container views. When absent, the existing inference behavior is unchanged.
- **Platform-specific overrides:** When `composition.yaml` contains `platforms.ios` region overrides for a screen, the ios-writer uses those in preference to the shared regions.

#### `plugins/vectis/skills/android-writer/SKILL.md`

Mirrors the ios-writer changes:

- **New input:** `composition.yaml` alongside `app.rs`, `tokens.yaml`, and spec shell sections.
- **Input Analysis table:** Same extraction rows as ios-writer.
- **Mapping priority:** Same precedence rule — composition artifact present means the group container tree provides deterministic layout instructions (`Row`/`Column`/`Box` with `Arrangement`/`Alignment`, `Modifier.fillMaxWidth()`, `Card`/`Surface` for decoration). When absent, inference-based.
- **Platform-specific overrides:** When `composition.yaml` contains `platforms.android` region overrides for a screen, the android-writer uses those in preference to the shared regions.

#### `plugins/vectis/skills/core-writer/SKILL.md`

The core-writer does **not** read `composition.yaml` directly. Layout is a shell concern; the core-writer's responsibility is the Crux shared crate (Model, Event, ViewModel, update, view). The relationship is mediated through `design.md`:

- Composition declares what fields each screen needs (via `bind` keys on items) → design formalizes them into per-page view structs → core-writer reads design and generates the Rust types.
- The core-writer's Artifact-to-Code Mapping table gains a note: "Per-page view struct fields align with `composition.yaml` field bindings via `design.md`. The core-writer reads `design.md`, not `composition.yaml`."

This preserves the Crux separation: core knows about data shape, not spatial arrangement.

#### `schemas/vectis/briefs/tasks.md`

The tasks brief currently declares `needs: [specs, design]`. With the composition artifact:

- **`needs`** stays `[specs, design]` — unchanged.
- **Composition awareness (prose-based).** The tasks brief gains prose instructions to check for a `composition.yaml` in the change directory. When present, the tasks brief expresses the dependency between shell tasks and `composition.yaml` in its task ordering. This is not a hard requirement — pre-RFC-7 changes have no composition artifact.
- The tasks brief's skill directive table gains no new skill — composition generation is part of the define pipeline, not a separate build skill. However, the task ordering guidance gains a note: "When `composition.yaml` is present, shell writer tasks (ios-writer, android-writer) depend on it. When composition validation fails, the corresponding shell task is blocked. When `composition.yaml` is absent, shell writers fall back to inference and no composition-related blocking applies."

#### `schemas/vectis/briefs/merge.md`

The merge brief currently instructs the agent to delegate delta-spec merging to the CLI (`specify spec preview`, `specify spec conflict-check`, `specify merge`, `specify validate`). With the composition artifact:

- The merge brief gains an instruction to include `composition.yaml` in the merge preview and conflict check. The CLI commands already handle this (see [CLI Impact](#cli-impact)), but the merge brief should explicitly mention: "The `specify merge` command merges both spec deltas (markdown) and composition deltas (YAML) in a single operation. Review the composition delta alongside spec changes in the `specify spec preview` output before confirming the merge."
- No change to the merge brief's `needs` — it already depends on `build`, which transitively ensures all artifacts are complete.

#### `plugins/spec/skills/define/SKILL.md`

The define skill orchestrates the define pipeline — it runs each brief in sequence (proposal → specs → design → tasks). With the composition artifact inserted between specs and design, the define skill needs updates:

- **Pipeline awareness.** The define skill reads `schema.yaml` to determine the brief sequence. With the composition stage added, the skill discovers it automatically via `specify schema pipeline`. No hardcoded brief list change is needed in the skill itself, but the skill must handle the new artifact type.
- **YAML output.** All existing briefs produce markdown artifacts. The composition brief produces a YAML file (`composition.yaml`). The define skill must write the agent's output as YAML rather than markdown for this stage. The skill's file-writing logic should dispatch on the `generates` extension: `.md` files are written as-is; `.yaml` files are written with YAML formatting validation (the agent's output must be valid YAML).
- **Skeleton passthrough.** The composition brief's prose instructions direct the agent to check for an existing `composition.yaml` in the change directory or baseline. The define skill does not need special handling for this — the agent reads the file system directly when the brief instructs it to look for an existing artifact.
- **Change directory placement.** The composition artifact is written to `.specify/changes/<name>/composition.yaml`, alongside `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. When the change uses delta mode, the composition artifact contains a `delta` key (not `screens`).

#### `plugins/spec/skills/extract/SKILL.md`

The extract skill currently produces specs and `design.md` from existing source code. It does **not** produce `composition.yaml`. With RFC-7 landed:

- **No immediate change required.** Extract continues to work as before — it produces specs and design, and the downstream pipeline (define or build) operates with or without a composition artifact. Shell writers fall back to inference when composition is absent, which is the pre-RFC-7 behavior.
- **Future integration (deferred).** When extract gains composition support (see [Legacy App Reverse-Engineering](#legacy-app-reverse-engineering)), it will produce a skeleton `composition.yaml` alongside the extracted specs. This skeleton flows through the normal enrichment pipeline during the next `/spec:define` run. Extract integration is not a prerequisite for any phase of this RFC.

### CLI Impact

The composition artifact introduces new responsibilities for the `specify` CLI (`augentic/specify-cli`). This section enumerates the required changes, organized by the phase in which they are needed.

#### Phase 1 Changes

| Command | Change |
| --- | --- |
| `specify change create` | Include `composition.yaml` in the change's artifact manifest. When the change directory is created, the lifecycle tracker knows that `composition.yaml` is an expected artifact (alongside `proposal.md`, `spec.md`, `design.md`, `tasks.md`). |
| `specify status` | Report `composition.yaml` completion status alongside other artifacts. Show whether the artifact is in skeleton mode (no `bind`/`event` keys) or wired mode. Report the number of screens. |
| `specify validate` | Add structural validation: parse `composition.yaml` against the JSON Schema (`schemas/vectis/composition.schema.json`). Report schema violations as errors. This is schema-only validation — cross-artifact checks come in Phase 2. |
| `specify merge` | Add a YAML delta merge codepath alongside the existing markdown spec merge. Read the per-change `composition.yaml` delta, apply `added`/`modified`/`removed` operations to the baseline `composition.yaml`, and detect conflicts at the screen-entry level. |
| `specify spec preview` | Include composition delta in the dry-run merge preview output. |
| `specify spec conflict-check` | Check for composition conflicts (added screen already exists, modified screen changed in baseline). |

#### Phase 2 Changes

| Command | Change |
| --- | --- |
| `specify validate` | Add cross-artifact validation checks: field coverage (every per-page view struct field has a `bind` on some item), event coverage (every shell-facing Event has an `event` wiring on some item), ViewModel mapping (`maps_to` references valid ViewModel variants from `design.md`), token resolution (token references resolve to `tokens.yaml` entries), overlay trigger consistency (every `trigger` matches an `event` in the same screen), and navigation graph consistency (`Navigate(X)` targets exist as screen entries and Route variants). |

#### Phase 3+ Changes

No additional CLI changes required. The Figma adapter (Phase 3) is a standalone tool that produces `composition.yaml` files consumed by the existing pipeline. The component library (Phase 4) extends the validation checks to resolve item names against `components.yaml` in addition to the built-in vocabulary.

## Incremental Adoption Path

### Phase 1: Composition artifact with skeleton support (low risk)

Add the `composition` brief to the vectis schema and update the define agent to produce `composition.yaml`. Support both skeleton mode (authored manually or imported from external tools) and wired mode (enriched by the pipeline). Shell writers read `composition.yaml` when present but fall back to inference when absent. No existing functionality changes. This is a pure addition.

Deliverables:
- `schemas/vectis/briefs/composition.md` brief file
- `schemas/vectis/composition.schema.json` JSON Schema (draft in [Appendix A](#appendix-a-composition-json-schema))
- Updated `schemas/vectis/schema.yaml` pipeline (add `composition` stage between `specs` and `design`)
- Updated `schemas/vectis/briefs/specs.md` brief (strengthen view-naming guidance for screen discovery)
- Updated `schemas/vectis/briefs/design.md` brief (`needs: [proposal, specs]`, prose-based composition awareness, ViewModel adoption instructions)
- Updated `schemas/vectis/briefs/tasks.md` brief (prose-based composition awareness, shell task dependency on `composition.yaml`)
- Updated `schemas/vectis/briefs/merge.md` brief (composition delta in merge preview guidance)
- Updated `plugins/spec/skills/define/SKILL.md` (YAML output handling, skeleton passthrough, change directory placement)
- Updated `plugins/vectis/skills/ios-writer/SKILL.md` (new input, Input Analysis table, mapping priority, platform overrides)
- Updated `plugins/vectis/skills/android-writer/SKILL.md` (same changes as ios-writer)
- Updated `plugins/vectis/skills/core-writer/SKILL.md` (Artifact-to-Code Mapping note on composition alignment via design)
- CLI changes: `specify change create` (composition in manifest), `specify status` (composition reporting), `specify validate` (schema validation), `specify merge` (YAML delta merge with per-screen checksums), `specify spec preview` and `specify spec conflict-check` (composition awareness)

### Phase 2: Validation

Add the composition-specific checks to `specify validate`. These catch drift between composition, specs, and design before the build phase runs.

Deliverables:
- Validation checks in the CLI (JSON Schema + cross-artifact checks against `composition.yaml`)
- Navigation graph derivation from `event: Navigate(...)` references, checked against Route enum
- Updated `schemas/vectis/briefs/build.md` brief (pre-shell validation gate, `composition.yaml` in handoff contract)

### Phase 3: Figma adapter

Introduce tooling that reads a Figma file and produces a `composition.yaml` skeleton. The adapter maps Figma's frame hierarchy to regions and items, applies best-match heuristics for unrecognized patterns, and marks uncertain mappings for human review. The composition brief then enriches the skeleton with bindings from specs.

Deliverables:
- Figma-to-composition adapter (standalone tool or `/spec:*` skill)
- Documentation for the Figma import workflow

### Phase 4: Component library

Introduce named item patterns — reusable compositions built from the primitive vocabulary. These live in the design system alongside `tokens.yaml`:

```yaml
# design-system/components.yaml
version: 1

components:
  search-bar:
    description: "Search input with clear and filter action"
    slots:
      query: { type: field, description: "Bound search text" }
    items:
      - icon: { name: search, color: onSurfaceVariant }
      - field: { slot: query }
      - action: { icon: clear, visible-when: "query is not empty" }

  list-item:
    description: "Standard list item with leading, content, and trailing"
    slots:
      leading: { type: item, description: "Left-side content (checkbox, icon, avatar)" }
      trailing: { type: item, description: "Right-side content (icon button, badge)" }
      title: { type: field, description: "Primary text" }
      subtitle: { type: field, description: "Secondary text" }
    items:
      - slot: leading
      - text: { slot: title, style: body }
      - text: { slot: subtitle, style: caption, color: onSurfaceVariant }
      - slot: trailing
```

Screen compositions reference these by name:

```yaml
body:
  list:
    each: items
    item:
      - list-item:
          leading:
            - checkbox: { bind: completed, event: ToggleTodo(id) }
          trailing:
            - action: { icon: trash, event: DeleteTodo(id) }
          title: title
          subtitle: due_date
```

This reduces repetition across screens and establishes a shared vocabulary between designers and the define agent. The component library is optional — compositions can always use primitive items directly.

### Phase 5: Web shell writer

With the item vocabulary and design system in place, a web shell writer can map the same `composition.yaml` to HTML/CSS/JS (or a framework like React, Leptos, or Yew). Regions and items map naturally:

| Region / Item | Web mapping |
| --- | --- |
| `header` | `<header>` / `<nav>` with title and action buttons |
| `body` (content array) | `<main>` with centered content |
| `body.list` | `<ul>` / virtual list |
| `body.grid` | CSS Grid |
| `body.form` | `<form>` with vertical input layout |
| `footer` | `<footer>` / bottom nav |
| `fab` | Fixed-position `<button>` |
| `group` (direction: row) | `<div style="display:flex; flex-direction:row">` |
| `group` (direction: column) | `<div style="display:flex; flex-direction:column">` |
| `group` (direction: stack) | `<div style="position:relative">` with absolute children |
| `group` with decoration | Styled `<div>` with background, border-radius, box-shadow |
| `text` | `<span>` / `<p>` / `<h*>` (based on `style`) |
| `field` | `<input>` / `<textarea>` |
| `button` | `<button>` |
| `action` | `<button>` with icon |
| `progress` | `<progress>` / CSS spinner |
| `dialog` overlay | `<dialog>` element / modal |
| `sheet` overlay | Side panel or modal overlay |
| `snackbar` overlay | Toast notification |

The web shell writer reads `composition.yaml`, `design.md`, and `tokens.yaml` — the same inputs as the iOS and Android writers. The design system gains a `design-system/web/` output directory for CSS custom properties generated from `tokens.yaml`.

## Alternatives Considered

**Image wireframes / mockups.** Rejected because they are not diffable, not mergeable by the existing spec-merge infrastructure, not verifiable by CLI tooling, and require external design tools. They also cannot be generated by the define agent in the text-based Specify workflow. A textual layout format preserves all the properties that make specs work.

**Extend specs with layout hints.** Rejected because it blurs the behavioral/visual boundary, makes specs brittle to visual changes, and couples the core-driving artifact to shell-specific concerns. See Motivation § "Why Specs Should Not Change."

**Extend design.md with layout sections.** Partially viable — the design already has `## iOS Shell Details` and `## Android Shell Details` sections. However, layout is a concern that cuts across all platforms and deserves its own artifact with dedicated validation. Embedding it in the design would make the design document responsible for both the type system (consumed by core-writer) and the visual arrangement (consumed by shell writers), violating the single-responsibility principle that keeps artifacts clean.

**Markdown `views.md` as the persisted artifact.** An earlier design used a markdown document (`views.md`) with indented bullet lists representing the component tree, `{field}` syntax for bindings, and `→ Event(args)` syntax for wiring. Rejected because layout is fundamentally structural data, not prose. Markdown required shell writers to reconstruct the component tree by pattern-matching on indented lists — fragile and impossible to schema-validate. It also created a format mismatch: machine producers (Figma adapters, the define agent) would generate structured data, render it to markdown, and then machine consumers (shell writers, validation CLI) would parse it back into structured data. The round-trip through a lossy text format added complexity without benefit. Re-imports from updated Figma designs would require cross-format diffing (YAML against markdown), which is harder than same-format diffing. YAML as the persisted format eliminates these issues and aligns with `tokens.yaml` as a structured design-layer artifact.

**Persisted `composition.yaml` alongside `views.md`.** An earlier variant maintained both a YAML composition model and a generated markdown view. Rejected because it duplicates the spatial tree, creating a permanent sync obligation — every layout edit would require updating the YAML and regenerating the markdown.

**Full design tool integration (Figma, Sketch).** A tight bidirectional sync with design tools was rejected due to authentication, API versioning, and workflow complexity. Instead, Figma is supported as a one-way *import source* (Phase 3). The adapter produces a `composition.yaml` skeleton that the composition brief enriches with bindings from specs. Re-imports produce a fresh skeleton that can be diffed against the existing `composition.yaml` to surface design changes.

**Component-tree YAML (deeply nested).** An earlier design used a deeply nested YAML tree where each screen was described as a hierarchy of PascalCase components (`Scaffold > TopBar > Text`, `ScrollableList > Card > Row > Checkbox + Column > Text`). Each component was a single-key YAML mapping with a `children` key for nesting and named slots for containers. Rejected because the nesting depth made the format hard to author, hard to review in diffs, and verbose — a simple two-screen todo app required ~170 lines of YAML. The format used platform-specific component names (Scaffold, TopBar) rather than platform-neutral abstractions. The current design uses `group` containers with a small flexbox property set instead — this provides the structural fidelity of a container tree without over-specifying platform-specific components, and keeps nesting shallow (2–3 levels typical).

**Flat item lists without container structure (earlier version of this RFC).** An earlier revision of this RFC used flat item lists within regions, with no `group` containers, sizing, or surface decoration. Shell writers were expected to infer container structure (rows vs columns, cards, spacing) from the item sequence and context. This was rejected after analysis showed that container structure — which elements group together and how — is the single most important factor for layout fidelity. Research (DOne 2025, Figma2Code 2025) and industry tools (Yoga, FigmaToCode, Plasmic) all converge on a flexbox-like container tree as the minimum viable layout model. The flat-list approach produced functionally correct but visually arbitrary output for any screen more complex than a simple list. The current design adds `group` containers with ~10 flexbox-like properties — the smallest property set that maps to Figma Auto Layout, CSS Flexbox, SwiftUI stacks, and Compose Row/Column/Box.

**Absolute positioning / pixel coordinates.** Rejected because models that map absolute coordinates produce rigid, non-responsive code. The Figma2Code benchmark (2025) shows that absolute-positioning approaches have dramatically worse responsiveness scores (APR rising to 31–58%) compared to flexbox-based approaches. The composition artifact uses relative layout properties (direction, gap, alignment, sizing modes) specifically because they produce responsive code across screen sizes.

**Constraint-based layout (Auto Layout / ConstraintLayout).** Constraint-based systems are powerful but significantly more verbose, harder to author manually, and don't have a clean cross-platform mapping. SwiftUI has moved away from constraint-based layout toward stacks; Compose uses Row/Column/Box rather than ConstraintLayout for most UIs. The flexbox subset covers the vast majority of layout patterns with a smaller, more portable property set.

**Full CSS as the layout model.** Rejected because CSS is platform-specific (web-only) and includes properties that don't map to native mobile (float, position, display: table, etc.). The flexbox subset is the intersection of CSS Flexbox, SwiftUI stacks, and Compose containers — the largest common denominator that maps cleanly to all targets.

## Decisions

### Platform-Divergent Layouts

When the same screen should look materially different on iOS vs Android, `composition.yaml` uses **per-platform region overrides within the screen entry** — the same shared-first principle used by specs (`## iOS Shell Requirements` / `## Android Shell Requirements`) and design (`## iOS Shell Details` / `## Android Shell Details`).

Each screen entry has shared regions that describe the default, cross-platform composition. When a platform requires a different arrangement, the screen gains a `platforms` map with per-platform region overrides:

```yaml
screens:
  settings:
    name: "Settings"
    maps_to: "ViewModel::Settings(SettingsView)"

    header:
      title: "Settings"

    body:
      list:
        each: sections
        item:
          - text: { bind: heading, style: label, color: onSurfaceVariant }
          - list:
              each: items
              item:
                - group:
                    direction: row
                    align: center
                    justify: space-between
                    items:
                      - text: { bind: title, style: body }
                      - switch: { bind: enabled, event: ToggleSetting(id) }

    platforms:
      ios:
        body:
          list:
            style: grouped
            each: sections
            item:
              - text: { bind: heading, style: label, color: onSurfaceVariant }
              - list:
                  each: items
                  item:
                    - group:
                        direction: row
                        align: center
                        justify: space-between
                        items:
                          - text: { bind: title, style: body }
                          - switch: { bind: enabled, event: ToggleSetting(id) }
```

Shell writers use the platform-specific regions when present, falling back to the shared regions when absent. Only overridden regions need to be specified in the platform block — shared regions like `header` carry through. Separate `composition-ios.yaml` / `composition-android.yaml` files are not used — a single file keeps the shared-first principle and avoids duplication of screens that look the same on both platforms.

### Accessibility Semantics

The composition artifact includes optional accessibility annotations on items (see [Accessibility Annotations](#accessibility-annotations) in the Item Vocabulary). The annotations cover:

- `label` — accessible label for screen readers
- `role` — semantic role override
- `hint` — additional context for assistive technology

This strikes a balance: shell writers continue to apply platform-specific defaults (interactive items get inferred labels, semantic roles follow item types), but the composition artifact can express intent where defaults are insufficient — action items that need explicit labels, images that need alt text, decorative elements that should be hidden from screen readers.

The ios-writer already checks for `accessibilityLabel` on interactive icons; the android-writer follows M3 semantics for `contentDescription`. Making these annotations explicit in the composition means both writers consume the same intent rather than inferring independently.

### Animation and Transitions

The composition artifact describes **static spatial arrangement** only. Page transitions, list item animations, gesture-driven interactions, and other motion concerns are explicitly out of scope for all phases of this RFC.

Rationale: animation is a high-variance concern — platform conventions differ significantly (iOS emphasizes spring-based transitions, Android uses Material Motion, web uses CSS transitions), and the right animation choices depend on performance characteristics that the composition artifact cannot express. Adding animation to the composition vocabulary would either force a lowest-common-denominator representation (losing platform fidelity) or require per-platform animation blocks (adding complexity with limited value at this stage).

Shell writers continue to apply platform-default animations (navigation transitions, list insertions/removals, state changes). If animation control becomes a requirement, it should be addressed in a future RFC that can evaluate whether animation belongs in the composition artifact, in a separate artifact, or in platform-specific shell configuration.

### Navigation Graph

The composition artifact does **not** include a separate navigation graph section. Navigation is expressed through two existing mechanisms:

1. **`event: Navigate(ScreenName)`** on interactive components describes forward navigation.
2. **`event: NavigateBack`** describes backward navigation.
3. The **Route enum** in `design.md` formalizes the navigation graph at the type level.

The validation pass (Phase 2) derives the navigation graph from `event: Navigate(...)` references across all screens and checks it against the Route enum in design. If an `event: Navigate(AddTodo)` reference has no corresponding `add-todo` screen entry in composition, or no `Route::AddTodo` variant in design, validation flags the mismatch.

A separate authored navigation section would duplicate information already expressed through event wiring and the Route enum. If navigation complexity grows to the point where the implicit graph is hard to follow (e.g., deep linking, conditional navigation, tab-based routing), a navigation visualization can be added as a validation output — a generated diagram, not an authored artifact.

## Open Questions

1. **Composition re-import merging.** When a Figma design is updated and a new skeleton `composition.yaml` is produced, the composition brief needs to merge it with the existing wired `composition.yaml` to surface what changed without losing `bind`/`event` enrichment. The delta merge strategy (see [Delta Operations](#delta-operations)) handles change-level merges, but re-import is a different operation: the new skeleton may have added, removed, or restructured items within a screen, not just added or removed screens. The re-import merge strategy needs definition — likely a screen-by-screen diff that presents changes for human review, preserving `bind`/`event` annotations on items that match between old and new versions. This is a Phase 3 concern (Figma adapter) and does not block Phases 1–2.

2. **Item vocabulary growth.** Item types like `avatar`, `chip`, `date-picker`, `tab-bar`, or `data-table` are common in real applications but not in the initial vocabulary. The Phase 1 approach uses `custom_items` as a validation allowlist (see [Custom Items](#custom-items-phase-1-escape-hatch)) and treats item validity as a warning rather than an error. The outstanding question is the graduation path: when should commonly-used custom items be promoted into the core vocabulary? A threshold like "used in 3+ shipped compositions" could work, but the governance process for vocabulary expansion is not yet defined. The component library (Phase 4) partially addresses this by providing formal `components.yaml` definitions, but the core vocabulary itself may also need periodic expansion.

## References

- `schemas/vectis/schema.yaml` — current pipeline definition
- `schemas/vectis/briefs/specs.md` — spec brief format (model for composition brief)
- `schemas/vectis/briefs/design.md` — design brief format
- `plugins/spec/skills/define/SKILL.md` — define skill orchestration (pipeline sequencing, artifact writing)
- `plugins/spec/skills/extract/SKILL.md` — extract skill (spec extraction from source code)
- `plugins/vectis/skills/ios-writer/SKILL.md` — iOS shell generation (Input Analysis, Spec-to-Code Mapping)
- `plugins/vectis/skills/android-writer/SKILL.md` — Android shell generation
- `plugins/vectis/skills/design-system-writer/SKILL.md` — design system generation from `tokens.yaml`
- `docs/vectis.md` — user-facing Vectis documentation
- [RFC-6: Vectis Bootstrap CLI](rfc-6-vectis-bootstrap.md) — CLI scaffolding (composition artifact consumed after scaffold)

## Appendix A: Composition JSON Schema

Draft JSON Schema for `schemas/vectis/composition.schema.json`. This is a Phase 1 deliverable — the draft below captures the structural rules from the Format Rules section and should be validated against the worked examples before shipping.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify/schemas/vectis/composition.schema.json",
  "title": "Specify Composition Artifact",
  "description": "Schema for composition.yaml — region-based screen layout descriptions.",
  "type": "object",
  "required": ["version"],
  "additionalProperties": false,
  "properties": {
    "version": { "const": 1 },
    "provenance": { "$ref": "#/$defs/provenance" },
    "custom_items": {
      "type": "array",
      "items": { "$ref": "#/$defs/customItem" }
    },
    "screens": {
      "type": "object",
      "propertyNames": { "$ref": "#/$defs/screenSlug" },
      "additionalProperties": { "$ref": "#/$defs/screenEntry" }
    },
    "delta": { "$ref": "#/$defs/deltaDocument" }
  },
  "oneOf": [
    { "required": ["screens"], "not": { "required": ["delta"] } },
    { "required": ["delta"], "not": { "required": ["screens"] } }
  ],

  "$defs": {
    "screenSlug": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9]*(-[a-z0-9]+)*$",
      "description": "Kebab-case screen identifier."
    },
    "itemTypeName": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9]*(-[a-z0-9]+)*$",
      "description": "Lowercase item type name from vocabulary or custom_items."
    },
    "eventValue": {
      "type": "string",
      "pattern": "^[A-Z][a-zA-Z0-9]*(\\([\\w, ]*\\))?$",
      "description": "Event name with optional arguments."
    },
    "triggerValue": {
      "type": "string",
      "pattern": "^[A-Z][a-zA-Z0-9]*$",
      "description": "Event name without arguments (overlay trigger)."
    },
    "bindValue": {
      "type": "string",
      "pattern": "^[a-z_][a-z0-9_]*(\\.[a-z_][a-z0-9_]*)?$",
      "description": "Field name, optionally dot-qualified for outer iteration context."
    },
    "tokenRef": {
      "type": "string",
      "description": "Reference to a design token name from tokens.yaml."
    },

    "provenance": {
      "type": "object",
      "required": ["sources"],
      "additionalProperties": false,
      "properties": {
        "sources": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/provenanceSource" }
        }
      }
    },
    "provenanceSource": {
      "type": "object",
      "required": ["kind"],
      "additionalProperties": false,
      "properties": {
        "kind": { "type": "string", "enum": ["figma", "legacy", "manual"] },
        "uri": { "type": "string", "format": "uri" },
        "captured_at": { "type": "string", "format": "date-time" }
      }
    },

    "customItem": {
      "type": "object",
      "required": ["name"],
      "additionalProperties": false,
      "properties": {
        "name": { "$ref": "#/$defs/itemTypeName" },
        "description": { "type": "string" }
      }
    },

    "headerRegion": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "title": { "type": "string" },
        "leading": { "$ref": "#/$defs/contentNodeArray" },
        "trailing": { "$ref": "#/$defs/contentNodeArray" }
      }
    },

    "listPattern": {
      "type": "object",
      "required": ["each", "item"],
      "additionalProperties": false,
      "properties": {
        "each": { "type": "string" },
        "item": { "$ref": "#/$defs/contentNodeArray" },
        "style": { "type": "string" }
      }
    },

    "gridPattern": {
      "type": "object",
      "required": ["each", "columns", "item"],
      "additionalProperties": false,
      "properties": {
        "each": { "type": "string" },
        "columns": { "type": "integer", "minimum": 1 },
        "item": { "$ref": "#/$defs/contentNodeArray" }
      }
    },

    "bodyRegion": {
      "oneOf": [
        {
          "type": "object",
          "required": ["list"],
          "additionalProperties": false,
          "properties": { "list": { "$ref": "#/$defs/listPattern" } }
        },
        {
          "type": "object",
          "required": ["grid"],
          "additionalProperties": false,
          "properties": { "grid": { "$ref": "#/$defs/gridPattern" } }
        },
        {
          "type": "object",
          "required": ["form"],
          "additionalProperties": false,
          "properties": { "form": { "$ref": "#/$defs/contentNodeArray" } }
        },
        { "$ref": "#/$defs/contentNodeArray" }
      ]
    },

    "screenEntry": {
      "type": "object",
      "required": ["name"],
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string" },
        "description": { "type": "string" },
        "maps_to": { "type": "string" },
        "header": { "$ref": "#/$defs/headerRegion" },
        "body": { "$ref": "#/$defs/bodyRegion" },
        "footer": { "$ref": "#/$defs/contentNodeArray" },
        "fab": { "$ref": "#/$defs/itemProps" },
        "states": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/stateEntry" }
        },
        "overlays": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/overlayEntry" }
        },
        "platforms": {
          "type": "object",
          "propertyNames": { "enum": ["ios", "android", "web"] },
          "additionalProperties": { "$ref": "#/$defs/platformOverride" }
        }
      }
    },

    "platformOverride": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "header": { "$ref": "#/$defs/headerRegion" },
        "body": { "$ref": "#/$defs/bodyRegion" },
        "footer": { "$ref": "#/$defs/contentNodeArray" },
        "fab": { "$ref": "#/$defs/itemProps" }
      }
    },

    "stateEntry": {
      "type": "object",
      "required": ["when"],
      "additionalProperties": false,
      "properties": {
        "when": { "type": "string", "minLength": 1 },
        "replaces": { "type": "string", "enum": ["body", "screen"] },
        "header": { "$ref": "#/$defs/headerRegion" },
        "body": { "$ref": "#/$defs/bodyRegion" },
        "footer": { "$ref": "#/$defs/contentNodeArray" },
        "fab": { "$ref": "#/$defs/itemProps" }
      }
    },

    "overlayEntry": {
      "type": "object",
      "required": ["kind", "content"],
      "additionalProperties": false,
      "properties": {
        "kind": { "type": "string", "enum": ["dialog", "sheet", "snackbar"] },
        "trigger": { "$ref": "#/$defs/triggerValue" },
        "title": { "type": "string" },
        "content": { "$ref": "#/$defs/contentNodeArray" }
      }
    },

    "contentNodeArray": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/contentNode" },
      "description": "Array of items and/or groups."
    },

    "contentNode": {
      "oneOf": [
        { "$ref": "#/$defs/item" },
        { "$ref": "#/$defs/groupItem" }
      ]
    },

    "groupItem": {
      "type": "object",
      "required": ["group"],
      "additionalProperties": false,
      "properties": {
        "group": { "$ref": "#/$defs/groupProps" }
      }
    },

    "groupProps": {
      "type": "object",
      "required": ["items"],
      "properties": {
        "direction": { "type": "string", "enum": ["row", "column", "stack"] },
        "gap": { "oneOf": [{ "type": "string" }, { "type": "number" }] },
        "padding": {
          "oneOf": [
            { "type": "string" },
            { "type": "number" },
            { "$ref": "#/$defs/paddingSpec" }
          ]
        },
        "align": { "type": "string", "enum": ["start", "center", "end", "stretch", "baseline"] },
        "justify": { "type": "string", "enum": ["start", "center", "end", "space-between", "space-around"] },
        "wrap": { "type": "boolean" },
        "size": { "$ref": "#/$defs/sizingSpec" },
        "background": { "$ref": "#/$defs/tokenRef" },
        "corner_radius": { "oneOf": [{ "type": "string" }, { "type": "number" }] },
        "elevation": { "$ref": "#/$defs/tokenRef" },
        "border": {
          "type": "object",
          "required": ["color", "width"],
          "additionalProperties": false,
          "properties": {
            "color": { "$ref": "#/$defs/tokenRef" },
            "width": { "type": "number" }
          }
        },
        "items": { "$ref": "#/$defs/contentNodeArray" },
        "label": { "type": "string" },
        "role": { "type": "string", "enum": ["heading", "button", "image", "link"] },
        "hint": { "type": "string" }
      },
      "additionalProperties": true
    },

    "paddingSpec": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "top": { "oneOf": [{ "type": "string" }, { "type": "number" }] },
        "right": { "oneOf": [{ "type": "string" }, { "type": "number" }] },
        "bottom": { "oneOf": [{ "type": "string" }, { "type": "number" }] },
        "left": { "oneOf": [{ "type": "string" }, { "type": "number" }] }
      }
    },

    "sizingSpec": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "width": { "$ref": "#/$defs/sizingValue" },
        "height": { "$ref": "#/$defs/sizingValue" }
      }
    },

    "sizingValue": {
      "oneOf": [
        { "type": "number", "minimum": 0 },
        { "type": "string", "enum": ["fill", "hug"] }
      ]
    },

    "item": {
      "type": "object",
      "minProperties": 1,
      "maxProperties": 1,
      "not": { "required": ["group"] },
      "additionalProperties": {
        "oneOf": [
          { "type": "null" },
          { "$ref": "#/$defs/itemProps" }
        ]
      }
    },

    "itemProps": {
      "type": "object",
      "properties": {
        "bind": { "$ref": "#/$defs/bindValue" },
        "event": { "$ref": "#/$defs/eventValue" },
        "error": { "type": "string" },

        "content": { "type": "string" },
        "name": { "type": "string" },
        "icon": { "type": "string" },
        "placeholder": { "type": "string" },
        "options": { "type": "array", "items": { "type": "string" } },

        "style": { "type": "string" },
        "color": { "$ref": "#/$defs/tokenRef" },
        "size": { "$ref": "#/$defs/sizingSpec" },
        "corner_radius": { "oneOf": [{ "type": "string" }, { "type": "number" }] },

        "each": { "type": "string" },
        "columns": { "type": "integer", "minimum": 1 },
        "item": { "$ref": "#/$defs/contentNodeArray" },

        "label": { "type": "string" },
        "role": { "type": "string", "enum": ["heading", "button", "image", "link"] },
        "hint": { "type": "string" }
      },
      "additionalProperties": true
    },

    "deltaDocument": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "added": {
          "type": "object",
          "propertyNames": { "$ref": "#/$defs/screenSlug" },
          "additionalProperties": { "$ref": "#/$defs/screenEntry" }
        },
        "modified": {
          "type": "object",
          "propertyNames": { "$ref": "#/$defs/screenSlug" },
          "additionalProperties": { "$ref": "#/$defs/screenEntry" }
        },
        "removed": {
          "type": "object",
          "propertyNames": { "$ref": "#/$defs/screenSlug" },
          "additionalProperties": {
            "type": "object",
            "required": ["reason"],
            "additionalProperties": false,
            "properties": {
              "reason": { "type": "string" }
            }
          }
        }
      }
    }
  }
}
```

**Notes on the draft schema:**

- `itemProps` and `groupProps` use `additionalProperties: true` to allow `*-when` conditional properties whose names are not enumerable in advance. Cross-artifact validation (Phase 2) checks that `*-when` values reference valid boolean fields.
- The `oneOf` constraint on the top level enforces that a document has either `screens` (baseline) or `delta` (per-change), never both.
- The `contentNode` union uses `oneOf` to distinguish items (any single key except `group`) from groups (`group` key required). The `item` def uses `not: { required: ["group"] }` to prevent ambiguity.
- The `bodyRegion` uses `oneOf` to accept four shapes: `{ list: ... }`, `{ grid: ... }`, `{ form: [...] }`, or a content node array. The first three are wrapped in single-key objects to disambiguate the polymorphic body.
- The `sizingValue` union accepts a number (fixed size), `"fill"`, or `"hug"`. When `size` is absent on an item or group, the default is hug (intrinsic sizing).
- `groupProps` captures the flexbox-like layout properties (`direction`, `gap`, `padding`, `align`, `justify`, `wrap`) and optional surface decoration (`background`, `corner_radius`, `elevation`, `border`).
- The `bindValue` pattern allows both bare names (`title`) and dot-qualified names (`section.heading`) for nested iteration contexts.
- `custom_items` is validated structurally but does not participate in cross-artifact item validity checks — that is a Phase 2 concern.
- The `fab` property on screen entries accepts `itemProps` directly (not wrapped in an item type key) since it is always a single floating action item.
