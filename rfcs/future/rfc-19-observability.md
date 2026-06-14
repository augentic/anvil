# RFC-19: CLI Observability

> Status: Draft · Complements the shipped journal (`crates/workflow/src/journal.rs`); durable CLI behaviour in [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md)

## Abstract

Add first-class observability to the Specify CLI using Rust's `tracing` ecosystem. The CLI should emit structured, filterable diagnostics for command execution, slice lifecycle transitions, plan orchestration, workspace operations, and WASI tool runs without changing the existing stdout contract for humans, skills, or CI.

The design keeps command output and observability separate: `--format text|json` continues to govern the command result on stdout, while logs and spans go to stderr. Humans get quiet-by-default diagnostics with `-v` escalation; automation gets `SPECIFY_LOG` / `RUST_LOG` filtering and optional newline-delimited JSON logs. Libraries instrument with `tracing` spans and events; only the binary initialises a subscriber.

This is the **ephemeral diagnostics** half of observability. Its durable counterpart — the closed-taxonomy journal at `.specify/journal.jsonl` — has already shipped; see §"Relationship to the journal". `tracing` is currently not a workspace dependency, so this RFC is unimplemented apart from that overlap.

## Relationship to the journal

The journal (`crates/workflow/src/journal.rs`) is the **durable, closed-taxonomy event log**: a stable `EventKind` set (`slice.build.*`, `slice.synthesize.*`, `plan.reconcile.completed`, `plan.transition.approved`, `lint-completed`, …) appended to `.specify/journal.jsonl`, consumed by `specify plan status` for re-entry and earmarked as the substrate for [RM-14](../roadmap.md#rm-14-local-structured-workflow-events). It answers **"what happened"** as committed workflow state.

`tracing` is the **ephemeral, open-ended diagnostics channel**: stderr spans and events for debugging a single run, filtered by level and target, never a stable protocol. It answers **"how it happened"**.

The two are complementary and must not duplicate. A durable state transition emits a journal event (`slice.build.succeeded`); the surrounding decision points, timings, and remote calls are `debug`/`trace` spans. This RFC adds the tracing channel; it neither replaces the journal nor promotes any log line into the journal taxonomy.

## Motivation

Specify is a multi-command framework whose hard failures are easy to classify but whose operational failures are hard to reconstruct after the fact:

- a plan loop may select, skip, block, retry, or self-heal entries across several repos;
- workspace sync and push touch remotes, branches, PR state, and `.specify/workspace/` clones;
- slice merge and drop mutate managed state through several validation gates;
- declared WASI tools have fetch, cache, permission, runtime, stdout, and stderr boundaries;
- skills and humans rely on a stable JSON envelope, so diagnostic detail cannot leak into stdout;
- CI failures need enough context to debug without rerunning a whole loop interactively.

Today the CLI has a solid result contract — `--format text|json`, kebab-case JSON envelopes, stable error discriminants, four public exit codes (`src/runtime/output.rs`) — and a durable journal. What it lacks is an equally deliberate *diagnostics* channel. The result is a mix of terse terminal errors, command-specific text hints, and whatever stdout/stderr a child tool emits.

Rust already has a mature idiom: libraries emit structured spans and events through `tracing`; binaries install a `tracing-subscriber` layer with an `EnvFilter`; operators use `RUST_LOG`-style filters or verbosity flags; JSON logs are a formatting choice, not a separate output mode.

## Design

### Principles

1. **Stdout remains data.** Success bodies and JSON envelopes stay on stdout. Logs never write to stdout.
2. **Stderr carries diagnostics.** Text errors, hints, warnings, and tracing output share stderr.
3. **Default success stays quiet.** A successful command with no warnings should not become noisy.
4. **Structured logs are opt-in.** JSON logs are newline-delimited records on stderr, independent of `--format json`.
5. **Libraries instrument, binaries subscribe.** Workspace crates may call `tracing::{event, instrument, span}`; they must not initialise global subscribers.
6. **Filters use Rust conventions.** The primary machine surface is an `EnvFilter` string compatible with `tracing-subscriber`.
7. **Sensitive content is not logged.** Artifact bodies, prompts, source snippets, tokens, secrets, and full tool payloads stay out of events.
8. **Logs never enter the journal.** The journal taxonomy is closed and durable; tracing output is ephemeral and must not be promoted into it.

### Dependency shape

Add the workspace dependencies:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
```

`tracing` is safe for every workspace crate because it is a lightweight facade. `tracing-subscriber` is used only by the root binary (or a small binary-owned module). Do not add subscriber initialisation to `specify-error` (the dependency leaf), `specify-workflow`, `specify-standards`, `specify-tool`, or any other library crate.

If initialisation grows beyond a small module, add a `specify-observe` crate that depends only on `tracing` / `tracing-subscriber`, exposes `init(settings)`, and defines shared field names. It must not depend on `specify-error`; the leaf stays a leaf.

### CLI surface

Add global observability flags to `Cli`:

```bash
specify -v status
specify -vv plan validate
specify --log-filter specify=debug,specify_tool=trace,warn tool run contract
specify --log-format json --log-filter specify=info plan next
```

- `-v`, `--verbose` — counted global flag. One `-v` enables `info`, two `debug`, three or more `trace` for Specify targets.
- `-q`, `--quiet` — suppresses warn/info/debug/trace logs. Command errors still render through the existing error path.
- `--log-filter <filter>` — a `tracing-subscriber::EnvFilter` directive string; overrides verbosity flags.
- `--log-format <compact|pretty|json>` — stderr log rendering. Default `compact`; `pretty` for interactive debugging; `json` for CI and log collectors.

Environment variables and precedence:

- `SPECIFY_LOG` → `--log-filter`; `RUST_LOG` is the fallback when `SPECIFY_LOG` and `--log-filter` are absent; `SPECIFY_LOG_FORMAT` → `--log-format`; `NO_COLOR` disables ANSI colour.
- Precedence: explicit CLI flags → `SPECIFY_LOG` / `SPECIFY_LOG_FORMAT` → `RUST_LOG` → verbosity flags → default filter.

The default filter shows warnings from Specify targets and errors from dependencies, so a normal successful run emits nothing (success-path progress is `info` or lower):

```text
warn,specify=warn,specify_workflow=warn,specify_tool=warn,specify_standards=warn
```

### Target names

Use crate and module targets, not hand-authored product names. The crate graph is leaf-to-root (`specify-error → specify-schema → specify-diagnostics → specify-model → … → specify-workflow → specify`), so targets follow it:

- `specify` — the root binary and command dispatch (`src/runtime/`);
- `specify_workflow` — change/plan, slice lifecycle and merge, workspace, registry, adapter resolution, and journal emission (`crates/workflow/src/`);
- `specify_tool` — declared WASI tool resolution and execution;
- `specify_standards` — rule resolution and the `lint` scan;
- `specify_diagnostics` — diagnostic rendering and blocking decisions.

```bash
SPECIFY_LOG=specify_workflow=debug,specify_tool=trace,warn specify plan validate
```

### Field conventions

Every command enters a top-level span before dispatch:

```text
command{name="plan", action="next", format="json", run_id="..."}
```

| Field | Meaning |
| --- | --- |
| `run_id` | Unique identifier for this CLI process. |
| `command` / `action` | Top-level command and nested verb path (e.g. `slice.merge.run`). |
| `format` | Existing output format, `text` or `json`. |
| `project_dir` / `project_name` | Project root path / configured name when loaded. |
| `slice` / `change` / `plan` / `entry` | Workflow nouns when in scope. |
| `tool` / `tool_version` / `source_kind` / `cache_status` | Declared-tool resolution facts. |
| `duration_ms` / `exit_code` / `error` | Timing, final `Exit` code, stable kebab-case error discriminant. |

Prefer low-cardinality structured fields over prose; JSON consumers key on fields, so event messages stay short.

### Levels

| Level | Use |
| --- | --- |
| `error` | Unexpected internal failure after the CLI has classified the public error. Avoid double-reporting user-facing validation failures unless they carry extra context. |
| `warn` | Recoverable oddities: compatibility fallbacks, stale caches, ignored non-critical data, surprising actions. |
| `info` | High-level lifecycle movement: selected plan entry, transition applied, tool fetched, workspace peer prepared, merge completed. |
| `debug` | Decision points and bounded local detail: validation branch, computed paths, cache metadata, remote status summary. |
| `trace` | Fine-grained loops, per-file parsing, payload *shape* (never contents), small-step timing. |

A normal successful command should be understandable at `info`, debuggable at `debug`, and need `trace` only for maintainer-level investigation.

### Instrumentation points

Start with the workflows hardest to reconstruct, then expand where debugging pain remains:

- command start/finish in the `src/runtime/` dispatch;
- project layout load and version-floor checks in `specify-workflow`;
- slice create, transition, task marking, merge preview/conflict/run, archive, and drop (`crates/workflow/src/slice/`);
- plan create, validate, next, transition, archive, finalize, and the plan-lock probe (`crates/workflow/src/change/plan/`);
- registry add/remove/show/validate and workspace sync, status, push, clone refresh, branch preparation, PR create/update;
- adapter resolve and pipeline selection (`crates/workflow/src/adapter/`);
- tool fetch, cache lookup, permission check, component load, WASI invocation, and guest exit status (`specify-tool`).

Do not instrument every helper at once: cover command boundaries plus the cross-repo and WASI paths first.

### Error correlation

Generate a `run_id` once at process start and attach it to the top-level span. Keep `run_id` in logs only for the first implementation; do not add it to JSON success or error envelopes (that would change the public wire shape). If direct error-to-log correlation is later needed, add an optional `diagnostic-id` to JSON error envelopes and bump `ENVELOPE_VERSION` if that additive field is treated as a contract change.

### WASI tool boundary

`specify tool run` has three streams: host result on stdout (governed by `--format`), host diagnostics on stderr (this RFC), and guest stdout/stderr (the declared tool contract). The host must not parse or reformat guest output as tracing events; it may log host-side facts (resolved source, cache hit/miss, permission decision, load/invocation duration, guest exit code, whether streams were non-empty). It must not log guest payload contents unless a future tool contract declares a bounded diagnostic channel.

### Redaction

Never log artifact body text (`spec.md`, `design.md`, `tasks.md`, `change.md`, contracts, source files); prompts, model outputs, or retrieved context; environment values except a short allowlist of non-secret booleans/modes; authorization headers, tokens, cookies, keys, or forge credentials; full HTTP bodies; or user source snippets. Paths are acceptable in local debug logs but avoid path fields at `info` unless the path identifies a managed artifact the command already reports. When a value may contain user content, log a count, hash, enum, or stable id instead.

### Metrics, OpenTelemetry

Specify is a short-lived CLI; the first observability layer expresses metrics as span-close and summary events (command duration/exit code, plan entries inspected/selected/resumed/completed, files parsed and findings counted, cache hit/miss/stale/install, tool fetch/run durations, workspace peers synced and PRs created/updated). JSON consumers aggregate these in CI. Do not add a metrics server, daemon, or OpenTelemetry export in the first implementation; the `tracing` API keeps an OTel bridge possible later behind an optional feature.

## Implementation plan

1. Extend `Cli` with counted verbosity, quiet mode, `--log-filter`, and `--log-format`; wire the environment variables through clap.
2. Initialise the subscriber once in `main`/`src/runtime` before command dispatch.
3. Add tests proving default success, `--format json` success, and JSON error envelopes are unchanged when logs are disabled.
4. Wrap dispatch in a top-level span with `run_id`, command, action, format, and final exit code.
5. Instrument the plan, workspace, slice merge/drop, and tool fetch/run paths with `info`/`debug`/`warn` events — distinct from the journal events those paths already emit.
6. Add JSON-log tests (`SPECIFY_LOG=specify=info` + `--log-format json`): logs are valid newline-delimited JSON on stderr; stdout stays valid command JSON.
7. Add redaction tests over representative secrets and artifact bodies.
8. Document operator usage in `docs/standards/architecture.md` and the troubleshooting docs.

## Migration

- **Operators:** command output and exit codes are unchanged; debug with `-v` / `-vv` / `SPECIFY_LOG=specify=debug`; use `--log-format json` only when stderr is captured.
- **Skill authors:** keep parsing stdout and relying on exit codes / JSON envelopes; rerun with `SPECIFY_LOG=specify=debug` for detail; do not scrape human text logs as a protocol.
- **CLI maintainers:** instrument library crates with `tracing`, initialise subscribers only in the binary; keep stdout writes inside the existing `Render` / envelope paths; prefer `tracing` events over new `eprintln!`; treat documented field names as part of the observability contract.

## Alternatives considered

- **`log` + `env_logger`.** Rejected — loses spans, structured fields, span timings, and a clean OTel path.
- **A `--format trace-json` output mode.** Rejected — `--format` means command-result shape; overloading it breaks the stdout contract skills depend on.
- **Logs on stdout in JSON mode.** Rejected — stdout is machine-readable command data; interleaving corrupts the envelope.
- **OpenTelemetry immediately.** Deferred — local/CI debuggability is the first need; `tracing` keeps an OTel bridge possible without a heavier default CLI.
- **Persisting telemetry under `.specify/` by default.** Rejected for v1 — that is the journal's durable role; ephemeral logs stay ephemeral.

## Non-Goals

- Changing the public exit-code table or the default JSON envelope.
- Making logs a stable machine protocol for skills, or promoting log lines into the journal taxonomy.
- Capturing or storing prompt/model telemetry.
- Building a dashboard, log collector, background daemon, metrics server, or long-running exporter.
- Parsing arbitrary WASI guest stderr into host events.

## Open Questions

1. Should `run_id` use `uuid` or a dependency-free timestamp/process/random suffix?
2. Should `SPECIFY_LOG` fully override `RUST_LOG`, or merge with `SPECIFY_LOG` precedence only for Specify targets?
3. Should `project_dir` appear in JSON logs at `debug`, or should absolute paths require `trace`?
4. Should `--quiet` suppress `tracing` warnings, or only info/debug/trace?
5. Should text-mode final errors include a `diagnostic-id` when logs are enabled, before JSON envelopes gain one?
6. Which field names should be documented as stable before the first release that includes observability?

## References

- [`docs/standards/architecture.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/architecture.md) and [`docs/standards/handler-shape.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/handler-shape.md) — the `Ctx` / `Render` / `emit` and exit-code shape this RFC must not disturb.
- [`src/runtime/output.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/output.rs) — the `Exit::from(&Error)` mapping and stdout contract.
- [`crates/workflow/src/journal.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/journal.rs) — the durable event log this RFC complements.
