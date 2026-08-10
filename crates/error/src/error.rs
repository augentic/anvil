//! `Error` enum and saphyr-error conversions.
//!
//! The `YamlDe` / `YamlSer` variants flatten `serde_saphyr`'s two error
//! types directly into the crate's error surface.

use std::borrow::Cow;

/// Structured error type for all `emery-*` crates.
///
/// Variants carry enough context for the CLI to assign exit codes and
/// choose an output format without string-parsing.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs, reason = "variant-level docs cover self-explanatory error fields")]
pub enum Error {
    /// The `.emery/project.yaml` file is missing.
    #[error("not initialized: .emery/project.yaml not found")]
    NotInitialized,

    /// Structured catch-all for diagnostics that don't have a dedicated
    /// variant. The `code` is a stable kebab-case discriminant surfaced
    /// in JSON envelopes; `detail` is the human-readable message.
    /// Promote a recurring `Diag` site to its own variant once the call
    /// shape stabilises.
    #[error("{code}: {detail}")]
    Diag { code: &'static str, detail: String },

    /// A user-supplied CLI argument is invalid for reasons clap cannot
    /// catch (kebab-case names, mutually exclusive flag combinations,
    /// unknown enum keys, etc.). Carries the offending flag/value plus a
    /// human-readable detail. Prefer this over [`Error::Diag`] for
    /// argument-shape validation so the CLI can map it onto the
    /// argument-error exit code.
    #[error("invalid argument {flag}: {detail}")]
    Argument { flag: &'static str, detail: String },

    /// A workflow-gating validation surface failed. Payload-free: the
    /// rendered findings (a `DiagnosticReport`) are emitted to stdout by
    /// the handler; this variant only carries the stable kebab `code`
    /// (the JSON `error` discriminant) and a human-readable `detail`,
    /// and routes to exit code 2 (`Exit::ValidationFailed`). Construct
    /// via [`Self::validation_failed`].
    #[error("{code}: {detail}")]
    Validation { code: Cow<'static, str>, detail: String },

    /// The installed CLI version is older than the project floor.
    #[error("emery version {found} is older than the project floor {required}; upgrade the CLI")]
    CliTooOld { required: String, found: String },

    /// The installed CLI version is older than an adapter's declared
    /// host-CLI compatibility floor (the `emery-floor` describe
    /// key). Routes to exit 3 (`Exit::VersionTooOld`) like
    /// [`Self::CliTooOld`] but carries the distinct `adapter-cli-too-old`
    /// discriminant so the operator sees which adapter outran the binary.
    #[error(
        "emery version {found} is older than the floor {required} required by adapter {adapter}; upgrade the CLI"
    )]
    AdapterCliTooOld { adapter: String, required: String, found: String },

    /// A required artifact was not found at the expected path.
    #[error("{kind} not found at {}", path.display())]
    ArtifactNotFound { kind: &'static str, path: std::path::PathBuf },

    /// A filesystem operation failed. The `op` field is a stable
    /// kebab-case suffix that, prefixed with `filesystem-`, becomes the
    /// JSON envelope's `error` discriminant (e.g. `filesystem-readdir`).
    #[error("filesystem-{op}: {} ({source})", path.display())]
    Filesystem {
        op: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An I/O error propagated from the standard library.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A YAML deserialization error (e.g. `serde_saphyr::from_str`).
    /// Library crates rely on `?`-propagation; the variant docstring is
    /// the canonical "you don't have to care which `serde_saphyr` API
    /// tripped" — match on either YAML variant when that distinction is
    /// irrelevant.
    #[error(transparent)]
    YamlDe(#[from] serde_saphyr::Error),

    /// A YAML serialization error (e.g. `serde_saphyr::to_string`).
    #[error(transparent)]
    YamlSer(#[from] serde_saphyr::ser::Error),
}

impl Error {
    /// Long-form recovery hint for tightened diagnostics. Returns
    /// `None` when the variant has no actionable follow-up beyond the
    /// `#[error("…")]` body.
    ///
    /// The renderer in `crates/transport/src/command/output.rs` calls this to surface guidance
    /// alongside the kebab discriminant on a TTY, while keeping the
    /// machine-readable JSON envelope compact. New hints land here
    /// (typed-arm for typed variants; `Self::Diag { code, .. }` arm for
    /// `Diag`-routed sites), not in the renderer.
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::NotInitialized => Some("run `emery init <adapter>` to scaffold .emery/ first"),
            Self::ArtifactNotFound {
                kind: "plan.yaml", ..
            } => Some(
                "author a plan first: run /emery:plan, or `emery plan author <name> --intent \"...\"`",
            ),
            Self::CliTooOld { .. } => Some(
                "update the installed binary through its install channel: `curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh` (or `brew upgrade emery`), then rerun the command",
            ),
            Self::AdapterCliTooOld { .. } => Some(
                "update the installed binary through its install channel: `curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh` (or `brew upgrade emery`); if the adapter itself is stale instead, `emery adapter update <name>` pulls its newest published version",
            ),
            Self::Diag { code, .. } => match *code {
                "plan-has-outstanding-work" => Some(
                    "complete or drop the listed entries, or rerun with --force to archive anyway",
                ),
                "slice-not-found" => {
                    Some("run `emery slice list` to see every slice and its status")
                }
                "plan-entry-not-found" => {
                    Some("run `emery plan status` to see the plan's entries and the next action")
                }
                "plan-source-unknown" => Some(
                    "bind the source at plan time (`emery plan author --source <key>=<adapter>:<path>`) or pass one of the bound keys",
                ),
                "slice-lifecycle" => Some(
                    "run `emery slice list` for each slice's current status, then re-run `emery plan execute` — the loop drives the missing phase",
                ),
                "plan-transition" => Some(
                    "per-entry status is projected from facts written by `emery plan execute` (the advance claim writes `in-progress`, the merge phase writes `done`); there is no direct status writer",
                ),
                "plan-already-exists" => Some(
                    "rerun with --force to replace the plan, or continue the existing one (`emery plan status`)",
                ),
                "slice-already-exists" => Some(
                    "continue the existing slice (`emery slice list` shows its status) or abandon it with `emery plan drop <slice>`",
                ),
                _ => None,
            },
            Self::Validation { code, .. } => match code.as_ref() {
                "init-adapter-required" => {
                    Some("`emery init <adapter>` scaffolds the project.\nsee: docs/init.md")
                }
                "slice-claim-conflict" => Some(
                    "claim a different slice, or wait for the current owner to release / retract their claim",
                ),
                "plan-epoch-stale" => Some(
                    "re-run `emery plan execute` — it opens a fresh epoch over the current plan and specs; durable deferrals carry over, nothing needs re-supplying",
                ),
                "plan-gaps-unresolved" => Some(
                    "resolve open gaps at their source and re-run `emery plan execute`, or defer them durably with `emery plan defer <slice>/<req> --reason \"<why>\"` (or run under `--gap-policy defer`)\nsee: docs/reference/diagnostics.md",
                ),
                "plan-deferral-invalid" => Some(
                    "defer names open `[unknown]` / `[conflict]` rows from `emery plan gaps` as `<slice>/<req>` and requires `--reason`; `--retract` must name a live deferral",
                ),
                "guest-marker-held" => Some(
                    "wait for the running execute session; if no run is live (a crash left the marker behind), delete `.emery/guest.lock` and retry\nsee: docs/how-to/recover-from-a-stale-guest-lock.md",
                ),
                _ => None,
            },
            _ => None,
        }
    }

    /// Kebab-case identifier used in structured CLI error payloads.
    ///
    /// Most arms borrow a `&'static str` literal at zero cost;
    /// [`Self::Filesystem`] is the lone owned arm, composing
    /// `filesystem-<op>`.
    #[must_use]
    pub fn variant_str(&self) -> Cow<'static, str> {
        match self {
            Self::NotInitialized => Cow::Borrowed("not-initialized"),
            Self::Diag { code, .. } => Cow::Borrowed(*code),
            Self::Argument { .. } => Cow::Borrowed("argument"),
            Self::Validation { code, .. } => code.clone(),
            Self::CliTooOld { .. } => Cow::Borrowed("emery-version-too-old"),
            Self::AdapterCliTooOld { .. } => Cow::Borrowed("adapter-cli-too-old"),
            Self::ArtifactNotFound { .. } => Cow::Borrowed("artifact-not-found"),
            Self::Filesystem { op, .. } => Cow::Owned(format!("filesystem-{op}")),
            Self::Io(_) => Cow::Borrowed("io"),
            Self::YamlDe(_) | Self::YamlSer(_) => Cow::Borrowed("yaml"),
        }
    }

    /// Build a payload-free `Validation` failure that lands on
    /// `Exit::ValidationFailed` (exit 2).
    ///
    /// `code` is the stable kebab discriminant surfaced as the JSON
    /// `error` field (and by [`Self::variant_str`]); `rule` (the
    /// human-readable invariant) and `detail` (the specific
    /// explanation) are folded into the rendered message.
    #[must_use]
    pub fn validation_failed(
        code: impl Into<Cow<'static, str>>, rule: impl Into<String>, detail: impl Into<String>,
    ) -> Self {
        let rule = rule.into();
        let detail = detail.into();
        let detail = if rule.is_empty() { detail } else { format!("{rule}: {detail}") };
        Self::Validation {
            code: code.into(),
            detail,
        }
    }
}
