# RFC-15 WASI Capability Tools

> Status: Implemented - Depends: [RFC-13](rfc-13-extensibility.md) - Resolves: [RFC-13 Open Questions #4](rfc-13-extensibility.md#open-questions)

## Abstract

[RFC-13](rfc-13-extensibility.md) keeps non-core deterministic behavior out of the `specify` binary. Specify skills need deterministic code for tasks such as contract validation and bounded Vectis verification helpers.

This RFC provides a framework for extending Specify with deterministic helper code in a standard manner. Capabilities declare WASI command modules in `capability.yaml`; `specify` resolves, caches, and runs them through an embedded Wasmtime host.

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
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/1.0.0/contract.wasm"
    sha256: "<hex-encoded sha256 of the component bytes>"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
        - "$PROJECT_DIR/.specify/specs"
      write: []
```

The first landing supports only:

- WASI Preview 2 command components built for `wasm32-wasip2`;
- absolute local paths, `file:` URIs, and `https:` URIs addressed by `source`;
- optional `sha256` pins over the component bytes;
- filesystem permissions expressed as directory preopens.

The runtime is fixed by the first implementation rather than configured per tool. `source` is a literal absolute local path, `file:` URI, or `https:` URI for the WASM component; relative paths, source variables, and other URI schemes are not supported. `version` remains explicit for display, validation, and cache metadata, but the resolver does not parse version information out of `source`. `sha256` is optional for local development, but first-party release declarations should pin it from the first landing so cached bytes can be verified before execution. Missing `tools:` means no behavior change.

RFC-15 reopens the RFC-13 closed manifest field set only to add optional tools:. The post-RFC manifest remains closed: unknown top-level fields are still rejected, and tools: is the only new top-level property. Capability references and capabilities/capability.schema.json must be updated in the same implementation change.

### Tool Shape

The first version uses the standard WASI CLI command world. A tool receives normal CLI arguments, stdin, stdout, and stderr, then exits with a process status code.

```bash
cargo build --target wasm32-wasip2 --release
```

This keeps tool authoring simple and is enough to replace the immediate host binaries and scripts. Custom WIT worlds can follow later when validators need typed diagnostics or structured imports and exports.

Command-world tools emit diagnostics through stdout / stderr and exit status in the first landing. That is intentionally a pilot shape, not the final validator ABI. Once a validator needs structured findings that skills should parse without scraping text, the tool should move to a custom WIT world with typed exports for diagnostics and a small command-world wrapper only for manual invocation.

### Resolver and Cache

When a resolved capability declares tools, `specify` ensures each requested module exists within the capability's cache:

```text
~/.cache/specify/<capability-identifier>/<tools>/<name>-<version>.wasm
```

The first landing treats cached source contents as immutable once the live manifest tuple matches the cache metadata. If bytes at a URI or local path change without the `source`, `version`, or `sha256` string changing, `specify` may continue using the cached copy until the operator removes it with `specify tool gc` or changes the manifest. When `sha256` is present, the resolver verifies fetched or copied bytes before installing them and rejects cache entries whose recorded digest disagrees with the live manifest. Signature formats can follow later.

For each tool, the resolver:

1. Validates that `source` is an absolute local path, `file:` URI, or `https:` URI.
2. Reuses a cached module when its metadata still matches the manifest, including `sha256` when present.
3. Otherwise copies the local file or downloads the URI, verifies `sha256` when declared, stages it in a temp directory, and atomically moves it into the cache.
4. Records the capability identifier, source, version, digest, and permissions snapshot.

The cache is global because WASI modules are portable across supported hosts. Capability briefs and references remain in the project-local capability cache.

### Execution Host

Skills and briefs invoke helpers through `specify`, not through cache paths:

```bash
specify tool run contract -- validate "$PROJECT_DIR/contracts"
```

On `run`, the host:

1. Resolves the current project's capability.
2. Resolves and loads the named tool.
3. Expands `$PROJECT_DIR` and `$CAPABILITY_DIR`.
4. Canonicalizes the project root, capability root, and expanded permission paths.
5. Rejects permission paths whose resolved targets escape those roots.
6. Sets the module working directory to the project root.
7. Instantiates the module with WASI.
8. Preopens declared read and write directories.
9. Wires args, stdio, and a minimal environment.
10. Returns the module exit code and typed resolver or runtime errors.

The minimal environment exposes only `PROJECT_DIR` and `CAPABILITY_DIR` in the first landing. No ambient host environment is inherited. Permission directories must already exist before they are preopened; a tool that needs to create nested paths should request a preopen for an existing parent directory. Symlinks are resolved during canonicalization, so a symlink that points outside `$PROJECT_DIR` or `$CAPABILITY_DIR` is denied even if its textual path starts inside an allowed root.

The host remains capability-agnostic. It knows modules, permissions, paths, and process IO. It does not know what contracts, Vectis, Omnia, or a third-party capability does. Wasmtime is linked into the main `specify` binary for the first landing so users still install a single executable, but the implementation should keep a narrow runner boundary around instantiation and execution. That boundary lets a later release move execution out of process, swap Wasmtime configuration, or add additional declared runner types without changing manifest parsing, cache resolution, or CLI output semantics.

### Permissions

Permissions are mandatory for tools that touch the filesystem:

```yaml
permissions:
  read:
    - "$PROJECT_DIR/contracts"
  write:
    - "$PROJECT_DIR/crates"
```

Rules:

- `read` and `write` entries are directory preopens, not globs.
- Manifests should list both read and write intent clearly, even if the host must grant read for writable preopens.
- `$PROJECT_DIR` and `$CAPABILITY_DIR` are the only first-landing variables.
- Absolute paths outside those roots are rejected.
- The first landing does not expose WASI network access to tools. Resolver network access for URI sources is separate from tool runtime permissions.
- Write permissions must not grant tools authority over Specify lifecycle state such as `.specify/project.yaml`, slice `.metadata.yaml` files, plan locks, or archive moves. Tools may write declared capability-owned artifacts; lifecycle transitions and structural `.specify/` mutations continue to flow through core CLI verbs.

This is narrower than agent tool execution by design. Capability helpers should operate on declared artifact directories, not on the whole machine.

### Determinism Policy

Declared WASI tools are for deterministic helper behavior. The first host enforces that by withholding ambient host environment, using a fixed working directory, denying runtime network access, passing only explicit argv / stdio, and granting filesystem access only through declared preopens.

Tools should not depend on wall-clock time, host randomness, inherited `PATH`, current user identity, or undeclared files. If a helper genuinely needs time, randomness, network, or host toolchain behavior, it should stay in the skill / host-tooling layer until a separate RFC defines a declared host runner and its policy model. This RFC only standardizes the WASI helper path.

### CLI Surface

Add a small `specify tool` surface for declared capability tools:

```bash
specify tool run <name> -- [args...]   # fetch if needed, then run the named tool through Wasmtime
specify tool list                      # show declared tools and cache status
specify tool fetch [<name>]            # prefetch one or all tools
specify tool show <name>               # show metadata, permissions, and cache status
specify tool gc                        # remove unused cached versions
```

`fetch` and `gc` mutate only `~/.cache/specify/<capability-identifier>/<tools>/`. `run` mutates project state only through directories granted by the tool manifest.

The first version does not expose a path-printing shortcut. Cached modules are not user-invoked host executables, and exposing cache paths would invite bypassing the Wasmtime host.

`specify tool` is a capability-agnostic core surface. It amends RFC-13 by adding a generic declared-tool runner next to `specify capability`; it does not add capability-specific commands or let capabilities replace the fixed slice loop.

### Trust and Offline Behavior

Tool trust follows capability trust: the operator already trusts the capability manifest, and that manifest names the modules it needs. A `sha256` pin narrows the supply-chain gap by making the bytes stable across cache fills and mirrors. It does not replace capability trust and it is not a signature; signed provenance can follow once the basic declared-tool path is working.

Cached modules work offline. First use without network fails with a typed resolver error. Air-gapped users can pre-populate the cache with `specify tool fetch` on a connected machine; with no name, `fetch` resolves every declared tool for the current capability.

Wasmtime does not remove the need to trust a capability. It narrows the blast radius and makes the helper boundary reviewable.

## Implementation Plan

1. **Manifest support.** Add `tools:` to the capability schema and parsed type. `specify capability check` validates names, exact SemVer versions, absolute local paths, `file:` or `https:` URIs in `source`, optional `sha256` pins, and permission paths.
2. **Resolver.** Add local-path and URI source resolution, SHA-256 verification when declared, atomic module caching, cache metadata, cache reuse, and failure tests.
3. **Wasmtime host.** Add a CLI-layer host that builds a WASI context from manifest permissions, preopens allowed directories, wires stdio, passes args, and propagates exit status behind a narrow runner boundary.
4. **CLI integration.** Add `specify tool {run,list,fetch,show,gc}`. Add `$CAPABILITY_DIR` substitution for permission expansion.
5. **First-party modules.** Replace the provisional contract validator binary with `contract.wasm`. Move narrow Vectis helper behavior to WASI modules where it fits the filesystem-only model; leave host toolchain calls in Vectis skills when they need platform SDKs, language toolchains, or networked registries.
6. **Docs and lints.** Document capability WASI tools and add lints for overly broad write access and skills invoking undeclared helper binaries when a declared tool exists.

Acceptance coverage should include manifest validation, valid and invalid SHA-256 pins, cache hit and miss, local-path source resolution, URI source resolution, network failure, allowed and denied filesystem access, denied writes to lifecycle state, non-zero exit propagation, and a fixture capability that runs a synthetic tool.

## Migration

This is additive for capabilities without `tools:`.

First-party capability changes:


| Draft RFC-13 shape                     | RFC-15 shape                                           |
| -------------------------------------- | ------------------------------------------------------ |
| `specify-contract` binary              | `contract.wasm` declared in `capability.yaml`          |
| manually installed `specify-vectis`    | narrow Vectis WASI modules where filesystem-only works |
| bare `specify-vectis verify` in skills | `specify tool run vectis-verify -- [args...]`           |


No compatibility shim is needed because these helper binaries have not shipped as public surface.

If accepted, RFC-13 should be amended to say open-ended capability plugins remain rejected while declared WASI helper modules are the approved deterministic helper mechanism.

## Alternatives Considered

**Host binaries.** Rejected because they are unsandboxed, host-specific, and harder for third-party capabilities to distribute.

**Co-located scripts.** Rejected as the default because they push Deno, Python, or other runtime prerequisites onto the operator. Skills may still run ordinary scripts when they are not part of the capability-tool contract.

**Bundling all helpers with `specify`.** Rejected because it moves capability-specific behavior back into the main release.

**`cargo install` / `cargo binstall`.** Rejected because it assumes Rust tooling and exposes distribution details to skills.

**Native fallback entries in `tools:`.** Rejected for the first landing because mixed runtimes blur the security story. Native host tooling should stay explicit in skills until a separate RFC expands the model.

## Non-Goals

- General package management.
- Replacing or capability-configuring the `define → build → merge` loop.
- Replacing specialist skills with hidden plugin logic.
- Installing host binaries through `capability.yaml`.
- Tool dependency graphs.
- WASI network access for tools in the first landing.
- Perfect air-gapped UX in the first landing.
- A general sandbox for all agent actions.

## Open Questions

1. **WASI target expansion.** The first landing standardizes on `wasm32-wasip2`; should a later release add `wasm32-wasip1` for older toolchains?
2. **Structured diagnostics.** When should `runtime: wasm-component` and a WIT interface become mandatory for validators?
3. **Permission UX.** Should operators see a one-time prompt when a newly resolved capability tool requests write access, or is capability trust enough?
4. **Signing and provenance.** SHA-256 pins cover byte integrity in v1. What signature, attestation, or provenance format should follow?
5. **Supported URI schemes.** `oci:`, `s3:`, and enterprise mirrors are plausible later source schemes.
6. **Version requirements and source templates.** SemVer ranges and source interpolation can follow once literal sources are working.
7. **Cache location.** Provisional: use the global cache by default, with `SPECIFY_TOOLS_CACHE` for CI and hermetic use.
8. **Resolver concurrency.** Use a per-tool cache lock if concurrent resolves become an issue.

## References

- [RFC-13: Immutable core + capability extensions](rfc-13-extensibility.md) - owns the capability protocol and the open distribution question.
- [RFC-12: Refine RFC-8](rfc-12-refine-rfc-8.md) - owns the contract validation behavior that moves to a helper.
- [RFC-1: `specify` CLI](rfc-1-cli.md) - owns the CLI and capability resolver.
- [RFC-5: Framework Linter](../rfc-5-lint.md) - home for the follow-up lints.

