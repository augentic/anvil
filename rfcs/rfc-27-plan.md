# RFC-27 Implementation Plan

> Subagent decomposition for [rfc-27-synthesis.md](rfc-27-synthesis.md). Target release: **Specify 2.1.0** across `augentic/specify-cli` (`cli`) and `augentic/specify` (`plg`). Every change is additive; no migration script.

## How to use this plan

Each **change** is sized for one subagent session (~1–3 hours, bounded file set). Changes list:

- **Repo** — `cli` or `plg`
- **Depends on** — must land first
- **Parallel with** — safe to run concurrently in separate subagents once dependencies are met
- **Acceptance** — scenario ids from RFC-27 §Acceptance scenarios
- **Done when** — concrete exit criteria

Legend for dependency diagrams:

```text
[A] ──► [B]     B depends on A
[A] ║ [B]       A and B can run in parallel
```

---

## Overview

```text
Phase 0 ──► Phase 1 ──► Phase 2 (6 parallel tracks) ──► Phase 3 (4 parallel tracks) ──► Phase 4 ──► Phase 5
 schemas     types        CLI features                    plugin bodies              e2e        docs
```

| Phase | Goal | Blocking? |
| --- | --- | --- |
| 0 | JSON Schema deltas | Yes — everything else validates against these |
| 1 | Domain types + schema embed | Yes — CLI handlers consume these |
| 2 | CLI verbs and validation | Partial — plugin skill rewrites need matching CLI |
| 3 | Source adapter, skills, synthesis docs | Partial — acceptance fixtures need adapter body |
| 4 | Golden fixtures + cross-repo tests | Yes — release gate |
| 5 | Operator docs + DECISIONS.md | No — can start after Phase 2 tracks complete |

**Release blockers:** acceptance scenarios **#26-1** and **#26-2** (D1 + D2/D3 authority widening).

---

## Phase 0 — Schema foundation

### Change 0.1 — All JSON Schema deltas

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | — |
| **Parallel with** | — (must complete before Phase 1) |
| **Decisions** | D1, D2, D3, D4, D6, D8 |
| **Acceptance** | unblocks #26-1, #26-3, #26-4, #26-6, #26-8 |

**Files**

| Schema | Change |
| --- | --- |
| `schemas/evidence.schema.json` | Add `example` to claim kind enum; optional top-level `authority-overrides`; open `fixture-digest`, `input`, `output` on claims |
| `schemas/discovery/candidate.schema.json` | Optional `aliases: []` |
| `schemas/plan/plan.schema.json` | Optional `slices[].authority-override`; document `divergence: likely` as CLI-written |
| `schemas/slice/fusion.schema.json` | **New** — `version`, `slice`, `generated-at`, `generator`, `requirements[]` |
| `schemas/adapter.schema.json` | Optional `cache: opt-out` |
| `schemas/source.schema.json` | Mirror optional `cache: opt-out` (axis schema is `additionalProperties: false`) |
| `schemas/target.schema.json` | Mirror optional `cache: opt-out` (axis schema is `additionalProperties: false`) |

**Done when**

- Every existing golden fixture under `tests/fixtures/` still validates unchanged
- New schema files referenced from `crates/domain/src/schema.rs` (`include_str!`)
- `cargo make check` schema-validation tests pass (or equivalent unit tests on embedded schemas)

---

## Phase 1 — Domain types

### Change 1.1 — Rust domain types for RFC-27

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 0.1 |
| **Parallel with** | — |
| **Decisions** | D1–D4, D6, D8 |
| **Acceptance** | unblocks #26-1, #26-3, #26-4, #26-6, #26-8 |

**Files (new modules)**

