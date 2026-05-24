# Identity password reset expiry — design

## Domain model

- `ResetExpiryMinutes` newtype wrapping the configured TTL in minutes.

## Provider trait dependencies

- The expiry-emitting handler depends on `Config` (read of the TTL key).

## APIs and integrations

- No external surface in this slice — the TTL value is read by the password-reset handler authored elsewhere.

## Configuration

- `PASSWORD_RESET_EXPIRY_MINUTES` — `Config::get` key for the TTL value. The numeric default is unresolved while REQ-001 is `[conflict]`; downstream code must read the value through `Config::get` rather than hard-coding the literal.

## Handler delegation

- Not applicable; this slice settles configuration only.

## Error mapping

- `Error::ServerError` if `Config::get` returns an unparseable value at runtime.

## Validation placement

- Structural validation at config-read time: the value must parse as a positive integer.

## Observability

- `tracing::info!(gauge.password_reset_expiry_minutes = <value>)` once at first handler invocation per process.
