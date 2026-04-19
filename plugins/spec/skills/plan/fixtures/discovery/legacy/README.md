# Legacy monolith — capability overview

This is the reference fixture tree used by
`plugins/spec/skills/plan/SKILL.md` §"Step 3(a) — Discovery" to pin
the shape of a single-`--source` capability inventory. It is a
deliberately small, hand-authored "legacy monolith" with three
modules; `/spec:extract` run against this directory should surface
the capabilities listed below in the accompanying
`../expected-discovery.md` golden.

## Modules

| Module          | Capabilities surfaced                                |
| --------------- | ---------------------------------------------------- |
| `src/user.rs`    | `registration`, `email_verification`                |
| `src/orders.rs`  | `cart_management`, `order_create`, `order_update`   |
| `src/payments.rs`| `checkout`, `payment_intent`                        |

## Capability dependencies

- `email_verification` depends on `registration` (the verification
  token is issued during sign-up).
- `cart_management` depends on `registration` (a cart is owned by a
  user row).
- `order_create` depends on `cart_management` (an order is
  materialised from a cart snapshot).
- `order_update` is independent — it modifies an existing order.
- `checkout` depends on `cart_management` (payment closes a cart).
- `payment_intent` depends on `checkout` (the intent is created
  from the checkout handler).

## Open questions (seeded deliberately)

- Should `email_verification` be a standalone migration slice or
  folded into `registration`?
- Is `payment_intent` a separate capability or an implementation
  detail of `checkout`?

These two questions round-trip into the `## Open questions` section
of the golden inventory so the fixture exercises that part of the
discovery output shape as well.
