//! Scripted [`omnia_guest::Model`] mock for this test binary: replies
//! are served in FIFO order. A dev-only, trimmed sibling of the copy in
//! `workflow-lib`'s test binary (no request recording — nothing here
//! asserts on requests); cross-crate test-support imports are not worth
//! a shared crate for ~30 lines.

use std::collections::VecDeque;
use std::sync::Mutex;

use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};

/// Scripted [`Model`] provider for native tests.
#[derive(Debug, Default)]
pub struct MockModel {
    replies: Mutex<VecDeque<Result<Reply, Error>>>,
}

impl MockModel {
    /// A mock whose every call succeeds with the given answers, in order.
    #[must_use]
    pub fn answering(answers: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            replies: Mutex::new(
                answers
                    .into_iter()
                    .map(|answer| {
                        Ok(Reply {
                            answer: answer.to_string(),
                            usage: None,
                        })
                    })
                    .collect(),
            ),
        }
    }
}

impl Model for MockModel {
    async fn create(&self, _request: Request) -> Result<Reply, Error> {
        self.replies
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockModel exhausted: script more replies")
    }
}
