# Tool Declarations

> [!NOTE]
> **Who needs this page.** Tool declarations are an advanced authoring topic for *adapter and project authors* who ship a deterministic helper alongside their briefs. If you are running changes, you can skip it — the first-party tools (the contract validator and the Vectis tools) are already declared for you. Read on only when you are packaging your own helper.

Specify tools are WASI components that a project or adapter declares for deterministic helper work. ([WASI](https://wasi.dev/) — the WebAssembly System Interface — lets a sandboxed WebAssembly module run with tightly scoped filesystem access and no network.) The `specify` binary resolves, caches, and runs them with explicit permissions through `specify extension`.

## Declaration sites

Tools may be declared in two places.

### Project scope

Project authors declare project-local tools in `.specify/project.yaml`:

```yaml
name: payments-service
target: https://github.com/augentic/specify/adapters/targets/contracts

tools:
  - name: contract
    version: 1.0.0
    source: "file:///Users/alex/tools/contract-dev.wasm"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Project-scope declarations are owned by the project author. They are available even when the project is a workspace project with no adapter, and they survive adapter changes.

Use project scope when a repo needs a local override, a development build, a private helper, or a tool that is not part of the adapter contract.

### Adapter scope

An adapter declares **at most one** WASI extension inline in `adapter.yaml`'s singular `extension:` object:

```text
adapters/targets/contracts/
├── adapter.yaml      # carries the `extension:` object per target.schema.json
├── adapter.wasm      # the committed, bundled extension binary
├── briefs/
└── extension/        # co-located Rust crate the wasm builds from (source-only)
```

```yaml
# adapters/targets/contracts/adapter.yaml (excerpt)
extension:
  name: contract           # optional run handle; defaults to the adapter name
  permissions:
    read:
      - "$PROJECT_DIR/contracts"
    write: []
```

At runtime `specify extension run <name>` resolves the extension directly from the installed adapter tree — there is no `tools.yaml` sidecar, no `load::plugin_sidecar()` reader, and no separate fetch step. The extension's bytes are a committed `adapter.wasm` at the adapter root, built from the co-located `extension/` Rust crate by `specify adapter build` and bundled into the published adapter artifact (it rides the adapter's own semver identity and content digest, so it carries no per-extension `version`, `source`, or `sha256`). For local development, override the bundled extension with a project-scope `tools[]` entry whose `source:` points at a locally built wasm.

Use adapter scope when the helper is part of the adapter's promised behavior, such as a merge validator or a deterministic artifact checker.

## Precedence

`specify extension` resolves the current project, loads both declaration sites, and merges by `name`.

Project scope wins on collision. This lets an operator redirect an adapter-shipped extension to a local build or a pinned internal mirror without editing the adapter. The CLI emits a `tool-name-collision` warning and keeps going.

Within a single declaration site, tool names must be unique.

## Variables and permissions

Permission entries may use:

- `$PROJECT_DIR` in both project-scope and adapter-scope declarations.
- `$CAPABILITY_DIR` only in adapter-scope declarations.

`$CAPABILITY_DIR` is rejected in project-scope tools because project declarations must remain valid even for workspace projects or projects whose adapter changes later.

Variables are expanded only in `permissions.read` and `permissions.write`. They are not expanded in `source`, and they are not expanded in arguments passed after `--`.

Permissions are directory preopens, not globs. The host canonicalizes every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. A tool that writes files should ask for the narrowest existing parent directory it needs. Use `$PROJECT_DIR` only when the tool's contract must create or update root-level files such as `Cargo.toml`.

Both sites carry explicit permissions: a project-scope `tools[]` entry declares them per entry, and an adapter declares them inline on its `extension:` object. There are no embedded first-party permission defaults — the retired scalar `specify:<tool>@<version>` declaration is gone.

## Cache segmentation

The global cache is segmented by declaration scope:

```text
<cache-root>/
├── project--payments-service/
│   └── contract/1.0.0/
│       ├── module.wasm
│       └── meta.yaml
└── adapter--contracts/
    └── contract/1.0.0/
        ├── module.wasm
        └── meta.yaml
```

Project and adapter entries stay isolated even when the name, version, and source are identical. This keeps ownership explicit and prevents one declarer from silently changing another declarer's cached bytes.

The cache root follows the `specify extension` reference order: `SPECIFY_TOOLS_CACHE`, then `XDG_CACHE_HOME`, then the platform cache directory, then `$HOME/.cache/specify/tools`.

## Package Sources and SHA-256 Pins

A package-backed project-scope `source:` resolves through wasm-pkg registry metadata (e.g. `augentic.io`) to OCI artifacts in GHCR. Operators still run only `specify extension fetch` and `specify extension run`; they do not install `wkg`. Adapter extensions never take this path — an adapter ships its extension as a committed `adapter.wasm` bundled into the published adapter artifact, covered by the adapter's own content digest (RFC-48 D3), so there is no per-extension package source and no separate `sha256`.

`sha256` pins object-declared (project-scope) component bytes. When present, the resolver verifies bytes before installation and rejects a cache entry whose cached digest no longer matches the live declaration.

Package-backed declarations validate package content through the registry client and record package/OCI metadata in `meta.yaml`. For local object declarations, changing a tool's bytes should also change either `version`, `source`, or `sha256`; otherwise existing caches may continue to use the earlier bytes until garbage collection removes them.

The `oci.reference` written into `meta.yaml` is derived best-effort from the resolved registry's well-known wasm-pkg metadata (`oci.registry`, `oci.namespacePrefix`). When a registry advertises no OCI backend or the metadata cannot be fetched, the field is omitted rather than synthesised, so the metadata stays truthful for any registry — not only `augentic.io`.

## Registry configuration

`specify extension fetch` and `specify extension run` resolve registries through wasm-pkg with a layered config (last write wins per key):

1. The wasm-pkg global defaults (`~/.config/wasm-pkg/config.toml`).
2. The project-local `.specify/wasm-pkg.toml`, when present.
3. The `WKG_CONFIG` override, when the env var is set.
4. An embedded `specify -> augentic.io` namespace fallback, applied only when no earlier layer mapped the `specify` namespace.

`specify init` (regular and workspace modes) scaffolds `.specify/wasm-pkg.toml` with the canonical contents:

```toml
default_registry = "augentic.io"

[namespace_registries]
specify = "augentic.io"
```

The file is checked in. Operators edit it to point first-party tool fetches at an internal mirror, register private namespaces, or override the default registry. The shape is intentionally compatible with `wkg --config .specify/wasm-pkg.toml ...` so maintainers can publish and pull packages with the same config the runtime honours.

Re-running `init` never overwrites an operator-edited file; deleting it falls back to the embedded `specify -> augentic.io` default so existing projects and workspace clones keep working without re-init.

## Choosing scope

Choose project scope when:

- The tool is repo-private.
- The project needs a temporary or permanent override of an adapter extension.
- The project is a workspace and has no adapter.
- The tool should remain available after changing adapters.

Choose adapter scope when:

- The tool is part of the adapter's documented behavior.
- Briefs or skills in the adapter call `specify extension run <name>`.
- The adapter author owns updates and distribution.
- `$CAPABILITY_DIR` is needed for read-only templates or bundled resources.

## Examples

Project-scope override of an adapter extension:

```yaml
# .specify/project.yaml
name: payments-service
target: https://github.com/augentic/specify/adapters/targets/contracts
tools:
  - name: contract
    version: 1.0.1-dev
    source: "/Users/alex/dev/specify-cli/crates/contract-validate/dist/contract.wasm"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Adapter-scope extension with a bundled read-only template directory — no `version`, `source`, or `sha256` appears (those are rejected); the bytes are the committed `adapter.wasm` built from the co-located `extension/` crate:

```yaml
# adapters/targets/example/adapter.yaml (excerpt)
extension:
  name: example-generate     # optional run handle; defaults to the adapter name
  permissions:
    read:
      - "$CAPABILITY_DIR/templates"
      - "$PROJECT_DIR/specs"
    write:
      - "$PROJECT_DIR/generated"
```

First-party adapter extension that must create root-level project files — the `$PROJECT_DIR` write preopen enables root-level scaffolding, and permissions are declared explicitly (there are no embedded first-party defaults):

```yaml
# adapters/targets/vectis/adapter.yaml (extension)
extension:
  # name omitted — the run handle defaults to the adapter name (`vectis`)
  permissions:
    read:
      - "$PROJECT_DIR"
      - "$CAPABILITY_DIR"
    write:
      - "$PROJECT_DIR"
```

Invocation:

```bash
specify extension fetch contract
specify extension run contract -- "$PROJECT_DIR/contracts" --format json
```

## Future lints

The framework linter reserves rule ids for this surface:

- `tool.write-permission-too-broad` may warn on broad writes, including `$PROJECT_DIR`, when a future framework linter has enough context to distinguish root-file scaffolding from unnecessarily broad authority.
- `tool.lifecycle-state-write-denied` rejects writes to Specify lifecycle state.
- `skill.invokes-host-binary-with-declared-tool-equivalent` will warn when a brief or skill shells out to a host helper after an equivalent declared tool exists.

The current CLI already validates tool declaration structure during `specify extension` commands. The framework checks also scan active first-party briefs and skills for retired helper invocations covered by the [Declared Tool Helper Inventory](../reference/declared-tool-helper-inventory.md).

## See also

- [specify extension](../reference/cli/extension.md) -- command reference
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) -- the adapter `extension` declaration
