# Specify artifact validation checklist

Runtime checklist for source adapters and synthesis self-review. Full artifact reference: [Artifact format](https://specify.augentic.io/reference/artifact-format.html).

## Behavioral specs

- One spec file per domain
- Each spec has Purpose, flat Requirement blocks, stable `ID: REQ-XXX` lines, Scenarios, and Error Conditions
- Specs stay behavioral and avoid platform-binding detail
- Traceability is present for each requirement via stable IDs

## Technical design

- `design.md` captures domain model, APIs, business logic, integrations, and configuration
- Unknowns are marked explicitly with unknown tokens
- Technical decisions live in design, not in specs

## Tasks

- `tasks.md` exists when `/spec:build` depends on it
- Tasks are implementation steps and checkpoints only
- Every task uses numbered checkbox format (`- [ ] X.Y …`) grouped under `## N.` headings

## Composition (Vectis only)

- `composition.yaml` conforms to the Vectis composition JSON Schema
- Screen slugs are kebab-case
- Every per-page view struct field has a `bind` on some item (composition.yaml only)
- Every shell-facing Event has an `event` wiring (composition.yaml only)
- `maps_to` values reference declared ViewModel variants from the design (composition.yaml only)
- Overlay `trigger` values match an `event` name in the same screen
- `Navigate(X)` targets have corresponding screen slugs and Route variants

## API contracts

- Every JSON Schema file has `$id`, `title`, and `description`
- `$id` values use the `urn:specify:schemas/<name>` format
- One type per schema file
- All `$ref` pointers in OpenAPI and AsyncAPI files resolve to existing schema files
- Request/response schemas in OpenAPI bindings use `$ref` to `../schemas/`, not inline definitions
- Message payload schemas in AsyncAPI bindings use `$ref` to `../schemas/`
- Every schema that appears as a top-level payload in a spec scenario has at least one protocol binding
- File names use kebab-case with `.yaml` extensions
- Contract files capture interface shape only; auth, rate limits, and retry policies remain in `design.md`
