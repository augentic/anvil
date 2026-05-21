# Identity user registration — design

## Domain model

- `UserId` newtype wrapping the persisted user identifier (string per the legacy `User.id` shape).
- `EmailAddress` newtype wrapping an RFC-5322-validated email; constructor returns `Error::BadRequest` on invalid input.
- `User` record mirroring the legacy shape: `{ id: UserId, email: EmailAddress, created_at: DateTime<Utc> }`. Drawn from `legacy-monolith` `src/users/repository.ts#L1-L4`.

## Provider trait dependencies

- `register_user` handler depends on `TableStore` (write) for the identity-store table.

## APIs and integrations

- HTTP `POST /users` — registration handler; request body `{ email, password }`, response `201` with the persisted user or `400` with `{ "error": "invalid-email" }`.
- Internal call: handler delegates persistence to a thin repository wrapper around `TableStore::put`, mirroring the `insertUser` shape from `legacy-monolith` `src/users/register.ts#L31`.

## Configuration

- `IDENTITY_STORE_TABLE_NAME` — `Config::get` key naming the identity-store table. Reuses the existing identity-store table per the design-notes decision; no new user store is introduced. (from identity-design-notes)

## Handler delegation

- `RegisterUserRequest` implements `Handler<P>`; `from_input` parses the body and constructs an `EmailAddress` (which performs the RFC-5322 check). `handle` writes via `TableStore` and returns the persisted record.

## Error mapping

- `Error::BadRequest` with `code = "invalid-email"` and `description = "email is not RFC-5322 valid"` when `EmailAddress::try_new` fails.
- `Error::ServerError` for unexpected `TableStore` failures during persistence.

## Validation placement

- Structural validation in `from_input`: required `email` and `password` fields; RFC-5322 check on `email` via `EmailAddress::try_new`.
- No temporal validation; persistence is idempotent over the request payload.

## Observability

- `tracing::info!(monotonic_counter.users_register_total = 1)` per invocation.
- `tracing::info!(monotonic_counter.users_register_rejected_total = 1)` on validation rejection.
