# Artifact Format

This is the definitive reference for the structure and conventions of Specify artifacts. For a high-level overview, see [Artifacts in Depth](../explanation/artifacts.md).

## Contents

- [Spec files (behavioral "what")](#spec-files-behavioral-what)
- [Design document (technical "how")](#design-document-technical-how)
- [Proposal document](#proposal-document)
- [Tasks document](#tasks-document)
- [Decision Records (design "why")](#decision-records-design-why)
- [Composition document (Vectis only)](#composition-document-vectis-only)
- [Contract artifacts (API "shape")](#contract-artifacts-api-shape)
- [Validation checklists](#validation-checklists)

## Spec files (behavioral "what")

One spec file per domain, at `specs/<domain>/spec.md` (a *domain* is one cohesive area of behaviour — a crate, module, or service).

Specs are behavioral. They describe what the system must do, not how it should be implemented in a particular framework.

### Baseline / new domain format

New specs and merged baselines use a flat requirement format:

````markdown
# <Domain Name> Specification

## Purpose

<1-2 sentence description of what this domain does>

### Requirement: <Behavior Name>

ID: REQ-001
Sources: [<source-key>, …]
Status: <agreed|unknown|conflict|divergence>

The system SHALL <behavioral description>.

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

### Delta spec format (modified domain)

When modifying an existing domain, the synthesis kernel emits operation headers (`## ADDED Requirements`, `## MODIFIED Requirements`) so merge can apply changes. Agents set `baseline-id` in the synthesis response when refining an existing requirement; net-new behaviour in a modified domain is additive and receives the next id after the baseline max. Hand-authored flat requirement blocks against a non-empty baseline are rejected at merge (`merge-delta-headers-required`).

````markdown
# <Domain Name> Specification

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

For multi-domain changes, structure the document with per-domain sections (`## Crate: <name>` or equivalent) each containing the relevant subsections.

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

`proposal.md` captures why the slice exists and what is in scope. The workflow owns three required H2 sections, in order — target guidance prompts may add sections after them (e.g. Vectis `## Platforms`) but must not rename or replace these:

```markdown
## Why

<One to three paragraphs explaining why the slice exists.>

## Domains

- <domain-slug> — <target-specific meaning and short scope summary>

## Non-goals

- <Out-of-scope behavior or surface, when known>
```

- **`## Why`** is the motivation section `specify slice validate` checks (`proposal.why-has-content`).
- **`## Domains`** is the only section the validator uses to locate spec files: every bullet maps one-to-one to `specs/<domain>/spec.md`. Domain slugs are kebab-case. Targets interpret what a domain means (Vectis feature, Omnia crate/service surface, contracts contract surface) in their guidance prompts, but the section name and file layout are identical for every target.
- **`## Non-goals`** is optional content when known; the heading is still required.

No provenance lines on `proposal.md` — provenance lives in spec files after synthesis.

For agent-authored synthesis responses, see [`plugins/spec/references/synthesis/substeps.md`](../../plugins/spec/references/synthesis/substeps.md) section 1 for per-source authoring guidance.

Keep proposals concise (one to two pages). Focus on the "why" not the "how" — implementation details belong in the design.

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

Tasks are implemented by the active target adapter's `build` operation (`adapters/targets/<target>/prose/prompts/build.md`), which carries the specialist orchestration (crate / test / guest / review for omnia, core / shells / composition for vectis, format-dispatched author-import-verify for contracts) inline. Tasks do not route to standalone specialist skills.

## Decision Records (design "why")

A slice may author zero or more **Decision Records** at `.specify/slices/<name>/decisions/<slug>.md` -- the durable *why* behind a design choice plus the alternatives it rejected. Each record is a YAML front-matter header (the author writes only `slug` and `status: accepted | rejected`) followed by a [Nygard-shaped](https://github.com/joelparkerhenderson/architecture-decision-record) body:

````markdown
---
slug: token-store-backing
status: accepted
---

## Context

<the forces and constraints that make this decision necessary>

## Decision

<the choice that was made>

## Consequences

<what becomes easier or harder as a result>
````

Decision Records store the *why*, never design *state* -- domain models and API shapes stay in `design.md` and the code.

`/spec:merge` promotes each record into the append-only baseline catalogue at `.specify/decisions/DEC-NNNN-<slug>.md` by opaque whole-file add, assigning the durable, project-global `DEC-NNNN` id (the CLI never reuses an id). A newer record's `supersedes:` flips its named targets to `status: superseded`. Records are opt-in -- a slice that takes no notable decision authors none.

## Composition document (Vectis only)

`composition.yaml` describes the spatial layout of each screen, enriched with the wiring (`bind`, `event`, `maps_to`, overlay `trigger`, navigation, `*-when`) that connects layout to ViewModels and specs. It is a schema-validated YAML document regenerated by the Vectis target's `build` operation from `spec.md` + `design.md`. The JSON Schema is adapter-owned at [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json).

### Layout vs composition

The pre-define and post-define surfaces are two sibling artifacts that share the same JSON Schema:

- **`layout.yaml` (unwired layout input)** — regions, group hierarchy, gap / padding / align / size, token references, asset references, and the optional cross-shell `component: <slug>` directive, *without* the wiring keys above. Produced by layout inferers (the [`screenshots` source adapter](https://github.com/augentic/specify-adapters/blob/main/sources/screenshots/adapter.yaml) is the first-party producer; future Figma and source-code inferers reuse the same contract) or hand-authored. Validated by the vectis adapter's in-guest `validate layout` behaviour, which enforces the unwired-subset rule and the structural-identity rule.
- **`composition.yaml` (wired lifecycle artifact)** — the same regions enriched with the wiring keys above. Produced during synthesis (the composition prompt reads `layout.yaml` when present) and consumed by shell writers. Validated by the vectis adapter's in-guest `validate composition` behaviour, which auto-invokes `tokens` / `assets` modes when sibling manifests exist.

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

For the full schema definition and item vocabulary, see the [Vectis composition schema](https://schemas.specify.dev/vectis/composition.schema.json).

## Contract artifacts (API "shape")

Contract artifacts capture the machine-readable shapes of APIs and message interfaces. They use three standard formats, each with its own subdirectory under `contracts/`:

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

- One spec file per domain
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
