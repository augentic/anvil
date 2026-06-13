# Specify & Specify-CLI — Improve-and-Optimise Review

**Date:** 2026-06-13
**Mode:** Improve / optimise / simplify (explicitly *not* add-features). Backwards compatibility is out of scope.
**Scope:** `augentic/specify` (plugin + docs repo) and `augentic/specify-cli` (Rust workspace).
**Method:** Structural recon (LOC, test counts, dependency map) plus five focused deep-dives — CLI crate graph/idiom, lint/standards engine, testing, cross-repo archeology, and plugin-repo YAGNI. Every claim below is grounded in a file path; spot-claims (dead trait, broken link, empty-tree rule) were independently verified.

---

## 0. Where to focus — the short version

The codebase is **well-engineered but over-built for its current maturity**. The workflow contract, the CLI verb surface, and the exercised path (intent + documentation + typescript-survey + omnia) are tight. The primary cost has accreted in the **generic lint engine** — sized for a third-party rule ecosystem that doesn't exist.

Two ways to read the focus list depending on appetite:

### Quick wins (days, low risk, do first)
| # | Action | Impact | Effort |
|---|--------|--------|--------|
| Q1 | **Archeology cleanup** — fix the broken RFC-43 link, delete `rfcs/archive/`, trim RFC/phase labels in `DECISIONS.md`, remove `names*.md`/`pitch.md` scratch | ★★★ clarity | S–M |
| Q2 | **Delete dead abstractions** — `DiagnosticProducer` (0 impls), `ShaResolver` trait, `tool::hash` re-export shim | ★★ | S |
| Q3 | **Trim test triple-stacks** — lint/project per-kind mirrors, slice-validate goldens, schema-accept clones | ★★ CI time | S–M |
| Q4 | **Drop empty-tree rules** — `CORE-031` (no `evals/recorded/`), audit sibling scenario rules | ★ | S |

### Strategic bets (weeks, higher risk, decide deliberately)
| # | Action | Impact | Effort |
|---|--------|--------|--------|
| S1 | **Right-size the lint engine** — closed rule set no longer needs a 14-kind generic dispatcher (~20k LOC / ~58 rules ≈ 350 LOC/rule). Merge single-use kinds, flatten Road B, slim the project scan | ★★★ | L |
| S2 | **Relocate `framework_tools/` (~3–5.5k LOC) from the binary into `specify-standards`** and flatten the `ToolRunner` indirection for in-process checkers | ★★★ | M |
| S3 | **Merge micro-crates** — `digest`→`schema`, `validate`→`model`, `agents`→`workflow` | ★★ | M |

> The single highest-leverage *theme* is **"stop paying for extensibility you don't have yet."** The lint engine is a framework-for-third-parties built ahead of any third party. It can be collapsed to a concrete, closed implementation now and re-generalised later if a real second consumer appears.

---

## Part 1 — Architecture & non-idiomatic code (`specify-cli`)

### 1.1 Crate decomposition (12 crates)

The split is **mostly defensible** — `wasmtime` isolation (`tool` vs `tool-manifest`) and the standards↔workflow↔validate dependency-direction invariants are load-bearing, not cosmetic. The over-decomposition is at the edges:

| Crate | LOC | Verdict | Action |
|-------|-----|---------|--------|
| `digest` | 78 | **Ceremonial.** Two SHA-256 hex helpers + a streaming `Hasher`. Its stated rationale ("keep Wasmtime out of standards") is a red herring — `sha2` pulls nothing heavy. | **Merge into `specify-schema`** (`schema::digest`) or `specify-error`; delete the `tool/src/hash.rs` re-export shim. |
| `agents` | 2,292 | **Marginal.** Init-only `AGENTS.md` fence logic; only consumer is the binary; carries a module-level `#![allow(...)]` blanket from a verbatim move. No dependency-isolation win (already depends on `model`). | **Merge into `specify-workflow::agents`** (or keep as `src/runtime/commands/agents/` with `#[cfg(test)]`). |
| `validate` | 1,875 | **Small enough to nest.** Lifecycle-free artifact rule registry. | **Merge into `specify-model::validate`** — preserves the "no lifecycle authority" invariant (still no `workflow` dep) while dropping a sibling crate. |
| `tool` / `tool-manifest` | 3,684 / 1,177 | **Keep split** — `workflow` loads tool declarations without linking Wasmtime. Real boundary. | Keep. |
| `diagnostics` / `standards` | 2,285 / 14,956 | **Keep split** — standards must not depend on workflow; diagnostics is the neutral finding currency. | Keep. |
| `schema` / `error` / `model` / `workflow` | 706 / 818 / 3,713 / 31,159 | **Keep** — correct leaves and the domain root. | Keep; address internal module size (§1.2). |

