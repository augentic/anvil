# Declared Tool Helper Inventory

This inventory tracks first-party deterministic helpers and where they live. Since the Omnia-migration cutover, adapter-owned helpers are **in-guest library code** compiled into each adapter's committed `guest.wasm` — the host dispatches no adapter WASI tool. The one surviving declared-tool surface is project-scope `tools[]` in `.specify/project.yaml`, resolved and run by `specify lint project` (see [Declared WASI tools](cli/extension.md)).

A helper belongs in-guest when it is adapter-owned, deterministic, and filesystem-bounded. Host workflow remains appropriate when the work requires language toolchains, platform SDKs, package managers, network registries, forge operations, or core Specify lifecycle authority.

## In-guest helpers

- `contract`: library code in the contracts adapter's guest ([`adapters/targets/contracts/`](https://github.com/augentic/specify-adapters/tree/main/targets/contracts)). It validates the merged `contracts/` baseline; the contracts build and merge orchestrations invoke it directly.
- `vectis` (`validate`): library code in the vectis core ([`adapters/targets/vectis/`](https://github.com/augentic/specify-adapters/tree/main/targets/vectis)). It owns deterministic Vectis UI input validation for `layout`, `composition`, `tokens`, `assets`, and `all`.
- `vectis` (`scaffold`): library code in the vectis core. It renders Vectis project scaffolds only; host post-processing stays with the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/briefs/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/briefs/merge.md) briefs.
- `vectis` (`sync`): library code in the vectis core. It re-renders agent-immutable `iOS/Makefile` and `iOS/project.yml` from embedded templates without build-prelude side effects; the Vectis iOS build brief also runs it at verify time.

## Active Caller Inventory

These active callers consume the in-guest helpers and should be kept in sync with the list above.

- Contract merge and validation: [`adapters/targets/contracts/prose/briefs/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/briefs/merge.md), [`adapters/targets/contracts/prose/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/briefs/build.md), and the verifier siblings under [`adapters/targets/contracts/prose/references/`](https://github.com/augentic/specify-adapters/tree/main/targets/contracts/prose/references/) (`openapi/verifier.md`, `asyncapi/verifier.md`, `json-schema/verifier.md`).
- Vectis validation and scaffold rendering: [`adapters/targets/vectis/prose/briefs/`](https://github.com/augentic/specify-adapters/tree/main/targets/vectis/prose/briefs/) (the consolidated `shape` / `build` / `merge` briefs that carry the `vectis-{core,test,ios,android}-writer`, `vectis-{core,ios,android}-reviewer`, and `vectis-template-updater` behaviors), the [`screenshots` source adapter](https://github.com/augentic/specify-adapters/blob/main/sources/screenshots/adapter.yaml) (which houses the `vectis-image-layout-inferer` body), and [`adapters/targets/vectis/prose/references/layout-inferer-contract.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/references/layout-inferer-contract.md).
- Operator docs: [`docs/reference/cli/contract.md`](cli/contract.md), [`docs/reference/cli/vectis.md`](cli/vectis.md), and [`docs/explanation/tool-declarations.md`](../explanation/tool-declarations.md).

## Host-Owned Surfaces

The following command families are intentionally outside this scope. They are not first-party helper binaries with in-guest equivalents.

- Core Specify lifecycle and orchestration: `specify slice *`, `specify plan *`, `specify source resolve`, `specify target resolve`, `specify registry *`, `specify workspace *`, and `specify init`.
- Forge and transport commands: `git`, `gh`, SSH, PR/MR merge queues, and future forge adapters.
- Language and platform toolchains: `cargo`, `rustup`, `swift`, `swiftformat`, `make`, `xcodebuild`, `gradle`, `./gradlew`, Java/Android SDK tools, `npm`, and `npx`.
- Repository maintenance scripts: `deno run scripts/check.ts`, plugin cache helpers, eval harnesses, and release/build scripts.
- Vectis host verification and template maintenance: verify, version-pin updates, and version queries have no direct in-guest wrapper in v1; active skills express them as host workflow or template-updater work.

## Current status

No additional first-party helper binary in the active skill and brief surface currently meets the migration threshold. Contracts and the filesystem-only Vectis helpers are already in-guest. `omnia` does not currently expose a first-party helper that should become guest library code.

Future migrations should update this inventory first, then land the behaviour as library code in the owning adapter's guest and rewrite active callers to consume it there.

## Enforcement

`make lint` includes the rule `skill.invokes-host-binary-with-declared-tool-equivalent`. The check scans active adapter briefs and plugin skills for retired first-party helper invocations when an in-guest equivalent exists. Historical RFCs and explanatory migration docs may still mention retired commands when the prose clearly describes them as retired.
