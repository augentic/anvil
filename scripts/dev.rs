#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
---

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{bail, ensure, Context as _, Result};
use serde::Deserialize;

struct Dev {
    framework: PathBuf,
    adapters: PathBuf,
}

/// The engine crates the standalone native harness pins by revision;
/// cross-repo developer commands override each with this checkout's
/// working tree through generated `--config` patch flags.
const ENGINE_CRATES: [&str; 6] =
    ["artifacts", "error", "scenario", "schema", "transport", "workflow"];

/// The git source the native harness pins its engine crates to.
const ENGINE_GIT: &str = "https://github.com/augentic/specify.git";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("dev: {error:#}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let current = env::current_dir().context("reading the current directory")?;
    let framework = env_path("SPECIFY_FRAMEWORK")
        .map_or_else(|| current.clone(), |path| absolute(&current, &path));
    let adapters = env_path("SPECIFY_ADAPTERS")
        .map_or_else(|| framework.join("../specify-adapters"), |path| absolute(&current, &path));
    let dev = Dev { framework, adapters };

    let mut args = env::args_os().skip(1);
    let first = args.next();
    let command = match first {
        Some(separator) if separator == "--" => args.next(),
        command => command,
    }
    .and_then(|value| value.into_string().ok())
    .unwrap_or_default();
    let rest: Vec<_> = args.collect();
    match command.as_str() {
        "doctor" => dev.doctor(&rest),
        "check" => dev.check(&rest),
        "run" => dev.run_project(&rest),
        "live" => dev.live(&rest),
        "full" => {
            ensure!(rest.is_empty(), "full accepts no arguments");
            dev.full()
        }
        _ => bail!("unknown command `{command}`; expected doctor, check, run, live, or full"),
    }
}

impl Dev {
    fn doctor(&self, args: &[OsString]) -> Result<()> {
        let live = match args {
            [] => false,
            [flag] if flag == "--live" => true,
            _ => bail!("doctor accepts only --live"),
        };
        let mut failed = false;

        println!("sibling layout");
        report(
            self.framework.join("Makefile.toml").is_file()
                && self.framework.join("crates/workflow").is_dir(),
            &format!("specify checkout at {}", self.framework.display()),
            &format!("clone augentic/specify there or set SPECIFY_FRAMEWORK=<path>"),
            &mut failed,
        );
        report(
            self.adapters.join("targets").is_dir() && self.adapters.join("sources").is_dir(),
            &format!("specify-adapters checkout at {}", self.adapters.display()),
            "clone augentic/specify-adapters there or set SPECIFY_ADAPTERS=<path>",
            &mut failed,
        );

        println!("toolchain");
        for tool in ["cargo", "rustup", "git"] {
            report(
                on_path(tool),
                &format!("{tool} on PATH"),
                &format!("install {tool}"),
                &mut failed,
            );
        }
        report(
            succeeds(Command::new("cargo").args(["make", "--version"])),
            "cargo-make",
            "cargo install cargo-make",
            &mut failed,
        );
        report(
            succeeds(Command::new("cargo").args(["nextest", "--version"])),
            "cargo-nextest",
            "cargo install cargo-nextest",
            &mut failed,
        );
        let targets = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .context("listing installed Rust targets")?;
        report(
            targets.status.success()
                && String::from_utf8_lossy(&targets.stdout)
                    .lines()
                    .any(|line| line == "wasm32-wasip2"),
            "wasm32-wasip2 target",
            "rustup target add wasm32-wasip2",
            &mut failed,
        );

        println!("model backend");
        if on_path("cursor-agent") {
            report(true, "cursor-agent on PATH", "", &mut failed);
            if live {
                println!("  ..    live credential probe (one real model call)");
                let output = Command::new("cursor-agent")
                    .args(["--print", "Reply with the single word OK"])
                    .output()
                    .context("probing cursor-agent credentials")?;
                report(
                    output.status.success()
                        && (!output.stdout.is_empty() || !output.stderr.is_empty()),
                    "command-mode credentials",
                    "run `cursor-agent login` or export CURSOR_API_KEY (`cursor-agent status` alone does not prove --print auth)",
                    &mut failed,
                );
            } else {
                println!(
                    "  ..    credential probe skipped (`doctor --live` runs one real model call)"
                );
            }
        } else {
            report(
                false,
                "cursor-agent on PATH",
                "install from https://cursor.com/docs/cli then `cursor-agent login` (only live runs need it)",
                &mut failed,
            );
        }

        if failed {
            bail!("doctor found failures; apply the fixes above and re-run");
        }
        println!("doctor: all checks passed");
        Ok(())
    }

