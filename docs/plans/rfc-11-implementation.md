# RFC-11 Implementation Plan: UI Specification Workflow

> Source RFC: [`rfcs/rfc-11-ui-spec.md`](../../rfcs/rfc-11-ui-spec.md) (frozen for execution).
> Repos in scope: `augentic/specify` (schemas, briefs, plugins, docs) and `augentic/specify-cli` (Rust CLI).

This plan is the agent-facing execution document for landing RFC-11. It is sequential: every phase assumes all prior phases are complete and merged. It does **not** repeat the RFC; each phase points at the relevant section. If a phase reveals fault or ambiguity in the RFC, stop and log it under [Issue Log](#issue-log) rather than improvising — the RFC is the source of truth and any divergence must be reconciled before moving on.

---

## How to use this plan

1. Find the next phase whose status is `[ ]` in the [Phase Control](#phase-control) table.
2. Open the matching `## Phase X.Y — …` section below; treat the RFC anchor list as required reading and the acceptance criteria as the merge bar.
3. Aim to complete the phase inside a single ~15-minute agent conversation. If a phase grows beyond that, stop and propose a sub-split as an [Issue Log](#issue-log) entry.
4. Sub-agent chores are listed where the bulk work (boilerplate, tests, find-and-replace) can safely be delegated; the parent agent owns design choices and final review.
5. On completion: run `make checks` from the affected repo, mark the phase `[x]` in the control table with a one-line note (commit SHA or PR link), and only then begin the next phase.
6. **Do not skip ahead.** Phases later in the sequence assume earlier deterministic surfaces (schemas, CLI verbs, briefs) already exist; running them out of order will introduce churn.

### Repo conventions

- **`specify`** owns `schemas/`, `plugins/`, `docs/`, `rfcs/`, and the brief markdown files. Validation runs via `make checks` (Deno + `scripts/checks.ts`).
- **`specify-cli`** owns the `specify` binary. The Vectis subcommand surface lives at `crates/vectis/src/` with the dispatcher at `src/commands/vectis.rs` and the `clap` action enum on `src/cli.rs` (`VectisAction`). Verification runs via `cargo test` and the repo's `Makefile.toml` tasks.
- All cross-repo phases call out which repo each acceptance bullet targets; do not touch the other repo unless the phase explicitly says so.

### Out-of-scope reminders

The RFC is explicit about deferred work — do not implement any of the following inside this plan:

- `figma-layout-inferer` and `code-layout-inferer` skills (RFC §B, §D — illustrative only).
- `tokens-inferer` / `assets-inferer` helper skills (RFC §E, §F).
- A `components.yaml` artifact or any cross-shell prop-shape contract beyond the `component:` directive (RFC §G).
- Multi-brand / multi-theme tokens, a shared parser library, a sibling `ui` plugin (RFC "Deferred beyond v1").
- Tightening the `.cursor/schemas/specify-schema.schema.json` `artifacts:` entry shape beyond `additionalProperties: { type: object }` (RFC §H).
- Promoting image-inferer fixtures into `make checks` (RFC §C).
- Adding `web` keys to `assets.schema.json` (RFC Appendix B note).

---

## Phase Control

Status legend: `[ ]` pending · `[~]` in progress · `[x]` complete · `[!]` blocked (see Issue Log).

| #     | Status | Phase                                                                  | Repo(s)        | RFC anchors                       |
| ----- | ------ | ---------------------------------------------------------------------- | -------------- | --------------------------------- |
| 1.1   | [x]    | Author `schemas/vectis/tokens.schema.json`                             | specify        | §F, Appendix A                    |
| 1.2   | [x]    | Author `schemas/vectis/assets.schema.json`                             | specify        | §E, Appendix B                    |
| 1.3   | [ ]    | Patch `composition.schema.json` (provenance kinds + `component` key)   | specify        | §A, §G, Appendix F.1, F.2         |
| 1.4   | [ ]    | Add `artifacts:` block + companion validator patch                     | specify        | §H, §L                            |
| 1.5   | [ ]    | Scaffold `specify vectis validate <mode> [path]` subcommand surface    | specify-cli    | §H, §I                            |
| 1.6   | [ ]    | Implement `validate tokens` mode                                       | specify-cli    | §H, Appendix A                    |
| 1.7   | [ ]    | Implement `validate assets` mode                                       | specify-cli    | §E, §H, Appendix B                |
| 1.8   | [ ]    | Implement `validate layout` mode                                       | specify-cli    | §A, §G, §H, Appendix F            |
| 1.9   | [ ]    | Implement `validate composition` mode (incl. auto-invoke)              | specify-cli    | §G, §H, §I                        |
| 1.10  | [ ]    | Implement `validate all` + `artifacts:` default-path resolution        | specify-cli    | §H                                |
| 2.1   | [ ]    | Author `plugins/vectis/references/layout-inferer-contract.md`          | specify        | §A, §G, §H                        |
| 2.2   | [ ]    | Add `vectis-image-layout-inferer` skill (+ fixtures)                   | specify        | §A, §C, §G, §J                    |
| 2.3   | [ ]    | Wire `composition.md` brief to `layout.yaml` + `validate` calls        | specify        | §H                                |
| 2.4   | [ ]    | Update `design.md` brief to read `composition.yaml`                    | specify        | §H, §K (chunk 2)                  |
| 3.1   | [ ]    | Migrate Swift token templates into `ios-writer` (3a, iOS half)         | specify        | §J, §K (chunk 3a), §L             |
| 3.2   | [ ]    | Migrate Kotlin token templates into `android-writer` (3a, Android)     | specify        | §J, §K (chunk 3a), §L             |
| 3.3   | [ ]    | Convert `vectis:design-system-writer` to a deprecated no-op alias (3b) | specify        | §J, §K (chunk 3b)                 |
| 3.4   | [ ]    | Rewrite `proposal.md` + `specs.md` (3c, vocabulary)                    | specify        | §K (chunk 3c), §L                 |
| 3.5   | [ ]    | Rewrite `build.md` + `tasks.md` + `merge.md` (3c, lifecycle)           | specify        | §I, §K (chunk 3c), §L             |
| 3.6   | [ ]    | Rewrite `composition.md` trigger + plan briefs (3c, plan/discovery)    | specify        | §K (chunk 3c), §L                 |
| 4.1   | [ ]    | Bump Vectis schema to `version: 3` and delete the alias                | specify        | §J, §L                            |

---

## Phase 1.1 — Author `schemas/vectis/tokens.schema.json`

**Repo.** `specify`.

**RFC anchors.** §F, Appendix A (the JSON Schema is normative).

**Scope.** Add a brand-new file `schemas/vectis/tokens.schema.json` whose body is the JSON Schema in Appendix A, verbatim where the appendix is normative. No CLI hookup yet — that arrives in Phase 1.6.

**Acceptance.**

- `schemas/vectis/tokens.schema.json` exists, contains the Appendix A schema, parses as valid JSON Schema 2020-12.
- `version: { "const": 1 }` is enforced; provenance enum is the six values from §F (`manual`, `figma-variables`, `style-dictionary`, `tokens-studio`, `dtcg`, `legacy`).
- Both Appendix D's example `tokens.yaml` and an empty-but-valid `{ "version": 1 }` document validate cleanly under any standard 2020-12 validator (use `ajv` or `python-jsonschema` as a one-shot smoke check; do not commit the smoke harness).
- `make checks` passes.

**Sub-agent chores.** None — this is a single file authored carefully against the appendix.

**Status.** `[x]` — landed locally on `main`; smoke-validated Appendix D, `{version:1}`, and a deliberately broken hex against `Ajv2020 + addFormats`; `make checks` passed. (Plan typo fixed in same change: "seven values" → "six values" — RFC §F enumerates six.)

---

## Phase 1.2 — Author `schemas/vectis/assets.schema.json`

**Repo.** `specify`.

**RFC anchors.** §E, Appendix B (normative). Note the §E §"Vector support" rules and the §"Missing vector exports" rule — these are CLI-mode concerns; the JSON Schema only validates structural shape (per Appendix B's preamble).

**Scope.** Add `schemas/vectis/assets.schema.json` per Appendix B verbatim. The `sources` map is `ios`/`android` only in v1; deliberately omit `web` (Appendix B's note documents why).

**Acceptance.**

- File exists with the Appendix B schema.
- All three `kind` variants (`raster`, `vector`, `symbol`) validate against Appendix E's example `assets.yaml`.
- The `vectorEntry` `anyOf` (`source` or `sources` required) is wired exactly as Appendix B specifies.
- Cross-artifact rules (file existence, composition reference resolution) are **not** in this schema — they belong to CLI `assets` mode (Phase 1.7).
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[x]` — landed locally on `screenshot`; smoke-validated Appendix E and 13 supplementary cases (including `vector` with only `source:`, only `sources:`, raster with no densities, asset-id case violation, `sources.web` rejection, and `provenance.kind: dtcg` rejection — distinct from `tokens.schema.json`'s broader provenance enum) against `Ajv2020 + addFormats`; `make checks` passed.

---

## Phase 1.3 — Patch `composition.schema.json` for provenance + component

**Repo.** `specify`.

**RFC anchors.** §A (`component:` directive in unwired subset), §G (cross-shell factoring), Appendix F.1, F.2, F.3.

**Scope.** Two additive patches against `schemas/vectis/composition.schema.json`:

1. **F.1.** Extend `provenanceSource.kind` enum to `["figma", "legacy", "manual", "screenshots", "code"]`. Note: this matches `assets.schema.json`'s provenance enum (Phase 1.2 / Appendix B) but is intentionally **distinct** from `tokens.schema.json`'s broader enum (`manual, figma-variables, style-dictionary, tokens-studio, dtcg, legacy`) — three artifacts, three deliberately scoped enums. Do not "harmonise".
2. **F.2.** Add the optional `component` key on `groupProps.properties` with the exact pattern + reserved-slug `not.enum` guard from Appendix F.2.

The document-level `version` constant **stays at `1`** (Appendix F preamble). F.3 is a non-action: the unwired-subset enforcement does **not** become a parallel `$defs` branch — it lives in CLI `layout` mode (Phase 1.8).

**Acceptance.**

- Both patches applied; existing `composition.yaml` baselines remain valid (verify by running the existing test suite or by validating any sample composition under the patched schema).
- Reserved slugs (`header`, `body`, `footer`, `fab`) are rejected by the `component` key's `not.enum`.
- The structural-identity rule from §G is **not** added to this schema (it's a CLI-mode rule).
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[ ]`

---

## Phase 1.4 — Add `artifacts:` block + companion validator patch

**Repo.** `specify`.

**RFC anchors.** §H (worked YAML shape, field semantics, `additionalProperties` policy), §L ("Schema definition" bullet).

**Scope.** Two coordinated edits:

1. Add the `artifacts:` block to `schemas/vectis/schema.yaml` exactly as written in §H's worked v1 shape (the YAML block with `layout`, `tokens`, `assets`, `asset-files`, `composition`, `design`, `specs`, `tasks`).
2. Add the companion patch to `.cursor/schemas/specify-schema.schema.json` so `artifacts` is permitted at the top level. The entry-value schema stays loose: `additionalProperties: { type: object }` — do **not** tighten it (§H, "Deferred beyond v1").

**Acceptance.**

- `schemas/vectis/schema.yaml` validates against the patched `specify-schema.schema.json`.
- The block contains every key and `consumed_by` / `produced_by` / `merge_strategy` / `validates_with` / `paths` field as written in §H.
- No skill or brief is wired to consume the block yet — that's Phase 2.3 (the lone v1 reader is the `composition.md` brief).
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[ ]`

---

## Phase 1.5 — Scaffold `specify vectis validate <mode> [path]` surface

**Repo.** `specify-cli`.

**RFC anchors.** §H (CLI validation modes), §I (build phase invocation).

**Scope.** Land the dispatcher and `clap` plumbing for the new `validate` verb without implementing any validator body. Each mode returns a `not-implemented` stub that exits non-zero so Phases 1.6–1.10 can fill them in incrementally without breaking the build between merges.

- Extend `VectisAction` (in `src/cli.rs`) with a `Validate` variant carrying `{ mode: ValidateMode, path: Option<PathBuf> }` and `mode in { Layout, Composition, Tokens, Assets, All }`.
- Add a `validate` module under `crates/vectis/src/` whose entrypoint accepts the mode + optional path, returns `CommandOutcome::Stub { command: "vectis validate <mode>" }` for now, and a small JSON shape for the dispatcher in `src/commands/vectis.rs`.
- Do **not** wire `artifacts:`-block default-path resolution in this phase (deferred to Phase 1.10 once all four modes exist).
- Wire JSON + text rendering through the existing v2 contract (look at `vectis_text_render_*` helpers for the pattern).

**Acceptance.**

- `specify vectis validate --help` lists `layout | composition | tokens | assets | all` with the optional `[path]` positional.
- All five modes exit non-zero with the `not-implemented` shape from `src/commands/vectis.rs`.
- `cargo test` passes; existing `vectis init|verify|add-shell|update-versions|versions` still work.
- No new validator dependencies pulled in beyond what an existing JSON Schema crate provides — choose the dependency now (e.g. `jsonschema`) and document it in the phase's PR description so Phases 1.6–1.10 inherit one canonical choice.

**Sub-agent chores.** Sub-agent can scaffold the new module structure (Cargo manifest entry, `mod.rs`, error variant additions on `VectisError`, JSON shape derives) once the parent agent has decided the JSON Schema crate.

**Status.** `[ ]`

---

## Phase 1.6 — Implement `validate tokens` mode

**Repo.** `specify-cli`.

**RFC anchors.** §H ("`tokens` mode"), Appendix A (the schema this mode validates against).

**Scope.** Replace the `Tokens` mode stub from Phase 1.5 with a real validator: parse the YAML at the supplied path, validate against the embedded `schemas/vectis/tokens.schema.json` from Phase 1.1, and report category/value-shape errors with paths the operator can act on.

- Embed `tokens.schema.json` into the binary (mirror however `specify-vectis` already embeds template assets — see `crates/vectis/embedded/`).
- Default path: `design-system/tokens.yaml` for now (the `artifacts:`-block default-path lookup arrives in Phase 1.10; until then accept the explicit `[path]` argument or use the canonical fallback).
- Exit codes per §H: non-zero on errors, zero with a printed warning report on warnings, zero silently on a clean run.
- JSON output uses the v2 contract (kebab-case top level, `errors`/`warnings` arrays with `path` + `message` shapes — match the conventions already in `VectisError::to_json`).

**Acceptance.**

- `specify vectis validate tokens path/to/valid-tokens.yaml` exits 0 silently.
- Appendix D's example validates cleanly.
- A deliberately broken tokens file (e.g. `colors.primary.light: "#xyz"`) reports a single error with a YAML-path-ish location.
- `cargo test` includes at least one happy-path test and one negative test for this mode.

**Sub-agent chores.** Sub-agent can author the test fixtures and the test bodies once the validator surface is in.

**Status.** `[ ]`

---

## Phase 1.7 — Implement `validate assets` mode

**Repo.** `specify-cli`.

**RFC anchors.** §E (rules, including the §"Missing vector exports" rule), §H ("`assets` mode"), Appendix B.

**Scope.** Schema-validate `assets.yaml` against the Phase 1.2 schema, then layer the cross-artifact checks §E demands:

- Verify that every `filePath` resolves to a file under the directory containing `assets.yaml` (typically `design-system/assets/**`). Missing files are errors.
- For every `vector` and `raster` asset referenced by a sibling `composition.yaml` (when one exists at the canonical paths from §H), require `sources.<platform>` for each *targeted* shell platform (the platform set is determined by the proposal — for v1 just check both `ios` and `android` if the platform is plausibly present; the formal "targeted shell platforms" wiring lands when the build brief invokes this mode in Phase 3.5).
- Missing optional densities are warnings unless the target platform has no usable source (then error).
- Asset references **from** `composition.yaml` are resolved here when a sibling composition is present; they are *also* re-checked from the `composition` mode in Phase 1.9 when that mode auto-invokes this one — make sure both invocations produce identical reports.

**Acceptance.**

- Appendix E's example validates cleanly when paired with Appendix C's `layout.yaml` (referenced asset IDs all resolve).
- A missing `1x` raster file produces an error pointing at the asset entry and the missing path.
- A missing optional density (e.g. only `2x` and `3x` present) is a warning.
- `cargo test` covers happy-path, missing-file, and missing-density scenarios.

**Sub-agent chores.** Sub-agent can build the cross-artifact resolver against fixtures.

**Status.** `[ ]`

---

## Phase 1.8 — Implement `validate layout` mode

**Repo.** `specify-cli`.

**RFC anchors.** §A (output rules + unwired subset), §G (structural-identity rule), §H ("`layout` mode"), Appendix F (additive composition diff).

**Scope.** Validate `layout.yaml` as the **unwired subset** of `composition.schema.json`:

- YAML syntax + composition schema validation (the patched schema from Phase 1.3).
- `screens` only — reject documents that use `delta`.
- Reject define-owned wiring keys anywhere in the document: `maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation events, and any `*-when` keys (e.g. `strikethrough-when`). Matched keys produce an error with the YAML path.
- Enforce the §G structural-identity rule for any `component:` directives present (see §G's three edge cases for `*-when`-gated sub-groups, state-replaced bodies, and per-instance `platforms.*` overrides). The same identity engine will be reused by `composition` mode in Phase 1.9 — factor it accordingly.
- Cross-artifact reference checks against sibling `tokens.yaml` / `assets.yaml` are **also** §A behavior; layered exactly the same way as Phase 1.9 does for composition (auto-invoke when the sibling files exist at the canonical paths).

**Acceptance.**

- Appendix C's `layout.yaml` validates cleanly.
- A `bind:` key anywhere in the document produces an error.
- A `delta:` document is rejected.
- Two groups in different screens carrying the same `component:` slug with materially different skeletons produce a structural-identity error; same skeletons with different `bind` / `event` / token refs / `*-when` *conditions* validate cleanly.
- `cargo test` covers schema, unwired-subset, and structural-identity scenarios.

**Sub-agent chores.** Sub-agent can grow the structural-identity test matrix from §G's edge cases.

**Status.** `[ ]`

---

## Phase 1.9 — Implement `validate composition` mode (with auto-invoke)

**Repo.** `specify-cli`.

**RFC anchors.** §G (structural identity), §H ("`composition` mode"), §I (validation gate).

**Scope.** Composition is the lifecycle artifact — it allows both `screens` (baseline) and `delta` (change-local) shapes. The mode performs:

1. Schema validation against the patched `composition.schema.json`.
2. Cross-artifact resolution: every `maps_to`, `bind`, `event`, overlay `trigger`, navigation target, token reference, and asset reference must resolve. (RFC-7 already specifies the field/event/ViewModel/overlay/navigation coverage rules; this phase carries them forward through whatever helper RFC-7 left in place.)
3. The §G structural-identity rule across all instances of every `component:` slug.
4. **Auto-invoke** `tokens` mode when a sibling `tokens.yaml` exists, and `assets` mode when a sibling `assets.yaml` exists. Reports from those modes are folded into the composition report so callers do not need to invoke them separately (this is what §H's last two paragraphs and §I's "validation gate" require).

**Acceptance.**

- A composition that references a token name not in `tokens.yaml` errors via the auto-invoked `tokens` resolver.
- An `image:` that points at an unknown asset ID errors via the auto-invoked `assets` resolver.
- Structural-identity rule fires identically to Phase 1.8 (shared engine).
- `cargo test` covers each cross-artifact failure mode plus a clean end-to-end validation against the Appendix C/D/E example trio (after the Appendix C example is wired enough to be a valid composition — for v1 you can author a small fixture composition that is wired-mode equivalent of Appendix C).

**Sub-agent chores.** Sub-agent can build the fixture composition + write the cross-artifact failure tests.

**Status.** `[ ]`

---

## Phase 1.10 — `validate all` + `artifacts:`-block default-path resolution

**Repo.** `specify-cli`.

**RFC anchors.** §H ("`composition` mode" defaults paragraph + "CLI validation modes" closing paragraph; §H field semantics under "Worked v1 shape").

**Scope.** Two work items that finally make the v1 reader of the `artifacts:` block real:

1. Implement default-path resolution: when no `[path]` is supplied, each mode reads `schemas/vectis/schema.yaml` `artifacts:` block and uses `paths.change_local` then `paths.project` (then `paths.baseline` for composition) in order. If `artifacts:` is absent, fall back to the canonical paths in §H "Inputs". An explicit `[path]` argument always wins.
2. Implement the `all` convenience verb: runs `layout` (against active change), `composition` (active change → baseline fallback), `tokens`, `assets` and emits a combined report. Exit code is the worst of any sub-mode (errors > warnings > clean). JSON shape: `{ "results": [{ "mode": ..., "report": ... }, ...] }` — keep it composable.

**Acceptance.**

- `specify vectis validate layout` (no path) discovers `.specify/changes/<active>/layout.yaml` then `design-system/layout.yaml` per the `artifacts:` block.
- `specify vectis validate all` runs all four sub-modes and prints a single combined summary.
- Removing the `artifacts:` block falls back to the canonical paths cleanly.
- `cargo test` covers `artifacts:`-driven discovery and the cascade.

**Sub-agent chores.** Sub-agent can wire the YAML reader for `schemas/vectis/schema.yaml` and the path resolver.

**Status.** `[ ]`

---

## Phase 2.1 — Author `plugins/vectis/references/layout-inferer-contract.md`

**Repo.** `specify`.

**RFC anchors.** §A (every "MUST", "MAY", "SHOULD"), §G (directive emission rules), §H ("CLI validation modes" — the contract must point at the verbs it expects every inferer to call).

**Scope.** Brand-new reference doc that establishes the producer contract. Synthesise §A's argument table, output rules, idempotence rules, and verification rules; pull in §G's emission policy ("≥2 screens" rule, candidate-component comments); and pull in §H's verb list as the deterministic verification step every inferer must run before reporting success. Future `figma-layout-inferer` and `code-layout-inferer` skills will read this file and the RFC explicitly says they should reuse the same contract unless their RFC changes it (§A, §B, §D).

**Acceptance.**

- File exists at `plugins/vectis/references/layout-inferer-contract.md` with the four §A subsections (Common arguments, Operator ergonomics, Output rules, Idempotence rules), the §G emission policy, and the §H verification step.
- Contract names `vectis-image-layout-inferer` as the first-pass implementer (per §J) but does not couple to its prompts.
- `make checks` passes.

**Sub-agent chores.** None — this is structural authoring against the RFC.

**Status.** `[ ]`

---

## Phase 2.2 — Add `vectis-image-layout-inferer` skill

**Repo.** `specify`.

**RFC anchors.** §A (shared contract), §C (image inferer specifics, including the positive vision-prereq check), §G (conservative directive emission), §J (skill naming + plugin layout).

**Scope.** Create `plugins/vectis/skills/image-layout-inferer/` with:

- `SKILL.md` whose frontmatter follows the house style (`name: vectis-image-layout-inferer`, plugin-prefixed, third-person `description`, ≤500 body lines, Critical Path block).
- A body that follows §C's pipeline (Triage → Crop chrome → Infer regions → Infer containers → Infer leaves → Detect candidate components → Emit gaps), the §A common arguments, the positive vision-prereq check, and the §G conservative emission policy.
- A `references/` directory with at minimum a pointer to the Phase 2.1 contract doc (use a symlink the way `plugins/vectis/references/review-checks.md` is symlinked from sibling skills, or a relative link — match the existing convention).
- A `fixtures/` directory containing **one** worked fixture pair (`fixtures/<name>/input.png` + `fixtures/<name>/expected.layout.yaml`). Use a synthetic two-screen fixture (e.g. an Appendix C-shaped task list / settings flow). v1 does not enforce these in `make checks` (RFC §C), so they're operator-runnable references only.
- A terminal-summary template that lists screens added/refined, warnings, unresolved gaps, source provenance, output path, **and** the candidate-components block §J requires.

**Acceptance.**

- `make checks` passes (skill schema, name uniqueness, frontmatter rules, body length).
- The skill invokes `specify vectis validate layout` before reporting success (per §A "Verification" + §H).
- The conservative `component:` emission policy is documented in the body.
- Slash command surface is `/vectis:image-layout-inferer`.

**Sub-agent chores.** Sub-agent can produce the fixture image (any agreed graphic), the expected-layout YAML, and the SKILL frontmatter once the parent agent has finalised the body outline.

**Status.** `[ ]`

---

## Phase 2.3 — Wire `composition.md` brief to `layout.yaml` + validators

**Repo.** `specify`.

**RFC anchors.** §H ("Inputs", "Wiring responsibilities", "Multi-source handling", "CLI validation modes" closing paragraph), §K (chunk 2's third bullet).

**Scope.** Edit `schemas/vectis/briefs/composition.md` so the brief becomes the v1 reader of the `artifacts:` block:

- Replace the existing "Existing `composition.yaml` found" / "No existing `composition.yaml`" branching at the top of the brief with a resolution rule that reads `artifacts.layout.paths.change_local` then `artifacts.layout.paths.project` (per §H field semantics) to discover `layout.yaml` first; falls back to existing `composition.yaml` (change-local then baseline) only when no layout is found.
- Insert a step that calls `specify vectis validate layout` on the resolved input before the brief consumes it (per §H closing paragraph).
- After the brief writes its `composition.yaml`, insert a step that calls `specify vectis validate composition` on the result for cross-artifact token / asset checks.
- Preserve §H's "Wiring responsibilities" rules: the brief MUST preserve layout-owned structure, MUST NOT silently insert/remove a `component:` slug (it MAY propose one as a `# GAP` comment), and MUST NOT rewrite token / asset names.
- Multi-source handling (§H): no separate pre-define merge ceremony; the brief consumes the single `layout.yaml` and reports conflicts as comments.

**Acceptance.**

- The brief's "Input Resolution" section names `layout.yaml` first and explicitly cites the `artifacts.layout.paths` chain.
- Two new explicit invocation lines for `specify vectis validate layout` (pre) and `specify vectis validate composition` (post).
- The "Wiring responsibilities" wording matches §H's bullet list.
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[ ]`

---

## Phase 2.4 — Update `design.md` brief to read `composition.yaml`

**Repo.** `specify`.

**RFC anchors.** §H ("Outputs" — `design.md` should not duplicate raw layout tree), §K (chunk 2's brief edit).

**Scope.** Edit `schemas/vectis/briefs/design.md` so it explicitly reads `composition.yaml` rather than `layout.yaml` for screen / ViewModel / binding / token / asset implications. Replace any "raw layout tree" guidance with "read `composition.yaml`". `design.md` becomes a *reader* of composition, not a parallel surface for the same information.

**Acceptance.**

- No remaining mentions of consuming `layout.yaml` directly from `design.md`.
- A clear instruction that `design.md` reads the wired `composition.yaml` for layout-derived implications.
- `design.md` does **not** reproduce the layout tree, asset manifest, or token list.
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[ ]`

---

## Phase 3.1 — Migrate Swift token templates into `ios-writer` (chunk 3a, iOS half)

**Repo.** `specify`.

**RFC anchors.** §J ("Reference migration"), §K ("Step 3a (firm prerequisite)"), §L ("Vectis plugin", "Generated layout").

**Scope.** This is the firm prerequisite from §K — it MUST land before Phase 3.3 (alias conversion). Touch only the iOS surface in this phase to keep the agent context tight; Android lands in Phase 3.2.

- Move `plugins/vectis/skills/design-system-writer/references/swift-token-templates.md` to `plugins/vectis/skills/ios-writer/references/swift-token-templates.md`. Use `git mv` so history is preserved.
- Rewrite `plugins/vectis/skills/ios-writer/references/design-system-integration.md` to describe **shell-local** token / theme code emission inside `iOS/<App>/Theme/` (§L "Generated layout"), HIG fallback policy when `tokens.yaml` is absent (§F "Fallback policy belongs to shell writers"), and copy-on-generate asset rules (§E, §I).
- Update `plugins/vectis/skills/ios-writer/SKILL.md` to read `tokens.yaml` / `assets.yaml` / `composition.yaml` directly and emit theme + asset catalog code inside the iOS shell tree. Generated apps MUST NOT depend on `import VectisDesign` (§L). Add the §I component-directive contract: when a `group` carries `component: <slug>`, emit a single named SwiftUI `View` per slug, PascalCased (`task-row` → `TaskRow`).

**Acceptance.**

- `swift-token-templates.md` lives under `ios-writer/references/`; old path is gone.
- `ios-writer/references/design-system-integration.md` no longer instructs the writer to consume an external Swift Package; it describes the shell-local theme code path.
- `ios-writer/SKILL.md` documents the `component:` directive contract.
- `make checks` passes.

**Sub-agent chores.** Sub-agent can do the find/replace pass for stale `VectisDesign`/Swift-Package references in `ios-writer/SKILL.md` and example files.

**Status.** `[ ]`

---

## Phase 3.2 — Migrate Kotlin token templates into `android-writer` (chunk 3a, Android half)

**Repo.** `specify`.

**RFC anchors.** §J ("Reference migration"), §K ("Step 3a"), §L ("Vectis plugin", "Generated layout").

**Scope.** Mirror Phase 3.1 for Android.

- Move `plugins/vectis/skills/design-system-writer/references/kotlin-token-templates.md` to `plugins/vectis/skills/android-writer/references/kotlin-token-templates.md` (`git mv`).
- Rewrite `plugins/vectis/skills/android-writer/references/design-system-integration.md` for shell-local theme / token / asset code inside `Android/app/src/main/kotlin/.../ui/theme/` (§L), Material 3 fallback policy when `tokens.yaml` is absent, and copy-on-generate asset rules.
- Update `plugins/vectis/skills/android-writer/SKILL.md` to read input artifacts directly and emit theme + drawable resources inside the Android tree. Generated apps MUST NOT include `:vectis-design` Gradle module references (§L). Add the §I component-directive contract: emit a single named `@Composable` per slug, PascalCased.

**Acceptance.**

- `kotlin-token-templates.md` lives under `android-writer/references/`; old path is gone.
- `android-writer/references/design-system-integration.md` describes the shell-local theme code path with Material 3 fallback.
- `android-writer/SKILL.md` documents the `component:` directive contract.
- `make checks` passes.

**Sub-agent chores.** Sub-agent can do the find/replace pass for stale `:vectis-design` / Gradle references in `android-writer/SKILL.md` and example files.

**Status.** `[ ]`

---

## Phase 3.3 — Convert `vectis:design-system-writer` to a deprecated no-op alias (chunk 3b)

**Repo.** `specify`.

**RFC anchors.** §J ("kept as a deprecated no-op alias …"), §K ("Step 3b"), §L ("Compatibility policy").

**Scope.** Sequencing matters: this MUST land *after* Phase 3.1 + Phase 3.2 (§K firm ordering constraint). Otherwise downstream regenerations between alias-conversion and template-migration lose theming entirely.

- Rewrite `plugins/vectis/skills/design-system-writer/SKILL.md` as a deprecated no-op alias: the body explains the new path (`vectis:ios-writer` and `vectis:android-writer` consume `tokens.yaml` / `assets.yaml` directly) and exits without generating any files.
- Empty / remove the `references/` subdirectory entries that have already been moved (the SKILL retains its directory until the schema-version bump in Phase 4.1 deletes it outright).
- The alias's frontmatter `description` should make the deprecation explicit so the discovery surface flags it for operators.

**Acceptance.**

- `/vectis:design-system-writer` invocation produces no files and prints the redirect message.
- Old SKILL body is gone; new body is a deprecation notice only.
- `make checks` passes.

**Sub-agent chores.** None.

**Status.** `[ ]`

---

## Phase 3.4 — Rewrite `proposal.md` + `specs.md` (chunk 3c, vocabulary)

**Repo.** `specify`.

**RFC anchors.** §K ("Step 3c"), §L ("Proposal brief", "Specs brief").

**Scope.** Two coordinated brief edits:

- `schemas/vectis/briefs/proposal.md`: drop `design-system` from the `Platforms` enumeration. Remaining values are `core`, `ios`, `android`, future `web` (§L). Update the `## Platforms` example block accordingly.
- `schemas/vectis/briefs/specs.md`: retire `## Design System Requirements`. Move requirements about tokens/assets/component usage either into the platform-neutral body (when they describe observable product behavior) or into `## iOS Shell Requirements` / `## Android Shell Requirements` (when they're platform-specific rendering obligations).

**Acceptance.**

- Neither brief mentions `design-system` as a `Platforms` value or a requirements section.
- Both briefs still validate `make checks`.
- Existing proposal/specs deltas remain readable; the migration path described in §K ("Existing proposals, specs, tasks, and plans") is preserved (`/spec:define` will rewrite legacy proposals on next regeneration).

**Sub-agent chores.** Sub-agent can do the find/replace pass for residual `design-system` strings in these two briefs.

**Status.** `[ ]`

---

## Phase 3.5 — Rewrite `build.md` + `tasks.md` + `merge.md` (chunk 3c, lifecycle)

**Repo.** `specify`.

**RFC anchors.** §I (build phase ordering, validation gate, shell handoff, merge handoff), §K ("Step 3c"), §L ("Build brief", "Tasks brief").

**Scope.** Three coordinated brief edits:

- `schemas/vectis/briefs/build.md`: phase ordering becomes **core → shells**; remove the design-system phase and any reference to a shared design-system verification step. Insert §I's validation gate (`specify vectis validate composition` and the auto-invoked tokens / assets checks) as an explicit pre-shell-generation step. Document the shell handoff: each shell writer receives `composition.yaml`, `tokens.yaml`, `assets.yaml`, image files, `app.rs`, `design.md`, and the platform-specific shell requirements. Shell generation can run in parallel; verification stays serial. Reviewers run in parallel where shell trees are disjoint.
- `schemas/vectis/briefs/tasks.md`: drop `vectis:design-system-writer` from the "Available Skills" table. Update the "ordered: design-system first, core second, shells last" sentence to "ordered: core first, shells second" (§L "Tasks brief").
- `schemas/vectis/briefs/merge.md`: extend the brief so the merge surface explicitly lists `composition.yaml`, `tokens.yaml`, `assets.yaml`, and `design-system/assets/**` deltas alongside spec/design/task changes (§I "Merge handoff"), and re-runs `specify vectis validate composition` (with auto-invoked tokens / assets) on the merged input set even when no platform was generated in the current change.

**Acceptance.**

- `build.md` has no `design-system` phase, has the validation gate wired, and documents shell-writer hand-off content.
- `tasks.md` skill table is `core-writer` / `core-reviewer` / `ios-writer` / `ios-reviewer` / `android-writer` / `android-reviewer` / `test-writer` only (no `design-system-writer`).
- `merge.md` mentions UI input deltas and the post-merge composition validation step.
- `make checks` passes.

**Sub-agent chores.** Sub-agent can audit the three briefs for residual `design-system` strings after the parent agent has authored the structural edits.

**Status.** `[ ]`

---

## Phase 3.6 — Rewrite `composition.md` trigger + plan briefs (chunk 3c, plan/discovery)

**Repo.** `specify`.

**RFC anchors.** §L ("Composition brief", "Plan briefs"), §K ("Step 3c").

**Scope.** Three coordinated brief edits:

- `schemas/vectis/briefs/composition.md`: the existing trigger that keys off `design-system` appearing in `Platforms` (the line near the bottom of the "Input Resolution" section) becomes "if `design-system/tokens.yaml` exists or an explicit `tokens.yaml` path is supplied by the change". Remove any other reference to `design-system` as a platform.
- `schemas/vectis/briefs/plan/discovery.md`: drop the design-system tier. Discovery now reports layout, tokens, assets, and (future) components as cross-cutting UI inputs, with ordering hints naming the shell capabilities that consume them (§L "Plan briefs").
- `schemas/vectis/briefs/plan/propose.md`: no longer creates a "design-tokens" rung between core and shells by default. Token / asset changes become plan entries only when they are independently reviewable input-artifact work; shell entries depend on them when needed.

**Acceptance.**

- Neither brief mentions `design-system` as a tier or platform.
- Composition brief's token-availability check fires off file existence, not platform membership.
- Plan briefs describe UI inputs as cross-cutting rather than as a tier.
- `make checks` passes.

**Sub-agent chores.** Sub-agent can sweep for residual `design-system` strings across the three briefs.

**Status.** `[ ]`

---

## Phase 4.1 — Bump Vectis schema to `version: 3` and delete the alias

**Repo.** `specify`.

**RFC anchors.** §J ("kept as a deprecated no-op alias until the next Vectis schema bump (`schemas/vectis/schema.yaml` `version: 3`) after dissolution merges. The alias is removed in the same change that bumps the schema version"), §L ("Compatibility policy").

**Scope.** This phase is sequenced **after** all of chunk 3 has merged. It is the only phase that bumps `schemas/vectis/schema.yaml:version` from `2` to `3`. The bump trigger is "the next schema bump after dissolution merges" — so this phase MAY be deferred until an unrelated schema change wants to bump for its own reasons. When it lands:

- Bump `schemas/vectis/schema.yaml:version` from `2` to `3`.
- Delete `plugins/vectis/skills/design-system-writer/` outright, including its now-empty `references/` directory.
- Sweep for any residual `vectis:design-system-writer` mentions in remaining briefs, references, plugin manifests, and `docs/` and remove them.
- Update any "what's new" / migration docs (e.g. `docs/explanation/whats-new.md`) with a single sentence noting the version bump and the alias removal.

**Acceptance.**

- `schemas/vectis/schema.yaml` reports `version: 3`.
- `plugins/vectis/skills/design-system-writer/` does not exist.
- `grep -r design-system-writer` across both repos returns zero matches.
- `make checks` passes; `cargo test` (in `specify-cli`) passes.
- A short note in `docs/explanation/whats-new.md` records the version bump.

**Sub-agent chores.** Sub-agent can do the residual-mention sweep across both repos.

**Status.** `[ ]`

---

## Issue Log

If a phase reveals fault, ambiguity, or an in-scope detail this plan missed, append an entry here rather than improvising. The plan stays in sync with the RFC; ambiguities are reconciled by consulting the RFC author or amending the RFC explicitly.

| ID    | Phase | Date | Description | Resolution |
| ----- | ----- | ---- | ----------- | ---------- |
| _none yet_ | — | — | — | — |

### Reconciliation rules

- If the issue is a typo or trivially clarifiable reading, log it here and proceed (note the inferred reading in the entry).
- If the issue would change RFC behavior, **stop the phase**, mark the phase `[!]` in the control table, and reconcile with the RFC author before resuming.
- If the issue is in-scope but the plan is silent, propose a phase amendment as an Issue Log entry and have it accepted before incorporating into the plan.

---

## Reference index

- RFC: [`rfcs/rfc-11-ui-spec.md`](../../rfcs/rfc-11-ui-spec.md)
- Vectis schema definition: [`schemas/vectis/schema.yaml`](../../schemas/vectis/schema.yaml)
- Composition schema (patched in Phase 1.3): [`schemas/vectis/composition.schema.json`](../../schemas/vectis/composition.schema.json)
- Specify-schema validator (patched in Phase 1.4): [`.cursor/schemas/specify-schema.schema.json`](../../.cursor/schemas/specify-schema.schema.json)
- Vectis briefs (touched across Phases 2.3, 2.4, 3.4, 3.5, 3.6): [`schemas/vectis/briefs/`](../../schemas/vectis/briefs/)
- Vectis skills (touched across Phases 2.2, 3.1, 3.2, 3.3, 4.1): [`plugins/vectis/skills/`](../../plugins/vectis/skills/)
- CLI dispatcher (touched across Phases 1.5–1.10): `specify-cli/src/commands/vectis.rs`, `specify-cli/src/cli.rs`, `specify-cli/crates/vectis/src/`
