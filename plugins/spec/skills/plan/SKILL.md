---
name: plan
description: |
  Author the initial .specify/plan.yaml for an initiative via the
  pipeline.plan brief pipeline. Layer 3 counterpart to /spec:execute:
  /spec:plan writes the plan, /spec:execute runs it.
license: MIT
argument-hint: "<initiative-name> [--from <path>...] [--against <path>] [--source <key>=<path-or-url>...] [--focus <area>] [--extend] [--dry-run]"
---

# Plan skill

Author `.specify/plan.yaml` for a new initiative by running the
`pipeline.plan` brief pipeline declared in the active schema's
`schema.yaml`. `/spec:plan` is the Layer 3 authoring counterpart to
`/spec:execute`: one *writes* the plan, the other *runs* it.

> **Status.** Layer 3 is fully landed as of RFC-2 closeout. Discovery
> (step 3(a)) and propose (step 3(b)) both ship with brief wiring for
> the Omnia and Vectis schemas; the core loop runs end-to-end against
> either. Authoring a plan with `/spec:plan` and then driving it with
> `/spec:execute --loop` is the supported happy path.

## Overview

Specify at authoring time is a three-layer stack — mirror of the
execution stack documented in
[`../execute/SKILL.md`](../execute/SKILL.md):

1. **Plan CLI** (`specify initiative {init, validate, next, status,
   create, amend, transition, archive, lock}`) — the library-backed
   verbs that read and write `.specify/plan.yaml`. The single writer
   of the plan file, used by humans (Layer 1), `/spec:execute` (Layer
   2), and this skill (Layer 3) alike.
2. **Authoring skill** (`/spec:plan`, this one) — the Layer 3 driver
   that runs the `pipeline.plan` brief pipeline and shells out to
   `specify initiative create` for each accepted slice.
3. **Driver skill** (`/spec:execute`) — the Layer 2 automation that
   consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{init, create, amend, transition, archive}`) | Ordered change list with per-entry status. `/spec:plan` writes only via `specify initiative init` (step 2) and `specify initiative create` (step 3b). |
| `.specify/plans/<name>/` | schema (`pipeline.plan` briefs) | Working directory for authoring artefacts — `discovery.md` from the discovery brief, `proposal.md` from the propose brief. Swept by `specify initiative archive` alongside the plan itself (RFC-2 L3.B). |
| `schema.yaml:pipeline.plan` | schema (`Phase::Plan`) | Declares the ordered list of authoring briefs for the project's schema. Resolved via `specify schema pipeline --phase plan`. |

See [RFC-2 §"Layer 3: Plan Authoring"](../../docs/links.md#rfc-2-layer-3)
for the full design, including the
[Core loop](../../docs/links.md#rfc-2-layer-3),
[Plan pipeline briefs](../../docs/links.md#rfc-2-layer-3),
[Working directory](../../docs/links.md#rfc-2-layer-3), and
[Integration with `/spec:execute`](../../docs/links.md#rfc-2-layer-3)
sections.

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

- **`<initiative-name>`** — kebab-case identifier; becomes the plan's
  top-level `name` field. Validated with the same rules as change
  names (regex `^[a-z][a-z0-9-]*$`) before any other work. An invalid
  name is a hard exit with a clear diagnostic — the skill never
  rewrites or "helps" the name.
- **`--from <path>`** — artefact file(s) or directory describing the
  target shape for greenfield authoring. Repeatable. Consumed by the
  discovery brief (L3.F).
- **`--against <path>`** — an existing codebase to delta against, used
  for refactor or modernisation initiatives. Consumed by the discovery
  brief (L3.F).
- **`--source <key>=<path-or-url>`** — a named source for migration.
  Repeatable. The `key` is a kebab-case identifier recorded in the
  plan's top-level `sources` map and referenced by individual plan
  entries via their `sources` list; the `value` is either a local
  filesystem path or a git URL. The skill forwards the tuple verbatim;
  cloning (if any) is the discovery brief's concern via
  `/spec:extract` and `git-cloner`.
- **`--focus <area>`** — optional scoping hint for the propose brief
  (L3.G). Free-form string; the propose brief decides how to interpret
  it.
- **`--extend`** — add to an existing `.specify/plan.yaml` instead of
  refusing. See [§Modes → `--extend`](#--extend) for the full
  contract; summary: step 2 is skipped, discovery is reused when
  `discovery.md` already exists, and draft slices whose names collide
  with existing entries are silently skipped with decision
  `skip-existing`.
- **`--dry-run`** — emit the readiness report and the proposed plan
  to stdout; write nothing. See [§Modes → `--dry-run`](#--dry-run).

At least one of `--from`, `--against`, or `--source` must be supplied.
A bare `/spec:plan <name>` is a hard exit — the skill cannot decide
the initiative's shape without at least one input.

## Core loop (five steps)

Follow these steps in order on every invocation. Each step is
normative; every shell-out is to the Layer 1 `specify` CLI; this
skill writes nothing to `.specify/plan.yaml` directly.

```text
1. Parse inputs; resolve source paths; assert plan.yaml absent
   (or --extend).

   - Validate <initiative-name> as kebab-case. Reject with a hard
     exit on failure.
   - Require at least one of --from, --against, --source. Reject
     with a hard exit on failure.
   - If .specify/plan.yaml exists and --extend was NOT supplied,
     refuse with a diagnostic pointing at `specify initiative
     archive`. (There is no --force. The refusal is deliberate:
     overwriting an existing plan would drop audit history.)
   - If --extend was supplied but .specify/plan.yaml does NOT
     exist, also refuse — there is nothing to extend. Point the
     operator at running without --extend.

