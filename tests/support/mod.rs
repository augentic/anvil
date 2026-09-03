//! Shared scenario plumbing for the root suites: one provider that
//! scripts every capability (`Model`, `Source`, `Plugins`, storage), the
//! live command grammar, and the in-process CLI runners.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use emery_adapter::types::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{DispatchError, Source};
use emery_engine::cli::{self, CommandResponse};
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, StateStore};
use omnia_test::guest::{Memory, Scripted, ScriptedLoader};
use serde_json::Value;

/// The default scripted requirement statement.
pub const GREETING: &str = "GET /greeting returns the static string 'hello'.";

/// Dispatched `(routed id, input)` pairs, in call order.
pub type Recorded = Vec<(String, SourceInput)>;

/// A full-length `sha256:` digest from one repeated hex pair.
pub fn digest(pair: &str) -> Digest {
    format!("sha256:{}", pair.repeat(32)).parse().expect("a valid digest")
}

/// Scripted `Source`: per-key evidence, per-adapter floors, and a
/// record of every dispatch. An unscripted key answers the greeting
/// requirement as documentation evidence.
#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    /// Extract outcomes keyed by binding key.
    pub evidence: BTreeMap<String, Result<Evidence, DispatchError>>,
    /// `emery` floors keyed by adapter name.
    pub floors: BTreeMap<String, String>,
    /// Every extract dispatch, recorded for call assertions.
    pub calls: Arc<Mutex<Recorded>>,
}

/// A claim of `kind` carrying one required extra.
pub fn claim(kind: ClaimKind, id: &str, extra: (&str, &str)) -> Claim {
    let mut extras = serde_json::Map::new();
    extras.insert(extra.0.to_string(), Value::String(extra.1.to_string()));
    Claim {
        kind,
        id: Some(id.to_string()),
        path: None,
        synopsis: None,
        backing: Some(Backing::Payload(extra.1.to_string())),
        extras,
    }
}

/// A requirement claim carrying its required `statement` extra.
pub fn requirement(id: &str, statement: &str) -> Claim {
    claim(ClaimKind::Requirement, id, ("statement", statement))
}

/// An evidence document over `claims`.
pub const fn evidence(authority: Authority, claims: Vec<Claim>) -> Evidence {
    Evidence { authority, claims }
}

/// The scripted provider behind every root scenario.
#[derive(Debug)]
pub struct Provider<S = Memory> {
    /// FIFO-scripted model answers.
    pub model: Scripted,
    /// The scripted `Source`.
    pub source: SourceScript,
    /// The scripted `Plugins` loader: an unscripted, unpinned package
    /// resolves to the fixed `digest("ab")` the `specify` scenarios
    /// assert in their envelopes; a pin that disagrees with a scripted
    /// digest refuses `refused`, mirroring the host's
    /// verify-before-validate step (proved against the real loader in
    /// the component rung, `examples/component/tests/component.rs`).
    pub plugins: ScriptedLoader,
    /// The scripted storage pair.
    pub storage: Arc<S>,
}

impl Provider<Memory> {
    /// A provider answering `answers` over fresh in-memory storage.
    pub fn answering<T: Into<String>>(answers: impl IntoIterator<Item = T>) -> Self {
        Self::over(Arc::new(Memory::default()), answers)
    }

    /// A provider whose model is never dispatched.
    pub fn idle() -> Self {
        Self::answering(Vec::<String>::new())
    }
}

impl<S> Provider<S> {
    /// A provider answering `answers` over `storage`.
    pub fn over<T: Into<String>>(storage: Arc<S>, answers: impl IntoIterator<Item = T>) -> Self {
        Self {
            model: Scripted::answering(answers),
            source: SourceScript::default(),
            plugins: ScriptedLoader::default().defaulting(digest("ab")),
            storage,
        }
    }
}

impl<S> Clone for Provider<S> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            source: self.source.clone(),
            plugins: self.plugins.clone(),
            storage: Arc::clone(&self.storage),
        }
    }
}

omnia_test::delegate!(impl[S: StateStore + BlobStore + Send + Sync + 'static] Provider<S> {
    Model => model,
    Plugins => plugins,
    StateStore + BlobStore => storage,
});

impl<S: Send + Sync + 'static> Source for Provider<S> {
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        self.source.calls.lock().expect("calls").push((id.to_string(), input.clone()));
        let outcome = self.source.evidence.get(&input.key).cloned().unwrap_or_else(|| {
            Ok(evidence(
                Authority::Documentation,
                vec![requirement("greeting.behaviour", GREETING)],
            ))
        });
        std::future::ready(outcome)
    }

    fn metadata(&self, id: &str) -> SourceMetadata {
        // Routed ids are `source:<name>` or a package reference
        // (`<namespace>:<name>@<version>`); floors key on the name.
        let name = id.split_once('@').map_or(id, |(stem, _)| stem);
        let name = name.rsplit_once(':').map_or(name, |(_, stem)| stem);
        SourceMetadata {
            emery_floor: self.source.floors.get(name).cloned(),
        }
    }
}

/// The live command grammar bound over `provider`.
pub fn router<S>(provider: &Provider<S>) -> cli::Cli<Provider<S>>
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    cli::router(provider.clone())
}

/// Runs one CLI invocation in-process, returning the raw response.
pub async fn cli<S>(provider: &Provider<S>, argv: &[&str]) -> CommandResponse
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    router(provider).execute(argv.iter().copied()).await
}

/// Runs one CLI invocation and asserts success.
pub async fn cli_ok<S>(provider: &Provider<S>, argv: &[&str]) -> CommandResponse
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli(provider, argv).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}
