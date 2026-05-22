# RFC-25 Implementation Plan

> Companion to [rfc-25-workflow.md](rfc-25-workflow.md). Decomposes the RFC's 17-step plan and 3-stage PR train into subagent-sized changes (changes), grouped into sequential waves with parallel slots identified. Each change is scoped to land in a focused subagent run.
>
> **Status:** Active. Reinstated after a brief retirement during the RFC implementation review pass. Use this document to track the remaining 2.0 cutover work; [rfc-25-workflow.md](rfc-25-workflow.md) remains the normative workflow contract.

## Progress snapshot

Last reviewed against the working tree on the 2.0 migration branch. **Done** means the chunk's deliverables exist; **partial** means bodies or fixtures landed but the acceptance tests named in the chunk have not passed end-to-end; **open** means not started or blocked.

| Chunk | Status | Notes |
| --- | --- | --- |
| W0.1–W0.3 | done | W0.3 ships as `crates/domain/src/adapter/` (axis-aware loader), not a `plugin/` rename — functionally equivalent. |
| W1.1–W1.4 | done | `divergence: likely` is still skill-written; CLI rejects it on `plan amend` (see [rfc-25-synthesis.md](rfc-25-synthesis.md) §4). |
| W2.1–W2.7 | partial | Manifests, briefs, and static fixtures under `sources/` and `targets/` landed; source/target harness replay is open (see **Outstanding work**). |
| W3.1 | partial | Synthesis playbook + `/spec:refine` skill landed; golden synthesis runner not wired. |
| W3.2 | done | `/spec:plan` skill + plan fixtures. |
| W3.3 | partial | `/spec:execute` skill + shape fixtures; executable replay open. |
| W3.4 | partial | `/spec:build` + `/spec:merge` skills + shape fixtures; executable replay open. |
| W3.5 | partial | `/spec:finalize` skill + transcript fixtures; PR-observation harness mock open. |
| W4.1 | partial | Core docs refreshed; `docs/reference/adapters/*` URLs and `skills-test-coverage.md` 2.0 retag still open. |
| W4.2 | done | CLI docs and man pages updated in `specify-cli`. |
| W5.1 | done | `scripts/migrate-to-2.0.sh`, `docs/migration/2.0.md`, `tests/migration_test.ts`. |
| W5.2 | done | `plugins/change/`, `/spec:define`, `/spec:extract` retired. |
| W5.3 | open | Full §Acceptance scenarios #1–#12 sweep + `specify-cli` / `specify` 2.0.0 tags. |

**Critical path to release:** W3.1 golden harness → W3.3–W3.4 fixture replay → W5.3 acceptance sweep.

## Outstanding work

These items were implicit in the original chunks but are easier to track as explicit follow-ups:

1. **Acceptance harness (`plg`)** — Replay fixture trees end-to-end:
   - `tests/fixtures/skills/refine/` (synthesis goldens; consumes W1.3 provenance parser)
   - `tests/fixtures/skills/{execute,build,merge}/`
   - `tests/fixtures/sources/{intent,documentation,code-typescript,screenshots}/`
   - `tests/fixtures/targets/{omnia,vectis}/`
   - Restore or replace the missing `tests/cross_repo.ts` hook referenced from `Makefile` `test` target.
2. **W5.3 release gate** — Run scenarios #1–#12 from [rfc-25-workflow.md](rfc-25-workflow.md) §Acceptance scenarios; scenario #1 is the release blocker. Today the cross-repo and plan packs remain manual (`docs/contributing/acceptance.md`); `specify-cli/tests/cross_repo.rs` covers CLI substrate only.
3. **Docs tail (W4.1)** — Retag `docs/contributing/skills-test-coverage.md` for 2.0 verbs; point install URLs at `targets/<name>` not `adapters/<name>`; stub or replace `plugins/spec/references/phase-outcome-contract.md` (see `REVIEW.md` F3).
4. **Vectis shared assets (follow-up)** — Move `adapters/vectis/{composition,tokens,assets}.schema.json`, `codex/`, and `examples/` under `targets/vectis/` or top-level `schemas/vectis/` without breaking published JSON Schema `$id` URLs. Not part of the original W5.2 deletion chunk.