2. Scaffold the plan.

     specify initiative init <initiative-name> \
         [--source <key>=<path-or-url> ...]

   Writes an empty .specify/plan.yaml with just the initiative
   `name` and the supplied `--source` entries in the top-level
   `sources` map. `changes: []` until step 3(b) populates it.

   Skipped entirely when --extend is set: the caller is explicitly
   adding to an existing plan, and `specify initiative init`
   refuses when .specify/plan.yaml already exists (see RFC-2
   §"CLI support").

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
     b. propose   — see §"Step 3(b) — Propose" below.

4. Final validation gate.

     specify initiative validate

   Runs the Layer 1 validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller — a plan that ships to `/spec:execute`
   without passing `specify initiative validate` is a regression.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify initiative status` — review the authored plan.
     - `/spec:execute --loop` — start executing it (Layer 2).

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Step 3(a) — Discovery

Step 3(a) invokes the discovery brief declared in `pipeline.plan` (for
Omnia, [`schemas/omnia/briefs/plan/discovery.md`](../../docs/links.md#omnia-discovery);
other schemas ship their own). Discovery consumes the `--from`,
`--against`, and `--source` inputs, invoking `/spec:extract` where a
source tree needs to be parsed, and merges the results into a single
neutral capability inventory at
`.specify/plans/<initiative-name>/discovery.md`. The skill's job is
to faithfully run the brief and pass inputs through; the algorithm
(per-input handling, dedup rules, ordering) lives in the brief — see
[`schemas/omnia/briefs/plan/discovery.md`](../../docs/links.md#omnia-discovery)
for the authoritative contract.

Discovery is read-only with respect to `plan.yaml`. The output header
is exactly `# Discovery — <initiative-name>` with no timestamps, run
IDs, or working-directory paths, and re-running discovery on unchanged
inputs MUST produce byte-equivalent output — the brief owns the
ordering, the skill does not impose its own. An existing
`discovery.md` is overwritten unless `--extend` is set (see
[§Modes → `--extend`](#--extend)). The shape of a single-`--source`
inventory against a small pre-seeded source tree is pinned by
[`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md)
against [`fixtures/discovery/legacy/`](fixtures/discovery/legacy/).

## Step 3(b) — Propose

Step 3(b) invokes the propose brief declared in `pipeline.plan` (for
Omnia, [`schemas/omnia/briefs/plan/propose.md`](../../docs/links.md#omnia-propose);
for Vectis, [`schemas/vectis/briefs/plan/propose.md`](../../docs/links.md#vectis-propose);
other schemas ship their own). Propose reads `discovery.md`, applies
the schema's slice heuristics to decompose the inventory into draft
change slices with `depends-on` / `affects` edges, and iterates with
the human on each slice (accept / edit / reject / abort). For every
accepted slice, the skill shells out to:

```text
specify initiative create <name> \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--affects <name> ...] \
    [--description "..."]
```

Propose is the single-writer edge for plan entries — every entry
lands via `specify initiative create`; the skill never edits
`plan.yaml` directly (see §"Single-writer invariant"). The full
decision trail (accepted, edited, rejected, skipped, aborted slices)
is captured in `.specify/plans/<initiative-name>/proposal.md`
regardless of per-slice decisions; the proposal header is exactly
`# Proposal — <initiative-name>` with the same idempotency contract
as `discovery.md`. The per-slice prompt shape, the four legal actions
(`y` / `edit` / `no` / `abort`), the edit sub-loop, and the rules
governing dropped `depends-on` edges when a slice is rejected all
live in the propose brief — see
[`schemas/omnia/briefs/plan/propose.md`](../../docs/links.md#omnia-propose)
for the authoritative contract. The shape of a five-slice migration
authoring run is pinned by
[`fixtures/propose/expected-plan.yaml`](fixtures/propose/expected-plan.yaml)
(final `.specify/plan.yaml`),
[`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md)
(audit trail), [`fixtures/propose/discovery.md`](fixtures/propose/discovery.md)
(step 3(a) inventory), and
[`fixtures/propose/transcript.md`](fixtures/propose/transcript.md)
(the interactive accept / edit / reject transcript).

On abort, the skill writes `proposal.md` with the slices decided so
far, skips step 4's validate (the plan is explicitly incomplete), and
exits non-zero pointing the operator at `/spec:plan --extend` to
resume. Partial plan entries from earlier accepted slices remain on
disk — they were written synchronously by `specify initiative create`
and the skill never rolls those writes back. On a clean end-of-loop,
step 4's `specify initiative validate` is the final acceptance gate:
any `Error`-level finding surfaces to the human with a recommended
`specify initiative amend` / `specify initiative transition skipped`
fix, never an in-skill edit.

## Single-writer invariant (RFC-2 §"Phase Boundary → Rule 2")

Every plan entry this skill writes goes through **`specify initiative
create`**. The skill never edits `.specify/plan.yaml` directly, never
rewrites existing entries, and never bundles multiple entries into a
batch write. This preserves the single-writer invariant established
in RFC-2 §"Phase Boundary → Rule 2": exactly two classes of writes
touch `plan.yaml` (entry writes via `Plan::{create, amend}` and
status writes via `Plan::transition`), and both route through the
library.

The invariant extends to `--extend`: additional entries are added via
`specify initiative create`; pre-existing entries are left untouched.
The skill has no path that calls `specify initiative amend` or
`specify initiative transition` — those verbs belong to the running
initiative (humans in Layer 1, `/spec:execute` in Layer 2), not to
the authoring step.

See [RFC-2 §"Phase Boundary → Rule 2"](../../docs/links.md#rfc-2-phase-boundary-rule-2)
for the full contract.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<initiative-name>/`,
mirroring the `.specify/changes/<name>/` pattern used by the phase
skills:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <initiative-name>/
        ├── discovery.md            # from the discovery brief (step 3a)
        └── proposal.md             # from the propose brief (step 3b)
```

The working directory is created lazily — by the discovery brief
itself when it writes `discovery.md`, not by the skill scaffold. Step
2 (`specify initiative init`) does not create it.

On archive, `specify initiative archive` (RFC-2 L3.B) sweeps this
directory alongside `plan.yaml` into
`.specify/archive/plans/<name>-<YYYYMMDD>/`, preserving the authoring
trail with the plan it produced.

## Modes

Each mode below describes only the *delta* from the core five-step
loop. The default mode runs the loop unchanged; `--extend` and
`--dry-run` each relax or suppress specific writes.

### Default (no mode flag)

Run the five-step loop exactly as written. `plan.yaml` is initialised
via step 2, populated via step 3(b), validated in step 4. A
pre-existing `.specify/plan.yaml` is refused at step 1 (the operator
is pointed at `specify initiative archive`).

### `--extend`

Add to an existing `.specify/plan.yaml` instead of refusing. The
skill-level contract is:

- **Step 1 refuses when `plan.yaml` is absent.** `--extend` is an
  explicit "I know there's a plan here" signal; the skill never
  silently creates a fresh plan under `--extend`.
- **Step 2 (`specify initiative init`) is skipped entirely.**
- **Step 3(a) is skipped when
  `.specify/plans/<initiative-name>/discovery.md` already exists**,
  with a log line `Discovery already present; reusing existing
  inventory.` Discovery is explicitly a one-shot artefact; an
  operator who wants to refresh it archives the plan
  (`specify initiative archive`) and re-runs without `--extend`. When
  `discovery.md` does not yet exist under `--extend` (e.g. a plan
  authored by hand, or an earlier run aborted), step 3(a) runs
  normally.
- **Step 3(b) skips collisions silently.** Draft slices whose
  proposed `name` collides with an existing plan entry are recorded
  in `proposal.md` with decision `skip-existing` and the existing
  entry's name in the "Plan entry" column; the human is not
  re-prompted. Slices whose names do not collide run through the
  usual accept / edit / reject / abort loop.
- **Pre-existing entries are never modified.** The skill has no path
  that calls `specify initiative amend` or `specify initiative
  transition` — a propose-time decision to modify an existing entry
  is surfaced to the human, who runs `specify initiative amend` by
  hand outside the authoring loop.

No new flag is introduced beyond `--extend`. A future Change may add
`--force-discovery` if refreshing the inventory mid-plan becomes a
real need; RFC-2 L3.F explicitly does not.

### `--dry-run`

Emit a readiness report, the would-be-produced capability inventory,
and the would-be-proposed plan to stdout; write nothing. Dry-run
folds the L3.E readiness gate, the L3.F discovery preview, and the
L3.G propose preview into a single pass.

Under `--dry-run` the skill MUST NOT:

- create `.specify/plans/<initiative-name>/`;
- shell out to `specify initiative init`, `specify initiative create`,
  `specify initiative amend`, or `specify initiative transition`;
- write any file under `.specify/`.

The discovery brief's input-reading side (reading `--from` files,
invoking `/spec:extract` against `--source` / `--against` inputs)
runs under `--dry-run` so the preview inventory is real; only the
write to `discovery.md` and the `.specify/plans/<name>/` directory
creation are suppressed. The propose brief's slice-decomposition pass
also runs (the preview plan shape is real against the previewed
inventory); the accept / edit / reject loop and every
`specify initiative create` call are skipped.

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

Dry-run — no specify initiative create calls made.

Would write .specify/plans/<initiative-name>/proposal.md:

# Proposal — <initiative-name>

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | <slice> | <keys> | <names or —> | dry-run | — |
| 2 | ... | ... | ... | dry-run | — |

## Notes

- Dry-run: no specify initiative create calls made; no entries written.

No files written. Remove --dry-run to run the pipeline.
```

Section rules:

- The `Sources:` block is omitted entirely when `--source` was not
  supplied. `--from` and `--against` inputs do not surface here — the
  `Sources:` block pins the top-level `sources` map shape that step 2
  would write to `plan.yaml`, not the full set of discovery inputs.
- The `Pipeline:` line names `pipeline.plan` and lists the brief IDs
  in the order `specify schema pipeline --phase plan` returns.
- The `[dry-run]` banner on the first line is enough — body lines do
  not need a per-line prefix.
- The `Would write ...:` preamble is emitted before the inventory
  body so the content is obviously a preview rather than written
  output.
- The readiness-report portion (banner through `Pipeline:` line) is
  pinned by [`fixtures/dry-run/expected-output.md`](fixtures/dry-run/expected-output.md).
  The inventory portion is pinned by
  [`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md)
  against the [`fixtures/discovery/legacy/`](fixtures/discovery/legacy/)
  source tree; under `--dry-run` the same content is emitted to
  stdout instead of written to disk.
- The proposal preview mirrors
  [`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md)
  with every slice's `Decision` column set to `dry-run` and
  `Plan entry` column set to `—`.

## Non-goals

- **Execute the plan.** Never. Execution is `/spec:execute`'s concern
  (Layer 2). `/spec:plan` exits with a hand-off summary that points
  the operator at `/spec:execute --loop`.
- **Modify existing plan entries.** Never. `--extend` is append-only;
  pre-existing entries are left untouched. Editing a pending entry
  mid-authoring is done via `specify initiative amend` by the human,
  not by this skill.
- **Skip `specify initiative validate`.** Never. Step 4 is
  unconditional — every run ends with a validation gate, and a
  non-clean validate exits non-zero. This is the contract the skill
  owes its caller.
- **Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`,
  or `/spec:execute`.** Never. `/spec:plan` only invokes the briefs
  declared in `schema.yaml`'s `pipeline.plan`, plus the `specify
  initiative` CLI for scaffolding, entry creation, and validation.
- **Hold a driver lock.** Never. `.specify/plan.lock` is reserved for
  `/spec:execute`; authoring runs outside that lock. A human running
  `specify initiative transition` or `specify initiative amend` by
  hand while `/spec:plan` is authoring is safe because every write
  goes through the atomic library functions.
- **Write `.specify/plan.yaml` directly.** Never. Every write goes
  through `specify initiative init` (step 2, skipped under `--extend`)
  or `specify initiative create` (step 3b, one call per accepted
  slice).
- **Clone git URLs.** Never. `--source` values that are git URLs are
  passed through to `/spec:extract` verbatim; cloning (if any)
  happens inside `/spec:extract` via `git-cloner`.
- **Author propose brief bodies.** Never. The propose brief body is
  owned by the schema (for Omnia,
  [`schemas/omnia/briefs/plan/propose.md`](../../docs/links.md#omnia-propose)
  from L3.D); the skill only drives the accept / edit / reject loop
  against whatever the brief emits.
- **Auto-repair a failing `specify initiative validate`.** Never.
  Step 4's validation gate is read-only; any `Error`-level finding
  surfaces to the human with a recommended `specify initiative amend`
  / `specify initiative transition skipped` fix, never an in-skill
  edit.

The state the skill mutates:

1. `.specify/plan.yaml` via `specify initiative init` (step 2;
   skipped under `--extend`) and `specify initiative create` (step
   3b; once per accepted slice).
2. `.specify/plans/<initiative-name>/discovery.md` written by the
   discovery brief (step 3a).
3. `.specify/plans/<initiative-name>/proposal.md` written by the
   propose brief (step 3b).

No other on-disk state is written by `/spec:plan` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`. Route every write through
  `specify initiative init` (step 2) or `specify initiative create`
  (step 3b). The single-writer invariant in RFC-2 §"Plan Mutation and
  Crash Safety" depends on it.
- Never skip `specify initiative validate` (step 4). A plan that
  ships to `/spec:execute` without a clean validate is a regression;
  the validator is the contract the skill owes the downstream driver.
- Validate `<initiative-name>` before any filesystem read or CLI
  shell-out. A bad name should never leave a half-written plan
  behind.
- For `--dry-run` specifically: the skill MUST NOT shell out to
  `specify initiative init`, `specify initiative create`, `specify
  initiative amend`, or `specify initiative transition`; MUST NOT
  create `.specify/plans/<name>/`; MUST NOT write `discovery.md` or
  any other file under `.specify/`. The discovery brief's
  input-reading side still runs so the stdout inventory preview is
  real; only the write-out and directory creation are suppressed. The
  first-line banner prefixes the rendered output with `[dry-run] `
  (the body lines do not need a per-line prefix — the banner is
  enough).
- For `--extend` specifically: step 2 is skipped in full; step 3(b)
  only appends entries via `specify initiative create` — it never
  calls `specify initiative amend` or `specify initiative transition`
  on existing entries. Draft slices whose names collide with existing
  plan entries are skipped with decision `skip-existing` in
  `proposal.md`; the human is not re-prompted for those. A
  propose-time decision to modify an existing entry is surfaced to
  the human, who runs `specify initiative amend` by hand outside the
  authoring loop.
- Treat an unexpected `specify schema pipeline --phase plan` response
  shape (missing keys, unknown brief IDs, empty pipeline) as a hard
  failure: print the raw JSON and exit non-zero. Do not speculate
  about brief ordering.
