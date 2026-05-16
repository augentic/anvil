# RT Migration

Fixture capture and regression testing for migration workflows. Source-code analysis lives in `/spec:extract`; cloning a source tree is an inlined guarded `git clone` snippet at the callers — see the *Cloning a source tree* subsection in [`plugins/change/skills/analyze/SKILL.md`](../change/skills/analyze/SKILL.md) (or [`plugins/rt/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap).

## Skills

| Skill | Description |
|-------|-------------|
| [replay-writer](skills/replay-writer/SKILL.md) | Add regression tests from captured real-world fixtures |
| [wiretapper](skills/wiretapper/SKILL.md) | Capture fixture data from legacy services |
