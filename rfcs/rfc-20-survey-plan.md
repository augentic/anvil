# RFC-20 Implementation Plan

> Companion to [`rfc-20-survey.md`](archive/rfc-20-survey.md). Repackages the RFC's normative §"Implementation Plan" into subagent-sized changes with explicit dependencies and parallelism. Each change is small enough to land in one subagent context without leaking into the next.

## Conventions

- **Repos.** `specify` is this repo (skills, briefs, fixtures, docs). `specify-cli` is the Rust workspace that produces the `specify` binary (schemas, CLI verb, detector trait, in-binary detectors).
- **Change shape.** Each change is a single PR-sized unit. Where a change must ship as one atomic release per the RFC, that constraint is called out inline.
- **Sequencing.** Phases are sequential. Inside a phase, changes are independent and may run in parallel subagents.
- **Single-writer rule.** No change in this plan hand-edits `plan.yaml`, `.metadata.yaml`, or any `.specify/archive/` path. Every CLI-touching change routes through `specify`.

## Dependency overview

```text
Phase 1 (parallel)
    A. surfaces.json + metadata.json schemas        (specify-cli)
    E. /change:analyze rewrite (documentation only) (specify)

Phase 2
    B. Detector trait + DetectorRegistry skeleton   (specify-cli)  depends on A

Phase 3 (parallel)
    C. specify change survey CLI verb               (specify-cli)  depends on A, B
    D. First-stack detectors (Express, NestJS, BullMQ) (specify-cli)  depends on B

Phase 4
    F. Discovery-brief handshake + /change:survey skill (specify)  depends on C, E
       (must ship as one PR per RFC step 5: "discovery + survey heading handshake")

Phase 5 (parallel)
    G. Acceptance fixtures                          (specify)      depends on C, D, E, F
    I. /change:draft SKILL.md + runbook updates     (specify)      depends on F

Phase 6
    H. Tutorials + legacy-migration-at-scale stub   (specify)      depends on F (design only)
       (may overlap with Phase 5 once F is merged)
```

## Phase 1 — Foundations (parallel)

### Change A — `surfaces.json` + `metadata.json` schemas

**Repo.** `specify-cli`

**Scope.**

- Add `schemas/surfaces.schema.json` matching the RFC §"Artifacts → `surfaces.json`" shape exactly. Required fields: `version` (const `1`), `source-key`, `language`, `surfaces[]`. Per-surface: `id`, `kind` (closed enum: `http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`), `identifier`, `handler`, `touches[]`, `declared-at[]`.
- Add `schemas/survey-metadata.schema.json` for `metadata.json` (language, LOC, module count, top-level modules — same shape `/change:analyze` writes today; reuse the v1 envelope).
- Add validators in `crates/domain/` enforcing: `version == 1`, closed `kind` enum, `surfaces[]` sorted by `id`, per-surface `touches[]` sorted alphabetically, `declared-at[]` non-empty and sorted, no absolute paths, no timestamps, no host-state leaks.
- DTOs + `serde_yaml` (saphyr) round-trip tests; byte-stable golden under `tests/fixtures/`.

**Out of scope.**

- Reading the schemas from any CLI verb (Change C does that).
- The detector trait (Change B).

**Done when.** `cargo make test` passes for new schema + validator coverage; goldens are byte-stable across two runs.

---

### Change E — `/change:analyze` rewritten for `documentation` only

**Repo.** `specify`

**Scope.**

- Edit `plugins/change/skills/analyze/SKILL.md`:
  - Remove the `kind` positional from the argument hint and the "Input kinds (closed enum)" table.
  - Drop the `legacy-code` branch entirely. Documentation is the only accepted input in v1.
  - Replace the existing per-capability YAML block with the unified fenced-YAML candidate block shape from RFC-20 §"Artifacts → `survey.md`" (`kind: candidate`, `sources`, `handler` (optional for doc-derived), `touches` (optional), `surfaces`, `declared-at`, `unresolved`).
  - Make explicit that `/change:analyze` appends blocks under a pre-existing `## Candidate inventory` heading (it does not write the heading itself).
- Move the legacy-code clustering content out of `plugins/change/skills/draft/briefs/<cap>/analyze.md` into a new reserved location `plugins/change/skills/survey/briefs/<cap>/cluster.md` (file created; only contents moved — Change F wires it up).
- Update `plugins/change/skills/draft/briefs/omnia/analyze.md` and `plugins/change/skills/draft/briefs/vectis/analyze.md` to retain only documentation prose and the new emitted block shape; remove the `kind` dispatch.
- Update `plugins/change/skills/analyze/fixtures/` so the existing scaffold-example reflects the new block shape and the documentation-only contract.

