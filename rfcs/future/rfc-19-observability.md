# RFC-19: CLI Observability

> Status: Draft - Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-15](archive/rfc-15-wasm-plugins.md)

## Abstract

Add first-class observability to the Specify CLI using Rust's `tracing` ecosystem. The CLI should emit structured, filterable diagnostics for command execution, slice lifecycle transitions, plan orchestration, workspace operations, and WASI tool runs without changing the existing stdout contract for humans, skills, or CI.

The design keeps command output and observability separate: `--format text|json` continues to govern the command result on stdout, while logs and spans go to stderr. Humans get quiet-by-default diagnostics with `-v` escalation. Automation gets `SPECIFY_LOG` / `RUST_LOG` filtering and optional newline-delimited JSON logs. Libraries instrument with `tracing` spans and events; only the binary initializes a subscriber.

## Motivation

Specify is now a multi-command framework whose hard failures are usually easy to classify, but whose operational failures can be hard to reconstruct after the fact:

- a plan loop may select, skip, block, retry, or self-heal entries across several repos;
- workspace sync and push touch remotes, branches, PR state, and `.specify/workspace/` clones;
- slice merge and drop operations mutate managed state through several validation gates;
- declared WASI tools have fetch, cache, permission, runtime, stdout, and stderr boundaries;
- skills and humans rely on a stable JSON envelope, so diagnostic detail cannot leak into stdout;
- CI failures need enough context to debug without rerunning a whole migration loop interactively.

Today the CLI has a solid result contract: `--format text|json`, kebab-case JSON envelopes, stable error discriminants, and four public exit codes. What it lacks is an equally deliberate diagnostics channel. The result is a mix of terse terminal errors, command-specific text hints, and whatever stdout/stderr a child tool emits.

Rust already has a mature idiom for this: libraries emit structured spans and events through `tracing`; binaries install a `tracing-subscriber` layer with an `EnvFilter`; operators use `RUST_LOG`-style filters or verbosity flags; JSON logs are a formatting choice, not a separate output mode. Specify should follow that idiom instead of inventing a bespoke logging API.

## Design

### Principles

Observability must preserve the existing CLI contract:

1. **Stdout remains data.** Success bodies and JSON envelopes stay on stdout. Logs never write to stdout.
2. **Stderr carries diagnostics.** Text errors, hints, warnings, and tracing output share stderr.
3. **Default success stays quiet.** A successful command with no warnings should not become noisy.
4. **Structured logs are opt-in.** JSON logs are newline-delimited records on stderr, independent of `--format json`.
5. **Libraries instrument, binaries subscribe.** Workspace crates may call `tracing::{event, instrument, span}`. They must not initialize global subscribers.
6. **Filters use Rust conventions.** The primary machine surface is an `EnvFilter` string compatible with `tracing-subscriber`.
7. **Sensitive content is not logged.** Artifact bodies, prompts, source snippets, tokens, secrets, and full tool payloads stay out of events.

### Dependency Shape