## Conventions

- **Repos.** `cli` = `augentic/specify-cli` (Rust workspace). `plg` = `augentic/specify` (plugins, skills, docs, RFCs).
- **Change id.** `Wn.k` where `n` is the wave and `k` is the slot inside the wave. Same-wave slots are parallelizable unless an explicit `requires:` line on the change adds a finer-grained dep.
- **Status convention.** Every change is sized so a single subagent can hold its files, briefs, and tests in context. If a change starts overflowing, split on the dotted-line subheadings inside it before adding new top-level entries.
- **RFC-step xref.** Each change carries the matching `rfc-25-workflow.md` §Implementation plan step ids in `[brackets]`.
- **Acceptance.** The `[acceptance]` field lists the §Acceptance scenarios that exercise the change end-to-end; a wave is "done" only when every acceptance scenario it claims passes.

```text
Wave 0 (cli)            schemas, type renames, plugin loader            sequential within wave
   |
Wave 1 (cli)            CLI verbs, lifecycle collapse, parser, journal  fully parallel
   |
Wave 2 (plg, cli)       source adapters x 4, target brief migration     fully parallel
   |
Wave 3 (plg)            synthesis lib, /spec:* skill bodies             parallel after W2
   |
Wave 4 (plg)            docs, AGENTS.md, project.mdc                    parallel
   |
Wave 5 (cli + plg)      migrate-to-2.0.sh, deletions, acceptance run    sequential
```

## Wave 0 — CLI foundation (must land first)

Sequential within the wave. Every downstream wave imports the schemas, renamed types, and loader landed here. Land in `cli`, in this order.

### W0.1 — Land RFC-25 JSON Schemas `[step 1]`

- Repo: `cli`
- Files: `schemas/plugin.schema.json`, `schemas/source.schema.json`, `schemas/target.schema.json`, `schemas/evidence.schema.json`, `schemas/discovery/candidate.schema.json`; update existing `schemas/plan/*` for `target` field, structured `slices[].sources[]` (`{ key, candidate }[]` plus bare-string shorthand), and the collapsed `pending | reviewed` plan-lifecycle enum.
- Logic: schema files + first-use validation hooks in `specify slice validate`, `plan add`, `plan amend`.
- Tests: schema golden fixtures under `crates/domain/tests/schemas/` plus `tests/fixtures/plan/v2/*` covering N=1 intent, multi-source, `divergence: likely`.
- `[acceptance]` #5g (invalid Evidence rejection).
- Subagent notes: the new `region` / `container` / `leaf` claim kinds belong in `evidence.schema.json` from day one — they are added by the Vectis source/target split (see W2.4).

### W0.2 — Rename `Adapter*` → `Target*` (domain types) `[step 2]`

- Repo: `cli`
- Files: `crates/domain/src/adapter*` and every call site; touch `slice.rs`, `plan.rs`, `change.rs`, `cmd.rs`, `validate.rs`, error variants, JSON envelope shapes, fixture YAMLs.
- Logic: pure rename + add `Plan::resolve_sources` constructor that resolves the new `Slice.sources: { key, candidate }[]` shape against `plan.yaml.sources`.
- Tests: rename test fixtures wholesale; `cargo make ci` must stay green.
- `requires`: W0.1 (schema is the wire contract the rename emits against).
- `[acceptance]` #5a (combined-evidence resolves the right bindings).
- Subagent notes: also rename `Slice.adapter` → `Slice.target` and any error discriminant containing `adapter` that names the output role (e.g. `init-requires-adapter-or-hub` → `init-requires-target-or-workspace`). Leave `adapter` alone where it names the shared shape (it becomes `plugin` in W0.3).

