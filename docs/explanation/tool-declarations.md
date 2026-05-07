# Tool Declarations

Specify tools are WASI components that a project or capability declares for deterministic helper work. The `specify` binary resolves, caches, and runs them with explicit permissions through `specify tool`.

## Declaration sites

Tools may be declared in two places.

### Project scope

Project authors declare project-local tools in `.specify/project.yaml`:

```yaml
name: payments-service
capability: https://github.com/augentic/specify/capabilities/contracts

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

Project-scope declarations are owned by the project author. They are available even when the project is a hub project with no capability, and they survive capability changes.

Use project scope when a repo needs a local override, a development build, a private helper, or a tool that is not part of the capability contract.

### Capability scope

Capability authors may ship a `tools.yaml` sidecar next to `capability.yaml`:

```text
capabilities/contracts/
├── capability.yaml
├── tools.yaml
└── briefs/
```

```yaml
# capabilities/contracts/tools.yaml
tools:
  - name: contract
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/1.0.0/contract.wasm"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

The sidecar has the same top-level shape as the project declaration: a `tools:` array. `capability.yaml` itself remains closed and does not gain a `tools:` field.

Use capability scope when the helper is part of the capability's promised behavior, such as a merge validator or a deterministic artifact checker.

## Precedence

`specify tool` resolves the current project, loads both declaration sites, and merges by `name`.

Project scope wins on collision. This lets an operator redirect a capability-shipped tool to a local build or a pinned internal mirror without editing the capability. The CLI emits a `tool-name-collision` warning and keeps going.

Within a single declaration site, tool names must be unique.

## Variables and permissions

Permission entries may use:

- `$PROJECT_DIR` in both project-scope and capability-scope declarations.
- `$CAPABILITY_DIR` only in capability-scope declarations.

`$CAPABILITY_DIR` is rejected in project-scope tools because project declarations must remain valid even for hub projects or projects whose capability changes later.

Variables are expanded only in `permissions.read` and `permissions.write`. They are not expanded in `source`, and they are not expanded in arguments passed after `--`.

Permissions are directory preopens, not globs. The host canonicalizes every path and rejects `..` segments, glob metacharacters, symlink escapes, and writes to Specify lifecycle state. A tool that writes files should ask for the narrowest existing parent directory it needs, not for `$PROJECT_DIR`.

## Cache segmentation

The global cache is segmented by declaration scope:

```text
<cache-root>/
├── project--payments-service/
│   └── contract/1.0.0/
│       ├── module.wasm
│       └── meta.yaml
└── capability--contracts/
    └── contract/1.0.0/
        ├── module.wasm
        └── meta.yaml
```

Project and capability entries stay isolated even when the name, version, and source are identical. This keeps ownership explicit and prevents one declarer from silently changing another declarer's cached bytes.

The cache root follows the `specify tool` reference order: `SPECIFY_TOOLS_CACHE`, then `XDG_CACHE_HOME`, then the platform cache directory, then `$HOME/.cache/specify/tools`.

## SHA-256 pins

`sha256` pins the component bytes. When present, the resolver verifies bytes before installation and rejects a cache entry whose sidecar digest no longer matches the live declaration.

Use `sha256` for released artifacts. First-party release declarations must include it. Omitting `sha256` is acceptable for local development, but it means cache reuse relies on the `(scope, name, version, source)` tuple alone.

Changing a tool's bytes should also change either `version`, `source`, or `sha256`; otherwise existing caches may continue to use the earlier bytes until garbage collection removes them.

## Choosing scope

Choose project scope when:

- The tool is repo-private.
- The project needs a temporary or permanent override of a capability tool.
- The project is a hub and has no capability.
- The tool should remain available after changing capabilities.

Choose capability scope when:

- The tool is part of the capability's documented behavior.
- Briefs or skills in the capability call `specify tool run <name>`.
- The capability author owns updates, digest pins, and distribution.
- `$CAPABILITY_DIR` is needed for read-only templates or bundled resources.

## Examples

Project-scope override of a capability tool:

```yaml
# .specify/project.yaml
name: payments-service
capability: https://github.com/augentic/specify/capabilities/contracts
tools:
  - name: contract
    version: 1.0.1-dev
    source: "/Users/alex/dev/specify-cli/crates/contract-validate/dist/contract.wasm"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Capability-scope tool with a bundled read-only template directory:

```yaml
# capabilities/example/tools.yaml
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

Invocation:

```bash
specify tool list
specify tool fetch contract
specify tool run contract -- "$PROJECT_DIR/contracts" --format json
```

## Future lints

RFC-5 owns the long-term framework linter. RFC-15 adds rule ids for this surface:

- `tool.write-permission-too-broad` flags tools that ask to write the whole project.
- `tool.lifecycle-state-write-denied` rejects writes to Specify lifecycle state.
- `skill.invokes-host-binary-with-declared-tool-equivalent` will warn when a brief or skill shells out to a host helper after an equivalent declared tool exists.

The current CLI already validates tool declaration structure during `specify tool` commands. The skill/brief invocation scan becomes meaningful after first-party helpers move to declared tools.

## See also

- [specify tool](../reference/cli/tool.md) -- command reference
- [Anatomy of a Capability](../contributing/capability-anatomy.md) -- capability sidecar conventions
- [RFC-15 WASI Capability Tools](../../rfcs/rfc-15-wasm-plugins.md) -- source RFC; the implementation uses the two declaration sites documented here
