#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "use-local-dev"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
---

//! Local dev bootstrap: install specify and refresh the Cursor plugin cache.
//! Requires Specify.local.toml with `cli.path`. Flag: `--plugins-only`. Env:
//! `SPECIFY_BIN_DIR`, `CURSOR_HOME`. Nightly cargo-script.
use std::error::Error;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Options {
    plugins_only: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("use-local-dev: error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = parse_options()?;
    let repo_root = env::current_dir()?;

    if opts.plugins_only {
        return populate_plugin_cache(&repo_root);
    }

    let cli_root = resolve_cli_checkout(&repo_root)?;
    let install_dir = specify_bin_dir()?;
    fs::create_dir_all(&install_dir)?;

    let installed = install_specify(&repo_root, &install_dir)?;

    populate_plugin_cache(&repo_root)?;
    print_summary(&installed, &cli_root);
    Ok(())
}

fn parse_options() -> Result<Options> {
    let mut opts = Options {
        plugins_only: false,
    };
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--plugins-only" => opts.plugins_only = true,
            other => {
                return Err(format!(
                    "unknown option: {other}\n\
                     Usage: scripts/use-local-dev.rs [--plugins-only]"
                )
                .into());
            }
        }
    }
    Ok(opts)
}

// Read the `cli` table whole from the first overlay that defines one — a gitignored
// Specify.local.toml, else the committed Specify.toml — and return its `cli.path`.
// use-local-dev installs specify from a local checkout, so a git/version pin (no
// path) is an error here.
fn cli_path() -> Result<String> {
    ["Specify.local.toml", "Specify.toml"]
        .iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .filter_map(|s| s.parse::<toml::Table>().ok())
        .find_map(|t| t.get("cli")?.as_table().cloned())
        .and_then(|cli| cli.get("path")?.as_str().map(str::to_owned))
        .ok_or_else(|| {
            "use-local-dev requires Specify.local.toml with cli = { path = \"../specify-cli\" }"
                .into()
        })
}

fn resolve_cli_checkout(repo_root: &Path) -> Result<PathBuf> {
    let path = cli_path()?;
    repo_root.join(&path).canonicalize().map_err(|_| {
        format!(
            "cli.path `{path}` not found (relative to {})",
            repo_root.display()
        )
        .into()
    })
}

fn specify_bin_dir() -> Result<PathBuf> {
    env::var_os("SPECIFY_BIN_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin")))
        .ok_or_else(|| "HOME is not set".into())
}

fn install_specify(repo_root: &Path, install_dir: &Path) -> Result<PathBuf> {
    println!("Materializing specify via scripts/specify.rs --install …");
    let built = materialize_specify(repo_root)?;
    let installed = install_dir.join("specify");
    link_or_copy(&built, &installed)?;
    println!("Installed specify → {}", installed.display());
    warn_path(install_dir, "specify");
    Ok(installed)
}

fn print_summary(installed: &Path, cli_root: &Path) {
    println!("\nLocal dev environment ready.");
    println!(
        "  specify: {}",
        which("specify").unwrap_or_else(|| installed.to_path_buf()).display()
    );
    println!("  cli checkout: {}", cli_root.display());
    println!("\nNext steps:");
    println!("  1. Restart Cursor to pick up local plugin changes.");
    println!("  2. Open your project (e.g. ../todo-app) in Cursor.");
    println!("  3. Run /spec:init to scaffold .specify/ and bind adapters.");
}

fn populate_plugin_cache(repo_root: &Path) -> Result<()> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Marketplace {
        name: String,
        #[serde(default)]
        metadata: Metadata,
        #[serde(default)]
        plugins: Vec<Plugin>,
    }
    #[derive(Deserialize, Default)]
    struct Metadata {
        #[serde(rename = "pluginRoot")]
        plugin_root: Option<String>,
    }
    #[derive(Deserialize)]
    struct Plugin {
        source: String,
    }

    let marketplace_path = repo_root.join(".cursor-plugin/marketplace.json");
    let text = fs::read_to_string(&marketplace_path).map_err(|_| {
        format!(
            "marketplace.json not found at {}",
            marketplace_path.display()
        )
    })?;
    let marketplace: Marketplace = serde_json::from_str(&text)?;
    let plugin_root = marketplace
        .metadata
        .plugin_root
        .as_deref()
        .unwrap_or("plugins");

    let cache_dir = cursor_home()?.join("plugins/cache").join(&marketplace.name);
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
    }

    for plugin in &marketplace.plugins {
        let src = repo_root.join(plugin_root).join(&plugin.source);
        if !src.is_dir() {
            eprintln!("Warning: {} not found, skipping", src.display());
            continue;
        }
        let dest = cache_dir.join(&plugin.source).join("main");
        fs::create_dir_all(&dest)?;
        copy_dir_all(&src, &dest)?;
        println!("Cached {} from local source", plugin.source);
    }

    println!("\nRestart Cursor to pick up local plugin changes.");
    Ok(())
}

fn cursor_home() -> Result<PathBuf> {
    match env::var_os("CURSOR_HOME") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".cursor"))
            .ok_or_else(|| "neither CURSOR_HOME nor HOME is set".into()),
    }
}

fn copy_dir_all(src: &Path, dest: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let _ = fs::remove_file(&to);
                symlink(fs::read_link(&from)?, &to)?;
            }
            #[cfg(not(unix))]
            if from.is_dir() {
                fs::create_dir_all(&to)?;
                copy_dir_all(&from, &to)?;
            } else {
                fs::copy(&from, &to)?;
            }
        } else if file_type.is_dir() {
            fs::create_dir_all(&to)?;
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn materialize_specify(repo_root: &Path) -> Result<PathBuf> {
    // Use the rustup shim — parent cargo-script's $CARGO rejects `+nightly`.
    let output = Command::new("cargo")
        .args(["+nightly", "-Zscript", "scripts/specify.rs", "--install"])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("scripts/specify.rs --install failed: {stderr}").into());
    }
    let rel = String::from_utf8(output.stdout)?.trim().to_string();
    let bin = repo_root.join(&rel);
    if !bin.is_file() {
        return Err(format!("specify bootstrap did not produce {}", bin.display()).into());
    }
    Ok(bin)
}

fn link_or_copy(source: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_file(dest)?;
    }
    #[cfg(unix)]
    symlink(source, dest)?;
    #[cfg(not(unix))]
    fs::copy(source, dest)?;
    Ok(())
}

fn which(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn warn_path(install_dir: &Path, bin: &str) {
    let on_path = env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|dir| dir == install_dir))
        .unwrap_or(false);
    if !on_path {
        eprintln!("Warning: {} is not on your PATH.", install_dir.display());
        eprintln!(
            "         Add to your shell profile: export PATH=\"{}:$PATH\"",
            install_dir.display()
        );
        return;
    }
    if let Some(resolved) = which(bin) {
        let target = install_dir.join(bin);
        if resolved != target {
            eprintln!(
                "Warning: {bin} resolves to {}, which shadows {}.",
                resolved.display(),
                target.display()
            );
        }
    }
}