### W0.3 — Plugin loader replaces `adapter/` loader `[step 3]`

- Repo: `cli`
- Files: new `crates/domain/src/plugin/`, retire `crates/domain/src/adapter/`; update `lib.rs`, `cmd.rs`, anything that loaded an adapter manifest.
- Logic: single resolver routed by `axis: source | target`; per-axis cache directory `.specify/.cache/{sources,targets}/<name>/`.
- Tests: load fixtures from both `sources/<name>/adapter.yaml` and `targets/<name>/adapter.yaml` paths; assert axis routing and cache placement.
- `requires`: W0.1, W0.2.
- `[acceptance]` #2 (documentation enumerate at the new entry point), #4 (code source resolves).

## Wave 1 — CLI verbs, lifecycle, parser, journal (parallel after Wave 0)

Four fully parallel subagents. Each touches a disjoint slice of `cli/`.

### W1.1 — CLI verbs: source/target resolve, plan amend sources, retirements `[step 8]`

- Repo: `cli`
- Files: `crates/domain/src/cmd.rs`, the `specify source`, `specify target`, `specify plan` subcommand modules, `--help` text, man-page xtask.
- Logic: add `specify source resolve <name>`, `specify target resolve <value>`, `specify plan amend --add-source <key>=<candidate>` / `--remove-source <key>` / `--divergence accepted|rejected`. Retire `specify adapter *`, `specify change *`, `specify change survey`, `specify adapter pipeline`. Update `specify plan add` / `amend` `--sources` to accept `<key>=<candidate-id>` (bare `<key>` shorthand under the §`Slice.sources` rule).
- Tests: integration tests under `tests/cli/`; golden output snapshots; `REGENERATE_GOLDENS=1` once and inspect.
- `[acceptance]` #3 (propose/edit/reject loop), #4, #7 (Gate-1 amend), #5b/#5c via `--divergence` path.

### W1.2 — Plan lifecycle collapses to `pending → reviewed` `[step 14]`

- Repo: `cli`
- Files: `crates/domain/src/plan.rs` (or equivalent), `cmd.rs`, error variants, schema enum (already shipped in W0.1, double-check), fixtures.
- Logic: drop every code, enum, error, fixture, and doc reference to plan-level `in-progress` and `drained`. `specify plan transition` accepts plan-level `reviewed` only. `plan next` is the only writer of per-entry `in-progress`. `/spec:plan` must not be capable of writing `reviewed` (CLI-level check: `specify plan transition reviewed` exits non-zero when run without a controlling tty and `SPECIFY_OPERATOR=1` env, or some equivalent guard the skill body documents — alternatively, leave this purely a skill-body responsibility and the CLI happily accepts whoever calls it; choose the lighter-touch path and document it).
- Tests: fixtures for legal/illegal transitions; "drained = all entries done" computed at read time.
- `[acceptance]` #1 (operator stamps reviewed explicitly), #7 (re-entry to Gate 1).
- Subagent notes: this is load-bearing for the workflow collapse. Coordinate with W3.2 (`/spec:plan` skill) which prints the literal `specify plan transition <name> reviewed` hint.

### W1.3 — `spec.md` provenance parser `[step 6]`

- Repo: `cli`
- Files: new module under `crates/domain/src/spec/` (or wherever spec parsing lives), consumed by `specify slice validate`.
- Logic: parse requirement blocks for `ID:`, `Sources:`, `Status:`; validate `Status ∈ {agreed, unknown, conflict, divergence}`; validate `Sources:` keys resolve against the slice's plan-level `sources:` bindings.
- Tests: parser unit tests per status/provenance variant from §Worked examples §Per-requirement provenance variants; integration via `specify slice validate` golden runs.
- `[acceptance]` #1 provenance, #5a–#5c provenance lines, #5b `[divergence]` round-trip.

