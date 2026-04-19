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

> **Status.** Layer 3 is fully landed as of RFC-2 closeout.
> Discovery (step 3(a)) and propose (step 3(b)) both ship with
> brief wiring for the Omnia and Vectis schemas; the core loop
> runs end-to-end against either. Authoring a plan with
> `/spec:plan` and then driving it with `/spec:execute --loop`
> is the supported happy path.

## Overview

Specify at authoring time is a three-layer stack — mirror of the
execution stack documented in [`../execute/SKILL.md`](../execute/SKILL.md):

1. **Plan CLI** (`specify plan {init, validate, next, status, create,
   amend, transition, archive, lock}`) — the library-backed verbs that
   read and write `.specify/plan.yaml`. The single writer of the plan
   file, used by humans (Layer 1), `/spec:execute` (Layer 2), and this
   skill (Layer 3) alike.
2. **Authoring skill** (`/spec:plan`, this one) — the Layer 3 driver
   that runs the `pipeline.plan` brief pipeline and shells out to
   `specify plan create` for each accepted slice.
3. **Driver skill** (`/spec:execute`) — the Layer 2 automation that
   consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{init, create, amend, transition, archive}`) | Ordered change list with per-entry status. `/spec:plan` writes only via `specify plan init` (step 2) and `specify plan create` (step 3b). |
| `.specify/plans/<name>/` | schema (`pipeline.plan` briefs) | Working directory for authoring artefacts — `discovery.md` from the discovery brief, `proposal.md` from the propose brief. Swept by `specify plan archive` alongside the plan itself (RFC-2 L3.B). |
| `schema.yaml:pipeline.plan` | schema (`Phase::Plan`) | Declares the ordered list of authoring briefs for the project's schema. Resolved via `specify schema pipeline --phase plan`. |

See [RFC-2 §"Layer 3: Plan Authoring"](../../../../rfcs/archive/rfc-2-execution.md)
for the full design, including the [Core loop](../../../../rfcs/archive/rfc-2-execution.md),
[Plan pipeline briefs](../../../../rfcs/archive/rfc-2-execution.md),
[Working directory](../../../../rfcs/archive/rfc-2-execution.md), and
[Integration with `/spec:execute`](../../../../rfcs/archive/rfc-2-execution.md)
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
  cloning (if any) is the discovery brief's concern via `/spec:extract`
  and `git-cloner`.
- **`--focus <area>`** — optional scoping hint for the propose brief
  (L3.G). Free-form string; the propose brief decides how to interpret
  it.
- **`--extend`** — add to an existing `.specify/plan.yaml` instead of
  refusing. Skips step 2 (scaffold) and reopens the propose loop
  against the existing plan; existing entries are never modified —
  `--extend` is additive-only. If
  `.specify/plans/<initiative-name>/discovery.md` already exists,
  step 3(a) is also skipped and the existing inventory is reused;
  otherwise step 3(a) runs normally. In step 3(b), each draft
  slice whose proposed `name` collides with an existing plan
  entry is skipped without prompting the human — the existing
  entry stands, and the collision is recorded in `proposal.md` as
  decision `skip-existing`. Only slices with fresh names run
  through the accept / edit / reject loop. To refresh the
  inventory, archive the plan (`specify plan archive`) and re-run
  without `--extend`.
- **`--dry-run`** — emit the readiness report and the proposed plan to
  stdout; write nothing. See §"Dry-run output".

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
     refuse with a diagnostic pointing at `specify plan archive`.
     (There is no --force. The refusal is deliberate: overwriting
     an existing plan would drop audit history.)
   - If --extend was supplied but .specify/plan.yaml does NOT
     exist, also refuse — there is nothing to extend. Point the
     operator at running without --extend.

2. Scaffold the plan.

     specify plan init <initiative-name> \
         [--source <key>=<path-or-url> ...]

   Writes an empty .specify/plan.yaml with just the initiative
   `name` and the supplied `--source` entries in the top-level
   `sources` map. `changes: []` until step 3(b) populates it.

   Skipped entirely when --extend is set: the caller is explicitly
   adding to an existing plan, and `specify plan init` refuses
   when .specify/plan.yaml already exists (see RFC-2 §"CLI
   support").

3. Run the plan brief pipeline from schema.yaml.

   Resolve the ordered list of briefs via:

     specify schema pipeline --phase plan \
         --change .specify/plans/<name> --format json

   The response lists every brief in the schema's `pipeline.plan`
   declaration in topological order, with each brief's absolute
   `path`, `needs` edges, `generates` target, and current
   `present` flag relative to this initiative's working directory.

   Then run each brief in order:

     a. discovery  — read --from artefacts directly; invoke
                     /spec:extract once per --source and --against
                     input; merge the results into a consolidated
                     capability inventory at
                     .specify/plans/<name>/discovery.md. See
                     §"Step 3(a) — Discovery" below for the full
                     algorithm.

     b. propose    — read discovery.md; decompose into change
                     slices with `depends-on` edges using the
                     schema's slice heuristics; materialise a
                     draft; iterate with the human (accept /
                     edit / reject / abort per slice); for each
                     accepted slice, shell out to:

                       specify plan create <name> \
                           [--sources <key> ...] \
                           [--depends-on <other-name> ...] \
                           [--affects <other-name> ...] \
                           [--description "..."]

                     The full proposal is captured in
                     .specify/plans/<name>/proposal.md regardless
                     of per-slice decisions. See §"Step 3(b) —
                     Propose" below for the full algorithm.

4. Final validation gate.

     specify plan validate

   Runs the Layer 1 validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller — a plan that ships to `/spec:execute`
   without passing `specify plan validate` is a regression.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify plan status` — review the authored plan.
     - `/spec:execute --loop` — start executing it (Layer 2).

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Step 3(a) — Discovery

