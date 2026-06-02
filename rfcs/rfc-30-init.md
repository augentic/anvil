# RFC-30: Init Bootstrap, Update, and Migration Lifecycle

> Status: Draft - Depends: [RFC-13](../done/rfc-13-extensibility.md), [RFC-25](../done/rfc-25-workflow.md) - Enables: [roadmap RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model)

## Abstract

`/spec:init` is the only Specify skill permitted to bootstrap the CLI on first contact. This RFC widens that carve-out into a coordinated bootstrap lifecycle that owns three concerns:

1. **Plugin cache drift.** The skills loaded from `~/.cursor/plugins/cache/augentic/<plugin>/<sha>/` can fall behind the marketplace, with no first-class detection or refresh path.
2. **Stale CLI binary.** A binary that is present but behind the latest release, across every install channel (`cargo`, Homebrew, pre-built binary).
3. **Project on an old major version.** Artifacts whose pinned `specify_version` major is older than the running binary.

The RFC delivers:

1. **`specrun upgrade`** — a channel-aware CLI self-update verb.
2. **`specrun plugins {doctor,refresh}`** — Cursor plugin cache inspection and invalidation.
3. **`specrun migrate`** — a registered, fixture-backed migrator family keyed off `project.yaml.specify_version`.
4. **`Error::ProjectNeedsMigration`** and a new exit code `4`.
5. **An expanded `/spec:init` runbook** that probes CLI version, plugin cache, and artifact major version before invoking `specrun init`, delegating each concern to its owning CLI verb.

The skill stays the orchestrator. The CLI stays the single writer.

## Motivation

The three concerns share substrate: all are bootstrap problems, all need single-writer CLI discipline, and all are naturally surfaced through one operator entry point. Treating them in separate RFCs would create three skill carve-outs, three journal-event sets, and three patterns for "ask, then act, then maybe restart."

The migration concern also changes a standing policy. Instead of treating every major-version bump as a flag day, **each major bump must register a migrator before the `specify_version` field rolls**. Migration becomes a covered, routine step rather than a breaking event — keeping the door open to schema improvements without shipping breakage.

## Principles

1. **CLI owns deterministic actions.** Version comparison, cache invalidation, channel detection, and schema migration are CLI verbs. Skills orchestrate intent and consent.
2. **`/spec:init` remains the only bootstrap entry point.** No new operator-facing slash commands for routine cases. `/spec:doctor` is reserved as a future read-only diagnostic, not the upgrade path.
3. **Drift is observable.** Every drift signal — stale CLI, stale plugin cache, project-on-old-major — has a structured detector that can be called outside `/spec:init`.
4. **Consent is explicit.** No upgrade, refresh, or migration runs without an `AskQuestion` confirmation. Headless invocations require an explicit `--yes` flag.
5. **No silent in-place restarts.** When a Cursor restart is required, the skill says so and stops. The CLI never relaunches the IDE.
6. **Migration is fixture-backed.** Every registered migrator has golden inputs and outputs. No migrator without coverage; no major-version bump without a registered migrator.
7. **Bootstrap concerns are independent.** Plugin refresh, CLI upgrade, and artifact migration may be invoked in any order, succeed or fail independently, and never depend on each other for correctness.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 CLI upgrade verb** | The CLI exposes `specrun upgrade` that detects its install channel and self-updates. | Add `src/runtime/commands/upgrade.rs`; add `InstallChannel` enum (`cargo`, `brew`, `binary`, `unknown`); shell out to the channel-native upgrade command after confirmation. |
| **D2 Plugin cache verbs** | The CLI exposes `specrun plugins doctor` (read-only drift report) and `specrun plugins refresh` (cache invalidation). | Add `src/runtime/commands/plugins.rs`; scope the cache by the marketplace's top-level `name` (`$CURSOR_HOME/plugins/cache/<name>/`); resolve the expected sha from the marketplace's backing git checkout and compare against the cached leaf sha. |
| **D3 Migration framework** | The CLI exposes `specrun migrate` with a closed registry of per-major migrators. | Add `crates/workflow/src/migrate.rs`; add `MigrationKind` closed enum; each variant registers a golden fixture under `tests/migrate/`. |
| **D4 ProjectNeedsMigration error** | `ProjectConfig::load` rejects a project whose `specify_version` major is older than the running binary. | Add `Error::ProjectNeedsMigration { from, to }` (plus its `variant_str()` and optional `hint()` arms); add an `Exit::MigrationRequired` variant returning `4` from `Exit::code()` and wire it through `Exit::from(&Error)`; update DECISIONS.md exit-code table. |
| **D5 Init re-entry semantics** | `specrun init --upgrade` rewrites `specify_version` and re-scaffolds preservation-safe files only. | Add `--upgrade` flag to `init` clap surface; `crates/workflow/src/init/regular.rs` and `init/hub.rs` route through the same preservation rules as the first-run case. |
| **D6 Init skill expansion** | `/spec:init` runbook adds three probe steps (CLI version, plugin cache, artifact major) before existing step 2. | Update `plugins/spec/skills/init/SKILL.md` Critical Path and Guardrails; add `references/init-runbook.md` sections 1b, 1c, and 2a. |
| **D7 Bootstrap journal events** | Every CLI-owned bootstrap action emits a journal event with kebab-case discriminant. | Add `cli-upgraded`, `plugins-refreshed`, `migration-applied`, and `migration-skipped` variants to the closed `EventKind` enum in `crates/workflow/src/journal.rs`. |

