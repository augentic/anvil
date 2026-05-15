# Discovery — demo

## Capability inventory

### user-registration
Source: legacy (src/user.rs) Description: User sign-up flow; creates a new user record.

### email-verification
Source: legacy (src/user.rs) Description: Verifies user email via a one-time link. Depends-on hints: user-registration

### order-create
Source: legacy (src/orders.rs) Description: Creates a new order from the cart. Depends-on hints: cart-management

### order-update
Source: legacy (src/orders.rs) Description: Modifies an existing order.

### cart-management
Source: legacy (src/orders.rs) Description: Add/remove items from the user's cart. Depends-on hints: user-registration

### checkout
Source: legacy (src/payments.rs) Description: Completes payment for a cart. Depends-on hints: cart-management

### payment-intent
Source: legacy (src/payments.rs) Description: Creates a payment authorisation against the gateway. Depends-on hints: checkout

## Open questions

- Should email-verification be a separate feature or folded into user-registration?
- Is payment-intent a standalone capability or an implementation detail of checkout?
