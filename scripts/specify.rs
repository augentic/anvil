#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "specify-bootstrap"
edition = "2021"

[dependencies]
toml = "0.8"
---

//! Build and run specify-cli from the `cli` table in Specify.local.toml (whole
//! overlay) or Specify.toml. Bootstrap flags: `--install`, `--resolved-ref`; all
//! other args pass through to `specify`. Nightly cargo-script (-Zscript) required.
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

const INSTALL_ROOT: &str = ".cli";
// Where `cargo install --root INSTALL_ROOT` lands the binary; keep in sync with INSTALL_ROOT.
const BIN: &str = ".cli/bin/specify";
const DEFAULT_GIT_HOST: &str = "github.com/augentic/specify-cli";

fn main() {
    let source = resolve_ref(&load_cli());
    match parse_args() {
        Mode::ResolvedRef => print_resolved_ref(&source),
        Mode::Install => match source {
            CliSource::Path(path) => {
                build_local(&path);
                println!("{path}/target/release/specify");
            }
            CliSource::Git { url, git } => {
                install(&git.cargo_selector(url));
                println!("{BIN}");
            }
        },
        Mode::Run(args) => match source {
            CliSource::Path(path) => run_local(&path, &args),
            CliSource::Git { url, git } => {
                install(&git.cargo_selector(url));
                run_installed(&args);
            }
        },
    }
}

fn die(msg: &str) -> ! {
    eprintln!("specify: error: {msg}");
    std::process::exit(1);
}

// Hand the current process off to `cmd` and never return: exec-replace on unix,
// else run to completion and exit with the child's status code.
fn exec_replacing(mut cmd: Command) -> ! {
    #[cfg(unix)]
    die(&format!("exec failed: {}", cmd.exec()));
    #[cfg(not(unix))]
    std::process::exit(cmd.status().map_or(1, |s| s.code().unwrap_or(1)));
}

fn run_to_completion(mut cmd: Command, fail_msg: &str) {
    if !cmd.status().is_ok_and(|s| s.success()) {
        die(fail_msg);
    }
}

enum Mode {
    Run(Vec<String>),
    Install,
    ResolvedRef,
}

enum CliSource {
    Path(String),
    Git { url: String, git: GitRef },
}

enum GitRef {
    Rev(String),
    Branch(String),
    Tag(String),
}

impl GitRef {
    // cargo-install ref selector. A branch is mutable, so `--force` reinstalls it
    // on every run; tags and revs are immutable and need no force.
    fn cargo_selector(self, url: String) -> Vec<String> {
        let mut selector = vec!["--git".to_owned(), url];
        match self {
            GitRef::Rev(r) => selector.extend(["--rev".to_owned(), r]),
            GitRef::Branch(b) => selector.extend(["--branch".to_owned(), b, "--force".to_owned()]),
            GitRef::Tag(t) => selector.extend(["--tag".to_owned(), t]),
        }
        selector
    }
}

fn str_field(table: &toml::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

// Read the `cli` table whole from the first overlay that defines one — a gitignored
// Specify.local.toml, else the committed Specify.toml. A file that exists but fails
// to parse is a hard error, not a silent fallthrough to the next candidate.
fn load_cli() -> toml::Table {
    for file in ["Specify.local.toml", "Specify.toml"] {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let table = text
            .parse::<toml::Table>()
            .unwrap_or_else(|e| die(&format!("{file}: {e}")));
        if let Some(cli) = table.get("cli").and_then(toml::Value::as_table) {
            return cli.clone();
        }
    }
    die("no `cli` source spec in Specify.local.toml or Specify.toml");
}

fn resolve_ref(cli: &toml::Table) -> CliSource {
    if let Some(path) = str_field(cli, "path") {
        return CliSource::Path(path);
    }

    let git = str_field(cli, "rev")
        .map(GitRef::Rev)
        .or_else(|| str_field(cli, "branch").map(GitRef::Branch))
        .or_else(|| str_field(cli, "tag").map(GitRef::Tag))
        .or_else(|| str_field(cli, "version").map(|v| GitRef::Tag(format!("v{v}"))))
        .unwrap_or_else(|| die("`cli` needs one of: path | git + rev/branch/tag | version"));

    let url = str_field(cli, "git").unwrap_or_else(|| format!("https://{DEFAULT_GIT_HOST}"));
    CliSource::Git { url, git }
}

fn print_resolved_ref(source: &CliSource) {
    let token = match source {
        CliSource::Path(p) => format!("path:{p}"),
        CliSource::Git { url, git } => match git {
            GitRef::Rev(r) => r.clone(),
            GitRef::Branch(name) | GitRef::Tag(name) => {
                ls_remote(url, name).unwrap_or_else(|| name.clone())
            }
        },
    };
    println!("{token}");
}

fn ls_remote(url: &str, refname: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["ls-remote", url, refname])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    let first = stdout.lines().next()?.split_whitespace().next()?;
    Some(first.to_owned())
}

fn install(selector: &[String]) {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "install",
        "--quiet",
        "--locked",
        "--root",
        INSTALL_ROOT,
        "--bin",
        "specify",
    ])
    .args(selector)
    .arg("specify");
    run_to_completion(cmd, "cargo install failed");
}

// Local-path install: build into the sibling checkout's own target dir so the
// build is incremental and shared with normal dev builds, then hand the caller
// the resolved binary path to symlink. Unlike `install()` (`cargo install` into
// an isolated `.cli` root), this avoids a from-scratch release rebuild per run.
fn build_local(path: &str) {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--quiet",
        "--release",
        "--manifest-path",
        &format!("{path}/Cargo.toml"),
        "--bin",
        "specify",
    ]);
    run_to_completion(cmd, "cargo build failed");
}

// `--release` so the heavy `lint framework` work (wasmtime + schema validation)
// runs under an optimized binary, and so this build shares the same incremental
// release target as `build_local` (`make install-cli`) rather than maintaining a
// separate, slower-at-runtime debug artifact set.
fn run_local(path: &str, args: &[String]) {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--release",
        "--manifest-path",
        &format!("{path}/Cargo.toml"),
        "--bin",
        "specify",
        "--",
    ])
    .args(args);
    exec_replacing(cmd);
}

fn run_installed(args: &[String]) {
    let mut cmd = Command::new(BIN);
    cmd.args(args);
    exec_replacing(cmd);
}

fn parse_args() -> Mode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        Some("--install") => Mode::Install,
        Some("--resolved-ref") => Mode::ResolvedRef,
        _ => Mode::Run(argv),
    }
}
