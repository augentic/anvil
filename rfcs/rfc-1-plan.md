# Implementation plan: RFC-1 (`specify` CLI) + RFC-1a (Deferred Validation)

> Companion to [rfc-1-cli.md](rfc-1-cli.md) and [rfc-1a-validation.md](rfc-1a-validation.md).

Below is a change-based plan for Phase 1 of RFC-1. Each Change is sized to be deliverable by one sub-agent in a single session. Dependencies are explicit so Changes without shared upstream work can run in parallel.

RFC-1a isn't a separate track — its single architectural decision (`Classification::{Structural, Semantic}` declared at the rule definition site, with `Pass`/`Fail`/`Deferred` outcomes) is folded into Change G where the rule registry is built.

Out of scope for this plan: RFC-2/3 (manifest, federation) beyond stubs; RFC-5 (`specify check`); `verify`/`diff`; orchestrator (RFC-2-orchestrator); release infrastructure beyond CI for PRs.

## Dependency graph

```text
A (workspace + error + CI)
├── B (schema / brief / PipelineView / cache-meta)
├── C (spec parser)
├── E (task parser)
├── F (change lifecycle)
└── H (drift + federation stubs)

C ──► D (merge engine)
B + C + E ──► G (validate + RFC-1a rules)
A + B ──► I (root lib: ProjectConfig, init, CLI plumbing)
I + D + E + F + G + H ──► J (subcommand wiring + e2e tests)
J ──► K (skill migrations)
(independent, any time after A) ──► L (release distribution)
```

B, C, E, F, H can be worked on in parallel after A lands. D joins once C is done. G joins once B/C/E are done. I needs B. J waits on nearly everything. K waits on J.

---

## Change A — Workspace scaffold, `specify-error`, CI

**Scope**

- Convert repo root into a Cargo workspace with a root package named `specify` (`src/main.rs` + `src/lib.rs`, both initially minimal — just enough to compile).
- Create empty placeholder crates under `crates/` for: `specify-error`, `specify-schema`, `specify-spec`, `specify-merge`, `specify-task`, `specify-validate`, `specify-change`, `specify-drift`, `specify-federation`. Each one is a stub library crate with a `lib.rs` containing `//! <crate purpose>` plus a trivial test so `cargo test --workspace` is non-empty.
- Implement `specify-error` fully per the `error.rs` section of RFC-1: the `Error` enum with every variant (`NotInitialized`, `SchemaResolution`, `Config`, `Validation`, `Merge`, `Lifecycle`, `SpecifyVersionTooOld`, `Io`, `Yaml`) using `thiserror`. Include `ValidationResult` as a forward-declared type or move it into `specify-validate` and have `Error::Validation` carry a `Vec<ValidationResultSummary>` — pick one and document the choice.
- Add `.github/workflows/ci.yml` with three jobs on `ubuntu-latest` + stable Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Trigger on `pull_request` and `push: main`.
- Update the `Makefile` per RFC-1 §Makefile Integration: `build` target that runs `cargo build --release` and copies the binary to repo root; keep `dev-plugins` and `prod-plugins` untouched.
- Verify `scripts/checks.ts` still runs (`make checks` unchanged).

**Deliverables**

- Workspace compiles: `cargo build --workspace` succeeds.
- `cargo test --workspace` green.
- CI workflow runs on a draft PR and passes.
- `specify-error` public API matches RFC-1 §`error.rs`.

**Acceptance**

- A later Change can add real logic to any crate without touching workspace wiring.
- `make build` produces `./specify` at the repo root.

---

## Change B — `specify-schema`: schema, brief, PipelineView, cache-meta

**Scope**

- Implement the full `schema.rs` surface from RFC-1: `Schema`, `Pipeline`, `PipelineEntry`, `ResolvedSchema`, `SchemaSource`, `Phase`, plus `Schema::resolve` (local and cache paths — no HTTP), `Schema::validate_structure` (embed `schemas/schema.schema.json` via `include_str!` and validate with a JSON-Schema crate such as `jsonschema`), `Schema::entries`, `Schema::entry`, and `Schema::merge` for composition via `extends`.
- Implement `brief.rs`: `BriefFrontmatter`, `Brief`, `Brief::load`, `Brief::parse`. Split `---`-delimited YAML frontmatter from body.
- Implement `PipelineView::{load, brief, phase}` with the four cross-reference validations called out in RFC-1 (brief path exists + parses; `frontmatter.id == PipelineEntry.id`; `needs` is a previously-defined brief; `tracks` refers to an in-schema brief).
- Implement `CacheMeta` (read-only: `path`, `load`, `validate_structure`, `matches`). No writer.
- Create `schemas/cache-meta.schema.json` documenting the `{ schema_url, fetched_at }` shape. Embed it in `specify-schema` via `include_str!` for `validate_structure`.
- Unit tests per public function, plus a parity test that resolves `schemas/omnia` and every brief underneath without error.

