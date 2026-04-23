# RFC-7: View Layout Artifact for UI Generation

> Status: Draft · Depends: — · Enables: web shell writer, improved iOS/Android shell fidelity

## Abstract

Introduce a structured **view layout artifact** (`views.md`) into the Specify define pipeline that describes the spatial composition of each screen. This bridges the gap between behavioral specs (which define *what* the app does) and shell writers (which must decide *how* to arrange it on screen), without polluting the spec format with visual concerns.

## Motivation

### The Inference Gap

Today the pipeline from spec to screen works like this:

1. **Spec** defines behaviors (`WHEN user taps add THEN item appears in list`)
2. **Design** defines the type system (`ViewModel::TodoList(TodoListView { items: Vec<ItemView>, count: String })`)
3. **Shell writer** infers layout from the ViewModel struct fields + design tokens

Step 3 is where fidelity breaks down. The shell writer sees `TodoListView { items: Vec<ItemView>, count: String }` and produces a reasonable default — a scrollable list with a count label somewhere — but has no guidance on whether the count should be in a header bar, a floating badge, or inline at the bottom. The design system gives colors, fonts, and spacing; it does not give composition.

The result is that generated UIs are *functionally correct* (every field is rendered, every event is wired) but *visually arbitrary* (layout choices are made by the LLM based on convention, not intent). For a todo app this is acceptable. For anything with a deliberate design — onboarding flows, dashboards, e-commerce product pages — it produces output that must be substantially reworked by hand.

### Why Specs Should Not Change

The BDD spec format (`GIVEN... WHEN... THEN...`) defines observable behavior. This is the right abstraction for driving the Crux shared core — every `Event` variant, every `update()` match arm, every state transition traces back to a behavioral requirement. Adding layout concerns to specs would:

- **Blur the what/how boundary.** The spec says "the user sees their todo items"; it should not say "the todo items appear in a scrollable list with swipe-to-delete." The first is a requirement; the second is a layout decision.
- **Make specs brittle.** Every visual tweak (move the count badge, change from a list to a grid) would become a spec change, triggering the full define → build → merge cycle for what is fundamentally a presentation adjustment.
- **Couple the core to the shell.** The Crux architecture deliberately separates business logic (core) from presentation (shell). Specs drive the core. Layout drives the shell. Mixing them in one artifact erodes this separation.

### What Is Missing

A layer that communicates **spatial composition** — the arrangement of components on each screen, the mapping from ViewModel fields to visual elements, and the interaction points that wire to Event variants. This layer should be:

- **Platform-neutral.** Described in abstract primitives, not SwiftUI or Compose types.
- **Verifiable.** Every ViewModel field must appear in the layout; every Event must be wired to an interaction point.
- **Diffable.** Layout changes show up in version control as text diffs and support the existing ADDED/MODIFIED/REMOVED delta operations.
- **Generatable.** The define agent can produce it from the spec and proposal, the same way it produces `design.md` today.

## Design Principles

| Use the view layout artifact when: | Keep in the spec when: | Keep in the design when: |
| --- | --- | --- |
| Deciding *where* a field appears on screen | Deciding *what* the field's value means | Deciding *what type* the field is |
| Choosing between a list, grid, or card layout | Specifying that items must be scrollable | Defining the ViewModel struct and its fields |
| Placing a floating action button | Specifying that tapping "add" creates an item | Mapping the Event variant to `update()` logic |
| Ordering elements within a screen | Specifying page transitions and navigation | Defining the Route and Page enums |
| Referencing design tokens for spacing/color | Specifying error states and recovery | Defining capability requirements |

The boundary follows the existing Specify principle: specs define behavior, design defines the technical contract, and the new artifact defines visual arrangement. Shell writers consume all three.

## Detailed Design

### New Artifact: `views.md`

A structured markdown document that describes the layout of each screen in the application using a platform-neutral component vocabulary. One section per ViewModel variant that carries data.

#### Example

For a todo app with `ViewModel::TodoList(TodoListView { items: Vec<ItemView>, count: String, filter: String })`:

```markdown
# Views

## TodoListScreen

Maps to: `ViewModel::TodoList(TodoListView)`

### Layout

- Scaffold
  - TopBar
    - Title: "My Todos" (typography: title)
    - Badge: {count} (color: primaryContainer, corner-radius: full)
  - Content: ScrollableList
    - each {items}:
      - Card (spacing: sm)
        - Row (spacing: md)
          - Checkbox: {completed} → ToggleTodo({id})
          - Column
            - Text: {title} (typography: body, strikethrough-when: {completed})
            - Text: {due_date} (typography: caption, color: onSurfaceVariant)
          - Spacer
          - IconButton: trash → DeleteTodo({id})
  - BottomBar
    - SegmentedControl: {filter} → SetFilter({value})
      - segments: ["All", "Active", "Completed"]
  - FloatingAction: plus → Navigate(AddTodo)

### Empty State

When {items} is empty:

- CenteredContent
  - Icon: clipboard (size: xl, color: onSurfaceVariant)
  - Text: "No todos yet" (typography: title)
  - Text: "Tap + to add your first todo" (typography: body, color: onSurfaceVariant)
```

