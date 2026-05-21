# Identity password reset — design

## Domain model

- `EmailAddress` newtype validated at parse time.
- `ResetToken` newtype wrapping the opaque token persisted alongside the user id, the issued-at timestamp, and a `expires_at` derived field.

## Provider trait dependencies

- `request_password_reset` handler depends on `TableStore` (read for the user row, write for the reset token row), `Publish` (dispatch the reset email), and `Config` (expiry window, token table name, email topic).

## APIs and integrations

- HTTP `POST /password-reset` — request handler; request body `{ email }`, response `202` in both branches so unknown emails cannot be enumerated.
- Internal: handler publishes a `password-reset.requested` message carrying the persisted token id; the email dispatcher subscribes elsewhere.

## Configuration

- `PASSWORD_RESET_TOKEN_TABLE_NAME` — `Config::get` key for the token table.
- `PASSWORD_RESET_EXPIRY_MINUTES` — `Config::get` key for the expiry window; default `30` per the design-notes requirement.
- `PASSWORD_RESET_EMAIL_TOPIC` — `Config::get` key for the email-dispatch topic.

## Handler delegation

- `RequestPasswordResetRequest` implements `Handler<P>`; `from_input` parses and validates the email; `handle` looks up the user, persists the token when found, publishes the email-dispatch event, and returns `202` in both branches.

## Error mapping

- `Error::BadRequest` for malformed email payloads.
- `Error::ServerError` for unexpected `TableStore` or `Publish` failures.

## Validation placement

- Structural validation in `from_input`: email present and RFC-5322 valid.
- Temporal validation in `handle`: compute `expires_at = Utc::now() + PASSWORD_RESET_EXPIRY_MINUTES`.

## Observability

- `tracing::info!(monotonic_counter.password_reset_requested_total = 1)` per invocation.
- `tracing::info!(monotonic_counter.password_reset_unknown_email_total = 1)` on the unknown-email branch.