| Type | Module |
| --- | --- |
| `ExampleClaim` | `crates/domain/src/evidence/claim/example.rs` (or extend existing claim module) |
| `AuthorityOverrides` | `crates/domain/src/evidence/authority.rs` |
| `SliceAuthorityOverride` | `crates/domain/src/change/plan/core/model.rs` |
| `FusionIndex`, `FusionRequirement`, `FusionResolution` | `crates/domain/src/slice/fusion.rs` |
| `CandidateAliases` | `crates/domain/src/discovery/candidate.rs` |
| `CacheFingerprint`, `CacheIndexEntry`, `SourceOperation` | `crates/domain/src/adapter/cache.rs` — `SourceOperation` is the source-axis (`enumerate \| extract`) sibling to the existing target-axis `adapter::Operation` (`shape \| build \| merge`); cache index entries key against the source set |
| `CacheMode` (Off / OptOut) on `Adapter` struct | `crates/domain/src/adapter/core.rs` — adapter manifest is `#[serde(deny_unknown_fields)]`, so the schema field needs a matching optional struct field or `cache: opt-out` deserialization will fail |
| `EventKind::SliceExtractCacheHit`, `::SliceExtractCacheMiss` | `crates/domain/src/journal.rs` |
| `EventKind::SliceFusionWritten`, `::SliceFixtureReplayCompleted`, `::PlanAmendAuthorityOverride` | `crates/domain/src/journal.rs` (§Observability) |

**Also update**

- `src/output.rs` — new error discriminants → exit code 2: `slice-authority-override-orphan-source-key`, `slice-fusion-drift`, `discovery-alias-collision`; exit code 1: `code-runtime-fixture-format-invalid`. Per the Diag-first policy in `DECISIONS.md` §Error variants, route these through `Error::validation_failed` / `Error::Diag` rather than minting typed `Error::*` variants until the codebase needs destructured payloads or non-default exit mapping.
- `DECISIONS.md` — stub rows (filled in Change 5.2)

**Done when**

- Serde round-trip unit tests for each new type
- Journal golden tests extended for new event wire shapes
- `rg` across both repos for deferred symbols finds no stale references

---

## Phase 2 — CLI features (parallel tracks)

All Phase 2 changes depend on **1.1**. They do **not** depend on each other unless noted.

```text
                    ┌── 2.1 Auto-review (D7) ──────────────┐
                    ├── 2.2 Divergence likely (D5) ────────┤
Phase 1 ──►         ├── 2.3 Authority override (D3) ───────┼──► Phase 3 skill rewrites
                    ├── 2.4 Candidate aliases (D6) ────────┤
                    ├── 2.5 Cache fingerprints (D8) ───────┤
                    └── 2.6 Fusion validate/show (D4) ───┘
```

### Change 2.1 — `specify plan create --auto-review` (D7)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.2, 2.3, 2.4, 2.5, 2.6 |
| **Acceptance** | #26-7 |

**Files**

- `src/commands/plan/create.rs` — `--auto-review` flag
- `src/commands/plan/cli.rs` — clap wiring
- `crates/domain/src/change/plan/core/create.rs` — atomic `lifecycle: reviewed` write
- `crates/domain/src/journal.rs` — single append with `plan.create` + `plan.transition.reviewed`

**Done when**

- N=1 intent, N=1 path-bound, and N>1 multi-slice creates all exit at `reviewed`
- Validation failure refuses create with or without flag
- Post-create `specify plan transition <name> reviewed` is a no-op
- Integration test in `tests/plan*.rs`

---

### Change 2.2 — CLI owns `divergence: likely` (D5)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.1, 2.3, 2.4, 2.5, 2.6 |
| **Acceptance** | #26-5 |

**Files**

- `crates/domain/src/change/plan/core/model.rs` — remove "likely reserved for skill" restriction on amend path
- `crates/domain/src/change/plan/core/amend.rs` — accept `--divergence likely`
- `src/commands/plan/create.rs` — `--divergence-likely <slice>` on create
- `src/commands/plan/cli.rs` — clap wiring
- `crates/domain/src/journal.rs` — `plan.propose.divergence` / `plan.amend.divergence` from CLI only

**Done when**

- `specify plan create … --divergence-likely <slice>` and `specify plan amend … --divergence likely` persist the field
- Skill-side YAML patch path documented in error message if something tries the old route
- Journal event fires once per CLI write

---

