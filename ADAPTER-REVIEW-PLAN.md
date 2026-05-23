# ADAPTER-REVIEW.md — Execution plan

Resolves the issues raised in [ADAPTER-REVIEW.md](ADAPTER-REVIEW.md) before the 2.0 wire-format freeze.

Each task below is sized to be implementable by a single subagent without overflowing context. Tasks are grouped into waves; **all tasks within a wave can run in parallel by separate subagents** unless a "Depends on" note says otherwise. Dropped review items (`1.A3`, `1.C2`, `1.C4`) are listed at the end with the rationale.

Cross-repo note: this work spans both `augentic/specify` (manifests, briefs, docs) and `augentic/specify-cli` (schemas, Rust types, CLI surface). Every Wave-1 schema task lands schema + Rust + tests + DECISIONS.md in the *same* PR — they are the wire contract.

---

## Locked decisions (operator-confirmed)

| Review item | Decision |
|---|---|
| 1.A2 — tool version pin format | **Semver only** (`x.y.z`). Tools without a release must cut one before being declared. |
| 1.A3 — `tools[].permissions[]` grammar | **Defer.** Leave free-form; revisit when the WASI runtime permission set stabilises. Dropped from this plan. |
| 1.C1 — adapter name uniqueness across axes | **Enforce unique names** across axes. Reject collisions at `specify init` and `Adapter::resolve`. |
| 1.C2 / 1.C4 — reserve `requires.cli` / `deprecated:` fields | **Neither.** Accept the cost the first time we need them. Dropped from this plan. |
| 2.A1 — `plan.yaml` sources binding | **Land the structured loader** (the deferred W0.3 task). `Plan::sources` becomes `BTreeMap<String, SourceBinding>`. |
| 2.A2 — journal failure events | **Add the missing variants** (`slice.build.failed`, `slice.merge.conflicted`, `plan.transition.archived`) to the closed taxonomy. |
| 2.A3 — `@vN` target suffix policy | **Required and parsed.** `target` must carry `@vN`, and N must agree with the resolved adapter's `version` field. |
| 2.B1 — `target` × `project` cross-field rule | **In schema** as a cross-field `oneOf`/`anyOf`. External consumers get the rule too. |
| 2.D — cache co-tenancy | **Formally separate.** `.specify/.cache/manifests/{sources,targets}/<name>/` for manifests; `.specify/.cache/extractions/<adapter>/<fp>/` for results. |

---

## Wave 0 — Foundation reshape

Three independent reshape tasks. They touch disjoint files and can run as three parallel subagents. Everything in Wave 1+ depends on at least one of these.

### Task A — Collapse `operations[]` into `briefs.keys()` (review 1.A1)

**Scope.** Drop the decorative `operations:` array from the manifest, schemas, and Rust struct.

**Files (specify):**
- `adapters/sources/*/adapter.yaml` (4 manifests)
- `adapters/targets/*/adapter.yaml` (4 manifests, plus `default/`)

**Files (specify-cli):**
- `schemas/adapter.schema.json` — remove `operations` property
- `schemas/source.schema.json` — remove `operations` property
- `schemas/target.schema.json` — remove `operations` property
- `crates/domain/src/adapter/core.rs` — remove `pub operations: Vec<String>`; expose `Adapter::operations()` deriving from `briefs.keys()` if any caller needs the iterator
- Any call site found by `rg 'manifest\.operations\b|\.operations\.iter' specify-cli/`

**Verification:**

```bash
rg -n '^operations:' adapters/
# (no matches)
rg -n 'manifest\.operations\b|\.operations\.iter' specify-cli/src specify-cli/crates
# (only the operations() accessor remains, or none at all)
cd specify-cli && cargo make ci
cd specify    && make checks
```

**Effort.** Low. ~9 manifest line deletions, 3 schema deletions, 1 Rust field removal, 0–2 call-site updates.

---

### Task B — Land structured sources loader (review 2.A1)

**Scope.** Replace the bare-string `Plan::sources` with the structured `SourceBinding` shape the schema already documents. Drop the `oneOf` 1.x compat branch from the schema.

**Files (specify-cli):**
- `crates/domain/src/change/plan/core/model.rs` — `pub sources: BTreeMap<String, String>` → `BTreeMap<String, SourceBinding>`. Mirror the existing `SliceSourceBinding::{Bare, Structured}` pattern at `model.rs:254`. Remove the deferred-TODO comment block at `model.rs:97`.
- `src/commands/plan/create.rs` — `build_source_map` returns the structured map. Add CLI flags for structured authoring (one syntax option: `--source key=adapter:path` or `--source key=adapter:value:<literal>` — pick during implementation and document in DECISIONS.md).
- `schemas/plan/plan.schema.json` — drop the `oneOf` branch on `sourceBinding`; keep only the structured object form.
- Plan validators / loaders touching `Plan::sources`.
- Test fixtures under `specify-cli/tests/fixtures/` and `specify/tests/cross-repo/` if any contain bare-string sources in `plan.yaml`.