**The "New workspace crates" bar in `DECISIONS.md:62-64` is good policy** — keep it, and apply it retroactively to retire `digest`/`agents`.

### 1.2 God modules (internal size is the real issue, not crate count)

`workflow` (31k) and `src/runtime` (14k) carry the complexity. Highest-signal splits:

| LOC | File | Problem | Action |
|-----|------|---------|--------|
| 503 | `src/runtime/commands/plan/lifecycle.rs` | **God handler** — `validate`, transitions, `next`, `status`, doctor in one file | Split to `validate.rs`/`transition.rs`/`next.rs`/`status.rs` |
| 615 | `crates/workflow/src/schema.rs` | Mixes JSON-schema wrappers, evidence-dir filesystem walks, and the `EvidenceDoc` domain type | Move evidence FS logic to `slice/evidence_validate.rs`; keep thin `validate_*` wrappers |
| 602 | `crates/workflow/src/journal/event.rs` | Event DTOs + wire tables split across `event.rs`/`wire_shapes.rs` only for line length | Acceptable; low priority |
| 577 | `crates/workflow/src/change/plan/core/model.rs` | Plan types | Acceptable |
| 524 | `crates/workflow/src/plugins.rs` | Marketplace parse + cache scan + git-sha + deletion | Low priority |

The `framework_tools/` subtree under `src/runtime/commands/lint/` is the biggest misplacement — see §2.1 / S2.

### 1.3 Non-idiomatic Rust & dead abstractions

| Sev | Item | Location | Issue | Action |
|-----|------|----------|-------|--------|
| **High** | `DiagnosticProducer` | `crates/standards/src/lint/producer.rs:29-36` | **Dead trait — zero `impl` anywhere (verified).** Both lint surfaces pass `producers: &[]` (`lint/project.rs:93`, `lint/framework.rs:83`). Residue of the removed imperative `Check` pass. | **Delete** trait + the `producers` pipeline field + runner hook. |
| **High** | `ShaResolver` | `crates/workflow/src/plugins.rs:198-230` | **Trait-for-testability** — forbidden by the repo's own `style.md:46-56`. Sole prod impl is `GitCli`; trait exists only for a `FakeResolver` test double. | Replace with a `CmdRunner`-style callable or a concrete test helper. |
| Med | `Platform` manual `Display`/`FromStr` | `crates/workflow/src/platform.rs:30-56` | Hand-written where peers use `strum` (`style.md:62-68` says derive) | Derive `strum::Display`/`EnumString`; delete manual impls |
| Low | `tool::hash` | `crates/tool/src/hash.rs:1-3` | Pure re-export of `specify_digest` | Delete with the `digest` merge |
| Low | `workflow::schema` re-export block | `crates/workflow/src/schema.rs:25-32` | Duplicates `specify_schema`'s surface | Callers import `specify_schema` directly |

> Note: `CmdRunner`, `ToolRunner` (project/WASI side), and `AtomicYaml` are **legitimate** trait boundaries blessed by `style.md` — don't touch them. The codebase is otherwise clean: no `RenderInput` wrappers, lint suppressions are sparse and reason-tagged (~30 files, mostly one `#[expect]` each).

### 1.4 Heavy dependency footprint

| Dep(s) | Pulled in for | Concern |
|--------|---------------|---------|
| `wasmtime` 45 + cranelift (12 dev-profile opt-level overrides in `Cargo.toml:205-228`) | `specify-tool` only — running 2 first-party WASI validators | Largest build-time/dep cost in the workspace |
| `tokio` + `wasm-pkg-client` + `futures-util` | **A single file**: `crates/tool/src/package.rs` (OCI package fetch behind `specify tool fetch`) | An entire async stack for a fetch path with no current first-party consumer (contract is checked-in, vectis is built locally) |
| `nursery` clippy group (`Cargo.toml:92`) | Lint strictness | `nursery` lints are unstable — a routine toolchain bump can spontaneously break CI. Consider pinning to `pedantic`+selected `restriction` and dropping `nursery`. |

