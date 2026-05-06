# RFC-15: Capability Helper Installation

> Status: Draft · Depends: [RFC-13](rfc-13-extensibility.md) · Resolves: [RFC-13 §Open Questions #4](rfc-13-extensibility.md#open-questions)

## Abstract

[RFC-13](rfc-13-extensibility.md) moves capability-specific deterministic behavior out of the `specify` binary and into capability-owned skills. Those skills still need helper code: contracts need SemVer / `info.x-specify-id` checks, and Vectis needs scaffolding and verification logic.

RFC-15 keeps the user experience to one installed binary: `specify`. Capability helpers are either:

- **Co-located scripts** shipped beside capability briefs and run from the resolved capability cache.
- **Declared tools** listed in `capability.yaml`, fetched and cached by `specify` on first use.

First-party impact: contract validation becomes a co-located script; `specify-vectis` remains a Rust binary but is declared as a capability tool and auto-installed by `specify`.

## Motivation

Today users install one CLI and skills delegate deterministic work to it. The draft RFC-13 Phase 4 plan would add separate helper binaries such as `specify-contract-validate` and `specify-vectis`. That solves the core-boundary problem, but creates a new operator problem: each activated capability can bring its own manual install instructions.

The goal is simple:

- keep concern-specific behavior out of `specify` core;
- avoid bundling every first-party capability helper into the main install;
- avoid asking users to manually install N helper binaries;
- let capability authors choose scripts or binaries according to the job.

## Design

### Principle

Capability helpers are resolved through the capability system. Skills should not say "install this helper first"; they should call either a script in the resolved capability directory or a tool path returned by `specify`.

### Co-Located Scripts

A capability may ship small helpers in its own tree:

```text
capabilities/contracts/
├── capability.yaml
├── briefs/
│   └── merge.md
└── scripts/
    └── check-semver.ts
```

Briefs get a `$CAPABILITY_DIR` substitution pointing at the resolved capability cache. A contracts merge brief can therefore run:

```bash
deno run --allow-read "$CAPABILITY_DIR/scripts/check-semver.ts" "$BASELINE_DIR"
```

Use this path for small, reviewable helpers that do not justify a release pipeline. Runtime requirements such as Deno or Python remain capability prerequisites.

### Declared Tools

Helpers that should stay as binaries are declared in `capability.yaml`:

```yaml
name: vectis
version: 2
description: Vectis Crux application workflow

pipeline:
  build:
    - id: implement
      brief: briefs/build.md

tools:
  - name: specify-vectis
    version: ^1.0
    source: github-release
    repo: augentic/specify-cli
    asset: "specify-vectis-{version}-{target}.tar.gz"
    sha256:
      aarch64-apple-darwin: "<64 hex chars>"
```

First landing supports only `source: github-release`. New sources can be added later without changing the skill-facing contract.

### Resolver Behavior

When a capability with `tools:` is resolved, `specify` ensures each tool is present in a global cache:

```text
~/.cache/specify/tools/<tool-name>/<version>/<target>/<tool-name>
```

For each tool, the resolver:

1. Infers the host target using the same OS / architecture mapping as `install.sh`.
2. Resolves the highest release matching `version`.
3. Reuses a cached binary when its metadata matches the manifest.
4. Otherwise downloads the release asset, verifies SHA256 when provided, extracts into a temp directory, and atomically moves it into the cache.

The cache is global because tool archives are identical across projects on the same machine. Capability briefs remain in the project-local capability cache.

### Skill Invocation

Skills invoke declared tools through a stable shim:

```bash
TOOL=$(specify capability tool specify-vectis)
"$TOOL" verify
```

Briefs may also use `$TOOL[specify-vectis]`, which expands to the resolved path and triggers resolution if needed.

The cache layout is not part of the skill contract. The shim is.

### CLI Surface

Add a `tool` subresource under `specify capability`:

```bash
specify capability tool <name>          # print resolved path, fetching if needed
specify capability tool list            # show declared tools and cache status
specify capability tool fetch [<name>]  # prefetch one or all tools
specify capability tool show <name>     # show metadata
specify capability tool gc              # remove unused cached versions
```

`fetch` and `gc` touch only `~/.cache/specify/tools/`; they do not mutate project state.

### Trust and Offline Behavior

Tool trust follows capability trust: the operator already trusts the capability manifest, and the manifest names the tools it needs. SHA256 pins should be warnings in the first landing and hard errors in the next minor release.

Cached tools work offline. First use without network fails with a typed resolver error. Air-gapped users can pre-populate the cache with `specify capability tool fetch --all` on a connected machine.

## Manifest Delta

`capabilities/capability.schema.json` gains an optional `tools:` array. The first schema only needs the `github-release` shape:

```yaml
tools:
  - name: <executable-name>
    version: <semver-requirement>
    source: github-release
    repo: <owner>/<repo>
    asset: <asset-template-with-{version}-and-{target}>
    sha256:
      <target>: <64-hex-digest>
```

Missing `tools:` means no new behavior.

## Implementation Scope

### Phase 1: Manifest Support

Add `tools:` to the capability schema and parsed capability type. `specify capability check` validates tool names, SemVer requirements, supported source values, asset templates, and SHA256 values.

Acceptance: manifests without `tools:` behave exactly as before; a Vectis fixture with `tools:` validates.

### Phase 2: Resolver

Add `crates/capability/src/tools/` with target inference, GitHub release resolution, SHA256 verification, atomic extraction, cache metadata, and cache reuse.

Acceptance: tests cover cache hit, cache miss, SHA256 mismatch, unsupported target, and network failure.

### Phase 3: CLI and Brief Integration

Add `specify capability tool {path,list,fetch,show,gc}` behavior under the existing capability command family, with `specify capability tool <name>` as the path-printing shorthand. Add `$TOOL[<name>]` and `$CAPABILITY_DIR` substitutions to brief rendering.

Acceptance: a fixture capability can resolve a synthetic tool and render a brief that references it.

### Phase 4: First-Party Capabilities

Replace the RFC-13 Phase 4.2a contract-validator binary with a co-located contracts script.

Keep the RFC-13 Phase 4.3a `specify-vectis` binary, but distribute it through `capabilities/vectis/capability.yaml:tools`. Update Vectis skills to call `$(specify capability tool specify-vectis)`.

Acceptance: contracts and Vectis workflows run with no user-visible install beyond `specify`.

### Phase 5: Docs and Lints

Document capability tools and update prerequisites to say helpers are auto-resolved. Add RFC-5 follow-up lints for:

- declared binary tools missing SHA256 pins;
- skills invoking bare helper names instead of `specify capability tool`.

## Migration

This is additive for capabilities without `tools:`.

For first-party capabilities:


| Draft RFC-13 shape                     | RFC-15 shape                                       |
| -------------------------------------- | -------------------------------------------------- |
| `specify-contract-validate` binary     | contracts script in the capability tree            |
| manually installed `specify-vectis`    | declared `tools:` entry, fetched by `specify`      |
| bare `specify-vectis verify` in skills | `$(specify capability tool specify-vectis) verify` |


No compatibility shim is needed because these helper binaries have not shipped as a public surface yet.

## Alternatives Considered

**Manual installs per capability.** Rejected because it makes capability adoption depend on separate package-manager instructions and version drift.

**Bundle all helpers with `specify`.** Rejected because it reintroduces capability-specific behavior into the main release and cannot scale to third-party capabilities.

**Rewrite every helper as a script.** Rejected because some helpers, especially Vectis, are better maintained as compiled tools.

**Use `cargo install` / `cargo binstall` directly.** Rejected as the default because it assumes Rust tooling and pushes distribution details into skills.

**Add a plugin runtime.** Rejected for the same reason RFC-13 rejects subprocess / WASM / dynamic-library plugins: skills already provide the imperative execution surface.

## Non-Goals

- General package management.
- Sandboxing helper execution beyond the host tool model skills already use.
- Tool dependency graphs.
- Managing Deno, Python, or other script runtimes.
- Perfect air-gapped UX in the first landing.

## Open Questions

1. **Signing.** SHA256 pins are enough for the first landing; signatures can follow if third-party binary tools become common.
2. **More sources.** `oci`, `cargo-binstall`, `s3`, and enterprise mirrors are plausible later `source:` values.
3. **Exact versions vs. SemVer ranges.** Provisional: allow SemVer requirements, with exact pins available through `=1.2.3`.
4. **Project-local tool cache.** Provisional: global cache by default; allow `SPECIFY_TOOLS_CACHE` for CI and hermetic use.
5. **Resolver concurrency.** Use a per-tool cache lock if concurrent resolves become an issue.
6. **Runtime declarations for scripts.** Provisional: document runtimes in capability prerequisites rather than adding manifest fields.

## References

- [RFC-13: Immutable core + capability extensions](rfc-13-extensibility.md) - owns the capability protocol and the open distribution question.
- [RFC-13 implementation plan](rfc-13-plan.md) - defines the provisional contract and Vectis helper binaries this RFC revises.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) - owns the contract validation behavior that moves to a script.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) - owns the CLI and capability resolver.
- [RFC-5: Framework Linter](rfc-5-lint.md) - home for the follow-up lints.
- [RFC-14: Workspaces](rfc-14-workspaces.md) - uses `extensions:` in `project.yaml`; this RFC uses `tools:` in `capability.yaml` to avoid vocabulary collision.

