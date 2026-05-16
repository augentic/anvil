# RFC-20 Implementation Plan

> Companion to [`rfc-20-survey.md`](rfc-20-survey.md). Repackages the RFC's normative §"Implementation Plan" into subagent-sized changes with explicit dependencies and parallelism. Each change is small enough to land in one subagent context without leaking into the next.

## Conventions

- **Repos.** `specify` is this repo (skills, briefs, fixtures, docs). `specify-cli` is the Rust workspace that produces the `specify` binary (schemas, CLI verb, validators, retired-but-retained `Detector` trait + `DetectorRegistry`).
- **Change shape.** Each change is a single PR-sized unit. Where a change must ship as one atomic release per the RFC, that constraint is called out inline.
- **Sequencing.** Phases are sequential. Inside a phase, changes are independent and may run in parallel subagents.
- **Single-writer rule.** No change in this plan hand-edits `plan.yaml`, `.metadata.yaml`, or any `.specify/archive/` path. Every CLI-touching change routes through `specify`.
- **Producer pivot.** v1 retires the in-tree mechanical detectors (Express, NestJS, BullMQ). The producer of `surfaces.json` is now an LLM driven by per-language enumeration briefs; the CLI is the validator/canonicalizer/writer. The `Detector` trait and `DetectorRegistry` stay in the workspace as deferred extension points (registry is empty in v1).

## Dependency overview

```text
Phase 1 (CLI repo, single atomic PR)
    A. Retire in-tree detectors + refactor `specify change survey` to ingest mode

Phase 2 (specify repo, parallel — per-language briefs)
    B. TypeScript / JavaScript enumeration brief
    C. C# enumeration brief
    D. Rust enumeration brief
    E. COBOL enumeration brief

Phase 3 (specify repo)
    F. /change:survey SKILL.md + references rewrite (agent enumeration + bounded repair loop)
       depends on A and at least one Phase 2 brief (TypeScript is the pilot)

Phase 4 (specify repo, parallel)
    G. Refresh survey acceptance fixtures
    H. Refresh /change:draft end-to-end fixture and SKILL.md / runbook wording
    I. Update project rules + AGENTS.md skill family wording (mechanical → agent)
       all depend on F

Phase 5 (specify repo)
    J. Tutorials update (monolith-decomposition.md, legacy-fleet-decomposition.md)
       depends on F; may overlap with Phase 4
```

## Phase 1 — CLI unwind and refactor

### Change A — Retire in-tree detectors + refactor `specify change survey` to ingest mode

**Repo.** `specify-cli`

**Atomicity.** One PR. The retirement and refactor must land together because the registry-empty intermediate state would brick the verb (`no-detectors` for every input).

**Scope.**

- **Retire in-tree detectors.**
  - Delete `crates/domain/src/survey/detectors/{bullmq,express,nestjs}.rs`.
  - Delete the shared scanning helpers in `crates/domain/src/survey/detectors.rs` that only those detectors used (regex helpers, BFS import resolver, `package.json` helpers, named-import binding resolver).
  - Delete `crates/domain/tests/{detectors_bullmq,detectors_express,detectors_nestjs}.rs`.
  - Delete `crates/domain/tests/fixtures/detectors/`.
  - Keep `crates/domain/src/survey/{detector.rs, registry.rs, merge.rs}` as deferred extension points. `DetectorRegistry::with_builtins()` returns an empty registry.
  - Keep `crates/domain/src/survey/{dto.rs, validate.rs, sources.rs}` as the artifact contract spine.
- **Add path-under-root invariant** to `validate_surfaces` in `crates/domain/src/survey/validate.rs`. Every entry in `touches[]` and `declared-at[]` must:
  - Be a relative path (no leading `/`, no Windows drive letter).
  - Contain no `..` path segments.
  - Resolve to a file under `<source-root>` when joined.
  - On violation, emit `Error::Diag { code: "surfaces-touches-out-of-tree", … }` keyed for the new exit discriminant.
- **Refactor `src/commands/change/survey.rs`** from "run detectors → write" to "ingest staged candidate → validate → canonicalize → capture metadata → write". New flag set:
  - Single-source form: `<source-path> --source-key <key> --surfaces <input.json> --out <dir>`.
  - Batch form: `--sources <file> --staged <dir> --out <dir>`.
  - Either form: `--validate-only` short-circuits metadata-and-write; useful for the skill's repair loop.
  - Single-source vs batch remain mutually exclusive with `--sources`.
