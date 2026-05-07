# Orchestration mode (`--orchestrate`)

When invoked with `--orchestrate`, `/change:plan` runs the full Layer 4 umbrella sequence: brief → registry validate → plan author → execute --loop → workspace push → optional workspace merge → initiative finalize. This is the cross-repo umbrella driver formerly exposed as the v1 `/spec:initiative create` skill.

`--orchestrate` is **composition only**. Every step shells out to a verb that already exists in the v1 CLI surface or to another Layer 3 / Layer 2 surface. The orchestration mode adds no new logic, owns no new on-disk state, and never invents a CLI verb. Any deviation from the underlying skill's behaviour is a bug in the underlying skill, not in this mode.

> **Status.** Layer 4 is fully landed (RFC-9 §2C). The mode honours every halt the underlying skills surface (self-heal, `stuck`, `registry-amendment-required`), is **idempotent on re-entry** (running it again resumes from the first incomplete step), and supports the three canonical initiative shapes — `migrate-legacy`, `new-feature`, `update-existing` — through a single uniform sequence. See [shapes.md](shapes.md) for the shape inference and validation rules; see [re-entry.md](re-entry.md) for the idempotent re-entry algorithm.

## On-disk contracts

The orchestration mode never writes any of these files itself. Every state mutation is a shell-out.

| File / verb | Owner | Role for this mode |
|---|---|---|
| `change.md` | `specify change create` | Step 1 — operator brief; absent → create + prompt. |
| `registry.yaml` | `specify registry {add, remove, show, validate}` | Step 2 — validated; multi-project enforces description invariant. |
| `plan.yaml` | `/change:plan` (default mode) | Step 3 — authored by the plan skill in default mode (the orchestration mode delegates to itself). |
| `.specify/slices/<name>/.metadata.yaml` | phase skills via `specify slice outcome set` | Step 4 — read indirectly via `/change:execute`. |
| `.specify/plan.lock` | `/change:execute` (Layer 2) | Held by the executor for the duration of step 4. |
| `.specify/workspace/<peer>/` | `specify workspace {sync, push, merge}` | Steps 5–6 — peer clones containing the merged commits. |

## Pre-flight

Run **all** of the following before any side-effect. Any failure exits non-zero with a clear diagnostic.

1. **Hub presence.** Walk upward from CWD for `.specify/project.yaml`. Absent → exit non-zero pointing the operator at `/spec:init` (and, for cross-repo work, `/spec:init` with the hub option per [Platform repo topologies](../../../../docs/explanation/platform-repo.md)).
2. **`specify` binary.** `which specify`. Missing → exit non-zero pointing the operator at the [installation instructions](../../../../docs/orientation/prerequisites.md). The skill does not attempt a prose fallback — every step depends on the binary.
3. **`<name>` validation.** Reject any value not matching `^[a-z][a-z0-9-]*$`.
4. **`gh` warning (advisory).** When `--auto-merge` is set, run `gh --version`. Missing → emit a warning (`--auto-merge requires gh; install via https://cli.github.com/`) and continue. Steps 1–3 do not require `gh`; step 6 does and will surface the missing binary as a per-project failure if the operator persists.
5. **Shape resolution.** Apply the rules in [shapes.md](shapes.md) to pin a single shape value before step 1. Conflicts (e.g. explicit `--shape new-feature` with `--source ...`) are hard exits.

## Internal sequence (composition only)

The seven steps below are normative. Each step lists its **invocation**, the **halts** it can surface, the **manual-fallback** sequence an operator would run by hand to perform the same step, and the **failure recovery** rule when the step exits non-zero.

### Step 1 — Brief

**Invocation.**

```bash
test -f change.md || specify change create <name>
```

When `change.md` is absent, run `specify change create <name>` and surface a prompt asking the operator to fill in the body (or accept the inferred defaults from [shapes.md](shapes.md)). When the file already exists, do not re-create it — `specify change create` refuses on a populated brief.

**Halts.** None — step 1 either runs or no-ops.

**Manual fallback.**

```bash
specify change create <name>
$EDITOR change.md
```

**Failure recovery.** A non-zero exit from `specify change create` (e.g. `<name>` failed kebab-case validation, or a previous run partially scaffolded the brief) surfaces verbatim and exits the umbrella non-zero. The operator removes the half-written `change.md` (or fixes the name) and re-runs.

