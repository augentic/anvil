# Declared WASI tools

The `extension` verb family (`run` / `fetch` / `gc` / `schema`) retired with the old stack at the Omnia-migration cutover. Adapter-owned deterministic helpers — the `contract` and `vectis` validators — are **in-guest library code** compiled into each adapter's committed `guest.wasm`; the guest orchestrations invoke them directly, and nothing host-side dispatches an adapter WASI tool anymore.

## What survives

Project-scope `tools[]` declarations in `.specify/project.yaml` remain the one WASI surface: `specify lint project` resolves a `kind: tool` lint hint against the declared inventory, fetches the component into the tool cache, and runs it through the embedded Wasmtime host with the declared filesystem permissions.

### Declaration shape

```yaml
tools:
  - name: my-checker
    version: 0.1.0
    source: "https://example.com/my-checker.wasm"
    sha256: "…" # optional pin
```

### Cache locations

The cache root is selected in this order:

1. `SPECIFY_TOOLS_CACHE`
2. `$XDG_CACHE_HOME/specify/tools`
3. `$HOME/.cache/specify/tools`

Inside the root, entries live under the `project--<project-name>` scope segment, keyed by tool name and version, each with `module.wasm` and a `meta.yaml` sidecar.

### Digest verification

When a `sha256` is present, fetched or copied bytes are verified before installing into the cache. A cache hit is accepted only when the live declaration tuple matches the sidecar metadata: scope, name, version, source, and `sha256`. If the tuple changes, the resolver stages fresh bytes and installs them atomically.

### Security notes

Filesystem access is deny-by-default: a tool receives only the read and write preopens declared in its manifest, permission paths must already exist, and runtime network access is disabled (resolver network access for `https://` sources happens before execution). The host passes only `PROJECT_DIR`; it does not inherit `PATH`, credentials, shell variables, user identity, or ambient filesystem access. Write permissions must not target Specify lifecycle state.

### Determinism policy

Declared tools are for deterministic helper behavior. Tools must not depend on host environment variables, host binaries on `PATH`, runtime network access, wall-clock time, host randomness, current user identity, or undeclared files for correctness.

## See also

- [Tool declarations](../../explanation/tool-declarations.md) -- where tools are declared and how the lint dispatch works
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) -- adapter helpers as in-guest library code
