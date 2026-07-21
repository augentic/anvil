//! Request-recording [`Harness`] model double and [`mcp_grants`],
//! shared by the adapter SDK and engine suites — local copies
//! pending their move upstream to `omnia_testkit::model`.

use std::sync::{Arc, Mutex, MutexGuard};

use omnia_guest::model::{Error, McpGrant, Model, Reply, Request, Tool};
use omnia_testkit::model::Scripted;

/// A model decorator that records requests before delegating them.
#[derive(Clone, Debug)]
pub struct Harness<B> {
    backend: B,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl<B> Harness<B> {
    /// Wrap `backend` with request recording.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a thread-safe snapshot of every request in call order.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        lock(&self.requests).clone()
    }
}

impl Harness<Scripted> {
    /// Build a recorded harness from ordered completion results.
    #[must_use]
    pub fn scripted(responses: impl IntoIterator<Item = Result<Reply, Error>>) -> Self {
        Self::new(Scripted::new(responses))
    }

    /// Build a recorded harness from ordered answer strings.
    #[must_use]
    pub fn answering<S>(answers: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self::new(Scripted::answers(answers))
    }

    /// Assert that every scripted result was consumed.
    ///
    /// # Panics
    ///
    /// Panics when one or more results remain.
    pub fn assert_exhausted(&self) {
        self.backend.assert_exhausted();
    }
}

impl<B> Model for Harness<B>
where
    B: Model,
{
    fn create(&self, request: Request) -> impl Future<Output = Result<Reply, Error>> + Send {
        lock(&self.requests).push(request.clone());
        self.backend.create(request)
    }
}

/// Return the MCP grants carried by a request.
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("testkit mutex is never poisoned")
}
