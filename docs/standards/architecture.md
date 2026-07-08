# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `Layout<'a>` boundary, time injection, network hardening, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Binary crate (`name = "specify"`) at the repo root. [`src/main.rs`](../../src/main.rs) is a single `omnia::runtime!` invocation in command mode over the cursor-bound backends — the binary carries no Specify vocabulary. Every verb, including the hidden `lint framework` dev tool, runs in the workflow guest (`crates/workflow`) through the shared dispatch grammar (`crates/dispatch`); `lint project` retired from the operational surface, and engineering standards reach consumer projects through `specify rules export`. Workspace member crates live under `crates/`; the dependency direction is leaf → root:

```text
specify-error                    # leaf — thiserror + serde-saphyr only
specify-schema                   # depends on specify-error (embedded JSON Schemas + jsonschema plumbing; also owns schema::digest — SHA-256 hex via sha2 + base16ct)
specify-diagnostics              # depends on specify-{error,schema} (Diagnostic substrate: report, fingerprint, validator, renderers, blocking)
specify-model                    # depends on specify-{error,diagnostics} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; model::validate artifact rule registry — NOT on specify-workflow-lib or anything named lint)
specify-standards                # standards layer — depends on specify-{error,schema,diagnostics}; NOT on specify-workflow-lib
specify-workflow-lib             # workflow layer — depends on specify-{error,schema,model,diagnostics} (also owns workflow::agents — init-time AGENTS.md context-fence generation — and config::tools, the parse-clean project-scope tools[] DTOs); NOT on specify-standards (no wasmtime in its graph)
specify-workflow                 # wasm32 wasi:cli/run core guest — depends on specify-dispatch + specify-workflow-lib
specify (root crate)             # the omnia::runtime! binary — depends on no specify-* crate
```