### Change 2.3 — Per-slice `authority-override` (D3)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.1, 2.2, 2.4, 2.5, 2.6 |
| **Acceptance** | #26-2, #26-3 |

**Files**

- `src/commands/plan/cli.rs`, `create.rs`, `amend.rs` — flags:
  - `--authority-override <slice> <kind>=<source-key>` (create + amend)
  - `--clear-authority-override`, `--clear-authority-overrides`
  - `plan add --authority-override <kind>=<key>` (repeatable)
- `crates/domain/src/change/plan/core/validate.rs` — orphan source-key check → `slice-authority-override-orphan-source-key`
- `src/commands/slice/` — `slice validate` invokes orphan check
- `crates/domain/src/spec/provenance.rs` — informational override trace on `Status: divergence` blocks
- `crates/domain/src/journal.rs` — `plan.amend.authority-override` event

**Done when**

- Orphan key rejected at exit code 2 before refine
- Round-trip: set override via amend, re-read plan.yaml, validate passes
- Unit test for resolution order (override → per-Evidence → default → conflict)

---

### Change 2.4 — Candidate aliases (D6)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.1, 2.2, 2.3, 2.5, 2.6 |
| **Acceptance** | #26-6 |

**Files**

- `crates/domain/src/discovery/candidate.rs` — alias resolution (`id` then `aliases[]`)
- `crates/domain/src/change/plan/core/amend.rs` — `--add-alias`, `--remove-alias`
- `src/commands/plan/add.rs` — `--sources <key>=<id-or-alias>` rewrites to canonical `id`
- New or extended: `src/commands/discovery/show.rs` — `--aliases` flag
- Validation: `discovery-alias-collision` when alias collides with another candidate's `id` or alias

**Done when**

- `specify plan add --sources legacy=password-reset` resolves alias → canonical id in persisted plan
- `specify discovery show --aliases` prints alias map
- Operator-added aliases survive re-enumeration (amend path preserves them)

---

### Change 2.5 — Cache fingerprints (D8)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.1, 2.2, 2.3, 2.4, 2.6 |
| **Acceptance** | #26-8 |

**Files**

- `crates/domain/src/adapter/cache.rs` — fingerprint computation (5 inputs, order-stable)
- Extract code path (locate via `rg "extract"` in `src/commands/` and domain adapter modules) — cache hit/miss lookup + write
- `.specify/.cache/sources/<adapter>/index.jsonl` append-only writer
- `src/commands/source/` — `--explain <adapter>` reads index log
- `crates/domain/src/journal.rs` — `slice.extract.cache-hit` / `.cache-miss` with `reason` enum
- Honor `cache: opt-out` on `adapter.yaml` → always miss, `reason: adapter-opt-out`

**Done when**

- Two consecutive extracts with unchanged inputs → hit then hit
- Adapter version bump → miss with `reason: adapter-version-changed`
- `index.jsonl` has one row per cache write
- `specify source resolve --explain` prints fingerprint chain

---

### Change 2.6 — Fusion index CLI surface (D4, CLI half)

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 2.1, 2.2, 2.3, 2.4, 2.5 |
| **Acceptance** | #26-4 |

**Files**

- `crates/domain/src/slice/fusion.rs` — parse/validate helpers, drift detection
- `src/commands/slice/fusion.rs` (new) — `specify slice fusion show <slice> [--format text|json]`
- `src/commands/slice/cli.rs` — subcommand wiring
- Slice validate path — `spec.md` ↔ `fusion.yaml` REQ id parity; contributing-claim → evidence claim resolution → `slice-fusion-drift`
- `crates/domain/src/journal.rs` — `slice.fusion.written` (emit when validate sees fresh fusion file, or on dedicated write path)

**Done when**

- Hand-edited `spec.md` without matching fusion entry → validate exit 2
- Hand-edited fusion contributing-claim pointing at missing evidence claim → exit 2
- `fusion show` prints inline `value` payloads for human review
- Golden test: sample `fusion.yaml` round-trips schema validation

