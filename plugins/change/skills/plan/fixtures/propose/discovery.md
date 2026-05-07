# Discovery — platform-v2

## Capability inventory

### user-registration
Source: monolith (/path/to/legacy-codebase) Description: User sign-up flow; creates a new user record.

### email-verification
Source: monolith (/path/to/legacy-codebase) Description: Verifies user email via a one-time link. Depends-on hints: user-registration

### product-catalog
Source: monolith (/path/to/legacy-codebase) Description: Browse and search the product catalogue.

### cart-management
Source: orders (git@github.com:org/orders-service.git) Description: Add/remove items from the user's cart. Depends-on hints: user-registration

### checkout
Source: payments (git@github.com:org/payments-service.git) Description: Completes payment for a cart. Depends-on hints: cart-management

## Open questions

- Should `email-verification` stay a separate plan entry or fold into `user-registration`?
- Does the new `shopping-cart` crate need to absorb the legacy `cart-management` *and* `order-create` logic, or is `order-create` a follow-up slice?
