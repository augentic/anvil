# RFC-15 WASM Plugins

> Status: Draft - Depends: [RFC-13](rfc-13-extensibility.md) - Resolves: [RFC-13 Open Questions #4](rfc-13-extensibility.md#open-questions)

## Abstract

[RFC-13](rfc-13-extensibility.md) moves capability-specific deterministic behavior out of the `specify` binary and into capability-owned skills. Those skills still need helper code: contracts need SemVer / `info.x-specify-id` checks, and Vectis needs scaffolding and verification logic.

This RFC keeps the user experience to one installed binary: `specify`. Capability helpers are declared in `capability.yaml` as WebAssembly modules and executed by `specify` using Wasmtime.

First-party impact: contract validation becomes a small WASI module; Vectis helper behavior moves behind a WASI command module or a WASM component. Capability skills invoke helpers through `specify capability tool run`, not through host-installed binaries or language runtimes.

## Motivation

Today users install one CLI and skills delegate deterministic work to it. The draft RFC-13 Phase 4 plan would add separate helper binaries such as `specify-contract-validate` and `specify-vectis`. The host-tool RFC-15 reduces manual installation by making `specify` fetch those helpers, but execution still happens as unsandboxed host binaries or scripts with external runtime prerequisites.

The goal is stronger:

- keep concern-specific behavior out of `specify` core;
- avoid bundling every first-party capability helper into the main install;
- avoid asking users to manually install N helper binaries or runtimes;
- give third-party capabilities a portable helper format from the first landing;
- make the extension trust boundary explicit through Wasmtime, WASI, and declared permissions;
- reuse runtime knowledge already present in Augentic's Rust WASM work.

Specify already targets Rust WASM in Omnia, and Wasmtime is not an unfamiliar dependency for the team. That changes the tradeoff from "new plugin runtime" to "one consistent helper runtime for deterministic capability code".

## Design

### Principle

Capability helpers are resolved through the capability system and executed through a single embedded runtime. Skills should not say "install this helper first"; they should ask `specify` to run a declared helper.

The core still does not switch on capability names. It owns a capability-tool host that can fetch, verify, cache, and execute declared WASM modules. Capability-specific behavior remains in capability-owned modules, briefs, skills, and references.

This intentionally revises RFC-13's rejection of WASM-component plugins. The rejection is still correct for replacing the skill layer or making the phase loop dynamically pluggable. It is too broad for deterministic helper execution, where a constrained Wasmtime host is a better fit than arbitrary host binaries.

### Declared WASM Tools

Helpers are declared in `capability.yaml`:

```yaml
name: contracts
version: 2
description: Contract authoring and validation workflow

pipeline:
  merge:
    - id: verify
      brief: briefs/merge.md

tools:
  - name: contract-validate
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

Vectis can use the same mechanism:

```yaml
tools:
  - name: specify-vectis
    version: ^1.0
    runtime: wasm-wasi
    source: github-release
    repo: augentic/specify-tools
    asset: "specify-vectis-{version}.wasm"
    sha256: "<64 hex chars>"
    permissions:
      read:
        - "$PROJECT_DIR"
      write:
        - "$PROJECT_DIR"
      network: false
```

The first landing supports `runtime: wasm-wasi` command modules. `runtime: wasm-component` can follow once the host needs structured imports / exports instead of CLI-style execution.

### Module Shape

The first version uses WASI command modules:

- `main(argv) -> exit code`;
- stdin, stdout, and stderr are wired to the calling process;
- filesystem access is limited to declared preopened directories;
- environment variables are explicit and minimal;
- network is unavailable unless a future runtime permission enables it.

This keeps the authoring target simple. A helper can be written in Rust and built with:

```bash
cargo build --target wasm32-wasip2 --release
```

The module receives normal CLI arguments:

```bash
specify capability tool run contract-validate -- validate "$PROJECT_DIR/contracts"
```

Future component-model helpers may expose a typed WIT interface:

```wit
package specify:tool;

interface diagnostics {
  record diagnostic {
    severity: string,
    path: option<string>,
    message: string,
  }
}

world capability-tool {
  export run: func(args: list<string>) -> result<list<diagnostics.diagnostic>, string>;
}
```

The first landing should not require WIT. CLI-style WASI is enough to replace host helper binaries and scripts.

### Resolver Behavior

When a capability with `tools:` is resolved, `specify` ensures each declared module is present in a global cache:

```text
~/.cache/specify/tools/<tool-name>/<version>/wasm/<tool-name>.wasm
```

For each tool, the resolver:

1. Resolves the highest release matching `version`.
2. Reuses a cached module when its metadata matches the manifest.
3. Otherwise downloads the release asset, verifies SHA256 when provided, stages it in a temp directory, and atomically moves it into the cache.
4. Records cache metadata: capability identifier, source URL, resolved version, runtime, hash, and permissions snapshot.

The cache is global because WASI modules are portable across supported host targets. Capability briefs remain in the project-local capability cache.

### Execution Host

`specify` embeds Wasmtime and provides a small host:

1. Resolve the current project's capability.
2. Resolve and verify the named tool.
3. Expand permission variables such as `$PROJECT_DIR` and `$CAPABILITY_DIR`.
4. Reject paths outside the current project or resolved capability cache unless explicitly allowed by a future global permission.
5. Instantiate the module with WASI.
6. Preopen read and write directories according to the manifest.
7. Pass args, stdin, stdout, stderr, and a minimal environment.
8. Return the module exit code and surface typed resolver / runtime errors.

The execution host is capability-agnostic. It knows about modules, permissions, paths, and process IO. It does not know what contracts, Vectis, Omnia, or any third-party capability does.

### Permissions

Permissions are mandatory for tools that read or write the filesystem:

```yaml
permissions:
  read:
    - "$PROJECT_DIR/contracts"
  write:
    - "$PROJECT_DIR/crates"
  network: false
```

Rules:

- `read` and `write` entries are directory preopens, not arbitrary path globs.
- `write` implies read for the same preopen only if the host implementation requires it; the manifest should still list intent clearly.
- `$PROJECT_DIR` and `$CAPABILITY_DIR` are the first supported variables.
- Absolute paths outside those roots are rejected in the first landing.
- `network: true` is rejected in the first landing. Network can become a future permission only with a concrete Wasmtime network story and review posture.

This is intentionally narrower than agent tool execution. Capability helpers should operate on declared artefact directories, not on the whole machine.

### Skill Invocation

Skills invoke declared helpers through a stable command:

```bash
specify capability tool run contract-validate -- validate "$PROJECT_DIR/contracts"
```

Briefs may also use a substitution:

```bash
$TOOL[contract-validate] validate "$PROJECT_DIR/contracts"
```

In this alternative, `$TOOL[...]` expands to a `specify capability tool run ... --` command fragment, not a host executable path. The cache layout and Wasmtime invocation details are not part of the skill contract.

If shell quoting makes command-fragment substitution too fragile, the first landing should omit `$TOOL[...]` and require explicit `specify capability tool run` calls in briefs.

### CLI Surface

Add a `tool` subresource under `specify capability`:

```bash
specify capability tool run <name> -- [args...] # fetch if needed, then execute through Wasmtime
specify capability tool list                    # show declared tools and cache status
specify capability tool fetch [<name>]          # prefetch one or all modules
specify capability tool show <name>             # show metadata, permissions, and cache path
specify capability tool gc                      # remove unused cached versions
```

`fetch` and `gc` touch only `~/.cache/specify/tools/`; they do not mutate project state. `run` mutates project state only through directories granted by the tool manifest.

There is intentionally no path-printing shorthand in the first version. WASM modules are not user-invoked host executables, and exposing cache paths invites bypassing the Wasmtime host.

### Trust and Offline Behavior

Tool trust follows capability trust: the operator already trusts the capability manifest, and the manifest names the modules it needs. SHA256 pins should be warnings in the first landing and hard errors in the next minor release.

Cached modules work offline. First use without network fails with a typed resolver error. Air-gapped users can pre-populate the cache with `specify capability tool fetch --all` on a connected machine.

Wasmtime does not remove the need to trust the capability. It narrows the blast radius of helper execution and makes the declared trust boundary reviewable.

## Manifest Delta

`capabilities/capability.schema.json` gains an optional `tools:` array. The first schema only needs the `github-release` + `wasm-wasi` shape:

```yaml
tools:
  - name: <tool-name>
    version: <semver-requirement>
    runtime: wasm-wasi
    source: github-release
    repo: <owner>/<repo>
    asset: <asset-template-with-{version}>
    sha256: <64-hex-digest>
    permissions:
      read:
        - <permission-path>
      write:
        - <permission-path>
      network: false
```

Missing `tools:` means no new behavior.

The first schema should not support host-specific assets. If a helper cannot fit WASI, it should remain skill-owned host tooling outside the capability-tool contract until a separate RFC expands the model.

## Implementation Scope

### Phase 1: Manifest Support

Add `tools:` to the capability schema and parsed capability type. `specify capability check` validates tool names, SemVer requirements, supported runtime values, supported source values, asset templates, SHA256 values, and permission paths.

Acceptance: manifests without `tools:` behave exactly as before; contracts and Vectis fixtures with `runtime: wasm-wasi` validate; invalid absolute permission paths are rejected.

### Phase 2: Resolver

Add `crates/capability/src/tools/` with GitHub release resolution, SHA256 verification, atomic module caching, cache metadata, and cache reuse.

Acceptance: tests cover cache hit, cache miss, SHA256 mismatch, unsupported source, invalid runtime, and network failure.

### Phase 3: Wasmtime Host

Add a Wasmtime-backed execution host in the CLI layer. The host builds a WASI context from manifest permissions, preopens allowed directories, wires stdio, passes args, and returns module exit status.

Acceptance: tests run a fixture WASI module that reads an allowed file, fails to read a denied file, writes to an allowed directory, fails to write to a denied directory, and propagates non-zero exit status.

### Phase 4: CLI and Brief Integration

Add `specify capability tool {run,list,fetch,show,gc}` behavior under the existing capability command family. Add `$CAPABILITY_DIR` substitution and either `$TOOL[<name>]` command-fragment substitution or a linted convention that briefs call `specify capability tool run` directly.

Acceptance: a fixture capability resolves a synthetic module and renders a brief that runs it.

### Phase 5: First-Party Modules

Replace the RFC-13 Phase 4.2a contract-validator binary with a WASI module distributed as a declared capability tool.

Replace the RFC-13 Phase 4.3a `specify-vectis` host binary plan with a WASI module if Vectis operations fit the first host's filesystem-only permission model. If not, split Vectis into narrower WASI modules for validation / template rendering and leave host toolchain calls in the Vectis skills until a later permission model exists.

Acceptance: contracts and Vectis workflows run with no user-visible install beyond `specify`; any remaining Vectis host-tool use is explicitly called out as skill-owned tooling, not a capability tool.

### Phase 6: Docs and Lints

Document capability WASM tools and update prerequisites to say helper runtimes are provided by `specify`. Add RFC-5 follow-up lints for:

- declared tools missing SHA256 pins;
- tools requesting broad project-root write access when a narrower path is available;
- skills invoking undeclared helper binaries when a declared tool exists;
- briefs using `$TOOL[...]` if command-fragment substitution is not adopted.

## Migration

This is additive for capabilities without `tools:`.

For first-party capabilities:


| Draft RFC-13 shape                     | Wasmtime RFC-15 shape                                  |
| -------------------------------------- | ------------------------------------------------------ |
| `specify-contract-validate` binary     | `contract-validate.wasm` declared in `capability.yaml` |
| manually installed `specify-vectis`    | `specify-vectis.wasm` or narrower Vectis WASI modules  |
| bare `specify-vectis verify` in skills | `specify capability tool run specify-vectis -- verify` |


No compatibility shim is needed because these helper binaries have not shipped as a public surface yet.

If this alternative is accepted, RFC-13's "WASM-component plugins" alternative should be amended: open-ended capability plugins remain rejected, while declared WASI helper modules become the approved deterministic helper mechanism.

## Alternatives Considered

**Host binary tools.** Rejected because they keep helper execution unsandboxed, require host-target release assets, and weaken the portability story for third-party capabilities.

**Co-located scripts.** Rejected as the default because they move runtime prerequisites such as Deno or Python onto the operator. Scripts remain available to skills as ordinary host commands, but they are not the capability-tool mechanism.

**Bundle all helpers with `specify`.** Rejected because it reintroduces capability-specific behavior into the main release and cannot scale to third-party capabilities.

**Use `cargo install` / `cargo binstall` directly.** Rejected because it assumes Rust tooling and pushes distribution details into skills.

**Use WASM components from the first landing.** Deferred because the component model is the better long-term interface for structured diagnostics, but CLI-style WASI command modules are enough to replace the immediate helper binaries with less design surface.

**Allow native tools as a fallback in `tools:`.** Rejected for the first landing because mixed runtimes blur the security story. If a helper truly needs native host access, a skill can still invoke host tooling explicitly.

## Non-Goals

- General package management.
- Replacing or capability-configuring the `define → build → merge` loop.
- Replacing specialist skills with hidden plugin logic.
- Host binary installation through `capability.yaml`.
- Tool dependency graphs.
- Network-enabled WASM helpers in the first landing.
- Perfect air-gapped UX in the first landing.
- A general sandboxed write-fence for all agent actions.

## Open Questions

1. **Wasmtime location.** Should Wasmtime live in the main `specify` binary, or behind an optional crate feature for smaller install size?
2. **WASI target.** Should the first landing standardize on `wasm32-wasip2`, or support `wasm32-wasip1` for broader toolchain compatibility?
3. **Structured diagnostics.** When should `runtime: wasm-component` and a WIT interface become mandatory for validators?
4. **Permission UX.** Should operators see a one-time prompt when a newly resolved capability tool requests write access, or is capability trust enough?
5. **Signing.** SHA256 pins are enough for the first landing; signatures can follow if third-party modules become common.
6. **More sources.** `oci`, `s3`, and enterprise mirrors are plausible later `source:` values.
7. **Exact versions vs. SemVer ranges.** Provisional: allow SemVer requirements, with exact pins available through `=1.2.3`.
8. **Project-local tool cache.** Provisional: global cache by default; allow `SPECIFY_TOOLS_CACHE` for CI and hermetic use.
9. **Resolver concurrency.** Use a per-tool cache lock if concurrent resolves become an issue.

## References

- [RFC-13: Immutable core + capability extensions](rfc-13-extensibility.md) - owns the capability protocol and the open distribution question.
- [RFC-13 implementation plan](rfc-13-plan.md) - defines the provisional contract and Vectis helper binaries this RFC revises.
- [RFC-15: Capability Helper Installation](rfc-15-capability-tools.md) - the host-script / host-binary alternative to this RFC.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) - owns the contract validation behavior that moves to a helper.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) - owns the CLI and capability resolver.
- [RFC-5: Framework Linter](rfc-5-lint.md) - home for the follow-up lints.

