---
name: plan
description: |
  Author the initial .specify/plan.yaml for an initiative via the pipeline.plan
  brief pipeline. Layer 3 counterpart to /spec:execute: /spec:plan writes the 
  plan, /spec:execute runs it. When `.specify/registry.yaml` declares more than
  one project, runs the sync-peers phase (`specify workspace sync`)
  before propose and emits `workspace.md` for cross-repo planning.
license: MIT
argument-hint: "<initiative-name> [--from <path>...] [--against <path>] [--source <key>=<path-or-url>...] [--focus <area>] [--extend] [--dry-run]"
---

## Critical Path (Quick Reference)

1. **Parse and validate inputs** — validate `<initiative-name>` as kebab-case. Require at least one of `--from`, `--against`, `--source`, or a populated `initiative.md:inputs`. Refuse if `plan.yaml` already exists (unless `--extend`).
2. **Scaffold the plan** — `specify plan create <initiative-name> [--source <key>=<path-or-url> ...]`. Skipped under `--extend`.
3. **Run the plan brief pipeline** from `schema.yaml`:
   - **(a) Discovery** — invoke the discovery brief via `/spec:analyze`; writes `discovery.md`.
   - **(b) Sync peers** (multi-repo only) — `specify workspace sync` + author `workspace.md`.
   - **(c) Propose** — run the propose brief; iterate accept/edit/reject/abort per slice; `specify plan add` for each accepted slice.
   - **(d) Assignment** (multi-repo only) — infer `project` per entry; `specify plan amend --project <project>`.
4. **Validate** — `specify plan validate`. Non-zero exit on any `Error`-level finding. Never skip this step.
5. **Exit with hand-off summary** — point the operator at `specify plan status` and `/spec:execute --loop`.

See detailed sections below for edge cases, guardrails, and error handling.

# Plan skill

Author `.specify/plan.yaml` for a new initiative by running the `pipeline.plan` brief pipeline declared in the active schema's `schema.yaml`. `/spec:plan` is the Layer 3 authoring counterpart to `/spec:execute`: one *writes* the plan, the other *runs* it.

> **Status.** Layer 3 is fully landed: discovery (step 3(a)) routes through `/spec:analyze`; when the registry declares **more than one project**, a **sync-peers** step (3(b)) runs `specify workspace sync` and authors `workspace.md` before propose (step 3(c)); after propose, an **assignment** step (3(d)) infers and writes `project` per entry for multi-repo routing.

## Overview

Specify at authoring time is a three-layer stack — mirror of the execution stack documented in [`../execute/SKILL.md`](../execute/SKILL.md):

1. **Plan CLI** (`specify plan {init, validate, next, status, create, amend, transition, archive, lock}`) — the library-backed verbs that read and write `.specify/plan.yaml`. The single writer of the plan file, used by humans (Layer 1), `/spec:execute` (Layer 2), and this skill (Layer 3) alike.
2. **Authoring skill** (`/spec:plan`, this one) — the Layer 3 driver that runs the `pipeline.plan` brief pipeline and shells out to `specify plan add` for each accepted slice.
3. **Driver skill** (`/spec:execute`) — the Layer 2 automation that consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{init, create, amend, transition, archive}`) | Ordered change list with per-entry status. `/spec:plan` writes only via `specify plan create` (step 2) and `specify plan add` (step 3c). |
| `.specify/plans/<name>/` | schema (`pipeline.plan` briefs) | Working directory for authoring artefacts — `discovery.md`, optional `workspace.md` (multi-repo), `proposal.md`, `analyze/<key>/metadata.json` (legacy-code). Swept by `specify plan archive` alongside the plan itself. |
| `schema.yaml:pipeline.plan` | schema (`Phase::Plan`) | Declares the ordered list of authoring briefs for the project's schema. Resolved via `specify schema pipeline --phase plan`. |

## Invocation

```text
/spec:plan <initiative-name> \
    [--from <path>...] \
    [--against <path>] \
    [--source <key>=<path-or-url>...] \
    [--focus <area>] \
    [--extend] \
    [--dry-run]
