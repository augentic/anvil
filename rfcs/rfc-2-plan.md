# Implementation plan: RFC-2 (Execution)

> Status: Draft — no changes landed yet.
>
> Companion to [rfc-2-execution.md](rfc-2-execution.md). Builds on the CLI and change-lifecycle foundations delivered by [rfc-1-cli.md](archive/rfc-1-cli.md) (see also [rfc-1-plan.md](archive/rfc-1-plan.md)). Architectural decisions made during the build go into [DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) in `augentic/specify-cli`.

This plan decomposes RFC-2 into a sequence of subagent-sized Changes. Each Change is scoped to be completed by one sub-agent in a single session, ends in a green `cargo test --workspace` (and, where applicable, a green fixture suite in `augentic/specify`), and has explicit dependencies so an orchestrator can execute them in order and know when it is safe to advance.

The plan mirrors the three-layer structure of RFC-2:

- **Layer 1** (Changes L1.A–L1.L) — Plan format, library, and CLI (MVP). Humans can drive plan-based initiatives end to end after this layer exits.
- **Layer 2** (Changes L2.A–L2.I) — `/spec:execute` driver skill, `last_phase_outcome` contract, `journal.yaml`, driver lock. `/spec:execute --loop` runs unattended after this layer exits.
- **Layer 3** (Changes L3.A–L3.I) — `/spec:plan` authoring skill and the `pipeline.plan` brief set. Plans are produced by skill rather than by hand after this layer exits.

Each layer is independently useful: Layer 1 stops at hand-driven plans, Layer 2 adds automation against hand-authored plans, Layer 3 closes the authoring gap. The orchestrator can halt after any layer and the system is coherent.

Out of scope for this plan:

- RFC-3 (multi-repo federation) beyond git-remote cloning already covered by `/spec:extract`.
- RFC-4 (DSL), RFC-5 (framework lint).
- "Future Capabilities" called out in RFC-2 §Future Capabilities (`specify plan doctor`, prior-attempt context replay, pre-plan baseline delta targeting, multiple concurrent plans, CLI namespace rename, change recommender, behavioural diff, cross-stack define).

---

## Repos in play

- **`augentic/specify-cli`** — library code (`crates/change`, `crates/schema`, `crates/error`), `specify` binary (`src/main.rs`), integration tests (`tests/`), `schemas/plan/` JSON Schema mirror.
- **`augentic/specify`** — RFC text, per-schema briefs (`schemas/omnia/`, `schemas/vectis/`), plugin skills (`plugins/spec/skills/*/SKILL.md`), top-level `schemas/plan/plan.schema.json`, authoring fixtures.

Changes that touch both repos name both in **Scope**. The orchestrator commits each repo on its own branch per Change and merges in lockstep once acceptance is green.

---

## Orchestration conventions

- **Branch per Change.** Naming: `rfc2/<change-id>-<slug>` (e.g. `rfc2/l1-a-plan-types`). Rebased on `main` at the start of the Change.
- **Definition of done per Change.**
  1. `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test --workspace` green in `specify-cli`.
  2. Every new CLI subcommand has at least one `assert_cmd` integration test in `tests/plan.rs` covering both `--format text` and `--format json`.
  3. Every RFC-2 table row or state-machine edge implemented by the Change has a matching test case.
  4. `DECISIONS.md` in `specify-cli` gains an entry when the Change locks a design call not already pinned by the RFC (lockfile layout, phase-outcome file location, etc.).
- **Context handed to each subagent.** This file, the specific RFC-2 sections named under **Scope**, and `git log --oneline` of previously-merged Changes. No other context is required.
- **Halt conditions.** Orchestrator stops and raises to a human when (a) acceptance tests are red after one repair pass, (b) the Change surfaces a design question not answered by RFC-2, or (c) the Change exceeds ~400 non-test LOC.
- **Single-writer invariant.** No Change may add a new writer of `plan.yaml` outside `Plan::{init, create, amend, transition, archive}`. A grep guard (`rg 'plan\.yaml' crates/ src/ plugins/`) is reviewed as part of each Change's acceptance.

---

## Dependency graph

```text
Layer 1 — library
  L1.A (plan.rs types)
  ├── L1.B (state machine)
  ├── L1.C (load/save atomic)
  │   ├── L1.D (validate)
  │   │   ├── L1.E (next_eligible + topo)
  │   │   └── L1.F (create/amend/transition)
  │   │       └── L1.G (archive)
  │   └── L1.H (plan.schema.json)

Layer 1 — CLI
  L1.{D,E,F,G}
  └── L1.I (plan {validate,next,status})
      └── L1.J (plan {create,amend,transition})
          └── L1.K (plan archive)
              └── L1.L (Layer 1 docs sweep)          ← Layer 1 exit gate

Layer 2
  L2.A (last_phase_outcome + CLI)
  L2.B (journal.yaml helpers)
  L2.C (plan.lock advisory lock)
  L2.D (phase-skill SKILL.md updates)        depends on L2.A, L2.B
  L2.E (/spec:execute scaffold + --dry-run)  depends on L1 exit, L2.A–C
  L2.F (single-change happy/fail/deferred)   depends on L2.D, L2.E
  L2.G (self-heal on startup)                depends on L2.F
  L2.H (--loop + terminal summary + lock)    depends on L2.G, L2.C
  L2.I (sources/affects execution wiring)    depends on L2.H          ← Layer 2 exit gate

Layer 3
  L3.A (specify plan init)                   depends on L1 exit
  L3.B (archive co-move)                     depends on L1.G, L3.A
  L3.C (Phase::Plan in specify-schema)       depends on L3.A
  L3.D (Omnia pipeline.plan briefs)          depends on L3.C
  L3.E (/spec:plan scaffold)                 depends on L3.C
  L3.F (discovery brief integration)         depends on L3.D, L3.E
  L3.G (propose brief integration)           depends on L3.F
  L3.H (Vectis pipeline.plan briefs)         depends on L3.C
  L3.I (RFC-2 closeout)                      depends on L3.G, L3.H    ← Layer 3 exit gate
```