    fn check(&self, args: &[OsString]) -> Result<()> {
        ensure!(args.len() <= 1, "check accepts at most one adapter name");
        if let Some(adapter) = args.first().filter(|value| !value.is_empty()) {
            let adapter = adapter.to_string_lossy();
            self.ensure_adapter(&adapter)?;
            let spec = self.adapter_spec(&adapter)?;
            println!("== native tests: {spec} (adapters checkout) ==");
            self.cargo(&self.adapters, ["nextest", "run", "-p", &spec, "--no-tests=pass"])?;
        } else {
            println!("== no adapter scoped (`check <name>` adds its native tests) ==");
        }
        println!("== native harness full-loop/seam tests (standalone workspace, sibling engine) ==");
        // Builtin `cargo test`, not nextest: an external subcommand spawns
        // its own inner cargo, which does not inherit `--config` patches.
        self.native_cargo(&[
            OsString::from("test"),
            OsString::from("--manifest-path"),
            self.native_manifest(),
        ])
    }

    fn run_project(&self, args: &[OsString]) -> Result<()> {
        let (project, args) =
            args.split_first().context("usage: dev run <project-dir> [specify-dev args...]")?;
        ensure!(!project.is_empty(), "project directory is required");
        let project = fs::canonicalize(project).with_context(|| {
            format!("project directory not found: {}", project.to_string_lossy())
        })?;
        let mut cargo_args = vec![
            OsString::from("run"),
            OsString::from("-q"),
            OsString::from("--manifest-path"),
            self.native_manifest(),
            OsString::from("--"),
            OsString::from("--project-dir"),
            project.into_os_string(),
        ];
        cargo_args.extend_from_slice(args);
        self.native_cargo(&cargo_args)
    }

    fn live(&self, args: &[OsString]) -> Result<()> {
        ensure!(args.len() <= 2, "live accepts at most an adapter and scenario");
        let adapter = args.first().filter(|value| !value.is_empty());
        let Some(adapter) = adapter else {
            println!("== live workflow profile: native-live ==");
            return self.quality("native-live");
        };
        let adapter = adapter.to_string_lossy();
        self.ensure_adapter(&adapter)?;
        let scenario = args
            .get(1)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string_lossy().into_owned())
            .or_else(|| default_scenario(&adapter).map(str::to_owned))
            .with_context(|| {
                format!(
                    "no default live scenario for `{adapter}`; pass `live {adapter} <live-test name from harness/live.rs>`"
                )
            })?;

        let target = self.target_dir(&self.adapters);
        let wasm = target.join("wasm32-wasip2/debug");
        let overlay = env::var("SPECIFY_PROSE_OVERLAY").ok().or_else(|| {
            let ready = wasm.join(format!("{adapter}.wasm")).is_file()
                && wasm.join("examples/eval_guest.wasm").is_file()
                && target.join("debug/examples/eval-driver").is_file();
            ready.then(|| {
                println!(
                    "== prose overlay on (artifacts present; SPECIFY_PROSE_OVERLAY=0 opts out) =="
                );
                "1".to_owned()
            })
        });

