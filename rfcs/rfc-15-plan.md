# RFC-15 — WASI tools

> **Plan reference for cross-session work.** Mirror of the in-flight plan
> originally created in plan mode. Update this file directly when the plan
> changes; agents picking up work should read it from
> `docs/plans/rfc-15-wasi-tools.md` rather than the per-user Cursor plan
> cache.

**Overview.** Implement [RFC-15 (`../../specify/rfcs/rfc-15-wasm-plugins.md`)](../../../specify/rfcs/rfc-15-wasm-plugins.md) as a **general-purpose extension mechanism for `specify-cli`**, not a capability-only feature. Add a new `specify-tool` crate that owns the tool manifest model, global tool cache, source resolver, and Wasmtime-backed runner. Expose `specify tool {run,list,fetch,show,gc}`. The first landing keeps the `specify` binary single, links Wasmtime in-process, and ships only the generic runner plus fixture coverage of both declaration sites. First-party `contract.wasm` migration is the last chunk and is *the only* chunk that touches `../specify`. Vectis migration is explicitly deferred because most of `specify-vectis verify` shells out to host toolchains and does not fit the filesystem-only WASI model RFC-15 §Permissions pins for the first landing.

**Design deviation from the source RFC.** [`rfc-15-wasm-plugins.md`](rfc-15-wasm-plugins.md) describes tools as a `tools:` array nested inside `capability.yaml`. This plan implements them as a general extension mechanism with **two declaration sites**:

1. `.specify/project.yaml` — top-level `tools:` array. Project-author scope. Always available, even for projects that resolve to `hub: true` and have no capability.
2. `<resolved-capability-dir>/tools.yaml` — sibling sidecar to `capability.yaml`. Capability-author scope. Optional; capabilities without a sidecar behave exactly as before.

The `specify-capability` crate is **not modified** by any chunk. `capability.yaml`'s schema stays closed against unknown top-level fields, and the `Capability` type gains no `tools` field. All tool parsing — including the capability sidecar — happens in `specify-tool`. The source RFC (`rfc-15-wasm-plugins.md`) needs a follow-up amendment to match this layout; that edit is out of scope for this plan and is logged in the post-execution log so a future change can land it.

**Read once before starting any chunk.** RFC-15 in full plus its §Implementation Plan, §Permissions, and §Resolver and Cache sections. The RFC's "tools nest inside capability.yaml" wording is superseded by this plan's §Decisions captured up-front; do not re-derive the manifest layout from the RFC alone.

## Chunk status

| ID | Chunk | Status |
| --- | --- | --- |
| chunk-0-decisions | Lock the on-disk + schema + dependency decisions; bump workspace deps for `wasmtime`, `wasmtime-wasi`, `ureq`, `sha2`, `dirs`; create the empty `crates/tool/` skeleton so later chunks can land in parallel without workspace-file collisions. | pending |
| chunk-1-manifest | Add `Tool`, `ToolPermissions`, `ToolSource`, and `ToolManifest` types to **the new `specify-tool` crate**; add a `tools: Vec<specify_tool::Tool>` field to `ProjectConfig` (in `src/config.rs`); add a `load_capability_sidecar(&Path)` loader for `<cap-dir>/tools.yaml`; ship `schemas/tool.schema.json`; add parser + validator unit tests. **No edits to `specify-capability`, `schemas/capability.schema.json`, or `../specify/capabilities/capability.schema.json`.** | pending |
| chunk-2-cache | Build the global tool cache: path computation, atomic stage-and-rename, sidecar metadata, `SPECIFY_TOOLS_CACHE` override, GC scan helper. No network, no Wasmtime — file ops only. Cache scoping uses generic `ToolScope` identifiers (`project:<project-name>` or `capability:<capability-slug>`), not capability-only. | pending |
| chunk-3-resolver | Add the source resolver (absolute local path / `file:` URI / `https:` URI), wire it into the cache from chunk 2, surface `Resolver` errors. The resolver is purely Source -> bytes and does not depend on `specify-capability`. | pending |
| chunk-4-host | Add the Wasmtime host: instantiate a WASI Preview 2 component, expand `$PROJECT_DIR` and (capability-scope tools only) `$CAPABILITY_DIR`, canonicalize permission paths, reject escapes, preopen, wire stdio + args + minimal env, propagate exit code. | pending |
| chunk-5-cli | Add `Commands::Tool { action }`, dispatch into the resolver + host, ship `run / list / fetch / show / gc`, merge tool declarations from project.yaml and the resolved capability's sidecar, and add the kebab-case JSON envelopes the CLI's v2 contract requires. | pending |
| chunk-6-fixture-acceptance | Add a `tools-test-project` fixture (declares tools in its own `project.yaml`) plus a `tools-test-cap` fixture capability (declares tools in a sidecar `tools.yaml`), plus the acceptance integration tests RFC-15 §Implementation Plan calls out (manifest validation, cache hit/miss, local + URI source, network failure, allowed + denied filesystem, non-zero exit propagation). | pending |
| chunk-7-docs-lints | Author `docs/reference/cli/tool.md`, add `docs/explanation/tool-declarations.md` covering the two declaration sites, refresh capability anatomy + cross-references, add the RFC-5 lint stubs (or doc TODOs) for "skill invokes a host binary when a declared tool exists" and "overly broad write permission". | pending |
| chunk-8-contract-wasm | Build `contract.wasm` from `crates/contract-validate`, ship it via a sidecar `tools.yaml` next to `capabilities/contracts/capability.yaml` (not by modifying `capability.yaml` itself), rewrite the contracts merge brief + verifier to call `specify tool run contract -- ...`, and either retire `crates/contract-validate`'s `[[bin]]` block or mark it transitional. Touches `../specify`. | pending |

When you complete a chunk, flip its status (`pending` → `in_progress` → `completed`) and add a short note under "Notes (post-execution log)" at the bottom if anything deviated from the plan. Forward-looking discoveries from one chunk should be folded into the bullets of later chunks (the `fold-vectis-into-specify.md` plan is the convention reference).

## Decisions captured up-front

These resolve the ambiguities flagged in the RFC-15 readiness review so subagents do not re-derive them. Where a choice deviates from a plain reading of the RFC, the rationale is recorded; if any of these turns out to be wrong on contact with the implementation, fix it here in the same commit that fixes the code.

### Declaration sites

Tools are declared in **one or both** of:

1. **Project scope.** A top-level `tools:` array in `.specify/project.yaml`. Available to every project, including hub projects that have no capability. Owned by the project author. Survives capability changes.
2. **Capability scope.** A `tools.yaml` file as a sibling of `capability.yaml` inside the resolved capability directory. Owned by the capability author and shipped with the capability. Capabilities without a sidecar work unchanged.

`specify tool ...` commands resolve their tool list by reading both sites and merging by `name`. **Project scope wins on collision** so operators can override capability-shipped declarations (e.g. to pin a different version or redirect `source:` to a local copy). Conflicts emit a typed `tool-name-collision` warning the first time they are observed in a session; merging proceeds.

The sidecar shape is the same `tools:` array as the project shape, so a single JSON Schema (`schemas/tool.schema.json`) governs both:

```yaml
# .specify/project.yaml (project scope)
tools:
  - name: contract
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/1.0.0/contract.wasm"
    sha256: "<hex-encoded sha256 of the component bytes>"
    permissions:
      read:  ["$PROJECT_DIR/contracts"]
      write: []

# <resolved-capability-dir>/tools.yaml (capability scope)
tools:
  - name: contract
    version: 1.0.0
    source: "file:///abs/path/to/contract-1.0.0.wasm"
    sha256: "<hex-encoded sha256 of the component bytes>"
    permissions:
      read:  ["$PROJECT_DIR/contracts"]
      write: []
```

`capability.yaml` is **never** modified by any chunk and never gains a `tools:` field. The `specify-capability` crate has no knowledge of tools.

### Crate boundary