Every arrow is a strict "must-complete-before" edge. Layer 2 Changes L2.A, L2.B, L2.C are independent of one another and can be taken in any order; L2.E folds them together. Layer 3 Changes L3.D and L3.H are schema-independent and can be taken in either order once L3.C lands.

---

# Layer 1 — Plan format, library, and CLI (MVP)

## Change L1.A — `plan.rs` types + serde round-trip

**Scope**

- In `augentic/specify-cli`, add `crates/change/src/plan.rs`. Define `Plan`, `PlanChange`, `PlanStatus`, `PlanChangePatch`, `ValidationLevel`, `ValidationResult` exactly as written in RFC-2 §"Library Implementation" (including `serde(rename_all = "kebab-case")` on every struct and enum).
- Extend `crates/error/src/lib.rs` with `Error::PlanTransition { from: PlanStatus, to: PlanStatus }` and `Error::PlanHasOutstandingWork { entries: Vec<String> }`. Update the `Display` impls.
- Re-export `plan::*` from `crates/change/src/lib.rs`.
- No runtime behaviour yet — structs, derives, and `Display` only.

**Deliverables**

- `crates/change/src/plan.rs` with the RFC-2 library surface as type declarations.
- Unit tests in `plan.rs` that round-trip the RFC-2 §"The Plan" example (`platform-v2`) through `serde_yaml` and assert round-trip equivalence.
- `specify-error` carries the two new variants and compiles.

**Acceptance**

- `cargo build -p specify-change` green.
- Round-trip test green.
- Later Changes can add `impl Plan { … }` blocks without touching the type declarations.

**Dependencies**

- None (builds on existing `specify-change` foundation).

---

## Change L1.B — `PlanStatus` state machine

**Scope**

- Implement `PlanStatus::can_transition_to(&self, target: &PlanStatus) -> bool` and `PlanStatus::transition(&self, target: PlanStatus) -> Result<PlanStatus, Error>` exactly per the matrix in RFC-2 §"Transition Rules".
- Disallow `done → *` (terminal) and every edge not explicitly listed.

**Deliverables**

- Public `PlanStatus` methods.
- Table-driven tests covering: every legal edge (10 total) and a representative set of illegal edges (at least `done → pending`, `done → in-progress`, `pending → done`, `skipped → failed`, `in-progress → pending`).

**Acceptance**

- Failing transitions surface as `Error::PlanTransition { from, to }` with both values preserved.

**Dependencies**

- L1.A.

---

## Change L1.C — `Plan::load` / `Plan::save` with atomic write

**Scope**

- `Plan::load(path: &Path) -> Result<Plan, Error>` parses YAML; tolerates missing trailing newline.
- `Plan::save(&self, path: &Path) -> Result<(), Error>` writes YAML via temp file + `fs::rename` in the same directory; always emits a trailing newline.
- Document atomicity semantics in doc-comments (partial file is never observed by readers).

**Deliverables**

- Implementations of `load` / `save`.
- Tests: round-trip equivalence (`load ∘ save` is identity on the RFC-2 example plan); pre-existing target file is replaced atomically; a simulated mid-write abort (writer returns an error before rename) leaves the original file untouched.

**Acceptance**

- No writer in the crate touches `plan.yaml` by any path other than `Plan::save`.

**Dependencies**

- L1.A.

---

## Change L1.D — `Plan::validate`

**Scope**

- Add `petgraph` to `crates/change/Cargo.toml` (pinned to the current workspace-approved version; add to `supply-chain` if required).
- Implement `Plan::validate(&self, changes_dir: Option<&Path>) -> Vec<ValidationResult>` covering every check called out in RFC-2 §"`specify plan validate`":
  - Duplicate names.
  - Cycle detection via `petgraph` topological sort.
  - Referential integrity: every `depends-on` and `affects` target resolves to an entry; every `sources` key resolves to a top-level `sources` entry.
  - Status values are well-formed (enforced by serde but also re-checked here for completeness on manually-constructed `Plan`s).
  - At most one entry with `status: in-progress`.
