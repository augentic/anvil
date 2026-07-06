# Runtime Capture

Runtime capture for migration workflows. Source-code survey lives in the [`typescript` source adapter](https://github.com/augentic/specify-adapters/blob/main/sources/typescript/prose/prompts/survey.md); capture consumption lives in the [`captures` source adapter](https://github.com/augentic/specify-adapters/tree/main/sources/captures/); replay test generation and build-time verification live in the [Omnia target `build` prompts](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/build.md). Cloning a source tree is an inlined guarded `git clone` snippet at the callers — see [`plugins/capture/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap.

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
specify plan execute               --> refine/build/merge; Omnia build/test.md + build/replay.md consume replays
```

Captured runtime data must follow [`captures/references/capture-format.md`](https://github.com/augentic/specify-adapters/blob/main/sources/captures/prose/references/capture-format.md).