**Deliverables**

- `specify-schema` public API exactly matches RFC-1 §`schema.rs`, §`brief.rs`.
- `schemas/cache-meta.schema.json` committed.
- Tests cover schema composition (`extends`), cross-ref validation failures, malformed frontmatter, and missing cache-meta.

**Dependencies**

- Change A.

---

## Change C — `specify-spec`: parser port of `merge-specs.py`

**Scope**

- Implement `RequirementBlock`, `Scenario`, `ParsedSpec`, `DeltaSpec`, `RenameEntry`.
- Implement `parse_baseline`, `parse_delta`, `has_delta_headers`.
- Declare the heading constants (`REQUIREMENT_HEADING`, `REQUIREMENT_ID_PREFIX`, `REQUIREMENT_ID_PATTERN`, `SCENARIO_HEADING`, `DELTA_ADDED`, `DELTA_MODIFIED`, `DELTA_REMOVED`, `DELTA_RENAMED`) as hardcoded `pub const` values per `spec-format.md`.
- Port `merge-specs.py` tests (or create equivalent) as Rust unit tests: baseline with multiple requirements, delta with all four section types, malformed ID, missing scenarios.

**Deliverables**

- `specify-spec` API matches RFC-1 §`spec.rs`.
- Parity: feed the same input to the Python parser and the Rust parser and get equivalent structured output for a handful of fixtures.

**Dependencies**

- Change A.

---

## Change D — `specify-merge`: atomic, transactional merge engine

**Scope**

- Implement `MergeResult`, `MergeOperation` (all five variants including `CreatedBaseline`).
- Implement `merge(baseline, delta)` as a pure function — port of `merge-specs.py` logic.
- Implement `validate_baseline(baseline, design)` coherence check returning `Vec<ValidationResult>`.
- Implement `merge_change(change_dir, specs_dir, archive_dir)`:
  - Discover every delta spec in `change_dir` via `PipelineView` + brief `generates` paths (so this Change transitively consumes Change B).
  - Compute every merged baseline in memory.
  - Run coherence on all merged baselines.
  - On success: write baselines, flip `.metadata.yaml.status` → `Merged` via `specify-change`, move change dir to `archive/YYYY-MM-DD-<name>/`.
  - On any failure before the commit point: return `Err`, filesystem untouched.
- Tests: two-spec change, rename-across-specs, coherence-failure rollback (confirm no partial writes), archive naming.

**Deliverables**

- Behavioural parity with `scripts/merge-specs.py` for existing fixtures.
- Transactional guarantees verified by tests that inject a coherence failure.

**Dependencies**