- When `changes_dir = Some(_)`, add the plan-to-change consistency checks: orphan directories become `Warning`, missing directories for `in-progress` entries become `Warning`.
- Stable `code` strings on `ValidationResult` (`duplicate-name`, `dependency-cycle`, `unknown-depends-on`, `unknown-affects`, `unknown-source`, `multiple-in-progress`, `orphan-change-dir`, `missing-change-dir-for-in-progress`).

**Deliverables**

- `validate` implementation.
- Tests: one fixture per diagnostic code; one "clean plan" baseline; one composite fixture that trips three diagnostics at once and proves results accumulate (no short-circuit).

**Acceptance**

- Every validation diagnostic in RFC-2 §"`specify plan validate`" is observable by at least one test.

**Dependencies**

- L1.A, L1.C.

---

## Change L1.E — `Plan::next_eligible` + `Plan::topological_order`

**Scope**

- `next_eligible(&self) -> Option<&PlanChange>` returns the first `pending` entry in list order whose `depends-on` is all `done`; returns `None` when any entry has `status: in-progress`.
- `topological_order(&self) -> Result<Vec<&PlanChange>, Error>` returns changes sorted by the `depends-on` DAG; ties broken by list order for determinism; returns `Err` on cycles.

**Deliverables**

- Both methods.
- Tests walking the RFC-2 example plan forward through progressive `done` transitions; test that any `in-progress` blocks `next_eligible`; test that a cycle yields `Err` from `topological_order` (and that `next_eligible` still works — it is not sort-dependent).

**Acceptance**

- Tie-break is deterministic and depends only on plan list order.

**Dependencies**

- L1.A, L1.B, L1.D.

---

## Change L1.F — `Plan::create` / `Plan::amend` / `Plan::transition`

**Scope**

- `Plan::create(&mut self, change: PlanChange) -> Result<(), Error>` appends with `status: pending` enforced; rejects duplicate names; runs `validate(None)` before returning success.
- `Plan::amend(&mut self, name: &str, patch: PlanChangePatch) -> Result<(), Error>` applies a `PlanChangePatch` to the named entry. Each field on the patch is `Option<T>`; `None` means "leave unchanged"; for `description: Option<Option<String>>`, `Some(None)` means "clear", `Some(Some(s))` means "replace". `status` is deliberately absent from the patch and cannot be mutated via this path. Runs `validate(None)` before returning.
- `Plan::transition(&mut self, name: &str, target: PlanStatus, reason: Option<&str>) -> Result<(), Error>` threads through `PlanStatus::transition`. Writes `status_reason` only when target ∈ {`Failed`, `Blocked`, `Skipped`}. Clears `status_reason` to `None` when target ∈ {`Pending`, `InProgress`, `Done`}. Rejects `--reason` combined with a target that does not permit it.

**Deliverables**

- All three mutators.
- Tests: full matrix of amend field semantics; reason clearing on re-entry to `pending`; attempting `amend` with a status-shaped field fails at compile time (the patch has no `status` field) — documented as a type-system guarantee in a doc comment.

**Acceptance**

- No call path writes `status` except through `Plan::transition`.

**Dependencies**

- L1.A, L1.B, L1.C, L1.D.

---

## Change L1.G — `Plan::archive`

**Scope**

- `Plan::archive(path: &Path, archive_dir: &Path, force: bool) -> Result<PathBuf, Error>` moves `plan.yaml` to `<archive_dir>/<plan-name>-<YYYYMMDD>.yaml`. Returns the archived path.
- Refuses (returns `Error::PlanHasOutstandingWork { entries }`) when any entry is in `Pending`, `InProgress`, `Blocked`, or `Failed`, unless `force = true`.
- Atomic move via `fs::rename` where possible, falling back to copy + delete across filesystems; archive directory is created if missing.

**Deliverables**

- Implementation + tests: happy path (`Done` + `Skipped` only); refusal without `force` reports all non-terminal entries; `force` path preserves non-terminal entries verbatim; archive directory creation; collision with an existing archive file for the same day yields a clear error.

**Acceptance**

- After a successful archive, `plan.yaml` is absent and the archived file contains exactly the contents of the pre-archive plan.

**Dependencies**

- L1.A, L1.C.

---

## Change L1.H — `plan.schema.json`

**Scope**

- In `augentic/specify`, author `schemas/plan/plan.schema.json` describing the full `plan.yaml` shape: top-level `name`, `sources` map, `changes` list; per-entry `name`, `status` enum, optional `depends-on`, `affects`, `sources`, `description`, `status-reason`.
- Enforce kebab-case for `name` / status values with a regex pattern.
- Mirror the file under `augentic/specify-cli/schemas/plan/` if CLI-embedded JSON Schema delivery follows the pattern used for `cache-meta.schema.json` / `schema.schema.json` in `specify-cli/schemas/`; otherwise ship only from `augentic/specify`.
- Add `# yaml-language-server: $schema=...` guidance to RFC-2 or to a short top-of-file comment in `schemas/plan/README.md`.

**Deliverables**

- The JSON Schema file(s).
- A validation test in `specify-cli/tests/plan.rs` (file introduced here) using the `jsonschema` crate: validates the RFC-2 §"The Plan" example; rejects two negative cases (unknown `status` value, non-kebab-case name).