- All tool types (`Tool`, `ToolPermissions`, `ToolSource`, `ToolManifest`, `ToolScope`) live in **`specify-tool`**. They depend only on `serde`, `semver`, `specify-error`, and (for resolution) `wasmtime`/`wasmtime-wasi`/`ureq`/`sha2`/`dirs`. They do **not** depend on `specify-capability`.
- `specify-capability` is **not modified** by any chunk. The `Capability` type, its serde shape, and `schemas/capability.schema.json` are all untouched.
- `src/config.rs::ProjectConfig` gains a single new field: `#[serde(default)] pub tools: Vec<specify_tool::Tool>`. The `specify` binary already imports both `specify-capability` and (after chunk 5) `specify-tool`, so this introduces no cycle.
- `specify-tool` exposes a `load_capability_sidecar(capability_dir: &Path) -> Result<Vec<Tool>, ToolError>` helper that the binary calls *after* `specify-capability` resolves the capability. The resolver itself never reads `capability.yaml`.
- CLI dispatch lives in **`src/commands/tool.rs`** with a `Commands::Tool { action }` variant in `src/cli.rs`.

### Cache layout

```
$SPECIFY_TOOLS_CACHE                                    # if set
  ↓ otherwise
$XDG_CACHE_HOME/specify/tools/                          # if set
  ↓ otherwise
$HOME/.cache/specify/tools/                             # POSIX default
```

Within the cache root:

```
<cache-root>/
└── <scope-segment>/
    └── <tool-name>/
        └── <version>/
            ├── module.wasm        # the cached component bytes
            └── meta.yaml          # sidecar metadata (see below)
```

- `<scope-segment>` is one of:
  - `project--<project-name>` for tools declared in `.specify/project.yaml`. The project name is the `name:` field from `project.yaml`.
  - `capability--<capability-slug>` for tools declared in a capability sidecar `tools.yaml`. The slug is the `name:` field from `capability.yaml`.
  Two unrelated declarers with identical `name` fields stay isolated. The `--` separator avoids collisions with tool names that contain a hyphen.
- `<version>` is the literal `version:` string from the `tools[]` entry. The resolver does not parse SemVer for path computation; it only validates SemVer at structural-validation time.
- `module.wasm` is always the literal filename. RFC-15's `<name>-<version>.wasm` shape is one directory deeper than its example suggests; keeping `module.wasm` flat lets `meta.yaml` sit next to its bytes without a name-mangling rule.

### Sidecar metadata (`meta.yaml`)

```yaml
schema-version: 1
scope: <scope-segment>            # e.g. project--my-app or capability--contracts
tool-name: <name>
tool-version: <version>
source: <literal source string from manifest>
fetched-at: <RFC-3339 timestamp>
permissions-snapshot:
  read:  [...]
  write: [...]
sha256: <optional hex digest copied from manifest>
```

`permissions-snapshot` is **informational only** in v1. RFC-15 §Trust says cached bytes are immutable until manifest source / version / digest changes; we do not invalidate the cache when permissions change, because permissions are evaluated per-`run` against the live manifest. Cache reuse rule: a sidecar whose `(scope, tool-name, tool-version, source, sha256)` tuple matches the live merged manifest is a cache hit. Any field mismatch forces a refetch into the same `<version>/` directory (atomic move). When `sha256` is present, the resolver verifies the fetched / copied bytes before installation and rejects existing sidecars whose recorded digest does not match the live manifest.

### Permission substitution + canonicalisation

- Substitutions apply **only** inside `tools[].permissions.{read,write}` entries. They do **not** apply to `tools[].source` (RFC-15 forbids relative paths and source variables) and they do **not** apply to `--` args passed to the module (the calling shell handles those; the host passes args verbatim).
- Supported variables:
  - `$PROJECT_DIR` — always available.
  - `$CAPABILITY_DIR` — only available to **capability-scope** tools (those declared in a sidecar `tools.yaml`). Project-scope declarations that reference `$CAPABILITY_DIR` are rejected at structural-validation time with `tool.capability-dir-out-of-scope`.
- After substitution: the path must be absolute. `..` segments are rejected before canonicalisation. The path is then canonicalised; if the canonical target is not a descendant of `PROJECT_DIR` or (for capability-scope tools) `CAPABILITY_DIR`, the request is denied even if the textual prefix matches (this catches symlink escapes per RFC-15 §Execution Host).
- `permissions:` absent **and** `permissions: { read: [], write: [] }` are equivalent: no preopens. The structural validator accepts both.
- `write:` entries must not grant authority over Specify lifecycle state. Reject writes to `.specify/project.yaml`, `.specify/slices/**/.metadata.yaml`, `.specify/archive/**/.metadata.yaml`, `.specify/plan.lock`, or any directory whose intended purpose is lifecycle transition / archive movement rather than capability-owned artifacts. Tools may write declared capability-owned outputs; lifecycle state still flows through core CLI verbs.

### Argument forwarding + environment

- `specify tool run <name> [-- <args>...]` forwards everything after `--` verbatim to the WASI module's `argv` (with `<name>` synthesised as `argv[0]`).
- Environment passed to the module is exactly two variables:
  - `PROJECT_DIR` — canonicalised project root (always set).
  - `CAPABILITY_DIR` — canonicalised resolved capability directory (set only for capability-scope tools; absent for project-scope tools, even if the project resolves to a capability).
- No host environment is inherited.
- Working directory of the module is the canonicalised project root.
- The first landing passes only explicit argv / stdio plus the two documented environment variables. Tools must not rely on inherited `PATH`, host user identity, wall-clock time, host randomness, runtime network access, or undeclared files for correctness. A helper that needs those belongs in a later declared host-runner RFC, not in this WASI runner.

### Exit code mapping

| Module / runner outcome | `specify tool run` exit code |
| --- | --- |
| Module exits 0 | 0 |
| Module exits N (1 ≤ N ≤ 255) | N |
| Module trap / panic at runtime | 2 (and a typed `runtime` error envelope) |
| Resolver error (manifest, source, network, permission) | 2 (typed `resolver` error envelope) |
| Project context missing for a verb that needs it | 1 (existing `not-initialized` envelope) |
| Tool name not found in either declaration site | 2 (typed `tool-not-declared` envelope) |

This mirrors the `0 / 1 / 2` shape `specify-contract-validate` already emits, so brief-side branching keeps working through the migration.

### Wasmtime configuration

- Pin `wasmtime` and `wasmtime-wasi` to the latest stable matching pair at the time chunk 0 lands. Use the **synchronous** WASI Preview 2 path (`wasmtime_wasi::add_to_linker_sync`) — RFC-15 explicitly disallows network access, and the synchronous path keeps the dependency tree free of `tokio`.
- Use `wasmtime::component::Component` (component model), not `wasmtime::Module` (core wasm). RFC-15 pins the WASI CLI command world.
- Disable filesystem access by default in the WASI context; preopens are added per-tool from manifest permissions only.
- Keep the execution implementation behind a narrow runner boundary (`ToolRunner` / `WasiRunner` or equivalent) so manifest parsing, cache resolution, and CLI output do not depend directly on Wasmtime. V1 still links Wasmtime in-process, but the boundary must make out-of-process execution or additional declared runner kinds possible later without rewriting the tool model.

### Diagnostics evolution

The first landing uses the WASI CLI command world: stdout / stderr plus exit code are the diagnostic channel. That is acceptable for the initial contract validator migration, but it is not the final validator ABI. When a helper needs machine-readable findings that skills must parse, add a custom WIT world with typed diagnostic exports and keep a thin command-world wrapper only for manual invocation.

### Cache concurrency

No file locks in v1. Two concurrent `specify tool run` invocations on a cold cache may both fetch and stage; the atomic rename in the resolver makes the steady state deterministic. Document this in the resolver module-level comment; defer the per-tool flock until it bites (RFC-15 Open Question 8).

### `specify tool gc` scope

