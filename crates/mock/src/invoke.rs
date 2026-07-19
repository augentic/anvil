//! Provider-neutral typed operation invocation for the workflow
//! suites.
//!
//! Typed operation execution alongside the CLI-argv path in
//! `native::command`, so a suite can inspect an operation's typed
//! output (e.g. the execute body's phases) without reimplementing
//! router machinery. Generic over the provider: [`native::Provider`]
//! and suite-local providers call through the same entry.

use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;

/// Invoke one operation against `provider`. The operation type leads
/// the generics so call sites write `run::<Op, _, _>(&provider, …)`.
///
/// # Errors
///
/// Propagates the operation's typed failure.
pub async fn run<R, B, P>(provider: &P, input: R::Input) -> Result<B, project::handler::Error>
where
    P: Clone + Send + Sync + 'static,
    R: Operation<P, Output = B, Error = project::handler::Error>,
    B: Send,
{
    Invoker::new("specify", provider.clone()).invoke::<R>(Invocation::new(input)).await
}
