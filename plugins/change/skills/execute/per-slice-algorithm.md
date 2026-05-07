# Per-slice algorithm

The per-slice algorithm runs a single plan entry to a terminal status (`done` / `failed` / `blocked`) and is the core of every mode. The [§Modes](modes.md) section wires this into the three invocations (`--dry-run`, supervised, `--loop`) by describing only the deltas from this algorithm.

The algorithm is normative. Every shell-out is to the Layer 1 `specify` CLI; this skill writes nothing to `plan.yaml`, `.metadata.yaml`, or `journal.yaml` directly.

```text
1. Resolve the project directory (walk upward from CWD looking for
   .specify/project.yaml). Exit non-zero with a clear diagnostic if
   no Specify project is found.

2. Acquire the driver lock:
     specify change plan lock acquire --pid <agent-session-pid>
   On Error::DriverBusy, report which PID holds the lock and exit 1
   without touching the plan.

3. Self-heal:
   Run the self-heal algorithm described in self-heal.md.
   Self-heal either (a) leaves plan.yaml untouched (no in-progress
   entries to reclaim), (b) resolves each in-progress entry to its
   terminal plan status by reading the phase outcome on disk, (c)
   signals "resume mid-slice" for an entry whose phase was in-flight
   at crash time, or (d) halts for human triage on ambiguity. Cases
   (a) and (b) fall through to step 4. Case (c) skips step 4 and
   picks up at the appropriate phase (step 6, 7, or 8) for the named
   entry. Case (d) releases the lock and exits non-zero without
   reaching step 4.

4. Pick the next slice:
     specify change plan next --format json
   Interpret the JSON:
     - `next != null`                → continue to step 5 with this
                                        name. Capture the entry's
                                        `project`, `description`, and
                                        `sources` for use in steps 5a
                                        and 6 (see argument-resolution.md).
     - `reason == "in-progress"`     → defence in depth: an active
                                        entry exists that self-heal
                                        did not resolve. In practice
                                        the halt in step 3 exits
                                        before reaching step 4. Emit
                                        a diagnostic naming the
                                        entry, release the lock, exit
                                        non-zero.
     - `reason == "all-done"`        → emit the terminal "change
                                        complete" summary, release
                                        the lock, exit 0.
     - `reason == "stuck"`           → emit the terminal "stuck"
                                        summary (pending / blocked /
                                        failed buckets), release the
                                        lock, exit 0. `--loop` treats
                                        this the same way.

5. Multi-repo workspace preflight and branch preparation.
   See multi-repo.md for the full routing algorithm. In short: if the
   `project` field from step 4 is non-null, resolve it through
   `registry.yaml`, materialise only that selected project when its
   slot is missing (`specify workspace sync <project>`), resolve source
   paths relative to the coordinator root, and run:

     specify workspace prepare-branch <project> \
         --change <change-name> \
         [--source <absolute-source-path> ...] \
         [--output <capability-owned-output-path> ...] \
         --format json

   This step happens before any phase writes and before the plan entry
   transitions to `in-progress`. Branch-preparation diagnostics such
   as `origin-head-unresolved`, `dirty-unrelated-tracked`, or
   `dirty-branch-mismatch` halt here: release the lock and exit
   non-zero without running /spec:define. If `project` is null, skip
   this step entirely (single-repo path).

5b. Transition the selected entry:
     specify change plan transition <name> in-progress
   This is the first plan write the driver performs. It must happen
   BEFORE /spec:define creates the slice directory: between this step and step 6 the
   plan briefly shows an in-progress entry with no matching slice
   directory, which `specify change plan validate` tolerates as a
   warning.

5c. CWD routing (multi-repo only).
   If step 5 prepared a project slot, `chdir` into the returned
   `slot_path` and emit the `Routing:` diagnostic. The coordinator
   root remains the owner of the plan lock and status transitions;
   this CWD change brackets phase execution only.

6. Resolve the plan entry's `sources` into define arguments (see
   argument-resolution.md) and invoke:

     /spec:define <name> \
         [--source <key>=<path-or-url> [--source ...]]

   The `description` field on the plan entry carries scope and
   delta-targeting intent; define reads it off the plan directly and
   the driver does not re-plumb it through the command line.

   When /spec:define returns, read the phase outcome per step 9.

7. If define returned success: invoke /spec:build <name>. On return,
   read the phase outcome per step 9.

8. If build returned success: invoke /spec:merge <name>. On return,
   read the phase outcome per step 9. For routed workspace entries,
   `specify slice merge run` commits only `.specify/specs/` and
   `.specify/archive/` as `specify: merge <name>`; non-baseline
   project residue is handled by step 9a.

9. Read the phase outcome.
   Run:
     specify slice outcome show <name> --format json
   and take `.outcome.outcome` (the field lives at
   .specify/slices/<name>/.metadata.yaml:outcome.outcome). Classify
   per the table below.

9a. Post-merge residue guard and commit (multi-repo merge success only).
   If the routed entry just returned `outcome: success` from the merge
   phase, run the post-merge residue algorithm in multi-repo.md before
   any terminal plan transition:
     - dirty `.specify/specs/` or `.specify/archive/` after merge
       success → halt with `baseline-residue-after-merge`;
     - clean non-baseline worktree → skip the residue commit;
     - dirty non-baseline residue → commit it as
       `specify: residue <name>`;
     - residue commit failure → halt with `residue-commit-failed`.
   A halt here leaves the entry `in-progress`, releases the lock, and
   exits non-zero. The driver never marks an entry `done` while
   baseline residue or uncommitted non-baseline residue remains.

9b. CWD restore (multi-repo only).
   If the CWD routing step (5c) changed the working directory,
   restore CWD to the saved initiating repo root. This ensures
   `specify change plan transition` (which reads `plan.yaml` in the
   initiating repo) runs from the correct directory. In `--loop`
   mode, the CWD routing and CWD restore steps bracket every
   iteration so that `specify change plan next` always runs from
   the initiating repo root.

10. Success wrap-up.
      specify change plan transition <name> done
    Emit the success transcript (see output-format.md). Run the
    post-merge cross-project contract check (multi-repo.md). Go to
    step 13.

11. Failure drop path.
    a. Capture `outcome.summary` (and, if present, `outcome.context`)
       from the phase that failed.
    b. Run:
         /spec:drop <name> --reason "<outcome.summary>"
       This is the existing drop skill — it archives partial
       artifacts and flips the slice lifecycle to `dropped`. It does
       NOT touch plan.yaml.
    c. Run:
         specify change plan transition <name> failed --reason "<outcome.summary>"
       The `--reason` value is copied VERBATIM from the phase's
       `outcome.summary`. Do not paraphrase, truncate, or re-render.
    d. Emit the failure transcript. Go to step 13.

12. Deferred path (same shape as failure, with `blocked` instead of
    `failed`).
    a. Capture `outcome.summary` (and optional `outcome.context`).
    b. Run:
         /spec:drop <name> --reason "<outcome.summary>"
    c. Run:
         specify change plan transition <name> blocked --reason "<outcome.summary>"
       `--reason` is copied verbatim, as in step 11c.
    d. Emit the deferred transcript. Go to step 13.

13. Release the driver lock:
      specify change plan lock release --pid <agent-session-pid>
    Run this on EVERY exit path — success, failure, deferral, stop-
    for-triage (step 4 in-progress branch), or any uncaught error
    after step 2. The release step is unconditional; think of it as
    the trailing edge of a `try` / `finally` wrapping steps 3–12.
    Exit with code 0 for success/failure/deferred outcomes (the
    slice reached a terminal plan status as designed), non-zero for
    step-4 in-progress stops, branch-preparation halts, post-merge
    residue halts, and step-9 synthetic-deferred cases where human
    triage is required.
```

