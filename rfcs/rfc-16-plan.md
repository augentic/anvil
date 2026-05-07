# RFC-16 Implementation Plan

> Source RFC: [RFC-16: Vectis WASI Tools](rfc-16-wasi-vectis.md)

## Goal

Implement RFC-16 in small, independently reviewable changes that can be assigned to separate subagents without over-consuming context. The work spans two repositories:

- `specify-cli`: Rust CLI, WASI host, Vectis extraction, release packaging, tests, and `specify-vectis` retirement.
- `specify`: Vectis capability sidecar, briefs, skills, and documentation that call the new declared tools.

RFC-16's implementation should leave operators with one installed binary, `specify`, while moving deterministic Vectis validation and render-only scaffolding into declared WASI tools:

- `vectis-validate`
- `vectis-scaffold`

Host-dependent behavior remains skill-owned: Cargo, Gradle, Xcode, `make`, registry queries, prerequisite checks, SDK discovery, Gradle wrapper bootstrap, `local.properties`, Java home, and NDK detection.

## Dependency Graph

```text
00 compatibility + interface audit
├── 01 tool-host cleanup
└── 02 workspace skeletons
    ├── 03 vectis-validate extraction
    └── 04 vectis-scaffold extraction
        └── 05 host post-processing split

03 + 04 ──> 06 WASI packaging + release artifacts
06 ──────> 07 specify tool integration tests
07 ──────> 08 Vectis tools.yaml
08 ──────> 09 capability brief migration
08 ──────> 10 writer/reviewer skill migration
08 ──────> 11 template-updater/version workflow migration
08 ──────> 12 docs migration
09 + 10 + 11 + 12 ──> 13 retire specify-vectis
13 ──────> 14 final validation sweep
```

Changes with the same prerequisite can usually run in parallel once that prerequisite lands. Changes `09` through `12` are intentionally parallel documentation/skill lanes after the tool names, arguments, and sidecar shape are stable.

## Change 00: Compatibility And Interface Audit

**Repo:** `specify-cli` plus `specify`

**Purpose:** Resolve decisions that affect every downstream subagent before code extraction begins.

**Scope:**

- Audit whether `specify-vectis` was ever published or treated as an external contract.
- Decide whether RFC-16 needs immediate deletion or a time-boxed deprecation wrapper.
- Freeze command arguments for:
  - `specify tool run vectis-validate -- <mode> [path]`
  - `specify tool run vectis-scaffold -- core <app-name> ...`
  - `specify tool run vectis-scaffold -- ios <app-name> ...`
  - `specify tool run vectis-scaffold -- android <app-name> ...`
- Decide how `vectis-scaffold` receives version pins: embedded defaults, explicit file argument, JSON/stdin, or command flags.
- Fix any obvious RFC naming drift before implementation, such as the current title wording if it still reads `Vectis WASI Vectis`.

**Likely files:**

- `specify/rfcs/rfc-16-wasi-vectis.md`
- `specify-cli/crates/vectis/Cargo.toml`
- `specify-cli/.github/workflows/release.yaml`
- `specify-cli/Makefile.toml`

**Acceptance criteria:**

- A short implementation note records the `specify-vectis` compatibility decision.
- Tool argument surfaces are stable enough for parallel extraction and docs work.
- Follow-on subagents know whether to delete or wrap `specify-vectis`.

**Parallelism:** Must run first.

**Implementation note (completed):** Repository evidence supports deleting `specify-vectis`, not wrapping it. `specify-cli/crates/vectis/Cargo.toml` sets `publish = false`, release archives package only the `specify` binary, and `specify-cli/docs/release.md` omits `specify-vectis` from the public crates.io publish order. Stale publish steps and stale active docs/comments exist, but they do not establish a shipped external contract. Downstream changes should remove the binary and stale release/publish wiring in Change `13` rather than add a deprecation wrapper.

Frozen v1 tool arguments:

