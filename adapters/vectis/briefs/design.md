---
id: design
description: Create the design document to explain HOW to implement the change
generates: design.md
needs: [proposal, specs]
---

Include sections based on the platforms declared in the proposal. The Domain Model and Capabilities sections are always present (core is always in scope). Platform-specific sections are included only when the corresponding platform is listed in the proposal.

`design.md` is a *reader* of the wired UI input set — `composition.yaml` (the lifecycle artifact emitted by [`briefs/composition.md`](composition.md) earlier in this define run), `tokens.yaml`, and `assets.yaml` — not a parallel surface for the same information (RFC-11 §H). It MUST NOT reproduce the layout tree, the asset manifest, or the token catalog; reference those artifacts by name and capture only the design implications they impose: screen names, ViewModel variants, per-page view structs, Route needs, `bind` field completeness, capability fan-out, token usage policy, asset usage policy, and platform-specific shell notes. Do not consume `layout.yaml` from `design.md` — that is the composition brief's job; `design.md` reads only the wired output.

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

<!-- When `contracts/http/` exists: reference the OpenAPI specifications
     there rather than re-describing endpoint shapes. Add implementation-level notes:
     auth, rate limits, caching, versioning strategy.
     
     When no baseline contracts exist: endpoints with method, URL,
     request/response shapes, errors. Include only when HTTP capability is used. -->

## iOS Shell Details

<!-- Include when ios is listed in Platforms.
Screen names and ViewModel variants come from `composition.yaml` — do not
re-list them here, only call out platform-specific customisations on top.
- Navigation style (single, stack, tabs)
- Per-screen customisations that go beyond what composition.yaml expresses
- Platform features (haptics, share sheet, etc.)
- Design system overrides -->

## Android Shell Details

<!-- Include when android is listed in Platforms.
Screen names and ViewModel variants come from `composition.yaml` — do not
re-list them here, only call out platform-specific customisations on top.
- Navigation patterns (single activity, bottom nav, drawer)
- Material 3 customisations per ViewModel variant that go beyond composition.yaml
- Platform features (edge-to-edge, system bars, haptics)
- Koin DI requirements (when multiple non-Render effects)
- Capability client details (Ktor for HTTP/SSE, SharedPreferences for KV) -->

## Implementation Constraints

<!-- Runtime and dependency constraints. Do not query a Vectis version CLI from
this brief. If the slice supplies an explicit scaffold version file, reference
that path and summarize the Crux, facet, and uniffi pins it declares; otherwise
state that the scaffold tool's embedded defaults apply until a version-update
workflow is run by `/vectis:template-updater`.

Standard platform constraints:
- Swift 6, iOS 17+ deployment target
- Kotlin 2.x, Jetpack Compose, Material 3, min SDK 34
- Java 21 LTS (NOT Java 25+)
-->

## Dependencies

<!-- External packages or services this change depends on -->

## Risks / Open Questions

<!-- Known risks, trade-offs, and unresolved decisions -->

## Notes

<!-- Additional observations or considerations -->
```

## Reading the wired composition

By the time this brief runs, the composition brief has already emitted `composition.yaml` for any UI-bearing slice (the define pipeline orders `composition` before `design` — see [`capability.yaml`](../capability.yaml)). Read `composition.yaml` along with sibling `tokens.yaml` and `assets.yaml` (when present in the slice or under `design-system/`) and use them as the authoritative sources for layout-derived implications. The declared tool command `specify tool run vectis -- validate composition` (with auto-invoked `tokens` / `assets` modes) is the deterministic gate; this brief does not duplicate its checks.

- **Resolution.** Look for `composition.yaml` first at `.specify/slices/<name>/composition.yaml`, then at `.specify/specs/composition.yaml`. The same lookup applies to `tokens.yaml` and `assets.yaml`: slice-local files first, then project-level `design-system/tokens.yaml` / `design-system/assets.yaml`.
- **ViewModel adoption.** Adopt the screen names, ViewModel variant names, and field names proposed by `composition.yaml`. Adjust naming only when Rust conventions or domain model considerations require it.
- **Field completeness.** Every `bind` value in `composition.yaml` must appear as a field in the corresponding per-page view struct. If a `bind` references a field not described in the spec, flag the mismatch.
- **Token usage policy.** Reference token names from `tokens.yaml` (e.g. `colors.primary.dark`) only as policy notes — for example, "the iOS shell falls back to system colors when this token is absent". Do NOT enumerate the token catalog; `tokens.yaml` is the source of truth and `specify tool run vectis -- validate tokens` is its gate.
- **Asset usage policy.** Reference assets by ID (matching `assets.yaml`) and capture only design-level usage notes — for example, "the empty-tasks hero is rendered at 2:1 aspect ratio". Do NOT enumerate the asset manifest; `assets.yaml` is the source of truth and `specify tool run vectis -- validate assets` is its gate.
- **Gap surfacing.** Report any `bind` in composition that has no spec backing, any spec-described data element with no composition binding, and any token / asset reference that does not resolve in the matching manifest.

When `composition.yaml` is absent (e.g. the change has no UI platforms in the proposal so the composition brief was skipped, or the project predates RFC-7 and has no baseline composition), infer the ViewModel shape from specs alone.
