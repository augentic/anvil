# RFC-30 Implementation Plan — Subagent Decomposition

> Companion to [`rfc-30-init.md`](./rfc-30-init.md). This plan re-cuts the RFC's Waves A–F into discrete, subagent-sized **changes**, sequences them so dependencies land first, and marks which changes are safe to run in parallel by separate subagents.

## How to read this plan

- Each **change** is scoped to fit comfortably in one subagent's context: a bounded set of files, one cohesive concern, and its own tests/fixtures.
- Changes are grouped into **phases**. A phase boundary is a hard dependency edge — do not start a phase until its predecessors are merged.
- **Parallel group** means the listed changes have no code dependency on each other and may be assigned to separate subagents simultaneously. Read the *Integration note* for the shared files they nonetheless touch.
- Repo column: `cli` = `augentic/specify-cli`, `spec` = `augentic/specify` (this repo). RFC-30 is a cross-repo change.

## Dependency graph

```text
            ┌──────────────────────── Phase 1 (parallel) ────────────────────────┐
            │                                                                     │
        A (error/exit/config)                                  J (journal events)
            │                                                          │
            ├───────────────────────────┬──────────────────────────┬──┘
            ▼                            ▼                          ▼
   ┌──── Phase 2 (parallel tracks) ─────────────────────────────────────────┐
   │  Track B: B1 (migrate core)     Track C: C (upgrade)   Track D: D (plugins)
   │             │                                                            │
   │             ▼                                                            │
   │           B2 (V1ToV2 + fixtures)                                         │
   │             │                                                            │
   │             ▼                                                            │
   │           B3 (migrate cmd + init --check-migration)                      │
   └──────────────┬─────────────────────────────────────────────────────────┘
                  │            (E depends only on A; may run alongside Phase 2)
                  ▼
        ┌──── Phase 3 ────┐
        │  E (init --upgrade)
        └────────┬─────────┘
                 ▼
   ┌──── Phase 4 (parallel) ────┐
   │  G (init skill expansion)   H (acceptance + idempotency fixtures)
   └──────────────┬──────────────┘
                  ▼
        ┌──── Phase 5 ────┐
        │  F (documentation sweep)
        └──────────────────┘
```

## Shared-file hazards (read before parallelizing)

Three files are edited by multiple otherwise-independent changes. Two are neutralized by ordering; one is a managed merge.

| Shared file | Touched by | Mitigation |
| --- | --- | --- |
| `crates/workflow/src/journal.rs` (`EventKind`) | B1, C, D | **Neutralized:** Change **J** adds all four variants up front in Phase 1; B1/C/D only *reference* them. |
| `src/runtime/cli.rs` (`Commands` enum) | B3, C, D, E | **Managed merge.** Each adds an additive variant/flag. Integrate sequentially (rebase); conflicts are mechanical (append-only enum arms). |
| `src/runtime/commands.rs` (`run()` dispatch + `mod` decls) | B3, C, D, E | **Managed merge.** Each adds one `match` arm + one `pub mod`. Same rebase discipline. |
| Exit-code tables (`AGENTS.md`, `DECISIONS.md`, `handler-shape.md`) | A (table rows), F (DECISIONS long-form prose) | **Partitioned:** A owns the table-row edits; F owns only the DECISIONS "Exit codes" rationale paragraph. No overlap. |

> Practical guidance for the parallel waves: develop B/C/D/E on separate branches, then integrate in the order B → C → D → E (or any fixed order). Because the `Commands` enum and dispatch edits are append-only, a `git rebase` resolves each in seconds. Do **not** have two subagents editing `cli.rs` against the same base commit and expect a clean automatic merge.

---

## Phase 1 — Foundation (2 changes, parallel)

These block the feature tracks. **A** and **J** are independent of each other and may run in parallel.

### Change A — Error variant, exit code 4, and config migration detection
- **Repo:** cli · **Blocks:** B1, B3, E · **Parallel with:** J
- **RFC coverage:** D4, Wave A (all items).
- **Scope:**
  - `crates/error/src/error.rs`: add `Error::ProjectNeedsMigration { from, to }`, its `variant_str()` arm (`"project-needs-migration"`), and an optional `hint()` arm pointing at `specrun migrate`.
  - `src/runtime/output.rs`: add `Exit::MigrationRequired`, return `4` from `Exit::code()`, map `Error::ProjectNeedsMigration` in `Exit::from(&Error)`.
  - `crates/workflow/src/config.rs`: add private `fn major(v: &str) -> Option<u64>` beside `version_is_older`; add the symmetric major-check branch in `ProjectConfig::load` immediately after the `CliTooOld` guard; add `ProjectConfig::load_for_migration` (returns parsed config + `(from, to)` tuple without raising `ProjectNeedsMigration`).
  - Exit-code **table rows** in `AGENTS.md`, `DECISIONS.md`, and `docs/standards/handler-shape.md` (code `4` = `EXIT_MIGRATION_REQUIRED`).
  - Unit tests in `config.rs` analogous to `load_refuses_future_specify_version`: older-major pin → `ProjectNeedsMigration`; unparseable pin stays permissive.