```

Flags:

- **`<initiative-name>`** — kebab-case identifier; becomes the plan's top-level `name` field. Validated with the same rules as change names (regex `^[a-z][a-z0-9-]*$`) before any other work. An invalid name is a hard exit with a clear diagnostic — the skill never rewrites or "helps" the name.
- **`--from <path>`** — artefact file(s) or directory describing the target shape for greenfield authoring. Repeatable. Consumed by the discovery brief (L3.F). Kind defaults to `documentation`; override via `:<kind>` suffix (see §*Kind defaults for CLI flags*).
- **`--against <path>`** — an existing codebase to delta against, used for refactor or modernisation initiatives. Consumed by the discovery brief (L3.F). Kind defaults to `legacy-code`; override via `:<kind>` suffix (see §*Kind defaults for CLI flags*).
- **`--source <key>=<path-or-url>`** — a named source for migration. Repeatable. The `key` is a kebab-case identifier recorded in the plan's top-level `sources` map and referenced by individual plan entries via their `sources` list; the `value` is either a local filesystem path or a git URL. The skill forwards the tuple verbatim; cloning (if any) is the discovery brief's concern via `/spec:analyze` and `git-cloner`. Kind defaults to `legacy-code`; override via `:<kind>` suffix (see §*Kind defaults for CLI flags*).
- **`--focus <area>`** — optional scoping hint for the propose brief (L3.G). Free-form string; the propose brief decides how to interpret it.
- **`--extend`** — add to an existing `.specify/plan.yaml` instead of refusing. See [§Modes → `--extend`](#--extend) for the full contract; summary: step 2 is skipped, discovery is reused when `discovery.md` already exists, and draft slices whose names collide with existing entries are silently skipped with decision `skip-existing`.
- **`--dry-run`** — emit the readiness report and the proposed plan to stdout; write nothing. See [§Modes → `--dry-run`](#--dry-run).

At least one of `--from`, `--against`, `--source`, or a populated `initiative.md:inputs` list must be supplied. A bare `/spec:plan <name>` with no CLI inputs **and** no `initiative.md` (or `initiative.md` with empty `inputs`) is a hard exit — the skill cannot decide the initiative's shape without at least one input.

When `initiative.md:inputs` is the only source of inputs, the skill reads them via `specify initiative show --format json` before entering the core loop and treats each entry as if it had been supplied on the command line: `kind: legacy-code` entries route through the same path as `--source <k>=<path>:legacy-code`, and `kind: documentation` entries route through the `--from` path. The closed `kind` enum and default-kind mapping for CLI flags are pinned under §*Input kinds* and §*Kind defaults for CLI flags* below. Both documentation and legacy-code dispatch are live via `/spec:analyze`. Plan-time `/spec:extract` call sites have been fully retired; `/spec:extract` now runs only at `/spec:define` time with scope inferred from the change's description.

## Input kinds (normative)

Every input eventually analysed by the plan flow — whether CLI-supplied (`--from` / `--against` / `--source`) or brief-supplied (`initiative.md:inputs[].kind`) — is classified by a **closed kind enum**:

| kind            | Purpose                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| `legacy-code`   | Source code to be inferred into capability summaries at plan time and extracted per-change at define time. |
| `documentation` | Prose, PDFs, runbooks, API specs — parsed for capability summaries, constraints, and open questions.    |

The enum is closed: any other value is a hard error at the analyse phase (see `/spec:analyze`). This enum is frozen — NEVER extend it from this skill. This keeps the plan-time discovery contract auditable — every line in `discovery.md` is traceable to the kind-branch that produced it.

## Kind defaults for CLI flags

When an input is supplied via a CLI flag, its kind is determined as follows. The suffix syntax applies to `--source`, `--from`, and `--against` identically, though in practice only `--source` tends to carry explicit suffixes.

| CLI flag            | Default kind    | Explicit override                |
| ------------------- | --------------- | -------------------------------- |
| `--source <k>=<p>`  | `legacy-code`   | `--source <k>=<p>:<kind>`        |
| `--from <p>`        | `documentation` | `--from <p>:<kind>`              |
| `--against <p>`     | `legacy-code`   | `--against <p>:<kind>`           |

Inputs supplied via `initiative.md:inputs` carry their `kind:` explicitly in the frontmatter; no default is applied.

An explicit `:<kind>` suffix whose value is not in the closed enum is a hard exit before the core loop begins (same diagnostic as `/spec:analyze`'s unknown-kind error). The suffix grammar is `<value>[:<kind>]`, where `<kind>` is one of `legacy-code` or `documentation` (kebab-case, case-sensitive).

## Core loop (five steps)

Follow these steps in order on every invocation. Each step is normative; every shell-out is to the Layer 1 `specify` CLI; this skill writes nothing to `.specify/plan.yaml` directly.

```text
1. Parse inputs; resolve source paths; assert plan.yaml absent
   (or --extend).

   - Validate <initiative-name> as kebab-case. Reject with a hard
     exit on failure.
   - Require at least one of --from, --against, --source, or a
     populated `initiative.md:inputs` list. Discover the brief's
     inputs via `specify initiative show --format json`
     (exit 0 and `"brief": null` ⇒ brief absent ⇒ no brief inputs;
     `"frontmatter.inputs": []` ⇒ present but empty ⇒ no brief
     inputs). Reject with a hard exit on failure. A bare
     /spec:plan <name> with neither CLI inputs nor a populated
     initiative.md:inputs is still a hard exit — the diagnostic
     MUST mention both possibilities so the operator knows the
     two alternatives.
   - If .specify/plan.yaml exists and --extend was NOT supplied,
     refuse with a diagnostic pointing at `specify plan
     archive`. (There is no --force. The refusal is deliberate:
     overwriting an existing plan would drop audit history.)
   - If --extend was supplied but .specify/plan.yaml does NOT
     exist, also refuse — there is nothing to extend. Point the
     operator at running without --extend.

