## Why

Operators want a simple, cross-platform task-tracking app to validate the Vectis source/target split end-to-end: spatial Evidence from `screenshots` feeds core synthesis, which folds it into `spec.md` + `design.md` under the Vectis target's `shape` brief. The same canonical artifacts then drive `composition.yaml` regeneration and per-platform shell generation in `build`.

## Source

Manual (this fixture's `Sources:` lines are stand-ins for real plan-resolved bindings — the matching source-side fixture lives at `tests/fixtures/sources/screenshots/task-list-two-screen/`).

## What Changes

- New `task-list` feature backed by a Crux shared core with HTTP + Key-Value capabilities.
- iOS shell (SwiftUI, single navigation stack) and Android shell (Jetpack Compose, single activity).
- Shell-local theme + asset code on each platform (no shared design-system library).

## Units

### New Units

- **task-list** — Today-view task list with completion toggling, deletion (with confirmation), an empty state, and an add-task FAB.

### Modified Units

None — this is a greenfield fixture.

## Platforms

- core
- ios
- android

## Impact

- No external API contracts changed; the slice depends on a local HTTP backend that returns `Task` records and accepts toggle / delete mutations.
- Local persistence via the Key-Value adapter for offline cache.
- No design-system module is introduced; both shells emit shell-local theme + asset code under `iOS/<App>/Theme/` and `Android/.../ui/theme/` respectively.
