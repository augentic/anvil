//! CLI self-update support.
//!
//! Owns the channel-aware upgrade primitives the `specify upgrade`
//! command drives: the closed [`InstallChannel`] enum and its
//! filesystem/path-based [`InstallChannel::detect`]; the shared
//! latest-release probe ([`latest_release_tag`]) `/spec:init` will reuse
//! at probe time; the pure per-channel command planner
//! ([`plan_upgrade`]); and the subprocess executor ([`run_plan`]) that
//! actually shells the channel-native upgrade.
//!
//! The planner is pure so `--dry-run` can render the exact commands that
//! would run, and the executor is a separate step so nothing mutates the
//! installed binary until the operator passes `--yes`. The HTTP probe
//! follows the AGENTS.md §"ureq fetch hardening" shape (explicit
//! timeouts, a body cap, `https_only`); probe failures are warnings —
//! they collapse to "no tag resolved" so the upgrade can proceed against
//! HEAD rather than aborting.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specify_error::{Error, Result};

/// `owner/repo` slug for the release probe and the cargo git source.
const REPO_SLUG: &str = "augentic/specify-cli";
/// Git source `cargo install --git` points at.
const REPO_GIT_URL: &str = "https://github.com/augentic/specify-cli";
/// Homebrew tap formula the `brew` channel upgrades.
const BREW_FORMULA: &str = "augentic/tap/specify";
/// Unauthenticated GitHub REST endpoint for the latest release; the
/// `gh`-less probe fallback reads `tag_name` from its JSON.
const RELEASES_LATEST_API: &str =
    "https://api.github.com/repos/augentic/specify-cli/releases/latest";
/// Env override: when set to a non-empty value, [`latest_release_tag`]
/// resolves to it verbatim, skipping `gh`/network. Lets CI and
/// air-gapped installs pin the target deterministically (and keeps the
/// command's tests off the network).
const RELEASE_TAG_ENV: &str = "SPECIFY_RELEASE_TAG";
/// Whole-call cap for the REST probe (DNS + connect + headers + body).
const PROBE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// TCP + TLS handshake cap for the REST probe.
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Body cap for the REST probe response — the JSON is a few `KiB`, but a
/// release with many assets can be larger; 8 `MiB` is a generous ceiling.
const MAX_PROBE_BYTES: u64 = 8 * 1024 * 1024;
/// User-Agent for the REST probe. GitHub rejects requests without one.
const PROBE_USER_AGENT: &str =
    concat!("specify/", env!("CARGO_PKG_VERSION"), " (+https://github.com/augentic/specify-cli)");

/// How the running `specify` binary was installed.
///
/// Resolved by [`InstallChannel::detect`] from the binary's on-disk
/// path; `--channel` on the command overrides detection. Each variant
/// drives a distinct [`plan_upgrade`] strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallChannel {
    /// Installed via `cargo install` — the binary lives under
    /// `$CARGO_HOME/bin/` (or `~/.cargo/bin/` when `CARGO_HOME` is unset).
    Cargo,
    /// Installed via Homebrew — the binary resolves into a Homebrew
    /// Cellar or under a Homebrew prefix.
    Brew,
    /// Installed from a pre-built release archive — the binary lives
    /// under a known system install location (`/usr/local/bin`,
    /// `/opt/specify/`, …).
    Binary,
    /// None of the above — the upgrade command refuses with a structured
    /// `unknown-install-channel` diagnostic instructing a manual upgrade.
    Unknown,
}

impl InstallChannel {
    /// Stable kebab-case wire id (`cargo | brew | binary | unknown`).
    /// Used for the `cli.upgraded` journal `channel` field and text
    /// rendering.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Brew => "brew",
            Self::Binary => "binary",
            Self::Unknown => "unknown",
        }
    }

    /// Classify the running binary by resolving `current_exe` (following
    /// symlinks so a Homebrew shim resolves into its Cellar) and the real
    /// `CARGO_HOME`, then delegating to the pure [`classify`]. Returns
    /// [`InstallChannel::Unknown`] when the executable path cannot be
    /// resolved at all.
    #[must_use]
    pub fn detect() -> Self {
        let Ok(exe) = std::env::current_exe() else {
            return Self::Unknown;
        };
        let resolved = std::fs::canonicalize(&exe).unwrap_or(exe);
        classify(&resolved, cargo_home_dir().as_deref())
    }
}

