---
id: discovery
description: Read --from artefacts and/or analyse codebases; emit a neutral capability inventory grouped by shared-core / iOS / Android, plus a cross-cutting UI inputs section for layout / tokens / assets work.
generates: .specify/plans/<name>/discovery.md
---

Produce a neutral, capability-agnostic inventory for the change, grouped into the three Crux-stack tiers Vectis ships in: **shared core** (Rust `App` traits, domain types, cross-platform business logic), **iOS shell** (SwiftUI views and bindings), and **Android shell** (Jetpack Compose views and bindings). When the change also touches **input artifacts** that the shells consume — `layout.yaml`, `tokens.yaml`, `assets.yaml`, and future `components.yaml` — surface them in a separate **cross-cutting UI inputs** section after the three tiers (RFC-11 §L: token / asset / layout work is input context, not a peer platform). Discovery is read-only: it does NOT write to `plan.yaml` and does NOT propose slices. Its only output is the inventory that `propose.md` will decompose.

## Inputs

- `--from <path>...` — artefact files or directories authored by a human (briefs, RFCs, product docs, ADRs). Zero or more.
- `--against <path>` — an existing codebase to delta against. At most one. Interpreted as a local filesystem path.
- `--source <key>=<path-or-url>...` — named sources for migration or legacy analysis. `<path-or-url>` is either a local path or a git URL. Zero or more. The `<key>` is the identifier recorded on each plan entry's `sources` list in the next brief.

At least one of `--from`, `--against`, or `--source` must be supplied.

## Process

1. **Analyse each `--source` and `--against` input.** For every non-`--from` input, invoke `/spec:extract` to produce a domain-level capability description:
   - For a git URL `--source`: materialise the URL into `legacy/<key>/` with the inlined guarded `git clone` snippet (see [`../../../../../spec/skills/analyze/SKILL.md` §*Cloning a source tree*](../../../../../spec/skills/analyze/SKILL.md)), then run `/spec:extract legacy/<key> .specify/plans/<name>/extract/<key>/`.
   - For a local path `--source` or `--against`: run `/spec:extract <path> .specify/plans/<name>/extract/<key>/` directly (use `against` as the key for `--against`).
   - The extract artefacts under `.specify/plans/<name>/extract/` are intermediate — the inventory below is the only human-facing output.
2. **Read each `--from` artefact.** Open every `--from` file (or every file under a `--from` directory). Parse any clearly delimited capability structure (e.g. headings named "Capability", "Feature", "Screen", "Component"); otherwise treat each top-level heading as a capability candidate and record the accompanying prose verbatim.
3. **Classify each capability into a Crux tier or UI input.** Every capability lands in exactly one of three tiers or, when it describes an operator-maintained input artifact, in the cross-cutting **UI inputs** section instead:
   - **Shared core** — cross-platform business logic expressed as a Crux `App` trait (`Model`, `Event`, `ViewModel`, `Effect`, `Command`). Anything that must run identically on iOS and Android belongs here. Heuristic: if a legacy screen has behaviour that is platform-agnostic (state machines, data fetching, validation), the behaviour is shared-core; only the rendering is shell.
   - **iOS shell** — SwiftUI views, iOS-specific bindings, platform extensions (`UIKit` bridges, Swift Package integrations). Names typically end in `-ios-view`, `-ios-binding`, or describe an iOS-only affordance.
   - **Android shell** — Jetpack Compose views, Material 3 components, Kotlin bindings. Names typically end in `-android-view`, `-android-binding`, or describe an Android-only affordance.
   - **Cross-cutting UI inputs** — operator-maintained input artifacts the shells read directly per RFC-11 §L: `layout.yaml`, `tokens.yaml`, `assets.yaml`, and future `components.yaml`. A capability lands here only when it describes the *input artifact itself* (e.g. "lift legacy CSS variables into `tokens.yaml`", "import Figma layout into `layout.yaml`"), not when it describes a feature screen that happens to consume tokens. UI inputs are NOT a peer Crux tier — they have no runtime presence — and `vectis:ios-writer` / `vectis:android-writer` consume them directly without an intermediate "design-system" generation step. Record the artifact name on each UI-input capability so `propose.md` can decide whether the work is independently reviewable. Capabilities that legitimately span tiers (e.g. "counter" covering the shared `App` AND the iOS/Android views) are split into one entry per tier so `propose.md` can slice them independently; capabilities that span a tier and a UI input (e.g. a screen that requires both a new shared-core ViewModel AND new tokens) are similarly split, with the UI input surfacing in the cross-cutting section.