```bash
specify tool run vectis-validate -- <mode> [path]
specify tool run vectis-scaffold -- core <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
specify tool run vectis-scaffold -- ios <app-name> [--caps <csv>] [--version-file <path>]
specify tool run vectis-scaffold -- android <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
```

`vectis-validate` modes are `tokens`, `assets`, `layout`, `composition`, and `all`. `vectis-scaffold` writes under `PROJECT_DIR`; v1 has no `--dir`, `--output`, combined `--shells`, stdin JSON, or per-pin version flags. Version pins come from embedded defaults unless `--version-file <path>` names a complete TOML override; the WASI tool must not read `~/.config/vectis/versions.toml` or implicitly discover project-local `versions.toml`.

## Change 01: Remove The Premature ToolRunner Abstraction

**Repo:** `specify-cli`

**Purpose:** Align the RFC-15 tool host with RFC-16 before new Vectis tools depend on it.

**Scope:**

- Remove the one-implementation `ToolRunner` trait.
- Expose a concrete `WasiRunner::run(...)` method or `run_wasi_tool(...)` function.
- Update `specify tool run` to use the concrete API.
- Update tests and crate docs that mention `ToolRunner`.

**Likely files:**

- `specify-cli/crates/tool/src/host.rs`
- `specify-cli/crates/tool/src/lib.rs`
- `specify-cli/src/commands/tool.rs`
- `specify-cli/tests/tool.rs`

**Acceptance criteria:**

- `cargo test -p specify-tool` passes.
- Existing `specify tool` integration tests pass.
- No public docs imply native runners are supported.

**Parallelism:** Can run after Change `00`, in parallel with Change `02`.

## Change 02: Add WASI Tool Workspace Skeletons

**Repo:** `specify-cli`

**Purpose:** Create stable crate boundaries so validation and scaffold extraction can proceed in parallel with minimal file conflicts.

**Scope:**

- Add empty or minimal crates:
  - `crates/vectis-validate`
  - `crates/vectis-scaffold`
- Add workspace membership and local build targets.
- Add placeholder command-world `main.rs` files that parse `--help` and fail with a clear not-implemented exit.
- Decide whether shared pure Vectis types live in one of these crates, a small shared crate, or duplicated minimal modules.

**Likely files:**

- `specify-cli/Cargo.toml`
- `specify-cli/crates/vectis-validate/Cargo.toml`
- `specify-cli/crates/vectis-validate/src/main.rs`
- `specify-cli/crates/vectis-validate/src/lib.rs`
- `specify-cli/crates/vectis-scaffold/Cargo.toml`
- `specify-cli/crates/vectis-scaffold/src/main.rs`
- `specify-cli/crates/vectis-scaffold/src/lib.rs`

**Acceptance criteria:**

- `cargo check --workspace` succeeds for native targets.
- `cargo build -p vectis-validate --target wasm32-wasip2` reaches the placeholder binary.
- `cargo build -p vectis-scaffold --target wasm32-wasip2` reaches the placeholder binary.

**Parallelism:** Can run after Change `00`, in parallel with Change `01`. Changes `03` and `04` depend on it.

## Change 03: Extract `vectis-validate`

**Repo:** `specify-cli`

**Purpose:** Move deterministic Vectis validation into a WASI command component.

**Scope:**

- Extract validation logic from `crates/vectis/src/validate.rs`.
- Move or share embedded schemas:
  - `tokens.schema.json`
  - `assets.schema.json`
  - `composition.schema.json`
- Preserve modes: `tokens`, `assets`, `layout`, `composition`, `all`.
- Preserve v2 validation envelope shape with `schema-version: 2`.
- Preserve recursive exit-code behavior for `composition` and `all`.
- Replace ambient current-directory assumptions with RFC-15-compatible `PROJECT_DIR` and preopened paths.
- Keep validation read-only and deterministic.

**Likely files:**