- **Per-row pipeline** (replaces `run_single_row`):
  1. Resolve staged candidate path: `--surfaces` (single) or `<staged-dir>/<source-key>.json` (batch). Missing → `staged-input-missing`. Not valid JSON → `staged-input-malformed`.
  2. Deserialize into `SurfacesDocument`. Schema mismatch → `surfaces-validation-failed`.
  3. Run `validate_surfaces` (closed `kind` enum, sort order, path-under-root, non-empty `declared-at`, `id` uniqueness). First failure surfaces a structured detail with the offending field path.
  4. Canonicalize: sort `surfaces[]` by `id`; sort each surface's `touches[]` and `declared-at[]` alphabetically.
  5. Compute `metadata.json` from the source path (existing logic stays). Skip when `--validate-only`.
  6. Atomic-write canonicalized `surfaces.json` and `metadata.json`. Skip when `--validate-only`.
  7. Continue to honour `source-key-mismatch` (existing logic stays).
- **New exit discriminants** wired through `Exit::from(&Error)` in `src/output.rs` and surfaced in CLI man-page output:
  - `staged-input-missing`, `staged-input-malformed`, `surfaces-validation-failed`, `surfaces-id-collision`, `surfaces-touches-out-of-tree`, `source-path-missing`, `source-path-not-readable`, `source-key-mismatch`, `sources-file-missing`, `sources-file-malformed`.
  - Drop: `no-detectors`, `detector-id-collision`, `detector-failure` (no longer reachable in v1; keep the variants commented as deferred extension-point wiring or remove and let the trait-shaped re-introduction add them back).
- **Integration tests.** Replace the deleted `detectors_*.rs` tests with `tests/survey_ingest.rs` covering: happy-path single-source, happy-path batch with two source-keys, every new exit discriminant, `--validate-only` short-circuit, byte-stable canonical output (golden under `tests/fixtures/`).
- **Man-page regen.** `cargo make xtask gen-man`.
- **Re-run `cargo make ci`** after the refactor; lint + nextest + doc + vet + outdated + deny + fmt.

**Out of scope.**

- Per-language enumeration briefs (Phase 2).
- Skill changes (Phase 3).
- The bounded repair loop (lives in the skill, Phase 3).

**Done when.** `cargo make ci` clean; `tests/survey_ingest.rs` covers every new exit discriminant; canonical-output golden is byte-identical across two consecutive runs of the happy-path single-source and batch fixtures; the retired detector code, tests, and fixtures are gone.

## Phase 2 — Per-language enumeration briefs (parallel)

Each change in this phase adds one markdown brief at `plugins/change/skills/survey/briefs/enumerate/<language>.md`. The four briefs are independent and may land in parallel subagents. They share a contract (described in `rfc-20-survey.md` §"Agent Enumeration") but no code dependencies.

**Common scope.** Each brief MUST contain:

- A frontmatter block declaring `id: enumerate`, `description`, and `language`.
- A "Scope" section listing the frameworks the brief covers in v1.
- A "Schema" section that repeats the closed `kind` enum and `Surface` field set verbatim from the schema, plus the path-under-source-root rule.
- One worked example per applicable `kind` showing input snippet → expected `Surface` JSON block.
- An "Anti-patterns" section listing what the brief MUST NOT emit (dead code, unreachable handlers, type-only files in `touches[]`, paths under skip-roots like `node_modules` / `vendor` / `target` / `.venv` / build directories, paths with `..` traversal).
- A "`handler` resolution" section spelling out how to identify the implementation entry for each `kind` in the language.
- A "`touches[]` resolution" section spelling out how to follow the import / project-reference graph from the handler to a finite, reachable file set.

### Change B — TypeScript / JavaScript enumeration brief

**Repo.** `specify`

**Scope.**

- Frameworks covered: Express, NestJS, BullMQ, Fastify, Next.js API routes (App Router and Pages Router).
- Worked examples for: `http-route` (Express + NestJS controller + Fastify + Next.js), `message-pub` (BullMQ producer), `message-sub` (BullMQ worker, NestJS `@MessagePattern` / `@EventPattern`), `scheduled-job` (BullMQ repeatable + node-cron), `ws-handler` (NestJS gateway, plain `ws`), `cli-command` (yargs / commander), `external-call-out` (`fetch` / `axios` / typed SDK call sites).
- `handler` resolution: the named export or class method backing each declaration; for inline arrow handlers, the declaring file with a synthetic suffix.
- `touches[]` resolution: BFS from the handler file across relative `import` / `require` graph, stopping at module boundaries; exclude `*.d.ts` and skip-root directories.

