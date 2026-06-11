# Identity password reset expiry

## Why

Settle the expiry window for password-reset links so the identity service can honour a single, operator-confirmed value.

## Domains

- password-reset-expiry — password-reset token TTL configuration

## Non-goals

- Reissue or rotation of in-flight reset tokens.
- Changing the reset-email transport.
