---
id: discovery
description: Read --from artefacts and/or analyse codebases; emit a neutral capability inventory grouped by shared-core / iOS / Android / design-system.
generates: .specify/plans/<name>/discovery.md
---

Produce a neutral, schema-agnostic capability inventory for the initiative, grouped into the Crux-stack tiers Vectis ships in: **shared core** (Rust `App` traits, domain types, cross-platform business logic), **iOS shell** (SwiftUI views and bindings), **Android shell** (Jetpack Compose views and bindings), and **design system** (tokens and the generated component libraries). Discovery is read-only: it does NOT write to `plan.yaml` and does NOT propose slices. Its only output is the inventory that `propose.md` will decompose.

## Inputs

- `--from <path>...` — artefact files or directories authored by a human (briefs, RFCs, product docs, ADRs). Zero or more.
- `--against <path>` — an existing codebase to delta against. At most one. Interpreted as a local filesystem path.
- `--source <key>=<path-or-url>...` — named sources for migration or legacy analysis. `<path-or-url>` is either a local path or a git URL. Zero or more. The `<key>` is the identifier recorded on each plan entry's `sources` list in the next brief.

At least one of `--from`, `--against`, or `--source` must be supplied.

## Process

1. **Analyse each `--source` and `--against` input.** For every non-`--from` input, invoke `/spec:extract` to produce a domain-level capability description:
   - For a git URL `--source`: clone via `/rt:git-cloner` into `legacy/<key>/` first, then run `/spec:extract legacy/<key> .specify/plans/<name>/extract/<key>/`.
   - For a local path `--source` or `--against`: run `/spec:extract <path> .specify/plans/<name>/extract/<key>/` directly (use `against` as the key for `--against`).
   - The extract artefacts under `.specify/plans/<name>/extract/` are intermediate — the inventory below is the only human-facing output.
2. **Read each `--from` artefact.** Open every `--from` file (or every file under a `--from` directory). Parse any clearly delimited capability structure (e.g. headings named "Capability", "Feature", "Screen", "Component"); otherwise treat each top-level heading as a capability candidate and record the accompanying prose verbatim.
3. **Classify each capability into a Crux tier.** Every capability lands in exactly one of four tiers:
   - **Shared core** — cross-platform business logic expressed as a Crux `App` trait (`Model`, `Event`, `ViewModel`, `Effect`, `Command`). Anything that must run identically on iOS and Android belongs here. Heuristic: if a legacy screen has behaviour that is platform-agnostic (state machines, data fetching, validation), the behaviour is shared-core; only the rendering is shell.
   - **iOS shell** — SwiftUI views, iOS-specific bindings, platform extensions (`UIKit` bridges, Swift Package integrations). Names typically end in `-ios-view`, `-ios-binding`, or describe an iOS-only affordance.
   - **Android shell** — Jetpack Compose views, Material 3 components, Kotlin bindings. Names typically end in `-android-view`, `-android-binding`, or describe an Android-only affordance.
   - **Design system** — design tokens (`tokens.yaml`) and the generated iOS Swift Package + Android `vectis-design` Compose library. A capability lands here only when it describes *tokens* or *reusable component primitives*, not when it describes a feature screen that happens to consume tokens. Record the chosen tier on each capability. Capabilities that legitimately span tiers (e.g. "counter" covering the shared `App` AND the iOS/Android views) are split into one entry per tier so `propose.md` can slice them independently.
4. **Merge into a single inventory.** Deduplicate capabilities that recur across sources within the same tier (e.g. "counter-core" in both a brief and a monolith extract). Record every source that mentions a capability rather than picking one.
5. **Write `.specify/plans/<name>/discovery.md`.** The output has a fixed shape (see "Output" below). Overwrite any existing file.

## Output

```markdown
# Discovery — <initiative-name>

## Capability inventory

### Shared core

#### <capability name>

- **Source(s)**: <key>, <path>, <literal artefact path>, ...
- **Description**: <one or two sentences, source-neutral>
- **Ordering hints**: <e.g. "depends on theme-core", "consumed
  by counter-ios-view"; omit if none>
- **Scope hints**: <e.g. "legacy iOS view logic to lift into
  App trait", "greenfield state machine"; omit if none>

<!-- repeat one subsection per shared-core capability -->

### iOS shell

#### <capability name>

- **Source(s)**: <key>, <path>, ...
- **Description**: <one or two sentences, source-neutral>
- **Ordering hints**: <e.g. "depends on counter-core",
  "consumes design-tokens"; omit if none>
- **Scope hints**: <e.g. "legacy SwiftUI view", "new Compose
  binding needed"; omit if none>

<!-- repeat one subsection per iOS-shell capability -->

### Android shell

<!-- same shape as iOS shell, one subsection per capability -->

### Design system

#### <capability name>

- **Source(s)**: <key>, <path>, ...
- **Description**: <one or two sentences, source-neutral>
- **Ordering hints**: <e.g. "consumed by counter-ios-view,
  counter-android-view"; omit if none>
- **Scope hints**: <omit if none>

<!-- repeat one subsection per design-system capability -->

## Open questions

- <question requiring human input before proposal>
- <...>
```

Empty tiers are emitted as the `### <tier>` heading followed by an `_No capabilities in this tier._` italic line; the four tier headings are always present so downstream tooling can rely on their order.

## Idempotency

Running discovery twice on the same inputs MUST produce the same `discovery.md`. Implications:

- Tier order is fixed: shared core, iOS shell, Android shell, design system. Within each tier, order capabilities alphabetically by name.
- Do not include timestamps, run IDs, or working-directory paths.
- `/spec:extract` re-runs on unchanged sources must yield equivalent inventory text; if a re-extract surfaces new detail, it replaces the prior inventory entry wholesale.

## Example fragment

```markdown
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
```
