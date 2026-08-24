//! A FIFO-scripted `Model` double recording every request.

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::sync::{Arc, Mutex};

use omnia_guest::model::{Error, McpGrant, Model, Reply, Request, Tool, ToolCall};

/// A FIFO model script of successes and typed failures, recording every
/// request for prompt and grant assertions.
#[derive(Clone, Debug)]
pub struct Scripted {
    script: Arc<Mutex<VecDeque<Result<Reply, Error>>>>,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Scripted {
    /// A script of ordered completion results.
    pub fn new(results: impl IntoIterator<Item = Result<Reply, Error>>) -> Self {
        Self {
            script: Arc::new(Mutex::new(results.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A success script of ordered answer strings.
    pub fn answering<S: Into<String>>(answers: impl IntoIterator<Item = S>) -> Self {
        Self::new(answers.into_iter().map(|answer| {
            Ok(Reply {
                answer: answer.into(),
                usage: None,
            })
        }))
    }

    /// Returns every request in call order.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("requests lock").clone()
    }

    /// Asserts that every scripted result was consumed.
    pub fn assert_exhausted(&self) {
        let left = self.script.lock().expect("script lock").len();
        assert_eq!(left, 0, "model script has {left} unconsumed result(s)");
    }

    fn next(&self, request: Request) -> Result<Reply, Error> {
        self.requests.lock().expect("requests lock").push(request);
        self.script
            .lock()
            .expect("script lock")
            .pop_front()
            .unwrap_or_else(|| Err(Error::Backend("model script exhausted".to_owned())))
    }
}

impl Model for Scripted {
    fn complete(&self, request: Request) -> impl Future<Output = Result<Reply, Error>> + Send {
        ready(self.next(request))
    }

    // A scripted reply never issues tool calls, so the handler is unused.
    fn complete_with<H, F>(
        &self, request: Request, _handler: H,
    ) -> impl Future<Output = Result<Reply, Error>> + Send
    where
        H: FnMut(ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        ready(self.next(request))
    }
}

/// Returns the MCP grants carried by a request.
#[must_use]
pub fn mcp_grants(request: &Request) -> Vec<&McpGrant> {
    request
        .tools
        .iter()
        .filter_map(|tool| match tool {
            Tool::Mcp(grant) => Some(grant),
            Tool::Function(_) => None,
        })
        .collect()
}
