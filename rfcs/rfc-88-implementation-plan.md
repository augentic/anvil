# RFC-88 Implementation Plan

> Status: **Draft for iteration** — not an RFC. This is the working implementation plan for [RFC-88 Detached Changes](rfc-88-detached-changes.md), structured as sequential agent-session-sized steps. The operator runs one agent session per step, strictly in sequence, committing each to the `rfc-88-impl` branch on `augentic/emery` and `augentic/emery-adapters`. One pull request per repository closes the whole plan. Git management is entirely operator-owned and appears nowhere in the steps.
>
> Companion inputs: [RFC-86](rfc-86-change-facts.md) (facts, waves, epochs — implemented), [RFC-86a](rfc-86a-gap-deferral.md) (gap deferral — implemented), [RFC-87](rfc-87-working-trees.md) (private workspaces — implemented; RFC-88 deletes its interim `apply`), [RFC-90](rfc-90-build-verification.md) / [RFC-91](rfc-91-refinement-stage.md) (implemented), [RFC-104](rfc-104-system-archaeology.md) (definition predecessor — **not implemented**; see Open Question 1), [RFC-95](rfc-95-publication-sets.md) / [RFC-96](rfc-96-concurrent-execution.md) (successors whose seams RFC-88 must preserve), and the [platform programme](platform.md) (§ "RFC-88 scope discipline" and the four internal cuts).

## How to run this plan

**Session protocol.** Every step below is designed to fit one fresh agent session without exhausting context. Each session must:

1. Read `rfcs/rfc-88-detached-changes.md`, this plan's step section (plus its listed RFC anchors), and the step's listed code areas. Do not re-read the whole RFC corpus.
2. Implement exactly the step's scope. Out-of-scope items are listed per step and belong to later steps — do not pull them forward.
3. Run `cargo make ci` in the repo(s) the step touches (plus the wasm32 compile check `cargo check --lib -p emery --examples --target wasm32-wasip2` when the engine guest or WIT is touched) before finishing.
4. Update this file: tick the step's checkbox, and record any deviation, discovered detail, or new open question in the step's **Notes** block (add one if needed). The next session inherits that state.
5. Never hand-edit engine-owned artifacts in test fixtures where a builder exists; extend the builder.

**Cross-repo choreography.** `emery-adapters` consumes the engine SDK (`emery-adapter`, `emery-native`, `emery-probe`) as git dependencies pinned by engine release tag. For the duration of this branch, adapter-repo steps use the committed `[patch."https://github.com/augentic/emery.git"]` block in the adapters root `Cargo.toml` (uncommented) to resolve against the sibling `../emery` checkout on `rfc-88-impl`. The operator decides when to re-point the pin and re-comment the patch at PR time; steps must not do this themselves. Engine steps that change the WIT package or SDK seam must state so in their Notes so the operator knows the adapters repo is stale until its matching step runs.

**Ordering rationale.** The steps follow the platform programme's four internal cuts, reordered only where the codebase makes a different order cheaper:

- **Cut A (steps 1–4):** change-home relocation, target-tree boundary, accepted-CID merge, deletion of interim `apply` — all still in-place, so the substrate change is isolated from the new authoring surface.
- **Cut B (steps 5–9):** RFC-104 handoff import DTOs, two-root plumbing, detached change home, bounded location binding, adapter catalog, `discovery.yaml`.
- **Cut C (steps 10–17):** canonical lead catalog, source WIT extension, model-capability profiles, conflict-domain decomposition (deterministic kernel first, judgment second), refinement boundary escalation, amendment proposals.
- **Cut D (steps 18–20):** execution digest chain and detached accepted-CID execution, adapter-repo fixtures and eval cases, documentation closure.

## Current state (verified against the codebase, 2026-08-13)

What exists and is load-bearing for this plan:

| Area | State |
| --- | --- |
| RFC-104 (`emery system *`, `system.wave.reviewed`, handoffs, definition home) | **Absent** — prose only. `EventKind::PlanExecuteStarted.discovery_digest` exists but is always `None`. |
| In-place `plan author` | Implemented: `crates/change/src/orchestrate/author.rs` (survey → pins → propose → `plan.yaml` + `discovery.md` + `change.md` at the repo root). |
| Plan model | `crates/project/src/plan/model/state.rs`: `Plan { name, sources, entries }`; `Entry.project: Option<String>` is the current target hook; no `targets:` map, no digests. |
| RFC-91 refinement | Implemented: `refinement.yaml` with `inputs.planning.{entry, leads, decomposition}` (decomposition currently the canonical single-node projection), `inputs.profile` = canonical empty digest placeholder. |
| RFC-86 waves / build records | Implemented in-place: `crates/project/src/wave.rs` (`.emery/targets/<target>/waves/<digest>.yaml`, one-member enforced), `crates/project/src/build_record.rs` (`builds/<digest>.yaml`). Wave base comes from `freeze()` at build open. |
| RFC-86a gap gate | Implemented (`gap.deferred`, debt projection). No RFC-88 work needed beyond coverage wiring. |
| Workspace kernel | Implemented: `crates/project/src/workspace/` + `snapshot.rs`. `Store::apply` **exists** (used by merge `apply_result`); `Workspaces` trait carries `freeze/prepare/capture/discard/apply/sweep`; snapshot ignore policy excludes `.git` **and `.emery`**. |
| Merge | `crates/slice/src/orchestrate/merge.rs`: preflight → deterministic delta-spec commit **into the operator checkout's `.emery/specs/`** → `Workspaces::apply` patch write-back → `target.merge.wave-committed` → postflight. |
| Registry / workspace slots | Already removed (router rejects `emery registry *` / `emery init --workspace`). D4 is mostly a no-op; only `Entry.project` vocabulary remains. |
| Source WIT | `survey(id) -> list<lead>` — no focus parameter, no child leads. Source adapters read `plan.yaml` from the lent project preopen to find their binding. |
| Adapter catalog / fingerprinting | Absent. |
| Model-capability profiles | Absent (only the manifest placeholder digest). |
| Adapters repo | Five sources (`intent`, `documentation`, `typescript`, `screenshots`, `captures`), three targets (`omnia`, `vectis`, `contracts`). Targets already receive RFC-87 workspaces + artifact stage; low churn expected. Eval workflow cases (`orders-contracts`, `omnia-r9k`) drive `init` + `plan author` from `case.toml` intent/source strings — no definition home. |

## Step overview

