# Vectis target — `shape`

Core synthesis reads this brief alongside `Evidence[]` and the slice's `proposal.md` when it writes `spec.md` and `design.md` for a `target: vectis` slice. The brief is **input to synthesis**, not a runtime step: it does not consume Evidence on its own, does not write artifacts, and does not transition lifecycle state. It tells the synthesiser how Vectis idioms organise canonical artifact content so the resulting `spec.md` and `design.md` can be implemented directly by the `build` brief.

`shape` only describes what synthesis should fold into the canonical artifacts. Implementation patterns, scaffold commands, verify-repair, and reviewers live in `build.md`. Operator-curated UI inputs (`tokens.yaml`, `assets.yaml`) are build-time inputs, not synthesis inputs — never describe their contents in `spec.md` / `design.md`.

## What a Vectis slice produces

A Vectis slice produces a buildable cross-platform application:

- One Crux **shared core** (Rust crate under `shared/`) carrying every requirement that is platform-neutral.
- Zero or more **platform shells** (`ios`, `android`, future `web`) that render the core's `ViewModel`, dispatch `Event` values from user interactions, and translate the core's `Effect`s into host I/O.
- A **`composition.yaml` manifest** regenerated each build from `spec.md` + `design.md` (see `build.md`). `composition.yaml` is not a Specify artifact — synthesis never writes it. The synthesiser must still describe screen-level structure precisely enough for `build` to reconstruct the composition deterministically.

`core` is always in scope. Platforms are an **app-level fact** declared once in `project.yaml.platforms` and carried verbatim to every slice's `proposal.md ## Platforms` (see below) — they are not per-slice opt-in.

## Synthesis substep notes

### `proposal.md`

- `## Source` is **Manual** for Vectis (the per-source provenance lives in `Sources:` lines on each requirement; `proposal.md` describes intent at a higher level).
- `## Domains` lists business features in kebab-case (`todo-app`, `weather-forecast`) — never implementation layers (`todo-core`, `todo-ios`). For Vectis, each domain is a business feature; each domain maps one-to-one to `specs/<domain>/spec.md`.
- `## Platforms` is the build router. Read `project.yaml.platforms` directly and stamp the full set verbatim — do not cherry-pick or trim per slice. Valid tokens: `core` (always required and always present in the set), `ios`, `android`, `web`, `desktop`. `web` and `desktop` are accepted tokens but have no build sub-briefs, scaffold support, or on-disk shell interpretation yet — do not invent shell sections for them. Tokens, assets, and layout are **not** platforms — they are build inputs to the shells. Per-shell scope (`vectis:ios-*` vs `vectis:android-*` work) is driven entirely by this list. Every slice carries the same platform set; build determines per-platform work (create / update / no-op).
- Modified domains list existing baseline spec folders that change behaviourally; new requirement-IDs append, modified IDs keep their number, removed IDs carry a deletion note.

### `spec.md` — behavioural requirements

- One spec file per domain at `specs/<domain>/spec.md`. The file is a single document.
- Requirement-IDs share **one flat `REQ-[0-9]{3}` namespace** across the core body, the `## iOS Shell Requirements` section, and the `## Android Shell Requirements` section. Never prefix per-platform (no `REQ-IOS-001`).
- The core body carries platform-neutral behaviour (what the app *does* regardless of shell). Platform sections capture shell-specific behaviour (navigation style, swipe gestures, haptics, Material 3 / HIG idioms, edge-to-edge handling).
- Every requirement carries the standard `ID:` / `Sources:` / `Status:` block from the synthesis contract; Vectis adds no extra header fields.
- **Spatial Evidence folding.** When upstream `screenshots`-source Evidence claims contribute `kind: region` / `kind: container` / `kind: leaf`, fold them into requirements that **describe what the user sees and when**, not raw geometry. Name each distinct view explicitly in its requirement title (`Requirement: Todo List View`, `Requirement: Add Todo Form`) — `build` derives screen slugs from these titles when it regenerates `composition.yaml`. Field-level claims (the leaves) inform the per-page view-struct fields described in `design.md`; group-level claims (containers) inform the screen state shape; region-level claims inform header / body / footer / fab placement. Surface the claim id (e.g. `Sources: [legacy-monolith#todos.list.header]`) on the requirement that consumed it.
- Adapters (HTTP, KV, SSE, Time, Platform), data-model field types, and per-screen wiring details belong in `design.md`, not `spec.md`. Specs describe *what* the system does.
- Token and asset references are allowed only as observable product behaviour (`the unread badge SHALL use the alert colour`, `the empty state SHALL render the empty-tasks-hero image`) — never as catalogue restatements. The catalogue is `tokens.yaml` / `assets.yaml`, validated at build time.
- Delta operations on a modified feature use `## ADDED Requirements` / `## MODIFIED Requirements` / `## REMOVED Requirements` / `## RENAMED Requirements` blocks (each requirement keeps its baseline `REQ-XXX`).

