# Add search filter — design

## Domain model

- `UserId` newtype wrapping the persisted user identifier.
- `SearchQuery` newtype wrapping the operator-supplied search string; constructor trims whitespace and rejects empty strings.

## Provider trait dependencies

- `list_users` handler depends on `TableStore` (read).

## APIs and integrations

- HTTP `GET /users?query={query}` — list handler; `query` is optional.

## Configuration

- `USERS_TABLE_NAME` — `Config::get` key for the underlying table name.

## Handler delegation

- `ListUsersRequest` implements `Handler<P>`; `from_input` parses the optional `query` into `Option<SearchQuery>`; `handle` reads the user table via `TableStore` and filters in-process.

## Error mapping

- `Error::BadRequest` for malformed `query` values (e.g. empty after trim).
- `Error::ServerError` for unexpected `TableStore` failures.

## Validation placement

- Structural validation in `from_input`: trim, non-empty when present.
- No temporal validation; the handler is pure-read.

## Observability

- `tracing::info!(monotonic_counter.users_list_total = 1)` per invocation.
- `tracing::info!(gauge.users_list_match_count = <n>)` per invocation with the matched count.
