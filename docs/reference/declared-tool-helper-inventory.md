# Declared Tool Helper Inventory

This inventory tracks first-party helper binaries that should run through [`specify tool`](cli/tool.md) instead of as separate host executables. It is the reviewable source for that migration boundary.

Use this document when adding or changing a first-party deterministic helper. A helper belongs behind `specify tool run` when it is capability-owned, deterministic, filesystem-bounded, and can run as a WASI Preview 2 command component with explicit permissions. Host workflow remains appropriate when the work requires language toolchains, platform SDKs, package managers, network registries, forge operations, or core Specify lifecycle authority.

## Declared Tools

- `contract`: declared as `specify:contract@0.3.0` by [`capabilities/contracts/tools.yaml`](../../capabilities/contracts/tools.yaml) and run as `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. It validates the merged `contracts/` baseline.
- `vectis` (`validate`): declared as `specify:vectis@0.3.0` by [`capabilities/vectis/tools.yaml`](../../capabilities/vectis/tools.yaml) and run as `specify tool run vectis -- validate <mode> [path]`. It owns deterministic Vectis UI input validation for `layout`, `composition`, `tokens`, `assets`, and `all`.
- `vectis` (`scaffold`): declared as `specify:vectis@0.3.0` by [`capabilities/vectis/tools.yaml`](../../capabilities/vectis/tools.yaml) and run as `specify tool run vectis -- scaffold <target> <app-name> ...`. It renders Vectis project scaffolds only; host post-processing stays with Vectis skills.

## Active Caller Inventory

These active callers are expected to use declared tools and should be kept in sync with the declarations above.

- Contract merge and validation: [`capabilities/contracts/briefs/merge.md`](../../capabilities/contracts/briefs/merge.md), [`capabilities/contracts/briefs/build.md`](../../capabilities/contracts/briefs/build.md), and the verifier siblings under [`plugins/contract/skills/`](../../plugins/contract/skills/).
- Vectis validation and scaffold rendering: [`capabilities/vectis/briefs/`](../../capabilities/vectis/briefs/), [`plugins/vectis/skills/ios-writer/SKILL.md`](../../plugins/vectis/skills/ios-writer/SKILL.md), [`plugins/vectis/skills/android-writer/SKILL.md`](../../plugins/vectis/skills/android-writer/SKILL.md), [`plugins/vectis/skills/image-layout-inferer/SKILL.md`](../../plugins/vectis/skills/image-layout-inferer/SKILL.md), and [`plugins/vectis/references/layout-inferer-contract.md`](../../plugins/vectis/references/layout-inferer-contract.md).
- Operator docs: [`docs/reference/cli/contract.md`](cli/contract.md), [`docs/reference/cli/vectis.md`](cli/vectis.md), and [`docs/explanation/tool-declarations.md`](../explanation/tool-declarations.md).

## Host-Owned Surfaces

The following command families are intentionally outside this scope. They are not first-party helper binaries with declared-tool equivalents.

- Core Specify lifecycle and orchestration: `specify slice *`, `specify change *`, `specify registry *`, `specify workspace *`, `specify capability *`, `specify codex *`, `specify context *`, and `specify init`.
- Forge and transport commands: `git`, `gh`, SSH, PR/MR merge queues, and future forge adapters.
- Language and platform toolchains: `cargo`, `rustup`, `swift`, `swiftformat`, `make`, `xcodebuild`, `gradle`, `./gradlew`, Java/Android SDK tools, `npm`, and `npx`.
- Repository maintenance scripts: `deno run scripts/checks.ts`, plugin cache helpers, acceptance harnesses, and release/build scripts.
- Vectis host verification and template maintenance: verify, version-pin updates, and version queries have no direct WASI wrapper in v1; active skills express them as host workflow or template-updater work, not as declared tools.

## Current status

No additional first-party helper binary in the active skill and brief surface currently meets the migration threshold. Contracts and the filesystem-only Vectis helpers are already declared tools. `default` and `omnia` do not currently expose a first-party helper that should become a WASI component.

Future migrations should update this inventory first, then add the WASI component, declare it in the owning capability's `tools.yaml`, rewrite active callers to `specify tool run`, and extend the consistency check for retired host-helper spellings.

## Enforcement

`make checks` includes the rule `skill.invokes-host-binary-with-declared-tool-equivalent`. The check scans active capability briefs and plugin skills for retired first-party helper invocations when a declared-tool equivalent exists. Historical RFCs and explanatory migration docs may still mention retired commands when the prose clearly describes them as retired.