- **Definition of done:** `cargo make check` green; new tests pass; exit-code tables consistent across the three docs.

### Change J — Bootstrap journal events
- **Repo:** cli · **Blocks:** B1, C, D (their journal usage) · **Parallel with:** A
- **RFC coverage:** D7.
- **Scope:** add four variants to the closed `EventKind` enum in `crates/workflow/src/journal.rs`, each with `#[serde(rename = "<wire-id>", rename_all = "kebab-case")]`:
  - `cli.upgraded` → `CliUpgraded { from, to, channel }`
  - `plugins.refreshed` → `PluginsRefreshed { deleted_paths: Vec<String>, marketplace }`
  - `migration.applied` → `MigrationApplied { kind, files_rewritten: usize, files_moved: usize }`
  - `migration.skipped` → `MigrationSkipped { kind, reason }`
- **Rationale for pulling this out:** it removes the only true code-level conflict between the three parallel feature tracks. Landing it first lets B1/C/D each just construct the events.
- **Definition of done:** enum compiles; any existing journal round-trip / serialization tests updated; wire ids verified kebab-case.

---

## Phase 2 — Feature modules (parallel tracks, after Phase 1)

Three independent tracks. **Track B**, **Change C**, and **Change D** may all run in parallel. Within Track B the changes are strictly sequential (B1 → B2 → B3).

### Track B — Migration framework

