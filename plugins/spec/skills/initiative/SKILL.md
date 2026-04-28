---
name: initiative
description: |
  Layer 4 umbrella skill that drives a cross-repo initiative end to end —
  brief, registry, plan, execute, push, optionally land, finalize. Composes
  /spec:plan, /spec:execute, and the v1 specify CLI verbs into a single
  operator action; adds no new logic of its own. Use when you want one
  command to take a platform initiative from "I have an idea" to "every PR
  is merged."
license: MIT
argument-hint: "create <name> [--shape migrate-legacy|new-feature|update-existing] [--from <path>...] [--against <path>] [--source <key>=<path-or-url>...] [--auto-merge] [--dry-run]"
allowed-tools: Read, Write, Shell, Grep, Glob, AskQuestion, Task, TodoWrite
---

## Critical Path (Quick Reference)

1. **Pre-flight** — refuse on missing `.specify/`, missing `specify` binary, non-`create` sub-verb, or non-kebab-case `<name>`. Resolve `--shape` (defaulting from CLI flags) and warn — do not fail — if `gh` is missing when `--auto-merge` was passed.
2. **Brief.** If `.specify/initiative.md` is absent, run `specify initiative create <name>` and prompt the operator to flesh out the body (or accept defaults from `--shape` + CLI flags).
3. **Registry.** Run `specify registry validate`. Multi-project registries must satisfy the description invariant (2A). Empty registry plus `--shape migrate-legacy|new-feature` → run the 2B greenfield path inside `/spec:plan`.
4. **Plan.** Invoke `/spec:plan <name>` with the forwarded `--from`, `--against`, `--source` flags. Stop after the plan-skill's own dry-run preview when `--dry-run` is set.
5. **Execute.** Invoke `/spec:execute --loop`. Halts on the same conditions as the underlying skill — self-heal halt, `stuck`, `registry-amendment-required` (RFC-9 §2B). Each halt is recoverable; the umbrella surfaces the halt to the operator and stops without retrying.
6. **Push.** Run `specify workspace push`.
7. **Land.** With `--auto-merge`, run `specify workspace merge` (4A). Without, list open PRs and stop.
8. **Finalize.** When every per-project PR is `MERGED`, run `specify initiative finalize` (4C). Re-running the umbrella with all PRs merged and a still-present `plan.yaml` resumes at this step.

See detailed sections below for halts, manual fallbacks, three-shape semantics, and the re-entry algorithm.

# Initiative skill

Drive a cross-repo Specify initiative end to end from a single operator action: brief → registry validate → `/spec:plan` → `/spec:execute --loop` → `specify workspace push` → optional `specify workspace merge` → `specify initiative finalize`.

`/spec:initiative` is **composition only**. Every step shells out to a verb that already exists in the v1 CLI surface or to a Layer 3 skill. The umbrella adds no new logic, owns no new on-disk state, and never invents a CLI verb. Any deviation from the underlying skill's behaviour is a bug in the underlying skill, not in this one.

> **Status.** Layer 4 is fully landed (RFC-9 §2C). The umbrella honours every halt the underlying skills surface (self-heal, `stuck`, `registry-amendment-required`), is **idempotent on re-entry** (running it again resumes from the first incomplete step), and supports the three canonical initiative shapes — `migrate-legacy`, `new-feature`, `update-existing` — through a single uniform sequence.

## Overview

`/spec:initiative` sits at Layer 4 of the [layered stack](../../../../docs/explanation/three-layer-stack.md). It is the only Layer 4 skill; the layer exists specifically to give the umbrella verb a clear home above plan-and-drive.

The on-disk contracts the umbrella relies on are exactly the contracts the underlying skills already enforce:

| File / verb | Owner | Role for this skill |
|---|---|---|
| `.specify/initiative.md` | `specify initiative create` | Step 1 — operator brief; absent → create + prompt. |
| `.specify/registry.yaml` | `specify registry {add, remove, show, validate}` | Step 2 — validated; multi-project enforces description invariant. |
| `.specify/plan.yaml` | `/spec:plan` (Layer 3) | Step 3 — authored; not touched by this skill directly. |
| `.specify/changes/<name>/.metadata.yaml` | phase skills via `specify change outcome set` | Step 4 — read indirectly via `/spec:execute`. |
| `.specify/plan.lock` | `/spec:execute` (Layer 2) | Held by the executor for the duration of step 4. |
| `.specify/workspace/<peer>/` | `specify workspace {sync, push, merge}` | Steps 5–6 — peer clones containing the merged commits. |