## Phase outcome classifications (RFC-9 §2B)

Step 9's classifier reads `.outcome.outcome` and routes to the per-slice algorithm's terminal branches:

| `.outcome.outcome` | Step | Plan transition | Notes |
|---|---|---|---|
| `success` | continue / step 10 | none mid-sequence; `done` after merge | Happy path. |
| `failure` | step 11 | `failed` (drop + transition) | `--reason` byte-identical to `outcome.summary`. |
| `deferred` | step 12 | `blocked` (drop + transition) | `--reason` byte-identical to `outcome.summary`. |
| `registry-amendment-required` | step 12a | `blocked` (drop + transition) | RFC-9 §2B. Driver records the structured proposal payload to the change's journal **before** the drop (`/spec:drop` does not touch `journal.yaml`, but the journal sweeps with the change into `.specify/archive/...`). See §"Registry amendment required (RFC-9 §2B)" below. |
| missing / malformed / contradicts lifecycle | step 12 | `blocked` with synthetic summary | Driver never speculates. |

## Registry amendment required (RFC-9 §2B)

When a phase emits `outcome: registry-amendment-required` (via `specify slice outcome set <name> <phase> registry-amendment-required ...`), the driver follows the **deferred** terminal branch with one extra writing step: it appends the proposal payload to the change's journal **before** invoking `/spec:drop`. The classification (`blocked`) is the existing `deferred` classification — no new plan status is introduced.