2. Scaffold the plan.

     specify plan create <initiative-name> \
         [--source <key>=<path-or-url> ...]

   Writes an empty .specify/plan.yaml with just the initiative
   `name` and the supplied `--source` entries in the top-level
   `sources` map. `changes: []` until step 3(c) populates it.

   Skipped entirely when --extend is set: the caller is explicitly
   adding to an existing plan, and `specify plan create`
   refuses when .specify/plan.yaml already exists.

3. Run the plan brief pipeline from schema.yaml.

   Resolve the ordered list of briefs via:

     specify schema pipeline --phase plan \
         --change .specify/plans/<name> --format json

   The response lists every brief in the schema's `pipeline.plan`
   declaration in topological order, with each brief's absolute
   `path`, `needs` edges, `generates` target, and current
   `present` flag relative to this initiative's working directory.

   Then run each brief in order:

     a. discovery — see §"Step 3(a) — Discovery" below.
     b. sync-peers — see §"Step 3(b) — Sync peers" below (multi-repo
         only; skipped when the registry is absent or single-project).
     c. propose   — see §"Step 3(c) — Propose" below.
     d. assignment — see §"Step 3(d) — Assignment" below (multi-repo
         only; skipped when the registry is absent or single-project).

4. Final validation gate.

     specify plan validate

   Runs the Layer 1 validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller — a plan that ships to `/spec:execute`
   without passing `specify plan validate` is a regression.

   The validator adds four cross-registry checks when `registry.yaml` is
   present: `project-not-in-registry` (error),
   `project-missing-multi-repo` (error),
   `description-missing-multi-repo` (error), and
   `schema-mismatch-workspace` (warning). The warning does not
   block validation — it surfaces a stale clone or registry typo.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify plan status` — review the authored plan.
     - `/spec:execute --loop` — start executing it (Layer 2).

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Step 3(a) — Discovery

Step 3(a) invokes the discovery brief declared in `pipeline.plan` (for Omnia, `schemas/omnia/briefs/plan/discovery.md`; other schemas ship their own). Discovery consumes the `--from`, `--against`, and `--source` inputs, dispatching each input per its `kind` through `/spec:analyze` (both `documentation` and `legacy-code` branches), and merges the results into a single neutral capability inventory at `.specify/plans/<initiative-name>/discovery.md`. The skill's job is to faithfully run the brief and pass inputs through; the algorithm (per-input handling, dedup rules, ordering) lives in the brief — see `schemas/omnia/briefs/plan/discovery.md` for the authoritative contract.

Discovery is read-only with respect to `plan.yaml`. The output header is exactly `# Discovery — <initiative-name>` with no timestamps, run IDs, or working-directory paths, and re-running discovery on unchanged inputs MUST produce byte-equivalent output — the brief owns the ordering, the skill does not impose its own. An existing `discovery.md` is overwritten unless `--extend` is set (see [§Modes → `--extend`](#--extend)). The shape of a single-`--source` inventory against a small pre-seeded source tree is pinned by [`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md) against [`fixtures/discovery/legacy/`](fixtures/discovery/legacy/).