## Operator surface

Routine first-run with no drift:

```bash
/spec:init https://github.com/augentic/specify/adapters/targets/omnia
```

Re-entry — running `/spec:init` against an existing project — runs three probes before the existing steps, each of which can branch into a confirmed action:

```text
[probe] specrun --version              → suggests `specrun upgrade` if stale
[probe] specrun plugins doctor          → suggests `specrun plugins refresh` if drifted
[probe] specrun init --check-migration  → suggests `specrun migrate` if needed
```

The lower-level breakouts may also be invoked directly by power users or CI:

```bash
specrun upgrade [--channel cargo|brew|binary] [--yes]
specrun plugins doctor [--format json]
specrun plugins refresh [--yes]
specrun migrate [--from <X.Y>] [--to <X.Y>] [--dry-run] [--yes]
```

Init gains one flag for bumping `specify_version` without re-scaffolding:

```bash
specrun init --upgrade
```

`--upgrade` is mutually exclusive with the `<adapter>` positional and `--hub`. It refuses to run when `Error::ProjectNeedsMigration` would fire — the operator must `specrun migrate` first.

## CLI upgrade verb (D1)

### Command

```bash
specrun upgrade [--channel cargo|brew|binary|auto] [--yes] [--format json]
```

### Channel detection

`InstallChannel::detect()` resolves the running binary's path and classifies it:

| Channel | Detection |
| --- | --- |
| `cargo` | Path matches `$CARGO_HOME/bin/specify` (or `~/.cargo/bin/specify` when `CARGO_HOME` is unset). |
| `brew` | Path resolves to a Homebrew Cellar, or `brew --prefix specify` matches the binary's parent. |
| `binary` | Path is under `/usr/local/bin`, `/opt/specify/`, or another known install location; or the file is symlinked from a tagged release artifact. |
| `unknown` | None of the above. `specrun upgrade` exits with a structured `unknown-install-channel` diagnostic instructing manual upgrade. |

`--channel` overrides detection.

### Upgrade actions

| Channel | Action |
| --- | --- |
| `cargo` | `cargo install --git https://github.com/augentic/specify-cli` (tag pinned to the resolved latest release when GitHub is reachable; HEAD otherwise with a warning). |
| `brew` | `brew upgrade augentic/tap/specrun`. |
| `binary` | Download the latest release archive for the current platform, verify the checksum sidecar, and replace the binary atomically. |

### Latest-version probe

Both `specrun upgrade` (mandatory) and `/spec:init` (optional probe) call the same resolver: `gh release view --json tagName -R augentic/specify-cli` when `gh` is on PATH, otherwise an unauthenticated `https://api.github.com/repos/augentic/specify-cli/releases/latest` request. Probe failures are warnings, not errors — the upgrade proceeds against HEAD with a journal note when the latest tag cannot be resolved.

### Journal

```text
cli-upgraded    { from: "0.42.1", to: "0.43.0", channel: "brew" }
```

`from` is the version observed before the upgrade. The new binary writes the event.

## Plugin cache verbs (D2)

### Layout

