# Declared Tool Helper Inventory

This inventory tracks first-party helper binaries that should run through [`specify extension`](cli/extension.md) instead of as separate host executables. It is the reviewable source for that migration boundary.

Use this document when adding or changing a first-party deterministic helper. A helper belongs behind `specify extension run` when it is adapter-owned, deterministic, filesystem-bounded, and can run as a WASI Preview 2 command component with explicit permissions. Host workflow remains appropriate when the work requires language toolchains, platform SDKs, package managers, network registries, forge operations, or core Specify lifecycle authority.

## Declared Tools

- `contract`: declared in the singular `extension` object of [`adapters/targets/contracts/adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/contracts/adapter.yaml) and run as `specify extension run contract -- "$PROJECT_ROOT/contracts" --format json`. It validates the merged `contracts/` baseline.
- `vectis` (`validate`): declared in the singular `extension` object of [`adapters/targets/vectis/adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/adapter.yaml) and run as `specify extension run vectis -- validate <mode> [path]`. It owns deterministic Vectis UI input validation for `layout`, `composition`, `tokens`, `assets`, and `all`.
- `vectis` (`scaffold`): declared in the singular `extension` object of [`adapters/targets/vectis/adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/adapter.yaml) and run as `specify extension run vectis -- scaffold <target> <app-name> ...`. It renders Vectis project scaffolds only; host post-processing stays with the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/merge.md) briefs.

## Active Caller Inventory

These active callers are expected to use declared tools and should be kept in sync with the declarations above.

- Contract merge and validation: [`adapters/targets/contracts/briefs/merge.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/contracts/briefs/merge.md), [`adapters/targets/contracts/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/contracts/briefs/build.md), and the verifier siblings under [`adapters/targets/contracts/references/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/contracts/references/) (`openapi/verifier.md`, `asyncapi/verifier.md`, `json-schema/verifier.md`).
- Vectis validation and scaffold rendering: [`adapters/targets/vectis/briefs/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/briefs/) (the consolidated `shape` / `build` / `merge` briefs that carry the `vectis-{core,test,ios,android}-writer`, `vectis-{core,ios,android}-reviewer`, and `vectis-template-updater` behaviors), the [`screenshots` source adapter](../../adapters/sources/screenshots/adapter.yaml) (which houses the `vectis-image-layout-inferer` body), and [`adapters/targets/vectis/references/layout-inferer-contract.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/references/layout-inferer-contract.md).
- Operator docs: [`docs/reference/cli/contract.md`](cli/contract.md), [`docs/reference/cli/vectis.md`](cli/vectis.md), and [`docs/explanation/tool-declarations.md`](../explanation/tool-declarations.md).

## Host-Owned Surfaces

The following command families are intentionally outside this scope. They are not first-party helper binaries with declared-tool equivalents.

- Core Specify lifecycle and orchestration: `specify slice *`, `specify plan *`, `specify source resolve`, `specify target resolve`, `specify registry *`, `specify workspace *`, and `specify init`.
- Forge and transport commands: `git`, `gh`, SSH, PR/MR merge queues, and future forge adapters.
- Language and platform toolchains: `cargo`, `rustup`, `swift`, `swiftformat`, `make`, `xcodebuild`, `gradle`, `./gradlew`, Java/Android SDK tools, `npm`, and `npx`.
- Repository maintenance scripts: `deno run scripts/check.ts`, plugin cache helpers, eval harnesses, and release/build scripts.
- Vectis host verification and template maintenance: verify, version-pin updates, and version queries have no direct WASI wrapper in v1; active skills express them as host workflow or template-updater work, not as declared tools.

## Current status

No additional first-party helper binary in the active skill and brief surface currently meets the migration threshold. Contracts and the filesystem-only Vectis helpers are already declared tools. `omnia` does not currently expose a first-party helper that should become a WASI component.

Future migrations should update this inventory first, then add the WASI component as a co-located `extension/` crate, declare it in the owning target's `adapter.yaml` `extension` object (committing the built `adapter.wasm`), rewrite active callers to `specify extension run`, and extend the consistency check for host-helper spellings.

## Enforcement

`make lint` includes the rule `skill.invokes-host-binary-with-declared-tool-equivalent`. The check scans active adapter briefs and plugin skills for retired first-party helper invocations when a declared-tool equivalent exists. Historical RFCs and explanatory migration docs may still mention retired commands when the prose clearly describes them as retired.
