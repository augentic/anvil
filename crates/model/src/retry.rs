//! Bounded retry over transport failures in a model completion.
//!
//! A spawned agent fails in two very different ways. It never reached the
//! provider — a refused connection, a killed spawn, a stalled stream — in
//! which case the same request would very likely succeed a moment later. Or it
//! reached the provider and came back with an answer the format gate rejected
//! twice, in which case retrying buys nothing but two more agent spawns
//! against the same bad answer.
//!
//! `complete` returns an untyped [`anyhow::Error`], so the split is drawn by
//! matching known transport phrasings across the error chain. That is a
//! heuristic, and it fails safe: an unrecognised error is simply not retried.

use std::time::Duration;

use anyhow::{Error, Result};

/// The transport-failure phrasings the spawned backends emit. Each names a
/// point before or during the exchange where nothing about the request itself
/// was rejected.
const TRANSPORT_SIGNATURES: &[&str] = &[
    // The provider was unreachable (`Failed to reach the Cursor API`).
    "failed to reach",
    // The agent process died — usually a non-zero exit carrying a network message.
    "exited with",
    // The watchdog's inactivity window closed on a silent stream.
    "inactive for",
    // The watchdog's absolute cap closed on a still-streaming spawn.
    "timed out after",
    // The binary could not be launched at all.
    "spawning",
    // The stream ended before the terminal event.
    "did not emit a terminal result event",
];

/// How many extra attempts a transport failure earns, and how long to wait
/// before the first of them.
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// Extra attempts after the first; zero disables retry.
    pub retries: u32,
    /// Backoff before the first retry; each subsequent wait doubles.
    pub backoff: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            retries: 2,
            backoff: Duration::from_secs(1),
        }
    }
}

/// Run `attempt` until it succeeds, fails with a non-transport error, or
/// exhausts the policy's retries.
///
/// # Errors
///
/// Returns the final attempt's error. A non-transport error is returned
/// immediately, without consuming a retry.
pub async fn with_retry<T, F, Fut>(policy: Policy, backend: &str, mut attempt: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut backoff = policy.backoff;
    for retries_left in (0..=policy.retries).rev() {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if retries_left == 0 {
                    return Err(error);
                }
                let Some(reason) = transport_reason(&error) else {
                    return Err(error);
                };
                tracing::warn!(
                    backend,
                    %reason,
                    retries_left,
                    backoff_ms = backoff.as_millis(),
                    "model completion hit a transport failure; retrying"
                );
                tokio::time::sleep(jittered(backoff)).await;
                backoff = backoff.saturating_mul(2);
            }
        }
    }
    // `policy.retries + 1` iterations always return.
    unreachable!("the last iteration returns")
}

/// The matched transport signature, when the error chain carries one.
fn transport_reason(error: &Error) -> Option<&'static str> {
    error.chain().find_map(|cause| {
        let text = cause.to_string().to_lowercase();
        TRANSPORT_SIGNATURES.iter().copied().find(|signature| text.contains(*signature))
    })
}

/// Spread concurrent retries across up to a quarter of the backoff window, so
/// several slices that failed on the same provider blip do not resynchronise.
///
/// The jitter source is the current nanosecond clock rather than a PRNG: it
/// costs no dependency, and a retry schedule needs decorrelation, not
/// randomness.
fn jittered(backoff: Duration) -> Duration {
    let quarter = backoff / 4;
    if quarter.is_zero() {
        return backoff;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    backoff
        + Duration::from_nanos(u64::from(nanos) % u64::try_from(quarter.as_nanos()).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use anyhow::anyhow;

    use super::{Policy, transport_reason, with_retry};

    const FAST: Policy = Policy {
        retries: 2,
        backoff: Duration::from_millis(1),
    };

    #[test]
    fn classifies_unreachable_provider() {
        let error = anyhow!("Failed to reach the Cursor API");
        assert_eq!(transport_reason(&error), Some("failed to reach"));
    }

    #[test]
    fn classifies_through_the_error_chain() {
        let error =
            anyhow!("cursor-agent exited with exit status: 1").context("running the extract leg");
        assert_eq!(
            transport_reason(&error),
            Some("exited with"),
            "the signature may sit under a context layer"
        );
    }

    #[test]
    fn classifies_watchdog_kills() {
        assert!(transport_reason(&anyhow!("claude inactive for 120s")).is_some());
        assert!(transport_reason(&anyhow!("claude timed out after 600s")).is_some());
        assert!(transport_reason(&anyhow!("spawning `claude`")).is_some());
    }

    #[test]
    fn schema_exhaustion_is_not_transport() {
        let error = anyhow!("claude did not return an answer after 2 attempts: missing `verdict`");
        assert!(
            transport_reason(&error).is_none(),
            "retrying a rejected answer just burns two more spawns"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn transport_failure_retries_until_it_succeeds() {
        let attempts = Cell::new(0);
        let value = with_retry(FAST, "claude", || async {
            attempts.set(attempts.get() + 1);
            if attempts.get() < 3 { Err(anyhow!("Failed to reach the API")) } else { Ok("ok") }
        })
        .await
        .expect("the third attempt succeeds");
        assert_eq!(value, "ok");
        assert_eq!(attempts.get(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn retries_are_bounded() {
        let attempts = Cell::new(0);
        let error = with_retry(FAST, "claude", || async {
            attempts.set(attempts.get() + 1);
            Err::<(), _>(anyhow!("Failed to reach the API"))
        })
        .await
        .expect_err("a permanently unreachable provider still fails");
        assert_eq!(attempts.get(), 3, "the first attempt plus two retries");
        assert!(error.to_string().contains("Failed to reach"));
    }

    #[tokio::test(start_paused = true)]
    async fn non_transport_failure_passes_straight_through() {
        let attempts = Cell::new(0);
        let error = with_retry(FAST, "cursor", || async {
            attempts.set(attempts.get() + 1);
            Err::<(), _>(anyhow!("did not return an answer after 2 attempts: bad shape"))
        })
        .await
        .expect_err("a rejected answer is not retried");
        assert_eq!(attempts.get(), 1);
        assert!(error.to_string().contains("bad shape"));
    }

    #[tokio::test(start_paused = true)]
    async fn zero_retries_makes_one_attempt() {
        let attempts = Cell::new(0);
        let policy = Policy {
            retries: 0,
            backoff: Duration::from_millis(1),
        };
        let _error = with_retry(policy, "claude", || async {
            attempts.set(attempts.get() + 1);
            Err::<(), _>(anyhow!("Failed to reach the API"))
        })
        .await
        .expect_err("retries disabled");
        assert_eq!(attempts.get(), 1);
    }
}