Cursor's plugin cache lives under `$CURSOR_HOME/plugins/cache/<name>/<plugin>/<sha>/`, where `<name>` is the marketplace's top-level `name` field (e.g. `augentic`). `$CURSOR_HOME` defaults to `~/.cursor` and is overridable. Each plugin directory carries the marketplace-resolved git sha as its leaf segment. `marketplace.json` declares the top-level `name`, the expected `plugins[]` (each with a `name` and a `source` relative to `pluginRoot`), and the `pluginRoot` they live under. The marketplace file declares no per-plugin shas — Cursor's marketplace machinery resolves those from each plugin's git `source`.

### Expected-sha resolution

Because `marketplace.json` carries no shas, `doctor` resolves the *expected* sha from the marketplace's backing git checkout:

- When the resolved marketplace path lives inside a git worktree (the common case — the augentic marketplace ships in this repo), the expected sha is `git -C <marketplace-repo> rev-parse HEAD`. Every same-repo plugin shares that HEAD; out-of-repo `source` URLs resolve via `git ls-remote <source>` against the declared ref.
- When the marketplace is not inside a git checkout and no `source` ref resolves, `expected-sha` is reported as `null` and the plugin's `status` collapses to `present` / `missing` only. This degradation is a finding, not an error.

`doctor` never invents an expected sha and never claims drift it cannot prove.

### `specrun plugins doctor`

Read-only diagnostic. For each plugin declared in `.cursor-plugin/marketplace.json` (or the user-configured marketplace), report:

```json
{
  "version": 1,
  "marketplace": "/.../specify/.cursor-plugin/marketplace.json",
  "cache-root": "/Users/me/.cursor/plugins/cache/augentic",
  "plugins": [
    {
      "name": "spec",
      "expected-sha": "f1b21b2193ecb722860762e52d1dd68a0244c865",
      "cached-sha": "a0c4...",
      "status": "drifted"
    }
  ],
  "summary": {
    "ok": 2,
    "drifted": 1,
    "missing": 0,
    "extra": 0
  }
}
```

`status` values: `ok` (cached sha matches expected), `drifted` (cached sha differs from expected), `present` (cache entry exists but expected sha is unresolvable, so drift cannot be asserted — `expected-sha` is `null`), `missing` (no cache entry), `extra` (cache entry not declared by marketplace).

`doctor` never exits non-zero on drift — drift is a finding. It exits non-zero only on filesystem or marketplace parse failures.

### `specrun plugins refresh`

Confirmed cache invalidation. After `--yes` or interactive confirmation:

1. Delete `$CURSOR_HOME/plugins/cache/<name>/` for the marketplace's top-level `name`.
2. Emit `plugins-refreshed { deleted-paths: [...] }`.
3. Print: `Plugin cache cleared. Restart Cursor to repopulate from the marketplace.`
4. Exit `0`.

The CLI does not restart Cursor and does not touch open IDE state. Hot-reload is a Cursor concern.

### Marketplace discovery

The CLI looks for `.cursor-plugin/marketplace.json` in this order: `--marketplace <path>` flag, `$project_dir/.cursor-plugin/marketplace.json`, then `$XDG_CONFIG_HOME/cursor/marketplace.json`. The first hit wins. Cache scope follows the marketplace's top-level `name` so multiple marketplaces coexist.

## Migration framework (D3)

### Command

```bash
specrun migrate [--from <X.Y>] [--to <X.Y>] [--dry-run] [--yes] [--format json]
```

`--from` defaults to `project.yaml.specify_version`; `--to` defaults to `CARGO_PKG_VERSION`. `--dry-run` prints the migration plan and the journal events that would fire, without writing.

### `MigrationKind`

```rust
/// Closed registry of per-major migration paths. Adding a major version
/// requires a new variant *and* a registered `Migrator` impl *and* a
/// golden fixture under `tests/migrate/<from>-to-<to>/`.
#[non_exhaustive]
pub enum MigrationKind {
    V1ToV2,
    // V2ToV3 lands when 3.0 is cut.
}
```

`MigrationKind::resolve(from, to)` returns the ordered slice of migrations needed to walk from `from` to `to`. Skipping majors composes individual migrators.

### `Migrator` trait

```rust
pub trait Migrator {
    /// Stable kebab-case id, e.g. `v1-to-v2`.
    fn id(&self) -> &'static str;

    /// Inspect the project and return the list of file actions
    /// without applying them. Used by `--dry-run` and by
    /// `specrun init` to render the "would migrate" preview.
    fn plan(&self, project_dir: &Path) -> Result<MigrationPlan>;

    /// Apply the plan atomically (staged write + rename).
    /// Emits per-file journal events for audit.
    fn apply(&self, project_dir: &Path, plan: &MigrationPlan) -> Result<MigrationReport>;
}
```

