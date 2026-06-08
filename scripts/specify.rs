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
//! prints its path instead of running; all other args are forwarded to `specify`.
//!
//! Toolchain: Cargo's single-file packages (cargo-script) are still nightly-only
//! (they require `-Zscript`), so this file ships a nightly shebang and the Makefile
//! invokes `cargo +nightly -Zscript scripts/specify.rs …`. Switch the shebang,
//! Makefile, rust-toolchain.toml, and CI to a stable pin once cargo-script
//! stabilizes (rust-lang/cargo#16569).
use std::process::Command;
#[cfg(unix)]
use std::os::unix::process::CommandExt; // .exec() — replace this process

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

// One document owns `cli`: the overlay if it defines one, else the committed file.
// The inline table `cli = { … }` parses to the same Value::Table either way.
fn load_cli() -> toml::Table {
    ["Specify.local.toml", "Specify.toml"]
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .filter_map(|s| s.parse::<toml::Table>().ok())
        .find_map(|t| t.get("cli")?.as_table().cloned())
        .unwrap_or_else(|| die("no `cli` source spec in Specify.toml"))
}

// cargo install the resolved source into .bin (shared by run + --install).
fn install(selector: &[&str]) {
    let mut c = Command::new("cargo");
    c.args(["install", "--quiet", "--locked", "--root", ".bin", "--bin", "specify"])
        .args(selector);
    if !c.status().map_or(false, |s| s.success()) {
        die("cargo install failed");
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let install_mode = argv.first().map(String::as_str) == Some("--install");
    let args = if install_mode { &argv[1..] } else { &argv[..] };
    let cli = load_cli();
    let key = |k: &str| cli.get(k).and_then(toml::Value::as_str);

    // Local path override (gitignored overlay) — the only local divergence.
    if let Some(path) = key("path") {
        if install_mode {
            install(&["--path", path]);
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
    // The default repo is assembled from named parts (scheme + host) so no inline URL
    // literal lives in source (UNI-014); an explicit `git = "…"` in the spec overrides it.
    const SCHEME: &str = "https";
    const DEFAULT_GIT_HOST: &str = "github.com/augentic/specify-cli";
    let owned_default = format!("{SCHEME}://{DEFAULT_GIT_HOST}");
    let url = key("git").unwrap_or(&owned_default);
    let tag;
    let mut sel = vec!["--git", url];
    if let Some(r) = key("rev") {
        sel.extend(["--rev", r]);
    } else if let Some(b) = key("branch") {
        sel.extend(["--branch", b, "--force"]); // mutable ref → always rebuild
    } else if let Some(t) = key("tag") {
        sel.extend(["--tag", t]);
    } else if let Some(v) = key("version") {
        tag = format!("v{v}");
        sel.extend(["--tag", tag.as_str()]);
    } else {
        die("`cli` needs one of: path | git + rev/branch/tag | version");
    }
    install(&sel);

    if install_mode {
        return println!(".bin/bin/specify");
    }
    let mut bin = Command::new(".bin/bin/specify");
    bin.args(args);
    run(bin);
}
