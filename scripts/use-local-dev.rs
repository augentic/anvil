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

//! Local dev bootstrap: install specify, build WASI tool sidecars, refresh Cursor
//! plugin cache. Requires Specify.local.toml with `cli.path`. Flags: `--skip-wasi`,
//! `--plugins-only`. Env: `SPECIFY_BIN_DIR`, `CURSOR_HOME`. Nightly cargo-script.
use std::error::Error;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use serde::Serialize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const WASM_TARGET: &str = "wasm32-wasip2";

/// `(cargo_pkg, bin_name, adapter_dir, tool_name)`.
const WASI_TOOLS: &[(&str, &str, &str, &str)] = &[
    ("specify-vectis", "vectis", "vectis", "vectis"),
    (
        "specify-contract",
        "specify-contract",
        "contracts",
        "contract",
    ),
];

struct Options {
    skip_wasi: bool,
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

    if opts.skip_wasi {
        println!("Skipping WASI tool build (--skip-wasi).");
    } else {
        build_wasi_sidecars(&repo_root, &cli_root)?;
    }

    populate_plugin_cache(&repo_root)?;
    print_summary(&repo_root, &installed);
    Ok(())
}

fn parse_options() -> Result<Options> {
    let mut opts = Options {
        skip_wasi: false,
        plugins_only: false,
    };
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--skip-wasi" => opts.skip_wasi = true,
            "--plugins-only" => opts.plugins_only = true,
            other => {
                return Err(format!(
                    "unknown option: {other}\n\
                     Usage: scripts/use-local-dev.rs [--skip-wasi] [--plugins-only]"
                )
                .into());
            }
        }
    }
    Ok(opts)
}

fn load_cli() -> Result<toml::Table> {
    ["Specify.local.toml", "Specify.toml"]
        .iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .filter_map(|s| s.parse::<toml::Table>().ok())
        .find_map(|t| t.get("cli")?.as_table().cloned())
        .ok_or_else(|| "no `cli` source spec in Specify.toml".into())
}

fn resolve_cli_checkout(repo_root: &Path) -> Result<PathBuf> {
    let cli = load_cli()?;
    let path = cli.get("path").and_then(toml::Value::as_str).ok_or(
        "use-local-dev requires Specify.local.toml with cli = { path = \"../specify-cli\" }",
    )?;
    repo_root.join(path).canonicalize().map_err(|_| {
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

fn adapter_dir(repo_root: &Path, name: &str) -> PathBuf {
    repo_root.join("adapters/targets").join(name)
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

fn build_wasi_sidecars(repo_root: &Path, cli_root: &Path) -> Result<()> {
    let wasi_dir = cli_root.join("wasi-tools");
    if !wasi_dir.is_dir() {
        eprintln!("Warning: wasi-tools/ not found in specify-cli, skipping WASI build");
        return Ok(());
    }
    if !wasm_target_installed() {
        eprintln!("Warning: {WASM_TARGET} target not installed, skipping WASI build");
        eprintln!("         Install with: rustup target add {WASM_TARGET}");
        return Ok(());
    }

    for &(cargo_pkg, bin_name, adapter_dir_name, tool_name) in WASI_TOOLS {
        build_wasi_sidecar(
            repo_root,
            &wasi_dir,
            cargo_pkg,
            bin_name,
            adapter_dir_name,
            tool_name,
        )?;
    }
    Ok(())
}

fn build_wasi_sidecar(
    repo_root: &Path,
    wasi_dir: &Path,
    cargo_pkg: &str,
    bin_name: &str,
    adapter_dir_name: &str,
    tool_name: &str,
) -> Result<()> {
    println!("Building {tool_name} WASI tool …");
    cargo(
        &[
            "build",
            "-p",
            cargo_pkg,
            "--target",
            WASM_TARGET,
            "--release",
        ],
        wasi_dir,
    )?;

    let wasm = wasi_dir
        .join(format!("target/{WASM_TARGET}/release/{bin_name}.wasm"));
    if !wasm.is_file() {
        eprintln!("Warning: {bin_name}.wasm not found after build, skipping sidecar");
        return Ok(());
    }

    let adapter_yaml = adapter_dir(repo_root, adapter_dir_name).join("adapter.yaml");
    let version = adapter_tool_version(&adapter_yaml, tool_name)?;
    let dest = adapter_dir(repo_root, adapter_dir_name).join("tools.yaml");
    fs::write(
        &dest,
        sidecar(tool_name, &version, &wasm.canonicalize()?)?,
    )?;
    println!("Installed {tool_name} sidecar → {}", dest.display());
    Ok(())
}

fn print_summary(repo_root: &Path, installed: &Path) {
    println!("\nLocal dev environment ready.");
    println!(
        "  specify: {}",
        which("specify").unwrap_or_else(|| installed.to_path_buf()).display()
    );
    for &(_, _, adapter_dir_name, tool_name) in WASI_TOOLS {
        let sidecar = adapter_dir(repo_root, adapter_dir_name).join("tools.yaml");
        if sidecar.is_file() {
            println!("  {tool_name}: {}", sidecar.display());
        }
    }
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

// Mirror specify-cli `first_party_permissions` (crates/tool/src/manifest.rs); sync manually.
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

fn cargo(args: &[&str], cwd: &Path) -> Result<()> {
    let ok = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(args)
        .current_dir(cwd)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
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
                .any(|line| line.trim() == WASM_TARGET)
        })
        .unwrap_or(false)
}

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

    let adapter: Adapter = serde_saphyr::from_str(&fs::read_to_string(adapter_yaml)?)?;
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
