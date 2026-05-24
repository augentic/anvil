# Identity user registration

## Motivation

Restate the legacy user-registration surface as an Omnia crate so the identity service can accept new user registrations with the same validation behaviour the legacy monolith exposes today, governed by the operator-supplied identity design notes.

## Scope

- A registration handler that accepts an email + password payload, validates the email per RFC-5322, and persists the user.
- Persistence reuses the existing identity-store table; no new user store is introduced (per the design-notes decision).

## Non-goals

- Migrating existing users from the legacy monolith — out of this slice.
- Adding a new notification or welcome-email path on registration — not in the legacy surface and not requested by design notes.