### `design.md` — implementation shape

`design.md` is the canonical reader of every upstream claim that `build` will turn into code or into `composition.yaml`. Include only the sections relevant to the platforms declared in `proposal.md` (which mirrors `project.yaml.platforms` verbatim). The Domain Model and Adapters sections are always present (core is always in scope).

Required sections in order:

- **`## Context`** — platforms in scope, purpose, background.
- **`## Domain Model`** — Crux type system: `App` struct (named after the app, e.g. `TodoApp`), `Model` (all internal state; must include `page: Page`), internal `Page` enum (one variant per view; derives `Default` only; `#[default]` on the initial variant), shell-facing `Route` enum (user-navigable destinations only; derives `Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq` and `#[repr(C)]`), `Event` enum (shell-facing variants serialisable; internal callback variants marked `#[serde(skip)] #[facet(skip)]`; include a `Navigate(Route)` variant when the shell initiates navigation), `ViewModel` enum (`#[repr(C)]`, one variant per view; variants without data have no payload, variants with data wrap a per-page view struct named `<Screen>View`), per-page view structs (all fields `pub`, use `String` for formatted display values), `Effect` enum (one variant per adapter, annotated `#[effect(facet_typegen)]`), and supporting domain types (newtypes / enums).
- **`## Adapters`** — table marking each Crux adapter Yes/No (`Render` always Yes; `HTTP` via `crux_http`; `Key-Value` via `crux_kv`; `Time` via `crux_time`; `Platform` via `crux_platform`; `Server-Sent Events` as a custom inline adapter — never a published crate).
- **`## API Contracts`** — when `contracts/http/` exists, reference the OpenAPI specs there rather than restating endpoint shapes; otherwise describe endpoints (method, URL, request/response, errors). Include only when the HTTP adapter is used.
- **`## iOS Shell Details`** (when `ios` in Platforms) — navigation style (single / stack / tabs), per-screen customisations that go beyond what `composition.yaml` will express, platform features (haptics, share sheet), HIG fallback policy when `tokens.yaml` is absent.
- **`## Android Shell Details`** (when `android` in Platforms) — navigation patterns (single activity, bottom nav, drawer), Material 3 customisations per ViewModel variant, platform features (edge-to-edge, system bars), Koin DI requirements when multiple non-Render effects are used, adapter-client details (Ktor for HTTP / SSE, SharedPreferences for KV), Material 3 fallback policy when `tokens.yaml` is absent.
- **`## Implementation Constraints`** — Swift 6 / iOS 17+ deployment target; Kotlin 2.x / Jetpack Compose / Material 3 / minSdk 34; Java 21 LTS (not Java 25+ — Gradle compatibility). When the slice supplies an explicit scaffold version file, reference its path and summarise the Crux / facet / uniffi pins it declares; otherwise note that the scaffold tool's embedded defaults apply until the operator runs the host-side template-updater workflow in `specify-cli` (symptom triage table: [`../references/known-drift.md`](../references/known-drift.md)).
- **`## Dependencies`** — external packages or services this slice depends on.
- **`## Risks / Open Questions`** — known risks, trade-offs, unresolved decisions.

`design.md` MUST fold every claim that `build` needs to regenerate `composition.yaml`:

- **Screen names and ViewModel variants.** Adopt the screen titles from `spec.md` requirements (e.g. `Requirement: Todo List View` → screen slug `todo-list`, ViewModel variant `TodoList`, per-page view struct `TodoListView`). Document the slug-derivation rule in `## Context` if any rename is necessary.
- **Per-page view struct fields.** Every `bind` value `build` writes into `composition.yaml` must correspond to a field on the matching per-page view struct described here. Surface field names in `snake_case` (`due_date`, `title_error`).
- **Event variants.** Every interactive item `build` writes into `composition.yaml` triggers an `Event` variant described here. Use `PascalCase` and document any payload (`ToggleTodo(id)`, `SaveTodo`).
- **Route variants.** Every navigation target the spec describes ("WHEN user taps add THEN the app navigates to the add todo form") is a `Route` variant. `Navigate(Route)` is the shell-facing entry point.
- **Capability matrix.** The `## Adapters` table tells `build` which capabilities to scaffold and which CAP-markers to expand.