**Files (specify):**
- `plugins/spec/skills/plan/SKILL.md` and any plan-authoring skill body that demonstrates `sources:` in `plan.yaml` — convert demo blocks to the structured form.
- `tests/cross-repo/scenario.md` / `tests/plan/*.md` fixtures.

**Verification:**

```bash
rg -n 'pub sources: BTreeMap<String, String>' specify-cli/crates/domain
# (no matches — replaced by SourceBinding)
rg -n '"adapter":' specify-cli/tests/fixtures
# structured form appears in at least one fixture
cd specify-cli && cargo make ci
cd specify    && make checks
```

**Effort.** Medium. Largest task in Wave 0 — touches the model, CLI flags, schema, and downstream skill demos.

---

### Task F — Closed journal failure events (review 2.A2)

**Scope.** Add `slice.build.failed`, `slice.merge.conflicted`, and `plan.transition.archived` to the closed `EventKind` taxonomy. Wire shape only — emitter sites can land later.

**Files (specify-cli):**
- `crates/domain/src/journal.rs` — three new `EventKind` variants with kebab-case `#[serde(rename = "…")]` discriminants and matching `snake_case` Rust idents.
- `DECISIONS.md` — extend the §"Journal event names" table with one row per new event.
- Any closed-set match on `EventKind` (compiler will surface them).

**Verification:**

```bash
rg -n 'SliceBuildFailed|SliceMergeConflicted|PlanTransitionArchived' specify-cli/crates/domain/src/journal.rs
# (three variants present)
cd specify-cli && cargo make ci
```

**Effort.** Low. ~30 LOC + 3 doc rows.

**Independent of Wave 0 A/B.** Listed in Wave 0 because it's a wire-format change with no upstream dependency.

---

## Wave 1 — Schema hardening and Rust cleanup

Five tasks, all parallel-safe with each other. Each depends on a specific Wave 0 task as noted.

### Task C — Adapter schema requireds (review 1.A2 + 1.A4)

**Depends on:** Task A (Wave 0). Same schema files.

**Scope.** Make `description` and `tools[].version` required on every adapter manifest schema. Lock semver-only on `version` via schema description and a `pattern`.

**Files (specify-cli):**
- `schemas/adapter.schema.json` — add `description` to top-level `required`; on `toolDeclaration` add `version` to `required` and set `"pattern": "^\\d+\\.\\d+\\.\\d+(-[0-9A-Za-z.-]+)?$"` (or equivalent strict semver).
- `schemas/source.schema.json` — same.
- `schemas/target.schema.json` — same.
- `DECISIONS.md` — entry §"Adapter manifest requireds" recording semver-only choice and the "every declared tool must pin" reproducibility argument.