`MigrationPlan` enumerates file moves, file rewrites, and structured edits. `MigrationReport` is the same shape post-apply, with checksums and a top-level `status`.

### Concrete migrators

`V1ToV2` is the first concrete migrator, covering the 1.x → 2.0 breaking changes:

- legacy `pipeline:` manifest key → axis-split `briefs:` keys;
- monolithic `adapter.yaml` → `adapters/sources/<name>/adapter.yaml` + `adapters/targets/<name>/adapter.yaml`;
- retired `change:` slash-namespace references in operator notes;
- `discovery.md` legacy candidate format → `## Candidate inventory` block with stable `id`;
- `plan.yaml` slices bind a `project` only; the per-slice `target` field is dropped (the target adapter resolves on demand from the bound project). The migrator strips any persisted `slices[].target` and ensures each slice carries a resolvable `project`.

### Atomicity

Migrations write to `.specify/.migrate/<kind>/staging/` and rename into place once every staged change validates. Partial failures leave the project untouched and emit `migration-skipped { reason: "<diagnostic>" }`.

### Journal

```text
migration-applied { kind: "v1-to-v2", files-rewritten: 12, files-moved: 5 }
migration-skipped { kind: "v1-to-v2", reason: "staged-validation-failed" }
```

## ProjectNeedsMigration error and exit code (D4)

### Error variant

```rust
/// The project's pinned `specify_version` has a smaller major than the
/// running binary; a migration must run before the CLI can operate.
#[error("project pinned to specify {from} but running {to}; run `specrun migrate`")]
ProjectNeedsMigration {
    from: String,
    to: String,
},
```

### Detection

Add a symmetric major check in `ProjectConfig::load` (`crates/workflow/src/config.rs`), immediately after the `CliTooOld` guard:

```rust
let current = env!("CARGO_PKG_VERSION");
if let Some(required) = &cfg.specify_version
    && version_is_older(current, required)
{
    return Err(Error::CliTooOld {
        required: required.clone(),
        found: current.to_string(),
    });
}
if let Some(pinned) = &cfg.specify_version
    && let Some(from) = major(pinned)
    && let Some(to) = major(current)
    && to > from
{
    return Err(Error::ProjectNeedsMigration {
        from: pinned.clone(),
        to: current.to_string(),
    });
}
```

Add a private `fn major(v: &str) -> Option<u64>` helper beside `version_is_older` (parsing via the `semver` crate). Unparseable pinned or current versions yield `None` and stay permissive, consistent with the `CliTooOld` arm.

### Exit code

| Code | Name | When |
| --- | --- | --- |
| 0 | `EXIT_SUCCESS` | unchanged |
| 1 | `EXIT_GENERIC_FAILURE` | unchanged |
| 2 | `EXIT_VALIDATION_FAILED` | unchanged |
| 3 | `EXIT_VERSION_TOO_OLD` | unchanged |
| 4 | `EXIT_MIGRATION_REQUIRED` | `Error::ProjectNeedsMigration` |

`EXIT_MIGRATION_REQUIRED` is the doc-table name for the `Exit::MigrationRequired` variant. The `Exit` enum in `src/runtime/output.rs` uses no explicit discriminants, so add the variant, return `4` from `Exit::code()`, and route `Error::ProjectNeedsMigration` to it in `Exit::from(&Error)`. Update the AGENTS.md exit-code table, `docs/standards/handler-shape.md`, and the DECISIONS.md "Exit codes" section.

### Bootstrap-command carve-out

`specrun migrate`, `specrun upgrade`, `specrun plugins {doctor,refresh}`, and `specrun init --upgrade` MUST NOT call `ProjectConfig::load` through the standard load path — they operate on projects explicitly in the "needs migration" state. Each uses a `ProjectConfig::load_for_migration` variant that returns the parsed config and the `(from, to)` migration tuple without raising `ProjectNeedsMigration`.

## Init re-entry semantics (D5)

### `specrun init --upgrade`

Add an `--upgrade` flag to `src/runtime/commands/init.rs` and a matching field on the `InitOptions` struct in `crates/workflow/src/init.rs`. `InitOptions` is `Copy` and borrow-shaped, so the new field is a scalar (`bool`) threaded through `init` → `regular::run` / `hub::run` by value. Behavior:

- Mutually exclusive with the `<adapter>` positional and `--hub`.
- Refuses to run if `Error::ProjectNeedsMigration` would fire (the operator must `specrun migrate` first).
- Preserves the existing `adapter:` field in `project.yaml`.
- Rewrites `specify_version` to `CARGO_PKG_VERSION`.
- Re-runs the `agents::generate_for_init` path (`src/runtime/commands/agents.rs`), but only when `AGENTS.md` is absent (the first-run preservation rule).
- Does not re-fetch the adapter cache unless `--refresh-cache` is also passed.

### Preservation invariant

`specrun init --upgrade` runs over a populated `.specify/`, so its write set is enumerated and closed. The only file it mutates is `project.yaml` (rewriting `specify_version`, preserving every other field including `adapter:` / `hub:`). It regenerates `AGENTS.md` only when absent. It MUST NOT touch any operator-authored artifact — `slices/`, `specs/`, `archive/`, `registry.yaml`, `.specify/design-system/components.yaml`, `tokens.yaml`, `assets.yaml`, or the adapter cache (the last only refetched under explicit `--refresh-cache`). Re-scaffolding is confined to files the first-run case would create and that are currently absent; an existing file is never overwritten.

### Idempotency

`specrun init --upgrade` is idempotent: once `project.yaml.specify_version` already equals `CARGO_PKG_VERSION` and `AGENTS.md` is present, a re-run is a no-op that writes nothing and exits `0`. Repeated invocations never accumulate edits.

The skill invokes `specrun init --upgrade` after confirmation; this is the real flag behind the runbook's reinit branch.

## Init skill expansion (D6)

### `SKILL.md` changes

`plugins/spec/skills/init/SKILL.md`:

1. Scope statement: replace "not for re-initializing an existing `.specify/`" with "supports first-run init, re-entry upgrades, plugin-cache refresh, and major-version migration handoff."
2. Critical Path gains three ordered steps before existing step 2:
   - `1b. Probe CLI version` — call `specrun upgrade --dry-run --format json` and report drift.
   - `1c. Probe plugin cache` — call `specrun plugins doctor --format json` and report drift.
   - `1d. Probe artifact major` — call `specrun init --check-migration --format json` and report drift.
3. Guardrails gain the parallel carve-outs: "`/spec:init` is the one Specify skill that may upgrade the CLI", "…refresh the Cursor plugin cache", and "…trigger a major-version migration."

### `init-runbook.md` changes

`plugins/spec/skills/init/references/init-runbook.md`:

- New step **1b. Probe CLI version**: parse `specrun upgrade --dry-run --format json`; on drift, AskQuestion → `specrun upgrade --yes` → print restart-not-required confirmation and continue.
- New step **1c. Probe plugin cache**: parse `specrun plugins doctor --format json`; on drift, AskQuestion → `specrun plugins refresh --yes` → print "Restart Cursor and re-run `/spec:init`" → **stop**.
- New step **1d. Probe artifact major**: parse `specrun init --check-migration --format json`; on `needs-migration: true`, AskQuestion → `specrun migrate --yes` → continue to existing step 2.
- Existing step 2's reinit branch routes through `specrun init --upgrade`.

### Output templates

Add a `migrated` template alongside `greenfield`, `brownfield`, and `hub` in `plugins/spec/skills/init/references/init-output-templates.md`. It renders the structured `MigrationReport` summary and points at the journal entry for full audit.

## Bootstrap journal events (D7)

Add four kebab-case variants to the closed `EventKind` enum in `crates/workflow/src/journal.rs`:

| Wire id | Rust variant | Payload |
| --- | --- | --- |
| `cli-upgraded` | `CliUpgraded` | `{ from: String, to: String, channel: String }` |
| `plugins-refreshed` | `PluginsRefreshed` | `{ deleted-paths: Vec<String>, marketplace: String }` |
| `migration-applied` | `MigrationApplied` | `{ kind: String, files-rewritten: usize, files-moved: usize }` |
| `migration-skipped` | `MigrationSkipped` | `{ kind: String, reason: String }` |

