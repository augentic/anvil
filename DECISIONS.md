# Decisions

Standing architectural decisions for the `specify` CLI. Read before changing error layering, exit codes, atomic writes, or the YAML library.

Each entry records the decision, why it was taken, and the consequences a change must reckon with — not how the feature works today. Current behavior lives in [`docs/standards/workflow.md`](./docs/standards/workflow.md) (the workflow contract), [`docs/standards/architecture.md`](./docs/standards/architecture.md) (workspace shape), and module-level rustdoc; entries here point at those rather than restating them.

## Error layering

`error` is the dependency leaf of the workspace. It depends only on `thiserror` and `serde-saphyr`; every other workspace crate may depend on it, and it depends on none of them. The leaf stays free of rich domain payloads: `Error::Validation { code, detail }` is payload-free (see [§"Drained `Error::Validation` and the `Diagnostic` substrate"](#drained-errorvalidation-and-the-diagnostic-substrate)) — the top-level wire `error` is the carried `code` discriminant, and rendered findings travel on stdout as a `DiagnosticReport`, not inside the error.

## Exit codes

The binary commits to a four-slot exit-code table. `Exit::from(&Error)` in `crates/cli/src/output.rs` is the single source of truth; every dispatcher routes its error through it. `Exit::Code(u8)` is reserved for the guest leg's exit-code passthrough.

| Code | Name                     | When                                                                                                   |
| ---- | ------------------------ | ------------------------------------------------------------------------------------------------------ |
| 0    | `EXIT_SUCCESS`           | Command succeeded.                                                                                     |
| 1    | `EXIT_GENERIC_FAILURE`   | Any `Error` variant not listed below (I/O, YAML, schema, merge, ...).                                  |
| 2    | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`.                                           |
| 3    | `EXIT_VERSION_TOO_OLD`   | `Error::CliTooOld` / `Error::AdapterCliTooOld` — a floor is newer than the running binary.             |

`Exit::ArgumentError` and `Exit::ValidationFailed` are distinct Rust variants that share code `2`, keeping the wire contract four-slot while preserving dispatcher-side clarity — anything actionable by the operator is in the JSON envelope's `code` discriminant, and per-finding detail is on the stdout `DiagnosticReport`. A pin **newer** than the binary is exit `3` (the binary must catch up); a pin *older* than the binary loads fine — pre-1.0 there are no compatibility shims and no migration framework (see [§"Bootstrap and upgrade lifecycle"](#bootstrap-and-upgrade-lifecycle)). The validate surfaces (`slice validate`, plan validation) return `2` only on a blocking finding (`kind: violation` AND `severity ∈ {critical, important}`); there is no lint verb on the CLI (see [§"Framework checks are cargo tests"](#framework-checks-are-cargo-tests)).

## Atomic writes

Use `yaml_write` (in `crates/artifacts/src/atomic.rs`) for any file a concurrent reader may observe mid-write: `plan.yaml`, `metadata.yaml`, and the registry. It serialises to `NamedTempFile::new_in(parent)` and `persist`-renames over the target so readers either see the prior bytes or the new bytes. Plain `fs::write` is reserved for files no other process reads concurrently with the writer.

## YAML library

The workspace uses `serde-saphyr` (pinned to a `0.0.x` release) for both deserialization and serialization. It is pure-Rust, panic-free, and actively maintained, in contrast to `serde_yaml` (deprecated) and `serde_yaml_ng` (community fork carrying the same debt). Saphyr omits a `Value` DOM, so code that needs untyped YAML access deserializes into `serde_json::Value`. Its separate deser/ser error types ride directly on `error::Error::YamlDe` and `Error::YamlSer` (both `#[error(transparent)]` `#[from]` variants), so `?` on a raw `serde_saphyr` result still propagates and the kebab discriminant on the wire stays `yaml` for either side.

## Diag-first error policy

`Error::Diag { code, detail }` is the default for new diagnostics. A typed `Error::*` variant exists only when (a) a test or skill destructures the variant's payload, (b) the variant routes to a non-default `Exit` slot, or (c) three or more call sites share the exact shape. The kebab `code` is the wire contract; the Rust variant is for callers that pattern-match.

## Hint colocation

Long-form recovery hints live on `Error::hint(&self) -> Option<&'static str>`, not on the renderer. Adding a new hint means extending `Error::hint`, not the renderer. Hints for collapsed `Diag` codes are looked up by the kebab `code` so a `Diag` site without a typed variant can still surface guidance.

## Wire compatibility

The CLI's JSON output is a flat envelope: every successful body is the typed `*Body` rendered directly, every failure body is `ErrorBody`. Skills grep on the `error` / `code` discriminants; tests assert on them. There is no top-level `envelope-version` integer — re-introduce one only if a breaking shape change ships and consumers need a version stamp to refuse output they cannot parse.

The kebab-case `code` discriminant on `Error::*` variants is the public contract: renaming or removing one is breaking; adding a fresh one is additive. CLI **input** flags are a peer wire surface under the same rules — adding an optional flag is additive, removing or renaming a flag is breaking. One non-additive input change has shipped: `specify init` enforces the `<adapter>`-with-`--workspace` conflict through clap, and a *missing* adapter is the typed `init-requires-adapter-or-workspace` (`Error::Validation`, exit 2) raised in `crates/workflow/src/init/`.

## Shell completions

`specify completions <shell>` writes a clap-generated completion script to stdout for any shell `clap_complete::Shell` covers. The script is a pure function of the live clap surface, so verb additions/removals are auto-tracked without extra plumbing.

## Crate layout

The crate graph (leaf → root, with per-crate roles) is pinned in [AGENTS.md §"Crate graph"](./AGENTS.md) and [architecture.md §"Workspace layout"](./docs/standards/architecture.md#workspace-layout); this entry records why the shape is what it is.

SHA-256 digest encoding lives in `schema::digest` (the `schema` leaf), so sibling crates share one digest implementation without depending on anything wasmtime-bearing. The neutral diagnostic substrate lives beside it at `schema::diagnostics` (see [§"Standards chain moved to the adapters; `diagnostics` merged into `schema`"](#standards-chain-moved-to-the-adapters-diagnostics-merged-into-schema)). `artifacts` exists so the artifact types and parsers sit on a lifecycle-free leaf; it also holds the artifact validation rule registry (`artifacts::validate`) and depends on neither `workflow` nor anything named lint, so a validation rule physically cannot reach a slice transition or plan stamp. The init-time `AGENTS.md` context-fence generation lives in `workflow::agents` (its native init consumer retired with the provisioning front; it awaits the in-guest init — see [§"One `specify` binary"](#one-specify-binary)). The dev-only `testkit` crate carries the shared scripted `Model` mock, reached exclusively through `[dev-dependencies]` (justified under [§"Integration tests: auto-discovered per-area binaries"](#integration-tests-auto-discovered-per-area-binaries)).

There is no lint engine and no `Check` substrate in any shipped crate: framework checks over the prose surfaces are plain cargo tests at `tests/framework/` — see [§"Framework checks are cargo tests"](#framework-checks-are-cargo-tests). Engineering-standards rules ship inside the target adapters (see [§"Standards chain moved to the adapters; `diagnostics` merged into `schema`"](#standards-chain-moved-to-the-adapters-diagnostics-merged-into-schema)).

### New workspace crates

New functionality lands in an existing module by default. A new workspace crate requires a paragraph in this file justifying why an existing module cannot host the code, and what dependency-direction invariant the new crate enforces (which leaf-→-root edge it preserves, and which existing crate would have grown a cycle). A new crate that does not strengthen the dependency direction is overhead; refactor within an existing module instead. Adapter-specific logic never lands as a workspace crate — it lands in the adapter's WASI carve-out.

## Integration tests: auto-discovered per-area binaries

**Decision (2026-07, supersedes the 2026-06 `tests/it.rs` consolidation).** Each `tests/<area>.rs` file is its own auto-discovered test binary — no `autotests = false`, no explicit `[[test]]` declarations, no `#[path]` submodule hub. This is the layout `augentic/specify-adapters` already uses, and converging the two repos on one test shape outweighs the link-cost saving the per-crate `it` binary bought: the consolidated hub cost per-area `cargo test --test <area>` selectivity, forced shared-helper access through a single crate root, and made every area file a special case (hub-level `cfg` gates, `#[path]` includes) that agents and contributors had to re-learn per repo.

Shared helpers use the dir form `tests/<helper>/mod.rs` (invisible to auto-discovery; the sole `mod.rs` exception blessed in coding-standards) and each consuming binary declares them with `mod <helper>;`. Cross-package test support — the scripted `Model` mock consumed by both `workflow` and `harness/runtime` — lives in the dev-only `testkit` workspace crate rather than duplicated `mod` includes; see [§"New workspace crates"](#new-workspace-crates) below for the justification. The workspace-shared `GIT_ENV` / `run_git` / `copy_dir` trio stays single-sourced at the repo-root `tests/fs_git.rs` via a `#[path]` include.

Goldens refresh through `REGENERATE_GOLDENS=1 cargo nextest run -p <crate>`. The root `specify-cli` crate keeps `autotests = false` with an explicit `[[test]]` target (`framework`): its `tests/` tree carries shared fixture files (`tests/fs_git.rs`) that must not compile as binaries.

The `testkit` crate (per the new-workspace-crate bar): test-support code shared across two packages cannot live in either package's `tests/` tree (test trees are not importable across packages), and homing it in `workflow`'s public API would ship test code in the library. `testkit` is `publish = false`, depends only on `omnia-guest`, and is reachable exclusively through `[dev-dependencies]`, so no shipped crate's dependency graph changes.

## Unit tests: minimized, coverage-gated

**Decision (2026-06).** The unit layer is kept deliberately thin. Integration (binary + crate `tests/`) owns every CLI-reachable behavior; a `#[cfg(test)]` unit test survives only when it covers a branch genuinely unreachable through the CLI, or is the cheap home for a dense parse/projection edge matrix whose case-per-cell integration port would explode the test-binary budget. It composes with the binary-layout decision above (auto-discovered per-area binaries). Two mechanics make aggressive reduction safe:

- **Collapse, don't enumerate.** A unit test that walks a closed `(input → code)` set is one table-driven `#[test]` with a block per case, not one `#[test]` per case.
- **Coverage is the brake.** `cargo llvm-cov nextest -p <crate> --summary-only` runs before and after a reduction; a `TOTAL` drop on still-live lines blocks the deletion until an integration assertion backfills it. `nextest` (not bare `cargo test`) is mandatory — its per-test process isolation is what lets the CWD/env-mutating suites pass, and it is the runner CI uses.

## Framework checks are cargo tests

**Decision (2026-07).** Framework invariants over this repo's prose and manifest surfaces are enforced as plain cargo tests at `tests/framework/` (`links`, `skills`, `scenarios`, `prose` modules): policy as module constants, failures as test failures. Why not a lint engine: with the codebase Rust-centric and the rules unreleased, a generic rule engine is a second programming model maintained solely to check this repo's own prose; a cargo test expresses the same predicate directly, is registered by compiling, and cannot silently go dead. Rule-*shape* validation lives in `augentic/specify-adapters` — the repo that authors the rules — as a cargo test. Judgment-prose link resolution is enforced at compile time: `crates/workflow/build.rs` inlines and link-checks the embedded corpus, so a dangling reference fails the build.

## Project-scope `tools[]`

> **Retired (2026-07, YAGNI).** The parse-clean `tools[]` DTOs and the `tools:` field on `project.yaml` are deleted. Nothing ever resolved or ran a declared tool after `specify lint project` retired, so the declaration shape carried no behaviour. If a project-scope tool surface earns its way back, it returns through a fresh RFC.

## Time-crate policy

UTC-only domain time on `jiff::Timestamp`; all persisted stamps route through `error::serde_rfc3339` so the wire shape stays `%Y-%m-%dT%H:%M:%SZ` byte-for-byte.

## Guest-to-guest transport: wRPC behind the dispatch seam

**Decision (2026-06).** All guest-to-guest dispatch — every host-mediated `source` / `target` call, local or remote — is carried over [wRPC](https://github.com/bytecodealliance/wrpc), the Bytecode Alliance's WIT-native, transport-agnostic component RPC. Only the transport varies by deployment: in-process or Unix-domain socket on one node, NATS or QUIC across a cluster. The project exists to scale from desktop to cloud without changing guest or host code; a single dispatch path whose transport is config delivers exactly that. The earlier dual-path design (native in-process locally, wRPC only across nodes) is superseded: it special-cased the local hot path and duplicated dispatch for a serialization cost that is negligible against `build` / `merge`.

- **The seam, not the transport, is the contract.** The per-axis `source` / `target` imports are the durable boundary; the `specify:adapter` package names no transport. A native in-process fast-path can return behind the same seam if profiling ever demands it, without touching guests or the contract.
- **Resources do not cross — uniformly.** wRPC carries WIT resources only as opaque handles, so a live `wasi:filesystem` descriptor is never transported: `build` / `merge` ship the content-addressed `revision` / `changeset` and the serving node re-materializes its own tree; `survey` / `extract` / `guidance` exchange plain records.
- **Pre-1.0 dependency on the universal hot path.** wRPC is pre-1.0 (currently the 0.7.x line via the `[patch.crates-io]` git pins for `wrpc-transport` / `wrpc-wasmtime`); it sits on every call, so the seam (native fast-path in reserve) is the mitigation. Pin wRPC and track its Wasmtime pin against the runtime's; prefer `wrpc-transport` + `wit-bindgen-wrpc` server bindings wired to the existing instance-per-call registry over `wrpc-runtime-wasmtime` (which would cede instantiation and add a second Wasmtime pin).
- **Async everywhere; Wasmtime 46 floor.** wRPC is async (Tokio), aligning with component-model-async / WASI 0.3, which landed default-on in Wasmtime 46.0.0 — the runtime floor for this dispatch path (the lockfile currently resolves 46.0.1). Instance-per-call is unchanged: recursive reentrance still traps, so callbacks land in fresh instances; only sibling reentrance is allowed.

## Plan execute loop

**Decision (2026-07).** The drained execute loop runs in the workflow guest as `specify plan execute` → `workflow::orchestrate::execute`: `plan status` projection → `plan next` claim → refine / build / merge until `drained` or a typed stop. Standing choices:

- **No loop-specific journal events.** The loop composes the per-phase cadence, so a journal reader cannot tell a drained execute run from the same phases driven one breakout at a time. A stop returns as a typed outcome carrying the status projection's closed `StopReason` + hint (`plan-execute-stopped`, exit 2). Phase failures leave the entry `in-progress` — merge stays the only `done` writer — so the loop is re-entry safe.
- **Guest marker.** The loop holds a create-exclusive advisory marker at `<plan-root>/.specify/guest.lock` (`OpenOptions::create_new`); a second in-guest execute fails `guest-marker-held` (exit 2). A crash that skips destructors leaves it behind and the next acquire tells the operator to delete it — no pid-liveness probe, because WASI has no process table. The marker is the only concurrency fence; the pure plan verbs run lock-free, exactly as they do for hand-edited artifacts.
- **Single-project scope.** The loop refuses workspace-routed plans up front (`plan-execute-workspace-unsupported`, exit 2). Refusing beats silently creating slices under the workspace root's own `.specify/` tree; workspace plans stay hand-driven until slot routing has an in-guest counterpart (its own RFC).
- **Elicitation to argv.** `specify plan create --intent <string>` is pure sugar for `--source intent=intent:value:<string>`, desugared before the source-map build so combining it with an explicit `intent` binding trips the existing duplicate-key gate.
- **Runtime model host.** The shipped binary binds `WasiModel: Cursor`; the parked rig's `runtime-replay` binary binds the replay backend so its deployments link without fixtures.

## Plan authoring: `plan author`

**Decision (2026-07).** The `/spec:plan` critical path is one guest orchestration, `specify plan author` → `workflow::orchestrate::author`: scaffold (same gates as `plan create`) → `survey_all` fan-out → `judgment::propose::reconcile` with the `Plan::propose_from` kernel as the check → Gate 1 prose persistence → the `plan validate` doctor sweep → exit at `pending` with the literal `specify plan transition <name> approved` hint. Standing choices:

- **A new verb, not a `plan create` extension.** `plan create` keeps its scaffold-only semantics (skills and operators depend on "create writes an empty `slices:` list and exits"); overloading it would make one spelling mean two flows. No `--auto-approve` and no `--authority-override` on the author surface — Gate 1 stamping stays operator-only, and override pre-seeding needs slice rows that don't exist until the propose leg has run (post-author `plan amend` covers it).
- **Gate 1 prose persistence is frame-deterministic, body-model.** The model authors section bodies only; the orchestrator owns every deterministic frame. `change.md` is written whole via the shared atomic writer; the `discovery.md` preamble lands through the validated writer `Discovery::set_preamble`, which rejects (`discovery-preamble-invalid`) any preamble that breaks the parse round-trip — the lead inventory rides through byte-untouched. The prose check runs *inside* the reconcile repair loop, so a missing `gate` (`plan-author-gate-missing`) re-prompts instead of failing after the projection.
- **Plan context rides the user message, not the request envelope**, so the model can author the source-inventory rows without widening the pinned `ProposalRequest` schema.
- **Journal cadence composes the verbs'.** Per-source `source.execution.agent` / `source.survey.completed` pairs, then one `plan.reconcile.completed` after the write commits — no new event ids. A workspace plan root refuses up front (`plan-author-workspace-unsupported`, exit 2), mirroring the execute loop.

## `slice refine` breakout

**Decision (2026-07).** `specify slice refine <name>` wraps `orchestrate::refine` for one named slice outside the execute loop. The breakout acts on the named slice directly against a `pending` or `in-progress` plan entry and never writes per-entry status — `plan next` stays the only `in-progress` writer, so a breakout refine leaves the entry exactly where it found it; the only guard is terminal (`slice-refine-entry-done`, hint names `plan transition <entry> --undo`). Target resolution is caller-free: the slice's own `metadata.yaml` when the slice directory exists, else the bound project's topology via `resolve_target`.

## Release identity

**Decision (2026-07).** The `specify` package is `publish = false`: it cannot go to crates.io while the omnia stack rides `[patch.crates-io]` path/git pins (patches do not propagate to dependents, so a published crate would be unbuildable), and the binary's distribution channel was never crates.io anyway. The release identity is the `release-binaries.yaml` archive workflow — platform archives named `specify-<target>.tar.gz` attached to the GitHub release, which `specify upgrade` and the install scripts consume. `docs/release.md` documents this as the only binary channel.

- **No crate-name/`[[bin]]` collision.** The root package owns the one `[[bin]] specify`; the parked `harness/runtime` crate's only binary target is the differently-named `runtime-replay` (dev/test surface, never released).
- **Re-pin discipline.** The `[patch.crates-io]` sibling-path pins must point at pushed git revs for reproducible CI/release builds; `publish = false` stays until the omnia crates are published and the patches drop.

## Deterministic guest merge

**Decision (2026-07).** The guest merge (`orchestrate::merge`) is **deterministic-only**: the `TargetSeam` trait carries `guidance` + `build` and deliberately omits the WIT contract's `merge` operation, so no target merge brief is ever dispatched from the guest. The delta merge, Decision Record promotion, `merged` transition, archive move, outcome-ledger append, and per-entry `done` stamp are the same `merge::slice::commit` code driven in-guest. The workspace-clone git commit leg is skipped with an explicit `slice.merge.commit-skipped` journal event — the guest owns no git surface, and lifecycle authority is `.specify/` state, so `done` still stamps and `slice.archive.created` lands with `merge-sha` absent (the leg returns with the git capability — see [§"Workspace git transport from the guest"](#workspace-git-transport-from-the-guest)). The WIT `merge` operation stays in the contract as the forward hook for a judgment-backed merge; adding a seam method later is additive.

## Workflow judgment prose is pasted, not shelved

**Decision (2026-07).** The workflow guest's prompt corpus — the propose and synthesize prompt bodies plus the synthesis playbook references — is embedded at build time (`crates/workflow/build.rs` copies and link-checks the corpus into `OUT_DIR`; `judgment/prose.rs` `include_str!`s it) and **pasted into the system prompt**, not served over MCP references. The corpus is about 50 KB and every section is load-bearing on every synthesis call, so lazy fetching buys nothing and an MCP route on the workflow guest would exist only for this. The adapters keep the opposite posture (MCP references up to 700 KB, fetched lazily) — a size-and-locality call, not a new architecture. Markdown stays the authoring source of truth; the dependency direction forbids importing the adapters' prose registry, so the crate embeds the repository files directly.

## The judgment capability is the upstream `omnia_guest::Model`

**Decision (2026-07).** The upstream `omnia_guest::Model` is the single judgment-capable capability — typed `Error`, the full `omnia:model/completion` request mirror, the plain `lend_workspace: bool` flag, and the zero-sized `WasiModel` implementor on `wasm32`. Every judgment leg takes a `P: omnia_guest::Model` bound; neither consumer repo carries a local mirror of the trait.

- **No test code in mainline.** Omnia ships no `MockModel`; each consumer repo keeps a dev-only scripted mock in its own test surface (here the `crates/testkit` dev-only crate; `specify-adapters` keeps its own testkit). The shared upstream trait is the conformance guarantee; a small local mock beats a cross-repo test-support dependency.
- **`workflow` depends on `omnia-guest` unconditionally.** The heavier native graph is the accepted cost of one capability instead of three; slimming omnia-guest's native graph is deferred upstream. The change rides under omnia **0.35.0** through the `[patch.crates-io]` path overrides in both consumer repos.

## `cli`: the wasm-clean CLI surface

**Decision (2026-07).** The CLI grammar, output envelopes, exit-code contract, project `Ctx`, and every pure workflow verb handler live in the wasm-clean `cli` crate, consumed by the specify guest shim (and natively by the always-on `crates/cli/tests/` seam gate and the parked harness rig).

- **One grammar, one operational dispatcher.** The full clap tree — provisioning verbs included — lives here, so `--help`, completions, and usage-error exits are stable regardless of where a verb runs. The guest's `guest::route` runs pure verbs in-process, returns the orchestrator verbs as an `Orchestration` descriptor, and refuses the provisioning verbs that have no guest implementation yet (see [§"One `specify` binary"](#one-specify-binary)).
- **The sync/async seam sits at the shim.** `cli` is fully synchronous and WIT-free; the guest shim owns the async surface, where the `Model + SourceSeam + TargetSeam` provider lives. This keeps the cli crate natively testable with no executor and keeps every wasm specific in the shim.
- **New-crate justification (per §"New workspace crates"):** the handlers could not stay in the root binary (the guest cannot depend on a bin crate), and they could not move into `workflow` (which is deliberately clap-free). The crate enforces the `workflow → cli → guest-shim` direction.

## Source and target adapter role names

The output-role domain types are spelled `Target*` (`Target`, `Slice.target`, the typed discriminants, every fixture, JSON envelope, and call site). Adapter resolution is the axis-aware module `crates/workflow/src/adapter/` (`SourceAdapter` / `TargetAdapter` / `AdapterRef`). The slice-metadata wire uses `TargetOperation { Build, Guidance, Merge }`. Per workflow §"Note to the implementing agent", touching any of these symbols requires an `rg` sweep across both the Rust workspace and the surrounding prose in the same PR.

## Adapter loader axis routing

`SourceAdapter::resolve(adapter_ref, project_dir)` / `TargetAdapter::resolve(adapter_ref, project_dir)` resolve a versioned identity to exactly one `.wasm` component. The decisions:

- **Resolution is project-local, plus the global store.** A pinned `(name, version)` resolves the global single-file store entry first; a bare name resolves the project component cache (`<project-cache>/components/<name>.wasm`) then the sibling/in-repo development release build (`target/wasm32-wasip2/release/<name>.wasm`). There is no environment-variable fallback to an out-of-tree framework checkout. A miss on every probe is `adapter-not-found`, naming every probed path and the remedies.
- **The axis is typed, not path-encoded.** Store entries are keyed `name@version` alone; a binding names the axis, and describe dispatch routes by the `<axis>:<name>` adapter id, so a component bound on the wrong axis fails at the dispatch seam rather than resolving.

### First-party `<adapter>` shorthand at init

`specify init <adapter>` accepts a local `.wasm` path (mirrored into the project component cache, name from the file stem), a package reference (`specify:<name>@<semver>`), or the first-party shorthand: `omnia@1.0.0` is package-reference sugar for `specify:omnia@1.0.0`; bare `omnia` is the development shorthand resolving the sibling/in-repo release build. A package reference is an immutable, content-addressed registry locator with a mandatory exact-SemVer pin and no branch/tag defaulting — a missing or non-SemVer version raises `adapter-package-ref-version-required`, and a package reference whose store entry is absent is `adapter-package-not-installed`. GitHub URLs are refused with the typed `adapter-github-uri-unsupported`: adapters distribute as published components, and a source checkout yields no usable artifact.

### Adapter identity: semver version + `AdapterRef`

An adapter's identity is `name@<semver>`, threaded through the value type `AdapterRef { name, version: Option<semver::Version> }`. Synthesized target refs render `name@<semver>` and `TargetRef::parse` requires that form. Non-identity metadata — the `specify` host-CLI compatibility floor, a target's `inputs[]` and `platforms` — comes from the component's own `describe` export: an unparseable floor is `adapter-floor-malformed`; a floor newer than the running binary raises `adapter-cli-too-old` on the exit-3 path — the adapter-granularity analog of the `project.yaml.specify` floor. Exact floor only (no ranges, matching the version-pin posture); absent means no floor. Third-party namespacing beyond `specify:`, a per-adapter release index, a semver-*range* floor policy, and a cross-version compatibility matrix stay deferred to RM-21.

## Adapter store

Pinned identities resolve from the global content-addressed single-file store. A store entry is one file — `<store-root>/<name>@<version>.wasm` — with the component-byte SHA-256 recorded in the sibling `<name>@<version>.meta`. The store root resolves `$SPECIFY_ADAPTER_STORE` (absolute override — the relocation lever for sandboxes and tests), else `$HOME/.specify/adapters`: an install store in Specify's per-user home, not an evictable OS cache. The layout helpers live in `schema`'s `cache` module.

- **Verify-on-read.** Resolve recomputes the component digest against the recorded sidecar, raising `adapter-digest-mismatch` on drift. An entry with no recorded `.meta` sidecar **fails open**: verify-on-read rejects a recorded-then-drifted digest, never an absent record.
- **Resolve-in-place.** Store entries are read-only and resolved in place, never mirrored per project.
- **Resolve-only until a fetch leg lands.** Nothing in the tree populates the store today; hydration is store-probe-only until an in-guest fetch leg lands (see [§"Adapter hydration, the committed lock, and the generated deployment"](#adapter-hydration-the-committed-lock-and-the-generated-deployment)).

## One component, no manifest

The adapter artifact is a single WebAssembly component: no manifest file, no packed prose tree, no committed per-adapter `guest.wasm`. Identity is the published package (`specify:<name>@<semver>`; the `augentic:` namespace is reserved and carries no first-party routing); metadata is the component's own `describe` answer. Pre-1.0 hard cut: no aliases — pinned projects re-init. The decisions:

- **Describe-driven resolve with a resolve-time cache.** Non-identity metadata comes from the deterministic `describe` export on the component's axis interface (a plain WIT `func` — deterministic metadata cannot fail, so no `result` wrapper). `workflow` stays wasmtime-free behind a process-global `DescribeRunner` function-pointer seam (`adapter/describe.rs`); the guest shim registers its runner at startup, routing each dispatch through the deployment's WIT `source` / `target` imports by adapter id. The answer persists as a `<component>.describe.json` sidecar keyed by the component file's SHA-256, so a store entry is described once per install and a development build once per rebuild. An unregistered runner is the typed `adapter-describe-unavailable`.
- **Axis by binding, mismatch at the dispatch seam.** The store carries no axis segment and there is no manifest `axis` field: the routed adapter id is `<axis>:<name>`, and a component deployed on the wrong axis exports no interface for that id — the dispatch fails at the Omnia seam.
- **wasm-pkg transport.** Publish is `wkg publish` in the adapters repo; consumers pull the same identity over wasm-pkg/OCI (see [§"Publishing and distribution: one transport, idempotent legs"](#publishing-and-distribution-one-transport-idempotent-legs)).
- **No committed artifacts.** No adapter tree commits a built component. Development resolution probes the release build under the project then the sibling checkout; a development component carries no package identity and resolves as the honest `0.0.0` placeholder in topology projections and envelopes.

## Composed-test fixtures and adapter acquisition

**Decision (2026-07).** Refines the no-committed-artifacts bullet of [§"One component, no manifest"](#one-component-no-manifest) for the engine repo's own composed-deployment tests.

- **Echo guests are examples, not shipped crates.** The `echo-source` / `echo-target` skeleton guests live as `cdylib` examples in the `fixtures` package (`harness/fixtures/echo/{source,target}.rs` — omnia's `examples/` pattern), built for wasm32-wasip2 by `cargo make build-guests` from inside `harness/`. They have no publishable identity, so they earn no product surface beyond the parked rig. They are deliberately *not* replaced by real adapters: the echo guests compile against this repo's own `wit/`, so a contract revision and its seam tests land in one engine PR, and their model-free hardcoded operations keep the seam tests decoupled from adapter prose and model behavior.
- **Real adapter components resolve store-first, sibling fallback, locate-only.** The composed tests resolve real first-party components through the global adapter store (verify-on-read), falling back to the sibling checkout's release build for development iteration. Tests never fetch or build: population is the explicit `cargo make fetch-adapters` task (idempotent skip-if-present `wkg get` at the committed pin). The pin's single source of truth is `harness/runtime/tests/adapters.pin`.
- **The repo-root `omnia.toml` is a dev convenience only.** No test consumes it; the composed tests render their own manifests over resolved components. It remains checked in for running guest verbs from the repo root (omnia.toml-wins).

## The composed-deployment rig is parked at `harness/`

**Decision (2026-07).** The composed-deployment test surface — the `runtime` crate (`runtime-replay` + the composed integration tests) and the echo `fixtures` package — lives under the top-level `harness/` directory as **workspace members excluded from `default-members`**. The rig is inert: no root invocation (`cargo make ci` / `test` / `lint` / `doc` / `vet` / `deny` / `fmt`) builds, lints, tests, or audits it, and no CI job runs it. It runs only when invoked manually from inside `harness/` — typically when the WIT contract, the guest shim, or the omnia pin changes. See `harness/README.md`.

- **Rationale.** The always-on value of the rig was re-proving plumbing wasmtime and omnia already own (argv delivery, exit passthrough) at the cost of a wasm guest build on every `cargo make test`. The deterministic cli contract the rig exercised is covered natively and cheaply by `crates/cli/tests/`, which is the always-on seam gate.
- **Why parked, not deleted.** Link dispatch to real adapter components, journal/plan writes over the shared `"."` preopen, and the per-adapter `/mcp/<name>` shelves have no native equivalent; the rig remains the only proof of those seams.
- **Accepted cost.** Outside every root gate, the rig can drift compile-wise against the engine crates; `cargo make lint` inside `harness/` is the first step of any revival.

## Plan lifecycle: two stored states

`plan.yaml.lifecycle` is `pending | approved` — no plan-level `in-progress` or `drained` ships in v1; "drained" is computed at read time as "every entry is `done`", not stored. Per-entry status is the closed `pending | in-progress | done` with split writer ownership (see [§"Lifecycle write-ownership"](#lifecycle-write-ownership)). `specify plan transition <plan-name> approved` is Gate 1 and operator-only: the CLI deliberately does not gate it, the `--help` text documents the rule, and `/spec:plan` skill bodies MUST NOT call it.

Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs — exactly `Done → InProgress` and `InProgress → Pending` per call, one `plan.transition.undone` journal event per rung. `Status::Reopened` does not exist: an undone `done` row walks back to `in-progress` so the operator can re-run `/spec:build` and re-merge without a new state. If an upstream revert demands a redo without re-running the slice, author a fresh slice. Plan-level lifecycle has no undo path in v1. Archive is a filesystem operation, not a lifecycle state: `specify plan archive` moves the plan artifacts into `.specify/archive/plans/` and the lifecycle stamp inside the archived file stays `approved` — the on-disk location is the signal.

## `SliceSourceBinding`: bare shorthand plus structured form

`plan.yaml.slices[].sources` is one in-memory struct (`{ source_key, lead_id: Option }`) with a custom `Deserialize` accepting two wire shapes and a `Serialize` emitting whichever shape produced the value: the bare string (lead falls back to the owning slice's name — the one-source-per-slice degenerate case) and the structured `{ source, lead }` (required whenever key and lead differ). Collapsing the variants into one struct means every consumer goes through the same `source_key()` / `lead_id()` accessors instead of `match`-ing a discriminator — the shorthand stays a pure parser concern.

## `Divergence` enum

`plan.yaml.slices[].divergence` is the closed enum `none | likely | accepted | rejected` (kebab-case wire; `none` is the elided default). `specify plan amend --divergence` only accepts `accepted | rejected` — `likely` is reserved for the propose sub-step of `/spec:plan`. The sibling `plan.yaml.slices[].disagreements[]` (`{ field, values: [{ source, value }] }`) is authored by the propose agent and carried onto the plan entry by the reconcile kernel. The CLI never decides materiality; `Plan::validate` only checks structural consistency and surfaces it as **advisory** (`Suggestion`) findings (`slice-divergence-unrecorded`, `slice-divergence-orphan-values`) — deliberately non-blocking, because `divergence` is operator-settable standalone via `plan amend --divergence` and a consistency finding may never break that contract-locked write.

## Plan per-slice authority overrides

`plan.yaml.slices[]` carries an optional `authority-override` map keyed by claim kind, valued by source key. Keys come from the closed claim-kind enum; values MUST be source keys present in the slice's own `sources[]` list — orphans are rejected by `specify slice validate` (`slice-authority-override-orphan-source`). The map is scoped to one slice; plan-wide and project-wide overrides are out of scope.

## Extraction is agent-only — no cache, no fingerprints

Source extraction supports exactly one execution mode: `agent`, re-running the prompt every time — there is no extraction-result cache and no fingerprinting. Agent outputs are non-deterministic, so no run could ever be served from a cache, and deterministic-extraction machinery would only constrain changes to the live agent path. If a deterministic source ever lands, add caching behind a fresh decision — the journal taxonomy is the seam to widen.

## Journal event names

The journal is a closed event taxonomy with a single writer per event. Events persist as newline-delimited JSON at `<project_dir>/.specify/journal.jsonl`; wire ids are dotted kebab-case, the Rust `EventKind` variants are `snake_case`, and the two are joined by `#[serde(rename = "…")]`. The authoritative id set is the `EventKind` enum in `crates/workflow/src/journal/event.rs` (`WIRE_EVENT_IDS`) — this file deliberately does not duplicate the table. The standing semantic choices:

- **Events fire only on real effects.** `plan.entry.advanced` fires only when an entry actually moves `pending → in-progress`; a no-op query emits nothing. `plan.reconcile.completed` fires once, after the `plan.yaml` write commits. `slice.merge.*` fire on the merge validator's outcome, not on a report. `workspace.push.completed` fires only on a non-dry-run with no failed project.
- **Orchestrator-owned completion events.** The guest orchestrations emit their own `slice.extract.completed` / `source.survey.completed` / `slice.synthesize.*` / `slice.build.*`; skills never emit them via `specify journal emit`.
- **Self-reported actor.** `plan.transition.approved` carries the closed `actor` enum (`operator | agent`, default `operator`) via `plan transition --actor` — grading evidence for eval probes, not enforcement; absent on pre-actor journal lines and deserialised as `operator`.
- **The durable ledger entry** is `slice.archive.created` (payload: slice, touched-specs, outcome summary, optional merge SHA) — see [§"History via git plus an outcome ledger"](#history-via-git-plus-an-outcome-ledger).

### `specify journal emit` — guarded front door

Deterministic commands emit their own events. Agent-orchestrated phases that have no deterministic emit command write through `specify journal emit <event-id> [--payload <json>]`. The verb mints **no event kinds of its own** — it is a guarded front door onto the same closed `EventKind` taxonomy, preserving "one closed taxonomy, one writer". The closed enum is itself the per-kind payload schema (no parallel JSON-schema registry): the handler reassembles the adjacently-tagged `{ event, payload }` shape and deserialises it into `EventKind`. An unknown tag fails `journal-emit-unknown-event`; a missing required field fails `journal-emit-payload-schema` (both exit 2). The CLI — never the agent — stamps the UTC `timestamp` and appends exactly one line.

## Lifecycle write-ownership

Per-entry status writes route to exactly one CLI verb. Skill bodies never write status by hand; the CLI is the single source of truth for each transition:

| State                     | Writer                                    | Trigger                                                                                        |
| ------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `pending` (per-entry)     | `specify plan add` / `specify plan amend` | Operator (or `/spec:plan`) authors / edits a slice row.                                        |
| `in-progress` (per-entry) | `specify plan next`                       | Sole writer; the plan execute loop calls it once per slice.                                    |
| `done` (per-entry)        | `specify slice merge`                     | The merge orchestration stamps it through the shared transition kernel on a successful merge.  |
| `pending` (plan-level)    | `specify plan create`                     | `/spec:plan` scaffolds the plan in `pending`.                                                  |
| `approved` (plan-level)   | `specify plan transition <plan> approved` | Operator-only (Gate 1). The CLI is ungated; `/spec:plan` MUST NOT call this verb.              |

The plan-level `approved` row is the lightest-touch shape the workflow allows: a wholly operator-driven stamp with no CLI-side authentication. Skills that drift from this contract get caught at review time.

## Plan source bindings

The on-disk shape of `plan.yaml.sources.<key>` is the structured `{ adapter, path?, value? }` object. Every binding carries an explicit kebab-case `adapter` and exactly one of `path` / `value`, enforced in both the JSON Schema and the Rust loader. The `--source` flag grammar mirrors the wire shape:

| Form                                       | Materialises as                                                 |
| ------------------------------------------ | --------------------------------------------------------------- |
| `--source <key>=<adapter>:<path>`          | `SourceBinding { adapter, path: Some(<path>), value: None }`    |
| `--source <key>=<adapter>:value:<literal>` | `SourceBinding { adapter, path: None, value: Some(<literal>) }` |

The adapter is the substring up to the first `:` after `=`; the binding payload is everything after it, so URLs containing `:` round-trip through the path form unchanged, and the `value:` sentinel switches the parser to literal mode. No shorthand exists for "the adapter name equals the key". Source keys are plan-scoped; each key maps to exactly one binding, but slices may reference the same key with different leads. An optional `version` pin on the binding carries the same `Option<semver::Version>` as `AdapterRef`.

## Adapter name uniqueness

Adapter names are unique across axes — a name appears under `sources/<name>/` xor `targets/<name>/` in `augentic/specify-adapters`, never both. The store carries no axis segment, so a colliding name would make a binding's axis ambiguous; the `<axis>:<name>` adapter-id routing at the describe/dispatch seam is the enforcement point.

## Target platform capability and init validation

Target adapters may declare an optional `platforms` capability (`{ required, allowed, default }`) via their `describe` answer. When `required` is true, `specify init` demands `--platforms <csv>` and enforces three validation rules (all exit 2): `project-platforms-required`, `project-platforms-must-include-core`, and `project-platforms-not-allowed`; the same rules re-fire as backstops at `TopologyProject::resolve`. The closed `Platform` enum (`Core | Ios | Android | Web | Desktop`, kebab on the wire) lives in `crates/workflow/src/platform.rs`; `platforms` rides the same topology projection rails as `target`.

Platform shell bootstrap is **not** a CLI concern. The reconcile kernel performs no filesystem detection and inserts no bootstrap slices: `project.yaml.platforms` is the sole authority for platform intent, and standing up an absent shell tree is the bound target adapter's build-time responsibility — the vectis adapter owns its Crux shell scaffolding (and any launcher-icon gating) in `augentic/specify-adapters`.

## Cache layout

`.specify/` is **Specify's directory: committed config plus system-of-record** (`project.yaml`, `specs/`, `slices/`, `archive/`, `journal.jsonl`, the lock sidecars). Its lone gitignored in-tree tenant is `.specify/scratch/` — transient per-run lanes recreated empty by their owning verb, deletable at any time. Everything regenerable and machine-owned lives *outside* the working tree:

- **The cache is out-of-tree.** The machine-owned regenerables live in a per-project directory inside the user's OS cache (`$SPECIFY_PROJECT_CACHE`, else `$XDG_CACHE_HOME/specify/projects/<project-id>/`), keyed by a stable digest of the canonicalised project path (`crates/schema/src/cache.rs`). Each checkout — including each materialised workspace slot — gets its own collision-free cache that survives `git clean` and never pollutes the working tree. It hosts two root-disjoint tenants: `components/` (operator-supplied local `.wasm` components mirrored at init) and `deployment/` (the generated `omnia.toml`).
- **Workspace slots are top-level.** Materialised registry peers live at `<project>/workspace/<peer>/`, not under `.specify/`. Remote peers are `git worktree`s of a persistent out-of-tree bare mirror (`$SPECIFY_MIRROR_CACHE`, else `$XDG_CACHE_HOME/specify/mirrors/<url-id>.git`), so a peer's object store is shared across changes; local peers stay symlinks.

`ensure_gitignore_entries` therefore writes exactly two entries — `.specify/scratch/` and the top-level `workspace/`. There is no in-tree `.specify/cache/` to ignore, and no extraction-result cache (see [§"Extraction is agent-only — no cache, no fingerprints"](#extraction-is-agent-only--no-cache-no-fingerprints)).

## Target adapter suffix policy

A plan slice does not store its target adapter. `plan.yaml.slices[]` carries only a `project`; the target (`name@<semver>`) is a denormalised copy of `project → adapter` and is **resolved on demand** from the bound project's topology rather than persisted. The 1:1 `project → target` invariant is what makes the denormalisation safe. The resolved target ref remains a load-bearing wire field wherever it *does* appear (`specify plan next`, slice `metadata.yaml`, the build request). `slices[].project` is optional on disk: omitted resolves to the sole project in the topology; a multi-project workspace requires it explicitly. `propose.rs::resolve_target` is the single read-time resolver; `TargetRef` is constructed by it, never deserialised from `plan.yaml`.

## Operations typed at parse boundary

Adapter operations are typed Rust enums past the parse boundary; string operation names never survive it. `operations()` returns the axis's closed WIT operation set, with the closed `SourceOperation` / `TargetOperation` enums (`crates/workflow/src/adapter/operation.rs`) as the typed sets carried by the axis-split `SourceAdapter` / `TargetAdapter` structs. **Wire invariant:** the resolve envelopes' `operations: [...]` arrays iterate in kebab-alphabetical order; derived `Ord` on the enums is intentional because variants are declared in that order.

## Source operations

`specify source survey <source>` and `specify source extract <source> <lead> --slice <slice>` are the guest-routed source adapter operations (`orchestrate::survey` / `orchestrate::extract`); resolution is via `plan.yaml.sources.<key>`. The standing decisions:

- **Validate-before-visible.** An invalid lead set leaves `discovery.md` untouched; a failed Evidence validation leaves the slice in `refining`.
- **`discovery.md` stores raw, unmerged, per-source leads.** The runner stamps `source` from the surveyed source (attribution is CLI-owned). A re-survey is a per-source fold by `(source, lead)`, never a cross-source collapse — unification is deferred to plan time, so the same `lead` may legally appear under different source keys.

## Lead reconciliation

Agent-led cross-source lead reconciliation runs through a CLI-owned projection kernel (`Plan::propose_from` in `crates/workflow/src/change/plan/core/propose.rs`), driven by the guest `plan author` orchestration (see [§"Plan authoring: `plan author`"](#plan-authoring-plan-author)). The standing decisions:

- **Replaceable gate.** The kernel replaces slices only while the plan is replaceable (`lifecycle: pending` AND every entry `pending`). Re-propose wholesale-replaces all slices — a fresh projection, not a merge.
- **Coverage invariant, no kernel grouping.** The kernel enforces total lead coverage plus at most one lead per source per slice. Fan-out is multiple ordinary slices joined by `depends-on`. Same-source fusion is rejected on purpose: each surveyed lead is the source adapter's own sizing judgment, so merging two leads from one source would override that sizing — the operator owns same-source re-sizing at Gate 1 via `plan amend --sources`. The at-most-one-lead-per-source invariant is enforced at every writer, not just propose. The kernel validates shape only — it never auto-merges, clusters, or forbids cross-source splits.
- **Explicit slice names.** Every response slice carries an explicit kebab-case `name` written verbatim; `depends-on` resolves against those names (cycles fail).
- **Project binding.** The agent binds each slice's `project` from the request's `projects[]`; an omitted `project` auto-binds only when exactly one project exists. The target adapter is **not** written to `plan.yaml` (see [§"Target adapter suffix policy"](#target-adapter-suffix-policy)).
- **Closed validation vocabulary.** The `plan-reconcile-*` codes are `Error::Validation` outcomes (exit 2), not new enum arms.
- **Split on doubt.** Matching rides on per-source `synopsis` headlines alone. The error-cost asymmetry is stated in the propose brief: an over-merge is expensive and downstream-poisoning, an over-split is cheap and Gate-1-reversible — a weakly-supported cross-source match stays as separate slices with the candidate pairing noted in `change.md` under `## Tentative merges`.
- **Deferred (rejected).** Kernel-side token-intersection auto-merge (shared slugs are unattested), kernel-side advisory clustering, and per-lead target-axis hints. Grouping uncertainty is the agent's to express through `change.md` prose, not a survey input signal.

## Target build envelope

`specify slice build <slice>` is guest-routed (`orchestrate::build`): request assembly, the report schema gate, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition all run inside the guest orchestrator; the bound target's guest owns code generation. The request/report DTOs and their gates live in `crates/workflow/src/slice/build/wire.rs`. The standing decisions:

- **Closed validation vocabulary.** The pinned `target-build-*` aborts (request/report schema, success-with-blocking-finding, input/output missing) are `Error::Validation` outcomes (exit 2), not new enum arms.
- **Cross-slice dependency is plan-level ordering** (`depends-on` + `specify plan next`), not envelope plumbing — there is no per-request cross-slice channel.
- **No merge envelope (v1).** `specify slice merge` stays the merge writer; `slice.merge.*` fire on its validator outcome, and the durable record stays `slice.archive.created`. A future merge-findings need reuses the build-report shape rather than authoring a second schema.
- **Build outputs are not cached**; generated code is reproduced by re-running the build.

## Plan-root override: global `--plan-dir` (env `SPECIFY_PLAN_DIR`)

Workspace routing runs phase work inside a materialised slot while the plan artifacts stay at the initiating workspace — by design no slot grows its own plan, and symlinked slots physically live outside the workspace tree so upward path-walking cannot find it. The bridge is an **explicit pass-through from the executor**, which already knows the workspace root: the global `--plan-dir <PATH>` flag (env `SPECIFY_PLAN_DIR`) names the directory holding the governing plan artifacts.

- **One seam.** `Ctx::layout()` applies the override via `Layout::with_plan_dir`; only the plan/change/discovery paths move. Every `.specify/`-anchored path stays on the project (slot) root — observability and slice state remain project-local.
- **Relative source bindings follow the plan.** Relative `sources.<key>.path` bindings are authored against the plan's home, so they join onto the plan root, not the slot.
- **Merge keeps its writer monopoly.** With the override, slot-side `specify slice merge` stamps per-entry `done` in the workspace plan — the "sole writer of `done`" contract holds in workspace mode without a second stamping verb.
- **No back-pointer, no discovery.** The CLI never guesses: an override naming a plan-less directory fails with the same typed errors, whose message cites the overridden path. Guest verbs refuse a `--plan-dir` other than the working directory (see [§"One `specify` binary"](#one-specify-binary)).

## Slot adapter provisioning via workspace sync

Slots carry no plan and no adapters by design, yet slot-side phase work must resolve the adapters the workspace's `plan.yaml.sources` bind. Pinned identities resolve from the *global* store (shared across projects — nothing to mirror) and development bare names resolve the sibling release build live, so the only workspace-owned state a slot cannot reach on its own is the workspace's mirrored local components. That is all `specify workspace sync` copies (`crates/workflow/src/registry/workspace/mirror.rs`): per-file copy-over into each synced slot's out-of-tree cache, foreign entries never pruned, a no-op when the workspace has no component cache. A `url: .` self-slot is skipped; peers without `.specify/` are skipped, never scaffolded. The rejected alternative — a resolve-time plan-root fallback — contradicted the loader contract (resolution is project-local plus the global store); staleness keeps its existing answer everywhere in workspace mode: re-run sync.

## Registry projection and topology cache

Give every fact one writer; derive everything else. A project's *authored intent* — target `adapter` and `description` — lives only in its `.specify/project.yaml`. Its *routing identity* is **derived, not authored**: a deterministic structural projection of the project's own baseline. There are no `capabilities` / `keywords` facets — a derived routing identity needs no second writer duplicating what the baseline already states. `registry.yaml` carries membership + location, cross-project `contracts` wiring, and an optional `adapter` used solely as a greenfield scaffold seed.

- **Derived identity cache.** Workspace plan-time topology is projected through a committed `.specify/topology.lock` (`TopologyLock` in `crates/workflow/src/registry/topology.rs`), regenerated by `workspace sync` from each slot's `project.yaml` plus the deterministic baseline projection. The projection is structural and byte-stable, never an LLM summary, so the committed lock verifies by regenerate-and-compare; it is machine-written write-if-changed and operators never hand-edit it.
- **Read path.** `workspace_topology` builds `ProjectRef[]` from `topology.lock`, not `registry.yaml`; an absent cache fails `topology-cache-missing`. A single regular project reads `project.yaml` plus its own projection live.
- **Staleness, not synchronisation.** `specify plan validate` emits `topology-cache-stale` (warning) on divergence — a regenerate-and-compare check whose fix is `workspace sync`. There is no top-down overwrite of `project.yaml`.

## Tool-owned schemas

Every JSON Schema is owned by the repo that runs it. Adapter prompts reference schemas exclusively by their canonical `$id` URL and never contain schema bodies. The Vectis runtime schemas live solely with the vectis adapter in `augentic/specify-adapters`, with no byte-identity duplication or manual mirroring obligation; this repo carries no vectis schema body.

## Schema `$id` convention

Tool-owned schemas use a stable `$id` of the form `https://schemas.specify.dev/<tool>/<name>.schema.json`. The URL is a logical identifier; it does not need to resolve to a hosted copy. CLI-owned framework schemas use `https://schemas.specify.dev/specify/<path>`. The `links.prompt-schema-link-resolve` framework check enforces that every `schemas.specify.dev` URL cited in adapter prompts resolves to a known schema.

## Component catalog

An operator-curated file at `.specify/design-system/components.yaml` declares shared UI components (`status: confirmed | rejected`); schema CLI-owned at `schemas/design-system/components.schema.json`, domain type `ComponentsCatalog` in `crates/workflow/src/design_system.rs`. The catalog is opt-in — projects without the file work exactly as before. `specify slice validate` enforces `slice-catalog-drift`: every Evidence claim carrying `component: <slug>` must resolve to a confirmed catalog entry.

## Vectis catalog consumer

The Vectis target's build prompts read the component catalog and factor shared component code per confirmed entry per in-scope shell tree. The vectis adapter's in-guest composition validation enforces catalog cross-references: every `component: <slug>` in `composition.yaml` must resolve to a confirmed entry (missing or rejected = error); an unreferenced confirmed entry is a warning. When the catalog is absent, the check is silently skipped.

## Standards chain moved to the adapters; `diagnostics` merged into `schema`

**Decision (2026-07).** Two rationalisations of the crate graph, both YAGNI-driven:

- **Engineering standards ship inside the target adapters.** The only operational consumers of the `UNI-*` universal rules are the target adapters' build review prompts, which run inside composed deployments in consumer projects — so the rules belong in `augentic/specify-adapters`, embedded in each target component's prose registry and served by its references server. The engine distributes no rules and carries no rules parser or export verb. Rule-shape validation stays where authorship is, as the adapters repo's cargo test.
- **`diagnostics` lives in `schema` as `schema::diagnostics`.** The substrate — types, fingerprint, validator, renderers, blocking predicate — sits beside the schema constants it validates against, on the leaf every producer can import (`schema → artifacts → workflow`). A separate crate boundary bought nothing once the substrate had a single downstream spine.

## Drained `Error::Validation` and the `Diagnostic` substrate

Every check surface — `specify slice validate`, plan validation, library validators, build reports — speaks one currency: `Diagnostic` / `DiagnosticReport`, housed at `schema::diagnostics`; it must never depend on artifacts or workflow so it stays importable by every producer.

- **Standards review and validate stay conceptually distinct surfaces.** They share the substrate, not the authority: **validate** gates a lifecycle transition — workflow-owned, non-negotiable, non-silenceable. Standards/policy compliance is codex-owned and lifecycle-neutral (may block CI, never transitions a slice). Convergence applies to the data type, fingerprint, validator, renderer, and blocking predicate — never to the concepts or their gate policies. The litmus test: `validate` must not depend on a crate or module named `lint`.
- **Two orthogonal axes** keep the concepts queryable on the one type: `source` (provenance: `deterministic | model-assisted | hybrid | human | tool`) and `kind` (nature: `violation` vs `review`).
- **Uniform blocking predicate, per-surface application.** `blocking()` returns true iff `kind == violation && severity ∈ {critical, important}`; `kind == review` never blocks anywhere. (The status-aware form — `Diagnostic.status`, `disposition`, the `specify-ignore` directive grammar — retired with the triage taxonomy, YAGNI: no shipped producer ever set a non-`open` status. A baseline/triage mechanism returns only through a fresh RFC with a producer attached.)
- **`Error::Validation` is payload-free.** `Error::Validation { code, detail }`; `variant_str()` returns the carried `code`, so the top-level wire `error` is the specific discriminant. Handlers own rendering: a gate failure renders the full `DiagnosticReport` on **stdout**, then returns the payload-free error purely to carry exit 2 and the discriminant on stderr.
- **Widened `ruleId` namespace.** The diagnostic `ruleId` pattern accepts both the closed codex family (`UNI-…-NNN`) and the runtime-validation discriminant form (dotted/kebab lowercase), so workflow and validate producers stamp their invariant ids onto the same finding shape.

## Composition validation is vectis-owned

The `artifacts::validate` registry carries no `composition` rule namespace. Deep composition validation (schema, structural identity, token/asset refs, catalog cross-references) is owned by the vectis adapter's in-guest composition validation; a shallow host-side duplicate would only drift from it. The host keeps exactly one composition touchpoint: `cross.composition-maps-to-consistent`, which checks `maps_to` well-formedness against the slice's specs. `Artifact::Composition` survives in `schema::diagnostics` — the vectis adapter and build wire still stamp findings with it.

## Single slice-model artifact

- **One artifact.** A synthesized slice persists exactly one structured file, `model.yaml`, with provenance inline (`requirements[].claims[]` carrying `winner`, plus `resolution`). There is no on-disk `provenance.yaml`.
- **Provenance is a projection.** `ProvenanceIndex` (`crates/workflow/src/slice/provenance.rs`) is computed from `model.yaml` and emitted on demand by `specify slice provenance`; it is never loaded from disk. There is no file-drift gate — a projection cannot drift from its source.
- **One schema.** `SLICE_MODEL_JSON_SCHEMA` validates both the agent's synthesis-response `model` and the persisted file; kernel-owned fields are optional, re-derived, and ignored if supplied (normalize, never reject).

## Projection over persistence

Derived state is projected on demand from the journal plus committed artifacts — or pinned to its single authored home by a framework test — never persisted as a second hand-maintained copy. A projection cannot drift from its source; a persisted copy drifts the moment the source moves.

- **The journal is the anchor.** `.specify/journal.jsonl` plus the committed artifacts are the only stores; everything downstream is recomputed per invocation.
- **Live projections.** Provenance (see [§"Single slice-model artifact"](#single-slice-model-artifact)) and `specify plan status`, which projects plan entries + slice lifecycle + the journal tail into a deterministic `next-action` with stop classification, writing nothing and emitting no event.
- **One read surface.** `specify journal show [--filter <prefix>] [--limit N]` is the read verb over the journal; eval probes and any future dashboard consume it instead of bespoke `jq` bridges.
- **Test-pinned homes.** Where derived prose must exist as a copy (the eval catalog's status table), a framework test pins it to its authored home so divergence is a test failure, not a review discovery.

## Architecture seam hardening

One hardening move per seam: the cross-repo contract is the WIT contract; control flow migrates into CLI verbs (the `specify plan status` next-action projection); status surfaces are projections per [§"Projection over persistence"](#projection-over-persistence), with `specify journal show` as the read verb; vocabulary restatements became links or test-pinned homes. The explicit anti-recommendation stands: `workflow` is not split.

## Composition accumulation and component inference

The split: deterministic component *identity* (structural fingerprint over the normalized skeleton) and stable, non-clobbering name *binding* are owned in-guest by the vectis adapter crate; component *identification and naming* are model judgement in the Vectis build prompts. The operator-curated catalog ([§"Component catalog"](#component-catalog)) and the composition cross-reference check ([§"Vectis catalog consumer"](#vectis-catalog-consumer)) carry the durable contract; there is no hard-coded slug-derivation ontology and no adapter name enters the `specify:adapter` contract.

## Authority: document-level plus one override (v1)

v1 resolves authority at document level (`intent` > `documentation` > `behaviour`) with a single override surface: the per-slice `authority-override` on `plan.yaml`, keyed by claim kind. Per-Evidence per-kind overrides and class-lifting are deferred to a future RFC; the closed `AuthorityClass` / `ClaimKind` enums stay.

## Slice synthesis engine

The durable contract for the slice synthesis kernel. Complements [§"Single slice-model artifact"](#single-slice-model-artifact) and [§"Authority: document-level plus one override (v1)"](#authority-document-level-plus-one-override-v1); the kernel-ownership split (agent authors claims + prose; kernel re-derives ids, status, winners, sources, provenance) is pinned in [workflow.md §"Slice synthesis"](./docs/standards/workflow.md#slice-synthesis). The standing decisions:

- **Judgment-dispatched.** There is no WASI tool path and no closed *request* wire shape. Authority is resolved by the kernel **after** the response returns, never shipped in the inputs envelope.
- **Authority kernel.** Resolution order per `(source, kind)`: per-slice override → document `authority` → default class order; a tie at the top class is a `conflict`; mixed-kind requirements resolve each claim independently and pick the strictly-greatest effective class. Pure modules under `crates/workflow/src/slice/synthesis/`.
- **Earned-core schema trim.** `model.schema.json` is trimmed to `required: [requirements, tasks]` — the deferred sub-trees (`domain` / `apis` / `configuration` / `technical-logic` / `observability`), `value` / `path` on claims, and `resolution` / `resolution-trace` are dropped until earned. `synthesis.schema.json` `$ref`s the model schema by relative URI; the `validate_synthesis_json` gate runs on raw bytes before structural deserialize.
- **`to_provenance_index` recompute.** With `value` / `path` / `resolution` gone from the model, the projection recomputes `resolution` via the authority kernel and reads each claim's `value` / `path` from on-disk Evidence keyed by `(source, id)`.
- **Drift validators.** `specify slice validate` emits the blocking `slice-model-*` / `slice-spec-provenance-stale` findings as `Diagnostic` findings on the `DiagnosticReport` surface (meanings tabulated in workflow.md).
- **Journal events.** The `slice.synthesize.*` lifecycle quartet is distinct from the per-requirement `slice.synthesis.{conflict,divergence,unknown}` tag events.

## `domain` replaces `unit` as the spec.md boundary noun

The slice-sized spec grouping — the `specs/<slug>/spec.md` directory segment, the `proposal.md` section heading, and the owning key on each model requirement — is named **domain**, not *unit*. *Unit* was target-neutral but colourless and collided with "unit test" prose; *domain* survives all three first-party targets (Omnia crate/service surface, Vectis business feature, contracts API domain). The wire keys, the `## Domains` proposal heading, and the validate rule ids all carry the noun. Note the name proximity to the *deferred* top-level `domain` sub-tree in the earned-core schema trim (§"Slice synthesis engine"): the requirement-level `domain` key is the spec grouping; the deferred sub-tree, if ever earned, must pick a non-colliding name.

## History via git plus an outcome ledger

The durable record of merged work is git history of the committed `.specify/specs/` baseline plus an append-only outcome ledger: a `slice.archive.created` journal event (payload: slice, touched-specs, outcome summary, merge SHA) emitted from the merge path. The archived slice folder under `.specify/archive/YYYY-MM-DD-<slice>/` is a prunable convenience cache governed by `specify archive prune` (retention policy), not the system of record.

## Bootstrap and upgrade lifecycle

The standing record for the two CLI-owned bootstrap concerns — stale binary and plugin-cache drift. The kernels live in `crates/workflow/src/{upgrade,plugins}.rs` behind the workflow crate's `native` feature (guest builds take `default-features = false`); the verbs parse in the shared `cli` grammar but are refused by the guest router until their in-guest implementations land (see [§"One `specify` binary"](#one-specify-binary)).

- **No migration framework, pre-1.0.** There are no compatibility shims, no versioned parsing, and no `specify migrate` verb: a major version cut means re-init (`specify init --upgrade` over an existing project bumps the pin; anything deeper is a fresh `specify init`). A pin older than the binary loads normally; only a pin *newer* than the binary refuses (exit 3). If a migration story is ever warranted post-1.0, it gets its own decision here first.
- **CLI owns the deterministic actions; skills orchestrate intent and consent only.** Every mutating action requires `--yes` (or interactive confirmation); `--dry-run` previews without writing and fires no event; the read-only probe (`plugins doctor`) never mutates.
- **Channel detection.** `InstallChannel::detect()` classifies the running binary's path into `cargo | brew | binary | unknown`. The latest-release probe order is `SPECIFY_RELEASE_TAG` override → `gh release view` → unauthenticated GitHub API; a probe failure is a **warning**, not an error.
- **Plugin-cache sha derivation.** `plugins doctor` scans `$CURSOR_HOME/plugins/cache/` against the discovered marketplace; the expected sha for relative-path sources is `git rev-parse HEAD` of the marketplace repo. An unresolvable expected sha degrades to `present` / `missing` rather than asserting unprovable drift; the closed status set is `ok | drifted | present | missing | extra`, and `doctor` never exits non-zero on drift (drift is a finding). `plugins refresh` deletes the cache directory, journals `plugins.refreshed`, and never restarts Cursor or touches IDE state.
- **Binary-channel self-replace deferred.** The `cargo` and `brew` executors are fully wired; the `binary`-channel in-process self-replace (download → verify checksum sidecar → atomic swap) is deferred until the release pipeline's archive/checksum naming contract lands. Today the `binary` channel emits a planned-action plus structured manual-upgrade guidance.

## Workspace git transport from the guest

**Decision (2026-07).** Standalone deployment routes `workspace sync` / `workspace push` / `workspace prepare` into the core guest, whose `workflow` world deliberately holds no network, no sockets, and no subprocess capability — yet the workspace verbs are today a pure subprocess-`git` surface funnelled through the `cmd::git` boundary (`crates/workflow/src/cmd.rs`) into `registry/workspace/` (sync, push, bootstrap, mirror) and `registry/branch/` (prepare). **The pinned mechanism is a generic, domain-free host git capability**: an Omnia host crate in the existing `wasi-*` family (working title `wasi-git`, interface `omnia:git/cli`) whose one operation executes the host machine's `git` binary on the guest's behalf — argv-shaped, root-scoped by deployment configuration, hardened non-interactive host-side.

Three properties of the native inventory drove the pin: it is wide (~20 git subcommands) and still evolving; roughly half is local object-database work against the mounted tree and the out-of-tree mirror, not network transport; and remote auth is entirely delegated — credential helpers, `ssh-agent`, and `~/.gitconfig` are host processes and host state that git consults itself, so any in-guest transport would have to re-solve auth from scratch.

- **One argv operation, not a typed per-operation WIT contract.** `run(cwd, argv)` carries exactly what `cmd::git` builds today. A closed per-operation contract would freeze Specify's git workflow into Omnia's WIT — precisely the domain leakage the "Omnia stays domain-free" invariant forbids — and would need re-cutting every time the workspace module grows a subcommand. The argv shape models *the git tool*, like `wasi:http` models HTTP.
- **Root-scoped, not generic exec.** A generic exec capability is rejected outright: arbitrary subprocess is ambient host authority and would void every fence at once. The capability is bounded on three axes the host enforces: the binary is fixed (`git`, never guest-named); `cwd` must resolve beneath a deployment-granted root; the environment is not passed through. Network egress exists only as git transport to remotes named in the operator's committed `registry.yaml`.
- **Credentials stay host-side.** The host git binary consults the operator's credential helpers exactly as the native verbs do today; no token, key, or helper output ever crosses the seam into the guest. This is the decisive advantage over both rejected options.
- **Non-interactive by construction.** The host wrapper pins `GIT_TERMINAL_PROMPT=0`, disables askpass, and runs ssh in batch mode; a credential miss is a typed capability error, never a prompt — cloud and CI runs fail loudly instead of hanging.
- **Rejected: wasm-native git in-guest (gitoxide on wasm32-wasip2).** The local object-database half is plausible, but the transport half is not: gitoxide's HTTP transports assume native TLS stacks, SSH spawns a subprocess the guest cannot have, and a `wasi:http`-backed transport would hand the workflow world the ambient network grant the guest-never-fetches posture exists to prevent. Auth is worse still — credential helpers are subprocesses, so secrets would have to enter the guest as configuration. Fails on every axis except purity.
- **Rejected: the model backend's agent.** Routing `git push` through a judgment leg makes a deterministic, mechanical operation non-deterministic, unauditable, and model-priced; it requires model access in CI for what is a subprocess call; and it inverts the architecture — judgment legs exist for judgment, not transport. Rejected without reservation.
- **Not landed yet.** The workspace verbs parse in the shared grammar but are refused by the guest router (see [§"One `specify` binary"](#one-specify-binary)); `wit/specify.wit` imports no git capability today. They come alive in the change that lands `wasi-git` and the world import; `crates/workflow/src/cmd.rs` is the port seam — the `CmdRunner` boundary lowers to the capability import on `wasm32`, so the workspace modules port without rewriting call sites. A later consequence: the guest merge's skipped git commit leg (§"Deterministic guest merge") regains its commit through the same capability.

## Publishing and distribution: one transport, idempotent legs

**Decision (2026-07).** Every wasm-shaped artifact — the adapter components, the core guest, the WIT contract — publishes over **one transport**: wasm-pkg/OCI behind the static well-known file at `https://augentic.io/.well-known/wasm-pkg/registry.json`. There is no registry service: the well-known file maps the first-party namespace to the backing OCI registry, consumers (plain `wkg get` included) resolve it automatically, and first-party packages are public and pull anonymously. The backing registry is an implementation detail; `augentic.io` and the `specify:` identities are the stable surface, and migrating hosts is editing one JSON file and re-pushing packages.

- **Idempotency is the immutability enforcement.** Every publish leg is probe-first (`scripts/wkg-publish-idempotent.sh`; a semantically identical sibling copy lives in `specify-adapters`): probe the registry for the exact identity, skip when present, build-and-push only on a definitive not-found. The probe must distinguish *absent* from *unreachable*: only a not-found fingerprint grants permission to publish; any other failure aborts non-zero. The fingerprints are coupled to `wkg`'s error text, so CI pins the `wkg` install — revalidate the fingerprints when bumping.
- **The binary↔core lockstep.** The `v*` tag publishes `specify:core@<VERSION>` (a job-level guard asserts the tag matches the `VERSION` file) and the release job carries `needs: [build, publish-core]`: a release whose core push failed never attaches binaries. `publish-wit` gates nothing: the WIT versions independently and most tags no-op that leg.
- **Publish auth is `GITHUB_TOKEN` only.** Both repos' workflows write a wkg config from `permissions: packages: write` + `GITHUB_TOKEN`. Every publish leg lives in a `cargo make publish-*` task and the workflow is a thin caller, so local emergency publishing is the same code path with a developer's own token.
- **WIT ownership.** This repo owns and publishes `specify:adapter` from `wit/specify.wit`; `specify-adapters` vendors the published package, pinned in exactly one place (its `WIT_PIN`) and refreshed by `cargo make wit-vendor`. One owner; everything else consumes.

## Adapter hydration, the committed lock, and the generated deployment

**Decision (2026-07).** The provisioning half of standalone deployment lands as one surface-agnostic hydration kernel plus one manifest generator. `hydrate(project_dir, refs, frozen, fetch)` (`crates/workflow/src/hydrate.rs`) resolves every declared pinned identity — gathered by `collect_refs` from the `project.yaml.adapter` pin, the `adapters:` prefetch list, and `plan.yaml` source pins — against the global store, pulling on miss through an **injected fetch leg**. No caller wires one today, so hydration is store-probe-only until an in-guest fetch leg lands; `workflow` stays wasmtime- and network-free, and the guest-never-hydrates fence is a dependency-direction invariant, not a convention. The intended triggers (`specify init` and `specify adapters sync`) parse in the shared grammar but await their in-guest implementations (see [§"One `specify` binary"](#one-specify-binary)). No prompt exists at or below the kernel.

- **`.specify/adapters.lock` semantics.** The committed cross-machine digest pin (versioned YAML mapping `<name>@<version>` to its `sha256:<hex>` component-byte digest, sorted, machine-written). Verify-when-carried (divergence is `adapter-digest-mismatch` naming both digests); append-on-first-install; write-if-changed (a clean re-verify leaves it byte-stable); frozen-read-only (`--frozen` turns a store miss into the typed `adapter-not-installed` naming the identity and the literal sync command, and never writes the lock). Undeclared entries are left in place — store entries are immutable and shared across projects.
- **Lock verification is one shared kernel pair.** `verify_resolved` (verify-on-read) + the read-only `verify_locked` (the lock gate) exist as one pair so any manifest-producing path re-verifies each pinned entry, and a warm-but-divergent store populated by another machine aborts before any manifest is written or guest driven. A drive never writes the lock.
- **The generated manifest is the sole deployment description.** `workflow::deploy::generate` renders the manifest atomically into `<project-cache>/deployment/omnia.toml`: one `[[guest]]` per resolved component, the core guest's link allow-list, the writable `"."` mount, and per-adapter `/mcp/<name>` routes. The generator verifies every referenced component exists before writing. The manifest is derived, never committed, never hand-edited; a project-root `omnia.toml` wins wholesale as the developer posture.

## One `specify` binary

**Decision (2026-07).** The shipped binary is a single, domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/main.rs`): no native clap surface, no native verb handlers, no Specify vocabulary. Every verb — `--help` / `--version` / bare invocation included — runs in the specify (core) guest, which parses argv through the shared `cli` grammar and passes exit codes through verbatim. This supersedes the earlier two-layer shape — a native provisioning front plus an in-process forwarded leg — whose front and `specify-runtime` crate are deleted; the macro-generated runtime `main` *is* the binary.

- **One help source of truth.** The full operator grammar — provisioning verbs included — lives in `cli`; the guest serves `--help` / `--version` / usage errors for everything (argv[0] pinned to `specify` in the guest parse so help renders the real binary name).
- **Provisioning verbs are refused, not native.** `init` (without `--scaffold-only`), `adapters sync`, `workspace *`, `upgrade`, and `plugins` parse in the shared grammar but have no guest implementation yet: `cli::guest::route` refuses them on the standard argument-error surface (exit 2). They return in-guest — hydration and the workspace git surface each have their own entries — and nothing regrows a native verb set in the interim.
- **In-guest adapter resolution.** Guest verbs resolve adapters inside the guest: the deployment mounts the global store **read-only**, and the guest shim registers a describe runner that routes the resolver's describe dispatch through the deployment's WIT `source` / `target` imports by adapter id. The no-fetch / no-store-write fence is untouched.
- **`--plan-dir` narrows at the seam.** A guest verb naming a plan root other than the working directory is refused (exit 2) — the guest anchors plan artifacts at the `"."` preopen, so any other plan root would be silently ignored.

## Operator onboarding

> **Superseded in part (2026-07).** The TTY elicitation veneer — line prompts for a missing `<adapter>`, the reactive `--platforms` re-run, the postflight `hydrated` report — lived in the native init handler and was deleted with the provisioning front (see [§"One `specify` binary"](#one-specify-binary)). The surviving posture: **flags are the substrate** — every init input is suppliable as a flag, a missing required input is the typed `Error::Validation` (exit 2) naming the requirement (`init-requires-adapter-or-workspace`, `project-platforms-required`), and nothing at or below hydration, manifest generation, or scaffolding can prompt. Idempotent re-entry stands: rerunning init over an initialized project is never an error. If a prompt veneer earns its way back, it sits strictly *above* the hydration kernel.

## Core versioned by the binary

**Decision (2026-07).** The workflow (core) guest is **resolved by the binary's own version**, not embedded: the identity is `specify:core@<CARGO_PKG_VERSION>` — the binary version *is* the core version, one knob, no pin surface for a project to drift on. Nothing is embedded, so guest-reachable code changes need no regeneration step. The `publish-core` release leg is load-bearing — a released binary consumes exactly the identity its tag published, and the `needs: [build, publish-core]` lockstep guarantees the pair ships together.

- **Interim resolution.** The native core-resolution leg (`resolve_core`, the `SPECIFY_CORE_PATH` override) retired with the provisioning front; until the in-guest provisioning story lands, the development posture is the repo-root `omnia.toml` naming the in-repo `specify.wasm` build (omnia.toml-wins). When resolution returns it keeps the same shape: dev override first, then the global store entry `core@<binary version>.wasm` through the same verification pair every pinned adapter passes; a miss never fetches at drive time.
- **The embed option stays open.** When Omnia's generic `runtime!` embed lands, opting in removes hydration and the manifest's core entry without touching the generator (`deploy::generate` takes the core as a plain path) — two interchangeable modes.