When upstream `screenshots` Evidence contributed `region` / `container` / `leaf` claims, `design.md` summarises the resulting screen inventory and view-struct fields in prose — it never reproduces the raw layout tree (that is `composition.yaml`'s job at build time). Treat `design.md` as the **handshake between synthesis and build**: anything `build` needs to know to write code or `composition.yaml` must appear here (or in `spec.md`).

Naming conventions to keep `design.md` and the eventual `composition.yaml` aligned:

- Screen slugs: `kebab-case` (`todo-list`, `add-todo`).
- ViewModel variants: `PascalCase` derived from the slug (`TodoList`, `AddTodo`).
- Per-page view structs: variant + `View` suffix (`TodoListView`, `AddTodoView`).
- Field names: `snake_case` (`due_date`, `title_error`).
- Event names: `PascalCase` (`ToggleTodo`, `SaveTodo`, `Navigate`).

### `tasks.md` — execution sequencing

- Tasks are organised by **build phase**, not by feature. All features in the slice share one task list, ordered: core first, shells second.
- Each task references the domain's spec at `specs/<domain>/spec.md`. The spec contains both core requirements and the platform-specific requirements sections.
- Tokens / assets / layout work is **input context** for the shells (the shell writers read `tokens.yaml` / `assets.yaml` / regenerated `composition.yaml` directly) — never a separate task tier.
- Tasks must be **agent-completable** with code or local tooling. No manual mobile-app testing, no real-world API calls, no production credentials, no visual inspection, no physical-device-only checks, no app-store-review tasks. Express verification through fixture-backed tests, mocked effects, and local build commands available to the `build` brief.

## Operator-curated build inputs (never synthesised)

The synthesiser must **never** invent or restate the contents of these files in `spec.md` / `design.md`. They are operator-curated configuration that `build` consumes directly:

- **`tokens.yaml`** — concrete token values (colours, typography, spacing, radii, elevation). Source of truth for the design system; validated by the declared Vectis tool. Reference token names from `design.md` as policy notes (`the iOS shell falls back to system colors when this token is absent`); never enumerate the catalogue.
- **`assets.yaml`** — asset manifest (raster, vector, SF Symbols, Material icons) with per-platform source mappings. Reference assets by id with usage notes (`the empty-tasks hero is rendered at 2:1 aspect ratio`); never enumerate the manifest.

The synthesiser also never writes `composition.yaml`. The build brief regenerates it from the canonical artifacts above on every run.

## Capability gating cues for the synthesiser

When folding Evidence into `design.md`'s `## Adapters` table, the following cues map directly onto Vectis capabilities so `build` can scaffold the right CAP-markers without guesswork:

- HTTP requests, REST clients, GraphQL — `HTTP` adapter (`crux_http`).
- Local persistence, cached state across app launches — `Key-Value` adapter (`crux_kv`).
- Server-sent events, streaming notifications — `SSE` custom adapter (inline; not a published crate).
- Timers, scheduling, time-of-day display — `Time` adapter (`crux_time`).
- Platform detection (iOS vs Android vs Web) — `Platform` adapter (`crux_platform`).
- Rendering (always) — `Render` adapter.

State the capability set explicitly in `design.md`; the `build` brief feeds this directly into `specify tool run vectis -- scaffold core --caps <csv>` and into the per-shell `Effect` switch generation.

## Source-adapter contract (what the synthesiser may encounter)

A `target: vectis` slice typically draws on one or more of:

- **`intent`** — operator briefs and overrides (`authority: intent`).
- **`documentation`** — written product / technical intent (`authority: documentation`).
- **`screenshots`** — vision-assisted spatial inference producing `region` / `container` / `leaf` claims (`authority: documentation`). The spatial claims are how upstream UI evidence reaches the synthesiser; treat them as the structural backbone for screen-bearing requirements and view-struct fields.
- **`typescript`** (or any future code source) — behavioural evidence from a legacy implementation (`authority: behaviour`). Useful when migrating an existing TypeScript surface to Crux.

When sources disagree, follow the authority precedence (`intent > documentation > behaviour`) and resolution order defined in [`authority.md`](../references/spec-runtime/synthesis/authority.md) — Vectis does not override the global authority order.
