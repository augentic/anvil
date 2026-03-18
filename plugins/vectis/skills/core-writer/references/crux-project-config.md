# Crux Project Configuration

This reference describes the Cargo workspace layout, dependency management, feature flags,
and toolchain configuration for a Crux core project pinned to `crux_core-v0.17.0-rc3`.

## Directory Layout

```
{project-dir}/
    Cargo.toml              # workspace manifest
    rust-toolchain.toml     # toolchain and targets
    shared/
        Cargo.toml          # library manifest
        src/
            lib.rs
            app.rs
            ffi.rs
            {custom}.rs     # optional custom capability modules
```

## Workspace `Cargo.toml`

```toml
[workspace]
members = ["shared"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"

[workspace.dependencies]
crux_core = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
serde = "1.0"
facet = "=0.31"

[workspace.lints.rust]
trivial_numeric_casts = "warn"
unused_extern_crates = "warn"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
all = "warn"
nursery = "warn"
pedantic = "warn"
cargo = "warn"
as_pointer_underscore = "warn"
assertions_on_result_states = "warn"
clone_on_ref_ptr = "warn"
deref_by_slicing = "warn"
disallowed_script_idents = "warn"
empty_drop = "warn"
empty_enum_variants_with_brackets = "warn"
empty_structs_with_brackets = "warn"
fn_to_numeric_cast_any = "warn"
if_then_some_else_none = "warn"
map_err_ignore = "warn"
redundant_type_annotations = "warn"
renamed_function_params = "warn"
semicolon_outside_block = "warn"
undocumented_unsafe_blocks = "warn"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
unneeded_field_pattern = "warn"
unused_result_ok = "warn"
```

Add capability crates based on what the app needs:

```toml
# Add if using HTTP
crux_http = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }

# Add if using Key-Value
crux_kv = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }

# Add if using Time
crux_time = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }

# Add if using Platform detection
crux_platform = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
```

When Crux 0.17 is published to crates.io, replace git dependencies with versioned ones:

```toml
crux_core = "0.17"
crux_http = "0.16"   # check actual published version
```

## Shared Crate `Cargo.toml`

### Minimal (Render only)

```toml
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "lib", "staticlib"]

[[bin]]
name = "codegen"
required-features = ["codegen"]

[features]
uniffi = ["dep:uniffi"]
wasm_bindgen = ["dep:wasm-bindgen", "getrandom/wasm_js"]
codegen = [
    "crux_core/cli",
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "uniffi",
]
facet_typegen = ["crux_core/facet_typegen"]

[dependencies]
crux_core.workspace = true
serde = { workspace = true, features = ["derive"] }
facet.workspace = true

# optional dependencies
clap = { version = "4", optional = true, features = ["derive"] }
getrandom = { version = "0.3", optional = true, default-features = false }
log = { version = "0.4", optional = true }
pretty_env_logger = { version = "0.5", optional = true }
uniffi = { version = "=0.29.4", optional = true }
wasm-bindgen = { version = "0.2", optional = true }

[lints]
workspace = true
```

### With HTTP capability

Add to `[dependencies]`:

```toml
crux_http.workspace = true
url = "2"
```

Add to `[features]`:

```toml
codegen = [
    "crux_core/cli",
    "crux_http/facet_typegen",    # add typegen for each capability
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "uniffi",
]
facet_typegen = [
    "crux_core/facet_typegen",
    "crux_http/facet_typegen",
]
```

### With Key-Value capability

Add to `[dependencies]`:

```toml
crux_kv.workspace = true
serde_json = "1.0"              # for serializing values to bytes
```

Add typegen features:

```toml
codegen = [
    "crux_core/cli",
    "crux_kv/facet_typegen",
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "uniffi",
]
facet_typegen = [
    "crux_core/facet_typegen",
    "crux_kv/facet_typegen",
]
```

### With custom SSE capability

Add to `[dependencies]`:

```toml
async-sse = "5"
futures = "0.3"
```

### Dev dependencies

```toml
[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
```

## `crate-type` Explained

The `shared` crate compiles to three library types:

| Type | Purpose | Used by |
|------|---------|---------|
| `lib` | Standard Rust library | Rust shells, `cargo test`, codegen binary |
| `staticlib` | Static library (`.a`) | iOS (linked into Swift app via Xcode) |
| `cdylib` | C-ABI dynamic library (`.so`/`.dylib`) | Android (loaded via JNA in Kotlin) |