## Step 3(b) — Sync peers (multi-repo only)

When **`.specify/registry.yaml`** exists and declares **more than one** project (`projects.length > 1` in the JSON from `specify registry show --format json`), `/spec:plan` enters the **sync-peers** phase between discovery and propose. Single-repo initiatives (absent registry or `projects.length ≤ 1`) skip this step entirely.

**Normative sequence**

1. Shell out to **`specify workspace sync`** from the project root. This materialises `.specify/workspace/<project-name>/` (symlink for local / relative `url:` values; shallow `git clone` / `git fetch` for remotes — see the CLI). Treat a non-zero exit as a hard failure for `/spec:plan`.
2. Walk each materialised peer root read-only and author **`.specify/plans/<initiative-name>/workspace.md`** — the peer inventory the propose brief consumes alongside `discovery.md`.

**`workspace.md` shape (pin for idempotency)**

```markdown
# Workspace — <initiative-name>

## <registry-project-name>

- **Slot:** `.specify/workspace/<registry-project-name>/`
- **Description:** <registry description text from registry.yaml>
- **Schema:** `<schema identifier from registry.yaml>`
- **Materialisation:** `symlink` \| `git-clone` \| `missing` (mirror
  `specify workspace status`).
- **Head:** `<40-char sha or —>` when the slot is a git work tree.
- **Dirty:** `yes` \| `no` \| `—`
- **Specify tree:** one bullet each if present: `plan.yaml`, active
  changes under `changes/`, baseline specs under `specs/`, cached
  schema under `.specify/.cache/` — paths relative to the peer slot.

<!-- one `##` section per registry project, alphabetically by name -->
```

Re-running on an unchanged registry + workspace cache MUST yield byte-identical `workspace.md` (stable ordering throughout).

**`--dry-run`.** Do **not** shell `specify workspace sync`; do **not** write `workspace.md`. You MAY print a short preview of what `workspace.md` *would* contain after a real sync, but only to stdout — no writes under `.specify/workspace/` or `.specify/plans/`.

**`--extend`.** Do **not** shell `specify workspace sync` during the sync-peers step — operators refresh clones explicitly between runs. If `.specify/workspace/` already exists, still **rewrite** `workspace.md` from the current on-disk cache (read-only walk) so propose sees an up-to-date peer inventory without an implicit `git fetch`.

Fixture for the inventory shape lives at [`fixtures/plan-layer2/workspace.md`](fixtures/plan-layer2/workspace.md) (placeholder peer names; copy the heading / bullet contract verbatim).

## Step 3(c) — Propose

Step 3(c) invokes the propose brief declared in `pipeline.plan` (for Omnia, `schemas/omnia/briefs/plan/propose.md`; for Vectis, `schemas/vectis/briefs/plan/propose.md`; other schemas ship their own). Propose reads `discovery.md`, applies the schema's slice heuristics to decompose the inventory into draft change slices with `depends-on` edges, and iterates with the human on each slice (accept / edit / reject / abort). For every accepted slice, the skill shells out to:

```text
specify plan add <name> \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--description "..."]
```

The `--description` carries scope and delta-targeting intent in prose; scoping is inferred from the description by the define skill at execution time.

Propose is the single-writer edge for plan entries — every entry lands via `specify plan add`; the skill never edits `plan.yaml` directly (see §"Single-writer invariant"). The full decision trail (accepted, edited, rejected, skipped, aborted slices) is captured in `.specify/plans/<initiative-name>/proposal.md` regardless of per-slice decisions; the proposal header is exactly `# Proposal — <initiative-name>` with the same idempotency contract as `discovery.md`. The shape of a five-slice migration authoring run is pinned by [`fixtures/propose/expected-plan.yaml`](fixtures/propose/expected-plan.yaml) (final `.specify/plan.yaml`), [`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md) (audit trail), [`fixtures/propose/discovery.md`](fixtures/propose/discovery.md) (step 3(a) inventory), and [`fixtures/propose/transcript.md`](fixtures/propose/transcript.md) (the interactive accept / edit / reject transcript). The per-slice prompt shape, the four legal actions (`y` / `edit` / `no` / `abort`), the edit sub-loop, and the rules governing dropped `depends-on` edges when a slice is rejected all live in the propose brief — see the schema's propose brief for the authoritative contract.

