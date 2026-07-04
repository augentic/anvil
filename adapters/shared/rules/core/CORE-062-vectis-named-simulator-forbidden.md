---
id: CORE-062
title: Vectis Briefs Forbid Named Simulator Destinations
severity: important
trigger: A Vectis target brief instructs agents to set or pick a named iOS simulator destination instead of the CLI-owned generic destination.
rule_hints:
  - kind: path-pattern
    value: adapters/targets/vectis/briefs/**/*.md
  - kind: regex
    value: "platform=iOS Simulator,name=|-destination[^\\n]*name=iPhone"
    description: Vectis build briefs must not instruct agents to substitute named simulator destinations in scaffold files.
---

## Rule

Vectis iOS verify and merge briefs must never tell agents to patch `iOS/Makefile`, `iOS/project.yml`, or `iOS/.vectis/sim-build.sh` with a named simulator (`name=iPhone …`, `platform=iOS Simulator,name=…`). The generic destination is CLI-owned in `iOS/.vectis/sim-build.sh`; the orchestrator runs `vectis sync ios-scaffold` when repair is needed.

## Look For

- Brief prose or shell recipes that set `platform=iOS Simulator,name=` or `name=iPhone` in agent-facing instructions.
- Verify-repair guidance that tells agents to pick a simulator from `xcrun simctl list` and edit scaffold files.

## Fix

Remove named-destination instructions. Point agents at `specify extension run vectis -- sync ios-scaffold` and Swift-only repair per the Vectis iOS build brief (`adapters/targets/vectis/briefs/build/ios/write.md` in specify-adapters).