/// Pure channel classifier — the testable core of
/// [`InstallChannel::detect`].
///
/// `exe_path` is the (ideally symlink-resolved) path to the running
/// binary; `cargo_home` is the resolved `CARGO_HOME` directory (or
/// `~/.cargo`) when known. Classification is purely path-based and
/// ordered: a `$CARGO_HOME/bin` match wins first, then a Homebrew
/// Cellar/prefix, then a known system binary location, else
/// [`InstallChannel::Unknown`].
#[must_use]
pub fn classify(exe_path: &Path, cargo_home: Option<&Path>) -> InstallChannel {
    if let Some(home) = cargo_home
        && exe_path.starts_with(home.join("bin"))
    {
        return InstallChannel::Cargo;
    }
    if is_homebrew_path(exe_path) {
        return InstallChannel::Brew;
    }
    if is_known_binary_path(exe_path) {
        return InstallChannel::Binary;
    }
    InstallChannel::Unknown
}

/// A Homebrew install resolves to `…/Cellar/<formula>/…` or sits under a
/// Homebrew prefix (`/opt/homebrew`, `/usr/local/Homebrew`).
fn is_homebrew_path(path: &Path) -> bool {
    path.components().any(|component| component.as_os_str() == "Cellar")
        || path.starts_with("/opt/homebrew")
        || path.starts_with("/usr/local/Homebrew")
}

/// Known pre-built-binary install locations (RFC §"Channel detection").
fn is_known_binary_path(path: &Path) -> bool {
    path.starts_with("/usr/local/bin") || path.starts_with("/opt/specify")
}

/// Resolve `CARGO_HOME` (when set and non-empty) or fall back to
/// `~/.cargo`. Returns `None` when neither `CARGO_HOME` nor a home
/// directory can be determined.
fn cargo_home_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("CARGO_HOME")
        && !value.is_empty()
    {
        return Some(PathBuf::from(value));
    }
    home_dir().map(|home| home.join(".cargo"))
}

/// Best-effort home directory via `HOME` (unix) or `USERPROFILE`
/// (windows). No `home` crate dependency — the env vars are sufficient
/// for channel detection.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Resolve the latest published release tag (e.g. `v0.43.0`).
///
/// Resolution order (RFC §"Latest-version probe"): the
/// `SPECIFY_RELEASE_TAG` env override first; then `gh release view
/// --json tagName -R <repo>` when `gh` is on PATH; then an
/// unauthenticated GET against the GitHub REST `releases/latest`
/// endpoint. Both `/spec:init`'s optional probe and `specify upgrade`
/// call this.
///
/// Probe failure is a warning, not an error: a missing `gh`, an
/// unreachable network, a non-200 response, or unparseable JSON all
/// collapse to `Ok(None)` so the caller can fall back to a HEAD install
/// with a journal note rather than aborting.
///
/// # Errors
///
/// Returns `Ok(None)` on every soft probe failure; the `Result` is part
/// of the signature so future hard-failure modes (e.g. an explicit
/// `--require-release` flag) can surface without a breaking change.
pub fn latest_release_tag() -> Result<Option<String>> {
    if let Ok(tag) = std::env::var(RELEASE_TAG_ENV) {
        let trimmed = tag.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }
    Ok(gh_release_tag().or_else(http_release_tag))
}

/// `gh release view` probe; `None` when `gh` is absent, exits non-zero,
/// or its JSON has no `tagName`.
fn gh_release_tag() -> Option<String> {
    let output = Command::new("gh")
        .args(["release", "view", "--json", "tagName", "-R", REPO_SLUG])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json = String::from_utf8(output.stdout).ok()?;
    tag_from_json(&json, "tagName")
}