### Step 2 — Registry

**Invocation.**

```bash
specify registry validate
```

Three branches based on the resolved registry:

1. **Empty registry + `--shape migrate-legacy | new-feature`.** Hand off to the 2B greenfield path inside `/change:plan`'s default mode — the discovery brief proposes an initial topology, the operator approves, and `specify registry add` runs once per accepted entry. The orchestration mode does **not** call `specify registry add` itself.
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
/change:plan <name> \
    [--from <path>...] \
    [--against <path>] \
    [--source <key>=<path-or-url>...] \
    [--dry-run]
```

Note: **the orchestration mode delegates to the default mode of the same `/change:plan` skill**. It does not recurse into `--orchestrate`. The default mode runs the five-step authoring loop (parse → scaffold → brief pipeline → validate → hand-off) documented in [SKILL.md](SKILL.md).

The orchestration mode forwards every supplied input flag verbatim — no flag is renamed, suppressed, or invented. Under `--dry-run`, append `--dry-run` to the `/change:plan` invocation; the plan skill emits its own preview output and the umbrella stops at end of step 3.

The plan skill internally:

- runs the discovery brief (step 3a in default mode);
- on multi-project registries, runs `specify workspace sync` and authors `workspace.md` (step 3b);
- proposes slices and lands accepted ones via `specify change plan add` (step 3c);
- on multi-project registries, assigns each entry to a project via `specify change plan amend --project` (step 3d);
- when assignment names a project not yet in the registry, runs the **registry-proposal sub-step** — `specify registry add` + `specify workspace sync` — and continues (RFC-9 §2B);
- gates the run on `specify change plan validate`.

**Halts.**

- Plan-skill abort during the propose loop (operator typed `abort`) → umbrella stops; `proposal.md` records the partial decision trail; resume by re-running `/change:plan --orchestrate <name>` (which re-enters step 3 under `--extend` semantics — see [re-entry.md](re-entry.md)).
- `specify change plan validate` failure → umbrella stops; the operator amends via `specify change plan amend` and re-runs.

**Manual fallback.**

```bash
specify change plan create <name> [--source ...]
# discovery / sync-peers / propose / assignment cycles run by hand or via /change:plan default
specify change plan add <slice-name> ...
specify change plan amend <slice-name> --project <project>
specify registry add <project> --url ... --schema ... --description "..."
specify workspace sync
specify change plan validate
```

**Failure recovery.** A non-zero exit from `/change:plan` leaves the partial plan on disk (entries written via `specify change plan add` are durable). The operator inspects `specify change plan status`, fixes the offending entry, and re-runs the umbrella with `--extend` semantics implicit (re-running `/change:plan --orchestrate <name>` against an already-populated plan is idempotent — see [re-entry.md](re-entry.md)).

### Step 4 — Execute

**Invocation.**

```bash
/change:execute --loop
```

The execute skill takes the `.specify/plan.lock` PID stamp, runs self-heal, then iterates `specify change plan next → /spec:define → /spec:build → /spec:merge → specify change plan transition` until no eligible change remains. Multi-repo entries route into `.specify/workspace/<project>/` via the executor's CWD-routing step.

**Halts.** All terminal classifications surface verbatim:

| Classification | Source | Operator action |
|---|---|---|
| `all-done` | every entry `done` / `skipped` | continue to step 5 |
| `stuck` | dependency chain blocked by `failed` / `blocked` predecessors | `specify change plan doctor` → `specify change plan transition <name> pending` (or `skipped`) → re-run umbrella |
| `halted` | self-heal saw an ambiguous on-disk state | manual triage of `.specify/slices/<name>/.metadata.yaml` against `plan.yaml` → re-run umbrella |
| `driver-interrupted` | SIGINT/SIGTERM mid-run | re-run umbrella; self-heal reclaims the in-flight entry on the next startup |
| `registry-amendment-required` | RFC-9 §2B; phase emitted the structured payload | review proposal in journal → run the canonical recovery sequence (below) → re-run umbrella |

The umbrella **only** continues to step 5 on `all-done`. Every other classification stops the umbrella and surfaces the executor's terminal summary verbatim.

**Canonical `registry-amendment-required` recovery.** The driver records the proposal payload to the dropped slice's `journal.yaml` before transitioning the entry to `blocked`. The operator reviews the proposal and runs:

```bash
specify registry add <proposed-name> --url <proposed-url> --schema <proposed-schema> --description "<proposed-description>"
specify workspace sync
specify change plan amend <slice-name> --project <proposed-name>
specify change plan transition <slice-name> pending
```

…then re-runs the umbrella. The umbrella never auto-applies registry amendments — every registry mutation passes through operator confirmation, mirroring the constraint `/change:execute` enforces directly (per the [executor's `registry-amendment-required` recovery contract](../execute/per-slice-algorithm.md#canonical-recovery-sequence-operator-driven)).

**Manual fallback.** Drive the loop by hand using the same Layer 1 verbs the executor uses:

```bash
specify change plan lock acquire --pid $$
specify change plan next --format json
specify change plan transition <name> in-progress
/spec:define <name>; /spec:build <name>; /spec:merge <name>
specify change plan transition <name> done
specify change plan lock release --pid $$
```

**Failure recovery.** Any non-zero exit from `/change:execute` (other than the documented terminal classifications, which exit zero) is treated as a halt — the umbrella stops and surfaces the diagnostic. The operator triages and re-runs.

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
specify change finalize
```

