# RFC-3a Implementation Plan

> Companion to [`rfc-3a-monoliths.md`](rfc-3a-monoliths.md). This plan decomposes
> the RFC into subagent-sized chunks in expected execution order. Each chunk
> cites the RFC section(s) it implements, lists concrete files touched, and
> states a crisp "done when" so a subagent can pick it up cold without having
> to re-read the whole RFC.

## Conventions

- **Repos.** `specify/` is the docs/prompt-engineering repo (RFC, skills,
  briefs, schemas). `specify-cli/` is the Rust CLI workspace (library crates +
  the `specify` binary). Every chunk names the repo it lands in.
- **Order.** Chunks are listed in the expected order of execution. `Depends on`
  makes the real DAG explicit for parallelism.
- **Stages.** RFC-3a §*Staged rollout* defines Stages A / B / C. This plan
  interleaves the orthogonal Layer 1 (registry + brief) and Layer 2 (sync
  peers) work around the Stage ordering so the repo stays shippable between
  chunks.
- **Acceptance.** Every chunk should end with `cargo test -p <crate>` green in
  `specify-cli/` and (where applicable) `make checks` green in `specify/`.
  Fixture adds/updates are part of the chunk that introduces the behaviour.
- **Single-writer invariant** (RFC-2 §*Phase Boundary → Rule 2*) is preserved
  throughout — every `plan.yaml` write still routes through
  `specify initiative {init, create, amend, transition}`. Skills never
  hand-edit the plan file.

---

## Phase 0 — Plan-schema + type foundations

These land first because every subsequent chunk relies on `scope` being a
valid field on a plan entry.

### C01 · `plan.schema.json` gains `scope`
- **Repo:** `specify/` (+ mirror into `specify-cli/`)
- **RFC:** §*The `scope` field*
- **Files:**
  - `specify/schemas/plan/plan.schema.json` (canonical)
  - `specify-cli/schemas/plan/plan.schema.json` (byte-identical mirror per
    `specify/schemas/plan/README.md` — must land in the same commit pair).