Step 3(a) invokes the discovery brief declared in `pipeline.plan`
(for Omnia, [`schemas/omnia/briefs/plan/discovery.md`](../../../../schemas/omnia/briefs/plan/discovery.md)
from L3.D) and produces the neutral capability inventory that step
3(b) will decompose. Discovery is read-only with respect to
`plan.yaml` — its only output is
`.specify/plans/<initiative-name>/discovery.md`.

### Algorithm

1. **Create the working directory.** Create
   `.specify/plans/<initiative-name>/` if it doesn't already exist.
   Skipped under `--dry-run` — see §"Dry-run output".

2. **Analyse each `--source` input.** For each
   `--source <key>=<path-or-url>` argument:
   - If `<path-or-url>` looks like a git URL, invoke
     `/spec:extract <url> .specify/plans/<name>/extract/<key>/`.
     `/spec:extract` composes Omnia's `git-cloner` (to clone the
     repo into a working location) and `analyze` (to parse and
     emit capabilities) plugins internally; discovery never calls
     those plugins directly.
   - If `<path-or-url>` is a local filesystem path, invoke
     `/spec:extract <path> .specify/plans/<name>/extract/<key>/`
     directly (no clone).
   - Capture the capabilities emitted by extract per source,
     tagged with the source `<key>`.

3. **Analyse the `--against` input (if any).** Treat `--against
   <path>` as a synonym for `--source against=<path>` with a
   downstream hint to the brief that the initiative is a delta
   against an existing codebase rather than a greenfield
   migration. Invoke `/spec:extract <path>
   .specify/plans/<name>/extract/against/` the same way. The
   `--against` input is always interpreted as a local path —
   against-a-remote is out of scope.

4. **Read each `--from` artefact.** For each `--from <path>`,
   open the file (or every file under a `--from` directory)
   directly. These are human-authored artefacts (briefs, RFCs,
   product docs, ADRs); `/spec:extract` is not invoked. Parse
   any clearly-delimited capability structure (headings,
   bulleted capability lists); otherwise treat each top-level
   heading as a capability candidate and record the accompanying
   prose verbatim as its description.

