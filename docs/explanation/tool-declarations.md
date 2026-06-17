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

Target adapter authors declare tools inline in `adapter.yaml`'s `tools:` array:

```text
adapters/targets/contracts/
├── adapter.yaml      # carries `tools:` per target.schema.json
└── briefs/
```

```yaml
# adapters/targets/contracts/adapter.yaml (excerpt)
tools:
  - name: contract
    version: 0.3.0
```

First-party target entries name the wasm-pkg package via `{ name, version }`; the CLI rewrites them to `specify:<name>@<version>` and applies embedded permission defaults for first-party tools. At runtime, `specify extension run` resolves plugin-scope tools from a `tools.yaml` sidecar next to `adapter.yaml` (via `load::plugin_sidecar()`). For published adapters the sidecar is generated during fetch; for local development, `make use-local-dev` (after a gitignored `Specify.local.toml` `cli = { path = … }` overlay) builds adapter WASI tools from that checkout and writes a sidecar with a `source:` pointing at the locally-built WASM binary. CLI install is delegated to `scripts/specify.rs --install`. The sidecar is gitignored and never checked in.

Use adapter scope when the helper is part of the adapter's promised behavior, such as a merge validator or a deterministic artifact checker.

## Precedence

`specify extension` resolves the current project, loads both declaration sites, and merges by `name`.

Project scope wins on collision. This lets an operator redirect a adapter-shipped tool to a local build or a pinned internal mirror without editing the adapter. The CLI emits a `tool-name-collision` warning and keeps going.

Within a single declaration site, tool names must be unique.

## Variables and permissions

Permission entries may use:

- `$PROJECT_DIR` in both project-scope and adapter-scope declarations.
- `$CAPABILITY_DIR` only in adapter-scope declarations.

`$CAPABILITY_DIR` is rejected in project-scope tools because project declarations must remain valid even for workspace projects or projects whose adapter changes later.

Variables are expanded only in `permissions.read` and `permissions.write`. They are not expanded in `source`, and they are not expanded in arguments passed after `--`.

Permissions are directory preopens, not globs. The host canonicalizes every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. A tool that writes files should ask for the narrowest existing parent directory it needs. Use `$PROJECT_DIR` only when the tool's contract must create or update root-level files such as `Cargo.toml`.

First-party scalar package declarations do not repeat permissions in YAML. `specify` embeds the current defaults for `specify:contract` and `specify:vectis`; project-local object declarations still carry explicit permissions.

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

First-party package sources use `specify:<tool>@<semver>` and resolve through wasm-pkg registry metadata at `augentic.io` to OCI artifacts in GHCR. Operators still run only `specify extension fetch` and `specify extension run`; they do not install `wkg`.

`sha256` pins object-declared component bytes. When present, the resolver verifies bytes before installation and rejects a cache entry whose sidecar digest no longer matches the live declaration.

Package-backed first-party declarations do not carry a separate `sha256`; the package resolver validates package content through the registry client and records package/OCI metadata in `meta.yaml`. For local object declarations, changing a tool's bytes should also change either `version`, `source`, or `sha256`; otherwise existing caches may continue to use the earlier bytes until garbage collection removes them.

The `oci.reference` written into `meta.yaml` is derived best-effort from the resolved registry's well-known wasm-pkg metadata (`oci.registry`, `oci.namespacePrefix`). When a registry advertises no OCI backend or the metadata cannot be fetched, the field is omitted rather than synthesised, so the sidecar stays truthful for any registry — not only `augentic.io`.

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
- The project needs a temporary or permanent override of a adapter tool.
- The project is a workspace and has no adapter.
- The tool should remain available after changing adapters.

Choose adapter scope when:

- The tool is part of the adapter's documented behavior.
- Briefs or skills in the adapter call `specify extension run <name>`.
- The adapter author owns updates, digest pins, and distribution.
- `$CAPABILITY_DIR` is needed for read-only templates or bundled resources.

## Examples

Project-scope override of a adapter tool:

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

Adapter-scope tool with a bundled read-only template directory:

```yaml
# adapters/targets/example/adapter.yaml (excerpt)
tools:
  - name: example-generate
    version: 1.2.0
    source: "https://example.com/specify/example-generate-1.2.0.wasm"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    permissions:
      read:
        - "$CAPABILITY_DIR/templates"
        - "$PROJECT_DIR/specs"
      write:
        - "$PROJECT_DIR/generated"
```

First-party adapter-scope package that must create root-level project files:

```yaml
# adapters/targets/vectis/adapter.yaml (tools[])
tools:
  - name: vectis
    version: "0.3.0"
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
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) -- adapter sidecar conventions