`gc` deletes any `<cache-root>/<scope-segment>/<tool-name>/<version>/` whose `(scope, name, version, source)` tuple is **not** referenced by the merged tool list of the **current project** (project.yaml + resolved-capability sidecar, when present). It does not scan other projects on the host (the CLI cannot enumerate them). With `--all`, it instead deletes every directory not referenced by the current project's merged list — i.e. the same scan, narrower default. RFC-15 leaves this open; this is the "current project only" reading.

## End state (post-chunk-7)

```
specify-cli/
├── Cargo.toml                                # workspace members += "crates/tool"
├── crates/
│   ├── capability/                           # UNCHANGED — no edits in any chunk
│   └── tool/                                 # NEW crate
│       ├── Cargo.toml                        # name = "specify-tool"
│       └── src/
│           ├── lib.rs                        # re-exports
│           ├── manifest.rs                   # Tool, ToolPermissions, ToolSource, ToolManifest, ToolScope
│           ├── validate.rs                   # structural validators (name, version, source, permissions)
│           ├── load.rs                       # load_project_tools, load_capability_sidecar, merge
│           ├── cache.rs                      # cache root, layout, sidecar, gc scan
│           ├── resolver.rs                   # local + file: + https: source resolution
│           ├── host.rs                       # Wasmtime + WASI preview 2 runner
│           ├── permissions.rs                # substitute, canonicalise, escape-check
│           └── error.rs                      # ToolError enum (folded into specify-error)
├── schemas/
│   ├── capability.schema.json                # UNCHANGED
│   └── tool.schema.json                      # NEW — canonical tool item shape
├── src/
│   ├── cli.rs                                # Commands::Tool { action }
│   ├── config.rs                             # ProjectConfig gains `tools: Vec<specify_tool::Tool>`
│   └── commands/
│       └── tool.rs                           # NEW — run/list/fetch/show/gc handlers
├── tests/
│   ├── tool.rs                               # NEW — CLI-level integration tests
│   └── fixtures/
│       ├── tools-test-project/               # NEW — fixture project (project-scope tools)
│       │   ├── .specify/
│       │   │   └── project.yaml              # declares tools: [...]
│       │   └── wasm/
│       │       ├── echo.wasm
│       │       ├── read-only.wasm
│       │       └── read-write.wasm
│       └── tools-test-cap/                   # NEW — fixture capability (capability-scope tools)
│           ├── capability.yaml               # closed schema, no tools: field
│           ├── tools.yaml                    # NEW sidecar shape
│           └── wasm/
│               └── exit-seven.wasm
└── docs/
    ├── reference/cli/tool.md                 # NEW — `specify tool` reference
    └── explanation/tool-declarations.md      # NEW — declaration sites + precedence

../specify/                                   # touched only by chunks 7 (docs cross-link) and 8
├── capabilities/
│   ├── capability.schema.json                # UNCHANGED
│   └── contracts/
│       ├── capability.yaml                   # UNCHANGED (no tools: field)
│       ├── tools.yaml                        # NEW sidecar — declares contract.wasm (chunk 8)
│       └── briefs/merge.md                   # invokes `specify tool run contract` (chunk 8)
└── docs/reference/cli/contract.md            # rewritten to point at `specify tool run` (chunk 8)
```

## Dependency graph + parallelism

```
                ┌──────────────────────────────┐
                │ chunk-0 (decisions + skeleton)│
                └──────────────┬───────────────┘
                               │
              ┌────────────────┼─────────────────┐
              ▼                ▼                 ▼
        ┌─────────┐      ┌──────────┐      ┌──────────┐
        │ chunk-1 │      │ chunk-2  │      │ chunk-4  │
        │ manifest│      │  cache   │      │   host   │
        └────┬────┘      └─────┬────┘      └────┬─────┘
             │                 │                │
             │      ┌──────────┘                │
             │      ▼                           │
             │ ┌─────────┐                      │
             │ │ chunk-3 │                      │
             │ │resolver │                      │
             │ └────┬────┘                      │
             │      │                           │
             └──────┴────────────┬──────────────┘
                                 ▼
                            ┌─────────┐
                            │ chunk-5 │
                            │   CLI   │
                            └────┬────┘
                                 ▼
                          ┌─────────────┐
                          │  chunk-6    │
                          │ acceptance  │
                          └────┬────────┘
                               │
                  ┌────────────┴───────────┐
                  ▼                        ▼
            ┌──────────┐             ┌──────────┐
            │ chunk-7  │             │ chunk-8  │
            │  docs    │             │ contract │
            └──────────┘             └──────────┘
```

**Parallelism budget.**

- Chunks 1, 2, and 4 can run concurrently after chunk 0. They touch entirely separate files: chunk 1 lives in `crates/tool/src/{manifest,validate,load}.rs` + `schemas/tool.schema.json` + `src/config.rs`; chunk 2 lives in `crates/tool/src/cache.rs`; chunk 4 lives in `crates/tool/src/{host,permissions}.rs`. Chunk 0's empty `crates/tool/` skeleton is what unlocks this.
- Chunk 3 needs both 1 (`ToolSource` enum) and 2 (cache layout).
- Chunk 4 needs only 1 (`Tool` + `ToolScope` enums).
- Chunk 5 needs everything below it.
- Chunks 7 and 8 run after chunk 6. They are independent and can run concurrently, but chunk 8 is the riskier one and may take longer; start chunk 7 immediately after chunk 6 and let chunk 8 stretch.

**Suggested wave plan for parallel agents:**

| Wave | Concurrent chunks | Notes |
| --- | --- | --- |
| 1 | chunk-0 | Sequential. Everything depends on it. |
| 2 | chunk-1, chunk-2, chunk-4 | Three parallel agents. Zero file overlap. |
| 3 | chunk-3 | Sequential after wave 2. |
| 4 | chunk-5 | Sequential. CLI integration. |
| 5 | chunk-6 | Sequential. Acceptance tests gate the rest. |
| 6 | chunk-7, chunk-8 | Two parallel agents. |

A single sequential agent walking 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 is also fine; the parallel breakdown is for compute-bound landings.

## Chunks (each runnable by a separate agent session)

Each chunk ends with a concrete verification step. A fresh agent should be able to pick up any chunk from `git status`, this plan, and the RFC alone. Where a chunk references "the decisions table" it means the §Decisions captured up-front section above; do not re-derive.

### Chunk 0 — Decisions + skeleton

Goal: lock workspace dependencies and create the empty `crates/tool/` skeleton so chunks 1, 2, and 4 can land independently without workspace-file conflicts.

- Add to `[workspace.dependencies]` in [`Cargo.toml`](../../Cargo.toml):
  - `wasmtime = { version = "<latest stable>", default-features = false, features = ["component-model", "runtime", "cranelift"] }`
  - `wasmtime-wasi = { version = "<matching wasmtime>", default-features = false, features = ["preview2"] }`
  - `ureq = { version = "3", features = ["json", "tls"] }` — already a transitive workspace dep via `crates/vectis`; promote to `[workspace.dependencies]` so chunk 3 can pick it up cleanly.
  - `sha2 = "0.10"` — used by chunk 3 for manifest `sha256` verification and by chunk 2 sidecar tests that compare digest metadata.
  - `dirs = "5"` — XDG cache root resolution.
- Resolve "latest stable" by running `cargo search wasmtime --limit 1` in the local toolchain at chunk-0 time. Pin both `wasmtime` and `wasmtime-wasi` to the same major.minor. Update [`supply-chain/config.toml`](../../supply-chain/config.toml) `exemptions` only if `cargo vet` complains; do not pre-emptively add audit entries.
- Add `crates/tool/` skeleton:
  - `crates/tool/Cargo.toml` with `name = "specify-tool"`, workspace inheritance for `version`/`edition`/`license`/`repository`, and dependencies on `specify-error`, `wasmtime`, `wasmtime-wasi`, `ureq`, `sha2`, `dirs`, `serde`, `serde_json`, `serde-saphyr`, `chrono`, `semver`, `thiserror`. **Do not depend on `specify-capability`.**
  - `crates/tool/src/lib.rs` with `pub mod manifest; pub mod validate; pub mod load; pub mod cache; pub mod resolver; pub mod host; pub mod permissions; pub mod error;` and empty `pub fn placeholder() {}` stubs in each submodule so the crate compiles.
