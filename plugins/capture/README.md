# Runtime Capture

Runtime capture for migration workflows. Source-code survey lives in the [`typescript` source adapter](../../adapters/sources/typescript/briefs/survey.md); capture consumption lives in the [`captures` source adapter](../../adapters/sources/captures/); replay test generation and build-time verification live in the [Omnia target `build` briefs](../../adapters/targets/omnia/briefs/build.md). Cloning a source tree is an inlined guarded `git clone` snippet at the callers — see [`plugins/capture/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap.

## Skills

| Skill | Description |
|-------|-------------|
| [wiretapper](skills/wiretapper/SKILL.md) | Capture runtime data from legacy TypeScript services |

## Migration workflow

```text
git clone "$URL" "$DEST"    --> bootstrap the legacy repo (inlined snippet in wiretapper)
/capture:wiretapper         --> instrument and capture runtime data
/spec:plan                  --> bind captures source: runtime=./captures/replays
specify plan transition ... approved
/spec:execute               --> refine/build/merge; Omnia build/test.md + build/replay.md consume replays
```

Captured runtime data must follow [`captures/references/capture-format.md`](../../adapters/sources/captures/references/capture-format.md).
