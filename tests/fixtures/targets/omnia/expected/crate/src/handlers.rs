use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use omnia_sdk::api::{Context, Handler, IntoBody, Reply};
use omnia_sdk::{Config, Error, Publish, Result};
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

impl IntoBody for ResetAck {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec(&self).context("serializing ResetAck")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetEvent {
    pub email: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

async fn handle<P>(_owner: &str, request: ResetRequest, provider: &P) -> Result<Reply<ResetAck>>
where
    P: Config + Publish,
{
    let issued_at = Utc::now();
    let expires_at = issued_at + Duration::minutes(RESET_LINK_TTL_MINUTES);
    let topic = provider
        .get(PASSWORD_RESET_TOPIC_KEY)
        .await
        .map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    let event = ResetEvent { email: request.email, issued_at, expires_at };
    let payload = serde_json::to_vec(&event)
        .context("serializing ResetEvent")
        .map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    provider
        .publish(&topic, payload)
        .await
        .map_err(|e| ResetError::PublishUnavailable(e.to_string()))?;
    tracing::info!(monotonic_counter.password_reset_requests = 1);
    Ok(Reply::ok(ResetAck { status: "queued" }))
}

impl<P> Handler<P> for ResetRequest
where
    P: Config + Publish,
{
    type Error = Error;
    type Input = Vec<u8>;
    type Output = ResetAck;

    fn from_input(input: Vec<u8>) -> Result<Self> {
        let request: ResetRequest = serde_json::from_slice(&input)
            .context("deserializing ResetRequest")
            .map_err(Into::into)?;
        if request.email.trim().is_empty() || !request.email.contains('@') {
            return Err(ResetError::InvalidEmail.into());
        }
        Ok(request)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<ResetAck>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}
