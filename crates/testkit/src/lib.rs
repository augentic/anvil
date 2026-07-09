//! Dev-only shared test support for the Specify workspace.
//!
//! Home of the scripted [`omnia_guest::Model`] mock consumed by the
//! `workflow` and `core/tests` composed suites. A library crate (not a
//! per-suite `mod` include) so every test binary shares one copy and
//! partially-used helpers never trip `dead_code`. Never a dependency of
//! shipped code — consumers reach it through `[dev-dependencies]` only.

use std::collections::VecDeque;
use std::sync::Mutex;

use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};

/// Scripted [`Model`] provider for native tests: replies are served in
/// FIFO order and every request is recorded for assertion.
#[derive(Debug, Default)]
pub struct MockModel {
    replies: Mutex<VecDeque<Result<Reply, Error>>>,
    requests: Mutex<Vec<Request>>,
}

impl MockModel {
    /// A mock that answers each call with the next scripted result.
    #[must_use]
    pub fn scripted(replies: impl IntoIterator<Item = Result<Reply, Error>>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A mock whose every call succeeds with the given answers, in order.
    #[must_use]
    pub fn answering(answers: impl IntoIterator<Item = &'static str>) -> Self {
        Self::scripted(answers.into_iter().map(|answer| {
            Ok(Reply {
                answer: answer.to_string(),
                usage: None,
            })
        }))
    }

    /// Every request the mock has served, in call order.
    ///
    /// # Panics
    ///
    /// Panics when the interior lock is poisoned — only possible after a
    /// prior panic in the same test.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("mock lock").clone()
    }
}

impl Model for MockModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        self.requests.lock().expect("mock lock").push(request);
        self.replies
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockModel exhausted: script more replies")
    }
}