On abort, the skill writes `proposal.md` with the slices decided so far, skips step 4's validate (the plan is explicitly incomplete), and exits non-zero pointing the operator at `/spec:plan --extend` to resume. Partial plan entries from earlier accepted slices remain on disk — they were written synchronously by `specify plan add` and the skill never rolls those writes back. On a clean end-of-loop, step 4's `specify plan validate` is the final acceptance gate: any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit.

### Context Auto-Population

When `/spec:plan` inserts changes, it automatically populates the `context` field on plan entries to help briefs focus on relevant baseline paths:

- **Contract changes**: When a contract change is inserted, implementation changes that depend on it get `context` entries for the contract paths the contract change will produce (e.g. `contracts/http/user-api.yaml`, `contracts/schemas/user.yaml`).
- **Spec changes**: When a change targets existing capabilities via `affects`, `context` entries are populated with the corresponding baseline spec paths (e.g. `specs/user-registration/spec.md`).
- **Manual authoring**: Operators can add context paths via `specify plan add --context <path>...` or `specify plan amend --context <path>...`.

Context paths are relative to `.specify/`. They are a focus hint — briefs may still read other baseline paths when instructed to.

## Step 3(d) — Assignment (multi-repo only)

After the propose brief completes step 3(c) and all accepted entries have been written to `plan.yaml` (without `project`), the plan skill runs the assignment pass when `workspace.md` is present and contains more than one project section. Single-project registries skip this step entirely.

**Normative sequence**

1. Read all entries created by the propose brief — the entries with `status: pending` and no `project` field.
2. For each entry, infer a project assignment using the following signal priority:
   - **Description match.** Compare the entry's `description` against each project's `Description` bullet in `workspace.md`. Domain-term overlap is the primary signal.
   - **Baseline spec affinity.** If a peer already has baseline specs whose names or domains overlap with the entry, that peer is a strong candidate. This signal is only available for brownfield (materialised workspace with existing specs listed in the `Specify tree` bullet).
   - **Schema compatibility.** If the entry's nature (e.g. UI vs backend logic) aligns with only one schema type in the registry (via the `Schema` bullet), use that as a tiebreaker.
   - **Ambiguity → human.** When no signal clearly differentiates, or when confidence is low, surface the assignment as "unresolved" and require operator input. Never silently assign a low-confidence match.
3. Present the full assignment table to the operator in a batch review:

   ```markdown
   ## Assignment

   | # | Entry | Project | Rationale |
   |---|---|---|---|
   | 1 | ingest-pipeline | traffic | description overlap: ingestion, Kafka |
   | 2 | operator-dashboard | command-centre | baseline spec: user-alerts exists |
   | 3 | shared-types | ? | ambiguous: matches both projects |
   ```

   The operator reviews the table and can override any assignment. For **unresolved** assignments (`?`), the operator must assign a project before the step can proceed. The `project` prompt is a pick-from-list field — the legal values are the project names from `workspace.md`. Invalid input re-prompts.

4. For each entry, shell out to:
   ```text
   specify plan amend <name> --project <project>
   ```
5. Append the assignment table (with final assignments and rationale) to `proposal.md` so the proposal reconstructs the full decision trail — decomposition (from the propose brief) followed by routing (from the assignment step).

