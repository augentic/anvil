# Specify + Specify-CLI Review — Improve & Optimise

_Review date: 2026-06-02. Scope: `augentic/specify` (docs/prompt repo) and `augentic/specify-cli` (Rust workspace). Mode: improve/optimise existing code — no new features._

## Snapshot

| Repo          | Size                                                                                                                              | Health                                                                                                                                                                                              |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify-cli` | ~53k LOC Rust / 355 files. Big crates: `workflow` (19k), `standards` (15.5k), binary `src/` (9.2k), `tool` (3.5k), `model` (2.8k) | Disciplined: typed errors, documented handler shape, `#[expect]` over `#[allow]`, no `TODO/FIXME/HACK`, minimal prod `unwrap`. Debt is structural (megamodules, duplication, mid-flight burn-down). |
| `specify`     | 499 markdown files, ~42k lines prose + YAML adapters/schemas                                                                      | Content is mostly sound but carries drift from recent standards/reconciliation churn: dead `/omnia:*` references, duplicated explanations, stale counts, a few broken links.                                    |

**Where to focus first (highest ROI):**
1. specify-cli: cache the schema validators + fix the swallowed git fetch (correctness/perf quick wins).
2. specify-cli: extract the 922-line `slice/validate.rs` domain logic out of the binary.
3. specify-cli: finish the imperative→declarative lint burn-down (today `specdev lint` double-walks and double-evaluates the framework tree).
4. specify: purge dead `/omnia:*` / `plugins/contract` references and finish the in-flight `update-docs` reconciliation pass.

---

## Part A — specify-cli (Rust)

### Tier 1 — Quick wins (correctness/perf, low risk)

1. **Uncached schema validators (perf).** `compile_synthesis_validator` / `compile_build_report_validator` rebuild a `jsonschema::Registry` and recompile the validator on *every* call. Verified at `crates/workflow/src/schema.rs:183` (and ~291). Hoist to `OnceLock`/`LazyLock` statics. Same pattern repeats across ~18 `validate_*` entry points that all do parse → validate → `err_from_failures`; collapse to one generic `validate_parsed_json(schema, code, rule, content)`.

2. **Swallowed git fetch error (correctness).** `sync.rs:254` ends a `git fetch --depth 1` with `.or(Ok(()))`, converting fetch failure into success and masking stale registry clones in hub/workspace flows. Log + surface a diagnostic or propagate.

3. **Production `expect` in fingerprint/cache serialisation.** `crates/workflow/src/adapter/cache.rs:79` (`canonical_bytes`) and `crates/diagnostics/src/fingerprint.rs:105,125` panic on `serde_json` edge cases for closed-shape data. Replace with infallible encoding or `unwrap_or_else(|_| unreachable!())` with rationale — these couple digest stability to panic policy.

4. **Production `panic!` on static regex compile.** `crates/standards/src/framework/check/tools.rs:95` panics if a retired-helper regex fails to compile. Move to `LazyLock<Regex>` like `framework/check/links.rs:179`.

5. **`crates/util` is phantom.** It does not exist on disk and isn't a workspace member (`Cargo.toml:51-61`); tokei's "0 lines" is a stale scan path. No action beyond ignoring it / fixing any local tooling that references it.

### Tier 2 — Targeted dedup & test debt (medium)

6. **`source/survey.rs` (526) ≈ `source/extract.rs` (489) are near-duplicates.** Parallel two-phase agent/tool/cache/journal flows with duplicated handoff DTOs, cache fingerprinting, and write formatters. Factor a shared `source/op.rs` kernel parameterised by `SourceOperation`. (Both also carry prod `expect` on prep invariants — `survey.rs:485`, `extract.rs:447`.)

7. **Fragmented git subprocess layer.** At least four parallel `Command::new("git")` wrappers with different error types: `registry/branch.rs:144`, `registry/workspace/git.rs:8`, `init/git.rs:50`, `registry/workspace/push/remote.rs`. `cmd.rs`'s `CmdRunner` test seam already exists — make it the single git boundary.

8. **Duplicated project-binding rules in `propose.rs`.** The "explicit project vs auto-bind sole project vs fail" logic appears in both `bind_projects` (`propose.rs:549`) and `resolve_target` (~602). Extract `resolve_project_binding(name, topology)`.

9. **Mirrored adapter manifests.** `SourceAdapter`/`TargetAdapter` in `adapter/core.rs` share ~80% of fields and a verbatim `effective_cache_mode` (`:415` and `:469`). The resolve pipeline is already deduped; extend that to a generic base + axis-specific `briefs` param.

10. **Duplicated lint eval helpers.** `lint/eval/set_coverage.rs` and `set_eq.rs` repeat identical `SOURCE_OPERATIONS`/`TARGET_OPERATIONS` constants and brief-iteration (`:48`/`:58`); `lint/index/agent_teams.rs:73` and `index/symlinks.rs:74` repeat path-rendering. Extract `adapter_briefs.rs` and `index/path_util.rs`.

