# RT Migration

Fixture capture for migration workflows. Source-code enumeration lives in the [`code-typescript` source adapter](../../adapters/sources/code-typescript/briefs/enumerate.md); runtime fixture consumption lives in the [`runtime-fixtures` source adapter](../../adapters/sources/runtime-fixtures/); replay test generation and build-time verification live in the [Omnia target `build` briefs](../../adapters/targets/omnia/briefs/build.md). Cloning a source tree is an inlined guarded `git clone` snippet at the callers — see [`plugins/rt/skills/wiretapper/SKILL.md`](skills/wiretapper/SKILL.md) for legacy-repo bootstrap.

## Skills

| Skill | Description |
|-------|-------------|
| [wiretapper](skills/wiretapper/SKILL.md) | Capture fixture data from legacy TypeScript services |

## Migration workflow

```text
git clone "$URL" "$DEST"   --> bootstrap the legacy repo (inlined snippet in wiretapper)
/rt:wiretapper             --> instrument and capture fixtures
/spec:plan                 --> bind runtime-fixtures source: runtime=./fixtures/replay
specify plan transition … reviewed
/spec:execute              --> refine/build/merge; Omnia build/test.md + build/replay.md consume fixtures
```

Captured fixtures must follow [`runtime-fixtures/references/fixture-format.md`](../../adapters/sources/runtime-fixtures/references/fixture-format.md).