**Done when.** `make checks` passes; the brief's worked examples each produce a `Surface` block that matches the schema verbatim when the LLM follows the brief.

### Change C — C# enumeration brief

**Repo.** `specify`

**Scope.**

- Frameworks covered: ASP.NET Core 6+ controllers, ASP.NET Core minimal API endpoints, MassTransit consumers and publishers, MediatR handlers, Hangfire scheduled jobs, Quartz schedules.
- Worked examples for: `http-route` (controller + minimal API + endpoint routing + `[ApiController]` conventions), `message-pub` (MassTransit publish), `message-sub` (MassTransit consumer + MediatR notification handler), `scheduled-job` (Hangfire / Quartz), `cli-command` (System.CommandLine), `external-call-out` (typed `HttpClient` + `IHttpClientFactory` named clients).
- `handler` resolution: containing class + method for controllers; containing file + endpoint name for minimal API; consumer / handler class for MassTransit / MediatR.
- `touches[]` resolution: walk project references and `using` graph from the handler file; stop at project boundaries.

**Done when.** `make checks` passes; the brief's worked examples cover every applicable kind from the closed enum.

### Change D — Rust enumeration brief

**Repo.** `specify`

**Scope.**

- Frameworks covered: Axum, Actix-web, Rocket, plus common message-broker crates (lapin, rdkafka, async-nats).
- Worked examples for: `http-route` (Axum `Router::route`, Actix `web::resource`, Rocket `#[get]`), `message-pub` (`Producer::send`, `BasicPublish`), `message-sub` (`Consumer::next`, `BasicConsume`, `subscribe`), `scheduled-job` (tokio interval / job-runner crates), `cli-command` (clap derive), `external-call-out` (reqwest / typed SDK call sites).
- `handler` resolution: function backing the route macro / handler closure; for closure handlers, the containing function with a synthetic suffix.
- `touches[]` resolution: walk module graph via `mod` / `pub use` from the handler module; stop at crate boundaries.

**Done when.** `make checks` passes; the brief's worked examples each produce a `Surface` block that matches the schema verbatim.

### Change E — COBOL enumeration brief

**Repo.** `specify`

**Scope.**

- Frameworks covered: CICS BMS maps, IMS DC, MQ Series, batch JCL job steps. Copybook flattening is a precondition.
- Worked examples for: `http-route` (CICS-supplied transactions where applicable), `message-pub` (`MQPUT`), `message-sub` (`MQGET` / triggered programs), `scheduled-job` (JCL job steps + scheduler triggers), `cli-command` (CLI-style entry points where applicable), `external-call-out` (CALL to external programs / DB2 stored procs).
- `handler` resolution: PROGRAM-ID + paragraph entry for the surface.
- `touches[]` resolution: copybook fan-in via `COPY` directives; called program graph via `CALL` statements (static + dynamic where resolvable); stop at the source root boundary (do not chase mainframe-side libraries).
- Acknowledge the brief's limits explicitly: COBOL enumeration is best-effort; operators should expect to edit candidates by hand more often than for the other languages.

**Done when.** `make checks` passes; the brief's worked examples cover at least `http-route`, `message-pub`, `message-sub`, `scheduled-job`, and `external-call-out`.

## Phase 3 — Skill rewrite

### Change F — `/change:survey` SKILL.md + references rewrite

**Repo.** `specify`

**Depends on.** Change A (new CLI shape) and at least Change B (TypeScript pilot brief). The remaining briefs (C / D / E) may land before or after F.

**Scope.**

- **`plugins/change/skills/survey/SKILL.md`.** Rewrite the Critical Path to reflect the agent producer:
  1. Enumerate sources from `change.md` / `plan.yaml`.
  2. Per source: detect language → resolve enumeration brief → drive LLM → write candidate to `<staged-dir>/<source-key>.json`.
  3. Run `specify change survey --sources <file> --staged <dir> --out .specify/plans/<change>/survey/`.
  4. On `surfaces-validation-failed` / `surfaces-id-collision` / `surfaces-touches-out-of-tree`: enter the bounded repair loop (v1: 3 retries with the structured validator output fed back to the LLM); on exhaustion, exit `surveyor-exhausted` and print the last failing candidate alongside the validator output.
  5. Read canonicalized sidecars; run candidate algorithm; write `survey.md`; append blocks under the discovery-owned `## Candidate inventory` heading.
