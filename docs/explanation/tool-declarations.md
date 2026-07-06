# Tool Declarations

> [!NOTE]
> **Who needs this page.** Tool declarations are an advanced authoring topic for *project authors* who ship a deterministic lint helper alongside their rules. If you are running changes, you can skip it. Adapter-owned helpers (the contract validator and the Vectis tools) are **not** declared tools anymore — they are in-guest library code inside each adapter's committed `guest.wasm` (Omnia-migration cutover).

Specify tools are WASI components a **project** declares for deterministic lint helper work. ([WASI](https://wasi.dev/) — the WebAssembly System Interface — lets a sandboxed WebAssembly module run with tightly scoped filesystem access and no network.) The `specify` binary resolves, caches, and runs them with explicit permissions when a `kind: tool` lint hint names them during `specify lint project`.

## Declaration site

Project authors declare project-local tools in `.specify/project.yaml`:

```yaml
name: payments-service
target: https://github.com/augentic/specify/adapters/targets/contracts

tools:
  - name: my-checker
    version: 1.0.0
    source: "file:///Users/alex/tools/my-checker-dev.wasm"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Project-scope declarations are owned by the project author. They are available even when the project is a workspace project with no adapter, and they survive adapter changes. The former adapter-scope `extension:` declaration retired with the `extension run` verb family — adapters ship helper behaviour inside their guests instead.

Within the declaration site, tool names must be unique.

## Variables and permissions

Permission entries may use `$PROJECT_DIR`. Variables are expanded only in `permissions.read` and `permissions.write`; they are not expanded in `source`.

Permissions are directory preopens, not globs. The host canonicalizes every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. A tool that writes files should ask for the narrowest existing parent directory it needs. Use `$PROJECT_DIR` only when the tool's contract must create or update root-level files.

## Cache segmentation

The global cache is segmented by declaration scope:

```text
<cache-root>/
└── project--payments-service/
    └── my-checker/1.0.0/
        ├── module.wasm
        └── meta.yaml
```

The cache root resolution order is `SPECIFY_TOOLS_CACHE`, then `XDG_CACHE_HOME`, then the platform cache directory, then `$HOME/.cache/specify/tools`.

## Package Sources and SHA-256 Pins

A package-backed project-scope `source:` resolves through wasm-pkg registry metadata (e.g. `augentic.io`) to OCI artifacts in GHCR; operators do not install `wkg`.

`sha256` pins object-declared component bytes. When present, the resolver verifies bytes before installation and rejects a cache entry whose cached digest no longer matches the live declaration.

Package-backed declarations validate package content through the registry client and record package/OCI metadata in `meta.yaml`. For local object declarations, changing a tool's bytes should also change either `version`, `source`, or `sha256`; otherwise existing caches may continue to use the earlier bytes.

The `oci.reference` written into `meta.yaml` is derived best-effort from the resolved registry's well-known wasm-pkg metadata (`oci.registry`, `oci.namespacePrefix`). When a registry advertises no OCI backend or the metadata cannot be fetched, the field is omitted rather than synthesised, so the metadata stays truthful for any registry — not only `augentic.io`.

## Registry configuration

Tool fetches resolve registries through wasm-pkg with a layered config (last write wins per key):

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

The file is checked in. Operators edit it to point tool fetches at an internal mirror, register private namespaces, or override the default registry. The shape is intentionally compatible with `wkg --config .specify/wasm-pkg.toml ...` so maintainers can publish and pull packages with the same config the runtime honours.

Re-running `init` never overwrites an operator-edited file; deleting it falls back to the embedded `specify -> augentic.io` default so existing projects and workspace clones keep working without re-init.

## Lints

The framework linter reserves rule ids for this surface:

- `tool.write-permission-too-broad` may warn on broad writes, including `$PROJECT_DIR`, when a future framework linter has enough context to distinguish root-file scaffolding from unnecessarily broad authority.
- `tool.lifecycle-state-write-denied` rejects writes to Specify lifecycle state.

Tool declaration structure is validated when `specify lint project` assembles its declared-tool inventory.

## See also

- [Declared WASI tools](../reference/cli/extension.md) -- the surviving tool surface behind `specify lint project`
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) -- adapter helpers as in-guest library code