**Acceptance**

- Editors picking up the schema get autocomplete + diagnostics on `.specify/plan.yaml`.

**Dependencies**

- L1.A.

---

## Change L1.I — CLI: `specify plan {validate, next, status}`

**Scope**

- In `augentic/specify-cli`, extend `Commands` in `src/main.rs` with `Plan { action: PlanAction }`. Add a `PlanAction` enum.
- Implement `validate`, `next`, and `status`:
  - `validate` runs `Plan::validate(Some(&changes_dir))` and renders each `ValidationResult` with its `level` / `code` / `entry` / `message`. Non-zero exit on any `Error` result.
  - `next` runs `Plan::next_eligible` and either names the selected entry, reports the active `in-progress` entry, or reports "all done" / "stuck on dependencies" per RFC-2 §"`specify plan next`".
  - `status` renders in topological order (falling back to list order on cycle detection with a banner pointing at `validate`), per-status counts (all six buckets shown, including zeros), current in-progress with its `LifecycleStatus`, blocked/failed entries with `status-reason`, next-eligible list, impact report for `affects` pointing at pending entries.
- Both `--format text` and `--format json` outputs. JSON shapes are pinned in fixtures; subsequent Changes cannot drift them.

**Deliverables**

- CLI subcommands wired into `run`.
- Integration tests in `tests/plan.rs` (one per command per format) using the existing `Project::init` harness from `tests/change.rs`.
- JSON fixture files under `tests/fixtures/plan/` for the three commands.

**Acceptance**

- `specify plan status --format json` output is stable across Changes L1.J–L1.L (re-run the test after each).

**Dependencies**

- L1.D, L1.E.

---

## Change L1.J — CLI: `specify plan {create, amend, transition}`

**Scope**

- `specify plan create <name> [--depends-on <name>...] [--affects <name>...] [--sources <key>...] [--description "..."]`.
- `specify plan amend <name> [--depends-on <name>...] [--affects <name>...] [--sources <key>...] [--description "..."]`. Empty flag (e.g. `--description ""`) clears the field; absent flag leaves it unchanged — document this in `--help`.
- `specify plan transition <name> <target> [--reason "..."]`. `target` is a `ValueEnum` over `PlanStatus`. `--reason` is accepted only when target ∈ {failed, blocked, skipped}.
- All three commands write via `Plan::{create, amend, transition}` → `Plan::save`; run `Plan::validate(None)` before persisting; refuse with a clear error when validation fails.
- Text and JSON outputs.

**Deliverables**

- CLI subcommands.
- Integration tests that reproduce the worked `registration-duplicate-email-crash` sequence from RFC-2 §"The Loop (Human-Driven)": starting from an empty `plan.yaml`, execute the exact CLI commands the RFC shows and assert the resulting `plan.yaml` matches a committed fixture byte-for-byte.
- Transition test covering every edge exercised via the CLI.

**Acceptance**

- The fixture-based "human replay" test is green; no other code path can mutate `plan.yaml`.

**Dependencies**

- L1.F, L1.I.

---

## Change L1.K — CLI: `specify plan archive`

**Scope**

- `specify plan archive [--force]`. Text + JSON output (JSON includes the archived path).
- Writes the archived file under `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`.

**Deliverables**

- CLI subcommand.
- Integration tests: happy path; refusal without `--force` on a plan with outstanding work (non-zero exit + clear message listing entries); `--force` path preserves entries verbatim; archived plan filename matches `<name>-<YYYYMMDD>.yaml`.

**Acceptance**

- `specify plan archive` is symmetric with `specify change archive` (both land under `.specify/archive/`).

**Dependencies**

- L1.G, L1.J.

---

## Change L1.L — Layer 1 documentation sweep

**Scope**

- In `augentic/specify`:
  - Update `README.md` with a short "Plans" section pointing at RFC-2 §"Layer 1" and listing the `specify plan` CLI verbs.
  - Update `AGENTS.md` to include `specify plan *` in the command reference.
  - Update `plugins/spec/skills/{define,build,merge,drop}/SKILL.md` where the human-driven loop is described, cross-linking `specify plan next`, `specify plan transition`, `specify plan create`, `specify plan amend`.
- Flip nothing in RFC-2 front-matter yet (status stays `Draft` until L3.I).

**Deliverables**

- Doc updates only. No code.

**Acceptance** (Layer 1 exit gate)

- Hand-review diff shows every RFC-2 §"Layer 1" primitive referenced by at least one of README / AGENTS.md / the relevant SKILL.md.
- A human can drive RFC-2's §"The Plan" example end-to-end using only `specify plan *` + existing phase skills, with no `/spec:execute` involvement.

**Dependencies**

- L1.K.

---

# Layer 2 — Automated execution

## Change L2.A — `last_phase_outcome` field + `specify change phase-outcome`

**Scope**

