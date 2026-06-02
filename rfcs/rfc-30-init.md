# RFC-30: Init Bootstrap, Update, and Migration Lifecycle

> Status: Draft - Depends: [RFC-13](../done/rfc-13-extensibility.md), [RFC-25](../done/rfc-25-workflow.md) - Enables: [roadmap RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model)

## Abstract

`/spec:init` is the only Specify skill that may bootstrap the CLI on first contact. Today that carve-out is narrow: the skill verifies `specify --version`, optionally runs `cargo install --git`, and exits. Three adjacent bootstrap concerns are unowned:

1. **Plugin cache drift.** The skills the agent loads at `~/.cursor/plugins/cache/augentic/<plugin>/<sha>/` can be stale relative to the marketplace, with no first-class detection or refresh path. Developers fall back to `scripts/use-team-plugins.sh` / `scripts/use-local-plugins.sh` and a Cursor restart.
2. **Stale CLI binary.** The current skill only installs when `specify` is missing. There is no probe for "binary present but newer release available," no support for the Homebrew install channel advertised in `README.md`, and the runbook's `$UPGRADE=true` sentinel maps to no CLI flag.
3. **Project-on-old-major-version.** `ProjectConfig::load` enforces a one-way floor with `Error::CliTooOld` (exit 3). The symmetric case — a binary that is a major version newer than the artifacts on disk — has no error, no migrator, and `AGENTS.md` explicitly states "2.0 is a hard cut from 1.x. No compatibility aliases."

This RFC adds a coordinated bootstrap lifecycle. Concretely:

1. **`specify upgrade`** — a channel-aware CLI self-update verb.
2. **`specify plugins {doctor,refresh}`** — Cursor plugin cache inspection and invalidation.
3. **`specify migrate`** — a registered, fixture-backed migrator family keyed off `project.yaml.specify_version`.
4. **`Error::ProjectNeedsMigration`** and a new exit code `4`.
5. **An expanded `/spec:init` runbook** that probes drift in CLI version, plugin cache, and artifact major version before invoking `specify init`, delegating each concern to its owning CLI verb.

The skill stays the orchestrator. The CLI stays the single writer. The lifecycle stops depending on operator memory.

## Motivation

The current contract has three sharp edges:

- A user who pulled the marketplace yesterday and Cursor again today may be running yesterday's `/spec:init` against today's `specify` against last week's plugin cache. The skill cannot tell. Failure modes show up as confusing error text rather than "your cache is stale; restart Cursor."
- The `cargo install --git` install path is one of several legitimate channels. README advertises `brew install augentic/tap/specify`. CI environments use pre-built artifacts. The runbook hardcodes one channel and silently breaks on the others.
- The hard-cut policy in `AGENTS.md` is a load-bearing promise *because* nothing else carries the migration weight. A 2.x → 3.x change without a migrator would either ship breakage or postpone schema improvements indefinitely. RFC-30 makes the policy soft: each major bump must register a migrator before the version field rolls, but it no longer has to be a flag day.

These concerns share substrate. All three are bootstrap problems, all three need the same single-writer CLI discipline, all three are reasonable to surface through one operator entry point. Treating them in three RFCs would create three skill carve-outs, three sets of journal events, and three patterns for "ask, then act, then maybe restart."

## Principles

