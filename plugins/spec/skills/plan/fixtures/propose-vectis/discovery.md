# Discovery — counter-migration

## Capability inventory

### Shared core

#### counter-core

- **Source(s)**: legacy-ios (legacy/counter-ios),
  legacy-android (legacy/counter-android)
- **Description**: Increment/decrement a single integer, with
  persistent storage of the last value.
- **Ordering hints**: consumed by counter-ios-view,
  counter-android-view.
- **Scope hints**: lift the shared state machine out of both
  legacy platforms into a new Crux `App` trait.

#### theme-core

- **Source(s)**: legacy-ios (legacy/counter-ios),
  legacy-android (legacy/counter-android)
- **Description**: Resolve the active light/dark theme; emit
  theme tokens to the shell via `ViewModel`.
- **Ordering hints**: consumed by design-tokens; read by
  counter-ios-view, counter-android-view indirectly via
  design-tokens.

### iOS shell

#### counter-ios-view

- **Source(s)**: legacy-ios (legacy/counter-ios)
- **Description**: SwiftUI view that renders the counter and
  forwards increment/decrement events to the shared core.
- **Ordering hints**: depends on counter-core, design-tokens.
- **Scope hints**: legacy `CounterView.swift` to be reshaped
  into a Crux-bound SwiftUI view.

### Android shell

#### counter-android-view

- **Source(s)**: legacy-android (legacy/counter-android)
- **Description**: Jetpack Compose screen that renders the
  counter and forwards increment/decrement events to the shared
  core.
- **Ordering hints**: depends on counter-core, design-tokens.
- **Scope hints**: legacy `CounterActivity.kt` + `CounterView`
  composable to be reshaped into a Crux-bound Compose screen.

### Design system

#### design-tokens

- **Source(s)**: legacy-tokens (legacy/design-tokens.yaml)
- **Description**: Colour, typography, and spacing tokens shared
  across iOS and Android; generated into a Swift Package and an
  Android `vectis-design` Compose library.
- **Ordering hints**: depends on theme-core; consumed by
  counter-ios-view, counter-android-view.

## Open questions

- Should `theme-core` own the light/dark mode toggle state, or
  is that a shell-local concern read from each OS's system
  appearance API?
- Do we ship the Android `vectis-design` library as a sibling
  module in the same Gradle build, or as a published artifact?