The umbrella never writes any of these files itself. Every state mutation is a shell-out.

## Invocation

```text
/spec:initiative create <name> \
    [--shape migrate-legacy | new-feature | update-existing] \
    [--from <path>[:<kind>]...] \
    [--against <path>[:<kind>]] \
    [--source <key>=<path-or-url>[:<kind>]...] \
    [--auto-merge] \
    [--dry-run]
```

Flags:

- **`<name>`** — kebab-case identifier matching `^[a-z][a-z0-9-]*$`. Becomes the initiative name in `.specify/initiative.md`, the plan name in `.specify/plan.yaml`, and the PR branch suffix `specify/<name>` for `specify workspace push`. An invalid name is a hard exit before any side-effect.
- **`--shape <shape>`** — explicit shape override; one of `migrate-legacy`, `new-feature`, `update-existing` (closed enum). When omitted, the skill infers the shape from the CLI flags using the rules in [§Shape inference](#shape-inference). The shape determines which inputs are mandatory and which validation messages the skill emits before invoking `/spec:plan`.
- **`--from <path>`** — documentation input forwarded to `/spec:plan`. Repeatable. Default kind is `documentation`; override per-input via `:<kind>` suffix (per the closed enum in `/spec:plan` §Input kinds).
- **`--against <path>`** — refactor-target codebase forwarded to `/spec:plan`. Single-valued. Default kind is `legacy-code`.
- **`--source <key>=<path-or-url>`** — named legacy source forwarded to `/spec:plan` and threaded through `/spec:execute` per-change. Repeatable. Default kind is `legacy-code`. Git URLs flow into `/spec:analyze` clones (tier-1 workspace); local paths are passed through verbatim.
- **`--auto-merge`** — when set, step 6 invokes `specify workspace merge` (4A) on all open PRs whose CI is green. Without `--auto-merge`, step 6 lists the open PR set and the umbrella stops; the operator merges by hand on the forge (or invokes `specify workspace merge` directly) and re-runs the umbrella to finalize.
- **`--dry-run`** — observation-only across the entire sequence. Runs all read-side checks (steps 1–3 in their respective dry-run modes), invokes `/spec:plan --dry-run`, and stops. Does **not** invoke `/spec:execute`, `specify workspace {push, merge}`, or `specify initiative finalize`. See [§`--dry-run` semantics](#--dry-run-semantics).

The skill exposes only the `create` sub-verb. Any other sub-verb (e.g. `/spec:initiative resume <name>`, `/spec:initiative finalize <name>`) is a hard exit with a diagnostic pointing at the equivalent CLI verb (`specify initiative finalize`) or at re-running `create` for idempotent resume. New sub-verbs are out of scope for this round; reserve the namespace by refusing unknown forms cleanly.

## Pre-flight

Run **all** of the following before any side-effect. Any failure exits non-zero with a clear diagnostic.

1. **Hub presence.** Walk upward from CWD for `.specify/project.yaml`. Absent → exit non-zero pointing the operator at `/spec:init` (and, for cross-repo work, `/spec:init` with the hub option per [Platform repo topologies](../../../../docs/explanation/platform-repo.md)).
2. **`specify` binary.** `which specify`. Missing → exit non-zero pointing the operator at the [installation instructions](../../../../docs/orientation/prerequisites.md). The skill does not attempt a prose fallback — every step depends on the binary.
3. **Sub-verb.** Only `create` is supported. Anything else → exit non-zero with the diagnostic above.
4. **`<name>` validation.** Reject any value not matching `^[a-z][a-z0-9-]*$`.
5. **`gh` warning (advisory).** When `--auto-merge` is set, run `gh --version`. Missing → emit a warning (`--auto-merge requires gh; install via https://cli.github.com/`) and continue. Steps 1–5 do not require `gh`; step 6 does and will surface the missing binary as a per-project failure if the operator persists.
6. **Shape resolution.** Apply the rules in [§Shape inference](#shape-inference) to pin a single shape value before step 1. Conflicts (e.g. explicit `--shape new-feature` with `--source ...`) are hard exits.

## Shape inference

When `--shape` is omitted the skill infers the shape from the CLI flags using a closed table:

| Flags supplied | Inferred shape | Notes |
|---|---|---|
| `--source <k>=<v>` (one or more) | `migrate-legacy` | `--from` may co-exist; `--against` may co-exist. |
| `--from <path>` (only) | `new-feature` | Documentation-driven greenfield/feature work. |
| `--against <path>` (only) | `new-feature` | Refactor-target without legacy migration sources. |
| neither `--source`, `--from`, nor `--against` | `update-existing` | Baseline-driven extension; depends on a populated `initiative.md:inputs` (or, when absent, a non-empty registry whose baseline specs are the dominant signal). |

When `--shape` **is** explicitly supplied, validate the flags against the table:

| Explicit shape | Required | Forbidden |
|---|---|---|
| `migrate-legacy` | at least one `--source` | — |
| `new-feature` | at least one `--from` OR `--against` OR a populated `initiative.md:inputs` | — |
| `update-existing` | — | `--from`, `--against`, `--source` (any of the three is a hard exit) |

A shape conflict is a hard exit before step 1; the diagnostic names the offending flag(s) so the operator can drop the flag or change the shape.

## Internal sequence (composition only)

The seven steps below are normative. Each step lists its **invocation**, the **halts** it can surface, the **manual-fallback** sequence an operator would run by hand to perform the same step, and the **failure recovery** rule when the step exits non-zero.

### Step 1 — Brief

**Invocation.**

```bash
test -f .specify/initiative.md || specify initiative create <name>
```

When `initiative.md` is absent, run `specify initiative create <name>` and surface a prompt asking the operator to fill in the body (or accept the inferred defaults below). When the file already exists, do not re-create it — `specify initiative create` refuses on a populated brief, mirroring the `specify plan create` posture.

Default body skeleton (offered for operator acceptance, written by the operator, never auto-committed by this skill):

| Shape | Suggested `inputs:` | Suggested prose |
|---|---|---|
| `migrate-legacy` | one `legacy-code` entry per `--source` | one paragraph per source describing the migration target |
| `new-feature` | one `documentation` entry per `--from`, plus any `--against` as `legacy-code` | one paragraph per requirement |
| `update-existing` | empty | one paragraph naming the capability being extended and the registered project(s) it lives in |

**Halts.** None — step 1 either runs or no-ops.

**Manual fallback.**

```bash
specify initiative create <name>
$EDITOR .specify/initiative.md
```

**Failure recovery.** A non-zero exit from `specify initiative create` (e.g. `<name>` failed kebab-case validation, or a previous run partially scaffolded the brief) surfaces verbatim and exits the umbrella non-zero. The operator removes the half-written `initiative.md` (or fixes the name) and re-runs.

### Step 2 — Registry

**Invocation.**

```bash
specify registry validate
```

Three branches based on the resolved registry:

1. **Empty registry + `--shape migrate-legacy | new-feature`.** Hand off to the 2B greenfield path inside `/spec:plan` — the discovery brief proposes an initial topology, the operator approves, and `specify registry add` runs once per accepted entry. The umbrella does **not** call `specify registry add` itself.
2. **Multi-project registry.** `specify registry validate` enforces the `description-missing-multi-repo` invariant (RFC-3b §Validation; per RFC-9 §2A). Refusal exits the umbrella non-zero pointing the operator at `specify registry add <name> --description "..."` for each entry missing a description.
3. **Single-project registry or `--shape update-existing` with a populated registry.** Pass-through — the validate output is reported and the umbrella continues.

**Halts.** Operator-actionable validation failures (missing description, kebab-case violation, invalid URL, schema typo) halt the umbrella with the validator's diagnostic verbatim.

**Manual fallback.**

```bash
specify registry validate
specify registry add <name> --url <url> --schema <schema> --description "..."
specify registry remove <name>     # if needed
specify workspace sync             # after any add/remove
```

**Failure recovery.** Validator failures stay on disk — the registry is not modified by this step. The operator amends `registry.yaml` via `specify registry add` / `specify registry remove`, runs `specify workspace sync` to refresh clones, and re-runs the umbrella.

### Step 3 — Plan

**Invocation.**

```bash
/spec:plan <name> \
    [--from <path>...] \
    [--against <path>] \
    [--source <key>=<path-or-url>...] \
    [--dry-run]
```

The umbrella forwards every supplied input flag verbatim — no flag is renamed, suppressed, or invented. Under `--dry-run`, append `--dry-run` to the `/spec:plan` invocation; the plan skill emits its own preview output (per `/spec:plan` §Modes → `--dry-run`) and the umbrella stops at end of step 3.

The plan skill internally:

- runs the discovery brief (step 3a);
- on multi-project registries, runs `specify workspace sync` and authors `workspace.md` (step 3b);
- proposes slices and lands accepted ones via `specify plan add` (step 3c);
- on multi-project registries, assigns each entry to a project via `specify plan amend --project` (step 3d);
- when assignment names a project not yet in the registry, runs the **registry-proposal sub-step** — `specify registry add` + `specify workspace sync` — and continues (RFC-9 §2B);
- gates the run on `specify plan validate`.

**Halts.**

- Plan-skill abort during the propose loop (operator typed `abort`) → umbrella stops; `proposal.md` records the partial decision trail; resume by re-running `/spec:initiative create <name>` (which re-enters step 3 under `--extend` semantics, see [§Re-entry / idempotency](#re-entry--idempotency)).
- `specify plan validate` failure → umbrella stops; the operator amends via `specify plan amend` and re-runs.

**Manual fallback.**

```bash
specify plan create <name> [--source ...]
# discovery / sync-peers / propose / assignment cycles run by hand or via /spec:plan
specify plan add <slice-name> ...
specify plan amend <slice-name> --project <project>
specify registry add <project> --url ... --schema ... --description "..."
specify workspace sync
specify plan validate
```

**Failure recovery.** A non-zero exit from `/spec:plan` leaves the partial plan on disk (entries written via `specify plan add` are durable). The operator inspects `specify plan status`, fixes the offending entry, and re-runs the umbrella with `--extend` semantics implicit (re-running `/spec:initiative create <name>` against an already-populated plan is idempotent — see [§Re-entry / idempotency](#re-entry--idempotency)).

### Step 4 — Execute

**Invocation.**

```bash
/spec:execute --loop
```

The execute skill takes the `.specify/plan.lock` PID stamp, runs self-heal, then iterates `specify plan next → /spec:define → /spec:build → /spec:merge → specify plan transition` until no eligible change remains. Multi-repo entries route into `.specify/workspace/<project>/` via the executor's CWD-routing step.

**Halts.** All three terminal classifications surface verbatim:

| Classification | Source | Operator action |
|---|---|---|
| `all-done` | every entry `done` / `skipped` | continue to step 5 |
| `stuck` | dependency chain blocked by `failed` / `blocked` predecessors | `specify plan doctor` → `specify plan transition <name> pending` (or `skipped`) → re-run umbrella |
| `halted` | self-heal saw an ambiguous on-disk state | manual triage of `.specify/changes/<name>/.metadata.yaml` against `plan.yaml` → re-run umbrella |
| `driver-interrupted` | SIGINT/SIGTERM mid-run | re-run umbrella; self-heal reclaims the in-flight entry on the next startup |
| `registry-amendment-required` | RFC-9 §2B; phase emitted the structured payload | review proposal in journal → run the canonical recovery sequence (below) → re-run umbrella |

The umbrella **only** continues to step 5 on `all-done`. Every other classification stops the umbrella and surfaces the executor's terminal summary verbatim.

**Canonical `registry-amendment-required` recovery.** The driver records the proposal payload to the dropped change's `journal.yaml` before transitioning the entry to `blocked`. The operator reviews the proposal and runs:

```bash
specify registry add <proposed-name> --url <proposed-url> --schema <proposed-schema> --description "<proposed-description>"
specify workspace sync
specify plan amend <change-name> --project <proposed-name>
specify plan transition <change-name> pending
```

…then re-runs the umbrella. The umbrella never auto-applies registry amendments — every registry mutation passes through operator confirmation, mirroring the constraint `/spec:execute` enforces directly (per the [executor's `registry-amendment-required` recovery contract](../execute/SKILL.md#canonical-recovery-sequence-operator-driven)).

**Manual fallback.** Drive the loop by hand using the same Layer 1 verbs the executor uses:

```bash
specify plan lock acquire --pid $$
specify plan next --format json
specify plan transition <name> in-progress
/spec:define <name>; /spec:build <name>; /spec:merge <name>
specify plan transition <name> done
specify plan lock release --pid $$
```

**Failure recovery.** Any non-zero exit from `/spec:execute` (other than the documented terminal classifications, which exit zero) is treated as a halt — the umbrella stops and surfaces the diagnostic. The operator triages and re-runs.

### Step 5 — Push

**Invocation.**

```bash
specify workspace push
```

For each project with local commits ahead of `main`, the verb creates or updates `specify/<name>`, force-pushes (`--force-with-lease`), and creates a PR via `gh pr create` if one does not already exist. Greenfield remotes get `gh repo create` first.

**Halts.** Per-project failures (`failed`, `local-only` for non-remote-tracking clones) appear in the per-project status table. The umbrella does not abort on a single project's failure — `specify workspace push` is best-effort across projects — but it does halt the umbrella **as a whole** if any project's status is `failed`. The operator re-runs the umbrella after fixing the upstream issue (network, auth, missing remote).

**Manual fallback.**

```bash
cd .specify/workspace/<peer>/
git push --force-with-lease -u origin specify/<name>
gh pr create --title "..." --body "..."
```

**Failure recovery.** A push failure typically indicates an auth problem or a remote that does not yet exist. Resolve it by hand, then re-run the umbrella (idempotent — `specify workspace push` reports `up-to-date` for clones it already pushed).

### Step 6 — Land

Two modes:

#### `--auto-merge` set

**Invocation.**

```bash
specify workspace merge
```

The verb checks `gh pr checks` for each project's `specify/<name>` PR; if every check is `pass` or `skipping`, runs `gh pr merge --squash`. Branch-pattern guard: refuses any PR whose `headRefName` is not `specify/<name>` exactly.

The exit code surfaces directly: `0` only when every project lands on `merged`, `would-merge`, or `no-branch`. Any of `failed`, `failed-checks`, `pending-checks`, `closed`, or `branch-pattern-mismatch` flips the exit code to `1`. The umbrella halts on a non-zero exit and surfaces the per-project status table.

**Halts.**

- `pending-checks` → operator waits for CI; re-runs the umbrella (idempotent).
- `failed-checks` → operator fixes CI, pushes, re-runs the umbrella.
- `closed` / `branch-pattern-mismatch` → operator triages; the umbrella never force-merges.

#### `--auto-merge` not set

The umbrella **lists** the open PRs (using `gh pr list --head specify/<name>` per project, or by parsing `specify workspace push --format json` output captured in step 5) and **stops**. Step 7 is **not** invoked. The operator merges PRs by hand on the forge, then re-runs the umbrella to land step 7.

**Manual fallback.**

```bash
specify workspace merge       # autonomous
# or
gh pr merge <pr> --squash     # per-PR by hand
```

**Failure recovery.** Same as the autonomous path — fix the upstream (CI, branch protection, manual merge), re-run the umbrella.

### Step 7 — Finalize

**Invocation.**

```bash
specify initiative finalize
```

The verb runs four guards in order: plan-presence, plan terminal-state, per-project PR-state (`MERGED` on remote), and workspace-cleanliness (`git status --porcelain` empty). All pass → `Plan::archive` sweeps `plan.yaml`, `.specify/initiative.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`. Any guard refuses → non-zero exit and the umbrella surfaces the per-project status table.

The umbrella runs `specify initiative finalize` **only** when:

- step 6 lands on `all-merged` (every project is `merged`); or
- a re-entry of the umbrella detects all PRs already merged on remote (re-entry path; see [§Re-entry / idempotency](#re-entry--idempotency)).

**Halts.** Any guard refusal halts the umbrella. The operator merges the outstanding PR (by hand or `specify workspace merge`), commits any dirty workspace state, and re-runs the umbrella.

**Manual fallback.**

```bash
specify initiative finalize
specify initiative finalize --clean    # also prune .specify/workspace/<peer>/
specify initiative finalize --dry-run  # preview the guard table
```

**Failure recovery.** Idempotent by design — re-running `finalize` after clearing the refused guard completes the archive on the next invocation. After a successful finalize, the verb returns `plan-not-found` (the explicit "already finalized" signal) and the umbrella reports the initiative as already closed.

## Three-shape handling

The three initiative shapes (RFC-9 §Motivation → *The three initiative shapes*) flow through the same seven-step sequence. Only the inputs to step 3 (Plan) differ; steps 4–7 are shape-agnostic.

### `migrate-legacy`

```text
/spec:initiative create migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge
```

Pre-flight asserts at least one `--source` (closed-enum kind defaults to `legacy-code`). Step 3 forwards `--source` to `/spec:plan`, which clones the source into `.specify/plans/<name>/analyze/<key>/` (tier-1 workspace) for shallow inventory; deep `/spec:extract` runs at define time per change. When the registry is empty, the discovery brief proposes a multi-project topology and the operator approves entries via the 2B greenfield path. Targets are existing or newly-minted registered projects.

Fixture: [`fixtures/migrate-legacy/`](fixtures/migrate-legacy/).

### `new-feature`

```text
/spec:initiative create dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md
```

Pre-flight asserts at least one of `--from`, `--against`, or a populated `initiative.md:inputs`. Step 3 forwards the documentation inputs to `/spec:plan`, which runs discovery against the docs, syncs peers (when the registry is multi-project), proposes slices, and assigns each slice to an existing project via the registry. New projects spawn at assignment time via the 2B registry-proposal sub-step when the operator's override names a project not yet in `registry.yaml`.

Fixture: [`fixtures/new-feature/`](fixtures/new-feature/).

### `update-existing`

```text
/spec:initiative create polish-pass \
    --shape update-existing
```

Pre-flight forbids `--from`, `--against`, and `--source`. Step 3 invokes `/spec:plan` with no input flags; the plan skill reads `initiative.md:inputs` (which may be empty) and falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` — the dominant signal for a baseline-driven extension. No new registry entries are added; targets are exclusively existing registered projects.

Fixture: [`fixtures/update-existing/`](fixtures/update-existing/).

## `--dry-run` semantics

Under `--dry-run` the umbrella is **observation-only** end-to-end. The skill MUST NOT:

- run `specify initiative create` (step 1 is a no-op when the brief is missing — diagnostic only);
- modify `.specify/registry.yaml` (step 2 runs `specify registry validate` only — read-only);
- invoke `/spec:execute` (step 4 is skipped entirely);
- run `specify workspace push` or `specify workspace merge` (steps 5 and 6 are skipped);
- run `specify initiative finalize` (step 7 is skipped);
- write any file under `.specify/` other than what `/spec:plan --dry-run` itself emits (and per `/spec:plan` §Modes → `--dry-run`, the plan skill writes nothing under `.specify/` either).

The skill MAY:

- read the registry, plan, and workspace state (`specify registry show`, `specify plan status`, `specify workspace status`);
- invoke `/spec:plan --dry-run` (which runs read-only — it reads `--from`/`--against`/`--source` inputs, runs the discovery brief in preview mode, and emits the readiness report and proposed-plan preview to stdout);
- emit a final preview block summarising what each subsequent step *would* do.

Output shape:

```text
[dry-run] /spec:initiative — <name>

Shape: <migrate-legacy | new-feature | update-existing>
Brief: <present | would-create>
Registry: <empty | single-project | multi-project> (<N> projects, descriptions: <ok | missing>)
Plan:
    <inline plan-skill --dry-run output>
Would invoke /spec:execute --loop (skipped under --dry-run).
Would invoke specify workspace push (skipped under --dry-run).
Would invoke specify workspace merge (skipped under --dry-run).            # only when --auto-merge
Would invoke specify initiative finalize (skipped under --dry-run).

No changes written. Remove --dry-run to run the full sequence.
```

## `--auto-merge` semantics

Without `--auto-merge`, step 6 lists open PRs and stops; step 7 is **not** invoked. The operator merges PRs by hand and re-runs the umbrella to finalize.

With `--auto-merge`, step 6 invokes `specify workspace merge` (RFC-9 §4A). The verb is best-effort across projects: a `pending-checks` or `failed-checks` on one project does not abort the others, but any non-`{merged, would-merge, no-branch}` exit code halts the umbrella before step 7. The umbrella never:

- merges PRs whose `headRefName` is not `specify/<name>` (branch-pattern guard);
- passes `--admin` or `--auto` to `gh pr merge`;
- overrides failing or pending checks.

These guards are inherited verbatim from `specify workspace merge` — the umbrella never re-implements them.

## Re-entry / idempotency

Running `/spec:initiative create <name>` a second time after a halt is the canonical resume mechanism. The umbrella inspects on-disk state and resumes at the first incomplete step, without prompting and without re-doing earlier work. The resume table:

| State at re-entry | Resume step |
|---|---|
| `.specify/initiative.md` absent | step 1 |
| brief present, `.specify/plan.yaml` absent | step 3 (with `/spec:plan` running fresh) |
| `plan.yaml` present, any entry not in `{done, failed, skipped}` | step 4 (`/spec:execute --loop` resumes — self-heal reclaims any `in-progress` left by a prior crash) |
| every plan entry terminal, no PRs pushed yet (no `specify/<name>` branch on any remote) | step 5 |
| PRs pushed, not all `MERGED` | step 6 (with `--auto-merge`) or list-and-stop (without) |
| every PR `MERGED`, plan still on disk | step 7 |
| plan archived (`plan-not-found`) | report "initiative already finalized" and exit 0 |

The umbrella never re-creates a brief, re-runs discovery, or re-pushes a clone whose remote is already up to date. Resume is purely additive — every shell-out underneath is itself idempotent (`specify initiative create` refuses on populated brief; `specify plan create` refuses on populated plan; `specify workspace push` reports `up-to-date`; `specify workspace merge` reports `merged` for already-landed PRs; `specify initiative finalize` refuses on `plan-not-found`).

When step 3 runs against a populated `plan.yaml`, the umbrella forwards `--extend` to `/spec:plan` so the plan skill appends new slices instead of refusing on a populated plan. Operators who want a fresh plan archive the old one first (`specify plan archive`) and re-run.

## Verb hygiene

Every shell-out in this skill is a v1 verb verbatim. The list a reviewer can grep:

| Step | Verb |
|---|---|
| Pre-flight | `specify --version`, `gh --version` |
| 1 Brief | `specify initiative create <name>` |
| 2 Registry | `specify registry validate`, `specify registry show --format json` |
| 3 Plan | `/spec:plan <name> [--from ...] [--against ...] [--source ...] [--dry-run] [--extend]` |
| 3 (internal to plan) | `specify plan create`, `specify plan add`, `specify plan amend`, `specify plan validate`, `specify registry add`, `specify workspace sync` |
| 4 Execute | `/spec:execute --loop` |
| 4 (internal to execute) | `specify plan lock {acquire, release}`, `specify plan next`, `specify plan transition`, `specify change outcome show`, `specify change journal append`, `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` |
| 5 Push | `specify workspace push` |
| 6 Land | `specify workspace merge`, `gh pr list`, `gh pr view` (read-only listing only when `--auto-merge` is unset) |
| 7 Finalize | `specify initiative finalize` |

**Pre-v1 forms that MUST NOT appear** (per [migrating-cli-v1.md](../../../../docs/explanation/migrating-cli-v1.md) and the v1.x rename rows for §1F+§1G):

- the hyphenated `phase-outcome` form → use `specify change outcome set`.
- the hyphenated `journal-append` form → use `specify change journal append`.
- the nested `initiative brief …` form → use `specify initiative {create, show, finalize}`.
- the nested `initiative registry …` form → use top-level `specify registry`.
- the v1 `initiative init <name>` form → use `specify initiative create <name>` (RFC-9 §1F).
- the v1 `plan init <name>` form → use `specify plan create <name>` (RFC-9 §1G).
- the v1 entry-append `plan create <entry>` form → use `specify plan add <entry>` (RFC-9 §1G).

The retired-verb checker in `scripts/checks.ts` enforces this list; any drift fails `make checks`.

## Composition discipline

This skill adds **no new logic**. Every step is a documented shell-out to either:

- a Layer 1 CLI verb that already shipped before 2C (1F, 1G, 2A, 4A, 4C); or
- a Layer 3 skill that already shipped before 2C (`/spec:plan`, `/spec:execute`); or
- the `gh` CLI for forge interactions, used in exactly the way `specify workspace {push, merge}` and `specify initiative finalize` already use it.

Concretely, this skill MUST NOT:

- introduce a new CLI verb (the v1 surface is closed for 2C);
- modify any file under `.specify/` directly (every write is a shell-out);
- re-implement halt classification (the underlying skills own it);
- re-implement the registry-amendment recovery (operator-driven via Layer 1, surfaced by Layer 2);
- batch multiple plan transitions, registry mutations, or PR merges (every mutation is one verb at a time);
- swallow halts (every halt is surfaced verbatim with the underlying skill's diagnostic).

If a behaviour drift surfaces between the umbrella and a manual run of the same seven verbs, the bug is in the underlying skill or CLI — not in this skill. File the gap against the underlying surface; this skill stays composition-only.

## Non-goals

- **Auto-creating registry entries.** Even on `--auto-merge`, registry mutations always pass through operator confirmation. The umbrella never silently runs `specify registry add` — that is the 2B registry-proposal sub-step's job (operator-driven, inside `/spec:plan`) or a manual recovery step after a `registry-amendment-required` halt.
- **Forge-agnostic land step.** Step 6 uses `gh`; non-GitHub forges fall back to the manual fallback path (merge by hand, re-run the umbrella to finalize).
- **Multi-plan output.** RFC-3a's single `plan.yaml` invariant is preserved. The umbrella drives one initiative at a time.
- **Driving completed initiatives.** Once `specify initiative finalize` returns `plan-not-found`, re-running the umbrella reports the initiative as already finalized and exits zero. There is no "rewind" verb.
- **New sub-verbs.** Only `create` ships in this round. `/spec:initiative resume`, `/spec:initiative finalize`, `/spec:initiative status` are reserved namespaces; refuse them cleanly.
- **Mutating `plan.yaml` directly.** The umbrella never bypasses the plan-skill / executor / Layer 1 plan CLI. Every plan write goes through one of those.
- **Phase invocation.** This skill never invokes `/spec:define`, `/spec:build`, or `/spec:merge` directly. The phase skills are reached only through `/spec:execute --loop` (step 4).

## Guardrails

- **No new CLI verbs.** Composition only. Any temptation to add a flag, a sub-verb, or a side-effect is a sign the work belongs in `/spec:plan`, `/spec:execute`, or one of the Layer 1 verbs underneath.
- **Surface halts verbatim.** Self-heal halt, `stuck`, `registry-amendment-required`, `pending-checks`, `failed-checks`, `branch-pattern-mismatch`, `dirty` workspace — every halt that the underlying skill or verb emits flows through to the operator unmodified. The umbrella never paraphrases a diagnostic.
- **Refuse cleanly when prerequisites are missing.** No `specify` binary → exit. No `.specify/` → exit. Bad `<name>` → exit. The pre-flight section is non-negotiable.
- **Idempotent by re-entry.** Running the umbrella twice with the same `<name>` and the same flags MUST advance through completed steps without re-doing them. The on-disk state is the source of truth; the umbrella never tracks its own progress.
- **`--dry-run` is observation-only.** No mutations under dry-run, period. The plan skill's own dry-run handles the heavy preview lifting; the umbrella adds the surrounding step-1/2/5/6/7 preview but writes nothing.
- **Pre-v1 verbs are an automatic regression.** A retired-verb hit in any fixture, transcript, or doc is a failing `make checks`. Use only the v1 verb shapes in the table above.
- **`--auto-merge` does not bypass safety.** The umbrella uses `specify workspace merge` verbatim; it inherits the branch-pattern guard, the no-`--admin` rule, and the no-CI-override rule. The umbrella never patches `gh` arguments.

## Cross-links

- [`/spec:plan`](../plan/SKILL.md) — Layer 3 plan-authoring skill (step 3).
- [`/spec:execute`](../execute/SKILL.md) — Layer 2 plan-driver skill (step 4); see also [`/spec:execute` §Registry amendment required](../execute/SKILL.md#registry-amendment-required-rfc-9-2b) for the recovery surface step 4 surfaces.
- [`specify initiative`](../../../../docs/reference/cli/initiative.md) — `create`, `show`, and `finalize` (steps 1 and 7).
- [`specify registry`](../../../../docs/reference/cli/registry.md) — `add`, `remove`, `show`, `validate` (step 2 and recovery).
- [`specify plan`](../../../../docs/reference/cli/plan.md) — Layer 1 plan CRUD and lifecycle (recovery and manual fallback).
- [`specify workspace`](../../../../docs/reference/cli/workspace.md) — `sync`, `status`, `push`, `merge` (steps 5–6).
- [Cross-Repo Initiatives tutorial](../../../../docs/tutorials/cross-repo-initiative.md) — worked example for all three shapes (RFC-9 §1C and §2C).
- [Migrating CLI v1](../../../../docs/explanation/migrating-cli-v1.md) — verb rename map; pin every shell-out against the v1 surface.
- [The Layered Stack](../../../../docs/explanation/three-layer-stack.md) — Layer 4's place in Specify's layered architecture.
- [Drop down a layer](../../../../docs/how-to/drop-down-a-layer.md) — when to bypass the umbrella and run the steps by hand.
- [RFC-9 §2C](../../../../rfcs/rfc-9-platform.md) — the design that introduced this skill.

## Fixtures

| Fixture | Pins |
|---|---|
| [`fixtures/migrate-legacy/`](fixtures/migrate-legacy/) | `--shape migrate-legacy --source monolith=<git-url>` end-to-end against an empty hub. Exercises the 2B greenfield registry path during step 3 and the autonomous land path (`--auto-merge`) at step 6. |
| [`fixtures/new-feature/`](fixtures/new-feature/) | `--shape new-feature --from ./docs/...` against a populated multi-project hub. No registry mutation; assignment routes both implementation entries to existing projects. |
| [`fixtures/update-existing/`](fixtures/update-existing/) | `--shape update-existing` (no `--from`, no `--source`) against a populated multi-project hub. Baseline-driven extension; supervised land path (no `--auto-merge`) so step 6 lists PRs and stops. |