When the registry is absent or single-project, step 3(d) is skipped entirely. No `--project` is written to plan entries.

## Single-writer invariant

Every plan entry this skill writes goes through **`specify plan add`**. The skill never edits `.specify/plan.yaml` directly, never rewrites existing entries, and never bundles multiple entries into a batch write. This preserves the single-writer invariant: exactly two classes of writes touch `plan.yaml` (entry writes via `Plan::{create, amend}` and status writes via `Plan::transition`), and both route through the library.

The invariant extends to `--extend`: additional entries are added via `specify plan add`; pre-existing entries are left untouched. The only path that calls `specify plan amend` is step 3(d) Assignment, which writes `--project` on multi-repo plans. The skill never calls `specify plan transition` — that verb belongs to the running initiative (humans in Layer 1, `/spec:execute` in Layer 2), not to the authoring step.

### Contract Role Population

When `/spec:plan` inserts a contract change for an API boundary between projects, it populates the `contracts` block on the relevant registry project entries:

1. **Producer project**: Add the contract file paths to `contracts.produces` on the project that implements the API.
2. **Consumer project**: Add the contract file paths to `contracts.consumes` on the project that calls the API.
3. **Import changes**: Add the contract file paths to `contracts.imports` on the project integrating with the external system.

Use `specify registry validate` to verify the invariants after populating roles. The validation is advisory — the operator can adjust role assignments.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<initiative-name>/`, mirroring the `.specify/changes/<name>/` pattern used by the phase skills:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <initiative-name>/
        ├── discovery.md            # from the discovery brief (step 3a)
        ├── workspace.md            # from sync-peers (step 3b; multi-repo only)
        ├── proposal.md             # from the propose brief (step 3c) + assignment table (step 3d)
        └── analyze/                # `/spec:analyze` sidecars (legacy-code): `<source-key>/metadata.json`
```

The working directory is created lazily — by the discovery brief itself when it writes `discovery.md`, not by the skill scaffold. Step 2 (`specify plan create`) does not create it.

`.specify/plans/<initiative-name>/analyze/<key>/` is the **tier-1** legacy-source clone — read-only, ephemeral, and bound to this initiative. The **tier-2** registered project clones materialised by step 3(b) live separately under `.specify/workspace/<project>/`, are read-write during execution, and outlive any single initiative. See [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md) for the full contrast.

