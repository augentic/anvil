#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "use-local-dev"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde-saphyr = "0.0.26"
serde_json = "1"
toml = "0.8"
---
//! Adapter-local dev setup: materialize `specify` via [`scripts/specify.rs`](specify.rs),
//! build adapter WASI tools from the local `cli.path` checkout, write each target
//! adapter's gitignored `tools.yaml` sidecar, and repopulate the Cursor plugin cache.
//!
//! Requires a gitignored `Specify.local.toml` overlay with `cli = { path = "…" }` so
//! WASI builds have a checkout; CLI install itself delegates to `specify.rs --install`.
//!
//! Usage: cargo +nightly -Zscript scripts/use-local-dev.rs [--skip-wasi] [--plugins-only]
//!        --plugins-only repopulates the Cursor plugin cache from the working tree
//!        (the `make use-local-plugins` path) and skips the CLI + WASI build.
//! Env:   SPECIFY_BIN_DIR (install dir; default ~/.local/bin), CURSOR_HOME
//!
//! Toolchain: cargo-script is still nightly-only (-Zscript); switch the shebang,
//! Makefile, and CI pins to stable once it stabilizes (rust-lang/cargo#16569).
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
#[cfg(unix)]
use std::os::unix::fs::symlink;

use serde::Serialize;

include!("load_cli.rs");

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// `(cargo_pkg, bin_name, adapter_dir, tool_name)`. Omnia declares no tools.
const WASI_TOOLS: &[(&str, &str, &str, &str)] = &[
    ("specify-vectis", "vectis", "vectis", "vectis"),
    ("specify-contract", "specify-contract", "contracts", "contract"),
];

fn main() {
    if let Err(err) = run() {
        eprintln!("use-local-dev: error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut skip_wasi = false;
    let mut plugins_only = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--skip-wasi" => skip_wasi = true,
            "--plugins-only" => plugins_only = true,
            other => {
                return Err(format!(
                    "unknown option: {other}\n\
                     Usage: scripts/use-local-dev.rs [--skip-wasi] [--plugins-only]"
                )
                .into());
            }
        }
    }

    let repo_root = env::current_dir()?;

    // Plugin-cache population needs neither a CLI checkout nor a WASI build, so
    // `--plugins-only` (backing `make use-local-plugins`) short-circuits here.
    if plugins_only {
        return populate_plugin_cache(&repo_root);
    }

    let cli = read_cli_spec().ok_or("no `cli` source spec in Specify.toml")?;
    let path = cli_path(&cli).ok_or(
        "use-local-dev requires Specify.local.toml with cli = { path = \"../specify-cli\" } \
         (local checkout needed for WASI tool builds)",
    )?;
    let cli_root = repo_root.join(path).canonicalize().map_err(|_| {
        format!("cli.path `{path}` not found (relative to {})", repo_root.display())
    })?;

    let install_dir = match env::var_os("SPECIFY_BIN_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".local/bin"))
            .ok_or("HOME is not set")?,
    };
    fs::create_dir_all(&install_dir)?;

    // ── Install the CLI (via the shared resolver) ────────────────
    println!("Materializing specify via scripts/specify.rs --install …");
    let built = materialize_specify(&repo_root)?;
    let installed = install_dir.join("specify");
    link_or_copy(&built, &installed)?;
    println!("Installed specify → {}", installed.display());
    warn_path(&install_dir, "specify");

    // ── Build WASI tools + write sidecars ────────────────────────
    let wasi_dir = cli_root.join("wasi-tools");
    if skip_wasi {
        println!("Skipping WASI tool build (--skip-wasi).");
    } else if !wasi_dir.is_dir() {
        eprintln!("Warning: wasi-tools/ not found in specify-cli, skipping WASI build");
    } else if !wasm_target_installed() {
        eprintln!("Warning: wasm32-wasip2 target not installed, skipping WASI build");
        eprintln!("         Install with: rustup target add wasm32-wasip2");
    } else {
        for &(cargo_pkg, bin_name, adapter_dir, tool_name) in WASI_TOOLS {
            println!("Building {tool_name} WASI tool …");
            cargo(
                &["build", "-p", cargo_pkg, "--target", "wasm32-wasip2", "--release"],
                &wasi_dir,
            )?;

            let wasm = wasi_dir
                .join("target/wasm32-wasip2/release")
                .join(format!("{bin_name}.wasm"));
            if !wasm.is_file() {
                eprintln!("Warning: {bin_name}.wasm not found after build, skipping sidecar");
                continue;
            }
            let wasm_abs = wasm.canonicalize()?;

            let adapter_yaml = repo_root
                .join("adapters/targets")
                .join(adapter_dir)
                .join("adapter.yaml");
            let version = adapter_tool_version(&adapter_yaml, tool_name)?;

            let dest = repo_root
                .join("adapters/targets")
                .join(adapter_dir)
                .join("tools.yaml");
            fs::write(&dest, sidecar(tool_name, &version, &wasm_abs)?)?;
            println!("Installed {tool_name} sidecar → {}", dest.display());
        }
    }

    // ── Populate the plugin cache ────────────────────────────────
    populate_plugin_cache(&repo_root)?;

    // ── Summary ──────────────────────────────────────────────────
    println!("\nLocal dev environment ready.");
    println!("  specify: {}", which("specify").unwrap_or(installed).display());
    for &(_, _, adapter_dir, tool_name) in WASI_TOOLS {
        let sidecar = repo_root
            .join("adapters/targets")
            .join(adapter_dir)
            .join("tools.yaml");
        if sidecar.is_file() {
            println!("  {tool_name}: {}", sidecar.display());
        }
    }
    println!("\nNext steps:");
    println!("  1. Restart Cursor to pick up local plugin changes.");
    println!("  2. Open your project (e.g. ../todo-app) in Cursor.");
    println!("  3. Run /spec:init to scaffold .specify/ and bind adapters.");
    Ok(())
}

