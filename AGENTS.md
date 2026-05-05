# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Workflow overview

Humans are expected to work through stock Specify:

- `/spec:init` (once per project)
- `/spec:define`
- `/spec:build`
- `/spec:merge`
- `/spec:drop`
- `/spec:extract` (extract Specify artifacts from existing source code)
- `/spec:execute` (drive an initiative's `plan.yaml` through define → build → merge; RFC-2 Layer 2, fully landed — `--dry-run` preview, supervised single-change run, self-heal on startup, `--loop` mode with terminal summary + SIGINT/SIGTERM handling, `sources` execution wiring, and post-merge cross-project contract validation per RFC-9 §3B)
- `/spec:plan` (author `plan.yaml` via `pipeline.plan`; RFC-2 Layer 3 + RFC-3a + RFC-3b — discovery through `/spec:analyze`, optional **sync-peers** when `registry.yaml` declares multiple projects (`specify workspace sync` + `workspace.md`), propose with glob or **manifest** scopes (Stage C), **project assignment** step for multi-repo plans (RFC-3b: infers `project` per entry from registry descriptions, writes via `specify plan amend --project`), `.specify/plans/<name>/` artefacts archived with the plan; see [rfcs/rfc-3a-monoliths.md](rfcs/archive/rfc-3a-monoliths.md) and [rfcs/archive/rfc-3b-platform.md](rfcs/archive/rfc-3b-platform.md))
- `/spec:plan --orchestrate` (Layer 4 umbrella mode that strings the cross-repo loop into one operator action: brief → registry validate → `/spec:plan` (default mode) → `/spec:execute --loop` → `specify workspace push` → optional `specify workspace merge` → `specify initiative finalize`; RFC-9 §2C, fully landed — composition only, idempotent on re-entry, supports `migrate-legacy` / `new-feature` / `update-existing` shapes through a single uniform sequence; was previously a separate `/spec:initiative` skill before being folded into `/spec:plan`)

This repository provides specialist skills and references that support that workflow.

### Skill / CLI responsibility split

The phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case name validation, `.metadata.yaml` reads and writes, lifecycle transitions, schema and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. The skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

CLI surface the skills depend on:

- `specify init <capability>` — scaffold `.specify/`, resolve/cache the capability identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `capability:` set. `--hub` (RFC-9 §1D) is the mutually exclusive alternative: it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true` (the `capability:` field is omitted). `specify init` invoked with neither (or both) errors with `init-requires-capability-or-hub`. See [RFC-13 §Migration "Hub project shape"](rfcs/rfc-13-extensibility.md#migration).
- `specify status` — project dashboard summarising registry, plan, and active changes (single-change view lives at `specify change status <name>`).
- `specify change {create, list, status, transition, touched-specs, overlap, archive, drop, validate, merge {preview, conflict-check, run}, task {progress, mark}, outcome {set, show}, journal {append, show}}` — every per-change verb. `outcome set` stamps the `.metadata.yaml:outcome` that `/spec:execute` reads; `journal append` writes `question` / `failure` / `recovery` entries into `journal.yaml`.
- `specify plan {create, validate, doctor, next, status, add, amend, transition, archive, lock}` — plan CRUD and lifecycle (RFC-2 Layer 1 + RFC-3a + RFC-9 §§1G/4B). `create` scaffolds an empty plan (renamed from `init` in v1.x); `add` appends an entry (renamed from the v1 entry-append `create`); `doctor` is a strict superset of `validate` with cycle / orphan-source / stale-clone / unreachable-entry diagnostics; `lock {acquire, release, status}` manages `.specify/plan.lock` for `/spec:execute`.
- `specify initiative {create, show, finalize}` — operator brief at `initiative.md` plus the canonical closure verb (RFC-9 §4C). `create` was renamed from v1 `init`; `finalize` confirms every per-project PR has merged before archiving.
- `specify registry {add, remove, show, validate}` — platform registry at `registry.yaml`. `add` and `remove` were added by RFC-9 §2A; both validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- `specify workspace {sync, status, push, merge}` — materialises `.specify/workspace/<peer>/` for multi-repo planning, pushes workspace clones to remotes after execution, and squash-merges the resulting PRs once CI is green (`merge`, RFC-9 §4A).
- `specify capability {resolve, check, pipeline}` — capability resolution and brief topology (renamed from `specify schema {resolve, check, pipeline}` by RFC-13 §Migration).

The previous standalone groups (`specify validate`, `specify spec`, `specify task`, `specify merge`) and the previous nested verbs (`specify initiative {brief, registry}`, `specify change phase-outcome`, `specify change journal-append`) were folded into `specify change` and top-level `registry` / `initiative` in the CLI cleanup. See [docs/explanation/migrating-cli-v1.md](docs/explanation/migrating-cli-v1.md) for the rename map.

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

### Contract skills

The contract plugin provides format-first specialist skills for API contract generation and validation. Each skill carries author / import / verify intents internally and dispatches via its own intent table:

- `/contract:openapi` — author, import, or verify HTTP / resource-style contracts (OpenAPI 3.1)
- `/contract:asyncapi` — author, import, or verify evented / pub-sub / streaming contracts (AsyncAPI 3.0)
- `/contract:json-schema` — author, import, or verify reusable payload schemas (JSON Schema)

Each skill exposes the same three intents through sibling files: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency and the post-merge cross-project consumer check via `--mode cross-project`). These skills are invoked by the `contracts` brief in the define pipeline (the brief id, the `contracts@v1` schema, and the `contracts/` baseline directory keep their original names — `contract` is the Cursor plugin / slash-command surface only). The brief is present in the `contracts` schema (for dedicated contract changes) and in the Omnia and Vectis schemas (for alignment validation during implementation changes).

The matching read-only CLI surface lives at `specify contract { list, validate }` (RFC-12 §"CLI surface") — it projects every top-level OpenAPI / AsyncAPI document under `contracts/` and runs the SemVer + id-format + cross-repo id-uniqueness checks; both verbs no-op with exit 0 when `contracts/` is absent.

### Plan-driven loop (RFC-2, all three layers landed)

When an initiative is coordinated through a `plan.yaml`, the recommended path is:

1. **Author.** `/spec:plan <initiative-name> --source <key>=<path-or-url> ...` — Layer 3 skill runs `pipeline.plan` briefs, optionally **sync-peers** + `workspace.md` when the registry is multi-project, then `specify plan create` + one `specify plan add` per accepted slice (globs or `--scope-manifest` per RFC-3a Stage C).
2. **Execute.** `/spec:execute --loop` — Layer 2 driver that repeatedly picks `specify plan next`, runs `/spec:define → /spec:build → /spec:merge` on the chosen entry, reads the phase outcome off `.metadata.yaml`, and transitions the plan entry to `done` / `failed` / `blocked`. Exits on `all-done`, `stuck`, self-heal halt, or SIGINT/SIGTERM.
3. **Archive.** `specify plan archive` sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback (RFC-2 Layer 1): skip `/spec:plan` and `/spec:execute`, author `plan.yaml` entry-by-entry with `specify plan {create, add, amend}`, and drive the loop yourself via `specify plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

The phase skills themselves stay unaware of the plan — they operate change-by-change. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *status* is only ever written via `specify plan transition`. A phase that discovers a neighbouring change mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specify plan add` / `specify plan amend` — the same commands humans run. See [rfcs/archive/rfc-2-execution.md](rfcs/archive/rfc-2-execution.md) for the full design.

### Commands

All commands are run from the repository root:

- **`make checks`** -- runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks
- **`make use-local-plugins`** -- use local plugins from the working tree for development/testing
- **`make use-team-plugins`** -- use Augentic marketplace plugins (reload Cursor after either)

### Skill authoring

- Every `SKILL.md` in this repository follows the house style codified in [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions); the long-form rationale (discovery model, why metadata is precious, examples of good/bad descriptions, the progressive-disclosure pattern, and the forbidden-frontmatter list) lives at [docs/explanation/skill-authoring.md](docs/explanation/skill-authoring.md).

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