- `specify-cli/crates/vectis/src/validate.rs`
- `specify-cli/crates/vectis/src/lib.rs`
- `specify-cli/crates/vectis/embedded/*.schema.json`
- `specify-cli/crates/vectis-validate/**`
- `specify-cli/crates/vectis/tests/specify_vectis_bin.rs`

**Acceptance criteria:**

- Native unit tests cover the extracted validation engine.
- WASI command emits the same validation success/findings JSON shape as the current `specify-vectis validate --format json`.
- WASI command exits `0` clean, `1` validation findings, and `2` invocation/IO/runtime failure.
- `wasm32-wasip2` build succeeds.

**Parallelism:** Depends on Change `02`. Can run in parallel with Change `04`.

## Change 04: Extract `vectis-scaffold`

**Repo:** `specify-cli`

**Purpose:** Move render-only Vectis scaffolding into a WASI command component.

**Scope:**

- Extract pure template rendering and file planning for:
  - core scaffold
  - iOS shell scaffold
  - Android shell scaffold
- Preserve placeholder substitution, capability-flag rendering, version-pin substitution, and target path planning.
- Preserve atomic refusal: reject all pre-existing target files before creating directories or writing bytes.
- Accept only explicit inputs and declared files; do not inspect host SDKs or run tools.
- Implement the Change `00` frozen scaffold arguments exactly; do not reintroduce `--dir`, `--output`, `--shells`, stdin JSON, or per-pin version flags.
- Leave host-derived post-processing outside the crate.

**Likely files:**

- `specify-cli/crates/vectis/src/templates/**`
- `specify-cli/templates/vectis/**`
- `specify-cli/crates/vectis/src/init/core.rs`
- `specify-cli/crates/vectis/src/init/ios.rs`
- `specify-cli/crates/vectis/src/init/android.rs`
- `specify-cli/crates/vectis-scaffold/**`

**Acceptance criteria:**

- Golden tests compare scaffold output against current render-only output.
- Existing overwrite-refusal behavior is covered.
- WASI command can write core, iOS, and Android scaffold files under allowed preopens.
- WASI command does not call `std::process`, read ambient `PATH`, inspect `$ANDROID_HOME`, or derive host-local config.
- `wasm32-wasip2` build succeeds.

**Parallelism:** Depends on Change `02`. Can run in parallel with Change `03`.

## Change 05: Split Host Post-Processing From Scaffold Rendering

**Repo:** `specify-cli`

**Purpose:** Preserve current scaffold behavior by making host-dependent steps explicit and callable outside the WASI renderer.

**Scope:**

- Isolate host steps that currently live inside scaffold flows:
  - prerequisite checks
  - iOS `make typegen`, `make package`, `make xcode`
  - Android `local.properties`
  - Java home detection
  - NDK detection
  - Gradle wrapper bootstrap
  - Android `make build` and Gradle assemble
- Expose or document these as host routines for skills/subagents rather than `vectis-scaffold`.
- Ensure render-only tests do not require host toolchains.

**Likely files:**

- `specify-cli/crates/vectis/src/init/mod.rs`
- `specify-cli/crates/vectis/src/init/ios.rs`
- `specify-cli/crates/vectis/src/init/android.rs`
- `specify-cli/crates/vectis/src/verify/**`
- `specify-cli/crates/vectis/src/prerequisites.rs`

**Acceptance criteria:**

- Host post-processing can be described as a sequence of ordinary shell commands or small helper functions.
- Render-only `vectis-scaffold` tests do not invoke host toolchains.
- No host behavior is silently hidden behind `specify tool run`.

**Parallelism:** Depends on Change `04`. Can run before docs/skill migration while release packaging proceeds.

## Change 06: Build And Package WASI Artifacts

**Repo:** `specify-cli`

**Purpose:** Produce distributable `vectis-validate.wasm` and `vectis-scaffold.wasm` artifacts with hashes.

**Scope:**

