# Runtime Capture Plugin

Runtime capture for migrations. The capture plugin instruments legacy TypeScript services so operators can bind captured runtime data to the [`captures` source adapter](../../../adapters/sources/captures/) at plan time. Replay test generation and build-time verification run through the [Omnia target `build` briefs](../../../adapters/targets/omnia/briefs/build.md) during `/spec:execute` — not through a separate capture skill.

Repository cloning is no longer a dedicated skill — `/capture:wiretapper` inlines a guarded `git clone` snippet directly (see [`plugins/capture/skills/wiretapper/SKILL.md`](../../../plugins/capture/skills/wiretapper/SKILL.md)).

## Skills

### /capture:wiretapper

Capture request/response and side-effect data from a legacy TypeScript service.

**Synopsis:**

```text
/capture:wiretapper <legacy-dir> [app-name <name>]
```

**Inputs:**
- `legacy-dir` — Path to the legacy TypeScript project.
- `--app-name` — Name for the captured wiretap file.

**Outputs:**
- `src/wiretap/` directory with core capture logic and per-pattern adapters.
- Modified entry point that routes through the wiretap layer.
- At runtime: `<app>.wiretap.json` containing captured request/response pairs and side effects.

**Behavior:**
1. Detects patterns in the legacy code (HTTP handlers, message consumers, WebSocket, etc. — patterns A through H).
2. Generates wiretap adapters for each detected pattern.
3. Wires the wiretap into the application entry point.
4. Verifies the modified project compiles.

Captured output must conform to [`captures/references/capture-format.md`](../../../adapters/sources/captures/references/capture-format.md) when converted to the `tests/data/replays/` tree for source binding.

## Migration workflow

```text
git clone "$URL" "$DEST"    --> bootstrap the legacy repo (inlined snippet in wiretapper)
/capture:wiretapper         --> instrument and capture runtime data
/spec:plan                  --> bind sources including captures: runtime=./captures/replays
specify plan transition <name> reviewed
/spec:execute               --> refine extracts Evidence; Omnia build/test.md generates replay tests;
                              build/replay.md runs replay (optional, advisory in v1)
```

Typical bindings alongside wiretap capture:

1. `code-typescript` — legacy code path for static analysis Evidence.
2. `captures` — runtime capture tree for behavioural `kind: example` Evidence.
3. Omnia target — generated crate + replay tests verified during `/spec:build`.

See [`plugins/capture/README.md`](../../../plugins/capture/README.md) and [`adapters/sources/captures/briefs/enumerate.md`](../../../adapters/sources/captures/briefs/enumerate.md) for binding details.