---

## Part 2 — YAGNI / over-engineering

### 2.1 The framework lint engine (S1) — over-engineered for a closed rule set

The lint stack is **~20,106 LOC enforcing ~58 hint-bearing rules (~350 LOC/rule)**. It was a sensible *migration chassis* (imperative `Check` → declarative burn-down), but with the migration complete and the rule catalog **closed and repo-owned**, the generality no longer pays.

**Evidence of disproportion:**
- **37 of ~95 rules carry no executable hint at all** — they're prose-only review guidance the engine skips (`eval.rs:352-356`).
- Single/low-use evaluator kinds carry full machinery: `set-eq` → **1 rule** (229 LOC, `eval/set_eq.rs`); `cross-reference` → 2 rules (407 LOC); `cli-contract` → 2 rules (963 LOC); `constant-eq`/`unique`/`field-grammar` → 2 each.
- The **dual `ScanProfile`** materialises a full `WorkspaceModel` then zeroes five fact families for project lint (`index.rs:174-179`); four kinds are framework-only in practice.
- The **Road B `ToolRunner`** indirection is pure ceremony for *in-process* checkers — they serialise to a JSON `DiagnosticReport` and re-parse it, when they could be `fn check_scenarios(...) -> Vec<Diagnostic>`.
- A `no_embedded_policy` guard test (~140 LOC) exists solely to force CORE caps/sets into markdown YAML frontmatter — optimising for multi-repo policy distribution that has one consumer (this monorepo).

**Phased simplification (conservative ~4–6k LOC / ~20–30%; aggressive ~10k+ / ~50%):**
1. Delete dead ceremony — `DiagnosticProducer`, `ResolverDegradation::SkipDeclarative` stub, stale producer docs (~300–500 LOC).
2. Merge `set-eq` into `set-coverage` (`mode: exact|subset`); demote `cli-contract` and `cross-reference` from generic "kinds" to bespoke modules called by their 2 rules each (~1.5–2.5k LOC).
3. Flatten Road B: move checkers into `specify-standards` as plain functions; keep `ToolRunner` only for the genuine project-side WASI path (~0.8–1.2k LOC).
4. Replace the project-profile `WorkspaceModel` with a lighter `ProjectScan` (files + frontmatter + links + ignore directives) (~1–1.5k LOC).
5. Optionally relocate CORE numeric policy into Rust constants keyed by rule id; keep rule markdown as human docs.

### 2.2 Placeholder / aspirational code (CLI)

| Sev | Item | Location | Action |
|-----|------|----------|--------|
| Low | `ShaResolver::ls_remote` | `crates/workflow/src/plugins.rs:208-211` | "Inert today: no shipping source is a URL." Delete until URL plugin sources ship. |
| Low | `binary`-channel self-replace | `crates/workflow/src/upgrade.rs` | Already deferred in `DECISIONS.md`; fine as-is. |

---

## Part 3 — Testing

Posture is sound (integration-first, ~410 integration + ~1,100 unit `#[test]`, no rstest matrices, the three-layer pyramid in `testing.md` is explicit). The imbalance is **triple-stacking the same behavior** across unit + crate-golden + binary layers — which `testing.md:21-34` itself forbids ("one layer owns a behavior").

### 3.1 Unit/integration overlap — fold or delete (HIGH confidence)

