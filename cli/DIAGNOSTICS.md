# Diagnostics lint unification — status and remaining scope

Status record for the lint unification work originally tracked as **A19** (unify lint output path + framework/consumer dispatch) and **A16** (imperative→declarative lint burn-down). The two former binaries (`specrun` runtime + `specdev` authoring lint) have since converged onto a single `specify` binary; the framework authoring lint is now `specify lint framework`.

The framework lint engine is now a generic dispatcher with no rule-specific logic or policy; the steady-state posture is below and the authoritative decision is [DECISIONS.md §"Framework lint engine: generic dispatcher (Road A / Road B)"](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b).

Scope: `augentic/specify-cli` (primary) and `augentic/specify` (CORE rule files, docs).

Related docs:

- [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](./DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate)
- [DECISIONS.md §"Crate layout"](./DECISIONS.md) — framework-authoring-checks steady state
- [docs/standards/handler-shape.md](./docs/standards/handler-shape.md) — "The two lint handlers share one tail"
- [adapters/shared/rules/core/README.md](https://github.com/augentic/specify/blob/main/adapters/shared/rules/core/README.md) (framework repo)
- [docs/contributing/checks.md](https://github.com/augentic/specify/blob/main/docs/contributing/checks.md) — parity contract and extension guide (framework repo)

---

## A19 — Unify lint output path and dispatch — COMPLETE

Both lint surfaces converge on one kernel. `specify lint project` ([src/runtime/commands/lint/project.rs](./src/runtime/commands/lint/project.rs)) and `specify lint framework` ([src/runtime/commands/lint/framework.rs](./src/runtime/commands/lint/framework.rs)) each return `Result<()>` and call `output::run_lint(format, || build_report(...))` in [src/output.rs](./src/output.rs). The kernel owns the shared tail — `emit_lint_report` runs the pipeline and renders the envelope, the internal `finish_lint` collapses the outcome into the terminal `Result<()>` (`deny_blocking_findings` on success, the empty-envelope JSON fallback on a pre-emit abort). The two handlers differ only in the `PipelineConfig` their `build_report` closure assembles.

What this closed (relative to the original A19 gap list):

- **Bespoke `Exit` enum** — gone. There is no `src/authoring/exit.rs`; the framework lint's terminal error maps through the one `Exit::from(&Error)` table in [src/runtime/output.rs](./src/runtime/output.rs).
- **Manual error / `eprintln!` paths** — gone. Neither handler writes its own `println!`/`eprintln!`; the abort fallback lives only in `output::finish_lint`.
- **Handler-shape divergence** — gone. Both handlers obey the same `Result<()>` contract documented in [handler-shape.md](./docs/standards/handler-shape.md).
- **Shared kernel** — landed. `output::run_lint` is the single build → emit → finish → blocking-gate kernel; `finish_lint` is internal to it.

Pinned wire contract (unchanged):

- `LintEmit::trailing_newline` — `true` for `specify lint project`, `false` for `specify lint framework` — preserves each surface's historical stdout shape. It is caller config, not normalised, until an intentional wire bump.
- `--format json` / `--output-format json` emit a `DiagnosticReport` on stdout even on infrastructure failure (empty all-zero envelope), so CI consumers keep a stable shape.

Verification: `cargo make check`; `cargo nextest run -p specify-standards --test lint_diagnostics_json --test lint_diagnostics_pretty`; `cargo nextest run -p specify lint`.

---

## A16 — Imperative→declarative lint burn-down — COMPLETE (steady state)

### Steady-state architecture

The engine is a generic dispatcher. Every framework `CORE-*` check is one of two roads (see [DECISIONS.md](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b)):

| Road | Ids (example) | How it runs |
| --- | --- | --- |
| Road A — declarative hint | most of `CORE-001..052` | a generic per-kind evaluator (`lint/eval/*`) interprets the rule's `kind:` (`schema`, `reference-resolves`, `cardinality`, `set-coverage`, `constant-eq`, `unique`, `fenced-block`, `regex`, `path-pattern`, `presence`, `field-grammar`, `cross-reference`, `cli-contract`) over `WorkspaceModel` facts; the mechanism selector rides `hint.value` (including the whole-tree `value: scenario` selector on `schema` and `unique`) and caps/sets/maps ride the rule's `config:` |
| Road B — referenced tool | `CORE-009`, `CORE-026`, the scenarios / skill-body / links-registry / marketplace / prose families | `kind: tool` value `<tool>` + a sentinel `path-pattern`; the engine resolves the tool by name (in-process framework checkers) and folds its `DiagnosticReport`. Policy rides the rule's `config:`, forwarded as a second positional arg |

`specify lint framework` resolves all `CORE-*` / `UNI-*` rules in one pass; no imperative `Check` producer runs on any invocation, and `kind: authoring-predicate` is not a valid rule kind.

### Policy lives in `specify`, not the engine

Every rule-specific value (a line cap, an owner→prefix map, an expected operation set, a canonical-doc path) rides the rule's `config:` in the `specify` repo — CORE-009's owner→prefix map, source-axis prefixes, and reserved-namespace owners included. The Layer-3 guard test [`crates/standards/tests/lint_engine_guards/no_embedded_policy.rs`](./crates/standards/tests/lint_engine_guards/no_embedded_policy.rs) fails if any eval arm reintroduces such a literal; the duplicated owner maps (`BUILTIN_NAMESPACES` / `TARGET_OWNERS`) are deleted.

### Framework tools

Six framework checkers — `scenarios`, `skill-body`, `links-registry`, `marketplace`, `prose`, `rules` — run in-process as native modules under [`crates/standards/src/lint/framework_tools/`](./crates/standards/src/lint/framework_tools.rs), inside `specify-standards`. The `kind: tool` evaluator resolves a checker name against the in-process `framework_tools` inventory before the `ToolRunner` trait, calling it directly for typed findings (see [DECISIONS.md §"Framework lint engine"](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b)).

### Performance

`make lint` on `augentic/specify` (release build, 2026-06-07): single-digit seconds — **~8s** wall (`real 8.7` for `make lint`, `real 7.8` for the bare release binary). Always measure against a `cargo build --release` binary; a debug/unoptimized build is many times slower and is not representative. Benchmark on your own hardware with `/usr/bin/time make lint`.

---

## Steady state

The lint surface is a generic dispatcher with no imperative `Check` substrate:

- Lint handlers share one `output::run_lint` kernel; `Exit::from(&Error)` is the only exit mapping on both binaries ([handler-shape.md](./docs/standards/handler-shape.md)).
- Road A declarative rules and Road B name-resolved checkers are the only producers; there is no `kind: authoring-predicate` and no `specify_standards::framework` substrate. Rust-quality predicates live dev-only at `tests/rust_quality/checks.rs`.
- No rule policy lives in the engine; the `lint_no_embedded_policy` Layer-3 guard enforces this. Coverage rests on the generic per-kind evaluator suite, the schema byte-match gate, and the tools' in-crate tests.

---

## Cross-repo touchpoints

| Topic | Repository | Files |
| --- | --- | --- |
| Steady-state posture | specify-cli | [DECISIONS.md §"Framework lint engine: generic dispatcher (Road A / Road B)"](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b), this file §"Steady state" |
| Contributor model + extension guide | specify | [docs/contributing/checks.md](https://github.com/augentic/specify/blob/main/docs/contributing/checks.md) |
| CORE rule authoring | specify | [adapters/shared/rules/core/README.md](https://github.com/augentic/specify/blob/main/adapters/shared/rules/core/README.md) |
| Declarative CORE rules | specify | `adapters/shared/rules/core/CORE-*.md` |
| Layer-3 policy guard | specify-cli | [crates/standards/tests/lint_engine_guards/no_embedded_policy.rs](./crates/standards/tests/lint_engine_guards/no_embedded_policy.rs) |
| Per-kind evaluator suite | specify-cli | `crates/standards/tests/lint_hint_*.rs` |