// ── Plugin cache (replaces scripts/use-local-plugins.sh + jq) ─────
//
// Reads .cursor-plugin/marketplace.json with serde, clears the marketplace-
// scoped Cursor cache, and copies each plugin's working-tree source in. Honors
// CURSOR_HOME exactly like the CLI's `plugins {doctor,refresh}` verbs so the
// populate path and the CLI's scan/clear agree on the cache root.

/// Replace the marketplace-scoped Cursor plugin cache with working-tree copies
/// so skill / rule / reference edits are testable before pushing to main.
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
    let text = fs::read_to_string(&marketplace_path)
        .map_err(|_| format!("marketplace.json not found at {}", marketplace_path.display()))?;
    let marketplace: Marketplace = serde_json::from_str(&text)?;
    let plugin_root = marketplace.metadata.plugin_root.as_deref().unwrap_or("plugins");

    // Clear only the marketplace-scoped cache, then repopulate from the tree.
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

/// `$CURSOR_HOME` when set and non-empty, else `~/.cursor` (matches the CLI).
fn cursor_home() -> Result<PathBuf> {
    match env::var_os("CURSOR_HOME") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".cursor"))
            .ok_or_else(|| "neither CURSOR_HOME nor HOME is set".into()),
    }
}

/// Recursively copy `src` into `dest`, preserving symlinks like `cp -R`.
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

/// Run `scripts/specify.rs --install` and return the repo-relative `.bin/bin/specify` path.
fn materialize_specify(repo_root: &Path) -> Result<PathBuf> {
    // Always invoke the rustup `cargo` shim — not `$CARGO` from a cargo-script parent,
    // which points at the active toolchain binary and rejects `+nightly`.
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
    {
        symlink(source, dest)?;
    }
    #[cfg(not(unix))]
    {
        fs::copy(source, dest)?;
    }
    Ok(())
}

// ── Sidecar model ─────────────────────────────────────────────────
//
// Typed mirror of the CLI's tools.yaml shape. Using serde here is the whole
// point of the rewrite: it removes the heredoc + awk fragility.

#[derive(Serialize)]
struct Sidecar {
    tools: Vec<ToolEntry>,
}

#[derive(Serialize)]
struct ToolEntry {
    name: String,
    version: String,
    source: String,
    permissions: Permissions,
}

#[derive(Serialize)]
struct Permissions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    read: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    write: Vec<String>,
}

// PERMISSIONS DUPLICATED — an intentional, accepted mirror of
// specify_tool::manifest::first_party_permissions() in augentic/specify-cli.
// No CLI verb owns dev sidecar wiring (by decision), so these literals stay here:
// the trade-off is no dependency on specify-cli, at the cost of no compiler check.
// Keep this table in sync with the CLI when first-party tool permissions change.
fn first_party_permissions(tool_name: &str) -> Result<Permissions> {
    match tool_name {
        "contract" => Ok(Permissions {
            read: vec!["$PROJECT_DIR/contracts".to_string()],
            write: vec![],
        }),
        "vectis" => Ok(Permissions {
            read: vec!["$PROJECT_DIR".to_string(), "$CAPABILITY_DIR".to_string()],
            write: vec!["$PROJECT_DIR".to_string()],
        }),
        other => Err(format!("unknown tool {other}, no embedded permissions").into()),
    }
}

fn sidecar(tool_name: &str, version: &str, source_abs: &Path) -> Result<String> {
    let manifest = Sidecar {
        tools: vec![ToolEntry {
            name: tool_name.to_string(),
            version: version.to_string(),
            source: source_abs.to_string_lossy().into_owned(),
            permissions: first_party_permissions(tool_name)?,
        }],
    };
    Ok(serde_saphyr::to_string(&manifest)?)
}

// ── Helpers ──────────────────────────────────────────────────────

fn cargo(args: &[&str], cwd: &Path) -> Result<()> {
    let cargo_bin = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo_bin).args(args).current_dir(cwd).status()?;
    if !status.success() {
        return Err(format!("cargo {} failed", args.join(" ")).into());
    }
    Ok(())
}

fn wasm_target_installed() -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|line| line.trim() == "wasm32-wasip2")
        })
        .unwrap_or(false)
}

/// Pull `tools[].version` for `tool_name` out of adapter.yaml via serde
/// (replaces the awk `/^tools:/ … version:` scrape).
fn adapter_tool_version(adapter_yaml: &Path, tool_name: &str) -> Result<String> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Adapter {
        #[serde(default)]
        tools: Vec<AdapterTool>,
    }
    #[derive(Deserialize)]
    struct AdapterTool {
        name: String,
        version: String,
    }

    let text = fs::read_to_string(adapter_yaml)?;
    let adapter: Adapter = serde_saphyr::from_str(&text)?;
    adapter
        .tools
        .into_iter()
        .find(|tool| tool.name == tool_name)
        .map(|tool| tool.version)
        .ok_or_else(|| format!("no tool `{tool_name}` in {}", adapter_yaml.display()).into())
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
