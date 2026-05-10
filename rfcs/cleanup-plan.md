# Cleanup Plan: Idiomatic Rust + Skill Polish

> Status: Draft (rev 2 — supersedes rev 1)
> Scope: `augentic/specify` (skills) + `augentic/specify-cli` (Rust workspace)
> Source: Code & Skills review, May 2026, ground-truthed against live `cargo make standards-check` and `make checks` output.

## Thesis

The framework already encodes a high standard: structured `specify-error::Error`, kebab-case discriminants, four-slot exit-code contract, `Render` + `emit` format-dispatch, atomic writes, supply-chain hygiene, an in-house `xtask standards-check` with seven AST/regex predicates and a per-file ratchet, and 28 documentation-discipline checks in the `specify` repo's Deno orchestrator. The dominant problem is not what's there — it's that the migration to those standards stalled before crossing the finish line. Live totals as of HEAD:

| Predicate | Total |
|---|---|
| `format-match-dispatch` | 47 |
| `inline-dtos` | 39 |
| `name-suffix-duplication` | 22 |
| `rfc-numbers-in-code` | 116 |
| `ritual-doc-paragraphs` | 41 |
| `no-op-forwarders` | 0 |

`cargo make standards-check` is **failing right now** in 6 places — most of them caused by an in-flight refactor of `vectis-validate` + `vectis-scaffold` into a single new `vectis` crate. `make checks` in the parent repo is **also failing** (link checker, including 3 false positives this plan introduced last revision — fixed below).

This plan is **chunked for subagent execution**. Each chunk is independently shippable, with explicit dependencies and acceptance criteria. Where chunks must run sequentially, the dependency is named.

## What changed since rev 1

Material findings from the second-pass review:

1. **Working-tree refactor is live.** `crates/vectis-scaffold/` and `crates/vectis-validate/` are *deleted*; a new `crates/vectis/` (lib + `vectis` bin) consolidates them. Rev 1 chunks that referenced the old crate names are obsolete and have been replaced.
2. **A new `crates/contract-validate/` exists.** It is a deliberate carve-out — a thin standalone WASI binary that does not (and should not) use `Render`/`emit`/`specify-error`. It is excluded from Phase 2.
3. **The standards-check ratchet drifted on the in-flight branch.** Three files have grown past their baseline by 1–2 lines (`compatibility/openapi_diff.rs`, `validate/lib.rs`, `commands/workspace.rs`). One new file blew the default 500-line cap (`vectis/src/scaffold.rs` at 648). These are the immediate failures and must be cleared first — they are Phase 0 below.
4. **Phase 2 is wider than rev 1 said.** The 47 `format-match-dispatch` hits land in **eleven** modules, not five. All eleven are listed.
5. **Phase 3 is wider too.** Sixteen non-test files exceed the 500 cap, not three. All sixteen are listed with split sketches.
6. **Skill caps are already 470 / 1024 / 30 lines** (`MAX_BODY_LINES`, `MAX_DESCRIPTION_CHARS`, `MAX_INLINE_JSON_LINES`). Tightening them is a deliberate policy choice, not "bringing files under cap" as rev 1 implied.
7. **`checkSymlinks` validates that symlinks *resolve*, not that they're forbidden.** Whether to disallow symlinks under `plugins/` outright is a policy call this plan defers to a future revision.
8. **AGENTS.md `Coding standards` is mature.** It already covers comments, naming, format dispatch, DTOs, errors, `#[non_exhaustive]`, deprecation, `#[allow]` posture, module layout, no-op forwarders, wired-but-ignored flags, and the mechanical-enforcement table. Rev 1's CL-20 ("lift to STYLE.md") was reaching; the right move is small, surgical additions to the existing section, not a relocation.
9. **rev 1 broke `make checks` in the parent repo** by using illustrative path syntax (`rfcs/...`, `./STYLE.md`, `../../references/specify.md`) inside prose. The link checker strips fenced blocks but not inline-code spans. Rev 2 routes every illustrative path through fenced blocks.

## Conventions

Every chunk follows the same shape:

- **Goal** — one sentence.
- **Repo / scope** — which files change.
- **Depends on** — earlier chunk IDs that must land first.
- **Steps** — ordered, executable.
- **Acceptance** — checked on PR. `cargo make ci` (in `specify-cli`) or `make checks` (in `specify`), plus chunk-specific assertions.
- **Out of scope** — what NOT to do in the same PR.

A chunk is one PR. Subagents should not bundle.

## Dependency graph

```text
Phase 0 — Working-tree cleanup (sequential, run first)
  CL-00a → CL-00b

Phase 1 — Foundation (sequential)
  CL-01 → CL-02

Phase 2 — Standards migration (parallel after Phase 1; one PR per file)
  CL-03  CL-04  CL-05  CL-06  CL-07  CL-08  CL-09  CL-10  CL-11

Phase 3 — Module splits (parallel after Phase 1; CL-MS-CLI needs Phase 2)
  CL-MS-CHANGE-FINALIZE   CL-MS-CHANGE-DOCTOR     CL-MS-CHANGE-LOCK
  CL-MS-MERGE-SLICE       CL-MS-SLICE-LIB         CL-MS-TOOL-CACHE
  CL-MS-CONTEXT           CL-MS-CONTEXT-DETECT    CL-MS-CONTEXT-FENCES
  CL-MS-REGISTRY-BRANCH   CL-MS-VECTIS-SCAFFOLD   CL-MS-CLI

Phase 4 — Error & naming polish (parallel after Phase 2)
  CL-E1  CL-E2  CL-E3  CL-N1  CL-N2

Phase 5 — Skills polish (parallel; specify repo)
  CL-S01  CL-S02  CL-S03  CL-S05  CL-S06

Phase 6 — Standards & docs (after Phase 4)
  CL-X1  CL-X2  CL-X3  CL-X4
```

`cargo make standards-tighten` is run at the end of every chunk that touches Rust code; the resulting `scripts/standards-allowlist.toml` diff is committed in the same PR.

---