/// Unauthenticated GitHub REST `releases/latest` probe; `None` on any
/// network, status, or parse failure.
fn http_release_tag() -> Option<String> {
    let mut response = release_probe_agent().get(RELEASES_LATEST_API).call().ok()?;
    if response.status().as_u16() != 200 {
        return None;
    }
    let mut body = String::new();
    response
        .body_mut()
        .with_config()
        .limit(MAX_PROBE_BYTES)
        .reader()
        .read_to_string(&mut body)
        .ok()?;
    tag_from_json(&body, "tag_name")
}

/// Read a non-empty `field` string from a release JSON payload.
/// Shared by the `gh` (`tagName`) and REST (`tag_name`) probes.
fn tag_from_json(json: &str, field: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get(field)?.as_str()?.trim();
    if tag.is_empty() { None } else { Some(tag.to_string()) }
}

/// Hardened ureq agent for the REST probe — explicit timeouts,
/// `https_only`, and a User-Agent (AGENTS.md §"ureq fetch hardening").
/// The body cap is applied at read time via [`MAX_PROBE_BYTES`].
fn release_probe_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(PROBE_REQUEST_TIMEOUT))
        .timeout_connect(Some(PROBE_CONNECT_TIMEOUT))
        .https_only(true)
        .http_status_as_error(false)
        .user_agent(PROBE_USER_AGENT)
        .build()
        .into()
}

/// One shell command the upgrade would run. Rendered verbatim by
/// `--dry-run` and executed by [`run_plan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlannedCommand {
    /// Program to spawn (e.g. `cargo`, `brew`).
    pub program: String,
    /// Arguments, in order.
    pub args: Vec<String>,
}

impl PlannedCommand {
    /// Build a planned command from a program and its arguments.
    fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    /// Render the command as a single shell-like line (`program arg …`).
    #[must_use]
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

/// The resolved upgrade action for a channel + target.
///
/// Produced by the pure [`plan_upgrade`] and consumed by [`run_plan`].
/// For the `cargo` / `brew` channels `commands` holds the shell command
/// to run; for `binary` it is empty and `guidance` carries the
/// manual-upgrade instructions (see [`plan_upgrade`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpgradePlan {
    /// Channel this plan upgrades.
    pub channel: InstallChannel,
    /// Commands to run, in order. Empty for the `binary` channel.
    pub commands: Vec<PlannedCommand>,
    /// `true` when the latest tag could not be resolved and the `cargo`
    /// channel falls back to a HEAD install (RFC §"Upgrade actions").
    pub head_fallback: bool,
    /// Manual-upgrade guidance for the `binary` channel; `None` for
    /// `cargo` / `brew`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
}

/// Plan the per-channel upgrade action (RFC §"Upgrade actions").
///
/// Pure: it computes the commands (or guidance) without spawning
/// anything, so `--dry-run` can render them.
///
/// - `cargo`: `cargo install --git <repo> --tag <tag>` pinned to `tag`;
///   when `tag` is `None`, a plain `--git` HEAD install with
///   `head_fallback` set.
/// - `brew`: `brew upgrade <formula>`.
/// - `binary`: no shell command — `commands` is empty and `guidance`
///   describes the download/verify/replace steps. Self-replacing the
///   running binary in-process is deliberately deferred (see the module
///   docs and the change report); [`run_plan`] surfaces a structured
///   diagnostic for this channel rather than a half-built self-overwrite.
///
/// # Errors
///
/// Returns the `unknown-install-channel` diagnostic for
/// [`InstallChannel::Unknown`] (RFC §"Channel detection") — there is no
/// upgrade action to plan.
pub fn plan_upgrade(channel: InstallChannel, tag: Option<&str>) -> Result<UpgradePlan> {
    match channel {
        InstallChannel::Cargo => {
            let command = tag.map_or_else(
                || PlannedCommand::new("cargo", &["install", "--git", REPO_GIT_URL]),
                |tag| {
                    PlannedCommand::new("cargo", &["install", "--git", REPO_GIT_URL, "--tag", tag])
                },
            );
            Ok(UpgradePlan {
                channel,
                commands: vec![command],
                head_fallback: tag.is_none(),
                guidance: None,
            })
        }
        InstallChannel::Brew => Ok(UpgradePlan {
            channel,
            commands: vec![PlannedCommand::new("brew", &["upgrade", BREW_FORMULA])],
            head_fallback: false,
            guidance: None,
        }),
        InstallChannel::Binary => Ok(UpgradePlan {
            channel,
            commands: Vec::new(),
            head_fallback: false,
            guidance: Some(binary_channel_guidance(tag)),
        }),
        InstallChannel::Unknown => Err(unknown_channel_error()),
    }
}

