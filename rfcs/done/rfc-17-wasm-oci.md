# RFC-17: Wasm Package Distribution for WASI Tools

> Status: Implemented - Depends: [RFC-15](rfc-15-wasm-plugins.md), [RFC-16](rfc-16-wasi-vectis.md) - Enables: [RM-21](../roadmap.md#rm-21-capability-ecosystem-operating-model)

## Abstract

Specify's declared WASI tools should be distributed as `wasm-pkg-tools` packages in the Augentic `specify` namespace, resolved through Augentic registry metadata to OCI artifacts in GitHub Container Registry, rather than as raw `.wasm` files attached to GitHub Releases.

This RFC standardizes the build, publish, fetch, local-test, and capability declaration update workflow for first-party `specify-cli` WASI extensions. Maintainers publish built components with Bytecode Alliance `wasm-pkg-tools` (`wkg`), capability `tools.yaml` declarations use exact package references such as `specify:contract@0.3.0`, and `specify tool fetch` resolves those references into the existing local tool cache.

Native `specify` CLI binaries may continue to use GitHub Releases. This RFC only replaces GitHub Release assets for declared WASI tools.

## Motivation

RFC-15 intentionally decoupled deterministic capability helpers from the `specify` binary. That extension path is now real, but the distribution story is still too ad hoc:

- first-party capability manifests point at GitHub Release asset URLs;
- release packaging for WASI tools is hand-written beside the native binary matrix;
- local development has separate paths for `contract` and `vectis`;
- published capability declarations must wait for raw release assets to be uploaded before they can be updated;
- the release workflow can drift from the set of first-party tools declared in `capabilities/*/tools.yaml`;
- consumers fetch raw bytes from a forge release surface rather than a package registry built for immutable artifacts.

WebAssembly components are packages. `wasm-pkg-tools` already defines package-name syntax, registry configuration, well-known registry metadata, SemVer version selection, and OCI/Warg protocol resolution. OCI registries provide the backing immutable tags, digest addressing, authentication, retention policy, package permissions, and familiar supply-chain surface. Specify should use the package layer as the manifest contract, rather than exposing raw OCI repository paths as first-party capability declarations.

The goal is to make first-party WASI tools feel like a small package ecosystem:

1. Build every declared first-party component the same way.
2. Publish every release component as a `wasm-pkg-tools` package backed by `ghcr.io/augentic`.
3. Reference components from capability manifests with package request entries.
4. Fetch components through `specify tool fetch` without requiring operators to install a second CLI.
5. Keep local development and CI smoke tests close to the release path.

## Design

### Distribution Model

First-party WASI tools are published as `wasm-pkg-tools` packages in the `specify` namespace. The canonical package request shape is:

```text
specify:<tool-name>@<semver>
```

Examples:

```text
specify:contract@0.3.0
specify:vectis@0.3.0
```

If RFC-16 later splits Vectis into multiple components, the same convention applies:

```text
specify:vectis-validate@0.3.0
specify:vectis-scaffold@0.3.0
```

The scalar `tools:` entry is the full package request. It identifies both the package and the exact SemVer version. First-party package declarations do not carry a separate `version` field.

Augentic hosts wasm-pkg registry metadata at `augentic.io`, backed by OCI artifacts in GHCR. `specify` embeds a first-party namespace default equivalent to the following `wkg` configuration, and maintainers may use the same config for publishing and local smoke tests:

```toml
default_registry = "augentic.io"

[namespace_registries]
specify = "augentic.io"
```

After the `specify` namespace maps to `augentic.io`, `https://augentic.io/.well-known/wasm-pkg/registry.json` provides protocol and storage metadata:

```json
{
  "preferredProtocol": "oci",
  "oci": {
    "registry": "ghcr.io",
    "namespacePrefix": "augentic/"
  }
}
```

With that metadata, `specify:vectis@0.3.0` resolves to the OCI artifact `ghcr.io/augentic/specify/vectis:0.3.0`. The resolved OCI reference is implementation metadata; first-party capability declarations should not expose it.

Package entries MUST include exact SemVer versions without a leading `v`. Capability declarations MUST NOT use `latest`, branch names, mutable prerelease aliases, or version ranges. If maintainers need prerelease testing, they publish SemVer prerelease versions such as `0.4.0-alpha.1`.

The `specify-cli` release version and the first-party WASI tool versions remain aligned for first-party tools. A future RFC may allow independently versioned tool packages once package support policy exists.

The first implementation supports only first-party `specify:*` package entries. Third-party or private namespaces can follow once Specify has a policy for namespace trust, registry configuration, and capability authorship.

### Tool Manifest Sources

RFC-15's `tools:` array grows one new first-party entry shape: an exact wasm-pkg package request string.

```yaml
tools:
  - "specify:contract@0.3.0"
```

Rules:

- First-party package entries MUST be valid wasm-pkg package requests in the `specify` namespace.
- First-party package entries MUST include an exact SemVer version, for example `specify:contract@0.3.0`.
- The CLI-facing tool name is derived from the package segment after `:` and before `@`. For example, `specify:contract@0.3.0` declares the tool name `contract`.
- Duplicate derived tool names are invalid within one capability declaration.
- Runtime filesystem permissions remain governed by RFC-15 and are only required when a tool needs filesystem preopens. They are not part of the package-distribution contract introduced here.
- Local development should use package prerelease versions or a local wasm-pkg registry configuration.

Keeping each `tools:` entry as a string matches wasm-pkg package request syntax and keeps project-scope overrides simple. A structured object can follow later if Specify needs aliases, signatures, fallback mirrors, or richer registry policy.

### Build And Publish

The release workflow installs `wkg`, configures the `specify` namespace to resolve through `augentic.io`, builds each releasable component with ordinary Cargo commands, and publishes the resulting `.wasm` directly. Publishing still writes OCI artifacts to GHCR because Augentic registry metadata selects the OCI protocol and GHCR backing registry.

Illustrative maintainer-equivalent commands:

```bash
cargo build -p specify-contract --target wasm32-wasip2 --release
wkg publish specify:contract@0.3.0 ./target/wasm32-wasip2/release/specify-contract.wasm

cargo build -p specify-vectis --target wasm32-wasip2 --release
wkg publish specify:vectis@0.3.0 ./target/wasm32-wasip2/release/vectis.wasm
```

The exact `wkg` CLI spelling may follow the installed `wasm-pkg-tools` release, but the workflow should use the package publish path rather than hand-constructing OCI repository names. The workflow authenticates to GHCR using GitHub Actions package permissions. It should prefer Docker credential configuration because `wkg` can read Docker credentials for OCI registries:

```bash
echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin
```

The release workflow needs:

```yaml
permissions:
  contents: write
  packages: write
```

`contents: write` is still needed for native GitHub Releases if that path remains. WASI publishing needs `packages: write`.

### Release Workflow

The `specify-cli` release workflow changes from a special `wasi-tools` job that builds Vectis only to a generic `wasi-tools` job:

1. Install stable Rust with `wasm32-wasip2`.
2. Install `wkg`.
3. Configure `wkg` with `default_registry = "augentic.io"` and `specify = "augentic.io"` namespace mapping, or rely on Augentic well-known registry metadata when available.
4. Log in to `ghcr.io`.
5. For each first-party package entry, run `cargo build -p <crate> --target wasm32-wasip2 --release`.
6. Publish the built `./target/wasm32-wasip2/release/<tool>.wasm` with `wkg publish <package-request>`.
7. Pull each just-published artifact through package resolution into a scratch directory.
8. Verify the pulled component can be read as bytes and, when practical, validated as a component.

The release job that creates GitHub Releases no longer attaches raw `.wasm` files. The authoritative component distribution surface is the wasm-pkg package name backed by GHCR.

### Runtime Fetching

Operators should not need to install `wkg`. `specify tool fetch` and `specify tool run` remain the only runtime surfaces.

The resolver adds wasm-pkg package support behind the existing source-resolution boundary:

```bash
specify tool fetch contract
specify tool run contract -- "$PROJECT_DIR/contracts" --format json
```

On a package entry such as `specify:contract@0.3.0`, the resolver:

1. Parses and validates the wasm-pkg package name and exact version from the `tools:` entry.
2. Reuses the existing cache when the sidecar matches `scope`, `tool-name`, and `source`.
3. Resolves the package registry through built-in Augentic first-party defaults, standard wasm-pkg config, or `.well-known/wasm-pkg/registry.json`.
4. Pulls the component bytes through the selected registry protocol. The first-party path resolves to OCI in GHCR.
5. Stages `module.wasm` and `meta.yaml` together.
6. Atomically installs into the existing tool cache layout.

Implementation should use `wasm-pkg-tools` crates inside `specify-tool` so package parsing, registry metadata, config loading, and OCI resolution follow the same rules as `wkg`. It MUST NOT shell out to `wkg` at operator runtime because RFC-15 and RFC-16 preserve the single-installed-binary contract.

The cache sidecar gains optional package and OCI metadata:

```yaml
schema-version: 1
scope: capability--contracts
tool-name: contract
tool-version: 0.3.0
source: specify:contract@0.3.0
fetched-at: "2026-05-10T00:00:00Z"
package:
  name: specify:contract
  version: 0.3.0
  registry: augentic.io
oci:
  reference: "ghcr.io/augentic/specify/contract:0.3.0"
```

The `package` and `oci` blocks are informational in the first implementation. Cache validity continues to be governed by the live declaration tuple.

### Authentication

Public first-party tool pulls should work anonymously when GHCR package visibility allows it. Private or internal registries use standard wasm-pkg registry configuration and the selected protocol's credentials.

Resolver credential order:

1. Standard wasm-pkg config, including `WKG_CONFIG` when set.
2. Docker credential config for OCI registries, matching `wkg` behavior.
3. Anonymous registry access when no credentials are configured.
4. Future `SPECIFY_REGISTRY_AUTH_*` environment variables only if standard wasm-pkg and Docker config prove insufficient.

The first implementation SHOULD avoid adding new Specify-specific credential files. Registry auth is already a solved workstation and CI problem in the wasm-pkg and OCI tooling layers.

Publish authentication remains CI-owned and uses the GitHub Actions token or an explicit package-publish token.

### Local Development

For rapid local iteration, developers still build local artifacts with:

```bash
cargo build -p specify-contract --target wasm32-wasip2 --release
```

They can test those artifacts through a temporary package prerelease source:

```yaml
tools:
  - "specify:contract@0.3.0-dev.<run-id>"
```

For local package smoke tests, maintainers can publish an explicitly temporary prerelease version:

```bash
wkg publish specify:contract@0.3.0-dev.<run-id> ./target/wasm32-wasip2/release/specify-contract.wasm
wkg pull specify:contract@0.3.0-dev.<run-id> -o /tmp/contract.wasm
```

The exact `wkg` pull command may follow the installed `wasm-pkg-tools` release. Temporary prerelease versions MUST NOT appear in checked-in first-party `tools.yaml`.

The local test loop is therefore: build the component, publish it under a unique prerelease package request, update a local `tools.yaml` override to that request, and run `specify tool fetch` or `specify tool run` with an isolated cache.

For cache-isolated tests:

```bash
SPECIFY_TOOLS_CACHE="$(mktemp -d)" specify tool fetch contract
```

This keeps local rebuilds from fighting the global cache. When reusing the global cache, developers still need to change `source` or run `specify tool gc`, because cache semantics intentionally treat unchanged declaration tuples as immutable.

### Capability Declaration Updates

The plugin repository's first-party declarations move from GitHub Release URLs:

```yaml
source: "https://github.com/augentic/specify-cli/releases/download/v0.2.0/contract.wasm"
```

to wasm-pkg package references:

```yaml
tools:
  - "specify:contract@0.3.0"
```

The declaration update is a direct edit to the checked-in capability manifests and any first-party declaration checks in `scripts/checks`.

### Verification

Release verification must fail before publish completion when any of these are true:

- a first-party capability declares a package request that the release workflow does not publish;
- the release workflow publishes a first-party package request that no first-party capability declares, unless explicitly marked internal;
- a first-party package request is not exact SemVer;
- a published component cannot be pulled back through package resolution;
- a pulled component cannot be read back from the package resolver;
- `specify tool fetch` cannot fetch a fixture declaration that points at the just-published package.

The checks should run in CI and locally. They are the replacement for manual "download release asset and inspect it by hand" steps.

Resolver tests should mock registry and package-client responses by default so ordinary PR checks are deterministic and do not depend on GHCR availability, credentials, or rate limits. Mocked coverage should include accepted and rejected package request syntax, embedded `specify -> augentic.io` namespace resolution, cache hit and miss behavior, unavailable packages, malformed registry metadata, auth failures, and invalid component bytes. A narrow local OCI registry fixture with wasm-pkg metadata may run as an integration smoke when the environment supports it. Public GHCR package pulls belong in release verification, not normal PR CI.

## Implementation Plan

1. **Publish with `wkg`.** Update `.github/workflows/release.yaml` to install `wkg`, configure the `specify` namespace, authenticate to GHCR, build each first-party component with `cargo build -p <crate> --target wasm32-wasip2 --release`, publish each package request, and pull/verify each component after publish.
2. **Add wasm-pkg resolver support.** Extend `specify-tool` tools-entry parsing, validation, cache sidecars, resolver tests, and fetch/show/list output to understand package entries.
3. **Keep runtime single-binary.** Implement package resolution and pulls inside `specify-tool`; do not require operator-installed `wkg`.
4. **Update first-party declarations.** Change `capabilities/contracts/tools.yaml` and `capabilities/vectis/tools.yaml` to scalar `specify:*@<semver>` package entries.
5. **Add release drift checks.** Extend framework checks so first-party tool declarations and release workflow package requests stay aligned.
6. **Revise docs.** Update `specify-cli/docs/release.md`, `docs/explanation/tool-declarations.md`, `docs/reference/cli/tool.md`, and capability-specific docs to describe wasm-pkg distribution and local override workflows.
7. **Add end-to-end smoke coverage.** Use a public test package or a local OCI registry fixture with wasm-pkg metadata for resolver tests, and run a release-pipeline smoke after publishing to GHCR.

## Migration

For capability authors:

- Replace first-party `https://github.com/.../*.wasm` sources with scalar package entries such as `specify:contract@0.3.0`.
- Use package prerelease sources or a local wasm-pkg registry for local development.

For operators:

- Continue installing only `specify`.
- Continue invoking `specify tool fetch`, `specify tool run`, `specify tool show`, and `specify tool gc`.
- No `wkg` installation is required unless the operator is publishing or manually inspecting packages.

For maintainers:

- Use `cargo build -p <crate> --target wasm32-wasip2 --release` to build WASI components.
- Use `wkg publish` package-name flows for manual publish tests.
- Use `wkg` package fetch/pull flows to inspect published components.
- Do not upload raw first-party `.wasm` files to GitHub Releases as the canonical distribution surface.

For existing caches:

- No compatibility shim is required for cached GitHub Release sources.
- Once declarations switch to package entries, the source tuple changes and `specify tool fetch` installs package-backed cache entries.
- `specify tool gc` removes unused old entries in scopes visible to the current project.

## Alternatives Considered

**Keep GitHub Release assets.** Rejected because release assets are a forge artifact surface, not a component package registry. They work for raw downloads but provide poor package naming, weaker publish/fetch symmetry, and more manual release choreography.

**Require users to install `wkg`.** Rejected because RFC-15 and RFC-16 deliberately preserve one installed `specify` binary for operators. `wkg` is appropriate for maintainers and CI; runtime fetch belongs in `specify`.

**Use direct `oci://` references in `tools.yaml`.** Rejected for first-party declarations. Direct OCI references are explicit and easy to bootstrap, but they leak storage layout into the capability contract, bypass wasm-pkg's package identity and registry metadata, and make a future move to Warg or a different OCI namespace harder.

**Attach both GitHub Release assets and wasm-pkg packages.** Rejected as the steady state because dual canonical sources drift. GitHub Releases may link to the package or include an audit manifest, but the wasm-pkg package should be authoritative for WASI tools.

**Use `oras` instead of `wkg`.** Rejected for this RFC because `wasm-pkg-tools` is component-aware and aligned with Bytecode Alliance conventions. `oras` remains useful for debugging generic OCI artifacts but should not define Specify's component workflow.

## Non-Goals

- Replacing native `specify` binary distribution.
- Designing an independent capability marketplace.
- Adding Warg registry support.
- Adding mutable version ranges to `tools.yaml`.
- Adding runtime WASI network access.
- Adding native host runners to declared tools.
- Designing signed provenance, SLSA attestations, or implementation hooks for future signing.
- Changing tool permission semantics.

## Open Questions

1. **Independent versions.** How soon should first-party WASI tools version independently from the `specify-cli` release?
2. **OCI metadata.** Which annotations should be mandatory on published components: source repository, license, description, revision, and build timestamp?
3. **Bundled defaults.** How should `specify` version and override its embedded `specify -> augentic.io` namespace default if Augentic registry metadata changes?

## References

- [RFC-15: WASI Capability Tools](rfc-15-wasm-plugins.md)
- [RFC-16: Vectis WASI Tools](rfc-16-wasi-vectis.md)
- [Specify Roadmap RM-21](../roadmap.md#rm-21-capability-ecosystem-operating-model)
- [Bytecode Alliance wasm-pkg-tools](https://github.com/bytecodealliance/wasm-pkg-tools)
- [WebAssembly Component Model: Distributing and Fetching Components and WIT](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html)