- In `augentic/specify-cli`, extend `ChangeMetadata` in `crates/change/src/lib.rs` with an optional `last_phase_outcome: Option<PhaseOutcome>` field. Define `PhaseOutcome { phase: Phase, outcome: Outcome, at: DateTime<Utc>, summary: String, context: Option<String> }` and `Outcome { Success, Failure, Deferred }`, both with kebab-case serde.
- Add `specify change phase-outcome <name> <phase> <outcome> [--summary ...] [--context ...]` subcommand under the existing `ChangeAction` tree. Writes the field atomically to `.metadata.yaml` per RFC-2 §"Phase Outcome Contract". Text + JSON output.
- The field is written by the CLI only; phases shell out.

**Deliverables**

- Extended `ChangeMetadata` + new subcommand.
- Integration tests stamp every (phase, outcome) combination, read the file back via `specify change status`, and assert shape.
- Round-trip tests ensure pre-existing `.metadata.yaml` files without the field still parse (`#[serde(default)]`).

**Acceptance**

- No phase skill or brief edits `.metadata.yaml` by any path other than the CLI.

**Dependencies**

- L1 exit gate. Independent of L2.B and L2.C.

---

## Change L2.B — `journal.yaml` append-only helpers

**Scope**

- Add `crates/change/src/journal.rs` with the on-disk representation of `<change_dir>/journal.yaml` per RFC-2 §"Question Recording" / §"Failure and Resumption":
  - `JournalEntry { timestamp, step, type: EntryType, summary, context }` where `EntryType ∈ { Question, Failure, Recovery }`.
  - `Journal::load(path)`, `Journal::append(path, entry)` with read-modify-write + atomic rename.
- Document the "pure audit log, never consumed as a signalling channel" contract in the module doc.

**Deliverables**

- `journal.rs` module with public API.
- Tests: append persists order; concurrent-append simulation via `tempfile` + threads preserves ordering (single-writer assumed; enforced by the driver lock in L2.C); malformed file is rejected on load with a diagnostic pointing at the line.

**Acceptance**

- `Journal::append` never truncates on crash: a mid-write abort leaves the prior file intact.

**Dependencies**

- L1 exit gate. Independent of L2.A and L2.C.

---

## Change L2.C — `.specify/plan.lock` advisory lock

**Scope**

- Add a small lock abstraction (inside `crates/change` as `plan::lock`, or a new `crates/lock` if it grows past ~100 LOC). Writes the current PID into `.specify/plan.lock` and holds an `flock(2)`-style exclusive lock on it for the lifetime of the guard.
- On acquire, if the lockfile exists and the recorded PID is not alive, reclaim it (overwrite) and log a diagnostic. If the PID is alive, return `Error::DriverBusy { pid }`.
- Document in the module: "advisory only; unreliable on NFS/SMB; Specify workspaces are local-FS" per RFC-2 §"Driver Concurrency".

**Deliverables**

- Lock module.
- Tests: sequential acquire/release; second acquire refused while first is held; stale-lock reclamation via an injected "is-pid-alive" seam; lock file removed on normal guard drop.

**Acceptance**

- `/spec:execute` (L2.E onward) wraps its whole run in this guard.

**Dependencies**

- L1 exit gate. Independent of L2.A and L2.B.

---

## Change L2.D — Phase-skill `SKILL.md` updates

**Scope**

- In `augentic/specify`, update `plugins/spec/skills/{define,build,merge}/SKILL.md` to:
  1. Name `success` / `failure` / `deferred` as the three outcomes a phase returns.
  2. Require the phase to call `specify change phase-outcome <name> <phase> <outcome>` as its last action before returning control.
  3. Require journal entries (`specify change journal append` if we choose to expose it in L2.B; else direct file append per the documented helper) on `type: question` and `type: failure` during the run, not just at the end.
  4. Describe when and how to shell out to `specify plan create` / `specify plan amend` mid-run (RFC-2 §"Phase Boundary → Rule 2"), including that `amend` may target the currently-active entry but `status` is off-limits.
- Add or update a small diagram reference pointing at the framework figure.

**Deliverables**

- Three updated `SKILL.md` files with consistent wording for the phase outcome contract.

**Acceptance**

- Hand-review diff shows all three skills in sync with each other and with RFC-2 §"Phase Boundary".

**Dependencies**

- L2.A, L2.B.

---

## Change L2.E — `/spec:execute` scaffold + `--dry-run`

**Scope**

- In `augentic/specify`, create `plugins/spec/skills/execute/SKILL.md` as the driver skill:
  - Invocation: `/spec:execute [--dry-run] [--loop]`.
  - Invariants table from RFC-2 §"Invariants".
  - Reference to the on-disk contracts (`last_phase_outcome`, `journal.yaml`, `plan.yaml`) and the driver lock.
  - Skill behaviour decomposed into the steps that later Changes (L2.F, L2.G, L2.H, L2.I) will flesh out.
- Implement `--dry-run`: read `.specify/plan.yaml`, call `specify plan next --format json`, emit the RFC-2 §"Output and Observability" progress block prefaced with a "dry-run" banner, do nothing else. Takes out the driver lock for consistency with full runs.

**Deliverables**

- The skill dir with `SKILL.md` and a `fixtures/dry-run/` directory.
- A dry-run fixture: input `plan.yaml` + expected rendered output (snapshot-tested via a small harness in `augentic/specify`).

**Acceptance**