/// Execute a planned upgrade (RFC §"Upgrade actions").
///
/// Spawns each [`PlannedCommand`] in order with inherited stdio so the
/// channel-native tool's output streams to the operator. The `binary`
/// channel returns a structured `binary-channel-manual-upgrade`
/// diagnostic (in-process self-replacement is deferred); the `unknown`
/// channel returns `unknown-install-channel` for completeness, though
/// [`plan_upgrade`] never produces such a plan.
///
/// # Errors
///
/// - `upgrade-command-spawn-failed` when a command cannot be spawned.
/// - `upgrade-command-failed` when a command exits non-zero.
/// - `binary-channel-manual-upgrade` for the `binary` channel.
/// - `unknown-install-channel` for the `unknown` channel.
pub fn run_plan(plan: &UpgradePlan) -> Result<()> {
    match plan.channel {
        InstallChannel::Cargo | InstallChannel::Brew => {
            for command in &plan.commands {
                run_command(command)?;
            }
            Ok(())
        }
        InstallChannel::Binary => Err(Error::Diag {
            code: "binary-channel-manual-upgrade",
            detail: plan.guidance.clone().unwrap_or_else(|| binary_channel_guidance(None)),
        }),
        InstallChannel::Unknown => Err(unknown_channel_error()),
    }
}

/// Spawn one planned command, mapping a spawn failure and a non-zero
/// exit to structured diagnostics.
fn run_command(command: &PlannedCommand) -> Result<()> {
    let status = Command::new(&command.program).args(&command.args).status().map_err(|source| {
        Error::Diag {
            code: "upgrade-command-spawn-failed",
            detail: format!("failed to spawn `{}`: {source}", command.display()),
        }
    })?;
    if !status.success() {
        return Err(Error::Diag {
            code: "upgrade-command-failed",
            detail: format!("`{}` exited with {status}", command.display()),
        });
    }
    Ok(())
}

/// Manual-upgrade guidance for the `binary` channel.
fn binary_channel_guidance(tag: Option<&str>) -> String {
    let release = tag.map_or_else(
        || format!("https://github.com/{REPO_SLUG}/releases/latest"),
        |tag| format!("https://github.com/{REPO_SLUG}/releases/tag/{tag}"),
    );
    format!(
        "binary-channel self-replacement is not automated; download the release archive for your \
         platform from {release}, verify its checksum sidecar, and replace the running binary, or \
         re-install via cargo (`cargo install --git {REPO_GIT_URL}`) or Homebrew \
         (`brew upgrade {BREW_FORMULA}`)"
    )
}

/// The `unknown-install-channel` diagnostic (RFC §"Channel detection").
fn unknown_channel_error() -> Error {
    Error::Diag {
        code: "unknown-install-channel",
        detail: "could not determine how specify was installed; upgrade it the way you installed \
                 it (cargo, Homebrew, or release archive), or pass --channel to override detection"
            .to_string(),
    }
}

#[cfg(test)]
mod tests;