The framework authoring checks behind `specify lint framework` run entirely through the declarative hint interpreter in `specify_standards::lint` plus the in-process Road B checkers beside it (`crates/standards/src/lint/framework_tools/`); there is no imperative `Check` substrate (see [DECISIONS.md §"Crate layout"](../../DECISIONS.md#crate-layout)).

`specify-standards` (standards) and `specify-workflow-lib` (workflow) are siblings: they never import each other. The artifact validation rule registry (`specify_model::validate`) is the validation analog: it sits on `specify-model`, which depends on neither `specify-workflow-lib` nor anything named lint, so an artifact rule cannot reach workflow lifecycle types — the same no-lifecycle-authority invariant `specify-standards` enforces. `specify-model` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry both `specify-standards` and `specify-workflow-lib` read, alongside `specify-schema` and `specify-error` at the bottom. The Phase 1B collapse from 13 crates and the standards-layer split that re-introduced `specify-standards` and `specify-schema` are logged in [DECISIONS.md §"Crate layout"](../../DECISIONS.md#crate-layout) and [DECISIONS.md §"Standards layer split into `specify-standards` and `specify-schema`"](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema).

### Standards layer vs workflow layer

`specify-standards` (standards) and `specify-workflow-lib` (workflow) are deliberately siblings. The §"Principles" / "No lifecycle authority in review" rule from [DECISIONS.md §"Standards layer split into `specify-standards` and `specify-schema`"](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema) is a type-system invariant rather than a coding convention: `specify-workflow-lib` MUST NOT depend on `specify-standards` (review code never reaches workflow lifecycle types), and `specify-standards` MUST NOT depend on `specify-workflow-lib` (review code cannot transition a slice or stamp a plan). Both depend on `specify-schema` so the embedded JSON Schemas live in one place, and both depend on the `specify-diagnostics` leaf for the neutral `Diagnostic` substrate — so a workflow validator mints findings without `specify-workflow-lib` ever depending on anything named `lint`. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](../../DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate). Refer to [DECISIONS.md §"Standards layer split into `specify-standards` and `specify-schema`"](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema).

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `specify-error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `specify-error` re-introduces the cycle the layering was designed to avoid; do not. The long-form rationale lives in [DECISIONS.md §"Error layering"](../../DECISIONS.md#error-layering).

**New workspace crates** are an exception, not the default. See [DECISIONS.md §"New workspace crates"](../../DECISIONS.md#new-workspace-crates) for the bar a new crate must clear.

The root `specify` crate is a binary-only package (`src/main.rs`, the `omnia::runtime!` invocation). The whole `specify` dispatch tree — including `commands/lint/framework.rs` for `specify lint framework` — lives in `crates/dispatch`; clap introspection for shell completions lives in [`crates/dispatch/src/commands.rs`](../../crates/dispatch/src/commands.rs) via `Cli::command()`.

## standards layer modules

Three `specify-standards` module trees carry the standards-layer contract; touching any of them requires a cross-repo `rg` sweep per [AGENTS.md §"When working in this repo"](../../AGENTS.md#when-working-in-this-repo).

- **`crates/standards/src/rules/`** — rules parser and resolver pipeline (`parse.rs`, `resolve.rs`, `resolve/{filter,sort}.rs`). The fingerprint algorithm and finding validators live in the `specify-diagnostics` leaf — import them from there directly. The resolver walks both `codex/rules/universal/` (`Origin::Shared`) and `codex/rules/core/` (`Origin::Core`) and tags every resolved rule with its origin so `specify lint` / `specify rules export` can default-exclude `CORE-*` unless `--include-core` is passed (§A3).
- **`crates/standards/src/lint/index/`** — dual-profile indexer that produces a `WorkspaceModel` from a tree on disk. The closed `ScanProfile::{Project, Framework}` enum picks the walk shape: the project profile roots at `project_dir` (or the supplied `artifact_paths`), records symlinks without traversing them, and runs only the shared per-file extractors (`frontmatter`, `markdown`, `ignore_directives`); the framework profile (`index/framework.rs`) applies the §F1 include set, follows symlinks with cycle detection (recording both endpoints), and runs the framework-only extractors (`skill.rs`, `scenario.rs`) alongside the shared passes. `lint::index::build(project_dir, ScanProfile::Framework, &[], &[])` is the entry point `specify lint framework` calls; the project profile survives the `lint project` retirement as the indexer's project-rooted walk shape (no CLI verb drives it today). Both profiles share the same `WorkspaceModel` assembly invariants (byte-stable enumeration, sorted output collections).
- **`crates/standards/src/lint/eval/`** — deterministic-hint interpreters, one `eval/<kind>.rs` module per hint kind in `schemas/rules/rule.schema.json`'s closed enum (`path-pattern`, `regex`, `schema`, `tool`, `unique`, `reference-resolves`, `constant-eq`, `fenced-block`, `presence`, `field-grammar`). No kind is reserved — adding a kind requires landing its interpreter in the same change. `lint-mode: model-assisted` rules are not skipped — they surface as `kind: review` diagnostics (the deterministic engine raises the question without scoring it). The four formatters live in the neutral `specify-diagnostics` leaf (`crates/diagnostics/src/render/{json,pretty,github,compact}.rs`) and consume the closed `Diagnostic` shape every surface emits.

## workflow domain modules

Four module trees carry the workflow contract — three in `specify-workflow`, plus `spec/provenance.rs` which now lives in `specify-model`; touching any of them requires a cross-repo `rg` sweep per [AGENTS.md §"When working in this repo"](../../AGENTS.md#when-working-in-this-repo).

- **`crates/workflow-lib/src/adapter/`** — axis-split loader. `SourceAdapter::resolve(name, project_dir)` and `TargetAdapter::resolve(name, project_dir)` are the per-axis entry points for loading a source or target adapter manifest; each carries its closed operation set (`SourceOperation` / `TargetOperation`) derived from the manifest's `axis` per the WIT contract, with serde rejecting unknown variants at the YAML parse boundary. The closed `Axis::{Source, Target}` enum routes cache paths and the runtime dispatcher used by `specify {source,target} resolve`; see [DECISIONS.md §"Adapter loader axis routing"](../../DECISIONS.md#adapter-loader-axis-routing) for the long form. Operation briefs are compiled into each adapter's guest — the CLI never resolves or reads brief bodies — and `ManifestMeta` is in [`init/cache.rs`](../../crates/workflow-lib/src/init/cache.rs).
- **`crates/model/src/spec/provenance.rs`** — parser and validator for the requirement-block provenance metadata (`ID:`, `Sources:`, `Status:`) that core synthesis emits at the top of every `spec.md` requirement. `RequirementStatus` is closed (`agreed | unknown | conflict | divergence`); the inline `[…]` tag on the requirement heading must agree with the `Status:` line. Findings aggregate so one malformed block does not mask later problems.
- **`crates/workflow-lib/src/journal.rs`** — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`. Closed `Event` / `EventKind` taxonomy; kebab-case dotted wire ids (`plan.transition.approved`, `plan.amend.divergence`, `slice.transition.refined`, `slice.extract.completed`, `slice.synthesis.{conflict,divergence,unknown}`) bridge to `snake_case` Rust variants via `#[serde(rename = "…")]`. Append is atomic and is the only mutation; readers tail the file and skip blank lines.
- **`crates/workflow-lib/src/schema.rs`** — workflow-aware validation wrappers for the on-disk workflow artifacts (`schemas/plan/plan.schema.json`, `schemas/evidence.schema.json`, the adapter/source/target manifest schemas, `schemas/discovery/lead.schema.json`). The raw embedded schema constants and the generic `jsonschema` plumbing live in `crates/schema/` (`specify-schema`) per [DECISIONS.md §"Standards layer split into `specify-standards` and `specify-schema"](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema); this module imports them and adds the workflow-shaped error aggregation (the `rule_id` strings the CLI surfaces, joined into the payload-free error `detail`). Validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`; surfaces that render findings (`slice validate`) emit a `DiagnosticReport` on stdout first. `specify plan add` / `plan amend` / `slice validate` are the first-use hooks. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](../../DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

## Per-axis cache layout

`SourceAdapter::resolve` / `TargetAdapter::resolve` probe — in order — the agent-populated out-of-tree cache at `<project-cache>/manifests/{sources,targets}/<name>/` and then the in-repo manifest at `<project_dir>/adapters/{sources,targets}/<name>/`. The `{sources,targets}` segment is keyed by `Axis`, so source and target adapters with colliding names disambiguate by axis. `cache_dir(axis, name)` returns the cache-side path. Do not collapse the two roots or special-case one axis — workflow §"Resolver and cache" pins the shape.

## WASI tool sidecar scope

Historical: the WASI tool cache (`$SPECIFY_EXTENSIONS_CACHE` → `$XDG_CACHE_HOME/specify/extensions/` → `$HOME/.cache/specify/extensions/`, `project--<project-name>` scope segments) deleted with the Wasmtime tool runner and the `specify-registry` crate, and the structural validation surface deleted with `specify-extension`. What survives is the parse-clean `tools[]` declaration shape on `project.yaml` (`specify_workflow_lib::config::tools`); nothing resolves, validates, or executes declared tools until the `tools[]` surface's fate is decided.

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — no longer live in this repo. They extracted to `augentic/specify-adapters` and are now in-guest adapter library code compiled into each adapter's published component. The carve-out discipline (leaner lint posture, minimal `[workspace.dependencies]`, no `specify-error` / `wasmtime` / `tokio` / `ureq` dependency) now lives in that repo's workspace. Crux shell presence and launcher-icon heuristics extracted with the vectis adapter too: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

The framework checkers behind `specify lint framework`'s Road B rules are not WASI components — they run in-process inside `specify-standards` (`crates/standards/src/lint/framework_tools/`), resolved by name from the `kind: tool` evaluator before the `ToolRunner` trait (which survives for the project-side WASI path).

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. The `specify-registry` and `specify-extension` crates are deleted — the declared-tool (`tools[]`) declaration shape survives as `specify_workflow_lib::config::tools` until that surface's fate is decided. No `specify-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.specify/` is framework-managed state every CLI verb writes through (configuration under `project.yaml`, `slices/`, `archive/`, `scratch/`, the journal, the `guest.lock` marker). Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root. The boundary is enforced by the `Layout<'a>` newtype in `specify-workflow` (`crates/workflow-lib/src/config.rs`): path helpers are inherent methods on `Layout<'a>`, and call sites write `Layout::new(&dir).plan_path()`. Do not hard-code `.specify/registry.yaml` or sibling paths, and do not declare free path-helper functions outside `crates/workflow-lib/src/config/`; any new `.specify/` path lands on `Layout`.

## Time injection

Functions that record a timestamp into a serialised artifact accept `now: jiff::Timestamp` from the dispatcher boundary. Library crates do not call `Timestamp::now()`; the call sites live in the `specify-dispatch` handlers so tests can pin time deterministically. The current carve-out — `slice_actions::*` and friends still consume an injected `now` argument — is the canonical shape to follow.

## ureq fetch hardening

Any `ureq` HTTP path in this workspace (today: the channel-aware self-update probe in `crates/workflow-lib/src/upgrade.rs`) runs with explicit per-call timeouts, a response-size cap checked on both the `Content-Length` header and the streamed body, and streams large bodies to a tempfile before persisting. Any new HTTP path must adopt the same shape (timeouts + size cap + stream-to-tempfile); do not buffer arbitrary remote bodies into memory.

## Atomic writes

Use `yaml_write` (in `crates/model/src/atomic.rs`) for any file a concurrent reader may observe mid-write: `plan.yaml`, `metadata.yaml`, and the registry. It serialises to `NamedTempFile::new_in(parent)` and `persist`-renames over the target so readers either see the prior bytes or the new bytes. Plain `fs::write` is reserved for files no other process reads concurrently with the writer (one-shot scratch output, fixtures inside a tempdir test).

The standards-side phrasing of the rule lives in [coding-standards.md §"YAML, JSON, and atomic writes"](./coding-standards.md#yaml-json-and-atomic-writes); the long-form rationale lives in [DECISIONS.md §"Atomic writes"](../../DECISIONS.md#atomic-writes).

## Toolchain

Rust stable per `rust-toolchain.toml` (channel `stable`, components `clippy`, `rust-src`, `rustfmt`). WASM targets pre-installed via `targets = ["aarch64-apple-darwin", "wasm32-wasip2", "x86_64-apple-darwin"]`.

`rustfmt.toml` uses unstable nightly features (`unstable_features = true`, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`). Format with nightly:

```bash
cargo +nightly fmt --all
```

`cargo make fmt` does this for you.

## Supply chain

`cargo-vet` and `cargo-deny` gate `cargo make ci`; `cargo-audit`, `cargo-outdated`, and `cargo-udeps` are advisory tasks run on demand (`cargo make audit` / `outdated` / `deps`). The vet task is check-only (`cargo vet --locked`) — regeneration is deliberately not part of the gate, since regenerating exemptions before checking would auto-exempt anything unaudited. When a new dependency lands:

1. Add it to `[workspace.dependencies]` in the root `Cargo.toml` with a major-version pin (e.g. `serde = { version = "1", features = ["derive"] }`). Per-crate `Cargo.toml` references it as `serde.workspace = true`.
2. Run `cargo make vet-regenerate` to refresh the supply-chain audits, review the `supply-chain/` diff, then commit it.
3. Check `deny.toml` allows the dependency's licence. The current allowlist is in `deny.toml`; add a new SPDX id only after confirming compatibility with MIT-OR-Apache-2.0.

`clippy::multiple_crate_versions` is silenced workspace-wide (`Cargo.toml`'s `[workspace.lints.clippy]`); duplicate transitive versions are audited by hand via `cargo tree --duplicates` on each `cargo update`, not gated through a ratchet.

## Skill / CLI responsibility split

Every deterministic operation lives in this CLI: kebab-case validation, `metadata.yaml` reads/writes, lifecycle transitions, plugin resolution (`specify source resolve` / `specify target resolve`), artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, plan/registry validation, schema validation of `plan.yaml` and per-source `Evidence`, journal event append. The plugin repo's `/spec:` skills (`/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:finalize`, `/spec:init`, `/spec:drop`) shell out for all of those; the execute loop is the CLI verb `specify plan execute`.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

The parent repo's [`AGENTS.md`](https://github.com/augentic/specify/blob/main/AGENTS.md) is the source of truth for workflow vocabulary (slice / change), skill family, plan-driven loop, and contract skills.