| # | Cut | Repo | Title |
| --- | --- | --- | --- |
| 1 | A | emery | Relocate the in-place change home to `.emery/change/` |
| 2 | A | emery | Target-tree boundary: `.git` + change home only |
| 3 | A | emery | Accepted-CID merge; delete interim `apply` |
| 4 | A | emery (+adapters) | Interim accepted-result access and eval survival |
| 5 | B | emery | Handoff + review-envelope DTOs, digests, fixture builder |
| 6 | B | emery | Two-root plumbing, detached change home, `plan author --from --wave` grammar |
| 7 | B | emery | Locator resolution, exact-revision ingestion, bounded-read policy |
| 8 | B | emery | Adapter catalog, recognition profiles, source keys, exact pins |
| 9 | B | emery | Plan model extension, `discovery.yaml`, wave-binding phase |
| 10 | C | emery | Canonical `leads.md` catalog + revision retention |
| 11 | C | emery | Source WIT extension: value-in, focused survey, child leads |
| 12 | C | adapters | Update the five source adapters to the new seam |
| 13 | C | emery | Model-capability profiles |
| 14 | C | emery | Decomposition substrate: DTOs, validators, compiler, projection |
| 15 | C | emery | Decomposition judgment legs + full detached `plan author` |
| 16 | C | emery | Refinement boundary escalation |
| 17 | C | emery | Amendment proposals + `plan amend --proposal` |
| 18 | D | emery | Execute digest chain + detached accepted-CID execution |
| 19 | D | adapters | Definition-home fixtures, eval and wasm cases |
| 20 | D | both | Documentation closure and final gates |

Steps 1–3 and 5–18 are strictly ordered. Step 4 is a policy decision (Open Question 2) that can be folded into step 3 if answered early. Step 12 must follow 11; step 19 must follow 18.

---

## Cut A — accepted-CID merge and deletion of interim `apply`

The first cut lands the execution-substrate change while everything is still in-place and single-target, so behaviour changes are testable against the existing suite before any new authoring surface exists.

### Step 1 — Relocate the in-place change home to `.emery/change/` [ ]

**RFC anchors:** D1 (change-home tree, in-place mode paragraph), implementation requirement "Operations take explicit target (product) and change roots… in-place changes use `<product>/.emery/change/`".

**Scope (emery):**

- Move every change-scoped artifact under `.emery/change/`: `plan.yaml`, `change.md`, `discovery.md` (renamed later, step 10), `slices/`, `events/`, `targets/` (wave manifests), and archive staging. Durable product state stays where it is: `.emery/project.yaml`, `.emery/specs/`, `.emery/decisions/`, `.emery/design-system/`.
- Rework `Layout<'a>` (`crates/project/src/config.rs`) to expose a `change_root()` beneath the project root and re-anchor every change-scoped path helper on it. This step deliberately keeps *one* root parameter; the two-root split is step 6. The goal here is that "change home" becomes a single directory the later steps can point elsewhere.
- Update `emery init` scaffolding, `plan archive` (archive move target), `archive prune`, journal writer paths, and every prompt/orchestration string that names a path (e.g. synthesis prompts referencing `.emery/slices/...`).
- Hard cut: no fallback read of the old flat layout (pre-1.0 posture; an existing project re-inits). `emery init --upgrade` re-scaffolds the new layout but does not migrate live changes.
- Sweep `rg`-able path references in `AGENTS.md` and `docs/` in the same change (per the AGENTS.md removal rule); full doc prose rewrite waits for step 20.

**Key code areas:** `crates/project/src/config.rs`, `crates/project/src/journal.rs`, `crates/project/src/wave.rs`, `crates/change/src/plan/handlers/`, `crates/slice/src/`, `crates/transport/tests/`, integration suites across `crates/{project,slice,change,transport}/tests/`.

**Tests / gates:** existing integration suites updated to the new layout; a dedicated layout test asserting durable state and change state land on the correct sides of the boundary; `cargo make ci`.

**Out of scope:** detached roots, `--from`/`--wave`, tree-boundary changes, any new artifact shape.

**Why first:** every later step (tree boundary, two-root dispatch, detached homes) needs "the change home" to be one directory, not a scatter of root-level files plus most-of-`.emery/`.

### Step 2 — Target-tree boundary: `.git` + change home only [ ]

**RFC anchors:** D1 (last paragraph), D5 ("the identified tree includes durable Emery state"), implementation requirement "Target trees ignore only `.git` and a nested change home. `.emery/` is otherwise included." Amends RFC-87 D4 / acceptance criterion 4.

**Scope (emery):**

- Change the snapshot ignore policy (`project::snapshot` ignore rules and the native `Store::snapshot` walker) from "exclude `.git` + `.emery`" to "exclude `.git` + the nested change home (`.emery/change/`)". `project.yaml`, `specs/`, `decisions/`, and the rest of `.emery/` fold into every snapshot.
- Update `freeze()` implementations (native provider, guest provider) and any root-plan-artifact exclusions that step 1 made obsolete.
- Verify digest identity stays deployment-independent (native vs guest kernels produce byte-identical manifests for the new boundary).
- Kernel round-trip tests: baseline (`.emery/specs/`), `project.yaml`, and `decisions/` survive `freeze → prepare → capture`; the change home never appears in a snapshot; a workspace prepared from such a snapshot exposes the baseline to the build agent.

**Key code areas:** `crates/project/src/snapshot.rs`, `crates/project/src/workspace/`, `crates/native/src/provider.rs`, `crates/guest/src/provider.rs`, workspace kernel tests.

**Tests / gates:** kernel tests above; existing build/merge integration suites still pass (they will see baselines inside workspaces from here on); `cargo make ci` + wasm32 compile check.

**Out of scope:** merge behaviour changes (step 3). Note: between steps 2 and 3, merge still writes the checkout baseline; that overlap is fine because the snapshot simply carries a copy.

### Step 3 — Accepted-CID merge; delete interim `apply` [ ]

**RFC anchors:** D7 (whole), D8 acceptance-criterion 7 semantics, implementation requirements "Route every detached target through RFC-86's immutable one-member wave…", "`Workspaces::apply` and its write-back machinery are removed", "merges update the baseline inside the workspace", RFC-87 D8 (interim `apply` deletion), RFC-91 D6 (base selected at wave open).

**Scope (emery):**