- `--dry-run` makes no writes to `plan.yaml`, `.metadata.yaml`, or `journal.yaml`.

**Dependencies**

- L1 exit, L2.A, L2.B, L2.C.

---

## Change L2.F — `/spec:execute` single-change happy / failure / deferred paths

**Scope**

- Extend `/spec:execute` (and its SKILL.md) to execute a single change (no `--loop` yet):
  1. Acquire driver lock.
  2. Call `specify plan next` to pick the entry.
  3. `specify plan transition <name> in-progress`.
  4. Invoke `/spec:define`, `/spec:build`, `/spec:merge` in order, with initial arguments derived from the plan entry (name, source paths, description).
  5. After each phase, read `.metadata.yaml.last_phase_outcome`.
     - `success` → continue to next phase; after `/spec:merge` succeeds, `specify plan transition <name> done`.
     - `failure` → invoke `/spec:drop <name> --reason "<outcome.summary>"`; `specify plan transition <name> failed --reason "<outcome.summary>"`.
     - `deferred` → invoke `/spec:drop`; `specify plan transition <name> blocked --reason "<outcome.summary>"`.
  6. Emit structured per-phase output per RFC-2 §"Output and Observability".

**Deliverables**

- SKILL.md updates.
- Three behavioural fixtures (success, failure, deferred) with seed `plan.yaml`, seed `.metadata.yaml` trajectories, and snapshot of both the final plan YAML and the rendered transcript.

**Acceptance**

- Outcome `summary` is copied verbatim into `status-reason`; journal entries from the phase are preserved; `/spec:execute` itself writes neither `last_phase_outcome` nor `journal` entries (phases do).

**Dependencies**

- L2.D, L2.E.

---

## Change L2.G — `/spec:execute` self-heal on startup

**Scope**

- Before any `get next change`, scan `plan.yaml` for entries with `status: in-progress`. For each:
  - Locate the most recent `.metadata.yaml` (active change dir first, then the most recent archive under `.specify/archive/`).
  - Read `last_phase_outcome`:
    - `success` on `merge` → transition to `done`.
    - `failure` → transition to `failed`, copy `summary` into `status-reason`.
    - `deferred` → transition to `blocked`, copy `summary` into `status-reason`.
  - Append a `type: recovery` entry to `journal.yaml` describing the self-heal.
  - If an active change directory exists and `LifecycleStatus` is not yet terminal and there is no `last_phase_outcome`, resume mid-change per RFC-2 §"Context Threading → Resumption Within a Change" — no plan transition is needed.
  - If the on-disk state is ambiguous (missing / malformed outcome, contradicts `LifecycleStatus`), halt with a non-zero exit code and leave the plan entry as `in-progress`. No speculative transitions.

**Deliverables**

- Self-heal step added to `/spec:execute`'s preamble in SKILL.md.
- Four fixtures: (a) clean start no-op, (b) on-disk `success` resolves to `done`, (c) on-disk `failure` resolves to `failed`, (d) ambiguous state halts with diagnostic.

**Acceptance**

- Self-heal runs under the driver lock; no two drivers can race on it.

**Dependencies**

- L2.F.

---

## Change L2.H — `/spec:execute --loop` + terminal summary

**Scope**

- Add `--loop`: iterate `get next change → execute change → update status` until `specify plan next` reports no eligible change (or a deferral halts the loop).
- Emit the RFC-2 §"Output and Observability" terminal summary, including the `Completion:` classification: `all-done`, `stuck` (pending with unmet deps), `halted` (failure/deferral stopped the loop), or `driver-interrupted` (SIGINT/SIGTERM).
- Handle SIGINT / SIGTERM: release the driver lock, write the terminal summary with `Completion: driver-interrupted`, exit non-zero.
- Ensure the driver lock (L2.C) wraps the entire `--loop` lifetime, not per-iteration.

**Deliverables**

- SKILL.md updates.
- Fixtures:
  - Multi-change plan that runs to `all-done`.
  - Plan with an intentional deferral mid-run → `halted`.
  - Second `/spec:execute` invocation while the first holds the lock → refused with the PID of the running driver.
  - SIGINT simulation → `driver-interrupted` + clean lock release.

**Acceptance**

- A post-run `specify plan validate` on every fixture is green.

**Dependencies**

- L2.C, L2.G.

---

## Change L2.I — `sources` / `affects` execution wiring

**Scope**

- `/spec:execute` resolves `sources` keys against the top-level `sources` map and passes the resolved (path-or-URL, key) tuples to `/spec:define` as initial arguments. `/spec:define`'s brief pipeline hands them to `/spec:extract` via the existing `git-cloner` + `analyze` plugin path; `/spec:execute` does not clone.
- `/spec:execute` passes the `affects` list (change names) to `/spec:define` for delta targeting against the corresponding baseline specs.
- Both signals may co-exist on a single entry; document this in the SKILL.md.

**Deliverables**

- SKILL.md updates.
- Two fixtures: one change with `sources: [monolith]` only, one with `affects: [user-registration]` only. Each fixture runs end-to-end and asserts that define receives the expected arguments (asserted via a stub phase skill that records its arguments to a file, consumed by the fixture snapshot).