## Phase 0 — Working-tree cleanup (run first)

These exist because the working tree at HEAD is currently failing both gates. They unblock everything else. They do not need subagents — they are short and mechanical.

### CL-00a — Re-bake drifted standards baselines

**Goal:** Clear the six `cargo make standards-check` failures introduced by the live in-flight refactor.

**Repo:** `specify-cli`. **Depends on:** none.

**Steps:**

1. The four files that grew by 1–2 lines from routine edits get their baselines re-baked. Run `cargo make standards-tighten` and commit `scripts/standards-allowlist.toml` only.
2. The three new failures that are *not* drift but real regressions get treated:
   - `crates/vectis/src/lib.rs` — 1 inline DTO. Promote it to a top-level struct + `Render` impl. Either delete the inline DTO or relocate it to a sibling module.
   - `crates/vectis/src/scaffold/versions.rs` — 1 RFC number in code. Move the citation into `crates/vectis/DECISIONS.md` or delete; the comment loses nothing.
   - `crates/vectis/src/scaffold.rs` — 648 lines, no allowlist entry, default cap is 500. Either accept the baseline by stamping it (interim — see CL-MS-VECTIS-SCAFFOLD for the real split) or split now.
3. Recommended path: stamp the baseline now (`module-line-count = 648`) so Phase 0 lands quickly; CL-MS-VECTIS-SCAFFOLD does the real split later.

**Acceptance:**

- `cargo make standards-check` exits 0.
- `cargo make ci` green.

---

### CL-00b — Fix `make checks` link-checker false positives

**Goal:** Clear the five link-checker failures in the parent repo.

**Repo:** `specify`. **Depends on:** none.

**Steps:**

