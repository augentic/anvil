# Identity password reset

## Motivation

Restate the password-reset surface as an Omnia crate so the identity service can issue reset links for registered users, governed by the operator-supplied identity design notes and informed by the legacy monolith's existing behaviour.

## Scope

- A reset-request handler that accepts an email payload, persists a reset token row, dispatches a reset email, and returns the same outward response regardless of whether the email is known.
- Reset-link expiry per the design notes' 30-minute window.

## Non-goals

- Migrating in-flight reset tokens from the legacy monolith — out of this slice.
- Changing the reset-email transport — the existing transactional email provider stays in place.
