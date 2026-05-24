# Identity password reset expiry

## Motivation

Settle the expiry window for password-reset links so the identity service can honour a single, operator-confirmed value.

## Scope

- Configure the password-reset token TTL.

## Non-goals

- Reissue or rotation of in-flight reset tokens.
- Changing the reset-email transport.