- **Scope:**
  - Add `scope` to `planChange.properties` as an object keyed by kebab-case
    source name whose value is `{include?: string[], exclude?: string[],
    manifest?: string}`.
  - Enforce the mutual-exclusion rule between `manifest` and
    `include`/`exclude` for the same source key with `if/then/not` (permits
    empty-object entries; avoids `oneOf`'s "matched 0 of N" error shape).
  - Keep `additionalProperties: false` on both `planChange` and the inner
    `scopeEntry` object.
  - Update the schema `description` to mention RFC-3a §*Large-Monolith
    Decomposition* for pointer discovery.
- **Done when:**
  - A synthetic `plan.yaml` carrying `scope` validates with
    `specify initiative validate` once C02 / C04 land. In this chunk, ad-hoc
    ajv runs against positive and negative fixtures pass, and the CLI mirror
    is `diff`-clean against the canonical.
  - The schema file still validates as `draft 2020-12`.
- **Status:** **Done.** Delivered the schema delta (`$defs.scopeEntry` + the
  outer `propertyNames` / `additionalProperties` `$ref`) and synced the CLI
  mirror. No fixture harness exists in `specify/` today; round-trip fixture
  coverage will land in C02's round-trip tests.
- **Depends on:** —

### C02 · `PlanChange` gains `scope` in the `change` crate — **DONE**

Resolved decisions (carry forward to later chunks):
- `Scope { include, exclude, manifest }` with `#[serde(deny_unknown_fields)]`.
- Custom `Deserialize` via a `ScopeShape` shadow struct → `TryFrom<ScopeShape>` → `Scope::try_new` — invariant enforced in **one** place.
- New error variant `Error::InvalidPlanScope(String)`; JSON kind **`invalid-plan-scope`** (covers `manifest` XOR `include/exclude`).
- `PlanChangePatch` still lacks `status`/`status-reason` / scope (C03 adds scope below).
- **Repo:** `specify-cli/`
- **RFC:** §*The `scope` field*, §*How `scope` travels through the pipeline*
- **Files:**
  - `crates/change/src/plan.rs`
  - `crates/change/src/lib.rs` (if re-exports needed)
  - existing round-trip tests in the `change` crate.
- **Scope:**
  - Add `Scope { include: Vec<String>, exclude: Vec<String>, manifest:
    Option<String> }` struct (serde, kebab-case).
  - Add `scope: BTreeMap<String, Scope>` to `PlanChange`, skipped on serialize
    when empty.
  - Enforce the `manifest` ⊕ `(include|exclude)` invariant in a `Scope::try_new`
    / deserialization hook so bad input produces a `specify_error::Error`.
  - Extend round-trip tests: a plan with `scope` round-trips byte-for-byte, and
    a plan that mixes `manifest` with `include` is rejected with a clear error.
  - Update the RFC-2 canonical fixture comment in `plan.rs` to note the new
    optional field.
- **Done when:**
  - `cargo test -p specify-change` passes.
  - `PlanChangePatch` still structurally lacks `status`/`status-reason`
    (invariant preserved) and gains no mutation path for scope through status
    transitions — `scope` edits go through `amend` only.
- **Depends on:** C01

### C03 · `specify initiative {create, amend}` accepts scope flags — **DONE**

Resolved decisions (carry forward to later chunks):
- `PlanChangePatch.scope: BTreeMap<String, Option<Scope>>` — wholesale replace per key, `None` removes the entry, absent keys untouched.
- `--scope-rm <key>` is `amend`-only.
- Referential-integrity check lives in the library: `Plan::create`/`amend` reject pre-write; `Plan::validate` emits the same code for read-time sweeps.
- New error `Error::InvalidPlanScopeKey { change, key }`; JSON kind **`scope-key-not-in-sources`** — the canonical ID the RFC §*Validation* names. C04 and downstream skill docs must use this exact string.
- Two distinct scope error JSON kinds are preserved: `invalid-plan-scope` (structural, C02) vs `scope-key-not-in-sources` (referential, C03). Do not collapse.
- **Repo:** `specify-cli/`
- **RFC:** §*How `scope` travels through the pipeline*
- **Files:**
  - `crates/change/src/actions.rs`
  - `src/main.rs` (CLI argument definitions)
  - tests under `tests/initiative.rs` (or nearest equivalent).
- **Scope:**
  - Add repeatable flags `--scope-include <key>=<glob>`,
    `--scope-exclude <key>=<glob>`, `--scope-manifest <key>=<path>` on
    `specify initiative create` and `specify initiative amend`.
  - Drive the flags through `Plan::create` / `Plan::amend` atomic writers.
  - Emit `scope-key-not-in-sources` early (before write) if a scope key is not
    in the change's declared `sources`. This anticipates C04's validator error
    and ensures the CLI never writes an invalid plan.
  - Add a `--scope-rm <key>` (or similar) on `amend` to clear a source's scope
    entry cleanly.
- **Done when:**
  - A scripted authoring sequence produces a `plan.yaml` matching the RFC
    §*How `scope` travels…* YAML fixture verbatim.
  - `cargo test -p specify-change -- --include-ignored` green.
- **Depends on:** C02

---

## Phase 1 — Validator updates

### C04 · Structural scope errors — **DONE**

Resolved decisions:
- Plan-level validation lives in `crates/change/src/plan.rs::Plan::validate`, not the `validate` crate (which handles brief artifacts).
- `Plan::validate` signature now `(changes_dir: Option<&Path>, project_dir: Option<&Path>)`. Path-existence checks are opt-in via `project_dir = Some(..)`.
- Helpers `glob_root()` and `is_remote()` added. `glob` crate (0.3) adopted (shell-style, `**` recursive); gitignore-style nuance flagged below.
- `scope-key-not-in-sources` (C03) + `scope-path-missing` (C04) both surface through `specify initiative validate --format json`. Exit code honours the Error vs Warning distinction.
- **Glob semantics contract (pinned):** shell-style glob with `**` for recursive match (via the `glob` crate). The RFC prose's "gitignore-style" phrasing is operationally close but not identical (gitignore anchors differently). SKILL docs and briefs should say "glob with `**` recursive" rather than "gitignore-style" to stay accurate. Revisit only if a genuine gitignore-anchoring requirement appears.
- **Repo:** `specify-cli/`
- **RFC:** §*Validation*
- **Files:**
  - `crates/validate/src/run.rs` (or wherever plan-level checks live)
  - test fixtures under `crates/validate/src/` and/or `tests/`.
- **Scope:**
  - Implement `scope-key-not-in-sources` (Error): every key under `scope:` on
    a change must be present in that change's `sources:`.
  - Implement path-existence (Error): every include glob root, every exclude
    glob root, and every manifest file referenced must resolve under
    `sources[<key>]`.
  - Each `ValidationResult` carries a stable `id` (`scope-key-not-in-sources`,
    `scope-path-missing`) so downstream docs and skills can reference them.
- **Done when:**
  - `specify initiative validate` surfaces both diagnostics on synthetic bad
    plans and stays silent on good plans.
  - `cargo test -p specify-validate` green.
- **Depends on:** C02

### C05 · Scope warnings (overlap, orphan) — **DONE**

Resolved decisions:
- `scope-overlap` and `scope-orphan` fire only when at least one change carries `scope` (back-compat).
- Orphan walk skips a hard-coded ignore list: `.git`, `.hg`, `.svn`, `node_modules`, `target`, `dist`, `build`, `__pycache__`, `.venv`, `venv`, `.tox`, `.next`, `.nuxt`, `.cache`.
- Overlap finding is one-per-file with sorted change-name lists; `entry: None` (cross-entry finding).
- Minimal in-place manifest loader (`load_manifest_includes` → `Option<Vec<String>>`) lives in `plan.rs`; C26 will swap it for the full manifest validator.
- Empty-include + non-empty-exclude is interpreted as "whole tree minus excludes" (the RFC is silent; defensible).
- Warnings-only validate runs exit 0 — existing behaviour in `run_initiative_validate` already distinguishes Error vs Warning correctly.
- **Repo:** `specify-cli/`
- **RFC:** §*Validation*
- **Files:**
  - `crates/validate/src/run.rs`
  - tests.
- **Scope:**
  - `scope-overlap` (Warning): a file claimed by >1 change's scope is flagged
    per-file. The fixture matches the RFC's shared-validation example.
  - `scope-orphan` (Warning): a file under any `sources[<key>]` claimed by
    zero changes. Useful "did I cover everything?" lint.
  - Both fire only when at least one change has `scope` (back-compat: plans
    without scope get no overlap/orphan surface).
- **Done when:**
  - New tests exercise both warnings.
  - `cargo test -p specify-validate` green.
- **Depends on:** C04

---

## Phase 2 — Stage A: flag plumbing into `/spec:extract`

### C06 · `/spec:extract` grows native filter flags — **DONE**

Resolved decisions (carry into C07/C08):
- Zero-match filter on extract ≡ **hard error, fail fast** (no empty artifacts).
- `--manifest` mutually exclusive with `--include`/`--exclude` at the extract layer *and* upstream — defensive duplication is intentional.
- "Sentinels always read" list copied verbatim; Step 1 (language/manifest discovery) is immune to filters; Step 2+ uses the filtered read set.
- Fixture landed under `plugins/spec/skills/extract/fixtures/scoped-monolith/` demonstrates `--include 'src/a/**'` shrinking the read set.
- Glob phrasing in SKILL intentionally omits "gitignore-style" to stay accurate to the chosen shell-glob semantics (see C04).
- **Repo:** `specify/`
- **RFC:** §*How `scope` travels through the pipeline*, §*Sentinels always read*
- **Files:**
  - `plugins/spec/skills/extract/SKILL.md`
  - `plugins/spec/skills/extract/references/` (any updated algorithm refs)
  - new `fixtures/` for a scoped-extract walk-through.
- **Scope:**
  - Add `--include <glob>` (repeatable), `--exclude <glob>` (repeatable), and
    `--manifest <manifest-path>` (single, mutually exclusive with the first
    two) to the `argument-hint` and "Derived Arguments" sections of SKILL.md.
  - Document that globs resolve relative to `<source-path>` and that empty
    filters ≡ today's behaviour (small-legacy / greenfield unchanged).
  - Document the **sentinels always read** list verbatim from the RFC and the
    rule that `include` cannot subtract sentinels and `exclude` cannot hide
    them.
  - Add a manifest-shape section covering the v1 `include`-only manifest with
    file paths relative to `<source-path>`.
  - Land a small monolith fixture that demonstrates `--include` shrinking the
    read set without disturbing language/dependency detection.
- **Done when:**
  - `make checks` green.
  - Manual walk-through: invoking extract with `--include` on the fixture
    emits specs only for the scoped files.
- **Depends on:** —

### C07 · `/spec:execute` forwards scope flags
- **Repo:** `specify/`
- **RFC:** §*How `scope` travels through the pipeline* (driver step)
- **Files:**
  - `plugins/spec/skills/execute/SKILL.md`
  - `plugins/spec/skills/execute/fixtures/` (argument-resolution table + a
    scoped-change walk-through).
- **Scope:**
  - Extend the *Argument resolution* table with three rows symmetric with
    `--source` and `--affects`: `--scope-include`, `--scope-exclude`,
    `--scope-manifest`. Each row states: read from `plan.yaml:scope.<key>`,
    emit one flag per glob / per manifest path, forward verbatim to
    `/spec:define`.
  - Keep driver semantics string-only — the driver never interprets globs.
- **Done when:**
  - `make checks` green.
  - Fixture shows a plan entry with scope driving the correct flag set passed
    to `/spec:define`.
- **Depends on:** C02, C06

### C08 · `/spec:define` forwards scope flags
- **Repo:** `specify/`
- **RFC:** §*How `scope` travels through the pipeline*
- **Files:**
  - `plugins/spec/skills/define/SKILL.md`
  - fixtures.
- **Scope:**
  - Accept the three scope flags with repeatability semantics identical to
    `--source` / `--affects` and pass them through to the schema's define
    brief unchanged.
  - Document the per-source collection rule: for the current source key `<k>`,
    collect all `--scope-*=<k>=…` flags into one scope bundle handed to the
    brief's per-source extract loop.
- **Done when:**
  - `make checks` green.
  - Fixture demonstrates the collection rule for a two-source change.
- **Depends on:** C07

---

## Phase 3 — Stage A: Omnia brief rewrites

### C09 · Omnia `specs.md` brief: per-source extract loop + merge
- **Repo:** `specify/`
- **RFC:** §*How `scope` travels through the pipeline* (schema-owned loop)
- **Files:**
  - `schemas/omnia/briefs/specs.md`
  - brief fixtures under `schemas/omnia/` as needed.
- **Scope:**
  - Rewrite the brief around two branches — source-driven and manual.
  - Source-driven branch iterates per source key, collects the `--scope-*` bundle
    for that key, invokes `/spec:extract <path> <change-dir>/.extract/<key>/
    --include … --exclude … [--manifest …]`, and merges `.extract/<key>/specs/`
    and `.extract/<key>/design.*` into `<change-dir>/specs/` and
    `<change-dir>/design.md`.
  - Name-collision merge rule (Omnia): propose brief should have forced
    distinct names or consolidated duplicates under one source; otherwise
    collision is a brief-level error.
  - Document `.extract/<key>/` scratch-dir lifecycle (keep vs clean after
    merge).
- **Done when:**
  - Brief walkthrough matches the `extract-shared-validation` worked fixture
    from the RFC.
  - `make checks` green.
- **Depends on:** C08

### C10 · Omnia `proposal.md` brief: collapse source-branching
- **Repo:** `specify/`
- **RFC:** *Stage A* (propose-side collapse)
- **Files:**
  - `schemas/omnia/briefs/proposal.md` (the define-phase proposal, not the
    plan-phase one)
- **Scope:**
  - Remove per-source conditional branches that only made sense when extract
    ran at plan time; the source-driven path now owns extraction entirely.
  - Keep manual-branch behaviour unchanged.
- **Done when:**
  - `make checks` green.
- **Depends on:** C09

### C11 · Omnia `specs.md` brief: `--affects` composition step
- **Repo:** `specify/`
- **RFC:** §*`--affects` composition with scope*
- **Files:**
  - `schemas/omnia/briefs/specs.md`
  - worked fixture under `schemas/omnia/` matching the canonical
    `extract-shared-validation` example.
- **Scope:**
  - Add the four-step Affects composition block: (1) run scoped extract, (2)
    emit DELTA blocks for capabilities whose names match `--affects`, (3)
    emit fresh specs for unmatched capabilities, (4) warn for `--affects`
    names with no matching extract capability.
  - Cross-link the define skill's "Delta-specific workflows" section.
- **Done when:**
  - Worked fixture reproduces the three expected emissions from the RFC
    (DELTA + DELTA + new-crate spec).
  - `make checks` green.
- **Depends on:** C09

---

## Phase 4 — Layer 1: registry + brief scaffolding (single-project shape)

These can land in parallel with Phase 2/3 once C02/C04 exist.

### C12 · `registry.yaml` parsing (single-project shape)
- **Repo:** `specify-cli/`
- **RFC:** §*The Registry*
- **Files:**
  - `crates/schema/src/` (or a new module `registry.rs`)
  - `crates/validate/src/run.rs` (shape-validation hook)
  - tests under the relevant crate + `tests/initiative.rs`.
- **Scope:**
  - Parse `.specify/registry.yaml` as `{ version: 1, projects: [ { name,
    url, schema } ] }` with kebab-case `name` and a required `schema` (e.g.
    `omnia@v1`). No JSON schema file for v1 per the RFC; enforce shape
    directly in code.
  - Treat absent or single-entry registry as single-repo (no behaviour
    change for the `/spec:plan` flow yet; that lands in C24).
  - Reject unknown top-level keys with a clear diagnostic.
- **Done when:**
  - `cargo test` green.
- **Depends on:** —

### C13 · `specify initiative registry {show, validate}` CLI
- **Repo:** `specify-cli/`
- **RFC:** §*Diagram labels → skills and CLI*, §*CLI surface additions*
- **Files:**
  - `src/main.rs` (CLI dispatch)
  - relevant actions module.
- **Scope:**
  - `show` prints the parsed registry (JSON + human views).
  - `validate` re-runs shape validation with file-level error diagnostics and
    non-zero exit on any error.
  - Behaves gracefully when `registry.yaml` is absent (exit 0, "no registry
    declared").
- **Done when:**
  - `cargo test` and `specify initiative registry validate` green on a bare
    repo.
- **Depends on:** C12

### C14 · `initiative.md` parser + brief actions
- **Repo:** `specify-cli/`
- **RFC:** §*The Initiative Brief*, §*When are `registry.yaml` and `initiative.md` required?*
- **Files:**
  - `crates/schema/src/` (or new `initiative_brief.rs`)
  - `src/main.rs`
  - test fixtures.
- **Scope:**
  - Parse frontmatter with required `name` (kebab-case) and optional
    `inputs: [ { path, kind } ]`. Closed `kind` enum: `legacy-code |
    documentation`. Unknown `kind` is a hard error.
  - Body is captured as prose but **not** parsed further in v1.
  - Add `specify initiative brief {init, show}`:
    - `init <name>` scaffolds `.specify/initiative.md` from a template.
    - `show` dumps the parsed frontmatter + body.
  - Add an archive-time sweep hook (real sweep lands in C33) so `initiative.md`
    is included in the archive manifest.
- **Done when:**
  - `cargo test` green.
  - `specify initiative brief init traffic-modernisation` produces the
    template byte-for-byte against a golden fixture.
- **Depends on:** —

### C15 · Widen `/spec:plan` readiness gate
- **Repo:** `specify/`
- **RFC:** §*When are `registry.yaml` and `initiative.md` required?*
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md`
  - fixtures under `plugins/spec/skills/plan/fixtures/`.
- **Scope:**
  - Update §*Invocation* and §*Core loop → Step 1* so the "at least one of
    `--from`, `--against`, `--source`" gate widens to "…or
    `initiative.md:inputs` is non-empty."
  - Document that a bare `/spec:plan <name>` with neither CLI inputs nor
    populated `initiative.md:inputs` is still a hard exit.
  - Add a dry-run fixture covering the `initiative.md`-only case.
- **Done when:**
  - `make checks` green.
- **Depends on:** C14

### C16 · Closed `kind` vocabulary + CLI default rules
- **Repo:** `specify/`
- **RFC:** §*Discovery dispatch*, §*`--source` flags and the brief*
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md`
  - `schemas/omnia/briefs/plan/discovery.md` (dispatch-aware edits; this is
    a doc-only pass here — the skill call-sites move in C19/C23).
- **Scope:**
  - Document the closed `kind` enum (`legacy-code`, `documentation`) as
    normative for v1, with unknown `kind` being a hard error at the analyse
    phase.
  - Document the default-kind mapping:
    - `--source <k>=<p>` with no `:<kind>` suffix → `legacy-code`.
    - `--source <k>=<p>:<kind>` honours the explicit suffix.
    - `--from` defaults to `documentation`.
    - `--against` defaults to `legacy-code`.
  - This chunk only lands the *contract*; the actual dispatch call lives in
    C19 (documentation branch) and C23 (code branch replacement).
- **Done when:**
  - `make checks` green.
- **Depends on:** C15

---

## Phase 5 — Create the `/spec:analyze` skill shell and documentation branch

Stage B *starts* here. The skill shell can land with only the documentation
branch wired; the code branch follows in Phase 6 and replaces the
plan-time `/spec:extract` call.

### C17 · `/spec:analyze` skill scaffold
- **Repo:** `specify/`
- **RFC:** §*Discovery dispatch*, §*Plan-time analysis, define-time extraction*
- **Files:**
  - `plugins/spec/skills/analyze/SKILL.md`
  - `plugins/spec/skills/analyze/references/` (as needed)
  - `plugins/spec/skills/analyze/fixtures/`.
- **Scope:**
  - Create SKILL.md declaring `argument-hint: "<input-path> <output-dir>
    --kind <legacy-code|documentation> [--source-key <k>]"`.
  - Document the unified output contract: append capability summaries to
    `<output-dir>/discovery.md` with on-disk shape `### <name>` + a fenced
    YAML block carrying `summary`, `sources`, `depends-on`, `hints`,
    `confidence`.
  - Document the branching rule: internally dispatches on `--kind`; both
    branches emit the same shape.
  - Document idempotency: byte-equivalent output on unchanged inputs; no
    timestamps; stable ordering.
  - Document where per-kind prompts live (schema-owned, under
    `schemas/<schema>/briefs/plan/analyze/…`).
- **Done when:**
  - `make checks` green.
- **Depends on:** C16

### C18 · Omnia documentation branch brief — **DONE**

Resolved decisions (carry into C19/C21):
- Brief landed at `schemas/omnia/briefs/plan/analyze.md` with a top-level
  field-order / sort contract repeated verbatim (`summary`, `sources`,
  `depends-on`, `hints`, `confidence`; alphabetical by capability name;
  `sources` / `depends-on` / `hints.entry_points` / `hints.external_deps`
  sorted alphabetically within each block). The brief links back to
  [`analyze/SKILL.md` §Output contract](../plugins/spec/skills/analyze/SKILL.md)
  as the normative source.
- Split into two second-level sections: `## Documentation branch
  (--kind documentation)` (landed) and `## Legacy-code branch
  (--kind legacy-code)` (**placeholder, C21 appends here**). C21 MUST NOT
  restructure the header hierarchy — just fill the code-branch body.
- Documentation branch specifies an inventory table (Markdown / AsciiDoc /
  OpenAPI / runbook / PDF / ticket shapes), capability-identification
  heuristics per shape, deep-link conventions for `sources` (JSON-pointer
  for OpenAPI, heading slug for Markdown, `#page=N` for PDF), and the
  confidence-marker rubric (`high` / `medium` / `low`).
- `## Constraints (from documentation)` and `## Open questions (from
  documentation)` appendix blocks are documentation-branch-only and always
  follow the last `### <name>` capability. Each entry cites its source
  artifact (path + optional fragment). Empty blocks are omitted entirely.
  The empty-inventory fallback writes `$DISCOVERY` with no `###` blocks
  plus a single open-question sentence naming `$INPUT_PATH`.
- `--source-key` marker injection is the skill's responsibility, not the
  brief's — the brief never emits `<!-- source-key: ... -->`.
- Fixture landed at
  `schemas/omnia/briefs/fixtures/plan/analyze/documentation/` (following
  the existing `schemas/omnia/briefs/fixtures/specs/<case>/` convention),
  not inside `schemas/omnia/briefs/plan/analyze/`. C21's code-branch
  fixture should follow the same layout — likely under
  `schemas/omnia/briefs/fixtures/plan/analyze/legacy-code/`.
- **Repo:** `specify/`
- **RFC:** §*Discovery dispatch* (documentation branch), §*Plan-time analysis…*
- **Files:**
  - `schemas/omnia/briefs/plan/analyze.md` (new)
  - `schemas/omnia/briefs/fixtures/plan/analyze/documentation/{README.md,inputs/ops-runbook.md,expected/discovery.md}`
- **Scope:**
  - Documentation-branch prompt: parse prose / PDFs / runbooks / OpenAPI,
    identify capabilities, extract constraints and open questions, emit
    capability summaries with `sources:` pointing at literal artifact
    paths, `confidence` reflecting extraction certainty.
  - Worked example + small runbook fixture + expected output.
- **Done when:**
  - `make checks` green (same pre-existing 10 broken-link failures; no new
    ones).
- **Depends on:** C17

### C19 · Discovery brief dispatches documentation inputs to `/spec:analyze`

Carry-forward from C18:
- Dispatch target is `/spec:analyze --kind documentation <input-path>
  .specify/plans/<name>/ [--source-key <k>]`. The documentation-branch
  prompt is now pinned at `schemas/omnia/briefs/plan/analyze.md` §*Documentation
  branch*; discovery only shells out, never re-implements the extraction.
- Merge semantics: the analyze skill appends into `$DISCOVERY` under
  `.specify/plans/<name>/discovery.md` with alphabetic dedup-by-name;
  the existing `discovery.md` structure (`## Capability inventory`
  heading wrapper, `## Open questions` appendix) needs to coexist with
  the analyze skill's appendix blocks. Decision for C19: discovery wraps
  the analyze output — analyze writes `### <name>` + fenced YAML blocks
  directly; discovery's Output section widens to accept that shape and
  hoists `## Constraints (from documentation)` + `## Open questions (from
  documentation)` into the combined `discovery.md`.
- `--source-key` selection: one key per dispatch invocation. For
  `--from <path>` use the basename (without extension) as the key; for
  `--source <k>=<p>` use `<k>`; for `--against <p>` use `against`. Pin
  this in the brief so re-runs are idempotent.
- **Repo:** `specify/`
- **RFC:** §*Discovery dispatch*
- **Files:**
  - `schemas/omnia/briefs/plan/discovery.md`
  - fixtures under `plugins/spec/skills/plan/fixtures/discovery/`.
- **Scope:**
  - Route every `documentation`-kind input (from `--from`, `--source:documentation`,
    or `initiative.md:inputs`) through `/spec:analyze --kind documentation`.
  - Continue routing `legacy-code`-kind inputs through `/spec:extract` **for
    now** (Stage A parity). The code-branch switch lands in C23.
  - Update the merge step to concatenate `/spec:analyze` output into
    `discovery.md` with stable ordering; reconcile with analyze's
    `## Constraints` / `## Open questions` appendix blocks.
- **Done when:**
  - Mixed-input fixture (one doc + one code source) round-trips with the
    documentation capabilities produced by `/spec:analyze` and code
    capabilities still produced by the pre-RFC-3 `/spec:extract` path.
  - `make checks` green.
- **Depends on:** C18

---

## Phase 6 — Stage B: code branch + propose rewrite

### C20 · Structural-metadata output slot under `.specify/plans/<name>/analyze/<key>/` — **DONE**

Resolved decisions (carry forward to C21 and C25):
- File name is `metadata.json` (JSON, single-purpose).
- Shape v1 pins six required fields, in this order: `version` (integer, `1`),
  `source_key` (string, matches the directory segment), `language` (string,
  kebab-case, e.g. `typescript`), `loc` (integer), `module_count` (integer),
  `top_level_modules` (array[string], alphabetically sorted, immediate
  children of the source root). All fields required. Field-order / sort
  contract is byte-stable — identical to the `$DISCOVERY` idempotency rules.
- No JSON schema file (v1 is shape-in-doc, matching the RFC-3a posture).
  Bumping the shape requires an RFC and a `version` increment.
- Per-field detection algorithms (what counts as a "module" in language X,
  whether `loc` is raw or non-blank-non-comment, etc.) are owned by the
  schema-specific code branch prompt at `schemas/<schema>/briefs/plan/analyze.md`
  (RFC-3a C21). The SKILL only pins field names and types.
- Write-side guardrail: the skill writes the sidecar **only** on
  `$KIND = legacy-code`. The documentation branch MUST leave the slot
  absent; no empty file, no stub JSON.
- Fixture: `plugins/spec/skills/analyze/fixtures/scaffold-example/expected/plans/scaffold-example/analyze/monolith/metadata.json`
  shows the populated shape for the scaffold-example tiny monolith.
- **Repo:** `specify/`
- **RFC:** §*Validation* (monolith-scale lint prerequisite)
- **Files:**
  - `plugins/spec/skills/analyze/SKILL.md` — new §*Structural metadata*
    subsection between §*Output contract* and §*Idempotency*; cross-
    reference signpost inside §*Output contract*; Process step 4 split into
    4a ($DISCOVERY, both branches) / 4b (metadata.json, legacy-code only);
    guardrail + error-handling bullets for the documentation-branch
    no-write rule; Fixtures entry expanded.
  - `plugins/spec/skills/analyze/fixtures/scaffold-example/expected/plans/scaffold-example/analyze/monolith/metadata.json` — new.
  - `plugins/spec/skills/analyze/fixtures/scaffold-example/README.md` —
    extended to describe the metadata sidecar.
- **Done when:**
  - `make checks` green (same pre-existing 10 broken-link failures; no new
    ones).
  - Fixture shows the slot populated for a small monolith.
- **Depends on:** C17

### C21 · Omnia code branch in `analyze.md`

Carry-forward from C18:
- The brief already carries a `## Legacy-code branch (--kind legacy-code)`
  placeholder heading. C21 replaces the placeholder body in place — do not
  renumber sections or alter the top-level field-order / sort contract,
  which is shared across both branches.
- The code branch MUST emit the same capability-summary shape as the
  documentation branch (`### <name>` + fenced YAML with fields in fixed
  order). `sources:` on the code branch carries file-hint paths relative
  to the source root (e.g. `src/users/register.ts`) rather than artifact
  paths with fragments — this is the only per-branch difference in the
  YAML body. Do NOT emit `## Constraints` / `## Open questions` blocks
  from the code branch; those are documentation-branch-only per C18.
- Fixture location: put the code-branch worked example under
  `schemas/omnia/briefs/fixtures/plan/analyze/legacy-code/` to mirror
  C18's `documentation/` sibling. The `plugins/spec/skills/analyze/fixtures/
  scaffold-example/` tree stays structural-only (per its README).
- Structural-metadata JSON (C20, done) lands at `.specify/plans/<name>/
  analyze/<key>/metadata.json`. The shape (v1) is pinned in
  [`plugins/spec/skills/analyze/SKILL.md` §*Structural metadata*](../plugins/spec/skills/analyze/SKILL.md):
  six required fields (`version`, `source_key`, `language`, `loc`,
  `module_count`, `top_level_modules`) in that exact order, alphabetically
  sorted `top_level_modules`, no other fields. C21 owns the per-language
  detection algorithm — what counts as a module, how `loc` is computed,
  the detected-language vocabulary for Omnia — but must not introduce new
  top-level fields (that requires an RFC + `version` bump).
- The brief MUST instruct the skill to emit `metadata.json` for every
  `--kind legacy-code` invocation, byte-stable across re-runs on
  unchanged inputs. The metadata-file path is outside the brief's
  capability-summary output contract; it's a required side-effect slot.
- **Repo:** `specify/`
- **RFC:** §*Plan-time analysis, define-time extraction*
- **Files:**
  - `schemas/omnia/briefs/plan/analyze.md` (replace the C18 placeholder
    body under `## Legacy-code branch`).
  - `schemas/omnia/briefs/fixtures/plan/analyze/legacy-code/` (new) —
    mirrors the C18 `documentation/` sibling.
- **Scope:**
  - Append the code-branch algorithm: cluster via import graph + endpoint
    names + docstrings + test names + READMEs; emit capability summaries with
    `sources:` file-hint lists, `depends-on:`, `hints.entry_points`,
    `hints.external_deps`, and a `confidence` marker (`high | medium | low`).
  - Emit the structural-metadata JSON in the slot from C20.
  - Pin a worked example on a small monolith fixture matching the RFC's
    `user-registration` sample entry byte-for-byte.
- **Done when:**
  - `make checks` green.
  - Re-running on the fixture is byte-equivalent.
- **Depends on:** C18, C20

### C22 · Monolith fixture + expected capability inventory — **DONE**

Resolved decisions (carry forward to C23 / C24):
- Fixture landed at `plugins/spec/skills/plan/fixtures/discovery/monolith/`
  (sibling of `mixed-inputs/`), picked build-option **(c)** — a purpose-
  built three-capability TypeScript tree authored fresh rather than
  reusing the C21 Omnia four-capability tree. Option (a) (trim C21) was
  rejected because the C22 guardrails forbid mutating the C21 fixture;
  option (b) (hide `billing-subscription` in expected output) was
  rejected as a drift trap.
- The fixture pins the **post-C23 Stage B steady state**: unified
  fenced-YAML capability summaries only, no Stage A demarcation
  comment, no pre-RFC-3 `/spec:extract` bullet blocks. During Stage A
  (C19–C22) the discovery brief's `/spec:extract` dispatch produces a
  different shape on the same inputs; that interim state is already
  pinned by `mixed-inputs/` and is not duplicated here. `make checks`
  validates frontmatter + cross-links + schema shapes, not runtime
  brief behaviour, so a forward-looking fixture is tractable today and
  becomes an executable round-trip gate once C23 lands.
- The `user-registration` block in `expected/discovery.md` is
  byte-identical to the canonical RFC sample in
  [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](rfc-3a-monoliths.md)
  and to the C21 Omnia fixture's rendering at
  `schemas/omnia/briefs/fixtures/plan/analyze/legacy-code/expected/discovery.md`.
  Any change to this block requires a coordinated update across all
  three pins.
- `expected/plans/traffic/analyze/monolith/metadata.json` populates
  the v1 shape pinned in C20: `version`, `source_key`, `language`,
  `loc: 74`, `module_count: 4`, `top_level_modules: [src/auth,
  src/common, src/users]`. The metric values are the C21 four-capability
  numbers minus `src/billing/` (removed `stripe` dep, one top-level
  module, ~28 LOC).
- **Repo:** `specify/`
- **RFC:** §*Plan-time analysis…*, §*Propose-brief capability → slice mapping*
- **Files:**
  - `plugins/spec/skills/plan/fixtures/discovery/monolith/` —
    `invocation.txt`, `inputs/` (package.json + four `.ts` files + per-dir
    README), `expected/discovery.md`,
    `expected/plans/traffic/analyze/monolith/metadata.json`, `README.md`,
    `notes.md`.
  - `plugins/spec/skills/plan/fixtures/discovery/mixed-inputs/notes.md` —
    updated forward-pointer ("landed at `../monolith/`") replaces the
    pre-C22 placeholder path (`fixtures/discovery-monolith/`).
- **Scope:**
  - Build a small-but-realistic 3-capability monolith (user-registration,
    email-verification, shared-validation) with import edges and docstrings
    sufficient to exercise the clustering heuristic.
  - Pin the expected capability inventory.
- **Done when:**
  - Fixture round-trips under `make checks`.
- **Depends on:** C21

### C23 · Discovery brief: switch legacy-code inputs to `/spec:analyze`

Carry-forward from C19:
- C19 left the discovery brief with a Stage A interim section
  `### kind: legacy-code (Stage A interim)` that still shells out to
  `/spec:extract`, plus an explicit **demarcation comment**
  `<!-- stage-a: pre-RFC-3 extract output below; removed in C23 -->`
  in the combined `discovery.md` output. C23 removes both:
  the `legacy-code` branch's body switches to
  `/spec:analyze --kind legacy-code`, and the demarcation comment
  disappears so the two capability-summary shapes collapse into one
  alphabetically-sorted block under `## Capability inventory`.
- The `--source-key` selection rules pinned in C19 carry over
  verbatim: `--source <k>=<p>` → `<k>`, `--against <p>` → `against`,
  `initiative.md:inputs[]` → basename without extension (kebab-cased).
  The documentation-branch rules also stay put — C23 is a pure
  legacy-code-branch rewrite.
- The C19 mixed-inputs fixture at
  `plugins/spec/skills/plan/fixtures/discovery/mixed-inputs/` pins
  the Stage A shape. C23 MUST rewrite that fixture's
  `expected/discovery.md`: drop the demarcation comment, convert the
  two `### ingest-*` bullet blocks to fenced-YAML capability
  summaries, and re-sort the full `## Capability inventory` block
  alphabetically across both kinds. The monolith fixture added in
  C22 (`plugins/spec/skills/plan/fixtures/discovery/monolith/`) is
  the primary exercise target — C22 already pinned the **post-C23
  Stage B shape** there, so C23's acceptance test is a byte-for-byte
  round-trip of `expected/discovery.md` +
  `expected/plans/traffic/analyze/monolith/metadata.json` against
  the post-C23 discovery brief running on `monolith/inputs/`. No
  edits to the C22 fixture should be required when C23 lands; if
  they are, that's a C22 / C23 contract mismatch to resolve in the
  C23 commit. Mixed-inputs stays as a two-kinds regression smoke
  test.
- Update `plugins/spec/skills/plan/SKILL.md` — the dispatch sentence
  currently reads "legacy-code dispatch still routes through
  `/spec:extract` during Stage A and moves to `/spec:analyze --kind
  legacy-code` in RFC-3a C23"; C23 flips it to "both documentation
  and legacy-code inputs dispatch to `/spec:analyze`". Step 3(a)
  gets the same treatment.
- **Repo:** `specify/`
- **RFC:** §*Discovery dispatch*, §*Staged rollout* (Stage B call-site move)
- **Files:**
  - `schemas/omnia/briefs/plan/discovery.md`
  - `plugins/spec/skills/plan/SKILL.md` (dispatch sentence + Step 3(a)).
  - `plugins/spec/skills/plan/fixtures/discovery/mixed-inputs/expected/discovery.md`
    (drop demarcation, convert bullet blocks, re-sort).
  - `plugins/spec/skills/plan/fixtures/discovery/mixed-inputs/notes.md`
    (collapse the two-shape explainer to a one-paragraph historical
    note, or delete outright).
- **Scope:**
  - Replace the remaining `/spec:extract` invocation inside the discovery
    brief with `/spec:analyze --kind legacy-code`.
  - Delete any plan-time `.specify/plans/<name>/extract/<key>/` scratch paths
    from the discovery contract — the extract call-site has fully moved to
    `/spec:define` time (Stage A).
  - Drop the `<!-- stage-a: ... -->` demarcation comment from the
    brief's `## Merge rule` and `## Output` sections; collapse the
    two-shape Output example into the single capability-summary
    shape.
- **Done when:**
  - Discovery re-run on the C22 monolith fixture
    (`plugins/spec/skills/plan/fixtures/discovery/monolith/inputs/`)
    reproduces `expected/discovery.md` and
    `expected/plans/traffic/analyze/monolith/metadata.json`
    byte-for-byte using `/spec:analyze` only.
  - C19 mixed-inputs fixture re-cut and passing.
  - `make checks` green.
- **Depends on:** C19, C21, C22

### C24 · Propose brief: 1:1 capability → slice mapping

Carry-forward from C19 / C23:
- C19 left the Stage A interim `discovery.md` carrying **two shapes**
  — fenced-YAML capability summaries (documentation branch) and
  pre-RFC-3 `/spec:extract` bullet blocks (legacy-code branch) —
  separated by the demarcation comment
  `<!-- stage-a: pre-RFC-3 extract output below; removed in C23 -->`.
  C23 collapses that into the unified YAML shape. C24 MUST target the
  **post-C23** `discovery.md`: its slice-decomposition heuristic
  parses fenced-YAML capability blocks only and assumes alphabetic
  sorting across all kinds.
- Order this chunk strictly after C23 (already encoded in
  `Depends on`). If C24 lands before C23 the bullet-shape legacy-code
  entries are unparseable by the new propose heuristic and every
  monolith-sourced capability silently drops out of the plan.
- Until C23 lands, operators who run `/spec:plan` with mixed inputs
  keep the pre-RFC-3 propose behaviour on the legacy-code side of
  `discovery.md`. The C19 mixed-inputs fixture's `notes.md` already
  documents this; no separate operator-facing warning is added by
  C19 / C23.
- **Repo:** `specify/`
- **RFC:** §*Propose-brief capability → slice mapping*
- **Files:**
  - `schemas/omnia/briefs/plan/propose.md`
  - `plugins/spec/skills/plan/fixtures/propose/` (update or add monolith
    fixture).
- **Scope:**
  - Rewrite propose's decomposition heuristic: one plan entry per discovered
    capability. Carry `name` from capability `name`, `sources` from the
    dispatch key, `scope.<key>.include` pre-filled from capability
    `sources:`, `depends-on` from capability edges.
  - Surface `confidence: low` capabilities with a "review before accepting"
    flag in the interactive loop.
  - Leave tangled / overlap handling as a comment pointing at C28; in this
    chunk, overlaps produce `scope-overlap` warnings only.
  - Extend `specify initiative create` call-sites in the brief to include the
    `--scope-include` flags plumbed in C03.
- **Done when:**
  - Monolith fixture (C22,
    `plugins/spec/skills/plan/fixtures/discovery/monolith/expected/discovery.md`)
    drives propose to emit three plan entries keyed by capability name
    (`user-registration`, `email-verification`, `shared-validation`),
    each carrying `sources: [monolith]`, `scope.monolith.include`
    pre-filled verbatim from the capability's `sources:` list, and
    `depends-on` edges lifted from the capability's `depends-on`
    (`user-registration` → `[email-verification, shared-validation]`).
    The C24 propose fixture consumes C22's `expected/discovery.md` as
    its starting-state input — no re-clustering, no second inference
    pass.
  - `specify initiative validate` on the produced plan is clean.
  - `make checks` green.
- **Depends on:** C03, C23

---

## Phase 7 — Stage A validator polish: monolith-scale lint

### C25 · `scope-missing-on-monolith` warning

Carry-forward from C20:
- Input file: `.specify/plans/<name>/analyze/<key>/metadata.json`, shape v1
  pinned in [`plugins/spec/skills/analyze/SKILL.md` §*Structural metadata*](../plugins/spec/skills/analyze/SKILL.md).
  Required fields relevant to this lint: `loc` (integer) and
  `module_count` (integer). Key match is the `<key>` directory segment —
  the validator iterates `.specify/plans/<name>/analyze/*/metadata.json`
  and reads `source_key` (or falls back to the directory name; they must
  agree).
- Omnia threshold: `loc >= 10_000 || module_count >= 20`. Other schemas
  own their own thresholds; C25 hardcodes Omnia's for v1 with a TODO
  comment pointing at a future schema-owned threshold slot.
- Absent-file semantics: if `metadata.json` does not exist for a given
  source key, the lint silently skips that key. Only `legacy-code`
  invocations produce the file, so documentation and greenfield sources
  never trip it.
- Malformed-file semantics: if `metadata.json` fails to parse or is
  missing any of the v1 required fields, emit a distinct diagnostic
  (`invalid-analyze-metadata`) rather than silently skipping. Drift
  across runs surfaces as file-level diff, not validator behaviour.
- **Repo:** `specify-cli/`
- **RFC:** §*Validation* (monolith-scale lint)
- **Files:**
  - `crates/validate/src/run.rs`
  - tests.
- **Scope:**
  - Read the `/spec:analyze` structural metadata from
    `.specify/plans/<name>/analyze/<key>/metadata.json` (slot and shape
    pinned in C20).
  - Fire a warning for changes whose `sources[<key>]` is monolith-scale
    (Omnia default: `loc >= 10_000 || module_count >= 20`) and which
    carry no `scope.<key>` entry.
  - Diagnostic text matches the RFC's example (cites module count and
    LOC from `metadata.json`).
  - Absent metadata → silently skip the check (small-legacy /
    documentation / greenfield never trip it). Malformed metadata →
    `invalid-analyze-metadata` error.
- **Done when:**
  - New test reproduces the RFC's example diagnostic for a synthetic
    monolith.
  - `cargo test -p specify-validate` green.
- **Depends on:** C04, C20

---

## Phase 8 — Stage C: manifest-based slices for tangled cases

### C26 · Manifest YAML + working-directory additions
- **Repo:** `specify/` (primary) and `specify-cli/` (validator extension)
- **RFC:** §*Manifest shape*, §*Working-directory additions*
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md` — add `slices/` to the working dir
    diagram.
  - `plugins/spec/skills/extract/SKILL.md` — document manifest loader.
  - `crates/validate/src/run.rs` — enforce manifest shape on validate.
- **Scope:**
  - Pin the manifest shape from the RFC: `{ version: 1, include: [str] }`
    with paths relative to `sources[<key>]`.
  - Document the mutual exclusion between `--manifest` and `--include` /
    `--exclude` per source key (validator surfaces it; CLI rejected at write
    time in C03 already).
  - Update the `/spec:plan` working-directory diagram to include
    `.specify/plans/<name>/slices/`.
- **Done when:**
  - `specify initiative validate` catches a plan that sets both `manifest`
    and `include` for the same source.
  - `make checks` green.
- **Depends on:** C04, C06

### C27 · Propose brief emits manifest-based slices for tangled capabilities

Carry-forward from C24:
- C24 rewrote `schemas/omnia/briefs/plan/propose.md` around the 1:1
  capability → slice mapping and left a `<!-- TODO(C27) -->` comment
  in the "Tangled / overlapping capabilities" section as the drop-in
  point for manifest-emission prose. C27 replaces the comment with
  the manifest-emission rule (pointer format + `--scope-manifest`
  invocation).
- The C24 monolith fixture
  (`plugins/spec/skills/plan/fixtures/propose/monolith/`) ships with
  a known `scope-overlap` on `src/auth/verify.ts` (in both
  `email-verification` and `user-registration`). C27 extends that
  fixture with a second expected output shape
  (`expected/plan-manifest.yaml` + `expected/slices/<change>.yaml`)
  using `scope.monolith.manifest` for the overlap, driven by the
  `confidence: low` capability the C27 fixture author adds.
- **Repo:** `specify/`
- **RFC:** §*Propose-brief capability → slice mapping* (tangled-cases paragraph),
  §*Staged rollout* (Stage C)
- **Files:**
  - `schemas/omnia/briefs/plan/propose.md`
  - a tangled monolith fixture under the plan skill's fixtures dir.
- **Scope:**
  - For capabilities whose `sources:` overlap with another capability's (or
    whose clean glob expression fails), write the explicit file list to
    `.specify/plans/<name>/slices/<change>.yaml` and set
    `scope.<src>.manifest` on the plan entry instead of `scope.<src>.include`.
  - Low-confidence capabilities still go through the human accept / edit /
    reject / abort loop.
  - Fixture demonstrates at least one tangled case → manifest emission and
    at least one clean case → glob emission in the same run.
- **Done when:**
  - `make checks` green.
  - Validator (C26) accepts the emitted plan.
- **Depends on:** C24, C26

---

## Phase 9 — Layer 2: multi-repo (sync peers)

Layer 2 can land any time after Phase 4 (registry parsing). It composes
unchanged with Phases 5–8.

### C28 · `registry.yaml` multi-project parsing + validation
- **Repo:** `specify-cli/`
- **RFC:** §*The Registry (multi-project)*
- **Files:**
  - `crates/schema/src/registry.rs`
  - `crates/validate/src/run.rs`
  - tests.
- **Scope:**
  - Extend the parser to handle `len(projects) > 1`, distinct names,
    well-formed URLs (local `.` / relative path / `git@…:org/repo.git` /
    `https://…`), and duplicate-name detection.
  - `specify initiative registry validate` returns non-zero on bad shape.
- **Done when:**
  - `cargo test` green on both single- and multi-project fixtures.
- **Depends on:** C12, C13

### C29 · `specify initiative workspace {sync, status}` CLI
- **Repo:** `specify-cli/`
- **RFC:** §*The sync-peers phase*, §*The workspace layout*, §*CLI surface additions*
- **Files:**
  - `src/main.rs`
  - new crate module or extension of an existing one.
  - `crates/validate/src/run.rs` (optional: `.gitignore` assertion helper).
- **Scope:**
  - `workspace sync`:
    - Read `registry.yaml`; for each project clone/fetch into
      `.specify/workspace/<name>/` (local `url: .` or relative → symlink, no
      clone).
    - Assert `.specify/workspace/` is present in `.gitignore`; append if
      missing. `specify init` should do this as part of scaffolding — land
      that update too.
    - No writes ever land in the peer clones themselves.
  - `workspace status`: prints per-peer state (synced commit, dirty flag,
    symlink vs clone).
  - Deterministic CLI — no agent in the loop; see RFC §*Alternatives
    Considered — `/rt:git-cloner`*.
- **Done when:**
  - `cargo test` green against a scripted local-only two-project fixture.
  - Bare repo without `registry.yaml` → `workspace sync` exits 0 with a
    clear "no registry" message.
- **Depends on:** C28

### C30 · `/spec:plan` runs sync-peers phase when `len(projects) > 1`
- **Repo:** `specify/`
- **RFC:** §*Planning Model Overview* (3-phase flow), §*The sync-peers phase*,
  §*The flow* (Layer 2)
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md`
  - fixtures.
- **Scope:**
  - Insert the sync-peers phase between *analyse inputs* and *generate plan*
    when the registry declares >1 project. The phase shells out to
    `specify initiative workspace sync`, then walks each peer's
    `.specify/` tree (baseline specs, active plans, schema) and writes
    `.specify/plans/<name>/workspace.md` (peer-by-peer summary).
  - Document `workspace.md` as a second input to the propose brief.
  - Single-writer invariant unchanged — the phase only writes under
    `.specify/plans/<name>/` and `.specify/workspace/`.
- **Done when:**
  - Multi-repo fixture produces `workspace.md` with the pinned peer
    inventory shape.
  - `make checks` green.
- **Depends on:** C29

### C31 · Propose brief consumes `workspace.md` (cross-repo slices)

Carry-forward from C24:
- C24 established the 1:1 capability → slice rule in
  `schemas/omnia/briefs/plan/propose.md`, keyed off the
  `<!-- source-key: <k> -->` marker. The brief has no concept of a
  peer project today. C31 extends the mapping rule to recognise
  capabilities whose `<source-key>` resolves to a peer registry
  entry (not a local `sources:` map key) and emits a plan entry
  where `sources` names the peer. The 1:1 rule is preserved — one
  capability, one plan entry; the delta is that the entry may point
  at a peer project.
- C24's mixed-inputs follow-up (noted under C31's Deliverables) is
  the natural home for the second propose fixture — a
  documentation-only fixture with `sources: []` entries. If C31
  picks that up, co-locate it with the cross-repo fixture so the
  two doc/cross-repo cases land together.
- **Repo:** `specify/`
- **RFC:** §*Plan output shape*
- **Files:**
  - `schemas/omnia/briefs/plan/propose.md`
  - fixtures.
- **Scope:**
  - Where a capability needs work in a peer project, emit a plan entry whose
    `sources` (and/or `affects`) references the peer by its registry name.
  - Document that executing such entries requires RFC-3b federation
    (out of scope here) — plan authoring is the ceiling in RFC-3a.
- **Done when:**
  - Multi-repo fixture produces a `plan.yaml` that `specify initiative
    validate` accepts (with any cross-repo reference shapes deferred to
    RFC-3b noted as warnings, not errors).
- **Depends on:** C24, C30

### C32 · `--dry-run` and `--extend` semantics for sync-peers
- **Repo:** `specify/`
- **RFC:** §*`--dry-run` and `--extend` under Layer 2*
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md`
  - fixtures.
- **Scope:**
  - `--dry-run`: inventory whatever is already cloned but do NOT clone new
    repos, write to `.specify/workspace/`, or write `workspace.md`.
  - `--extend`: reuse existing clones; never implicitly fetch. Operators
    refresh via `specify initiative workspace sync` between runs.
  - Banner on dry-run output is unchanged (`[dry-run] /spec:plan — <name>`).
- **Done when:**
  - Dry-run + extend fixtures pin the behaviour.
  - `make checks` green.
- **Depends on:** C30

---

## Phase 10 — Archive, docs, and wrap

### C33 · Archive sweep covers `workspace.md`, `initiative.md`, `slices/`
- **Repo:** `specify-cli/`
- **RFC:** §*Working-directory additions*, §*Relation to RFC-2*
- **Files:**
  - `crates/change/src/actions.rs` (archive action, if that's where the
    sweep lives — otherwise the appropriate crate)
  - tests.
- **Scope:**
  - Ensure `specify initiative archive` sweeps the new artifacts under
    `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`:
    - `initiative.md` (if present at repo root under `.specify/`)
    - `workspace.md`
    - `slices/` directory.
  - Fixture round-trip.
- **Done when:**
  - `cargo test` green.
- **Depends on:** C14, C27, C30

### C34 · Update `/spec:plan` SKILL.md top-matter + AGENTS.md
- **Repo:** `specify/`
- **RFC:** §*Diagram labels → skills and CLI*, §*Relation to RFC-2*
- **Files:**
  - `plugins/spec/skills/plan/SKILL.md`
  - `AGENTS.md`
  - `README.md` (top-level) if there are any workflow descriptions that
    mention `/spec:plan`.
- **Scope:**
  - Rewrite the overview and status callout to reflect the registry- /
    brief-aware skill, the fixed three-phase flow, and the `/spec:analyze`
    skill's role as the sole plan-time discovery skill.
  - Cross-link `rfc-3a-monoliths.md` and `rfc-3b-layer-3.md`.
  - `make checks` green (including `scripts/checks.ts` consistency lints).
- **Done when:**
  - `make checks` green.
- **Depends on:** C24, C31, C32

### C35 · End-to-end test harness
- **Repo:** `specify-cli/`
- **RFC:** *(covers all of the above)*
- **Files:**
  - `tests/initiative.rs`, `tests/plan.rs`, `tests/e2e.rs`.
- **Scope:**
  - Add three e2e scenarios that exercise the full shapes:
    1. Single-repo monolith with `initiative.md` only (Stage A+B path).
    2. Single-repo tangled monolith forcing a manifest slice (Stage C).
    3. Multi-repo (two projects, both local) driving the sync-peers phase and
       producing a cross-repo plan entry.
  - Each scenario runs against pinned fixtures and asserts
    `specify initiative validate` exit code + pinned `plan.yaml` +
    `.specify/plans/<name>/*.md` outputs.
- **Done when:**
  - `cargo test --all` green.
  - CI pipeline green for both repos.
- **Depends on:** C24, C27, C31, C33

---

## Implementation notes (C26–C35 close-out)

Per-chunk **Files** lists above are authoritative for intent; the
following records where the work actually landed:

- **C26** — Manifest validation is in **`specify-cli/crates/change/src/plan.rs`**
  (`check_scope_manifest_shapes`, `Plan::validate`), not
  `crates/validate/src/run.rs`. `Scope` deserialisation (C03) still
  enforces `manifest` ⊕ `include`/`exclude` at parse time.
- **C27** — Stage C prose + `--scope-manifest` in
  `schemas/omnia/briefs/plan/propose.md`; tangled fixture additions under
  `plugins/spec/skills/plan/fixtures/propose/monolith/` (`discovery-manifest.md`,
  `expected/plan-manifest.yaml`, `expected/slices/user-registration.yaml`,
  `expected/create-invocations-manifest.md`).
- **C28** — URL / symlink classification in **`specify-cli/crates/schema/src/registry.rs`**
  (`validate_shape`, `RegistryProject::url_materialises_as_symlink`).
- **C29** — **`specify-cli/src/workspace.rs`** + `initiative workspace {sync,status}` in
  `src/main.rs`; `specify init` gitignore upsert for `.specify/workspace/`.
- **C30–C32, C34** — **`plugins/spec/skills/plan/SKILL.md`** (step 3(a½) sync-peers,
  `workspace.md` pin, `--dry-run` / `--extend` rules), fixture
  `plugins/spec/skills/plan/fixtures/plan-layer2/workspace.md`,
  **`AGENTS.md`** workflow refresh + RFC links.
- **C31** — `workspace.md` listed as propose **Input**; peer execution
  explicitly deferred to **RFC-3b** (plan entries still use only keys in
  `plan.sources` for RFC-3a validate cleanliness).
- **C33** — Whole-tree co-move of `.specify/plans/<name>/` already carries
  `workspace.md` + `slices/`; **`specify-cli/crates/change/src/plan.rs`**
  adds `archive_moves_workspace_md_and_slices_with_plan_working_dir`.
- **C35** — **`specify-cli/tests/initiative.rs`** (`rfc3a_c35_*`) smoke-tests
  initiative+brief+validate, absent-registry workspace sync, two-local-peer
  symlink sync, manifest scope + validate.

---

## Cross-cutting checks (run after every phase)

- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
  and `cargo test --workspace` in `specify-cli/`.
- `make checks` in `specify/`.
- `specify initiative validate` against the canonical fixtures.
- Single-writer invariant spot-check: `rg` over the repo for direct edits to
  `plan.yaml` outside `Plan::{init, create, amend, transition}`; should
  return zero hits outside the `change` crate.

## Deferrals (explicitly not in this plan)

- **RFC-3b** (cross-repo `@peer:capability` references, contract reconciliation,
  peer status roll-up) — covered separately.
- **Symbol-level scope** — file-level only in v1 (§*Non-goals*).
- **Auto-sub-slicing at define time** — amend edge already composes with
  `scope` (§*Non-goals*).
- **`planning.yaml`** — intentionally absent; the fixed flow *is* the only
  shape (§*Alternatives Considered — Configurable planning pipeline*).