### W1.4 — RFC-19 journal events for extract & synthesis tags `[step 13]`

- Repo: `cli`
- Files: journal event types (RFC-19 module), emit sites in `specify slice merge` / `slice transition` and from the `/spec:refine`-invoked `slice validate`.
- Logic: emit `plan.transition.reviewed`, `plan.propose.divergence`, `plan.amend.divergence`, `slice.transition.refined`, `slice.extract.completed`, `slice.synthesis.{conflict,divergence,unknown}` per §Observability table.
- Tests: journal-event golden snapshots.
- `[acceptance]` #5b–#5d (tag emission), #5e (propose-time divergence event).

## Wave 2 — Source adapters and target brief migration (fully parallel after Wave 0)

Seven parallel subagents. The four source adapters in `plg/sources/` and the three target brief migrations in `plg/targets/` touch disjoint trees.

### W2.1 — `sources/intent/` source adapter `[step 4]`

- Repo: `plg`
- Files: `sources/intent/adapter.yaml`, `sources/intent/briefs/{enumerate,extract}.md`, schemas folder symlink/reference.
- Logic: degenerate `enumerate` (one candidate per operator brief), trivial `extract` writing a one-claim `Evidence` with `authority: intent` and `kind: intent`.
- Tests: fixture run from the `cli`-side acceptance harness (`specify source resolve intent`) once W0.3 is in.
- `[acceptance]` #1 release-blocker.

### W2.2 — `sources/documentation/` source adapter `[step 4]`

- Repo: `plg`
- Files: `sources/documentation/adapter.yaml`, `sources/documentation/briefs/{enumerate,extract}.md`.
- Logic: walk a bound directory, enumerate one candidate per top-level concept (per existing 1.x doc-source rules), extract structured claims (`requirement`, `criterion`, `decision`, `section`) with `authority: documentation`.
- Tests: fixture under `tests/fixtures/sources/documentation/`.
- `[acceptance]` #2.

### W2.3 — `sources/code-typescript/` source adapter `[step 4]`

- Repo: `plg`
- Files: `sources/code-typescript/adapter.yaml`, `sources/code-typescript/briefs/{enumerate,extract}.md`. Rehome the body of the retired `change survey` TypeScript enumerator + extract logic.
- Logic: enumeration grammar stays adapter-internal (no surfaces.json sibling artifact); `extract` emits `excerpt` / `type` / `call` claims with `authority: behaviour`.
- Tests: TypeScript fixture repo under `tests/fixtures/sources/code-typescript/` mirroring a small legacy service.
- `[acceptance]` #4.

### W2.4 — `sources/screenshots/` source adapter (Vectis split) `[step 4]`

- Repo: `plg` (plus a tiny schema confirmation in `cli`)
- Files: new `sources/screenshots/adapter.yaml`, `sources/screenshots/briefs/{enumerate,extract}.md` (briefs lift verbatim from the retired `plugins/vectis/skills/image-layout-inferer/` SKILL body, resliced into the two source-adapter operations). Retire the skill folder. Confirm `region` / `container` / `leaf` claim kinds in `schemas/evidence.schema.json` (landed by W0.1).
- Logic: vision-assisted enumeration of candidate screens; spatial extraction per candidate.
- Tests: golden screenshot fixtures preserved from the inferer SKILL's existing acceptance harness.
- `[acceptance]` covered indirectly via Vectis target build (W2.6) consuming spatial Evidence; no dedicated #-scenario in v1.
- Subagent notes: do **not** touch baseline `layout.yaml` here — that retirement is part of W5 migration.

### W2.5 — `targets/omnia/` brief migration `[step 9]`