Add the following workspace dependencies to `specify-cli`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
```

`tracing` is safe for every workspace crate because it is a lightweight facade. `tracing-subscriber` should be used only by the root binary, or by a small helper crate/module owned by the binary. Do not add subscriber initialization to `specify-error`, `specify-change`, `specify-slice`, `specify-tool`, or other library crates.

If the initialization code grows beyond a small module, add a `specify-observe` workspace crate with no dependency on domain crates. It may depend on `tracing` and `tracing-subscriber`, expose an `init(settings)` function, and define shared field names. It must not depend on `specify-error`; `specify-error` stays a leaf.

### CLI Surface

Add global observability flags to `Cli`:

```bash
specify -v status
specify -vv plan validate
specify --log-filter specify=debug,specify_tool=trace,warn tool fetch contract
specify --log-format json --log-filter specify=info plan next
```

Target clap shape:

- `-v`, `--verbose` as a counted global flag. One `-v` enables `info`, two enable `debug`, three or more enable `trace` for Specify targets.
- `-q`, `--quiet` suppresses warning/info/debug/trace logs from the subscriber. Command errors still render through the existing error path.
- `--log-filter <filter>` accepts a `tracing-subscriber::EnvFilter` directive string. It overrides verbosity flags.
- `--log-format <compact|pretty|json>` controls stderr log rendering. Default is `compact`; `pretty` is for interactive debugging; `json` is for CI and log collectors.

Environment variables:

- `SPECIFY_LOG` is the Specify-specific filter and maps directly to `--log-filter`.
- `RUST_LOG` is accepted as a fallback when `SPECIFY_LOG` and `--log-filter` are absent.
- `SPECIFY_LOG_FORMAT` maps to `--log-format`.
- `NO_COLOR` should disable ANSI color in text log formats when the selected layer would otherwise use color.

Precedence:

1. Explicit CLI flags.
2. `SPECIFY_LOG` / `SPECIFY_LOG_FORMAT`.
3. `RUST_LOG`.
4. Verbosity flags.
5. Default filter.

The default filter should show warnings from Specify targets and errors from dependencies:

```text
warn,specify=warn,specify_cli=warn,specify_change=warn,specify_slice=warn,specify_tool=warn
```

In normal successful runs, this emits nothing because success-path progress events are `info` or lower.

### Target Names

Use crate and module targets rather than hand-authored product names. Examples:

- `specify` for the root binary;
- `specify_change` for change umbrella and plan-entry events;
- `specify_slice` for slice lifecycle and merge events;
- `specify_tool` for declared WASI tool resolution and execution;
- `specify_registry` for registry validation and peer operations;
- `specify_config` for project layout and version-floor diagnostics.

This keeps filters unsurprising:

```bash
SPECIFY_LOG=specify_change=debug,specify_tool=trace,warn specify plan validate
```

### Field Conventions

Every command should enter a top-level span before dispatch:

```text
command{name="plan", action="next", format="json", run_id="..."}
```

Common fields:

| Field | Meaning |
| --- | --- |
| `run_id` | Unique identifier for this CLI process. |
| `command` | Top-level CLI command. |
| `action` | Nested verb path, such as `slice.merge.run` or `change.plan.next`. |
| `format` | Existing output format, `text` or `json`. |
| `project_dir` | Project root path when needed for local debugging. |
| `project_name` | Configured project name when loaded. |
| `slice` | Slice name. |
| `change` | Change name. |
| `plan` | Plan name. |
| `entry` | Plan entry id or slice name, depending on the plan shape. |
| `tool` | Declared tool name. |
| `tool_version` | Declared tool version. |
| `source_kind` | Tool source kind such as `file`, `https`, or `oci`. |
| `cache_status` | `hit`, `miss`, `stale`, or `installed`. |
| `duration_ms` | Elapsed duration for explicit timing events. |
| `exit_code` | Final `Exit` code. |
| `error` | Stable kebab-case error discriminant when available. |

Prefer low-cardinality structured fields over prose. Event messages should be short, because JSON log consumers should primarily key on fields.

### Levels

Use levels consistently:

| Level | Use |
| --- | --- |
| `error` | Unexpected internal failure after the CLI has classified the public error. Avoid double-reporting every user-facing validation failure as an error log unless it carries extra diagnostic context. |
| `warn` | Recoverable oddities, compatibility fallbacks, stale caches, ignored non-critical data, or actions that may surprise an operator. |
| `info` | High-level lifecycle movement: selected plan entry, transition applied, tool fetched, cache installed, workspace peer prepared, merge completed. |
| `debug` | Decision points and bounded local detail: validation branch, computed paths, cache metadata comparison, remote status summary. |
| `trace` | Very fine-grained loops, per-file parsing, command payload shape without payload contents, and timing around small internal steps. |

As a rule, a normal successful command should be understandable at `info`, debuggable at `debug`, and only need `trace` for maintainer-level investigation.

### Instrumentation Points

Start with the workflows that are hardest to reconstruct:

- command start and finish in `src/commands.rs`;
- project config load and version-floor checks in `specify-config`;
- slice create, transition, task marking, outcome setting, merge preview, conflict check, merge run, archive, and drop in `specify-slice` / `specify-merge`;
- plan create, validate, next, transition, archive, finalize, and lock operations in `specify-change`;
- `/spec:finalize` remote verification;
- registry add, remove, show, and validate;
- workspace sync, status, push, clone refresh, branch preparation, and PR creation/update calls;
- adapter resolve, check, and pipeline selection;
- tool fetch, cache lookup, permission check, component load, WASI invocation, and guest exit status.

Do not instrument every helper at once. The first implementation should cover command boundaries plus the cross-repo and WASI paths, then expand where debugging pain remains.

### Error Correlation

Generate a `run_id` once at process start and attach it to the top-level command span. For the first implementation, keep `run_id` in logs only. Do not add it to every JSON success or error envelope, because that would change the public wire shape.

If operators later need direct error-to-log correlation, add an optional `diagnostic-id` field to JSON error envelopes and bump `ENVELOPE_VERSION` if the repository treats that additive field as a contract change. Text errors may include a short trailing line only when diagnostics were enabled:

```text
diagnostic-id: 20260511T004212Z-12345
```

This keeps the initial rollout observability-only and avoids coupling the log design to the result envelope.

### WASI Tool Boundary

`specify tool run` has three distinct streams:

1. Host command result on stdout, governed by `--format`.
2. Host diagnostics on stderr, governed by this RFC.
3. Guest stdout/stderr, governed by the declared tool contract and the tool itself.

The host must not parse or reformat guest output as tracing events in the first implementation. It may log host-side facts around the invocation: resolved source, cache hit/miss, permission decision, component load duration, invocation duration, guest exit code, and whether stdout/stderr were non-empty. It must not log guest payload contents unless a future tool contract declares a bounded diagnostic channel.

For JSON log mode, host log records remain newline-delimited JSON on stderr. Guest stderr may be arbitrary text on stderr, so CI consumers that require pure JSON logs should run tool diagnostics in a mode that routes guest output separately. That separation can be a future enhancement.

### Redaction Policy

Never log:

- artifact body text from `spec.md`, `design.md`, `tasks.md`, `change.md`, contracts, or source files;
- prompts, model outputs, or retrieved context chunks;
- environment variable values except a short allowlisted set of non-secret booleans or modes;
- authorization headers, tokens, cookies, SSH keys, private registry credentials, or forge tokens;
- full HTTP request/response bodies;
- user source code snippets.

Paths are acceptable in local debug logs, but avoid path fields at `info` unless the path identifies a managed artifact the command already reports. For cross-project logs, prefer project names, registry keys, slice names, plan entries, tool names, and cache status.

When a value may contain user-controlled content, log a count, hash, enum, or stable id instead of the raw value.

### Metrics

Specify should not run a metrics server or daemon. It is a short-lived CLI, so the first observability layer should express metrics as structured span close events and summary events:

- command duration and exit code;
- plan entries inspected, selected, resumed, and completed in v1; skipped, blocked, and failed counters return only if those terminal states are reintroduced;
- files parsed and validation findings counted;
- cache hit/miss/stale/install counts;
- tool fetch and run durations;
- workspace peers synced and PRs created or updated.

JSON log consumers can aggregate these records in CI. If recurring operations need local historical metrics, add a separate explicit artifact later, such as `.specify/telemetry/`, with its own opt-in policy. Do not quietly persist telemetry in the first implementation.

### OpenTelemetry

Do not add OpenTelemetry export in the first implementation. The Rust `tracing` API keeps the instrumentation compatible with an OpenTelemetry bridge later, but a default CLI should avoid background exporters, batch flushing complexity, service names, endpoint configuration, and larger dependency trees until a real deployment target needs them.

A future RFC may add an optional `otel` feature or a separate wrapper binary for long-running orchestrators. That work should build on the same spans and field names defined here.

## Implementation Plan

1. **Add the observability settings.** Extend `Cli` with counted verbosity, quiet mode, `--log-filter`, and `--log-format`. Wire the corresponding environment variables through clap.
2. **Initialize tracing once.** In `main`, parse `Cli`, initialize the subscriber before command dispatch, and then call `commands::run(cli)` as today.
3. **Preserve stdout tests.** Add tests proving that default success output, `--format json` success output, and JSON error envelopes are unchanged when logs are disabled.
4. **Add command spans.** Wrap dispatch in a top-level span with `run_id`, command, action, format, and final exit code.
5. **Instrument critical workflows.** Add `info`/`debug`/`warn` events to plan, workspace, slice merge/drop, and tool fetch/run paths.
6. **Add JSON log tests.** Exercise `SPECIFY_LOG=specify=info` plus `--log-format json`, assert logs go to stderr as valid newline-delimited JSON, and assert stdout remains valid command JSON.
7. **Add redaction tests.** Cover representative secrets and artifact bodies to ensure they do not appear in stderr logs.
8. **Document operator usage.** Update CLI architecture docs, troubleshooting docs, and any skill guidance that recommends rerunning commands with diagnostics.

## Migration

For operators:

- Existing command output and exit codes remain unchanged.
- Debug a command with `-v`, `-vv`, or `SPECIFY_LOG=specify=debug`.
- Use `--log-format json` only when stderr is captured by CI or a log collector.

For skill authors:

- Continue to parse stdout only.
- Continue to rely on existing exit codes and JSON envelopes.
- When a skill needs more diagnostic detail, rerun the same CLI command with `SPECIFY_LOG=specify=debug` and inspect stderr. Do not scrape human text logs as a protocol.

For CLI maintainers:

- Instrument library crates with `tracing`; initialize subscribers only in the binary.
- Keep `println!` / stdout writes inside the existing `Render` and JSON envelope paths.
- Avoid `eprintln!` for new diagnostics except the existing final error and hint renderer. Prefer `tracing` events so filters and JSON logs work uniformly.
- Treat field names as part of the observability contract once documented.

## Alternatives Considered

**Use `log` plus `env_logger`.** Rejected. It is familiar, but it loses spans, structured fields, span close timings, and a clean path to OpenTelemetry. `tracing` is the idiomatic choice for modern Rust applications that need structured diagnostics across library boundaries.

**Add a new `--format trace-json` output mode.** Rejected. `--format` already means command result shape. Overloading it with logs would break the stdout contract that skills depend on.

**Write logs to stdout in JSON mode.** Rejected. Stdout is machine-readable command data. Interleaving logs would corrupt the JSON envelope and force every caller to demultiplex streams.

**Adopt OpenTelemetry immediately.** Deferred. The first need is local and CI debuggability. `tracing` instrumentation keeps an OpenTelemetry bridge possible without making the default CLI heavier.

**Persist telemetry under `.specify/` by default.** Rejected for the first implementation. Quietly writing historical telemetry changes the managed-state surface and raises retention questions. Explicit log capture is enough for the initial use case.

**Depend on `tracing-log` to bridge existing `log` users.** Deferred. Specify does not currently have a meaningful internal `log` surface to bridge. Add it later only if a dependency's diagnostics need to be captured.

## Non-Goals

- Changing the public exit-code table.
- Changing the default JSON envelope.
- Making logs a stable machine protocol for skills.
- Capturing or storing prompt/model telemetry.
- Building a dashboard or log collector.
- Adding a background daemon, metrics server, or long-running exporter.
- Parsing arbitrary WASI guest stderr into host events.
- Replacing existing command result rendering through `Render` and `emit`.

## Open Questions

1. Should `run_id` use a dependency such as `uuid`, or a dependency-free timestamp/process/random suffix generated by the binary?
2. Should `SPECIFY_LOG` override `RUST_LOG`, or should the CLI merge both filters with `SPECIFY_LOG` taking precedence only for Specify targets?
3. Should `project_dir` appear in JSON logs by default at `debug`, or should absolute paths require `trace`?
4. Should `--quiet` suppress warnings emitted through `tracing`, or only suppress info/debug/trace while preserving warnings?
5. Should text-mode final errors include a `diagnostic-id` when logs are enabled, even before JSON envelopes gain one?
6. How should CI capture host JSON logs when `specify tool run` also forwards arbitrary guest stderr?
7. Which field names should be documented as stable before the first release that includes observability?

## References

- [RFC-1: CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-15: WASI Adapter Tools](archive/rfc-15-wasm-plugins.md)
- `specify-cli/src/output.rs`
- `specify-cli/docs/contributing/cli-architecture.md`
