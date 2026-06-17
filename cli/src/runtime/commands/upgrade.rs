//! `specify upgrade` — channel-aware CLI self-update.
//!
//! Bootstrap verb: it operates on the running binary, not a project, so
//! it never calls [`ProjectConfig::load`] (the bootstrap carve-out).
//! It resolves the install channel (flag override or
//! [`InstallChannel::detect`]), probes the latest release, and either
//! reports the plan (`--dry-run`) or, with `--yes`, runs the
//! channel-native upgrade and journals `cli.upgraded`.

use std::io::Write;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::journal::{self, Event, EventKind};
use specify_workflow::upgrade::{self, InstallChannel, PlannedCommand, UpgradePlan};

use crate::runtime::cli::{ChannelArg, Format};
use crate::runtime::output;

/// Display value for an unresolved target version (HEAD fallback).
const HEAD_TARGET: &str = "HEAD";

pub(super) fn run(format: Format, channel: ChannelArg, yes: bool, dry_run: bool) -> Result<()> {
    let resolved = resolve_channel(channel);
    let from = env!("CARGO_PKG_VERSION").to_string();
    let tag = upgrade::latest_release_tag()?;
    let to = target_display(tag.as_deref());
    // `plan_upgrade` errors for `Unknown`, so the carve-out diagnostic
    // surfaces on both the dry-run and apply paths.
    let plan = upgrade::plan_upgrade(resolved, tag.as_deref())?;

    if dry_run {
        return emit(format, &Body::planned(resolved, from, to, &plan));
    }

    if !yes {
        return Err(Error::Diag {
            code: "upgrade-consent-required",
            detail: "refusing to upgrade the binary without consent; pass --yes to apply or \
                     --dry-run to preview"
                .to_string(),
        });
    }

    upgrade::run_plan(&plan)?;
    let journaled = journal_upgrade(&from, &to, resolved)?;
    emit(format, &Body::applied(resolved, from, to, &plan, journaled))
}

/// Map the clap `--channel` flag onto an [`InstallChannel`], resolving
/// `auto` via filesystem detection.
fn resolve_channel(channel: ChannelArg) -> InstallChannel {
    match channel {
        ChannelArg::Auto => InstallChannel::detect(),
        ChannelArg::Cargo => InstallChannel::Cargo,
        ChannelArg::Brew => InstallChannel::Brew,
        ChannelArg::Binary => InstallChannel::Binary,
    }
}

/// Render the resolved target version: the tag with any leading `v`
/// stripped, or [`HEAD_TARGET`] when no tag resolved.
fn target_display(tag: Option<&str>) -> String {
    tag.map_or_else(
        || HEAD_TARGET.to_string(),
        |tag| tag.strip_prefix('v').unwrap_or(tag).to_string(),
    )
}

/// Append `cli.upgraded` to the CWD project's journal when a `.specify/`
/// root is discoverable; returns whether an event was written.
///
/// The journal lives under a project's `.specify/`, but `upgrade` may
/// run outside any project. When no root is found from the CWD the
/// upgrade still succeeds and journaling is skipped silently. The
/// running process is the *old* binary, so per the RFC the event is
/// emitted with `from` = the pre-upgrade `CARGO_PKG_VERSION` and `to` =
/// the resolved target.
fn journal_upgrade(from: &str, to: &str, channel: InstallChannel) -> Result<bool> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let Some(root) = ProjectConfig::find_root(&cwd) else {
        return Ok(false);
    };
    let event = Event::new(
        Timestamp::now(),
        EventKind::CliUpgraded {
            from: from.to_string(),
            to: to.to_string(),
            channel: channel.as_str().to_string(),
        },
    );
    journal::append_batch(Layout::new(&root), std::slice::from_ref(&event))?;
    Ok(true)
}

/// Wire-stable `specify upgrade` envelope (text + JSON). Change G's
/// `/spec:init` skill parses `channel`, `to`, and `commands` from the
/// `--dry-run --format json` shape.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "JSON wire DTO: each bool is a stable, independently consumed field on the upgrade envelope."
)]
struct Body {
    /// Schema marker; `1` for this shape.
    version: u32,
    /// Resolved install channel (`cargo | brew | binary`).
    channel: InstallChannel,
    /// Pre-upgrade version (`CARGO_PKG_VERSION`).
    from: String,
    /// Target version — the resolved release (leading `v` stripped) or
    /// `HEAD` when no release tag resolved.
    to: String,
    /// `true` for `--dry-run`; `false` on the apply path.
    dry_run: bool,
    /// `true` when the channel strategy actually ran (apply path only).
    applied: bool,
    /// `true` when the `cargo` channel fell back to a HEAD install
    /// because the latest tag could not be resolved.
    head_fallback: bool,
    /// `true` when a `cli.upgraded` journal event was written (apply
    /// path with a discoverable project root).
    journaled: bool,
    /// Commands that would run (`--dry-run`) or did run (apply). Empty
    /// for the `binary` channel.
    commands: Vec<PlannedCommand>,
    /// Manual-upgrade guidance for the `binary` channel; absent
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    guidance: Option<String>,
}

impl Body {
    /// Dry-run envelope: nothing ran, nothing was journaled.
    fn planned(channel: InstallChannel, from: String, to: String, plan: &UpgradePlan) -> Self {
        Self {
            version: 1,
            channel,
            from,
            to,
            dry_run: true,
            applied: false,
            head_fallback: plan.head_fallback,
            journaled: false,
            commands: plan.commands.clone(),
            guidance: plan.guidance.clone(),
        }
    }

    /// Apply envelope: the channel strategy ran successfully.
    fn applied(
        channel: InstallChannel, from: String, to: String, plan: &UpgradePlan, journaled: bool,
    ) -> Self {
        Self {
            version: 1,
            channel,
            from,
            to,
            dry_run: false,
            applied: true,
            head_fallback: plan.head_fallback,
            journaled,
            commands: plan.commands.clone(),
            guidance: plan.guidance.clone(),
        }
    }
}

fn write_text(w: &mut dyn Write, body: &Body) -> std::io::Result<()> {
    let verb = if body.dry_run { "Would upgrade" } else { "Upgraded" };
    writeln!(w, "{verb} via {} channel: {} -> {}", body.channel.as_str(), body.from, body.to)?;
    if body.head_fallback {
        writeln!(w, "  (latest release tag unresolved; installing from HEAD)")?;
    }
    for command in &body.commands {
        let label = if body.dry_run { "would run" } else { "ran" };
        writeln!(w, "  {label}: {}", command.display())?;
    }
    if let Some(guidance) = &body.guidance {
        writeln!(w, "  {guidance}")?;
    }
    if body.journaled {
        writeln!(w, "  journaled cli.upgraded")?;
    }
    Ok(())
}

fn emit(format: Format, body: &Body) -> Result<()> {
    output::emit(&mut std::io::stdout().lock(), format, body, write_text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_display_strips_v_prefix() {
        assert_eq!(target_display(Some("v0.43.0")), "0.43.0");
        assert_eq!(target_display(Some("0.43.0")), "0.43.0");
    }

    #[test]
    fn target_display_head_when_unresolved() {
        assert_eq!(target_display(None), HEAD_TARGET);
    }

    #[test]
    fn resolve_channel_maps_forced_flags() {
        assert_eq!(resolve_channel(ChannelArg::Cargo), InstallChannel::Cargo);
        assert_eq!(resolve_channel(ChannelArg::Brew), InstallChannel::Brew);
        assert_eq!(resolve_channel(ChannelArg::Binary), InstallChannel::Binary);
    }
}