**Out of scope.**

- The `## Candidate inventory` heading-writing logic (lives in discovery brief, Change F).
- `/change:survey` skill (Change F).

**Done when.** `make checks` passes; analyze fixtures show the unified candidate block; no remaining `kind == legacy-code` paths in this skill.

## Phase 2 — Detector contract

### Change B — Detector trait + `DetectorRegistry` skeleton

**Repo.** `specify-cli`

**Depends on.** Change A (the `Surface` DTO must match the schema verbatim).

**Scope.**

- Add the `Detector` trait, `DetectorInput<'a>`, and `DetectorOutput` shapes per RFC §"Detector Contract":
  ```rust
  struct DetectorInput<'a> { source_root: &'a Path, language_hint: Option<Language> }
  struct DetectorOutput { surfaces: Vec<Surface> }
  ```
- Add `DetectorRegistry` populated at binary build time. Empty registry is fine for this change; Change D wires real detectors.
- Add the merge + dedup helper that the CLI verb (Change C) will call: merge `surfaces[]` across all detectors, assert no duplicate `id`, surface a structured error keyed for `detector-id-collision` and `detector-failure`.
- Carve detector code into its own module under `crates/domain/src/survey/` (no `mod.rs`; follow [coding-standards.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/coding-standards.md)).

**Out of scope.**

- Any real detector implementations (Change D).
- The CLI verb wiring (Change C).
- WASI tool packaging — explicitly deferred per RFC.

**Done when.** Trait compiles, registry instantiates empty, merge helper round-trips through schema validation with a synthetic `Surface` vector.

## Phase 3 — CLI verb + first detectors (parallel)

### Change C — `specify change survey` CLI verb

**Repo.** `specify-cli`

**Depends on.** Changes A, B.

**Scope.**

- Add the `Survey` variant to `ChangeAction` in `src/commands/change/cli.rs` (peer of `Draft`, `Show`, `Finalize`).
- Wire both invocation forms:
  - Single-source: `<source-path>` + required `--source-key <key>` + `--out <dir>`.
  - Batch: `--sources <file>` + `--out <dir>`. Define and parse the small `--sources` YAML (`version: 1`, `sources[].{key,path}`) under `crates/domain/`.
- Enforce mutual exclusion of `<source-path>` / `--source-key` vs `--sources`.
- Output contract:
  - Single-source: write `<dir>/surfaces.json` + `<dir>/metadata.json` atomically.
  - Batch: write `<dir>/<source-key>/surfaces.json` + `<dir>/<source-key>/metadata.json` atomically per row. Independent per-row writes; row failure leaves that row's files untouched and does not touch other rows' files.
  - Refuse to overwrite a `surfaces.json` whose `source-key` does not match the requested key.
- Coarse `metadata.json` capture (language, LOC, module count, top-level modules) — reuse the convention already pinned by `/change:analyze`.
- Map every RFC-listed exit discriminant onto an `Error` variant + `Exit::from(&Error)` row:
  - `no-detectors`, `detector-id-collision`, `source-path-missing`, `source-path-not-readable`, `detector-failure`, `sources-file-missing`, `sources-file-malformed`.
- Integration tests covering each exit discriminant + byte-stable golden output on a small synthetic source tree.
- Man-page regen (`cargo make xtask gen-man`).

**Out of scope.**

- Real detectors (Change D).
- The `--format` flag (intentionally absent in v1 per RFC).
- Anything Markdown — the verb is JSON-only.

**Done when.** `cargo make ci` clean; single-source and batch forms round-trip with an empty registry against a synthetic source.

---

### Change D — First-stack detectors (Express, NestJS, BullMQ)

**Repo.** `specify-cli`

**Depends on.** Change B. May land in parallel with Change C — Change C consumes the registry through the trait, not concrete detectors.

**Scope.**

- Implement `ExpressDetector`, `NestJsDetector`, `BullMqDetector` against the RFC §"Detector Contract" trait.
- Each detector self-reports applicability (empty `DetectorOutput` when framework signatures are absent).
- Per detector, populate `Surface` with the legacy spelling preserved in `identifier`, a stable per-source `id`, the handler resolution where static analysis can do so, `touches[]` covering reachable source files, and `declared-at[]` pointing at the route mount / job declaration / subscription registration site.
- Register them in `DetectorRegistry` at binary build time.
- Integration tests with small synthetic Express / NestJS / BullMQ trees under `tests/fixtures/`. Each fixture asserts byte-stable `surfaces.json` and exercises the no-duplicate-`id` invariant.