1. **CLI owns deterministic actions.** Version comparison, cache invalidation, channel detection, and schema migration are CLI verbs. Skills orchestrate intent and consent.
2. **`/spec:init` remains the only bootstrap entry point.** No new operator-facing slash commands for routine cases. `/spec:doctor` is reserved as a future read-only diagnostic, not the upgrade path.
3. **Drift is observable.** Every drift signal — stale CLI, stale plugin cache, project-on-old-major — has a structured detector that can be called outside `/spec:init`.
4. **Consent is explicit.** No upgrade, refresh, or migration runs without an `AskQuestion` confirmation. Headless invocations require an explicit `--yes` flag.
5. **No silent in-place restarts.** When a Cursor restart is required, the skill says so and stops. The CLI never tries to relaunch the IDE.
6. **Migration is fixture-backed.** Every registered migrator has golden inputs and golden outputs. No migrator without coverage; no major version bump without a registered migrator.
7. **Bootstrap concerns are independent.** Plugin refresh, CLI upgrade, and artifact migration may be invoked in any order, succeed or fail independently, and never depend on each other for correctness.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 CLI upgrade verb** | The CLI exposes `specify upgrade` that detects its install channel and self-updates. | Add `src/commands/upgrade.rs`; add `InstallChannel` enum (`cargo`, `brew`, `binary`, `unknown`); shell out to the channel-native upgrade command after confirmation. |
| **D2 Plugin cache verbs** | The CLI exposes `specify plugins doctor` (read-only drift report) and `specify plugins refresh` (cache invalidation). | Add `src/commands/plugins.rs`; locate `$CURSOR_HOME/plugins/cache/<org>/`; compare against `.cursor-plugin/marketplace.json`. |
| **D3 Migration framework** | The CLI exposes `specify migrate` with a closed registry of per-major migrators. | Add `crates/workflow/src/migrate.rs`; add `MigrationKind` closed enum; each variant registers a golden fixture under `tests/migrate/`. |
| **D4 ProjectNeedsMigration error** | `ProjectConfig::load` rejects a project whose `specify_version` major is older than the running binary. | Add `Error::ProjectNeedsMigration { from, to }`; add `Exit::MigrationRequired = 4`; update DECISIONS.md exit-code table. |
| **D5 Init re-entry semantics** | `specify init --upgrade` rewrites `specify_version` and re-scaffolds preservation-safe files only. | Add `--upgrade` flag to `init` clap surface; `crates/workflow/src/init/regular.rs` and `init/hub.rs` route through the same preservation rules as today's first-run case. |
| **D6 Init skill expansion** | `/spec:init` runbook adds three probe steps (CLI version, plugin cache, artifact major) before existing step 2. | Update `plugins/spec/skills/init/SKILL.md` Critical Path and Guardrails; add `references/init-runbook.md` sections 1b, 1c, and 2a. |
| **D7 Bootstrap journal events** | Every CLI-owned bootstrap action emits a journal event with kebab-case discriminant. | Add `cli-upgraded`, `plugins-refreshed`, `migration-applied`, and `migration-skipped` variants to the closed `EventKind` enum in `crates/workflow/src/journal.rs`. |

## Operator surface

Routine first-run with no drift stays identical to today:

```bash
/spec:init https://github.com/augentic/specify/adapters/targets/omnia
```

Routine re-entry — the operator runs `/spec:init` against an existing project — gains three possible branches before any of the existing steps fire:

```text
[probe] specify --version              → suggests `specify upgrade` if stale
[probe] specify plugins doctor          → suggests `specify plugins refresh` if drifted
[probe] specify init --check-migration  → suggests `specify migrate` if needed
```

The three lower-level breakouts may also be invoked directly by power users or CI:

```bash
specify upgrade [--channel cargo|brew|binary] [--yes]
specify plugins doctor [--format json]
specify plugins refresh [--yes]
specify migrate [--from <X.Y>] [--to <X.Y>] [--dry-run] [--yes]
```

Init itself gains one new flag for the case where the operator wants to bump `specify_version` without re-scaffolding:

```bash
specify init --upgrade
```

`--upgrade` is mutually exclusive with the `<adapter>` positional and `--hub`. It refuses to run when `Error::ProjectNeedsMigration` would fire — the operator must `specify migrate` first.

## CLI upgrade verb (D1)

### Command

```bash
specify upgrade [--channel cargo|brew|binary|auto] [--yes] [--format json]
```

### Channel detection

`InstallChannel::detect()` resolves the running binary's path and classifies it:

| Channel | Detection |
| --- | --- |
| `cargo` | Path matches `$CARGO_HOME/bin/specify` (or `~/.cargo/bin/specify` when `CARGO_HOME` is unset). |
| `brew` | Path resolves to a Homebrew Cellar, or `brew --prefix specify` matches the binary's parent. |
| `binary` | Path is under `/usr/local/bin`, `/opt/specify/`, or another known install location; or the file is symlinked from a tagged release artifact. |
| `unknown` | None of the above. `specify upgrade` exits with a structured `unknown-install-channel` diagnostic instructing manual upgrade. |

`--channel` overrides detection.

### Upgrade actions

Per channel:

| Channel | Action |
| --- | --- |
| `cargo` | `cargo install --git https://github.com/augentic/specify-cli` (tag pinned to the resolved latest release when GitHub is reachable; HEAD otherwise with a warning). |
| `brew` | `brew upgrade augentic/tap/specify`. |
| `binary` | Download the latest release archive for the current platform, verify the checksum sidecar, and replace the binary atomically. |