        println!("== live adapter eval: {adapter}::{scenario} ==");
        let filter = format!("{adapter}::{scenario}");
        let mut command = self.cargo_command(&self.adapters);
        command.args([
            "test",
            "-p",
            "harness",
            "--test",
            "live",
            "--",
            "--ignored",
            "--nocapture",
            "--exact",
            &filter,
        ]);
        if overlay.as_deref() == Some("1") {
            command.env("SPECIFY_PROSE_OVERLAY", "1");
        }
        execute(&mut command, "live adapter eval")
    }

    fn full(&self) -> Result<()> {
        println!("==== dev full: the explicit outer gate (WASM + live model) ====");
        self.doctor(&[OsString::from("--live")])?;
        println!("== deterministic native rung ==");
        self.check(&[])?;
        println!("== composed WASM/WIT coverage (adapters checkout) ==");
        self.cargo(
            &self.adapters,
            ["test", "-p", "harness", "--test", "composed"],
        )?;
        println!("== composed workflow profile: wasm-live ==");
        self.quality("wasm-live")?;
        println!("==== dev full: complete ====");
        Ok(())
    }

    fn quality(&self, profile: &str) -> Result<()> {
        let mut command = Command::new("cargo");
        command.current_dir(&self.framework).args(["run", "-q", "-p", "quality", "--", "run", profile]);
        execute(&mut command, &format!("{profile} quality profile"))
    }

    fn ensure_adapter(&self, adapter: &str) -> Result<()> {
        ensure!(
            self.adapters.join("targets").join(adapter).is_dir()
                || self.adapters.join("sources").join(adapter).is_dir(),
            "no adapter `{adapter}` under {}/{{targets,sources}}",
            self.adapters.display()
        );
        Ok(())
    }

    fn adapter_spec(&self, adapter: &str) -> Result<String> {
        let output = self
            .cargo_command(&self.adapters)
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output()
            .context("reading adapter workspace metadata")?;
        ensure!(
            output.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        let metadata: Metadata = serde_json::from_slice(&output.stdout)
            .context("decoding adapter workspace metadata")?;
        let package = metadata
            .packages
            .iter()
            .find(|package| {
                package.name == adapter && metadata.workspace_members.contains(&package.id)
            })
            .with_context(|| {
                format!(
                    "adapter `{adapter}` is not a workspace member of {}",
                    self.adapters.display()
                )
            })?;
        Ok(format!("{}@{}", package.name, package.version))
    }

    /// The standalone native-harness manifest in the adapters checkout.
    fn native_manifest(&self) -> OsString {
        self.adapters.join("harness/native/Cargo.toml").into_os_string()
    }

    /// Run cargo against the standalone native-harness workspace with its
    /// engine pin patched to this checkout's crates. The default manifest
    /// stays revision-pinned — the overrides live only on this command
    /// line, never in tracked Cargo configuration. Cargo rewrites the
    /// harness lockfile while a patch is active, so the pinned lock is
    /// snapshotted and restored around the invocation.
    fn native_cargo(&self, args: &[OsString]) -> Result<()> {
        let lock = self.adapters.join("harness/native/Cargo.lock");
        let saved = fs::read(&lock).ok();
        let mut command = self.cargo_command(&self.adapters);
        for name in ENGINE_CRATES {
            command.arg("--config");
            command.arg(format!(
                "patch.\"{ENGINE_GIT}\".{name}.path=\"{}\"",
                self.framework.join("crates").join(name).display()
            ));
        }
        command.args(args);
        let result = execute(&mut command, "cargo (native harness, sibling engine)");
        if let Some(bytes) = saved {
            fs::write(&lock, bytes).context("restoring the pinned native-harness lockfile")?;
        }
        result
    }

    fn cargo<const N: usize>(&self, directory: &Path, args: [&str; N]) -> Result<()> {
        let mut command = self.cargo_command(directory);
        command.args(args);
        execute(&mut command, "cargo")
    }

    fn cargo_command(&self, directory: &Path) -> Command {
        let mut command = Command::new("cargo");
        command.current_dir(directory);
        if env::var_os("CARGO_TARGET_DIR").is_some() {
            command.env("CARGO_TARGET_DIR", self.target_dir(directory));
        }
        command
    }

    fn target_dir(&self, repository: &Path) -> PathBuf {
        env_path("CARGO_TARGET_DIR").map_or_else(
            || repository.join("target"),
            |root| root.join(repository.file_name().unwrap_or_default()),
        )
    }
}

fn report(ok: bool, label: &str, remediation: &str, failed: &mut bool) {
    if ok {
        println!("  ok    {label}");
    } else {
        *failed = true;
        println!("  FAIL  {label}");
        println!("        fix: {remediation}");
    }
}

fn succeeds(command: &mut Command) -> bool {
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn on_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn absolute(current: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        current.join(path)
    }
}

fn default_scenario(adapter: &str) -> Option<&'static str> {
    match adapter {
        "contracts" => Some("design"),
        "vectis" => Some("single_screen"),
        _ => None,
    }
}

fn execute(command: &mut Command, action: &str) -> Result<()> {
    let status = command.status().with_context(|| action.to_owned())?;
    ensure!(status.success(), "{action} failed with {status}");
    Ok(())
}
