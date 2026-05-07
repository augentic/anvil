# RFC-16: Vectis WASI Tools

> Status: Implemented - Depends: [RFC-15](archive/rfc-15-wasm-plugins.md), [RFC-13](archive/rfc-13-extensibility.md) - Defers: [WASI process spawning](https://github.com/WebAssembly/WASI/issues/899)

## Abstract

RFC-15 gives Specify one portable extension path for deterministic helper code: declared WASI components run through `specify tool run`. Vectis is the first first-party capability that exposes the hard edge of that model. Some Vectis behavior is pure validation and template logic that fits WASI today; other behavior shells out to Cargo, Swift, Xcode, Gradle, `rustup`, registries, and platform SDKs. WASI does not yet provide a process-spawning API that can run those host tools with a clear permission model.

This RFC splits Vectis accordingly:

- pure Vectis validation and render-only scaffold generation move into declared WASI tools;
- host toolchain orchestration moves into Vectis skills and their verify-repair sub-agents;
- `specify-vectis` is retired rather than preserved as a second installed CLI;
- `specify` keeps the generic `specify tool` surface and does not gain capability-specific Vectis verbs;
- the first RFC-15 implementation removes the provisional `ToolRunner` trait and keeps only a concrete WASI host boundary until a second declared runtime exists.

Users still install one binary: `specify`.

## Motivation

### The RFC-15 boundary is intentionally narrow

RFC-15 withholds ambient host environment, inherited `PATH`, network access, and process spawning from WASI tools. That is the right security posture. Declared tools should be reviewable filesystem helpers, not hidden native scripts with a `.wasm` extension.

Vectis currently contains both sides of that line:

- validation of `tokens.yaml`, `assets.yaml`, `layout.yaml`, and `composition.yaml`;
- scaffold/template rendering for Crux, iOS, and Android project files;
- build verification through `cargo`, `cargo clippy`, `cargo deny`, `cargo vet`, `make`, `xcodebuild`, `gradle`, and `./gradlew`;
- workstation prerequisite checks through `rustup`, `xcode-select`, `$ANDROID_HOME`, Java, Android SDK/NDK, and installed Cargo subcommands;
- version update checks through networked registries and a scratch cap-matrix build.

The validation group and render-only parts of scaffold/template rendering cleanly fit RFC-15 today. The build, prerequisite, network, registry, and host-derived scaffold post-processing behaviors do not fit until WASI has process spawning and Specify has a policy model for granting it.

### A second Vectis CLI is the wrong compatibility layer

`specify-vectis` was useful as an RFC-13 transition shape, but keeping it as the long-term hybrid boundary creates three problems:

1. Operators have to install or discover a second binary even though RFC-15 promises `specify` as the declared-tool entrypoint.
2. Skills get two deterministic surfaces (`specify tool run ...` and `specify-vectis ...`) with different security stories.
3. Vectis remains a first-class capability-specific CLI, which pulls against RFC-13's "immutable core plus capabilities" split.

The hybrid model should be explicit in the skills, not hidden behind a native helper binary. When a skill needs host authority, it should run the host commands it needs and surface their output as agent-visible evidence. When it needs deterministic validation, it should call a declared WASI tool through `specify`.

### Abstracting over runners is premature

RFC-15 asked for a narrow runner boundary so manifest parsing, cache resolution, and CLI output would not be tangled with Wasmtime. The first implementation satisfied that with a `ToolRunner` trait and one implementation, `WasiRunner`.

That trait does not buy anything yet. It suggests there may be a second runtime inside `tools:` when the RFC-15 policy explicitly rejects native fallback runners. A concrete `WasiRunner` or `run_wasi_tool(...)` function is enough of a boundary until Specify accepts a second declared runtime through a future RFC.

## Design

### Responsibility Split

Vectis behavior is divided by authority, not by historical CLI verb.

| Behavior | Owner after this RFC | Rationale |
| --- | --- | --- |
| UI input validation (`tokens`, `assets`, `layout`, `composition`, `all`) | Declared WASI tool | Filesystem-only, deterministic, no host process or network need. |
| Structured validation diagnostics | WASI tool stdout/stderr in v1; typed WIT later | RFC-15's command world is enough for the first migration. |
| Template rendering for scaffold files | Declared WASI tool | Pure embedded-template rendering and declared project writes fit RFC-15 when separated from host post-processing. |
| Core/iOS/Android build verification | Vectis skills | Requires host processes and platform toolchains. |
| Prerequisite checks | Vectis skills | Requires host environment, installed binaries, SDK paths, and user-specific setup. |
| Scaffold post-processing (`local.properties`, Java home, Gradle wrapper, Android SDK/NDK detection) | Vectis skills | Mutates platform-specific project files based on host Gradle/Android/JDK state. |
| Version registry queries | Vectis skills | Requires network access, which RFC-15 withholds from WASI tools. |
| `update-versions --verify` cap matrix | Vectis `template-updater` skill | Combines network, scaffolding, host build tools, and diagnosis. |

The line is simple: **a WASI tool may read and write declared files; a skill may run host processes.**

### Tool Declarations

The Vectis capability declares validation and render-only scaffold tools in its RFC-15 sidecar:

```yaml
# capabilities/vectis/tools.yaml
tools:
  - name: vectis-validate
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/vectis-validate-1.0.0/vectis-validate.wasm"
    sha256: "<hex-encoded sha256 of the component bytes>"
    permissions:
      read:
        - "$PROJECT_DIR/.specify"
        - "$PROJECT_DIR/design-system"
      write: []
  - name: vectis-scaffold
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/vectis-scaffold-1.0.0/vectis-scaffold.wasm"
    sha256: "<hex-encoded sha256 of the component bytes>"
    permissions:
      read:
        - "$PROJECT_DIR"
        - "$CAPABILITY_DIR"
      write:
        - "$PROJECT_DIR"
```

The exact release URIs can change before implementation, but the declared names are stable.

### Frozen v1 Tool Arguments

`vectis-validate` keeps the old validator's mode-plus-optional-path shape:

```bash
specify tool run vectis-validate -- <mode> [path]
```

`<mode>` is one of `tokens`, `assets`, `layout`, `composition`, or `all`. For single-artifact modes, `[path]` points at that artifact; if omitted, the tool uses the existing Vectis default-path cascade relative to `PROJECT_DIR`. For `all`, `[path]` is the project root to scan; if omitted, it is `PROJECT_DIR`.

`vectis-scaffold` owns only deterministic render-and-write work. It receives explicit scaffold inputs and assembly selection, then writes the same embedded-template outputs the old Vectis scaffold produced:

```bash
specify tool run vectis-scaffold -- core <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
specify tool run vectis-scaffold -- ios <app-name> [--caps <csv>] [--version-file <path>]
specify tool run vectis-scaffold -- android <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
```

`<csv>` is a comma-separated subset of `http`, `kv`, `time`, `platform`, and `sse`, deduplicated in input order. The output root is `PROJECT_DIR`; v1 does not accept `--dir`, `--output`, or a combined `--shells` flag. `core` writes the shared Crux project files, `ios` writes the iOS shell tree, and `android` writes the Android shell tree. `--android-package` defaults to `com.vectis.<lowercase-app-name>` for `core` and `android`.

Version pins are delivered by embedded defaults plus one explicit override file. `--version-file <path>` names a complete TOML document with the same schema as today's embedded `versions.toml`; the path is resolved relative to `PROJECT_DIR` unless the host preopen policy already makes an absolute path available, and the file must exist. When the flag is omitted, `vectis-scaffold` uses its embedded defaults. The WASI tool does not read `~/.config/vectis/versions.toml`, implicitly discover project-local `versions.toml`, accept JSON on stdin, or expose per-pin command flags in v1.

The tool preserves the current atomic refusal contract: compute every target path first, reject pre-existing targets before writing anything, then create directories and write rendered files. It does not run host commands or derive host-local configuration.

The validation component emits the existing v2 Vectis validation envelope:

```json
{
  "schema-version": 2,
  "mode": "composition",
  "path": ".specify/specs/composition.yaml",
  "errors": [],
  "warnings": []
}
```

Exit semantics stay unchanged:

| Outcome | Exit code |
| --- | --- |
| No validation errors | 0 |
| Validation errors found | 1 |
| Invocation, IO, parse crash, or runtime failure | 2 |

Warnings do not make the tool fail.

### Validation Component Scope

The first WASI component contains only validation logic:

1. JSON Schema validation for `tokens.yaml`, `assets.yaml`, `layout.yaml`, and `composition.yaml`.
2. Unwired-subset checks for layout documents.
3. Structural identity checks for repeated `component:` slugs.
4. Token and asset reference checks from composition to sibling manifests.
5. Asset source existence checks under declared project directories.
6. Default path resolution using `PROJECT_DIR` and declared preopens.

The component does not:

- inspect generated Rust, Swift, or Kotlin code;
- run `cargo`, `make`, Xcode, Gradle, `rustup`, shell scripts, or package managers;
- query crates.io, Maven, Google Maven, GitHub, or any other networked registry;
- read ambient environment variables other than RFC-15's `PROJECT_DIR` and, for capability-scoped tools, `CAPABILITY_DIR`;
- write `.specify` lifecycle state.

### Scaffold Component Scope

The first scaffold WASI component contains only deterministic rendering logic:

1. Render embedded Crux core templates from explicit app name, package name, capability flags, and version pins.
2. Render embedded iOS shell templates from explicit app name, capability flags, and version pins.
3. Render embedded Android shell templates from explicit app name, package name, capability flags, and version pins.
4. Preserve the existing refusal-to-overwrite semantics by planning target paths before creating directories or files.
5. Write only declared project files under the project root.

The component does not:

- inspect generated Rust, Swift, or Kotlin code after rendering;
- run `cargo`, `make`, Xcode, Gradle, `rustup`, shell scripts, or package managers;
- query registries or resolve latest dependency versions;
- read `$ANDROID_HOME`, `JAVA_HOME`, `PATH`, `/usr/libexec/java_home`, or host SDK directories;
- bootstrap Gradle wrapper artifacts;
- write SDK-derived `local.properties` or host-derived Java/NDK settings;
- write `.specify` lifecycle state.

### Validation Implementation Shape

The existing `specify-vectis` validation module should be extracted into a WASI-buildable crate rather than duplicated.

Proposed `specify-cli` workspace shape:

```text
crates/
  vectis-validate/
    Cargo.toml
    src/
      lib.rs        # validation engine and JSON envelope types
      main.rs       # WASI command-world wrapper
```

The crate should avoid dependencies that fail on `wasm32-wasip2`. It should own only the validation engine and the command wrapper. Host-only Vectis code remains outside this crate.

The current `crates/vectis/src/validate.rs` may be reduced to tests, moved wholesale, or deleted depending on how much of `crates/vectis` remains after `specify-vectis` is retired. The important invariant is that there is one validation implementation and one envelope contract.

The extracted validation engine may continue to return payload objects internally, but the WASI command wrapper must emit the complete v2 envelope that `specify-vectis validate --format json` emitted: `schema-version: 2` first, then the validation payload fields. It must also preserve the recursive validation exit-code behavior for `composition` and `all` envelopes that fold sub-results. This keeps downstream automation byte-shape compatible for validation success and findings cases even though resolver, permission, and runtime failures now come from `specify tool run`.

### Skill-Owned Host Verification

The Vectis build and merge briefs stop invoking `specify-vectis verify`. They make host authority explicit by running verification steps from skills and sub-agents.

The core verify-repair loop owns:

```bash
cd "$PROJECT_DIR" && cargo fmt --check
cd "$PROJECT_DIR" && cargo check
cd "$PROJECT_DIR" && cargo clippy --all-targets -- -D warnings
cd "$PROJECT_DIR" && cargo test
```

The post-merge cap-matrix check owns the broader end-to-end steps:

```bash
cd "$PROJECT_DIR" && cargo check
cd "$PROJECT_DIR" && cargo clippy --all-targets -- -D warnings
cd "$PROJECT_DIR" && cargo deny check
cd "$PROJECT_DIR" && cargo vet
```

When an iOS shell exists, the iOS verify sub-agent owns:

```bash
cd "$IOS_SHELL_DIR" && make typegen
cd "$IOS_SHELL_DIR" && make package
cd "$IOS_SHELL_DIR" && make xcode
cd "$IOS_SHELL_DIR" && xcodebuild build ...
```

When an Android shell exists, the Android verify sub-agent owns:

```bash
cd "$ANDROID_SHELL_DIR" && make build
cd "$ANDROID_SHELL_DIR" && ./gradlew :shared:cargoBuild
cd "$ANDROID_SHELL_DIR" && ./gradlew :app:assembleDebug
```

The brief or skill text should preserve the existing structured reporting contract: each verification step records `name`, `passed`, and a failure snippet. The structure moves from a native binary's JSON envelope into the sub-agent's returned verification object and journal entries.

### Scaffolding Transition

This RFC requires render-only scaffold generation to move to WASI in the first implementation. Anything that can be expressed as deterministic template rendering from explicit inputs belongs in a declared tool.

Scaffolding is split into:

- `vectis-scaffold`, a pure renderer that produces files from embedded templates and explicit inputs;
- skill-owned host post-processing and build verification that runs after files are written.

`vectis-scaffold` is a separate declared tool, not a mode on `vectis-validate`. It needs write permissions over project source directories and therefore deserves a separate manifest entry, audit surface, and docs page.

The WASI scaffold cut must not bootstrap Gradle, write SDK-derived `local.properties`, inspect `/usr/libexec/java_home`, inspect `$ANDROID_HOME`, run `make`, or call `cargo`. Those remain skill-owned host operations.

### Retiring `specify-vectis`

`specify-vectis` is removed as a public binary. There is no replacement `specify vectis` subcommand tree.

Change 00 audited the `specify-cli` repository evidence before implementation: `crates/vectis/Cargo.toml` names the package `specify-vectis` but sets `publish = false`; the release workflow's archive steps package only the `specify` binary; and `docs/release.md` describes a public crates.io publish order that omits `specify-vectis`. The repository does contain stale publish steps and active docs/comments that describe `specify-vectis` as if it shipped, but those are implementation drift rather than evidence of an externally released artifact. RFC-16 therefore chooses deletion in Change 13, not a time-boxed deprecation wrapper. Change 13 must remove the stale release/publish wiring and active docs at the same time it removes the binary.

The durable command mapping is:

| Old surface | New owner |
| --- | --- |
| `specify-vectis validate <mode> [path]` | `specify tool run vectis-validate -- <mode> [path]` |
| `specify-vectis verify` | Vectis verify-repair skill/sub-agent steps |
| `specify-vectis init` | `specify tool run vectis-scaffold -- core <app-name>` plus optional shell render steps and skill-owned host post-processing |
| `specify-vectis add-shell` | `specify tool run vectis-scaffold -- ios|android <app-name>` plus iOS/Android writer skills |
| `specify-vectis update-versions` | Vectis `template-updater` skill |
| `specify-vectis versions` | Documentation and skill-readable version manifest; future WASI helper only if needed |

Because the audit found no shipped external contract, the implementation removes the binary outright and does not add a wrapper that could become a second long-term host runner.

### `specify` CLI Surface

`specify` remains capability-agnostic. This RFC adds no Vectis-specific top-level verbs.

The only CLI surface used by the WASI half is RFC-15:

```bash
specify tool run vectis-validate -- composition
specify tool run vectis-scaffold -- core Counter --caps http,kv
specify tool list
specify tool fetch vectis-validate
specify tool fetch vectis-scaffold
specify tool show vectis-validate
```

The host-process half is deliberately not hidden behind `specify tool`. Skills run host commands directly because they are the authority-bearing layer until WASI process spawning and its permission model are specified.

### Host Boundary Cleanup

The `specify-tool` crate should expose a concrete WASI execution API rather than a one-implementation trait:

```rust
let runner = WasiRunner::new()?;
let exit = runner.run(&resolved, &run_ctx)?;
```

or:

```rust
let exit = specify_tool::host::run_wasi_tool(&resolved, &run_ctx)?;
```

The `ToolRunner` trait is removed from the first implementation. Tests that need to avoid Wasmtime should test resolver, cache, permission, and CLI selection logic below the execution call, or factor helper functions that do not require trait-object injection.

A runner trait can return in a future RFC if all of the following are true:

1. There is a second declared runtime accepted by policy.
2. The manifest schema carries enough runtime information to select it.
3. The second runtime has the same security envelope or an explicitly different one that operators can review.

Native host processes do not satisfy those conditions under this RFC.

## Implementation Plan

1. **Extract Vectis validation.** Move the validation engine and v2 JSON envelope into a `wasm32-wasip2`-buildable crate. Keep existing validation behavior and exit semantics.
2. **Extract render-only scaffolding.** Move deterministic core, iOS, and Android template rendering into a `wasm32-wasip2`-buildable scaffold crate. Keep overwrite refusal, path planning, placeholder substitution, capability-flag rendering, and version-pin substitution. Leave host-derived post-processing outside the crate.
3. **Build `vectis-validate.wasm` and `vectis-scaffold.wasm`.** Add release packaging for both WASI command components and integration fixtures that run them through `specify tool run`.
4. **Declare the tools.** Add `capabilities/vectis/tools.yaml` with `vectis-validate` and `vectis-scaffold`, SHA-256 pinning for release builds, read-only validation permissions, and scaffold write permissions over the project output surface.
5. **Rewrite Vectis briefs.** Replace every `specify-vectis validate ...` invocation with `specify tool run vectis-validate -- ...`. Replace scaffold invocations with `specify tool run vectis-scaffold -- ...` followed by explicit skill-owned host post-processing where needed. Replace `specify-vectis verify` guidance with explicit skill-owned verification steps and structured sub-agent return contracts.
6. **Retire the native binary.** Remove the `[[bin]] name = "specify-vectis"` target, release/publish wiring, and CLI docs. Do not add a deprecation wrapper; Change 00 found no shipped external contract.
7. **Remove `ToolRunner`.** Replace the trait with a concrete WASI host API. Keep resolver/cache/permission code independent from Wasmtime where practical, but do not pretend native runners are supported.
8. **Update tests and docs.** Add acceptance coverage for validation success, validation findings, missing files, denied filesystem access, scaffold overwrite refusal, scaffold file output parity, scaffold permission denial, non-zero exit propagation, and brief command examples.

## Migration

For Vectis capability authors and skills:

| Before | After |
| --- | --- |
| `specify-vectis validate composition` | `specify tool run vectis-validate -- composition` |
| `specify-vectis validate all` | `specify tool run vectis-validate -- all` |
| `specify-vectis init <app-name>` | `specify tool run vectis-scaffold -- core <app-name>` plus optional `ios` / `android` render steps and skill-owned post-processing |
| `specify-vectis add-shell ios` | `specify tool run vectis-scaffold -- ios <app-name>` plus iOS writer/verification steps |
| `specify-vectis add-shell android` | `specify tool run vectis-scaffold -- android <app-name> [--android-package <package>]` plus Android host post-processing |
| `specify-vectis verify --dir "$PROJECT_ROOT"` | Skill-owned host verify steps |
| `specify-vectis update-versions --verify` | `/vectis:template-updater` host workflow |

For operators:

- Install only `specify`.
- Run `specify tool fetch vectis-validate` and `specify tool fetch vectis-scaffold` when preparing offline Vectis work.
- Treat missing Cargo/Xcode/Gradle/Android prerequisites as skill-side environment blockers, not WASI tool failures.

For downstream automation:

- Parse the validation envelope exactly as before.
- Stop depending on `specify-vectis verify` JSON. The Vectis merge/build skills provide the structured verification summary and journal entries instead.

## Alternatives Considered

**Keep `specify-vectis` as the hybrid runner.** Rejected because it preserves a second installed CLI, hides host process authority behind a capability binary, and competes with RFC-15's declared tool path.

**Add native fallback entries to `tools:`.** Rejected for the same reason RFC-15 rejected native fallbacks: mixed runtimes blur the trust and permission model. Host tools stay explicit in skills until a separate RFC defines host runners.

**Reintroduce `specify vectis ...` inside the main binary.** Rejected because it makes the core CLI capability-aware again. `specify` should know tools, caches, permissions, and lifecycle; it should not know Vectis commands.

**Move all Vectis behavior to WASI immediately.** Rejected because verification, registry querying, Gradle wrapper bootstrap, SDK-derived local configuration, and platform SDK detection need host process, network, and environment access that RFC-15 intentionally withholds.

**Keep `ToolRunner` for future-proofing.** Rejected because the future runtime is intentionally not designed yet. A concrete boundary is easier to read and less misleading.

## Non-Goals

- Defining WASI process-spawning permissions.
- Adding network access to RFC-15 tools.
- Adding native host runners to `tools:`.
- Replacing Vectis specialist skills.
- Moving host-dependent scaffold post-processing to WASI in the first implementation.
- Preserving unshipped `specify-vectis` behavior as a compatibility requirement.
- Designing typed WIT diagnostics for validators; command-world JSON remains acceptable for v1.

## Open Questions

1. **Tool granularity.** Should `vectis-scaffold` later split into `vectis-scaffold-core`, `vectis-scaffold-ios`, and `vectis-scaffold-android` so permissions can be narrower?
2. **Validation ABI.** When validation findings need machine-readable stability beyond today's JSON envelope, should Vectis be the first tool to move from command-world stdout to a custom WIT world?
3. **Post-WASI-899 model.** Once WASI process spawning lands, should Specify add a new declared runner type, or should host process orchestration remain skill-owned for auditability?

## References

- [RFC-15: WASI Capability Tools](archive/rfc-15-wasm-plugins.md)
- [RFC-13: Immutable core + capability extensions](archive/rfc-13-extensibility.md)
- [WASI process spawning issue](https://github.com/WebAssembly/WASI/issues/899)
