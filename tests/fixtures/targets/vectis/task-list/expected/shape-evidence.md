# Shape-evidence checklist

Sections that MUST appear in `input/specs/task-list/spec.md` and `input/design.md` because the Vectis target's `shape` brief was injected during core synthesis (W3.1 / `/spec:refine`). The synthesis golden test should validate each item; the items are stable across alternate source mixes (pure-intent vs. screenshots-sourced) — only the `Sources:` lines differ.

## `input/specs/task-list/spec.md`

- One spec file per unit at `specs/<unit>/spec.md`. The fixture's single unit is `task-list`.
- Single flat `REQ-[0-9]{3}` namespace across the core body and platform sections. The fixture spans REQ-001..REQ-010 with no `REQ-IOS-*` or `REQ-ANDROID-*` prefixes.
- Every requirement carries the standard `ID:` / `Sources:` / `Status:` block (synthesis-contract default; Vectis adds no extra header fields).
- Named screen requirements drive `composition.yaml` screen slugs in `build`:
    - `Requirement: Task List View` → screen slug `task-list`, ViewModel variant `TaskList`, per-page view struct `TaskListView`.
    - `Requirement: Add Task Form View` → screen slug `add-task`, ViewModel variant `AddTask`, per-page view struct `AddTaskView`.
- Platform sections present because `proposal.md` lists `ios` and `android`:
    - `## iOS Shell Requirements` follows the core body.
    - `## Android Shell Requirements` follows the iOS section.
- Spatial Evidence (`screens` source) was folded into observable behaviour, never raw geometry: the empty-state copy in REQ-001 comes from the `task-list.states.empty.hero.*` leaf claims; the dialog message in REQ-004 comes from `task-list.overlays.delete-confirm.message`.
- Token / asset references appear only as observable product behaviour (`empty-tasks-hero` image in REQ-001), never as catalogue restatements.
- `Sources:` lines carry the source key plus optional `#claim-id` for spatial provenance (the fixture uses bare source keys for readability; real synthesis output may append claim ids).

## `input/design.md`

- `## Context` names every platform in scope and references the operator-curated `tokens.yaml` / `assets.yaml` as build inputs (not synthesis inputs).
- `## Domain Model` carries the Crux 0.17 type system: `App` struct, `Model` (with `page: Page`), internal `Page` enum (`Default` only, `#[default]` on `Loading`), shell-facing `Route` enum (`Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq` + `#[repr(C)]`), `Event` enum (shell-facing variants + `#[serde(skip)] #[facet(skip)]` callbacks + `Navigate(Route)`), `ViewModel` enum (`#[repr(C)]`, one variant per view), per-page view structs (`TaskListView`, `AddTaskView`, `ErrorView`), `Effect` enum annotated `#[effect(facet_typegen)]`, supporting types (`TaskId`, `Task`, `AddTaskForm`, `DomainError`).
- `## Adapters` table marks `Render` Yes (always), `HTTP` Yes (via `crux_http`), `Key-Value` Yes (via `crux_kv`), and `Time` / `SSE` / `Platform` No.
- `## API Contracts` enumerates the four HTTP endpoints with their methods, URLs, and bodies (and would reference `contracts/http/tasks.yaml` when present).
- `## iOS Shell Details` (present because `ios` is in scope) — navigation style, swipe-to-delete wiring, HIG fallback policy.
- `## Android Shell Details` (present because `android` is in scope) — single activity + Compose Navigation, Material 3 fallback policy, edge-to-edge handling, Koin not required at this scope, UniFFI library override requirement.
- `## Implementation Constraints` pins Swift 6 / iOS 17+, Kotlin 2.x / Compose / Material 3 / minSdk 34, Java 21 LTS.
- Naming conventions are documented (kebab-case screen slugs, PascalCase ViewModel / Event names, snake_case fields) so `build` regenerates `composition.yaml` deterministically.
- `## Notes` reiterates that `composition.yaml` is a build output (not authored at synthesis time) and that `tokens.yaml` / `assets.yaml` are operator-curated.

## `input/tasks.md`

- Tasks organised by build phase, not by feature: `## Core` first, then `## iOS shell`, then `## Android shell`.
- No standalone token / asset / layout phase — these are input context for the shells.
- Every task is agent-completable through writer / reviewer work and local build / test commands; no manual mobile testing, no production credentials, no visual inspection.

## `expected/composition.yaml`

- One `screens:` entry per named screen requirement in `spec.md`: `task-list`, `add-task`, `settings`.
- `maps_to` on every screen wires to the matching `ViewModel::<Variant>(<Variant>View)` from `design.md`.
- Every `bind` value resolves to a field in the matching per-page view struct.
- Every `event` value resolves to an `Event` variant in `design.md` (`ToggleTask(id)`, `RequestDelete(id)`, `ConfirmDelete`, `CancelDelete`, `AddTaskTitleChanged(value)`, `AddTaskSubtitleChanged(value)`, `SaveNewTask`, `Navigate(Route::Settings)`).
- The `task-row` `component:` directive is preserved from the upstream spatial Evidence; structural-identity holds across instances.
- `states.empty` replaces the `body` region on the Task List screen; `overlays.delete-confirm` carries `kind: dialog` and `trigger: RequestDelete`.
- `platforms.ios` overrides the Task List body's row interaction to include the swipe-to-delete gesture (REQ-008); `platforms.android` adds the system-bars padding on the FAB host (REQ-010).