Per the [RFC-19 journal contract](https://github.com/augentic/specify-cli/blob/main/AGENTS.md), wire ids stay kebab-case and Rust variants stay `snake_case` joined by `#[serde(rename)]`.

## Implementation Plan

### Wave A — Error and exit code

1. Add `Error::ProjectNeedsMigration { from, to }` to `crates/error/src/error.rs`, plus its `variant_str()` arm (`"project-needs-migration"`) and an optional `hint()` arm pointing at `specrun migrate`.
2. Add the `Exit::MigrationRequired` variant in `src/runtime/output.rs`, return `4` from `Exit::code()`, and map `Error::ProjectNeedsMigration` to it in `Exit::from(&Error)`.
3. Update AGENTS.md, DECISIONS.md, and `docs/standards/handler-shape.md` exit-code tables in the same PR.
4. Add the `major()` helper and detection branch in `ProjectConfig::load`, plus the `load_for_migration` variant the bootstrap commands use.
5. Add unit tests in `crates/workflow/src/config.rs` analogous to `load_refuses_future_specify_version` (older-major pin returns `ProjectNeedsMigration`; unparseable pin stays permissive).

### Wave B — Migration framework

1. Add `crates/workflow/src/migrate.rs` with `MigrationKind`, `Migrator`, `MigrationPlan`, `MigrationReport`.
2. Add `src/runtime/commands/migrate.rs` with `--from`, `--to`, `--dry-run`, `--yes`, and `--format` flags.
3. Implement `V1ToV2`; check in golden fixtures under `tests/migrate/v1-to-v2/{before,after}/`.
4. Add `migration-applied` and `migration-skipped` journal events.
5. Add `specrun init --check-migration` (read-only probe used by the skill).

### Wave C — CLI upgrade verb

1. Add `crates/workflow/src/upgrade.rs` with `InstallChannel::detect` and per-channel upgrade strategy.
2. Add `src/runtime/commands/upgrade.rs` and `--channel`, `--yes`, `--format`, `--dry-run` flags.
3. Add the latest-version probe (`gh release view` first, `api.github.com` fallback).
4. Add `cli-upgraded` journal event.

### Wave D — Plugin cache verbs

1. Add `crates/workflow/src/plugins.rs` with marketplace discovery and cache scanning.
2. Add `src/runtime/commands/plugins.rs` with `doctor` and `refresh` subcommands.
3. Add `plugins-refreshed` journal event.
4. Add cross-platform `$CURSOR_HOME` detection (default `~/.cursor`, overridable).

### Wave E — Init flag and skill expansion

1. Add `--upgrade` flag to `src/runtime/commands/init.rs` and `InitOptions`.
2. Update `crates/workflow/src/init/{regular,hub}.rs` to honor the flag.
3. Update `plugins/spec/skills/init/SKILL.md` Critical Path, scope, and Guardrails.
4. Update `plugins/spec/skills/init/references/init-runbook.md` with steps 1b, 1c, 1d.
5. Add the `migrated` output template.
6. Update acceptance fixtures to cover the four init shapes: greenfield, brownfield, hub, migrated.
7. Add a re-entry idempotency fixture: run `specrun init --upgrade` over a populated brownfield (and hub) tree, then a second time, asserting that only `project.yaml.specify_version` changes on the first run, every operator-authored artifact is byte-stable, and the second run is a no-op.

### Wave F — Documentation

1. Update `README.md` "Installing the CLI" to mention `specrun upgrade`.
2. Update `docs/orientation/prerequisites.md` to describe channel detection.
3. Update `docs/reference/quick-reference.md` with the new commands.
4. Update the AGENTS.md "Gotchas": replace the "2.0 is a hard cut" sentence with "Major version bumps require a registered `MigrationKind`."
5. Update the [DECISIONS.md "Exit codes"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#exit-codes) section with code 4.

## Migration

Projects without a pinned `specify_version`, and projects pinned to the current major, load unchanged. A project pinned to an older major emits `Error::ProjectNeedsMigration` once a migrator is registered for the path; until then `MigrationKind::resolve(from, to)` returns an empty slice and `ProjectConfig::load` falls through.

For the skill: healthy projects see no behavior change. The three probes are fast no-ops when nothing has drifted, and they run before any prompt that would otherwise fire — so prompt counts only grow when the operator actually needs to choose.

For the CLI: `specrun init --upgrade` is the explicit re-entry flag; `specrun init` without it still refuses to overwrite `project.yaml`. The flag is additive.

## Non-Goals

- A `/spec:upgrade` or `/spec:doctor` slash command. Routine drift handling stays under `/spec:init`.
- Hot-reloading Cursor plugins from inside a running session. The CLI prints a restart instruction and stops.
- Automatic CLI updates without confirmation. Every upgrade requires `--yes` or interactive AskQuestion.
- Cross-major migrations that require operator judgment in artifact contents. RFC-30 migrators are structural; semantic re-extraction belongs to `/spec:plan` + `/spec:execute`.
- Replacing `cargo install --git` as the development install method.
- Defining `tooling check` or `specify review`. RFC-5 and RFC-28 own those surfaces.
- Cross-platform binary distribution beyond what the release pipeline supports.

## Alternatives Considered

**Split into three RFCs (RFC-30a/b/c).** Rejected. The three concerns share the journal event taxonomy, consent pattern, AskQuestion structure, skill carve-out wording, and `/spec:init` runbook structure. Splitting them would create three near-identical change sets with a 4× chance of inconsistency.

**Make `/spec:doctor` the entry point instead of expanding `/spec:init`.** Rejected for routine drift. A second slash command is an onboarding tax — the operator must remember to run it — whereas folding the three probes into `/spec:init` costs one extra `AskQuestion` only when drift exists. `/spec:doctor` may still ship later as a read-only diagnostic for power users.

**Auto-upgrade without confirmation.** Rejected. Downstream projects pin Specify via `specify_version`; surprise upgrades would break the version-floor invariant in subtle ways.

**Treat every major bump as flag-day.** Rejected. Flag-day cuts are not sustainable indefinitely, and the migrator-registration discipline is the cheapest way to keep the door open to schema improvements.

**Ship plugin refresh as a developer-only `make` script.** Rejected. RFC-30 makes refresh a first-class concern for end users who cannot reason about marketplace internals.

**Embed update logic in `specrun init` itself.** Rejected. Folding upgrade, plugin-cache, and migration into one handler would defeat the single-responsibility split the `crates/workflow/src/init/` module enforces. Separate verbs keep tests, fixtures, and journal events orthogonal.

## Open Questions

1. Should `specrun upgrade` for the `cargo` channel pin the latest release tag (`cargo install --git ... --tag vX.Y.Z`) or always track HEAD? Current preference: pin to latest release when reachable; fall back to HEAD with a journal note.
2. Should `specrun plugins doctor` warn about `extra` cache entries? Current preference: report them but exit `0`; cleanup is a `refresh` concern.
3. Should `specrun migrate` support partial migrations (`--only <kind>`)? Current preference: no for v1 — migrators compose to span majors but never run halfway.
4. Should the `migrated` init output template include a diff summary, or only the structured `MigrationReport`? Current preference: structured summary; full diff stays in the journal.
5. Should `specrun init --check-migration` be its own subcommand (e.g. `specrun migrate plan`) instead of a flag on `init`? Current preference: keep it on `init` because the skill is its only caller.
6. Where does CI fit? `cli-upgraded`, `plugins-refreshed`, and `migration-applied` are useful telemetry but require a user identity boundary. Defer to [roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events).
7. Should `Error::ProjectNeedsMigration` carry the migration plan in its payload so a single `--format json` round-trip can drive the agent prompt? Current preference: no — keep the error narrow; the skill calls `--check-migration` to fetch the plan.

## References

- [RFC-13: Extensibility](../done/rfc-13-extensibility.md) — adapter resolution and `specrun init` shape.
- [RFC-25: Workflow](../done/rfc-25-workflow.md) — closed lifecycle vocabulary the migrator preserves.
- [From sources to slices](../../docs/explanation/reconciliation.md) — per-slice fan-out drops `slices[].target` (the target resolves on demand from the bound `project`), so the `V1ToV2` migrator strips that field and binds `project`.
- [Specify CLI `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — exit-code table and the major-version policy this RFC softens.
- [Specify CLI `DECISIONS.md` — Exit codes](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#exit-codes) — long-form rationale for codes 0–3.
- [Roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events) — downstream consumer of the new journal events.
- [Roadmap RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model) — adapter ecosystem migration guidance the framework hooks into.
- [`plugins/spec/skills/init/SKILL.md`](../../plugins/spec/skills/init/SKILL.md) — scope and guardrails this RFC expands.
- [`plugins/spec/skills/init/references/init-runbook.md`](../../plugins/spec/skills/init/references/init-runbook.md) — procedural runbook this RFC extends.
- [`scripts/use-team-plugins.sh`](../../scripts/use-team-plugins.sh) and [`scripts/use-local-plugins.sh`](../../scripts/use-local-plugins.sh) — developer refresh path the CLI verb generalizes.
