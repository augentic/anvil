# RT Migration

Fixture capture and regression testing for migration workflows. Source-code enumeration lives in the [`code-typescript` source adapter](../../adapters/sources/code-typescript/briefs/enumerate.md); cloning a source tree is an inlined guarded `git clone` snippet at the callers — see [`plugins/rt/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap.

## Skills

| Skill | Description |
|-------|-------------|
| [replay-writer](skills/replay-writer/SKILL.md) | Add regression tests from captured real-world fixtures |
| [wiretapper](skills/wiretapper/SKILL.md) | Capture fixture data from legacy services |
