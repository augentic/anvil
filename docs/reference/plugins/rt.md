# RT Plugin

Fixture capture for migrations. The RT plugin instruments legacy TypeScript services so operators can bind captured runtime fixtures to the [`runtime-fixtures` source adapter](../../../adapters/sources/runtime-fixtures/) at plan time. Replay test generation and build-time verification run through the [Omnia target `build` briefs](../../../adapters/targets/omnia/briefs/build.md) during `/spec:execute` — not through a separate RT skill.

Repository cloning is no longer a dedicated skill — `/rt:wiretapper` inlines a guarded `git clone` snippet directly (see [`plugins/rt/skills/wiretapper/SKILL.md`](../../../plugins/rt/skills/wiretapper/SKILL.md)).

## Skills

### /rt:wiretapper

Capture request/response and side-effect data from a legacy TypeScript service.

**Synopsis:**

```text
/rt:wiretapper <legacy-dir> [app-name <name>]
```

**Inputs:**
- `legacy-dir` — Path to the legacy TypeScript project.
- `--app-name` — Name for the captured fixture file.

**Outputs:**
- `src/wiretap/` directory with core capture logic and per-pattern adapters.
- Modified entry point that routes through the wiretap layer.
- At runtime: `<app>.wiretap.json` containing captured request/response pairs and side effects.

**Behavior:**
1. Detects patterns in the legacy code (HTTP handlers, message consumers, WebSocket, etc. — patterns A through H).
2. Generates wiretap adapters for each detected pattern.
3. Wires the wiretap into the application entry point.
4. Verifies the modified project compiles.

Captured output must conform to [`runtime-fixtures/references/fixture-format.md`](../../../adapters/sources/runtime-fixtures/references/fixture-format.md) when converted to the `tests/data/replay/` tree for source binding.

## Migration workflow

```text
git clone "$URL" "$DEST"   --> bootstrap the legacy repo (inlined snippet in wiretapper)
/rt:wiretapper             --> instrument and capture fixtures
/spec:plan                 --> bind sources including runtime-fixtures: runtime=./fixtures/replay
specify plan transition <name> reviewed
/spec:execute              --> refine extracts Evidence; Omnia build/test.md generates replay tests;
                             build/replay.md runs fixture replay (optional, advisory in v1)
```

Typical bindings alongside wiretap capture:

1. `code-typescript` — legacy code path for static analysis Evidence.
2. `runtime-fixtures` — captured fixture tree for behavioural `kind: example` Evidence.
3. Omnia target — generated crate + replay tests verified during `/spec:build`.

See [`plugins/rt/README.md`](../../../plugins/rt/README.md) and [`adapters/sources/runtime-fixtures/briefs/enumerate.md`](../../../adapters/sources/runtime-fixtures/briefs/enumerate.md) for binding details.