- Add build scripts or release workflow steps for both WASI command components.
- Decide artifact naming and release location.
- Produce SHA-256 hashes for first-party `tools.yaml` pins.
- Document local development override flow using `file://` sources.

**Likely files:**

- `specify-cli/.github/workflows/release.yaml`
- `specify-cli/Makefile.toml`
- `specify-cli/crates/vectis-validate/**`
- `specify-cli/crates/vectis-scaffold/**`
- `specify-cli/README.md` or release docs, if present

**Acceptance criteria:**

- Release build produces both `.wasm` components.
- Hashes are reproducible and recorded.
- Local `file://` smoke path is documented for development before public releases.

**Parallelism:** Depends on Changes `03` and `04`. Can overlap with Change `05` after artifact command surfaces are stable.

## Change 07: Add `specify tool run` Integration Coverage For Vectis

**Repo:** `specify-cli`

**Purpose:** Prove the new tools work through the actual RFC-15 runner, cache, and permission boundary.

**Scope:**

- Add fixture capability or test project declaring `vectis-validate` and `vectis-scaffold`.
- Test `tool list`, `tool fetch`, `tool show`, and `tool run`.
- Cover allowed and denied filesystem access.
- Cover validation findings and scaffold overwrite refusal.
- Cover non-zero guest exit propagation.

**Likely files:**

- `specify-cli/tests/vectis_tool.rs`
- `specify-cli/tests/fixtures/**`
- `specify-cli/crates/tool/src/host.rs`
- `specify-cli/src/commands/tool.rs`

**Acceptance criteria:**

- Tests exercise the final `specify tool run vectis-validate -- ...` and `specify tool run vectis-scaffold -- ...` commands.
- Permission failures come from the tool host, not ad hoc Vectis code.
- The tests can run in CI without Xcode, Android SDK, Gradle, or Cargo subprocess verification.

**Parallelism:** Depends on Change `06`. Can run before `specify` repo docs/skills are updated.

## Change 08: Add Vectis `tools.yaml`

**Repo:** `specify`

**Purpose:** Declare the new Vectis WASI tools in the capability package.

**Scope:**

- Add `capabilities/vectis/tools.yaml`.
- Mirror the contracts capability sidecar pattern.
- Include `vectis-validate` and `vectis-scaffold`.
- Use release URLs and SHA-256 pins from Change `06`, or documented local placeholders if this lands before release artifacts.

**Likely files:**

- `specify/capabilities/vectis/tools.yaml`
- `specify/capabilities/vectis/capability.yaml`
- `specify/docs/explanation/tool-declarations.md`, only if Vectis needs an example

**Acceptance criteria:**

- `specify tool list` sees both Vectis tools in a Vectis project.
- The sidecar grants read-only validation permissions and scaffold write permissions.
- No `tools:` field is added to `capability.yaml`.

**Parallelism:** Depends on Change `06` for final URLs and hashes. Can begin with local placeholders but should not be released until artifacts are available.

## Change 09: Migrate Vectis Capability Briefs

**Repo:** `specify`

**Purpose:** Replace old `specify-vectis` guidance in capability-owned phase briefs.

**Scope:**

- Rewrite validation gates to use `specify tool run vectis-validate -- ...`.
- Rewrite scaffold guidance to use `specify tool run vectis-scaffold -- ...` plus explicit host post-processing.
- Rewrite verification guidance to skill-owned host commands and structured journal entries.
- Remove any assumption that `specify-vectis verify` emits the canonical JSON report.

**Likely files:**

- `specify/capabilities/vectis/briefs/build.md`
- `specify/capabilities/vectis/briefs/merge.md`
- `specify/capabilities/vectis/briefs/composition.md`
- `specify/capabilities/vectis/briefs/design.md`
- `specify/capabilities/vectis/briefs/specs.md`

**Acceptance criteria:**

- No active Vectis brief invokes `specify-vectis`.
- Build and merge briefs distinguish WASI tool failures from host prerequisite failures.
- Merge journaling still records `name`, `passed`, and failure snippets for host steps.

