# password-reset — design.md

## Domain model

```rust
pub struct EmailAddress(String); // RFC-5322 validated newtype
pub struct ResetRequest {
    pub email: EmailAddress,
}
pub struct ResetEvent {
    pub email: String,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
```

Reset link expiry is encoded as `expires_at = issued_at + Duration::minutes(30)` in the event payload.

## Provider trait dependencies

| Handler        | Traits consumed         |
|----------------|--------------------------|
| `ResetRequest` | `Config`, `Publish`     |

## Handler delegation

`ResetRequest` implements `Handler<P> where P: Config + Publish` and delegates to a standalone:

```rust
async fn handle<P>(owner: &str, request: ResetRequest, provider: &P) -> Result<Reply<ResetAck>>
where P: Config + Publish { ... }
```

`type Input = Vec<u8>` (HTTP POST body); `from_input` deserialises JSON into `ResetRequest` and runs RFC-5322 email validation. `Utc::now()` is called inside `handle`, never `from_input`.

## External surfaces

| Kind         | Identifier              |
|--------------|--------------------------|
| `http-route` | `POST /password-reset`  |
| `message-pub`| `password-reset.requested` (topic name from `Config::get("PASSWORD_RESET_TOPIC")`) |

## Configuration

| Key                     | Default              | Notes                                           |
|-------------------------|----------------------|--------------------------------------------------|
| `PASSWORD_RESET_TOPIC`  | `password-reset.requested` | Validated in `Provider::new`; missing → `ServerError`. |

## Error mapping

```rust
#[derive(thiserror::Error, Debug)]
pub enum ResetError {
    #[error("invalid email")]
    InvalidEmail,
    #[error("publish failed: {0}")]
    PublishUnavailable(String),
}

impl From<ResetError> for omnia_sdk::Error {
    fn from(err: ResetError) -> Self {
        match &err {
            ResetError::InvalidEmail => omnia_sdk::Error::BadRequest {
                code: "invalid_email".into(),
                description: err.to_string(),
            },
            ResetError::PublishUnavailable(_) => omnia_sdk::Error::BadGateway {
                code: "publish_unavailable".into(),
                description: err.to_string(),
            },
        }
    }
}
```

## Validation placement

| Check                          | Location           | Reason                                  |
|--------------------------------|--------------------|------------------------------------------|
| Email is non-empty             | `from_input()`     | Structural; depends only on parse result. |
| Email matches RFC-5322         | `from_input()`     | Structural; constant pattern.            |
| `Utc::now()` for `issued_at`   | `handle()`         | Runtime time; not parse-time.            |

## Observability

- `tracing::info!(monotonic_counter.password_reset_requests = 1)` per accepted request (regardless of address validity).
- `tracing::info!(monotonic_counter.password_reset_unknown_email = 1)` when the email is not in any known user state — keeps the public response opaque while preserving operator-visible signal.
- `tracing::debug!("publish topic={topic}")` once the publish succeeds.