**Acceptance** (Layer 2 exit gate)

- `/spec:execute --loop` drives the RFC-2 §"The Plan" example plan to `all-done` on a seeded workspace.
- An injected mid-build SIGKILL, followed by a re-run, recovers via self-heal and completes the initiative.
- A second `/spec:execute` invocation during the first is refused with `Error::DriverBusy`.

**Dependencies**

- L2.H.

---

# Layer 3 — Plan authoring

## Change L3.A — `specify plan init`

**Scope**

- In `augentic/specify-cli`, add `Plan::init(name: &str, sources: BTreeMap<String, String>) -> Plan` and the corresponding `specify plan init <initiative-name> [--source <key>=<path-or-url>...]` subcommand.
- Writes a minimal `.specify/plan.yaml`:
  ```yaml
  name: <initiative-name>
  sources: {}
  changes: []
  ```
  (plus whatever sources were supplied).
- Refuses when `.specify/plan.yaml` already exists — no `--force`; humans run `specify plan archive` first (as RFC-2 §"CLI support" specifies).
- Kebab-case validation on `<initiative-name>` matching the change-name rules.

**Deliverables**

- Library function + CLI subcommand.
- Integration tests: happy path (with and without `--source`); refusal when plan exists; name validation rejection.

**Acceptance**

- Resulting `plan.yaml` passes `specify plan validate`.

**Dependencies**

- L1 exit gate.

---

## Change L3.B — `Plan::archive` co-move

**Scope**

- Extend `Plan::archive` (and `specify plan archive`) to also move `.specify/plans/<name>/` to `.specify/archive/plans/<name>-<YYYYMMDD>/` when present.
- Move is atomic within a filesystem; falls back to copy + delete across filesystems.
- When the working directory is absent, the archive proceeds unchanged.

**Deliverables**

- Updated `archive` signature/behaviour (no new parameters).
- Tests: working dir present → moved alongside the plan; working dir absent → unchanged behaviour; destination already exists → clear error.

**Acceptance**

- Post-archive, `.specify/plans/<name>/` is gone and `.specify/archive/plans/<name>-<YYYYMMDD>/` contains the original tree byte-for-byte.

**Dependencies**

- L1.G, L3.A.

---

## Change L3.C — `Phase::Plan` in `specify-schema`

**Scope**

- Add `Phase::Plan` to `crates/schema/src/lib.rs` (alongside `Define` / `Build` / `Merge`).
- Parse `pipeline.plan` from `schema.yaml`.
- Extend `specify schema pipeline --phase plan` to list authoring briefs.
- Update `schemas/schema.schema.json` (in `augentic/specify-cli`) to allow `pipeline.plan`.
- Update `PhaseArg` / `PipelineView` accordingly.

**Deliverables**

- Type additions + CLI wiring.
- Integration test: schema validation accepts a `pipeline.plan`; `specify schema pipeline --phase plan` lists briefs in declared order (test uses a fixture schema under `tests/fixtures/schema/plan-pipeline/`).

**Acceptance**

- Pre-existing schemas without `pipeline.plan` continue to parse unchanged.

**Dependencies**

- L3.A.

---

## Change L3.D — Omnia `pipeline.plan` briefs

**Scope**

- In `augentic/specify`, ship `schemas/omnia/briefs/plan/discovery.md` and `schemas/omnia/briefs/plan/propose.md` per RFC-2 §"Plan pipeline briefs" and §"Worked example: migration authoring". Include the `needs` / `generates` declarations called out in the table.
- Update `schemas/omnia/schema.yaml` to declare `pipeline.plan` pointing at the two briefs.

**Deliverables**

- Two briefs + schema update.
- Integration test in `specify-cli`: `specify schema pipeline --phase plan` against `schemas/omnia/` resolves the two briefs in order.

**Acceptance**

- Omnia's existing `pipeline.{define,build,merge}` continue to resolve unchanged.

**Dependencies**

- L3.C.

---

## Change L3.E — `/spec:plan` skill scaffold

**Scope**

- In `augentic/specify`, create `plugins/spec/skills/plan/SKILL.md`:
  - Invocation surface: `/spec:plan <initiative-name> [--from <path>...] [--against <path>] [--source <key>=<path-or-url>...] [--focus <area>] [--extend] [--dry-run]`.
  - Core loop (five steps) per RFC-2 §"Core loop".
  - Single-writer invariant: every plan entry is written via `specify plan create`.
  - Working directory layout under `.specify/plans/<name>/`.
  - Constraints (refuses when plan exists without `--extend`; `--dry-run` writes nothing; kebab-case initiative name).
- Subsequent Changes fill in discovery (L3.F) and propose (L3.G).

**Deliverables**

- `SKILL.md` + a `fixtures/` stub directory.
- Scaffold fixture: invocation with `--dry-run` emits a pre-authoring readiness report and writes nothing.

**Acceptance**

- Skill loads cleanly in the plugin registry; `--dry-run` makes no writes.

**Dependencies**

- L3.C.

---

## Change L3.F — `/spec:plan` discovery brief integration

**Scope**

