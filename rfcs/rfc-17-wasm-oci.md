# RFC-17: OCI Distribution for Wasm Components

> Status: Draft - Depends: [RFC-15](archive/rfc-15-wasm-plugins.md), [RFC-16](archive/rfc-16-wasi-vectis.md) - Enables: [RM-21](roadmap.md#rm-21-capability-ecosystem-operating-model)

## Abstract

Specify's declared WASI tools should be distributed as OCI artifacts in Augentic's GitHub Container Registry namespace, `ghcr.io/augentic`, rather than as raw `.wasm` files attached to GitHub Releases.

This RFC standardizes the build, publish, fetch, local-test, and manifest-update workflow for first-party `specify-cli` WASI extensions. Maintainers publish built components with Bytecode Alliance `wasm-pkg-tools` (`wkg`), capability `tools.yaml` declarations point at immutable OCI references, and `specify tool fetch` resolves those references into the existing local tool cache.

Native `specify` CLI binaries may continue to use GitHub Releases. This RFC only replaces GitHub Release assets for declared WASI tools.

## Motivation

RFC-15 intentionally decoupled deterministic capability helpers from the `specify` binary. That extension path is now real, but the distribution story is still too ad hoc:

- first-party capability manifests point at GitHub Release asset URLs;
- release packaging for WASI tools is hand-written beside the native binary matrix;
- local development has separate paths for `contract` and `vectis`;
- published capability declarations must wait for raw release asset checksums before they can be pinned;
- the release workflow can drift from the set of first-party tools declared in `capabilities/*/tools.yaml`;
- consumers fetch raw bytes from a forge release surface rather than a package registry built for immutable artifacts.

WebAssembly components are packages. OCI registries already provide immutable tags, digest addressing, authentication, retention policy, package permissions, and a familiar supply-chain surface. `wasm-pkg-tools` gives Specify a standard way to publish and pull components without inventing a bespoke artifact protocol.

The goal is to make first-party WASI tools feel like a small package ecosystem:

1. Build every declared first-party component the same way.
2. Publish every release component to `ghcr.io/augentic`.
3. Reference components from capability manifests with OCI sources.
4. Fetch components through `specify tool fetch` without requiring operators to install a second CLI.
5. Keep local development and CI smoke tests close to the release path.

## Design

### Distribution Model

First-party WASI tools are published as OCI artifacts under `ghcr.io/augentic`.

The canonical OCI reference shape is:

```text
ghcr.io/augentic/specify-cli/<tool-name>:<semver>
```

Examples:

```text
ghcr.io/augentic/specify-cli/contract:0.3.0
ghcr.io/augentic/specify-cli/vectis:0.3.0
```

If RFC-16 later splits Vectis into multiple components, the same convention applies:

```text
ghcr.io/augentic/specify-cli/vectis-validate:0.3.0
ghcr.io/augentic/specify-cli/vectis-scaffold:0.3.0
```

Tags MUST be exact SemVer versions without a leading `v`. Capability declarations MUST NOT use `latest`, branch names, or mutable prerelease aliases. If maintainers need prerelease testing, they publish SemVer prerelease tags such as `0.4.0-alpha.1`.

The `specify-cli` release version and the first-party WASI tool versions remain aligned for first-party tools. A future RFC may allow independently versioned tool packages once compatibility policy exists.

### Tool Manifest Sources

RFC-15's string `source` field grows one new source kind: `oci://`.

```yaml
tools:
  - name: contract
    version: 0.3.0
    source: "oci://ghcr.io/augentic/specify-cli/contract:0.3.0"
    sha256: "<component-byte-sha256>"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Rules:

- `oci://` sources are direct OCI artifact references. The resolver strips the scheme before calling the OCI client.
- The tag in `source` MUST match `version` exactly for first-party declarations.
- `sha256` keeps its RFC-15 meaning: a lowercase SHA-256 over the resolved component bytes, not the OCI manifest digest.
- First-party release declarations MUST include `sha256`.
- Local development may continue to use absolute paths or `file://` sources with `sha256` omitted.
- `https://` raw-byte sources remain supported for compatibility, but first-party capability declarations move to `oci://`.

Keeping `source` as a string avoids a manifest shape break and keeps project-scope overrides simple. A structured source object can follow later if Specify needs multiple registry protocols, signatures, or fallback mirrors.

### Publishing With `wkg`

The release workflow installs `wkg` and publishes each built component with `wkg oci push`.

Example maintainer-equivalent command:

```bash
wkg oci push ghcr.io/augentic/specify-cli/contract:0.3.0 target/wasm32-wasip2/release/specify-contract.wasm
```

For Vectis:

```bash
wkg oci push ghcr.io/augentic/specify-cli/vectis:0.3.0 target/wasm32-wasip2/release/vectis.wasm
```

The workflow authenticates to GHCR using GitHub Actions package permissions. It should prefer Docker credential configuration because `wkg` can read Docker credentials for OCI registries:

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

### Build And Dist Layout

`specify-cli` gains one shared dist command that builds every first-party WASI tool and writes a machine-readable manifest.

Target surface:

```bash
cargo make wasi-artifacts
```

Output:

```text
target/wasi-tools/release/
|-- manifest.json
|-- contract.wasm
|-- contract.wasm.sha256
|-- vectis.wasm
`-- vectis.wasm.sha256
```

`manifest.json` records enough information for both CI and humans:

```json
{
  "schema-version": 1,
  "version": "0.3.0",
  "tools": [
    {
      "name": "contract",
      "package": "specify-contract",
      "target": "wasm32-wasip2",
      "path": "target/wasi-tools/release/contract.wasm",
      "oci": "ghcr.io/augentic/specify-cli/contract:0.3.0",
      "sha256": "<component-byte-sha256>"
    },
    {
      "name": "vectis",
      "package": "specify-vectis",
      "target": "wasm32-wasip2",
      "path": "target/wasi-tools/release/vectis.wasm",
      "oci": "ghcr.io/augentic/specify-cli/vectis:0.3.0",
      "sha256": "<component-byte-sha256>"
    }
  ]
}
```

The workflow publishes from this manifest rather than duplicating one shell block per tool. Adding a new first-party WASI tool becomes a one-line manifest/config change plus tests, not a bespoke release-job edit.

### Release Workflow

The `specify-cli` release workflow changes from a special `wasi-tools` job that builds Vectis only to a generic `wasi-tools` job:

1. Install stable Rust with `wasm32-wasip2`.
2. Install `wkg`.
3. Build all first-party WASI artifacts with `cargo make wasi-artifacts`.
4. Verify every manifest entry has a file and a SHA-256.
5. Log in to `ghcr.io`.
6. Push every manifest entry with `wkg oci push`.
7. Pull each just-published artifact with `wkg oci pull` into a scratch directory.
8. Recompute SHA-256 and compare with `manifest.json`.
9. Upload `manifest.json` and checksum files as workflow artifacts for audit only.

The release job that creates GitHub Releases no longer attaches raw `.wasm` files. The GitHub Release may include the `manifest.json` for discoverability, but the authoritative component distribution location is GHCR.

### Runtime Fetching

Operators should not need to install `wkg`. `specify tool fetch` and `specify tool run` remain the only runtime surfaces.

The resolver adds OCI support behind the existing source-resolution boundary:

```bash
specify tool fetch contract
specify tool run contract -- "$PROJECT_DIR/contracts" --format json
```

On an `oci://` source, the resolver:

1. Parses and validates the OCI reference.
2. Reuses the existing cache when the sidecar matches `scope`, `tool-name`, `tool-version`, `source`, and `sha256`.
3. Otherwise pulls the component bytes from the OCI registry.
4. Verifies the component-byte `sha256` when present.
5. Stages `module.wasm` and `meta.yaml` together.
6. Atomically installs into the existing tool cache layout.

Implementation should use `wasm-pkg-tools` crates or an equivalent narrow OCI client inside `specify-tool`. It MUST NOT shell out to `wkg` at operator runtime because RFC-15 and RFC-16 preserve the single-installed-binary contract.

The cache sidecar gains optional OCI metadata:

```yaml
schema-version: 1
scope: capability--contracts
tool-name: contract
tool-version: 0.3.0
source: oci://ghcr.io/augentic/specify-cli/contract:0.3.0
fetched-at: "2026-05-10T00:00:00Z"
permissions-snapshot:
  read:
    - "$PROJECT_DIR/contracts"
  write: []
sha256: "<component-byte-sha256>"
oci:
  reference: "ghcr.io/augentic/specify-cli/contract:0.3.0"
  manifest-digest: "sha256:<oci-manifest-digest>"
```

The `oci` block is informational in the first implementation. Cache validity continues to be governed by the live declaration tuple and component-byte `sha256`.

### Authentication

Public first-party tool pulls should work anonymously when GHCR package visibility allows it. Private or internal registries use existing OCI credentials.

Resolver credential order:

1. Docker credential config, matching `wkg` behavior.
2. `WKG_CONFIG` / standard wasm-pkg config if the selected library supports it without extra user ceremony.
3. Future `SPECIFY_OCI_AUTH_*` environment variables only if Docker/wkg config proves insufficient.

The first implementation SHOULD avoid adding new Specify-specific credential files. OCI auth is already a solved workstation and CI problem.

Publish authentication remains CI-owned and uses the GitHub Actions token or an explicit package-publish token.

### Local Development

For rapid local iteration, developers keep using project-scope or local capability-scope overrides:

```bash
cargo make wasi-artifacts
```

Project-scope override example:

```yaml
tools:
  - name: contract
    version: 0.3.0-dev
    source: "file:///absolute/path/to/specify-cli/target/wasi-tools/release/contract.wasm"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

For local OCI smoke tests, maintainers can publish to an explicitly temporary tag:

```bash
wkg oci push ghcr.io/augentic/specify-cli/contract:0.3.0-dev.<run-id> target/wasi-tools/release/contract.wasm
wkg oci pull ghcr.io/augentic/specify-cli/contract:0.3.0-dev.<run-id> -o /tmp/contract.wasm
```

Temporary tags MUST NOT appear in checked-in first-party `tools.yaml`.

For cache-isolated tests:

```bash
SPECIFY_TOOLS_CACHE="$(mktemp -d)" specify tool fetch contract
```

This keeps local rebuilds from fighting the global cache. When reusing the global cache, developers still need to change `version`, `source`, or `sha256`, or run `specify tool gc`, because RFC-15 cache semantics intentionally treat unchanged declaration tuples as immutable.

### Capability Declaration Updates

The plugin repository's first-party declarations move from GitHub Release URLs:

```yaml
source: "https://github.com/augentic/specify-cli/releases/download/v0.2.0/contract.wasm"
```

to OCI references:

```yaml
source: "oci://ghcr.io/augentic/specify-cli/contract:0.3.0"
```

The declaration update should be generated from `target/wasi-tools/release/manifest.json` or by a small helper command rather than hand-edited. The helper updates:

- `version`;
- `source`;
- `sha256`;
- any first-party declaration checks in `scripts/checks`.

Target helper surface:

```bash
specify tool manifest update-first-party --manifest target/wasi-tools/release/manifest.json --repo ../specify
```

That command name is provisional. It may land as an `xtask` in `specify-cli` instead if keeping framework-repo mutation out of the runtime binary is cleaner.

### Verification

Release verification must fail before publish completion when any of these are true:

- a first-party capability declares a WASI tool that is absent from `manifest.json`;
- `manifest.json` contains a tool that no first-party capability declares, unless explicitly marked internal;
- a declaration's `version` does not match the OCI tag;
- a declaration's `sha256` does not match the built component bytes;
- a published component cannot be pulled back from GHCR;
- a pulled component's SHA-256 differs from `manifest.json`;
- `specify tool fetch` cannot fetch a fixture declaration that points at the just-published OCI artifact.

The checks should run in CI and locally. They are the replacement for manual "download release asset and recompute checksum" steps.

## Implementation Plan

1. **Define the first-party WASI tool manifest.** Add a checked-in list of releasable WASI components in `specify-cli`, including tool name, package, built filename, OCI repository suffix, and capability declaration target.
2. **Unify local artifact builds.** Replace `contract-wasm`, `vectis-wasm`, and `vectis-wasi-artifacts` drift with one `cargo make wasi-artifacts` path. Keep compatibility aliases temporarily if useful for maintainers.
3. **Publish with `wkg`.** Update `.github/workflows/release.yaml` to install `wkg`, authenticate to GHCR, push every manifest entry, and pull/verify each component after publish.
4. **Add `oci://` resolver support.** Extend `specify-tool` source parsing, validation, cache sidecars, resolver tests, and fetch/show/list output to understand OCI sources.
5. **Keep runtime single-binary.** Implement OCI pulls inside `specify-tool`; do not require operator-installed `wkg`.
6. **Update first-party declarations.** Change `capabilities/contracts/tools.yaml` and `capabilities/vectis/tools.yaml` to `oci://ghcr.io/augentic/specify-cli/...` sources with real SHA-256 pins.
7. **Add release drift checks.** Extend framework checks so first-party tool declarations must match the manifest-derived OCI source, version, permissions, and SHA-256 format.
8. **Revise docs.** Update `specify-cli/docs/release.md`, `docs/explanation/tool-declarations.md`, `docs/reference/cli/tool.md`, and capability-specific docs to describe OCI distribution and local override workflows.
9. **Add end-to-end smoke coverage.** Use a public test artifact or a local OCI registry fixture for resolver tests, and run a release-pipeline smoke after pushing to GHCR.

## Migration

For capability authors:

- Replace first-party `https://github.com/.../*.wasm` sources with `oci://ghcr.io/augentic/specify-cli/<tool>:<version>`.
- Keep `sha256` pins over component bytes.
- Continue using project-scope `file://` overrides for local development.

For operators:

- Continue installing only `specify`.
- Continue invoking `specify tool fetch`, `specify tool run`, `specify tool show`, and `specify tool gc`.
- No `wkg` installation is required unless the operator is publishing or manually inspecting packages.

For maintainers:

- Use `cargo make wasi-artifacts` to build all WASI components.
- Use `wkg oci push` for manual publish tests.
- Use `wkg oci pull` to inspect published components.
- Do not upload raw first-party `.wasm` files to GitHub Releases as the canonical distribution surface.

For existing caches:

- Existing cached GitHub Release sources stay valid until declarations move.
- Once a declaration switches to `oci://`, the source tuple changes and `specify tool fetch` installs a new cache entry.
- `specify tool gc` removes unused old entries in scopes visible to the current project.

## Alternatives Considered

**Keep GitHub Release assets.** Rejected because release assets are a forge artifact surface, not a component package registry. They work for raw downloads but provide poor package naming, weaker publish/fetch symmetry, and more manual checksum choreography.

**Require users to install `wkg`.** Rejected because RFC-15 and RFC-16 deliberately preserve one installed `specify` binary for operators. `wkg` is appropriate for maintainers and CI; runtime fetch belongs in `specify`.

**Use `wkg publish` package names in `tools.yaml`.** Deferred. Package names such as `specify:contract@0.3.0` are attractive, but they require registry configuration or well-known metadata. Direct OCI references are explicit, work immediately with GHCR, and avoid adding a configuration dependency to capability resolution.

**Make `sha256` the OCI manifest digest.** Rejected for the first implementation. RFC-15 already defines `sha256` as component-byte integrity. OCI manifest digests are useful metadata, but changing the meaning of `sha256` would confuse local path, `file://`, `https://`, and `oci://` sources.

**Attach both GitHub Release assets and OCI packages.** Rejected as the steady state because dual canonical sources drift. GitHub Releases may link to the OCI package or include an audit manifest, but GHCR should be authoritative for WASI tools.

**Use `oras` instead of `wkg`.** Rejected for this RFC because `wasm-pkg-tools` is component-aware and aligned with Bytecode Alliance conventions. `oras` remains useful for debugging generic OCI artifacts but should not define Specify's component workflow.

## Non-Goals

- Replacing native `specify` binary distribution.
- Designing an independent capability marketplace.
- Adding Warg registry support.
- Adding mutable version ranges to `tools.yaml`.
- Adding runtime WASI network access.
- Adding native host runners to declared tools.
- Designing signed provenance or SLSA attestations for the first implementation.
- Changing tool permission semantics.
- Requiring a public `.well-known/wasm-pkg/registry.json` endpoint for Augentic.

## Open Questions

1. **Registry path.** Is `ghcr.io/augentic/specify-cli/<tool>` the final namespace, or should first-party components live under `ghcr.io/augentic/specify-tools/<tool>` to decouple tool packages from the CLI repository name?
2. **Independent versions.** How soon should first-party WASI tools version independently from the `specify-cli` release?
3. **OCI metadata.** Which annotations should be mandatory on published components: source repository, license, description, revision, and build timestamp?
4. **Provenance.** Should the next step after byte SHA-256 be Sigstore signing, GitHub artifact attestations, Warg, or another verification model?
5. **Private capability registries.** Should third-party/private capabilities use the same `oci://` source syntax with their own registries, or should Specify define a package-name abstraction first?
6. **Local OCI registry tests.** Should CI run resolver tests against a local registry fixture, a public GHCR test package, or mocked registry responses?
7. **Well-known metadata.** Should Augentic eventually host `.well-known/wasm-pkg/registry.json` so declarations can use package names such as `specify:contract@0.3.0`?

## References

- [RFC-15: WASI Capability Tools](archive/rfc-15-wasm-plugins.md)
- [RFC-16: Vectis WASI Tools](archive/rfc-16-wasi-vectis.md)
- [Specify Roadmap RM-21](roadmap.md#rm-21-capability-ecosystem-operating-model)
- [Bytecode Alliance wasm-pkg-tools](https://github.com/bytecodealliance/wasm-pkg-tools)
- [WebAssembly Component Model: Distributing and Fetching Components and WIT](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html)