- **Accepted-CID projection.** Add a per-target accepted-CID projection computed from `target.merge.wave-committed` facts: initial CID → chain of `{base, result}` transitions; reject a broken chain (a committed fact whose `base` is not the prior accepted CID) with a typed error. In-place mode's initial CID is the freeze taken when the target's *first* wave opens (RFC-91 D6 semantics preserved); the projection lives beside the wave/fact machinery in `crates/project/`.
- **Wave open selects the accepted CID.** `open_wave` (`crates/slice/src/orchestrate/target.rs`) uses the projection: first wave freezes ambient (in-place) and records it as the initial CID; every later wave opens against the current accepted CID, not a fresh freeze. Record base CID on the wave manifest as today.
- **Merge inside the workspace.** Rework `crates/slice/src/orchestrate/merge.rs`: prepare a writable workspace from the member's build-record `result` CID; run preflight (unchanged gate order); run the deterministic delta-spec merge and identity-map finalization *inside that workspace* (baseline now lives in the tree per step 2); capture the final candidate CID; append `target.merge.wave-committed` carrying base and final-result CIDs, identity maps, baseline digest, deferred members; then postflight in stable order with persisted reports and crash-resume at the first missing report (existing behaviour, re-anchored). A crash before the commit fact leaves the prior accepted CID authoritative — assert this with a crash-boundary test.
- **Delete `apply`.** Remove `Store::apply`, the two `seam::Workspaces::apply` legs (native, guest), merge's `apply_result`, and the `slice.code.applied` event kind. The operator checkout is no longer written by merge — in-place included. `merged` continues to project only from the committed fact.
- Update `plan archive` completion conditions if they referenced applied trees; update debt/baseline conservation tests (baseline debt now folds inside the workspace tree during merge).

**Key code areas:** `crates/slice/src/orchestrate/merge.rs` + `merge/`, `crates/slice/src/merge/` (engine + commit), `crates/project/src/wave.rs`, `crates/project/src/journal/event.rs`, `crates/project/src/workspace/store.rs`, `crates/{native,guest}/src/provider.rs`, `crates/slice/src/orchestrate/target.rs`.

**Tests / gates:** integration coverage for: one-member wave over accepted CID; dependent second wave opening against the first's accepted result; commit/postflight crash boundaries (fail before fact → no merge projected; fail after fact → accepted CID stands, resumable stop); broken-chain rejection; checkout untouched by merge. `cargo make ci` + wasm32 check.

**Out of scope:** detached targets, multi-target execution (step 18), materialization surface (step 4).

**Note:** this is the largest Cut A step. If a session runs hot, the accepted-CID projection + wave-open change can be split from the merge rework at the recorded seam (projection lands first, merge rework consumes it).

### Step 4 — Interim accepted-result access and eval survival [ ]

**Blocked on Open Question 2 — confirm before running.**

After step 3, merged results exist only as store snapshots until RFC-95 lands. Operators, the eval rungs, and the wasm examples currently inspect produced code in the checkout. This step adds the agreed interim surface — the recommendation is a read-only `emery target materialize <target> <dir>` debug verb (store → directory export of the current accepted CID, refusing existing non-empty destinations), scoped to be deleted or demoted when RFC-95 lands — and updates the engine-side integration helpers accordingly. Adapters-repo eval/wasm graders are updated in step 19; if the eval rungs must stay green between steps 3 and 19, the operator should schedule step 4 immediately after step 3 (it is written to allow that).

**Tests / gates:** materialize round-trip test (accepted CID → tree bytes); refusal cases; `cargo make ci`.

---

## Cut B — detached change home, wave import, delivery binding

### Step 5 — Handoff + review-envelope DTOs, digests, fixture builder [ ]

**RFC anchors:** D1 (`imports/` retention), D3 (digest coverage paragraph), D8 (verification list), implementation requirements "Handoff and binding DTOs reject unknown fields and use typed canonical digests… Integration tests use reviewed definition fixtures"; RFC-104 D10 (handoff shape, `system.wave.reviewed` fact, resolution rules).

**Scope (emery):**

- New module (recommended: `crates/project/src/definition/`) owning the *read side* of the RFC-104 contract that RFC-88 consumes — deliberately placed so a future RFC-104 implementation writes the same types (Open Question 8):
  - `Handoff` DTO matching RFC-104 D10's YAML (version, definition, scope/coverage/sources/system-model/migration-plan digests, `wave` block with targets, evidence-scopes, delivery-mappings, element lists, and every `{id, digest}` reference list). `deny_unknown_fields`, typed digests, canonical digest function (schema-validated content, format-independent — reuse the canonicalization approach of `plan::projection`).
  - `system.wave.reviewed` event envelope DTO: RFC-88 *parses and verifies* this event from a definition home's `events/<writer>.jsonl` but **never writes it** ("RFC-88 has no event writer for that kind") — so it does not enter the engine's own closed `EventKind`; it is a definition-scoped foreign envelope with `(writer, sequence, event-digest)` identity.
  - Definition-home read surface: given a definition root and wave id, resolve the single current handoff projection under `handoffs/<digest>.yaml`, fail closed on missing or ambiguous projections, and verify the matching review fact.
- **Fixture builder** in `crates/mock` (host-only test support): mint a valid definition home — handoff with correct canonical digests, review event, degenerate and multi-target variants — so every downstream step and the adapters-repo eval cases can author against fixtures until RFC-104 lands. This builder is the linchpin of Open Question 1's recommended answer.
- Typed diagnostics: missing review fact, ambiguous/missing current projection, digest mismatch, unknown fields, malformed envelope.

**Key code areas:** new `crates/project/src/definition/`, `crates/project/src/journal.rs` (read-side envelope parsing only), `crates/mock/src/` (fixture builder), `crates/project/tests/`.

**Tests / gates:** round-trip + canonical-digest stability tests (YAML reformatting does not change the digest); fail-closed resolution matrix; fixture-builder self-checks. `cargo make ci`.

**Out of scope:** any CLI surface, binding, or import-into-change-home behaviour (steps 6/9).

### Step 6 — Two-root plumbing, detached change home, author grammar [ ]

**RFC anchors:** D1 (detached tree, "Operations therefore receive separate target (product) and change roots"), acceptance criterion 1; implementation requirements "Operations take explicit target (product) and change roots… Detached homes have no synthetic `project.yaml`".

**Scope (emery):**

