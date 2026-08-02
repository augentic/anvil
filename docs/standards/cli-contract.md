# CLI Contract

The deterministic surface skills depend on. Every phase skill in this repository (`/emery:init`, `/emery:plan`, `/emery:execute`, `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop`, `/emery:finalize`) shells out to the `emery` binary; each is an ultrathin wrapper over one guest-routed verb (`/emery:execute` and `/emery:finalize` compose a read-only status probe and an operator confirmation gate around theirs), and the orchestration underneath owns name validation, `metadata.yaml` reads and writes, lifecycle transitions, adapter resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, registry shape validation, and plan CRUD.

The CLI itself is built in the in-tree Cargo workspace at the repo root. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

Phase skills are ultrathin invoke-and-relay wrappers: they elicit missing arguments, invoke one `emery` verb, and relay its output. Guest orchestrations own judgment legs (survey, extract, synthesis, target build); target-adapter prompts own domain generation. Skill markdown must not grow orchestration, synthesis, or validation prose.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb and have the skill call it. The wrong fix is to make the skill smarter. See [AGENTS.md § Skill / CLI responsibility split](../../AGENTS.md#skill--cli-responsibility-split).

Never hand-edit `metadata.yaml`, never `mkdir -p .emery/...`, and never `mv` anything into `.emery/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

The CLI surface the skills depend on, grouped by resource:

### Project

- `emery init <adapter>` — scaffold `.emery/`, resolve/cache the adapter identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `adapter:` set. `--workspace` is the mutually exclusive alternative: it scaffolds a registry-only workspace whose `project.yaml` carries only `workspace: true` (the `adapter:` field is omitted). `emery init` invoked with neither (or both) exits `2` with clap's standard parse-error diagnostic.
- Read-only state inspection is direct file inspection (`plan.yaml`, `registry.yaml`, `metadata.yaml`, `model.yaml`, `discovery.md`) rather than formatted dashboard commands. The provenance audit view is projected on demand by `emery slice provenance`, not a persisted file.

### Slice (per-slice lifecycle)

- `emery slice list` — read-only listing of every slice with its lifecycle status and target.
- `emery slice validate` — artifact and coherence validation for one slice.
- `emery slice drop` — abandon a slice without merging (archives it and stamps the plan entry `dropped`). Slice directories are minted by the refine orchestration; lifecycle transitions are owned by the orchestrations (`refined`, `built`) and the merge/drop verbs — there is no standalone create or transition verb.
- `emery slice refine` — guest-routed refinement: slice create (re-entry safe), extraction per bound source, plus the synthesis leg that projects the agent response into `model.yaml` and the Markdown artifacts (the only writer).
- `emery slice build` — guest-routed target build: one orchestration assembles the build request, drives the adapter guest's build brief, validates the report, and gates the `built` transition.
- `emery slice {model show, provenance}` — read-only views: `model show` renders `model.yaml`; `provenance` projects the audit-only inline-provenance view on demand.
- `emery slice merge` — the merge into the baseline, with `--preview` / `--conflict-check` as its read-only dry-run flags.

### Change plan

- `emery plan {author, validate, advance, status, add, amend, remove, undo, archive, execute}` — plan CRUD and per-entry status. `author` scaffolds `plan.yaml` and is the guest-routed authoring orchestration and the default slice writer (surveys bound sources, reconciles leads, validates the partition, derives slice names and per-slice `target`, and replaces `slices[]` on a replaceable plan); `execute` is the guest-routed driver loop — running it is the operator's approval of the plan; `add` appends an entry and `remove` drops a pending entry; `validate` checks plan structure plus the `cycle-in-depends-on` / `orphan-source` / `stale-workspace-clone` health diagnostics; `advance` is the sole writer of per-entry `in-progress`; `slice merge` is the sole writer of per-entry `done`, and `undo` is the reverse walk (one rung per call, several with `--to`); `status` is the read-only next-action projection (`refine|build|merge <slice>` / `stop <reason>` / `drained`) over plan entries, slice metadata, and the journal tail; its body also carries the re-entry fields `current-step` / `last-completed` / `resume` (the literal command that makes progress, `null` when no single command does). Driver mutual exclusion is guest-owned: the guest-routed verbs hold the `.emery/guest.lock` marker for the run's lifetime; there is no native lock wrapper.

### Change umbrella

- `emery plan archive` — canonical archive verb for `plan.yaml`, `change.md`, and the plan working directory. In 2.0 the umbrella collapsed into `emery plan *`; `/emery:finalize` runs this verb after the operator confirms branch publication and the required repository workflow are complete.

### Registry and workspace topology

- `emery registry {validate, add, remove}` — platform registry at `registry.yaml`. `add` and `remove` validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- Top-level `workspace/<peer>/` slots and `.emery/topology.lock` remain inputs to multi-repo plan validation and routing. Materializing slots, preparing branches, committing, publishing, and completing pull requests are operator-owned operations outside Emery; there is no `emery workspace` command group.

### Source / target adapters

- `emery source {resolve, survey, extract}` and `emery target {resolve}` — the axis-split adapter debug/breakout surface. `resolve` locates the adapter component and reports its axis-derived operations; `survey` / `extract` are guest-routed workflow operations that merge leads into `discovery.md` and persist Evidence. The plan and refine orchestrations run these legs themselves; the standalone verbs exist for debugging and hand-driven breakouts. There is no declared-tool surface; adapter helpers are in-guest library code.

### Journal

- `emery journal show` — the observability surface over `.emery/journal.jsonl`: the read-only projection — `--filter <event-id-prefix>` keeps a dotted-prefix family, `--limit N` tails the most recent matches, text mode emits the canonical JSONL lines (probes pipe them to `jq -c .payload`), and `--format json` wraps the same events in the standard envelope. There is no emit verb — every write is a CLI-verb or orchestration side effect. See [Journal events](#journal-events).

Today the per-slice verbs live under `emery slice *` and the umbrella verbs live under `emery plan *`.

## Plan-driven loop composition

When a change is coordinated through a `plan.yaml`, the recommended skill / CLI composition is:

1. **Author.** `/emery:plan <change-name> source <key>=<path-or-url> ...` runs each bound source adapter's `survey` operation, reconciles leads across sources into proposed `slices[]` rows, validates the plan, and exits after authoring. The skill stops at the operator review seam — execution does not start automatically and the literal `emery plan execute` command is printed for the operator.
2. **Execute.** Invoking `emery plan execute` is the operator's approval of the plan — nothing is stamped or recorded. `/emery:plan` never runs it; `/emery:execute` wraps it. Under the guest lock the loop advances → refines → builds → merges per entry, routing project-bound entries into the corresponding materialized slot. Per-entry `done` is stamped by `emery slice merge`. Exits on the first `stop <reason>` (the `plan-execute-stopped` error envelope on stderr, exit 2, with the canonical plan-status stop card on stdout — no follow-up `emery plan status` call needed) or on `drained` (the success body carries `plan` and `phases[]`; text mode prints the phase lines and closes with the canonical `drained — run /emery:finalize <plan>` line). A fresh plan's `plan status` projection exposes `/emery:execute` as its `resume` so the operator path starts with execute.
3. **Publish and finalize.** After execution drains, the operator commits, publishes, and completes the required repository workflow. `/emery:finalize <change-name>` confirms publication is complete, then runs `emery plan archive`, which sweeps `plan.yaml` and the `.emery/plans/<name>/` authoring trail into `.emery/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback: skip `emery plan execute` and `/emery:finalize`, author the plan with `/emery:plan` and adjust entries with `emery plan {add, amend}`, drive the loop yourself via `emery plan advance → /emery:refine → /emery:build → /emery:merge` (per-entry `in-progress` is written by `emery plan advance`; per-entry `done` is written by `emery slice merge`), complete publication through normal repository tooling, then run `emery plan archive` by hand.

The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Plan *entries* are written via `emery plan author` (default), `emery plan add`, `emery plan amend`, and `emery plan remove`; per-entry status moves forward through `emery plan advance` and `emery slice merge`, and backwards only through `emery plan undo`. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `emery plan add` / `emery plan amend` — the same commands humans run.

The change lifecycle (`/emery:plan`, `/emery:execute`, `/emery:finalize`) has no umbrella that drives all three in one shot. The pause between `/emery:plan` and execute is the operator review seam — `/emery:plan` never chains into execution, so starting the loop is always the operator's own act. Each skill is idempotent on re-entry; halts surface verbatim and resume by re-running the same skill.

## Contracts validation surface

The contracts target adapter's `build` brief carries author / import / verify intents for OpenAPI, AsyncAPI, and JSON Schema as format sub-flows. Each sub-flow dispatches to sibling references under `targets/contracts/prose/references/<format>/` in the adapters repo: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency plus merge-time baseline validation in cross-project mode). The brief id, the `contracts@1.0.0` adapter, and the `contracts/` baseline directory keep their original names.

The matching validation surface is the contract validator compiled into the contracts adapter's guest. It walks a baseline `contracts/` directory and runs the SemVer, id-format, and cross-repo id-uniqueness checks. Contracts is a first-party adapter owning its own validation behaviour; the contracts merge orchestration invokes the in-guest validator as the post-merge baseline gate.

Cross-project consumer-impact classification is deferred until a real consumer workflow exists. Today the contracts target relies on the in-guest contract verifier report.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat body**: the command-specific fields at the top level of a single JSON object. On success the body is exactly that — there is no `ok` discriminant, no `data` wrapper around the payload, and no top-level envelope-version stamp. On failure the flat object carries three top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the body and branch on the `error` field rather than on stdout text.

Stream roles are part of the contract: the semantic result body (text or JSON) is stdout; the failure body and live host tracing are stderr. In text mode the failure body's `error:` line renders in ANSI red so it stands out from the surrounding tracing; `NO_COLOR` (any non-empty value), a missing `TERM`, and `TERM=dumb` all disable it, and the JSON envelope never carries styling. (The engine guest's WASI stderr exposes no terminal probe, so the guest colours by these environment guards alone; the native deployment additionally requires stderr to be a terminal.) Host tracing is selected by the reserved host log flags, peeled from argv before the guest sees it: bare invocations default to INFO progress with the noisy OTel targets muted, `--quiet` turns tracing off, and `--debug` adds `omnia_cursor` / `omnia_wasi_http` debug (both flags win over any ambient `RUST_LOG`). Skills follow the plugin rule's tracing contract — bare for the long-running orchestrations (`plan author`, `plan execute`, `slice refine`, `slice build`, committed `slice merge`), `--quiet` for probes and short deterministic verbs, and `--debug` on every subprocess when the operator asks for debug — and relay the semantic result once without repeating tracing lines.