11. **Schema embed fragmentation + no parity guard.** `crates/schema/src/constants.rs` is the intended single embed hub, but `include_str!` also appears in `adapter/core.rs:60-62`, `standards/rules/parse.rs:54` (same `rule.schema.json`), `tool/validate.rs:14`, `tool/cache.rs:30`. Three test layers (`schema/tests/schemas.rs`, inline `workflow/src/schema.rs`, `workflow/tests/schemas.rs`) overlap with no byte-equality check. Add one CI test asserting every embed == on-disk file; add missing compile tests for `LEAD`/`PROPOSAL`/`TOPOLOGY` and disk-only `cache-meta`/`context-lock` schemas.

12. **Under-tested leaf paths.** `crates/model/src/atomic.rs` (shared writer for all `.specify/*.yaml`) has **zero tests**; `crates/diagnostics` render formatters are only tested indirectly from `standards`; `crates/schema/src/validate.rs` and `error/src/serde_rfc3339*.rs` are untested; `validate_slice` rule modules (`specs.rs`, `cross.rs`, `composition.rs`) are only exercised via goldens.

13. **Golden brittleness.** `tests/plan_orchestrate.rs` is **2,986 lines** and the main `REGENERATE_GOLDENS` churn surface; decompose it. Standards lint pretty-render snapshots are wording-sensitive.

14. **Cargo hygiene.** `clap` is a prod dep of `specify-model` only for two `ValueEnum` derives (`Cargo.toml:17`) — move CLI mapping to the boundary. `sha2` is pulled directly into `specify-tool` despite `specify-digest` (`tool/Cargo.toml:23`, `package.rs:140`). Duplicate dev-deps in `model/Cargo.toml`; missing `rust-version.workspace` on `tool`/`error`.

### Tier 3 — Structural (multi-session)

15. **Extract `slice/validate.rs` (922 LOC) out of the binary.** It holds provenance scanning, seven model-drift gates, catalog drift, ID-grammar, and journal emission — all testable domain logic trapped in `src/runtime/commands/slice/validate.rs` (`collect_model_drift_findings` at `:494`). This violates handler-shape ("no domain logic in the binary") and is the single biggest architecture win. Move to `specify-validate` or `specify-workflow::slice::validate`; leave a thin `ctx.write` + `validation_failed` handler.

16. **Finish the imperative→declarative lint burn-down.** `specdev lint` currently runs the imperative `framework/check::run` (30 predicates, `framework/check.rs:64`) *and* the declarative CORE rules, and walks the framework tree twice. CORE-001..008 are migrated (parity tests exist); CORE-009 is **dual-implemented** (`lint/eval/namespace_owner.rs` mirrors `framework/check/rules.rs:35`); CORE-010..051 remain fully imperative (`framework/builder.rs:41`). Prioritise migrating predicates that already have CORE ids and parity tests (`skill_body.rs`, `scenarios.rs`), then drop `AuthoringProducer` to roughly halve framework CI cost. (`skill_body.rs:546` also recompiles many regexes per file per check — cache via `LazyLock`.)

17. **Split the megamodules** (each mixes DTOs + kernel + ~40% inline tests):
    - `journal.rs` (1,294) — extract `EventKind` taxonomy to `journal/kinds/`, move wire tests to `tests/`.
    - `change/plan/core/propose.rs` (1,255) — split `wire.rs` / `kernel.rs` / `tests.rs`.
    - `rules/resolve.rs` (1,071) and `resolve/filter.rs` (686).
    - `model/src/discovery/document.rs` (1,246).
    - `adapter/core.rs` (950), `merge/engine.rs`/`merge/slice/read.rs`, `registry/workspace/push.rs` — these four carry `#[expect(clippy::too_many_lines)]`, an explicit "this outgrew one function" signal.

18. **Unify diagnostic/finding types.** Three parallel shapes exist beside the canonical `specify_diagnostics::Diagnostic`: `change/plan/core/model.rs:664` (`Finding`), `change/plan/doctor.rs:46` (`Diagnostic`), `registry/branch.rs:107` (`Diagnostic`). Add adapter mappings to the neutral currency.

19. **Unify lint output path + `specdev`/`specrun` dispatch.** Lint handlers use raw `println!`/`print!` (`runtime/commands/lint/run.rs:95`, `authoring/commands/lint/run.rs:67`) instead of `ctx.write`; `specdev` uses a bespoke `Exit` enum + manual `eprintln!` while `specrun` uses `scoped → Result → report`. Route both through a shared `emit_diagnostic_report` / dispatcher.

20. **Stringly-typed identifiers.** `plan_name`/`slice_name`/`source` are bare `String` across journal, plan, and slice call chains despite strong kebab-case invariants. Introduce `SliceName`/`PlanName` newtypes at module boundaries — cheap, high bug-prevention value.

---

## Part B — specify (docs / skills / adapters)

### Tier 1 — Quick wins