- Changes A, B, C, F (for the lifecycle flip — can be stubbed to a TODO if F isn't ready, but J will need real F).

---

## Change E — `specify-task`: parser + mark_complete

**Scope**

- Implement `Task`, `SkillDirective`, `TaskProgress`.
- Implement `parse_tasks(content)` recognising grouped-under-heading tasks, `- [ ]` / `- [x]` checkboxes, optional `[plugin/skill]` skill directives.
- Implement `mark_complete(content, task_number)` — string-level rewrite, idempotent (marking an already-complete task returns input unchanged, not an error).
- No `next_pending` helper (explicitly deferred per RFC-1).
- Tests: nested headings, duplicate task numbers, idempotent double-mark, absent task number returns `Error`.

**Deliverables**

- `specify-task` API matches RFC-1 §`task.rs`.

**Dependencies**

- Change A.

---

## Change F — `specify-change`: lifecycle state machine

**Scope**

- Implement `ChangeMetadata`, `LifecycleStatus`, `TouchedSpec`, `SpecType` with serde annotations (`kebab-case` for outer struct, `lowercase` for enums).
- Implement `LifecycleStatus::{initial, is_terminal, can_transition_to, transition}` covering every edge in the state-machine table (including `Defined → Defining` for `--force-reset` and `Defining → Complete` for baseline extract).
- Implement `ChangeMetadata::load(change_dir)` and `save(change_dir)` helpers (the RFC doesn't specify them by name but every consumer needs them).
- Property test: every ordered pair of states has a deterministic `can_transition_to` answer; terminal states accept no outgoing edges.

**Deliverables**

- All eight edges round-tripped through `transition`.
- `.metadata.yaml` deserialization tested against a real file from an existing change under `.specify/changes/` (if available).

**Dependencies**

- Change A.

---

## Change G — `specify-validate` + RFC-1a (rule registry, Pass/Fail/Deferred)

**Scope**

- Implement `ValidationResult` (the `Pass`/`Fail`/`Deferred` enum with stable `rule_id` and human-readable `rule`).
- Implement `ValidationReport` (keyed by brief id or generated artifact path).
- Implement `Classification`, `RuleOutcome`, `Rule`, `BriefContext`, `CrossRule`.
- Implement the primitive checkers (`has_section`, `has_content_after_heading`, `all_requirements_have_scenarios`, `all_requirements_have_ids`, `ids_match_pattern`, `all_tasks_use_checkbox`, `tasks_grouped_under_headings`, `proposal_deliverables_have_specs`, `design_references_exist`).
- Build the `rules_for(brief_id)` registry for the four brief types (`proposal`, `specs`, `design`, `tasks`) using the rules in RFC-1a's representative table. Every rule has a stable `rule_id` (e.g. `proposal.why-has-content`).
- Build `cross_rules()` covering `cross.proposal-crates-have-specs` and `cross.design-references-valid`.
- Implement `validate_change(change_dir, pipeline)` — discovers artifacts via `PipelineView`, looks up rules, never calls `check` for `Semantic` rules (always emits `Deferred`).
- Golden tests: feed a known-good change directory and a known-bad one, assert the full JSON shape against a checked-in golden file so `schema_version: 1` is pinned.

**Deliverables**

- Full rule registry covering the RFC-1a table.
- Deferred rules never invoke their checker function (enforced by a test that panics from within a semantic checker's `check` field — proving it's never called).
- Golden JSON output checked in under `tests/fixtures/` or equivalent.

**Dependencies**

- Changes A, B, C, E.

---

## Change H — Stubs: `specify-drift`, `specify-federation`

**Scope**

- `specify-drift`: `DriftEntry`, `DriftStatus`, `baseline_inventory(specs_dir)` that walks `specs/*/spec.md`, parses each via `specify-spec`, and returns `Vec<(String, Vec<RequirementBlock>)>`. No drift logic yet — that's RFC-2.
- `specify-federation`: `PeerRepo`, `parse_federation_config(config) -> Vec<PeerRepo>` returning `vec![]` until RFC-3 defines the shape.
- Tests: empty repo, single spec, multiple specs for drift; empty/no-federation config for federation.

**Deliverables**

- Both crates compile and re-export their public API from the root `specify` lib.
- RFC-2 / RFC-3 can extend without restructuring.

**Dependencies**

- Changes A, B, C.

---

## Change I — Root package: `ProjectConfig`, `init`, CLI plumbing

**Scope**

- Implement `ProjectConfig` in `src/lib.rs` exactly per RFC-1 §`config.rs`, including path helpers (`specify_dir`, `changes_dir`, `specs_dir`, `cache_dir`, `rule_path`).
- Implement the `init` function in `src/lib.rs` per §`init.rs` including `InitOptions`, `VersionMode`, `InitResult`.
  - Creates `.specify/{changes,specs,archive,.cache}/`.
  - Writes `project.yaml` with `name`, `domain`, `schema`, `specify_version`, and one empty `rules:` entry per `pipeline.define` brief (resolved via `PipelineView`).
  - Upserts `.specify/.cache/` into the project `.gitignore`.
  - Detects cache presence (`cache_present` flag) without writing to the cache.
- Write the clap `Cli` / `Commands` / `TaskAction` / `SchemaAction` enums in `src/main.rs` exactly per §CLI Subcommands. Include `--format text|json` as a global flag.
- Implement the output-format wrapper that emits `{ "schema_version": 1, ... }` for every JSON response.
- Implement the `specify_version` floor check: on load, compare `env!("CARGO_PKG_VERSION")` against `ProjectConfig::specify_version` and return `Error::SpecifyVersionTooOld` if older. Map this to exit code 3.
- Define the exit-code table (0 success, 1 generic failure, 2 validation failed, 3 version-too-old, etc. — document the table).
- At this Change's end, subcommands dispatch to "not yet implemented" stubs.

**Deliverables**

- `specify --help` and `specify <sub> --help` print reasonable output.
- `specify init` works end to end against a fixture project.
- Version-floor violation produces exit code 3 with the expected JSON error shape.

**Dependencies**

- Changes A, B.

---

## Change J — Subcommand wiring + end-to-end integration tests

**Scope**

- Wire every subcommand from §CLI Subcommands to its domain crate:
  - `specify init` → `specify::init` (with `--upgrade` toggling `VersionMode`).
  - `specify validate <change_dir>` → `specify_validate::validate_change`.
  - `specify merge <change_dir>` → `specify_merge::merge_change`.
  - `specify status [change]` → `PipelineView` + `ChangeMetadata` + `TaskProgress`.
  - `specify task progress|mark`.
  - `specify schema resolve|check`.
- Implement text rendering for each subcommand alongside JSON.
- Under `tests/` at the root package, build fixture change directories in `tests/fixtures/` and add golden-file integration tests that spawn the built `specify` binary via `assert_cmd` (or equivalent) and diff stdout against checked-in JSON and text goldens.
- Cover at minimum: `validate` on good + bad fixtures, `merge` on a two-spec change, `task progress`/`mark`, `schema resolve` for both local and cached schemas, `init` on an empty directory.

**Deliverables**

- All Phase-1 subcommands functional.
- Golden JSON tests pin `schema_version: 1` contract.
- CI runs the integration tests.

**Dependencies**

- Changes I, D, E, F, G, H.

---

## Change K — Skill migrations (close the loop)

**Scope**

- Migrate `plugins/spec/skills/init/SKILL.md` to invoke `specify init` (pre-populating `.specify/.cache/` per the cache-write ownership rule) with the hard-fail install instruction verbatim when the binary is missing.
- Migrate `plugins/spec/skills/merge/SKILL.md` to invoke `specify merge`; remove the `merge-specs.py`/prose fallback.
- Migrate `plugins/spec/skills/build/SKILL.md` to invoke `specify validate` and `specify task progress` / `specify task mark`. Delete the ~40 lines of prose validation instructions; replace with the six-line block in RFC-1 §Output Format.
- Migrate `plugins/spec/skills/status/SKILL.md` to invoke `specify status`.
- Update `scripts/checks.ts` if it validates skill prose patterns that change.
- Do **not** delete `scripts/merge-specs.py` yet — archive it under `scripts/legacy/` and add a README noting it's reference-only.

**Deliverables**

- Affected skills shrink measurably (line counts documented in PR description).
- Manual dry-run on a real change passes validate + build + merge end-to-end using the CLI.
- Install-failure message is identical across all migrated skills.

**Dependencies**

- Change J.

---

## Change L — Release distribution (can run in parallel with B–J)

**Scope**

- Add `.github/workflows/release.yml` that, on tag push, builds release binaries for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and publishes them to GitHub Releases.
- Publish `specify` to crates.io (add `[package.metadata]` as needed; document the `cargo publish` step).
- Create the Homebrew tap repo / formula (or PR it into this repo's `Formula/` and document the tap setup).
- Author `install.sh` and decide on the `specify.sh` domain (or park on `install.specify.sh`), documented in `docs/`.

**Deliverables**

- Tagging `v0.1.0` produces downloadable binaries plus published crate.
- `brew install augentic/tap/specify` works end to end.

**Dependencies**

- Change A (workspace must build); otherwise independent.

---

## Suggested execution order

1. **Change A** solo (blocks everything).
2. **Changes B, C, E, F, H** in parallel (five independent sub-agents).
3. **Change D** as soon as C lands.
4. **Change G** as soon as B, C, E are all done.
5. **Change I** as soon as B lands (can start while D/G are still in flight).
6. **Change J** once I, D, E, F, G, H are all done.
7. **Change K** once J is done.
8. **Change L** any time after A — can be a long-running background stream.

## Cross-cutting checks to enforce in every Change

- No networking dependencies in any crate under `crates/` (HTTP stays with the agent per RFC-1 §`schema.rs`).
- No `std::process::exit` anywhere outside `src/main.rs`.
- Every public function on a domain crate returns `Result<T, specify_error::Error>`.
- JSON output wraps the payload with `"schema_version": 1` at the outermost object.
- Every newly-added rule (Change G or later extensions) ships with a stable `rule_id` and explicit `Classification`.
