# Emery

Emery routes specification generation through the `/emery:specify` wrapper over the `emery` CLI. The v1 delivery workflow (plan / refine / execute / status / finalize and the `system-*` definition loop) is frozen and archived at tag `v1` while the spec-generator remediation programme is in flight (ADR-0008); its skill wrappers are deleted, not hidden — they return with the new surface when the walking skeleton lands.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Orchestration and validation live in the CLI.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [specify](skills/specify/SKILL.md) | `/emery:specify` | Generate `spec.md` / `design.md` from the named sources (`emery specify`) |