On archive, `specify plan archive` sweeps this directory alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`, preserving the authoring trail with the plan it produced.

## Modes

Each mode below describes only the *delta* from the core five-step loop. The default mode runs the loop unchanged; `--extend` and `--dry-run` each relax or suppress specific writes.

### Default (no mode flag)

Run the five-step loop exactly as written. `plan.yaml` is initialised via step 2, populated via step 3(c), validated in step 4. A pre-existing `.specify/plan.yaml` is refused at step 1 (the operator is pointed at `specify plan archive`).

### `--extend`

Add to an existing `.specify/plan.yaml` instead of refusing. The skill-level contract is:

- **Step 1 refuses when `plan.yaml` is absent.** `--extend` is an explicit "I know there's a plan here" signal; the skill never silently creates a fresh plan under `--extend`.
- **Step 2 (`specify plan create`) is skipped entirely.**
- **Step 3(a) is skipped when `.specify/plans/<initiative-name>/discovery.md` already exists**, with a log line `Discovery already present; reusing existing inventory.` Discovery is explicitly a one-shot artefact; an operator who wants to refresh it archives the plan (`specify plan archive`) and re-runs without `--extend`. When `discovery.md` does not yet exist under `--extend` (e.g. a plan authored by hand, or an earlier run aborted), step 3(a) runs normally.
- **Step 3(c) skips collisions silently.** Draft slices whose proposed `name` collides with an existing plan entry are recorded in `proposal.md` with decision `skip-existing` and the existing entry's name in the "Plan entry" column; the human is not re-prompted. Slices whose names do not collide run through the usual accept / edit / reject / abort loop.
- **Sync-peers (step 3(b)):** when the registry declares more than one project, **do not** shell `specify workspace sync`. Still regenerate `.specify/plans/<initiative-name>/workspace.md` from the existing `.specify/workspace/` cache (read-only walk) so propose stays deterministic without an implicit `git fetch`.
- **Pre-existing entries are never modified.** The skill never calls `specify plan transition` on existing entries. The only `specify plan amend` call is step 3(d) Assignment (`--project`), which tags newly created entries — it does not modify pre-existing ones. A propose-time decision to modify an existing entry is surfaced to the human, who runs `specify plan amend` by hand outside the authoring loop.

No new flag is introduced beyond `--extend`. A future Change may add `--force-discovery` if refreshing the inventory mid-plan becomes a real need.

### `--dry-run`

Emit a readiness report, the would-be-produced capability inventory, and the would-be-proposed plan to stdout; write nothing. Dry-run folds the L3.E readiness gate, the L3.F discovery preview, and the L3.G propose preview into a single pass.

Under `--dry-run` the skill MUST NOT:

- create `.specify/plans/<initiative-name>/`;
- shell out to `specify plan create`, `specify plan add`, `specify plan amend`, or `specify plan transition`;
- shell out to **`specify workspace sync`** or write **`.specify/plans/<initiative-name>/workspace.md`** (sync-peers dry-run rule);
- write any file under `.specify/` (including under `.specify/workspace/`).

The discovery brief's input-reading side (reading `--from` files, invoking `/spec:analyze` against `--source` / `--against` inputs) runs under `--dry-run` so the preview inventory is real; only the write to `discovery.md` and the `.specify/plans/<name>/` directory creation are suppressed. The propose brief's slice-decomposition pass also runs (the preview plan shape is real against the previewed inventory); the accept / edit / reject loop and every `specify plan add` call are skipped.

Output shape:

```text
[dry-run] /spec:plan — <initiative-name>

Initiative: <initiative-name>
Sources:
  - <key>: <path-or-url>
  - ...
Pipeline: pipeline.plan (<brief-id>, <brief-id>...)

Would write .specify/plans/<initiative-name>/discovery.md:

# Discovery — <initiative-name>

## Capability inventory

### <capability-name>
Source: <key> (<path-or-url>)
Description: ...
Depends-on hints: ...

<!-- one subsection per merged capability -->

## Open questions

- <question>
- <...>

Would propose plan entries (not written):

  1. <slice-name>
       sources:     [<key> ...]
       depends-on:  [<name> ...]
       description: <one-line summary>
  2. ...

Dry-run — no specify plan add calls made.

Would write .specify/plans/<initiative-name>/proposal.md:

# Proposal — <initiative-name>

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | <slice> | <keys> | <names or —> | dry-run | — |
| 2 | ... | ... | ... | dry-run | — |

## Notes

- Dry-run: no specify plan add calls made; no entries written.

Would assign projects (multi-repo only; not written):

| # | Entry | Project | Rationale |
|---|---|---|---|
| 1 | <slice-name> | <project-name> | <rationale> |
| 2 | ... | ... | ... |

Dry-run — no specify plan amend --project calls made.