**Note:** Agent-side `fusion.yaml` authoring lives in Change 3.2 (refine skill). CLI owns validation and inspection only unless a future `specify slice fusion write` verb is added to enforce single-writer — follow RFC writer-ownership table and wire skill to atomic file write + validate.

---

## Phase 3 — Plugin repo bodies (parallel tracks)

Phase 3 depends on **Phase 0** (schemas). Skill rewrites additionally depend on their matching **Phase 2** CLI track.

```text
Phase 0 ──► 3.1 code-runtime adapter (D1) ──► 3.4 RT/Omnia replay hook
         ║
         ├── 3.2 refine skill + fusion.md (D4) ── depends on 2.6
         ├── 3.3 plan skill rewrite (D5) ─────── depends on 2.2
         └── 3.5 authority.md amend (D2,D3) ──── depends on 2.3 ( prose only; can start early )
```

### Change 3.1 — `sources/code-runtime/` adapter (D1)

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 0.1 |
| **Parallel with** | 3.5; also 3.2/3.3 once their CLI deps land |
| **Acceptance** | #26-1 |

**Files**

- `sources/code-runtime/adapter.yaml`
- `sources/code-runtime/briefs/enumerate.md` — one candidate per handler entry point
- `sources/code-runtime/briefs/extract.md` — `kind: example` claims; 64 KiB inline cap; `fixture-digest: sha256:…`
- Reference: `plugins/rt/skills/replay-writer/references/fixture-format.md`

**Done when**

- `make checks` passes (adapter manifest validates against `source.schema.json`)
- Manual dry-run: enumerate + extract against a sample fixture tree produces valid Evidence YAML
- Brief documents binding grammar (`plan.yaml.sources.runtime.path`)

---

### Change 3.2 — Refine skill + `fusion.md` (D4, agent half)

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 2.6, 3.5 (authority rules) |
| **Parallel with** | 3.3, 3.4 (after deps met) |
| **Acceptance** | #26-4 |

**Files**

- `plugins/spec/skills/refine/SKILL.md` — insert step 5: write `fusion.yaml` atomically between tasks and validate; renumber validate → 6, transition → 7
- `plugins/spec/references/synthesis/fusion.md` — **new** reconciliation index playbook
- `plugins/spec/references/synthesis/README.md` — link new page
- `plugins/spec/references/synthesis/claim-fusion.md` — note `example` claim kind from `code-runtime`

**Done when**

- Skill body references `fusion.md` for resolution enum values and inline `value` truncation rules
- Fixture under `plugins/spec/skills/refine/fixtures/` (optional) documents expected fusion shape
- `make checks` skill schema predicates pass

---

### Change 3.3 — Plan skill rewrite (D5)

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 2.2 |
| **Parallel with** | 3.2, 3.4 |
| **Acceptance** | #26-5 |

**Files**

- `plugins/spec/skills/plan/SKILL.md` — step 3 (`divergence: likely`): replace YAML patch with `specify plan amend <name> <slice> --divergence likely`; journal event via CLI
- Remove guardrail exception for direct `plan.yaml` edit (single-writer restored)
- Update `plugins/spec/skills/plan/fixtures/` if golden transcripts reference old path

**Done when**

- Skill prose contains zero instructions to hand-edit `plan.yaml` for divergence
- `make checks` passes

---

### Change 3.4 — RT replay → Omnia build hook (D1, target half)

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 3.1 |
| **Parallel with** | 3.2, 3.3 |
| **Acceptance** | #26-1 |

**Files**

- `plugins/rt/skills/replay-writer/SKILL.md` — thin wrapper pointing at code-runtime extract + Omnia build hook
- `targets/omnia/briefs/build.md` — optional fixture-replay step; write `fixture-replay:` block to `.metadata.yaml`
- `plugins/spec/skills/merge/SKILL.md` (if exists) or merge CLI hint — surface one-line replay summary when block present
- CLI side (may spill to **2.x** follow-up): `merge` closing message reads `fixture-replay` from metadata

**Done when**

- Omnia build brief documents optional hook; omission is not an error
- `merge` message format documented in RFC-27 worked example
- Journal event `slice.fixture-replay.completed` documented for target implementers