| Location | Why safe to trim |
|----------|------------------|
| `tests/lint/project.rs:563-814` (7 per-kind tests) | Mirror `crates/standards/tests/lint_hint/` + eval unit tests. `testing.md:32`: "never one binary test per rule outcome." **Keep one smoke (`review_emits_important_exits_2`) + the unit eval tests; delete the seven mirrors.** |
| `crates/workflow/tests/goldens/slice_validate.rs:86-94` | Same `DiagnosticReport` pinned **three** ways (here + `tests/slice/validate.rs` + `tests/e2e.rs` golden). **Keep one binary golden; delete the crate golden.** |
| `crates/workflow/src/schema/tests.rs:17-87` (RFC-accept clones) | Schema acceptance already covered in `crates/schema/tests/schemas.rs`. **Keep only the wrapper-error tests; drop the accept clones.** |
| `crates/workflow/src/change/plan/core/propose/tests.rs:33-83` | Happy-path request envelope duplicates `tests/workflow/propose.rs` goldens. **Keep the error/reconcile matrix unit-side; drop happy-path JSON clones.** |
| `crates/workflow/src/merge/engine/tests.rs:4-21` | Greenfield happy paths superseded by `tests/goldens/merge_engine.rs`. **Keep only the error tests.** |
| `crates/workflow/src/init/tests.rs:7-65` | `init-requires-adapter-or-workspace` duplicated with `tests/init/base.rs`. **Keep one layer (prefer integration).** |

> Correctly-split (do **not** touch): `registry`, `workspace`, `journal`, `provenance` unit/integration pairs are documented intentional splits per the pyramid.

### 3.2 CPU sinks (ranked)

| Rank | Test(s) | Cost | Recommendation |
|------|---------|------|----------------|
| 1 | `tests/plan/end_to_end.rs` (1,175 LOC, 33× binary spawn) | Full fan-in/fan-out loop; dominates the plan binary | **Keep** (RM-05 acceptance proof) but treat as expensive acceptance, not a place to add cases |
| 2 | WASM-JIT paths — `catalog_infer.rs` report tests, `end_to_end` vectis build, `tool/schema.rs`, `tool/run.rs` | First-run wasmtime/cranelift compile; nextest forces `max-threads=1` | See gap below — they **soft-skip silently** when the vectis blob is absent |
| 3–5 | `journal.rs` (841), `slice/synthesize.rs` (885), `lint/project.rs` (844) | Repeated init+verb+golden scaffolds | Trim per §3.1; share scaffold helpers |

### 3.3 Keep — high value, low cost (don't "optimise" these away)
- `crates/schema/tests/schemas.rs` byte-parity (`embedded_schemas_match_on_disk_sources`) — catches stale `include_str!` drift; churn is *intentional* on wire-contract edits.
- `tests/rust_quality/` **gated** predicates (test-fn-name length, workflow clock reads, allow-without-reason) — cheap, enforce invariants clippy can't. **Drop only the disabled `archaeology-in-doc-comment` advisory** (it over-fires on canonical vocabulary).
- `diagnostics` fingerprint golden — algorithm canary, nanoseconds.

### 3.4 Coverage gaps (worth closing)
| Gap | Sev |
|-----|-----|
| **Vectis WASM tests soft-skip with no assertion** when `vectis.wasm` is absent (`catalog_infer.rs:163-168`) — CI may be silently green without running them | **HIGH** — make CI always build the blob or mark the skip explicitly |
| `specify agents` commands — only an init smoke; no binary regression | Med |
| `specify archive prune` retention policy | Med |

---

## Part 4 — Archeology removal plan

`DECISIONS.md` is a **hybrid**: a live decision log (keep) wrapped in RFC/phase rollout narrative (delete). The `rfcs/` tree is dead weight, and — notably — **`CORE-016` already bans design-history RFC citations in operator prose, yet `DECISIONS.md`/`workflow.md`/`roadmap.md` violate it.** Clean up, then the rule stops being self-contradicting.

### 4.1 Tier 0 — safe to delete outright
1. **Root scratch files:** `names.md`, `names-2.md`, `names-3.md`, `pitch.md` (branding exercise, not framework content). Move to a private product repo or gitignore.
2. **`rfcs/archive/`** (RFC-40/44/45, ~858 lines of "implemented and archived" narrative) — outcomes already live in `DECISIONS.md`.
3. **Fix the broken link first:** `rfcs/archive/rfc-44-architecture-seams.md` references `../rfc-43-release-proving.md`, which **does not exist** (verified). It vanishes when the archive is deleted.
4. **`CORE-031`** — requires `evals/recorded/**/*.jsonl`; that tree is **empty** (verified). Delete the rule until recorded traces ship; audit ~8 sibling scenario rules for similar empty-tree assumptions.
5. **Rust comment archaeology:** `merge/artifact_class.rs:45` (chunk/phase refs), vectis `MANIFEST.md` chunk notes, `agents/src/lock.rs:47` R6 note, `tests/lint/project.rs` phase comments.

