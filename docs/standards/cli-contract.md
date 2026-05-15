# CLI Contract

The deterministic surface skills depend on. Every phase skill in this repository (`/spec:init`, `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:extract`, `/change:plan`, `/change:execute`) shells out to the `specify` binary for every deterministic operation: name validation, `.metadata.yaml` reads and writes, lifecycle transitions, capability and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, registry shape validation, and plan CRUD.

The CLI itself is built in the sibling [augentic/specify-cli](https://github.com/augentic/specify-cli) repository. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

The phase skills are agent-driven orchestrators. The skill markdown drives the agent-side work — eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries. Everything else runs through `specify`.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb in the CLI repo and have the skill call it. The wrong fix is to make the skill smarter. The same rule is mirrored in the CLI repo's `AGENTS.md` under "Skill / CLI responsibility split".

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

The CLI surface the skills depend on, grouped by resource:

### Project

- `specify init <capability>` — scaffold `.specify/`, resolve/cache the capability identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `capability:` set. `--hub` is the mutually exclusive alternative: it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true` (the `capability:` field is omitted). `specify init` invoked with neither (or both) errors with `init-requires-capability-or-hub`.
- `specify status` — project dashboard summarising registry, active change, and active slices (single-slice view lives at `specify slice status <name>`).

### Slice (per-slice lifecycle)

- `specify slice {create, list, status, transition, touched-specs, overlap, archive, drop, validate}` — slice CRUD and lifecycle reads.
- `specify slice merge {preview, conflict-check, run}` — three-phase merge into the baseline.
- `specify slice task {progress, mark}` — per-task progress writes.
- `specify slice outcome {set, show}` — `outcome set` stamps `.metadata.yaml:outcome`, which `/change:execute` reads as the phase outcome.
- `specify slice journal {append, show}` — writes `question` / `failure` / `recovery` entries into `journal.yaml`.

### Change plan

- `specify plan {create, validate, doctor, next, status, add, amend, transition, archive, lock}` — plan CRUD and lifecycle. `create` scaffolds an empty plan; `add` appends an entry; `doctor` is a strict superset of `validate` with cycle / orphan-source / stale-clone / unreachable-entry diagnostics; `lock {acquire, release, status}` manages `.specify/plan.lock` for `/change:execute`.

### Change umbrella

- `specify change {create, show, finalize}` — operator brief at `change.md` plus the canonical closure verb. `create` was renamed from the v1 `init`; `finalize` confirms every per-project PR has merged before archiving.

### Registry and workspace

- `specify registry {add, remove, show, validate}` — platform registry at `registry.yaml`. `add` and `remove` validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- `specify workspace {sync, status, push}` — `sync` materialises `.specify/workspace/<peer>/` for multi-repo planning and selected execution preparation; `push` transports prepared `specify/<change-name>` branches and creates/updates PRs only. `specify workspace merge` has been removed and must not be called by skills; operators merge through the forge UI or explicit `gh pr merge`, then `specify change finalize` verifies remote PR state.

### Capability and declared tools

- `specify capability {resolve, check, pipeline}` — capability resolution and brief topology.
- `specify tool {list, fetch, show, run}` — declared WASI command components. Tools are declared either in `.specify/project.yaml` (project scope) or in a `tools.yaml` sidecar next to `capability.yaml` (capability scope); project scope wins on collision. Permissions are directory preopens with `$PROJECT_DIR` (both scopes) and `$CAPABILITY_DIR` (capability scope only); the host canonicalises paths and rejects `..`, glob metacharacters, symlink escapes, and writes to Specify lifecycle state. Released first-party tool declarations require `sha256`.

Today the per-slice verbs live under `specify slice *` and the umbrella verbs live under `specify change *`.

## Plan-driven loop composition

When a change is coordinated through a `plan.yaml`, the recommended skill / CLI composition is:

1. **Author.** `/change:plan <change-name> source <key>=<path-or-url> ...` runs the planning brief pipeline, optionally **sync-workspace** + `workspace.md` when the registry is multi-project, then `specify change create` (which scaffolds `change.md` + `plan.yaml` together) + one `specify plan add` per accepted slice (globs or `--scope-manifest`). Plan-time sync-workspace is discovery-oriented and may sync all registered projects.
2. **Execute.** `/change:execute loop` repeatedly picks `specify plan next`, prepares only the selected entry's project slot on exact branch `specify/<change-name>` when `project` is set, runs `/spec:define → /spec:build → /spec:merge`, reads the phase outcome off `.metadata.yaml`, and transitions the plan entry to `done` / `failed` / `blocked`. Exits on `all-done`, `stuck`, self-heal halt, or SIGINT/SIGTERM.
3. **Archive.** `specify plan archive` sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback: skip `/change:plan` and `/change:execute`, author `plan.yaml` entry-by-entry with `specify plan {create, add, amend}`, and drive the loop yourself via `specify plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *status* is only ever written via `specify plan transition`. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specify plan add` / `specify plan amend` — the same commands humans run.

The `/change:plan <name> orchestrate` umbrella mode strings the cross-repo loop into one operator action: brief → registry validate → `/change:plan` (default mode) → `/change:execute loop` → `specify workspace push` → operator PR merge → `specify change finalize`. It is composition only, idempotent on re-entry; opens/updates PRs but never merges them; supports `migrate-legacy`, `new-feature`, and `update-existing` shapes through a single uniform sequence.

## Contracts as a declared WASI tool

The contract plugin (`/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema`) carries author / import / verify intents internally and dispatches via its own intent table. Each skill exposes three intents through sibling files: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency plus merge-time baseline validation in cross-project mode). The brief id, the `contracts@v1` capability, and the `contracts/` baseline directory keep their original names — `contract` is the Cursor plugin / slash-command surface only.

The matching CLI surface is the declared `contract` WASI tool, run through `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. It walks a baseline `contracts/` directory and runs the SemVer, id-format, and cross-repo id-uniqueness checks, exiting `0` clean / `1` findings / `2` tool or invocation error. The earlier in-binary `specify contract { list, validate }` family was retired when contracts became a first-party capability owning its own validation behaviour; the contracts capability merge brief now shells out through `specify tool run` as the post-merge baseline gate.

Cross-project consumer-impact classification is exposed separately as `specify compatibility check` (with `--change <name>` to echo the change name and `--report-only` for read-only audits), which classifies producer-to-consumer contract deltas as `additive`, `breaking`, `ambiguous`, or `unverifiable` and exits validation-failed for every non-additive risk unless `--report-only` is passed.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat envelope**: a top-level `envelope-version` integer plus the command-specific body fields at the same level. On success the body is exactly that — there is no `ok` discriminant and no `data` wrapper around the payload. On failure the same flat object carries three extra top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the envelope and branch on the `error` field rather than on stdout text.

The canonical envelope shapes — including the success / error variants and per-command body examples regenerated from the CLI's `tests/fixtures/` — live in [plugins/references/cli-output-shapes.md](../../plugins/references/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline; the `checkNoEnvelopeExamples` predicate enforces the rule. The reference is regenerated by `make doc-envelopes`; CI runs the same generator with `--check` so the document and fixtures cannot drift.

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `init-requires-capability-or-hub` — `specify init` invoked with neither or both of `<capability>` / `--hub`.
- `registry-amendment-required` — `/change:execute` phase outcome carrying a structured proposal payload for capabilities that need a new registry project.
- `description-missing-multi-repo` — `specify registry` shape validation invariant.
- `cycle-in-depends-on` / `orphan-source-key` / `stale-workspace-clone` / `unreachable-entry` — `specify plan validate` health diagnostics.
- `no-branch` — `specify workspace push` invoked on `main`, `master`, `origin/HEAD`, or any non-`specify/<change-name>` branch.
- `legacy-layout` — every project-aware verb refusing a v1-layout project.

## Exit codes

The CLI uses a four-slot exit-code table. The authoritative definition (variants, mapping from `Error::*` types, and the `Exit::Code(u8)` WASI passthrough used by `specify tool run`) lives in the [CLI repo `AGENTS.md` "Error handling and exit codes" section](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#error-handling-and-exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse `data`. |
| `1` | `EXIT_GENERIC_FAILURE` | Default `Error` mapping; parse the top-level `error` discriminant. |
| `2` | `EXIT_VALIDATION_FAILED` | Validation errors, undeclared/over-permissioned tool, argument errors. |
| `3` | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` — the project's `specify_version` floor is higher than this binary; surface the upgrade hint. |

Skills should branch on the exit code first (success vs failure class) and on the top-level `error` discriminant second (the specific failure mode). New exit codes are not invented by skills or the CLI; if a class of failure does not fit the four slots, the wire contract changes in the CLI repo and the kebab `error` discriminant distinguishes the case within an existing slot.

## Cross-references

- [docs/standards/skill-authoring.md](skill-authoring.md) — the skill-side rules that surround this contract (description / argument-hint grammar, body caps, references discipline, guardrails).
- [plugins/references/cli-output-shapes.md](../../plugins/references/cli-output-shapes.md) — canonical envelope shapes and stable anchors per verb.
- [plugins/references/guardrails.md](../../plugins/references/guardrails.md) — cross-cutting "skills MUST NOT" rules tied to this CLI surface.
- [specify-cli `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — authoritative source for exit codes, error variants, and CLI architecture.
