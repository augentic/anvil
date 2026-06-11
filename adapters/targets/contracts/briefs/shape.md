# contracts.shape

Idiom guidance core synthesis (`/spec:refine`) folds into the generated `specs/<domain>/spec.md` and `design.md` for slices that target the `contracts` adapter. The contracts target produces **contract artifacts** — machine-readable interface definitions under `contracts/` — not application code, so the synthesised specs and `design.md` shape themselves around the contract's behavioural surface and its persisted format, not around runtime providers or DI patterns.

## Contract domains

Each `proposal.md ## Domains` bullet names one contract surface — an HTTP API domain, an event family, or a schema vocabulary scope — and maps one-to-one to `specs/<domain>/spec.md`. For a single HTTP API the domain is typically the API domain slug (e.g. `billing-api`); for a mixed-format slice the domain is the overarching contract surface (e.g. `order-lifecycle` when it spans both HTTP endpoints and event channels). Target briefs may describe what a domain means for the contracts target, but they do not rename `## Domains` or move spec files out of the canonical `specs/` layout.

## What core synthesises for a contracts slice

For a contracts target, the canonical artifacts answer two questions:

- `specs/<domain>/spec.md` — **what the contract promises**. Requirements capture endpoints, channels, payloads, error responses, status codes, message kinds, and acceptance rules that consumers can rely on. Each requirement carries the standard `ID:` / `Sources:` / `Status:` provenance lines; `Sources:` typically resolves to one or more `intent`, `documentation`, or code-source bindings depending on whether the slice authors a new contract or imports an existing one.
- `design.md` — **how the contract is expressed and constrained**. Records the contract format the slice will produce (OpenAPI 3.1, AsyncAPI 3.0, JSON Schema), the file layout under `contracts/`, any cross-contract dependencies (schemas reused across HTTP and evented surfaces), and the validation rules the merge gate enforces. No application-side DI patterns, runtime providers, or implementation crates belong here — those are Omnia / Vectis target concerns.

## Contract format selection

The contracts target dispatches at `build` time to one of three contract sub-types based on the slice's needs. Core synthesis should make the format choice explicit in `design.md` so `/spec:build` can route the work without re-deriving the decision:

| Contract surface | Format | `design.md` should note |
|------------------|--------|--------------------------|
| HTTP / resource-style endpoints | OpenAPI 3.1 | Files land under `contracts/http/<api-domain>.yaml`. |
| Evented / pub-sub / streaming / WebSocket | AsyncAPI 3.0 | Files land under `contracts/messages/<event-domain>-events.yaml`. |
| Reusable payload schemas referenced by HTTP or evented surfaces | JSON Schema (Draft 2020-12) | Files land under `contracts/schemas/<type>.yaml`; one named type per file; URN `$id`. |

A slice MAY produce more than one format (e.g. a JSON Schema for payload shapes plus an OpenAPI document that `$ref`s into it). When the slice's surface mixes formats, `design.md` should list every format and the shared-schema reuse expectations. The three sub-types are **not** separate target adapters — they are intent-dispatched inside the single `build` brief.

## Validation rules synthesis must surface

The merge gate runs `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` against the merged baseline. The tool enforces rules the contract authors have to design around; `design.md` should reflect them so a slice's contract artifacts ship merge-ready:

- **SemVer `info.version`.** Every top-level OpenAPI / AsyncAPI document under `contracts/` must set `info.version` to a value that parses as SemVer (per [semver.org](https://semver.org)), including optional prerelease labels. New contracts pick an initial version (typically `0.1.0` or `1.0.0`); revisions bump major / minor / patch per the consumer-impact rules under `references/cross-project-compatibility.md`.
- **Kebab-case `info.x-specify-id`.** When a top-level contract document carries `info.x-specify-id`, the value matches `^[a-z][a-z0-9-]*$` and is ≤ 64 characters. It is a rename-stable hint that survives file moves and version bumps. New contracts SHOULD set it (typically to the file stem); imports preserve any source-supplied value verbatim.
- **Cross-repo `x-specify-id` uniqueness.** Every present `info.x-specify-id` is unique across the entire baseline `contracts/` directory. Synthesis cannot enforce this at refine time (the baseline state is not in scope yet), but `design.md` should note that the operator picks an `x-specify-id` distinct from any sibling contract in the same project.

## Source-driven authoring vs import

The contracts target supports both contract-first authoring (specs drive a new contract) and contract-given import (operator supplies an external OpenAPI / AsyncAPI / JSON Schema file and the build step normalises it). The `Sources:` provenance on each requirement in `specs/<domain>/spec.md` tells `/spec:build` which path applies — a `documentation` or `intent` source typically signals authoring; a code-source binding (`typescript`, future contract source adapters) typically signals import or reverse-engineering from observed behaviour.

When a slice is import-driven, `design.md` should name the supplied file path and the format detected. The `build` brief's format sub-flows handle version detection and upgrades (Swagger 2.0 → OpenAPI 3.1, AsyncAPI 2.x → AsyncAPI 3.0, JSON Schema draft-04/06/07/2019-09 → 2020-12) per `references/import-upgrade-policy.md`.

## What synthesis MUST NOT do

- Do not emit application-layer guidance (provider traits, error variants, crate layout, runtime sandbox notes). Those belong to Omnia / Vectis target shapes, not contracts.
- Do not write contract YAML inline into `specs/<domain>/spec.md` or `design.md`. The contract files themselves are `/spec:build`'s output, landing under `contracts/`.
- Do not invent endpoints, channels, or schemas the Evidence does not justify. Mark gaps with `[unknown]` per the standard synthesis rules and let the operator fill them in before build.