### 4.2 Tier 1 — migrate anchors, *then* delete labels (load-bearing)
`D1`/`D2`/`D9`/`§F1` and several `DECISIONS.md#…-rfc-NN` hashes are **cited across ~50 files in `specify-cli` and ~8 in `specify`**. Rename the headings to topic names and update the cite graph in one coordinated pass:
- `Slice synthesis engine (RFC-29 M2b)` → `Slice synthesis engine`
- `Source operations (D1)` / `Lead reconciliation (D2)` / `Adapter execution mode (D9)` → drop the suffixes
- `… (RFC-45/44/40/36)` headings → topic names
- Rename `tests/fixtures/rfc-29/` → `tests/fixtures/fan-in-fan-out/` and update `end_to_end.rs` + the DECISIONS proof paragraph
- Strip `B-2 exit` / `authoring-predicate bridge` / `Wave-0` / `review F3`/`F8` migration narrative from `DECISIONS.md`/`DIAGNOSTICS.md`/`AGENTS.md` (both repos) and `.cursor/rules/project.mdc`

### 4.3 Tier 2 — `rfcs/roadmap.md` & `CORE-016`
- **`rfcs/roadmap.md`** is a live forward plan **contaminated** with `RFC-28/32/34/36/38` and "was RFC-N" parked ideas. Either rewrite without RFC ids or relocate to `docs/explanation/roadmap.md`. Migrate the `RM-05` status it tracks into `evals/scenarios/README.md`. Update `CORE-023` (cites `rfcs/roadmap.md`) and the framework-lint walker include set (`framework.rs:260`) accordingly.
- **`CORE-016`** — **keep it** (it encodes the desired forward-looking posture), but fix the violations *first*, then optionally add a `DECISIONS.md` exemption if it remains an engineering log with trimmed labels. Deleting it would let archeology creep back.

### 4.4 Genuinely current — do NOT remove
`Phase N` in build briefs (contracts/typescript) is **current procedural numbering**, not RFC history. `RFC-5322`/`3339`/`8141` are **IETF** standards in email/serde code. `style.md`/`skill-authoring.md` "history is deleted" prose is **current enforcement policy**. "No migration framework, pre-1.0" is **current** bootstrap policy.

---

## Appendix A — Effort × impact map

```
            IMPACT →
        low            medium             high
  E  ┌──────────────────────────────────────────────────┐
  F  │              │ Q4 empty rules  │ Q1 archeology    │
  F h│              │                 │ S2 framework_tools│
  O i│              │                 │   relocate        │
  R g│              │ Q2 dead traits  │                   │
  T  ├──────────────────────────────────────────────────┤
  ↑ m│              │ Q3 test trims   │ S1 lint engine    │
    e│              │ S3 micro-crates │                   │
    d│              │                 │                   │
  ─  ├──────────────────────────────────────────────────┤
    l│              │ §1.2 god-module │                   │
    o│              │   splits        │                   │
    w│              │                 │                   │
     └──────────────────────────────────────────────────┘
```

**Suggested sequence:** Q1→Q2→Q4 (clear the decks) → Q3 + S2 (test + layering) → S1 + S3 (engine + crates).

## Appendix B — Headline metrics
- **CLI Rust:** ~82k LOC across 12 crates (`workflow` 31k, `standards` 15k, binary 14k) + ~18k test LOC.
- **Lint stack:** ~20k LOC / ~58 executable rules (~350 LOC/rule); 37 rules carry no hint.
- **Plugin repo markdown:** Omnia ~17k, Vectis ~13k, contracts ~6k, typescript ~3.8k lines.
- **Verified dead/broken:** `DiagnosticProducer` (0 impls), `rfc-43-release-proving.md` link (missing target), `evals/recorded/` (empty, breaks `CORE-031`).
- **Evals:** 13 scenarios (1 pending), 12 passing runs; coverage real for intent/documentation/typescript-survey/omnia, absent for vectis/screenshots/contracts/captures.
