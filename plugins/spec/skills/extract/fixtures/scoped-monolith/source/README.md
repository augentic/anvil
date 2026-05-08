# scoped-monolith

A two-capability toy monolith:

- `src/a/handler.ts` — capability **a**: classifies an order by `amount`.
- `src/b/handler.ts` — capability **b**: computes a loyalty tier from a customer id.
- `src/common/util.ts` — cross-cutting helpers.

Used as an `/spec:extract include 'src/a/**'` walk-through fixture.