**Files (specify):**
- `adapters/sources/code-runtime/adapter.yaml` — add `version:` pin to `fixture-index` (the one currently-unpinned declaration).
- Any other in-tree manifest the schema rejects after the change. (Spec'd to be zero, since every manifest already sets `description`; double-check.)

**Verification:**

```bash
rg -nA2 '^tools:' adapters/
# every block under `tools:` shows a `version:` line
cd specify-cli && cargo make ci
cd specify    && make checks
```

**Effort.** Low. Three schema edits, one manifest pin, one DECISIONS.md entry.

---

### Task D — Plan schema hardening (review 2.A3 + 2.B1 + 2.C3)

**Depends on:** Task B (Wave 0). Same `plan.schema.json` file.

**Scope.** Lock `slices[].target` to `name@vN` form and reconcile against the resolved adapter's `version`. Move the `target × project` cross-field rule into the schema. Document the `@vN` policy.

**Files (specify-cli):**
- `schemas/plan/plan.schema.json`:
  - `target`: replace free string with `"pattern": "^[a-z][a-z0-9-]*@v\\d+$"`.
  - Add cross-field `oneOf` on `slices[]` items: either `project` is a non-null string OR `target` is present (encoding the rule from `model.rs:122`).
- `crates/domain/src/change/plan/core/model.rs` — parse `target` into `(name, version)` tuple; reject if the suffix is missing.
- `crates/domain/src/change/plan/core/validate.rs` (or wherever resolution happens) — after `Adapter::resolve(Axis::Target, name, project_dir)`, assert `parsed.version == adapter.version`; emit a typed `Error::Validation` with a kebab discriminant (`plan-target-version-mismatch` or similar — pick during implementation).
- Tests in `crates/domain/src/change/plan/core/validate/tests.rs` covering the mismatch path.
- `DECISIONS.md` — entry §"Target adapter suffix policy" recording the "`@vN` required and parsed" decision.

**Verification:**

```bash
rg -n 'adapter: "[a-z][a-z0-9-]*"' specify-cli/crates/domain
# (no matches — every fixture carries @vN)
cd specify-cli && cargo make ci
```

**Effort.** Medium. Schema regex + cross-field rule + Rust parser + reconciliation + tests + DECISIONS.md.

---

### Task E — Type `Operation` into the Rust manifest (review 1.B1)

**Depends on:** Task A (Wave 0). Same Rust file (`adapter/core.rs`).

**Scope.** Push the string boundary out to the YAML parse step. After 1.A1 collapses `operations[]`, `briefs.keys()` is the canonical iterator — type it.

**Files (specify-cli):**
- `crates/domain/src/adapter/core.rs` — choose one of:
  - Split `Adapter` into `SourceAdapter` / `TargetAdapter` structs.
  - Keep one generic `Adapter<Op>` parameterised over `Op: AdapterOperation`.

  Either way: `briefs: BTreeMap<SourceOperation, String>` / `BTreeMap<TargetOperation, String>`; `brief_path(operation: <Operation>)` becomes typed.
- `crates/domain/src/adapter/operation.rs` — add any missing `Operation` enum if going the generic route.
- All call sites of `brief_path("…")` — convert to enum arms.

**Verification:**

```bash
rg -n 'brief_path\(.*?"\w+"\)' specify-cli/
# (no string-literal call sites remain)
cd specify-cli && cargo make ci
```

**Effort.** Medium. The mechanical refactor is small; choosing between "split struct" vs "generic parameter" is a 30-minute design call. Document the choice in `DECISIONS.md`.

---

### Task G — Enforce unique adapter names across axes (review 1.C1)

**Depends on:** Task A (Wave 0). Same Rust file (`adapter/core.rs`).

**Scope.** Reject any project where the same `name` lives under both `adapters/sources/` and `adapters/targets/`.

**Files (specify-cli):**
- `crates/domain/src/adapter/core.rs` — in `Adapter::resolve`, after locating the requested axis, also probe the sibling axis under the same name; if both resolve, return `Error::Validation` with a kebab discriminant (`adapter-name-axis-collision` or similar).
- `src/commands/init.rs` (or whichever handler implements `specify init`) — same check at scaffold time, with a clear hint pointing the operator at both directories.
- Tests covering the collision path.
- `DECISIONS.md` — entry §"Adapter name uniqueness" recording the cross-axis uniqueness invariant and the journal/error-message contract.

**Files (specify):**
- `AGENTS.md` (§Vocabulary) — one sentence noting cross-axis uniqueness so the rule is discoverable from the docs entry point.

**Verification:**

```bash
cd specify-cli && cargo make ci
cd specify    && make checks
```

**Effort.** Low. ~40 LOC + one test + two doc entries.

---

### Task H — Formally separate manifest and extraction caches (review 2.D)

**Depends on:** Task A (Wave 0). Touches `adapter/core.rs` near the `Adapter::locate` cache region.

**Scope.** Split the co-tenancy at `.specify/.cache/adapters/<axis>/<name>/` into two distinct trees.

**Files (specify-cli):**
- `crates/domain/src/adapter/core.rs` — `Adapter::locate` (and any helpers) write/read manifest cache under `.specify/.cache/manifests/{sources,targets}/<name>/`. Remove the "probe for `adapter.yaml`" heuristic comment.
- `crates/domain/src/` wherever RFC-27 §D8 extraction cache writes — relocate to `.specify/.cache/extractions/<adapter>/<fingerprint>/` (with `index.jsonl` co-located).
- `.gitignore` or any path constants — update.
- `DECISIONS.md` — entry §"Cache layout" recording the split and the rationale (no probe heuristic; each cache owns its own root).

**Files (specify):**
- `AGENTS.md` §Vocabulary — update the RFC-27 §D8 paragraph that names `.specify/.cache/adapters/sources/<adapter>/index.jsonl`.
- Any reference doc under `docs/` mentioning the old cache path (`rg '\.specify/\.cache/adapters'` to find them).

**Verification:**

```bash
rg -n '\.specify/\.cache/adapters' specify-cli/ specify/
# (no matches — replaced by manifests/ or extractions/)
cd specify-cli && cargo make ci
cd specify    && make checks
```

**Effort.** Medium. The path change is small but ripples through cache helpers, journal events that carry paths, and several reference docs.

---

## Wave 2 — Docs + README cleanup

Two tasks, both parallel. Both depend on every Wave-0 and Wave-1 task being merged so the docs describe the final state.

### Task I — Rewrite `schemas/README.md` (review 2.A4)

**Depends on:** Tasks A, B, C, D (every schema change must have landed).

**Scope.** Fix the documented schema inventory.

**Files (specify-cli):**
- `schemas/README.md`:
  - Replace the `plugin.schema.json` row — that file no longer exists. `adapter.schema.json` is now the shared shape.
  - Drop the "Pre-RFC-25; retained for v1.x manifests" qualifier on `adapter.schema.json`.
  - Add a row for `tool.schema.json` (currently missing from the table).
  - Replace "RFC-13 §Adapter manifest" references with "RFC-25 §Adapter implementation shape".

**Verification:**

```bash
rg -n 'plugin\.schema\.json|RFC-13' specify-cli/schemas/README.md
# (no matches)
rg -n 'tool\.schema\.json' specify-cli/schemas/README.md
# (one match, in the table)
```

**Effort.** Trivial. Pure doc edit.

---

### Task J — Roundup of DECISIONS.md and explanatory doc entries

**Depends on:** None (these are pure-docs, no code state needed). Can also run in Wave 0 alongside A/B/F.

**Scope.** Close the small holes the review identified as "one paragraph in `DECISIONS.md`".

**Files (specify-cli) — `DECISIONS.md` additions:**
- §"Exit codes" table — add `Exit::ArgumentError → 2` row alongside `Exit::ValidationFailed → 2` (review 2.B2).
- §"Plan lifecycle" — one paragraph confirming `Status::Done` is absorbing in v1 (review 2.C1).
- §"Plan lifecycle" — one paragraph noting "archive is a filesystem operation, not a lifecycle state" (review 2.C2). Tied to Task F's `plan.transition.archived` event.
- §"Source bindings" — one sentence: "source keys are plan-scoped; each key maps to exactly one binding under `Plan::sources`, but slices may reference the same key with different candidates" (review 2.C4).

**Files (specify) — adapter-vs-plugin boundary (review 1.D):**
- `docs/explanation/adapter-anatomy.md` already exists. Append a short section ("Adapter manifests vs Cursor plugin manifests") explaining:
  - Cursor `.cursor-plugin/plugin.json` files register Cursor IDE surface (skills, rules, slash commands).
  - `adapter.yaml` files are loaded by the `specify` CLI via `Adapter::resolve(axis, name, project_dir)`.
  - The two systems are independent; they share no fields and no loader.
- Cross-link from `AGENTS.md` §Vocabulary if not already linked.

**Verification:**

```bash
cd specify-cli && cargo make ci    # DECISIONS.md is doc-checked
cd specify    && make checks
```

**Effort.** Low. ~5 short paragraphs across two repos.

---

## Dependency graph

```text
Wave 0 (parallel)
├── Task A — Collapse operations[] (1.A1)
├── Task B — Structured sources loader (2.A1)
├── Task F — Journal failure events (2.A2)
└── Task J — DECISIONS.md roundup (2.B2 + 2.C1 + 2.C2 + 2.C4 + 1.D)   ← can also wait until Wave 2

Wave 1 (parallel; each depends on Wave 0 as noted)
├── Task C — Adapter schema requireds (1.A2 + 1.A4)              [after A]
├── Task D — Plan schema hardening (2.A3 + 2.B1 + 2.C3)          [after B]
├── Task E — Type Operation into Rust manifest (1.B1)            [after A]
├── Task G — Unique adapter names across axes (1.C1)             [after A]
└── Task H — Separate manifest / extraction caches (2.D)         [after A]

Wave 2
└── Task I — Rewrite schemas/README.md (2.A4)                    [after A, B, C, D]
```

**Recommended driving order if you have only one runner:** A → B → F → C → D → E → G → H → I → J.

**Recommended parallel fan-out (e.g. four runners):**
- Run A, B, F, J in parallel as Wave 0.
- Once A merges, fan out C, E, G, H in parallel.
- Once B merges, start D.
- Once A, B, C, D have all merged, run I.

---

## Items dropped from this plan (with rationale)

| Review item | Reason |
|---|---|
| 1.A3 — close `tools[].permissions[]` grammar | Operator chose "leave free-form; defer until the runtime permission set is more stable". Reconsider in 2.1+. |
| 1.C2 — reserve `requires.cli` field | Operator chose "accept the cost the first time we need it." No schema reservation at 2.0. |
| 1.C3 — declarative brief I/O contract | ADAPTER-REVIEW.md itself says "do not land in 2.0." Defer to 2.1+. |
| 1.C4 — reserve `deprecated:` block | Same choice as 1.C2 — no schema reservation at 2.0. |

---

## Cross-cutting verification

After every wave (and definitely before the 2.0 tag), run the full suite from both repo roots:

```bash
cd specify-cli && cargo make ci    # lint + size + test + doc + vet + outdated + deny + fmt
cd specify    && make checks       # doc + workflow consistency
cd specify    && make test         # cross-repo Deno acceptance (needs SPECIFY_BIN)
```

The cross-repo test in particular validates that schema edits (Wave 1) and skill-body demo updates (Task B's downstream edits) stay coherent.
