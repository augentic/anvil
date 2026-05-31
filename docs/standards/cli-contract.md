# CLI Contract

The deterministic surface skills depend on. Every phase skill in this repository (`/spec:init`, `/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:execute`, `/spec:finalize`) shells out to the `specify` binary for every deterministic operation: name validation, `.metadata.yaml` reads and writes, lifecycle transitions, adapter and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, registry shape validation, and plan CRUD.

The CLI itself is built in the sibling [augentic/specify-cli](https://github.com/augentic/specify-cli) repository. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

The phase skills are agent-driven orchestrators. The skill markdown drives the agent-side work — eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries. Everything else runs through `specify`.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb in the CLI repo and have the skill call it. The wrong fix is to make the skill smarter. The same rule is mirrored in the CLI repo's `AGENTS.md` under "Skill / CLI responsibility split".

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

The CLI surface the skills depend on, grouped by resource:

### Project

- `specrun init <adapter>` — scaffold `.specify/`, resolve/cache the adapter identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `adapter:` set. `--hub` is the mutually exclusive alternative: it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true` (the `adapter:` field is omitted). `specrun init` invoked with neither (or both) exits `2` with clap's standard parse-error diagnostic.
- Read-only state inspection is direct file inspection (`plan.yaml`, `registry.yaml`, `.metadata.yaml`, `provenance.yaml`, `discovery.md`) rather than formatted dashboard commands.

### Slice (per-slice lifecycle)

- `specrun slice {create, list, status, transition, touched-specs, overlap, archive, drop, validate}` — slice CRUD and lifecycle reads.
- `specrun slice merge {preview, conflict-check, run}` — three-phase merge into the baseline.
- `specrun slice task {progress, mark}` — per-task progress writes.
- `specrun slice outcome {set, show}` — `outcome set` stamps `.metadata.yaml:outcome`, which `/spec:execute` reads as the phase outcome.
- `specrun slice journal {append, show}` — writes `question` / `failure` / `recovery` entries into `journal.yaml`.

### Change plan

- `specrun plan {create, propose, validate, doctor, next, status, add, amend, remove, transition, archive, lock}` — plan CRUD and lifecycle. `create` scaffolds an empty plan; `propose --dry-run` returns the flat lead catalog + project topology for the agent, and `propose --from <response.json>` is the default slice writer (validates the partition, derives slice names and per-slice `target`, and replaces `slices[]` on a replaceable plan); `add` appends an entry and `remove` drops a pending entry; `doctor` is a strict superset of `validate` with cycle / orphan-source / stale-clone / unreachable-entry diagnostics; `lock {acquire, release, status}` manages `.specify/plan.lock` for `/spec:execute`.

### Change umbrella

- `specrun plan archive` — canonical archive verb for `plan.yaml`, `change.md`, and the plan working directory. In 2.0 the umbrella collapsed into `specrun plan *`; PR-state confirmation belongs to `/spec:finalize` and its `gh pr view` observation loop before this verb runs.

### Registry and workspace

- `specrun registry {add, remove, show, validate}` — platform registry at `registry.yaml`. `add` and `remove` validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- `specrun workspace {sync, status, push}` — `sync` materialises `.specify/workspace/<peer>/` for multi-repo planning and selected execution preparation; `push` transports prepared `specify/<change-name>` branches and creates/updates PRs only. `specrun workspace merge` has been removed and must not be called by skills; operators merge through the forge UI or explicit `gh pr merge`, then `/spec:finalize` verifies remote PR state with `gh pr view` before archiving via `specrun plan archive`.

### Adapter and declared tools

- `specify adapter {resolve, check, pipeline}` — adapter resolution and brief topology.
- `specrun tool {list, fetch, show, run}` — declared WASI command components. Tools are declared either in `.specify/project.yaml` (project scope) or in a `tools.yaml` sidecar next to `adapter.yaml` (adapter scope); project scope wins on collision. Permissions are directory preopens with `$PROJECT_DIR` (both scopes) and `$ADAPTER_DIR` (adapter scope only); the host canonicalises paths and rejects `..`, glob metacharacters, symlink escapes, and writes to Specify lifecycle state. Released first-party tool declarations require `sha256`.

Today the per-slice verbs live under `specrun slice *` and the umbrella verbs live under `specrun plan *`.

## Plan-driven loop composition

When a change is coordinated through a `plan.yaml`, the recommended skill / CLI composition is:

1. **Author.** `/spec:plan <change-name> source <key>=<path-or-url> ...` runs each bound source adapter's `survey` operation, reconciles leads across sources into proposed `slices[]` rows, validates the plan, and exits at `plan.lifecycle: pending`. The skill stops at the operator review seam — execution does not start automatically and the literal `specrun plan transition <change-name> approved` command is printed for the operator.
2. **Gate 1.** Operator runs `specrun plan transition <change-name> approved` — the only writer of `approved`. `/spec:plan` never stamps `approved` itself.
3. **Execute.** `/spec:execute` refuses unless the plan is `approved`; it repeatedly picks `specrun plan next`, prepares only the selected entry's project slot on exact branch `specify/<change-name>` when `project` is set, runs `/spec:refine → /spec:build → /spec:merge`, reads the phase outcome off `.metadata.yaml`, and transitions the plan entry to `done` / `failed` / `blocked`. Exits on `all-done`, `stuck`, self-heal halt, or SIGINT/SIGTERM.
4. **Finalize.** `/spec:finalize <change-name>` runs `specrun workspace push`, observes PR state via `gh pr view`, and runs `specrun plan archive` once every PR is `MERGED`. The CLI verb sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback: skip `/spec:plan`, `/spec:execute`, and `/spec:finalize`, author `plan.yaml` entry-by-entry with `specrun plan {create, add, amend}`, drive the loop yourself via `specrun plan next → /spec:refine → /spec:build → /spec:merge` (per-entry `in-progress` is written by `specrun plan next`; per-entry `done` is written by `specrun slice merge`), then run `specrun workspace push`, verify PRs with `gh pr view`, and run `specrun plan archive` by hand.

The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Plan *entries* are written via `specrun plan propose --from` (default), `specrun plan add`, `specrun plan amend`, and `specrun plan remove`; plan *status* is only ever written via `specrun plan transition`. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specrun plan add` / `specrun plan amend` — the same commands humans run.

The three change-lifecycle skills (`/spec:plan`, `/spec:execute`, `/spec:finalize`) are peers; there is no umbrella that drives all three in one shot. Operators who want a single command can write a thin shell wrapper, accepting that the wrapper opts out of the Gate-1 operator review pause between plan and execute. Each skill is idempotent on re-entry; halts surface verbatim and resume by re-running the same skill.

## Contracts as a declared WASI tool

The contracts target adapter's `build` brief carries author / import / verify intents for OpenAPI, AsyncAPI, and JSON Schema as format sub-flows. Each sub-flow dispatches to sibling references under `adapters/targets/contracts/references/<format>/`: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency plus merge-time baseline validation in cross-project mode). The brief id, the `contracts@v1` adapter, and the `contracts/` baseline directory keep their original names.

The matching CLI surface is the declared `contract` WASI tool, run through `specrun tool run contract -- "$PROJECT_ROOT/contracts" --format json`. It walks a baseline `contracts/` directory and runs the SemVer, id-format, and cross-repo id-uniqueness checks, exiting `0` clean / `1` findings / `2` tool or invocation error. The earlier in-binary `specify contract { list, validate }` family was retired when contracts became a first-party adapter owning its own validation behaviour; the contracts adapter merge brief now shells out through `specrun tool run` as the post-merge baseline gate.

Cross-project consumer-impact classification is deferred until a real consumer workflow exists. Today the contracts target relies on the declared contract WASI verifier report emitted through `specrun tool run contract -- "$PROJECT_ROOT/contracts" --format json`.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat envelope**: a top-level `envelope-version` integer plus the command-specific body fields at the same level. On success the body is exactly that — there is no `ok` discriminant and no `data` wrapper around the payload. On failure the same flat object carries three extra top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the envelope and branch on the `error` field rather than on stdout text.

The canonical envelope shapes — including the success / error variants and per-command body examples — live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline; the `checkNoEnvelopeExamples` predicate enforces the rule. The reference is a hand-curated illustration of the happy path per command; full variant coverage (including failure envelopes) lives in the CLI repo under [`tests/fixtures/plan/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/plan) and [`tests/fixtures/e2e/goldens/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/e2e/goldens).

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `registry-amendment-required` — `/spec:execute` phase outcome carrying a structured proposal payload for adapters that need a new registry project.
- `description-missing-multi-repo` — `specrun registry` shape validation invariant.
- `cycle-in-depends-on` / `orphan-source-key` / `stale-workspace-clone` / `unreachable-entry` — `specrun plan validate` health diagnostics.
- `no-branch` — `specrun workspace push` invoked on `main`, `master`, `origin/HEAD`, or any non-`specify/<change-name>` branch.
- `legacy-layout` — every project-aware verb refusing a v1-layout project.

## Exit codes

The CLI uses a four-slot exit-code table. The authoritative definition (variants, mapping from `Error::*` types, and the `Exit::Code(u8)` WASI passthrough used by `specrun tool run`) lives in the [CLI repo `AGENTS.md` "Error handling and exit codes" section](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#error-handling-and-exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse `data`. |
| `1` | `EXIT_GENERIC_FAILURE` | Default `Error` mapping; parse the top-level `error` discriminant. |
| `2` | `EXIT_VALIDATION_FAILED` | Validation errors, undeclared/over-permissioned tool, argument errors. |
| `3` | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` — the project's `specify_version` floor is higher than this binary; surface the upgrade hint. |

Skills should branch on the exit code first (success vs failure class) and on the top-level `error` discriminant second (the specific failure mode). New exit codes are not invented by skills or the CLI; if a class of failure does not fit the four slots, the wire contract changes in the CLI repo and the kebab `error` discriminant distinguishes the case within an existing slot.

## Cross-references

- [docs/standards/skill-authoring.md](skill-authoring.md) — the skill-side rules that surround this contract (description / argument-hint grammar, body caps, references discipline, guardrails).
- [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) — canonical envelope shapes per verb.
- [docs/standards/skill-guardrails.md](./skill-guardrails.md) — cross-cutting "skills MUST NOT" rules tied to this CLI surface.
- [specify-cli `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — authoritative source for exit codes, error variants, and CLI architecture.