- Extend `ExecutionPaths` / anchoring (`crates/project/src/handler/paths.rs`, `crates/launcher/`) with an explicit change root distinct from any product root. In-place mode: change root = `<product>/.emery/change/` (step 1's layout), product root = the repo. Detached mode: change root is an operator-selected directory with **no** `.emery/project.yaml` and no Git requirement; there is no ambient product root at all (product trees only ever appear as store-materialized workspaces, per D7).
- Change-root anchoring policy for detached homes (Open Question 6): recommended — `plan author <name>` creates/uses `<cwd>/<name>` (or an explicit `--change-dir`), and every later verb anchors by walking for a change-home marker written at creation. Decide and record before implementing.
- Transport grammar: `emery plan author <name> --from <definition-root> --wave <id> [--force]` (detached), and in-place `--from .emery/system/` binding for a colocated degenerate definition. The old author grammar (bindings from `case.toml`-style init) is removed in step 9 when binding fully replaces survey-driven authoring; this step lands the flags, root resolution, and change-home scaffolding, with the new path returning a typed "not yet implemented" until step 9 completes the phases. (If the operator prefers no dead grammar on the branch, fold this step's CLI surface into step 9 and keep only the root plumbing here.)
- Launcher: mount policy for the detached change root (writable) and the definition root (read-only) as preopens; keep the store host-owned. Guest `Layout` consumes the change root.
- `emery init` is not required for a detached change (acceptance criterion 1: empty non-Git directory works); make init-dependence explicit and remove it from the detached path.

**Key code areas:** `crates/project/src/handler/paths.rs`, `crates/project/src/config.rs`, `crates/launcher/src/`, `crates/transport/src/command/`, `crates/guest/`, `src/main.rs` mount expressions.

**Tests / gates:** router/grammar tests; detached-root resolution tests (no project.yaml, no git); in-place `--from .emery/system/` resolution; `cargo make ci` + wasm32 check.

**Out of scope:** actual binding (step 9), locator resolution (step 7).

### Step 7 — Locator resolution, exact-revision ingestion, bounded-read policy [ ]

**RFC anchors:** D2 (whole), D5 (locator + `cid` for targets), D9 (whole); implementation requirements "Repository ingestion accepts an exact revision; workspace preparation accepts an explicit target and CID", "Plan CIDs remain GC roots for the change lifetime"; acceptance criteria 2 and 10.

**Scope (emery):**

- Locator grammar and typed parse: Git reference (`url@revision`), change-relative path, external local path, bounded HTTPS URL; optional `path` selector (default `.`); `value` inline alternative. One row shape shared by sources and targets.
- Resolution kernel: resolve each locator once → stage temporarily → store as a tree under its CID (a file becomes a one-file tree); mutable Git refs resolve to exact revisions at the host; later runs use the recorded CID and never reread the origin. Both-roles reuse (same repo as target and source → one CID).
- Git ingestion at an exact revision (host-side, launcher-adjacent like `launcher::install`): no hooks, no submodules, no LFS filters, no escaping symlinks; `.git` and nested change home excluded from the ingested tree (step 2's boundary). Moved-branch = freshness warning; unavailable recorded commit = error.
- HTTPS reads: HTTPS-only, credential-free URLs, no private-network targets, redirect and body budgets; GitHub document pages resolve to raw content.
- Versioned bounded-read policy DTO (limits on bindings, API requests, concurrency, time, inspected bytes, imported trees, redirects, HTTPS bodies) with compiled defaults (Open Question 10 fixes the numbers); budget exhaustion fails the wave for upstream narrowing — no namespace fallback.
- Read-only source views: prepare-with-empty-writable-scope over the recorded CID (RFC-87 D2 path); `capture` already refuses read-only workspaces.
- GC roots: plan/discovery CIDs registered as store GC roots for the change lifetime; `plan archive`'s existing `Store::sweep` picks them up when the change ends.

**Key code areas:** new `crates/project/src/binding/` (or similar) for the locator/resolution kernel, `crates/launcher/src/` (network + git side), `crates/project/src/workspace/`, `crates/project/src/seam.rs` if the host capability surface grows, fixtures with a local git repository and local HTTP server.

**Tests / gates:** integration tests over local git fixtures (exact revision, moved branch, missing commit), local HTTPS fixtures (budgets, redirect caps, private-network refusal), file-vs-tree CID shape, both-roles CID reuse, GC-root retention. `cargo make ci`.

**Out of scope:** adapter selection (step 8), writing `discovery.yaml` (step 9).

### Step 8 — Adapter catalog, recognition profiles, source keys, exact pins [ ]

**RFC anchors:** D6 (whole); acceptance criterion 3.

**Scope (emery):**

- Host-supplied bounded, versioned adapter catalog: source adapters with recognition profiles (deterministic fingerprint rules over a staged source value — e.g. extension/manifest probes for `typescript`, `documentation`, `screenshots`, `captures`), target adapters with platform constraints. The engine consumes the catalog through a seam capability; the shipped deployment compiles the first-party catalog into the launcher/native host (mirroring the existing first-party GHCR mapping); `intent` is explicit-only.
- Deterministic matching: reuse pins already carried by the handoff verbatim; fingerprint only a newly focused source the handoff left open; exactly one match pins, zero/many block that wave member with `source-adapter-no-match` / `source-adapter-ambiguous`. No ranking, no model fallback.
- Exact package pins everywhere in detached topology (`emery:<name>@<semver>`); unversioned local components are refused for detached binding with a typed diagnostic.
- Source key generation: locator → normalized basename; inline value → adapter name; `intent` reserved; stable digest suffixes on collision; unchanged bindings keep keys; duplicate bindings rejected. Persisted key is authoritative downstream.

**Key code areas:** `crates/project/src/adapter/` (catalog seam + matching kernel), `crates/native/src/` and `crates/launcher/src/` (catalog supply), `crates/mock` (catalog fixtures for tests).

**Tests / gates:** key determinism + collision-stability matrix; no-match/ambiguous blocking; pin reuse from handoff fixtures; duplicate rejection. `cargo make ci`.

### Step 9 — Plan model extension, `discovery.yaml`, wave-binding phase [ ]

**RFC anchors:** D1 (`discovery.yaml`, `imports/`), D3 (phase 1 + verification paragraph), D5 (whole), D6 (binding outputs); implementation requirement "`Plan` gains…"; acceptance criteria 1 and 4.

**Scope (emery):**

- **Plan DTO extension** (`crates/project/src/plan/model/`): `Plan` gains `discovery-digest`, `leads-digest`, `decomposition-digest`, the `definition` identity block (handoff digest, review identity + event digest, system/system-model/migration-plan/wave ids), a `targets:` map (adapter pin, locator, cid, model-capability-profile id + digest — profile fields filled from step 13 onward), source rows in the shared shape (adapter pin, locator xor value, cid), and singular `slices[].target` replacing `Entry.project` (Open Question 12). Hard cut on the wire shape; update every consumer and golden.
- **`discovery.yaml`** DTO + writer: reviewed-handoff identities, pinned targets and sources with CIDs and adapter pins. Unknown fields rejected; canonical digest recorded into `plan.yaml.discovery-digest`.
- **Wave-binding phase** of detached `plan author` (D3 phase 1): resolve the single current reviewed handoff (step 5), copy byte-identical handoff + review envelopes to `imports/handoffs/<digest>.yaml` / `imports/reviews/<digest>.json`, resolve target locators to exact revisions + CIDs (step 7), validate each target's pinned `.emery/project.yaml` (product identity, `platforms:`, target-axis adapter present — mismatch blocks authoring), pin sources (step 7 + 8), generate keys, write `discovery.yaml`. Independent binding reads may run concurrently under the D9 policy's concurrency bound; results merge into one validated document.
- Retire the old survey-driven author entry path: in-place authoring now also binds a (degenerate, colocated) definition via `--from .emery/system/`. The old `plan author` bindings-from-init flow is deleted (no flag-only bypass). Note: from this step until step 15, `plan author` produces `discovery.yaml` + imports and stops with a typed "decomposition pending" outcome — the plan cannot be fully authored until the decomposition phases land. The operator accepts a red-ish window on the branch here; existing end-to-end suites that exercised the old author path are quarantined to fixture-driven equivalents progressively through Cut C (record status in Notes).
- `--force` semantics: rebind the same reviewed handoff and redo binding; selecting a changed wave requires a new handoff + review fact.

**Key code areas:** `crates/project/src/plan/model/`, new `discovery` module beside it, `crates/change/src/orchestrate/author.rs`, `crates/change/src/plan/handlers/author.rs`, answer-schema goldens (`crates/project/answers/`), `crates/transport/` projections.

**Tests / gates:** acceptance-criterion-4 matrix (missing/ambiguous handoff, missing review fact, edits, unknown fields, stale revisions, changed pins all block); byte-identical import retention; empty-non-git-dir authoring reaches `discovery.yaml` (criterion 1, binding half). `cargo make ci`.

---

## Cut C — leads, decomposition, profiles, proposals

### Step 10 — Canonical `leads.md` catalog + revision retention [ ]

**RFC anchors:** D1 (`leads.md` / `leads/<digest>.md` paragraphs), D3 (focused-scope import), acceptance criterion 6.

**Scope (emery):**

- Reshape the lead inventory into the authoritative parsed catalog `leads.md`: per source key, every lead id, synopsis, topic, parent relation, and source-local focus. Extend the `artifacts::discovery` parser (rename module to `artifacts::leads`) with parent/focus fields; canonical `leads-digest` over the parsed content.
- Wave import populates the initial catalog from the handoff's evidence scopes (D3 phase 2's import half); focused-survey appends land in later steps but the catalog shape and append semantics (new revision, never mutate a referenced meaning) land here.
- Revision retention: before any decomposition revision or build fact references a `leads-digest`, copy the exact document to `leads/<digest>.md`; immutable thereafter.
- Wire `refinement.yaml.inputs.planning.leads` to the real contributing-lead-closure digest over the new catalog (the projection machinery exists; it now reads `leads.md`).
- `discovery.md` disappears as an artifact name (Open Question 5): the summary/source-inventory preamble moves to `change.md` or is dropped; record the decision.

**Key code areas:** `crates/artifacts/src/discovery/` → `leads/`, `crates/change/src/orchestrate/author.rs`, `crates/project/src/plan/projection.rs`, retention plumbing in the change-home layout.

**Tests / gates:** catalog parse/serialize round-trip; digest stability; retention-on-reference; criterion-6 edit-invalidates-digest cases. `cargo make ci`.

### Step 11 — Source WIT extension: value-in, focused survey, child leads [ ]

**RFC anchors:** D2 ("Source operations receive the source key, its read-only workspace or inline value, and read-only change artifacts. They never parse `plan.yaml`…", focused-survey paragraphs), implementation requirement "The source WIT receives either a read-only workspace or inline value. Extend `survey` with an optional parent-lead focus and stable child-lead response…".

**Scope (emery — WIT + SDK + engine dispatch; adapters follow in step 12):**

- WIT `source` interface: `survey` gains a typed input carrying the source key, either a read-only workspace handle or the inline value, read-only change-artifact access, and an **optional parent-lead focus**; the response distinguishes top-level leads from stable child leads under the named parent. `extract` gains the same value-in shape (workspace-or-value; terminal `(source, lead)` unchanged). No third source operation.
- SDK (`crates/adapter`): `Source` trait and seam DTOs updated (`Lead` gains parent/focus fields aligned with step 10's catalog); `LEADS_ANSWER_SCHEMA` regenerated; `source!` macro re-wired. This is a WIT-breaking change — record it in Notes for the release choreography.
- Engine dispatch (`crates/change/src/orchestrate/survey.rs`, `crates/slice/src/source.rs`): prepare the read-only source view from the recorded CID (step 7) and pass it; sources never receive the project preopen or parse `plan.yaml`; the engine owns which lead is focused and merges child leads into the catalog as a new revision (stable merge order).
- Mock adapter (`crates/mock`) updated, including scripted focused-survey answers, so engine integration tests cover the new seam without the adapters repo.

**Key code areas:** `wit/emery.wit` (via `crates/adapter`), `crates/adapter/src/{operations,seam,source,answers}.rs`, `crates/change/src/orchestrate/survey.rs`, `crates/slice/src/`, `crates/mock/`, `crates/{native,guest}` providers.

**Tests / gates:** engine-side focused-survey integration over the mock catalog; extract-over-view tests; schema goldens regenerated; `cargo make ci` + wasm32 check.

### Step 12 — Update the five source adapters to the new seam (adapters repo) [ ]

**RFC anchors:** as step 11; adapters-repo AGENTS.md contract.

**Scope (emery-adapters, path-patched to sibling engine):**

- Rework `sources/{intent,documentation,typescript,screenshots,captures}/src/operations.rs` to the new trait: consume the passed source root / inline value instead of reading `plan.yaml` from a lent project tree; implement focused survey (return stable child leads under a requested parent, inheriting parent context for extraction).
- Update survey/extract prompts (`prose/prompts/`) for the new input model and focused re-survey semantics; keep `BINDING_NOTE`-style plan parsing out.
- Target adapters: audit for assumptions broken by steps 2–3 (baseline now inside the workspace tree; merge runs against a workspace, not the checkout) — expected small; contracts preflight/postflight baseline reads are the main suspects.
- Update adapter native tests (`tests/operations.rs`) to the new DTOs.

**Tests / gates:** `cargo make check` + `cargo make ci` in emery-adapters; focused-survey unit coverage per source adapter. Live eval rungs stay operator-invoked and are not gating here.

### Step 13 — Model-capability profiles [ ]

**RFC anchors:** D3 (profile paragraphs: closed assessment dimensions, engine-computed weighted sum, operation thresholds), acceptance criterion 5 (profile digests as planning inputs); RFC-92 note (it patches the profile shape later — keep the DTO closed but versioned).

**Blocked on Open Question 4 (profile provenance + initial values) — confirm before running.**

**Scope (emery):**

- Closed `ModelCapabilityProfile` DTO: id, version, weights over the five dimensions (behavioural breadth, coupling, uncertainty, context volume, verification surface — integers 0–10 come from judgment; weights and thresholds from the profile), thresholds (`slice-split` consumed here; `task` recorded for RFC-96), canonical digest.
- Engine resolution of one profile per target from the configured model class (per OQ4's answer — recommended: compiled-in versioned defaults keyed by model class, overridable via deployment config, never a model answer).
- Persistence: full closed bodies + ids + digests into `decomposition.yaml` (step 14 writes it; this step supplies the type and resolution), ids + digests copied into `plan.yaml.targets`.
- `refinement.yaml.inputs.profile` becomes the real bound-target profile digest; freshness recomputes it; changing a profile stales manifests and (with step 18) invalidates the epoch.

**Key code areas:** new `crates/project/src/profile.rs` (or under `plan/`), `crates/project/src/refinement.rs` + `crates/slice/src/refinement/`, config plumbing per OQ4.

**Tests / gates:** digest stability; scoring kernel (weighted sum + threshold application) dense matrix; manifest staleness on profile change. `cargo make ci`.

### Step 14 — Decomposition substrate: DTOs, validators, compiler, projection [ ]

**RFC anchors:** D1 (`decomposition.yaml`, `decompositions/<digest>.yaml`), D3 (split/leaf validation rules, containment-vs-dependency, budgets, deterministic engine ownership), implementation requirement "Add the closed `decomposition.yaml` shape… RFC-95 target-contraction cycle check"; acceptance criteria 5 and 6; RFC-95 D4/implementation note (shared contraction kernel).

**Scope (emery — deterministic only, no judgment):**

- `decomposition.yaml` DTO: version, `leads-digest`, model-capability-profiles (full bodies from step 13), root, nodes (children, parent, contributing `(source, lead)` scopes, target bindings, ownership envelopes, dependencies, local gate kinds, terminal `slice:` mapping). Unknown fields rejected; canonical digest; revision retention at `decompositions/<digest>.yaml` on first reference.
- Deterministic validators, each a typed diagnostic: at-least-once lead coverage; cross-cutting lead retention on every informed child; child-target-set containment; strict normalized-scope-measure reduction per split; at-most-one-terminal-lead-per-source per leaf; sibling ownership-overlap requires explicit order or fan-in child (ambiguity blocks); leaf completeness (one target, ownership manifest, acceptance boundary); depth/node budget enforcement; acyclic leaf graph; **target-contraction cycle check** (`publication-target-cycle`) as a pure kernel shared with future RFC-95 validation.
- Domain-dependency compiler: expand domain→domain dependencies into exit-leaf → entry-leaf `depends-on` edges, deterministically.
- Deterministic projector: terminal domains → `plan.yaml.slices` (byte-stable), copying bound topology and digests; exact-projection validator (plan ↔ decomposition drift is a typed failure; hand-edited drift never executes).
- `refinement.yaml.inputs.planning.decomposition` switches from the single-node placeholder projection to the real leaf-scoped projection (retained ancestry, dependency closure, terminal mapping) — RFC-91 D4 declared the fields absent-as-canonical-empty precisely so this lands without staling unrelated digests; verify that property with a test.
- One-slice degenerate tree (root → leaf) covered.

**Key code areas:** new `crates/project/src/plan/decomposition/` (DTOs + validators + compiler + projector), `crates/project/src/plan/projection.rs`, retention plumbing, extensive fixture-based tests.

**Tests / gates:** validator matrix over hand-built fixtures (every blocking rule); byte-stable projection; contraction-cycle fixtures (leaf-acyclic but target-cyclic); ≥3-level multi-target fixture; revision retention. `cargo make ci`.

### Step 15 — Decomposition judgment legs + full detached `plan author` [ ]

**RFC anchors:** D3 (phases 2–3, judgment `split | leaf` responses, provisional complexity, bounded boundary review, focused-survey requeue, budget exhaustion), platform § scope discipline ("the engine owns recursion, budgets, validation, and projection"); acceptance criterion 5.

**Scope (emery):**

- Judgment answer schemas + prompts (in `crates/change/prompts/`): typed `split | leaf` partition response (children with scopes, targets, ownership, dependencies, rationale) and the closed five-dimension complexity assessment; schema goldens under `crates/project/answers/` conventions.
- Engine-owned recursion: deterministic queue over open domains, one bounded judgment per dispatch, validation via step 14's kernels, repair budgets on invalid responses, requeue-after-focused-survey (step 11's engine-controlled focus), leaf-readiness gate (provisional score vs `slice-split` threshold → one bounded boundary review → close-with-rationale / focused-split / unready-blocks-authoring).
- Wire `plan author` end to end: bind (step 9) → focus delivery scopes (import + focused survey where a broad scope needs child detail) → decompose → validate complete tree → publish `decomposition.yaml` + `plan.yaml` together (complete-tree policy) → author `change.md` review prose (uncertainties preserved for the operator).
- Journal: reuse `plan.reconcile.completed` for the successful author write (or extend its payload with the new digests — decide in-session and record); no new authorization semantics.
- Uncertain boundaries and estimate caveats render into `change.md`; a source→target contradiction with the reviewed wave escalates as the D8 definition-revision request (inert stub here; full proposal machinery is step 17 — emit the typed stop now, attach the proposal document shape in 17).

**Key code areas:** `crates/change/src/orchestrate/author.rs` (+ new `decompose.rs`), `crates/change/src/judgment/`, `crates/change/prompts/`, `crates/mock` scripted answers for the new judgment legs.

**Tests / gates:** end-to-end detached author over mock catalog + fixture definition (single-leaf degenerate; multi-target ≥3 levels; budget exhaustion parks; invalid-split repair path; overlap ambiguity blocks). This closes step 9's "decomposition pending" window — the full author path is green again. `cargo make ci`.

### Step 16 — Refinement boundary escalation [ ]

**RFC anchors:** D3 (refinement paragraphs: typed `proceed | boundary-escalation`, Evidence-informed reassessment, focused resurvey + nearest-domain re-decomposition into an **inert** proposal, no artifact promotion, budgets/parking), implementation requirement "The refinement judgment adds the typed `proceed | boundary-escalation` outcome"; acceptance criterion 9 (refinement half).

**Scope (emery):**

- Extend the synthesis/refinement judgment with the typed outcome before bundle promotion: `proceed` continues today's path; `boundary-escalation` names affected terminal `(source, lead)` pairs + typed rationale, validated against the pinned profile and the same closed dimensions.
- On escalation: run focused survey for the named parents (engine-controlled), build the candidate lead-catalog revision and candidate nearest-domain decomposition revision, and persist them **inert** inside one amendment-proposal document (shape from step 17 — coordinate: if 16 runs before 17, land the proposal DTO skeleton here and let 17 own application; record in Notes). Current `leads.md` / `decomposition.yaml` / `plan.yaml` unchanged; no `refined` transition; no build work; child slices later re-extract their own child leads (parent Evidence not reused).
- Budget exhaustion parks the leaf with a typed stop; `plan refine` re-entry behaviour and `plan status` next-actions updated.

**Key code areas:** `crates/slice/src/orchestrate/` (synthesis judgment), `crates/slice/src/answers.rs` (+ goldens), `crates/change/src/orchestrate/refine.rs`, prompts.

**Tests / gates:** scripted-escalation integration (inert proposal produced, planning artifacts untouched, no transition); proceed-path regression; parking. `cargo make ci`.

### Step 17 — Amendment proposals + `plan amend --proposal` [ ]

**RFC anchors:** D1 (`planning/proposals/`), D8 (ownership proposals, boundary proposals, envelope escalation shape for RFC-96, definition-revision requests, compare-and-set application, preservation rules, lowering of direct plan mutations); acceptance criterion 9 (application half); implementation requirement "Add closed ownership and refinement-boundary proposal DTOs plus `emery plan amend --proposal <digest>`…".

**Scope (emery):**

- Closed proposal DTOs under `planning/proposals/<digest>.yaml`: **ownership** (nearest domain, new dependency or fan-in leaf, expected planning digests, expected accepted CID per target, committed leaf→wave set, affected open-wave/claim frontier), **boundary** (failed leaf, assessment + profile digest, candidate lead catalog, candidate nearest-domain decomposition, same expected frontiers), the **envelope** escalation record shape (authored by RFC-96 later — DTO only, no producer), and the inert **definition-revision request** (conflicting handoff reference, stops the affected scope; not an amendment).
- `emery plan amend --proposal <digest>`: validate → compare-and-set every expected revision and accepted frontier → refuse live affected claims/waves (operator quiesces first) → for boundary proposals atomically activate candidate lead + decomposition revisions → reproject `plan.yaml` via step 14's projector → retain new revisions → invalidate the old closed-plan epoch. Preservation checks: committed leaves keep identity, binding, target, dependencies, terminal mapping; accepted leaves may gain dependants but cannot be removed/rebound/reordered-behind/depublished. Stale, malformed, cyclic, accepted-history-changing, or ambiguous proposals change nothing (typed refusals).
- Lower existing `plan add` / `amend` / `remove` to the same domain-mutation + reprojection path; refuse when no unambiguous hierarchy edit exists. `plan drop` (in-scope exclusion) is audited against the new model but keeps its semantics.
- Runtime ownership-overlap detection (build/merge surfaces) authors ownership proposals but never applies them — wire the authoring hook where overlap is detected today (merge staleness/validate diagnostics), keeping it inert.

**Key code areas:** new `crates/project/src/plan/proposal/`, `crates/change/src/plan/handlers/amend.rs` (+ new proposal leg), `crates/change/src/orchestrate/`, transport grammar.

**Tests / gates:** compare-and-set matrix (each expected-revision mismatch refuses); preservation-violation refusals; boundary activation + reprojection round-trip; add/amend/remove lowering including refusal; epoch invalidation observed by step 18's checks (assert the fact-side effect now, full chain in 18). `cargo make ci`.

---

## Cut D — execution, fixtures, closure

### Step 18 — Execute digest chain + detached accepted-CID execution [ ]

**RFC anchors:** D7 (execution over accepted CIDs, workspaces from accepted CID, read-only change mounts), D8 (verification list, coverage contents), D9 ("execution never repeats target binding"); implementation requirements on coverage; acceptance criteria 7, 8, 11.

**Scope (emery):**

- **Verification chain at execute start** (before any epoch/workspace/wave): plan ↔ `discovery.yaml` match; imported handoff + review-envelope bytes match digests and identities; the definition's *current* handoff still has its review fact and names the selected wave; `leads-digest` identifies the retained revision; plan is the exact projection of `decomposition.yaml`; recorded source/target/CID/adapter pins remain valid; profile digests match. Each failure a typed diagnostic; authoring-time validation (steps 9/14/15) shares these kernels — extract them accordingly.
- **Coverage:** `plan.execute.started` carries `plan-digest`, **required** `discovery-digest` (populate the existing optional field and tighten it), and sorted per-leaf refinement digests. The plan digest transitively binds definition/system-model/migration-plan/wave/lead/decomposition/profile digests — verify transitivity with a test that mutates each underlying revision and observes epoch invalidation.
- **Detached execution loop:** serial scheduler selects leaves whose dependencies are accepted (per target); wave open uses the bound target's accepted-CID projection (step 3) seeded from `plan.yaml.targets[].cid`; workspaces prepare from the accepted CID with change artifacts mounted read-only (two-root dispatch from step 6); build/verify/repair/review machine unchanged (RFC-90); merge (step 3) advances the per-target accepted CID; dependent leaves across targets open later waves against accepted results; multi-target drain to one accepted CID per touched target and **no commit or branch** (RFC-95's job).
- In-place execution runs the identical path (the change home is just colocated); the checkout is never a write target.
- `plan status` / `plan gaps` / `emery debt` / `plan archive` audited against the new chain (archive's carried-debt summary, sweep of change-scoped GC roots including plan CIDs, RFC-95 gate note).

**Key code areas:** `crates/change/src/orchestrate/{execute,epoch}.rs`, `crates/change/src/orchestrate/` scheduler, `crates/slice/src/orchestrate/target.rs`, `crates/project/src/journal/event.rs` (coverage payload), status/gaps handlers.

**Tests / gates:** acceptance-criteria 7/8/11 integration fixtures over the mock catalog (two-target plan with a cross-target dependency; postflight-failure resumable stop; live-claim-without-epoch refusal; stale-definition refusal mid-plan); `cargo make ci` + wasm32 check.

### Step 19 — Definition-home fixtures, eval and wasm cases (adapters repo) [ ]

**RFC anchors:** implementation requirement "Integration tests use reviewed definition fixtures, local repository-host, HTTP, content-addressed store, and component fixtures"; adapters-repo testing map.

**Scope (emery-adapters):**

- Rework the eval runner (`examples/eval/`) case shape: `case.toml` supplies a definition home (fixture or generated via the engine fixture builder exposed through `probe`/lab shim) + wave id instead of intent/source strings; runner drives `plan author --from --wave → plan refine → plan execute` and grades against materialized accepted CIDs (step 4 surface) instead of checkout trees.
- Update `orders-contracts` and `omnia-r9k` cases and the wasm example scripts (`examples/wasm/`) to the new choreography; add one multi-target workflow case (even a small two-target contracts+omnia fixture) to exercise cross-target dependencies live — this is the only rung that exercises D7 across targets with real adapters.
- Confirm the five updated source adapters (step 12) behave over pinned read-only source views in the wasm rung.

**Tests / gates:** `cargo make ci` in emery-adapters; eval/wasm rungs remain operator-invoked (run at least `wasm-contracts` once before closing the step; record the outcome in Notes).

### Step 20 — Documentation closure and final gates (both repos) [ ]

**Scope:**

- **emery:** rewrite affected prose — `AGENTS.md` (vocabulary: change home paths, `discovery.yaml` / `leads.md` / `decomposition.yaml`, targets map, accepted CID, removed `apply`/`slice.code.applied`, detached mode), `docs/standards/workflow.md` (the workflow contract spans both repos), `docs/reference/` and `docs/explanation/` pages touching artifacts/adapter-contract/provenance, skills wrapper text (`plugins/emery/` — `/emery:plan` elicits `--from`/`--wave`), RFC status headers (RFC-87: `apply` deleted; RFC-86: stand-ins resolved; RFC-88: status → implemented contract wording per house style; platform.md "Where we are"). Run the AGENTS.md rule: `rg` every removed/renamed symbol across Rust *and* prose in both repos.
- **emery-adapters:** `AGENTS.md` (source contract now value-in + focused survey), `docs/authoring.md`, `docs/testing.md`, eval README.
- Final gates: `cargo make ci` in both repos, wasm32 compile check, `cargo make links`.

---

## Open questions

Iterate on these before (or while) running the affected steps. Each has a recommendation so a "no objection" answer is cheap.

1. **RFC-104 sequencing.** RFC-88's public surface (`plan author --from <definition-home> --wave <id>`, no bypass) consumes RFC-104 outputs that nothing can produce yet — `emery system survey/plan/review` do not exist. **Recommendation:** implement RFC-88 against fixture definition homes (the step-5 fixture builder), accepting that the end-to-end operator workflow is fixture-only until RFC-104 lands next. The step-5 DTOs are placed in `crates/project/src/definition/` so RFC-104 later implements the write side of the same types. The alternative — landing RFC-104 first — is a larger programme decision outside this plan. *Affects steps 5, 9, 15, 19.*
2. **Interim access to accepted results (pre-RFC-95).** After step 3, merged code exists only as store snapshots; nothing materializes it for the operator, eval graders, or manual inspection until RFC-95. **Recommendation:** add a small read-only `emery target materialize <target> <dir>` debug verb (step 4), documented as interim. Alternative: store-reading test helpers only, and accept that operators cannot see results until RFC-95. *Affects steps 3, 4, 19.*
3. **In-place hard-cut confirmation.** Step 1 relocates the change home to `.emery/change/` and step 3 stops writing merge results to the checkout, both without migration. Existing projects re-init; live changes are recreated. Confirm this pre-1.0 posture is acceptable in one cut on this branch. *Affects steps 1, 3.*
4. **Model-capability profile provenance.** D3 says the profile is "an engine input, not a model answer" resolved "for the configured model class", but not where it lives. **Recommendation:** compiled-in versioned defaults keyed by model class, with a deployment-config override hook; initial weights/thresholds taken from the RFC's worked example (`frontier-large-v1`, slice-split 80, task 35) as declared starting values per the platform's honesty posture. RFC-92 later patches the shape. *Affects step 13.*
5. **Fate of `discovery.md`.** RFC-88's change home has `leads.md` + `discovery.yaml` and no `discovery.md`. **Recommendation:** `leads.md` replaces `discovery.md` wholesale; the authored summary preamble moves into `change.md`. Confirm nothing downstream (status projections, prompts) needs a separate survey narrative document. *Affects step 10.*
6. **Detached change-root anchoring.** How do post-author verbs find a detached change home — marker file written at creation + ancestor walk (recommended), always-explicit `--change-dir`, or cwd convention? Also: does detached `plan author <name>` create `<cwd>/<name>` or use cwd itself? *Affects step 6.*
7. **Source-seam change-artifact grant.** D2 says source operations receive "read-only change artifacts" — which ones, concretely? **Recommendation:** the lead catalog revision and (for extract) the slice's own artifacts only; never `plan.yaml`. Needs a decision before the WIT is cut. *Affects steps 11, 12.*
8. **Handoff DTO ownership.** Confirm `crates/project/src/definition/` as the shared home (RFC-104 will add the write side there), versus a new `definition` crate. Recommendation: module in `project`, promote to a crate only if RFC-104 needs it. *Affects step 5.*
9. **`intent` as an inline-value source.** Current intent bindings carry `value` in `plan.yaml.sources`. Under D2/D6 `intent` is the reserved explicit key with inline value protected by the plan digest. Confirm the mapping is 1:1 (no `intent` locator form) and that the degenerate definition's handoff carries intent as an evidence scope. *Affects steps 8, 9, and the fixture builder.*
10. **Budget and policy numbers.** Initial values for: decomposition depth/node/judgment/repair budgets (D3), D9's versioned read policy (bindings, API requests, concurrency, time, bytes, trees, redirects, body caps). **Recommendation:** compile conservative constants beside the existing engine budget constants (`MAX_REPAIRS` precedent), documented as starting values; a versioned policy DTO records them in `discovery.yaml` or the decomposition so digests cover them. Decide whether policy values are digest-covered. *Affects steps 7, 14, 15.*
11. **`Entry.project` → `slices[].target`.** Confirm the rename and the removal of the registry-flavoured `project` vocabulary from the plan wire shape in one cut (step 9), including the `topology.rs` sole-project fallback. *Affects step 9.*
12. **Multi-member-wave seam checks.** RFC-88 must preserve RFC-96's ability to widen wave membership "without changing the merge WIT operation". Step 3 keeps `Wave` members plural with `enforce_one_member` — confirm no step narrows the member model to a scalar. *Affects steps 3, 18.*
13. **WIT release choreography.** Step 11 breaks the WIT package; until step 12 lands, the adapters repo does not build against the sibling engine. The operator sequences sessions accordingly (11 → 12 back-to-back). Confirm the committed path-patch approach on the branch (see Cross-repo choreography) rather than tag-pinned intermediate engine releases. *Affects steps 11, 12, 19.*

## Deliberately out of scope (guard rails for every session)

- No RFC-104 write surface (`emery system *`), no `system.wave.reviewed` writer, no coverage dispositions in `discovery.yaml`.
- No RFC-95 publication: no push/branch/PR/seal, no forge provider work beyond what D9's bounded reads need.
- No RFC-96: no concurrent scheduler, task decomposition, multi-member waves, or envelope-escalation *producer* (the DTO ships in step 17).
- No streaming/partial publication (complete-tree policy only), no `--create`/provisioning surface, no `adapter.component-digest`, no second source-digest scheme, no undo/preview verbs.
- Removed concepts stay removed (acceptance criterion 12) — do not reintroduce registry vocabulary, `snapshot` as a wire field name for tree identity, or ambient product roots.
