# Design — adapter `a` (scoped extract)

## Context

- **Source component**: `plugins/spec/skills/extract/fixtures/scoped-monolith/source`
- **Scope filter**: `--include 'src/a/**'`
- **Target runtime**: language-agnostic (extracted as an intermediate artifact)
- **Source language**: TypeScript (detected from `package.json`, `*.ts` extensions)
- **Source files analyzed**:
  - `src/a/handler.ts` — in scope
  - `package.json` — sentinel, language + dependency detection only
  - `README.md` (top-level) — sentinel
- **Out of scope (not analyzed)**:
  - `src/b/handler.ts`
  - `src/common/util.ts` (referenced by `src/a/handler.ts` via `import { nonEmpty } from "../common/util"`; flagged as `[unknown]` because the util module is outside the filtered read set)

## Domain Model

| Field      | Type                             | Optional? |
| ---------- | -------------------------------- | --------- |
| `orderId`  | `string`                         | no        |
| `amount`   | `number`                         | no        |
| `currency` | `string`                         | no        |

| Field     | Type                             | Optional? |
| --------- | -------------------------------- | --------- |
| `orderId` | `string`                         | no        |
| `class`   | `"small" \| "medium" \| "large"` | no        |

## Structures

- `classifyOrder(input: OrderInput): OrderClassification` — exported from `src/a/handler.ts`.
- Imports: `nonEmpty` from `../common/util` — behaviour `[unknown]` (outside scope).

## API Contracts

None exposed in scope. `classifyOrder` is a library function — no HTTP / RPC surface visible under `src/a/**`.

## External Services

None in scope.

## Constants & Configuration

- `SMALL_MAX` — source: hardcoded; value: `50`; semantics: upper bound (inclusive) for the `small` class.
- `MEDIUM_MAX` — source: hardcoded; value: `500`; semantics: upper bound (inclusive) for the `medium` class.

## Business Logic

### `classifyOrder(input: OrderInput): OrderClassification`

- **Execution mode**: synchronous.
- **Algorithm**:
  1. [domain] If `input.orderId` fails `nonEmpty` (`[unknown]` — util out of scope), throw `"orderId is required"`.
  2. [domain] If `input.amount < 0`, throw `"amount must be non-negative"`.
  3. [domain] If `input.amount <= SMALL_MAX` (50), set `class = "small"`.
  4. [domain] Else if `input.amount <= MEDIUM_MAX` (500), set `class = "medium"`.
  5. [domain] Else set `class = "large"`.
  6. [mechanical] Return `{ orderId: input.orderId, class }`.
- **Errors raised**: `Error("orderId is required")`, `Error("amount must be non-negative")`.
- **Unknowns**: exact `nonEmpty` semantics (out of scope).

## Dependencies

| Package      | Manifest specifier | Lock version | Kind   |
| ------------ | ------------------ | ------------ | ------ |
| `zod`        | `3.23.8`           | `[unknown]`  | direct |
| `typescript` | `5.4.5`            | `[unknown]`  | dev    |

> Lock file not present in fixture; `[unknown]` flagged per SKILL.md guidance.

## Risks / Open Questions

- `nonEmpty` behaviour is `[unknown]` because `src/common/util.ts` was excluded by the scope filter. If the caller needs the util's semantics captured, widen scope to `--include 'src/a/**' --include 'src/common/**'` or use a manifest.

## Notes

Illustrative output for the `scoped-monolith` walk-through. Not a byte-for-byte reproducibility target.
