//! RFC-96 D1/D3: bounded-pool semantics over scripted futures —
//! canonical join order at every cap, duplicate-claim rejection,
//! claim release on every terminal path, drain vs cancel, and the
//! poll-driven inactivity timeout.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use project::pool::{Claim, Claims, Job, OnFailure, Outcome, run};

/// Settles after `steps` self-waking polls — deterministic staggered
/// completion without a clock.
struct Steps {
    left: u32,
    output: Result<u32, String>,
}

impl Future for Steps {
    type Output = Result<u32, String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.left == 0 {
            return Poll::Ready(self.output.clone());
        }
        self.left -= 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

const GENEROUS: Duration = Duration::from_mins(1);

fn claim(item: &str) -> Claim {
    Claim {
        item: item.to_string(),
        operation: "extract".to_string(),
        attempt: 1,
    }
}

fn job(item: &str, steps: u32, output: Result<u32, String>) -> Job<'static, u32, String> {
    Job {
        claim: claim(item),
        budget: GENEROUS,
        future: Box::pin(Steps { left: steps, output }),
    }
}

/// Project the settled success values in join order (`None` marks a
/// non-settled row).
fn values(outcomes: Vec<Outcome<u32, String>>) -> Vec<Option<u32>> {
    outcomes.into_iter().map(Outcome::ok).collect()
}

#[tokio::test]
async fn cap_one_serial_reference() {
    // Staggered completions: under cap four the later job finishes
    // first, yet the joined order is identical to cap one.
    let build = || vec![job("a", 8, Ok(1)), job("b", 4, Ok(2)), job("c", 0, Ok(3))];
    let claims = Claims::default();
    let serial = values(run(1, &claims, OnFailure::Drain, build()).await);
    let pooled = values(run(4, &claims, OnFailure::Drain, build()).await);
    assert_eq!(serial, [Some(1), Some(2), Some(3)]);
    assert_eq!(serial, pooled, "join order is input order at every cap");
    assert!(claims.is_empty());
}

#[tokio::test]
async fn duplicate_claim_rejected() {
    // The second holder of a live claim is rejected without blocking
    // its siblings; the first holder still settles.
    let claims = Claims::default();
    let jobs = vec![job("same", 6, Ok(1)), job("same", 6, Ok(2)), job("other", 0, Ok(3))];
    let outcomes = run(4, &claims, OnFailure::Drain, jobs).await;
    assert!(matches!(outcomes[0], Outcome::Settled(Ok(1))));
    assert!(matches!(outcomes[1], Outcome::Rejected));
    assert!(matches!(outcomes[2], Outcome::Settled(Ok(3))));
    assert!(claims.is_empty(), "released on every terminal path");
}

#[tokio::test]
async fn failure_drains_siblings() {
    // Work-item failure stops admission; in-flight siblings drain to
    // terminal reports and queued jobs never start.
    let claims = Claims::default();
    let jobs = vec![
        job("a", 1, Err("boom".to_string())),
        job("b", 10, Ok(2)),
        job("c", 5, Ok(3)),
        job("d", 0, Ok(4)),
    ];
    let outcomes = run(3, &claims, OnFailure::Drain, jobs).await;
    assert!(matches!(&outcomes[0], Outcome::Settled(Err(reason)) if reason == "boom"));
    assert!(matches!(outcomes[1], Outcome::Settled(Ok(2))), "in-flight sibling drains");
    assert!(matches!(outcomes[2], Outcome::Settled(Ok(3))), "in-flight sibling drains");
    assert!(matches!(outcomes[3], Outcome::Skipped), "admission stopped");
    assert!(claims.is_empty());
}

#[tokio::test]
async fn failure_cancels_siblings() {
    // Cancel policy: drop in-flight siblings, release their claims,
    // persist nothing — a sibling that would settle later reports
    // cancelled instead.
    let claims = Claims::default();
    let jobs =
        vec![job("a", 1, Err("assembly".to_string())), job("b", 50, Ok(2)), job("c", 0, Ok(3))];
    let outcomes = run(2, &claims, OnFailure::Cancel, jobs).await;
    assert!(matches!(&outcomes[0], Outcome::Settled(Err(_))));
    assert!(matches!(outcomes[1], Outcome::Cancelled));
    assert!(matches!(outcomes[2], Outcome::Skipped));
    assert!(claims.is_empty());
}

#[tokio::test]
async fn inactivity_times_out() {
    // A zero-budget job trips the poll-driven inactivity check and is
    // treated as a work-item failure: siblings drain, the queue stops.
    let claims = Claims::default();
    let mut timed = job("slow", 1000, Ok(1));
    timed.budget = Duration::ZERO;
    let jobs = vec![timed, job("b", 3, Ok(2)), job("c", 0, Ok(3))];
    let outcomes = run(2, &claims, OnFailure::Drain, jobs).await;
    assert!(matches!(outcomes[0], Outcome::TimedOut));
    assert!(matches!(outcomes[1], Outcome::Settled(Ok(2))), "sibling drains to terminal");
    assert!(matches!(outcomes[2], Outcome::Skipped));
    assert!(claims.is_empty());
}

#[tokio::test]
async fn rejected_never_releases() {
    // A rejected duplicate must not release the live holder's claim:
    // while the first holder runs, the registry still refuses a third.
    let claims = Claims::default();
    let jobs = vec![job("same", 4, Ok(1)), job("same", 0, Ok(2)), job("same", 0, Ok(3))];
    let outcomes = run(4, &claims, OnFailure::Drain, jobs).await;
    assert!(matches!(outcomes[0], Outcome::Settled(Ok(1))));
    assert!(matches!(outcomes[1], Outcome::Rejected));
    assert!(matches!(outcomes[2], Outcome::Rejected));
    assert!(claims.is_empty());
}

mod cap {
    use project::pool::{DEFAULT_CAP, MAX_CAP, cap};

    #[test]
    #[expect(
        unsafe_code,
        reason = "EMERY_POOL is the launcher cap seam; nextest isolates the process"
    )]
    fn env_policy() {
        // RFC-96: EMERY_POOL rides guest-env inheritance; absent or
        // malformed values fall back to the default, everything
        // clamps to 1..=MAX_CAP.
        // SAFETY: nextest isolates the test process; no concurrent reader.
        unsafe { std::env::remove_var("EMERY_POOL") };
        assert_eq!(cap(), DEFAULT_CAP);
        // SAFETY: as above.
        unsafe { std::env::set_var("EMERY_POOL", "2") };
        assert_eq!(cap(), 2);
        // SAFETY: as above.
        unsafe { std::env::set_var("EMERY_POOL", "0") };
        assert_eq!(cap(), 1, "zero clamps to one");
        // SAFETY: as above.
        unsafe { std::env::set_var("EMERY_POOL", "99") };
        assert_eq!(cap(), MAX_CAP, "over-cap clamps to the compiled maximum");
        // SAFETY: as above.
        unsafe { std::env::set_var("EMERY_POOL", "many") };
        assert_eq!(cap(), DEFAULT_CAP, "malformed falls back");
        // SAFETY: as above.
        unsafe { std::env::remove_var("EMERY_POOL") };
    }
}