The verb runs four guards in order: plan-presence, plan terminal-state, per-project PR-state (`MERGED` on remote), and workspace-cleanliness (`git status --porcelain` empty). All pass → `Plan::archive` sweeps `plan.yaml`, `change.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`. Any guard refuses → non-zero exit and the umbrella surfaces the per-project status table.

The umbrella runs `specify change finalize` **only** when:

- step 6 lands on `all-merged` (every project is `merged`); or
- a re-entry of the umbrella detects all PRs already merged on remote (re-entry path; see [re-entry.md](re-entry.md)).

**Halts.** Any guard refusal halts the umbrella. The operator merges the outstanding PR (by hand or `specify workspace merge`), commits any dirty workspace state, and re-runs the umbrella.

**Manual fallback.**

```bash
specify change finalize
specify change finalize --clean    # also prune .specify/workspace/<peer>/
specify change finalize --dry-run  # preview the guard table
```

**Failure recovery.** Idempotent by design — re-running `finalize` after clearing the refused guard completes the archive on the next invocation. After a successful finalize, the verb returns `plan-not-found` (the explicit "already finalized" signal) and the umbrella reports the change as already closed.

## `--dry-run` semantics (orchestration mode)

Under `--dry-run` with `--orchestrate` the mode is **observation-only** end-to-end. The skill MUST NOT:

- run `specify change create` (step 1 is a no-op when the brief is missing — diagnostic only);
- modify `registry.yaml` (step 2 runs `specify registry validate` only — read-only);
- invoke `/change:execute` (step 4 is skipped entirely);
- run `specify workspace push` or `specify workspace merge` (steps 5 and 6 are skipped);
- run `specify change finalize` (step 7 is skipped);
- write any file under `.specify/` other than what `/change:plan --dry-run` itself emits (and the plan skill's default mode under `--dry-run` writes nothing under `.specify/` either).

The skill MAY:

- read the registry, plan, and workspace state (`specify registry show`, `specify change plan status`, `specify workspace status`);
- invoke `/change:plan --dry-run` (default mode dry-run, which runs read-only — it reads `--from`/`--against`/`--source` inputs, runs the discovery brief in preview mode, and emits the readiness report and proposed-plan preview to stdout);
- emit a final preview block summarising what each subsequent step *would* do.

Output shape:

```text
[dry-run] /change:plan --orchestrate — <name>

Shape: <migrate-legacy | new-feature | update-existing>
Brief: <present | would-create>
Registry: <empty | single-project | multi-project> (<N> projects, descriptions: <ok | missing>)
Plan:
    <inline plan-skill --dry-run output>
Would invoke /change:execute --loop (skipped under --dry-run).
Would invoke specify workspace push (skipped under --dry-run).
Would invoke specify workspace merge (skipped under --dry-run).            # only when --auto-merge
Would invoke specify change finalize (skipped under --dry-run).

No changes written. Remove --dry-run to run the full sequence.
```

## `--auto-merge` semantics

Without `--auto-merge`, step 6 lists open PRs and stops; step 7 is **not** invoked. The operator merges PRs by hand and re-runs the umbrella to finalize.

With `--auto-merge`, step 6 invokes `specify workspace merge` (RFC-9 §4A). The verb is best-effort across projects: a `pending-checks` or `failed-checks` on one project does not abort the others, but any non-`{merged, would-merge, no-branch}` exit code halts the umbrella before step 7. The umbrella never:

- merges PRs whose `headRefName` is not `specify/<name>` (branch-pattern guard);
- passes `--admin` or `--auto` to `gh pr merge`;
- overrides failing or pending checks.

These guards are inherited verbatim from `specify workspace merge` — the umbrella never re-implements them.

## Verb hygiene

Every shell-out in this mode is a v1 verb verbatim. The list a reviewer can grep:

| Step | Verb |
|---|---|
| Pre-flight | `specify --version`, `gh --version` |
| 1 Brief | `specify change create <name>` |
| 2 Registry | `specify registry validate`, `specify registry show --format json` |
| 3 Plan | `/change:plan <name> [--from ...] [--against ...] [--source ...] [--dry-run] [--extend]` (default mode, not `--orchestrate`) |
| 3 (internal to plan default) | `specify change plan create`, `specify change plan add`, `specify change plan amend`, `specify change plan validate`, `specify registry add`, `specify workspace sync` |
| 4 Execute | `/change:execute --loop` |
| 4 (internal to execute) | `specify change plan lock {acquire, release}`, `specify change plan next`, `specify change plan transition`, `specify slice outcome show`, `specify slice journal append`, `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` |
| 5 Push | `specify workspace push` |
| 6 Land | `specify workspace merge`, `gh pr list`, `gh pr view` (read-only listing only when `--auto-merge` is unset) |
| 7 Finalize | `specify change finalize` |

**Pre-v1 forms that MUST NOT appear** (per [migrating-cli-v1.md](../../../../docs/explanation/migrating-cli-v1.md) and the v1.x rename rows for §1F+§1G):

- the hyphenated `phase-outcome` form → use `specify slice outcome set`.
- the hyphenated `journal-append` form → use `specify slice journal append`.
- the nested `change brief …` form → use `specify change {create, show, finalize}`.
- the nested `initiative registry …` form → use top-level `specify registry`.
- the v1 `initiative init <name>` form → use `specify change create <name>` (RFC-9 §1F).
- the v1 `plan init <name>` form → use `specify change plan create <name>` (RFC-9 §1G).
- the v1 entry-append `plan create <entry>` form → use `specify change plan add <entry>` (RFC-9 §1G).

The retired-verb checker in `scripts/checks.ts` enforces this list; any drift fails `make checks`.

## Composition discipline

This mode adds **no new logic**. Every step is a documented shell-out to either:

- a Layer 1 CLI verb that already shipped before 2C (1F, 1G, 2A, 4A, 4C); or
- a Layer 3 / Layer 2 surface that already shipped before 2C (`/change:plan` default mode, `/change:execute`); or
- the `gh` CLI for forge interactions, used in exactly the way `specify workspace {push, merge}` and `specify change finalize` already use it.

Concretely, this mode MUST NOT:

- introduce a new CLI verb (the v1 surface is closed for 2C);
- modify any file under `.specify/` directly (every write is a shell-out);
- re-implement halt classification (the underlying skills own it);
- re-implement the registry-amendment recovery (operator-driven via Layer 1, surfaced by Layer 2);
- batch multiple plan transitions, registry mutations, or PR merges (every mutation is one verb at a time);
- swallow halts (every halt is surfaced verbatim with the underlying skill's diagnostic).

If a behaviour drift surfaces between the orchestration mode and a manual run of the same seven verbs, the bug is in the underlying skill or CLI — not in this mode. File the gap against the underlying surface; the orchestration stays composition-only.

## Non-goals (orchestration mode)

- **Auto-creating registry entries.** Even on `--auto-merge`, registry mutations always pass through operator confirmation. The orchestration never silently runs `specify registry add` — that is the 2B registry-proposal sub-step's job (operator-driven, inside `/change:plan` default mode) or a manual recovery step after a `registry-amendment-required` halt.
- **Forge-agnostic land step.** Step 6 uses `gh`; non-GitHub forges fall back to the manual fallback path (merge by hand, re-run the umbrella to finalize).
- **Multi-plan output.** RFC-3a's single `plan.yaml` invariant is preserved. The orchestration drives one initiative at a time.
- **Driving completed initiatives.** Once `specify change finalize` returns `plan-not-found`, re-running the orchestration reports the change as already finalized and exits zero. There is no "rewind" verb.
- **Phase invocation.** This mode never invokes `/spec:define`, `/spec:build`, or `/spec:merge` directly. The phase skills are reached only through `/change:execute --loop` (step 4).

## Guardrails (orchestration mode)

- **No new CLI verbs.** Composition only. Any temptation to add a flag, a sub-verb, or a side-effect is a sign the work belongs in the plan skill's default mode, `/change:execute`, or one of the Layer 1 verbs underneath.
- **Surface halts verbatim.** Self-heal halt, `stuck`, `registry-amendment-required`, `pending-checks`, `failed-checks`, `branch-pattern-mismatch`, `dirty` workspace — every halt that the underlying skill or verb emits flows through to the operator unmodified. The orchestration never paraphrases a diagnostic.
- **Refuse cleanly when prerequisites are missing.** No `specify` binary → exit. No `.specify/` → exit. Bad `<name>` → exit. The pre-flight section is non-negotiable.
- **Idempotent by re-entry.** Running the orchestration twice with the same `<name>` and the same flags MUST advance through completed steps without re-doing them. The on-disk state is the source of truth; the orchestration never tracks its own progress. See [re-entry.md](re-entry.md).
- **`--dry-run` is observation-only.** No mutations under dry-run, period.
- **Pre-v1 verbs are an automatic regression.** A retired-verb hit in any fixture, transcript, or doc is a failing `make checks`. Use only the v1 verb shapes in the table above.
- **`--auto-merge` does not bypass safety.** The orchestration uses `specify workspace merge` verbatim; it inherits the branch-pattern guard, the no-`--admin` rule, and the no-CI-override rule. The orchestration never patches `gh` arguments.

## Cross-links

- [`/change:plan` default mode](SKILL.md) — the authoring path step 3 delegates to.
- [`/change:execute`](../execute/SKILL.md) — Layer 2 plan-driver skill (step 4); see also [`/change:execute` per-slice-algorithm.md §Registry amendment required](../execute/per-slice-algorithm.md#registry-amendment-required-rfc-9-2b) for the recovery surface step 4 surfaces.
- [`specify change`](../../../../docs/reference/cli/change.md) — `create`, `show`, and `finalize` (steps 1 and 7).
- [`specify registry`](../../../../docs/reference/cli/registry.md) — `add`, `remove`, `show`, `validate` (step 2 and recovery).
- [`specify plan`](../../../../docs/reference/cli/plan.md) — Layer 1 plan CRUD and lifecycle (recovery and manual fallback).
- [`specify workspace`](../../../../docs/reference/cli/workspace.md) — `sync`, `status`, `push`, `merge` (steps 5–6).
- [Cross-Repo Initiatives tutorial](../../../../docs/tutorials/cross-repo-change.md) — worked example for all three shapes (RFC-9 §1C and §2C).
- [Migrating CLI v1](../../../../docs/explanation/migrating-cli-v1.md) — verb rename map; pin every shell-out against the v1 surface.
- [The Layered Stack](../../../../docs/explanation/three-layer-stack.md) — Layer 4's place in Specify's layered architecture.
- [Drop down a layer](../../../../docs/how-to/drop-down-a-layer.md) — when to bypass the orchestration and run the steps by hand.
- [RFC-9 §2C](../../../../rfcs/archive/rfc-9-platform.md) — the design that introduced this orchestration.

## Fixtures

| Fixture | Pins |
|---|---|
| `fixtures/migrate-legacy/` | `--orchestrate --shape migrate-legacy --source monolith=<git-url>` end-to-end against an empty hub. Exercises the 2B greenfield registry path during step 3 and the autonomous land path (`--auto-merge`) at step 6. |
| `fixtures/new-feature/` | `--orchestrate --shape new-feature --from ./docs/...` against a populated multi-project hub. No registry mutation; assignment routes both implementation entries to existing projects. |
| `fixtures/update-existing/` | `--orchestrate --shape update-existing` (no `--from`, no `--source`) against a populated multi-project hub. Baseline-driven extension; supervised land path (no `--auto-merge`) so step 6 lists PRs and stops. |