1. **Purge dead `/omnia:*` and `plugins/contract` references.** `plugins/omnia/` and `plugins/rt/` no longer exist and the build skill says those skills are retired (`plugins/spec/skills/build/SKILL.md:34`), yet `<!-- skill: omnia:crate-writer -->` delegation is still taught in `docs/reference/artifact-format.md:241`, `augentic-specify-usage.md:65`, `docs/reference/slice-skills/index.md:27`, and `AGENTS.md:75`. `.cursor/rules/project.mdc:64-69` lists nonexistent `plugins/contract/` and `plugins/references/`.

2. **Fix the spec template in `docs/reference/artifact-format.md:35`.** It shows `Source:` (singular) but the synthesis kernel and validators require the `ID:` / `Sources:` / `Status:` triple (`plugins/spec/references/synthesis/requirement-block.md:3`).

3. **Fix spec-path drift.** `docs/how-to/resolve-spec-conflicts.md:19,28` uses `.specify/slices/<name>/spec.md` while everything else uses `specs/<unit>/spec.md`.

4. **Repair broken RFC links.** `rfcs/roadmap.md` links to `rfc-35-synthesis-determinism.md` without the `done/` prefix where applicable.

5. **Sync the one drifting schema.** `.cursor/schemas/rule.schema.json` still uses an open `kebabToken` for artifact categories while the CLI copy moved to a closed `enum`.

6. **Fix stale counts/status.** `docs/standards/skill-authoring.md:119` says "~29 skills" (actual: 10) and its examples cite a retired `/omnia:code-reviewer` pattern. `tests/fixtures/sources/captures/user-registration/README.md:15` documents `expected/provenance.yaml` though provenance is now inline in `model.yaml`.

### Tier 2 — Consolidation (medium/structural)

7. **One review-team protocol.** `adapters/targets/omnia/references/review-team-protocol.md` is a *divergent copy* of the canonical `docs/reference/review-team-protocol.md` (omnia briefs link the copy; vectis correctly uses the `agent-teams.md` symlink). Collapse to one canonical doc.

8. **Finish the `update-docs` reconciliation pass.** The untracked `docs/explanation/reconciliation.md` and `docs/reference/sources/index.md` are good additions and the slimmed `augentic-specify-usage.md` is healthy — but `concepts.md:145` still omits `captures` from first-party sources and says "one `spec.md`" without `specs/<unit>/`. Land the pass coherently.

9. **De-duplicate explanations.** Authority hierarchy is restated in 10+ adapter reference files (e.g. `adapters/targets/contracts/references/openapi/author.md:14`) instead of linking `plugins/spec/references/synthesis/authority.md`. The workflow rhythm and artifact responsibilities are independently re-explained across `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/explanation/*`, and `docs/reference/*`. Trim `project.mdc:25-50` to links-only for vocabulary/artifacts.

10. **Collapse triple skill docs.** Each phase is documented in `plugins/spec/skills/<x>/SKILL.md`, `docs/reference/slice-skills/<x>.md`, and CLI refs — `docs/reference/slice-skills/refine.md:39` mirrors `refine/SKILL.md:13` almost verbatim. Pick one operator surface; link the others.

11. **Normalise SKILL.md outliers.** `drop` (79 lines, uses `## Steps`), `capture/wiretapper` (81 lines, `## Overview`/`## Process`, no Critical Path), and `client/sow-writer` (no Guardrails, references a dead `define` lifecycle term) diverge from the `skill-authoring.md` house structure.

12. **Wire or remove orphans.** `plugins/spec/skills/merge/delta-merge.md` (101 lines) has zero inbound links (merge uses `merge-runbook.md`). Refresh the captures fixture README.

---

## Part C — Cross-cutting & process

- **CI asymmetry.** `specify`'s CI is a single 15-min `specdev lint` job; finishing the lint burn-down (Part A #16) directly speeds it up. `specify-cli` delegates to a shared org workflow — confirm it runs full `cargo make ci` (clippy `-D warnings`, deny, fmt, nextest) on PRs.
- **Schema is a cross-repo seam.** Workflow schemas live in `specify-cli/schemas` (authoritative), mirrored into `specify/.cursor/schemas`. A parity check (Part A #11) should ideally span both repos, since the only current drift (`rule.schema.json`) is exactly here.
- **Inline tests inflate "real" LOC** across `workflow`, `standards`, and `model` (35–45% of the largest files). Extracting tests to `tests/` modules (the pattern already used by `archive/tests.rs`, `validate/tests.rs`) improves navigation and makes complexity legible.

## Suggested sequencing

1. **Week 1 (correctness/perf quick wins):** A1 (cache validators), A2 (git fetch), A3/A4 (panic removal), B1–B6 (doc quick fixes).
2. **Week 2–3 (dedup + test debt):** A6–A14 — biggest is the survey/extract kernel and the schema parity guard; add `atomic.rs`/render/validate-rule unit tests.
3. **Ongoing (structural):** A15 (extract `slice/validate`), A16 (lint burn-down), A17 (megamodule splits), then B7–B12 doc consolidation.

Do the correctness fixes and validator caching before structural rewrites, so behaviour is pinned by tests before modules move.