- **Sources file shape.** Update `plugins/change/skills/survey/references/` (or add `references/sources-yaml-shape.md`) to document the `--sources` YAML format, including the staged-input convention.
- **Determinism policy.** New section in `SKILL.md` referencing `rfc-20-survey.md` §"Determinism Policy": schema-stable, sort-stable, idempotent on unchanged inputs, pinned brief, bounded repair loop.
- **Failure-mode table.** Update the existing table in `SKILL.md` to map the new exit discriminants from Change A; remove `no-detectors` / `detector-id-collision` / `detector-failure` rows (no longer reachable).
- **Brief resolution.** Document the lookup at `plugins/change/skills/survey/briefs/enumerate/<language>.md`. Define behaviour when no brief exists for the detected language: fail closed with a clear operator message ("no enumeration brief for `<language>`; supported languages: …") rather than degrading to a no-op.
- **Repair loop reference.** Add `references/repair-loop.md` carrying the structured-error feedback grammar, retry budget (3), and the `surveyor-exhausted` exit contract.
- **Capability briefs unchanged.** `briefs/omnia/cluster.md` and `briefs/vectis/cluster.md` keep their current shape — they live on a separate axis (clustering, not enumeration). Confirm this is wired into the orientation paragraph.
- **Reference table.** Add rows for `briefs/enumerate/` (per-language enumeration briefs) and `references/repair-loop.md`.

**Out of scope.**

- Fixture refresh (Phase 4 / Change G).
- Tutorials (Phase 5 / Change J).
- `/change:draft` SKILL.md edits beyond the wording for the survey step (Phase 4 / Change H).

**Done when.** `make checks` passes; one fresh end-to-end run against the existing `survey-end-to-end` draft fixture exercises the full pipeline (LLM enumeration → CLI validation → canonicalization → candidate algorithm → discovery handshake) and produces a stable canonical `surfaces.json` + `survey.md` after at most one repair-loop iteration on a synthetic shape error.

## Phase 4 — Fixture and surface alignment (parallel)

### Change G — Refresh survey acceptance fixtures

**Repo.** `specify`

**Depends on.** Change F.

**Scope.** The existing fixtures under `plugins/change/skills/survey/fixtures/` mostly already pre-bake `surfaces.json` inputs. Confirm and extend:

- `single-source-monolith/`, `single-source-small/`, `multi-source-fleet/`, `too-large-unresolved/`, `heading-handshake/`, `greenfield-doc-only/`: each fixture's `inputs/` ships the staged `surfaces.json` candidate(s) and the `--sources` file. Expected directory captures the canonicalized `surfaces.json` + `metadata.json` + `survey.md` + `discovery.md` after CLI ingest.
- New: `repair-loop/` fixture exercising the bounded retry path. Stage an initial candidate that fails `surfaces-touches-out-of-tree`, expect the skill to repair and succeed within the retry budget. Includes a sibling `repair-loop-exhausted/` fixture where the candidate exhausts the budget and the skill exits `surveyor-exhausted`.
- Drop: any remaining fixtures that exercised the retired in-binary detectors (none should remain in `specify` — they all lived under `specify-cli/crates/domain/tests/fixtures/detectors/`, retired in Change A).
- All expected outputs are byte-stable.

**Out of scope.**

- Cross-source pairing fixtures (deferred per RFC §"Out Of Scope").

**Done when.** Each fixture regenerates byte-identically over two consecutive runs; the repair-loop fixture proves the retry path and the exhausted fixture proves the exit contract.

### Change H — `/change:draft` SKILL.md + end-to-end fixture wording

**Repo.** `specify`

**Depends on.** Change F.

**Scope.**

- Update `plugins/change/skills/draft/SKILL.md` step 4(c) wording to drop "mechanical" framing and reference the agent enumeration model. The step itself still invokes `/change:survey`; the description changes.
- Update `plugins/change/skills/draft/references/runbook.md` for the same wording shift.
- Update `plugins/change/skills/draft/fixtures/survey-end-to-end/README.md` and any expected outputs that reflect the agent producer (e.g. validator-error replays in the readme prose).
- Confirm the discovery brief still writes `## Candidate inventory` exactly once and that no draft brief reintroduced the heading after Phase 1.

**Out of scope.**

- Tutorials (Change J).

**Done when.** `make checks` passes; the survey-end-to-end fixture regenerates byte-identically and the readme reflects agent enumeration.