#### Change B1 — Migration framework core
- **Repo:** cli · **Depends on:** A, J · **Blocks:** B2, B3
- **RFC coverage:** D3, Wave B item 1; journal wiring (consumes J's `migration.*`).
- **Scope:** add `crates/workflow/src/migrate.rs` with:
  - `MigrationKind` (`#[non_exhaustive]`, `V1ToV2`) and `MigrationKind::resolve(from, to)` returning the ordered slice of migrations (composes across majors; returns empty when none needed).
  - `Migrator` trait (`id`, `plan`, `apply`), `MigrationPlan` (file moves / rewrites / structured edits), `MigrationReport` (same shape + checksums + top-level `status`).
  - Atomicity scaffolding: staging under `.specify/.migrate/<kind>/staging/` with rename-into-place; partial failure emits `migration.skipped`.
- **Definition of done:** types compile; `resolve` unit-tested (empty path, single hop, multi-hop composition); no concrete migrator logic yet.

#### Change B2 — `V1ToV2` migrator + golden fixtures
- **Repo:** cli · **Depends on:** B1 · **Blocks:** B3 (e2e test)
- **RFC coverage:** D3 "Concrete migrators", Wave B item 3.
- **Scope:** implement the `V1ToV2` `Migrator` covering the five 1.x→2.0 transforms:
  - legacy `pipeline:` key → axis-split `briefs:` keys;
  - monolithic `adapter.yaml` → `adapters/{sources,targets}/<name>/adapter.yaml`;
  - retired `change:` slash-namespace references in operator notes;
  - `discovery.md` legacy candidate format → `## Candidate inventory` block with stable `id`;
  - strip `slices[].target` from `plan.yaml` and ensure each slice carries a resolvable `project`.
  - Golden fixtures under `tests/migrate/v1-to-v2/{before,after}/`.
- **Definition of done:** golden test passes (honor `REGENERATE_GOLDENS`); `plan` then `apply` produces byte-identical `after/` tree; failure leaves `before/` untouched.

#### Change B3 — `specrun migrate` command + `init --check-migration` probe
- **Repo:** cli · **Depends on:** B1, A (e2e needs B2) · **Touches shared:** `cli.rs`, `commands.rs`
- **RFC coverage:** D3 command surface, Wave B items 2, 5.
- **Scope:**
  - `src/runtime/commands/migrate.rs` with `--from`, `--to`, `--dry-run`, `--yes`, `--format`; `--from` defaults to `project.yaml.specify_version`, `--to` to `CARGO_PKG_VERSION`; uses `load_for_migration` (bootstrap carve-out).
  - `Commands::Migrate` clap variant + dispatch arm + `pub mod migrate`.
  - `specrun init --check-migration --format json` read-only probe (`needs-migration` bool + plan preview) on the init command. Bootstrap carve-out (no standard `ProjectConfig::load`).
- **Definition of done:** `--dry-run` prints plan + would-fire journal events without writing; `--yes` applies and journals `migration.applied`; `--check-migration` JSON shape stable; integration test over the B2 fixture green.

### Change C — CLI upgrade verb
- **Repo:** cli · **Depends on:** J · **Parallel with:** Track B, D · **Touches shared:** `cli.rs`, `commands.rs`
- **RFC coverage:** D1, Wave C (all items).
- **Scope:**
  - `crates/workflow/src/upgrade.rs`: `InstallChannel` enum (`cargo`/`brew`/`binary`/`unknown`) + `InstallChannel::detect()`; per-channel upgrade strategy (cargo `--git --tag` pinned to latest with HEAD fallback; `brew upgrade`; binary download+checksum+atomic replace).
  - Latest-version probe shared with init: `gh release view --json tagName` first, unauthenticated `api.github.com/.../releases/latest` fallback; probe failure is a warning + journal note, not an error.
  - `src/runtime/commands/upgrade.rs` with `--channel`, `--yes`, `--dry-run`, `--format`; `unknown` channel exits with structured `unknown-install-channel` diagnostic.
  - `Commands::Upgrade` clap variant + dispatch arm + `pub mod upgrade`; journal `cli.upgraded` (new binary writes it; `from` = pre-upgrade version).
- **Definition of done:** `--dry-run` reports detected channel + target version without mutating; detection unit-tested per channel; `cargo make check` green.

### Change D — Plugin cache verbs
- **Repo:** cli · **Depends on:** J · **Parallel with:** Track B, C · **Touches shared:** `cli.rs`, `commands.rs`
- **RFC coverage:** D2, Wave D (all items).
- **Scope:**
  - `crates/workflow/src/plugins.rs`: marketplace discovery (`--marketplace` → `$project_dir/.cursor-plugin/marketplace.json` → `$XDG_CONFIG_HOME/cursor/marketplace.json`, first hit wins); cross-platform `$CURSOR_HOME` detection (default `~/.cursor`, overridable); cache scan under `$CURSOR_HOME/plugins/cache/<name>/<plugin>/<sha>/`; expected-sha resolution from the marketplace's backing git checkout (`git -C <repo> rev-parse HEAD` for relative-path sources; `git ls-remote` branch specified but inert; `null` → `present` degradation).
  - `src/runtime/commands/plugins.rs`: `doctor` (read-only JSON report, `ok`/`drifted`/`present`/`missing`/`extra`, exits non-zero only on FS/parse failure) and `refresh` (confirmed delete of cache scope + journal `plugins.refreshed` + restart instruction, exit 0).
  - `Commands::Plugins` clap variant + dispatch arm + `pub mod plugins`.
- **Validate-before-asserting (RFC "Assumption to validate in Wave D"):** confirm the cache-leaf `<sha>` derivation against a real Cursor install before emitting `drifted`; if irreproducible, ship the `expected-sha: null` → `present` safety net only.
- **Definition of done:** `doctor` JSON matches the RFC schema; `refresh` deletes only the scoped cache dir and never touches IDE state; tests cover missing/extra/degraded paths.

---

## Phase 3 — Init re-entry (1 change)

### Change E — `specrun init --upgrade` flag + preservation
- **Repo:** cli · **Depends on:** A · **Touches shared:** `cli.rs`, `commands.rs`
- **RFC coverage:** D5, Wave E items 1, 2, 7.
- **Notes on sequencing:** depends only on **A**, so it *can* start alongside Phase 2, but it edits the `Init` clap variant and dispatch arm — coordinate its `cli.rs`/`commands.rs` merge with B3/C/D (see hazard table). Listed in its own phase to make that merge ordering explicit.
- **Scope:**
  - `--upgrade` flag on `src/runtime/commands/init.rs` and a scalar `bool` field on `InitOptions` in `crates/workflow/src/init.rs` (threaded by value into `regular::run` / `hub::run`).
  - Mutually exclusive with `<adapter>` positional and `--hub`; refuses when `ProjectNeedsMigration` would fire (operator must `specrun migrate` first).
  - Preservation invariant: only mutates `project.yaml` (`specify_version` → `CARGO_PKG_VERSION`, all other fields preserved incl. `adapter:`/`hub:`); regenerates `AGENTS.md` only when absent; never touches `slices/`, `specs/`, `archive/`, `registry.yaml`, design-system files, or the adapter cache (latter only under `--refresh-cache`).
  - Idempotency: second run with matching `specify_version` + present `AGENTS.md` writes nothing, exits 0.
  - Re-entry idempotency fixtures (brownfield + hub): first run changes only `specify_version`, operator artifacts byte-stable; second run no-op (Wave E item 7).
- **Definition of done:** clap exclusivity enforced; preservation + idempotency fixtures green; `--upgrade` over a `needs-migration` project returns exit `4`.

---

## Phase 4 — Skill expansion & acceptance (2 changes, parallel)

Both depend on the command wire surfaces being locked (they are, per RFC "Open Questions" resolutions) and on the verbs existing so end-to-end runbook/fixtures reference real commands.

### Change G — Init skill expansion
- **Repo:** spec · **Depends on:** B3, C, D, E (locked surfaces) · **Parallel with:** H
- **RFC coverage:** D6, Wave E items 3, 4, 5.
- **Scope:**
  - `plugins/spec/skills/init/SKILL.md`: rewrite scope statement; add Critical-Path steps `1b` (probe CLI version via `specrun upgrade --dry-run --format json`), `1c` (probe plugin cache via `specrun plugins doctor --format json`), `1d` (probe artifact major via `specrun init --check-migration --format json`) before existing step 2; add the three Guardrail carve-outs (CLI upgrade / plugin refresh / major-version migration).
  - `plugins/spec/skills/init/references/init-runbook.md`: add steps 1b/1c/1d with their AskQuestion → action → continue/stop flows; route existing step 2's reinit branch through `specrun init --upgrade`.
  - `plugins/spec/skills/init/references/init-output-templates.md`: add a `migrated` template alongside `greenfield`/`brownfield`/`hub` rendering the `MigrationReport` summary + journal pointer.
- **Definition of done:** `make lint` (`specdev lint`) green; skill caps respected; runbook commands match the shipped CLI flags exactly.

### Change H — Acceptance & init-shape fixtures
- **Repo:** cli (+ spec acceptance docs if applicable) · **Depends on:** B3, E · **Parallel with:** G
- **RFC coverage:** Wave E item 6.
- **Scope:** update acceptance fixtures to cover the four init shapes — `greenfield`, `brownfield`, `hub`, `migrated`. (The re-entry idempotency fixtures themselves live in Change E; this change is the broader acceptance matrix that exercises the migrated path end-to-end.)
- **Definition of done:** all four shapes pass in the acceptance suite; the `migrated` shape exercises `specrun migrate` → `specrun init --upgrade`.

---

## Phase 5 — Documentation (1 change, last)

### Change F — Documentation sweep
- **Repo:** cli (+ spec for cross-repo AGENTS) · **Depends on:** all feature changes
- **RFC coverage:** Wave F (all items).
- **Scope:**
  - `README.md` "Installing the CLI" → mention `specrun upgrade`.
  - `docs/orientation/prerequisites.md` → describe channel detection.
  - `docs/reference/quick-reference.md` → add `upgrade`, `plugins doctor/refresh`, `migrate`, `init --upgrade`.
  - `AGENTS.md` "Gotchas" → replace the "2.0 is a hard cut" sentence with "Major version bumps require a registered `MigrationKind`."
  - `DECISIONS.md` "Exit codes" → long-form rationale for code `4` (prose only; the table row already landed in Change A).
- **Definition of done:** docs consistent with shipped surfaces; no dangling references to removed/renamed items (`rg` audit per the cli repo's symbol-removal rule).

---

## Subagent assignment summary

| Wave | Changes (parallel within a wave) | Gate before starting |
| --- | --- | --- |
| 1 | **A**, **J** | none |
| 2 | **B1**, **C**, **D** | A + J merged |
| 2′ | **B2** | B1 merged |
| 2″ | **B3** | B1 + B2 (+ A) merged |
| 3 | **E** | A merged (coordinate `cli.rs` merge with B3/C/D) |
| 4 | **G**, **H** | B3, C, D, E merged |
| 5 | **F** | all feature changes merged |

**Maximum useful parallelism:** Wave 2 fans out to 3 subagents (B1, C, D), with E joinable as a 4th if `cli.rs`/`commands.rs` integration is serialized. Wave 4 fans out to 2 (G, H).

## Cross-cutting reminders for every subagent

- Run `cargo make check` (cli repo) before declaring a change done; `cargo make ci` before the final integration. For spec-repo changes run `make lint`.
- The CLI is the single writer; never hand-edit `.specify/` artifacts in fixtures except through the migrator under test.
- Honor the bootstrap carve-out: `migrate`, `upgrade`, `plugins {doctor,refresh}`, and `init --upgrade` must use `load_for_migration`, never the standard `ProjectConfig::load`.
- Wire ids and payload fields stay kebab-case on the wire (`#[serde(rename_all = "kebab-case")]`); Rust variants stay `snake_case`/`PascalCase`.
- Per the cli repo's symbol-removal rule, any touched symbol that appears in `AGENTS.md`/`DECISIONS.md`/`docs/` (or the parent `augentic/specify` repo) must be updated in the same change.