---

### Change 3.5 — Synthesis authority docs (D2 + D3)

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 0.1 (schema names); **soft** dependency on 2.3 for worked examples |
| **Parallel with** | 3.1 |
| **Acceptance** | #26-2, #26-3 |

**Files**

- `plugins/spec/references/synthesis/authority.md` — remove line-105 "deferred" note; add per-kind Evidence overrides + per-slice plan overrides; resolution order
- `plugins/spec/references/synthesis/claim-fusion.md` — `example` kind defaults to behaviour-class

**Done when**

- `make checks` documentation link predicates pass
- Resolution order matches RFC-27 §Authority widening verbatim

---

## Phase 4 — Acceptance fixtures and tests

### Change 4.1 — Plugin golden fixtures

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 3.1, 3.4 |
| **Parallel with** | — |
| **Acceptance** | #26-1 |

**Files**

- `tests/fixtures/sources/code-runtime/` — fixture tree, expected Evidence YAML, sample `fusion.yaml`
- `tests/fixtures/targets/omnia/` — one target with fixture-replay block, one without

**Done when**

- Fixtures validate against updated schemas when driven through `make test` harness

---

### Change 4.2 — Cross-repo acceptance tests

| | |
| --- | --- |
| **Repo** | `cli` (primary); consumes `plg` fixtures |
| **Depends on** | 2.1–2.6, 3.1–3.4, 4.1 |
| **Parallel with** | 5.x docs |
| **Acceptance** | #26-1 … #26-8 (all) |

**Files**

- `tests/` — one integration test module per scenario (or extend `tests/cross_repo.ts` driver in `plg`)
- Map each `#26-N` to a deterministic assertion

**Scenario checklist**

| Scenario | Assert |
| --- | --- |
| #26-1 | code-runtime enumerate/extract; Sources line includes runtime; fixture-replay metadata optional |
| #26-2 | per-slice override; fusion resolution-trace `per-slice-authority-override` |
| #26-3 | Evidence `authority-overrides`; fusion records both resolution paths |
| #26-4 | fusion round-trip; drift detection; re-refine clears drift |
| #26-5 | CLI-only divergence likely; journal event count |
| #26-6 | alias resolution; canonical id persisted |
| #26-7 | auto-review on three plan shapes |
| #26-8 | cache hit/miss journal + index.jsonl rows |

**Done when**

- `SPECIFY_BIN=… make test` passes in `plg` repo
- `cargo make ci` passes in `cli` repo

---

## Phase 5 — Documentation (parallel)

Can start once the corresponding Phase 2/3 change merges. All of Phase 5 can run in parallel.

```text
5.1 AGENTS.md (plg) ║ 5.2 DECISIONS.md (cli) ║ 5.3 migration/2.1.md (plg) ║ 5.4 project.mdc (plg)
```

### Change 5.1 — `AGENTS.md` vocabulary

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 3.1, 3.2 (soft) |
| **Parallel with** | 5.2, 5.3, 5.4 |

Add paragraphs: `code-runtime`, `fusion.yaml`, per-slice `authority-override`, cache fingerprints.

---

### Change 5.2 — `DECISIONS.md` rows

| | |
| --- | --- |
| **Repo** | `cli` |
| **Depends on** | 1.1 |
| **Parallel with** | 5.1, 5.3, 5.4 |

Four rows: per-kind authority on Evidence, per-slice authority on plan, fusion.yaml audit-only posture, cache fingerprint inputs.

---

### Change 5.3 — Migration note

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | Phase 2 complete (soft) |
| **Parallel with** | 5.1, 5.2, 5.4 |

**Files:** `docs/migration/2.1.md` — additive upgrade path, no script, opt-in features listed.

---

### Change 5.4 — Cursor rules + docs index

| | |
| --- | --- |
| **Repo** | `plg` |
| **Depends on** | 3.1 |
| **Parallel with** | 5.1, 5.2, 5.3 |

**Files**

