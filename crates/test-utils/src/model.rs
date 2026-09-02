//! A FIFO-scripted `WasiModelCtx` double recording every request.
//!
//! The host-side counterpart of `emery_testkit::Scripted` (a guest `Model`
//! double): answers are `serde_json::Value`s the wasi-model host projects to
//! the guest through its format gate, and a turn may drive declared tools
//! through the session's [`ToolHost`] before answering.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use omnia::futures::FutureExt as _;
use omnia_wasi_model::{Answer, Format, FutureResult, Request, Tool, ToolHost, WasiModelCtx};
use serde_json::Value;

/// One recorded request: the fields a scenario asserts on.
#[derive(Clone, Debug)]
pub struct Seen {
    /// The system prompt.
    pub system: Option<String>,
    /// Message bodies in turn order.
    pub messages: Vec<String>,
    /// The requested output shape.
    pub format: Format,
    /// Declared function-tool names.
    pub tools: Vec<String>,
}

/// One tool call the backend drove and the outcome the guest answered.
#[derive(Clone, Debug)]
pub struct Exchange {
    /// Tool name.
    pub tool: String,
    /// JSON arguments as sent.
    pub arguments: String,
    /// The guest's answer; `Err` is the tool's model-visible failure text.
    pub outcome: Result<String, String>,
}

#[derive(Debug)]
struct Turn {
    calls: Vec<(String, String)>,
    answer: Value,
}

/// A FIFO model script recording every request and tool exchange.
#[derive(Clone, Debug)]
pub struct ScriptedModel {
    script: Arc<Mutex<VecDeque<Turn>>>,
    requests: Arc<Mutex<Vec<Seen>>>,
    exchanges: Arc<Mutex<Vec<Exchange>>>,
}

impl ScriptedModel {
    /// A script of ordered answers.
    pub fn answering(answers: impl IntoIterator<Item = Value>) -> Self {
        Self {
            script: Arc::new(Mutex::new(
                answers
                    .into_iter()
                    .map(|answer| Turn {
                        calls: Vec::new(),
                        answer,
                    })
                    .collect(),
            )),
            requests: Arc::new(Mutex::new(Vec::new())),
            exchanges: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Attaches `(tool, arguments)` calls to the turn at `index`; the
    /// backend drives them through the session before that turn answers.
    ///
    /// # Panics
    ///
    /// Panics when no turn is scripted at `index`.
    #[must_use]
    pub fn calling<'a>(
        self, index: usize, calls: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Self {
        let mut script = self.script.lock().expect("script lock");
        let turn = script.get_mut(index).expect("a scripted turn at the call index");
        turn.calls.extend(calls.into_iter().map(|(tool, args)| (tool.to_owned(), args.to_owned())));
        drop(script);
        self
    }

    /// Every request in call order.
    ///
    /// # Panics
    ///
    /// Panics if a lock is poisoned.
    #[must_use]
    pub fn requests(&self) -> Vec<Seen> {
        self.requests.lock().expect("requests lock").clone()
    }

    /// Every driven tool exchange in call order.
    ///
    /// # Panics
    ///
    /// Panics if a lock is poisoned.
    #[must_use]
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.exchanges.lock().expect("exchanges lock").clone()
    }

    /// Asserts that every scripted answer was consumed.
    ///
    /// # Panics
    ///
    /// Panics if any scripted answer remains.
    pub fn assert_exhausted(&self) {
        let left = self.script.lock().expect("script lock").len();
        assert_eq!(left, 0, "model script has {left} unconsumed answer(s)");
    }

    fn next(&self, request: &Request) -> Option<Turn> {
        self.requests.lock().expect("requests lock").push(Seen {
            system: request.system.clone(),
            messages: request.messages.iter().map(|message| message.content.clone()).collect(),
            format: request.format.clone(),
            tools: request
                .tools
                .iter()
                .filter_map(|tool| match tool {
                    Tool::Function(function) => Some(function.name.clone()),
                    Tool::Mcp(_) => None,
                })
                .collect(),
        });
        self.script.lock().expect("script lock").pop_front()
    }
}

impl WasiModelCtx for ScriptedModel {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let Some(turn) = self.next(&request) else {
            return async { Err(anyhow::anyhow!("model script exhausted")) }.boxed();
        };
        let exchanges = Arc::clone(&self.exchanges);
        async move {
            for (tool, arguments) in turn.calls {
                let outcome = tool_host.call_tool(tool.clone(), arguments.clone()).await?;
                exchanges.lock().expect("exchanges lock").push(Exchange {
                    tool,
                    arguments,
                    outcome,
                });
            }
            Ok(turn.answer.into())
        }
        .boxed()
    }
}
