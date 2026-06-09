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

const INSTALL_ROOT: &str = ".bin";
const BIN: &str = ".bin/bin/specify";
const DEFAULT_GIT_HOST: &str = "github.com/augentic/specify-cli";

fn die(m: &str) -> ! {
    eprintln!("specify: error: {m}");
    std::process::exit(1);
}

fn run(mut cmd: Command) -> ! {
    #[cfg(unix)]
    die(&format!("exec failed: {}", cmd.exec()));
    #[cfg(not(unix))]
    std::process::exit(cmd.status().map_or(1, |s| s.code().unwrap_or(1)));
}

enum Mode {
    Run,
    Install,
    ResolvedRef,
}

enum Sel {
    Path(String),
    Git { url: String, git: GitRef },
}

enum GitRef {
    Rev(String),
    Branch(String),
    Tag(String),
}

fn str_field(table: &toml::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

fn load_cli() -> toml::Table {
    ["Specify.local.toml", "Specify.toml"]
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .filter_map(|s| s.parse::<toml::Table>().ok())
        .find_map(|t| t.get("cli")?.as_table().cloned())
        .unwrap_or_else(|| die("no `cli` source spec in Specify.toml"))
}

fn resolve_ref(cli: &toml::Table) -> Sel {
    if let Some(path) = str_field(cli, "path") {
        return Sel::Path(path);
    }

    let git = str_field(cli, "rev")
        .map(GitRef::Rev)
        .or_else(|| str_field(cli, "branch").map(GitRef::Branch))
        .or_else(|| str_field(cli, "tag").map(GitRef::Tag))
        .or_else(|| str_field(cli, "version").map(|v| GitRef::Tag(format!("v{v}"))))
        .unwrap_or_else(|| die("`cli` needs one of: path | git + rev/branch/tag | version"));

    const SCHEME: &str = "https";
    let url = str_field(cli, "git").unwrap_or_else(|| format!("{SCHEME}://{DEFAULT_GIT_HOST}"));
    Sel::Git { url, git }
}

fn print_resolved_ref(sel: &Sel) {
    let token = match sel {
        Sel::Path(p) => format!("path:{p}"),
        Sel::Git { url, git } => match git {
            GitRef::Rev(r) => r.clone(),
            GitRef::Branch(b) => ls_remote(url, b).unwrap_or_else(|| b.to_owned()),
            GitRef::Tag(t) => ls_remote(url, t).unwrap_or_else(|| t.to_owned()),
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

fn install(selector: &[&str]) {
    let ok = Command::new("cargo")
        .args([
            "install",
            "--quiet",
            "--locked",
            "--root",
            INSTALL_ROOT,
            "--bin",
            "specify",
        ])
        .args(selector)
        .arg("specify")
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        die("cargo install failed");
    }
}

fn run_local(path: &str, args: &[String]) {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--manifest-path",
        &format!("{path}/Cargo.toml"),
        "--bin",
        "specify",
        "--",
    ])
    .args(args);
    run(cmd);
}

fn run_installed(args: &[String]) {
    let mut cmd = Command::new(BIN);
    cmd.args(args);
    run(cmd);
}

fn parse_args() -> (Mode, Vec<String>) {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        Some("--install") => (Mode::Install, argv[1..].to_vec()),
        Some("--resolved-ref") => (Mode::ResolvedRef, argv[1..].to_vec()),
        _ => (Mode::Run, argv),
    }
}

fn main() {
    let (mode, args) = parse_args();
    let sel = resolve_ref(&load_cli());

    if matches!(mode, Mode::ResolvedRef) {
        print_resolved_ref(&sel);
        return;
    }

    match sel {
        Sel::Path(path) => {
            if matches!(mode, Mode::Install) {
                install(&["--path", &path]);
                println!("{BIN}");
                return;
            }
            run_local(&path, &args);
        }
        Sel::Git { url, git } => {
            let mut selector = vec!["--git".to_owned(), url];
            match git {
                GitRef::Rev(r) => selector.extend(["--rev".to_owned(), r.clone()]),
                GitRef::Branch(b) => {
                    selector.extend(["--branch".to_owned(), b.clone(), "--force".to_owned()])
                }
                GitRef::Tag(t) => selector.extend(["--tag".to_owned(), t.clone()]),
            }

            let selector: Vec<&str> = selector.iter().map(String::as_str).collect();
            install(&selector);
            if matches!(mode, Mode::Install) {
                println!("{BIN}");
                return;
            }
            run_installed(&args);
        }
    }
}