## `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "rustc-dev"]
targets = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-linux-android",
    "wasm32-unknown-unknown",
    "x86_64-apple-ios",
]
profile = "minimal"
```

This ensures the correct toolchain and cross-compilation targets are available.
Targets can be trimmed if not all platforms are needed.

## Feature Flags

| Feature | Purpose | When to enable |
|---------|---------|----------------|
| `uniffi` | UniFFI bindings for iOS/Android | Building for native mobile |
| `wasm_bindgen` | wasm-bindgen for Web | Building for WASM |
| `codegen` | Type generation CLI | Running `cargo run --bin codegen` |
| `facet_typegen` | Facet-based type generation | Used by codegen feature |

During normal development and testing, no features need to be enabled.
`cargo test` and `cargo check` work with default features.

## Codegen Binary

The `codegen` binary generates foreign-language types using `TypeRegistry` from
facet. It extracts types at compile time from the Rust type system (no RustDoc
JSON parsing), making it compatible with any Rust version >= 1.85.

The binary is declared in `Cargo.toml`:

```toml
[[bin]]
name = "codegen"
required-features = ["codegen"]
```

### Source: `shared/src/bin/codegen.rs`

Replace `MyApp` with the app struct name. The Swift package name `"SharedTypes"`
must match what the iOS shell's `project.yml` expects.

```rust
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::{
    cli::{BindgenArgsBuilder, bindgen},
    type_generation::facet::{Config, TypeRegistry},
};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::MyApp;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
    Typescript,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    language: Language,
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    let typegen_app = TypeRegistry::new()
        .register_app::<MyApp>()?
        .build()?;

    let name = match args.language {
        Language::Swift => "SharedTypes",
        Language::Kotlin => "com.example.app",
        Language::Typescript => "shared_types",
    };
    let config = Config::builder(name, &args.output_dir)
        .add_extensions()
        .add_runtimes()
        .build();

    match args.language {
        Language::Swift => {
            info!("Generating Swift types");
            typegen_app.swift(&config)?;
        }
        Language::Kotlin => {
            info!("Generating Kotlin types");
            typegen_app.kotlin(&config)?;

            info!("Generating Kotlin bindings");
            let bindgen_args = BindgenArgsBuilder::default()
                .crate_name(env!("CARGO_PKG_NAME").to_string())
                .kotlin(&args.output_dir)
                .build()?;
            bindgen(&bindgen_args)?;
        }
        Language::Typescript => {
            info!("Generating TypeScript types");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
```

Run it with:

```bash
cargo run --bin codegen --features codegen,facet_typegen -- --language swift --output-dir generated
```

The `--features codegen,facet_typegen` flags are both required: `codegen` enables the
CLI dependencies and `facet_typegen` enables type extraction via facet.

## `clippy.toml`

```toml
doc-valid-idents = []

allowed-duplicate-crates = []
```

- `doc-valid-idents` -- add domain-specific identifiers that appear in doc comments
  (prevents `doc_markdown` lint false positives).
- `allowed-duplicate-crates` -- suppress false-positive duplicate crate warnings from
  transitive dependencies. Run `cargo tree -d` after adding dependencies and populate
  this list with crate names that cannot be deduplicated.

## `.gitignore`

```
/target
Cargo.lock
```

Include `Cargo.lock` in `.gitignore` for library crates (the standard Rust convention).
If the project is an application (has binary targets other than codegen), keep `Cargo.lock`
tracked instead.

## Complete Example: Workspace with HTTP + KV

```toml
[workspace]
members = ["shared"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"

[workspace.dependencies]
crux_core = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
crux_http = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
crux_kv = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
serde = "1.0"
serde_json = "1.0"
facet = "=0.31"

[workspace.lints.rust]
trivial_numeric_casts = "warn"
unused_extern_crates = "warn"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
all = "warn"
nursery = "warn"
pedantic = "warn"
cargo = "warn"
as_pointer_underscore = "warn"
assertions_on_result_states = "warn"
clone_on_ref_ptr = "warn"
deref_by_slicing = "warn"
disallowed_script_idents = "warn"
empty_drop = "warn"
empty_enum_variants_with_brackets = "warn"
empty_structs_with_brackets = "warn"
fn_to_numeric_cast_any = "warn"
if_then_some_else_none = "warn"
map_err_ignore = "warn"
redundant_type_annotations = "warn"
renamed_function_params = "warn"
semicolon_outside_block = "warn"
undocumented_unsafe_blocks = "warn"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
unneeded_field_pattern = "warn"
unused_result_ok = "warn"
```