- Repo: `plg`
- Files: `targets/omnia/adapter.yaml` (renamed from `adapters/omnia/adapter.yaml`), `targets/omnia/briefs/{shape,build,merge}.md`. Move `plugins/omnia/skills/{crate-writer,test-writer,guest-writer,code-reviewer}/SKILL.md` bodies into briefs where they accompany `build` / `merge` invocations rather than direct skill calls (the `/spec:build` skill orchestrates them).
- Logic: target adapter no longer owns `spec.md` / `design.md` synthesis; `shape` brief carries the idiom guidance core synthesis consumes (provider DI patterns, WASM guardrails, error variant rules).
- Tests: synthesis golden runs with `shape` injected; cargo-check on the generated crate fixture.
- `[acceptance]` #5h (target `shape` injection).

### W2.6 — `targets/vectis/` brief migration `[step 9]`

- Repo: `plg`
- Files: `targets/vectis/adapter.yaml`, `targets/vectis/briefs/{shape,build,merge}.md`. The `build` brief gains responsibility for regenerating `composition.yaml` from synthesised `spec.md` + `design.md` (per §Target-specific structured outputs); `tokens.yaml` and `assets.yaml` remain operator-curated, consumed by `build`.
- Logic: vectis target stays three-capability (no fourth `refine` slot).
- Tests: fixture covering a Vectis slice that exercises `screenshots`-sourced spatial Evidence → spec.md + design.md → composition.yaml regen.
- `[acceptance]` #5h.
- `requires`: W2.4 (screenshots) for the end-to-end fixture, but the brief authoring itself does not.

### W2.7 — `targets/contracts/` brief migration `[step 9]`

- Repo: `plg`
- Files: `targets/contracts/adapter.yaml`, `targets/contracts/briefs/{shape,build,merge}.md`. Lift `plugins/contract/skills/{openapi,asyncapi,json-schema}/SKILL.md` bodies into the appropriate briefs.
- Logic: same three-capability shape; `build` runs the `contract` WASI tool per RFC-12; `merge` runs the post-merge baseline gate.
- Tests: existing contract validation fixtures under `tests/fixtures/contracts/` should pass unchanged.
- `[acceptance]` #5h (variant against contracts).

## Wave 3 — Synthesis library and `/spec:*` skill bodies (parallel after Wave 2)

Five parallel subagents in `plg`. All depend on Wave 0 (schemas + types) and Wave 1 (CLI verbs + parser); W3.1 must finish before any other change in this wave can land its end-to-end test, but body authoring is independent.

### W3.1 — Core synthesis library + `/spec:refine` SKILL `[step 5]`

- Repo: `plg`
- Files: new `plugins/spec/references/synthesis/` (synthesis playbook for the agent — substep order, authority hierarchy, tag grammar, requirement-block templates), new `plugins/spec/skills/refine/SKILL.md` (renamed from `define/`, plan-resolved sources, serial `extract` per binding).
- Logic: pipeline per §`/spec:refine` pipeline — resolve target+sources → `specify slice create` → serial `extract` → synthesize (proposal → specs → design → tasks) → `specify slice validate` → transition `refined`. Tags `[unknown]`, `[conflict]`, `[divergence]` are review signals; lifecycle stays `refining → refined`.
- Tests: golden synthesis runs across §Worked examples; spec.md provenance lines parse via W1.3.
- `[acceptance]` #5, #5a–#5c, #5e, #5f, #5g, #5h, #5j.

### W3.2 — `/spec:plan` SKILL + `propose` sub-step + `discovery.md` form `[steps 7, 11]`

- Repo: `plg`
- Files: new `plugins/spec/skills/plan/SKILL.md` (from `change/draft`), discovery template under `plugins/spec/references/discovery.md`.
- Logic: pre-flight → scaffold `change.md` + `plan.yaml` → workspace registry validate + `workspace sync` when needed → enumerate each source → write `discovery.md` (Summary, Source inventory, Candidate inventory) → run `propose` (agent-driven candidate fusion, `tentative: true` annotations + `## Tentative merges` block in `change.md`, `slices[].divergence: likely` on materially-disagreeing summaries + `## Likely divergences` block) → assign workspace projects when needed → validate → exit at `pending` with literal `specify plan transition <name> reviewed` hint.
- Tests: scenario goldens for #1, #3, #5e.
- `[acceptance]` #1, #3, #5e, #6 (workspace), #7, #12 (dual-driving refused).
- `requires`: W1.1 (`plan amend --add-source`), W1.4 (journal events).

