Create the design document to explain HOW to implement the change.

Include sections based on the platforms declared in the proposal.
The Domain Model and Capabilities sections are always present (core
is always in scope). Platform-specific sections are included only when
the corresponding platform is listed in the proposal.

## Output Structure

```markdown
## Context

<!-- Platforms in scope (from proposal), purpose, and background for this change -->

## Domain Model

<!-- Crux type system design.

Define these types (see guidance below each):

### App struct
- Name derived from Overview (e.g., TodoApp, NoteEditor)

### Model
- All internal state fields with types
- Must include `page: Page` field
- Use newtypes and enums for domain concepts

### Page (internal)
- Enum with one variant per view
- Derives Default only (no Facet, no Serialize)
- #[default] on initial variant (typically Loading)

### Route (shell-facing)
- Enum enumerating user-navigable destinations
- Excludes internal states (Loading, Error)
- Derives Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq
- #[repr(C)]

### Event
- Shell-facing variants (serializable, sent by UI)
- Internal variants (#[serde(skip)] #[facet(skip)], used as effect callbacks)
- Navigate(Route) variant for shell-initiated navigation

### ViewModel
- Enum with #[repr(C)] and one variant per view
- Variants without data have no payload; variants with data wrap per-page view structs
- Derives Facet, Serialize, Deserialize, Clone, Debug, Default

### Per-page view structs
- One struct per ViewModel variant that carries data
- All fields pub, use String for formatted display values

### Effect
- One variant per capability, annotated with #[effect(facet_typegen)]

### Supporting types
- Domain structs/enums used in Model, Event, or view structs -->

## Capabilities

<!-- Crux capability table. Mark each Yes/No with details.

| Capability | Needed? | Details |
|---|---|---|
| **Render** | Yes (always) | |
| **HTTP** (`crux_http`) | | |
| **Key-Value** (`crux_kv`) | | |
| **Timer / Time** (`crux_time`) | | |
| **Server-Sent Events** (custom) | | |
| **Platform** (`crux_platform`) | | | -->

## API Contracts

<!-- Endpoints with method, URL, request/response shapes, errors.
Include only when HTTP capability is used. -->

## iOS Shell Details

<!-- Include when ios is listed in Platforms.
- Navigation style (single, stack, tabs)
- Screen customizations per ViewModel variant
- Platform features (haptics, share sheet, etc.)
- Design system overrides -->

## Android Shell Details

<!-- Include when android is listed in Platforms.
- Navigation patterns (single activity, bottom nav, drawer)
- Material 3 screen customizations per ViewModel variant
- Platform features (edge-to-edge, system bars, haptics)
- Koin DI requirements (when multiple non-Render effects)
- Capability client details (Ktor for HTTP/SSE, SharedPreferences for KV) -->

## Design System Details

<!-- Include when design-system is listed in Platforms.
- Token categories and value shapes
- Downstream consumers -->

## Implementation Constraints

<!-- Runtime and dependency constraints relevant to generation.
Standard constraints for Crux projects:
- Crux 0.17.0 (crates.io)
- facet = "=0.31" (exact pin required)
- uniffi = "=0.29.4" (exact pin, must match crux_core bundled version)
- Swift 6, iOS 17+ deployment target
- Kotlin 2.x, Jetpack Compose, Material 3, min SDK 34
- Java 21 LTS (NOT Java 25+)
- VectisDesign Swift Package for all styling tokens -->

## Dependencies

<!-- External packages or services this change depends on -->

## Risks / Open Questions

<!-- Known risks, trade-offs, and unresolved decisions -->

## Notes

<!-- Additional observations or considerations -->
```