**Out of scope.**

- WASI packaging or out-of-tree detector packs (deferred per RFC).
- LLM-fallback detection (RFC non-goal).
- Other frameworks (Fastify, Koa, RabbitMQ, …) — re-open trigger per RFC §"Out Of Scope".

**Done when.** All three detectors register without collision and produce byte-stable surfaces against their fixture trees.

## Phase 4 — Skill + handshake

### Change F — Discovery-brief handshake + `/change:survey` skill (atomic)

**Repo.** `specify`

**Depends on.** Changes C, E.

**Atomicity constraint.** RFC §"Implementation Plan" step 5 mandates: ship the discovery-brief heading edit *together with* the `/change:survey` skill in **one PR** to avoid a half-state where survey expects a heading the brief does not write.

**Scope.**

- **Discovery brief edit.** Update `plugins/change/skills/draft/briefs/omnia/discovery.md` and `plugins/change/skills/draft/briefs/vectis/discovery.md` to write the `## Candidate inventory` heading wrapper into `discovery.md` exactly once, before either analyze or survey runs. Update `plugins/change/skills/draft/discovery.md` to reflect that both `/change:analyze` and `/change:survey` append under this heading.
- **New `/change:survey` skill.** Add `plugins/change/skills/survey/SKILL.md` plus per-capability briefs at `plugins/change/skills/survey/briefs/omnia/cluster.md` and `plugins/change/skills/survey/briefs/vectis/cluster.md` (cluster.md content moved during Change E).
- **Skill responsibilities** (per RFC §"Skill Responsibility Split"):
  1. Build the `--sources` batch file from the change's recorded `legacy-code` sources.
  2. Invoke `specify change survey --sources <file> --out .specify/plans/<change>/survey/` once.
  3. Compose all `surfaces.json` files into one inventory.
  4. Size each candidate (production LOC; `acceptable < 1000`, `too-large >= 1000`).
  5. Apply minimal same-source clustering: shared `touches` overlap (≥ 50%), explicit documentation grouping in `discovery.md`, shared handler/call site.
  6. Mark `too-large` candidates that cannot be split as `unresolved: true`.
  7. Render the thin `declared-at` block (sorted `file` or `file:line` entries — single renderer, no detector-hand-written prose).
  8. Write `.specify/plans/<change>/survey.md` with required sections in order: `Summary`, `Source inventory`, `Candidate inventory`. Byte-stable.
  9. Append candidate blocks under the discovery-owned `## Candidate inventory` heading in `discovery.md` using the unified fenced-YAML grammar.
- **Wire into `/change:draft` pipeline.** Insert the survey step between workspace sync (4b) and propose (4c) per RFC §"`/change:draft` Analysis Flow". Update the propose brief to refuse `unresolved: true` candidates until the operator edits them.
- **Routing.** Survey-derived candidates carry no `target-project`; assignment continues to use today's signals (RFC §"Routing Behavior (v1)").
- **Brownfield.** Survey treats baseline projects as opaque routing targets; it does not read `.specify/specs/` (RFC §"Brownfield Behavior (v1)").
- **Single fixture** at minimum to prove the heading handshake (the full set lands in Change G).
- Update `make checks` predicates if any new skill-authoring rule is introduced.

**Out of scope.**

- Tutorials and the legacy-migration-at-scale doc (Change H).
- Full acceptance fixture set (Change G).
- Cross-source pairing, dependency ordering, alias files (all deferred per RFC §"Out Of Scope").

**Done when.** `make checks` passes; one fresh `/change:draft` end-to-end fixture shows `## Candidate inventory` emitted exactly once and survey blocks appended underneath.

## Phase 5 — Polish (parallel)

### Change G — Acceptance fixtures

**Repo.** `specify`

**Depends on.** Changes C, D, E, F.

**Scope.** Ship the six v1 fixtures from RFC §"Implementation Plan" step 7:

1. Single-source L monolith producing surface-sized candidates with one minimal same-source cluster (core happy path).
2. Multi-source change with **≥ 2 source-keys** producing one combined inventory with separate source-local candidates (proves repo-fleet handling without cross-source pairing).
3. Greenfield documentation-only pass-through (survey skipped entirely).
4. Single-source-already-S no-op (source is its own terminal candidate without further partitioning).
5. `too-large` candidate produced by minimal same-source clustering that cannot be split — emitted `unresolved: true`.
6. Fresh `/change:draft` end-to-end exercising the discovery + `/change:survey` handshake; asserts `## Candidate inventory` is emitted exactly once.