### Latest-version probe

Both `specify upgrade` (mandatory) and `/spec:init` (optional probe) call the same resolver: `gh release view --json tagName -R augentic/specify-cli` when `gh` is on PATH, otherwise an unauthenticated `https://api.github.com/repos/augentic/specify-cli/releases/latest` request. Probe failures are warnings, not errors — the upgrade proceeds against HEAD with a journal note when the latest tag cannot be resolved.

### Journal

```text
cli-upgraded    { from: "0.42.1", to: "0.43.0", channel: "brew" }
```

`from` is the version observed before the upgrade. The new binary writes the event because the old one no longer exists.

## Plugin cache verbs (D2)

### Layout assumptions

Cursor's plugin cache lives under `$CURSOR_HOME/plugins/cache/<org>/<plugin>/<sha>/`. `$CURSOR_HOME` defaults to `~/.cursor` and is overridable. Each plugin directory contains the marketplace-resolved git sha as its leaf segment; `marketplace.json` declares the expected plugins and the `pluginRoot` they live under.

### `specify plugins doctor`

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

`status` values: `ok` (cached sha matches expected), `drifted` (cached sha exists, differs), `missing` (no cache entry), `extra` (cache entry not declared by marketplace).

`specify plugins doctor` is always read-only and never exits non-zero on drift — drift is a finding, not an error. It exits non-zero only on filesystem or marketplace parse failures.

### `specify plugins refresh`

Confirmed cache invalidation. After `--yes` or interactive confirmation:

1. Delete `$CURSOR_HOME/plugins/cache/<org>/` for the marketplace's declared org.
2. Emit `plugins-refreshed { deleted-paths: [...] }`.
3. Print: `Plugin cache cleared. Restart Cursor to repopulate from the marketplace.`
4. Exit `0`.

The CLI does not restart Cursor and does not touch open IDE state. Hot-reload is a Cursor concern.

### Marketplace discovery

The CLI looks for `.cursor-plugin/marketplace.json` in this order: `--marketplace <path>` flag, `$project_dir/.cursor-plugin/marketplace.json`, then `$XDG_CONFIG_HOME/cursor/marketplace.json`. The first hit wins. Cache scope follows the marketplace's `metadata.org` (defaulting to the marketplace filename's parent slug) so multiple orgs coexist.

## Migration framework (D3)

### Command

```bash
specify migrate [--from <X.Y>] [--to <X.Y>] [--dry-run] [--yes] [--format json]
```

`--from` defaults to `project.yaml.specify_version`; `--to` defaults to `CARGO_PKG_VERSION`. `--dry-run` prints the migration plan and journal events that would fire, without writing.

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
    /// `specify init` to render the "would migrate" preview.
    fn plan(&self, project_dir: &Path) -> Result<MigrationPlan>;

    /// Apply the plan atomically (staged write + rename).
    /// Emits per-file journal events for audit.
    fn apply(&self, project_dir: &Path, plan: &MigrationPlan) -> Result<MigrationReport>;
}
```

`MigrationPlan` enumerates file moves, file rewrites, and structured edits. `MigrationReport` is the same shape post-apply, with checksums and a top-level `status`.

### Concrete migrators

The first concrete migrator is `V1ToV2`, covering the breaking changes called out in `AGENTS.md`:

- legacy `pipeline:` manifest key → axis-split `briefs:` keys;
- monolithic `adapter.yaml` → `adapters/sources/<name>/adapter.yaml` + `adapters/targets/<name>/adapter.yaml`;
- retired `change:` slash-namespace references in `AGENTS.md`-style operator notes;
- `discovery.md` legacy candidate format → `## Candidate inventory` block with stable `id`;
- `plan.yaml` slices bind a `project` only; the per-slice `target` field was dropped (the target adapter resolves on demand from the bound project). A V1→V2 migrator must strip any persisted `slices[].target` and ensure each slice carries a resolvable `project`.

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
#[error("project pinned to specify {from} but running {to}; run `specify migrate`")]
ProjectNeedsMigration {
    from: String,
    to: String,
},
```

### Detection

`ProjectConfig::load` gains a symmetric major check next to the existing `CliTooOld` arm:

```rust
let current = env!("CARGO_PKG_VERSION");
let pinned = &cfg.specify_version;