1. Three of the failures came from `rfcs/cleanup-plan.md` rev 1 illustrative paths (`rfcs/...`, `./STYLE.md`, `../../references/specify.md`). Rev 2 of this file (the document you're reading) wraps every illustrative path in a fenced block, so they are no longer flagged. No further action.
2. Two of the failures are pre-existing in `AGENTS.md`:
   - `AGENTS.md` line 106 contains an inline-code span with a markdown link inside it. Specifically, the line says (in fenced form to avoid breaking this very check):

     ```markdown
     > See [Phase outcome contract](../../references/phase-outcome-contract.md).
     ```

     The link checker strips fenced blocks but not inline-code spans, so it parses the link.
   - One option: extend `scripts/checks/links.ts::checkMarkdownLinks` to also strip inline-code spans (`/`[^`]+`/g`) before the link regex. This is the right structural fix and benefits every other doc.
   - Other option: replace the inline-code span with a fenced block. Less invasive but loses some compactness.
3. Pick option 1 (the structural fix) — it's three lines of code in `links.ts` and removes a foot-gun for everyone.

**Acceptance:**

- `make checks` exits 0 on a clean clone.
- A new test fixture (`tests/plan/inline-code-link.md`) verifies that a link inside a single-backtick span is no longer flagged.

---

## Phase 1 — Foundation

### CL-01 — Extract `crates/config/`

**Goal:** Move `ProjectConfig` and the `.specify/` / repo-root path helpers into a dedicated workspace crate so every consumer (binary, `crates/change`, future crates) shares one source of truth. Currently `crates/change/src/finalize.rs` lines 62–89 duplicate four path helpers from `src/config.rs`, and `find_project_root` lives in `src/context.rs:64-74` with no good reason.

**Repo:** `specify-cli`. **Depends on:** CL-00a.

**Steps:**

1. Create `crates/config/` with `Cargo.toml` (deps: `specify-error`, `serde`, `serde-saphyr`, `semver`, `specify-tool` for `Tool` re-use, `specify-slice` for `SLICES_DIR_NAME`, `specify-capability` for `CHANGE_BRIEF_FILENAME`).
2. Move the body of `src/config.rs` to `crates/config/src/lib.rs`. Move `find_project_root` from `src/context.rs:64-74` to `ProjectConfig::find_root` on the new crate.
3. Add `specify-config = { path = "crates/config" }` to root `Cargo.toml` `[workspace.dependencies]`.
4. Make the binary depend on `specify-config`. Replace `use specify::config::ProjectConfig` with `use specify_config::ProjectConfig` everywhere (binary + `crates/change`).
5. Delete the inlined path helpers in `crates/change/src/finalize.rs:74-89` (`specify_dir`, `plan_path`, `change_brief_path`, `archive_dir`); use `ProjectConfig::*` instead. Add `specify-config` to `crates/change/Cargo.toml`.
6. Move `is_workspace_clone_path` (`src/config.rs:201-208`) into `specify_config`; update call sites.
7. Run `cargo make standards-tighten` and commit the allowlist diff.

**Hard rule:** `specify-error` is the workspace leaf and depends on no other workspace crate (AGENTS.md `Workspace layout`). `specify-config` depends on `specify-error`, never the reverse.

**Acceptance:**

- `cargo make ci` green.
- `crates/change/src/finalize.rs` lines 62–89 (the "Path helpers" comment block + four `fn`s) no longer exist.
- `rg 'specify::config::' src crates` returns zero hits.
- `crates/config/src/lib.rs` carries the tests previously in `src/config.rs`.

**Out of scope:** any logic changes to `ProjectConfig::load`, the version-floor check, or the cache-meta interaction.

---

### CL-02 — Extract `crates/init/`

**Goal:** Lift `src/init.rs` (1024 lines, 12 RFC citations) into `crates/init/`, restoring the AGENTS.md invariant that the binary owns only argv parsing, formatting, and dispatch.

**Repo:** `specify-cli`. **Depends on:** CL-01.

**Steps:**

1. Create `crates/init/` with `Cargo.toml` (deps: `specify-config`, `specify-error`, `specify-capability`, `specify-registry`, `chrono`).
2. Move `src/init.rs` body to `crates/init/src/`. Split along its existing seams while moving:
   - `crates/init/src/lib.rs` — public `init`, `InitOptions`, `VersionMode`, `InitResult`.
   - `crates/init/src/regular.rs` — current non-hub body (lines 107–179).
   - `crates/init/src/hub.rs` — current `init_hub` (lines 224–296).
   - `crates/init/src/capability_uri.rs` — `CapabilityUri`, `GithubCapabilityUri`, `is_github_url`, `split_ref_suffix`.
   - `crates/init/src/git.rs` — `sparse_checkout_github`, `run_git`, `unique_temp_dir`.
   - `crates/init/src/cache.rs` — `cache_capability`, `refresh_cached_capability`, `copy_dir_recursive`, `write_cache_meta`, `cache_sibling_default_capability`.
3. Each new file ≤ 300 lines.
4. Update `src/lib.rs` to delete `pub mod init;`. If only `pub mod config;` remained from CL-01 cleanup, delete `src/lib.rs` entirely (the binary no longer needs a library) and remove the `[lib]` section from `Cargo.toml`. Update `src/main.rs` and `src/commands/init.rs` accordingly.
5. Strip RFC citations from doc comments while moving (currently 12; allowlist baseline drops to 0). Move durable rationale into the new crate's `DECISIONS.md` if necessary, otherwise delete.
6. **Preserve the floor-check skip.** `specify init` runs before `project.yaml` exists and intentionally bypasses the `specify_version` floor (AGENTS.md `Gotchas`). The new module must not call `ProjectConfig::load`.
7. Run `cargo make standards-tighten`.

**Acceptance:**

- `cargo make ci` green.
- Every file in `crates/init/src/` ≤ 300 lines.
- `src/init.rs` no longer exists.
- `rfc-numbers-in-code` baseline for `crates/init/**` is 0.
- Either `src/lib.rs` is deleted, or it contains only re-exports under 10 lines.

**Out of scope:** changes to init behaviour, error messages, or the `--hub` discriminator. Pure relocation + ergonomic split.

---

## Phase 2 — Standards migration

Each chunk migrates one binary command module from the legacy pattern (inline DTOs, hand-rolled `match ctx.format`, hand-rolled error envelopes) to the canonical pattern (top-level `*Body` DTOs, `Render` + `emit`, typed `Error` variants → `emit_error`). AGENTS.md §Format dispatch + §DTOs + §Errors are normative.

**Reference implementation:** `src/commands/codex.rs` (cited as the canonical pattern in AGENTS.md line 205) and `src/commands/slice/lifecycle.rs:40-77`.

**Common steps for CL-03 through CL-11:**

1. For each handler in the target module: define a top-level `<Action>Body` (or `<Action>Row`) struct, `derive(Serialize)`, `#[serde(rename_all = "kebab-case")]`, and `impl Render for <Action>Body`.
2. Replace every inline `#[derive(Serialize)] struct …` (inside a function body, match arm, or closure) with a call to `emit(ctx.format, &body)?`.
3. Delete every `match ctx.format { Json => emit_response(…), Text => println!(…) }` block.
4. Replace every hand-rolled error envelope (`Ok(CliResult::GenericFailure)` + ad-hoc `emit_response(<error body>)`, or direct construction of `output::ErrorResponse`) with `Err(Error::*)`. Where no typed variant fits, use `Error::Diag { code, detail }`. Recurring shapes are promoted in CL-E1.
5. Preserve `ok: true` literal fields on success bodies for now — keep them as `bool` fields on the top-level `*Body` DTO so existing `tests/cli.rs` JSON snapshots stay byte-identical. CL-E3 strips them system-wide and updates the snapshots in one focused PR.
6. Drop redundant per-file `#![allow(clippy::needless_pass_by_value, items_after_statements, …)]`. The parent `src/commands.rs` already carries the project-wide waiver.
7. Run `cargo make standards-tighten`.

**Common acceptance for CL-03 through CL-11:**

- Module's allowlist baselines for `inline-dtos`, `format-match-dispatch`, `name-suffix-duplication` drop to 0.
- Module's `module-line-count` baseline typically drops by ≥ 15% for files with ≤ 4 handlers. Files with more handlers, or with `Render` impls that own non-trivial text rendering (e.g. multi-status counters), sit at a structural floor and may shrink less or even grow by single-digit percent — note the reason in the commit message when below 15%. The load-bearing acceptance is the inline-dtos / format-match-dispatch / name-suffix-duplication predicates dropping to 0 and the cumulative workspace LoC dropping.
- JSON envelope keys are preserved verbatim — the migration is structural, not contractual. Per-module `#[cfg(test)]` updates that mechanically follow renames in the Common-steps rename table are expected.
- `tests/cli.rs` golden JSON snapshots unchanged.

If a handler's failure envelope is non-standard (carries `path` / `kind` / additional fields that tests pin), keep that shape: introduce a top-level `<Action>ErrBody` + `Render`, emit via `emit`, and return `Ok(CliResult::ValidationFailed)` (mirroring `codex::validate`). Do not force such handlers through `output::ErrorResponse` — that would break the contract.

The full Phase 2 worklist, sorted by violation density:

| ID | File | inline-dtos | format-match | name-suffix | LoC | Notes |
|---|---|---:|---:|---:|---:|---|
| CL-03 | `src/commands/registry.rs` | 7 | 7 | 7 | 927 | Highest debt; rename `show_registry → show`, `validate_registry → validate`, `add_to_registry → add`, `remove_from_registry → remove`, `print_registry_text → print`, `plan_references_for → plan_refs`, `write_registry → save`. |
| CL-04 | `src/commands/workspace.rs` | 8 | 7 | 0 | 438 | Three duplicated `workspace-no-registry` Diag sites consolidate to `Error::RegistryMissing` (already exists at `crates/error/src/lib.rs:209`). |
| CL-05 | `src/commands/change/plan/lifecycle.rs` | 7 | 8 | 0 | 349 | The `ValidationRow` shape is shared with `commands/codex.rs`; promote a shared `Validation` rendering helper to `src/output.rs`. |
| CL-06 | `src/commands/change.rs` | 7 | 7 | 0 | 413 | Promote `BriefCreateErr` → `Error::ChangeBriefExists { path }`, `PlanNotFound` → `Error::PlanNotFound`, `NonTerminal` → `Error::PlanNonTerminalEntries`. The legacy `initiative` field is renamed to `change` on the **non-terminal failure envelope only**. The success-path `finalize::Outcome.name` still serializes as `initiative` and stays that way until a separate, focused chunk renames it (touches `tests/cli.rs:1479,1503` + `tests/cross_repo.rs:593`). |
| CL-07 | `src/commands/tool.rs` | 0 | 4 | 1 | 451 | DTOs are already top-level — easiest of the eleven. Just swap four `match ctx.format` blocks for `emit`. Rename `find_tool → find`, `select_tools → select`. |
| CL-08 | `src/commands/change/plan/status.rs` | 4 | 2 | 1 | 257 | |
| CL-09 | `src/commands/change/plan/lock.rs` | 3 | 3 | 0 | 140 | |
| CL-10 | `src/commands/change/plan/create.rs` | 2 | 2 | 0 | 117 | |
| CL-11 | `src/commands/{change/plan,change/plan/doctor,context}.rs` | 0 | 1+1+2 | 0+0+1 | 177+117+682 | Three small clean-ups; `context.rs` carries the largest residual since it's also targeted by CL-MS-CONTEXT for splitting. Migrate format dispatch only; the split lands separately. |

`src/commands/codex.rs` (398 LoC) is **already migrated** — do not touch (CL-05 promoted its shared validation row to `src/output.rs::ValidationRow`).
`src/output.rs` is the dispatcher; its `match ctx.format { Json, Text }` blocks (today on `emit`, `emit_err`, `emit_response`) are legitimate. CL-05 added `emit_err` for non-standard failure envelopes that need stderr text routing.
`crates/contract-validate/src/main.rs` (147 LoC, 1 format-match-dispatch) is a deliberate carve-out (standalone WASI binary, no `specify-error` dependency by design) — **leave alone**.

---

## Phase 3 — Module splits

Sixteen non-test files exceed the 500-line cap or sit close enough to it that a routine edit pushes them over. Each is one chunk; each follows the same shape:

1. Pick the natural seam (the file's existing comment-banner sections, or one verb / one concern per submodule).
2. Convert `src/<parent>/<module>.rs` into `src/<parent>/<module>/mod.rs` only if absolutely necessary; per AGENTS.md `Module layout`, **prefer `<parent>/<module>.rs` + `<parent>/<module>/<concern>.rs`**.
3. No public-API change; pure relocation.
4. Strip RFC citations from doc comments while moving — most splits drop the file's `rfc-numbers-in-code` baseline by 5+.
5. If a fat inline `#[cfg(test)] mod tests` dominates the LoC, move it to `<module>/tests.rs`. The standards-check skips files whose path ends in `tests.rs` for the `module-line-count` predicate, so the test block doesn't have to be artificially split across the new submodules.
6. `cargo make standards-tighten`.

**Common acceptance:** every resulting file ≤ 500 LoC (or the file is grandfathered with a justification baseline); `cargo make ci` green; tests pass unchanged.

| ID | File | LoC | Sketch of seams |
|---|---|---:|---|
| CL-MS-CHANGE-FINALIZE | `crates/change/src/finalize.rs` | 1569 | `mod.rs` (public surface) / `archive.rs` (atomic plan/brief/plans-dir sweep) / `probe.rs` (`gh pr view`, `git status`, branch matching) / `summary.rs` (aggregation, `blocked_reason`, `is_passing`). Also strips 7 RFC citations. |
| CL-MS-CHANGE-DOCTOR | `crates/change/src/plan/doctor.rs` | 1128 | `mod.rs` (public `run`, `Diagnostic`) / `cycle.rs` / `orphan_source.rs` / `stale_clone.rs` / `unreachable.rs`. Strips 8 RFC citations. |
| CL-MS-CHANGE-LOCK | `crates/change/src/plan/lock.rs` | 691 | `mod.rs` (public surface) / `acquire.rs` / `release.rs` / `status.rs` / `pid.rs` (PID-validity check). Strips 2 RFC citations + 7 ritual-doc paragraphs. |
| CL-MS-MERGE-SLICE | `crates/merge/src/slice.rs` | 761 | `mod.rs` / `read.rs` (delta-spec discovery, baseline read) / `write.rs` (atomic baseline write, archive move) / `parse.rs` (`parse_rfc3339`, mtime conversion). The 30+ `Error::Diag { code: "merge-*-failed" }` sites surface as the right place to introduce `Error::Filesystem { op, path, source }` — see CL-E1. |
| CL-MS-SLICE-LIB | `crates/slice/src/lib.rs` | 709 | `mod.rs` (re-exports) / `metadata.rs` (`Metadata` struct + atomic save) / `lifecycle.rs` (state machine) / `outcome.rs` (`Outcome` enum + transitions). Strips 6 RFC citations. |
| CL-MS-TOOL-CACHE | `crates/tool/src/cache.rs` | 778 | `mod.rs` / `fetch.rs` (download + atomic install) / `gc.rs` (orphan-cache scan) / `meta.rs` (cache-meta read/write). |
| CL-MS-CONTEXT | `src/commands/context.rs` | 682 | `mod.rs` (public `run`) / `generate.rs` / `check.rs`. Two `format-match-dispatch` go away in CL-11; the rest of the size is structural and survives the migration. |
| CL-MS-CONTEXT-DETECT | `src/commands/context/detect.rs` | 735 | `detect.rs` (parent) / `runtimes.rs` (per-language `Detector` passes + orchestrator) / `markers.rs` (TOML / JSON-with-comments / Makefile / go.mod parsers). The earlier "AGENTS.md detection / fence + lock-file probes" sketch was a mis-attribution — those concerns live in `context/fences.rs` and `context/lock.rs`, both already split out. |
| CL-MS-CONTEXT-FENCES | `src/commands/context/fences.rs` | 472 | Just under cap today but routinely grows. Split now: `mod.rs` / `parse.rs` / `render.rs`. Drops the 1 `name-suffix-duplication` baseline. |
| CL-MS-REGISTRY-BRANCH | `crates/registry/src/branch.rs` | 642 | `mod.rs` / `prepare.rs` / `infer.rs` / `validate.rs`. Strips 2 RFC citations + 2 name-suffix duplication. |
| CL-MS-VECTIS-SCAFFOLD | `crates/vectis/src/scaffold.rs` | 648 | The new file from the in-flight refactor. `mod.rs` keeps the clap derive types + dispatch; `runtime.rs` carries the planner + writer. The `scaffold/error.rs`, `scaffold/templates*`, `scaffold/versions.rs` submodules already exist; the dispatcher is what's oversized. |
| CL-MS-CLI | `src/cli.rs` | 704 | Move every per-verb action enum into `src/commands/<verb>/cli.rs`. Keep `Cli`, `Commands`, `OutputFormat`, `parse_source_kv` in `src/cli.rs`; everything else relocates. Consolidate `OutcomeKindAction::RegistryAmendmentRequired`'s 7 named `#[arg(long)]` fields into a `RegistryAmendmentArgs` struct used via `#[clap(flatten)]`. RFC citations inside clap doc-comments are preserved (they render in `--help`); convert to hyperlinks in a separate chunk. **Depends on Phase 2** (so the per-command modules are settled). |

The following are slightly over cap (500–565) but worth deferring as they are tightly cohesive and split poorly:

- `crates/tool/src/resolver.rs` (565) — single dependency-resolution algorithm; splitting hurts readability.
- `crates/slice/src/actions.rs` (529) — one action per public function; the file is already a flat module of related verbs.
- `src/commands/slice/merge.rs` (475) — under cap but worth watching.

Bake their current LoC into the allowlist as the cap and revisit if they grow past 600.

---

## Phase 4 — Error & naming polish

### CL-E1 — Promote recurring `Diag` sites to typed `Error` variants

**Goal:** Eliminate `Error::Diag` sites that recur in three or more places with the same shape, per AGENTS.md `Errors` (`Promote a recurring Diag site to its own variant once the call shape stabilises.`).

**Repo:** `specify-cli`. **Depends on:** Phase 2 (which normalises Diag sites surfaced from migration).

**Steps:**

1. Tally `code:` values across every `Error::Diag { code: "<kebab>", … }` site. The current top recurring codes (≥ 3 sites): `workspace-no-registry` (3), `merge-readdir-failed` (4), `merge-dir-entry-failed` (4), `merge-mkdir-failed` (3), `merge-path-prefix-failed` (3), `capability-manifest-missing` (3 across two modules), `workspace-git-spawn-failed` (2 — borderline, defer).
2. For each ≥ 3-site code, add a typed variant (or route to an existing one). Mappings:
   - `workspace-no-registry` → already-existing `Error::RegistryMissing`. Just route to it. (No new variant.)
   - `capability-manifest-missing` → new `Error::CapabilityManifestMissing { dir }`.
   - The `merge-*-failed` cluster → new `Error::Filesystem { op: &'static str, path: PathBuf, source: io::Error }` with `op = "readdir" | "dir-entry" | "mkdir" | "copy" | "path-prefix" | …`. The `code` exposed in JSON is `filesystem-<op>` so existing skill greps that match on the kebab discriminant continue to work.
3. Update `Error::variant_str` in `crates/error/src/lib.rs:253-281`.
4. Update tests that grep for the kebab-case code; the discriminant value is preserved.
5. Run `cargo make standards-tighten`.

**Acceptance:**

- Every `Error::Diag` **construction site** with `code` matching `merge-{readdir,dir-entry,mkdir,path-prefix,copy}-failed` or `capability-manifest-missing` is gone (count construction sites only — pattern-match sites in tests and doc cross-references dominate the global `rg 'Error::Diag \{'` count and are not load-bearing).
- New variants documented in `crates/error/src/lib.rs` with a doc comment naming the canonical call site.
- All existing JSON snapshots unchanged (the `error` discriminant is preserved).

**Out of scope:** the new `Error::Filesystem` variant for I/O operations that carry no useful kebab discriminant should *not* be introduced; those should remain `Error::Io` and propagate via `?`. The variant is justified only where the filesystem operation has a stable name in the JSON envelope.

---

### CL-E2 — Tighten `Error` variant prose; move hints to `emit_error`

**Goal:** Shrink multi-paragraph `#[error("…")]` strings to one-liners; route long-form help through the renderer. Strip embedded GitHub URLs (URLs rot when RFCs reorganise).

**Repo:** `specify-cli`. **Depends on:** Phase 2 complete.

**Steps:**

1. The flagship offender is `Error::InitNeedsCapability` (`crates/error/src/lib.rs:213-220`): 6-line message with a `https://github.com/...` URL. Tighten to:
   ```rust
   #[error("init-requires-capability-or-hub: pass <capability> or --hub")]
   InitNeedsCapability,
   ```
   Add a hint block in `src/output.rs::emit_error`:
   ```rust
   if let Error::InitNeedsCapability = err {
       eprintln!("hint: `specify init <capability>` for a regular project, or `specify init --hub` for a platform hub.");
       eprintln!("see: docs/init.md");
   }
   ```
2. Tighten `ContextUnfenced`, `ContextDrift`, `ContextLockTooNew` — each currently runs to 2–3 lines. One-liners with the kebab discriminant + the immediate cause.
3. Create `specify-cli/docs/init.md` with the long-form explanation. Likewise for any other variant whose tightened message lost durable detail.
4. `rg 'github.com' crates/error/src/lib.rs` — every hit is removed. URLs rot.
5. **Preserve every kebab-case discriminant verbatim.** They are public contract per AGENTS.md `Errors`. Do not bump `JSON_SCHEMA_VERSION`.

**Acceptance:**

- `rg '#\[error\("' crates/error/src/lib.rs | awk '{ print length }' | sort -n | tail -1` ≤ 120 chars.
- `rg 'github.com' crates/error/src/lib.rs` returns zero hits.
- `tests/cli.rs::init_*` cases assert the kebab-case discriminant in the JSON envelope, not the prose body.

---

### CL-E3 — Drop `ok: true` and consolidate exit paths

**Goal:** A single error-emission path. Drop the redundant `ok: true` field from success bodies. Promote any remaining hand-rolled error envelopes uncovered by Phase 2.

**Repo:** `specify-cli`. **Depends on:** Phase 2 + CL-E1.

**Steps:**

1. `rg 'ok: (true|false)' src crates` — every hit is dropped. The success-vs-failure signal is the presence of `error:` in the envelope (failure) or its absence (success). Three files have hits today: `src/commands/registry.rs`, `src/commands/change.rs`, `src/commands/codex.rs`. (Tests already do not grep for `"ok":` — verified.)
2. `rg 'output::ErrorResponse \{' src crates` — every hit outside `src/output.rs` is a hand-rolled error envelope. After CL-11 promoted `change/plan.rs::emit_structural_error` to `Error::PlanStructural`, only one site remains: `src/commands/registry.rs`. Replace it with `Err(Error::*)` → `emit_error`.
3. Update `tests/cli.rs` and `tests/*.rs` JSON snapshots for the removed `ok` field.
4. **Carve-out:** `crates/contract-validate/src/main.rs` is the standalone WASI binary; it does not use `specify-error`. Its `"ok": true` field is part of the legacy contract-validator JSON envelope and stays. Do not touch.

**Acceptance:**

- `rg '"ok": (true|false)' tests` returns zero hits in non-`contract-validate` snapshots.
- `rg 'output::ErrorResponse \{' src crates` returns hits only in `src/output.rs`.

---

### CL-N1 — Name-shortening pass

**Goal:** Apply AGENTS.md `Naming` and `name-suffix-duplication` across the codebase in one focused PR. Phase 2 already hits the per-module renames; this chunk tackles cross-cutting items.

**Repo:** `specify-cli`. **Depends on:** Phase 2.

**Rename table:**

| From | To | Where |
|---|---|---|
| `CommandContext::require` | `CommandContext::load` | `src/context.rs:25` |
| `find_project_root` (free fn) | `ProjectConfig::find_root` (assoc fn) | moved by CL-01 |
| `with_project` / `bare` | `with_project` / `unscoped` | `src/commands.rs:99,115` |
| `JSON_SCHEMA_VERSION` | `JSON_ENVELOPE_VERSION` | `src/output.rs:60` (the const is the *envelope* version, not the JSON-Schema version) |
| `CliResult::Exit(u8)` | `CliResult::Code(u8)` | `src/output.rs:20` (avoids clash with `std::process::ExitCode`) |
| `absolute_string` | `path_string` | `src/output.rs:189` (the function falls back when `canonicalize` fails; the name "absolute" over-promises) |

Plus `cargo make standards-tighten` to settle residual `name-suffix-duplication` baselines (`crates/registry/src/branch.rs`, `crates/registry/src/workspace/sync.rs`, `crates/slice/src/journal.rs`, `crates/merge/src/slice.rs`, etc.).

**Acceptance:**

- `cargo make ci` green.
- `name-suffix-duplication` total decreases by ≥ 50% from HEAD (HEAD is 22; target ≤ 11).
- Every renamed identifier appears in CHANGELOG.md (post-1.0). Pre-1.0, just note.

---

### CL-N2 — Drop wired-but-ignored flags

**Goal:** Apply AGENTS.md `No-op forwarders` + `Wired-but-ignored flags` proactively (the predicate is at 0 today; this is a one-time audit to keep it there).

**Repo:** `specify-cli`. **Depends on:** none.

**Steps:**

1. `rg -n 'Currently' src/cli.rs src/commands` — every clap doc-comment containing the word "Currently" is suspect. (Today: 0 hits — the rule is being honored. This chunk is about codifying the audit cadence, not removing existing offenders.)
2. Add a CI step: `xtask currently-audit` greps for `Currently` in clap doc-comments and fails non-zero. Predicate is regex-only; trivial to implement next to `no-op-forwarders` in `xtask/src/standards.rs`.
3. Add the predicate to AGENTS.md §Mechanical enforcement table.

**Acceptance:**

- New `currently-audit` predicate exists; `cargo make standards-check` includes it.
- Adding "Currently equivalent to the default" to any clap doc fails CI on a synthetic test.

---

## Phase 5 — Skills polish

The framework's skill-discipline checks are mature: 28 predicates run from `scripts/checks.ts`, including `checkBodyLineCount` (≤ 470), `checkCriticalPath` (5–7 items, required when body ≥ 150 lines), `checkInlineJsonBlocks` (≤ 30), `checkDescriptionLength` (≤ 1024), `checkNoRfcCitationsInSkillBody`, `checkOneGuardrailsBlockPerSkill`, `checkNoFrontmatterRestatement`, `checkNoPhaseOutcomeContractRestatement`. Phase 5 is about *tightening the dials these predicates expose*, not adding new ones.

### CL-S01 — Tighten `MAX_DESCRIPTION_CHARS`

**Goal:** Drive skill descriptions toward action-first one-liners. Today's cap is `1024`. Anthropic's skill-discovery surface renders the description verbatim; long descriptions hurt discoverability.

**Repo:** `specify`. **Depends on:** none.

**Steps:**

1. Audit every `plugins/**/SKILL.md` `description` and tighten to `<verb> <object>. Use <when>.` form. Examples already approximated by `omnia-crate-writer`, `omnia-test-writer`, `client-sow-writer`. Counter-example to fix: `spec-define`, which leads with "Define a new Specify slice with all artifacts generated in one step." but does not lead with a strong verb.
2. Lower `MAX_DESCRIPTION_CHARS` in `scripts/checks/skill_frontmatter.ts:141` from `1024` to `512`. (200 was rev 1's target; that's too aggressive given some skills genuinely need to enumerate "Use when …" cases. 512 catches the worst offenders without forcing artificial truncation.)
3. Run `make checks`; fix any new failures by editing the descriptions, not by raising the cap.

**Acceptance:**

- Every `description` ≤ 512 chars.
- `make checks` green.

---

### CL-S02 — Tighten `MAX_BODY_LINES`

**Goal:** Lower the body-line cap from `470` to `400` across all skills. Six skills currently sit between 402 and 465 body lines (`client-sow-writer` 465, `omnia-crate-writer` 455, `vectis-ios-reviewer` 437, `vectis-core-writer` 423, `vectis-android-reviewer` 421, `vectis-core-reviewer` 409, `vectis-android-writer` 402); each has obvious trim opportunities.

**Repo:** `specify`. **Depends on:** none.

**Steps (per skill):**

1. Apply the AGENTS.md `Skill body discipline` rule: "the SKILL.md keeps the Critical Path, the invocation surface, the dispatch table, and the canonical decision points." Move long enumerations of Required References / Examples / verbatim CLI envelope shapes into `references/` siblings; the SKILL.md links once.
2. Apply `checkNoFrontmatterRestatement` results: any prose in the first H2 that re-states `description` or `argument-hint` is removed.
3. Lower `MAX_BODY_LINES` in `scripts/checks/skill_body.ts:18` from `470` to `400` once all seven pass.

**Acceptance:**

- All seven skills ≤ 400 body lines.
- `MAX_BODY_LINES = 400` lands in `scripts/checks/skill_body.ts`.
- `make checks` green.
- The Critical Path of each skill still makes sense when the body is read top-down.

---

### CL-S03 — Sweep migration prose and stray RFC citations

**Goal:** Drive the `checkNoRfcCitationsInSkillBody` baselines down. The check already runs and grandfathers existing hits.

**Repo:** `specify`. **Depends on:** none.

**Steps:**

1. Run the baseline check: every SKILL.md with `RFC[- ]?\d+` outside fenced code or trailing `## References` is a candidate.
2. For each hit: move the citation into a trailing `## References` block — link form similar to the example below — or, for migration prose, into `docs/explanation/decision-log.md`:

   ```markdown
   ## References

   - [RFC-13](../../rfcs/archive/rfc-13-extensibility.md)
   ```
3. Specific known offenders (verified in this review):
   - `plugins/change/skills/plan/SKILL.md:150-153` — RFC-13 §3.11 paragraph.
   - `plugins/spec/skills/define/SKILL.md` — assorted post-cut-over verb-name updates.
4. Tighten the baseline (no per-file allowlist exists for skill-discipline; the check is binary pass/fail and counts via the standards-allowlist mechanism). Re-run `make checks`.

**Acceptance:**

- `make checks` green with no `checkNoRfcCitationsInSkillBody` baseline grandfathering.
- `rg '(Pre|Post)-RFC' plugins/**/SKILL.md` returns zero hits.

---

### CL-S05 — Per-skill description cleanup pass

**Goal:** Once CL-S01's cap is set, do a final per-skill description audit for clarity (not just length). This is the human-judgment chunk; subagents should defer to the skill author when in doubt.

**Repo:** `specify`. **Depends on:** CL-S01.

**Steps:**

1. For each SKILL.md, evaluate against the test: "If a Claude agent sees only this `description`, will it correctly fire / not-fire when the user types a request that should match?" The current 1024-char limit lets descriptions hide behind enumeration; the new 512-char limit forces precision.
2. Patterns to break: descriptions that begin with bare nouns ("A Cursor Canvas is …") rather than verbs; descriptions that fail to name the trigger ("Use when …").
3. Skip skills already at the target shape (`omnia-test-writer`, `omnia-code-reviewer`, `client-sow-writer`).

**Acceptance:**

- Every SKILL.md description begins with a verb.
- Every description names a concrete trigger phrase.

---

### CL-S06 — Skills test review

**Goal:** Confirm that `tests/plan/` traces and `tests/scenarios/` fixtures cover every skill's primary path. This is a coverage audit, not a skill change.

**Repo:** `specify`. **Depends on:** none.

**Steps:**

1. Enumerate every SKILL.md, every plugin in `plugins/`, every command listed in its frontmatter `argument-hint`.
2. Cross-reference against `tests/plan/*.md` and `tests/scenarios/*.json`.
3. Open issues in the parent repo for any skill without at least one trace covering the primary path.

**Acceptance:**

- A coverage matrix exists at `docs/contributing/skills-test-coverage.md`.
- An issue is open for every gap.

---

## Phase 6 — Standards & docs

### CL-X1 — Add new xtask predicates

**Goal:** Mechanise the rules surfaced by this review so regressions can't re-enter.

**Repo:** `specify-cli`. **Depends on:** Phase 4.

**Steps:**

Add to `xtask/src/standards.rs` `Counts` + predicates:

1. `error-envelope-inlined` — regex `output::ErrorResponse\s*\{|output::ValidationErrorResponse\s*\{` outside `src/output.rs`.
2. `path-helper-inlined` — regex `fn\s+(specify_dir|plan_path|change_brief_path|archive_dir)\b` outside `crates/config/`.
3. `ok-literal-in-body` — AST predicate: any `#[derive(Serialize)]` struct with a field named `ok` whose type is `bool`. (CL-E3 drives the count to 0; CL-X1 keeps it there.)
4. `currently-audit` — the predicate from CL-N2.

Update `scripts/standards-allowlist.toml` with baseline 0 for every file (CL-E3 / CL-N2 already drove counts to zero).

Add the four predicates to AGENTS.md §Mechanical enforcement table.

**Acceptance:**

- Four new predicates live in the allowlist with baseline 0 for every file.
- `cargo make standards-check` green.
- A deliberate `git revert` of any CL-E3 / CL-N2 chunk fails CI on the matching predicate.

---

### CL-X2 — Refresh AGENTS.md (specify-cli)

**Goal:** Bring `specify-cli/AGENTS.md` up to current state.

**Repo:** `specify-cli`. **Depends on:** Phase 1, Phase 3 (vectis split).

**Steps:**

1. The `Workspace layout` section lists `crates/vectis-{validate,scaffold}` (now deleted) and shows `specify-error` not as the leaf. Update to reflect:
   ```text
   specify-error                    # leaf
   specify-config                   # depends on specify-error (NEW from CL-01)
   specify-capability               # depends on specify-error
   specify-spec | specify-task      # depend on specify-capability
   specify-slice | specify-merge    # depend on specify-spec
   specify-validate                 # depends on specify-spec
   specify-change                   # depends on specify-slice + specify-spec
   specify-tool                     # WASI tool runner
   specify-init                     # depends on specify-config (NEW from CL-02)
   specify (root crate)             # wires the CLI binary
   crates/contract-validate         # standalone WASI binary
   crates/vectis                    # consolidated WASI tool (CONSOLIDATED)
   ```
2. Append a §"Path helpers live in one crate" subsection (one paragraph; cite CL-01).
3. Append §"Error envelopes are not constructed in handlers" (one paragraph; cite CL-X1's `error-envelope-inlined` predicate).
4. Update §Mechanical enforcement table with the four new predicates from CL-X1.
5. Update the §Gotchas line about `src/lib.rs` ("hosts only the local `config` and `init` modules") to match the post-CL-02 reality (likely `src/lib.rs` no longer exists).

**Acceptance:**

- `cargo make ci` green; AGENTS.md describes HEAD accurately.

---

### CL-X3 — Refresh AGENTS.md (specify)

**Goal:** Codify rules surfaced by Phase 5.

**Repo:** `specify`. **Depends on:** Phase 5.

**Steps:**

1. Add §"Description tightness" with the 512-char target (cite CL-S01).
2. The §Mechanical enforcement table already lists the predicates that apply; refresh the "Predicates surfaced by Skills-1:" list with any predicate added in Phase 5.

**Acceptance:**

- `make checks` green.

---

### CL-X4 — Quarterly migration cadence + initial sweep

**Goal:** Codify a cadence to prevent the standards-allowlist from accumulating new debt.

**Repo:** `specify-cli`. **Depends on:** Phases 1–4 complete.

**Steps:**

1. Add to AGENTS.md `Coding standards`:
   > **Quarterly migration cadence.** A scheduled PR — first business week of each quarter — reviews `scripts/standards-allowlist.toml`, identifies the top five files by total grandfathered violations, and either drives them to zero or documents in this section why they cannot be reduced this quarter. PR title: `chore: q<N> standards-allowlist sweep`.
2. After Phase 4 completes, run `cargo make standards-tighten` once more and commit the resulting allowlist as the post-cleanup baseline.
3. Update `docs/contributing/maintenance.md` (create if absent) with the playbook: `cargo make standards-check`, picking targets, updating baselines, opening a sweep PR.

**Acceptance:**

- Post-cleanup totals show ≥ 80% reduction vs HEAD on `inline-dtos`, `format-match-dispatch`, `name-suffix-duplication`, and `rfc-numbers-in-code`.
- AGENTS.md `Coding standards` carries the cadence.
- `docs/contributing/maintenance.md` exists.

---

## Final verification

After CL-X4 lands, the following invariants hold (and a one-shot subagent can run them as a closure check):

1. `cargo make ci` green on a fresh clone of `specify-cli`.
2. `make checks` green on a fresh clone of `specify`.
3. `cargo make standards-check` totals on `inline-dtos`, `format-match-dispatch`, `name-suffix-duplication` are each ≤ 5 (≥ 90% reduction from HEAD's 39 / 47 / 22). `rfc-numbers-in-code` is ≤ 30 (≥ 75% reduction from HEAD's 116).
4. Every file in `specify-cli/src/`, `specify-cli/src/commands/`, and `specify-cli/crates/*/src/` has `module-line-count` ≤ 500 in the allowlist (or a justification baseline ≤ 600 with a comment).
5. Every body in `specify/plugins/**/SKILL.md` is ≤ 400 lines.
6. `rg 'match\s+(?:ctx\.|self\.)?format\s*\{' specify-cli/src specify-cli/crates --glob '!output.rs' --glob '!**/contract-validate/**'` returns zero hits.
7. `rg 'github.com' specify-cli/crates/error/src/lib.rs` returns zero hits.
8. The four new predicates (`error-envelope-inlined`, `path-helper-inlined`, `ok-literal-in-body`, `currently-audit`) all exist in `xtask/src/standards.rs` and have allowlist baseline 0 for every file.

When all eight hold, the framework matches the standards it has set itself.

## Out of scope for this plan

Items surfaced by review that should track separately:

- **Replace `chrono` with `time` or `jiff`** — defer, separate dependency-hygiene RFC.
- **Add `tracing`** — defer, separate observability proposal.
- **Refactor `crates/validate/src/lib.rs::Rule` from `fn` pointers to a trait** — defensible either way; pick the dragon you want to fight.
- **Test-binary naming and helper consolidation** — cosmetic; let it ride.
- **Tighter version pins on `serde-saphyr`** — re-evaluate once it ships 1.0.
- **`specify check` framework linter** — covered by RM-16 in `roadmap.md`. The xtask predicates added in CL-X1 belong to the *Rust workspace* lint surface, not the framework's `specify check` for plugin authors. Two separate surfaces; do not conflate.

These are tracked separately and are not blockers for the framework being exemplary today.

## Tooling notes for subagents

When a subagent picks up a chunk:

```bash
# specify-cli
cd specify-cli
cargo make ci                    # full pre-PR check
cargo make standards-check       # just the predicates
cargo make standards-tighten     # re-bake baselines after a migration

# specify
cd specify
make checks                      # all 28 predicates
make test                        # cross-repo acceptance
```

Per AGENTS.md, both repos require *the* full check (`cargo make ci` / `make checks`) before commit. Do not rely on narrower substitutes.