### Step 12a — Record the proposal in the journal

After step 9 classifies the outcome as `registry-amendment-required` and **before** step 12.b (`/spec:drop`), shell out exactly once:

```bash
specify slice journal append <name> <outcome.phase> failure \
    --summary "registry-amendment-required: <outcome.proposal.proposed-name>" \
    --context "$(cat <<'YAML'
proposed-name: <outcome.proposal.proposed-name>
proposed-url: <outcome.proposal.proposed-url>
proposed-schema: <outcome.proposal.proposed-schema>
proposed-description: <outcome.proposal.proposed-description or "—">
rationale: |
  <outcome.proposal.rationale verbatim>
YAML
)"
```

Notes:

- The journal entry uses the existing `failure` kind (the canonical `EntryKind::{Question, Failure, Recovery}` set per `specify-cli/crates/change/src/journal.rs`); no new kind is introduced. Readers grep `summary` for the `registry-amendment-required:` prefix to filter.
- The full structured payload lives in `--context` as YAML, mirroring the contract used by the cross-project contract check (RFC-9 §3B). The `--summary` carries the proposed project name so an operator scanning the journal sees what was proposed at a glance.
- Read `outcome.proposal.*` from `specify slice outcome show <name> --format json` — the CLI emits the proposal as a sibling object (`outcome.proposal`) so existing consumers that only read `.outcome.outcome` (a kebab-case string) keep working.
- After the journal append, fall through to step 12.b (`/spec:drop`) and step 12.c (`specify change plan transition <name> blocked --reason "<outcome.summary>"`) with the **same `--reason` rule** the `deferred` branch follows: `outcome.summary` is copied byte-for-byte. The default summary stamped by the CLI is `registry-amendment-required: <proposed-name>`, but a phase that supplied a richer `--summary` keeps that exact text.

### Surface the proposal in the per-slice transcript

The `Deferred` transcript (see [output-format.md](output-format.md) → Supervised / per-slice transcript → Deferred) renders an extra `Proposed registry amendment` block immediately after the `Status: blocked` line **only** when the deferral was a `registry-amendment-required`:

```text
Proposed registry amendment (RFC-9 §2B)
  Name:        <proposed-name>
  URL:         <proposed-url>
  Schema:      <proposed-schema>
  Description: <proposed-description or "—">
  Rationale:   <rationale verbatim>

  Action needed: review the proposal, then run the canonical recovery
    sequence below to land the new project and re-queue the change.
```

The block is omitted entirely when the deferral was a plain `deferred` outcome.

### Canonical recovery sequence (operator-driven)

The driver does **not** apply registry amendments automatically — `specify registry add` is reserved for operator-initiated topology changes. Once the operator has reviewed the proposal, they run this exact sequence (or its supervised equivalent inside the §2C `/change:plan --orchestrate` umbrella):

