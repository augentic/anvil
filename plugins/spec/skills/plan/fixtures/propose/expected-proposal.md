# Proposal — platform-v2

## Slices

| # | Slice                     | Source(s) | Depends on                        | Decision      | Plan entry         |
|---|---------------------------|-----------|-----------------------------------|---------------|--------------------|
| 1 | user-registration         | monolith  | —                                 | accept        | user-registration  |
| 2 | email-verification        | monolith  | user-registration                 | accept        | email-verification |
| 3 | extract-shared-validation | —         | email-verification                | reject        | —                  |
| 4 | product-catalog           | monolith  | —                                 | accept        | product-catalog    |
| 5 | cart-management           | orders    | user-registration                 | edit → accept | shopping-cart      |
| 6 | checkout                  | payments  | cart-management                   | edit → accept | checkout-api       |

## Notes

- Heuristics applied (Omnia, from `schemas/omnia/briefs/plan/propose.md`):
  one plan entry per WASM crate or cohesive handler group; leaf
  services first; cross-cutting refactors land as standalone
  entries with explicit `depends-on` edges, presented *before* the
  feature slices that depend on them so a reject only trims edges
  from upcoming drafts (never from already-written entries).
- Slice 3 (`extract-shared-validation`) was proposed by the Omnia
  cross-cutting heuristic after observing the overlap between
  `user-registration` input sanitisation and `email-verification`
  token parsing. Rejected for this initiative — the operator
  preferred to defer the refactor until `checkout-api` lands and
  the full validation surface is visible. Slice 4 (`product-catalog`)
  and slice 5 (`cart-management` draft) therefore had the implicit
  `depends-on: [extract-shared-validation]` edge dropped from their
  drafts before they were presented.
- Slice 5 renamed from `cart-management` (discovery's source-side
  name) to `shopping-cart` (the target Omnia crate name) and
  gained `depends-on: [product-catalog]` during edit.
- Slice 6 renamed from `checkout` to `checkout-api` (matching the
  Omnia WASM crate name) and had its `depends-on` rewritten from
  `cart-management` to `shopping-cart` to follow slice 5's rename.
- Open questions from discovery answered inline:
  - *Should `email-verification` stay a separate plan entry?* —
    yes, kept separate (slice 2).
  - *Does the new `shopping-cart` crate need to absorb `order-create`?*
    — no, `order-create` is deferred to a follow-up plan.
- `specify plan validate` — no errors.