### Change I — Project rules + AGENTS.md skill family wording

**Repo.** `specify`

**Depends on.** Change F.

**Scope.**

- `.cursor/rules/project.mdc`: update the `/change:survey` row in the skill family table to "decompose legacy-code sources into slice-sized candidates by driving per-language enumeration briefs and validating the result against the closed `surfaces.json` schema". Drop the "mechanically scanning surfaces" phrasing.
- `AGENTS.md` (root): update the matching skill description.
- Search the repo for any remaining "mechanically decompose" / "mechanical scanner" / "mechanical detector" phrasing tied to RFC-20 and either rewrite or remove. (Leave RFC-20 §"Design History" intact — it explicitly explains the pivot.)

**Out of scope.**

- Tutorials (Change J).

**Done when.** `make checks` passes; `rg -i "mechanical (scanner|detector|decompos)" plugins/ docs/ AGENTS.md .cursor/` returns only references inside `rfcs/` (RFC text is allowed to reference the prior design).

## Phase 5 — Tutorials

### Change J — Tutorials update

**Repo.** `specify`

**Depends on.** Change F. May overlap with Phase 4 once F is merged.

**Scope.**

- `docs/tutorials/monolith-decomposition.md`: rewrite the survey-step section to walk the operator through agent enumeration on a TypeScript monolith. Show one repair-loop iteration so the operator sees the validator error → re-prompt → success path. Keep the existing candidate-algorithm walk-through.
- `docs/tutorials/legacy-fleet-decomposition.md`: same treatment, two source-keys, plus a worked example of operator-driven candidate combination during `propose` (since v1 still does not pair across sources mechanically).
- `docs/explanation/legacy-migration-at-scale.md`: keep the existing Decision-log indirection for RFC references (per the no-RFC-citation rule on user-facing docs); update prose so the producer model is described as "per-language enumeration brief plus deterministic validator" rather than "mechanical scanner".

**Out of scope.**

- Reconciliation, cross-source pairing, dependency ordering content — all deferred per RFC §"Out Of Scope".

**Done when.** `make checks` passes; both tutorials render cleanly and the explanation doc no longer cites a removed mechanical scanner.

## Parallelism summary

| Phase | Parallel subagents                           | Sequential gate                                                  |
| ----- | -------------------------------------------- | ---------------------------------------------------------------- |
| 1     | A (CLI, single atomic PR)                    | —                                                                |
| 2     | B ∥ C ∥ D ∥ E (specify; per-language briefs) | A merged for end-to-end testing only; briefs may draft against A |
| 3     | F (specify, atomic single PR)                | A; at least one Phase 2 brief (TypeScript = pilot)               |
| 4     | G (specify) ∥ H (specify) ∥ I (specify)      | F                                                                |
| 5     | J (specify) — may overlap with Phase 4       | F (design); content can draft against the merged F PR            |

## Cross-cutting guardrails

These apply to every change in this plan and are enforced by `make checks` / `cargo make ci`:

- **Schema-stable + sort-stable outputs.** Canonical `surfaces.json`, `metadata.json`, `survey.md`, and `discovery.md` are produced by the CLI in canonical form. Fixed field order, sorted lists, no timestamps, no absolute paths, no host-state.
- **Single-writer for plan state.** No change in this plan hand-edits `plan.yaml`, `.metadata.yaml`, or anything under `.specify/archive/`. Route through `specify change draft`, `specify plan {add, amend, transition}`, and `specify change finalize`.
- **Closed `kind` enum** on `surfaces[]`. Extensions are an RFC update, not a feature flag or a brief edit.
- **Atomic per-row writes** in the batch survey form. Row failure must not touch other rows' files.
- **Fail-closed on unknowns.** Unknown surface kinds, paths outside the source root, malformed sidecars, and missing staged inputs fail validation. Ambiguity becomes `unresolved: true` at the candidate level, never silent merging.
- **No LLM in the validator.** `specify change survey` is deterministic. The repair loop lives in the skill; the CLI's only job is to validate, canonicalize, and write.
- **Bounded repair loop.** v1 retry budget is 3. Exhaustion exits `surveyor-exhausted` with the last failing candidate and validator output preserved for the operator.
- **Detector trait + registry retained.** `crates/domain/src/survey/{detector.rs, registry.rs, merge.rs}` stay in the workspace as deferred extension points. Re-introducing an in-binary detector for a (language, framework) pair is an RFC update, not a feature flag.