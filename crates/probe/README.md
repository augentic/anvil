# Prompt evaluation

The lab-only `probe` library: the typed eval case runner, deterministic
grading, and telemetry over the native host. The composition example that
drives it is [`examples/eval/`](../../examples/eval/README.md)
(`cargo make eval` / `cargo make lab`). Outputs are graded by
deterministic validators — not a model.

## Quick start

The runner drives the same `model::ModelBackend` the shipped binary links, so
it honours `EMERY_MODEL_BACKEND` — `cursor` (default) or `claude`. Authenticate
whichever one you select.

For cursor-agent, log in:

```bash
[cursor-]agent login
```

or set `CURSOR_API_KEY` in `.env` at the repository root. Note that
`cursor-agent status` proves an IDE login, not the `--print` path the backend
spawns.

For Claude Code, run `claude login` or set `ANTHROPIC_API_KEY`, then:

```bash
EMERY_MODEL_BACKEND=claude cargo make eval auth --restart
```

Only the selected backend is connected, so each mode needs only its own CLI on
`PATH`.

```bash
cargo make eval                       # list the cases
cargo make eval auth --restart        # run the engine's auth workflow case
cargo make eval auth --restart --until plan   # stop after plan author
```

Each case keeps one stable retained sandbox at `sandbox/<case>/` (the
composition-owned root beside the wasm example's `sandbox/wasm/`), on
success and failure alike. `--restart` is the only runner-owned reset: it
replaces that case's sandbox before running. An existing sandbox without
`--restart` refuses before mutation — the runner never infers workflow
progress from an existing tree. Continue or debug a retained sandbox
explicitly through command passthrough:

```bash
cargo make lab -- --project-dir sandbox/auth plan execute
```

Driver-side knobs (read by `probe::client`):

| Knob                      | Effect                                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `--debug` / `--quiet`     | Reserved host log flags, peeled anywhere in argv before dispatch (mutually exclusive) — the same contract as the shipped `emery` binary. `--quiet` turns tracing off; `--debug` selects `info,omnia_cursor=debug,omnia_wasi_http=debug`. A flag wins over `RUST_LOG`. That preset predates the Claude backend and does not name it — for Claude spawn detail use `RUST_LOG=info,model=debug` with no flag. |
| `EMERY_MODEL_BACKEND=<name>` | Which agent CLI serves completions: `cursor` (default) or `claude`. An unrecognised value fails at startup rather than falling back. |
| `EMERY_MODEL_RETRIES=<u32>` | Extra attempts after a transport failure (unreachable provider, killed spawn, stalled stream). Unset → 2. A rejected answer is never retried here — the backend's own two-attempt format repair covers that. |
| `EMERY_MODEL_RETRY_BACKOFF_MS=<u64>` | Wait before the first retry, doubling with jitter thereafter. Unset → 1000. |
| `CURSOR_MODEL` / `CLAUDE_MODEL` | Default model when a request leaves `model` unset; blank/unset lets the CLI choose. A guest-supplied id always wins. |
| `CURSOR_TIMEOUT_SECS` / `CLAUDE_TIMEOUT_SECS` | Per-spawn wall-clock bound (seconds). Unset → backend default 600. `cargo make eval` sets `300` for cursor. |
| `CURSOR_INACTIVITY_SECS` / `CLAUDE_INACTIVITY_SECS` | Kill a spawn after this long with no stream events, so a stalled agent dies well before the absolute cap. Unset → 120. |
| `CLAUDE_BARE=1`           | Pass `--bare`, forcing `ANTHROPIC_API_KEY` and ignoring the CLI's stored OAuth/subscription login. Unset → off. |
| `RUST_LOG=<filter>`       | The env escape hatch when no flag is passed. Example: `info,omnia_cursor=debug` or `info,model=debug`. Flagless with `RUST_LOG` unset defaults to `info`. |
| `EVAL_LOG=<path>`         | Log-file override. When unset, a named eval case logs to `<sandbox>/logs/<case>/eval-<stamp>.log` (announced at startup) and passthrough commands log to console only. The file receives an ANSI-free copy of the console output under the same filter; missing parent directories are created. |

Console tracing goes to stderr; stdout stays the semantic command
output.

A run's spans — `eval.case` (this crate), `emery.command` (the
transport router), the engine orchestration spans (`plan.author`,
`plan.execute.entry`, `slice.refine` / `slice.build` / `slice.merge`,
`source.survey` / `source.extract`, `judgment.leg`), and
`model.request` (the selected model backend) — carry only bounded labels (case
id/kind, command label, slice/adapter ids, judgment leg, repair count,
effective model id, exit code), never raw argv, intent/source values,
prompts, or project paths. The lab exports no OTLP telemetry: the
client installs a console subscriber (plus the optional `EVAL_LOG`
file copy), and OTLP export stays with the shipped runtime binary.

## Cases

A case is a data directory under the composition root's `cases/` tree —
`cases/<id>/case.toml` plus (usually) a sibling `fixture/` copied into the
fresh sandbox. Two kinds exist:

- **`kind = "workflow"`** — the operator rhythm over real verbs:
  `init <target>`, `plan author <change> [--intent] [--source k=v …]`,
  then (past `--until plan`) the genuine drained `plan execute`
  (running it is the approval), and (at `--until finalize`) `plan archive`. The default
  stop is `execute`; `case.toml`'s `until` sets a case default and
  `--until` overrides per run. An optional `clone = { url, dest }`
  (mutually exclusive with `fixture`) shallow-clones an upstream tree
  into the case's own `fixture/<dest>` on first run (stripping
  `.git`) and reuses that gitignored cache afterwards — for source
  trees that cannot ship as committed fixtures; delete the cached
  tree to refresh the snapshot.
- **`kind = "build"`** — one `emery slice build <slice>` against a
  committed refined fixture (valid project + slice metadata; the runner
  never stamps lifecycle state), then the built gates.

The auth case's hard synthesis case is **authority divergence** — the
`session-timeout` lead (scripted goldens name the slice `session-policy`),
not evidence volume. Docs claim a 30-minute idle timeout; code claims 15;
documentation authority should win. Grading still only asserts lifecycle +
provenance; it does not require a `[divergence]` tag.

`judgment-model-failed` on refine usually means the model backend timed out
or returned unparseable JSON — the engine `MAX_REPAIRS` loop never sees
that failure. Transport failures (an unreachable provider, a killed spawn, a
stalled stream) are retried inside the backend before they reach here, so a
`judgment-model-failed` that survives the retries is worth reading as a real
answer problem rather than a flaky connection.

## Model judgment

Emery core has two steps that require a model's judgement.


| Schema      | Step       | Crate    | Purpose                                                                                                                                     |
| ----------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `proposal`  | Propose    | `change` | Reconcile *surveyed* leads across sources into plan slices and author the review prose.                                                    |
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
| workflow (plan)    | Entries    | `plan author` produces at least one entry; every entry stays `pending` |
| workflow (execute) | Entries    | Every plan entry is `done`                                           |
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
