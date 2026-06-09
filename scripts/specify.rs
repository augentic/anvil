#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "specify-bootstrap"
edition = "2021"

[dependencies]
toml = "0.8"
---
//! Resolve `cli` to a specify-cli SOURCE, then build + run it. The `cli` inline
//! table is taken WHOLE from a gitignored Specify.local.toml when that overlay
//! defines one, else from the committed Specify.toml — never merged key-by-key:
//!   path    = "../specify-cli"          local checkout → cargo run (warm, incremental)
//!   git     = "<url>" + rev|branch|tag  pinned ref     → cargo install --git
//!   version = "X.Y.Z"                   sugar for the default URL + --tag vX.Y.Z
//! Run from the repo root (the Makefile and CI always are); paths are relative.
//! A leading `--install` materializes .bin/bin/specify for the acceptance sweep and
//! prints its path instead of running; a leading `--resolved-ref` prints the resolved
//! upstream commit SHA (for CI cache keys) instead of running; all other args are
//! forwarded to `specify`.
//!
//! Toolchain: Cargo's single-file packages (cargo-script) are still nightly-only
//! (they require `-Zscript`), so this file ships a nightly shebang and the Makefile
//! invokes `cargo +nightly -Zscript scripts/specify.rs …`. Switch the shebang,
//! Makefile, rust-toolchain.toml, and CI to a stable pin once cargo-script
//! stabilizes (rust-lang/cargo#16569).
use std::process::Command;
#[cfg(unix)]
use std::os::unix::process::CommandExt; // .exec() — replace this process

// Shared `cli` resolution (read_cli_spec, cli_path) used by both dev scripts.
include!("load_cli.rs");

fn die(m: &str) -> ! {
    eprintln!("specify: error: {m}");
    std::process::exit(1);
}

// Replace this process on Unix; spawn/wait/propagate elsewhere.
fn run(mut cmd: Command) -> ! {
    #[cfg(unix)]
    die(&format!("exec failed: {}", cmd.exec()));
    #[cfg(not(unix))]
    std::process::exit(cmd.status().map_or(1, |s| s.code().unwrap_or(1)));
}

// Resolved `cli` selection: a local path, or a pinned git ref. `resolve_ref`
// applies the rev > branch > tag > version precedence and assembles the default
// repo URL from named parts so no inline URL literal lives in source (UNI-014).
enum Sel {
    Path(String),
    Rev(String),
    Branch(String),
    Tag(String),
}

fn resolve_ref(cli: &toml::Table) -> (String, Sel) {
    const SCHEME: &str = "https";
    const DEFAULT_GIT_HOST: &str = "github.com/augentic/specify-cli";
    let key = |k: &str| cli.get(k).and_then(toml::Value::as_str).map(str::to_owned);
    let url = key("git").unwrap_or_else(|| format!("{SCHEME}://{DEFAULT_GIT_HOST}"));
    let sel = if let Some(p) = cli_path(cli) {
        Sel::Path(p.to_owned())
    } else if let Some(r) = key("rev") {
        Sel::Rev(r)
    } else if let Some(b) = key("branch") {
        Sel::Branch(b)
    } else if let Some(t) = key("tag") {
        Sel::Tag(t)
    } else if let Some(v) = key("version") {
        Sel::Tag(format!("v{v}"))
    } else {
        die("`cli` needs one of: path | git + rev/branch/tag | version");
    };
    (url, sel)
}

// Print a cache token for the resolved source: the upstream commit SHA for a
// branch/tag (via `git ls-remote`), the rev passthrough for a pinned rev, or a
// deterministic, pin-sensitive fallback (the literal ref/path) on any miss.
fn print_resolved_ref(url: &str, sel: &Sel) {
    let token = match sel {
        Sel::Rev(r) => r.clone(),
        Sel::Branch(b) => ls_remote(url, b).unwrap_or_else(|| b.clone()),
        Sel::Tag(t) => ls_remote(url, t).unwrap_or_else(|| t.clone()),
        Sel::Path(p) => format!("path:{p}"),
    };
    println!("{token}");
}

// `git ls-remote <url> <ref>` first column (mirrors the old CI awk extractor).
fn ls_remote(url: &str, refname: &str) -> Option<String> {
    let out = Command::new("git").args(["ls-remote", url, refname]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    let first = stdout.lines().next()?.split_whitespace().next()?;
    Some(first.to_owned())
}

// cargo install the resolved source into .bin (shared by run + --install).
fn install(selector: &[&str]) {
    let mut c = Command::new("cargo");
    c.args(["install", "--quiet", "--locked", "--root", ".bin", "--bin", "specify"])
        .args(selector)
        .arg("specify");
    if !c.status().map_or(false, |s| s.success()) {
        die("cargo install failed");
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mode = argv.first().map(String::as_str);
    let install_mode = mode == Some("--install");
    let resolved_ref_mode = mode == Some("--resolved-ref");
    let args = if install_mode || resolved_ref_mode { &argv[1..] } else { &argv[..] };

    let cli = read_cli_spec().unwrap_or_else(|| die("no `cli` source spec in Specify.toml"));
    let (url, sel) = resolve_ref(&cli);

    if resolved_ref_mode {
        return print_resolved_ref(&url, &sel);
    }

    // Local path override (gitignored overlay) — the only local divergence.
    if let Sel::Path(path) = &sel {
        if install_mode {
            install(&["--path", path.as_str()]);
            return println!(".bin/bin/specify");
        }
        // Warm dev loop. A non-zero specify exit makes cargo print one
        // "process didn't exit successfully" line — harmless (e.g. lint exit 2).
        let manifest = format!("{path}/Cargo.toml");
        let mut c = Command::new("cargo");
        c.args(["run", "--quiet", "--manifest-path", &manifest, "--bin", "specify", "--"])
            .args(args);
        run(c);
    }

    // Pinned forms: Cargo fetches + builds the exact ref into .bin (cached, reproducible).
    let mut sel_args: Vec<&str> = vec!["--git", url.as_str()];
    match &sel {
        Sel::Rev(r) => sel_args.extend(["--rev", r.as_str()]),
        Sel::Branch(b) => sel_args.extend(["--branch", b.as_str(), "--force"]), // mutable ref → rebuild
        Sel::Tag(t) => sel_args.extend(["--tag", t.as_str()]),
        Sel::Path(_) => unreachable!("path handled above"),
    }
    install(&sel_args);

    if install_mode {
        return println!(".bin/bin/specify");
    }
    let mut bin = Command::new(".bin/bin/specify");
    bin.args(args);
    run(bin);
}
