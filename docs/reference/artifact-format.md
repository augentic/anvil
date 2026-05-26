# Artifact Format

This is the definitive reference for the structure and conventions of Specify artifacts. For a high-level overview, see [Artifacts in Depth](../explanation/artifacts.md).

## Contents

- [Spec files (behavioral "what")](#spec-files-behavioral-what)
- [Design document (technical "how")](#design-document-technical-how)
- [Proposal document](#proposal-document)
- [Tasks document](#tasks-document)
- [Composition document (Vectis only)](#composition-document-vectis-only)
- [Contract artifacts (API "shape")](#contract-artifacts-api-shape)
- [Validation checklists](#validation-checklists)

## Spec files (behavioral "what")

One spec file per adapter, at `specs/<name>/spec.md`.

Specs are behavioral. They describe what the system must do, not how it should be implemented in a particular framework.

### Baseline / new adapter format

New specs and merged baselines use a flat requirement format:

````markdown
# <Adapter Name> Specification

## Purpose

<1-2 sentence description of what this adapter does>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.
Source: <source function or design section>

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>

## Metrics

- `<metric_name>` -- type: <counter|gauge|histogram>; emitted: <when>
````

Key rules:

- Each requirement has a stable `ID: REQ-XXX` line. This is the merge key used during `/spec:merge`.
- Scenarios use WHEN/THEN format. GIVEN is optional context.
- Error conditions are listed at the end as a cross-cutting summary.
- Metrics are only included when they are explicit in the source material.

### Delta spec format (modified adapter)

When modifying an existing adapter, specs use operation headers to describe what changed:

````markdown
# <Adapter Name> Specification

## Purpose

<updated purpose if changed>

## ADDED Requirements

### Requirement: <New Behavior>

ID: REQ-005

The system SHALL <new behavioral description>.

#### Scenario: <Happy Path>
...

## MODIFIED Requirements

### Requirement: <Changed Behavior>

ID: REQ-002

The system SHALL <updated behavioral description>.

#### Scenario: <Updated Scenario>
...

## REMOVED Requirements

### Requirement: <Removed Behavior>

ID: REQ-003

Reason: <why this requirement is being removed>

## RENAMED Requirements

### Requirement: <New Name> (was: <Old Name>)

ID: REQ-004
````

The four operation sections are `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`, and `## RENAMED Requirements`. All are optional -- include only the sections relevant to the slice.

The stable `ID: REQ-XXX` line is the merge key, not the requirement title. Requirement titles may change across deltas; IDs must not.

## Design document (technical "how")

`design.md` carries the technical shape needed to implement the slice.

### Format

````markdown
# Technical Design

## Context

- Source: <TypeScript component path | design document>
- Purpose: <component or change summary>
- Source paths: <analyzed files, if applicable>

## Domain Model

## API Contracts

<!-- When contracts/http/ exists: reference the OpenAPI specifications
     rather than re-describing endpoint shapes. Add implementation-level notes
     not captured in the contract: auth schemes, rate limits, caching, versioning. -->

## External Services

## Constants & Configuration

## Business Logic

## Publication & Timing Patterns

<!-- When contracts/messages/ exists: reference the AsyncAPI specifications
     rather than re-describing message shapes. Add implementation-level notes
     not captured in the contract: ordering guarantees, retry policies, DLQ strategy. -->

## Implementation Constraints

## Source Capabilities Summary

## Dependencies

## Risks / Open Questions

## Notes
````

### When to create a full design

Create a full design if any of the following apply:

- Cross-cutting change (multiple services or modules) or new architectural pattern
- New external dependency or significant data model changes
- Security, performance, or migration complexity
- Ambiguity that benefits from technical decisions before coding

If none apply, create a minimal `design.md` noting that a full design is not warranted and referencing the proposal and specs.

For multi-adapter changes, structure the document with adapter-specific sections (`## Crate: <name>` or equivalent) each containing the relevant subsections.

### Business logic tags

Tags classify business logic blocks in the design:

| Tag | Use case | Example |
|-----|----------|---------|
| `[domain]` | Business rules and validation | Validate order total matches line items |
| `[infrastructure]` | External calls or persistence | Fetch data from API or publish to queue |
| `[mechanical]` | Data transformations | Parse JSON or map fields |
| `[unknown]` | Explicitly unresolved detail | Dependency behavior not specified |

### Unknown tokens

Use explicit unknown markers instead of guessing:

| Token | When to use |
|-------|-------------|
| `unknown -- not specified in source` | The source material does not say |
| `unknown -- ambiguous requirement` | The requirement is unclear or conflicting |
| `unknown -- inferred from context` | Best-effort summary, not explicitly stated |
| `unknown -- open question` | The design material marks it as unresolved |

### Design ownership

The design document is where project-specific technical detail belongs: domain models, type shapes, API contracts, message shapes, business logic, external integrations, configuration, risks, and migration notes. Generator-owned binding decisions (e.g. Omnia trait composition, Crux effect types) remain in specialist skills and references.

When design sections reference behavior from specs, cite the stable requirement IDs (e.g. `REQ-003`) rather than relying on requirement titles.

## Proposal document

`proposal.md` captures why the slice exists and what is in scope. The schema's brief file provides the full output template.

The **Crates** (or **Adapters**) section creates the contract between proposal and specs phases -- each adapter listed will need a corresponding spec file at `specs/<name>/spec.md`.

Keep proposals concise (one to two pages). Focus on the "why" not the "how" -- implementation details belong in the design.

## Tasks document

`tasks.md` is an implementation checklist, not a requirements document.

### Format

```markdown
## 1. Implementation

- [ ] 1.1 Refine proposal/spec/design artifacts
- [ ] 1.2 Implement code changes via `/spec:build`
- [ ] 1.3 Verify and review output
```

### Rules

- Group related tasks under `##` numbered headings.
- Each task MUST be a checkbox: `- [ ] X.Y Task description`. The build phase parses this format to track progress.
- Tasks should be small enough to complete in one session.
- Order tasks by dependency (what must be done first).
- Reference specs for what needs to be built, design for how to build it.
- Each task should be verifiable -- you know when it is done.

### Skill directive tags

Tasks may include a skill directive as an HTML comment. The build phase parses these tags and delegates the task to the named specialist skill:

```markdown
- [ ] 2.1 Generate the domain crate <!-- skill: omnia:crate-writer -->
- [ ] 2.2 Generate test suites <!-- skill: omnia:test-writer -->
- [ ] 2.3 Manual integration step
```

Tasks without a skill tag are implemented via the adapter's default build instruction.

## Composition document (Vectis only)

`composition.yaml` describes the spatial layout of each screen, enriched with the wiring (`bind`, `event`, `maps_to`, overlay `trigger`, navigation, `*-when`) that connects layout to ViewModels and specs. It is a schema-validated YAML document regenerated by the Vectis target's `build` brief from `spec.md` + `design.md`. The JSON Schema is tool-owned at [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json) (retrieve with `specrun tool schema vectis composition`). (See [Decision Log: Composition as a separate artifact](../explanation/decision-log.md#composition-as-a-separate-artifact-not-embedded-in-specs-or-design) for the rationale.)

### Layout vs composition

The pre-define and post-define surfaces are two sibling artifacts that share the same JSON Schema:

- **`layout.yaml` (unwired layout input)** — regions, group hierarchy, gap / padding / align / size, token references, asset references, and the optional cross-shell `component: <slug>` directive, *without* the wiring keys above. Produced by layout inferers (the [`screenshots` source adapter](../../adapters/sources/screenshots/adapter.yaml) is the first-party producer; future Figma and source-code inferers reuse the same contract) or hand-authored. Validated by `specrun tool run vectis -- validate layout`, which enforces the unwired-subset rule and the structural-identity rule.
- **`composition.yaml` (wired lifecycle artifact)** — the same regions enriched with the wiring keys above. Produced by the define pipeline (the composition brief reads `layout.yaml` when present) and consumed by shell writers. Validated by `specrun tool run vectis -- validate composition`, which auto-invokes `tokens` / `assets` modes when sibling manifests exist.

### Format

```yaml
version: 1

provenance:              # optional: where this layout came from
  sources:
    - kind: manual       # figma | legacy | manual | screenshots | code

screens:
  <screen-slug>:         # kebab-case screen identifier
    name: "Screen Name"
    maps_to: "ViewModel::ScreenName(ScreenNameView)"  # composition.yaml only — not in layout.yaml

    header:
      title: "Title"
      leading: [...]     # left-side items (back button, menu)
      trailing: [...]    # right-side items (badges, actions)

    body:                # one of: list, grid, form, or content node array
      list:
        each: <field>
        item: [...]

    footer: [...]        # bottom bar items

    fab: { icon: plus, event: Navigate(AddTodo) }

    states:
      <state-slug>:
        when: "<field> is <true|false|empty|not empty>"
        body: [...]

    overlays:
      <overlay-slug>:
        kind: dialog | sheet | snackbar
        trigger: <EventName>
        content: [...]

    platforms:            # per-platform region overrides (optional)
      ios:
        body: { ... }
```

### Regions

Each screen is divided into named regions that map to platform-native screen structure:

| Region | Description | iOS | Android |
|--------|-------------|-----|---------|
| `header` | Top navigation bar | `NavigationTitle` + toolbar | `TopAppBar` |
| `body` | Main content (list, grid, form, or items) | Content view | Scaffold `content` |
| `footer` | Bottom bar | `TabView` or toolbar | `BottomAppBar` |
| `fab` | Floating action button | `.overlay` / `ZStack` | `FloatingActionButton` |

### Groups and items

Content within regions is a tree of **items** (leaf elements like `text`, `button`, `field`) and **groups** (container nodes with flexbox-like layout properties). Groups carry `direction` (row/column/stack), `gap`, `padding`, `align`, `justify`, optional `size`, and optional surface decoration (`background`, `corner_radius`, `elevation`). These map to SwiftUI stacks, Compose Row/Column/Box, and CSS Flexbox.

### Delta format

Per-change composition artifacts use a `delta` key (not `screens`) with `added`, `modified`, and `removed` sub-keys. Delta operations are screen-level -- `modified` replaces the entire screen entry, not individual regions.

```yaml
version: 1

delta:
  added:
    new-screen: { name: "New Screen", ... }
  modified:
    existing-screen: { name: "Existing Screen", ... }
  removed:
    old-screen: { reason: "Replaced by new-screen" }
```

### Key rules

- A document has either `screens` (baseline) or `delta` (per-slice), never both.
- Screen slugs are kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).
- Every field in a per-page view struct should appear as a `bind` value (composition.yaml only).
- Every shell-facing Event should have an `event` wiring (composition.yaml only).
- `event` values follow PascalCase: `EventName` or `EventName(arg1, arg2)`.

For the full schema definition and item vocabulary, see the [Vectis composition schema](https://schemas.specify.dev/vectis/composition.schema.json) (retrieve with `specrun tool schema vectis composition`).

## Contract artifacts (API "shape")

Contract artifacts capture the machine-readable shapes of APIs and message interfaces. (See [Decision Log: Contracts as platform-level artifacts](../explanation/decision-log.md#contracts-as-platform-level-artifacts-not-per-project) for the rationale.) They use three standard formats, each with its own subdirectory under `contracts/`:

### Directory structure

```text
contracts/
├── schemas/                # JSON Schema payload definitions (always present)
│   ├── user-registration.yaml
│   ├── error-response.yaml
│   └── order-placed.yaml
├── http/                   # OpenAPI 3.1 bindings (when HTTP is used)
│   └── user-api.yaml       # $ref → ../schemas/
└── messages/               # AsyncAPI 3.0 bindings (when messaging is used)
    └── order-events.yaml    # $ref → ../schemas/
```

### Naming conventions

- **All files** use kebab-case names with `.yaml` extensions.
- **Schema files** are named after the domain type: `user-registration.yaml`, `error-response.yaml`. One type per file.
- **HTTP binding files** are named after the API domain: `user-api.yaml`, `billing-api.yaml`.
- **Message binding files** are named after the event domain: `order-events.yaml`, `notification-events.yaml`.

### JSON Schema files

Each schema file defines a single domain type:

```yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
$id: "urn:specify:schemas/user-registration"
title: "UserRegistration"
description: "Payload for creating a new user account"
type: object
properties:
  email:
    type: string
    format: email
  password:
    type: string
required: [email, password]
```

Key fields:

- **`$id`** -- stable URI in the format `urn:specify:schemas/<filename-without-extension>`. Once assigned, must not change.
- **`title`** -- PascalCase type name matching the domain concept.
- **`description`** -- concise sentence describing the type's role.
- **`$ref`** -- shared sub-types reference other schema files rather than inlining definitions.

### OpenAPI bindings

OpenAPI 3.1 files define HTTP endpoint bindings. All request/response schemas use `$ref` pointers to `../schemas/` -- type definitions are never inlined in binding files.

### AsyncAPI bindings

AsyncAPI 3.0 files define messaging bindings. Message payload schemas use `$ref` pointers to `../schemas/`, following the same shared-schema principle as OpenAPI bindings.

### Scope boundary

Contracts capture *interface shape* -- endpoint paths, methods, payload schemas, error codes, channel names, message structures. Everything else belongs in `design.md`: authentication schemes, rate limits, retry policies, caching strategies, versioning approaches, and ordering guarantees. If a concern affects wire compatibility, it belongs in the contract; if it affects operational behavior, it belongs in the design.

### Delta semantics

Contract files use **opaque replacement** semantics during merge -- unlike spec files which use the ADDED/MODIFIED/REMOVED delta format, contract files are replaced wholesale. When a slice modifies an existing binding (e.g. adding new endpoints to `user-api.yaml`), the delta file must include both existing and new paths -- the merge replaces the baseline file entirely.

Contract deletion is rare and handled as a manual baseline edit. The slice-level directory can express additions and replacements but not deletions.

## Validation checklists

### Behavioral specs

- One spec file per adapter
- Each spec has Purpose, flat Requirement blocks, stable `ID: REQ-XXX` lines, Scenarios, and Error Conditions
- Specs stay behavioral and avoid platform-binding detail
- Traceability is present for each requirement via stable IDs

### Technical design

- `design.md` captures domain model, APIs, business logic, integrations, and configuration
- Unknowns are marked explicitly with unknown tokens
- Technical decisions live in design, not in specs

### Tasks

- `tasks.md` exists when `/spec:build` depends on it
- Tasks are implementation steps and checkpoints only
- Every task uses checkbox format (`- [ ]`)

### Composition (Vectis only)

- `composition.yaml` conforms to the JSON Schema at [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json)
- Screen slugs are kebab-case
- Every per-page view struct field has a `bind` on some item (composition.yaml only)
- Every shell-facing Event has an `event` wiring (composition.yaml only)
- `maps_to` values reference declared ViewModel variants from the design (composition.yaml only)
- Overlay `trigger` values match an `event` name in the same screen
- `Navigate(X)` targets have corresponding screen slugs and Route variants

### API contracts

- Every JSON Schema file has `$id`, `title`, and `description`
- `$id` values use the `urn:specify:schemas/<name>` format
- One type per schema file
- All `$ref` pointers in OpenAPI and AsyncAPI files resolve to existing schema files
- Request/response schemas in OpenAPI bindings use `$ref` to `../schemas/`, not inline definitions
- Message payload schemas in AsyncAPI bindings use `$ref` to `../schemas/`
- Every schema that appears as a top-level payload in a spec scenario has at least one protocol binding
- File names use kebab-case with `.yaml` extensions
- Contract files capture interface shape only; auth, rate limits, and retry policies remain in `design.md`
