# Identity password reset

## Why

Restate the password-reset surface as an Omnia crate so the identity service can issue reset links for registered users, governed by the operator-supplied identity design notes and informed by the legacy monolith's existing behaviour.

## Units

- password-reset — reset-request handler that accepts an email payload, persists a reset token row, dispatches a reset email, and returns the same outward response regardless of whether the email is known

## Non-goals

- Migrating in-flight reset tokens from the legacy monolith — out of this slice.
- Changing the reset-email transport — the existing transactional email provider stays in place.
