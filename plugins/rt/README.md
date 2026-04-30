# RT Migration

Fixture capture and regression testing for migration workflows. Source code analysis has moved to `/spec:extract` in the spec plugin; the previous clone skill has been retired in favour of an inlined guarded `git clone` snippet at the two callers — see the *Cloning a source tree* subsection in [`plugins/spec/skills/analyze/SKILL.md`](../spec/skills/analyze/SKILL.md) (or [`plugins/rt/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap).

## Skills

| Skill | Description |
|-------|-------------|
| [replay-writer](skills/replay-writer/SKILL.md) | Add regression tests from captured real-world fixtures |
| [wiretapper](skills/wiretapper/SKILL.md) | Capture fixture data from legacy services |
