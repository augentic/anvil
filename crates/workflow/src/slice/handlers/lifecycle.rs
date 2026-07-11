//! Slice lifecycle verbs: create / transition / drop.

use std::io::Write;
use std::path::PathBuf;

use error::Error;
use jiff::Timestamp;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Render};
use crate::slice::{CreateIfExists, Created, LifecycleStatus, actions as slice_actions};

// ---------------------------------------------------------------------------
// slice create
// ---------------------------------------------------------------------------

/// Wire input for `slice create`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateInput {
    /// Kebab-case slice name.
    pub name: String,
    /// Target-adapter identifier; defaults to the value in
    /// `.specify/project.yaml`.
    #[serde(default)]
    pub target: Option<String>,
    /// Behaviour when `<slices_dir>/<name>/` already exists — `fail`
    /// (default), `continue`, or `restart`. Typed on both transports;
    /// an unknown value fails the one deserialize path.
    #[serde(default)]
    pub if_exists: CreateIfExists,
}

/// `specify slice create <name>` — create a slice directory with an
/// initial `metadata.yaml`.
#[derive(Clone, Copy, Debug)]
pub struct Create;

impl<P: Anchor> Operation<P> for Create {
    type Error = crate::handler::Error;
    type Input = CreateInput;
    type Output = Created;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let CreateInput {
            name,
            target,
            if_exists,
        } = input;
        let target_value = target.map_or_else(
            || {
                cx.config.adapter.clone().ok_or_else(|| Error::Diag {
                    code: "slice-create-target-missing",
                    detail: "no project target declared; pass `--target <id>` explicitly or \
                             run `specify init <adapter>` first (workspaces cannot create \
                             changes)"
                        .to_string(),
                })
            },
            Ok,
        )?;
        let slices_dir = cx.layout().slices_dir();
        std::fs::create_dir_all(&slices_dir).map_err(Error::Io)?;

        let outcome =
            slice_actions::create(&slices_dir, &name, &target_value, if_exists, cx.now())?;
        Ok(outcome)
    }
}

impl Render for Created {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.created {
            writeln!(w, "Created slice {}", self.dir.display())?;
        } else {
            writeln!(w, "Reusing existing slice {}", self.dir.display())?;
        }
        if self.restarted {
            writeln!(w, "  (previous directory was removed)")?;
        }
        writeln!(w, "  target: {}", self.metadata.target)?;
        writeln!(w, "  status: {}", self.metadata.status)
    }
}

// ---------------------------------------------------------------------------
// slice transition
// ---------------------------------------------------------------------------

/// Wire input for `slice transition`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionInput {
    /// Slice name.
    pub name: String,
    /// Target status (`refining`, `refined`, `built`, or `dropped`).
    /// `merged` is reserved for `specify slice merge run`.
    pub target: LifecycleStatus,
}

/// `specify slice transition <name> <target>`.
///
/// Apply a validated lifecycle transition. `merged` is not a valid
/// target: the only legal writer of `Merged` is `specify slice merge
/// run`, which performs the spec merge, status transition, and archive
/// move atomically.
#[derive(Clone, Copy, Debug)]
pub struct Transition;

impl<P: Anchor> Operation<P> for Transition {
    type Error = crate::handler::Error;
    type Input = TransitionInput;
    type Output = TransitionBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let TransitionInput { name, target } = input;
        if matches!(target, LifecycleStatus::Merged) {
            return Err(Error::Argument {
                flag: "<target>",
                detail: "use `specify slice merge run` to reach `merged`".to_string(),
            }
            .into());
        }
        let slice_dir = cx.layout().slices_dir().join(&name);
        let metadata = slice_actions::transition(&slice_dir, target, cx.now())?;
        Ok(TransitionBody {
            name,
            status: metadata.status,
            defined_at: metadata.defined_at,
            completed_at: metadata.completed_at,
            merged_at: metadata.merged_at,
            dropped_at: metadata.dropped_at,
        })
    }
}

/// Success envelope for `slice transition`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionBody {
    /// Slice name.
    pub name: String,
    /// Status after the transition.
    pub status: LifecycleStatus,
    /// Lifecycle timestamps as persisted.
    #[serde(with = "crate::serde_time::rfc3339_opt")]
    pub defined_at: Option<Timestamp>,
    /// See `defined_at`.
    #[serde(with = "crate::serde_time::rfc3339_opt")]
    pub completed_at: Option<Timestamp>,
    /// See `defined_at`.
    #[serde(with = "crate::serde_time::rfc3339_opt")]
    pub merged_at: Option<Timestamp>,
    /// See `defined_at`.
    #[serde(with = "crate::serde_time::rfc3339_opt")]
    pub dropped_at: Option<Timestamp>,
}

impl Render for TransitionBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "{}: status = {}", self.name, self.status)
    }
}

// ---------------------------------------------------------------------------
// slice drop
// ---------------------------------------------------------------------------

/// Wire input for `slice drop`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropInput {
    /// Slice name.
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and
    /// the archive path.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `specify slice drop <name>` — transition a slice to `dropped` and
/// archive it.
#[derive(Clone, Copy, Debug)]
pub struct Drop;

impl<P: Anchor> Operation<P> for Drop {
    type Error = crate::handler::Error;
    type Input = DropInput;
    type Output = DropBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let DropInput { name, reason } = input;
        let slice_dir = cx.layout().slices_dir().join(&name);
        let archive_dir = cx.layout().archive_dir();
        let (metadata, archive_path) =
            slice_actions::discard(&slice_dir, &archive_dir, reason.as_deref(), cx.now())?;
        Ok(DropBody {
            name,
            status: metadata.status,
            archive_path,
            drop_reason: metadata.drop_reason,
        })
    }
}

/// Success envelope for `slice drop`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropBody {
    /// Slice name.
    pub name: String,
    /// Status after the drop.
    pub status: LifecycleStatus,
    /// Path of the archived slice directory (serialised as its display
    /// string).
    pub archive_path: PathBuf,
    /// Recorded drop reason, when supplied.
    pub drop_reason: Option<String>,
}

impl Render for DropBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "{}: dropped and archived to {}", self.name, self.archive_path.display())?;
        if let Some(r) = &self.drop_reason {
            writeln!(w, "  reason: {r}")?;
        }
        Ok(())
    }
}
