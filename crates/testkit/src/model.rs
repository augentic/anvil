//! A FIFO-scripted `Model` double recording every request.

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::sync::{Arc, Mutex};

use omnia_guest::model::{Error, Function, Model, Reply, Request, Tool, ToolCall};

/// One `(call, output)` handler exchange recorded by `complete_with`.
pub type Exchange = (ToolCall, Result<String, String>);

/// One scripted completion turn: the tool calls fed to the handler,
/// then the turn's result.
#[derive(Debug)]
struct Turn {
    calls: Vec<ToolCall>,
    result: Result<Reply, Error>,
}

/// A FIFO model script of successes and typed failures, recording
/// every request for prompt and tool assertions. A turn may carry
/// scripted tool calls: `complete_with` feeds them to the handler and
/// records each exchange before the turn's result returns.
#[derive(Clone, Debug)]
pub struct Scripted {
    script: Arc<Mutex<VecDeque<Turn>>>,
    requests: Arc<Mutex<Vec<Request>>>,
    exchanges: Arc<Mutex<Vec<Exchange>>>,
}

impl Scripted {
    /// A script of ordered completion results.
    pub fn new(results: impl IntoIterator<Item = Result<Reply, Error>>) -> Self {
        Self {
            script: Arc::new(Mutex::new(
                results
                    .into_iter()
                    .map(|result| Turn {
                        calls: Vec::new(),
                        result,
                    })
                    .collect(),
            )),
            requests: Arc::new(Mutex::new(Vec::new())),
            exchanges: Arc::new(Mutex::new(Vec::new())),
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

    /// Attaches tool calls to the scripted turn at `index`; the
    /// handler answers them before that turn's result returns.
    ///
    /// # Panics
    ///
    /// Panics when no turn is scripted at `index`.
    #[must_use]
    pub fn calling(self, index: usize, calls: impl IntoIterator<Item = ToolCall>) -> Self {
        {
            let mut script = self.script.lock().expect("script lock");
            let turn = script.get_mut(index).expect("a scripted turn at the call index");
            turn.calls.extend(calls);
        }
        self
    }

    /// Returns every request in call order.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("requests lock").clone()
    }

    /// Returns every handler exchange in call order.
    #[must_use]
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.exchanges.lock().expect("exchanges lock").clone()
    }

    /// Asserts that every scripted result was consumed.
    pub fn assert_exhausted(&self) {
        let left = self.script.lock().expect("script lock").len();
        assert_eq!(left, 0, "model script has {left} unconsumed result(s)");
    }

    fn next(&self, request: Request) -> Turn {
        self.requests.lock().expect("requests lock").push(request);
        self.script.lock().expect("script lock").pop_front().unwrap_or_else(|| Turn {
            calls: Vec::new(),
            result: Err(Error::Backend("model script exhausted".to_owned())),
        })
    }
}

impl Model for Scripted {
    // A single-shot completion has no handler; scripting tool calls on
    // its turn is a harness bug.
    fn complete(&self, request: Request) -> impl Future<Output = Result<Reply, Error>> + Send {
        let turn = self.next(request);
        assert!(turn.calls.is_empty(), "scripted tool calls require complete_with");
        ready(turn.result)
    }

    fn complete_with<H, F>(
        &self, request: Request, mut handler: H,
    ) -> impl Future<Output = Result<Reply, Error>> + Send
    where
        H: FnMut(ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        let turn = self.next(request);
        let exchanges = Arc::clone(&self.exchanges);
        async move {
            for call in turn.calls {
                let output = handler(call.clone()).await;
                exchanges.lock().expect("exchanges lock").push((call, output));
            }
            turn.result
        }
    }
}

/// Returns the function tools declared by a request.
#[must_use]
pub fn function_tools(request: &Request) -> Vec<&Function> {
    request
        .tools
        .iter()
        .filter_map(|tool| match tool {
            Tool::Function(function) => Some(function),
            Tool::Mcp(_) => None,
        })
        .collect()
}