The canonical envelope shapes — including the success / error variants and per-command body examples — live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline (house style applied in review). The reference is a hand-curated illustration of the happy path per command; variant coverage lives in the integration suites under `crates/*/tests/`.

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `registry-amendment-required` — execute-loop phase outcome carrying a structured proposal payload for adapters that need a new registry project.
- `description-missing-multi-repo` — `emery registry` shape validation invariant.
- `cycle-in-depends-on` / `orphan-source` / `stale-workspace-clone` — `emery plan validate` health diagnostics.
- `legacy-layout` — every project-aware verb refusing a v1-layout project.

## Journal events

Durable run telemetry is the newline-delimited JSON journal at `.emery/journal.jsonl`. Each line serialises `timestamp` first, then `event`, then the kebab-case `payload`:

```json
{"timestamp": "2026-06-11T00:00:00Z", "event": "slice.build.started", "payload": {"slice": "user-auth"}}
```

The event taxonomy is **closed** — the `EventKind` enum in the CLI repo's `crates/project/src/journal/event.rs` is the single source of truth; every append routes through the internal typed APIs, so an id outside the enum cannot reach the file. Keep the ids below aligned with that enum when the taxonomy changes:

| Family | Event ids | Emitted by |
|---|---|---|
| Plan | `plan.transition.undone`, `plan.entry.advanced`, `plan.reconcile.completed`, `plan.amend.authority-override`, `plan.amend.divergence` | `emery plan undo`, `emery plan advance`, the `plan author` reconcile kernel, `emery plan amend` |
| Slice synthesis | `slice.synthesize.started`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.synthesis.conflict`, `slice.synthesis.divergence`, `slice.synthesis.unknown`, `slice.extract.completed`, `slice.transition.refined` | the `slice refine` synthesis leg and its `refined` transition, `emery source extract` |
| Slice build | `slice.build.started`, `slice.build.succeeded`, `slice.build.failed` | the guest-routed `emery slice build` orchestration |
| Slice merge | `slice.merge.started`, `slice.merge.succeeded`, `slice.merge.failed`, `slice.archive.created` | `emery slice merge` (fired on its validator outcome) |
| Source / target | `source.survey.completed`, `source.execution.agent`, `target.execution.agent` | `emery source survey` / `extract`, the `slice build` request-assembly leg |

Writer ownership follows the same single-writer discipline as the lifecycle fields: CLI verbs append their own events as a side effect of the operation; skills never append — there is no emit verb, and nothing writes the file by hand. The journal is append-only telemetry — reading it back never gates a lifecycle transition. Reads route through `emery journal show` (eval probes, operators) or a CLI projection that consumes the tail internally (`emery plan status`'s stop classification); nothing re-parses the JSONL by hand.

## Exit codes

The CLI uses a four-slot exit-code table. The authoritative definition (variants and the mapping from `Error::*` types) lives in the [`AGENTS.md` "Exit codes" section](../../AGENTS.md#exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse `data`. |
| `1` | `EXIT_GENERIC_FAILURE` | Default `Error` mapping; parse the top-level `error` discriminant. |
| `2` | `EXIT_VALIDATION_FAILED` | Validation errors, undeclared/over-permissioned tool, argument errors. |
| `3` | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` (`emery-version-too-old`) — the project's `emery` pin is **newer** than this binary — or `Error::AdapterCliTooOld` (`adapter-cli-too-old`) — an adapter's declared `emery` compatibility floor is newer than this binary; tell the operator to update the installed binary through its install channel. |

Skills should branch on the exit code first (success vs failure class) and on the top-level `error` discriminant second (the specific failure mode). New exit codes are not invented by skills or the CLI; if a class of failure does not fit the four slots, the wire contract changes in the CLI repo and the kebab `error` discriminant distinguishes the case within an existing slot.

## Cross-references

- [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) — canonical envelope shapes per verb.
- [`AGENTS.md`](../../AGENTS.md#the-rust-workspace-emery-cli) — authoritative source for exit codes, error variants, and CLI architecture.
