# Prompt evaluation

The lab-only `probe` library: the typed eval case runner, deterministic
grading, and telemetry over the native host. The composition example that
drives it is [`examples/eval/`](../../examples/eval/README.md)
(`cargo make eval` / `cargo make specify`). Outputs are graded by
deterministic validators — not a model.

## Quick start

Login to the Cursor agent:

```bash
[cursor-]agent login
```

or set `CURSOR_API_KEY` in `.env` at the repository root.

```bash
cargo make eval                       # list the cases
cargo make eval auth --restart        # run the engine's auth workflow case
cargo make eval auth --restart --until plan   # stop at Gate 1
```

Each case keeps one stable retained sandbox at
`examples/eval/sandbox/<case>/`, on success and failure alike. `--restart`
is the only runner-owned reset: it replaces that case's sandbox before
running. An existing sandbox without `--restart` refuses before mutation —
the runner never infers workflow progress from an existing tree. Continue
or debug a retained sandbox explicitly through command passthrough:

```bash
cargo make specify -- --project-dir examples/eval/sandbox/auth plan approve
cargo make specify -- --project-dir examples/eval/sandbox/auth plan execute
```

Driver-side knobs (read by `probe::client`):

| Env                       | Effect                                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `EVAL_MODEL=<model-id>`   | Override the model for a run; unset means the cursor backend's default.                                         |
| `EVAL_TIMEOUT_SECS=<u64>` | Per-spawn `cursor-agent` wall-clock bound (seconds). Unset → backend default 120. `cargo make eval` sets `300`. |
| `RUST_LOG=<filter>`       | `tracing` filter for the native composition (`probe::client` initializes `omnia::Telemetry`). Example: `info,opentelemetry_sdk=off`. |
| `OTEL_GRPC_URL=<url>`     | Optional OTLP gRPC endpoint; unset uses OpenTelemetry defaults (`http://localhost:4317`).                       |

With `OTEL_GRPC_URL` set, a run emits `eval.case` (this crate),
`specify.command` (the transport router), the engine orchestration spans
(`plan.author`, `plan.execute.entry`, `slice.refine` / `slice.build` /
`slice.merge`, `source.survey` / `source.extract`, `judgment.leg`), and
`model.request` (the cursor backend) — every span carries only bounded
labels (case id/kind, command label, slice/adapter ids, judgment leg,
repair count, effective model id, exit code), never raw argv,
intent/source values, prompts, or project paths. The client initializes
`omnia::Telemetry` once and calls `omnia::telemetry::flush` before exit,
so even a fast `cargo make specify -- slice list` flushes its span.

## Cases

A case is a data directory under the composition root's `cases/` tree —
`cases/<id>/case.toml` plus (usually) a sibling `fixture/` copied into the
fresh sandbox. Two kinds exist:

- **`kind = "workflow"`** — the operator rhythm over real verbs:
  `init <target>`, `plan author <change> [--intent] [--source k=v …]`,
  then (past `--until plan`) `plan approve` and the genuine drained
  `plan execute`, and (at `--until finalize`) `plan archive`. The default
  stop is `execute`; `case.toml`'s `until` sets a case default and
  `--until` overrides per run. An optional `clone = { url, dest }`
  (mutually exclusive with `fixture`) shallow-clones an upstream tree
  into the case's own `fixture/<dest>` on first run (stripping
  `.git`) and reuses that gitignored cache afterwards — for source
  trees that cannot ship as committed fixtures; delete the cached
  tree to refresh the snapshot.
- **`kind = "build"`** — one `specify slice build <slice>` against a
  committed refined fixture (valid project + slice metadata; the runner
  never stamps lifecycle state), then the built gates.

The auth case's hard synthesis case is **authority divergence** — the
`session-timeout` lead (scripted goldens name the slice `session-policy`),
not evidence volume. Docs claim a 30-minute idle timeout; code claims 15;
documentation authority should win. Grading still only asserts lifecycle +
provenance; it does not require a `[divergence]` tag.

`judgment-model-failed` on refine usually means the cursor layer timed out
or returned unparseable JSON — the engine `MAX_REPAIRS` loop never sees
that failure.

## Model judgment

Specify core has two steps that require a model's judgement.


| Schema      | Step       | Crate    | Purpose                                                                                                                                     |
| ----------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `proposal`  | Propose    | `change` | Reconcile *surveyed* leads across sources into plan slices and author the Gate 1 prose.                                                     |
| `synthesis` | Synthesize | `slice`  | Reconcile *extracted* evidence with baseline context and target guidance, to produce build artifacts such as `proposal.md`, `spec.md`, etc. |


The execution and repair loop lives in the `project` crate and are considered infrastructure, not judgment. Source `survey` / `extract` and target `guidance` / `build` / `merge` are adapter operations.

A workflow case stopped at `--until plan` isolates the proposal leg. A full
workflow case runs synthesis as part of the complete `refine → build →
merge` loop. The synthesis artifacts are fields of one model response, so
the harness does not invoke `spec.md` or `design.md` generation as
independent judgment legs.

## Gates

Every command must exit successfully; the case kind adds only its
observable gates:


| Case kind          | Check      | Pass condition                                                       |
| ------------------ | ---------- | -------------------------------------------------------------------- |
| workflow (plan)    | Entries    | `plan author` produces at least one entry; lifecycle stays `pending` |
| workflow (execute) | Lifecycle  | Every plan entry is `done`                                           |
| workflow (execute) | Provenance | Every evidenced requirement carries sources; ids are present         |
| build              | Lifecycle  | Slice metadata is `built`                                            |
| build              | Report     | The authoritative `build/report.yaml` exists under the slice         |
| build              | Expect     | Every confined `expect` path holds a file                            |


Per-leg request / repair counts are **reported, not asserted**. After the
gates, the runner prints one line per judgment leg (keyed by answer-schema
name) with its request count and derived repairs — requests beyond one per
leg invocation (one propose per case, one synthesis per plan entry), e.g.
`leg synthesis: 4 request(s) over 3 slice(s), 1 repair(s)`. A leg drifting
from zero repairs toward the budget is the early signal that a prompt or
answer-schema change degraded the model's first answer.
