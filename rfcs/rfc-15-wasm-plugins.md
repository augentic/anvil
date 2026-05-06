# RFC-15 WASM Capability Tools

> Status: Draft - Depends: [RFC-13](rfc-13-extensibility.md) - Resolves: [RFC-13 Open Questions #4](rfc-13-extensibility.md#open-questions)

## Abstract

[RFC-13](rfc-13-extensibility.md) keeps capability-specific deterministic behavior out of the `specify` binary. Capability skills still need helper code for tasks such as contract validation, Vectis scaffolding, and Vectis verification.

This RFC gives those helpers one portable distribution path. Capabilities declare WASI modules in `capability.yaml`; `specify` resolves, verifies, caches, and runs them through an embedded Wasmtime host.

Users still install one binary: `specify`.

## Motivation

Today there is no standard way to extend `specify` deterministic behavior without adding code to the main binary or asking users to install companion tools. That creates three problems:

- first-party helpers bloat the core CLI;
- third-party capabilities have no portable helper format;
- skills must mention host binaries, language runtimes, or install steps.

WASI modules solve the immediate problem without making the whole Specify workflow dynamically pluggable. The core CLI remains capability-agnostic. It only knows how to fetch and run declared tools inside a constrained host.

## Design

### Capability Manifest

`capability.yaml` gains an optional `tools:` array:

```yaml
tools:
  - name: contract
    version: ^1.0
    runtime: wasm-wasi
    source: github-release
    repo: augentic/specify-tools
    asset: "contract-{version}.wasm"
    sha256: "<64 hex chars>"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
        - "$PROJECT_DIR/.specify/specs"
      write: []
      network: false
```

The first landing supports only:

- `runtime: wasm32-wasi`;
- `source: github-release`;
- SemVer version requirements;
- SHA256-pinned release assets;
- filesystem permissions expressed as directory preopens;
- `network: false`.

Missing `tools:` means no behavior change.

### Tool Shape

The first version uses WASI command modules, not WIT components. A tool receives normal CLI arguments, stdin, stdout, and stderr, then exits with a process status code.

```bash
cargo build --target wasm32-wasip2 --release
```

This keeps tool authoring simple and is enough to replace the immediate host binaries and scripts. `runtime: wasm-component` can follow later when validators need typed diagnostics or structured imports and exports.

### Resolver and Cache

When a resolved capability declares tools, `specify` ensures each requested module exists in a global cache:

```text
~/.cache/specify/tools/<tool-name>/<version>/wasm/<tool-name>.wasm
```

For each tool, the resolver:

1. Resolves the highest release matching `version`.
2. Reuses a cached module when its metadata still matches the manifest.
3. Otherwise downloads the release asset, verifies SHA256 when present, stages it in a temp directory, and atomically moves it into the cache.
4. Records the capability identifier, source URL, resolved version, runtime, hash, and permissions snapshot.

The cache is global because WASI modules are portable across supported hosts. Capability briefs and references remain in the project-local capability cache.

### Execution Host

Skills and briefs invoke helpers through `specify`, not through cache paths:

```bash
specify ext run contract -- validate "$PROJECT_DIR/contracts"
```

On `run`, the host:

1. Resolves the current project's capability.
2. Resolves and verifies the named tool.
3. Expands `$PROJECT_DIR` and `$CAPABILITY_DIR`.
4. Rejects permission paths outside those roots.
5. Instantiates the module with WASI.
6. Preopens declared read and write directories.
7. Wires args, stdio, and a minimal environment.
8. Returns the module exit code and typed resolver or runtime errors.

The host remains capability-agnostic. It knows modules, permissions, paths, and process IO. It does not know what contracts, Vectis, Omnia, or a third-party capability does.

### Permissions

Permissions are mandatory for tools that touch the filesystem:

```yaml
permissions:
  read:
    - "$PROJECT_DIR/contracts"
  write:
    - "$PROJECT_DIR/crates"
  network: false
```

Rules:

- `read` and `write` entries are directory preopens, not globs.
- Manifests should list both read and write intent clearly, even if the host must grant read for writable preopens.
- `$PROJECT_DIR` and `$CAPABILITY_DIR` are the only first-landing variables.
- Absolute paths outside those roots are rejected.
- `network: true` is rejected until a later RFC defines a concrete Wasmtime network model and review posture.

This is narrower than agent tool execution by design. Capability helpers should operate on declared artifact directories, not on the whole machine.

### CLI Surface

Add a `tool` subresource under `specify capability`:

```bash
specify ext run <name> -- [args...] # fetch if needed, then run through Wasmtime
specify ext list                    # show declared tools and cache status
specify ext fetch [<name>]          # prefetch one or all tools
specify ext show <name>             # show metadata, permissions, and cache path
specify ext gc                      # remove unused cached versions
```

`fetch` and `gc` mutate only `~/.cache/specify/tools/`. `run` mutates project state only through directories granted by the tool manifest.

The first version does not expose a path-printing shortcut. Cached modules are not user-invoked host executables, and exposing cache paths would invite bypassing the Wasmtime host.

### Trust and Offline Behavior

Tool trust follows capability trust: the operator already trusts the capability manifest, and that manifest names the modules it needs. SHA256 pins should warn in the first landing and become hard errors in the next minor release.

Cached modules work offline. First use without network fails with a typed resolver error. Air-gapped users can pre-populate the cache with `specify capability tool fetch --all` on a connected machine.

Wasmtime does not remove the need to trust a capability. It narrows the blast radius and makes the helper boundary reviewable.

## Implementation Plan

1. **Manifest support.** Add `tools:` to the capability schema and parsed type. `specify capability check` validates names, SemVer requirements, runtime, source, asset templates, SHA256 values, and permission paths.
2. **Resolver.** Add GitHub release resolution, SHA256 verification, atomic module caching, cache metadata, cache reuse, and failure tests.
3. **Wasmtime host.** Add a CLI-layer host that builds a WASI context from manifest permissions, preopens allowed directories, wires stdio, passes args, and propagates exit status.
4. **CLI integration.** Add `specify capability tool {run,list,fetch,show,gc}`. Add `$CAPABILITY_DIR` substitution for permission expansion.
5. **First-party modules.** Replace the provisional contract validator binary with `contract.wasm`. Move Vectis helper behavior to WASI modules where it fits the filesystem-only model; leave host toolchain calls in Vectis skills when it does not.
6. **Docs and lints.** Document capability WASM tools and add lints for missing SHA256 pins, overly broad write access, and skills invoking undeclared helper binaries when a declared tool exists.

Acceptance coverage should include manifest validation, cache hit and miss, SHA256 mismatch, unsupported runtime or source, network failure, allowed and denied filesystem access, non-zero exit propagation, and a fixture capability that runs a synthetic tool.

## Migration

This is additive for capabilities without `tools:`.

First-party capability changes:


| Draft RFC-13 shape                     | RFC-15 shape                                           |
| -------------------------------------- | ------------------------------------------------------ |
| `specify-contract` binary              | `contract.wasm` declared in `capability.yaml`          |
| manually installed `specify-vectis`    | `specify-vectis.wasm` or narrower Vectis WASI modules  |
| bare `specify-vectis verify` in skills | `specify capability tool run specify-vectis -- verify` |


No compatibility shim is needed because these helper binaries have not shipped as public surface.

If accepted, RFC-13 should be amended to say open-ended capability plugins remain rejected while declared WASI helper modules are the approved deterministic helper mechanism.

## Alternatives Considered

**Host binaries.** Rejected because they are unsandboxed, host-specific, and harder for third-party capabilities to distribute.

**Co-located scripts.** Rejected as the default because they push Deno, Python, or other runtime prerequisites onto the operator. Skills may still run ordinary scripts when they are not part of the capability-tool contract.

**Bundling all helpers with `specify`.** Rejected because it moves capability-specific behavior back into the main release.

`**cargo install` / `cargo binstall`.** Rejected because it assumes Rust tooling and exposes distribution details to skills.

**WASM components from the first landing.** Deferred. Components are likely right for structured diagnostics, but CLI-style WASI command modules are enough for the immediate helper problem.

**Native fallback entries in `tools:`.** Rejected for the first landing because mixed runtimes blur the security story. Native host tooling should stay explicit in skills until a separate RFC expands the model.

## Non-Goals

- General package management.
- Replacing or capability-configuring the `define → build → merge` loop.
- Replacing specialist skills with hidden plugin logic.
- Installing host binaries through `capability.yaml`.
- Tool dependency graphs.
- Network-enabled WASM helpers in the first landing.
- Perfect air-gapped UX in the first landing.
- A general sandbox for all agent actions.

## Open Questions

1. **Wasmtime location.** Should Wasmtime live in the main `specify` binary, or behind an optional crate feature for smaller installs?
2. **WASI target.** Should the first landing standardize on `wasm32-wasip2`, or also support `wasm32-wasip1`?
3. **Structured diagnostics.** When should `runtime: wasm-component` and a WIT interface become mandatory for validators?
4. **Permission UX.** Should operators see a one-time prompt when a newly resolved capability tool requests write access, or is capability trust enough?
5. **Signing.** SHA256 pins are enough for the first landing; signatures can follow if third-party modules become common.
6. **More sources.** `oci`, `s3`, and enterprise mirrors are plausible later `source:` values.
7. **Version pins.** Provisional: allow SemVer requirements, with exact pins through `=1.2.3`.
8. **Cache location.** Provisional: use the global cache by default, with `SPECIFY_TOOLS_CACHE` for CI and hermetic use.
9. **Resolver concurrency.** Use a per-tool cache lock if concurrent resolves become an issue.

## References

- [RFC-13: Immutable core + capability extensions](rfc-13-extensibility.md) - owns the capability protocol and the open distribution question.
- [RFC-13 implementation plan](rfc-13-plan.md) - defines the provisional contract and Vectis helper binaries this RFC revises.
- [RFC-15: Capability Helper Installation](rfc-15-capability-tools.md) - the host-script / host-binary alternative to this RFC.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) - owns the contract validation behavior that moves to a helper.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) - owns the CLI and capability resolver.
- [RFC-5: Framework Linter](rfc-5-lint.md) - home for the follow-up lints.