4. **Merge into a single inventory.** Deduplicate capabilities that recur across sources within the same tier (e.g. "counter-core" in both a brief and a monolith extract). Record every source that mentions a capability rather than picking one.
5. **Write `.specify/plans/<name>/discovery.md`.** The output has a fixed shape (see "Output" below). Overwrite any existing file.

## Output

```markdown
# Discovery — <change-name>

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

## Cross-cutting UI inputs

<!-- UI inputs (layout.yaml, tokens.yaml, assets.yaml, future
components.yaml) are operator-maintained input artifacts the shells
consume — they are NOT a peer Crux tier (RFC-11 §L). Surface each
input the change authors, migrates, or refines as a subsection
here. Omit the entire section when no UI-input work is in scope.
Subsections are level-3 headings (no enclosing tier wrapper, unlike
the capability subsections above which sit one level deeper inside
their tier heading). -->

### <input name>

- **Artifact**: `tokens.yaml` | `assets.yaml` | `layout.yaml` |
  `components.yaml`
- **Source(s)**: <key>, <path>, ...
- **Description**: <one or two sentences, source-neutral>
- **Ordering hints**: <e.g. "consumed by counter-ios-view,
  counter-android-view"; omit if none>
- **Scope hints**: <e.g. "lift legacy SCSS variables into
  tokens.yaml"; omit if none>

<!-- repeat one subsection per UI input in scope -->

## Open questions

- <question requiring human input before proposal>
- <...>
```

Empty tiers are emitted as the `### <tier>` heading followed by an `_No capabilities in this tier._` italic line; the three tier headings (shared core, iOS shell, Android shell) are always present so downstream tooling can rely on their order. The `## Cross-cutting UI inputs` section is omitted entirely when no input-artifact work is in scope — its absence is meaningful (no UI inputs to surface), so do not emit a placeholder italic line for it.

## Idempotency

Running discovery twice on the same inputs MUST produce the same `discovery.md`. Implications:

- Tier order is fixed: shared core, iOS shell, Android shell. Within each tier, order capabilities alphabetically by name. The cross-cutting UI inputs section, when emitted, follows the three tiers — order its subsections alphabetically by input name (the same as a tier).
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

## Cross-cutting UI inputs

### design-tokens

- **Artifact**: `tokens.yaml`
- **Source(s)**: legacy-tokens (legacy/design-tokens.yaml)
- **Description**: Colour, typography, and spacing tokens migrated
  from the legacy iOS / Android codebases into a single
  `tokens.yaml` catalogue. `vectis:ios-writer` and
  `vectis:android-writer` read it directly per RFC-11 §L; there is
  no separate design-system generation step.
- **Ordering hints**: depends on theme-core; consumed by
  counter-ios-view, counter-android-view.
- **Scope hints**: lift the legacy iOS Asset Catalog colour set and
  the Android `colors.xml` palette into a single `tokens.yaml`
  authored under `design-system/`.

## Open questions

- Should `theme-core` own the light/dark mode toggle state, or
  is that a shell-local concern read from each OS's system
  appearance API?
- The legacy Android codebase ships custom motion / elevation
  tokens that have no iOS counterpart — surface them as
  Android-only theme entries during shell generation, or omit
  them from `tokens.yaml` entirely until iOS catches up?
```