No files written. Remove --dry-run to run the pipeline.
```

Section rules:

- The `Sources:` block is omitted entirely when `--source` was not supplied. `--from` and `--against` inputs do not surface here — the `Sources:` block pins the top-level `sources` map shape that step 2 would write to `plan.yaml`, not the full set of discovery inputs.
- The `Pipeline:` line names `pipeline.plan` and lists the brief IDs in the order `specify schema pipeline --phase plan` returns.
- The `[dry-run]` banner on the first line is enough — body lines do not need a per-line prefix.
- The `Would write ...:` preamble is emitted before the inventory body so the content is obviously a preview rather than written output.
- The readiness-report portion (banner through `Pipeline:` line) is pinned by [`fixtures/dry-run/expected-output.md`](fixtures/dry-run/expected-output.md). The inventory portion is pinned by [`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md) against the [`fixtures/discovery/legacy/`](fixtures/discovery/legacy/) source tree; under `--dry-run` the same content is emitted to stdout instead of written to disk.
- The proposal preview mirrors [`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md) with every slice's `Decision` column set to `dry-run` and `Plan entry` column set to `—`.

## Non-goals

- **Execute the plan.** Never. Execution is `/spec:execute`'s concern (Layer 2). `/spec:plan` exits with a hand-off summary that points the operator at `/spec:execute --loop`.
- **Modify existing plan entries.** Never. `--extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify plan amend` by the human, not by this skill.
- **Skip `specify plan validate`.** Never. Step 4 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero. This is the contract the skill owes its caller.
- **Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, or `/spec:execute`.** Never. `/spec:plan` only invokes the briefs declared in `schema.yaml`'s `pipeline.plan`, plus the `specify plan` CLI for scaffolding, entry creation, and validation.
- **Hold a driver lock.** Never. `.specify/plan.lock` is reserved for `/spec:execute`; authoring runs outside that lock. A human running `specify plan transition` or `specify plan amend` by hand while `/spec:plan` is authoring is safe because every write goes through the atomic library functions.
- **Write `.specify/plan.yaml` directly.** Never. Every write goes through `specify plan create` (step 2, skipped under `--extend`), `specify plan add` (step 3c, one call per accepted slice), or `specify plan amend` (step 3d, `--project` assignment on multi-repo plans).
- **Clone git URLs from this skill.** Never for **discovery** inputs: `--source` git URLs are passed through to `/spec:analyze` verbatim. Multi-repo **workspace** materialisation is exclusively `specify workspace sync` (Layer 1 CLI), invoked only in the sync-peers step when `len(registry.projects) > 1`.
- **Author propose brief bodies.** Never. The propose brief body is owned by the schema (for Omnia, `schemas/omnia/briefs/plan/propose.md`); the skill only drives the accept / edit / reject loop against whatever the brief emits.
- **Auto-repair a failing `specify plan validate`.** Never. Step 4's validation gate is read-only; any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit.

The state the skill mutates:

1. `.specify/plan.yaml` via `specify plan create` (step 2; skipped under `--extend`), `specify plan add` (step 3c; once per accepted slice), and `specify plan amend` (step 3d; `--project` assignment on multi-repo plans).
2. `.specify/plans/<initiative-name>/discovery.md` written by the discovery brief (step 3a).
3. `.specify/plans/<initiative-name>/proposal.md` written by the propose brief (step 3c).
4. `.specify/plans/<initiative-name>/workspace.md` written by step 3(b) when the registry declares more than one project.

No other on-disk state is written by `/spec:plan` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`. Route every write through `specify plan create` (step 2), `specify plan add` (step 3c), or `specify plan amend` (step 3d, `--project` assignment). The single-writer invariant depends on it.
- Never skip `specify plan validate` (step 4). A plan that ships to `/spec:execute` without a clean validate is a regression; the validator is the contract the skill owes the downstream driver.
- Validate `<initiative-name>` before any filesystem read or CLI shell-out. A bad name should never leave a half-written plan behind.
- For `--dry-run` specifically: the skill MUST NOT shell out to `specify plan create`, `specify plan add`, `specify plan amend`, or `specify plan transition`; MUST NOT create `.specify/plans/<name>/`; MUST NOT write `discovery.md` or any other file under `.specify/`. The discovery brief's input-reading side still runs so the stdout inventory preview is real; only the write-out and directory creation are suppressed. The first-line banner prefixes the rendered output with `[dry-run] ` (the body lines do not need a per-line prefix — the banner is enough).
- For `--extend` specifically: step 2 is skipped in full; step 3(c) only appends entries via `specify plan add` — it never calls `specify plan transition` on existing entries. The only `specify plan amend` call is step 3(d) Assignment (`--project`), which tags newly created entries, not pre-existing ones. Draft slices whose names collide with existing plan entries are skipped with decision `skip-existing` in `proposal.md`; the human is not re-prompted for those. A propose-time decision to modify an existing entry is surfaced to the human, who runs `specify plan amend` by hand outside the authoring loop.
- Treat an unexpected `specify schema pipeline --phase plan` response shape (missing keys, unknown brief IDs, empty pipeline) as a hard failure: print the raw JSON and exit non-zero. Do not speculate about brief ordering.