Each fixture lives under `plugins/change/skills/survey/fixtures/` (or the existing `/change:draft` fixtures dir where the fixture spans the full pipeline). Byte-stable expectations.

**Out of scope.**

- Escape-hatch fixtures (cross-source pairing, `depends-on` cycle, alias-resolved `unresolved` on ≥ 3-source-key plans). Deferred per RFC §"Out Of Scope".

**Done when.** All six fixtures regenerate byte-identically on a clean run.

---

### Change I — `/change:draft` SKILL.md + runbook updates

**Repo.** `specify`

**Depends on.** Change F.

**Scope.**

- Update `plugins/change/skills/draft/SKILL.md` "Critical Path" and "Orientation" sections to insert the survey step between sync-workspace (4b) and propose (4c) for legacy-code changes.
- Update `plugins/change/skills/draft/references/runbook.md` (the verbatim six-step loop is now seven for legacy-code changes; documentation-only changes still skip survey entirely per RFC §"Migration").
- Update the Reference Documentation table in draft SKILL.md to point at the new `/change:survey` skill.
- Update `/Users/andrewweston/github.com/augentic/specify/AGENTS.md` and `/Users/andrewweston/github.com/augentic/specify/.cursor/rules/project.mdc` to add `/change:survey` to the skill family table.

**Out of scope.**

- Tutorials (Change H).

**Done when.** `make checks` passes; the survey step is reachable from both AGENTS.md and the project rule in three navigational hops.

## Phase 6 — Documentation

### Change H — Tutorials + legacy-migration-at-scale stub

**Repo.** `specify`

**Depends on.** Change F design only — may overlap with Phase 5 once F is merged.

**Scope.**

- Add `docs/tutorials/monolith-decomposition.md` — a single `legacy-code` source through `/change:draft` showing surface-sized candidates and one minimal same-source cluster.
- Add `docs/tutorials/legacy-fleet-decomposition.md` — multi-source change showing separate source-local candidates and the operator review point where related candidates may be combined.
- Add the stub at `docs/explanation/legacy-migration-at-scale.md` referencing RFC-21 / RFC-22 as the eventual home for cross-change scale concerns (per RFC §"Implementation Plan" step 8).
- Cross-link from `docs/SUMMARY.md` and any relevant orientation pages.

**Out of scope.**

- Reconciliation, cross-source pairing, dependency ordering content — all deferred per RFC §"Out Of Scope".

**Done when.** `make checks` passes; both tutorials render cleanly in the book.

## Parallelism summary

| Phase | Parallel subagents                      | Sequential gate                                          |
| ----- | --------------------------------------- | -------------------------------------------------------- |
| 1     | A (CLI) ∥ E (specify)                   | —                                                        |
| 2     | B (CLI)                                 | A                                                        |
| 3     | C (CLI) ∥ D (CLI)                       | B                                                        |
| 4     | F (specify, atomic single PR)           | C, E                                                     |
| 5     | G (specify) ∥ I (specify)               | F (and C, D, E for G)                                    |
| 6     | H (specify) — may overlap with Phase 5  | F (design); content can draft against the merged F PR    |

## Cross-cutting guardrails

These apply to every change in this plan and are enforced by `make checks` / `cargo make ci`:

- **Byte-stable outputs.** No timestamps, absolute paths, or host-state in `surfaces.json`, `metadata.json`, `survey.md`, or `discovery.md`. Fixed field order, sorted lists.
- **Single-writer for plan state.** No change in this plan hand-edits `plan.yaml`, `.metadata.yaml`, or anything under `.specify/archive/`. Route through `specify change draft`, `specify plan {add, amend, transition}`, and `specify change finalize`.
- **Closed `kind` enum** on `surfaces[]`. Extensions are an RFC update, not a feature flag.
- **Atomic per-row writes** in the batch survey form. Row failure must not touch other rows' files.
- **Fail-closed on unknowns.** Unknown surface kinds and malformed sidecars fail validation; ambiguity becomes `unresolved: true`, never silent merging.
- **No LLM in the scanner.** `specify change survey` is mechanical only. LLM-fallback detection is an RFC §"Out Of Scope" item with an explicit re-open trigger.