- Wire step 3(a) of RFC-2 §"Core loop": the discovery brief invokes `/spec:extract` (via the Omnia `git-cloner` + `analyze` plugins) once per `--source` / `--against` input and writes a consolidated capability inventory to `.specify/plans/<name>/discovery.md`.
- When only `--from` is supplied (greenfield), discovery reads the artefact files and emits the inventory without calling `/spec:extract`.

**Deliverables**

- SKILL.md updates for the discovery step.
- Fixture: small pre-cloned source tree under `fixtures/plan/discovery/legacy/`; expected `discovery.md` golden file; test snapshots the output.

**Acceptance**

- Running the discovery step twice is idempotent (re-running overwrites `discovery.md` with equivalent content).

**Dependencies**

- L3.D, L3.E.

---

## Change L3.G — `/spec:plan` propose brief integration

**Scope**

- Wire step 3(b) of RFC-2 §"Core loop":
  - The propose brief reads `discovery.md`, decomposes into slices using schema-specific heuristics (leaf-service-first for Omnia migrations).
  - Presents each slice to the human with **accept** / **edit** / **reject** actions.
  - On accept, shells out to `specify plan create <name> --sources ... --depends-on ... --affects ... --description "..."`.
  - On edit, loops with the user's adjustments.
  - On reject, drops the slice.
  - Writes the full proposal to `.specify/plans/<name>/proposal.md` regardless of per-slice decisions.
- Final step of the skill runs `specify plan validate` and exits with a summary pointing at `specify plan status` and `/spec:execute`.
- Honours `--dry-run` (no `specify plan create` calls; emits the proposed plan to stdout) and `--extend` (does not call `specify plan init`; runs `specify plan create` against the existing file).

**Deliverables**

- SKILL.md updates.
- Fixture reproducing RFC-2 §"Worked example: migration authoring": three sources, five accepted slices; snapshot the resulting `plan.yaml`, `discovery.md`, and `proposal.md`.

**Acceptance** (plus, after L3.H and L3.I, Layer 3 exit gate)

- The resulting `plan.yaml` is byte-identical to the shape of RFC-2 §"The Plan" for the equivalent slices.
- `specify plan validate` green.
- `--dry-run` writes nothing.
- `--extend` refuses to run `specify plan init` and only appends entries.

**Dependencies**

- L3.F.

---

## Change L3.H — Vectis `pipeline.plan` briefs

**Scope**

- Mirror L3.D for Vectis: `schemas/vectis/briefs/plan/discovery.md` and `propose.md`, applying Vectis slice heuristics (shared-core-first, per-shell-last). Update `schemas/vectis/schema.yaml`.

**Deliverables**

- Two briefs + schema update.
- Integration test: `specify schema pipeline --phase plan` against `schemas/vectis/` resolves the two briefs.
- One Vectis authoring fixture that exercises `/spec:plan` end-to-end analogous to the Omnia one in L3.G.

**Acceptance**

- Same skill, same CLI; only brief bodies differ.

**Dependencies**

- L3.C. Can ship in parallel with L3.D–L3.G.

---

## Change L3.I — RFC-2 closeout

**Scope**

- In `augentic/specify`:
  - Flip RFC-2 front-matter from `Status: Draft` to `Status: Implemented`.
  - Update `README.md` and `AGENTS.md` to link `/spec:plan` and `/spec:execute`.
  - Cross-link RFC-2 from RFC-3's federation section (the pieces RFC-3 now depends on).
  - Add a CHANGELOG entry (or equivalent — `DECISIONS.md` in `specify-cli` if that is where milestones land) summarising Layer 1 / Layer 2 / Layer 3 delivery.

**Deliverables**

- Doc-only changes across the two repos.

**Acceptance** (Layer 3 exit gate)

- `/spec:plan` on a fresh workspace authors a `plan.yaml` that `/spec:execute --loop` drives to `all-done` without human intervention, against both Omnia and Vectis schemas.
- A human navigating from the top of `augentic/specify`'s README reaches `/spec:plan` / `/spec:execute` in at most two clicks.

**Dependencies**

- L3.G, L3.H.

---

## Post-plan follow-ups (not in this plan)

These are listed here to prevent scope creep during execution; none of them gate a layer exit.

- `specify plan doctor` (RFC-2 §Future Capabilities).
- Prior-attempt context replay on retry (§Future Capabilities).
- Pre-plan baseline delta targeting for `affects` (§Future Capabilities).
- Multiple concurrent plans (§Future Capabilities).
- Renaming the `specify plan` CLI namespace to avoid the `/spec:plan` collision (§Future Capabilities).
- Change recommender, behavioural diff, cross-stack define (§Future Capabilities).
- RFC-3 cross-repo spec references.

## References

- [rfc-2-execution.md](rfc-2-execution.md) — the specification this plan implements.
- [rfc-1-plan.md](archive/rfc-1-plan.md) — analogous plan document for the RFC-1 build; structural template for this document.
- [rfc-1-cli.md](archive/rfc-1-cli.md) — prerequisite CLI / library foundation.
- [rfc-3-multi-repo.md](rfc-3-multi-repo.md) — downstream; cross-repo federation picks up where this plan leaves off.
