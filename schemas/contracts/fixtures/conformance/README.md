# Conformance Fixture — Contract-First Pattern

Demonstrates the `/interfaces:openapi` author and verifier behavior when baseline contracts already exist (the recommended contract-first workflow). The HTTP/resource specs in this fixture select `/interfaces:openapi` as the primary format skill; its author intent validates alignment between the change's specs and the baseline contracts and produces a small or empty delta.

## Scenario

An order processing API where the baseline already defines the contract (schemas + OpenAPI binding). An implementation change writes specs that conform to the existing contract. The writer validates alignment and produces no delta — the specs match the baseline.

## Inputs

- `baseline/contracts/` — pre-existing baseline contracts (schemas + OpenAPI)
- `specs/order-processing/spec.md` — behavioral spec written to conform to the baseline contract

## Expected Outputs

- Empty delta (no new contract files generated)
- Alignment report showing all spec interactions covered by baseline

## Expected Verifier Output

Clean pass — nothing to verify since the delta is empty.