#### Format Rules

1. **Screen sections.** Each `## ScreenName` section maps to one ViewModel variant. The `Maps to:` line establishes traceability.

2. **Component tree.** Indented bullet lists describe the component hierarchy. Each bullet is a component with optional properties in parentheses.

3. **Field bindings.** Curly braces `{field}` bind to per-page view struct fields. Every field in the view struct must appear at least once.

4. **Event wiring.** The `→ EventVariant(args)` syntax wires interactions to shell-facing Event variants. Every shell-facing Event that belongs to this screen must be wired.

5. **Design token references.** Parenthetical properties reference design system tokens by name (`typography: title`, `color: primary`, `spacing: md`). Shell writers resolve these to `VectisTypography.title`, `VectisColors.primary`, `VectisSpacing.md` on each platform.

6. **Conditional rendering.** `When {field} is {value}:` blocks describe conditional layout. `{field}-when: {condition}` is shorthand for conditional styling on a single property (e.g., `strikethrough-when: {completed}`).

7. **Iteration.** `each {collection}:` describes repeated content bound to a `Vec<T>` field.

### Component Vocabulary

The vocabulary is deliberately small — a wireframing-level set of primitives, not a UI framework. Shell writers map these to platform-native components.

| Component | Description | SwiftUI | Compose |
| --- | --- | --- | --- |
| `Scaffold` | Screen-level container with slots for TopBar, Content, BottomBar, FloatingAction | `NavigationStack` + body | `Scaffold` |
| `TopBar` | Top navigation bar | `NavigationTitle` + toolbar | `TopAppBar` |
| `BottomBar` | Bottom bar or tab bar | `TabView` or toolbar | `BottomAppBar` / `NavigationBar` |
| `FloatingAction` | Floating action button | `.overlay` / `ZStack` | `FloatingActionButton` |
| `ScrollableList` | Scrollable list of items | `List` / `ScrollView + LazyVStack` | `LazyColumn` |
| `Grid(columns: N)` | Grid layout | `LazyVGrid` | `LazyVerticalGrid` |
| `Row` | Horizontal stack | `HStack` | `Row` |
| `Column` | Vertical stack | `VStack` | `Column` |
| `Card` | Elevated container | Rounded container with shadow | `Card` / `ElevatedCard` |
| `Spacer` | Flexible space | `Spacer()` | `Spacer(Modifier.weight(1f))` |
| `Text` | Text label | `Text` | `Text` |
| `Icon` | Icon display | `Image(systemName:)` | `Icon` |
| `IconButton` | Tappable icon | `Button` with `Image` | `IconButton` |
| `Button` | Text button | `Button` | `Button` / `TextButton` |
| `Checkbox` | Toggle control | `Toggle` | `Checkbox` |
| `TextField` | Text input | `TextField` | `OutlinedTextField` |
| `SegmentedControl` | Segment picker | `Picker(.segmented)` | `SingleChoiceSegmentedButtonRow` |
| `CenteredContent` | Centered empty/loading state | `VStack` with `Spacer` padding | `Box(contentAlignment = Center)` |
| `Divider` | Visual separator | `Divider()` | `HorizontalDivider()` |
| `Image` | Image display | `AsyncImage` / `Image` | `AsyncImage` / `Image` |

New components can be added as needed. The vocabulary is intentionally open — if a layout requires a component not in the table, introduce it with a descriptive name and document the platform mapping in the view layout.

### Pipeline Integration

#### Schema Change

Add the `views` brief to `schemas/vectis/schema.yaml`:

```yaml
pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: views
      brief: briefs/views.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
```

The `views` brief declares:

```yaml
---
id: views
description: Define the visual layout of each screen
generates: views.md
needs: [specs, proposal]
---
```

It reads the spec to know which screens exist (ViewModel variants from spec requirements about views/pages) and what interactions they support (Event variants from spec requirements about features). It reads the proposal to know which platforms are targeted (determines whether to include platform-specific layout sections).

#### Why `views` precedes `design`

In the current pipeline, the design brief infers per-page view struct fields from the spec. With the views artifact, this inference becomes explicit: the layout declares which fields appear on screen and how, and the design brief reads the layout to confirm the view struct has the right shape. This ordering means:

