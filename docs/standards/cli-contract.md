# CLI Contract

The deterministic surface skills depend on. Every skill in this repository (`/emery:init`, `/emery:plan`, `/emery:refine`, `/emery:execute`, `/emery:status`, `/emery:finalize`, `/emery:system-survey`, `/emery:system-plan`, `/emery:system-review`) shells out to the `emery` binary; each is an ultrathin wrapper over one verb (`/emery:finalize` composes a read-only status probe and an operator confirmation gate around its archive verb), and the orchestration underneath owns name validation, `metadata.yaml` reads and writes, lifecycle transitions, adapter resolution, artifact-completion checks, baseline conflict detection, delta merge, coherence validation, archive moves, and plan CRUD.

The CLI itself is built in the in-tree Cargo workspace at the repo root. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

Skills are ultrathin invoke-and-relay wrappers: they elicit missing arguments, invoke one `emery` verb, and relay its output. Guest orchestrations own judgment legs (survey, extract, synthesis, target build); target-adapter prompts own domain generation. Skill markdown must not grow orchestration, synthesis, or validation prose.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb and have the skill call it. The wrong fix is to make the skill smarter. See [AGENTS.md § Skill / CLI responsibility split](../../AGENTS.md#skill--cli-responsibility-split).

Never hand-edit `metadata.yaml`, `plan.yaml`, `discovery.yaml`, `leads.md`, or `decomposition.yaml`; never `mkdir -p .emery/...`; and never `mv` anything into `.emery/change/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

The CLI surface the skills depend on, grouped by resource:

### Project

- `emery init <adapter>` — scaffold `.emery/`, resolve/cache the adapter identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `adapter:` set. `emery init` invoked without an adapter exits `2` with `init-adapter-required`.
- Read-only state inspection is direct file inspection (`plan.yaml`, `metadata.yaml`, `model.yaml`, `leads.md`) rather than formatted dashboard commands. The provenance audit view is projected on demand by `emery slice provenance`, not a persisted file.

### Slice (read-only projections)

- `emery slice list` — read-only listing of every slice with its lifecycle status and target.
- `emery slice validate` — artifact and coherence validation for one slice, including the refinement-freshness (`slice-refinement-missing` / `slice-refinement-stale`) and baseline-conflict review advisories.
- `emery slice {model show, provenance}` — read-only views: `model show` renders `model.yaml`; `provenance` projects the audit-only inline-provenance view on demand.

Refinement is the standalone `emery plan refine` drain (slice directories are minted by it), while the build and merge phases are internal to the `emery plan execute` loop; lifecycle transitions are owned by those orchestrations (`refined`, `built`, `merged`) and `emery plan drop` — there is no per-slice create, transition, or phase-breakout verb.

### Change plan

- `emery plan {author, refine, execute, status, gaps, validate, add, amend, remove, drop, archive}` — the whole plan surface. `author` binds a reviewed handoff from `emery system review` (`--from` / `--wave`), imports the wave's surface leads into `leads.md` (focused child survey only when an imported lead is still coarser than a buildable boundary), decomposes the catalog, and publishes `decomposition.yaml` + `plan.yaml` together (required `slices[].target`; topology-only; no refine) — the tree persists incrementally, a failed cut disposes (close-as-leaf, else park + `plan-author-stopped`), and re-entry resumes the open and parked domains. `--change-dir` selects a detached change root; omitted, cwd is the change home when no ancestor carries `.emery/project.yaml`. `refine` is the guest-routed serial refinement drain — per in-scope leaf in dependency order it extracts every bound source, synthesizes + validates the slice artifacts, and atomically writes the slice's `refinement.yaml` manifest, skipping fresh manifests and stopping typed on the first failure (`plan-refine-stopped`) — no epoch, no workspace, no wave, no code work; `execute` is the guest-routed driver loop — it requires a fresh refinement manifest for every in-scope leaf (typed `plan-refinement-required` otherwise; execute never refines), at start appends `plan.execute.started` (authorization epoch covering the exact per-leaf refinement digests), then claims → builds → merges per entry under gap gates; `add` appends an entry (`--target` required), `remove` drops a pending entry, and `drop` abandons an already-refined entry's slice without merging; `amend` edits topology fields (reprojecting through `decomposition.yaml` when it exists), divergence stamps, authority overrides, the `allow-composition-replace` merge authorization, and `emery plan amend --proposal <digest>` (journals `plan.amend.applied`); `validate` checks plan structure plus the `cycle-in-depends-on` / `orphan-source` health diagnostics; `status` is the read-only next-action projection (`refine|build|merge <slice>` / `stop <reason>` / `drained`, plus Ready / Authorized) over artifacts and the fact union; `gaps` is the typed gap inventory; status also carries `current-step` / `last-completed` / `resume`. Driver mutual exclusion is guest-owned: the guest-routed verbs hold the `.emery/change/guest.lock` marker for the run's lifetime; per-slice exclusivity is claims.

### Change umbrella

- `emery plan archive` — canonical archive verb for `plan.yaml`, `change.md`, and the plan working directory. In 2.0 the umbrella collapsed into `emery plan *`; `/emery:finalize` runs this verb after the operator confirms branch publication is complete. Archive verifies publication itself by observing the forge (RFC-95 D5): it repeats the `publication-target-cycle` contraction check, projects the publication set (one typed GitHub read per Git-bound member — pull request matched on both `Emery-Change` trailers, zero/one/several rule, merged state, `merged-at` landing order), journals `plan.publication.projected` and per-member `plan.publication.member-landed`, and refuses `publication-unverified` (naming every failing member) until the set verifies. `--unverified` bypasses the verdict — journaling `plan.publication.unverified-archive` first — but never a forge transport or auth failure, which stays its own typed outcome. `--force` skips only the outstanding-work ladder check; the two flags compose.

### Source / target adapters

- `emery source {resolve, survey, extract}` and `emery target {resolve}` — the axis-split adapter debug surface. `resolve` locates the adapter component and reports its axis-derived operations; `survey` / `extract` are guest-routed workflow operations that merge leads into `leads.md` and persist Evidence. The `plan author` and `plan refine` orchestrations run these legs themselves; the standalone verbs exist for debugging. There is no declared-tool surface; adapter helpers are in-guest library code.

### Journal

- `emery journal show` — the observability surface over `.emery/change/events/<writer>.jsonl`: the read-only union projection — `--filter <event-id-prefix>` keeps a dotted-prefix family, `--limit N` tails the most recent matches, text mode emits the canonical JSONL lines (probes pipe them to `jq -c .payload`), and `--format json` wraps the same events in the standard envelope. There is no emit verb — every write is a CLI-verb or orchestration side effect. See [Journal events](#journal-events).

### Definition (RFC-104)

- `emery system {survey, plan, review, status}` — the definition loop over a hand-authored definition home (`--dir` else CWD; no `project.yaml` walk, no `system init`). `survey` materializes and extracts every included source with coverage accounting and correlates `as-is` into `system.yaml`; `plan` proposes the initial `target` + `migration.yaml` once and reprojects views + content-addressed `handoffs/<digest>.yaml`; `review <wave> --handoff <digest>` appends `system.wave.reviewed` to `<system>/events/`; `status` is the read-only next-action projection. Invoked by `/emery:system-survey`, `/emery:system-plan`, and `/emery:system-review`.

Today the read-only per-slice projections live under `emery slice *`, every delivery workflow verb lives under `emery plan *`, and the definition loop lives under `emery system *`.

## Plan-driven loop composition

When a change is coordinated through a `plan.yaml`, the recommended skill / CLI composition is:

1. **Define (when the engagement starts from an estate).** `/emery:system-survey` → `/emery:system-plan` → `/emery:system-review <wave> --handoff <digest>` produce the reviewed handoff. Simple changes use a degenerate definition through the same stages.
2. **Author.** `/emery:plan <change-name> --from <definition-home> --wave <id>` binds the reviewed handoff, imports its surface leads, decomposes the catalog into `decomposition.yaml` + `plan.yaml`, and exits after authoring. The skill stops at the operator review seam — execution does not start automatically. Later skills inherit the Cursor workspace cwd as the change root and may elicit `--change-dir`.
3. **Execute.** Invoking `emery plan execute` opens the authorization epoch (`plan.execute.started` with typed `closed-plan` coverage carrying per-leaf refinement digests; a leaf without a fresh refinement manifest refuses `plan-refinement-required` before the epoch, pointing at `emery plan refine` — execute never refines) and drives the loop. Before each build the gap gate joins durable deferral dispositions — deferred rows leave build scope, and every remaining open row is auto-deferred at the gate as a journaled `gap.deferred` fact, so build always proceeds. `/emery:plan` never runs it; `/emery:execute` wraps it. Under the guest lock the loop claims → builds → merges per entry (gap gate before each build). Per-entry `done` projects from merge / archive facts. Exits on the first `stop <reason>` (the `plan-execute-stopped` error envelope on stderr, exit 2, with the canonical plan-status stop card on stdout — no follow-up `emery plan status` call needed), on a hard epoch refusal (`plan-epoch-stale`), or on `drained` (the success body carries `plan` and `phases[]`; text mode prints the phase lines and closes with the canonical `drained — run /emery:finalize <plan>` line). A fresh plan's `plan status` projection exposes `/emery:execute` as its `resume` so the operator path starts with execute.
4. **Publish and finalize.** After execution drains — which also materializes each publication member's worktree on `change/<plan>` (RFC-95 D11) — the operator reviews, commits with both `Emery-Change` trailers, pushes, and opens the pull requests (see [publish a change](../how-to/publish-a-change.md)). `/emery:finalize <change-name>` confirms publication is complete, then runs `emery plan archive`, which verifies the publication set against the forge (D5, `--unverified` to bypass) before sweeping `plan.yaml` and the `.emery/change/plans/<name>/` authoring trail into `.emery/change/archive/plans/<YYYYMMDD>-<name>/`.

Recovery composes the same verbs: after a stop, fix the reported problem (curate entries with `emery plan {add, amend, remove, drop}` as needed) and re-run the command the stop card names — `emery plan refine` for refinement stops and staleness, `emery plan execute` for build / merge stops (the loop resumes at the parked phase). There are no phase-breakout verbs and no undo; forward re-execution is the only recovery direction.

Plan *entries* are written via `emery plan author` (default), `emery plan add`, `emery plan amend`, and `emery plan remove`; projected ladders move forward through claim / phase / merge facts. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `emery plan add` / `emery plan amend` — the same commands humans run.

The change lifecycle (`/emery:plan`, `/emery:refine`, `/emery:execute`, `/emery:finalize`) has no umbrella that drives all of them in one shot. The pause between `/emery:plan` and refine is the topology-review seam and the pause between `/emery:refine` and execute is the specification-review seam — `/emery:plan` never chains into refinement or execution, so starting each stage is always the operator's own act (an automation runner may invoke them back to back; the seams are opportunities for review, not attestations). Each skill is idempotent on re-entry; halts surface verbatim and resume by re-running the same skill.

## Drive-loop contract

The operator is not required to be a person. No verb reads stdin, prompts on a TTY, or opens an editor; `--force` on `plan author` / `plan archive` is a flag gate rather than an interactive confirmation; `--format json` (env `EMERY_FORMAT`) is global; and `emery plan status` projects a closed next-action set. An automated caller can therefore drive the whole loop, and the eval case runner already does.

That makes the following a **wire contract**, on the same footing as the exit-code table and the `error` discriminants: it changes through a deliberate wire-contract change, never by silent reshaping.

| Surface | Closed set | Source of truth |
|---|---|---|
| `plan status` `action` | `refine`, `build`, `merge`, `materialize`, `stop`, `drained` | `NextActionKind` in `crates/project/src/plan/status.rs` |
| `plan status` `stop.reason` | `refine-failed`, `refinement-required`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`, `boundary-escalation`, `refine-budget-exhausted`, `domain-frontier-failed`, `domain-complete-failed`, `publication-worktree-dirty`, `publication-provision-failed` | `StopReason`, same module |
| `plan status` `resume` | literal command string, or absent when no single command resumes | `StatusBody::resume` |
| Failure discriminant | kebab-case `error` on the flat body | [JSON envelope](#json-envelope) |
| Failure class | four-slot table | [Exit codes](#exit-codes) |

Three properties a driver must not rediscover the hard way:

- **Open gaps never redirect the loop.** There is no gap-review `action`: open `[unknown]` / `[conflict]` rows are advisory (`emery plan gaps` is the inventory), and when the caller dispatches `emery plan execute` the build gate auto-defers every remaining open row as a journaled `gap.deferred` fact — nothing blocks and nothing needs pre-supplying.
- **`stop` is not a verb either.** Branch on `stop.reason` and follow `resume`; a `stuck` or `slice-dropped` stop carries no `resume` and needs plan curation (`emery plan {amend, remove, drop}`) before the loop can continue.
- **A stop already carries the status card.** `plan refine` and `plan execute` stops exit 2 with the `plan-refine-stopped` / `plan-execute-stopped` envelope on stderr *and* the canonical stop card on stdout, so a driver parses the card it already has rather than issuing a follow-up `plan status`.

Do not build a separate driver for the phases themselves. `emery plan execute` is already the drain — it claims, builds, and merges per entry until the plan projects `drained` or a stop halts it. What an external caller adds is recovery judgment between runs, so the loop is thin: run the stage, read the stop card, fix inputs or curate entries, re-run. Putting that loop in a skill body would violate the ultrathin invoke-and-relay rule; it belongs in the caller.

Concurrency is bounded per project, not per caller: `plan refine` and `plan execute` hold a create-exclusive `.emery/change/guest.lock`, so a second concurrent driver on the same project fails with `guest-marker-held` (exit 2) rather than interleaving.

## Contracts validation surface

The contracts target adapter's `build` brief carries author / import / verify intents for OpenAPI, AsyncAPI, and JSON Schema as format sub-flows. Each sub-flow dispatches to sibling references under `targets/contracts/prose/references/<format>/` in the adapters repo: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency plus merge-time baseline validation in cross-project mode). The brief id, the `contracts@1.0.0` adapter, and the `contracts/` baseline directory keep their original names.

The matching validation surface is the contract validator compiled into the contracts adapter's guest. It walks a baseline `contracts/` directory and runs the SemVer, id-format, and cross-repo id-uniqueness checks. Contracts is a first-party adapter owning its own validation behaviour; the contracts merge orchestration invokes the in-guest validator as the post-merge baseline gate.

Cross-project consumer-impact classification is deferred until a real consumer workflow exists. Today the contracts target relies on the in-guest contract verifier report.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat body**: the command-specific fields at the top level of a single JSON object. On success the body is exactly that — there is no `ok` discriminant, no `data` wrapper around the payload, and no top-level envelope-version stamp. On failure the flat object carries three top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the body and branch on the `error` field rather than on stdout text.

Stream roles are part of the contract: the semantic result body (text or JSON) is stdout; the failure body and live host tracing are stderr. In text mode the failure body's `error:` line renders in ANSI red so it stands out from the surrounding tracing; `NO_COLOR` (any non-empty value), a missing `TERM`, and `TERM=dumb` all disable it, and the JSON envelope never carries styling. (The engine guest's WASI stderr exposes no terminal probe, so the guest colours by these environment guards alone; the native deployment additionally requires stderr to be a terminal.) Host tracing is selected by the reserved host log flags, peeled from argv before the guest sees it: bare invocations default to INFO progress with the noisy OTel targets muted, `--quiet` turns tracing off, and `--debug` adds `omnia_cursor` / `omnia_wasi_http` debug (both flags win over any ambient `RUST_LOG`). Skills follow the plugin rule's tracing contract — bare for the long-running orchestrations (`plan author`, `plan refine`, `plan execute`), `--quiet` for probes and short deterministic verbs, and `--debug` on every subprocess when the operator asks for debug — and relay the semantic result once without repeating tracing lines.

The canonical envelope shapes — including the success / error variants and per-command body examples — live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline (house style applied in review). The reference is a hand-curated illustration of the happy path per command; variant coverage lives in the integration suites under `crates/*/tests/`.

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `plan-refine-stopped` — the refinement drain halted on a failed refinement; the stop card names the slice, and re-running `emery plan refine` resumes the missing or stale work.
- `plan-refinement-required` — execute reached an in-scope leaf without a fresh refinement manifest; run `emery plan refine` first — execute never refines.
- `plan-execute-stopped` — the execute loop halted on a stop condition; the stop card names the reason and the resume command.
- `plan-proposal-*` / `plan-mutation-ambiguous` / `plan-ownership-overlap` — amendment application refusals (`emery plan amend --proposal`).
- `cycle-in-depends-on` / `orphan-source` — `emery plan validate` health diagnostics.
- `legacy-layout` — every project-aware verb refusing a v1-layout project.

## Journal events

Durable run telemetry is the newline-delimited JSON per-writer event log at `.emery/change/events/<writer>.jsonl`. Each line serialises `timestamp` first, then `writer` and `sequence`, then `event`, then the kebab-case `payload`:

```json
{"timestamp": "2026-06-11T00:00:00Z", "writer": "local", "sequence": 1, "event": "slice.build.started", "payload": {"slice": "user-auth"}}
```

The event taxonomy is **closed** — the `EventKind` enum in the CLI repo's `crates/project/src/journal/event.rs` is the single source of truth; every append routes through the internal typed APIs, so an id outside the enum cannot reach the file. Keep the ids below aligned with that enum when the taxonomy changes:

| Family | Event ids | Emitted by |
|---|---|---|
| Plan | `plan.entry.advanced`, `plan.reconcile.completed`, `plan.amend.authority-override`, `plan.amend.divergence`, `plan.execute.started`, `plan.merge-postflight.acknowledged` | the execute loop's claim step, the `plan author` reconcile kernel, `emery plan amend`, `emery plan execute` (authorization epoch at start — RFC-86 D6) |
| Gap disposition | `gap.deferred` | the build gate of `emery plan execute` — one durable digest-bound deferral fact per open gap row, minted unconditionally (RFC-86a D2) |
| Claim | `slice.claimed`, `slice.released` | exclusive per-slice claim / release facts (RFC-86 D7 / D23); the execute loop's claim step |
| Slice synthesis | `slice.synthesize.started`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.synthesis.conflict`, `slice.synthesis.divergence`, `slice.synthesis.unknown`, `slice.extract.completed`, `slice.transition.refined` | the `plan refine` drain's synthesis leg and its `refined` transition, `emery source extract` |
| Slice build | `slice.build.started`, `slice.build.succeeded`, `slice.build.failed`, `slice.build.phase-completed` | the execute loop's build phase (`phase-completed` is per-attempt ordinal evidence for each engine-selected build phase — RFC-90 D6) |
| Slice merge | `slice.merge.started`, `slice.merge.succeeded`, `slice.merge.failed`, `slice.archive.created`, `target.merge.wave-committed`, `target.merge.wave-succeeded`, `target.merge.wave-postflight-failed` | the execute loop's merge phase (wave commit + postflight; RFC-86 D9) |
| Source / target | `source.survey.completed`, `source.execution.agent`, `target.execution.agent`, `target.wave.opened` | `emery source survey` / `extract`, the build phase's request-assembly leg, target-wave open (RFC-86 D9; the payload names the frozen member list — multi-member since RFC-96 D7) |
| Domain convergence | `domain.convergence.recorded` | the execute loop's convergence step (RFC-96 D8) — one fact per bound target each time a durable `DomainRound` (frontier or complete) is written under `targets/<target>/domains/` |
| Definition | `system.wave.reviewed` | RFC-104 `emery system review` appends it to a definition-home event log; RFC-88 parses and verifies it and never appends it to the change journal |

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