5. **Merge into a single inventory.** Deduplicate capabilities
   by name across sources — a capability mentioned in both a
   `--from` brief and a `--source` extract surfaces once, with
   every source that mentioned it listed on the entry. Order
   the inventory stably: capabilities group by the source they
   first appeared in (invocation order of `--source` / `--from` /
   `--against`), and within a source they keep the order
   `/spec:extract` (or the `--from` file parser) emitted them.
   This yields a deterministic output without requiring
   per-source alphabetic sorting — the on-disk order reflects the
   shape of the source tree, which is what human reviewers expect
   when skimming the inventory alongside the legacy code.

6. **Write `.specify/plans/<initiative-name>/discovery.md`.**
   Shape:

   ```markdown
   # Discovery — <initiative-name>

   ## Capability inventory

   ### <capability-name>
   Source: <key> (<path-or-url>)[, <key2> (<path2>)...]
   Description: <one or two sentences, source-neutral>
   Depends-on hints: <other-capability>, <...>  (omit if none)
   Scope hints: <free-form>  (omit if none)

   ## Open questions

   - <question requiring human input before propose>
   - <...>
   ```

   The header is exactly `# Discovery — <initiative-name>` — no
   date, no run ID, no working-directory paths. This is a hard
   idempotency requirement: running discovery twice on the same
   inputs MUST produce byte-equivalent output (see
   [`schemas/omnia/briefs/plan/discovery.md`](../../../../schemas/omnia/briefs/plan/discovery.md)
   §"Idempotency"). Any existing `discovery.md` is overwritten.

7. **Emit a one-line-per-input summary to stdout.** Shape:

   ```text
   Discovery:
     - <path-or-url> (--source <key>): <N> capabilities
     - <path-or-url> (--source <key>): <N> capabilities
     - <path> (--from): <N> capability hints
   Inventory written to .specify/plans/<initiative-name>/discovery.md
   ```

   One line per input in invocation order; counts are taken from
   the merged inventory, not the per-source extract output (a
   capability that recurs in three inputs counts once per input
   line but appears once in the inventory).

### Idempotency

Running step 3(a) twice on the same inputs MUST produce a
byte-equivalent `discovery.md`. This is enforced by:

- Stable capability ordering (grouped by source in invocation
  order; within a source, the extract/parser emission order is
  preserved — see algorithm step 5).
- No timestamps, run IDs, or working-directory paths in the
  output (the header is `# Discovery — <initiative-name>`
  exactly).
- `/spec:extract` re-runs on unchanged sources produce
  equivalent inventory text; if a re-extract surfaces new
  detail, it replaces the prior inventory entry wholesale rather
  than appending.

The acceptance gate for this step (RFC-2 L3.F) is: running the
discovery step twice in a row overwrites `discovery.md` with
equivalent content.

### `--extend` and existing `discovery.md`

When `--extend` is set AND
`.specify/plans/<initiative-name>/discovery.md` already exists,
step 3(a) is SKIPPED. The skill logs:

```text
Discovery already present; reusing existing inventory.
```

