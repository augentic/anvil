//! RFC-96 D1/D3: the bounded in-process operation pool and the
//! operation-claim registry. One scheduler path for every cap —
//! cap-one is the serial reference.

use std::collections::{HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

/// Compiled ceiling on the operation cap (the `MAX_REPAIRS` pattern).
pub const MAX_CAP: usize = 8;

/// Default cap when the deployment injects none: serial. Concurrency
/// is the operator's opt-in via `EMERY_POOL` until the inactivity
/// wake works on every deployment (Phase 0 containment).
pub const DEFAULT_CAP: usize = 1;

/// Effective operation cap from launcher deployment policy.
///
/// `EMERY_POOL` rides the same guest-environment inheritance as
/// `EMERY_WRITER` (the runtime snapshots the host environment into
/// every guest store), so one read works on both deployments. An
/// absent or unparseable value is the default; every value clamps to
/// `1..=MAX_CAP`. Read-only — the cap never changes engine state.
#[must_use]
pub fn cap() -> usize {
    let injected = std::env::var("EMERY_POOL")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_CAP);
    injected.clamp(1, MAX_CAP)
}

/// Compiled inactivity budgets per seam operation — starting values;
/// RFC-92 retained telemetry refines them later.
///
/// The seam futures wake only on host-side progress, so inactivity
/// approximates total runtime: budgets are generous upper bounds,
/// not targets.
pub mod budget {
    use std::time::Duration;

    /// Source survey (unfocused or focused).
    pub const SURVEY: Duration = Duration::from_mins(10);
    /// Source extract.
    pub const EXTRACT: Duration = Duration::from_mins(15);
    /// Schema-gated judgment legs (synthesis, propose, correlate).
    pub const JUDGMENT: Duration = Duration::from_mins(20);
    /// Target build-loop operations (build / verify / repair / review).
    pub const BUILD: Duration = Duration::from_hours(1);
    /// Target merge gates.
    pub const MERGE: Duration = Duration::from_mins(15);
}

/// One in-process operation claim: work-item identity plus the seam
/// operation and attempt ordinal it covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Claim {
    /// Work-item identity — the `(slice, phase, input-digest)` key
    /// rendered by the scheduler, or an operation-local unit key for
    /// read-heavy fan-out (source / binding / lead / domain).
    pub item: String,
    /// The seam operation under claim.
    pub operation: String,
    /// Attempt ordinal beneath the identity.
    pub attempt: u32,
}

/// In-process claim registry held by one supervisor.
///
/// Prevents duplicate execution of one operation inside the process;
/// it is *not* a durable fence — `guest.lock`, `slice.claimed`,
/// attempt directories, wave facts, and the accepted-CID chain stay
/// the cross-process authorities. No on-disk file and no journal
/// event; a duplicate is an [`Outcome::Rejected`] row, never an error
/// discriminant.
#[derive(Debug, Default)]
pub struct Claims {
    held: Mutex<HashSet<Claim>>,
}

impl Claims {
    /// Acquire `claim`; `false` when an identical claim is live.
    ///
    /// # Panics
    ///
    /// On a poisoned registry mutex (a panic on another thread).
    pub fn acquire(&self, claim: &Claim) -> bool {
        self.held.lock().expect("claims mutex").insert(claim.clone())
    }

    /// Release a held claim (idempotent).
    ///
    /// # Panics
    ///
    /// On a poisoned registry mutex (a panic on another thread).
    pub fn release(&self, claim: &Claim) {
        self.held.lock().expect("claims mutex").remove(claim);
    }

    /// Whether no claim is live.
    ///
    /// # Panics
    ///
    /// On a poisoned registry mutex (a panic on another thread).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.lock().expect("claims mutex").is_empty()
    }
}

/// Sibling handling once one admitted job fails (typed failure or
/// inactivity timeout). Both stop admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFailure {
    /// Drain in-flight siblings to their terminal reports (work-item
    /// failure inside a drain).
    Drain,
    /// Drop in-flight siblings — best-effort spend abort, release the
    /// claim, persist nothing (operator interrupt, proposal-assembly
    /// failure).
    Cancel,
}

/// One admitted unit of work: a claim, an inactivity budget, and the
/// seam future that settles it.
pub struct Job<'a, T, E> {
    /// The in-process claim this job holds while in flight.
    pub claim: Claim,
    /// Compiled inactivity budget ([`budget`]).
    pub budget: Duration,
    /// The operation future.
    pub future: Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>,
}

impl<T, E> std::fmt::Debug for Job<'_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("claim", &self.claim)
            .field("budget", &self.budget)
            .finish_non_exhaustive()
    }
}

