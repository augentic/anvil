# specify tool

Run declared WASI tools through the `specify` binary.

`specify tool` is the operator surface for deterministic helper code declared by a project or by its resolved adapter. Tools are WASI Preview 2 command components. The CLI resolves the declaration, fills the global cache, applies the declared filesystem permissions, and runs the component through the embedded Wasmtime host.

## Subcommands

### specify tool run

Fetch if needed, then run a declared tool.

```bash
specify tool run <name> -- [args...]
```

Arguments after `--` are forwarded verbatim to the WASI component. The tool name is supplied as `argv[0]`; forwarded arguments start at `argv[1]`.

Successful guest stdout and stderr are not wrapped by Specify. Command-world tools own their own diagnostics, so a validator can keep emitting the JSON or text shape its callers already understand. Resolver, permission, and runtime errors still use the standard Specify error envelope when `--format json` is selected.

### specify tool list

List the merged tool declarations for the current project.

```bash
specify tool list [--format json]
```

The list includes project-scope tools from `.specify/project.yaml` and adapter-scope tools from the resolved adapter's `tools.yaml` sidecar. When both sites declare the same name, the project-scope declaration wins and a `tool-name-collision` warning is emitted.

### specify tool fetch

Populate the cache for one tool, or every declared tool when the name is omitted.

```bash
specify tool fetch [<name>] [--format json]
```

`fetch` performs source resolution and digest verification, then stores the component bytes in the global tool cache. It does not instantiate or run the component.

### specify tool show

Show one declared tool's metadata, permissions, digest pin, and cache state.

```bash
specify tool show <name> [--format json]
```

`show` reports the live declaration plus cache metadata when the tool has already been fetched. The cache metadata includes the `fetched-at` timestamp from `meta.yaml` and, for package-backed tools, package and OCI metadata.

### specify tool gc

Remove cached versions that are no longer referenced by the current project's merged tool list.

```bash
specify tool gc [--all] [--format json]
```

The first implementation scans only scopes present in the current project: `project--<project-name>` and, when the project resolves a adapter, `adapter--<adapter-name>`. `--all` is accepted for the future broader mode; today it reports the same current-project scan.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The CLI operation succeeded, or a WASI tool exited `0`. |
| `1` | Project context is missing for a verb that requires `.specify/project.yaml`, or another generic CLI failure occurred. |
| `2` | Validation, resolver, permission, runtime, or undeclared-tool error. JSON errors use `validation`, `tool-resolver`, `tool-permission-denied`, `tool-runtime`, or `tool-not-declared`. |
| `N` (`1`-`255`) | `specify tool run` returns the guest's non-zero exit status when the component exits normally. |

Tool exit codes are clamped to `0..=255`. Runtime traps and resolver failures do not reuse guest status codes; they return a typed Specify error.

## JSON envelopes

Structured responses use the standard CLI envelope:

```json
{
  "envelope-version": 6
}
```

`list` returns:

```json
{
  "envelope-version": 6,
  "tools": [
    {
      "name": "contract",
      "version": "0.3.0",
      "source": "specify:contract@0.3.0",
      "scope": "adapter",
      "scope-detail": "contracts",
      "cache-status": "hit",
      "cached-path": "/Users/alex/.cache/specify/tools/adapter--contracts/contract/0.3.0/module.wasm"
    }
  ],
  "warnings": []
}
```

`fetch` uses the same row shape and adds `fetched: true|false`. `show` returns a single `tool` row with `permissions`, optional `sha256`, `fetched-at`, and optional `package` / `oci` blocks. `gc` returns `removed`, `all`, and `warnings`.

Validation failures include rule results so callers can branch on rule ids such as `tool.lifecycle-state-write-denied`:

```json
{
  "envelope-version": 6,
  "error": "validation",
  "message": "validation failed with 1 error(s)",
  "exit-code": 2,
  "results": [
    {
      "status": "fail",
      "rule-id": "tool.lifecycle-state-write-denied",
      "rule": "tool write permissions do not target Specify lifecycle state",
      "detail": "write path `$PROJECT_DIR/.specify` targets `.specify` lifecycle state"
    }
  ]
}
```

## Cache locations

The cache root is selected in this order:

1. `SPECIFY_TOOLS_CACHE`
2. `$XDG_CACHE_HOME/specify/tools`
3. `$HOME/.cache/specify/tools`

Inside the root, cache entries are segmented by declaration scope:

```text
<cache-root>/
└── <scope-segment>/
    └── <tool-name>/
        └── <version>/
            ├── module.wasm
            └── meta.yaml
```

Scope segments are `project--<project-name>` for tools declared in `.specify/project.yaml` and `adapter--<adapter-name>` for tools declared by a adapter sidecar. This keeps unrelated projects and adapters isolated even when they use the same tool name and version.

## Digest verification

`sha256` is optional for local development object declarations. First-party package declarations do not include a separate `sha256`; the package client validates package content and records package metadata in `meta.yaml`.

When a digest is present, Specify verifies fetched or copied bytes before installing them into the cache. A cache hit is accepted only when the live declaration tuple matches the sidecar metadata: scope, name, version, source, and `sha256`. If the tuple changes, the resolver stages fresh bytes and installs them atomically.

Without `sha256`, cache reuse is based on the manifest tuple. If bytes at a URL or local path change without changing `source` or `version`, the existing cache may continue to be used until the manifest changes or `specify tool gc` removes the cached version.

## Security notes

Operators still install one binary: `specify`. Cached modules are never executed directly as host binaries; `specify tool run` always goes through the Wasmtime host.

Filesystem access is deny-by-default. A tool receives only the read and write preopens declared in its manifest or embedded for first-party package declarations, and permission paths must already exist. Runtime network access is disabled for WASI tools; resolver network access for `https://` and wasm-pkg sources is separate and happens before execution.

The host passes only `PROJECT_DIR` and, for adapter-scope tools, `ADAPTER_DIR`. It does not inherit `PATH`, credentials, shell variables, user identity, current working directory authority, or ambient filesystem access.

Write permissions must not directly target Specify lifecycle state such as `.specify/project.yaml`, slice metadata, archive metadata, plan locks, or archive movement directories. `$PROJECT_DIR` write preopens are valid for tools that must create root-level files, but declarations should still prefer narrower existing parent directories when possible. Lifecycle transitions remain core CLI operations.

## Determinism policy

Declared tools are for deterministic helper behavior. Tools must not depend on host environment variables, host binaries on `PATH`, runtime network access, wall-clock time, host randomness, current user identity, or undeclared files for correctness.

A helper that genuinely needs host toolchains, network access, or platform SDKs belongs in a skill or a future declared host-runner model, not in the WASI runner.

## See also

- [Tool declarations](../../explanation/tool-declarations.md) -- where tools are declared and how precedence works
- [Anatomy of a Adapter](../../contributing/adapter-anatomy.md) -- optional adapter `tools.yaml` sidecar