```text
specify registry add <proposed-name> \
    --url <proposed-url> \
    --schema <proposed-schema> \
    --description "<proposed-description>"

specify workspace sync

specify change plan amend <slice-name> --project <proposed-name>

specify change plan transition <slice-name> pending
```

Notes:

- **Verb order matters.** The registry must be amended **before** the plan can amend `--project` (the validator rejects `project` values not in `registry.yaml`). The workspace sync between them materialises the new clone slot under `.specify/workspace/<proposed-name>/` so subsequent `/change:execute --loop` runs route into a real working tree.
- **`pending` re-queues.** The slice was dropped at step 12.b; the next `/change:execute --loop` pass picks it up via `specify change plan next`. The drop archived the prior journal under `.specify/archive/...-<slice-name>/` — the recovery `pending → in-progress` re-creates a fresh slice directory at step 6.
- **Manual fallback.** Every step is a v1 verb the operator can run by hand; the umbrella skill (RFC-9 §2C, `/change:plan --orchestrate`) wraps the same sequence into a single composition. `/change:execute` itself never invokes any of these verbs — that boundary is what keeps the registry under operator control.

### Self-heal interaction

Self-heal ([self-heal.md](self-heal.md)) treats `outcome.outcome == registry-amendment-required` exactly like `deferred`: it emits the journal append (the same way step 12.a above does), runs `/spec:drop`, and applies `specify change plan transition <name> blocked --reason "<outcome.summary>"`. The diagnostic line uses the canonical `registry-amendment-required` qualifier so the operator can tell from a single log line which deferred branch the self-heal pass took:

```text
Self-heal: <name> → blocked (define registry-amendment-required: "<outcome.summary verbatim>")
```

The dry-run variant uses the same line with `(if executed)` appended (mirroring the existing failed / blocked dry-run lines).

## Subtleties

- **`/change:execute` writes only plan transitions, workspace Git state, and a narrow set of journal entries.** Every write this skill performs against `plan.yaml` goes through `specify change plan transition`. It never writes `outcome` to `.metadata.yaml` (the phase does that, via `specify slice outcome set`). For routed entries it may prepare `specify/<change-name>` before phase writes and commit non-baseline residue after merge success. The driver appends to `journal.yaml` in exactly three situations: (1) the self-heal step emits one `type: recovery` entry per reclaimed or resumed in-progress entry; (2) branch preparation fails during a resume and a slice journal already exists; (3) the post-merge cross-project contract check emits one `type: failure` entry per finding the validator reports, with the canonical `cross-project-warning:` summary prefix. The define / build / merge phases own all other `type: question` and `type: failure` entries; the driver never touches those.

- **Summary is copied verbatim into `status-reason`.** The string passed to `specify change plan transition … --reason "…"` in steps 11c and 12c is byte-identical to `outcome.summary` stamped by the phase. The fixtures under `fixtures/single-slice/` pin this: every `plan.yaml.after` carries `status-reason: "<exact summary from the metadata file>"`. Do not paraphrase, truncate, or reformat.

- **Journal entries from the phase are preserved.** Whatever `type: question` / `type: failure` entries the phase wrote during its run stay on disk unchanged. The driver does not rewrite, merge, or summarise them. Humans reading the journal after a failure or deferral see the full trail the phase recorded, not a driver-authored post-hoc rollup.

- **Release the lock on every exit path.** Every branch of the algorithm — success, failure, deferred, stop-for-triage, unhandled error — MUST run `specify change plan lock release` before returning control to the caller. Treat the release as the invariant trailing edge of the run.

- **Single `in-progress` at a time.** The driver never has more than one plan entry in `in-progress` at any point in time. Step 5 is the only place the driver enters that state; steps 10/11/12 are the only places the driver leaves it. Self-heal is the only other step that mutates plan status, and only to resolve a pre-existing `in-progress` left by a prior crashed run.