/// Terminal state of one job, indexed like the admission order.
#[derive(Debug)]
pub enum Outcome<T, E> {
    /// The future settled — a terminal report either way.
    Settled(Result<T, E>),
    /// Dropped mid-flight by [`OnFailure::Cancel`].
    Cancelled,
    /// Inactivity budget exhausted; the future was dropped.
    TimedOut,
    /// Duplicate in-process claim — never admitted, siblings
    /// unaffected.
    Rejected,
    /// Admission stopped before this job started.
    Skipped,
}

impl<T, E> Outcome<T, E> {
    /// The settled success value, when this outcome carries one.
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Settled(Ok(value)) => Some(value),
            _ => None,
        }
    }
}

/// Drive `jobs` through one bounded pool.
///
/// Jobs are admitted in input order (the caller passes canonical
/// order) with at most `cap` in flight; results are joined in input
/// order, never completion order — the returned vector is indexed
/// like `jobs`. The first failure (typed or timeout) stops admission
/// and applies `on_failure` to in-flight siblings; never-admitted
/// jobs settle [`Outcome::Skipped`]. Every admitted claim is released
/// on every terminal path.
///
/// # Panics
///
/// On a poisoned claim or waker mutex (a panic on another thread).
pub async fn run<T, E>(
    cap: usize, claims: &Claims, on_failure: OnFailure, jobs: Vec<Job<'_, T, E>>,
) -> Vec<Outcome<T, E>> {
    let cap = cap.clamp(1, MAX_CAP);
    let total = jobs.len();
    let mut results: Vec<Option<Outcome<T, E>>> = (0..total).map(|_| None).collect();
    let mut queue: VecDeque<(usize, Job<'_, T, E>)> = jobs.into_iter().enumerate().collect();
    let mut running: Vec<Running<'_, T, E>> = Vec::new();
    let mut stopped = false;
    let timer = timer::Timer::default();

    std::future::poll_fn(|cx| {
        loop {
            // Admission: canonical input order, up to the cap, until
            // the first failure closes the gate.
            while !stopped && running.len() < cap {
                let Some((index, job)) = queue.pop_front() else { break };
                if claims.acquire(&job.claim) {
                    running.push(Running::admit(index, job, cx.waker()));
                } else {
                    results[index] = Some(Outcome::Rejected);
                }
            }

            let mut progressed = false;
            let mut slot = 0;
            while slot < running.len() {
                match running[slot].step(cx.waker()) {
                    Step::Pending => slot += 1,
                    Step::Done(result) => {
                        let failed = result.is_err();
                        let job = running.swap_remove(slot);
                        claims.release(&job.claim);
                        results[job.index] = Some(Outcome::Settled(result));
                        progressed = true;
                        if failed {
                            stopped = true;
                        }
                    }
                    Step::TimedOut => {
                        let job = running.swap_remove(slot);
                        claims.release(&job.claim);
                        results[job.index] = Some(Outcome::TimedOut);
                        progressed = true;
                        stopped = true;
                    }
                }
            }

            if stopped && on_failure == OnFailure::Cancel {
                for job in std::mem::take(&mut running) {
                    claims.release(&job.claim);
                    results[job.index] = Some(Outcome::Cancelled);
                }
            }
            if !progressed && (stopped || running.len() >= cap || queue.is_empty()) {
                break;
            }
        }

        if running.is_empty() && (stopped || queue.is_empty()) {
            for (index, _) in std::mem::take(&mut queue) {
                results[index] = Some(Outcome::Skipped);
            }
            return Poll::Ready(());
        }
        // Real inactivity wake (S4): arm a monotonic-clock wake at the
        // earliest in-flight deadline so a hung job times out even when
        // no sibling ever progresses. Poll-driven detection above stays
        // the check; the timer only guarantees a poll happens.
        if let Some(deadline) = running.iter().map(Running::deadline).min() {
            timer.arm(deadline, cx.waker());
        }
        Poll::Pending
    })
    .await;

    results.into_iter().map(|outcome| outcome.expect("every job settles")).collect()
}

/// One in-flight job with its activity-tracking waker.
struct Running<'a, T, E> {
    index: usize,
    claim: Claim,
    budget: Duration,
    future: Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>,
    activity: Arc<Activity>,
    waker: Waker,
}

enum Step<T, E> {
    Pending,
    Done(Result<T, E>),
    TimedOut,
}

impl<'a, T, E> Running<'a, T, E> {
    fn admit(index: usize, job: Job<'a, T, E>, pool: &Waker) -> Self {
        let activity = Arc::new(Activity {
            pool: Mutex::new(pool.clone()),
            last: Mutex::new(Instant::now()),
        });
        let waker = Waker::from(Arc::clone(&activity));
        Self {
            index,
            claim: job.claim,
            budget: job.budget,
            future: job.future,
            activity,
            waker,
        }
    }