- `.cursor/rules/project.mdc` — add `code-runtime` to source adapter list
- `docs/SUMMARY.md` — link migration/2.1.md if not already present

---

## Subagent dispatch cheat sheet

Use this table to assign work. **Never start a row until its "Depends on" column is merged.**

| Change | Repo | Depends on | Parallel group |
| --- | --- | --- | --- |
| 0.1 | cli | — | — |
| 1.1 | cli | 0.1 | — |
| 2.1 | cli | 1.1 | CLI-A |
| 2.2 | cli | 1.1 | CLI-A |
| 2.3 | cli | 1.1 | CLI-A |
| 2.4 | cli | 1.1 | CLI-A |
| 2.5 | cli | 1.1 | CLI-A |
| 2.6 | cli | 1.1 | CLI-A |
| 3.1 | plg | 0.1 | PLG-A |
| 3.5 | plg | 0.1 | PLG-A |
| 3.3 | plg | 2.2 | PLG-B |
| 3.2 | plg | 2.6, 3.5 | PLG-B |
| 3.4 | plg | 3.1 | PLG-B |
| 4.1 | plg | 3.1, 3.4 | — |
| 4.2 | cli+plg | 2.*, 3.*, 4.1 | — |
| 5.1–5.4 | both | soft | DOC |

**Maximum parallelism:** after **1.1** lands, dispatch up to **6 CLI subagents** (2.1–2.6) and **2 plugin subagents** (3.1, 3.5) simultaneously.

**Known sequencing constraint inside CLI-A:** 2.1, 2.2, 2.3, 2.4 all touch `src/commands/plan/{cli,create,amend}.rs` and overlap; 2.3 and 2.6 both extend `src/commands/slice/validate.rs`. Run those four serially in one worktree or use isolated worktrees; 2.5 and 2.6 are non-overlapping with the plan-command set and can run in parallel with the serial round.

**Pre-existing `make checks` breakage in `plg`:** 28 broken-link failures from RFC-25 archive paths in `tests/cross-repo/runs/2.0.0/*.md`, `tests/fixtures/{skills/execute,targets/vectis}/README.md`, `docs/contributing/index.md`, and `plugins/spec/skills/plan/fixtures/README.md`. Pre-existing; plg subagents should diff against a baseline rather than treating `make checks` as a clean signal.

**Deferred:** `wasi-tools/fixture-index` (the WASI tool RFC-27 references for `code-runtime`) is not yet authored. 3.1's `adapter.yaml` declares `- name: fixture-index` alone; `version:` and `declared:` wiring is a follow-up after the WASI tool ships.

---

## Suggested merge order (serial fallback)

If running sequentially with one agent:

1. 0.1 → 1.1
2. 2.2 → 3.3 (smallest vertical slice, unblocks plan skill)
3. 2.1 (N=1 ergonomics — auto-review)
4. 2.3 → 3.5 → 3.2 (authority + fusion)
5. 2.4 (aliases)
6. 2.5 (cache)
7. 3.1 → 3.4 → 4.1 (runtime adapter e2e)
8. 2.6 if not done in step 4
9. 4.2 → 5.*

---

## Out of scope for v2.1 (explicit)

Per RFC-27 §Non-goals — do not implement in any subagent:

- Per-claim authority overrides
- New authority classes beyond `intent | documentation | behaviour`
- Auto-refusal of `merge` on fixture-replay failure
- Graph-of-claims persistence
- Hosted cache index (RM-22)
- Replacing RT wiretapper skill

---

## Verification commands

| Repo | Command |
| --- | --- |
| `cli` | `cargo make ci` |
| `plg` | `make checks` |
| cross-repo | `SPECIFY_BIN=/path/to/specify make test` (see `docs/contributing/acceptance.md`) |

---

## References

- [rfc-27-synthesis.md](rfc-27-synthesis.md) — normative spec
- [rfc-25-workflow.md](archive/rfc-25-workflow.md) — baseline workflow (archived)
- [specify-cli AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — crate conventions
- [docs/contributing/acceptance.md](../docs/contributing/acceptance.md) — cross-repo test setup