**Parallelism:** Depends on stable command surfaces from Changes `03` and `04`; should land after or alongside Change `08`. Can run in parallel with Changes `10`, `11`, and `12`.

## Change 10: Migrate Vectis Writer And Reviewer Skills

**Repo:** `specify`

**Purpose:** Update operator-facing generation/review skills to the RFC-16 execution model.

**Scope:**

- Replace `specify-vectis init` and `add-shell` with `vectis-scaffold` render steps.
- Replace `specify-vectis validate` with `vectis-validate`.
- Replace `specify-vectis verify` with explicit host verify commands.
- Remove text that says `specify-vectis` is the canonical standalone binary.
- Keep generated-shell writer responsibilities unchanged after scaffold/render.

**Likely files:**

- `specify/plugins/vectis/skills/core-writer/SKILL.md`
- `specify/plugins/vectis/skills/ios-writer/SKILL.md`
- `specify/plugins/vectis/skills/android-writer/SKILL.md`
- `specify/plugins/vectis/skills/android-writer/rules.md`
- `specify/plugins/vectis/skills/*-reviewer/references/*.md`
- `specify/plugins/vectis/rules/vectis.mdc`

**Acceptance criteria:**

- No current writer/reviewer skill tells the agent to install or call `specify-vectis`.
- Skills describe scaffold as render-only and host verification as explicit commands.
- Reviewer references still point to the validation behavior, not stale binary names.

**Parallelism:** Depends on stable command surfaces. Can run in parallel with Changes `09`, `11`, and `12`.

## Change 11: Migrate Template-Updater And Version Workflows

**Repo:** `specify`

**Purpose:** Move version registry and cap-matrix behavior fully into host-owned skill workflows.

**Scope:**

- Reframe `update-versions --verify` as `/vectis:template-updater` host workflow.
- Remove instructions to run `target/debug/specify-vectis update-versions`.
- Preserve known-drift diagnosis and cap-matrix intent.
- Clarify that registry queries and scratch builds are not WASI tools.

**Likely files:**

- `specify/plugins/vectis/skills/template-updater/SKILL.md`
- `specify/plugins/vectis/skills/template-updater/references/known-drift.md`
- `specify/plugins/vectis/skills/core-writer/references/examples/*.md`

**Acceptance criteria:**

- Template updater still gives a concrete host workflow.
- No instructions require a separate installed Vectis binary.
- Version pins remain skill-readable until a future RFC adds `vectis-versions`.

**Parallelism:** Depends on Change `00` compatibility decision and stable command naming. Can run in parallel with Changes `09`, `10`, and `12`.

## Change 12: Migrate Vectis Docs And Layout Inferer References

**Repo:** `specify`

**Purpose:** Clean up user-facing and reference docs so RFC-16 has a single story.

**Scope:**

- Rewrite CLI docs away from `specify-vectis`.
- Update layout inferer validation commands.
- Update artifact docs, glossary, plugin docs, and CLI architecture docs.
- Decide whether historical RFCs get a short superseded note rather than full rewrites.

**Likely files:**

- `specify/docs/reference/cli/vectis.md`
- `specify/docs/reference/cli/index.md`
- `specify/docs/SUMMARY.md`
- `specify/docs/reference/capabilities/vectis.md`
- `specify/docs/reference/plugins/vectis.md`
- `specify/docs/contributing/cli-architecture.md`
- `specify/docs/explanation/artifacts.md`
- `specify/docs/appendices/glossary.md`
- `specify/plugins/vectis/skills/image-layout-inferer/SKILL.md`
- `specify/plugins/vectis/skills/image-layout-inferer/references/layout-inferer-contract.md`
- `specify/plugins/vectis/references/layout-inferer-contract.md`

**Acceptance criteria:**

- Current docs consistently tell users to run `specify tool run vectis-validate` and `specify tool run vectis-scaffold`.
- Historical docs are either clearly archived/superseded or left untouched by explicit decision.
- `make checks` has no new failures from these edits.