1. **Spec** defines behavior and identifies screens.
2. **Views** defines how each screen is composed and which data it needs.
3. **Design** defines the type system, now with views as an additional input to validate view struct completeness.

If the views artifact shows `{due_date}` on the TodoListScreen but the spec never mentions a due date, the design brief can surface this as a gap.

#### Brief Content (`schemas/vectis/briefs/views.md`)

The brief instructs the define agent to:

1. Read the spec and identify every screen (ViewModel variant).
2. For each screen, identify the data it displays (fields from spec requirements about views) and the interactions it supports (Event variants from spec requirements about features).
3. Compose a layout using the component vocabulary, binding every field and wiring every interaction.
4. Reference design system tokens for styling (if `design-system` is in the proposal's Platforms list, or if `design-system/tokens.yaml` exists).
5. Include platform-specific layout notes in dedicated sections when the proposal targets multiple platforms and the layout differs between them.

### Shell Writer Consumption

#### Input Analysis Changes

Both `ios-writer` and `android-writer` currently have an Input Analysis step that reads `app.rs` types. The updated input analysis additionally reads `views.md`:

| Extract | Source | Maps to |
| --- | --- | --- |
| Screen layout trees | `views.md` screen sections | View body composition |
| Field bindings | `views.md` `{field}` references | Property bindings in views |
| Event wiring | `views.md` `→ Event(args)` | `onEvent()` / interaction handlers |
| Token references | `views.md` `(typography: ...)` | `VectisTypography.*` / `VectisColors.*` |
| Conditional rendering | `views.md` `When` blocks | `if`/`switch` in view code |
| Iteration | `views.md` `each` blocks | `ForEach` / `LazyColumn items` |

#### Mapping Priority

When the views artifact is present, shell writers use it as the primary layout guide. When absent (for backward compatibility with existing changes that predate RFC-7), shell writers fall back to the current inference behavior. The fallback ensures existing projects and in-flight changes are not disrupted.

### Delta Operations

The views artifact supports the same delta operations as specs:

```markdown
## ADDED Screens

## TodoListScreen
Maps to: ViewModel::TodoList(TodoListView)
### Layout
...

## MODIFIED Screens

## HomeScreen
Maps to: ViewModel::Home(HomeView)
### Layout
...

## REMOVED Screens

## OnboardingScreen
**Reason**: Onboarding flow replaced by in-app tooltips
```

This integrates with the existing spec-merge infrastructure. When `/spec:merge` runs, the views artifact merges into `views.md` in the baseline alongside the spec files.

### Validation

The `specify validate` command gains checks for the views artifact:

| Check | Description |
| --- | --- |
| **Field coverage** | Every field in each per-page view struct (from design) appears in the corresponding screen layout |
| **Event coverage** | Every shell-facing Event variant relevant to a screen has a `→` wiring in that screen's layout |
| **Token resolution** | Every token reference (`typography: X`, `color: Y`, `spacing: Z`) resolves to an entry in `tokens.yaml` (when the design system exists) |
| **Component validity** | Every component name is in the vocabulary or explicitly introduced in the layout |
| **ViewModel mapping** | Every `Maps to:` line references a declared ViewModel variant from the design |

These checks run during the build phase before shell writers are invoked, catching mismatches between the views artifact and the spec/design early.

## Incremental Adoption Path

### Phase 1: Manual views artifact (low risk)

Add the `views` brief to the vectis schema and update the define agent to produce `views.md`. Shell writers **read** `views.md` when present but fall back to inference when absent. No existing functionality changes. This is a pure addition.

Deliverables:
- `schemas/vectis/briefs/views.md` brief file
- Updated `schemas/vectis/schema.yaml` pipeline
- Updated ios-writer and android-writer Input Analysis sections
- Updated core-writer Artifact-to-Code Mapping table (views artifact feeds shell writers, not core — but the core-writer's view struct fields should align)

### Phase 2: Validation

Add the views-specific checks to `specify validate`. These catch drift between views, specs, and design before the build phase runs.

Deliverables:
- Validation checks in the CLI
- Updated build brief to run views validation before shell generation

### Phase 3: Component library

Introduce named compositions — reusable patterns built from the primitive vocabulary. These live in the design system alongside `tokens.yaml`:

```yaml
# design-system/components.yaml
search-bar:
  layout:
    - Row (spacing: sm)
      - Icon: search (color: onSurfaceVariant)
      - TextField: {query} → UpdateSearch({value})
      - IconButton: clear → ClearSearch
        when: {query} is not empty

item-card:
  layout:
    - Card (spacing: sm)
      - Row (spacing: md)
        - slot: leading
        - Column
          - Text: {title} (typography: body)
          - Text: {subtitle} (typography: caption, color: onSurfaceVariant)
        - Spacer
        - slot: trailing
```

Screen layouts reference these by name:

```markdown
- SearchBar: {query} → UpdateSearch({value})
- ScrollableList
  - each {items}:
    - ItemCard
      - leading: Checkbox: {completed} → ToggleTodo({id})
      - trailing: IconButton: trash → DeleteTodo({id})
```

This reduces repetition across screens and establishes a shared vocabulary between designers and the define agent. The component library is optional — layouts can always use primitive components directly.

### Phase 4: Web shell writer

With the layout vocabulary and design system in place, a web shell writer can map the same `views.md` to HTML/CSS/JS (or a framework like React, Leptos, or Yew). The layout primitives map naturally:

| Component | Web mapping |
| --- | --- |
| `Scaffold` | Page layout with header, main, footer |
| `ScrollableList` | `<ul>` / virtual list |
| `Grid(columns: N)` | CSS Grid |
| `Row` | Flexbox row |
| `Column` | Flexbox column |
| `Card` | `<article>` or `<div>` with card styling |
| `FloatingAction` | Fixed-position button |

The web shell writer reads `views.md`, `design.md`, and `tokens.yaml` — the same inputs as the iOS and Android writers. The design system gains a `design-system/web/` output directory for CSS custom properties generated from `tokens.yaml`.

## Alternatives Considered

**Image wireframes / mockups.** Rejected because they are not diffable, not mergeable by the existing spec-merge infrastructure, not verifiable by CLI tooling, and require external design tools. They also cannot be generated by the define agent in the text-based Specify workflow. A textual layout format preserves all the properties that make specs work.

**Extend specs with layout hints.** Rejected because it blurs the behavioral/visual boundary, makes specs brittle to visual changes, and couples the core-driving artifact to shell-specific concerns. See Motivation § "Why Specs Should Not Change."

**Extend design.md with layout sections.** Partially viable — the design already has `## iOS Shell Details` and `## Android Shell Details` sections. However, layout is a concern that cuts across all platforms and deserves its own artifact with dedicated validation. Embedding it in the design would make the design document responsible for both the type system (consumed by core-writer) and the visual arrangement (consumed by shell writers), violating the single-responsibility principle that keeps artifacts clean.

**YAML or JSON instead of markdown.** Viable for the layout tree itself (more deterministically parseable), but inconsistent with every other Specify artifact. The indented-list format in markdown is a reasonable middle ground: human-readable, diff-friendly, and parseable by pattern matching in the shell writer skills. If parsing reliability becomes a problem in practice, a structured format (YAML) can be introduced as an alternative representation alongside the markdown, similar to how `tokens.yaml` coexists with `spec.md` in the design system.

**Full design tool integration (Figma, Sketch).** Rejected for the same reasons as image wireframes, plus the additional complexity of maintaining an API integration, authentication, and sync workflow. The textual format can be authored by the same define agent that writes specs and design, keeping the workflow unified. Nothing prevents a future tool from *generating* `views.md` from a Figma file — the format is the contract, not the authoring tool.

## Open Questions

1. **Granularity of the component vocabulary.** The initial vocabulary is deliberately small. Should it include higher-level components like `NavigationDrawer`, `BottomSheet`, or `Dialog` from the start, or grow on demand?

2. **Platform-divergent layouts.** When the same screen should look materially different on iOS vs Android (e.g., bottom tabs vs navigation drawer), should `views.md` have platform-specific sections (like specs do), or should there be separate `views-ios.md` / `views-android.md` files?

3. **Animation and transitions.** The current vocabulary covers static layout. Page transitions, list item animations, and gesture-driven interactions are visual concerns that may need representation. Should they be part of the views artifact, or a separate concern?

4. **Accessibility semantics.** Should the views artifact include accessibility hints (roles, labels, traits) or leave those to the shell writers? The current ios-writer already checks for `accessibilityLabel` on interactive icons.

## References

- `schemas/vectis/schema.yaml` — current pipeline definition
- `schemas/vectis/briefs/specs.md` — spec brief format (model for views brief)
- `schemas/vectis/briefs/design.md` — design brief format
- `plugins/vectis/skills/ios-writer/SKILL.md` — iOS shell generation (Input Analysis, Spec-to-Code Mapping)
- `plugins/vectis/skills/android-writer/SKILL.md` — Android shell generation
- `plugins/vectis/skills/design-system-writer/SKILL.md` — design system generation from `tokens.yaml`
- `docs/vectis.md` — user-facing Vectis documentation
- [RFC-6: Vectis Bootstrap CLI](rfc-6-vectis-bootstrap.md) — CLI scaffolding (views artifact consumed after scaffold)
