# RFC-46 baseline snapshot (R46-S00)

Captured **2026-06-11** on branch `rfc-46` before Phase 0 implementation.

## Commits

| Repo | Branch | SHA | Message |
|------|--------|-----|---------|
| `augentic/specify-cli` | `rfc-46` | `1711ebc` | Renumber asset materialization RFC cross-link from 45 to 46. |
| `augentic/specify` | `rfc-46` | `9d3886e` | CI check fix |

## Assurance outputs (all exit 0)

| Command | Log |
|---------|-----|
| `cargo make check` (specify-cli) | [`specify-cli-cargo-make-check.log`](./specify-cli-cargo-make-check.log) |
| `cargo test -p specify-workflow propose` | [`specify-cli-workflow-propose-tests.log`](./specify-cli-workflow-propose-tests.log) — 32 unit tests |
| `cargo test propose` (specify-cli root, `tests/plan.rs`) | [`specify-cli-integration-propose-tests.log`](./specify-cli-integration-propose-tests.log) — 21 integration tests |
| `cargo test -p specify-vectis` (wasi-tools/) | [`specify-cli-vectis-tests.log`](./specify-cli-vectis-tests.log) — 142 tests (34 lib + 18 cli + 90 engine) |
| `make lint` (specify) | [`specify-make-lint.log`](./specify-make-lint.log) — 0 findings |

## Pre-change behaviour (platform bootstrap)

### `specify plan propose --from` and `--reconcile-platforms`

- **`--reconcile-platforms` is opt-in.** `propose.rs` gates bootstrap insertion on the flag (`reconcile_platforms: bool`); when false, `project_missing` is an empty vec and no bootstrap slices are inserted.
- **With the flag**, reconciliation calls workflow `detect_missing_platforms` in `crates/workflow/src/change/plan/core/propose/platforms.rs` — **not** `vectis verify --mode detect`.
- Integration tests (`tests/workflow/propose.rs`) always pass `--reconcile-platforms` for bootstrap scenarios (`propose_reconcile_ok` helper).
- `--reconcile-platforms` conflicts with `--dry-run` (clap `conflicts_with` in `src/runtime/commands/plan/cli.rs`).

### Shell presence heuristics (workflow vs vectis — aligned today)

Both implementations use the same on-disk probes for supported platforms (`core`, `ios`, `android`):

| Platform | Present when |
|----------|--------------|
| `core` | `shared/src/app.rs` exists |
| `ios` | `iOS/` directory contains ≥1 `.swift` file |
| `android` | `Android/` directory contains ≥1 `.kt` file |
| `web`, `desktop` | Treated as present (no on-disk interpretation) |

### Vectis detect JSON shape (`render_detect`)

Confirmed in `wasi-tools/vectis/src/verify.rs` and exercised by `verify::tests::detect_*`:

```json
{
  "mode": "detect",
  "project-root": "<path>",
  "platforms": [{ "platform": "core", "declared": true, "present": false }, …],
  "missing": ["core", "ios", "android"],
  "info": [{ "platform": "web", "id": "platform-not-yet-supported", … }]
}
```

- `missing` is a **string array** of kebab-case platform names for declared-but-absent supported platforms.
- `web` / `desktop` appear in `info`, not `missing`.
- Greenfield (`core,ios,android` declared, no shells): `missing` = `["core","ios","android"]` (`detect_greenfield_returns_all_supported_missing`).
- Partial shell (core + android present, ios absent): `missing` = `["ios"]` (`detect_missing_ios_returns_ios_in_missing`).

**Handoff for R46-S01:** Host helper should parse `missing[]` from this envelope; heuristics already match workflow's `detect_missing_platforms`, so behaviour should be equivalent once wired through vectis.