**Parallelism:** Depends on stable command naming. Can run in parallel with Changes `09`, `10`, and `11`.

## Change 13: Retire Or Deprecate `specify-vectis`

**Repo:** `specify-cli`

**Purpose:** Remove the second binary once the replacement surfaces exist.

**Scope:**

- Apply the Change `00` compatibility decision: delete the binary outright because repo evidence does not show a shipped external contract.
- Remove or shrink `crates/vectis` to host-only reusable internals, if any remain.
- Remove stale release and publish steps.
- Update or delete `specify_vectis_bin.rs` tests.
- Ensure root `specify` remains capability-agnostic and does not gain `specify vectis` commands.

**Likely files:**

- `specify-cli/crates/vectis/Cargo.toml`
- `specify-cli/crates/vectis/src/bin/specify-vectis.rs`
- `specify-cli/crates/vectis/tests/specify_vectis_bin.rs`
- `specify-cli/Cargo.toml`
- `specify-cli/.github/workflows/release.yaml`
- `specify-cli/Makefile.toml`
- `specify-cli/Cargo.lock`

**Acceptance criteria:**

- Operators no longer need a second binary for Vectis.
- No deprecation wrapper remains.
- Release automation no longer attempts to publish or package the retired unpublished crate.

**Parallelism:** Depends on Changes `03`, `04`, `07`, and enough of Changes `09` through `12` that no active instructions require `specify-vectis`.

## Change 14: Final Validation And Release Readiness

**Repo:** both

**Purpose:** Prove the full RFC-16 migration is coherent end to end.

**Scope:**

- Run Rust workspace checks in `specify-cli`.
- Build both WASI components for `wasm32-wasip2`.
- Run `specify tool` integration tests with local Vectis tool declarations.
- Run documentation checks in `specify`.
- Search both repos for stale active `specify-vectis` instructions.
- Verify no active path suggests `specify vectis` top-level commands.

**Likely commands:**

```bash
cd specify-cli && cargo check --workspace
cd specify-cli && cargo test --workspace
cd specify-cli && cargo build -p vectis-validate --target wasm32-wasip2 --release
cd specify-cli && cargo build -p vectis-scaffold --target wasm32-wasip2 --release
cd specify && make checks
```

**Acceptance criteria:**

- All required checks pass, or residual failures are documented as pre-existing and unrelated.
- `rg "specify-vectis|specify vectis"` shows only archived docs, deprecation wrappers, or explicit migration notes.
- The RFC-16 implementation can be released without requiring a second installed binary.

**Parallelism:** Runs last after all implementation and documentation changes.

## Suggested Parallel Execution Batches

### Batch A: Foundations

Run first:

- Change `00`

Then run in parallel:

- Change `01`
- Change `02`

### Batch B: WASI Tool Extraction

After Change `02`, run in parallel:

- Change `03`
- Change `04`

Then run:

- Change `05`
- Change `06`
- Change `07`

Change `05` can overlap with Change `06` once `vectis-scaffold` command behavior is stable.

### Batch C: Specify Repo Migration

After Change `07` proves local tools through `specify tool run`, run:

- Change `08`

Then run in parallel:

- Change `09`
- Change `10`
- Change `11`
- Change `12`

### Batch D: Removal And Final Sweep

After active docs and skills no longer require `specify-vectis`, run:

- Change `13`
- Change `14`

## Notes For Subagents

- Keep each change narrowly scoped. Do not combine Rust extraction with docs migration unless the change explicitly says to.
- Preserve existing behavior before deleting old surfaces. Use golden tests for validation envelopes and scaffold output parity.
- Treat host commands as evidence-producing skill steps, not hidden tool behavior.
- Do not add Vectis-specific commands to the main `specify` CLI.
- Do not add native fallback runners to `tools.yaml`.
- Do not grant WASI tools write access to Specify lifecycle state.
