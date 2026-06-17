# Handler shape

The contract every CLI command handler obeys: how `Ctx` is constructed, how output flows through `ctx.write` / `output::write` / `emit`, which exit code a terminal `Error` maps to, and what the dispatcher between `clap` and the workspace crates is allowed to do.

## Ctx construction

Handlers take `&Ctx`, whose module path `crate::runtime::context::Ctx` carries the noun. `Ctx` exposes the resolved project dir, layout, output format, and a few thin facade methods for handler ergonomics; everything else flows through workspace crates. `Layout<'a>` lives on `Ctx` rather than at call sites so path helpers stay anchored in `specify-workflow` — see [architecture.md §"Layout boundary"](./architecture.md#layout-boundary).

## Default handler signature

Command handlers default to `Result<()>` (success-path conversion happens at the dispatcher boundary). Surface non-success exits through typed errors that `Exit::from(&Error)` maps to the five-slot exit table — do **not** return `Result<Exit>` to thread a non-zero code by hand.

```rust
// GOOD — default shape
pub(crate) fn handle(ctx: &Ctx, args: &SomeArgs) -> Result<()> {
    let body = some_crate::do_work(ctx.layout(), args)?;
    ctx.write(&SomeBody::from(&body), write_text)?;
    Ok(())
}

// GOOD — explicit Result<Exit> only when the handler needs a
// non-success exit and a typed *ErrBody (rare — workspace::push is one).
pub(crate) fn handle(ctx: &Ctx) -> Result<Exit, Error> { /* ... */ }
```

A free `fn ... -> Result<Exit>` belongs in `src/runtime/commands.rs`. Elsewhere, default to `Result<()>` and let the dispatcher collapse the success path.

## ctx.write, output::write, and emit

Success bodies leave handlers via `ctx.write(&body, write_text)?;`. `Ctx::write` chooses the JSON vs text path based on `Format`; the handler never sees the branch. The `write_text` closure has signature `FnOnce(&mut dyn Write, &T) -> std::io::Result<()>` and is colocated with each handler so the response shape stays in a single block of code; the JSON path goes through `serde::Serialize` automatically.

Handlers never pick a stdout/stderr sink directly — `Ctx::write` (the success path), `output::report` (the failure path), and the free `output::emit` (the rare format-only path) are the sink-bearing entry points. Format-only handlers that run before (or outside of) a `Ctx` — `commands::init::run` and the unified `commands::resolve_adapter` shared by `source resolve` / `target resolve` (the source/target adapter resolve verbs) — receive a bare `Format` and call `output::emit(&mut std::io::stdout().lock(), format, &body, write_text)?;` directly because `Ctx::write` is not available.

For the full DTO and dispatch rules see [coding-standards.md §"Format dispatch"](./coding-standards.md#format-dispatch), [§"One emit path"](./coding-standards.md#one-emit-path), and [§"DTOs"](./coding-standards.md#dtos).

### Gate handlers render, then fail payload-free

Check surfaces that gate on findings — `slice validate`, the lint sentinels — own their rendering. They collect `Vec<Diagnostic>`, assemble a `DiagnosticReport`, and render it on **stdout** via `ctx.write` (the success sink), then, if any diagnostic blocks, return a payload-free `Error::validation_failed(code, detail)` purely to carry exit 2 and the discriminant on stderr. `Error::Validation` is `{ code, detail }` with no findings payload — the rich report already went to stdout. Single operational errors that are not findings (e.g. `tool-not-declared`, `discovery-lead-unknown`) take the same payload-free shape but render no report. The blocking decision uses the uniform predicate (`kind == violation && status == open && severity ∈ {critical, important}`); `kind: review` diagnostics surface but never block. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](../../DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

### The two lint handlers share one tail

`specify lint project` and `specify lint framework` are the same handler shape with different pipeline config. Both return `Result<()>` and call the one `run_lint` kernel in [`src/output.rs`](../../src/output.rs), passing the format plus a `build` closure that assembles surface-specific `ResolveInputs` + `PipelineConfig` and calls `emit_lint_report`. Inside the kernel: `emit_lint_report` runs the pipeline and renders the envelope on stdout; the internal `finish_lint` collapses the outcome into the terminal `Result<()>` — `deny_blocking_findings` on success, the empty-envelope stdout fallback on a pre-emit abort. The fallback owns only the **stdout** side (an all-zero `DiagnosticReport`, JSON only, so CI consumers keep a stable shape); the stderr `error: …` line is the dispatcher's `output::report`, so the two sinks compose without double-printing. Neither handler writes its own `println!`/`eprintln!`.

`specify lint framework` (the framework authoring lint) is just another action on the one `specify` binary: it obeys this same `Result<()>` contract and maps its terminal error through the one `Exit::from(&Error)` table in [`src/runtime/output.rs`](../../src/runtime/output.rs) exactly as every other verb does. Only bootstrap verbs (`upgrade`) justify a bespoke exit subset; lint does not.

## Exit codes

The four-slot CLI exit-code table is fixed:

| Code | Name | When |
|---|---|---|
| 0 | `EXIT_SUCCESS` | Command succeeded |
| 1 | `EXIT_GENERIC_FAILURE` | Default `Error` → exit 1 |
| 2 | `EXIT_VALIDATION_FAILED` | `Error::Validation`, undeclared/over-permissioned tool, `Error::Argument` |
| 3 | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` (`specify-version-too-old` in JSON) |

`Exit::from(&Error)` in [`src/runtime/output.rs`](../../src/runtime/output.rs) is the single source of truth. Every dispatcher in `src/runtime/commands/*` routes its terminal error through `report`, which calls `Exit::from`. Do not invent new exit codes. The long-form decision (including `Exit::Code(u8)`'s WASI passthrough role) lives in [DECISIONS.md §"Exit codes"](../../DECISIONS.md#exit-codes).

## Dispatcher contract

`src/runtime/cli.rs` declares the clap derive surface. Every command has a doc comment that doubles as `--help` output — keep it accurate and operator-facing (no internal jargon or historical labels). Add new commands as enum variants on `Commands` with a nested action enum where the verb has subactions; mirror existing groups (`SliceAction`, `PlanAction`, `SourceAction`, `TargetAction`, …).

`--source key=value` arguments are parsed via the typed `SourceArg` (`impl FromStr for SourceArg`) so call sites read named fields instead of tuple positions.

Dispatchers live in `src/runtime/commands/<verb>.rs` and call back into the workspace crates. The discipline is:

1. Clap parses argv → `Commands` enum.
2. `src/runtime/commands.rs` matches the variant and calls the dispatcher in `src/runtime/commands/<verb>.rs`.
3. The dispatcher loads `ProjectConfig` (which enforces the `specify_version` floor for free) and any other state it needs.
4. The dispatcher delegates the deterministic work to a workspace crate (`specify_slice`, `specify_change`, etc.) and converts the result to a `*Body` for `ctx.write(&body, write_text)`.

Failure envelopes leave handlers as `Err(Error::*)`; the dispatcher in `src/runtime/commands.rs` routes them through `output::report(format, &err)`. No handler writes its own stderr envelope.

Never put domain logic in the binary. If a function needs unit tests, it belongs in a workspace crate. The binary owns argv parsing, formatting, and dispatch only. For the crate dependency direction this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Adapter-resolve verb shapes

`source resolve <name>` and `target resolve <value>` are format-only
handlers — both clap arms in `src/runtime/commands.rs` dispatch to a single
private `commands::resolve_adapter(format, axis, value, project_dir)`
helper that takes a bare `Format` plus the project dir, switches on
`axis` to invoke `specify_workflow::adapter::SourceAdapter::resolve(name,
project_dir)?` or `TargetAdapter::resolve(name, project_dir)?`, and
emits a `ResolveBody { axis, name, resolved_path, location,
operations, description }` via the direct `output::emit` path
described above. They never load a `Ctx`, because adapter resolution
is read-only and runs before any project mutation. The unified helper
peels an opaque `@version` suffix only on `Axis::Target` (per the workflow contract
§CLI surface); the axis discriminator is otherwise the sole branch,
so adding a third axis later is a one-extra-arm addition to the
existing `match`.

`plan amend` extends the canonical `with_state::<Plan, _, _>(...)`
handler shape with the three `--sources` flag families
axis: `--sources <binding>...` (wholesale replace), `--add-source
<binding>` (repeatable), `--remove-source <key>` (repeatable). The
parser routes `--add-source` / `--remove-source` *after* the
wholesale `Plan::amend(name, patch)` call so wholesale replacement
plus targeted edits compose cleanly in a single invocation. The
`--divergence` flag accepts only `accepted | rejected` from the
wire and emits a `plan.amend.divergence` journal event when (and
only when) the field flips — see [DECISIONS.md §"Journal event
names"](../../DECISIONS.md#journal-event-names).

`plan transition <name> <target>` is one verb that dispatches on
the operands: `<plan-name> approved` is the Gate 1 stamp and emits
a `plan.transition.approved` journal event; `<entry-name> done` is
the per-entry close (`/spec:merge` is the canonical caller).
Anything else is an `Error::Argument` (exit 2). The journal append
runs *after* `with_state` returns so the plan write and the journal
append cannot interleave on failure.

## Gotcha — `specify init` and the version floor

`specify init` bypasses the `specify_version` floor check (the file doesn't exist yet); every other project-aware verb inherits it for free via `ProjectConfig::load`. Don't reimplement the floor check at a subcommand site.
