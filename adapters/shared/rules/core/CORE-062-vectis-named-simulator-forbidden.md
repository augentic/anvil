---
id: CORE-062
title: Vectis Prompts Forbid Named Simulator Destinations
severity: important
trigger: A Vectis target prompt instructs agents to set or pick a named iOS simulator destination instead of the CLI-owned generic destination.
rule_hints:
  - kind: path-pattern
    value: adapters/targets/vectis/prose/prompts/**/*.md
  - kind: regex
    value: "platform=iOS Simulator,name=|-destination[^\\n]*name=iPhone"
    description: Vectis build prompts must not instruct agents to substitute named simulator destinations in scaffold files.
---

## Rule

Vectis iOS verify and merge prompts must never tell agents to patch `iOS/Makefile`, `iOS/project.yml`, or `iOS/.vectis/sim-build.sh` with a named simulator (`name=iPhone …`, `platform=iOS Simulator,name=…`). The generic destination is CLI-owned in `iOS/.vectis/sim-build.sh`; the orchestrator runs `vectis sync ios-scaffold` when repair is needed.

## Look For

- Prompt prose or shell recipes that set `platform=iOS Simulator,name=` or `name=iPhone` in agent-facing instructions.
- Verify-repair guidance that tells agents to pick a simulator from `xcrun simctl list` and edit scaffold files.

## Fix

Remove named-destination instructions. Point agents at `specify extension run vectis -- sync ios-scaffold` and Swift-only repair per the Vectis iOS build prompt (`targets/vectis/prose/prompts/build/ios/write.md` in specify-adapters).
