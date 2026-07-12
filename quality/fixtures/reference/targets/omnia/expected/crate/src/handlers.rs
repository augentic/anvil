use chrono::{DateTime, Duration, Utc};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use omnia_guest::api::Provider;
use omnia_guest::{Config, Error, Message, Publish, Result};
use serde::{Deserialize, Serialize};

use crate::error::ResetError;

const RESET_LINK_TTL_MINUTES: i64 = 30;
const PASSWORD_RESET_TOPIC_KEY: &str = "PASSWORD_RESET_TOPIC";

#[derive(Debug, Deserialize)]
pub struct ResetRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetAck {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetEvent {
    pub email: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

async fn reset<P>(request: ResetRequest, provider: &P) -> Result<ResetAck>
where
    P: Config + Publish,
{
    let issued_at = Utc::now();
    let expires_at = issued_at + Duration::minutes(RESET_LINK_TTL_MINUTES);
    let topic = provider
        .get(PASSWORD_RESET_TOPIC_KEY)
        .await
        .map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    let event = ResetEvent {
        email: request.email,
        issued_at,
        expires_at,
    };
    let payload =
        serde_json::to_vec(&event).map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    provider
        .send(&topic, &Message::new(&payload))
        .await
        .map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    tracing::info!(monotonic_counter.password_reset_requests = 1);
    Ok(ResetAck { status: "queued" })
}

#[derive(Clone, Copy, Debug)]
pub struct Reset;

impl<P> Operation<P> for Reset
where
    P: Provider + Config + Publish,
{
    type Error = Error;
    type Input = ResetRequest;
    type Output = ResetAck;

    async fn call(input: Self::Input, context: CallContext<'_, P>) -> Result<Self::Output> {
        if input.email.trim().is_empty() || !input.email.contains('@') {
            return Err(ResetError::InvalidEmail.into());
        }
        reset(input, context.provider).await
    }
}
