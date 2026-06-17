# Decisions

Standing architectural decisions for the `specify` CLI. Read before changing error layering, exit codes, atomic writes, or the YAML library.

Each entry records the decision, why it was taken, and the consequences a change must reckon with — not how the feature works today. Current behavior lives in [`docs/standards/workflow.md`](./docs/standards/workflow.md) (the workflow contract), [`docs/standards/architecture.md`](./docs/standards/architecture.md) (workspace shape), and module-level rustdoc; entries here point at those rather than restating them.

## Error layering

`specify-error` is the dependency leaf of the workspace. It depends only on `thiserror` and `serde-saphyr`; every other `specify-*` crate may depend on it, and it depends on none of them. The leaf stays free of rich domain payloads: `Error::Validation { code, detail }` is payload-free (see [§"Drained `Error::Validation` and the `Diagnostic` substrate"](#drained-errorvalidation-and-the-diagnostic-substrate)) — the top-level wire `error` is the carried `code` discriminant, and rendered findings travel on stdout as a `DiagnosticReport`, not inside the error.

## Exit codes

The binary commits to a four-slot exit-code table. `Exit::from(&Error)` in `src/runtime/output.rs` is the single source of truth; every dispatcher routes its error through it. `Exit::Code(u8)` is reserved for `specify extension run` WASI passthrough.

| Code | Name                      | When                                                                                                                            |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`            | Command succeeded.                                                                                                              |
| 1    | `EXIT_GENERIC_FAILURE`    | Any `Error` variant not listed below (I/O, YAML, schema, merge, tool resolver/runtime, ...).                                    |
| 2    | `EXIT_VALIDATION_FAILED`  | Validation findings, `Error::Validation`, `Error::Argument`, or a tool request rejected as undeclared.                          |
| 3    | `EXIT_VERSION_TOO_OLD`    | `project.yaml.specify_version` is newer than `CARGO_PKG_VERSION`.                                                               |

`Exit::ArgumentError` and `Exit::ValidationFailed` are distinct Rust variants that share code `2`, keeping the wire contract four-slot while preserving dispatcher-side clarity — anything actionable by the operator is in the JSON envelope's `code` discriminant, and per-finding detail is on the stdout `DiagnosticReport`.

A pin **newer** than the binary is exit `3` (`Error::CliTooOld` — the binary must catch up). A pin *older* than the binary loads fine: pre-1.0 there are no compatibility shims and no migration framework — a major cut means re-init, not migration (see [§"Bootstrap and upgrade lifecycle"](#bootstrap-and-upgrade-lifecycle)).

`specify lint project` is the one finding-driven exit slot: it returns `2` only on a finding with `status: open` AND `severity ∈ {critical, important}` — see [§"Lint finding status, disposition, and exit"](#lint-finding-status-disposition-and-exit).

## Atomic writes

Use `yaml_write` (in `crates/model/src/atomic.rs`) for any file a concurrent reader may observe mid-write: `plan.yaml`, `metadata.yaml`, and the registry. It serialises to `NamedTempFile::new_in(parent)` and `persist`-renames over the target so readers either see the prior bytes or the new bytes. Plain `fs::write` is reserved for files no other process reads concurrently with the writer (one-shot scratch output, fixtures inside a tempdir test). `plan.lock` is the deliberate exception: `specify plan lock` writes its diagnostic body in place (`set_len(0)` + write on the locked fd), never via a rename — an atomic `persist`-rename would replace the inode and sever the OS advisory lock the open descriptor holds. The body is diagnostic noise (the lock identity is the file lock itself), so a reader that races the in-place write simply falls back to `holder-pid=unknown`.

## YAML library

The workspace uses `serde-saphyr` (pinned to a `0.0.x` release) for both deserialization and serialization. It is pure-Rust, panic-free, and actively maintained, in contrast to `serde_yaml` (deprecated) and `serde_yaml_ng` (community fork carrying the same debt). Saphyr omits a `Value` DOM, so code that needs untyped YAML access deserializes into `serde_json::Value`. Its separate deser/ser error types ride directly on `specify_error::Error::YamlDe` and `Error::YamlSer` (both `#[error(transparent)]` `#[from]` variants), so `?` on a raw `serde_saphyr` result still propagates and the kebab discriminant on the wire stays `yaml` for either side.

## Diag-first error policy

`Error::Diag { code, detail }` is the default for new diagnostics. A typed `Error::*` variant exists only when (a) a test or skill destructures the variant's payload, (b) the variant routes to a non-default `Exit` slot, or (c) three or more call sites share the exact shape. The kebab `code` is the wire contract; the Rust variant is for callers that pattern-match. See AGENTS.md §"Errors" for the full rule.

## Hint colocation

Long-form recovery hints live on `Error::hint(&self) -> Option<&'static str>`, not on the renderer. `ErrorBody::render_text` calls it. Adding a new hint means extending `Error::hint`, not the renderer. Hints for collapsed `Diag` codes are looked up by the kebab `code` so a `Diag` site without a typed variant can still surface guidance.

## Wire compatibility

The CLI's JSON output is a flat envelope: every successful body is the typed `*Body` rendered directly, every failure body is `ErrorBody`. Skills grep on the `error` / `code` discriminants; tests assert on them. There is no top-level `envelope-version` integer — re-introduce one only if a breaking shape change ships and consumers need a version stamp to refuse output they cannot parse.

The kebab-case `code` discriminant on `Error::*` variants is the public contract: renaming or removing one is breaking; adding a fresh one is additive. CLI **input** flags are a peer wire surface under the same rules — adding an optional flag is additive, removing or renaming a flag is breaking. One non-additive input change has shipped: `specify init` enforces the `<adapter>` xor `--workspace` invariant through clap rather than the historical post-parse `init-requires-adapter-or-workspace` envelope (the discriminant survives in `crates/workflow/src/init/` for embedders).

## Shell completions

`specify completions <shell>` writes a clap-generated completion script to stdout for any shell `clap_complete::Shell` covers. The script is a pure function of the live clap surface, so verb additions/removals are auto-tracked without extra plumbing.

## Crate layout

The crate graph (leaf → root, with per-crate roles) is pinned in [AGENTS.md §"Crate graph"](./AGENTS.md) and [architecture.md §"Workspace layout"](./docs/standards/architecture.md#workspace-layout); this entry records why the shape is what it is.

SHA-256 digest encoding lives in `specify_schema::digest` (the `specify-schema` leaf), so siblings such as `specify-standards` and `specify-diagnostics` share one digest implementation without depending on `specify-registry` (and therefore Wasmtime). `specify-model` exists so the artifact types and parsers sit on a lifecycle-free leaf; it also holds the artifact validation rule registry (`specify_model::validate`) and depends on neither `specify-workflow` nor anything named lint, so a validation rule physically cannot reach a slice transition or plan stamp — the same no-lifecycle-authority invariant `specify-standards` enforces. The init-time `AGENTS.md` context-fence generation lives in `specify_workflow::agents`, whose only consumer is the root binary.

No framework `CORE-*` rule uses the deleted `specify_standards::framework` `Check` substrate — every rule resolves through the generic lint dispatcher, with Road B checkers running in-process under `crates/standards/src/lint/framework_tools/`; see [§"Framework lint engine: generic dispatcher (Road A / Road B)"](#framework-lint-engine-generic-dispatcher-road-a--road-b). The former `Check` substrate (trait, `Context`, `builder.rs`, helpers) is deleted: its only live content was the repo-local Rust-quality predicates, which now live dev-only beside their single consumer at `tests/rust_quality/checks.rs` in the root crate, and the brief path-classification, which `crates/standards/src/lint/index/brief.rs` owns.

### New workspace crates

New functionality lands in an existing module by default. A new workspace crate requires a paragraph in this file justifying why an existing module cannot host the code, and what dependency-direction invariant the new crate enforces (which leaf-→-root edge it preserves, and which existing crate would have grown a cycle). A new crate that does not strengthen the dependency direction is overhead; refactor within an existing module instead. Adapter-specific logic never lands as a workspace crate — it lands in the adapter's WASI carve-out.

## Integration tests: one binary per area, themed submodules via `#[path]`

**Decision (2026-06).** Each `tests/*.rs` compiles to its own integration binary that links the entire crate-under-test, so total link time scales with the *number* of binaries, not lines of test code. We keep **one binary per area** rather than either extreme:

- A single `tests/it.rs` umbrella (all integration tests in one binary) was measured and rejected — the cold-build win was 7.3 % cargo-reported, below the 20 % bar we apply to "Idiomatic Rust Cleanup" chunks, and a mega-binary makes `cargo test --test <area>` useless for local iteration.
- Strictly one file per binary leaves dozens of near-identical binaries that each re-link the crate-under-test.

The middle ground: conceptually-related suites that share a helper module collapse their themed files under a sibling `tests/<area>/` directory wired with `#[path = "<area>/<concern>.rs"] mod <concern>;`. The hub `tests/<area>.rs` declares the shared helper once (`mod common;` / `mod eval_support;` / `mod engine_support;`); submodules reference it as `crate::common` etc. Merges never cross crate boundaries — each crate's `tests/` is its own compilation unit, and helpers like `copy_dir` are single-sourced per crate.

This collapsed 73 integration binaries to 30 (standards 24 → 5, host 34 → 16, workflow 9 → 7, vectis 6 → 2) while keeping every area runnable via `cargo test --test <area>` and every golden refreshable through the hub binary name (`REGENERATE_GOLDENS=1 cargo nextest run [-p <crate>] --test <area>`).

## Framework lint engine: generic dispatcher (Road A / Road B)

**Decision (2026-06).** The `specify lint framework` engine is a generic, rule-agnostic dispatcher carrying **no rule-specific check logic and no rule policy**. Every framework `CORE-*` check resolves through one of two roads, and `specify` (the framework repo) owns both the checks and the values they enforce:

- **Road A — declarative hint.** The rule carries a `kind:` ∈ `schema | reference-resolves | cardinality | set-coverage | constant-eq | unique | fenced-block | regex | path-pattern | presence | field-grammar | cross-reference | cli-contract`, interpreted by a generic per-kind evaluator in `crates/standards/src/lint/eval/*` over `WorkspaceModel` facts. (The `set-coverage` kind subsumes the former `set-eq`: its `config: { mode: exact }` provides the two-sided comparison.) `hint.value` names the mechanism selector and `hint.config` carries the policy. The per-kind selector semantics live in the eval modules' rustdoc and [AGENTS.md §"Crate graph"](./AGENTS.md) (`crates/standards/src/lint/` bullet).
- **Road B — referenced tool.** The rule carries `kind: tool, value: <tool>` plus a sentinel `path-pattern`. `lint/eval/tool.rs` resolves the tool **by name**, runs it once per lint, and folds its findings (stamped with the tool's own `rule_id` / `severity`; the engine restamps only `id` / `fingerprint`). For framework lint the inventory is the in-process `framework_tools` checker table in `specify-standards` (described below), called directly for typed findings; for project lint the name routes through the `ToolRunner` trait to the declared WASI tool inventory, whose `DiagnosticReport` stdout the evaluator parses.

**Policy lives in the rule's `config:`**, in `specify`. Road A reads it directly; Road B has it forwarded as a second positional argument — the engine relays, it never interprets. Enforced permanently by the Layer-3 guard test [`crates/standards/tests/lint_engine_guards/no_embedded_policy.rs`](./crates/standards/tests/lint_engine_guards/no_embedded_policy.rs), which fails if any eval arm reintroduces a rule-specific literal. The only engine-side constants left are mechanism (evidence-size / snippet / iteration bounds).

**There is no `kind: authoring-predicate`.** The engine carries no imperative rule predicates and no duplicated owner maps; all policy rides the rule's `config:`.

**Seven framework checkers run in-process.** The Road B framework checkers (`scenarios`, `skill-body`, `links-registry`, `marketplace`, `prose`, `rules`, `extension`) are native Rust modules under [`crates/standards/src/lint/framework_tools/`](./crates/standards/src/lint/framework_tools.rs), beside the engine in `specify-standards`. The `kind: tool` evaluator resolves a checker name against the `framework_tools` inventory (`is_framework_checker` / `run_checker`) before the `ToolRunner` trait is consulted, calling it directly and folding the typed `Diagnostic` findings — no `ToolRunner` hop and no JSON serialise→reparse round-trip. Each checker receives `(candidate-path, config-json)` argv, reads policy only from the rule's forwarded `config:`, and returns findings the evaluator restamps (`id` / `fingerprint`): `requested_rule` matches the candidate filename exactly; the prose checker parses `description.maxLength` from the canonical embedded skill schema; a missing project dir surfaces findings instead of silently passing. The `ToolRunner` trait (and `NoopToolRunner`, which backs the framework surface) survives only for the project-side WASI path. WASM remains for the adapter validators (`contract`, `vectis`), which ship to consumer projects and keep the sandbox + digest sidecar trust anchor.

Test coverage rests on the per-kind evaluator unit suites (`crates/standards/src/lint/eval/*`), the `no_embedded_policy` guard, the schema byte-match gate (`crates/schema/tests/schemas.rs`), and each checker module's unit tests.

## Extension architecture

`specify-registry` owns the WASI extension cache, resolver, Wasmtime-backed execution host, and the RFC-48 OCI transport plus content-addressed adapter store, deliberately independent of the adapter loader: the binary resolves adapters, then hands this crate the extension declarations. The manifest DTOs (`Extension`, `ExtensionSource`, `ExtensionPermissions`, `ExtensionScope`, …) and their structural validation live in the wasmtime-free leaf `specify-extension-manifest`, re-exported by `specify-registry` under the `manifest` / `validate` module paths. The split exists for one consumer: `specify-workflow` reads the `tools:` field on `project.yaml` and the adapter's singular `extension` declaration and must not pull `wasmtime` into its compile graph — workflow depends on `specify-extension-manifest` only, never on `specify-registry`. The standing policy choices:

- **Declaration sites (D11).** Two scopes. **Adapter scope:** a singular `extension` object on `adapter.yaml` (`{ name?, permissions? }`, validated by `adapter.schema.json`) — an adapter ships at most one binary built from its co-located `extension/` crate, that wasm rides the adapter's own RFC-47 semver identity and content digest (D4) so a per-extension `version` / `source` / `sha256` is rejected, and the run handle defaults to the adapter name. **Project scope:** a `tools[]` array in `.specify/project.yaml`, each entry `{ name, version, source, sha256?, permissions? }`, validated by `extension.schema.json`. `adapter.yaml` never gains a project-style `tools[]` array.
- **Cache layout.** Root resolves `$SPECIFY_EXTENSIONS_CACHE` → `$XDG_CACHE_HOME/specify/extensions/` → `$HOME/.cache/specify/extensions/`; scope segments (`project--<name>` / `adapter--<axis>--<slug>`) and the `--` separator are pinned in [architecture.md §"WASI tool sidecar scope"](./docs/standards/architecture.md#wasi-tool-sidecar-scope). The wasmtime compilation cache lives beside the scope dirs at `<root>/wasmtime/` (override: `$SPECIFY_WASMTIME_CACHE`); it is best-effort and never consulted by `specify extension gc`.
- **Sidecar invalidation.** The cache sidecar (`extension-sidecar.schema.json`) records `(scope, tool-name, tool-version, source, fetched-at, permissions-snapshot)` plus optional `sha256` / `package`; any mismatch with the live declaration forces a refetch via atomic move. When `sha256` is present, fetched bytes are verified before installation. Permissions changes alone never invalidate the cache — permissions are evaluated per `run`.
- **Permission substitution.** `$CAPABILITY_DIR` / `$PROJECT_DIR` substitutions apply only inside `permissions.{read,write}` (not module argv). After substitution paths must be absolute, free of `..`, and canonicalise inside the granted root. `write:` entries targeting Specify lifecycle state (`.specify/`) are rejected by both the schema and the runner.
- **Argument forwarding and environment.** `specify extension run <name> [-- <args>...]` forwards everything after `--` verbatim with `<name>` as `argv[0]`. The module receives exactly the variables it needs (`PROJECT_DIR` always, the `$CAPABILITY_DIR` adapter root only for adapter-scope extensions) plus stdio; no host environment is inherited.
- **Exit-code mapping.** Module exit `N` passes through (`Exit::Code`); runtime trap and resolver error are `2` with typed envelopes; missing project context is `1` (`not-initialized`); an undeclared extension request is `2`.
- **Wasmtime configuration.** Pin `wasmtime` / `wasmtime-wasi` to a matching stable pair, use the synchronous WASI Preview 2 path and `component::Component`, and disable filesystem access by default — preopens come from declared permissions only. Execution stays behind the concrete runner boundary in `crates/registry/src/host.rs`.
- **`specify extension gc` scope.** Deletes cache entries not referenced by the live declaration set of the current project; it does not scan other projects on the host.
- **Registry resolution.** Wasm-pkg config is layered, last-write-wins: global defaults → project-local `.specify/wasm-pkg.toml` → `WKG_CONFIG` → an embedded `specify -> augentic.io` namespace fallback. `specify init` scaffolds the checked-in `.specify/wasm-pkg.toml` and never overwrites an operator-edited file. RFC-48 adapter packages transit a single content-addressed OCI layer rather than the wasm-pkg package path (see [§"RFC-48 adapter store and transport"](#rfc-48-adapter-store-and-transport)).
- **Time crate.** UTC-only domain on `jiff::Timestamp`; all persisted stamps route through `specify_error::serde_rfc3339` so the wire shape stays `%Y-%m-%dT%H:%M:%SZ` byte-for-byte.

## Source and target adapter role names

The output-role domain types are spelled `Target*` (`Target`, `Slice.target`, the `slice-create-target-missing` / `init-requires-adapter-or-workspace` discriminants, plus every fixture, JSON envelope, and call site). The shared manifest *shape* is loaded by the axis-aware module `crates/workflow/src/adapter/` (`SourceAdapter` / `TargetAdapter` / `Axis` / `ResolvedAdapter` / `AdapterLocation`). Briefs are resolved by path through `briefs.<op>`; they carry no YAML frontmatter and the CLI never reads their bodies. The slice-metadata wire uses `Operation { Shape, Build, Merge }` (`phase: shape | build | merge`).

Per workflow §"Note to the implementing agent", touching any of these symbols requires a cross-repo `rg` sweep against `augentic/specify-cli` and `augentic/specify` in the same PR.

## Adapter loader axis routing

`SourceAdapter::resolve` / `TargetAdapter::resolve` probe the out-of-tree manifest cache (`<project-cache>/manifests/{sources,targets}/<name>/`) then the in-repo tree (`adapters/{sources,targets}/<name>/`) — order and layout pinned in [workflow.md §"Resolver and cache"](./docs/standards/workflow.md#resolver-and-cache). The decisions:

- **Resolution is project-local, plus the global package store.** There is no environment-variable fallback to an out-of-tree framework checkout. A project carries a manifest-cache mirror or a vendored `adapters/` tree, or lets the first-party shorthand fetch at `init` time; RFC-48 D5 adds one non-project source — the global content-addressed adapter store, probed **first** for a pinned `(name, version)` package ref (see [§"RFC-48 adapter store and transport"](#rfc-48-adapter-store-and-transport)). A miss on every probe is `adapter-not-found`.
- **The axis segment is load-bearing.** `sources` / `targets` keeps colliding names disambiguated by axis; cache placement matches the probe layout (`cache_dir(axis, name)`). See [§"Cache layout"](#cache-layout).

### First-party `<adapter>` shorthand at init

`specify init <adapter>` accepts a first-party **shorthand** — `^[a-z][a-z0-9-]*(@<semver>)?$` (`omnia`, `omnia@1.0.0`) — alongside the local-path and GitHub-URL forms. A bare name carries no version pin (resolves the single installed identity); a `name@<semver>` pin records the full `name@<semver>` identity on `project.yaml.adapter` (RFC-47). `AdapterUri::parse` (`crates/workflow/src/init/adapter_uri.rs`) expands the shorthand to the canonical published adapter `https://github.com/augentic/specify/adapters/targets/<name>@<git-ref>`, deriving the **git checkout ref `v<major>`** from the pinned semver. RFC-48 has since landed, but the first-party shorthand stays git-based (H3) — only the `specify:<name>@<semver>` package-ref form pulls from the OCI store (see [§"RFC-48 adapter store and transport"](#rfc-48-adapter-store-and-transport)). `init` is target-only, so the shorthand resolves under `adapters/targets/`; anything carrying `:` or `/` (including a `@v<major>` git-ref form) is not semver shorthand and continues through `from_local` / `from_github` unchanged.

### Adapter identity: semver version + `AdapterRef` (RFC-47)

`adapter.yaml.version` is a **required semver string** (`x.y.z` with optional `-prerelease` / `+build`), enforced by the per-axis JSON Schema and parsed into `SourceAdapter.version` / `TargetAdapter.version` as a typed `semver::Version`. It is the adapter's **identity**, not a descriptive field: synthesized target refs (`topology.lock`, proposal, slice metadata, build-report) render `name@<semver>`, and `TargetRef::parse` requires the `name@<semver>` form (the legacy `name@v<major>` is no longer a valid target).

The loader threads identity through a value type — `AdapterRef { name: String, version: Option<semver::Version> }` (`crates/workflow/src/adapter/core.rs`). `SourceAdapter::resolve` / `TargetAdapter::resolve` take `&AdapterRef`; `locate_axis` stays name-only for the probe (project-local single-identity world). Two gates back the identity:

- **`adapter-version-malformed`** — the post-schema load gate (`check_version`) rejects a manifest whose raw `version` is absent or not exact semver, before typed deserialization, so the diagnostic names the field rather than surfacing a generic parse error.
- **`adapter-version-required`** — `check_requested_version` rejects a pin (`AdapterRef.version = Some(_)`) that does not match the installed manifest identity. Latent in the single-identity world (a `None` pin always picks the installed identity); wired now so RFC-48's multi-identity store widens the same seam.
- **`adapter-cli-too-old`** — RFC-47 D3 host-CLI compatibility floor. `adapter.yaml` carries an optional `specify` semver string (parsed into `requires_specify: Option<semver::Version>`); the post-schema gate `check_requires_specify` compares it against the running binary (`env!("CARGO_PKG_VERSION")`) and aborts with `Error::AdapterCliTooOld` on the exit-3 `EXIT_VERSION_TOO_OLD` path when the binary is older — the adapter-granularity analog of the `project.yaml.specify_version` floor. Exact floor only (no ranges, matching the version-pin posture); absent means no floor. The check is transport-independent (identical for `Local`, `Cached`, and any future registry tree).

A `sources.<key>.version` optional pin on `SourceBinding` (and `sourceBinding` in `plan.schema.json`) carries the same `Option<semver::Version>` — additive, so existing `plan.yaml` binds parse unchanged.

**Transport: RFC-47 baseline, RFC-48 has since landed.** RFC-47 itself changed no transport — the first-party shorthand still derives the git checkout ref `v<major>` from the pinned semver and distributes as a git sparse checkout of `augentic/specify` (H3 keeps the shorthand git-based). RFC-48 then landed the packaging/transport layer RFC-47 had deferred: single-layer OCI packaging and a global content-addressed `(name, version)` store backing `specify:<name>@<semver>` package refs (see [§"RFC-48 adapter store and transport"](#rfc-48-adapter-store-and-transport)). Third-party namespacing beyond the `specify:` namespace, a per-adapter release index, a semver-*range* `specify`-floor policy, and a cross-version compatibility matrix stay deferred to RM-21.

## RFC-48 adapter store and transport

RFC-48 lands adapter packaging and transport on top of the RFC-47 identity. Implementation lives in `specify-registry` (`crates/registry/src/{pack,oci,store,host}.rs`). The deliberate decisions — including the deviations from the RFC's stated defaults:

- **Single-layer OCI packaging (O1).** An adapter publishes as **one** packed `tar+zstd` layer carrying prose and the built `adapter.wasm` together, not the RFC's two-layer prose+wasm default. The Step-1 spike showed `wasm-pkg-client` / `wkg` reject an opaque (non-component) blob in both directions, so a multi-layer opaque artifact bought nothing and one content-addressed layer is simpler. Consequence: an extension-only rebuild re-pushes the whole layer (there is no per-file delta).
- **Blocking install lock (O3).** `store::install_layer` serialises concurrent installers of one identity behind a sibling lock taken with a **blocking** `File::lock()` (wait), not the `File::try_lock` family the `/spec:execute` driver lock uses. For an install, waiting for the peer to finish materialising the immutable entry is preferable to failing fast; the post-lock re-check makes the loser a no-op.
- **Separate `AdapterLocation::Store` variant (D5).** Store-sourced adapters get their own `AdapterLocation::Store` arm rather than reusing `Cached`. `locate_axis` probes the store **first** for a pinned `(name, version)` (ahead of the manifest cache and the vendored tree), so a pin always prefers the immutable store entry.
- **D4 verify-on-read is wired (H2).** `install_tofu` records a deterministic tree content digest of the unpacked store entry into a sibling `<store>/<name>@<version>.meta`; resolve recomputes the digest and verifies it for `AdapterLocation::Store` entries only — raising `adapter-digest-mismatch` — leaving `Local` / `Cached` exempt. A store entry with no recorded `.meta` sidecar **fails open**: verify-on-read rejects a recorded-then-drifted digest, never an absent record (so a pre-D4 entry resolves unchanged). The registry layer digest stays the publish-time anchor (verified on the publish pull-back); under single-layer OCI, read-time verification uses the cheaper tree content digest rather than re-tarring the entry.
- **First-party shorthand stays git-based (H3).** `specify init <name>@<ver>` still resolves through a git sparse checkout plus the per-project manifest cache; only the `specify:<name>@<semver>` package-ref form pulls from the OCI store. Routing the shorthand through the store would need a version-resolution step (RM-21) and is deferred.
- **Resolve-in-place for store entries (L3).** Store-sourced adapters skip the per-project manifest-cache copy — the read-only, content-addressed store entry is resolved in place rather than mirrored into `<project-cache>/manifests/`.

## Plan lifecycle: two stored states

`plan.yaml.lifecycle` is `pending | approved` — no plan-level `in-progress` or `drained` ships in v1; "drained" is computed at read time as "every entry is `done`", not stored. Per-entry status is the closed `pending | in-progress | done` with split writer ownership (see [§"Lifecycle write-ownership"](#lifecycle-write-ownership)). `specify plan transition <plan-name> approved` is Gate 1 and operator-only: the CLI deliberately does not gate it (operators run it from any shell), the `--help` text documents the rule, and `/spec:plan` skill bodies MUST NOT call it.

Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs — exactly `Done → InProgress` and `InProgress → Pending` per call, one `plan.transition.undone` journal event per rung. `Status::Reopened` does not exist: an undone `done` row walks back to `in-progress` so the operator can re-run `/spec:build` and re-merge without a new state. If an upstream revert demands a redo without re-running the slice, author a fresh slice; the original `done` row stays as the historical record. Plan-level lifecycle has no undo path in v1.

Archive is a filesystem operation, not a lifecycle state: `specify plan archive` moves `change.md` + `plan.yaml` into `.specify/archive/plans/`, and the lifecycle stamp inside the archived file stays `approved`. There is no `archived` enum variant — the on-disk location is the signal.

## `SliceSourceBinding`: bare shorthand plus structured form

`plan.yaml.slices[].sources` is one in-memory struct (`{ source_key, lead_id: Option }`) with a custom `Deserialize` accepting two wire shapes and a `Serialize` emitting whichever shape produced the value: the bare string `legacy` (lead falls back to the owning slice's name via `lead_id(slice_name)` — the one-source-per-slice degenerate case, predominantly `intent`), and the structured `{ source, lead }` (required whenever key and lead differ).

Collapsing the variants into one struct means every consumer goes through the same `source_key()` / `lead_id()` accessors instead of `match`-ing a discriminator — the shorthand stays a pure parser concern. Construct in tests via `SliceSourceBinding::bare(..)` / `::structured(..)`. Refer to workflow §"Source".

## `Divergence` enum

`plan.yaml.slices[].divergence` is the closed enum `none | likely | accepted | rejected` (kebab-case wire; `none` is the elided default). `specify plan amend --divergence` only accepts `accepted | rejected` — `likely` is reserved for the `propose` sub-step of `/spec:plan`. Operators flipping the field after Gate 1 use `accepted | rejected` exclusively. Refer to workflow §"Plan-time reconciliation".

RFC-46 D4 adds the sibling `plan.yaml.slices[].disagreements[]` (`{ field, values: [{ source, value }] }`, the `Disagreement` / `DisagreementValue` types), authored by the propose agent alongside a `divergence` flag and carried from the proposal `responseSlice` onto the plan entry by the reconcile kernel. The CLI never decides materiality; `Plan::validate` only checks structural consistency and surfaces it as **advisory** (`Suggestion`) findings: `slice-divergence-unrecorded` (a live `likely`/`accepted` flag without ≥2 distinct source values per recorded field) and `slice-divergence-orphan-values` (recorded values with no flag). Both are deliberately non-blocking — `divergence` is operator-settable standalone via `plan amend --divergence` (advisory metadata in v1), so a divergence-consistency finding may never break that contract-locked write; it surfaces at `plan validate` / Gate 1 instead.

## Plan per-slice authority overrides

`plan.yaml.slices[]` carries an optional `authority-override` map keyed by claim kind, valued by source key. Keys come from the closed claim-kind enum; values MUST be source keys present in the slice's own `sources[]` list — orphans are rejected by `specify slice validate` (`slice-authority-override-orphan-source`). The map is scoped to one slice; plan-wide and project-wide overrides are out of scope.

## Extraction is agent-only — no cache, no fingerprints

Source extraction supports exactly one execution mode: `agent`. The deterministic-extraction substrate that once sat behind it — the extraction cache at `.specify/cache/extractions/<adapter>/`, the closed five-input `CacheFingerprint`, the `cache: opt-out` manifest field, the `source.survey.cache-hit/-miss` and `slice.extract.cache-hit/-miss` journal events with their `CacheMissReason` enum, the `Flow::dispatch_tool` seam in the source op kernel, and `specify source resolve --explain` — was deleted per YAGNI. Agent outputs are non-deterministic, so no run was ever served from the cache; the machinery only constrained changes to the live agent path. `source.schema.json` now enumerates `execution: ["agent"]` for sources (targets keep `agent | tool` — the target-side WASI build dispatch is real). The finalize phases emit plain completion events (`source.survey.completed` / `slice.extract.completed`) instead of cache-probe outcomes. If a deterministic source ever lands, re-add caching behind a fresh decision — the journal taxonomy and manifest schema are the seams to widen.

## Journal event names

`crates/workflow/src/journal.rs` emits the closed journal event taxonomy. The wire ids are dotted kebab-case; the Rust `EventKind` variants are `snake_case` and bridge to the wire via `#[serde(rename = "…")]`. The taxonomy is:

| Wire id                                                 | Emitted by                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `plan.transition.approved`                              | `specify plan transition <plan> approved` (Gate 1 stamp) and `specify plan create --auto-approve`. Payload carries `plan-name` plus the closed `actor` enum (`operator \| agent`, default `operator`) — self-reported via `plan transition --actor` (create always records `operator`); grading evidence for eval probes, not enforcement. Absent on pre-actor journal lines; deserialises as `operator`. |
| `plan.transition.undone`                                | `specify plan transition <entry> --undo` (per-entry reverse rung; one event per rung).                                                                                                                                                                                                                 |
| `plan.entry.advanced`                                   | `specify plan next`, only when an entry actually moves `pending → in-progress` (the sole writer of that status). Returning the active entry or reporting drained/stuck emits nothing, so probes can read "parked, did not advance" from the journal window. Payload carries `plan-name` and `slice-name`. |
| `plan.amend.divergence`                                 | `specify plan amend --divergence likely\|accepted\|rejected` on any change to a slice's `divergence` field (the `/spec:plan` agent stages `likely`; the operator flips `accepted`/`rejected`).                                                                                                         |
| `plan.reconcile.completed`                              | `specify plan propose --from`, once, after the `plan.yaml` write commits (payload: `plan-name`, `slice-count`, `slice-names[]`).                                                                                                                                                                       |
| `slice.transition.refined`                              | `specify slice transition <slice> refined`.                                                                                                                                                                                                                                                            |
| `slice.extract.completed`                               | `specify source extract --phase finalize`, once per `(source, slice)` after the Evidence validates and persists. CLI-owned — the `/spec:refine` skill never emits it via `specify journal emit`.                                                                                                       |
| `slice.synthesize.started`                              | `specify slice synthesize --from` at the start of the projecting/persisting pass. Payload carries `slice-name`.                                                                                                                                                                                        |
| `slice.synthesize.agent`                                | `specify slice synthesize --dry-run` after assembling the agent inputs envelope. One event per invocation; payload carries `slice-name`.                                                                                                                                                               |
| `slice.synthesize.completed`                            | `specify slice synthesize --from` once every artifact validated and persisted. Payload carries `slice-name` and the persisted `artifacts[]`.                                                                                                                                                           |
| `slice.synthesize.failed`                               | `specify slice synthesize --from` aborted before all artifacts were persisted. Payload carries `slice-name` and a short `reason` / finding code.                                                                                                                                                       |
| `slice.synthesis.conflict` / `.divergence` / `.unknown` | `specify slice validate`, one per requirement-block tag emitted by the synthesis substep. (Distinct from the `slice.synthesize.*` lifecycle quartet above — see §"Slice synthesis engine".)                                                                                               |
| `slice.build.started` / `.succeeded` / `.failed`        | `/spec:build`'s target-adapter build flow; one per slice. Payloads carry `slice-name`; the `.failed` variant adds a short `reason` / finding code.                                                                                                                                        |
| `slice.merge.started` / `.succeeded` / `.failed`        | `specify slice merge`'s validator outcome — fires on the validator result, not on a merge report. Payloads carry `slice-name`; the `.failed` variant adds a short `reason` / finding code.                                                                                                |
| `source.survey.completed`                               | `specify source survey --phase finalize`, once the lead set validates and merges into `discovery.md`; payload carries `source` and `adapter`. CLI-owned.                                                                                                                                               |
| `source.execution.agent`                                | The `survey` / `extract` runner on every `execution: agent` invocation; payload carries `source`, `adapter`, and the closed `SourceOperation` (`survey` \| `extract`).                                                                                                                                 |
| `target.execution.agent`                                | `/spec:build`'s target-adapter build flow on every agent invocation; payload carries `slice` and `target` derived from the bound project.                                                                                                                                                 |
| `slice.archive.created`                                 | `specify slice merge`'s archive step (the append-only outcome ledger). Payload carries `slice-name`, `touched-specs`, `outcome-summary`, and the optional `merge-sha`. See §"History via git plus an outcome ledger".                                                                                  |
| `slice.replay.completed`                                | Target adapter's `build` step when it consumes runtime captures; optional in v1. runtime capture semantics.                                                                                                                                                                                            |
| `plan.amend.authority-override`                         | `specify plan create --authority-override`, `specify plan amend --authority-override` / `--clear-authority-override` / `--clear-authority-overrides`. per-slice authority override semantics.                                                                                                          |
| `lint-completed`                                        | `specify lint project` after each scan (the `specify lint framework` development surface does **not** journal — it sets `journal: false` in [`src/output.rs`](./src/output.rs)); payload carries `scope`, `duration_ms`, per-status `counts.{open, ignored, false_positive}`, and the resolved `exit_code`. Wire field names are snake_case to match the journal payload verbatim. |
| `cli.upgraded`                                          | `specify upgrade` after the new binary self-updates; payload carries `from`, `to`, and the resolved install `channel` (`cargo \| brew \| binary`).                                                                                                                                                     |
| `plugins.refreshed`                                     | `specify plugins refresh` after it invalidates the Cursor plugin cache; payload carries the removed `deleted-paths[]` and the resolved `marketplace` file path.                                                                                                                                        |
| `workspace.sync.completed`                              | `specify workspace sync` after the selected slots materialise and `topology.lock` regenerates; payload carries the synced `projects[]` names. The registry-less no-op path emits nothing.                                                                                                              |
| `workspace.push.completed`                              | `specify workspace push` after a non-dry-run invocation with no failed project (non-failure outcomes like `local-only` / `up-to-date` count as success); payload carries `plan-name`, the `specify/<plan-name>` `branch`, and the covered `projects[]`. Dry runs and failed pushes emit nothing.        |

Events persist as newline-delimited JSON at `<project_dir>/.specify/journal.jsonl`. The closed `from` / `to` enum on the divergence events is `none | likely | accepted | rejected`. Refer to workflow §"Observability".

### `specify journal emit` — guarded front door (D12)

Deterministic commands emit their own events. Agent-orchestrated phases that have no deterministic emit command write through `specify journal emit <event-id> [--payload <json>] [--format json]`. The verb mints **no event kinds of its own** — it is a guarded front door onto the same closed `EventKind` taxonomy, preserving "one closed taxonomy, one writer". The closed enum is itself the per-kind payload schema (no parallel JSON-schema registry): the handler reassembles the adjacently-tagged `{ event, payload }` shape and deserialises it into `EventKind`. An unknown tag fails `journal-emit-unknown-event`; a missing required field fails `journal-emit-payload-schema` (both exit 2). The CLI — never the agent — stamps the UTC `timestamp` and appends exactly one line.

## `$CAPABILITY_DIR` replaces `$ADAPTER_DIR`

The WASI extension runner's adapter-scope substitution variable is `$CAPABILITY_DIR`. It expands to the resolved adapter's root directory (the out-of-tree `<project-cache>/manifests/{sources,targets}/<name>/` or the in-repo equivalent) and is only valid in `permissions.{read,write}` entries; project-scope references are rejected as `tool.capability-dir-out-of-scope`. The paired extension cache scope segment is `adapter--<axis>--<slug>` (project-scope tools keep `project--<project-name>`). Refer to workflow §"Sandboxing".

`$CAPABILITY_DIR` is also the read-only manifest-cache root of the four-root source-operation sandbox; see [§"Source operations"](#source-operations).

## Lifecycle write-ownership

Per-entry status writes route to exactly one CLI verb. Skill bodies never write status by hand; the CLI is the single source of truth for each transition:

| State                     | Writer                                    | Trigger                                                                                                                                                    |
| ------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pending` (per-entry)     | `specify plan add` / `specify plan amend` | Operator (or `/spec:plan`) authors / edits a slice row.                                                                                                    |
| `in-progress` (per-entry) | `specify plan next`                       | Sole writer; the `/spec:execute` loop calls it once per slice.                                                                                             |
| `done` (per-entry)        | `specify plan transition <entry> done`    | Called by `/spec:merge` after `specify slice merge` succeeds.                                                                                              |
| `pending` (plan-level)    | `specify plan create`                     | `/spec:plan` scaffolds the plan in `pending`.                                                                                                              |
| `approved` (plan-level)   | `specify plan transition <plan> approved` | Operator-only (Gate 1). The CLI is ungated; `/spec:plan` MUST NOT call this verb — `--help` text documents the rule and the skill body is the actual gate. |

The plan-level `approved` row is the lightest-touch shape the workflow allows: a wholly operator-driven stamp with no CLI-side authentication. Skills that drift from this contract get caught at review time. Refer to workflow §"CLI surface" and §"Writer ownership".

## Plan source bindings

The on-disk shape of `plan.yaml.sources.<key>` is the structured `{ adapter, path?, value? }` object — the 1.x bare-string shorthand was dropped at the Specify 2.0 cut. Every binding carries an explicit kebab-case `adapter` and exactly one of `path` / `value`, enforced in both the JSON Schema and the Rust loader. The `specify plan create --source` flag grammar mirrors the wire shape:

| Form                                       | Materialises as                                                 |
| ------------------------------------------ | --------------------------------------------------------------- |
| `--source <key>=<adapter>:<path>`          | `SourceBinding { adapter, path: Some(<path>), value: None }`    |
| `--source <key>=<adapter>:value:<literal>` | `SourceBinding { adapter, path: None, value: Some(<literal>) }` |

The adapter is the substring up to the first `:` after `=`; the binding payload is everything after it, so URLs containing `:` round-trip through the path form unchanged, and the `value:` sentinel switches the parser to literal mode (the literal may contain any character without escaping). No shorthand exists for "the adapter name equals the key". Source keys are plan-scoped; each key maps to exactly one binding, but slices may reference the same key with different leads.

## Adapter manifest requireds

`description` is required at the top level of every adapter manifest — sources and targets alike — alongside `name`, `version`, `axis`, and `briefs` (`adapter.schema.json` `required: [name, version, axis, description, briefs]`). The adapter `extension` is **optional and singular** (`{ name?, permissions? }`): an adapter ships at most one WASI binary, built from its co-located `extension/` crate, and that wasm rides the adapter's own RFC-47 semver identity and content digest (D4) — a per-extension `version` / `source` / `sha256` is rejected outright, so there is no `tools[].version` requirement at the adapter level. Project-scope `tools[]` (in `.specify/project.yaml`, validated by `extension.schema.json`) keep requiring `name`, a strict-semver `version`, and `source` per entry. Top-level adapter requireds are enforced uniformly by all three adapter schemas.

## Adapter name uniqueness

Adapter names are unique across axes — a name is declared under `adapters/sources/<name>/` xor `adapters/targets/<name>/`, never both (likewise their manifest-cache mirrors). Eagerly enforced at `specify init` (`init/cache.rs::cache_adapter`) and at every `*Adapter::resolve` (`adapter/core.rs::locate_axis` probes the opposite axis for a sibling manifest; `specify` is fork-and-exit, so two `is_file` probes beat memoised process-global state). Collisions surface as `adapter-name-axis-collision`, naming both axes so operators can rename or delete one side without grepping the tree.

## Target platform capability and init validation

Target adapters may declare an optional `platforms` capability (`{ required: bool, allowed: [Platform], default: [Platform] }`) in their manifest. `PlatformsCapability` lives on `TargetAdapter` in `crates/workflow/src/adapter/core.rs`; the schema shape is in `target.schema.json`. When `required` is true, `specrun init` demands `--platforms <csv>` and enforces three validation rules (all exit 2): `project-platforms-required` (target requires platforms but flag absent), `project-platforms-must-include-core` (`core` missing from the set), and `project-platforms-not-allowed` (a token outside `allowed`). The same three rules re-fire as backstops at `TopologyProject::resolve` (`topology-cache-project-platforms-*`). The mutation path is `specrun init --upgrade --platforms <csv>` — `--platforms` is deliberately excluded from the `--upgrade` conflict set. The `Platform` enum (`Core | Ios | Android | Web | Desktop`, kebab on the wire) lives in `crates/workflow/src/platform.rs`.

`platforms` rides the same topology projection rails as `target`: `TopologyProject` and `topology-lock.schema.json` carry it, `workspace sync` re-projects it, and the propose `projectRef` envelope threads it into the reconciliation request. Greenfield `scaffold_greenfield` injects the manifest's `default` when the target declares `platforms.required`.

Plan-time platform reconciliation is a CLI-owned deterministic pass, not agent judgment. `propose --from` always runs the bootstrap post-pass when a bound project declares non-empty `project.yaml.platforms` (no opt-out flag). For Vectis-bound projects, [`vectis_missing_platforms`](crates/workflow/src/platform/detect.rs) links [`specify-vectis-shell-detect`](crates/vectis-shell-detect/) in-process — the same heuristics the vectis `verify --mode detect` WASM wrapper exposes — to probe for on-disk shell trees; propose and plan validate do not dispatch vectis WASM. Non-Vectis targets return an empty missing set. For any supported platform (`core`, `ios`, `android`) declared but absent on disk, `Plan::reconcile_platforms` inserts a bootstrap slice (`app-foundation` for greenfield, `bootstrap-<platform>` for incremental) with all agent-proposed feature slices wired as `depends-on`, in the same atomic `plan.yaml` write. `plan-reconcile-bootstrap-name-collision` extends the closed D2 vocabulary. Bootstrap context for later `app-icon` gates keys off this detect output only — see [RFC-46 Phase 0](https://github.com/augentic/specify/blob/main/rfcs/rfc-46-asset-materialization.md#phase-0--platform-bootstrap-inference-prerequisite) and §6.1 there.

## Cache layout

`.specify/` is **Specify's directory: committed config plus system-of-record** (`project.yaml`, `specs/`, `slices/`, `archive/`, `journal.jsonl`, the lock sidecars). Its lone gitignored in-tree tenant is **`.specify/scratch/`** — transient working state: per-run lanes recreated empty by their owning verb, deletable at any time at zero cost. Everything regenerable and machine-owned now lives *outside* the working tree:

- **The cache is out-of-tree.** The adapter manifest mirror and the distributed codex live in a per-project directory inside the user's OS cache (`$SPECIFY_PROJECT_CACHE`, else `$XDG_CACHE_HOME/specify/projects/<project-id>/`, else `~/.cache/...`), keyed by a stable digest of the canonicalised project path (see [`crates/schema/src/cache.rs`](./crates/schema/src/cache.rs)). Each checkout — including each materialised workspace slot — gets its own collision-free cache that survives `git clean` and never pollutes the working tree.
- **Workspace slots are top-level.** Materialised registry peers live at `<project>/workspace/<peer>/`, not under `.specify/`. Remote peers are `git worktree`s of a persistent out-of-tree bare mirror (`$SPECIFY_MIRROR_CACHE`, else `$XDG_CACHE_HOME/specify/mirrors/<url-id>.git`), so a peer's object store is shared across changes and fresh checkouts; local peers stay symlinks.

`ensure_gitignore_entries` therefore writes exactly two entries — `.specify/scratch/` and the top-level `workspace/`. There is no in-tree `.specify/cache/` to ignore.

The out-of-tree per-project cache hosts two root-disjoint tenants, each self-describing via a provenance stamp so the cache root holds only directories:

- `manifests/{sources,targets}/<name>/` — the adapter manifest mirror (per-axis because adapter names are unique per axis), with provenance at `manifests/manifest-meta.yaml`.
- `codex/` — the distributed shared-rules codex, with provenance at `codex/codex-meta.yaml`. See [§"Shared codex distribution"](#shared-codex-distribution).

There is no extraction-result cache — see [§"Extraction is agent-only — no cache, no fingerprints"](#extraction-is-agent-only--no-cache-no-fingerprints). A manifest-cache directory is therefore always a manifest mirror; the loader never probes for co-tenancy.

`.specify/scratch/` hosts the per-run lanes: `<adapter>/{survey,<slice>}/` (the `$SCRATCH_DIR` preopens, recreated empty at `prepare` time) and `plan/` (the plan-phase handoff lane — `plan propose --dry-run` recreates it empty so `--from` can never consume a stale `propose-response.json`). The write-only `$SCRATCH_DIR` preopen stays rooted inside `.specify/scratch/`, structurally disjoint from the out-of-tree cache, so a scratch write can never pollute a cache artifact.

## Target adapter suffix policy

A plan slice does not store its target adapter. `plan.yaml.slices[]` carries only a `project`; the target (`name@vN`) is a denormalised copy of `project → adapter` and is **resolved on demand** from the bound project's topology rather than persisted. The 1:1 `project → target` invariant ("one target per project") is what makes the denormalisation removal safe. The integer `N` remains a load-bearing wire field wherever a resolved target *does* appear (`specify plan next`, slice `metadata.yaml`, the build request).

- `slices[].project` is optional on disk: omitted resolves to the sole project in the topology; a multi-project workspace requires it explicitly. The plan schema carries no `target` property — a slice may legitimately carry neither field.
- `propose.rs::resolve_target` is the single read-time resolver (`plan-reconcile-project-orphan` / `plan-reconcile-project-binding-required` / `plan-target-malformed`); `TargetRef` is constructed by it, never deserialised from `plan.yaml`.
- `specify plan validate` flags an omitted `project` only when a multi-project registry makes it ambiguous; `specify plan next` resolves best-effort and reports `target: null` rather than failing the lifecycle query — the build phase re-resolves before use.

## Operations typed at parse boundary

Adapter operations are typed Rust enums by the time YAML parsing finishes; string operation names never survive past the manifest loader. The decorative `operations:` array was removed from every manifest and schema — `briefs.keys()` is the canonical iterator over an adapter's declared operations, with the closed `SourceOperation` / `TargetOperation` enums (`crates/workflow/src/adapter/operation.rs`) as the typed key sets carried by the axis-split `SourceAdapter` / `TargetAdapter` structs.

**Wire invariant.** The `source resolve` / `target resolve` envelopes' `operations: [...]` arrays iterate in kebab-alphabetical order (`["extract", "survey"]`, `["build", "merge", "shape"]`). Derived `Ord` on the enums is intentional because variants are declared in kebab-alphabetical wire order.

## Adapter execution mode

Every adapter manifest declares a closed `execution` enum, `required` at the top level of both per-axis schemas. The loader rejects a manifest that omits the field with `adapter-execution-mode-required` rather than defaulting silently (`check_execution` in `crates/workflow/src/adapter/core.rs`; exit 2).

Source adapters are **agent-only**: `source.schema.json` enumerates `execution: ["agent"]` (see [§"Extraction is agent-only — no cache, no fingerprints"](#extraction-is-agent-only--no-cache-no-fingerprints)). Target manifests may declare `agent` (brief run by an agent, two-phase prepare/finalize dispatch, `*.execution.agent` journal event per invocation) or `tool` (target-axis only: `build` / `merge` dispatch through a declared WASI tool or built-in deterministic Rust path, single-phase). All eight first-party manifests ship `execution: agent`; no first-party target owns a build tool yet, so the target `tool` branch is wired and schema-valid but unexercised. Refer to workflow §"Adapter implementation shape".

## Source operations

`specify source survey <source> [--plan <name>] [--phase prepare|finalize]` and `specify source extract <source> <lead> --slice <slice> [--phase prepare|finalize]` are the CLI-owned source adapter operations; the operational contract — resolution via `plan.yaml.sources.<key>`, the two-phase agent handoff, the handoff envelope fields, and the four-root sandbox — is pinned in [workflow.md §"Source adapter contract"](./docs/standards/workflow.md#source-adapter-contract) and [§"Sandboxing"](./docs/standards/workflow.md#sandboxing). The standing decisions:

- **Validate-before-visible.** An invalid lead set leaves `discovery.md` untouched; a failed Evidence validation leaves the slice in `refining`.
- **`discovery.md` stores raw, unmerged, per-source leads.** The runner stamps `source` from the surveyed source (attribution is CLI-owned). A re-survey is a per-source fold by `(source, lead)`, never a cross-source collapse — unification is deferred to plan time, so the same `lead` may legally appear under different source keys and alias-collision scoping is per `source`.
- **`$PROJECT_DIR` is not visible to the adapter.** Lifecycle state stays off-limits; the scratch lane is recreated empty at `prepare` time so a stale artifact from a prior run can never be finalized as this run's output.
- **The CLI never blocks on agent work.** `prepare` returns after printing the handoff envelope; `finalize` validates, persists, and journals the completion event.
- **Shared prep seam.** Adapter resolution, brief-directory resolution, sandbox layout, and `evidence/` scaffolding live in one helper (`src/runtime/commands/source/prep.rs`) shared by the workflow-free `specify source preview` and the workflow-integrated runners — none of it is re-implemented per verb.
- **Value-binding envelope.** Value-bound sources (`intent`) get no `$SOURCE_DIR` preopen and carry `value-inline`; path bindings carry `source-path`.

## Lead reconciliation

`specify plan propose` wraps agent-led cross-source lead reconciliation in a CLI-owned projection kernel (kernel in `crates/workflow/src/change/plan/core/propose.rs`). The two mutually-exclusive modes (`--dry-run` read-only request envelope; `--from <response.json>` the only slice writer) are pinned in [workflow.md §"Plan-time reconciliation"](./docs/standards/workflow.md#plan-time-reconciliation). The standing decisions:

- **Replaceable gate.** `--from` replaces slices only while the plan is replaceable (`lifecycle: pending` AND every entry `pending`; `plan-reconcile-plan-not-replaceable` otherwise). Re-propose wholesale-replaces all slices — a fresh projection, not a merge — discarding prior per-slice operator edits.
- **Coverage invariant, no kernel grouping.** The kernel carries no `scope` noun. It enforces total lead coverage (every surveyed `(source, lead)` referenced by at least one slice) plus at most one lead per source per slice. A lead may appear in more than one slice — fan-out is multiple ordinary slices joined by `depends-on`, and same-project multi-homing of a cross-cutting lead is equally legal (no `depends-on` implied). Same-source fusion is rejected on purpose: each surveyed lead is the source adapter's own sizing judgment, so merging two leads from one source would override that sizing; the operator owns same-source re-sizing at Gate 1 via `specify plan amend --sources`. The at-most-one-lead-per-source invariant is enforced at every writer, not just propose: `Plan::validate` carries the `duplicate-source-key` finding (folded into the validate-and-rollback gates inside `Plan::create` / `Plan::amend`), and the `plan amend --add-source` path re-gates via `reject_duplicate_source_keys` — a duplicate key would otherwise silently overwrite `evidence/<source>.yaml` at refine time. The kernel validates shape only — it never auto-merges, clusters, or forbids cross-source splits.
- **Explicit slice names.** Every response slice carries an explicit kebab-case `name` written verbatim; `depends-on` resolves against those names (cycles fail). Name uniqueness is the sole duplicate gate.
- **Project binding.** The agent binds each slice's `project` from the request's `projects[]`; an omitted `project` auto-binds only when exactly one project exists. The target adapter is **not** written to `plan.yaml` (see [§"Target adapter suffix policy"](#target-adapter-suffix-policy)).
- **Closed validation vocabulary.** `plan-reconcile-empty-catalog`, `-lead-orphan`, `-partition`, `-slice-source-collision`, `-slice-name-invalid`, `-slice-name-collision`, `-depends-on-cycle`, `-project-binding-required`, `-project-orphan`, `-plan-not-replaceable`, plus `plan-propose-mode-required` — all `Error::Validation` outcomes (exit 2), not new enum arms.
- **Single-event journal.** One `plan.reconcile.completed` fires only after the `plan.yaml` write commits. The `/spec:plan` skill never calls `specify journal emit` for reconciliation.
- **Split on doubt.** Matching rides on per-source `synopsis` headlines alone, so the synopsis carries a contentfulness expectation (taught in survey briefs; surfaced as the non-blocking `discovery-lead-synopsis-thin` advisory). The error-cost asymmetry is stated in the propose brief: an over-merge is expensive and downstream-poisoning, an over-split is cheap and Gate-1-reversible — so a weakly-supported cross-source match stays as separate slices with the candidate pairing noted in `change.md` under `## Tentative merges`.
- **Deferred (rejected).** Kernel-side token-intersection auto-merge (shared slugs are unattested), kernel-side advisory clustering (would need per-lead `blocking-keys[]` survey metadata), and per-lead target-axis hints (`target` stays kernel-derived). Grouping uncertainty is the agent's to express through `change.md` prose, not a survey input signal.

## Target build envelope

`specify slice build <slice> [--phase prepare|finalize]` owns the per-slice build envelopes — the symmetric target-side twin of the source runners: the CLI owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; the bound target's `build` brief owns only code generation. The envelope shapes, two-phase flow, and what the request deliberately omits (`target`, `execution`, brief paths, `model.yaml`) are pinned in [workflow.md §"Target adapter contract"](./docs/standards/workflow.md#target-adapter-contract). The standing decisions:

- **Closed validation vocabulary.** The five pinned aborts — `target-build-request-schema`, `target-build-report-schema`, `target-build-success-with-blocking-finding`, `target-build-input-missing`, `target-build-output-missing` — are `Error::Validation` outcomes (exit 2), not new enum arms. The handler also raises adjacent operational diagnostics (`target-build-report-missing`, `-report-slice-mismatch`, `-failed`, `-tool-unsupported`, `-brief-missing`).
- **Cross-slice dependency is plan-level ordering** (`depends-on` + `specify plan next`), not envelope plumbing — there is no per-request cross-slice channel.
- **No merge envelope (v1).** `specify slice merge` stays the merge writer; `slice.merge.*` fire on its validator outcome, and the durable record stays `slice.archive.created`. A future merge-findings need reuses the build-report shape as `build/merge-report.yaml` rather than authoring a second schema.
- **Build outputs are not cached** in either execution mode; generated code is reproduced by re-running the build.
- **Acceptance proof.** One end-to-end fixture proves fan-in twice and fan-out once together: [`tests/plan/end_to_end.rs`](./tests/plan/end_to_end.rs) over `tests/fixtures/fan-in-fan-out/` asserts envelope/ordering/determinism. The separate generated-output-correctness release gate (replay/golden suites plus `cargo check` / `cargo test` for generated crates) is a manual/CI acceptance step, not part of the deterministic test.

## Workspace terminology

The word **workspace** overloads three related concepts. Use them verbatim in operator-facing prose:

| Term               | Meaning                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Workspace**      | Registry-only platform repo: `workspace: true` in `project.yaml`, `registry.yaml`, plan artifacts at the repo root |
| **Workspace slot** | Materialised peer at `workspace/<project>/`                                                                        |
| **Workspace sync** | `specify workspace sync` — materialise slots and regenerate `topology.lock`                                        |

`/spec:init workspace` and `specify init --workspace` scaffold a workspace; init chains an initial workspace sync before returning.

## Plan-root override: global `--plan-dir` (env `SPECIFY_PLAN_DIR`)

Workspace routing runs phase work inside a materialised slot while `plan.yaml` / `change.md` / `discovery.md` stay at the initiating workspace — by design no slot grows its own plan, and symlinked slots physically live outside the workspace tree so upward path-walking cannot find it. The bridge is an **explicit pass-through from the executor**, which already knows the workspace root: the global `--plan-dir <PATH>` flag (env `SPECIFY_PLAN_DIR`) names the directory holding the governing plan artifacts.

- **One seam.** `Ctx::layout()` applies the override via `Layout::with_plan_dir`; only `plan_path()`, `change_brief_path()`, and `discovery_path()` move. Every `.specify/`-anchored path (slices, journal, scratch, cache, archive) stays on the project (slot) root — observability and slice state remain project-local.
- **Relative source bindings follow the plan.** `plan.yaml.sources.<key>.path` relative bindings are authored against the plan's home, so `resolve_source_path` joins them onto the plan root, not the slot.
- **Merge keeps its writer monopoly.** With the override, slot-side `specify slice merge` stamps per-entry `done` in the workspace plan — the "sole writer of `done`" contract holds in workspace mode without a second stamping verb at the workspace.
- **No back-pointer, no discovery.** The CLI never guesses: an override naming a plan-less directory fails with the same typed errors (e.g. `slice-synthesize-plan-missing`), whose message cites the overridden path. Adapter resolution is untouched — slot-side source adapters resolve project-locally (vendored tree or manifest cache) per §"Adapter loader axis routing".

## Slot adapter provisioning via workspace sync

Slots carry no plan and no adapters by design, yet slot-side phase work must resolve the adapters the workspace's `plan.yaml.sources` bind. The loader stays exactly as recorded (§"Adapter loader axis routing": resolution is project-local only); `specify workspace sync` provisions the probe location the loader already consults — it mirrors the workspace's adapter set (both axes, vendored tree and manifest-cache mirror alike, each adapter's `extension` included) into each synced slot's out-of-tree `<slot-cache>/manifests/{sources,targets}/`. Mirroring is unconditional over the workspace adapter set: no plan parsing in sync, and the cache is out-of-tree so slots carry no repo residue. Implementation: `crates/workflow/src/registry/workspace/mirror.rs`.

- **Per-name delete-then-copy, no GC of foreign names.** Each workspace-owned name is removed and re-copied per sync, so re-sync refreshes. Names the workspace does not own are never pruned — the slot cache has a second legitimate writer (`specify init` caches greenfield adapter seeds), and a per-axis wipe could delete an adapter only the slot has. A name present at the workspace in both probe locations copies from the manifest cache, matching the loader's probe order.
- **Slot-vendored names are skipped at mirror time, cross-axis.** The loader probes the cache *before* the vendored tree, so "the slot's own copy wins" cannot come from probe order — the mirror skips any name the slot vendors under `adapters/{sources,targets}/<name>/` on either axis. The same-axis skip keeps the slot copy winning resolution; the opposite-axis skip means the mirror can never manufacture an `adapter-name-axis-collision` in a previously healthy slot.
- **Local symlink slots are mirrored too** — unlike the contracts distribution, which skips them — because the adapter gap is slot-side resolution regardless of slot backing, and the write lands only under the peer's out-of-tree per-project cache. A `url: .` self-slot is skipped: mirroring the workspace onto itself would remove-then-copy the cache from itself. Peers without `.specify/` are skipped, never scaffolded.

The rejected alternative — a resolve-time plan-root fallback — shipped mid-run and was removed (`204e3867`): it contradicted the loader contract, keyed on the `adapter-not-found` string discriminant, and covered only the source axis. Staleness keeps its existing answer everywhere in workspace mode: re-run sync (the per-slice sync in the execute loop makes that automatic).

## Registry projection and topology cache

Give every fact one writer; derive everything else. A project's *authored intent* — target `adapter` and `description` — lives only in its `.specify/project.yaml`. Its *routing identity* is **derived, not authored**: a deterministic structural projection of the project's own baseline. There are no `capabilities` / `keywords` facets — a derived routing identity needs no second writer duplicating what the baseline already states. `registry.yaml` carries membership + location, cross-project `contracts` wiring, and an optional `adapter` used solely as a greenfield scaffold seed.

- **Derived identity cache.** Workspace plan-time topology is projected through a committed `.specify/topology.lock` (`TopologyLock` in `crates/workflow/src/registry/topology.rs`), regenerated by `workspace sync` from each slot's `project.yaml` plus the deterministic baseline projection (`surface[]` = per-domain spec titles capped at `SURFACE_TITLE_CAP = 8`; `recent[]` = last `RECENT_TAIL = 10` `slice.archive.created` summaries). The projection is structural and byte-stable, never an LLM summary, so the committed lock verifies by regenerate-and-compare; it is machine-written write-if-changed and operators never hand-edit it. `TopologyProject` does `deny_unknown_fields`, so a pre-upgrade lock fails to load until `workspace sync` rewrites it — the ordinary machine-rewrite fix.
- **Read path.** `workspace_topology` builds `ProjectRef[]` from `topology.lock`, not `registry.yaml`; an absent cache fails `topology-cache-missing`. Empty `surface` / `recent` stay off the wire, so a greenfield project degrades cleanly to `description` only. A single regular project reads `project.yaml` plus its own projection live (`regular_topology`).
- **Staleness, not synchronisation.** `specify plan validate` emits `topology-cache-stale` (warning) on divergence — a regenerate-and-compare check whose fix is `workspace sync`. There is no top-down overwrite of `project.yaml` and no `--check` flag; CI uses the exit-2 gate of `plan validate`. Both topology codes are plan-doctor findings on exit 2.

## Tool-owned schemas

Every JSON Schema is owned by the repo of the WASI tool (or the CLI) that runs it. Plugin briefs reference schemas exclusively by their canonical `$id` URL and never contain schema bodies. The three Vectis runtime schemas live solely with the vectis extension crate in `augentic/specify-adapters` (`adapters/targets/vectis/extension/embedded/`), with no byte-identity duplication or manual mirroring obligation; this repo no longer carries any vectis schema body.

## Schema `$id` convention

Tool-owned schemas use a stable `$id` of the form `https://schemas.specify.dev/<tool>/<name>.schema.json`. The URL is a logical identifier; it does not need to resolve to a hosted copy. CLI-owned framework schemas use `https://schemas.specify.dev/specify/<path>`. The `links.brief-schema-link-resolve` predicate enforces that every `schemas.specify.dev` URL cited in adapter briefs resolves to a known schema.

## `specify source preview`

`specify source preview <adapter> --source <path> [--lead <id>...] [--out <path>]` is a workflow-free verb: it resolves the source adapter, validates `--source`, scaffolds `${out}/evidence/`, and emits a summary. No `.specify/` writes, no journal events; it uses `dispatch` (not `scoped`) so no `.specify/` directory is required. Implementation at `src/runtime/commands/source/preview.rs`.

## Component catalog

An operator-curated file at `.specify/design-system/components.yaml` declares shared UI components (`status: confirmed | rejected`); schema CLI-owned at `schemas/design-system/components.schema.json`, domain type `ComponentsCatalog` in `crates/workflow/src/design_system.rs`. The catalog is opt-in — projects without the file work exactly as before. `specify slice validate` enforces `slice-catalog-drift`: every Evidence claim carrying `component: <slug>` must resolve to a confirmed catalog entry. `notes.candidate_component` annotations are informational-only and never trigger drift.

## Vectis catalog consumer

The Vectis target's `build` brief reads the component catalog and factors shared component code per confirmed entry per in-scope shell tree (brief additions in the plugin repo). The Vectis WASI tool's `validate composition` mode enforces catalog cross-references: every `component: <slug>` in `composition.yaml` must resolve to a confirmed entry (missing or rejected = error); an unreferenced confirmed entry is a warning. When the catalog is absent, the check is silently skipped.

## Standards layer split into `specify-standards` and `specify-schema`

The standards surface (rules parser / resolver, `WorkspaceModel`, indexer, deterministic hint interpreter, `specify lint` runner) lives in `specify-standards`, a **sibling** of `specify-workflow` rather than a module inside it. `specify-schema` is the shared leaf owning every embedded JSON Schema constant plus the `jsonschema` plumbing, so workflow and standards consume schemas from one place.

Dependency direction is the decision: `specify-standards` does **not** depend on `specify-workflow`, and `specify-workflow` does **not** depend on `specify-standards`. The sibling shape makes "no lifecycle authority in review" a type-system invariant — review code cannot reach slice or plan transitions because the workflow types are not visible. The `kind: tool` lint evaluator is wired through a `ToolRunner` trait at the CLI boundary, not a `specify-registry` dependency. The neutral diagnostic substrate lives one layer further down in `specify-diagnostics` (see [§"Drained `Error::Validation` and the `Diagnostic` substrate"](#drained-errorvalidation-and-the-diagnostic-substrate)), so `specify-workflow` and the `specify_model::validate` registry mint diagnostics without depending on anything named `lint`. The root binary wires the halves together at the dispatcher boundary; they never call each other directly. See [architecture.md §"Standards layer vs workflow layer"](./docs/standards/architecture.md#standards-layer-vs-workflow-layer).

## Drained `Error::Validation` and the `Diagnostic` substrate

Every check surface — `specify lint`, `specify lint framework`, `specify slice validate`, plan validation, library validators — speaks one currency: `Diagnostic` / `DiagnosticReport`, housed in the `specify-diagnostics` leaf (depends only on `specify-{error,schema,digest}`; must never depend on standards, model, or workflow so it stays importable by every producer).

**Lint and validate stay conceptually distinct surfaces.** They share the substrate, not the authority: **validate** gates a lifecycle transition — workflow-owned, non-negotiable, non-silenceable (ignore directives are off). **lint** is standards/policy compliance — codex-owned, lifecycle-neutral (may block CI, never transitions a slice), silenceable with an in-source rationale. Convergence applies to the data type, fingerprint, validator, renderer, and blocking predicate — never to the concepts or their gate policies. The litmus test: `validate` (or any non-lint producer) must not depend on a crate or module named `lint`.

**Two orthogonal axes** keep the concepts queryable on the one type: `source` (provenance: `deterministic | model-assisted | hybrid | human | tool`) and `kind` (nature: `violation` vs `review` — a deterministically-raised request for judgment; the former `Deferred` classification and `lint-mode: model-assisted` rules both surface as `kind: review`, `source: deterministic`).

**Uniform blocking predicate, per-surface application.** `blocking()` returns true iff `kind == violation && status == open && severity ∈ {critical, important}`. `kind == review` never blocks anywhere. Each surface applies the same predicate, differing only in whether ignore directives run first (lint: yes; validate: no).

**`Error::Validation` is payload-free.** `Error::Validation { code, detail }`; `variant_str()` returns the carried `code`, so the top-level wire `error` is the specific discriminant rather than a generic `"validation"`. Handlers own rendering: a gate failure renders the full `DiagnosticReport` on **stdout**, then returns the payload-free error purely to carry exit 2 and the discriminant on stderr. Single operational errors that are not findings take the same shape via `Error::validation_failed(code, detail)` but render no report.

**Widened `ruleId` namespace.** The diagnostic `ruleId` pattern accepts both the closed codex family (`UNI-`/`CORE-`/… `-NNN`) and the runtime-validation discriminant form (dotted/kebab lowercase), so workflow and validate producers stamp their invariant ids onto the same finding shape the codex engine uses.

## Composition validation is vectis-tool-owned

The `specify_model::validate` registry carries no `composition` rule namespace. The registry once registered four `composition.*` rules, but `validate_slice`'s canonical artifact set never fed them — they were dead at the runner while their unit tests asserted the registry entry existed (false confidence). Rather than wiring `composition.yaml` into `CANONICAL_ARTIFACTS`, the namespace was deleted: deep composition validation (schema, structural identity, token/asset refs, catalog cross-references) is owned by the vectis WASI tool's `validate composition` mode, and a shallow host-side duplicate would only drift from it. The host keeps exactly one composition touchpoint: `cross.composition-maps-to-consistent`, which checks `maps_to` well-formedness against the slice's specs. `Artifact::Composition` survives in `specify-diagnostics` — the vectis tool and build wire still stamp findings with it.

## Codex-rule schema: one source of truth

The rules parser consumes the canonical rule schema directly via `specify_schema::RULE_JSON_SCHEMA` (paired with the typed `Rule` DTO). One source of truth means no vendored copy and no drift check: the canonical schema lives at `schemas/rules/rule.schema.json` and is embedded through `specify-schema` like every other schema.

## Lint finding status, disposition, and exit

`Diagnostic.status` is a closed kebab-case enum. The fingerprint algorithm excludes both `status` and `disposition`, so demoting a finding from `open` to `ignored` (or `false-positive`) never changes its identity.

| Value            | Set by            | Meaning                                                                                                                              |
| ---------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `open`           | scanner (default) | Freshly emitted finding before post-passes run. The only value that contributes to the `specify lint` exit-code decision by default. |
| `ignored`        | directive pass    | An in-source `specify-ignore` directive matched the finding's `(path, line, rule-id)`. Carries `disposition.directive`.              |
| `false-positive` | directive pass    | A directive matched and the rationale was prefixed `false-positive:`. Reported separately in dashboards.                             |
| `fixed`          | reserved          | Reserved for the cross-run baseline diff verb. No producer in v1.                                                                    |
| `accepted`       | reserved          | Reserved for explicit operator acceptance via the baseline file. No producer in v1.                                                  |

`disposition` is an optional sibling object (`{ source, directive?, since? }`), populated only when `status != open`; `disposition.source` is a closed enum whose only v1 value is `directive` — a future baseline producer adds `baseline` additively.

`specify lint project` resolves the process exit with **status-aware severity**: exit `2` only when a finding has `status: open` AND `severity ∈ {critical, important}`. The synthetic findings for malformed (`UNI-022`) and orphan (`UNI-023`) directives default to `status: open` (the shared codex ships both at `important`). **Graceful degradation:** when the codex resolver does not produce `UNI-022`/`UNI-023` — a consumer without the shared codex tree — synthetic emission silently skips while status stamping continues; the fix is codex distribution (see [§"Shared codex distribution"](#shared-codex-distribution)). The operator-facing directive grammar lives in the parent repo's [`docs/reference/ignore-directives.md`](https://github.com/augentic/specify/blob/main/docs/reference/ignore-directives.md).

## Shared codex distribution

Consumer projects resolve shared `UNI-*` rules without a co-located framework checkout or a manual `--rules-root` (RM-07). The shared codex ships beside the target adapter in its source repo (`adapters/shared/rules/{universal,core}/`); `specify init` and `specify rules sync` mirror it into the out-of-tree `<project-cache>/codex/`, **pinned to the same adapter source/ref**.

- **Probe order** (`probe_rules_root` in `crates/standards/src/rules/resolve.rs`): explicit `--rules-root` → monorepo `adapters/shared/rules/universal/` → codex cache → `rules-root-required`. The cache rung is a derived root, so the fallback overlay stays skipped, exactly like the monorepo case; both `specify lint` and `specify rules export` honour it.
- **Distribution.** `cache_codex` / `sync_codex` walk up from the resolved adapter `source_dir` to the nearest ancestor carrying the `universal/` pack and copy it (plus `core/` under `--include-framework`); git sources fetch in the same sparse checkout as the adapter. **Fail-soft:** a source tree without the pack leaves the cache empty and the consumer falls back to `--rules-root`.
- **Provenance.** `CodexMeta` (`codex-meta.yaml`) records the pinned source, `include_framework`, and `fetched_at`. Audit-only; the resolver never reads it.
- **Distribution vs evaluation are independent.** `--include-framework` controls what lands in the cache; the resolver's `include_core` controls whether `CORE-*` rules are evaluated/exported. Consumer projects default to neither.

## Single slice-model artifact

- **One artifact.** A synthesized slice persists exactly one structured file, `model.yaml`, with provenance inline (`requirements[].claims[]` carrying `winner`, plus `resolution`). There is no on-disk `provenance.yaml`.
- **Provenance is a projection.** `ProvenanceIndex` (`crates/workflow/src/slice/provenance.rs`) is computed from `model.yaml` and emitted on demand by `specify slice provenance`; it is never loaded from disk. There is no file-drift gate (a projection cannot drift from its source); spec-vs-model staleness and `(source, id)` orphan checks apply. There is no `slice.provenance.written` journal kind.
- **One schema.** `SLICE_MODEL_JSON_SCHEMA` validates both the agent's synthesis-response `model` and the persisted file; kernel-owned fields are optional, re-derived, and ignored if supplied (normalize, never reject), so `DRAFT_MODEL_JSON_SCHEMA` and the `slice-synthesize-kernel-field-usurped` abort both retire.

## Projection over persistence

Derived state is projected on demand from the journal plus committed artifacts — or pinned to its single authored home by lint — never persisted as a second hand-maintained copy. A projection cannot drift from its source; a persisted copy drifts the moment the source moves.

- **The journal is the anchor.** `.specify/journal.jsonl` ([§"Journal event names"](#journal-event-names)) plus the committed artifacts (`plan.yaml`, slice `metadata.yaml`, the specs baseline) are the only stores; everything downstream is recomputed per invocation.
- **Live projections.** Provenance ([§"Single slice-model artifact"](#single-slice-model-artifact) — "Provenance is a projection") and `specify plan status`, which projects plan entries + the candidate slice's lifecycle + the journal tail into a deterministic `next-action` with stop classification read from the `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` terminals, writing nothing and emitting no event. RM-15's re-entry fields extend the same body rather than persisting a status file.
- **One read surface.** `specify journal show [--filter <event-id-prefix>] [--limit N]` is the read verb over the journal; eval probes and any future dashboard consume it instead of bespoke `jq` bridges over the JSONL.
- **Lint-pinned homes.** Where derived prose must exist as a copy (the eval catalog's status table), a framework rule pins it to its authored home (CORE-056 catalog↔runs agreement) so divergence is a lint finding, not a review discovery. (The `content-digest-eq` digest-pinning kind retired with its last rule consumers — restatements became links.)

## Architecture seam hardening

One hardening move per seam, each carried by its own standing decision: the machine-checked cross-repo contract is `specify contract dump` plus the `cli-contract` lint kind (R1); control flow keeps migrating into CLI verbs — the `specify plan status` next-action projection, and both the acquisition and the read-side probe of the driver lock in `crates/workflow/src/plan_lock.rs` (R2): `specify plan lock -- <cmd>` is the `flock(1)`-style command-wrapper that takes the lock for the spawned child's lifetime (so lock handling is cross-platform and Python/`flock(1)`/`zsystem`-free), and the plan-state-writing verbs `require_held`; status surfaces are projections per [§"Projection over persistence"](#projection-over-persistence), with `specify journal show` as the read verb (R3); vocabulary restatements became links or lint-pinned homes (R5). The explicit anti-recommendation stands: `specify-workflow` is not split.

## Composition accumulation and component inference

The landed split: deterministic component *identity* (structural fingerprint over the normalized skeleton) and stable, non-clobbering name *binding* live in the CLI (`specify catalog infer`); component *identification and naming* are model judgement in the Vectis build brief. The operator-curated catalog ([§"Component catalog"](#component-catalog)) and the Vectis tool's composition cross-reference check ([§"Vectis catalog consumer"](#vectis-catalog-consumer)) carry the durable contract; the hard-coded slug-derivation ontology was deleted, not extended.

## Authority: document-level plus one override (v1)

v1 resolves authority at document level (`intent` > `documentation` > `behaviour`) with a single override surface: the per-slice `authority-override` on `plan.yaml`, keyed by claim kind. The per-Evidence `authority-overrides` field and per-kind class-lifting are removed for v1 and deferred to a future RFC; the `AuthorityOverrides` type is deleted accordingly, while the closed `AuthorityClass` / `ClaimKind` enums stay.

## Slice synthesis engine

The durable contract for `specify slice synthesize`, its projection kernel, and the schema/event additions. Complements [§"Single slice-model artifact"](#single-slice-model-artifact) and [§"Authority: document-level plus one override (v1)"](#authority-document-level-plus-one-override-v1). The two-phase command surface (`--dry-run` inputs envelope / `--from` sole writer) and the kernel-ownership split (agent authors claims + prose; kernel re-derives ids, status, winners, sources, provenance) are pinned in [workflow.md §"Slice synthesis"](./docs/standards/workflow.md#slice-synthesis). The standing decisions:

- **Two-phase, agent-dispatched.** Mirrors `plan propose`: exactly one of the two modes is required (`slice-synthesize-mode-required`); there is no WASI tool path and no closed *request* wire shape. Authority is resolved by the kernel **after** the response returns, never shipped in the inputs envelope.
- **Authority kernel.** Resolution order per `(source, kind)`: per-slice override → document `authority` → default class order; a tie at the top class is a `conflict`; mixed-kind requirements resolve each claim independently and pick the strictly-greatest effective class. Pure modules under `crates/workflow/src/slice/synthesis/` (`authority.rs`, `project.rs`, `render.rs`, `wire.rs`).
- **Earned-core schema trim.** `model.schema.json` is trimmed to `required: [requirements, tasks]` — the deferred `domain` / `apis` / `configuration` / `technical-logic` / `observability` sub-trees and their id grammars, `value` / `path` on claims, and `resolution` / `resolution-trace` are dropped until earned. `synthesis.schema.json` `$ref`s the model schema by relative URI; the two compile together through a `jsonschema::Registry` pinning `MODEL_SCHEMA_URL`. The `validate_synthesis_json` gate runs on raw bytes before structural deserialize (`synthesis-schema`, exit 2).
- **`to_provenance_index` recompute.** With `value` / `path` / `resolution` gone from the model, the projection recomputes `resolution` via the authority kernel and reads each claim's `value` / `path` from on-disk Evidence keyed by `(source, id)`.
- **Drift validators.** `specify slice validate` emits seven blocking typed-model findings (exit 2) — `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, `slice-model-id-grammar` — as `Diagnostic` findings on the `DiagnosticReport` surface (meanings tabulated in [workflow.md §"Slice synthesis"](./docs/standards/workflow.md#slice-synthesis)).
- **Journal events.** The `slice.synthesize.{started|agent|completed|failed}` lifecycle quartet is distinct from the per-requirement `slice.synthesis.{conflict,divergence,unknown}` tag events. See [§"Journal event names"](#journal-event-names).

## `domain` replaces `unit` as the spec.md boundary noun

The slice-sized spec grouping — the `specs/<slug>/spec.md` directory segment, the `proposal.md` section heading, and the owning key on each model requirement — is named **domain**, not *unit*. *Unit* was target-neutral but colourless and collided with "unit test" prose; *domain* survives all three first-party targets (Omnia crate/service surface, Vectis business feature, contracts API domain) and was already latent in the contracts shape brief ("API domain slug"). The rename was a hard cut with no migrator (pre-1.0, no downstream projects): the wire keys (`synthesis.schema.json` / `model.schema.json` `domain`, `topology-lock.schema.json` / `proposal.schema.json` `surface[].domain`), the `## Domains` proposal heading, and the validate rule ids (`proposal.domains-listed`, `cross.proposal-domains-have-specs`) all renamed in place with no compatibility aliases. Note the name proximity to the *deferred* top-level `domain` sub-tree in the earned-core schema trim (§"Slice synthesis engine"): the requirement-level `domain` key is the spec grouping; the deferred sub-tree, if ever earned, must pick a non-colliding name.

## History via git plus an outcome ledger

Revises the archive posture. The durable record of merged work is git history of the committed `.specify/specs/` baseline plus an append-only outcome ledger: a `slice.archive.created` journal event (payload: slice, touched-specs, outcome summary, merge SHA) emitted from the merge path. The archived slice folder under `.specify/archive/YYYY-MM-DD-<slice>/` becomes a prunable convenience cache governed by `specify archive prune` (retention policy mirroring the tool-cache GC), not the system of record. `.specify/specs/` stays committable (init gitignores only `.specify/scratch/` and the top-level `workspace/`; the cache is out-of-tree, so there is nothing in-tree to ignore).

## Bootstrap and upgrade lifecycle

The standing record for the two CLI-owned bootstrap concerns — stale binary and plugin-cache drift. The `/spec:init` skill stays the orchestrator; each deterministic action is its own CLI verb (`upgrade`, `plugins {doctor,refresh}`; handlers under `src/runtime/commands/`, domain logic under `crates/workflow/src/`).

- **No migration framework, pre-1.0.** There are no compatibility shims, no versioned parsing, and no `specify migrate` verb: a major version cut means re-init (`specify init --upgrade` over an existing project bumps the pin; anything deeper is a fresh `specify init`). A pin older than the binary loads normally; only a pin *newer* than the binary refuses (`Error::CliTooOld`, exit 3). If a migration story is ever warranted post-1.0, it gets its own decision here first.
- **CLI owns the deterministic actions; skills orchestrate intent and consent only.** Every mutating action requires `--yes` (or interactive confirmation); `--dry-run` previews without writing; the read-only probe (`plugins doctor`) never mutates.
- **Channel detection.** `InstallChannel::detect()` classifies the running binary's path into `cargo | brew | binary | unknown` (the last a structured diagnostic with manual-upgrade guidance). The latest-release probe order is `SPECIFY_RELEASE_TAG` override → `gh release view` → unauthenticated GitHub API; a probe failure is a **warning**, not an error.
- **Plugin-cache sha derivation.** `plugins doctor` scans `$CURSOR_HOME/plugins/cache/` against the discovered marketplace; the expected sha for relative-path sources is `git rev-parse HEAD` of the marketplace repo, shared by every plugin. An unresolvable expected sha degrades to `present` / `missing` rather than asserting unprovable drift. The closed status set is `ok | drifted | present | missing | extra`; `doctor` never exits non-zero on drift (drift is a finding), only on FS/parse failure. `plugins refresh` deletes the cache directory, journals `plugins.refreshed`, and never restarts Cursor or touches IDE state.
- **Two bootstrap journal events** — `cli.upgraded`, `plugins.refreshed` (see [§"Journal event names"](#journal-event-names)). `--dry-run` writes nothing and fires no event.
- **Binary-channel self-replace deferred.** The `cargo` and `brew` executors are fully wired; the `binary`-channel in-process self-replace (download → verify checksum sidecar → atomic swap) is deferred until the release pipeline's archive/checksum naming contract lands. Today the `binary` channel emits a planned-action plus structured manual-upgrade guidance.
