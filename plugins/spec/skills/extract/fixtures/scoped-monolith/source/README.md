# scoped-monolith

A two-adapter toy monolith:

- `src/a/handler.ts` — adapter **a**: classifies an order by `amount`.
- `src/b/handler.ts` — adapter **b**: computes a loyalty tier from a customer id.
- `src/common/util.ts` — cross-cutting helpers.

Used as an `/spec:extract include 'src/a/**'` walk-through fixture.