if version_is_older(current, pinned) {
    return Err(Error::CliTooOld { ... });
}
if let Some(pinned) = pinned
    && major(current) > major(pinned)
{
    return Err(Error::ProjectNeedsMigration {
        from: pinned.clone(),
        to: current.to_string(),
    });
}
```

Unparseable pinned versions stay permissive, matching the current `CliTooOld` behavior.

### Exit code

| Code | Name | When |
| --- | --- | --- |
| 0 | `EXIT_SUCCESS` | unchanged |
| 1 | `EXIT_GENERIC_FAILURE` | unchanged |
| 2 | `EXIT_VALIDATION_FAILED` | unchanged |
| 3 | `EXIT_VERSION_TOO_OLD` | unchanged |
| 4 | `EXIT_MIGRATION_REQUIRED` | `Error::ProjectNeedsMigration` |

Update `src/output.rs`'s `Exit::from(&Error)` mapping, the AGENTS.md exit-code table, `docs/standards/handler-shape.md`, and the DECISIONS.md "Exit codes" section.

### Bootstrap-command carve-out

`specify migrate`, `specify upgrade`, `specify plugins {doctor,refresh}`, and `specify init --upgrade` MUST NOT call `ProjectConfig::load` through the standard load path — they need to operate on projects that are explicitly in the "needs migration" state. Each command uses a `ProjectConfig::load_for_migration` variant that returns the parsed config and the `(from, to)` migration tuple without raising `ProjectNeedsMigration`.

## Init re-entry semantics (D5)

### `specify init --upgrade`

Add an `--upgrade` flag to `src/commands/init.rs` and `crates/workflow/src/init/InitOptions`. Behavior:

- Mutually exclusive with `<adapter>` positional and `--hub`.
- Refuses to run if `Error::ProjectNeedsMigration` would fire (the operator must `specify migrate` first).
- Preserves the existing `adapter:` field in `project.yaml`.
- Rewrites `specify_version` to `CARGO_PKG_VERSION`.
- Re-runs the same `context::generate_for_init` path the first-run case uses, but only when `AGENTS.md` is absent (same preservation rule).
- Does not re-fetch the adapter cache unless `--refresh-cache` is also passed.

### Existing `--upgrade` skill sentinel

The runbook's `$UPGRADE=true` pseudo-flag becomes a real CLI flag. The skill body changes from "treat reinit as an upgrade path owned by the CLI" to "invoke `specify init --upgrade` after confirmation."

## Init skill expansion (D6)

### `SKILL.md` changes

`plugins/spec/skills/init/SKILL.md`:

1. Scope statement loses the absolute "not for re-initializing an existing `.specify/`" line and gains "supports first-run init, re-entry upgrades, plugin-cache refresh, and major-version migration handoff."
2. Critical Path gains three new ordered steps before existing step 2:
   - `1b. Probe CLI version` — call `specify upgrade --dry-run --format json` and report drift.
   - `1c. Probe plugin cache` — call `specify plugins doctor --format json` and report drift.
   - `1d. Probe artifact major` — call `specify init --check-migration --format json` and report drift.
3. Guardrails gain the parallel carve-outs: "`/spec:init` is the one Specify skill that may upgrade the CLI" and "...refresh the Cursor plugin cache" and "...trigger a major-version migration."

### `init-runbook.md` changes

`plugins/spec/skills/init/references/init-runbook.md`:

- New step **1b. Probe CLI version**: parse `specify upgrade --dry-run --format json`; on drift, AskQuestion → `specify upgrade --yes` → print restart-not-required confirmation and continue.
- New step **1c. Probe plugin cache**: parse `specify plugins doctor --format json`; on drift, AskQuestion → `specify plugins refresh --yes` → print "Restart Cursor and re-run `/spec:init`" → **stop**.
- New step **1d. Probe artifact major**: parse `specify init --check-migration --format json`; on `needs-migration: true`, AskQuestion → `specify migrate --yes` → continue to existing step 2.
- Existing step 2's reinit branch redirects through `specify init --upgrade` instead of the freeform "treat as upgrade" prose.

### Output templates

Add `migrated` template alongside `greenfield`, `brownfield`, and `hub` in `plugins/spec/skills/init/references/init-output-templates.md`. The migrated template renders the structured `MigrationReport` summary and points at the journal entry for full audit.

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

1. Add `Error::ProjectNeedsMigration` to `crates/error/src/error.rs`.
2. Add `Exit::MigrationRequired = 4` and update `src/output.rs` mapping.
3. Update AGENTS.md, DECISIONS.md, and `docs/standards/handler-shape.md` exit-code tables in the same PR.
4. Add detection branch in `ProjectConfig::load` plus `load_for_migration` variant.
5. Add unit tests in `crates/workflow/src/config.rs` analogous to `load_refuses_future_specify_version`.

### Wave B — Migration framework

1. Add `crates/workflow/src/migrate.rs` with `MigrationKind`, `Migrator`, `MigrationPlan`, `MigrationReport`.
2. Add `src/commands/migrate.rs` with `--from`, `--to`, `--dry-run`, `--yes`, and `--format` flags.
3. Implement `V1ToV2` migrator; check in golden fixtures under `tests/migrate/v1-to-v2/{before,after}/`.
4. Add `migration-applied` and `migration-skipped` journal events.
5. Add `specify init --check-migration` (read-only probe used by the skill).

### Wave C — CLI upgrade verb

1. Add `crates/workflow/src/upgrade.rs` with `InstallChannel::detect` and per-channel upgrade strategy.
2. Add `src/commands/upgrade.rs` and `--channel`, `--yes`, `--format`, `--dry-run` flags.
3. Add the latest-version probe (`gh release view` first, `api.github.com` fallback).
4. Add `cli-upgraded` journal event.

### Wave D — Plugin cache verbs

1. Add `crates/workflow/src/plugins.rs` with marketplace discovery and cache scanning.
2. Add `src/commands/plugins.rs` with `doctor` and `refresh` subcommands.
3. Add `plugins-refreshed` journal event.
4. Add cross-platform `$CURSOR_HOME` detection (default `~/.cursor`, overridable).

### Wave E — Init flag and skill expansion

1. Add `--upgrade` flag to `src/commands/init.rs` and `InitOptions`.
2. Update `crates/workflow/src/init/{regular,hub}.rs` to honor the flag.
3. Update `plugins/spec/skills/init/SKILL.md` Critical Path, scope, and Guardrails.
4. Update `plugins/spec/skills/init/references/init-runbook.md` with steps 1b, 1c, 1d.
5. Add the `migrated` output template.
6. Update the acceptance fixtures to cover the four init shapes: greenfield, brownfield, hub, migrated.

### Wave F — Documentation

1. Update `README.md` "Installing the CLI" to mention `specify upgrade`.
2. Update `docs/orientation/prerequisites.md` to describe channel detection.
3. Update `docs/reference/quick-reference.md` with the new commands.
4. Update `AGENTS.md` "Gotchas" — remove the absolute "2.0 is a hard cut" sentence; replace with "Major version bumps require a registered `MigrationKind`."
5. Update the [DECISIONS.md "Exit codes"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#exit-codes) section with code 4.

## Migration

Existing projects without a pinned `specify_version` continue to load as before. Projects pinned to the current major continue to load. Projects pinned to an older major start emitting `Error::ProjectNeedsMigration` only once a migrator is registered for the path; until then, `MigrationKind::resolve(from, to)` returns an empty slice and `ProjectConfig::load` falls through (preserving today's behavior).

For the skill: existing `/spec:init` invocations on healthy projects see no behavior change. The three probes are fast no-ops when nothing has drifted. The runbook explicitly orders probes before any prompt that would have fired in the old flow, so prompt counts only grow when the operator actually needs to choose.

For the CLI: `specify init --upgrade` replaces the implicit `$UPGRADE=true` runbook sentinel, but the existing reinit confirmation flow still works because `specify init` without `--upgrade` continues to refuse to overwrite `project.yaml`. The flag is additive.

## Non-Goals

- A `/spec:upgrade` or `/spec:doctor` slash command. Routine drift handling stays under `/spec:init`.
- Hot-reloading Cursor plugins from inside a running session. The CLI prints a restart instruction and stops.
- Automatic CLI updates without confirmation. Every upgrade requires `--yes` or interactive AskQuestion.
- Cross-major migrations that require operator judgment in artifact contents. RFC-30 migrators are structural; semantic re-extraction belongs to `/spec:plan` + `/spec:execute`.
- Replacing `cargo install --git` as the development install method.
- Defining `tooling check` or `specify review`. RFC-5 and RFC-28 own those surfaces.
- Cross-platform binary distribution beyond what the existing release pipeline supports.

## Alternatives Considered

**Split into three RFCs (RFC-30a/b/c).** Rejected. The three concerns share the journal event taxonomy, the consent pattern, the AskQuestion structure, the skill carve-out wording, and the `/spec:init` runbook structure. Splitting them would create three near-identical change sets with a 4× chance of inconsistency.

**Make `/spec:doctor` the entry point instead of expanding `/spec:init`.** Rejected for routine drift. A second slash command would be a real onboarding tax — the operator has to remember to run it, when the cost of folding the three probes into `/spec:init` is one extra `AskQuestion` only when drift exists. `/spec:doctor` may still ship later as a read-only diagnostic for power users.

**Auto-upgrade without confirmation.** Rejected. Specify is a CLI that downstream projects pin via `specify_version`. Surprise upgrades would break the version-floor invariant in subtle ways.

**Skip the migration framework and treat every major bump as flag-day.** Rejected. The existing hard-cut policy is sustainable for 1.x → 2.0 because the user base is small. It is not sustainable indefinitely, and the migrator-registration discipline is the cheapest way to keep the door open.

**Ship plugin refresh as a developer-only `make` script.** Rejected. `scripts/use-team-plugins.sh` already covers the developer workflow. RFC-30's contribution is making refresh a first-class concern for end users who cannot reason about the marketplace internals.

**Embed update logic in `specify init` itself.** Rejected. `init` is already the busiest single command; folding upgrade, plugin-cache, and migration into one Rust handler would defeat the single-responsibility split that the existing `crates/workflow/src/init/` module already enforces. Separate verbs keep tests, fixtures, and journal events orthogonal.

## Open Questions

1. Should `specify upgrade` for the `cargo` channel pin the latest release tag (`cargo install --git ... --tag vX.Y.Z`) or always track HEAD? Current preference: pin to latest release when reachable; fall back to HEAD with a journal note.
2. Should `specify plugins doctor` warn about `extra` cache entries (plugins present in cache but not in the marketplace)? Current preference: report them but exit `0`; cleanup is a `refresh` concern.
3. Should `specify migrate` support partial migrations (`--only <kind>`)? Current preference: no for v1 — migrators compose to span majors but never run halfway.
4. Should the `migrated` init output template include a diff summary, or only the structured `MigrationReport`? Current preference: structured summary; full diff stays in the journal.
5. Should `specify init --check-migration` be its own subcommand (e.g. `specify migrate plan`) instead of a flag on `init`? Current preference: keep it on `init` because the skill is its only caller.
6. Where does CI fit? `cli-upgraded`, `plugins-refreshed`, and `migration-applied` are useful telemetry but require a user identity boundary. Defer to [roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events).
7. Should `Error::ProjectNeedsMigration` carry the migration plan in its payload so a single `--format json` round-trip can drive the agent prompt? Current preference: no — keep the error narrow; the skill calls `--check-migration` to fetch the plan.

## References

- [RFC-13: Extensibility](../done/rfc-13-extensibility.md) — adapter resolution and `specify init` shape.
- [RFC-25: Workflow](../done/rfc-25-workflow.md) — closed lifecycle vocabulary the migrator preserves.
- [From sources to slices](../../docs/explanation/reconciliation.md) — per-slice fan-out drops `slices[].target` (the target resolves on demand from the bound `project`), so the `V1ToV2` migrator strips that field and binds `project`.
- [Specify CLI `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — current exit-code table and the "2.0 is a hard cut" policy this RFC softens.
- [Specify CLI `DECISIONS.md` — Exit codes](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#exit-codes) — long-form rationale for the existing codes 0–3.
- [Roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events) — downstream consumer of the new journal events.
- [Roadmap RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model) — adapter ecosystem migration guidance the framework hooks into.
- [`plugins/spec/skills/init/SKILL.md`](../../plugins/spec/skills/init/SKILL.md) — current scope and guardrails this RFC expands.
- [`plugins/spec/skills/init/references/init-runbook.md`](../../plugins/spec/skills/init/references/init-runbook.md) — current procedural runbook this RFC extends.
- [`scripts/use-team-plugins.sh`](../../scripts/use-team-plugins.sh) and [`scripts/use-local-plugins.sh`](../../scripts/use-local-plugins.sh) — existing developer-only refresh path the CLI verb generalizes.
