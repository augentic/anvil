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

| Operation      | Traits consumed         |
|----------------|--------------------------|
| `ResetRequest` | `Config`, `Publish`     |

## Operation delegation

`Reset` implements `Operation<P> where P: Config + Publish`, takes `ResetRequest` as its typed input, and delegates to a standalone:

```rust
async fn reset<P>(request: ResetRequest, provider: &P) -> Result<ResetAck>
where P: Config + Publish { ... }
```

`type Input = ResetRequest`; the typed HTTP router deserialises the POST body and `Operation::call` performs RFC-5322 email validation before delegation. `Utc::now()` is called by the runtime operation path, never during transport decoding.

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

impl From<ResetError> for omnia_guest::Error {
    fn from(err: ResetError) -> Self {
        match &err {
            ResetError::InvalidEmail => omnia_guest::Error::BadRequest {
                code: "invalid_email".into(),
                description: err.to_string(),
            },
            ResetError::PublishUnavailable(_) => omnia_guest::Error::BadGateway {
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
| Email is non-empty             | `Operation::call`  | Structural; depends only on typed input. |
| Email matches RFC-5322         | `Operation::call`  | Structural; constant pattern.            |
| `Utc::now()` for `issued_at`   | delegated runtime function | Runtime time; not decode-time. |

## Observability

- `tracing::info!(monotonic_counter.password_reset_requests = 1)` per accepted request (regardless of address validity).
- `tracing::info!(monotonic_counter.password_reset_unknown_email = 1)` when the email is not in any known user state — keeps the public response opaque while preserving operator-visible signal.
- `tracing::debug!("publish topic={topic}")` once the publish succeeds.