### W3.3 — `/spec:execute` SKILL + plan-lock semantics `[step 15]`

- Repo: `plg`
- Files: new `plugins/spec/skills/execute/SKILL.md` (from `change/execute loop`).
- Logic: refuse unless `reviewed`; acquire `.specify/plan.lock` (workspace-root in workspace mode); loop `plan next` → workspace project resolution + `workspace sync` of active slot when needed → `/spec:refine` if needed → `/spec:build` → `/spec:merge` → residue commit + return to workspace root when needed → exit when no `pending` / `in-progress` per-entry remains. Stop on build-non-zero / merge-conflict with structured hint.
- Tests: scenario goldens for #8, #9, #10, #11.
- `[acceptance]` #8, #9, #10, #11.
- `requires`: W1.2 (lifecycle collapse), W3.1 (refine), W3.4 (build/merge), W3.5 (finalize).

### W3.4 — `/spec:build` + `/spec:merge` SKILLs `[step 5]`

- Repo: `plg`
- Files: `plugins/spec/skills/build/SKILL.md` (rewrite — plan-resolved active slice, refuse only on slice lifecycle not on synthesis tags), `plugins/spec/skills/merge/SKILL.md` (rewrite — only writer of per-entry `done`, runs `specify slice merge`).
- Logic: shared bodies with the loop (`/spec:execute` invokes the same skill files); operate on the active `in-progress` entry from `plan next`.
- Tests: build-failure replay + merge-conflict replay from §Execution model table.
- `[acceptance]` #9, #11.

### W3.5 — `/spec:finalize` SKILL `[step 5]`

- Repo: `plg`
- Files: `plugins/spec/skills/finalize/SKILL.md` (from `change/finalize`).
- Logic: require all per-entry `done`; push branches; observe PRs to `MERGED`; run `specify plan finalize` to archive.
- Tests: PR-observation harness mock + archive-move integration fixture.
- `[acceptance]` covered by #1 end-to-end and #10 multi-project finalize.

## Wave 4 — Documentation (parallel after Wave 3)

Two parallel subagents. Pure docs, no code paths.

### W4.1 — Repo docs refresh `[step 10]`

- Repo: `plg`
- Files: `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/explanation/decision-log.md`, `docs/explanation/adapter-anatomy.md` (new — formalises the source/target split).
- Logic: rewrite for `/spec:plan` → `/spec:execute` → `/spec:finalize` rhythm, `source` / `target` vocabulary, Gate 1 = operator stamps `reviewed`, change vs slice vocabulary intact.
- `[acceptance]` documentation-review pass; `make checks`.

### W4.2 — CLI docs refresh

- Repo: `cli`
- Files: `AGENTS.md`, `DECISIONS.md`, `docs/standards/handler-shape.md`, `docs/standards/architecture.md`, regenerate `target/man/` via `cargo make xtask gen-man`.
- Logic: update for renamed types (W0.2), new verbs (W1.1), collapsed lifecycle (W1.2), per-axis cache layout (W0.3).
- `[acceptance]` `cargo make ci` green; man-page snapshot updated.

## Wave 5 — Cutover (sequential, lands last)

Three changes in strict order. The migration script ships in `plg` but is exercised against `cli` fixtures.

### W5.1 — `migrate-to-2.0.sh`

