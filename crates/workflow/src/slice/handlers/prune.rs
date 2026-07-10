//! `specify archive prune` — the retention GC over
//! `.specify/archive/`. Pure filesystem inside the project root, so
//! the verb runs on every transport.

use std::io::Write;

use error::Error;
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Out, Render};
use crate::slice::actions::{Retention, prune};

/// Wire input for `archive prune`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PruneInput {
    /// Keep at most this many most-recent archived slices.
    #[serde(default)]
    pub keep: Option<usize>,
    /// Prune archived slices older than this many days.
    #[serde(default)]
    pub older_than: Option<i64>,
    /// Report what would be pruned without removing anything.
    #[serde(default)]
    pub dry_run: bool,
}

/// `specify archive prune` — prune archived slice folders that fall
/// outside the supplied retention bounds.
///
/// The archive is a prunable convenience cache, not the system of
/// record — git history of `.specify/specs/` plus the
/// `slice.archive.created` journal entries are. At least one of
/// `keep` / `older_than` is required; a folder is pruned when it falls
/// outside the newest-`keep` window or is older than `older_than`
/// days.
#[derive(Clone, Copy, Debug)]
pub struct Prune {
    input: PruneInput,
}

impl<P: Anchor> Handler<P> for Prune {
    type Error = crate::handler::Error;
    type Input = PruneInput;
    type Output = Out<PruneBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        if input.keep.is_none() && input.older_than.is_none() {
            return Err(Error::Argument {
                flag: "--keep/--older-than",
                detail:
                    "supply at least one retention bound (`--keep <n>` and/or `--older-than <days>`)"
                        .to_string(),
            }
            .into());
        }
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let PruneInput {
            keep,
            older_than,
            dry_run,
        } = self.input;
        let retention = Retention {
            keep,
            max_age_days: older_than,
        };
        let archive_dir = cx.archive_dir();
        let candidates = prune::scan(&archive_dir, retention, cx.now())?;
        if !dry_run {
            prune::prune(&candidates)?;
        }
        let pruned: Vec<String> = candidates.into_iter().map(|e| e.name).collect();

        Ok(Reply::ok(Out(PruneBody { dry_run, pruned })))
    }
}

/// Success envelope for `archive prune`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PruneBody {
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Names of the pruned (or would-be-pruned) archive folders.
    pub pruned: Vec<String>,
}

impl Render for PruneBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let verb = if self.dry_run { "Would prune" } else { "Pruned" };
        if self.pruned.is_empty() {
            return writeln!(w, "Nothing to prune.");
        }
        writeln!(w, "{verb} {} archived slice(s):", self.pruned.len())?;
        for name in &self.pruned {
            writeln!(w, "  {name}")?;
        }
        Ok(())
    }
}