- Add `crates/tool` to `[workspace] members` in the root `Cargo.toml` (between `crates/vectis` and `crates/contract-validate` to keep the alphabetical-ish order).
- Add `specify-tool = { path = "crates/tool" }` to `[workspace.dependencies]`.
- Do **not** add `specify-tool` to the root `specify` binary's `[dependencies]` yet — chunk 1 does that (because `ProjectConfig` references `specify_tool::Tool`).
- Append the §Decisions captured up-front block of this plan into module-level rustdoc on `crates/tool/src/lib.rs` so downstream chunks can read it without leaving the codebase.
- Verify: `cargo build --workspace` is green; `cargo test --workspace` is green (no new tests yet, just confirming the skeleton compiles and existing tests don't regress); `cargo clippy --workspace --all-targets -- -D warnings` is clean.

### Chunk 1 — Manifest types + JSON Schema + ProjectConfig field

Goal: model the tool declaration, parse it from both declaration sites, and ship the JSON Schema. **No edits to `specify-capability`, `schemas/capability.schema.json`, or `../specify/`.**

- Add `crates/tool/src/manifest.rs` with the canonical model:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub version: String,             // exact SemVer; validated separately
    pub source: ToolSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,       // optional hex digest over component bytes
    #[serde(default)]
    pub permissions: ToolPermissions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ToolSource {
    LocalPath(PathBuf),  // absolute paths
    FileUri(String),     // file:// URIs
    HttpsUri(String),    // https:// URIs
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolPermissions {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
}

/// A `tools:` array as it appears in either declaration site.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolManifest {
    #[serde(default)]
    pub tools: Vec<Tool>,
}

/// Identifies which declaration site a tool came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolScope {
    Project { project_name: String },
    Capability { capability_slug: String, capability_dir: PathBuf },
}
```

  Note: the wire format for `source:` in YAML is a single string (e.g. `source: "https://..."`); add a `Deserialize` impl that classifies the string into the enum variant on parse, rather than asking authors to write a tagged form. The classifier rules: starts with `https://` → `HttpsUri`; starts with `file://` → `FileUri`; starts with `/` (or a Windows drive letter, if we ever care) → `LocalPath`; anything else is a parse error.

- Add `crates/tool/src/load.rs` with the two declaration-site loaders and the merger:

```rust
/// Read the project-scope `tools:` from a parsed `ProjectConfig`.
pub fn project_tools(cfg: &specify::ProjectConfig) -> Vec<(ToolScope, Tool)> { ... }

/// Read the capability-scope `tools.yaml` next to a resolved `capability.yaml`.
/// Returns Ok(vec![]) if the sidecar does not exist (capabilities without
/// tools are valid and remain unchanged).
pub fn load_capability_sidecar(
    capability_dir: &Path,
    capability_slug: &str,
) -> Result<Vec<(ToolScope, Tool)>, ToolError>;

/// Merge project + capability lists, with project precedence on name collisions.
/// Emits a `tool-name-collision` warning the first time a name is overridden.
pub fn merge_scoped(
    project: Vec<(ToolScope, Tool)>,
    capability: Vec<(ToolScope, Tool)>,
) -> (Vec<(ToolScope, Tool)>, Vec<Warning>);
```

  The `specify::ProjectConfig` reference here is resolved by the binary; `specify-tool` itself takes a generic `Vec<Tool>` parameter so it does not depend on the binary either. The signature above shows the conceptual API; the actual `project_tools` lives in the binary (`src/commands/tool.rs`) and just constructs the `Vec<(ToolScope, Tool)>` from `ProjectConfig::tools`.

- Add `crates/tool/src/validate.rs` with `Tool::validate_structure(&self, scope: &ToolScope) -> Vec<ValidationResult>`:
  - Rules: `tool.name-format` (kebab-case + ≤ 64 chars), `tool.version-is-semver` (uses `semver` crate, exact SemVer), `tool.source-is-supported-uri` (matches the `ToolSource` enum), `tool.sha256-format` (optional 64-character lowercase hex digest), `tool.permission-path-form` (every read/write entry is absolute or starts with `$PROJECT_DIR` or `$CAPABILITY_DIR`, no `..` segments, no glob chars), `tool.lifecycle-state-write-denied` (write entries may not target Specify lifecycle state paths), `tool.capability-dir-out-of-scope` (project-scope declarations cannot reference `$CAPABILITY_DIR`), `tool.name-unique` (enforced at the manifest level across `tools[]` within a single declaration site).
  - `ValidationResult` here is the same enum re-exported from `specify-capability`. To avoid the dependency, mirror the enum in `specify-tool` and convert at the binary layer; or move `ValidationResult` to `specify-error` (cleaner long-term, but a bigger refactor — defer unless the duplication bites). For chunk 1 the local mirror is fine; flag in the post-execution log.

- Ship `schemas/tool.schema.json` — canonical tool item + `tools:` array shape. This is the only schema chunk 1 ships.

```jsonc
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://specify.cli/schemas/tool.schema.json",
  "title": "WASI tool declaration",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "tools": {
      "type": "array",
      "items": { "$ref": "#/$defs/tool" }
    }
  },
  "$defs": {
    "tool": {
      "type": "object",
      "additionalProperties": false,
      "required": ["name", "version", "source"],
      "properties": {
        "name":    { "type": "string", "pattern": "^[a-z][a-z0-9-]*$", "maxLength": 64 },
        "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+(-[\\w.-]+)?(\\+[\\w.-]+)?$" },
        "source":  { "type": "string", "minLength": 1 },
        "sha256":  { "type": "string", "pattern": "^[a-f0-9]{64}$" },
        "permissions": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "read":  { "type": "array", "items": { "type": "string", "minLength": 1 } },
            "write": { "type": "array", "items": { "type": "string", "minLength": 1 } }
          }
        }
      }
    }
  }
}
```

- Extend `src/config.rs::ProjectConfig` with a single new field:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub tools: Vec<specify_tool::Tool>,
```

  This is the only `ProjectConfig` change. Add `specify-tool = { workspace = true }` to the root `specify` binary's `[dependencies]` in `Cargo.toml` so the import resolves. Existing `project.yaml` files that omit `tools:` continue to load unchanged because of the `default`.

- **Do not modify `crates/capability/`, `schemas/capability.schema.json`, or `../specify/capabilities/capability.schema.json`.** A future chunk-7 doc edit cross-links the tool docs from the capability anatomy doc, but no schema or code changes happen in either capability schema.

- Tests (live in `crates/tool/src/manifest.rs` + `crates/tool/src/load.rs` test modules):
  - YAML round-trip of a `ToolManifest` containing each `ToolSource` variant.
  - `load_capability_sidecar` returns `Ok(vec![])` when no `tools.yaml` exists.
  - `load_capability_sidecar` rejects a `tools.yaml` whose top-level shape is not `{ tools: [...] }`.
  - `merge_scoped` precedence: identical `name` from both scopes → project wins; emits one `Warning::ToolNameCollision { name }`.
  - JSON Schema rejection: bad name (uppercase), bad version (non-SemVer), bad source (relative path, `oci://`), bad sha256 (wrong length / uppercase / non-hex), permission path with `..`, lifecycle-state write path, duplicate tool names, unknown property under `tools[].permissions`.
  - `Tool::validate_structure` rejects `$CAPABILITY_DIR` in a project-scope tool with `tool.capability-dir-out-of-scope`.
  - `ProjectConfig` round-trip: `tools:` absent → `Vec::new()`; `tools:` present → parses into `Vec<Tool>`; serialise back is byte-stable when present, omits `tools:` when empty.

