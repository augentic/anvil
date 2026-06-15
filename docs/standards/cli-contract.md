# CLI Contract

The deterministic surface skills depend on. Every phase skill in this repository (`/spec:init`, `/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:execute`, `/spec:finalize`) shells out to the `specify` binary for every deterministic operation: name validation, `metadata.yaml` reads and writes, lifecycle transitions, adapter and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, registry shape validation, and plan CRUD.

The CLI itself is built in the sibling [augentic/specify-cli](https://github.com/augentic/specify-cli) repository. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

The phase skills are agent-driven orchestrators. The skill markdown drives the agent-side work — eliciting user intent, reading brief bodies, writing artifacts, running the target adapter's build brief, and rendering summaries. Everything else runs through `specify`.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb in the CLI repo and have the skill call it. The wrong fix is to make the skill smarter. The same rule is mirrored in the CLI repo's `AGENTS.md` under "Skill / CLI responsibility split".

Never hand-edit `metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

The CLI surface the skills depend on, grouped by resource:

### Project

- `specify init <adapter>` — scaffold `.specify/`, resolve/cache the adapter identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `adapter:` set. `--workspace` is the mutually exclusive alternative: it scaffolds a registry-only workspace whose `project.yaml` carries only `workspace: true` (the `adapter:` field is omitted). `specify init` invoked with neither (or both) exits `2` with clap's standard parse-error diagnostic.
- Read-only state inspection is direct file inspection (`plan.yaml`, `registry.yaml`, `metadata.yaml`, `model.yaml`, `discovery.md`) rather than formatted dashboard commands. The provenance audit view is projected on demand by `specify slice provenance`, not a persisted file.

### Slice (per-slice lifecycle)

- `specify slice {create, validate, transition, touched-specs, overlap, drop}` — slice CRUD and lifecycle.
- `specify slice synthesize` — two-phase synthesis: `--dry-run` emits the agent inputs envelope, `--from <response.json>` projects the agent response into `model.yaml` plus the Markdown artifacts (the only writer).
- `specify slice build` — two-phase target build: `--phase prepare` assembles the build request, `--phase finalize` validates the report and gates the `built` transition.
- `specify slice {model show, provenance}` — read-only views: `model show` renders `model.yaml`; `provenance` projects the audit-only inline-provenance view on demand.
- `specify slice merge {preview, conflict-check, run}` — three-phase merge into the baseline.
- `specify slice task {progress, mark}` — per-task progress writes.

### Change plan

- `specify plan {create, propose, validate, next, status, add, amend, remove, transition, archive, lock}` — plan CRUD, lifecycle, and the driver lock. `create` scaffolds an empty plan; `propose --dry-run` returns the flat lead catalog + project topology for the agent, and `propose --from <response.json>` is the default slice writer (validates the partition, derives slice names and per-slice `target`, and replaces `slices[]` on a replaceable plan); `add` appends an entry and `remove` drops a pending entry; `validate` checks plan structure plus the `cycle-in-depends-on` / `orphan-source` / `stale-workspace-clone` health diagnostics; `next` is the sole writer of per-entry `in-progress` and `transition` the sole writer of plan-level `approved` and per-entry `done`; `status` is the read-only next-action projection (`refine|build|merge <slice>` / `stop <reason>` / `drained`) over plan entries, slice metadata, and the journal tail; its body also carries the re-entry fields `current-step` / `last-completed` / `resume` (the literal command that makes progress, `null` when no single command does). The `/spec:execute` driver lock is `specify plan lock -- <cmd>` — the `flock(1)`-style command-wrapper that takes the exclusive `.specify/plan.lock` for the spawned child's lifetime, passes the child exit code through, refuses a busy lock with `plan-lock-busy` (exit 2), and skips re-acquisition under `SPECIFY_PLAN_LOCK_HELD=1` (see [`plugins/spec/references/plan-lock.md`](../../plugins/spec/references/plan-lock.md)). The CLI also probes the lock: `plan next`, per-entry `plan transition`, and plan-backed `slice merge run` refuse an unlocked driver with `plan-lock-not-held` (exit 2); the plan-level `approved` stamp is exempt.

### Change umbrella

- `specify plan archive` — canonical archive verb for `plan.yaml`, `change.md`, and the plan working directory. In 2.0 the umbrella collapsed into `specify plan *`; `/spec:finalize` runs this verb after `specify workspace push`. Pull-request creation and merging are operator-owned and happen outside Specify.

### Registry and workspace

- `specify registry {validate, add, remove}` — platform registry at `registry.yaml`. `add` and `remove` validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- `specify workspace {sync, push}` — `sync` materialises top-level `workspace/<peer>/` for multi-repo planning and selected execution preparation; `push` transports prepared `specify/<change-name>` branches to `origin` only — it does not create remote repositories or pull requests. the retired `workspace merge` subcommand has been removed and must not be called by skills; operators open and merge pull requests through the forge UI or explicit `gh` invocations, entirely outside Specify, while `/spec:finalize` runs `specify workspace push` then archives via `specify plan archive`.

### Source / target adapters and declared tools

- `specify source {resolve, survey, extract, preview}` and `specify target {resolve}` — the axis-split adapter surface. `resolve` locates a manifest and reports its declared operations; `survey` / `extract` are the two-phase (`--phase prepare|finalize`) workflow operations that merge leads into `discovery.md` and persist Evidence; `source preview` runs survey + extract in isolation without touching `.specify/`.
- `specify tool {run, fetch, gc, schema}` — declared WASI command components. Tools are declared either in `.specify/project.yaml` (project scope) or in a `tools.yaml` sidecar next to `adapter.yaml` (adapter scope); project scope wins on collision. Permissions are directory preopens with `$PROJECT_DIR` (both scopes) and `$ADAPTER_DIR` (adapter scope only); the host canonicalises paths and rejects `..`, glob metacharacters, symlink escapes, and writes to Specify lifecycle state. Released first-party tool declarations require `sha256`.

### Journal

- `specify journal {emit, show}` — the observability surface over `.specify/journal.jsonl`. `emit` is the guarded front door for agent-orchestrated phases (closed taxonomy: unknown ids and bad payloads exit 2); `show` is the read-only projection — `--filter <event-id-prefix>` keeps a dotted-prefix family, `--limit N` tails the most recent matches, text mode emits the canonical JSONL lines (probes pipe them to `jq -c .payload`), and `--format json` wraps the same events in the standard envelope. See [Journal events](#journal-events).

Today the per-slice verbs live under `specify slice *` and the umbrella verbs live under `specify plan *`.

## Plan-driven loop composition

When a change is coordinated through a `plan.yaml`, the recommended skill / CLI composition is:

1. **Author.** `/spec:plan <change-name> source <key>=<path-or-url> ...` runs each bound source adapter's `survey` operation, reconciles leads across sources into proposed `slices[]` rows, validates the plan, and exits at `plan.lifecycle: pending`. The skill stops at the operator review seam — execution does not start automatically and the literal `specify plan transition <change-name> approved` command is printed for the operator.
2. **Gate 1.** Operator runs `specify plan transition <change-name> approved` — the only writer of `approved`. `/spec:plan` never stamps `approved` itself.
3. **Execute.** `/spec:execute` refuses unless the plan is `approved` (rendered as `specify plan status`'s `stop plan-not-approved`); under the plan lock it loops `specify plan status` → `specify plan next` → the phase skill the CLI named (`/spec:refine` / `/spec:build` / `/spec:merge`), preparing only the selected entry's project slot on exact branch `specify/<change-name>` when `project` is set. Per-entry `done` is stamped by `specify slice merge`. Exits on the first `stop <reason>` or on `drained`.
4. **Finalize.** `/spec:finalize <change-name>` runs `specify workspace push`, then runs `specify plan archive`. The CLI verb sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`. Opening and merging the pull requests is the operator's job, done outside Specify.

Hand-driven fallback: skip `/spec:plan`, `/spec:execute`, and `/spec:finalize`, author `plan.yaml` entry-by-entry with `specify plan {create, add, amend}`, hold the plan lock for the session by driving the loop under `specify plan lock -- <cmd>` (see [`plugins/spec/references/plan-lock.md`](../../plugins/spec/references/plan-lock.md) — the loop verbs refuse `plan-lock-not-held` otherwise), drive the loop yourself via `specify plan next → /spec:refine → /spec:build → /spec:merge` (per-entry `in-progress` is written by `specify plan next`; per-entry `done` is written by `specify slice merge`), then run `specify workspace push` and `specify plan archive` by hand; open and merge the pull requests yourself outside Specify.

The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Plan *entries* are written via `specify plan propose --from` (default), `specify plan add`, `specify plan amend`, and `specify plan remove`; plan *status* is only ever written via `specify plan transition`. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specify plan add` / `specify plan amend` — the same commands humans run.

The three change-lifecycle skills (`/spec:plan`, `/spec:execute`, `/spec:finalize`) are peers; there is no umbrella that drives all three in one shot. Operators who want a single command can write a thin shell wrapper, accepting that the wrapper opts out of the Gate-1 operator review pause between plan and execute. Each skill is idempotent on re-entry; halts surface verbatim and resume by re-running the same skill.

## Contracts as a declared WASI tool

The contracts target adapter's `build` brief carries author / import / verify intents for OpenAPI, AsyncAPI, and JSON Schema as format sub-flows. Each sub-flow dispatches to sibling references under `adapters/targets/contracts/references/<format>/`: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency plus merge-time baseline validation in cross-project mode). The brief id, the `contracts@1.0.0` adapter, and the `contracts/` baseline directory keep their original names.

The matching CLI surface is the declared `contract` WASI tool, run through `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. It walks a baseline `contracts/` directory and runs the SemVer, id-format, and cross-repo id-uniqueness checks, exiting `0` clean / `1` findings / `2` tool or invocation error. Contracts is a first-party adapter owning its own validation behaviour; the contracts adapter merge brief shells out through `specify tool run` as the post-merge baseline gate.

Cross-project consumer-impact classification is deferred until a real consumer workflow exists. Today the contracts target relies on the declared contract WASI verifier report emitted through `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat envelope**: a top-level `envelope-version` integer plus the command-specific body fields at the same level. On success the body is exactly that — there is no `ok` discriminant and no `data` wrapper around the payload. On failure the same flat object carries three extra top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the envelope and branch on the `error` field rather than on stdout text.

The canonical envelope shapes — including the success / error variants and per-command body examples — live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline; the `checkNoEnvelopeExamples` predicate enforces the rule. The reference is a hand-curated illustration of the happy path per command; full variant coverage (including failure envelopes) lives in the CLI repo under [`tests/fixtures/plan/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/plan) and [`tests/fixtures/e2e/goldens/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/e2e/goldens).

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `registry-amendment-required` — `/spec:execute` phase outcome carrying a structured proposal payload for adapters that need a new registry project.
- `description-missing-multi-repo` — `specify registry` shape validation invariant.
- `cycle-in-depends-on` / `orphan-source` / `stale-workspace-clone` / `unreachable-entry` — `specify plan validate` health diagnostics.
- `no-branch` — `specify workspace push` invoked on `main`, `master`, `origin/HEAD`, or any non-`specify/<change-name>` branch.
- `plan-lock-busy` — `specify plan lock -- <cmd>` found the `.specify/plan.lock` driver lock already held by another session; carries the holder pid and exits before spawning the child.
- `plan-lock-not-held` — `specify plan next`, per-entry `specify plan transition`, or a plan-backed `specify slice merge run` invoked by a session that does not hold the `.specify/plan.lock` driver lock.
- `legacy-layout` — every project-aware verb refusing a v1-layout project.

## Journal events

Durable run telemetry is the newline-delimited JSON journal at `.specify/journal.jsonl`. Each line serialises `timestamp` first, then `event`, then the kebab-case `payload`:

```json
{"timestamp": "2026-06-11T00:00:00Z", "event": "slice.build.started", "payload": {"slice": "user-auth"}}
```

The event taxonomy is **closed** — the `EventKind` enum in the CLI repo's `crates/workflow/src/journal.rs` is the single source of truth, and `specify journal emit <event> --payload` (the guarded front door for agent-orchestrated phases) rejects ids outside it. The ids below are cross-checked against the running binary by CORE-057 on every `make lint`, so this list cannot drift silently:

| Family | Event ids | Emitted by |
|---|---|---|
| Plan | `plan.transition.approved`, `plan.transition.undone`, `plan.entry.advanced`, `plan.reconcile.completed`, `plan.amend.authority-override`, `plan.amend.divergence` | `specify plan transition` (with the `actor` field on `approved`), `specify plan next`, `specify plan propose --from`, `specify plan amend` |
| Slice synthesis | `slice.synthesize.started`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.synthesis.conflict`, `slice.synthesis.divergence`, `slice.synthesis.unknown`, `slice.extract.completed`, `slice.transition.refined` | `specify slice synthesize`, `specify source extract`, `specify slice transition` |
| Slice build | `slice.build.started`, `slice.build.succeeded`, `slice.build.failed` | `specify slice build` (two-phase handler) |
| Slice merge | `slice.merge.started`, `slice.merge.succeeded`, `slice.merge.failed`, `slice.archive.created` | `specify slice merge` (fired on its validator outcome) |
| Slice replay | `slice.replay.completed` | the replay target hook |
| Source / target | `source.survey.completed`, `source.execution.agent`, `target.execution.agent` | `specify source survey` / `extract`, `specify slice build --phase prepare` |
| Workspace | `workspace.sync.completed`, `workspace.push.completed` | `specify workspace sync` / `push` |
| Bootstrap and standards | `cli.upgraded`, `plugins.refreshed`, `lint-completed` | `specify upgrade`, `specify plugins refresh`, `specify lint` |

Writer ownership follows the same single-writer discipline as the lifecycle fields: CLI verbs append their own events as a side effect of the operation; skills append only through `specify journal emit`, never by writing the file. The journal is append-only telemetry — reading it back never gates a lifecycle transition. Reads route through `specify journal show` (eval probes, operators) or a CLI projection that consumes the tail internally (`specify plan status`'s stop classification); nothing re-parses the JSONL by hand.

## Exit codes

The CLI uses a four-slot exit-code table. The authoritative definition (variants, mapping from `Error::*` types, and the `Exit::Code(u8)` WASI passthrough used by `specify tool run`) lives in the [CLI repo `AGENTS.md` "Error handling and exit codes" section](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#error-handling-and-exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse `data`. |
| `1` | `EXIT_GENERIC_FAILURE` | Default `Error` mapping; parse the top-level `error` discriminant. |
| `2` | `EXIT_VALIDATION_FAILED` | Validation errors, undeclared/over-permissioned tool, argument errors. |
| `3` | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` — the project's `specify_version` is **newer** than this binary; surface the upgrade hint (`specify upgrade`). |

Skills should branch on the exit code first (success vs failure class) and on the top-level `error` discriminant second (the specific failure mode). New exit codes are not invented by skills or the CLI; if a class of failure does not fit the four slots, the wire contract changes in the CLI repo and the kebab `error` discriminant distinguishes the case within an existing slot.

## Cross-references

- [docs/standards/skill-authoring.md](skill-authoring.md) — the skill-side rules that surround this contract (description / argument-hint grammar, body caps, references discipline, guardrails).
- [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) — canonical envelope shapes per verb.
- [docs/standards/skill-guardrails.md](./skill-guardrails.md) — cross-cutting "skills MUST NOT" rules tied to this CLI surface.
- [specify-cli `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — authoritative source for exit codes, error variants, and CLI architecture.