    /// The instant this job's inactivity budget exhausts, from its
    /// last observed activity.
    fn deadline(&self) -> Instant {
        *self.activity.last.lock().expect("activity mutex") + self.budget
    }

    /// Check the inactivity budget, then poll once. Detection is
    /// poll-driven: the check runs whenever the pool wakes (a sibling,
    /// this job's own backend, or the armed inactivity timer).
    fn step(&mut self, pool: &Waker) -> Step<T, E> {
        {
            let mut current = self.activity.pool.lock().expect("waker mutex");
            if !current.will_wake(pool) {
                current.clone_from(pool);
            }
        }
        let last = *self.activity.last.lock().expect("activity mutex");
        if last.elapsed() > self.budget {
            return Step::TimedOut;
        }
        let mut cx = Context::from_waker(&self.waker);
        match self.future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => Step::Done(result),
            Poll::Pending => Step::Pending,
        }
    }
}

/// Per-job waker: a wake is observed activity — it stamps the clock
/// and forwards to the pool's waker.
struct Activity {
    pool: Mutex<Waker>,
    last: Mutex<Instant>,
}

impl Wake for Activity {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        *self.last.lock().expect("activity mutex") = Instant::now();
        self.pool.lock().expect("waker mutex").wake_by_ref();
    }
}

/// Monotonic-clock wake for the inactivity budget.
///
/// Native deployments run one timer thread per pool run: it sleeps
/// until the earliest armed deadline and wakes the pool waker, so a
/// hung job is timed out without any sibling progress. The wasm32
/// guest has no thread to sleep on and no clock-wake import — the
/// guest stays poll-driven until a deployment wake capability lands
/// (Phase 2b / RFC-96 D8 note); the launcher-hosted native path is
/// the one that fans out today.
#[cfg(not(target_arch = "wasm32"))]
mod timer {
    use std::sync::{Arc, Condvar, Mutex};
    use std::task::Waker;
    use std::time::Instant;

    /// One lazily started timer thread, rearmed per pool poll and shut
    /// down on drop.
    #[derive(Default)]
    pub struct Timer {
        shared: Arc<Shared>,
        thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    }

    #[derive(Default)]
    struct Shared {
        state: Mutex<State>,
        signal: Condvar,
    }

    #[derive(Default)]
    struct State {
        armed: Option<(Instant, Waker)>,
        shutdown: bool,
    }

    impl Timer {
        /// (Re)arm the wake: at `deadline` the pool waker fires unless
        /// rearmed or dropped first.
        pub fn arm(&self, deadline: Instant, waker: &Waker) {
            let mut state = self.shared.state.lock().expect("timer mutex");
            state.armed = Some((deadline, waker.clone()));
            drop(state);
            self.shared.signal.notify_one();
            let mut thread = self.thread.lock().expect("timer thread handle");
            if thread.is_none() {
                let shared = Arc::clone(&self.shared);
                *thread = Some(std::thread::spawn(move || tick(&shared)));
            }
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            self.shared.state.lock().expect("timer mutex").shutdown = true;
            self.shared.signal.notify_one();
            let handle = self.thread.lock().expect("timer thread handle").take();
            if let Some(handle) = handle {
                drop(handle.join());
            }
        }
    }

    fn tick(shared: &Shared) {
        let mut state = shared.state.lock().expect("timer mutex");
        loop {
            if state.shutdown {
                return;
            }
            match &state.armed {
                None => {
                    state = shared.signal.wait(state).expect("timer mutex");
                }
                Some((deadline, _)) => {
                    let now = Instant::now();
                    if now >= *deadline {
                        if let Some((_, waker)) = state.armed.take() {
                            drop(state);
                            waker.wake();
                            state = shared.state.lock().expect("timer mutex");
                        }
                    } else {
                        let wait = *deadline - now;
                        state = shared.signal.wait_timeout(state, wait).expect("timer mutex").0;
                    }
                }
            }
        }
    }
}

/// wasm32: no thread and no clock-wake import — arming is a no-op and
/// timeout detection stays poll-driven (Phase 2b / RFC-96 D8).
#[cfg(target_arch = "wasm32")]
mod timer {
    use std::task::Waker;
    use std::time::Instant;

    /// Inert guest-side stand-in for the native timer thread.
    #[derive(Default)]
    pub struct Timer;

    impl Timer {
        /// No-op: the guest has no wake source to arm.
        pub fn arm(&self, _deadline: Instant, _waker: &Waker) {}
    }
}