- Verify: `cargo test -p specify-tool`; `cargo test -p specify` (the binary's `config` tests still pass with the new field); `cargo build --workspace`; manual `specify capability check schemas/omnia` still passes (omnia declares no tools and `capability.yaml` is unchanged).

### Chunk 2 — Cache layout + sidecar + gc scan

Goal: deterministic on-disk cache management. No network, no Wasmtime, no manifest knowledge beyond the data shapes already in chunk 1.

- In `crates/tool/src/cache.rs`, implement:
  - `pub fn cache_root() -> Result<PathBuf, ToolError>` — applies the §Decisions cache root precedence (`SPECIFY_TOOLS_CACHE` → `XDG_CACHE_HOME/specify/tools` → `$HOME/.cache/specify/tools`). Use `dirs::cache_dir`.
  - `pub fn scope_segment(scope: &ToolScope) -> String` — formats `project--<name>` or `capability--<slug>`.
  - `pub fn tool_dir(scope: &ToolScope, name: &str, version: &str) -> PathBuf` — builds `<cache-root>/<scope-segment>/<name>/<version>/`.
  - `pub struct Sidecar { ... }` plus `read_sidecar(&Path)` / `write_sidecar(&Path, &Sidecar)`. Include the optional `sha256` copied from the live manifest. Format is YAML for symmetry with `.cache-meta.yaml`. Schema lives in a new `crates/tool/schemas/tool-sidecar.schema.json` with `include_str!` validation on read.
  - `pub fn cache_status(scope, tool, sidecar) -> CacheStatus { Hit, MissNotFound, MissChanged }` — implements the `(scope, tool-name, tool-version, source, sha256)` reuse rule.
  - `pub fn stage_and_install(staged: &Path, dest: &Path) -> Result<(), ToolError>` — copies into a sibling tempdir and `rename`s atomically. Mirrors the pattern used in `crates/registry/src/...` if one exists; if not, model on `tempfile::NamedTempFile::persist`.
  - `pub fn scan_for_gc(scope: &ToolScope, kept: &HashSet<(String, String, String)>) -> Result<Vec<PathBuf>, ToolError>` — returns directories under the scope segment that are **not** in the `(name, version, source)` keep-set. Caller decides whether to delete.

- `crates/tool/src/error.rs` defines `pub enum ToolError` with variants `CacheRoot`, `Io`, `SidecarParse`, `SidecarSchema`, `AtomicMoveFailed`, `ToolNotDeclared`, `ToolNameCollision`. Add a `From<ToolError> for specify_error::Error` so the CLI can surface these through the existing exit-code mapper. Use the existing `Error::Config(String)` carrier if no new variant fits cleanly; if a new variant is justified, add it to `crates/error/src/lib.rs` in the same chunk.

- Tests:
  - `cache_root` honours each precedence step (use `temp_env::with_var` or a process-local helper since `std::env::set_var` is global; if no helper exists, gate the test behind `#[serial_test::serial]` or a manual mutex — match the existing test convention if any).
  - `scope_segment` round-trips both `Project` and `Capability` variants and rejects empty names.
  - `stage_and_install` is atomic: simulate a process kill mid-copy by writing to the tempdir and `rename`ing manually, assert the destination either exists fully or not at all.
  - Sidecar round-trip + JSON Schema rejection, including digest mismatch producing `MissChanged`.
  - `scan_for_gc` correctly distinguishes kept vs unkept entries and isolates project from capability scopes.

- Verify: `cargo test -p specify-tool`; `cargo clippy -p specify-tool --all-targets -- -D warnings`.

### Chunk 3 — Source resolver

Goal: turn a `ToolSource` (from chunk 1) into a populated cache directory (from chunk 2). Network only happens here.

- In `crates/tool/src/resolver.rs`, implement:
  - `pub fn resolve(scope: &ToolScope, tool: &Tool) -> Result<ResolvedTool, ToolError>` — the orchestrator. Looks at `cache_status`; on `Hit` returns the existing `module.wasm` path only when the sidecar's optional `sha256` matches the live manifest. On miss, dispatches by `ToolSource` variant, stages the bytes in a tempdir, validates the file is non-empty (defer real WASM signature validation to the host on first instantiation — the host will reject malformed components anyway), verifies `sha256` when declared, writes the sidecar, atomic-installs.
  - `LocalPath` → file copy (chase symlinks; reject non-files).
  - `FileUri` → strip scheme, then `LocalPath`.
  - `HttpsUri` → `ureq::get(url).call()` with a 30s timeout; require 200; cap response size at 64 MiB by default and return a typed error above that. Do not follow `http://` redirects targeting non-`https://` (defense in depth).

- `pub struct ResolvedTool { pub bytes_path: PathBuf, pub scope: ToolScope, pub sidecar: Sidecar }` — the value chunk-4's host consumes. Carries the scope so chunk 4 knows whether to expose `$CAPABILITY_DIR`.

- Reuse `ureq` (already a workspace dep after chunk 0). Construct an `Agent` with sensible timeouts; mirror the pattern in `crates/vectis/src/update_versions/query.rs`.

- Tests:
  - Local-path source: cache-miss copies the file, cache-hit reuses, cache-changed re-stages.
  - Digest verification: correct `sha256` installs and runs; wrong digest fails before cache install and leaves the previous cache entry untouched.
  - `file://` source: equivalence with absolute local paths.
  - `https://` source: use `httpmock` or a hand-rolled `std::net::TcpListener` fixture (if `httpmock` is not in workspace deps yet, add it as a `[dev-dependencies]` line on `crates/tool/Cargo.toml`); cover 200, 404, 500, payload-too-large, timeout, malformed URL.
  - Air-gapped: with the network blocked and an empty cache, resolution returns a typed error whose message names the URL.
  - Project vs capability scope produce isolated cache dirs even when `(name, version, source)` matches.

- Verify: `cargo test -p specify-tool`; manual `cargo run -p specify-tool --example resolve <url>` is **not** required — the resolver is library-only until chunk 5.

### Chunk 4 — Wasmtime host

Goal: instantiate a WASI Preview 2 component, preopen exactly the declared directories, run it, propagate exit code. No CLI dispatch yet.

- In `crates/tool/src/permissions.rs`:
  - `pub fn substitute(template: &str, project_dir: &Path, capability_dir: Option<&Path>) -> Result<String, ToolError>` — performs the `$PROJECT_DIR` / `$CAPABILITY_DIR` substitution. Rejects `..` segments. Rejects `$CAPABILITY_DIR` when `capability_dir` is `None` (project-scope tools). Rejects strings that reference any other variable.
  - `pub fn canonicalise_under(target: &Path, allowed_roots: &[&Path]) -> Result<PathBuf, ToolError>` — canonicalises `target` (must exist; RFC-15 §Permissions: "Permission directories must already exist before they are preopened"), then rejects when the canonical target is not a descendant of any `allowed_root`. This is the symlink-escape catcher.

- In `crates/tool/src/host.rs`:
  - Define a narrow runner boundary before introducing Wasmtime-specific code: `pub trait ToolRunner { fn run(&self, resolved: &ResolvedTool, ctx: &RunContext) -> Result<i32, ToolError>; }` (or an equivalent small abstraction). The v1 implementation is `WasiRunner`; callers depend on the trait / boundary, not directly on Wasmtime types.
  - `pub struct WasiRunner { engine: Engine, ... }` constructed once per process. The `Engine` is reusable across runs.
  - `pub fn run(&self, resolved: &ResolvedTool, ctx: &RunContext) -> Result<i32, ToolError>` where `RunContext` carries `project_dir`, `args`, `stdio` handles, and (when `resolved.scope` is `Capability`) the canonical capability dir.
  - Steps inside `run`:
    1. Canonicalise `project_dir` and (if capability-scope) `capability_dir`.
    2. Substitute + canonicalise each `read` and `write` permission entry; collect into `Vec<(PathBuf, DirPerms, FilePerms)>`. The substitution helper receives `Option<&Path>` for capability dir based on scope.
    3. Build a `wasmtime_wasi::WasiCtxBuilder`, add args (with `<tool-name>` as `argv[0]`), env (`PROJECT_DIR` always; `CAPABILITY_DIR` only for capability-scope tools), inherit stdin/stdout/stderr from the host process. Do not inherit host env, current user identity, `PATH`, network capability, or implicit filesystem access. The command-world host does not promise stable wall-clock time or randomness; deterministic tools must not depend on those for correctness.
    4. For each preopen, call `WasiCtxBuilder::preopened_dir` with the matching `DirPerms` / `FilePerms`. Read-only preopens: `DirPerms::READ`, `FilePerms::READ`. Read-write preopens: `DirPerms::READ | DirPerms::MUTATE`, `FilePerms::READ | FilePerms::WRITE`. (Wasmtime's WASI `mutate` covers create/unlink/rename.)
    5. Compile `Component::from_file(&engine, &resolved.bytes_path)`, link, instantiate.
    6. Call the `wasi:cli/run` export. Exit code 0 = `0`; explicit `wasi::cli::exit::exit(N)` traps via Wasmtime's typed exit propagation — convert to `i32` and clamp to `0..=255`.
    7. Map wasmtime traps + linker errors to `ToolError::Runtime(String)`; map filesystem permission denials to `ToolError::PermissionDenied(PathBuf)`.

- Tests (using a tiny precompiled fixture from chunk 6 — for chunk 4 itself, embed a 50-line `tools-test/src/echo.wasm` byte blob via `include_bytes!` *or* check in a tiny `.wat`-derived `.wasm`; chunk 6 will replace this with a richer fixture):
  - Non-zero exit propagates: a fixture that calls `process::exit(7)` returns `Ok(7)` from `Host::run`.
  - Permission denial: a fixture that tries to read a path outside its preopens fails; the host surfaces a typed error.
  - Symlink escape: create a symlink inside `PROJECT_DIR` pointing to `/tmp`, declare it in `permissions.read`, expect the canonicaliser to reject before instantiation.
  - Lifecycle-state write denial: declaring `permissions.write: ["$PROJECT_DIR/.specify"]` or a slice `.metadata.yaml` path fails before instantiation with `tool.lifecycle-state-write-denied` / `ToolError::PermissionDenied`.
  - Argument forwarding: a fixture that `println!`s its args returns the expected stdout.
  - Env exposure (capability-scope): a fixture that prints `env::var("PROJECT_DIR")` + `env::var("CAPABILITY_DIR")` matches the canonicalised paths.
  - Env exposure (project-scope): the same fixture run under a project-scope `ResolvedTool` errors on `env::var("CAPABILITY_DIR")` (variable absent) and `env::var("PATH")` errors (no host env inherited).
  - `$CAPABILITY_DIR` in a project-scope permission entry: rejected at substitute time.

- Verify: `cargo test -p specify-tool` passes the new tests; `cargo clippy -p specify-tool --all-targets -- -D warnings` is clean.

### Chunk 5 — `specify tool` CLI surface

Goal: stitch chunks 1-4 into the binary. Expose the five verbs RFC-15 specifies, merge the two declaration sites, and emit kebab-case JSON envelopes.

- `specify-tool = { path = "crates/tool" }` is already in the root `Cargo.toml` `[dependencies]` (chunk 1 added it for `ProjectConfig::tools`).

- In [`src/cli.rs`](../../src/cli.rs), extend `Commands` with:

```rust
/// WASI tool runner (RFC-15).
Tool {
    #[command(subcommand)]
    action: ToolAction,
},
```

  And define:

```rust
#[derive(Subcommand)]
pub enum ToolAction {
    Run {
        name: String,
        /// Args forwarded to the tool after `--`.
        #[arg(last = true)]
        args: Vec<String>,
    },
    List,
    Fetch { name: Option<String> },
    Show { name: String },
    Gc {
        /// Without --all, removes only entries the merged tool list does not reference.
        #[arg(long)]
        all: bool,
    },
}
```

- In [`src/commands/mod.rs`](../../src/commands/mod.rs), add `pub mod tool;` and wire `Commands::Tool { action } => match action { ... }`. Every verb requires project context (uses `run_with_project`); after loading `ProjectConfig`, the dispatcher resolves the capability (if any) via `specify-capability`, then builds the merged tool list:

```rust
fn merged_tools(
    project: &ProjectConfig,
    project_dir: &Path,
    capability: Option<&ResolvedCapability>,
) -> Result<Vec<(ToolScope, Tool)>, Error> {
    let project_scope = ToolScope::Project { project_name: project.name.clone() };
    let mut merged: Vec<(ToolScope, Tool)> = project
        .tools
        .iter()
        .cloned()
        .map(|t| (project_scope.clone(), t))
        .collect();
    if let Some(cap) = capability {
        let sidecar = specify_tool::load::load_capability_sidecar(
            &cap.root_dir,
            &cap.schema.name,
        )?;
        let (joined, warnings) = specify_tool::load::merge_scoped(merged, sidecar);
        for w in warnings { /* surface as kebab JSON warning entries */ }
        merged = joined;
    }
    Ok(merged)
}
```

  `tool fetch` without a `<name>` resolves the merged list and prefetches every tool, exactly as RFC-15 §Trust and Offline Behavior spells out.

- In `src/commands/tool.rs`:
  - `run_tool_run(ctx, name, args)` — load merged tools, find by `name` (project-scope wins on collision), resolve via `specify_tool::resolver::resolve`, build a `RunContext` (passing the capability dir only when scope is `Capability`), call the v1 `WasiRunner` through the tool-runner boundary, return a `CliResult` whose code matches the §Decisions exit-code table. If `name` is not in the merged list, return `Error::ToolNotDeclared { name }`.
  - `run_tool_list(ctx)` — for every merged tool, report `name`, `version`, `source`, `scope`, cache status (hit / miss). JSON envelope: `{ "schema-version": 2, "tools": [{ "name": ..., "version": ..., "source": ..., "scope": "project|capability", "scope-detail": "<project-name|capability-slug>", "cache-status": "hit|miss", "cached-path": "..." }] }`.
  - `run_tool_fetch(ctx, name)` — when `name` is `None`, fetch every tool. Prints per-tool result. JSON envelope reuses the `list` shape but includes a `fetched: true|false` flag.
  - `run_tool_show(ctx, name)` — show metadata, permissions, scope, cache status. JSON envelope is the `list` row plus the full `permissions` block and the sidecar's `fetched-at`.
  - `run_tool_gc(ctx, all)` — invoke `cache::scan_for_gc` once per scope present in the merged list, delete the unused dirs, report `removed: [...]`.

- Output contract: every JSON envelope flows through `emit_response` (re-uses the kebab-case + `schema-version: 2` injection that other handlers already use). Text formatters are humanised one-liner-per-tool tables, with a `[scope]` annotation column.

- Add new error variants to [`crates/error/src/lib.rs`](../../crates/error/src/lib.rs) **only if** `ToolError → Error` cannot fit the existing variants cleanly. Likely candidates: `Error::ToolResolver(String)`, `Error::ToolRuntime { exit_code: i32, detail: String }`, `Error::ToolPermissionDenied(String)`, `Error::ToolNotDeclared { name: String }`. Mirror the kebab variant strings into [`src/output.rs`](../../src/output.rs)'s error formatter.

- `Cli::Tool` integration test sketch (real coverage lands in chunk 6):
  - `specify tool --help` lists `run`, `list`, `fetch`, `show`, `gc`.
  - `specify tool list` outside a project errors with `not-initialized`.
  - `specify tool list` inside a hub project (no capability) returns only project-scope tools.

- Verify: `cargo build --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.

### Chunk 6 — Fixture project + capability + acceptance tests

Goal: prove the full pipeline end-to-end across **both declaration sites**, and pin the RFC-15 acceptance list against integration tests inside `specify-cli`.

- Create `tests/fixtures/tools-test-project/` (project-scope coverage):
  - `.specify/project.yaml` declaring three tools at the top-level `tools:` array: `echo` (no permissions), `read-only` (a single `read` preopen on `$PROJECT_DIR/inputs`), `read-write` (`read` on `$PROJECT_DIR/inputs`, `write` on `$PROJECT_DIR/outputs`). All three use `source: file://<absolute path to checked-in .wasm>` so the test does not need network. `capability:` is omitted (`hub: true`) to prove the project-scope path works without a capability.
  - `wasm/echo.wasm`, `wasm/read-only.wasm`, `wasm/read-write.wasm` — three tiny WASI Preview 2 components. Build them in a sibling crate `tests/fixtures/tools-test-project/src-rust/` with `crate-type = ["cdylib"]` and `cargo build --target wasm32-wasip2 --release`; commit the resulting `.wasm` blobs to the repo. Add a `Makefile` target `make tools-test-fixtures` for regen, but do not gate CI on rebuild — the binary blobs are checked in.
  - Each tool is ~30 lines: `echo` prints argv + env to stdout and exits 0; `read-only` reads `inputs/probe.txt` and prints its contents; `read-write` reads `inputs/probe.txt`, writes a derived value to `outputs/result.txt`.

- Create `tests/fixtures/tools-test-cap/` (capability-scope coverage):
  - `capability.yaml` with the standard closed shape and **no `tools:` field** (capability schema is unmodified). Pipelines reference one no-op brief per phase.
  - `tools.yaml` sidecar declaring one tool: `exit-seven` with `source: file://<wasm path>`, `permissions: {}`, `version: 0.1.0`. This proves the sidecar path.
  - `wasm/exit-seven.wasm` — ~10-line tool that exits 7. Used for non-zero-exit propagation.
  - A driver project under `tests/fixtures/tools-test-project-cap/` that declares `capability: <abs path to tools-test-cap>` and **no project-scope tools**, so `specify tool run exit-seven` exercises the capability sidecar exclusively.

- Author `tests/tool.rs` covering RFC-15 §Implementation Plan acceptance:
  - **Manifest validation, project scope.** `specify tool list` inside `tools-test-project` returns three tools with `scope: project`. Mutated copies (bad name, bad version, bad source, bad `sha256`, perm with `..`, lifecycle-state write permission, `$CAPABILITY_DIR` in a project-scope perm) each fail with the matching `rule-id`.
  - **Manifest validation, capability scope.** `specify tool list` inside the driver project returns one tool with `scope: capability`. A mutated `tools.yaml` (bad source, perm with `..`) fails with the matching `rule-id`. `capability.yaml` itself stays valid throughout.
  - **Cache miss + hit.** Cold run resolves and populates `<SPECIFY_TOOLS_CACHE>/project--tools-test/echo/0.1.0/`. Second run is a cache hit (assert sidecar `fetched-at` is unchanged). Capability-scope tools land in `<SPECIFY_TOOLS_CACHE>/capability--tools-test-cap/exit-seven/0.1.0/`. Changing only `sha256` forces `MissChanged` and refetch.
  - **Digest verification.** Correct `sha256` runs. Wrong `sha256` fails before execution, reports a typed resolver error, and does not install the staged bytes into the cache.
  - **Local-path source.** Covered by the fixture's `file://` sources.
  - **URI source.** Spin up a `httpmock` server, point a fixture tool at `http://127.0.0.1:<port>/echo.wasm`, assert the resolver fetches and runs it. (RFC-15 forbids plaintext `http:` in production manifests; the schema rejects it. For this test, hand-construct the `Tool` struct in-process or use a private test-only `ToolSource::HttpsUri` that the test resolver bypasses TLS for.)
  - **Network failure.** Point a tool at a port nobody is listening on; the run errors with a typed `tool-resolver` envelope; exit code `2`.
  - **Allowed filesystem access.** `read-write` writes `outputs/result.txt`; the file appears.
  - **Denied filesystem access.** A fourth fixture tool attempts to read `/etc/passwd`; assert the run fails with `tool-permission-denied`. (Implement this as an integration test that mutates the fixture project's `tools:` array in a tempdir copy of `project.yaml`, so the in-tree fixture stays clean.)
  - **Denied lifecycle write access.** A fixture manifest that grants `write: ["$PROJECT_DIR/.specify"]` or a slice `.metadata.yaml` path fails during structural validation / permission preparation; no WASM component is instantiated.
  - **Non-zero exit propagation.** `specify tool run exit-seven` from the capability-driver project exits 7.
  - **Synthetic-tool fixture run.** `specify tool run echo -- hello world` prints `hello world` to stdout.
  - **`SPECIFY_TOOLS_CACHE` override.** Setting the env var redirects every cache write to a tempdir; assert the `<scope-segment>` paths exist.
  - **Tool-name collision.** Both fixtures declare a tool named `echo` (project-scope and capability-scope variants). `specify tool list` shows the project-scope one with `scope: project`, emits a `tool-name-collision` warning, and `specify tool run echo` runs the project-scope version.
  - **Hub project.** A driver project with `hub: true` and `capability:` omitted but `tools: [echo]` runs `specify tool run echo` successfully — no capability, no sidecar.

- Update the `tests/fixtures/` README (or create one) explaining how to rebuild the `.wasm` blobs and why they are checked in.

- Verify: `cargo test --workspace`; manually run the acceptance scenarios above as a sanity pass; `cargo clippy --workspace --all-targets -- -D warnings`.

### Chunk 7 — Docs + lints

Goal: teach operators about `specify tool` and tighten the lint surface so RFC-15's invariants are enforced going forward.

- Author `../specify/docs/reference/cli/tool.md`. Mirror the structure of [`../specify/docs/reference/cli/capability.md`](../../../specify/docs/reference/cli/capability.md): one section per verb (`run`, `list`, `fetch`, `show`, `gc`), exit-code table, JSON envelope shape, cache-location explanation, digest verification behavior, security notes (single-binary, mandatory permissions, no host env, no network from tools), and the determinism policy (no inherited `PATH`, clock/random dependence, undeclared files, or lifecycle-state writes).
- Author `../specify/docs/explanation/tool-declarations.md` — explains the two declaration sites, scope precedence, `$CAPABILITY_DIR` availability rules, the cache-segmentation strategy, optional `sha256` pins, and when to pick project vs capability scope. Include a worked example for each.
- Add a row for `specify tool` to [`../specify/docs/reference/cli/index.md`](../../../specify/docs/reference/cli/index.md). Also delete the stale `specify-contract-validate` and `specify-vectis` rows **only after chunk 8 lands** — for chunk 7, leave them. Track this in the post-execution log.
- Update [`../specify/docs/contributing/capability-anatomy.md`](../../../specify/docs/contributing/capability-anatomy.md) with a §"Optional tool sidecar" subsection that:
  - Shows that capabilities **may** ship a `tools.yaml` next to `capability.yaml`, but the `capability.yaml` schema itself remains closed and unchanged.
  - States that absolute local paths and `file://` are intended for vendored / first-party tools; `https://` for third-party.
  - Recommends `sha256` for every released tool artifact and requires it for first-party release declarations.
  - Cross-links RFC-15, the new `tool.md`, and the new `tool-declarations.md`.
- Lint stubs in `crates/validate` (or wherever the existing RFC-5 lints live):
  - `tool.write-permission-too-broad` — flags `write:` entries whose canonicalised target is `$PROJECT_DIR` itself (i.e. write everywhere). Implementer's call whether to error or warn; default to warn for v1.
  - `tool.lifecycle-state-write-denied` — errors when a tool asks for write access to Specify lifecycle state (`.specify/project.yaml`, slice metadata, archive metadata, plan locks, or archive movement directories).
  - `skill.invokes-host-binary-with-declared-tool-equivalent` — scans `briefs/*.md` for `specify-contract-validate` / `specify-vectis` invocations when the merged tool list declares a tool of the same purpose. Default to warn; this becomes an error after chunk 8 lands.
  - If RFC-5 is not yet implemented end-to-end, leave a `// TODO(rfc-5)` comment with the rule id and a fixture and ship the doc update only.
- Cross-link RFC-15 from RFC-13's "Open Question 4" disposition note: append a one-liner saying "Resolved by RFC-15 (`../rfc-15-wasm-plugins.md`)" under `../specify/rfcs/archive/rfc-13-extensibility.md` §Open Questions item 4. **This is the only `../specify` RFC edit chunk 7 makes.**
- Log in the post-execution notes that `rfc-15-wasm-plugins.md` itself still describes the RFC's original "tools nest inside capability.yaml" shape and needs a follow-up amendment to match the implemented two-site layout. Do not edit the RFC source in chunk 7.
- Verify: `make checks` from `../specify` is green (the existing `validateCapabilityYaml` step still passes because `capability.yaml`'s schema is unchanged); `cargo test --workspace` from `specify-cli` is green.

### Chunk 8 — First-party `contract.wasm` migration

Goal: retire `specify-contract-validate` as the operator-visible binary and make `specify tool run contract` the canonical merge gate. Touches both repos. **No edits to `capability.yaml`** — the tool is declared in a sidecar `tools.yaml`.

- In `specify-cli`:
  - Add a `wasm32-wasip2` target build for `crates/contract-validate`. Two viable shapes:
    - **(a)** Add a `[lib]` block that exposes the validator as a library (it already mostly is via `specify-validate`), plus a thin `crates/contract-validate/wasm/` subcrate with `crate-type = ["cdylib"]` that wraps the library in a WASI Preview 2 entry point. Build with `cargo build --target wasm32-wasip2 --release -p specify-contract-validate-wasm`. Commit the resulting `.wasm` to `crates/contract-validate/dist/contract-<version>.wasm`.
    - **(b)** Move the `[[bin]]` to a fresh `crates/contract-validate-wasm/` and drop the existing `[[bin]]` block once the WASI entry point is verified.
    Pick (a) unless the dependency on `specify-validate` (which itself depends on `specify-error`, `specify-spec`, etc.) doesn't compile clean for `wasm32-wasip2`. If it doesn't, vendor the validator's logic into a smaller standalone module — `wasm32-wasip2` is unforgiving about transitive deps that touch threads, sockets, or `std::process`.
  - Decide where to host the `.wasm`: chunk 8 lands the bytes in-tree under `crates/contract-validate/dist/`; the `tools.yaml` sidecar uses `file:///<absolute path>` for development. The §Open Questions in RFC-15 leave the public hosting choice (`https://github.com/augentic/specify-tools/releases/...`) for follow-up work; document the gap in the post-execution log.
  - Run the chunk 6 acceptance tests against the new `contract.wasm` to make sure the WASI build behaves identically to the host binary on a representative baseline.
  - Update the `crates/contract-validate` `[[bin]]` block: either delete it (preferred) or annotate as transitional with a doc comment pointing at `specify tool run contract`.

- In `../specify`:
  - Create [`capabilities/contracts/tools.yaml`](../../../specify/capabilities/contracts/tools.yaml) **as a new file** (sibling to `capability.yaml`):

```yaml
# Capability-scope tool declarations for the contracts capability.
# Schema: ../../../specify-cli/schemas/tool.schema.json
tools:
  - name: contract
    version: <validator version>
    source: "file:///<absolute path within specify-cli>/crates/contract-validate/dist/contract-<version>.wasm"
    sha256: "<hex-encoded sha256 of contract-<version>.wasm>"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

  - **Do not modify [`capabilities/contracts/capability.yaml`](../../../specify/capabilities/contracts/capability.yaml).** The capability manifest schema remains closed and unchanged.
  - Rewrite [`capabilities/contracts/briefs/merge.md`](../../../specify/capabilities/contracts/briefs/merge.md) and [`plugins/contract/skills/openapi/verifier.md`](../../../specify/plugins/contract/skills/openapi/verifier.md): replace every `specify-contract-validate "$PROJECT_ROOT/contracts" --format json` invocation with `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. The shape of the JSON output is unchanged because the WASI module wraps the same library.
  - Refresh [`docs/reference/cli/contract.md`](../../../specify/docs/reference/cli/contract.md): retitle to "Contract validator (WASI tool)", redirect the "Distribution" section to `specify tool run contract`, drop the `cargo install specify` / `brew install` instructions (the binary is no longer the public surface), and link to RFC-15 and the new `tool-declarations.md`.
  - Delete the `specify-contract-validate` row from [`docs/reference/cli/index.md`](../../../specify/docs/reference/cli/index.md). (Chunk 7 deferred this; chunk 8 owns the actual delete.)
  - Refresh `verifier.md`'s "Pre-conditions" block: `specify tool run contract` now requires `specify` on `$PATH`, not `specify-contract-validate`.

- Verify, end-to-end:
  - From `specify-cli`: `cargo test --workspace` is green.
  - From `../specify`: `make checks` is green.
  - In a sandbox project that resolves to the contracts capability: `specify tool list` shows `contract` at `scope: capability`. `specify slice merge run` followed by the contracts capability merge brief calls `specify tool run contract -- "$PROJECT_ROOT/contracts"` and emits the same JSON envelope `specify-contract-validate` used to emit. Exit code 0 on a clean baseline, 1 on findings, 2 on resolver/runtime errors.

## Cross-cutting notes for any agent picking up a chunk

- **Read RFC-15 first**, then this plan's §Decisions, then your assigned chunk. The §Decisions block resolves the ambiguities the readiness review surfaced **and** captures the deviation from RFC-15's "tools nest inside capability.yaml" shape — do not re-derive these from the RFC alone.
- **Both repos are separate git checkouts.** `specify-cli` is the implementation home. `../specify` is touched only by chunks 7 (one cross-link in `rfc-13-extensibility.md`, plus new docs) and 8 (new `tools.yaml` sidecar + briefs + docs). Commit changes there as sibling commits; do not mix repo histories.
- **`specify-capability` is off-limits.** No chunk modifies `crates/capability/`, `schemas/capability.schema.json`, or `../specify/capabilities/capability.schema.json`. If a chunk feels like it needs to, stop and re-read this plan's §Crate boundary decision.
- **Wasmtime is a heavyweight dependency.** Cold `cargo build --release` will go from ~90s to ~3-5 minutes after chunk 0 lands. Set `cargo` profiles' `opt-level = 1` for `[profile.dev]` builds of the new crate if local build times become painful; the Wasmtime team itself recommends this.
- **No behavioural changes for projects without `tools:`.** Every chunk before 8 must keep `cargo test --workspace` green for the omnia / contracts / vectis fixtures and integration tests. Run the existing `tests/capability.rs`, `tests/e2e.rs`, and `tests/slice_merge.rs` suites after every chunk.
- **Vectis WASI migration is out of scope.** RFC-15 §Implementation Plan §5 says "where it fits the filesystem-only model"; most of `specify-vectis verify` runs `cargo`, `swift`, `gradle`, and `cargo deny`, none of which fit. Leave Vectis alone. A separate plan can revisit individual narrow Vectis helpers (e.g. version-pin extraction) if and when one fits.
- **Trace `<scope-segment>` collisions in the post-execution log.** Two declarers with different sources and identical `(scope, name)` pairs will collide in the cache. RFC-15 Open Question 4 covers signing / hashing; track real-world hits so the follow-up RFC has data.
- **Source RFC amendment is out of scope.** `rfc-15-wasm-plugins.md` still says tools live in `capability.yaml`. This plan implements the two-site layout instead. Log the discrepancy in the post-execution notes; a separate RFC-15 amendment can land the wording fix later.

## Notes (post-execution log)

_(Empty — append per-chunk completion notes here, mirroring the convention in `fold-vectis-into-specify.md`.)_