- Repo: `plg`
- Files: `scripts/migrate-to-2.0.sh`, accompanying `docs/migration/2.0.md`, fixture under `tests/fixtures/migration/1.x/`.
- Logic: rename `adapters/` → `targets/`; rewrite `project.yaml` (`adapter` → `target`, `specify_version` → `specify-version`, `hub:` → `workspace:` where applicable); rewrite `registry.yaml`, `plan.yaml`, `sources.yaml`, cache, archive fields; rewrite `plan.yaml.slices[].sources` from 1.x form into structured `{ key, candidate }[]` lifting any standalone `slices[].candidate`; move retired skills (`plugins/vectis/skills/image-layout-inferer/` → `sources/screenshots/`); retire baseline `layout.yaml` paths and warn when an existing `composition.yaml` is found; bump `specify-version` to `2.0.0`; add plan-lifecycle `reviewed` on first read where appropriate.
- Tests: dry-run against a 1.x fixture, diff golden; idempotent re-run.
- `requires`: every preceding wave landed.

### W5.2 — Delete `/change:*` and `/spec:define`, `/spec:extract` `[step 17]`

- Repo: `plg`
- Files: delete `plugins/change/`, `plugins/spec/skills/define/`, `plugins/spec/skills/extract/`, retire matching `.cursor-plugin/` manifest entries; delete `plugins/vectis/skills/image-layout-inferer/` (already lifted in W2.4 but the directory may remain until cutover).
- Logic: removal-only commit.
- `requires`: W5.1 (migration script in operators' hands first).

### W5.3 — Acceptance scenario sweep + release tag

- Repo: both
- Files: run the §Acceptance scenarios matrix (#1–#12) end-to-end against the merged repos; capture goldens; tag `specify-cli v2.0.0` and `specify` 2.0.0 release.
- Logic: blocker is scenario #1 (pure intent, one slice) per §Implementation plan release-blocker rule.
- `requires`: W5.1, W5.2.

## Parallelism quick-reference

| Wave | Parallel slots                        | Total subagents in wave |
|------|---------------------------------------|-------------------------|
| 0    | W0.1 → W0.2 → W0.3 (sequential)       | 3                       |
| 1    | W1.1, W1.2, W1.3, W1.4                | 4 parallel              |
| 2    | W2.1, W2.2, W2.3, W2.4, W2.5, W2.6, W2.7 | 7 parallel           |
| 3    | W3.1, W3.2, W3.3, W3.4, W3.5          | 5 parallel              |
| 4    | W4.1, W4.2                            | 2 parallel              |
| 5    | W5.1 → W5.2 → W5.3 (sequential)       | 3                       |

Total: 24 subagent-sized changes. Critical path (sequential length): W0.1 → W0.2 → W0.3 → (any W1) → (any W2) → W3.1 → W3.3 → W5.1 → W5.2 → W5.3 = ten serial steps; everything else parallelises into the four mid-plan waves.

## Subagent dispatch notes

- Each change above is intended to be the prompt body for one subagent (`generalPurpose` or `shell` agent type depending on whether tests need to run). Include the RFC reference, the file list, the acceptance scenario id, and the `requires:` line verbatim so the subagent reads `rfc-25-workflow.md` for its source of truth and does not re-derive scope.
- Renames in Wave 0 are cross-cutting and benefit from one focused subagent per rename; do not bundle W0.2 with W0.3 even though both touch domain modules — the rename's blast radius (fixtures, errors, JSON envelope) is large enough on its own.
- Wave 2's seven slots are deliberately split per adapter even where they share boilerplate; this is the cheapest way to keep each adapter's brief revision in a single subagent's context window.
- Wave 3 is the only wave where parallel slots have intra-wave dependencies for end-to-end testing: W3.3 (`/spec:execute`) cannot finish its goldens until W3.1, W3.4, and W3.5 have landed. Body authoring of all five skills can still proceed in parallel; the goldens just need to be regenerated after W3.4/W3.5 merge.
- Wave 5 is sequential and must run after **every** other wave is in to avoid migrating against a moving target.