and proceeds directly to step 3(b) against the existing file.
`--extend` is additive-only at the `plan.yaml` level (§"Single-
writer invariant"); re-running discovery automatically would
churn the inventory every time an operator added a single slice.
Operators who want to refresh the inventory archive the plan
(`specify plan archive`) and re-run `/spec:plan` without
`--extend`.

When `--extend` is set but `discovery.md` does not yet exist
(e.g. the plan was authored by hand, or an earlier `/spec:plan`
run aborted mid-way), step 3(a) runs normally and writes a fresh
inventory. The skill does not refuse; the absence of
`discovery.md` under `--extend` is interpreted as "fill in the
missing inventory", not as a hard error.

No new flag is introduced by this Change — the `--extend` switch
is sufficient. A future Change may add a `--force-discovery`
flag if refreshing the inventory mid-plan becomes a real need;
RFC-2 L3.F explicitly does not.

### Reference fixture

The shape of a single-`--source` inventory against a small
pre-seeded source tree is pinned by
[`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md)
against the `legacy/` source tree under
[`fixtures/discovery/legacy/`](fixtures/discovery/legacy/). The
golden is pinned by hand (there is no automated test harness in
this Change); it captures the intent of the algorithm above and
serves as a reference for what a brief-driven run on that input
should produce.

## Step 3(b) — Propose

Step 3(b) invokes the propose brief declared in `pipeline.plan`
(for Omnia, [`schemas/omnia/briefs/plan/propose.md`](../../../../schemas/omnia/briefs/plan/propose.md)
from L3.D) and produces the final `plan.yaml` via per-slice
`specify plan create` calls. Propose is the single-writer edge
for plan entries — every entry lands via `specify plan create`;
the skill never edits `plan.yaml` directly.

### Algorithm

1. **Read the discovery inventory.** Open
   `.specify/plans/<initiative-name>/discovery.md` (written by
   step 3(a)). If the file is absent, exit non-zero with a
   diagnostic pointing at re-running without `--extend`, or at
   archiving the plan if the operator intended a refresh.

2. **Apply schema-specific slice heuristics.** The brief itself
   owns the heuristics; the skill faithfully follows whatever
   the propose brief emits:

   - **Omnia** ([`schemas/omnia/briefs/plan/propose.md`](../../../../schemas/omnia/briefs/plan/propose.md),
     L3.D): one plan entry per WASM crate or cohesive handler
     group; leaf services first (favour entries with few
     dependents); cross-cutting refactors ("extract shared
     validation", "consolidate error types", etc.) become
     standalone entries with explicit `depends-on` edges from
     the feature slices that need them — never folded into a
     feature slice. `sources` points at the discovery `--source`
     key the slice migrates from (or `against` for delta
     initiatives; greenfield slices reference the literal
     `--from` artefact path).
   - **Vectis** ([`schemas/vectis/briefs/plan/propose.md`](../../../../schemas/vectis/briefs/plan/propose.md)):
     shared-core-first, per-shell-last — mirror of the Omnia
     heuristic for the Crux stack. See the brief for the full
     heuristic (core crate slices before iOS / Android / design-
     system slices; cross-shell refactors as standalone entries).
   - **Other schemas** ship their own `propose.md`; the
     decomposition rules come from there. The skill never
     second-guesses a schema brief.

3. **Materialise a draft proposal.** Derive an ordered list of
   slices (leaves first per the active brief), each with:

   - Proposed `name` (kebab-case).
   - Proposed `sources` (keys from the plan's top-level
     `sources` map; empty list for greenfield).
   - Proposed `depends-on` (names of preceding slices in the
     draft, seeded from discovery's depends-on hints).
   - Proposed `affects` (for cross-cutting slices).
   - A one-sentence `description`.

4. **Iterate with the human.** For each slice in draft order,
   present the proposal and read an action:

   ```text
   Slice 2/5: email-verification
     sources: [monolith]
     depends-on: [user-registration]
     description: Verify user email via a one-time link.

   Accept? [y / edit / no / abort]
   ```

   Four actions are legal:

   - **accept (`y`)** → step 5.
   - **edit** → step 6.
   - **reject (`no`)** → step 7.
   - **abort** → step 8.

   Slices are presented in the heuristic order produced by
   step 3; decisions on earlier slices never re-order later
   ones beyond dropping stale `depends-on` edges (see step 7).

5. **Accept.** Shell out to:

   ```text
   specify plan create <name> \
       [--sources <key> ...] \
       [--depends-on <name> ...] \
       [--affects <name> ...] \
       [--description "..."]
   ```

   One flag repetition per list value. `specify plan create`
   writes the entry atomically and re-runs `Plan::validate`
   before saving, so a write that would break the plan is
   refused at this point (propose keeps going with the next
   slice — the rejected entry is recorded with decision
   `create-failed` in the proposal's Notes section). Record the
   final entry name in the proposal table.

6. **Edit.** Re-prompt for the changed field(s) (`name`,
   `sources`, `depends-on`, `affects`, `description`); update
   the draft slice in place and loop back to step 4 against the
   updated slice. The edit count on a single slice is unbounded
   in principle; in practice the human accepts or rejects after
   a small number of passes. Record the decision as
   `edit → accept` (or `edit → reject`) in the proposal table,
   capturing the delta between the original draft and the final
   form.

7. **Reject.** Drop the slice entirely. Later slices that held
   an implicit `depends-on` on the rejected slice lose that
   edge; if a later slice is semantically blocked by the
   rejection (the brief flags it during that slice's review),
   the human decides whether to reject the dependent slice too
   or carry on with a reduced set. Record the decision as
   `reject` in the proposal table.

8. **Abort.** Stop the slice loop immediately. Partial plan
   entries remain on disk — they were written synchronously by
   `specify plan create` as each earlier slice was accepted,
   and the skill never rolls those writes back. The skill then:

   - Writes `proposal.md` with the slices decided so far
     (accepted + edited + rejected) and a Notes entry recording
     the abort at slice `<N>/<total>`.
   - Skips step 9's validate (the plan is explicitly incomplete
     — running validate now would surface expected errors from
     dangling `depends-on` edges, which is noise rather than
     signal).
   - Exits non-zero with a summary that points the operator at
     `/spec:plan --extend` to resume.

9. **Write `.specify/plans/<initiative-name>/proposal.md`.** Once
   every slice has a decision (including a clean abort), write
   the proposal regardless of per-slice outcomes. Shape:

   ```markdown
   # Proposal — <initiative-name>

   ## Slices

   | # | Slice | Source(s) | Depends on | Decision | Plan entry |
   |---|---|---|---|---|---|
   | 1 | <proposed name> | <keys> | <slice names or —> | accept | <final name> |
   | 2 | ... | ... | ... | edit → accept | <final name> |
   | 3 | ... | ... | ... | reject | — |

   ## Notes

   - <free-form notes: why slices were edited, why rejected,
     deferred work, unresolved open questions from discovery,
     abort context if applicable>
   ```

   The table MUST list every slice presented to the human —
   edited and rejected rows alongside accepted ones — so the
   proposal reconstructs the full decision trail. The heading is
   exactly `# Proposal — <initiative-name>`; no dates, run IDs,
   or working-directory paths (same idempotency contract as
   `discovery.md`).

10. **Validate the authored plan.** Run:

    ```text
    specify plan validate
    ```

    Print every `ValidationResult` verbatim. On any `Error`-level
    finding, recommend `specify plan amend <name> ...` to fix the
    offending entry (or `specify plan transition <name> skipped
    --reason "..."` to exclude it) and exit non-zero. Do NOT
    auto-repair plan errors from within the skill — human triage
    is required. A clean validate is the final acceptance gate.

11. **Exit with a hand-off summary.** Shape:

    ```text
    Plan authored: <initiative-name>
    Entries: <A> accepted (<E> edited, <R> rejected, <B> aborted)
    Proposal: .specify/plans/<initiative-name>/proposal.md
    Validate: OK           (or: N errors — see above)

    Next:
      - Review: specify plan status
      - Execute: /spec:execute --loop
    ```

    `<A>` is the count of accepted slices (including edited-then-
    accepted); `<E>` is the subset of accepted slices that went
    through at least one edit; `<R>` is the count of rejected
    slices; `<B>` is the count of slices not presented because
    of a mid-loop abort (or `0` on a clean run).

### `--dry-run` and `--extend`

Propose's dry-run and extend behaviours are folded into the
skill-level §"Dry-run output" and §"Constraints → `--extend`..."
sections — see those for the full rules. Summary:

- **`--dry-run`**: skip steps 4–10 entirely. Emit the proposed
  plan shape (slice names, `depends-on` edges, descriptions) and
  the `proposal.md` *preview* to stdout, prefixed with
  `Dry-run — no specify plan create calls made.`. Write nothing
  to disk; make no `specify plan create` calls.
- **`--extend`**: step 2 (`specify plan init`) is already skipped
  at the skill level; within propose, compare each draft slice's
  proposed `name` against existing `plan.yaml` entries. For each
  collision, skip the slice (do not re-present to the human) and
  record decision `skip-existing` in the proposal with the
  existing entry's name in the "Plan entry" column. Slices whose
  names do NOT collide run through the usual accept / edit /
  reject / abort loop.

### Reference fixture

The shape of a five-slice migration authoring run is pinned by
[`fixtures/propose/expected-plan.yaml`](fixtures/propose/expected-plan.yaml)
(final `.specify/plan.yaml` after the five `specify plan create`
calls), [`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md)
(the authoring audit trail), [`fixtures/propose/discovery.md`](fixtures/propose/discovery.md)
(the step 3(a) inventory the brief consumes), and
[`fixtures/propose/transcript.md`](fixtures/propose/transcript.md)
(the interactive accept / edit / reject transcript that produced
the plan). The expected plan is byte-identical to what
`serde_yaml` + `Plan::save` emits, so it doubles as a regression
pin for the Layer 1 serialization format. It mirrors RFC-2
§"Worked example: migration authoring" and the shape of RFC-2
§"The Plan" for the five equivalent slices (the RFC's
`platform-v2` plan has additional cross-cutting entries that
this fixture deliberately simplifies out).

## Single-writer invariant (RFC-2 §"Phase Boundary → Rule 2")

Every plan entry this skill writes goes through **`specify plan
create`**. The skill never edits `.specify/plan.yaml` directly, never
rewrites existing entries, and never bundles multiple entries into a
batch write. This preserves the single-writer invariant established
in RFC-2 §"Phase Boundary → Rule 2": exactly two classes of writes
touch `plan.yaml` (entry writes via `Plan::{create, amend}` and
status writes via `Plan::transition`), and both route through the
library.

The invariant extends to `--extend`: additional entries are added via
`specify plan create`; pre-existing entries are left untouched. The
skill has no path that calls `specify plan amend` or `specify plan
transition` — those verbs belong to the running initiative
(humans in Layer 1, `/spec:execute` in Layer 2), not to the authoring
step.

See [RFC-2 §"Phase Boundary → Rule 2"](../../../../rfcs/archive/rfc-2-execution.md)
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
itself when it writes `discovery.md`, not by the skill scaffold.
Step 2 (`specify plan init`) does not create it.

On archive, `specify plan archive` (RFC-2 L3.B) sweeps this directory
alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`,
preserving the authoring trail with the plan it produced.

## Dry-run output

Under `--dry-run`, the skill emits a **readiness report** followed
by the **would-be-produced capability inventory** and the
**would-be-proposed plan** in one rendering, then exits without
writing anything. Dry-run folds the L3.E readiness gate, the L3.F
discovery preview, and the L3.G propose preview into a single
pass: inputs PLUS the inventory the briefs would emit PLUS the
plan decomposition the propose brief would offer against that
inventory.

The combined shape:

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

Dry-run — no specify plan create calls made.

Would write .specify/plans/<initiative-name>/proposal.md:

# Proposal — <initiative-name>

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | <slice> | <keys> | <names or —> | dry-run | — |
| 2 | ... | ... | ... | dry-run | — |

## Notes

- Dry-run: no specify plan create calls made; no entries written.

No files written. Remove --dry-run to run the pipeline.
```

Section rules:

- The `Sources:` block is omitted entirely when `--source` was not
  supplied. `--from` and `--against` inputs do not surface here — the
  `Sources:` block pins the top-level `sources` map shape that step 2
  would write to `plan.yaml`, not the full set of discovery inputs.
- The `Pipeline:` line names `pipeline.plan` and lists the brief IDs
  in the order `specify schema pipeline --phase plan` returns.
- Every line prefixed with `[dry-run]` on the banner is enough — the
  body lines do not need a per-line prefix.
- The `Would write ...:` preamble is emitted before the inventory
  body to make it obvious the content is a preview rather than
  written output.
- The readiness-report portion (banner through `Pipeline:` line) is
  pinned by [`fixtures/dry-run/expected-output.md`](fixtures/dry-run/expected-output.md).
  The inventory portion is pinned by
  [`fixtures/discovery/expected-discovery.md`](fixtures/discovery/expected-discovery.md)
  against the [`fixtures/discovery/legacy/`](fixtures/discovery/legacy/)
  source tree; under `--dry-run` the same content is emitted to
  stdout instead of written to disk.
- The proposal preview portion mirrors the shape of
  [`fixtures/propose/expected-proposal.md`](fixtures/propose/expected-proposal.md)
  but with every slice's `Decision` column set to `dry-run` and
  the `Plan entry` column set to `—` (no `specify plan create`
  call is made, so no final entry name exists). The Notes section
  is replaced with the stock `Dry-run: no specify plan create
  calls made; no entries written.` line.

Under `--dry-run` the skill MUST NOT:

- create `.specify/plans/<initiative-name>/`;
- shell out to `specify plan init`, `specify plan create`, `specify
  plan amend`, or `specify plan transition`;
- write any file under `.specify/`.

The discovery brief's input-reading side (reading `--from` files,
invoking `/spec:extract` to parse `--source` / `--against` inputs)
runs under `--dry-run` so the preview inventory is real; only the
write to `discovery.md` and the `.specify/plans/<name>/` directory
creation are suppressed. The propose brief's slice-decomposition
pass also runs (the preview plan shape is real against the
previewed inventory); the accept / edit / reject loop and every
`specify plan create` call are skipped.

## Constraints

- **`.specify/plan.yaml` already exists without `--extend`.** Refuse
  with a diagnostic pointing at `specify plan archive`. There is no
  `--force`; a human wanting to start over runs archive first. This
  matches the `specify plan init` CLI contract (RFC-2 §"CLI support").
- **`--extend` with no existing plan.** Refuse with a diagnostic
  pointing at re-running without `--extend`. The skill never
  silently creates a fresh plan under `--extend` — the flag is an
  explicit "I know there's a plan here" signal.
- **`--extend` with an existing `discovery.md`.** Skip step 3(a)
  and reuse the existing inventory (see §"Step 3(a) — Discovery
  → `--extend` and existing `discovery.md`"). This is a skip, not
  a refusal — discovery is explicitly a one-shot artefact; an
  operator who wants to refresh it archives the plan first.
- **`--dry-run` writes nothing.** No `specify plan init`, no
  `specify plan create`, no `discovery.md`, no `proposal.md`. The
  dry-run contract is read-only end to end; an editor watching the
  filesystem for writes during a dry-run should see nothing under
  `.specify/`.
- **Kebab-case `<initiative-name>`.** Validated before any other
  work (including before reading `.specify/plan.yaml`, before
  resolving `--source` paths, and before any CLI shell-out). A bad
  name exits non-zero with a clear diagnostic and no side effects.
- **No driver lock.** `/spec:plan` holds no locks that `/spec:execute`
  observes. The `.specify/plan.lock` PID stamp is the execution
  side's concern — authoring and execution are strictly ordered
  (authoring produces the plan, execution consumes it), so there is
  no shared-state race to guard against with a lock. A human running
  `specify plan transition` or `specify plan amend` by hand while
  `/spec:plan` is authoring is safe because every write goes through
  the atomic library functions.

## What this skill does NOT do

| Surface | Status |
|---|---|
| Execute the plan | Never. Execution is `/spec:execute`'s concern (Layer 2). `/spec:plan` exits with a hand-off summary that points the operator at `/spec:execute --loop`. |
| Modify existing plan entries | Never. `--extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify plan amend` by the human, not by this skill. |
| Skip `specify plan validate` | Never. Step 4 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero. This is the contract the skill owes its caller. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, or `/spec:execute` | Never. `/spec:plan` only invokes the briefs declared in `schema.yaml`'s `pipeline.plan`, plus the `specify plan` CLI for scaffolding, entry creation, and validation. |
| Hold a driver lock | Never. `.specify/plan.lock` is reserved for `/spec:execute`; authoring runs outside that lock. |
| Write `.specify/plan.yaml` directly | Never. Every write goes through `specify plan init` (step 2, skipped under `--extend`) or `specify plan create` (step 3b, one call per accepted slice). |
| Clone git URLs | Never. `--source` values that are git URLs are passed through to `/spec:extract` verbatim; cloning (if any) happens inside `/spec:extract` via `git-cloner`. |
| Author propose brief bodies | Never. The propose brief body is owned by the schema (for Omnia, [`schemas/omnia/briefs/plan/propose.md`](../../../../schemas/omnia/briefs/plan/propose.md) from L3.D); the skill only drives the accept / edit / reject loop against whatever the brief emits. |
| Auto-repair a failing `specify plan validate` | Never. Step 4's validation gate is read-only; any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit. |

The state the skill mutates:

1. `.specify/plan.yaml` via `specify plan init` (step 2; skipped
   under `--extend`) and `specify plan create` (step 3b; once per
   accepted slice).
2. `.specify/plans/<initiative-name>/discovery.md` written by the
   discovery brief (step 3a — see §"Step 3(a) — Discovery").
3. `.specify/plans/<initiative-name>/proposal.md` written by the
   propose brief (step 3b — see §"Step 3(b) — Propose").

No other on-disk state is written by `/spec:plan` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`. Route every write through
  `specify plan init` (step 2) or `specify plan create` (step 3b).
  The single-writer invariant in RFC-2 §"Plan Mutation and Crash
  Safety" depends on it.
- Never skip `specify plan validate` (step 4). A plan that ships to
  `/spec:execute` without a clean validate is a regression; the
  validator is the contract the skill owes the downstream driver.
- Validate `<initiative-name>` before any filesystem read or CLI
  shell-out. A bad name should never leave a half-written plan
  behind.
- For `--dry-run` specifically: the skill MUST NOT shell out to
  `specify plan init`, `specify plan create`, `specify plan amend`,
  or `specify plan transition`; MUST NOT create
  `.specify/plans/<name>/`; MUST NOT write `discovery.md` or any
  other file under `.specify/`. The discovery brief's input-reading
  side (reading `--from` files, invoking `/spec:extract` against
  `--source` / `--against` inputs) still runs so the stdout
  inventory preview is real; only the write-out and directory
  creation are suppressed. The first-line banner prefixes the
  rendered output with `[dry-run] ` (the body lines do not need a
  per-line prefix — the banner is enough).
- For `--extend` specifically: step 2 is skipped in full; step 3(b)
  only appends entries via `specify plan create` — it never calls
  `specify plan amend` or `specify plan transition` on existing
  entries. Draft slices whose names collide with existing plan
  entries are skipped with decision `skip-existing` in
  `proposal.md`; the human is not re-prompted for those. A
  propose-time decision to modify an existing entry is surfaced
  to the human, who runs `specify plan amend` by hand outside the
  authoring loop.
- Treat an unexpected `specify schema pipeline --phase plan`
  response shape (missing keys, unknown brief IDs, empty pipeline)
  as a hard failure: print the raw JSON and exit non-zero. Do not
  speculate about brief ordering.
